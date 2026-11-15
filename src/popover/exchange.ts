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
// A module-level singleton: one Popover window, never destroyed.
import {
  cancelExchange,
  copyToClipboard,
  discardCapture,
  describeError,
  followUp,
  getPopoverView,
  insertAnswer,
  onDelta,
  onDone,
  onExchangeError,
  onFirstToken,
  onInterrupted,
  onPopoverCapture,
  onPopoverView,
  retryExchange,
  submitInput,
  Subscriptions,
} from "../lib/ipc";
import { Notifier } from "../lib/store";
import type { Capture, CaptureNotice, CapturePayload, Failure, PopoverView } from "../lib/types";

/**
 * Whether there is anything to send. A Capture is input on its own: the Action's
 * own prompt is the question being asked about it.
 *
 * A free function rather than a getter, because the draft lives in the composer
 * and the guard lives in the store — one rule, read from both sides.
 */
export const sendable = (text: string, captures: Capture[]) =>
  text !== "" || captures.length > 0;

/** What a set of Captures weighs, for the one line of prose that describes it.
 *  Here for the same reason as `sendable`: the rail and a sent turn's card both
 *  say it, and they must not round it differently. */
export const totalBytes = (captures: Capture[]) =>
  captures.reduce((sum, capture) => sum + capture.bytes, 0);

export type Status =
  | "waiting-first-token"
  | "streaming"
  | "done"
  | "interrupted"
  | "cancelled"
  | "error";

export interface Turn {
  question: string;
  /** The Captures that went with the question, in the order they were taken.
   *  Empty for a turn that carried only words. */
  captures: Capture[];
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

/** How long a button says what its gesture came to — Copied, or either
 *  refusal. One number, exported for the code block's own Copy
 *  (`Markdown.tsx`), because all of them are one behaviour at two sizes. */
export const REPORT_FOR = 1600;

/** How often the "waiting" counter reads the clock. */
const TICK = 250;

/**
 * How many events are held for an Exchange this window has not adopted yet.
 * What is being caught is the handful a turn emits before the window learns its
 * id — a refusal, a first token — so the ceiling is only there to bound what a
 * superseded stream can leave behind before the next adoption clears it.
 */
const EARLY_CAP = 64;

/** Failures the user can actually do something about in Settings. */
export function settlesInSettings(kind: string | undefined) {
  return kind === "no-credential" || kind === "read-error" || kind === "auth" || kind === "config";
}

/** A turn Rust is still working on. The three events that settle a turn apply
 *  only to one of these: Esc lands here first and the stream's own ending a
 *  moment later, and a turn the user stopped must not be relabelled "done" —
 *  or failed — by the event that lost the race. */
function isLive(status: Status) {
  return status === "waiting-first-token" || status === "streaming";
}

/** A turn that will not change again, so its answer can be offered for copying. */
export function isSettled(status: Status) {
  return status === "done" || status === "interrupted" || status === "cancelled";
}

/**
 * The standing notice a conversation carries when it has no turns yet. None of
 * them is an alarm, so the Popover renders each
 * of them as ordinary prose rather than as a `Callout`.
 */
export type Notice = "none" | "no-view" | "awaiting-input";

class ExchangeStore extends Notifier {
  view: PopoverView | null = null;
  turns: Turn[] = [];
  /**
   * Attached and not yet sent, oldest first. Rust owns them and hands
   * them over on `popover:capture`; this is that value, not a second opinion
   * about it.
   */
  captures: Capture[] = [];
  /** What the last snip had to say, if it had anything. Cleared by the next. */
  captureNotice: CaptureNotice | null = null;
  copiedTurn: number | null = null;
  /**
   * The turn whose Copy just failed, if one did.
   *
   * A second field rather than a tri-state on `copiedTurn`, because the two are
   * not the same shape: a success is one turn's checkmark and a failure is one
   * turn's warning, and a value that is either would have to be read as both
   * everywhere it is used. Copy is how a result leaves Beckon, so the
   * checkmark simply not appearing is not a report — that is indistinguishable
   * from a button that did nothing.
   */
  copyFailedTurn: number | null = null;
  /**
   * The turn whose Insert was refused, if one was.
   *
   * There is no `insertedTurn` beside it: an Insert that worked closed the
   * window, so there is nothing left to draw a checkmark on. Only the refusal
   * has a window to be reported in.
   */
  insertFailedTurn: number | null = null;
  /**
   * Bumped by every reveal. The window is reused, so the composer
   * still holds the last trigger's draft at the last trigger's height; keying
   * it on this remounts it, which clears both at once.
   */
  epoch = 0;
  /**
   * Bumped by every snip run that came back, landed or not. The window was
   * hidden while the snip tool owned the screen, so focus comes back to the
   * window rather than to the box inside it; this is the one signal that says
   * "a run finished", which a count of what is attached is not — a refusal and
   * a cancel both leave the count alone.
   */
  captureRun = 0;

