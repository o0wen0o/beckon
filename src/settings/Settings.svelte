<script lang="ts">
  // The shell. It owns the subscriptions and the focus signal, and nothing
  // else: every piece of state and every write lives in store.svelte.ts
  // (ADR-0003), so a field component cannot acquire an opinion about the disk.
  //
  // Actions are not edited here — that is the Launcher's job, next to the list
  // they already appear in. What is left is what is global to the app.
  import { onMount } from "svelte";
  import { onActionsChanged, onConfigChanged, onSettingsOpened, Subscriptions } from "../lib/ipc";
  import StatusBar from "../lib/ui/StatusBar.svelte";
  import SettingsNav from "./SettingsNav.svelte";
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
    subscriptions
      .add(
        onConfigChanged((next) => {
          settings.adoptConfig(next);
          void settings.refreshStartupErrors();
        }),
      )
      // Not for the Actions themselves, which this window no longer shows: an
      // Action's Direct Hotkey is what Triggering reports as a startup error.
      .add(onActionsChanged(() => void settings.refreshStartupErrors()))
      .add(
        onSettingsOpened(() => {
          settings.resetTransient();
          void settings.refreshAll();
        }),
      );
    return () => void subscriptions.dispose();
  });

  const route = $derived(settings.route);
</script>

<!-- The window can be hidden mid-edit; a blur is the last chance to write. -->
<svelte:window onblur={() => settings.flush()} />

<div class="shell">
  <div class="columns">
    <SettingsNav />

    <!-- The navigation lives outside the pane on purpose: clicking a nav item
         moves focus out of it, which fires focusout, which flushes the save
         slot. Changing section can never strand an unwritten edit. -->
    <main class="pane" bind:this={paneElement} onfocusout={() => settings.flush()}>
      {#if route === "connection"}
        <Connection />
      {:else if route === "triggering"}
        <Triggering />
      {:else if route === "appearance"}
        <Appearance />
      {:else}
        <ModelDefaults />
      {/if}
    </main>
  </div>

  <StatusBar busy={settings.configSlot.busy} error={settings.saveError} />
</div>

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
