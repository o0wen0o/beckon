<script lang="ts">
  // The Launcher: the universal entry point to every Action. Keyboard only —
  // it is summoned by a hotkey, so the hands are already on the keys.
  import { onMount } from "svelte";
  import Search from "lucide-svelte/icons/search";
  import TriangleAlert from "lucide-svelte/icons/triangle-alert";
  import {
    getActions,
    hideLauncher,
    onActionsChanged,
    onLauncherOpened,
    pickAction,
    showSettings,
    Subscriptions,
  } from "../lib/ipc";
  import { rank } from "../lib/fuzzy";
  import type { Action, RegistrySnapshot } from "../lib/types";

  let snapshot = $state<RegistrySnapshot>({ actions: [], errors: [], hotkey_errors: {} });
  let query = $state("");
  let selected = $state(0);
  let selectionChars = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let list = $state<HTMLUListElement | null>(null);

  const matches = $derived(
    rank(snapshot.actions, query, (action) => [action.name, action.description ?? ""]),
  );

  // Clamp rather than reset: the list re-ranks on every keystroke.
  $effect(() => {
    if (selected >= matches.length) selected = Math.max(0, matches.length - 1);
  });

  // Focus stays in the query box, so the highlighted row has to be announced
  // through `aria-activedescendant` rather than by moving focus onto it.
  // Indexes, not Action ids: an id comes from a filename and need not be a
  // valid HTML id.
  const activeId = $derived(matches.length > 0 ? `launcher-option-${selected}` : undefined);

  const subscriptions = new Subscriptions();

  onMount(() => {
    void refresh();
    subscriptions
      .add(onActionsChanged((next) => (snapshot = next)))
      .add(
        onLauncherOpened((chars) => {
          // A fresh summon starts from a clean slate.
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

  async function refresh() {
    snapshot = await getActions();
  }

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

  function onKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        void hideLauncher();
        break;
      case "ArrowDown":
        event.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        break;
      case "Enter":
        event.preventDefault();
        run(matches[selected]);
        break;
      case "Tab":
        // Nothing else is focusable; keep focus in the query box.
        event.preventDefault();
        move(event.shiftKey ? -1 : 1);
        break;
      case ",":
        if (event.ctrlKey) {
          event.preventDefault();
          void hideLauncher();
          void showSettings();
        }
        break;
    }
  }

  function hotkeyError(action: Action) {
    return snapshot.hotkey_errors[action.id];
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  <div class="query-row">
    <Search class="query-icon" size={17} aria-hidden="true" />
    <input
      bind:this={input}
      bind:value={query}
      class="query"
      placeholder="Search Actions…"
      aria-label="Search Actions"
      role="combobox"
      aria-expanded="true"
      aria-controls="launcher-list"
      aria-activedescendant={activeId}
      aria-autocomplete="list"
      spellcheck="false"
      autocomplete="off"
    />
  </div>

  <!-- A listbox: the query box keeps focus, the list is driven from the keyboard. -->
  <ul class="list" id="launcher-list" bind:this={list} role="listbox" aria-label="Actions">
    {#each matches as action, index (action.id)}
      <!-- Keyboard interaction is handled window-wide (↑↓/Enter/Esc) so focus
           can stay in the query box; the rows are a mouse convenience. -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <li
        class="row"
        class:selected={index === selected}
        id="launcher-option-{index}"
        role="option"
        aria-selected={index === selected}
        onmousemove={() => (selected = index)}
        onclick={() => run(action)}
      >
        <div class="text">
          <span class="name">{action.name}</span>
          {#if action.description}
            <span class="description">{action.description}</span>
          {/if}
        </div>
        <div class="meta">
          <span class="source">{action.input_source}</span>
          {#if action.hotkey}
            <kbd class:bad={hotkeyError(action)}>{action.hotkey}</kbd>
          {/if}
        </div>
      </li>
    {/each}

    {#if matches.length === 0}
      <li class="empty">
        {#if snapshot.actions.length === 0}
          <p>No Actions yet.</p>
          <button class="primary" onclick={() => showSettings()}>Open Settings</button>
        {:else}
          <p>Nothing matches “{query}”.</p>
        {/if}
      </li>
    {/if}
  </ul>

  <footer>
    <span class="hint tabular">
      {#if selectionChars > 0}
        {selectionChars} characters selected
      {:else}
        No selection
      {/if}
    </span>
    <span class="hint keys">
      <kbd>↑↓</kbd> navigate <kbd>Enter</kbd> run <kbd>Esc</kbd> close
    </span>
    {#if snapshot.errors.length > 0}
      <button class="badge bad broken" onclick={() => showSettings()}>
        <TriangleAlert size={12} aria-hidden="true" />
        {snapshot.errors.length} broken file{snapshot.errors.length === 1 ? "" : "s"}
      </button>
    {/if}
  </footer>
</div>

<style>
  /* The query line is the window's masthead: one row, no card, no fill — the
     hairline under it is the only thing separating it from the results. */
  .query-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }

  .query-row :global(.query-icon) {
    flex: none;
    color: var(--text-faint);
  }

  .query {
    border: none;
    border-radius: 0;
    background: none;
    font-size: 16px;
    letter-spacing: -0.006em;
    padding: 15px 0;
  }

  .query:focus,
  .query:hover {
    outline: none;
    border: none;
    background: none;
  }

  .list {
    flex: 1;
    margin: 0;
    padding: var(--space-2);
    list-style: none;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 7px 10px;
    border-radius: var(--radius);
    color: var(--text-dim);
    cursor: default;
  }

  /* Two signals, not one: the tinted row, and the name coming up to full
     contrast against the dimmed rows around it. */
  .row.selected {
    background: var(--accent-soft);
    color: var(--text);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    color: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row.selected .name {
    font-weight: 500;
  }

  .description {
    font-size: 12px;
    color: var(--text-faint);
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
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  kbd.bad {
    color: var(--danger);
    border-color: var(--danger);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-3);
    color: var(--text-dim);
  }

  .empty p {
    margin: 0;
  }

  footer {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
  }

  footer .keys {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .broken {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
  }
</style>