  /** Zero unless a turn is waiting for its first token. */
  #waitingSince = 0;
  #waited = 0;

  /**
   * Events that arrived for an Exchange this window did not know yet, oldest
   * first, to be replayed when it learns the id.
   *
   * Rust starts the turn and *then* says the view changed (`open_action`), and
   * the window learns the id only when `getPopoverView` — or `submitInput` —
   * comes back. A turn that fails at once (no key stored, a provider row that
   * is gone, no model picked) emits its `exchange:error` inside that gap. It
   * used to be dropped as "not the Exchange on screen", and then `load` drew a
   * turn waiting for a first token that had already been refused: a counter
   * that climbs for ever over the one failure the user most needs to read,
   * because it is the one that says to open Settings.
   */
  #early: Array<{ exchangeId: string; apply: (turn: Turn) => void }> = [];

  /** Bumped by every `load`, so one overtaken by a newer trigger while it was
   *  waiting for the view can tell, and leave the adoption to that one. */
  #loads = 0;

  /**
   * Which set is being looked at full size, and where in it — never the images
   * themselves. `"composer"` is the pending tray, a number is that turn's own
   * set; the two are different sets and the arrows must not cross between them.
   *
   * Frontend-only, like `reasoningOpen`: nothing about looking at an image
   * changes what would be sent, so Rust has no opinion about it. Naming the set
   * rather than copying it is what keeps it from becoming a second opinion
   * about what is attached — the tray shrinking under an open
   * preview shortens it, and emptying closes it, with nothing to invalidate.
   */
  #preview: { source: "composer" | number; index: number } | null = null;

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

  /** Structural rather than a phase read: whether there is a view, and whether
   *  it has turns yet. No phase picks between notices here: an empty grab
   *  lands in the same notice typed input does — which is why no `PopoverPhase` is consulted below. */
  get notice(): Notice {
    if (this.view === null) return "no-view";
    if (this.turns.length > 0) return "none";
    return "awaiting-input";
  }

  /** Which of the two words the bar along the bottom says. `busy` decides
   *  whether the bar is there at all; this picks between "streaming" and
   *  "waiting", and the words themselves are the catalog's. */
  get streaming() {
    return this.current?.status === "streaming";
  }

  // --- lifecycle ----------------------------------------------------------

