<script lang="ts">
  // The Launcher: the universal entry point to every Action, and — since the
  // list of Actions is here — where one is added, edited and deleted too.
  // Picking is keyboard only: it is summoned by a hotkey, so the hands are
  // already on the keys. Editing is a second view over the same window
  // (ADR-0007: the window is created once and reused), and while it is open
  // Rust suspends the Launcher's hide-on-blur, because a form that vanishes
  // when you click another app is not a form.
  //
  // This file is the window: which of the two views is up, the keys that belong
  // to the window rather than to a field, and the delete dialog. The views are
  // `ActionList` and `EditorPane`; what they edit is in `actions.svelte.ts`.
  import { onMount } from "svelte";
  import {
    hideLauncher,
    onActionsChanged,
    onConfigChanged,
    onLauncherOpened,
    pickAction,
    showSettings,
    Subscriptions,
  } from "../lib/ipc";
  import { rank } from "../lib/fuzzy";
  import ConfirmDialog from "../lib/ui/ConfirmDialog.svelte";
  import type { Action } from "../lib/types";
  import ActionList from "./ActionList.svelte";
  import EditorPane from "./EditorPane.svelte";
  import { actions } from "./actions.svelte";

  let query = $state("");
  let selected = $state(0);
  let selectionChars = $state(0);
  let list = $state<ActionList | null>(null);

  const editing = $derived(actions.editing);
  const pendingDelete = $derived(actions.pendingDelete);

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
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  {#if editing}
    <EditorPane {editing} />
  {:else}
    <ActionList
      bind:this={list}
      {matches}
      bind:query
      bind:selected
      {selectionChars}
      onrun={run}
      onedit={edit}
    />
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
