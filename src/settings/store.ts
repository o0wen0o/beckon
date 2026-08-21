// Settings is an *editor of files*, not their owner (ADR-0003): every change
// commits to disk (debounced), and `config-changed` re-renders the form.
//
// This store owns what is global — the credential, the Launcher hotkey, the
// theme, the model defaults. The Actions are edited in the same window by
// `./actions.ts`, which reads the defaults and the model catalog from here;
// `src/lib/saveSlot.ts` is the write machinery both share.
//
// The adoption rules live here rather than in the components because a save is
// echoed straight back at this window (`save_config` → `reload_config` →
// `config-changed`), so the events defended against are mostly our own writes
// arriving mid-keystroke. Spread across a dozen fields, one clobbering bug
// becomes twelve.
//
// A module-level singleton: one Settings window, never destroyed (ADR-0007).
import {
  describeError,
  getConfig,
  getKeyStatus,
  getModels,
  getStartupErrors,
  saveConfig,
} from "../lib/ipc";
import { SaveSlot, textFocusHeld } from "../lib/saveSlot";
import { Notifier } from "../lib/store";
import type { Config, KeyStatus, ModelCatalog } from "../lib/types";

export type SectionRoute = "connection" | "actions" | "triggering" | "appearance" | "defaults";

export interface TestState {
  state: "idle" | "running" | "ok" | "failed";
  message?: string;
}

class SettingsStore extends Notifier {
  config: Config | null = null;
  keyStatus: KeyStatus | null = null;
  models: ModelCatalog | null = null;
  modelsLoading = false;
  startupErrors: string[] = [];
  route: SectionRoute = "connection";

  // Transient, and therefore reset on every open (ADR-0007).
  keyDraft = "";
  keyMessage: string | null = null;
  test: TestState = { state: "idle" };

  /** The pane element. Set by the shell; the suppression test needs it. */
  pane: HTMLElement | null = null;

  #deferredConfig: Config | null = null;
  #routedForKey = false;

  readonly configSlot = new SaveSlot(
    () => this.#release(),
    () => void this.#resyncConfig(),
    () => this.notify(),
  );

  get suppressed() {
    return textFocusHeld(this.pane) || this.configSlot.busy;
  }

  get saveError() {
    return this.configSlot.error;
  }

  /** First run is "no key readable", never a file check (ADR-0005). */
  get firstRun() {
    return this.keyStatus !== null && this.keyStatus.kind !== "present";
  }

  // --- transient fields ---------------------------------------------------

  /** The typed API key. Held here rather than in the input so that
   *  `resetTransient` can clear it from outside the React tree. */
  setKeyDraft(value: string) {
    this.keyDraft = value;
    this.notify();
  }

  setKeyResult(status: KeyStatus | null, message: string | null) {
    if (status) this.keyStatus = status;
    this.keyMessage = message;
    this.notify();
  }

  setTest(test: TestState) {
    this.test = test;
    this.notify();
  }

  // --- adoption -----------------------------------------------------------

  adoptConfig(next: Config) {
    if (this.suppressed) {
      this.#deferredConfig = next;
      return;
    }
    this.config = next;
    this.notify();
  }

  /** A snapshot refused while the user was typing is held, not dropped —
   *  otherwise an external edit leaves the form permanently stale. */
  #release() {
    if (this.suppressed || !this.#deferredConfig) return;
    this.config = this.#deferredConfig;
    this.#deferredConfig = null;
    this.notify();
  }

  async #resyncConfig() {
    this.config = await getConfig();
    this.notify();
  }

  // --- editing ------------------------------------------------------------

  /**
   * The only way config is ever changed. Mutates the live object so the field
   * updates, then clones it for the write — the IPC boundary must not be handed
   * something that can still change under it.
   */
  editConfig(mutate: (config: Config) => void, immediate = false) {
    if (!this.config) return;
    mutate(this.config);
    const next = structuredClone(this.config);
    this.notify();
    this.configSlot.schedule(() => saveConfig(next), immediate);
  }

  // --- routing ------------------------------------------------------------

  go(section: SectionRoute) {
    // Leaving a pane must not strand an unwritten edit.
    this.flush();
    this.route = section;
    this.notify();
  }

  // --- lifecycle ----------------------------------------------------------

  /** Focus left the pane: whatever was being typed is finished. */
  flush() {
    this.configSlot.flush();
    // `focusout` fires before focus moves, so `activeElement` is still the
    // outgoing element here. Re-check once the browser has settled.
    queueMicrotask(() => this.#release());
  }

  async refreshAll() {
    // Through `adoptConfig`, not straight assignment: a refresh triggered by
    // reopening the window must obey the same suppression as an event.
    this.adoptConfig(await getConfig());
    this.keyStatus = await getKeyStatus();
    this.startupErrors = await getStartupErrors();

    if (!this.#routedForKey && this.firstRun) {
      // Only on the first open: yanking someone to Connection every time they
      // come to change the theme, because they have not stored a key, is rude.
      this.#routedForKey = true;
      this.route = "connection";
    }
    this.notify();

    // Not awaited: this one can go to the network, and the rest of the form
    // must not wait on it. The dropdown renders from the current value until
    // the catalog lands.
    void this.refreshModels();
  }

  /** Startup errors are re-read on every reload: the tray clears them when the
   *  hotkeys come back, so a fixed conflict must stop being reported. */
  async refreshStartupErrors() {
    this.startupErrors = await getStartupErrors();
    this.notify();
  }

  async refreshModels() {
    this.modelsLoading = true;
    this.notify();
    try {
      // Populate from the documented catalog first: the live fetch is
      // deliberately unbounded (no HTTP timeout, by design), and a dropdown
      // holding only its own current value meanwhile is the regression this
      // prevents. A refresh keeps the list already on screen.
      if (!this.models) {
        this.models = await getModels(false);
        this.notify();
      }
      this.models = await getModels(true);
    } catch (error) {
      // The command is infallible by design; if it ever is not, keep whatever
      // list is already on screen rather than emptying the dropdowns.
      this.configSlot.error = describeError(error).message;
    } finally {
      this.modelsLoading = false;
      this.notify();
    }
  }

  /**
   * The window is reused (ADR-0007), so a fresh open is an event and this is
   * where the last visit's leftovers go — including a typed-but-unsaved API
   * key sitting in a hidden window's DOM.
   */
  resetTransient() {
    this.keyDraft = "";
    this.keyMessage = null;
    this.test = { state: "idle" };
    this.configSlot.error = null;
    this.notify();
  }
}

export const settings = new SettingsStore();
