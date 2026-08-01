<script lang="ts">
  // The Popover. Its states are the point of this window: "waiting for the
  // first token" must not look like "streaming", and an interrupted stream must
  // keep the text it already produced (README).
  //
  // Output is rendered as plain text with preserved whitespace: acceptable for
  // the MVP, and it cannot inject anything into the WebView.
  import { onMount } from "svelte";
  import Check from "lucide-svelte/icons/check";
  import Copy from "lucide-svelte/icons/copy";
  import X from "lucide-svelte/icons/x";
  import {
    cancelExchange,
    copyToClipboard,
    describeError,
    followUp,
    getPopoverView,
    hidePopover,
    onDelta,
    onDone,
    onExchangeError,
    onFirstToken,
    onInterrupted,
    onPopoverView,
    retryExchange,
    showSettings,
    submitInput,
    Subscriptions,
  } from "../lib/ipc";
  import type { Failure, PopoverView } from "../lib/types";

  type Status =
    | "waiting-first-token"
    | "streaming"
    | "done"
    | "interrupted"
    | "cancelled"
    | "error";

  interface Turn {
    question: string;
    answer: string;
    reasoning: string;
    status: Status;
    /** Set for `interrupted` and `error`. */
    note?: string;
    errorKind?: string;
  }

  let view = $state<PopoverView | null>(null);
  let turns = $state<Turn[]>([]);
  let draft = $state("");
  let copied = $state(false);
  let waitingSince = $state(0);
  let now = $state(0);
  let scroller = $state<HTMLDivElement | null>(null);
  let draftBox = $state<HTMLTextAreaElement | null>(null);

  const current = $derived(turns.length > 0 ? turns[turns.length - 1] : null);
  const busy = $derived(
    current?.status === "waiting-first-token" || current?.status === "streaming",
  );
  const canFollowUp = $derived(
    view !== null && view.exchange_id !== null && current !== null && !busy,
  );
  const waitedSeconds = $derived(
    waitingSince > 0 ? Math.max(0, Math.floor((now - waitingSince) / 1000)) : 0,
  );

  const subscriptions = new Subscriptions();

  onMount(() => {
    void load();
    subscriptions
      // The window is reused, so a new trigger arrives as an event, not a mount.
      .add(onPopoverView(() => void load()))
      .add(onFirstToken((payload) => forCurrent(payload.exchange_id, markStreaming)))
      .add(
        onDelta((payload) =>
          forCurrent(payload.exchange_id, (turn) => {
            turn.answer += payload.content;
            turn.reasoning += payload.reasoning;
            if (turn.status === "waiting-first-token") markStreaming(turn);
            scrollToBottom();
          }),
        ),
      )
      .add(
        onDone((payload) =>
          forCurrent(payload.exchange_id, (turn) => {
            turn.status = "done";
            waitingSince = 0;
            focusDraft();
          }),
        ),
      )
      .add(
        onInterrupted((payload) =>
          forCurrent(payload.exchange_id, (turn) => {
            // Keep whatever was produced; mark it beneath (README).
            turn.status = "interrupted";
            turn.note = payload.message;
            waitingSince = 0;
          }),
        ),
      )
      .add(
        onExchangeError((payload) =>
          forCurrent(payload.exchange_id, (turn) => applyFailure(turn, payload)),
        ),
      );

    // Only the "waiting for the first token" counter reads `now`, so outside
    // that wait this would be a write per quarter second for the process's
    // lifetime — the Popover window is never destroyed (ADR-0007).
    const ticker = setInterval(() => {
      if (waitingSince > 0) now = Date.now();
    }, 250);
    return () => {
      clearInterval(ticker);
      void subscriptions.dispose();
    };
  });

  async function load() {
    view = await getPopoverView();
    draft = "";
    copied = false;
    if (!view) {
      turns = [];
      return;
    }
    if (view.phase === "running") {
      turns = [newTurn(view.input ?? "")];
    } else {
      turns = [];
      focusDraft();
    }
  }

  function newTurn(question: string): Turn {
    waitingSince = Date.now();
    now = Date.now();
    return { question, answer: "", reasoning: "", status: "waiting-first-token" };
  }

  function markStreaming(turn: Turn) {
    turn.status = "streaming";
    waitingSince = 0;
  }

  /** Apply an event only if it belongs to the Exchange on screen. */
  function forCurrent(exchangeId: string, fn: (turn: Turn) => void) {
    if (!view || view.exchange_id !== exchangeId) return;
    const turn = turns[turns.length - 1];
    if (!turn) return;
    fn(turn);
  }

  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (scroller) scroller.scrollTop = scroller.scrollHeight;
    });
  }

  function focusDraft() {
    requestAnimationFrame(() => draftBox?.focus());
  }

  async function send() {
    const text = draft.trim();
    if (text === "" || busy) return;
    draft = "";

    if (view && view.exchange_id && turns.length > 0) {
      turns = [...turns, newTurn(text)];
      try {
        await followUp(view.exchange_id, text);
      } catch (error) {
        failCurrent(error);
      }
      return;
    }

    turns = [newTurn(text)];
    try {
      const exchangeId = await submitInput(text);
      if (view) view = { ...view, phase: "running", exchange_id: exchangeId, input: text };
    } catch (error) {
      failCurrent(error);
    }
  }

  /** The one place a failure lands on a turn, whether it arrived as an event
   * or as a rejected command. */
  function applyFailure(turn: Turn, failure: Failure) {
    turn.status = "error";
    turn.note = failure.message;
    turn.errorKind = failure.kind;
    waitingSince = 0;
  }

  function failCurrent(error: unknown) {
    const turn = turns[turns.length - 1];
    if (turn) applyFailure(turn, describeError(error));
  }

  async function retry() {
    if (!view?.exchange_id || !current) return;
    current.status = "waiting-first-token";
    current.note = undefined;
    current.answer = "";
    current.reasoning = "";
    waitingSince = Date.now();
    try {
      await retryExchange(view.exchange_id);
    } catch (error) {
      failCurrent(error);
    }
  }

  async function copy(text: string) {
    // A user-requested clipboard write: not restored (ADR-0002).
    await copyToClipboard(text);
    copied = true;
    setTimeout(() => (copied = false), 1600);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      // Esc cancels a live request first, so partial text stays readable;
      // a second Esc closes the window (README: both behaviours).
      if (busy && view?.exchange_id) {
        void cancelExchange(view.exchange_id);
        if (current) {
          current.status = "cancelled";
          waitingSince = 0;
        }
        return;
      }
      void hidePopover();
      return;
    }

    // Copy is the only export path, so it gets a shortcut that works while the
    // input box has focus.
    if (event.key.toLowerCase() === "c" && event.ctrlKey && event.shiftKey) {
      const answer = current?.answer;
      if (answer) {
        event.preventDefault();
        void copy(answer);
      }
    }
  }

  function onDraftKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void send();
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  <header data-tauri-drag-region>
    <span class="title">{view?.action_name ?? "Beckon"}</span>
    <span class="model">
      <span class="model-id">{view?.model.model}</span>
      {#if view?.model.thinking}<span class="thinking-badge">thinking</span>{/if}
    </span>
    <button class="icon close" aria-label="Close" title="Close (Esc)" onclick={() => hidePopover()}>
      <X size={15} aria-hidden="true" />
    </button>
  </header>

  <div class="body" bind:this={scroller}>
    {#if view === null}
      <p class="hint">Nothing to show.</p>
    {:else if view.phase === "empty-selection" && turns.length === 0}
      <div class="notice">
        <p><strong>{view.action_name}</strong> works on selected text, and nothing was selected.</p>
        <p class="hint">
          Select some text and press the hotkey again. Elevated windows cannot be read at all.
        </p>
      </div>
    {:else if turns.length === 0}
      <div class="notice">
        <p class="hint">Type what you want to send to <strong>{view.action_name}</strong>.</p>
      </div>
    {/if}

    {#each turns as turn, index (index)}
      <article class="turn">
        {#if turn.question}
          <div class="question">{turn.question}</div>
        {/if}

        {#if turn.reasoning}
          <details class="reasoning" open={turn.answer === ""}>
            <summary>Thinking</summary>
            <div class="reasoning-text">{turn.reasoning}</div>
          </details>
        {/if}

        {#if turn.status === "waiting-first-token"}
          <div class="waiting">
            <span class="pulse"></span>
            <span class="tabular"
              >Sent — waiting for the first token{waitedSeconds > 0
                ? ` · ${waitedSeconds}s`
                : ""}</span
            >
          </div>
        {/if}

        {#if turn.answer}
          <div class="answer" class:streaming={turn.status === "streaming"}>{turn.answer}</div>
        {/if}

        {#if turn.status === "streaming"}
          <div class="status-line">Streaming…</div>
        {/if}

        {#if turn.status === "interrupted"}
          <div class="status-line interrupted">
            {turn.answer ? "Interrupted" : "Interrupted before any output"}{turn.note
              ? ` — ${turn.note}`
              : ""}
          </div>
        {/if}

        {#if turn.status === "cancelled"}
          <div class="status-line interrupted">
            Cancelled. <kbd>Esc</kbd> again closes the Popover.
          </div>
        {/if}

        {#if turn.status === "error"}
          <div class="failure">
            <p class="error">{turn.note}</p>
            <div class="failure-actions">
              <button onclick={() => retry()}>Retry</button>
              {#if turn.errorKind === "no-credential" || turn.errorKind === "read-error" || turn.errorKind === "auth" || turn.errorKind === "config"}
                <button onclick={() => showSettings()}>Open Settings</button>
              {/if}
            </div>
          </div>
        {/if}

        {#if turn.answer && (turn.status === "done" || turn.status === "interrupted" || turn.status === "cancelled")}
          <div class="turn-actions">
            <button class="primary copy" onclick={() => copy(turn.answer)}>
              {#if copied && index === turns.length - 1}
                <Check size={14} aria-hidden="true" /> Copied
              {:else}
                <Copy size={14} aria-hidden="true" /> Copy
              {/if}
            </button>
            <span class="hint"><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>C</kbd></span>
          </div>
        {/if}
      </article>
    {/each}
  </div>

  {#if view && (view.phase === "needs-input" || canFollowUp)}
    <footer>
      <textarea
        bind:this={draftBox}
        bind:value={draft}
        onkeydown={onDraftKeydown}
        rows="2"
        aria-label={turns.length === 0 ? "Input" : "Follow-up"}
        placeholder={turns.length === 0 ? "Your input…" : "Ask a follow-up…"}
      ></textarea>
      <button class="primary" disabled={draft.trim() === "" || busy} onclick={() => send()}>
        Send
      </button>
    </footer>
  {:else if busy}
    <footer class="busy-footer">
      <span class="hint">
        <kbd>Esc</kbd> cancels the request; <kbd>Esc</kbd> again closes the Popover.
      </span>
    </footer>
  {/if}
</div>

<style>
  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--border);
    cursor: default;
    user-select: none;
  }

  .title {
    font-weight: 600;
    letter-spacing: -0.011em;
  }

  .model {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  /* A model id is a machine string; setting it in the mono face says so, and
     stops `deepseek-v3.2` from reading as prose next to the Action name. */
  .model-id {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .thinking-badge {
    flex: none;
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border-radius: var(--radius-sm);
    padding: 1px 6px;
    background: var(--bg-input);
    color: var(--warn);
  }

  .close {
    margin-left: auto;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .notice p {
    margin: 0 0 var(--space-2);
  }

  .turn {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* The prompt is context, not content: recessed, capped, and scrollable so a
     long Selection cannot push the answer off screen. */
  .question {
    font-size: 12px;
    color: var(--text-dim);
    background: var(--bg-input);
    border-radius: var(--radius);
    padding: var(--space-2) var(--space-3);
    white-space: pre-wrap;
    max-height: 5.5em;
    overflow-y: auto;
  }

  .answer {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  /* A caret that only exists while tokens are arriving: the visible difference
     between "streaming" and "done". */
  .answer.streaming::after {
    content: "";
    display: inline-block;
    width: 7px;
    height: 1em;
    margin-left: 2px;
    vertical-align: text-bottom;
    border-radius: 1px;
    background: var(--accent);
    animation: blink 1s steps(2, start) infinite;
  }

  @keyframes blink {
    to {
      visibility: hidden;
    }
  }

  /* Not a generic spinner: it says what it is waiting for. */
  .waiting {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-dim);
    font-size: 13px;
  }

  .pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.25;
      transform: scale(0.8);
    }
    50% {
      opacity: 1;
      transform: scale(1.2);
    }
  }

  /* Both loops above mark a live state, so with motion off they hold their
     visible frame instead of stopping wherever the last keyframe left them —
     a caret frozen invisible would read as "done". */
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      opacity: 1;
      transform: none;
    }

    .answer.streaming::after {
      visibility: visible;
    }
  }

  .status-line {
    font-size: 12px;
    color: var(--text-faint);
  }

  .status-line.interrupted {
    color: var(--warn);
  }

  .reasoning {
    font-size: 12px;
    color: var(--text-dim);
    background: var(--bg-input);
    border-radius: var(--radius);
    padding: var(--space-2) var(--space-3);
  }

  .reasoning summary {
    cursor: default;
    color: var(--text-faint);
  }

  .reasoning-text {
    white-space: pre-wrap;
    margin-top: var(--space-2);
    max-height: 8em;
    overflow-y: auto;
  }

  .failure {
    background: var(--danger-soft);
    border-radius: var(--radius);
    padding: var(--space-3);
  }

  .failure p {
    margin: 0 0 var(--space-2);
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

  .copy {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 96px;
  }

  .turn-actions .hint {
    display: flex;
    gap: 3px;
  }

  footer {
    display: flex;
    gap: var(--space-2);
    align-items: flex-end;
    padding: var(--space-3);
    border-top: 1px solid var(--border);
  }

  footer textarea {
    min-height: 44px;
  }

  .busy-footer {
    justify-content: center;
  }
</style>
