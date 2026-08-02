// Settings is an *editor of files*, not their owner (ADR-0003): every change
// commits to disk (debounced), and `config-changed` re-renders the form.
//
// It owns what is global — the credential, the Launcher hotkey, the theme, the
// model defaults. The Actions themselves are authored from the Launcher, which
// is where their list already lives; `src/launcher/actions.svelte.ts` is that
// half, and `src/lib/saveSlot.svelte.ts` is the write machinery both share.
//
// All of it lives here rather than in the components, for one reason: a save is
// echoed straight back at this window. `save_config` calls
// `reload::reload_config`, which emits `config-changed` to every window
// including this one — so the events being defended against are mostly our own
// writes, arriving mid-keystroke. Spreading that defence across a dozen field
// components is how one clobbering bug becomes twelve.
//
// A module-level singleton is right here specifically because there is exactly
// one Settings window and it is never destroyed (ADR-0007).
import {
  describeError,
  getConfig,
  getKeyStatus,
  getModels,
  getStartupErrors,
  saveConfig,
} from "../lib/ipc";
import { SaveSlot, textFocusHeld } from "../lib/saveSlot.svelte";
import type { Config, KeyStatus, ModelCatalog } from "../lib/types";

export type SectionRoute = "connection" | "triggering" | "appearance" | "defaults";

class SettingsStore {
  config = $state<Config | null>(null);
  keyStatus = $state<KeyStatus | null>(null);
  models = $state<ModelCatalog | null>(null);
  modelsLoading = $state(false);
  startupErrors = $state<string[]>([]);
  route = $state<SectionRoute>("connection");

  // Transient, and therefore reset on every open (ADR-0007).
  keyDraft = $state("");
  keyMessage = $state<string | null>(null);
  test = $state<{ state: "idle" | "running" | "ok" | "failed"; message?: string }>({
    state: "idle",
  });

  /** The pane element. Set by the shell; the suppression test needs it. */
  pane: HTMLElement | null = null;

  #deferredConfig: Config | null = null;
  #routedForKey = false;

  readonly configSlot = new SaveSlot(
    () => this.#release(),
    () => void this.#resyncConfig(),
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

  // --- adoption -----------------------------------------------------------

  adoptConfig(next: Config) {
    if (this.suppressed) {
      this.#deferredConfig = next;
      return;
    }
    this.config = next;
  }

  /** A snapshot refused while the user was typing is held, not dropped —
   *  otherwise an external edit leaves the form permanently stale. */
  #release() {
    if (this.suppressed || !this.#deferredConfig) return;
    this.config = this.#deferredConfig;
    this.#deferredConfig = null;
  }

  async #resyncConfig() {
    this.config = await getConfig();
  }

  // --- editing ------------------------------------------------------------

  /**
   * The only way config is ever changed. Mutates the live state so the field
   * updates, then snapshots a plain object for the write — a `$state` proxy
   * cannot cross the IPC boundary.
   */
  editConfig(mutate: (config: Config) => void, immediate = false) {
    if (!this.config) return;
    mutate(this.config);
    const next = $state.snapshot(this.config);
    this.configSlot.schedule(() => saveConfig(next), immediate);
  }

  // --- routing ------------------------------------------------------------

  go(section: SectionRoute) {
    // Leaving a pane must not strand an unwritten edit.
    this.flush();
    this.route = section;
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

    // Not awaited: this one can go to the network, and the rest of the form
    // must not wait on it. The dropdown renders from the current value until
    // the catalog lands.
    void this.refreshModels();
  }

  /** Startup errors are re-read on every reload: the tray clears them when the
   *  hotkeys come back, so a fixed conflict must stop being reported. */
  async refreshStartupErrors() {
    this.startupErrors = await getStartupErrors();
  }

  async refreshModels() {
    this.modelsLoading = true;
    try {
      // Populate from the documented catalog first. The live fetch is
      // deliberately unbounded (no HTTP timeout, by design), and a dropdown
      // holding nothing but its own current value while that is in flight is
      // the regression the fallback exists to prevent. A refresh keeps the
      // list already on screen instead of flashing back to the catalog.
      if (!this.models) this.models = await getModels(false);
      this.models = await getModels(true);
    } catch (error) {
      // The command is infallible by design; if it ever is not, keep whatever
      // list is already on screen rather than emptying the dropdowns.
      this.configSlot.error = describeError(error).message;
    } finally {
      this.modelsLoading = false;
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
  }
}

export const settings = new SettingsStore();
