<script lang="ts">
  import {
    Check,
    Warning,
  } from "../../lib/icons";
  import {
    deleteApiKey,
    describeError,
    getKeyStatus,
    setApiKey,
    testConnection,
  } from "../../lib/ipc";
  import Callout from "../../lib/ui/Callout.svelte";
  import Field from "../../lib/ui/Field.svelte";
  import { describeFailure } from "../failures";
  import { settings } from "../store.svelte";

  const config = $derived(settings.config);

  async function saveKey() {
    const key = settings.keyDraft.trim();
    if (key === "") return;
    try {
      settings.keyStatus = await setApiKey(key);
      settings.keyDraft = "";
      settings.keyMessage = "Saved.";
      void settings.refreshModels();
    } catch (error) {
      settings.keyMessage = describeError(error).message;
    }
  }

  async function removeKey() {
    try {
      settings.keyStatus = await deleteApiKey();
      settings.keyMessage = "Removed.";
      void settings.refreshModels();
    } catch (error) {
      settings.keyMessage = describeError(error).message;
    }
  }

  async function runTest() {
    settings.test = { state: "running" };
    try {
      await testConnection();
      settings.test = { state: "ok", message: "The key and base URL work." };
    } catch (error) {
      settings.test = { state: "failed", message: describeFailure(describeError(error)) };
    }
    settings.keyStatus = await getKeyStatus();
  }
</script>

<h1>Connection</h1>

{#if settings.firstRun}
  <Callout>
    <p>
      <strong>Welcome.</strong> Beckon needs a DeepSeek API key before it can do anything.
    </p>
    <p>The key goes into the Windows Credential Manager, never into a file.</p>
  </Callout>
{/if}

<Field label="API key">
  {#snippet control({ id, describedBy })}
    <div class="row">
      <input
        {id}
        aria-describedby={describedBy}
        type="password"
        bind:value={settings.keyDraft}
        placeholder={settings.keyStatus?.kind === "present"
          ? `stored — ends in ${settings.keyStatus.last4}`
          : "sk-…"}
        autocomplete="off"
        onkeydown={(event) => event.key === "Enter" && saveKey()}
      />
      <button class="primary" disabled={settings.keyDraft.trim() === ""} onclick={saveKey}>
        Save
      </button>
      {#if settings.keyStatus?.kind === "present"}
        <button class="danger" onclick={removeKey}>Remove</button>
      {/if}
    </div>
  {/snippet}
</Field>

<!-- The three key states stay three distinguishable outcomes all the way to
     the UI (ADR-0005): stored, never stored, and unreadable. -->
{#if settings.keyStatus?.kind === "present"}
  <p class="key-state ok"><Check size={13} /> Stored — ends in <code>{settings.keyStatus.last4}</code></p>
{:else if settings.keyStatus?.kind === "no-credential"}
  <p class="key-state">No key stored yet.</p>
{:else if settings.keyStatus?.kind === "read-error"}
  <p class="key-state error">
    <Warning size={13} />
    The Credential Manager could not be read: {settings.keyStatus.message}. Save the key again to
    recreate the credential.
  </p>
{/if}

{#if settings.keyMessage}
  <p class="key-state">{settings.keyMessage}</p>
{/if}

{#if config}
  <Field
    label="Base URL"
    hint="Any OpenAI-compatible endpoint. Requests go to /v1/chat/completions."
  >
    {#snippet control({ id, describedBy })}
      <input
        {id}
        aria-describedby={describedBy}
        value={config.api.base_url}
        spellcheck="false"
        oninput={(event) => {
          const next = event.currentTarget.value;
          settings.editConfig((draft) => (draft.api.base_url = next));
        }}
      />
    {/snippet}
  </Field>
{/if}

<div class="row test">
  <button onclick={runTest} disabled={settings.test.state === "running"}>
    {settings.test.state === "running" ? "Testing…" : "Test connection"}
  </button>
  {#if settings.test.message}
    <span
      class="hint"
      class:error={settings.test.state === "failed"}
      class:ok={settings.test.state === "ok"}
    >
      {settings.test.message}
    </span>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .test {
    margin-top: var(--space-2);
  }

  .key-state {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin: calc(var(--space-4) * -1) 0 var(--space-4);
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .key-state.ok {
    color: var(--ok);
  }

  .key-state.error {
    color: var(--danger);
    align-items: flex-start;
  }
</style>
