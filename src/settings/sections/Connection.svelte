<script lang="ts">
  import { Check, Warning } from "../../lib/icons";
  import {
    deleteApiKey,
    describeError,
    getKeyStatus,
    openApiKeyPage,
    setApiKey,
    testConnection,
  } from "../../lib/ipc";
  import Callout from "../../lib/ui/Callout.svelte";
  import Field from "../../lib/ui/Field.svelte";
  import { describeFailure } from "../../lib/failures";
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
    <p>
      <button class="quiet link" onclick={() => openApiKeyPage()}>
        Get a key from platform.deepseek.com
      </button>
    </p>
  </Callout>
{/if}

<!-- The state line lives inside the field rather than after it: it is what the
     field currently holds, so it reads on the field's own rhythm. It used to sit
     outside with a negative top margin cancelling the field's spacing. -->
<Field label="API key">
  {#snippet control({ id, describedBy })}
    <div class="key">
      <div class="row">
        <input
          {id}
          aria-describedby={describedBy}
          type="password"
          bind:value={settings.keyDraft}
          placeholder="sk-…"
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

      <!-- The three key states stay three distinguishable outcomes all the way
           to the UI (ADR-0005): stored, never stored, and unreadable. -->
      {#if settings.keyStatus?.kind === "present"}
        <p class="key-state ok">
          <Check size={13} /> Stored — ends in <code>{settings.keyStatus.last4}</code>
        </p>
      {:else if settings.keyStatus?.kind === "no-credential"}
        <p class="key-state">No key stored yet.</p>
      {:else if settings.keyStatus?.kind === "read-error"}
        <p class="key-state error">
          <Warning size={13} />
          The Credential Manager could not be read: {settings.keyStatus.message}. Save the key again
          to recreate the credential.
        </p>
      {/if}

      {#if settings.keyMessage}
        <p class="key-state">{settings.keyMessage}</p>
      {/if}
    </div>
  {/snippet}
</Field>

{#if config}
  <Field
    label="Base URL"
    hint="Any OpenAI-compatible endpoint. Requests go to /v1/chat/completions."
  >
    {#snippet control({ id, describedBy })}
      <input
        {id}
        class="url"
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

  /* The key, its buttons and its state are one thing; capped so Save and Remove
     stay beside the field instead of at the far edge of the pane. */
  .key {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    max-width: 620px;
  }

  .url {
    max-width: var(--input-max);
  }

  .test {
    margin-top: var(--space-2);
  }

  .key-state {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin: 0;
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  /* Inside a Callout, so it has to read as prose that can be clicked rather
     than as a button competing with the Save beside it. */
  .link {
    padding: 0;
    font-family: inherit;
    color: var(--accent);
    text-decoration: underline;
  }

  .link:hover:not(:disabled) {
    background: none;
    color: var(--accent-strong);
  }

  .key-state.ok {
    color: var(--ok);
  }

  .key-state.error {
    color: var(--danger);
    align-items: flex-start;
  }
</style>
