<script lang="ts">
  // The Popover window: a header, a scrolling body of turns, and a composer
  // when there is something to type into it. The states that make this window
  // worth having live in `exchange.svelte.ts`; what is left here is the shell,
  // the two pieces of DOM the store cannot reach — the scroller and the
  // composer — and the keys that belong to the window rather than to a field.
  import { onMount } from "svelte";
  import { hidePopover, Subscriptions } from "../lib/ipc";
  import { BrandMark, TextSelect } from "../lib/icons";
  import Composer from "./Composer.svelte";
  import PopoverHeader from "./PopoverHeader.svelte";
  import TurnCard from "./TurnCard.svelte";
  import { exchange } from "./exchange.svelte";

  let scroller = $state<HTMLDivElement | null>(null);
  let composer = $state<Composer | null>(null);

  const view = $derived(exchange.view);
  const subscriptions = new Subscriptions();

  onMount(() => {
    exchange.onStream = scrollToBottom;
    exchange.onIdle = () => composer?.focus();
    exchange.onReset = () => {
      composer?.reset();
      if (scroller) scroller.scrollTop = 0;
    };

    void exchange.load();
    exchange.listen(subscriptions);
    const stopClock = exchange.startClock();

    return () => {
      stopClock();
      void subscriptions.dispose();
    };
  });

  function scrollToBottom() {
    if (!scroller) return;
    // Follow the stream only when the user is already at the bottom: scrolling
    // up to re-read something must not be yanked back by the next delta.
    const distance = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    if (distance > 48) return;
    requestAnimationFrame(() => {
      if (scroller) scroller.scrollTop = scroller.scrollHeight;
    });
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      // Esc cancels a live request first, so partial text stays readable;
      // a second Esc closes the window (README: both behaviours).
      if (!exchange.cancel()) void hidePopover();
      return;
    }

    // Copy is the only export path, so it gets a shortcut that works while the
    // input box has focus.
    if (event.key.toLowerCase() === "c" && event.ctrlKey && event.shiftKey) {
      const answer = exchange.current?.answer;
      if (answer) {
        event.preventDefault();
        void exchange.copy(answer, exchange.turns.length - 1);
      }
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  <PopoverHeader onclose={() => hidePopover()} />

  <div class="body" bind:this={scroller}>
    {#if view === null}
      <p class="hint">Nothing to show.</p>
    {:else if view.phase === "empty-selection" && exchange.turns.length === 0}
      <div class="notice">
        <TextSelect size={22} />
        <p><strong>{view.action_name}</strong> works on a Selection, and nothing was selected.</p>
        <p class="hint">
          Select some text and press the hotkey again. Elevated windows cannot be read at all.
        </p>
      </div>
    {:else if exchange.turns.length === 0}
      <div class="notice">
        <BrandMark size={26} />
        <p class="hint">Type what you want to send to <strong>{view.action_name}</strong>.</p>
      </div>
    {/if}

    {#each exchange.turns as turn, index (index)}
      <TurnCard {turn} {index} />
    {/each}
  </div>

  {#if view && (view.phase === "needs-input" || exchange.canFollowUp)}
    <Composer
      bind:this={composer}
      placeholder={exchange.turns.length === 0 ? "Your input…" : "Ask a follow-up…"}
      disabled={exchange.busy}
      onsend={(text) => void exchange.send(text)}
    />
  {/if}
</div>

<style>
  .body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .notice {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    text-align: center;
    padding: var(--space-4) var(--space-2);
    color: var(--text-dim);
  }

  .notice p {
    margin: 0;
  }
</style>
