<script lang="ts">
  // The Launcher: the universal entry point to every Action, and only that.
  // Picking is keyboard only — it is summoned by a hotkey, so the hands are
  // already on the keys — and the window dies with its focus. Authoring an
  // Action lives in Settings (ADR-0003), a window that can survive a click
  // elsewhere; a form inside a picker cannot.
  //
  // This file is the window: the query, the selection, and the keys that belong
  // to the window rather than to the field. The list is `ActionList`.
  import { onMount } from "svelte";
  import {
    hideLauncher,
    onActionsChanged,
    onLauncherOpened,
    pickAction,
    showSettings,
    Subscriptions,
  } from "../lib/ipc";
  import { rank } from "../lib/fuzzy";
  import type { Action } from "../lib/types";
  import ActionList from "./ActionList.svelte";
  import { actions } from "./actions.svelte";

  let query = $state("");
  let selected = $state(0);
  let selectionChars = $state(0);
  let list = $state<ActionList | null>(null);

  const matches = $derived(
    rank(actions.snapshot.actions, query, (action) => [action.name, action.description ?? ""]),
  );

  // Clamp rather than reset: the list re-ranks on every keystroke.
  $effect(() => {
    if (selected >= matches.length) selected = Math.max(0, matches.length - 1);
  });

  const subscriptions = new Subscriptions();

  onMount(() => {
    void actions.refresh();
    subscriptions.add(onActionsChanged((next) => actions.adoptActions(next))).add(
      onLauncherOpened((chars) => {
        // A fresh summon starts from a clean slate: the window is reused
        // (ADR-0007), so the last visit's query is still sitting in it.
        selectionChars = chars;
        query = "";
        selected = 0;
        list?.focusQuery();
      }),
    );
    list?.focusQuery();
    return () => void subscriptions.dispose();
  });

  function move(delta: number) {
    if (matches.length === 0) return;
    selected = (selected + delta + matches.length) % matches.length;
    list?.scrollSelectedIntoView();
  }

  function run(action: Action | undefined) {
    if (!action) return;
    void pickAction(action.id);
  }

  /** Editing is elsewhere. `show_settings` hides this window itself — doing it
   *  from here first would let the foreground go back to the app underneath,
   *  and Settings would open behind it. */
  function openSettings() {
    void showSettings();
  }

  function onKeydown(event: KeyboardEvent) {
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
      case ",":
        if (event.ctrlKey) {
          event.preventDefault();
          openSettings();
        }
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  <ActionList
    bind:this={list}
    {matches}
    bind:query
    bind:selected
    {selectionChars}
    onrun={run}
    onsettings={openSettings}
  />
</div>
