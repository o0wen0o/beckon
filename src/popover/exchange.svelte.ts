// The Popover's view of the Exchange on screen.
//
// The states are the point of this window: "waiting for the first token" must
// not look like "streaming", and an interrupted stream must keep the text it
// already produced (README). All of that is decided here, so the components
// only render turns — the state machine cannot be half-implemented in markup.
//
// Rust drives it with events, not return values, so this module is mostly a
// reducer over `exchange:*`. The three `on…` hooks are the DOM work a store has
// no business doing: scrolling the body, focusing the composer, resetting it.
//
// A module-level singleton, because there is exactly one Popover window and it
// is never destroyed (ADR-0007).
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

class ExchangeStore {
  view = $state<PopoverView | null>(null);
  turns = $state<Turn[]>([]);
  copiedTurn = $state<number | null>(null);

  /** Zero unless a turn is waiting for its first token. */
  #waitingSince = $state(0);
  #now = $state(0);

  /** DOM work the store must not do itself. Set by the component on mount. */
  onStream: () => void = () => {};
  onIdle: () => void = () => {};
  onReset: () => void = () => {};

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
    if (this.#waitingSince === 0) return 0;
    return Math.max(0, Math.floor((this.#now - this.#waitingSince) / 1000));
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
            this.onStream();
          }),
        ),
      )
      .add(
        onDone((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            turn.status = "done";
            this.#waitingSince = 0;
            this.onIdle();
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
   * that wait this would be a write per quarter second for the process's
   * lifetime — the Popover window is never destroyed (ADR-0007).
   */
  startClock() {
    const ticker = setInterval(() => {
      if (this.#waitingSince > 0) this.#now = Date.now();
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
    this.onReset();
    if (!this.view) {
      this.turns = [];
      return;
    }
    if (this.view.phase === "running") {
      this.turns = [this.#newTurn(this.view.input ?? "")];
    } else {
      this.turns = [];
      this.onIdle();
    }
  }

  // --- the user's side ----------------------------------------------------

  async send(text: string) {
    if (text === "" || this.busy) return;

    if (this.view && this.view.exchange_id && this.turns.length > 0) {
      this.turns = [...this.turns, this.#newTurn(text)];
      try {
        await followUp(this.view.exchange_id, text);
      } catch (error) {
        this.#failCurrent(error);
      }
      return;
    }

    this.turns = [this.#newTurn(text)];
    try {
      const exchangeId = await submitInput(text);
      if (this.view) {
        this.view = { ...this.view, phase: "running", exchange_id: exchangeId, input: text };
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
    this.#waitingSince = Date.now();
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
    }
    return true;
  }

  async copy(text: string, index: number) {
    // A user-requested clipboard write: not restored (ADR-0002).
    await copyToClipboard(text);
    this.copiedTurn = index;
    setTimeout(() => {
      if (this.copiedTurn === index) this.copiedTurn = null;
    }, 1600);
  }

  toggleReasoning(turn: Turn) {
    turn.reasoningTouched = true;
    turn.reasoningOpen = !turn.reasoningOpen;
  }

  // --- internals ----------------------------------------------------------

  #newTurn(question: string): Turn {
    this.#waitingSince = Date.now();
    this.#now = Date.now();
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
    if (turn) this.#applyFailure(turn, describeError(error));
  }
}

export const exchange = new ExchangeStore();
