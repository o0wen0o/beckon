<script lang="ts">
  // The Launcher: the universal entry point to every Action. Keyboard only —
  // it is summoned by a hotkey, so the hands are already on the keys.
  import { onMount } from "svelte";
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
  <input
    bind:this={input}
    bind:value={query}
    class="query"
    placeholder="Search Actions…"
    spellcheck="false"
    autocomplete="off"
  />

  <!-- A listbox: the query box keeps focus, the list is driven from the keyboard. -->
  <ul class="list" bind:this={list} role="listbox" aria-label="Actions">
    {#each matches as action, index (action.id)}
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
          No Actions yet. <button onclick={() => showSettings()}>Open Settings</button>
        {:else}
          Nothing matches “{query}”.
        {/if}
      </li>
    {/if}
  </ul>

  <footer>
    <span class="hint">
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
      <button class="badge bad" onclick={() => showSettings()}>
        {snapshot.errors.length} broken file{snapshot.errors.length === 1 ? "" : "s"}
      </button>
    {/if}
  </footer>
</div>

<style>
  .query {
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 12px 12px 0 0;
    background: var(--bg-raised);
    font-size: 17px;
    padding: 14px 16px;
  }

  .query:focus {
    outline: none;
  }

  .list {
    flex: 1;
    margin: 0;
    padding: 6px;
    list-style: none;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 8px;
    cursor: default;
  }

  .row.selected {
    background: var(--bg-raised);
    box-shadow: inset 2px 0 0 var(--accent);
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

  .description {
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .source {
    font-size: 11px;
    color: var(--text-faint);
  }

  kbd.bad {
    color: var(--danger);
    border-color: var(--danger);
  }

  .empty {
    padding: 24px 12px;
    text-align: center;
    color: var(--text-dim);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: 0 0 12px 12px;
  }

  footer .keys {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
