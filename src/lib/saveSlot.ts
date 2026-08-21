// The write half of "the filesystem is the source of truth" (ADR-0003), shared
// by the two stores that edit files: `settings/store.ts` edits `config.toml`,
// `settings/actions.ts` edits `actions\*.toml`.
//
// Both face the same hazard, which is why this is one module and not two: a
// save is echoed straight back at the window that caused it. `save_config` /
// `save_action` call into `reload`, which broadcasts `config-changed` /
// `actions-changed` to every window — so the snapshots being defended against
// are mostly our own writes arriving mid-keystroke, not the file watcher.
//
// Plain fields and an `onchange` callback rather than reactive state of its
// own: the surface that owns this is React now, and a store that reaches for a
// framework's reactivity primitive can only be used by that framework.

import { describeError } from "./ipc";

const SAVE_DEBOUNCE = 400;

export type Write = () => Promise<void>;

/**
 * Whether adopting a snapshot right now would fight the user.
 *
 * Read from the DOM at the instant the event arrives rather than tracked in
 * flags: flags get wired to the fields that existed when they were written, so
 * every field added afterwards silently opts out of the protection.
 *
 * `document.hasFocus()` matters on Windows: `activeElement` survives the window
 * losing OS focus, so without it an external edit made while Beckon is in the
 * background would never be adopted at all.
 *
 * Selects, checkboxes and buttons deliberately do not count — they commit
 * immediately, so a snapshot arriving under one of them is never a surprise.
 */
export function textFocusHeld(pane: HTMLElement | null): boolean {
  if (!pane || !document.hasFocus()) return false;
  const active = document.activeElement;
  if (!(active instanceof HTMLElement) || !pane.contains(active)) return false;
  return active.matches("textarea, input:not([type=checkbox]):not([type=radio])");
}

/**
 * One debounced write target. ADR-0003 makes disk authoritative, so every edit
 * has to land there — but not on every keystroke. `busy` is the second half of
 * the suppression rule: it stops a snapshot that was already in flight when the
 * user typed from being adopted back over the newer local value.
 */
export class SaveSlot {
  #busy = false;
  #error: string | null = null;

  #timer: ReturnType<typeof setTimeout> | undefined;
  #pending: Write | undefined;
  #inflight = 0;

  constructor(
    private readonly onsettle: () => void,
    /** Rust refused the write: the in-memory value is a lie until we re-read. */
    private readonly onreject: () => void,
    /** Fired whenever `busy` or `error` changes, so the owning store can
     *  re-render. Both are read straight off this object; only the fact that
     *  they moved is announced. */
    private readonly onchange: () => void = () => {},
  ) {}

  get busy() {
    return this.#busy;
  }

  get error() {
    return this.#error;
  }

  /** Settable: the stores use this line for failures of their own — a refused
   *  raw write, a model list that could not be fetched. */
  set error(message: string | null) {
    if (this.#error === message) return;
    this.#error = message;
    this.onchange();
  }

  #setBusy(busy: boolean) {
    if (this.#busy === busy) return;
    this.#busy = busy;
    this.onchange();
  }

  schedule(write: Write, immediate = false) {
    clearTimeout(this.#timer);
    this.#pending = write;
    this.#setBusy(true);
    if (immediate) void this.#run();
    else this.#timer = setTimeout(() => void this.#run(), SAVE_DEBOUNCE);
  }

  /** Write anything still pending. Called when focus leaves the pane. */
  flush() {
    if (this.#pending === undefined) return;
    clearTimeout(this.#timer);
    this.#timer = undefined;
    void this.#run();
  }

  async #run() {
    const write = this.#pending;
    this.#pending = undefined;
    this.#timer = undefined;
    if (!write) {
      this.#settle();
      return;
    }
    this.#inflight += 1;
    try {
      await write();
      this.error = null;
    } catch (failure) {
      // `save_config` refuses a Launcher hotkey it cannot register, and
      // `save_action` re-probes the Direct Hotkey and refuses the whole write.
      this.error = describeError(failure).message;
      this.onreject();
    } finally {
      this.#inflight -= 1;
      this.#settle();
    }
  }

  #settle() {
    if (this.#timer !== undefined || this.#inflight > 0 || this.#pending) return;
    this.#setBusy(false);
    this.onsettle();
  }
}
