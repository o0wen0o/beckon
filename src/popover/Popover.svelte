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
  import {
    BrandMark,
    Check,
    ChevronRight,
    Close,
    Copy,
    Retry,
    Send,
    TextSelect,
    Warning,
  } from "../lib/icons";
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
    reasoningOpen: boolean;
    /** Once the user has toggled it, the auto-collapse stops fighting them. */
    reasoningTouched: boolean;
    questionExpanded: boolean;
  }

  let view = $state<PopoverView | null>(null);
  let turns = $state<Turn[]>([]);
  let draft = $state("");
  let copiedTurn = $state<number | null>(null);
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
            // Thinking arrives before the answer, so show it while it is all
            // there is and get out of the way once real text lands — unless
            // the user has said otherwise by toggling the disclosure.
            if (!turn.reasoningTouched) turn.reasoningOpen = turn.answer === "";
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

  /**
   * The reveal hook. The window outlives every trigger (ADR-0007), so this —
   * not `onMount` — is where per-Exchange state is reset. Anything added to
   * this component that survives a hide has to be cleared here; the per-turn
   * flags come free, because `Turn` objects themselves are rebuilt.
   */
  async function load() {
    view = await getPopoverView();
    draft = "";
    copiedTurn = null;
    resetDraftHeight();
    if (scroller) scroller.scrollTop = 0;
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
    return {
      question,
      answer: "",
      reasoning: "",
      status: "waiting-first-token",
      reasoningOpen: false,
      reasoningTouched: false,
      questionExpanded: false,
    };
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
    if (!scroller) return;
    // Follow the stream only when the user is already at the bottom: scrolling
    // up to re-read something must not be yanked back by the next delta.
    const distance = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    if (distance > 48) return;
    requestAnimationFrame(() => {
      if (scroller) scroller.scrollTop = scroller.scrollHeight;
    });
  }

  function focusDraft() {
    requestAnimationFrame(() => draftBox?.focus());
  }

  /** Grow with the text up to five rows, then scroll inside the box. */
  function growDraft() {
    if (!draftBox) return;
    draftBox.style.height = "auto";
    draftBox.style.height = `${Math.min(draftBox.scrollHeight, 120)}px`;
  }

  function resetDraftHeight() {
    // Clearing `draft` does not shrink an element that was grown inline.
    if (draftBox) draftBox.style.height = "";
  }

  async function send() {
    const text = draft.trim();
    if (text === "" || busy) return;
    draft = "";
    resetDraftHeight();

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

  async function copy(text: string, index: number) {
    // A user-requested clipboard write: not restored (ADR-0002).
    await copyToClipboard(text);
    copiedTurn = index;
    setTimeout(() => {
      if (copiedTurn === index) copiedTurn = null;
    }, 1600);
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
        void copy(answer, turns.length - 1);
      }
    }
  }

  function onDraftKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void send();
    }
  }

  function toggleReasoning(turn: Turn) {
    turn.reasoningTouched = true;
    turn.reasoningOpen = !turn.reasoningOpen;
  }

  /** Failures the user can actually do something about in Settings. */
  function settlesInSettings(kind: string | undefined) {
    return (
      kind === "no-credential" || kind === "read-error" || kind === "auth" || kind === "config"
    );
  }

  const STATE_LABEL: Partial<Record<Status, string>> = {
    "waiting-first-token": "Waiting",
    streaming: "Streaming",
    interrupted: "Interrupted",
    cancelled: "Cancelled",
    error: "Failed",
  };
</script>

<svelte:window on:keydown={onKeydown} />

