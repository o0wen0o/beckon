<script lang="ts">
  // The Popover's title bar: what is running, how it is going, and the two
  // things you can do to the window. It is also the drag region, so nothing in
  // it may take focus except the buttons.
  import { BrandMark, Check, Close, Copy, Warning } from "../lib/icons";
  import { exchange, type Status } from "./exchange.svelte";

  const STATE_LABEL: Partial<Record<Status, string>> = {
    "waiting-first-token": "Waiting",
    streaming: "Streaming",
    interrupted: "Interrupted",
    cancelled: "Cancelled",
    error: "Failed",
  };

  const current = $derived(exchange.current);
  const lastIndex = $derived(exchange.turns.length - 1);

  let { onclose }: { onclose: () => void } = $props();
</script>

<header data-tauri-drag-region class:live={current?.status === "streaming"}>
  <span class="mark"><BrandMark size={16} /></span>
  <span class="title">{exchange.view?.action_name ?? "Beckon"}</span>

  {#if current && STATE_LABEL[current.status]}
    <span class="state" data-state={current.status}>
      <span class="dot"></span>
      {STATE_LABEL[current.status]}
    </span>
  {/if}

  <span class="model">
    {exchange.view?.model.model}{#if exchange.view?.model.thinking}<span class="thinking-badge"
        >thinking</span
      >{/if}
  </span>

  <!-- Esc cancels a live request, and nothing said so: the window offered only
       a close button, so the choice between "stop this" and "throw it away" was
       invisible unless you already knew the shortcut. -->
  {#if exchange.busy}
    <button class="stop" title="Stop the request (Esc)" onclick={() => exchange.cancel()}>
      <Warning size={13} /> Stop
    </button>
  {/if}

  {#if current?.answer}
    <button
      class="icon-button"
      aria-label="Copy answer"
      title="Copy answer"
      onclick={() => exchange.copy(current.answer, lastIndex)}
    >
      {#if exchange.copiedTurn === lastIndex}<Check size={14} />{:else}<Copy size={14} />{/if}
    </button>
  {/if}
  <button class="icon-button" aria-label="Close" title="Close" onclick={onclose}>
    <Close size={14} />
  </button>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: var(--surface-radius) var(--surface-radius) 0 0;
    background-clip: padding-box;
    cursor: default;
    user-select: none;
  }

  /* A whole-window "alive" signal, readable from the corner of the eye. */
  header.live {
    border-bottom-color: transparent;
    box-shadow: inset 0 -1px 0 0 var(--brand-to);
  }

  .mark {
    display: flex;
  }

  .title {
    font-family: var(--font-display);
    font-weight: var(--weight-semibold);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    flex: none;
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-dim);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--brand-from), var(--brand-to));
  }

  .state[data-state="waiting-first-token"] .dot,
  .state[data-state="streaming"] .dot {
    animation: breathe 1400ms ease-in-out infinite alternate;
  }

  .state[data-state="interrupted"] .dot,
  .state[data-state="cancelled"] .dot {
    background: var(--warn);
  }

  .state[data-state="error"] .dot {
    background: var(--danger);
  }

  .model {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex: none;
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
  }

  /* `--accent`, not `--warn`: thinking being on is a capability in use, not a
     condition to act on, and the amber read as "something is wrong here". */
  .thinking-badge {
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    padding: 0 var(--space-2);
    color: var(--accent);
  }

  .stop {
    flex: none;
    padding: 2px var(--space-2);
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-dim);
  }

  .stop:hover:not(:disabled) {
    border-color: var(--warn);
    color: var(--warn);
  }

  .icon-button {
    flex: none;
    border: none;
    background: none;
    color: var(--text-dim);
    padding: var(--space-1);
    border-radius: var(--radius-sm);
  }

  .icon-button:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: transparent;
    color: var(--text);
  }

  /* A frozen breath is just a dot; the colour still carries the state. */
  @media (prefers-reduced-motion: reduce) {
    .state .dot {
      animation: none;
      opacity: 1;
    }
  }
</style>
