<script lang="ts">
  // The Launcher's default view: a query box, the ranked list, and the window's
  // own controls along the bottom.
  //
  // A listbox, not a menu: the query box keeps focus and the list is driven
  // from the keyboard by the window (↑↓/Enter/Esc), so the rows here are a
  // mouse convenience. The two elements the window has to reach — the input and
  // the scrolling list — are exposed as functions rather than bound outward.
  import { flip } from "svelte/animate";
  import { highlight } from "../lib/fuzzy";
  import { Auto, BrandMark, Prompt, Search, Sliders, TextSelect, Warning } from "../lib/icons";
  import type { Action, InputSource } from "../lib/types";
  import { actions } from "./actions.svelte";

  let {
    matches,
    query = $bindable(),
    selected = $bindable(),
    selectionChars,
    onrun,
    onsettings,
  }: {
    matches: Action[];
    query: string;
    selected: number;
    selectionChars: number;
    onrun: (action: Action) => void;
    /** Everything editable — new, edit, repair — is over there (ADR-0003). */
    onsettings: () => void;
  } = $props();

  let input = $state<HTMLInputElement | null>(null);
  let list = $state<HTMLUListElement | null>(null);

  const snapshot = $derived(actions.snapshot);

  const SOURCE_ICON = { selection: TextSelect, prompt: Prompt, auto: Auto };

  export function focusQuery() {
    input?.focus();
    input?.select();
  }

  export function scrollSelectedIntoView() {
    // Wait for the class to land on the new row before scrolling to it.
    requestAnimationFrame(() => {
      list?.querySelector(".row.selected")?.scrollIntoView({ block: "nearest" });
    });
  }

  /** Title case for display; the value itself stays the CONTEXT.md term. */
  function sourceLabel(source: InputSource) {
    return source.charAt(0).toUpperCase() + source.slice(1);
  }
</script>

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

<ul class="list" bind:this={list} role="listbox" aria-label="Actions">
  {#each matches as action, index (action.id)}
    {@const conflict = snapshot.hotkey_errors[action.id]}
    {@const SourceIcon = SOURCE_ICON[action.input_source]}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <li
      class="row"
      class:selected={index === selected}
      role="option"
      aria-selected={index === selected}
      onmousemove={() => (selected = index)}
      onclick={() => onrun(action)}
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
      </div>
    </li>
  {/each}

  <!-- A file that does not parse is reported, never dropped (ADR-0003). It
       cannot be run, and the raw editor that repairs it is in Settings. -->
  {#each snapshot.errors as error (error.file_name)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <li class="row broken" role="option" aria-selected="false" onclick={onsettings}>
      <span class="rail"></span>
      <div class="text">
        <span class="name mono">{error.file_name}</span>
        <span class="description">does not parse — repair it in Settings</span>
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
        <button class="primary" onclick={onsettings}>
          <Sliders size={14} /> Add one in Settings
        </button>
      {:else}
        <p>Nothing matches <code>{query}</code>.</p>
      {/if}
    </li>
  {/if}
</ul>

<footer>
  <span class="hint status">
    {#if selectionChars > 0}
      {selectionChars} characters selected
    {:else}
      No selection
    {/if}
  </span>
  <button class="quiet" title="Settings (Ctrl+,)" aria-label="Settings" onclick={onsettings}>
    <Sliders size={15} />
  </button>
</footer>

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
</style>
