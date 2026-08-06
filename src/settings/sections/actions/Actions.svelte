<script lang="ts">
  // One section, two views over the same list: every Action in the pane, and
  // the one being edited. Authoring lives here rather than in the Launcher —
  // the Launcher is summoned by a hotkey to pick something and get out of the
  // way, and a form is the opposite of that.
  //
  // There is no Save button and there must never be one (ADR-0003); the write
  // is scheduled by the store as the fields change.
  import {
    ArrowLeft,
    Auto,
    ChevronRight,
    Plus,
    Prompt,
    TextSelect,
    Warning,
  } from "../../../lib/icons";
  import type { InputSource } from "../../../lib/types";
  import { actionStore } from "../../actions.svelte";
  import ActionEditor from "./ActionEditor.svelte";
  import RawFileEditor from "./RawFileEditor.svelte";

  let form = $state<HTMLElement | null>(null);

  // The store tests focus against this element to decide whether adopting an
  // incoming snapshot would fight the user.
  $effect(() => {
    actionStore.form = form;
    return () => {
      actionStore.form = null;
    };
  });

  const snapshot = $derived(actionStore.snapshot);
  const editing = $derived(actionStore.editing);

  const SOURCE_ICON = { selection: TextSelect, prompt: Prompt, auto: Auto };

  /** Title case for display; the value itself stays the CONTEXT.md term. */
  function sourceLabel(source: InputSource) {
    return source.charAt(0).toUpperCase() + source.slice(1);
  }

  /** The display name, or the bare file name for one that does not parse. */
  const title = $derived(
    editing === null
      ? ""
      : editing.kind === "raw"
        ? editing.file
        : actionStore.draft?.name || editing.file,
  );
</script>

{#if editing}
  <header class="editor-head">
    <button class="quiet back" aria-label="Back to Actions" onclick={() => actionStore.close()}>
      <ArrowLeft size={15} />
    </button>
    <div class="editor-title">
      <span class="name">{title}</span>
      <!-- The filename is the identity (ADR-0003): renaming never moves it. In
           the raw editor the heading *is* the filename, so repeating it here
           would print the same string twice, one line apart. -->
      <span class="file">
        {editing.kind === "raw" ? "does not parse — edited as text" : editing.file}
      </span>
    </div>
  </header>

  <!-- Focus leaving the form is the last chance to write whatever was typed;
       the store then adopts the snapshot it held back while it was held. -->
  <div bind:this={form} onfocusout={() => actionStore.flush()}>
    {#if editing.kind === "raw"}
      <RawFileEditor />
    {:else if actionStore.selected}
      <ActionEditor action={actionStore.selected} />
    {:else}
      <p class="hint">That Action is gone.</p>
    {/if}
  </div>
{:else}
  <header class="list-head">
    <h1>Actions</h1>
    <button class="primary" onclick={() => actionStore.create()}>
      <Plus size={14} /> New Action
    </button>
  </header>

  <ul class="list">
    {#each snapshot.actions as action (action.id)}
      {@const conflict = snapshot.hotkey_errors[action.id]}
      {@const SourceIcon = SOURCE_ICON[action.input_source]}
      <li>
        <button class="row" onclick={() => actionStore.open(action.file_name)}>
          <span class="rail"></span>
          <span class="text">
            <span class="name">{action.name || action.file_name}</span>
            {#if action.description}
              <span class="description">{action.description}</span>
            {/if}
          </span>
          <!-- Two fixed slots, not a shrink-to-fit row: with the hotkey chip
               optional, an ordinary flex row parks each Input Source pill at a
               different x and the column reads as a ragged edge. -->
          <span class="meta">
            <span class="source" title="Input Source: {sourceLabel(action.input_source)}">
              <SourceIcon size={13} />
              {sourceLabel(action.input_source)}
            </span>
            <span class="hotkey-slot">
              {#if conflict}
                <span class="badge bad" title={conflict}><Warning size={12} /> {action.hotkey}</span>
              {:else if action.hotkey}
                <kbd>{action.hotkey}</kbd>
              {/if}
            </span>
          </span>
          <!-- The rows are the only way into the editor, and a name over a
               description reads as a list of facts unless something says it
               opens. -->
          <span class="go"><ChevronRight size={16} /></span>
        </button>
      </li>
    {/each}

    <!-- A file that does not parse is reported, never dropped (ADR-0003), and
         the way back is the raw editor this row opens. -->
    {#each snapshot.errors as error (error.file_name)}
      <li>
        <button class="row broken" onclick={() => actionStore.openRaw(error.file_name)}>
          <span class="rail"></span>
          <span class="text">
            <span class="name mono">{error.file_name}</span>
            <!-- The parse error itself, not a tooltip holding it: this row is
                 the only report that the file exists, and a `title` is invisible
                 to everything except a resting mouse. -->
            <span class="description bad">{error.message}</span>
          </span>
          <span class="meta">
            <span class="badge bad"><Warning size={12} /> Repair</span>
          </span>
          <span class="go"><ChevronRight size={16} /></span>
        </button>
      </li>
    {/each}

    {#if snapshot.actions.length === 0 && snapshot.errors.length === 0}
      <li class="empty">
        <p>No Actions yet. One Action is one prompt, stored as its own file.</p>
      </li>
    {/if}
  </ul>
{/if}

<style>
  .list-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    text-align: left;
    border: none;
    background: none;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    transition: background-color var(--dur-fast) var(--ease-out);
  }

  .row:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: transparent;
  }

  /* Equal rows: a description is optional, and without a floor the list's
     rhythm changes every time one is missing. The Launcher's list reads the
     same token, so the two lists of Actions stay one list. */
  .row {
    min-height: var(--row-h);
  }

  .go {
    flex: none;
    display: flex;
    color: var(--text-faint);
    transition:
      color var(--dur-fast) var(--ease-out),
      transform var(--dur-fast) var(--ease-out);
  }

  .row:hover .go,
  .row:focus-visible .go {
    color: var(--accent);
    transform: translateX(2px);
  }

  /* The brand rail. Decorative: the row's tint is what says "hovered". */
  .rail {
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: var(--radius-pill);
    background: linear-gradient(var(--brand-from), var(--brand-to));
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease-out);
  }

  .row:hover .rail,
  .row:focus-visible .rail {
    opacity: 1;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .name.mono {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .description {
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .description.bad {
    color: var(--danger);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* Reserved whether or not this Action has a hotkey, so the pills line up. */
  .hotkey-slot {
    display: flex;
    justify-content: flex-end;
    flex: none;
    min-width: 96px;
  }

  .source {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    padding: 1px var(--space-2);
  }

  .empty {
    padding: var(--space-6) 0;
    color: var(--text-dim);
  }

  .empty p {
    margin: 0;
  }

  .editor-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-5);
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
    font-size: var(--text-xl);
    font-weight: var(--weight-semibold);
  }

  .editor-title .file {
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
