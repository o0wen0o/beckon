<script lang="ts">
  // One turn: what was asked, what the model thought, what it answered, and —
  // when it went wrong — what to do about it. Every state a turn can be in is
  // rendered here, so the ones that must not look alike sit side by side.
  //
  // Output is plain text with preserved whitespace: acceptable for the MVP, and
  // it cannot inject anything into the WebView.
  import { describeFailure } from "../lib/failures";
  import { Check, ChevronRight, Copy, Retry, Warning } from "../lib/icons";
  import { showSettings } from "../lib/ipc";
  import { exchange, settlesInSettings, type Turn } from "./exchange.svelte";

  let { turn, index }: { turn: Turn; index: number } = $props();

  /**
   * The cause named first, then the provider's own words — the same sentence
   * Settings builds. Printing `note` bare handed the user a raw reqwest chain
   * for a `network` failure while Settings said "Could not reach the API".
   */
  const failure = $derived(
    turn.status === "error"
      ? describeFailure({ kind: turn.errorKind ?? "error", message: turn.note ?? "" })
      : null,
  );

  const settled = $derived(
    turn.status === "done" || turn.status === "interrupted" || turn.status === "cancelled",
  );

  /** The counter only appears once there is a second to show. */
  const waited = $derived(exchange.waitedSeconds > 0 ? ` · ${exchange.waitedSeconds}s` : "");
</script>

