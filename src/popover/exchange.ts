// The Popover's view of the Exchange on screen.
//
// The states are the point: "waiting for the first token" must not look like
// "streaming", and an interrupted stream keeps the text it already produced
// (README). All of it is decided here so the components only render turns.
//
// Rust drives it with events, not return values, so this is mostly a reducer
// over `exchange:*`. It touches no DOM — following the stream and focusing the
// composer are effects in the components, keyed off what changed here.
//
// A module-level singleton: one Popover window, never destroyed (ADR-0007).
import {
  cancelExchange,
  copyToClipboard,
  discardCapture,
  describeError,
  followUp,
  getPopoverView,
  onDelta,
  onDone,
  onExchangeError,
  onFirstToken,
  onInterrupted,
  onPopoverCapture,
  onPopoverView,
  retryExchange,
  startCapture,
  submitInput,
  Subscriptions,
} from "../lib/ipc";
import { Notifier } from "../lib/store";
import type { Capture, Failure, PopoverView } from "../lib/types";

export type Status =
  | "waiting-first-token"
  | "streaming"
  | "done"
  | "interrupted"
  | "cancelled"
  | "error";

export interface Turn {
  question: string;
  /** The Capture that went with the question, if one did (ADR-0016). */
  capture: Capture | null;
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
  /**
   * Attached and not yet sent. Rust owns it (ADR-0003) and hands it over on
   * `popover:capture`; this is that value, not a second opinion about it.
   */
  capture: Capture | null = null;
  /** The last snip came back with nothing. Cleared by the next one. */
  captureCancelled = false;
  /** A screenshot that was taken and cannot be sent, by cause. */
  captureError: Failure | null = null;
  copiedTurn: number | null = null;
  /**
   * Bumped by every reveal. The window is reused (ADR-0007), so the composer
   * still holds the last trigger's draft at the last trigger's height; keying
   * it on this remounts it, which clears both at once.
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

  /** Whether there is anything to send. A Capture is input on its own: the
   *  Action's own prompt is the question being asked about it (ADR-0016). */
  get sendable() {
    return this.capture !== null;
  }

  /** `PopoverPhase` is resolved in Rust so the rule lives in one place; picking
   *  the notice here keeps the second half of it out of markup. */
  get notice(): Notice {
    if (this.view === null) return "no-view";
    if (this.turns.length > 0) return "none";
    return this.view.phase === "empty-selection" ? "empty-selection" : "awaiting-input";
  }

  /** Which of the two words the bar along the bottom says. `busy` decides
   *  whether the bar is there at all; this picks between "streaming" and
   *  "waiting", and the words themselves are the catalog's (ADR-0015). */
  get streaming() {
    return this.current?.status === "streaming";
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
        onPopoverCapture((payload) => {
          this.capture = payload.capture;
          this.captureCancelled = payload.cancelled;
          this.captureError = payload.error;
          this.notify();
        }),
      )
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
            // Thinking arrives first: show it while it is all there is, get
            // out of the way once real text lands — unless the user toggled.
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
   * Only the "waiting for the first token" counter reads the clock; the window
   * is never destroyed (ADR-0007), so this would otherwise re-render four times
   * a second forever. It publishes only when the displayed second changes.
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
   * survives a hide has to be cleared here.
   */
  async load() {
    this.view = await getPopoverView();
    this.copiedTurn = null;
    this.epoch += 1;
    // A Capture belongs to the Popover that was showing, so a fresh trigger
    // starts with nothing attached — and Rust says so, rather than this
    // assuming it.
    this.capture = this.view?.capture ?? null;
    this.captureCancelled = this.view?.capture_cancelled ?? false;
    this.captureError = this.view?.capture_error ?? null;
    if (!this.view) {
      this.turns = [];
      this.notify();
      return;
    }
    this.turns = this.view.phase === "running" ? [this.#newTurn(this.view.input ?? "")] : [];
    this.notify();
  }

  /**
   * The screenshot button. The window hides while the OS snip tool owns the
   * screen and comes back on `popover:capture` — so there is deliberately
   * nothing to await here and no local "capturing" flag: the window is not on
   * screen to show one.
   */
  capturing() {
    void startCapture();
  }

  discardCapture() {
    // Straight to Rust, which owns it; the event is what clears it here.
    void discardCapture();
  }

  // --- the user's side ----------------------------------------------------

  async send(text: string) {
    if ((text === "" && !this.sendable) || this.busy) return;
    // Taken now: Rust consumes it as the turn is started, and the turn keeps it
    // for its own question card.
    const capture = this.capture;
    this.capture = null;
    this.captureCancelled = false;
    this.captureError = null;

    if (this.view && this.view.exchange_id && this.turns.length > 0) {
      this.turns = [...this.turns, this.#newTurn(text, capture)];
      this.notify();
      try {
        await followUp(this.view.exchange_id, text);
      } catch (error) {
        this.#failCurrent(error);
      }
      return;
    }

    this.turns = [this.#newTurn(text, capture)];
    this.notify();
    try {
      const exchangeId = await submitInput(text);
      if (this.view) {
        this.view = {
          ...this.view,
          phase: "running",
          exchange_id: exchangeId,
          input: text,
          capture: null,
          capture_cancelled: false,
          capture_error: null,
        };
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

  #newTurn(question: string, capture: Capture | null = null): Turn {
    this.#startWaiting();
    return {
      question,
      capture,
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

  /** Both callers sit inside `#forCurrent`, which publishes once the event has
   *  landed — publishing here too would re-render twice per first token. */
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
