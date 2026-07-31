<script lang="ts">
  // The Popover. Its states are the point of this window: "waiting for the
  // first token" must not look like "streaming", and an interrupted stream must
  // keep the text it already produced (README).
  //
  // Output is rendered as plain text with preserved whitespace: acceptable for
  // the MVP, and it cannot inject anything into the WebView.
  import { onMount } from "svelte";
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
  import type { PopoverView } from "../lib/types";

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
          forCurrent(payload.exchange_id, (turn) => {
            turn.status = "error";
            turn.note = payload.message;
            turn.errorKind = payload.kind;
            waitingSince = 0;
          }),
        ),
      );

    const ticker = setInterval(() => (now = Date.now()), 250);
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

  function failCurrent(error: unknown) {
    const failure = describeError(error);
    const turn = turns[turns.length - 1];
    if (!turn) return;
    turn.status = "error";
    turn.note = failure.message;
    turn.errorKind = failure.kind;
    waitingSince = 0;
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
      {view?.model.model}{#if view?.model.thinking}<span class="thinking-badge">thinking</span>{/if}
    </span>
    <button class="close" title="Close (Esc)" onclick={() => hidePopover()}>✕</button>
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
            <span>Sent — waiting for the first token{waitedSeconds > 0 ? ` · ${waitedSeconds}s` : ""}</span>
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
              {copied && index === turns.length - 1 ? "Copied ✓" : "Copy"}
            </button>
            <span class="hint"><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd></span>
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
    gap: 10px;
    padding: 8px 8px 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: 12px 12px 0 0;
    cursor: default;
    user-select: none;
  }

  .title {
    font-weight: 600;
  }

  .model {
    font-size: 11px;
    color: var(--text-faint);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .thinking-badge {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 6px;
    color: var(--warn);
  }

  .close {
    margin-left: auto;
    border: none;
    background: none;
    color: var(--text-dim);
    padding: 2px 8px;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .notice p {
    margin: 0 0 6px;
  }

  .turn {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .question {
    font-size: 12px;
    color: var(--text-dim);
    background: var(--bg-input);
    border-left: 2px solid var(--border-strong);
    border-radius: 0 6px 6px 0;
    padding: 6px 10px;
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
    gap: 8px;
    color: var(--text-dim);
    font-size: 13px;
  }

  .pulse {
    width: 8px;
    height: 8px;
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
    border: 1px dashed var(--border);
    border-radius: 8px;
    padding: 6px 10px;
  }

  .reasoning summary {
    cursor: default;
    color: var(--text-faint);
  }

  .reasoning-text {
    white-space: pre-wrap;
    margin-top: 6px;
    max-height: 8em;
    overflow-y: auto;
  }

  .failure {
    border: 1px solid var(--danger);
    border-radius: 8px;
    padding: 8px 10px;
  }

  .failure p {
    margin: 0 0 8px;
  }

  .failure-actions {
    display: flex;
    gap: 8px;
  }

  .turn-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .copy {
    min-width: 92px;
  }

  footer {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: 0 0 12px 12px;
  }

  footer textarea {
    min-height: 44px;
  }

  .busy-footer {
    justify-content: center;
  }
</style>
