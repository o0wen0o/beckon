<script lang="ts">
  // Settings is an *editor of files*, not their owner (ADR-0003): every change
  // commits to disk (debounced), and `config-changed` / `actions-changed` events
  // re-render the form. The only local state is the field currently being typed
  // into — adopting a snapshot mid-keystroke would fight the user.
  import { onMount } from "svelte";
  import {
    createAction,
    deleteAction,
    deleteApiKey,
    describeError,
    getActions,
    getConfig,
    getKeyStatus,
    getStartupErrors,
    onActionsChanged,
    onConfigChanged,
    readActionRaw,
    revealConfigDir,
    saveAction,
    saveConfig,
    setApiKey,
    Subscriptions,
    testConnection,
    writeActionRaw,
  } from "../lib/ipc";
  import type {
    Action,
    ActionFile,
    Config,
    InputSource,
    KeyStatus,
    RegistrySnapshot,
  } from "../lib/types";
  import HotkeyInput from "./HotkeyInput.svelte";

  const SAVE_DEBOUNCE = 400;

  let config = $state<Config | null>(null);
  let snapshot = $state<RegistrySnapshot>({ actions: [], errors: [], hotkey_errors: {} });
  let keyStatus = $state<KeyStatus | null>(null);
  let startupErrors = $state<string[]>([]);
  let saveError = $state<string | null>(null);

  let keyDraft = $state("");
  let keyMessage = $state<string | null>(null);
  let test = $state<{ state: "idle" | "running" | "ok" | "failed"; message?: string }>({
    state: "idle",
  });

  let selectedFile = $state<string | null>(null);
  let draft = $state<ActionFile | null>(null);
  let editorFocused = $state(false);
  let configFocused = $state(false);
  let raw = $state<{ file: string; text: string; error?: string } | null>(null);

  const selected = $derived(
    selectedFile === null
      ? null
      : (snapshot.actions.find((action) => action.file_name === selectedFile) ?? null),
  );
  const firstRun = $derived(keyStatus !== null && keyStatus.kind !== "present");

  const subscriptions = new Subscriptions();

  /**
   * One debounced save slot. ADR-0003 makes disk authoritative, so every edit
   * has to land there — but not on every keystroke.
   */
  function saveSlot() {
    let timer: ReturnType<typeof setTimeout> | undefined;
    return (write: () => Promise<void>, immediate: boolean) => {
      clearTimeout(timer);
      const run = async () => {
        try {
          await write();
          saveError = null;
        } catch (error) {
          saveError = describeError(error).message;
        }
      };
      if (immediate) void run();
      else timer = setTimeout(run, SAVE_DEBOUNCE);
    };
  }

  const saveConfigSoon = saveSlot();
  const saveActionSoon = saveSlot();

  onMount(() => {
    void refreshAll();
    subscriptions
      .add(
        onConfigChanged((next) => {
          // Do not yank a field out from under the cursor.
          if (!configFocused) config = next;
        }),
      )
      .add(
        onActionsChanged((next) => {
          snapshot = next;
          if (!editorFocused) syncDraft();
        }),
      );
    return () => void subscriptions.dispose();
  });

  async function refreshAll() {
    config = await getConfig();
    snapshot = await getActions();
    keyStatus = await getKeyStatus();
    startupErrors = await getStartupErrors();
    syncDraft();
  }

  function syncDraft() {
    if (!selected) {
      draft = null;
      return;
    }
    draft = structuredClone({
      name: selected.name,
      description: selected.description ?? null,
      input_source: selected.input_source,
      hotkey: selected.hotkey ?? null,
      prompt: { system: selected.prompt.system, user: selected.prompt.user ?? null },
      model: { ...selected.model },
    });
  }

  function select(action: Action) {
    selectedFile = action.file_name;
    raw = null;
    syncDraft();
  }

  // --- config -------------------------------------------------------------

  function commitConfig(immediate = false) {
    if (!config) return;
    const next = structuredClone(config);
    saveConfigSoon(() => saveConfig(next), immediate);
  }

  function setLauncherHotkey(accelerator: string | null) {
    if (!config || !accelerator) return;
    config.launcher_hotkey = accelerator;
    commitConfig(true);
  }

  // --- actions ------------------------------------------------------------

  function commitAction(immediate = false) {
    if (!draft || !selectedFile) return;
    const fileName = selectedFile;
    const next = structuredClone(draft);
    saveActionSoon(() => saveAction(fileName, next), immediate);
  }

  function setActionHotkey(accelerator: string | null) {
    if (!draft) return;
    draft.hotkey = accelerator;
    commitAction(true);
  }

  async function addAction() {
    try {
      const fileName = await createAction("New Action");
      snapshot = await getActions();
      selectedFile = fileName;
      syncDraft();
    } catch (error) {
      saveError = describeError(error).message;
    }
  }

  async function removeAction(action: Action) {
    if (!confirm(`Delete ${action.file_name}? The file is removed from disk.`)) return;
    try {
      await deleteAction(action.file_name);
      if (selectedFile === action.file_name) {
        selectedFile = null;
        draft = null;
      }
      snapshot = await getActions();
    } catch (error) {
      saveError = describeError(error).message;
    }
  }

  async function openRaw(fileName: string) {
    try {
      raw = { file: fileName, text: await readActionRaw(fileName) };
      selectedFile = null;
      draft = null;
    } catch (error) {
      saveError = describeError(error).message;
    }
  }

  async function saveRaw() {
    if (!raw) return;
    try {
      await writeActionRaw(raw.file, raw.text);
      raw = { ...raw, error: undefined };
      snapshot = await getActions();
    } catch (error) {
      raw = { ...raw, error: describeError(error).message };
    }
  }

  // --- secrets ------------------------------------------------------------

  async function saveKey() {
    if (keyDraft.trim() === "") return;
    try {
      keyStatus = await setApiKey(keyDraft);
      keyDraft = "";
      keyMessage = "Saved to the Windows Credential Manager.";
      test = { state: "idle" };
    } catch (error) {
      keyMessage = describeError(error).message;
    }
  }

  async function removeKey() {
    try {
      keyStatus = await deleteApiKey();
      keyMessage = "Removed.";
    } catch (error) {
      keyMessage = describeError(error).message;
    }
  }

  // Kinds matter here: a rejected key is not an unreachable API, and neither is
  // a missing credential (ADR-0005).
  const TEST_FAILURE_PREFIX: Record<string, string> = {
    auth: "The API rejected this key",
    network: "Could not reach the API",
    "no-credential": "No key stored",
    "read-error": "The Credential Manager could not be read",
  };

  async function runTest() {
    test = { state: "running" };
    try {
      await testConnection();
      test = { state: "ok", message: "The key and base URL work." };
    } catch (error) {
      const failure = describeError(error);
      const prefix = TEST_FAILURE_PREFIX[failure.kind] ?? "Failed";
      test = { state: "failed", message: `${prefix}: ${failure.message}` };
    }
  }

  // --- small helpers ------------------------------------------------------

  const sources: InputSource[] = ["selection", "prompt", "auto"];

  function thinkingChoice(value: boolean | null): string {
    return value === null ? "inherit" : value ? "on" : "off";
  }

  function setThinking(value: string) {
    if (!draft) return;
    draft.model.thinking = value === "inherit" ? null : value === "on";
    commitAction(true);
  }

  function numberOrNull(value: string): number | null {
    const trimmed = value.trim();
    if (trimmed === "") return null;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : null;
  }

  function actionHotkeyError(action: Action | null): string | undefined {
    return action ? snapshot.hotkey_errors[action.id] : undefined;
  }