<article class="turn">
  {#if turn.question}
    <div class="question" class:expanded={turn.questionExpanded}>{turn.question}</div>
    {#if turn.question.length > 160}
      <button class="link" onclick={() => (turn.questionExpanded = !turn.questionExpanded)}>
        {turn.questionExpanded ? "Show less" : "Show all"}
      </button>
    {/if}
  {/if}

  {#if turn.reasoning}
    <div class="reasoning" class:open={turn.reasoningOpen}>
      <button
        class="reasoning-summary"
        aria-expanded={turn.reasoningOpen}
        onclick={() => exchange.toggleReasoning(turn)}
      >
        <span class="chevron"><ChevronRight size={12} /></span>
        Thinking
      </button>
      {#if turn.reasoningOpen}
        <div class="reasoning-text">{turn.reasoning}</div>
      {/if}
    </div>
  {/if}

  {#if turn.status === "waiting-first-token"}
    <!-- Three independent proofs the request is alive: a moving highlight, a
         counting integer, and the model it went to. -->
    <div class="waiting">
      <div class="waiting-rail"></div>
      <span class="waiting-text">
        Sent to {exchange.view?.model.model} — waiting for the first token{waited}
      </span>
    </div>
  {/if}

  {#if turn.answer}
    <div
      class="answer"
      class:streaming={turn.status === "streaming"}
      class:partial={turn.status === "interrupted" || turn.status === "cancelled"}
    >
      {turn.answer}
    </div>
  {/if}

  {#if turn.status === "interrupted"}
    <p class="status-line warn">
      <Warning size={12} />
      {turn.answer ? "Interrupted" : "Interrupted before any output"}{turn.note
        ? ` — ${turn.note}`
        : ""}
    </p>
  {/if}

  {#if turn.status === "cancelled"}
    <p class="status-line warn">Cancelled.</p>
  {/if}

  {#if turn.status === "error"}
    <div class="failure">
      <p class="failure-message"><Warning size={14} /> {failure}</p>
      <div class="failure-actions">
        <button class="primary" onclick={() => exchange.retry()}><Retry size={14} /> Retry</button>
        {#if settlesInSettings(turn.errorKind)}
          <button onclick={() => showSettings()}>Open Settings</button>
        {/if}
      </div>
    </div>
  {/if}

  {#if turn.answer && settled}
    <div class="turn-actions">
      <button class="copy" onclick={() => exchange.copy(turn.answer, index)}>
        {#if exchange.copiedTurn === index}
          <Check size={13} /> Copied
        {:else}
          <Copy size={13} /> Copy
        {/if}
      </button>
    </div>
  {/if}
</article>

<style>
  .turn {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* A follow-up is a new question, and a gap alone does not say where the last
     answer stopped — a long answer runs straight into the next question. */
  .turn + :global(.turn) {
    padding-top: var(--space-4);
    border-top: 1px solid var(--border);
  }

  /* Clamped, not scrollable: a scroller inside the body's scroller is a trap. */
  .question {
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
    background: var(--bg-sunken);
    border-left: 2px solid var(--border-strong);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    padding: var(--space-2) var(--space-3);
    white-space: pre-wrap;
  }

  .question:not(.expanded) {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
  }

  .link {
    align-self: flex-start;
    border: none;
    background: none;
    padding: 0;
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
    text-decoration: underline;
  }

  .link:hover:not(:disabled) {
    background: none;
    border-color: transparent;
    color: var(--accent);
  }

  /* The only prose in the product, so it gets its own leading. */
  .answer {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.6;
  }

  .answer.partial {
    border-left: 2px solid var(--warn);
    padding-left: var(--space-3);
  }

  /* Not a blinking text caret — a blink says "type here". A steady bar that
     breathes says "output is arriving". */
  .answer.streaming::after {
    content: "";
    display: inline-block;
    width: 8px;
    height: 1.05em;
    margin-left: 2px;
    vertical-align: text-bottom;
    border-radius: 2px;
    background: linear-gradient(135deg, var(--brand-from), var(--brand-to));
    box-shadow: 0 0 10px -2px var(--accent-glow);
    animation: breathe 1200ms ease-in-out infinite alternate;
  }

  .waiting {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .waiting-rail {
    position: relative;
    height: 2px;
    border-radius: var(--radius-pill);
    background: var(--border);
    overflow: hidden;
  }

  .waiting-rail::after {
    content: "";
    position: absolute;
    inset: 0;
    width: 40%;
    border-radius: var(--radius-pill);
    background: linear-gradient(90deg, var(--brand-from), var(--brand-to));
    animation: travel 1500ms linear infinite;
  }

  /* The counter is the indicator that survives reduced motion. */
  .waiting-text {
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }

  .status-line {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin: 0;
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-faint);
  }

  .status-line.warn {
    color: var(--warn);
  }

  .reasoning {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .reasoning-summary {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    border: none;
    background: none;
    padding: var(--space-1) var(--space-2);
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-faint);
    justify-content: flex-start;
  }

  .reasoning-summary:hover:not(:disabled) {
    background: none;
    border-color: transparent;
    color: var(--text-dim);
  }

  .chevron {
    display: flex;
    transition: transform var(--dur-base) var(--ease-out);
  }

  .reasoning.open .chevron {
    transform: rotate(90deg);
  }

  .reasoning-text {
    white-space: pre-wrap;
    max-height: 10em;
    overflow-y: auto;
    padding: 0 var(--space-2) var(--space-2);
  }

  .failure {
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .failure-message {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    margin: 0;
    color: var(--danger);
  }

  .failure-actions {
    display: flex;
    gap: var(--space-2);
  }

  .turn-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  /* Small, and quiet: the answer is the content, and the header carries the
     same action for the turn on screen. Fixed width so the Copied swap cannot
     reflow the row. */
  .copy {
    min-width: 88px;
    padding: var(--space-1) var(--space-2);
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .copy:hover:not(:disabled) {
    color: var(--text);
  }

  /* The looping indicators need a static form, not a frozen frame: freezing
     the travelling rail mid-slide would read as a stalled progress bar. */
  @media (prefers-reduced-motion: reduce) {
    .waiting-rail::after {
      animation: none;
      transform: none;
      width: 100%;
      opacity: 0.7;
    }

    .answer.streaming::after {
      animation: none;
      opacity: 1;
    }
  }
</style>
