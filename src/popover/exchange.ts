// The Popover's view of the Exchange on screen.
//
// The states are the point of this window: "waiting for the first token" must
// not look like "streaming", and an interrupted stream must keep the text it
// already produced (README). All of that is decided here, so the components
// only render turns — the state machine cannot be half-implemented in markup.
//
// Rust drives it with events, not return values, so this module is mostly a
// reducer over `exchange:*`. It touches no DOM at all: following the stream and
// focusing the composer are effects in the components, keyed off what changed
// here.
//
// A module-level singleton, because there is exactly one Popover window and it
// is never destroyed (ADR-0007). Components reach it through `useStore`.
import {
  cancelExchange,
  copyToClipboard,
  describeError,
  followUp,
  getPopoverView,
  onDelta,
  onDone,
  onExchangeError,
  onFirstToken,
  onInterrupted,
  onPopoverView,
  retryExchange,
  submitInput,
  Subscriptions,
} from "../lib/ipc";
import { Notifier } from "../lib/store";
import type { Failure, PopoverView } from "../lib/types";

export type Status =
  | "waiting-first-token"
  | "streaming"
  | "done"
  | "interrupted"
  | "cancelled"
  | "error";

export interface Turn {
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

/** How often the "waiting" counter reads the clock. */
const TICK = 250;

/** Failures the user can actually do something about in Settings. */
export function settlesInSettings(kind: string | undefined) {
  return kind === "no-credential" || kind === "read-error" || kind === "auth" || kind === "config";
}

/** A turn that will not change again, so its answer can be offered for copying. */
export function isSettled(status: Status) {
  return status === "done" || status === "interrupted" || status === "cancelled";
}

/**
 * The standing notice a conversation carries when it has no turns yet. Only one
 * of them is an alarm, which is what the Popover renders as a `Callout`.
 */
export type Notice = "none" | "no-view" | "empty-selection" | "awaiting-input";

class ExchangeStore extends Notifier {
  view: PopoverView | null = null;
  turns: Turn[] = [];
  copiedTurn: number | null = null;
  /**
   * Bumped by every reveal. The window is reused (ADR-0007), so the composer
   * from the last trigger is still mounted with the last trigger's draft in it
   * and grown to the last trigger's height; keying it on this remounts it, and
   * a fresh element is the only reliable way to clear both at once.
   */
  epoch = 0;

  /** Zero unless a turn is waiting for its first token. */
  #waitingSince = 0;
  #waited = 0;

  get current(): Turn | null {
    return this.turns.length > 0 ? this.turns[this.turns.length - 1] : null;
  }

  get busy() {
    const status = this.current?.status;
    return status === "waiting-first-token" || status === "streaming";
  }

  get canFollowUp() {
    return (
      this.view !== null && this.view.exchange_id !== null && this.current !== null && !this.busy
    );
  }

  get waitedSeconds() {
    return this.#waited;
  }

  /** Whether the composer belongs on screen: the Action asked for typed input,
   *  or the Exchange has settled and a follow-up is possible. */
  get composing() {
    return this.view !== null && (this.view.phase === "needs-input" || this.canFollowUp);
  }

  /**
   * `PopoverPhase` is resolved in Rust precisely so the rule lives in one place;
   * choosing the notice from it in a JSX ternary chain put a second copy of that
   * rule in the one file no test can reach.
   */
  get notice(): Notice {
    if (this.view === null) return "no-view";
    if (this.turns.length > 0) return "none";
    return this.view.phase === "empty-selection" ? "empty-selection" : "awaiting-input";
  }

  /** What the bar along the bottom says a live turn is doing. `busy` decides
   *  whether the bar is there at all; this is the same register, so it is
   *  resolved beside it rather than re-split in markup. */
  get runLabel() {
    return this.current?.status === "streaming" ? "Streaming" : "Waiting";
  }

  // --- lifecycle ----------------------------------------------------------

  /**
   * Wire the window up. The Popover outlives every trigger (ADR-0007), so a
   * new one arrives as `popover:view`, not as a mount.
   */
  listen(subscriptions: Subscriptions) {
    subscriptions
      .add(onPopoverView(() => void this.load()))
      .add(
        onFirstToken((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => this.#markStreaming(turn)),
        ),
      )
      .add(
        onDelta((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            turn.answer += payload.content;
            turn.reasoning += payload.reasoning;
            // Thinking arrives before the answer, so show it while it is all
            // there is and get out of the way once real text lands — unless
            // the user has said otherwise by toggling the disclosure.
            if (!turn.reasoningTouched) turn.reasoningOpen = turn.answer === "";
            if (turn.status === "waiting-first-token") this.#markStreaming(turn);
          }),
        ),
      )
      .add(
        onDone((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            turn.status = "done";
            this.#waitingSince = 0;
          }),
        ),
      )
      .add(
        onInterrupted((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            // Keep whatever was produced; mark it beneath (README).
            turn.status = "interrupted";
            turn.note = payload.message;
            this.#waitingSince = 0;
          }),
        ),
      )
      .add(
        onExchangeError((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => this.#applyFailure(turn, payload)),
        ),
      );
  }