<div class="surface">
  <header data-tauri-drag-region class:live={current?.status === "streaming"}>
    <span class="mark"><BrandMark size={16} /></span>
    <span class="title">{view?.action_name ?? "Beckon"}</span>

    {#if current && STATE_LABEL[current.status]}
      <span class="state" data-state={current.status}>
        <span class="dot"></span>
        {STATE_LABEL[current.status]}
      </span>
    {/if}

    <span class="model">
      {view?.model.model}{#if view?.model.thinking}<span class="thinking-badge">thinking</span>{/if}
    </span>

    {#if current?.answer}
      <button
        class="icon-button"
        aria-label="Copy answer"
        title="Copy answer"
        onclick={() => copy(current.answer, turns.length - 1)}
      >
        {#if copiedTurn === turns.length - 1}<Check size={14} />{:else}<Copy size={14} />{/if}
      </button>
    {/if}
    <button class="icon-button" aria-label="Close" title="Close" onclick={() => hidePopover()}>
      <Close size={14} />
    </button>
  </header>

  <div class="body" bind:this={scroller}>
    {#if view === null}
      <p class="hint">Nothing to show.</p>
    {:else if view.phase === "empty-selection" && turns.length === 0}
      <div class="notice">
        <TextSelect size={22} />
        <p><strong>{view.action_name}</strong> works on a Selection, and nothing was selected.</p>
        <p class="hint">
          Select some text and press the hotkey again. Elevated windows cannot be read at all.
        </p>
      </div>
    {:else if turns.length === 0}
      <div class="notice">
        <BrandMark size={26} />
        <p class="hint">Type what you want to send to <strong>{view.action_name}</strong>.</p>
      </div>
    {/if}

    {#each turns as turn, index (index)}
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
              onclick={() => toggleReasoning(turn)}
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
          <div class="waiting">
            <div class="waiting-rail"></div>
            <span class="waiting-text">
              Sent to {view?.model.model} — waiting for the first token{waitedSeconds > 0
                ? ` · ${waitedSeconds}s`
                : ""}
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
            <p class="failure-message"><Warning size={14} /> {turn.note}</p>
            <div class="failure-actions">
              <button class="primary" onclick={() => retry()}><Retry size={14} /> Retry</button>
              {#if settlesInSettings(turn.errorKind)}
                <button onclick={() => showSettings()}>Open Settings</button>
              {/if}
            </div>
          </div>
        {/if}

        {#if turn.answer && (turn.status === "done" || turn.status === "interrupted" || turn.status === "cancelled")}
          <div class="turn-actions">
            <button class="copy" onclick={() => copy(turn.answer, index)}>
              {#if copiedTurn === index}
                <Check size={13} /> Copied
              {:else}
                <Copy size={13} /> Copy
              {/if}
            </button>
          </div>
        {/if}
      </article>
    {/each}
  </div>

  {#if view && (view.phase === "needs-input" || canFollowUp)}
    <footer>
      <div class="composer">
        <textarea
          bind:this={draftBox}
          bind:value={draft}
          oninput={growDraft}
          onkeydown={onDraftKeydown}
          rows="1"
          placeholder={turns.length === 0 ? "Your input…" : "Ask a follow-up…"}
        ></textarea>
        <button class="primary send" disabled={draft.trim() === "" || busy} onclick={() => send()}>
          <Send size={14} /> Send
        </button>
      </div>
    </footer>
  {/if}
</div>

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

  @keyframes breathe {
    to {
      opacity: 0.4;
    }
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

  .thinking-badge {
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    padding: 0 var(--space-2);
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

  .turn {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
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

  /* Three independent proofs the request is alive: a moving highlight, a
     counting integer, and the model it went to. The counter is the one that
     survives reduced motion. */
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

  @keyframes travel {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(350%);
    }
  }

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

  footer {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: 0 0 var(--surface-radius) var(--surface-radius);
    background-clip: padding-box;
  }

  .composer {
    display: flex;
    align-items: flex-end;
    gap: var(--space-2);
  }

  /* One row deep to start, and exactly as tall as the button beside it — both
     read `--control-h`, so the pair cannot drift apart. It grows with the text
     from there, and the button stays put at the bottom of the row. */
  .composer textarea {
    min-height: var(--control-h);
    height: var(--control-h);
    max-height: 120px;
    padding-top: var(--space-2);
    padding-bottom: var(--space-2);
    resize: none;
  }

  .send {
    flex: none;
    height: var(--control-h);
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

    .answer.streaming::after,
    .state .dot {
      animation: none;
      opacity: 1;
    }
  }
</style>
