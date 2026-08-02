<script lang="ts">
  // The Launcher's second view: one Action, as a form or — when the file does
  // not parse — as text. There is no Save button and there must never be one
  // (ADR-0003); the write is scheduled by the store as the fields change.
  import { ArrowLeft } from "../lib/icons";
  import StatusBar from "../lib/ui/StatusBar.svelte";
  import ActionEditor from "./ActionEditor.svelte";
  import RawFileEditor from "./RawFileEditor.svelte";
  import { actions, type Editing } from "./actions.svelte";

  let { editing }: { editing: Editing } = $props();

  let form = $state<HTMLElement | null>(null);

  // The store tests focus against this element to decide whether adopting an
  // incoming snapshot would fight the user.
  $effect(() => {
    actions.form = form;
    return () => {
      actions.form = null;
    };
  });

  /** The display name, or the bare file name for one that does not parse. */
  const title = $derived(
    editing.kind === "raw" ? editing.file : actions.draft?.name || editing.file,
  );
</script>

<header class="editor-head">
  <button class="quiet back" onclick={() => actions.close()}>
    <ArrowLeft size={15} />
  </button>
  <div class="editor-title">
    <span class="name">{title}</span>
    <span class="file">
      {editing.file}{editing.kind === "raw" ? " — does not parse, edited as text" : ""}
    </span>
  </div>
</header>

<!-- Focus leaving the form is the last chance to write whatever was typed;
     the store then adopts the snapshot it held back while it was held. -->
<div class="editor" bind:this={form} onfocusout={() => actions.flush()}>
  {#if editing.kind === "raw"}
    <RawFileEditor />
  {:else if actions.selected}
    <ActionEditor action={actions.selected} />
  {:else}
    <p class="hint">That Action is gone.</p>
  {/if}
</div>

<StatusBar busy={actions.slot.busy} error={actions.slot.error} />

<style>
  .editor-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: var(--surface-radius) var(--surface-radius) 0 0;
  }

  .back {
    flex: none;
  }

  .editor-title {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .editor-title .name {
    font-family: var(--font-display);
    font-weight: var(--weight-semibold);
  }

  /* The filename is the identity (ADR-0003): renaming above never moves it. */
  .editor-title .file {
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4);
  }
</style>