  /**
   * Only the "waiting for the first token" counter reads the clock, so outside
   * that wait this would be a re-render per quarter second for the process's
   * lifetime — the Popover window is never destroyed (ADR-0007). It publishes
   * only when the whole second it displays actually changes.
   */
  startClock() {
    const ticker = setInterval(() => {
      const seconds =
        this.#waitingSince === 0
          ? 0
          : Math.max(0, Math.floor((Date.now() - this.#waitingSince) / 1000));
      if (seconds === this.#waited) return;
      this.#waited = seconds;
      this.notify();
    }, TICK);
    return () => clearInterval(ticker);
  }

  /**
   * The reveal hook. The window outlives every trigger, so this — not a mount —
   * is where per-Exchange state is reset. Anything added to this store that
   * survives a hide has to be cleared here; the per-turn flags come free,
   * because `Turn` objects themselves are rebuilt.
   */
  async load() {
    this.view = await getPopoverView();
    this.copiedTurn = null;
    this.epoch += 1;
    if (!this.view) {
      this.turns = [];
      this.notify();
      return;
    }
    this.turns = this.view.phase === "running" ? [this.#newTurn(this.view.input ?? "")] : [];
    this.notify();
  }

  // --- the user's side ----------------------------------------------------

  async send(text: string) {
    if (text === "" || this.busy) return;

    if (this.view && this.view.exchange_id && this.turns.length > 0) {
      this.turns = [...this.turns, this.#newTurn(text)];
      this.notify();
      try {
        await followUp(this.view.exchange_id, text);
      } catch (error) {
        this.#failCurrent(error);
      }
      return;
    }

    this.turns = [this.#newTurn(text)];
    this.notify();
    try {
      const exchangeId = await submitInput(text);
      if (this.view) {
        this.view = { ...this.view, phase: "running", exchange_id: exchangeId, input: text };
        this.notify();
      }
    } catch (error) {
      this.#failCurrent(error);
    }
  }

  async retry() {
    const current = this.current;
    if (!this.view?.exchange_id || !current) return;
    current.status = "waiting-first-token";
    current.note = undefined;
    current.answer = "";
    current.reasoning = "";
    this.#startWaiting();
    this.notify();
    try {
      await retryExchange(this.view.exchange_id);
    } catch (error) {
      this.#failCurrent(error);
    }
  }

  /**
   * Esc cancels a live request first, so partial text stays readable; the
   * caller closes the window when there is nothing to cancel (README).
   */
  cancel(): boolean {
    if (!this.busy || !this.view?.exchange_id) return false;
    void cancelExchange(this.view.exchange_id);
    if (this.current) {
      this.current.status = "cancelled";
      this.#waitingSince = 0;
      this.notify();
    }
    return true;
  }

  async copy(text: string, index: number) {
    // A user-requested clipboard write: not restored (ADR-0002).
    await copyToClipboard(text);
    this.copiedTurn = index;
    this.notify();
    setTimeout(() => {
      if (this.copiedTurn !== index) return;
      this.copiedTurn = null;
      this.notify();
    }, 1600);
  }

  toggleReasoning(turn: Turn) {
    turn.reasoningTouched = true;
    turn.reasoningOpen = !turn.reasoningOpen;
    this.notify();
  }

  expandQuestion(turn: Turn) {
    turn.questionExpanded = !turn.questionExpanded;
    this.notify();
  }

  // --- internals ----------------------------------------------------------

  #newTurn(question: string): Turn {
    this.#startWaiting();
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

  #startWaiting() {
    this.#waitingSince = Date.now();
    this.#waited = 0;
  }

  /** Both callers are inside `#forCurrent`, which publishes once the event has
   *  landed — so this must not publish too, or every first token re-renders the
   *  window twice. */
  #markStreaming(turn: Turn) {
    turn.status = "streaming";
    this.#waitingSince = 0;
  }

  /** Apply an event only if it belongs to the Exchange on screen. */
  #forCurrent(exchangeId: string, fn: (turn: Turn) => void) {
    if (!this.view || this.view.exchange_id !== exchangeId) return;
    const turn = this.current;
    if (!turn) return;
    fn(turn);
    this.notify();
  }

  /** The one place a failure lands on a turn, whether it arrived as an event
   *  or as a rejected command. */
  #applyFailure(turn: Turn, failure: Failure) {
    turn.status = "error";
    turn.note = failure.message;
    turn.errorKind = failure.kind;
    this.#waitingSince = 0;
  }

  #failCurrent(error: unknown) {
    const turn = this.current;
    if (!turn) return;
    this.#applyFailure(turn, describeError(error));
    this.notify();
  }
}

export const exchange = new ExchangeStore();