  /**
   * Wire the window up. The Popover outlives every trigger, so a
   * new one arrives as `popover:view`, not as a mount.
   */
  listen(subscriptions: Subscriptions) {
    subscriptions
      .add(onPopoverView(() => void this.load()))
      .add(
        onPopoverCapture((payload) => {
          this.#adoptCapture(payload);
          // The run is over whatever it produced, which is what the composer
          // needs to know to take focus back.
          this.captureRun += 1;
          this.notify();
        }),
      )
      .add(
        onFirstToken((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            // Only out of waiting. Esc lands here first and Rust's token a
            // moment later, and a cancelled turn marked streaming again has no
            // event coming to settle it — the Stop bar would stay up over it.
            if (turn.status === "waiting-first-token") this.#markStreaming(turn);
          }),
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
            if (!isLive(turn.status)) return;
            turn.status = "done";
            this.#waitingSince = 0;
          }),
        ),
      )
      .add(
        onInterrupted((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            if (!isLive(turn.status)) return;
            // Keep whatever was produced; mark it beneath (README).
            turn.status = "interrupted";
            turn.note = payload.message;
            this.#waitingSince = 0;
          }),
        ),
      )
      .add(
        onExchangeError((payload) =>
          this.#forCurrent(payload.exchange_id, (turn) => {
            if (isLive(turn.status)) this.#applyFailure(turn, payload);
          }),
        ),
      );
  }

  /**
   * Only the "waiting for the first token" counter reads the clock; the window
   * is never destroyed, so this would otherwise re-render four times
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
    const load = ++this.#loads;
    const view = await getPopoverView();
    // A newer trigger asked while this one was waiting. Its `load` reads the
    // same view and adopts it; doing it here as well would replay the early
    // events onto a turn that one is about to replace.
    if (load !== this.#loads) return;
    this.view = view;
    this.copiedTurn = null;
    this.copyFailedTurn = null;
    this.insertFailedTurn = null;
    this.epoch += 1;
    // A Capture belongs to the Popover that was showing, so a fresh trigger
    // starts with nothing attached — and Rust says so, rather than this
    // assuming it.
    this.#adoptCapture({
      captures: this.view?.captures ?? [],
      notice: this.view?.capture_notice ?? null,
    });
    // A trigger is a new conversation; nothing from the last one is still
    // being looked at.
    this.#preview = null;
    if (!this.view) {
      this.turns = [];
      this.#early = [];
      this.notify();
      return;
    }
    this.turns = this.view.phase === "running" ? [this.#newTurn(this.view.input ?? "")] : [];
    this.#adoptEarly();
    this.notify();
  }

  discardCapture(index: number) {
    // Straight to Rust, which owns the list; the event is what shortens it here,
    // so there is nothing local to notify about. An open preview follows it
    // down, because `preview` is derived from the list rather than a copy of it.
    void discardCapture(index);
  }

  // --- the preview --------------------------------------------------------

  /**
   * What is on screen full size, resolved from the set it names. The index is
   * clamped rather than trusted: the tray can shrink under an open preview, and
   * an empty set is no preview at all — which is how sending closes it.
   */
  get preview(): { items: Capture[]; index: number } | null {
    const at = this.#preview;
    if (!at) return null;
    const items =
      at.source === "composer" ? this.captures : (this.turns[at.source]?.captures ?? []);
    if (items.length === 0) return null;
    return { items, index: Math.min(at.index, items.length - 1) };
  }

  /** Look at one Capture full size. `source` is the set it belongs to — the
   *  pending tray, or the turn at that index — so the arrows walk that set and
   *  nothing else. */
  openPreview(source: "composer" | number, index: number) {
    this.#preview = { source, index };
    this.notify();
  }

  closePreview() {
    if (!this.#preview) return false;
    this.#preview = null;
    this.notify();
    return true;
  }

  /** Wraps, because a set of two with a "next" that stops is a dead button. */
  stepPreview(by: number) {
    // The clamped view of `#preview`, so a set that shrank under it steps from
    // where it is being drawn rather than from the index it was opened at.
    const at = this.preview;
    if (!at || !this.#preview) return;
    this.#preview.index = (at.index + by + at.items.length) % at.items.length;
    this.notify();
  }

  // --- the user's side ----------------------------------------------------

  async send(text: string) {
    if (!sendable(text, this.captures) || this.busy) return;
    // Taken now: Rust consumes them as the turn is started, and the turn keeps
    // them for its own question card.
    const captures = this.captures;
    // Emptying the tray closes a preview of it on its own: the turn's own card
    // is where those images live from here, and that is a different set.
    this.#adoptCapture({ captures: [], notice: null });

    if (this.view && this.view.exchange_id && this.turns.length > 0) {
      this.turns = [...this.turns, this.#newTurn(text, captures)];
      this.notify();
      try {
        await followUp(this.view.exchange_id, text);
      } catch (error) {
        this.#failCurrent(error);
      }
      return;
    }

    this.turns = [this.#newTurn(text, captures)];
    this.notify();
    try {
      const exchangeId = await submitInput(text);
      if (this.view) {
        // The two capture fields are deliberately left as they were: they are
        // only ever read by `load`, which replaces the whole view with Rust's
        // own, so nulling them here would be a write nothing reads.
        this.view = { ...this.view, phase: "running", exchange_id: exchangeId, input: text };
        // The turn was already running while the id was on its way back.
        this.#adoptEarly();
        this.notify();
      }
    } catch (error) {
      this.#failCurrent(error);
    }
  }

  /** Whether the last turn can be run again: there is an Exchange to run it
   *  against, and nothing is in flight. The button and the chord both ask. */
  get canRetry() {
    return this.view?.exchange_id != null && this.current !== null && !this.busy;
  }

  /**
   * Run the last turn again, as it stands. Two buttons end up here and they are
   * one operation: Retry on a turn that failed, Regenerate on one that settled.
   * Rust resends the same message and drops whatever came after it
   * (`retry_turn`), so the answer on screen is cleared to match.
   */
  async retry() {
    const current = this.current;
    // `busy` as well: the chord reaches this with no button to have hidden, and
    // a second request against a turn still streaming would interleave two
    // answers into one.
    if (!this.view?.exchange_id || !current || this.busy) return;
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
    // Both callers are `void exchange.copy(...)` — the button and the window's
    // own chord — so a rejection escaping here is swallowed entirely and the
    // export path fails in silence. Caught rather than thrown on: the
    // report belongs on the button that was pressed.
    try {
      // A user-requested clipboard write: not restored.
      await copyToClipboard(text);
    } catch {
      // The cause is not quoted. `write_clipboard_text` fails when another
      // process holds the clipboard, which is a sentence about Win32 rather
      // than about anything the reader can act on — and this label sits inside
      // a 12px button. Everywhere a cause *is* worth quoting there is a toast
      // or a field to quote it into; the Popover has neither (the
      // window is a conversation, not a form).
      this.copiedTurn = null;
      this.copyFailedTurn = index;
      this.notify();
      setTimeout(() => {
        if (this.copyFailedTurn !== index) return;
        this.copyFailedTurn = null;
        this.notify();
      }, REPORT_FOR);
      return;
    }
    this.copyFailedTurn = null;
    this.copiedTurn = index;
    this.notify();
    setTimeout(() => {
      if (this.copiedTurn !== index) return;
      this.copiedTurn = null;
      this.notify();
    }, REPORT_FOR);
  }

  /**
   * Paste an answer into the app Beckon was summoned over. Rust
   * copies it, closes this window and sends the paste shortcut; the view event
   * that follows is what empties the store, so success leaves nothing to do
   * here.
   *
   * A refusal — nothing to insert into, a window that has since closed, a
   * clipboard that would not take the text — comes back with the Popover still
   * up, and is reported on the button for the reason a failed Copy is.
   */
  async insert(text: string, index: number) {
    try {
      await insertAnswer(text);
    } catch {
      this.insertFailedTurn = index;
      this.notify();
      setTimeout(() => {
        if (this.insertFailedTurn !== index) return;
        this.insertFailedTurn = null;
        this.notify();
      }, REPORT_FOR);
    }
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

  /** Rust owns all three; they arrive together and are adopted
   *  together, from the event and from a fresh view alike. */
  #adoptCapture(payload: CapturePayload) {
    this.captures = payload.captures;
    this.captureNotice = payload.notice;
  }

  #newTurn(question: string, captures: Capture[] = []): Turn {
    this.#startWaiting();
    return {
      question,
      captures,
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

  /** Apply an event only if it belongs to the Exchange on screen — and hold it
   *  if it belongs to one this window may be about to adopt (`#early`). */
  #forCurrent(exchangeId: string, fn: (turn: Turn) => void) {
    if (!this.view || this.view.exchange_id !== exchangeId) {
      // Not known to be stale: an id this window has not seen is either a
      // superseded Exchange still draining or the next one arriving early, and
      // the two look the same from here. Adoption tells them apart.
      this.#early.push({ exchangeId, apply: fn });
      if (this.#early.length > EARLY_CAP) this.#early.shift();
      return;
    }
    const turn = this.current;
    if (!turn) return;
    fn(turn);
    this.notify();
  }

  /**
   * The window has just learned which Exchange it is showing: replay what that
   * Exchange already said, in order, and forget the rest. Everything else held
   * belonged to an Exchange that is gone, so it is dropped here
   * rather than kept against an id that will never be adopted.
   *
   * The caller publishes; this only applies.
   */
  #adoptEarly() {
    const held = this.#early;
    this.#early = [];
    const exchangeId = this.view?.exchange_id;
    const turn = this.current;
    if (!exchangeId || !turn) return;
    for (const event of held) {
      if (event.exchangeId === exchangeId) event.apply(turn);
    }
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
