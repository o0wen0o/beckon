<script lang="ts">
  // A file that fails to parse is never dropped (ADR-0003) — it is reported and
  // stays editable as text, which is the only way back from a bad hand-edit.
  import Callout from "../lib/ui/Callout.svelte";
  import { actions } from "./actions.svelte";

  const raw = $derived(actions.raw);
  const parseError = $derived(
    actions.snapshot.errors.find((error) => error.file_name === raw?.file)?.message,
  );
</script>

{#if raw}
  {#if parseError}
    <Callout tone="danger"><p>{parseError}</p></Callout>
  {/if}

  <textarea class="raw" bind:value={actions.raw!.text} spellcheck="false"></textarea>

  {#if raw.error}
    <p class="hint error">{raw.error}</p>
  {/if}

  <div class="row">
    <button class="primary" onclick={() => actions.saveRaw()}>Save file</button>
    <span class="hint">It reloads the moment it parses.</span>
  </div>
{/if}

<style>
  .raw {
    min-height: 220px;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>