</script>

<main>
  <h1>Beckon</h1>

  {#if firstRun}
    <div class="banner">
      <strong>Welcome.</strong> Beckon needs a DeepSeek API key before it can do anything. The key
      goes into the Windows Credential Manager, never into a file.
    </div>
  {/if}

  {#if startupErrors.length > 0}
    <div class="banner bad">
      <strong>A hotkey is not active.</strong>
      <ul>
        {#each startupErrors as error}<li>{error}</li>{/each}
      </ul>
      Record a different combination below; it is registered the moment you record it.
    </div>
  {/if}

  {#if saveError}
    <div class="banner bad"><strong>Not saved:</strong> {saveError}</div>
  {/if}

  <!-- API ------------------------------------------------------------- -->
  <section>
    <h2>API</h2>

    <label>
      <span>API key</span>
      <div class="row">
        <input
          type="password"
          bind:value={keyDraft}
          placeholder={keyStatus?.kind === "present"
            ? `stored — ends in ${keyStatus.last4}`
            : "sk-…"}
          autocomplete="off"
          onkeydown={(event) => event.key === "Enter" && saveKey()}
        />
        <button class="primary" disabled={keyDraft.trim() === ""} onclick={saveKey}>Save</button>
        {#if keyStatus?.kind === "present"}
          <button class="danger" onclick={removeKey}>Remove</button>
        {/if}
      </div>
      {#if keyStatus?.kind === "read-error"}
        <p class="hint error">
          The Credential Manager could not be read: {keyStatus.message}. Save the key again to
          recreate the credential.
        </p>
      {:else if keyStatus?.kind === "no-credential"}
        <p class="hint">No key stored yet.</p>
      {:else if keyMessage}
        <p class="hint">{keyMessage}</p>
      {/if}
    </label>

    {#if config}
      <label>
        <span>Base URL</span>
        <input
          bind:value={config.api.base_url}
          onfocus={() => (configFocused = true)}
          onblur={() => {
            configFocused = false;
            commitConfig(true);
          }}
          oninput={() => commitConfig()}
          spellcheck="false"
        />
        <p class="hint">Any OpenAI-compatible endpoint. Requests go to <code>/v1/chat/completions</code>.</p>
      </label>
    {/if}

    <div class="row">
      <button onclick={runTest} disabled={test.state === "running"}>
        {test.state === "running" ? "Testing…" : "Test connection"}
      </button>
      {#if test.message}
        <span class="hint" class:error={test.state === "failed"} class:ok={test.state === "ok"}>
          {test.message}
        </span>
      {/if}
    </div>
  </section>

  <!-- Triggering ------------------------------------------------------ -->
  {#if config}
    <section>
      <h2>Triggering</h2>

      <label>
        <span>Launcher hotkey</span>
        <HotkeyInput value={config.launcher_hotkey} onchange={setLauncherHotkey} />
        <p class="hint">
          Recorded hotkeys are registered immediately — if the combination is taken it goes red and
          is not saved.
        </p>
      </label>

      <label class="checkbox">
        <input
          type="checkbox"
          bind:checked={config.autostart}
          onchange={() => commitConfig(true)}
        />
        <span>Start with Windows</span>
      </label>
    </section>

    <!-- Model defaults ------------------------------------------------ -->
    <section>
      <h2>Model defaults</h2>
      <p class="hint">Any Action can override these in its own <code>[model]</code> table.</p>

      <div class="grid">
        <label>
          <span>Model</span>
          <input
            bind:value={config.defaults.model}
            onfocus={() => (configFocused = true)}
            onblur={() => {
              configFocused = false;
              commitConfig(true);
            }}
            oninput={() => commitConfig()}
            spellcheck="false"
          />
        </label>

        <label>
          <span>Temperature</span>
          <input
            type="number"
            step="0.1"
            min="0"
            max="2"
            bind:value={config.defaults.temperature}
            onfocus={() => (configFocused = true)}
            onblur={() => {
              configFocused = false;
              commitConfig(true);
            }}
            oninput={() => commitConfig()}
          />
        </label>

        <label class="checkbox">
          <input
            type="checkbox"
            bind:checked={config.defaults.thinking}
            onchange={() => commitConfig(true)}
          />
          <span>Thinking mode</span>
        </label>
      </div>
      <p class="hint">
        DeepSeek thinks by default. Leaving it on adds seconds of latency to translation-shaped
        Actions, which is why this is off unless you ask for it.
      </p>
    </section>
  {/if}

  <!-- Actions --------------------------------------------------------- -->
  <section>
    <div class="section-head">
      <h2>Actions</h2>
      <div class="row">
        <button onclick={addAction}>New Action</button>
        <button onclick={() => revealConfigDir()}>Open folder</button>
      </div>
    </div>

    <div class="actions">
      <ul class="action-list">
        {#each snapshot.actions as action (action.file_name)}
          <li>
            <button
              class="action-row"
              class:selected={selectedFile === action.file_name}
              onclick={() => select(action)}
            >
              <span class="action-name">{action.name}</span>
              <span class="action-file">{action.file_name}</span>
              {#if actionHotkeyError(action)}
                <span class="badge bad" title={actionHotkeyError(action)}>hotkey</span>
              {:else if action.hotkey}
                <kbd>{action.hotkey}</kbd>
              {/if}
            </button>
          </li>
        {/each}

        {#each snapshot.errors as error (error.file_name)}
          <li>
            <button class="action-row broken" onclick={() => openRaw(error.file_name)}>
              <span class="action-name">{error.file_name}</span>
              <span class="badge bad">does not parse</span>
            </button>
          </li>
        {/each}

        {#if snapshot.actions.length === 0 && snapshot.errors.length === 0}
          <li class="hint empty">No Actions yet.</li>
        {/if}
      </ul>

      <div
        class="editor"
        onfocusin={() => (editorFocused = true)}
        onfocusout={() => (editorFocused = false)}
      >
        {#if raw}
          <h3>{raw.file}</h3>
          <p class="hint">
            This file does not parse, so it is edited as text. Fix it and save; it reloads
            immediately.
          </p>
          <textarea class="raw" bind:value={raw.text} spellcheck="false"></textarea>
          {#if raw.error}<p class="hint error">{raw.error}</p>{/if}
          <div class="row">
            <button class="primary" onclick={saveRaw}>Save file</button>
            <button onclick={() => (raw = null)}>Close</button>
          </div>
        {:else if draft && selected}
          <div class="editor-head">
            <h3>{draft.name || selected.file_name}</h3>
            <span class="hint">
              file: <code>{selected.file_name}</code> — the identity. Renaming below changes the
              display name only.
            </span>
          </div>

          <div class="grid">
            <label>
              <span>Name</span>
              <input
                bind:value={draft.name}
                oninput={() => commitAction()}
                onblur={() => commitAction(true)}
              />
            </label>

            <label>
              <span>Description</span>
              <input
                value={draft.description ?? ""}
                oninput={(event) => {
                  draft!.description = event.currentTarget.value || null;
                  commitAction();
                }}
                onblur={() => commitAction(true)}
              />
            </label>

            <label>
              <span>Input source</span>
              <select
                value={draft.input_source}
                onchange={(event) => {
                  draft!.input_source = event.currentTarget.value as InputSource;
                  commitAction(true);
                }}
              >
                {#each sources as source}<option value={source}>{source}</option>{/each}
              </select>
            </label>

            <label>
              <span>Direct Hotkey</span>
              <HotkeyInput value={draft.hotkey ?? null} clearable onchange={setActionHotkey} />
              {#if actionHotkeyError(selected)}
                <p class="hint error">{actionHotkeyError(selected)}</p>
              {/if}
            </label>
          </div>

          <label>
            <span>System prompt</span>
            <textarea
              class="system"
              bind:value={draft.prompt.system}
              oninput={() => commitAction()}
              onblur={() => commitAction(true)}
            ></textarea>
          </label>

          <label>
            <span>User template</span>
            <input
              value={draft.prompt.user ?? ""}
              placeholder="{'{{input}}'}"
              oninput={(event) => {
                draft!.prompt.user = event.currentTarget.value || null;
                commitAction();
              }}
              onblur={() => commitAction(true)}
            />
            <p class="hint">
              <code>{"{{input}}"}</code> is replaced by the Selection or the typed input. Empty means
              just the input.
            </p>
          </label>

          <h4>Model overrides</h4>
          <div class="grid">
            <label>
              <span>Model</span>
              <input
                value={draft.model.model ?? ""}
                placeholder="inherit"
                oninput={(event) => {
                  draft!.model.model = event.currentTarget.value || null;
                  commitAction();
                }}
                onblur={() => commitAction(true)}
              />
            </label>

            <label>
              <span>Thinking</span>
              <select
                value={thinkingChoice(draft.model.thinking)}
                onchange={(event) => setThinking(event.currentTarget.value)}
              >
                <option value="inherit">inherit</option>
                <option value="on">on</option>
                <option value="off">off</option>
              </select>
            </label>

            <label>
              <span>Temperature</span>
              <input
                type="number"
                step="0.1"
                min="0"
                max="2"
                value={draft.model.temperature ?? ""}
                placeholder="inherit"
                oninput={(event) => {
                  draft!.model.temperature = numberOrNull(event.currentTarget.value);
                  commitAction();
                }}
                onblur={() => commitAction(true)}
              />
            </label>
          </div>

          <div class="row footer-row">
            <span class="hint">Every change is written to disk as you type.</span>
            <button class="danger" onclick={() => removeAction(selected)}>Delete Action</button>
          </div>
        {:else}
          <p class="hint empty">Select an Action to edit it, or create a new one.</p>
        {/if}
      </div>
    </div>
  </section>
</main>

<style>
  main {
    max-width: 900px;
    margin: 0 auto;
    padding: 24px 28px 48px;
  }

  h1 {
    font-size: 20px;
    margin: 0 0 20px;
  }

  h2 {
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    margin: 0 0 14px;
  }

  h3 {
    font-size: 15px;
    margin: 0;
  }

  h4 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
    margin: 6px 0 0;
  }

  section {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    padding: 18px;
    margin-bottom: 18px;
  }

  .section-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  label {
    display: block;
    margin-bottom: 14px;
  }

  label > span {
    display: block;
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 5px;
  }

  label.checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  label.checkbox input {
    width: auto;
  }

  label.checkbox span {
    margin: 0;
    font-size: 14px;
    color: var(--text);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 0 16px;
    align-items: end;
  }

  .banner {
    border: 1px solid var(--accent-dim);
    border-left-width: 3px;
    border-radius: 8px;
    padding: 10px 14px;
    margin-bottom: 18px;
    background: var(--bg-raised);
  }

  .banner.bad {
    border-color: var(--danger);
  }

  .banner ul {
    margin: 6px 0;
    padding-left: 20px;
  }

  .actions {
    display: grid;
    grid-template-columns: minmax(200px, 260px) 1fr;
    gap: 16px;
    align-items: start;
  }

  .action-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 460px;
    overflow-y: auto;
  }

  .action-row {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    text-align: left;
    background: none;
    border-color: transparent;
  }

  .action-row.selected {
    background: var(--bg-input);
    border-color: var(--border-strong);
  }

  .action-row.broken {
    border-color: var(--danger);
  }

  .action-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action-file {
    font-size: 11px;
    color: var(--text-faint);
    width: 100%;
  }

  .editor {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    background: var(--bg);
    min-height: 240px;
  }

  .editor-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 14px;
  }

  textarea.system {
    min-height: 150px;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 13px;
  }

  textarea.raw {
    min-height: 320px;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 13px;
  }

  .footer-row {
    justify-content: space-between;
    margin-top: 8px;
  }

  .empty {
    padding: 20px 0;
  }

  .ok {
    color: var(--ok);
  }

  code {
    font-family: ui-monospace, Consolas, monospace;
    font-size: 12px;
    color: var(--text-dim);
  }
</style>
