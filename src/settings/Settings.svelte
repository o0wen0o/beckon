<script lang="ts">
  // Settings is an *editor of files*, not their owner (ADR-0003): every change
  // commits to disk (debounced), and `config-changed` / `actions-changed` events
  // re-render the form. The only local state is the field currently being typed
  // into — adopting a snapshot mid-keystroke would fight the user.
  import { onMount } from "svelte";
  import Eye from "lucide-svelte/icons/eye";
  import EyeOff from "lucide-svelte/icons/eye-off";
  import FolderOpen from "lucide-svelte/icons/folder-open";
  import Plus from "lucide-svelte/icons/plus";
  import RefreshCw from "lucide-svelte/icons/refresh-cw";
  import Trash2 from "lucide-svelte/icons/trash-2";
  import {
    createAction,
    deleteAction,
    deleteApiKey,
    describeError,
    getActions,
    getConfig,
    getKeyStatus,
    getModels,
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
    ModelCatalog,
    ModelOption,
    RegistrySnapshot,
    Theme,
  } from "../lib/types";
  import Select, { type Option } from "../lib/ui/Select.svelte";
  import HotkeyInput from "./HotkeyInput.svelte";

  const SAVE_DEBOUNCE = 400;

  /** "Use the default" as a dropdown value. Not `""`: an empty string is how a
   * Select spells "nothing chosen", and inheriting is a choice. */
  const INHERIT = "__inherit";

  let config = $state<Config | null>(null);
  let snapshot = $state<RegistrySnapshot>({ actions: [], errors: [], hotkey_errors: {} });
  let keyStatus = $state<KeyStatus | null>(null);
  let models = $state<ModelCatalog | null>(null);
  let modelsLoading = $state(false);
  let startupErrors = $state<string[]>([]);
  let saveError = $state<string | null>(null);

  let keyDraft = $state("");
  let keyVisible = $state(false);
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
    // Not awaited: this one can go to the network, and the rest of the form
    // must not wait on it. The dropdowns render from the current value until
    // the catalog lands.
    void refreshModels();
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
      keyVisible = false;
      keyMessage = "Saved to the Windows Credential Manager.";
      test = { state: "idle" };
      // A key is what the live model list was missing.
      void refreshModels();
    } catch (error) {
      keyMessage = describeError(error).message;
    }
  }

  async function removeKey() {
    try {
      keyStatus = await deleteApiKey();
      keyMessage = "Removed.";
      void refreshModels();
    } catch (error) {
      keyMessage = describeError(error).message;
    }
  }

  // Kinds matter here: a rejected key is not an unreachable API, and neither is
  // a missing credential (ADR-0005). One map for every consumer of a
  // `Failure.kind`, so a new kind cannot reach one banner and miss another.
  const FAILURE_PREFIX: Record<string, string> = {
    auth: "The API rejected this key",
    network: "Could not reach the API",
    "no-credential": "No API key stored",
    "read-error": "The Credential Manager could not be read",
    empty: "The endpoint listed no models",
  };

  async function runTest() {
    test = { state: "running" };
    try {
      await testConnection();
      test = { state: "ok", message: "The key and base URL work." };
    } catch (error) {
      const failure = describeError(error);
      const prefix = FAILURE_PREFIX[failure.kind] ?? "Failed";
      test = { state: "failed", message: `${prefix}: ${failure.message}` };
    }
  }

  // --- models -------------------------------------------------------------

  // Rust decides the option set — the catalog it derives from is the same table
  // the request layer maps `thinking` with, so the dropdown can never offer a
  // model that would then be refused. Nothing here re-derives it.
  async function refreshModels() {
    modelsLoading = true;
    try {
      // Populate from the documented catalog first. The live fetch is
      // deliberately unbounded (no HTTP timeout, by design), and a dropdown
      // holding nothing but its own current value while that is in flight is
      // the regression the fallback exists to prevent. A refresh keeps the
      // list already on screen instead of flashing back to the catalog.
      if (!models) models = await getModels(false);
      models = await getModels(true);
    } catch (error) {
      // The command is infallible by design; if it ever is not, keep whatever
      // list is already on screen rather than emptying the dropdowns.
      saveError = describeError(error).message;
    } finally {
      modelsLoading = false;
    }
  }

  /**
   * The options to render, with `current` guaranteed present. Rust already
   * appends a configured-but-unknown model; this only covers the moment before
   * the catalog has arrived, so a select is never rendered without its own
   * value in it — a select whose value is missing would silently reset it.
   */
  function modelOptions(current: string): ModelOption[] {
    const options = models?.options ?? [];
    if (current === "" || options.some((option) => option.id === current)) return options;
    return [
      { id: current, label: current, description: "", thinking: null, origin: "configured" },
      ...options,
    ];
  }

  /** The same list as a Select's options. The description rides on the row it
   * describes, which is the whole reason this is not a native `<select>`. */
  function modelChoices(current: string): Option[] {
    return modelOptions(current).map((option) => ({
      value: option.id,
      label: option.label,
      description: option.description || undefined,
    }));
  }

  function modelOption(id: string): ModelOption | undefined {
    return modelOptions(id).find((option) => option.id === id);
  }

  /** A model only the config vouches for: say so instead of dropping it. */
  function unknownModelHint(id: string | null): string | null {
    if (!id) return null;
    if (modelOption(id)?.origin !== "configured") return null;
    const missing = models?.live
      ? "not in the endpoint's model list"
      : "not one of the models Beckon knows";
    return `${id} is ${missing}. Kept because your configuration names it.`;
  }

  const modelNotice = $derived.by(() => {
    if (!models || models.live) return null;
    const failure = models.fallback;
    if (!failure) return null;
    const prefix = FAILURE_PREFIX[failure.kind] ?? "The model list could not be fetched";
    return `${prefix} — showing the documented models. ${failure.message}`;
  });

  // Rendered under the default-model select. Derived rather than called from
  // the markup: each call rebuilds the option list.
  const defaultModelHint = $derived(unknownModelHint(config?.defaults.model ?? null));
  const defaultModelInfo = $derived(
    config ? (modelOption(config.defaults.model)?.description ?? "") : "",
  );

  // --- small helpers ------------------------------------------------------

  // Spelling out what each source does on the row itself: "auto" is the one
  // nobody guesses right from the word alone.
  const SOURCE_CHOICES: Option[] = [
    { value: "selection", label: "selection", description: "Send the selected text" },
    { value: "prompt", label: "prompt", description: "Always ask; ignore the Selection" },
    { value: "auto", label: "auto", description: "The Selection if there is one, else ask" },
  ];

  const THEME_CHOICES: Option[] = [
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
    { value: "system", label: "Follow Windows" },
  ];

  const THINKING_CHOICES: Option[] = [
    { value: INHERIT, label: "inherit" },
    { value: "on", label: "on" },
    { value: "off", label: "off" },
  ];

  function thinkingChoice(value: boolean | null): string {
    return value === null ? INHERIT : value ? "on" : "off";
  }

  function setThinking(value: string) {
    if (!draft) return;
    draft.model.thinking = value === INHERIT ? null : value === "on";
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
        <div class="key-field">
          <input
            type={keyVisible ? "text" : "password"}
            bind:value={keyDraft}
            placeholder={keyStatus?.kind === "present"
              ? `stored — ends in ${keyStatus.last4}`
              : "sk-…"}
            autocomplete="off"
            spellcheck="false"
            onkeydown={(event) => event.key === "Enter" && saveKey()}
          />
          <button
            class="icon reveal"
            aria-label={keyVisible ? "Hide the key" : "Show the key"}
            aria-pressed={keyVisible}
            onclick={() => (keyVisible = !keyVisible)}
          >
            {#if keyVisible}
              <EyeOff size={15} aria-hidden="true" />
            {:else}
              <Eye size={15} aria-hidden="true" />
            {/if}
          </button>
        </div>
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
        <p class="hint">
          Any OpenAI-compatible endpoint. Requests go to <code>/v1/chat/completions</code>.
        </p>
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

    <!-- Appearance ---------------------------------------------------- -->
    <section>
      <h2>Appearance</h2>

      <label>
        <span>Theme</span>
        <div class="narrow">
          <Select
            value={config.theme}
            options={THEME_CHOICES}
            onchange={(value) => {
              config!.theme = value as Theme;
              commitConfig(true);
            }}
          />
        </div>
        <p class="hint">
          Applies to the Launcher, the Popover and this window at once. Beckon starts light unless
          you say otherwise — “Follow Windows” is the only setting that reads the system preference.
        </p>
      </label>
    </section>

    <!-- Model defaults ------------------------------------------------ -->
    <section>
      <h2>Model defaults</h2>
      <p class="hint">Any Action can override these in its own <code>[model]</code> table.</p>

      <div class="grid">
        <label>
          <span>Model</span>
          <!-- Controlled, never `bind:` — binding would write back whatever the
               list settled on before the catalog arrived, which is exactly how
               a configured model gets silently replaced. -->
          <Select
            value={config.defaults.model}
            options={modelChoices(config.defaults.model)}
            onchange={(value) => {
              config!.defaults.model = value;
              commitConfig(true);
            }}
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

      {#if defaultModelHint}
        <p class="hint error">{defaultModelHint}</p>
      {:else if defaultModelInfo}
        <p class="hint">{defaultModelInfo}</p>
      {/if}

      <div class="row">
        <button class="with-icon" onclick={refreshModels} disabled={modelsLoading}>
          <RefreshCw class={modelsLoading ? "spinning" : ""} size={14} aria-hidden="true" />
          {modelsLoading ? "Loading models…" : "Refresh models"}
        </button>
        {#if models?.live}
          <span class="hint">Listed by the endpoint at your base URL.</span>
        {:else if modelNotice}
          <span class="hint">{modelNotice}</span>
        {/if}
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
        <button class="with-icon" onclick={addAction}>
          <Plus size={14} aria-hidden="true" /> New Action
        </button>
        <button class="with-icon" onclick={() => revealConfigDir()}>
          <FolderOpen size={14} aria-hidden="true" /> Open folder
        </button>
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
          {@const draftModelHint = unknownModelHint(draft.model.model)}
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
              <Select
                value={draft.input_source}
                options={SOURCE_CHOICES}
                onchange={(value) => {
                  draft!.input_source = value as InputSource;
                  commitAction(true);
                }}
              />
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
              <Select
                value={draft.model.model ?? INHERIT}
                options={[
                  {
                    value: INHERIT,
                    label: `inherit (${config?.defaults.model ?? "default"})`,
                  },
                  ...modelChoices(draft.model.model ?? ""),
                ]}
                onchange={(value) => {
                  draft!.model.model = value === INHERIT ? null : value;
                  commitAction(true);
                }}
              />
              {#if draftModelHint}
                <p class="hint error">{draftModelHint}</p>
              {/if}
            </label>

            <label>
              <span>Thinking</span>
              <Select
                value={thinkingChoice(draft.model.thinking)}
                options={THINKING_CHOICES}
                onchange={setThinking}
              />
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
            <button class="danger with-icon" onclick={() => removeAction(selected)}>
              <Trash2 size={14} aria-hidden="true" /> Delete Action
            </button>
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
    max-width: 880px;
    margin: 0 auto;
    padding: var(--space-7) var(--space-6) var(--space-7);
  }

  h1 {
    font-size: 24px;
    letter-spacing: -0.02em;
    margin: 0 0 var(--space-6);
  }

  /* Section titles are labels, not headlines: small, spaced, dimmed. The size
     hierarchy is carried by whitespace between the cards instead. */
  h2 {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-faint);
    margin: 0 0 var(--space-4);
  }

  h3 {
    font-size: 15px;
    margin: 0;
  }

  h4 {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-faint);
    margin: var(--space-2) 0 0;
  }

  section {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-raised);
    padding: var(--space-5);
    margin-bottom: var(--space-4);
  }

  .section-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  label {
    display: block;
    margin-bottom: var(--space-4);
  }

  label > span {
    display: block;
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: var(--space-1);
  }

  label.checkbox {
    display: flex;
    align-items: center;
    gap: var(--space-2);
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
    gap: var(--space-2);
  }

  /* The reveal toggle sits inside the field's box rather than beside it, so the
     row stays one control wide. */
  .key-field {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .key-field input {
    padding-right: 36px;
  }

  .reveal {
    position: absolute;
    top: 50%;
    right: 3px;
    transform: translateY(-50%);
    color: var(--text-faint);
    background: none;
  }

  .narrow {
    max-width: 220px;
  }

  .with-icon {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .with-icon :global(.spinning) {
    animation: spin 900ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 0 var(--space-4);
    align-items: end;
  }

  /* A tinted strip, not an outlined box: the banner is a notice inside the page
     rather than a fourth kind of card. */
  .banner {
    border-radius: var(--radius);
    border-left: 2px solid var(--accent);
    background: var(--accent-soft);
    padding: var(--space-3) var(--space-4);
    margin-bottom: var(--space-4);
  }

  .banner.bad {
    border-left-color: var(--danger);
    background: var(--danger-soft);
  }

  .banner ul {
    margin: var(--space-2) 0;
    padding-left: 20px;
  }

  .actions {
    display: grid;
    grid-template-columns: minmax(200px, 250px) 1fr;
    gap: var(--space-4);
    align-items: start;
  }

  .action-list {
    list-style: none;
    margin: 0;
    padding: 2px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 460px;
    overflow-y: auto;
  }

  .action-row {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    text-align: left;
    font-weight: 400;
    border-color: transparent;
  }

  .action-row.selected {
    background: var(--accent-soft);
    border-color: transparent;
    color: var(--text);
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
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-faint);
    width: 100%;
  }

  .editor {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    background: var(--bg);
    min-height: 240px;
  }

  .editor-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: var(--space-4);
  }

  textarea.system {
    min-height: 150px;
    font-family: var(--font-mono);
    font-size: 13px;
  }

  textarea.raw {
    min-height: 320px;
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .footer-row {
    justify-content: space-between;
    margin-top: var(--space-2);
  }

  .empty {
    padding: var(--space-5) 0;
  }

  .ok {
    color: var(--ok);
  }
</style>
