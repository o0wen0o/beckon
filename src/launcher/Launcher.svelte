<script lang="ts">
  // The Launcher: the universal entry point to every Action, and — since the
  // list of Actions is here — where one is added, edited and deleted too.
  // Picking is keyboard only: it is summoned by a hotkey, so the hands are
  // already on the keys. Editing is a second view over the same window
  // (ADR-0007: the window is created once and reused), and while it is open
  // Rust suspends the Launcher's hide-on-blur, because a form that vanishes
  // when you click another app is not a form.
  import { onMount } from "svelte";
  import { flip } from "svelte/animate";
  import {
    hideLauncher,
    onActionsChanged,
    onConfigChanged,
    onLauncherOpened,
    pickAction,
    showSettings,
    Subscriptions,
  } from "../lib/ipc";
  import { highlight, rank } from "../lib/fuzzy";
  import {
    ArrowLeft,
    Auto,
    BrandMark,
    Pencil,
    Plus,
    Prompt,
    Search,
    Sliders,
    TextSelect,
    Warning,
  } from "../lib/icons";
  import ConfirmDialog from "../lib/ui/ConfirmDialog.svelte";
  import StatusBar from "../lib/ui/StatusBar.svelte";
  import type { Action, InputSource } from "../lib/types";
  import ActionEditor from "./ActionEditor.svelte";
  import RawFileEditor from "./RawFileEditor.svelte";
  import { actions } from "./actions.svelte";

  let query = $state("");
  let selected = $state(0);
  let selectionChars = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let list = $state<HTMLUListElement | null>(null);
  let form = $state<HTMLElement | null>(null);

  const snapshot = $derived(actions.snapshot);
  const editing = $derived(actions.editing);
  const pendingDelete = $derived(actions.pendingDelete);

  const matches = $derived(
    rank(snapshot.actions, query, (action) => [action.name, action.description ?? ""]),
  );

  // Clamp rather than reset: the list re-ranks on every keystroke.
  $effect(() => {
    if (selected >= matches.length) selected = Math.max(0, matches.length - 1);
  });

  $effect(() => {
    actions.form = form;
  });

  const subscriptions = new Subscriptions();

  onMount(() => {
    void actions.refresh();
    subscriptions
      .add(onActionsChanged((next) => actions.adoptActions(next)))
      .add(onConfigChanged((next) => actions.adoptConfig(next)))
      .add(
        onLauncherOpened((chars) => {
          // A fresh summon starts from a clean slate — including the editor,
          // which the previous visit may have left open.
          actions.close();
          selectionChars = chars;
          query = "";
          selected = 0;
          input?.focus();
          input?.select();
        }),
      );
    input?.focus();
    return () => void subscriptions.dispose();
  });

  function move(delta: number) {
    if (matches.length === 0) return;
    selected = (selected + delta + matches.length) % matches.length;
    scrollSelectedIntoView();
  }

  function scrollSelectedIntoView() {
    // Wait for the class to land on the new row before scrolling to it.
    requestAnimationFrame(() => {
      list?.querySelector(".row.selected")?.scrollIntoView({ block: "nearest" });
    });
  }

  function run(action: Action | undefined) {
    if (!action) return;
    void pickAction(action.id);
  }

  function edit(action: Action | undefined) {
    if (!action) return;
    void actions.open(action.file_name);
  }

  function onKeydown(event: KeyboardEvent) {
    // The editor is a form: every key belongs to whatever has focus, and only
    // Esc is still the window's.
    if (editing) {
      if (event.key === "Escape") {
        event.preventDefault();
        actions.close();
      }
      return;
    }

    switch (event.key) {
      case "Escape":
        event.preventDefault();
        void hideLauncher();
        return;
      case "ArrowDown":
        event.preventDefault();
        move(1);
        return;
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        return;
      case "Enter":
        event.preventDefault();
        run(matches[selected]);
        return;
      case "Tab":
        // Focus never leaves the query box, so Tab is just another arrow.
        event.preventDefault();
        move(event.shiftKey ? -1 : 1);
        return;
      case "e":
      case "E":
        if (event.ctrlKey) {
          event.preventDefault();
          edit(matches[selected]);
        }
        return;
      case "n":
      case "N":
        if (event.ctrlKey) {
          event.preventDefault();
          void actions.create();
        }
        return;
      case ",":
        if (event.ctrlKey) {
          event.preventDefault();
          void hideLauncher();
          void showSettings();
        }
    }
  }

  function hotkeyError(action: Action) {
    return snapshot.hotkey_errors[action.id];
  }

  const SOURCE_ICON = { selection: TextSelect, prompt: Prompt, auto: Auto };

  /** Title case for display; the value itself stays the CONTEXT.md term. */
  function sourceLabel(source: InputSource) {
    return source.charAt(0).toUpperCase() + source.slice(1);
  }

  /** The heading of the editor view: the display name, or the bare file name
   *  for one that does not parse. */
  const editorTitle = $derived.by(() => {
    if (!editing) return "";
    if (editing.kind === "raw") return editing.file;
    return actions.draft?.name || editing.file;
  });
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  {#if editing}
    <header class="editor-head">
      <button class="quiet back" onclick={() => actions.close()}>
        <ArrowLeft size={15} /> Actions
      </button>
      <div class="editor-title">
        <span class="name">{editorTitle}</span>
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
  {:else}
    <div class="search">
      <span class="search-icon"><Search size={18} /></span>
      <input
        bind:this={input}
        bind:value={query}
        class="query"
        placeholder="Search Actions…"
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <!-- A listbox: the query box keeps focus, the list is driven from the keyboard. -->
    <ul class="list" bind:this={list} role="listbox" aria-label="Actions">
      {#each matches as action, index (action.id)}
        {@const conflict = hotkeyError(action)}
        {@const SourceIcon = SOURCE_ICON[action.input_source]}
        <!-- Keyboard interaction is handled window-wide (↑↓/Enter/Esc) so focus
             can stay in the query box; the rows are a mouse convenience. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          class="row"
          class:selected={index === selected}
          role="option"
          aria-selected={index === selected}
          onmousemove={() => (selected = index)}
          onclick={() => run(action)}
          animate:flip={{ duration: 180 }}
        >
          <span class="rail"></span>
          <div class="text">
            <span class="name">
              <!-- Keyed by position, not content: "aXaX" matched by "aa" yields
                   two identical runs, and a content key would collide. -->
              {#each highlight(action.name, query) as run, at (at)}
                <span class:hit={run.hit}>{run.text}</span>
              {/each}
            </span>
            {#if action.description}
              <span class="description">{action.description}</span>
            {/if}
          </div>
          <div class="meta">
            <span class="source" title="Input Source: {sourceLabel(action.input_source)}">
              <SourceIcon size={13} />
              {sourceLabel(action.input_source)}
            </span>
            {#if conflict}
              <span class="badge bad" title={conflict}><Warning size={12} /> {action.hotkey}</span>
            {:else if action.hotkey}
              <kbd>{action.hotkey}</kbd>
            {/if}
            <!-- Editing is a different verb from running, so it gets its own
                 target rather than a modifier on the row's click. -->
            <button
              class="quiet edit"
              aria-label="Edit {action.name || action.file_name}"
              title="Edit (Ctrl+E)"
              onclick={(event) => {
                event.stopPropagation();
                edit(action);
              }}
            >
              <Pencil size={14} />
            </button>
          </div>
        </li>
      {/each}

      <!-- A file that does not parse is reported, never dropped (ADR-0003), and
           the way back is the raw editor this row opens. -->
      {#each snapshot.errors as error (error.file_name)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          class="row broken"
          role="option"
          aria-selected="false"
          onclick={() => actions.openRaw(error.file_name)}
        >
          <span class="rail"></span>
          <div class="text">
            <span class="name mono">{error.file_name}</span>
            <span class="description">does not parse — click to repair</span>
          </div>
          <div class="meta">
            <span class="badge bad" title={error.message}><Warning size={12} /></span>
          </div>
        </li>
      {/each}

      {#if matches.length === 0 && snapshot.errors.length === 0}
        <li class="empty">
          {#if snapshot.actions.length === 0}
            <BrandMark size={28} />
            <p>No Actions yet.</p>
            <button class="primary" onclick={() => actions.create()}>
              <Plus size={14} /> New Action
            </button>
          {:else}
            <p>Nothing matches <code>{query}</code>.</p>
          {/if}
        </li>
      {/if}
    </ul>

    <footer>
      <span class="hint status">
        {#if actions.slot.error}
          <span class="error">{actions.slot.error}</span>
        {:else if selectionChars > 0}
          {selectionChars} characters selected
        {:else}
          No selection
        {/if}
      </span>
      <button class="quiet" title="New Action (Ctrl+N)" onclick={() => actions.create()}>
        <Plus size={14} /> New Action
      </button>
      <button
        class="quiet"
        title="Settings (Ctrl+,)"
        aria-label="Settings"
        onclick={() => showSettings()}
      >
        <Sliders size={15} />
      </button>
    </footer>
  {/if}
</div>

<!-- Hosted here, not in the editor: confirming deletes the Action, which
     unmounts the editor — and a <dialog> removed from the DOM while open never
     calls close(), leaving the whole window inert behind an invisible modal. -->
<ConfirmDialog
  open={pendingDelete !== null}
  title={`Delete “${pendingDelete?.name || pendingDelete?.file_name}”?`}
  confirmLabel="Delete file"
  destructive
  onconfirm={() => pendingDelete && actions.deleteAction(pendingDelete)}
  oncancel={() => (actions.pendingDelete = null)}
>
  {#snippet body()}
    <p>
      The file <code>{pendingDelete?.file_name}</code> is removed from disk. This cannot be undone.
    </p>
  {/snippet}
</ConfirmDialog>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4);
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    border-radius: var(--surface-radius) var(--surface-radius) 0 0;
    /* The field's own focus is drawn here, so the ring can be suppressed on the
       input without leaving the one focusable thing in the window unmarked. */
    box-shadow: inset 0 -1px 0 transparent;
    transition: box-shadow var(--dur-fast) var(--ease-out);
  }

  .search:focus-within {
    box-shadow: inset 0 -1px 0 var(--accent);
  }

  .search-icon {
    display: flex;
    color: var(--text-faint);
    transition: color var(--dur-fast) var(--ease-out);
  }

  .search:focus-within .search-icon {
    color: var(--accent);
  }

  .query {
    border: none;
    background: none;
    border-radius: 0;
    font-size: var(--text-lg);
    padding: var(--space-4) 0;
  }

  .query:focus {
    outline: none;
  }

  .list {
    flex: 1;
    margin: 0;
    padding: var(--space-2);
    list-style: none;
    overflow-y: auto;
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    cursor: default;
    transition: background-color var(--dur-fast) var(--ease-out);
  }

  /* The brand rail. Decorative: the row's tint is what actually says
     "selected", so nothing depends on reading a 2px gradient. */
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

  .row.selected {
    background: var(--bg-hover);
  }

  .row.selected .rail {
    opacity: 1;
  }

  .row.broken:hover {
    background: var(--bg-hover);
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

  /* Never the only signal — the ranking order already carries the match. */
  .name .hit {
    color: var(--accent);
    font-weight: var(--weight-semibold);
  }

  .description {
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
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

  /* Present only where the pointer already is: a pencil on every row would
     compete with the hotkey badge that actually identifies the Action. */
  .edit {
    padding: var(--space-1);
    opacity: 0;
  }

  .row:hover .edit,
  .row.selected .edit,
  .edit:focus-visible {
    opacity: 1;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-3);
    text-align: center;
    color: var(--text-dim);
  }

  .empty p {
    margin: 0;
  }

  footer {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: 0 0 var(--surface-radius) var(--surface-radius);
  }

  /* The Selection count is status; the buttons are the window's own controls,
     so they sit at the far corner. */
  .status {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

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
