<script lang="ts">
  // The shell. It owns the subscriptions and the focus signal, and nothing
  // else: every piece of state and every write lives in a store — the global
  // config in store.svelte.ts, the Actions in actions.svelte.ts (ADR-0003) —
  // so a field component cannot acquire an opinion about the disk.
  import { onMount } from "svelte";
  import { onActionsChanged, onConfigChanged, onSettingsOpened, Subscriptions } from "../lib/ipc";
  import ConfirmDialog from "../lib/ui/ConfirmDialog.svelte";
  import StatusBar from "../lib/ui/StatusBar.svelte";
  import SettingsNav from "./SettingsNav.svelte";
  import { actionStore } from "./actions.svelte";
  import Actions from "./sections/actions/Actions.svelte";
  import Appearance from "./sections/Appearance.svelte";
  import Connection from "./sections/Connection.svelte";
  import ModelDefaults from "./sections/ModelDefaults.svelte";
  import Triggering from "./sections/Triggering.svelte";
  import { settings } from "./store.svelte";

  let paneElement = $state<HTMLElement | null>(null);

  $effect(() => {
    settings.pane = paneElement;
  });

  const subscriptions = new Subscriptions();

  onMount(() => {
    // The first open builds the window, so `settings:opened` fires before
    // anything is listening — this component always loads itself.
    void settings.refreshAll();
    void actionStore.refresh();
    subscriptions
      .add(
        onConfigChanged((next) => {
          settings.adoptConfig(next);
          void settings.refreshStartupErrors();
        }),
      )
      .add(
        onActionsChanged((next) => {
          actionStore.adoptActions(next);
          // An Action's Direct Hotkey is what Triggering reports as a startup
          // error, so the two sections move together.
          void settings.refreshStartupErrors();
        }),
      )
      .add(
        onSettingsOpened(() => {
          settings.resetTransient();
          // The window is reused (ADR-0007), so a fresh open must not resume
          // whatever Action the last visit left open in the editor.
          actionStore.close();
          void settings.refreshAll();
          void actionStore.refresh();
        }),
      );
    return () => void subscriptions.dispose();
  });

  const route = $derived(settings.route);
  const pendingDelete = $derived(actionStore.pendingDelete);

  /** Both stores hold a write; whatever moved focus ends both of them. */
  function flush() {
    settings.flush();
    actionStore.flush();
  }
</script>

<!-- The window can be hidden mid-edit; a blur is the last chance to write. -->
<svelte:window onblur={flush} />

<div class="shell">
  <div class="columns">
    <SettingsNav />

    <!-- The navigation lives outside the pane on purpose: clicking a nav item
         moves focus out of it, which fires focusout, which flushes the save
         slot. Changing section can never strand an unwritten edit. -->
    <main class="pane" bind:this={paneElement} onfocusout={flush}>
      {#if route === "connection"}
        <Connection />
      {:else if route === "actions"}
        <Actions />
      {:else if route === "triggering"}
        <Triggering />
      {:else if route === "appearance"}
        <Appearance />
      {:else}
        <ModelDefaults />
      {/if}
    </main>
  </div>

  <StatusBar
    busy={settings.configSlot.busy || actionStore.slot.busy}
    error={settings.saveError ?? actionStore.slot.error}
  />
</div>

<!-- Hosted here, not in the editor: confirming deletes the Action, which
     unmounts the editor — and a <dialog> removed from the DOM while open never
     calls close(), leaving the whole window inert behind an invisible modal. -->
<ConfirmDialog
  open={pendingDelete !== null}
  title={`Delete “${pendingDelete?.name || pendingDelete?.file_name}”?`}
  confirmLabel="Delete file"
  destructive
  onconfirm={() => pendingDelete && actionStore.deleteAction(pendingDelete)}
  oncancel={() => (actionStore.pendingDelete = null)}
>
  {#snippet body()}
    <p>
      The file <code>{pendingDelete?.file_name}</code> is removed from disk. This cannot be undone.
    </p>
  {/snippet}
</ConfirmDialog>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .columns {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .pane {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: var(--space-6) var(--space-8) var(--space-10);
  }

  .pane :global(h1) {
    margin: 0 0 var(--space-5);
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-semibold);
  }

  .pane :global(h2) {
    margin: var(--space-6) 0 var(--space-3);
    font-family: var(--font-small);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
</style>
