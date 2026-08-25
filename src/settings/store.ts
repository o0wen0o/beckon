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
  getInputPermission,
  getKeyStatus,
  getKeyStatuses,
  getModels,
  getProviderPresets,
  getStartupErrors,
  saveConfig,
} from "../lib/ipc";
import { describeFailure } from "../lib/failures";
import { i18n } from "../lib/i18n";
import { keyProblem } from "../lib/providers";
import { SaveSlot, textFocusHeld } from "../lib/saveSlot";
import { Notifier } from "../lib/store";
import { toasts } from "../lib/toast";
import type { Config, InputPermission, KeyStatus, ModelCatalog, Provider } from "../lib/types";

export type SectionRoute = "connection" | "actions" | "triggering" | "appearance";

/** Why a model list is being read. See `SettingsStore.refreshModels`. */
export type RefreshMode = "prime" | "live" | "asked";

/** What the Test connection button is doing. The *outcome* is a toast
 *  (src/lib/toast.ts), so nothing but the button reads this: a sentence about a
 *  request that has finished has no field to sit under, and rendering it beside
 *  the button pushed the button off its own line. `ok`/`failed` are kept apart
 *  from `idle` all the same — a row already tested is not a row never tried. */
export type TestState = "idle" | "running" | "ok" | "failed";

class SettingsStore extends Notifier {
  config: Config | null = null;
  /**
   * One credential status per provider row (ADR-0021). A map, not a value: with
   * an endpoint per Action, one row can be missing its key while every other one
   * works, so there is no such thing as "the" key status.
   */
  keyStatuses: Record<string, KeyStatus> = {};
  /**
   * One model catalog per provider row, for the same reason: there is one
   * `base_url` and one key per row, so the models on offer are a different list
   * per endpoint. Keyed by `Provider.id`.
   */
  models: Record<string, ModelCatalog> = {};
  /** Which rows have a fetch in flight, so each Refresh answers for itself
   *  rather than for whichever row was asked last. */
  modelsLoading: ReadonlySet<string> = new Set();
  /** The rows "Add from preset" offers. Read once; it is a constant in Rust. */
  presets: Provider[] = [];
  /** Which endpoint's own screen is open, or `null` for the inventory. */
  editingProvider: string | null = null;
  startupErrors: string[] = [];
  /** `null` until the first answer, so nothing is claimed before it is known. */
  inputPermission: InputPermission | null = null;
  route: SectionRoute = "connection";

  // Transient, and therefore reset on every open (ADR-0007).
  keyDraft = "";
  keyMessage: string | null = null;
  /** Keyed by provider, so a failed test on one row does not colour another. */
  test: Record<string, TestState> = {};

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

  // --- the provider table (ADR-0021) --------------------------------------

  /** The row an Action that overrides nothing goes to. */
  get defaultProvider(): Provider | undefined {
    const config = this.config;
    if (!config) return undefined;
    return (
      config.api.providers.find((one) => one.id === config.defaults.provider) ??
      config.api.providers[0]
    );
  }

  /** One row by id, or the default for an Action that named none. */
  provider(id: string | null | undefined): Provider | undefined {
    if (id === null || id === undefined) return this.defaultProvider;
    return this.config?.api.providers.find((one) => one.id === id);
  }

  testFor(providerId: string): TestState {
    return this.test[providerId] ?? "idle";
  }

  /**
   * First run is "no key readable", never a file check (ADR-0005) — and asked of
   * the **default row only** (ADR-0021). An endpoint no Action has been pointed
   * at yet is not what makes this a first run, and a local one wants no key at
   * all, so nothing stored for one is a working setup rather than a fault.
   */
  get firstRun() {
    const row = this.defaultProvider;
    return row !== undefined && keyProblem(row, this.keyStatuses[row.id]) !== null;
  }

  // --- transient fields ---------------------------------------------------

  /** The typed API key. Held here rather than in the input so that
   *  `resetTransient` can clear it from outside the React tree. */
  setKeyDraft(value: string) {
    this.keyDraft = value;
    this.notify();
  }

  setKeyResult(providerId: string, status: KeyStatus | null, message: string | null) {
    if (status) this.keyStatuses = { ...this.keyStatuses, [providerId]: status };
    this.keyMessage = message;
    this.notify();
  }

  setTest(providerId: string, test: TestState) {
    this.test = { ...this.test, [providerId]: test };
    this.notify();
  }

  /** Open one endpoint's own screen, or go back to the inventory (ADR-0012). */
  editProvider(id: string | null) {
    // Leaving a screen must not strand an unwritten edit, exactly as changing
    // section does not.
    this.flush();
    this.editingProvider = id;
    this.keyDraft = "";
    this.keyMessage = null;
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
    // Four independent reads, so they go out together rather than paying four
    // round trips in series — this runs on every reveal of a reused window
    // (ADR-0007), while the pane is still showing the last visit's contents.
    const [config, keyStatuses, startupErrors, inputPermission, presets] = await Promise.all([
      getConfig(),
      getKeyStatuses(),
      getStartupErrors(),
      getInputPermission(),
      getProviderPresets(),
    ]);

    // Through `adoptConfig`, not straight assignment: a refresh triggered by
    // reopening the window must obey the same suppression as an event.
    this.adoptConfig(config);
    this.keyStatuses = keyStatuses;
    this.startupErrors = startupErrors;
    this.inputPermission = inputPermission;
    this.presets = presets;

    if (!this.#routedForKey && this.firstRun) {
      // Only on the first open: yanking someone to Connection every time they
      // come to change the theme, because they have not stored a key, is rude.
      this.#routedForKey = true;
      this.route = "connection";
    }
    this.notify();

    // Not awaited: these can go to the network, and the rest of the form must
    // not wait on them. Each dropdown renders from its own current value until
    // its own catalog lands.
    //
    // Primed offline for every row — which touches neither the network nor the
    // credential store — and asked for the *live* list only where one is about
    // to be read: the default row, which every Action with no override inherits.
    // The endpoint screen and the Action editor fetch their own. N deliberately
    // unbounded requests and N credential reads to fill dropdowns nobody has
    // opened is what opening this window used to cost.
    for (const provider of config.api.providers) {
      void this.refreshModels(
        provider.id,
        provider.id === config.defaults.provider ? "live" : "prime",
      );
    }
  }

  /** Re-read whenever this window comes back: the switch is thrown outside
   *  Beckon, in System Settings, and the user returns expecting it to know.
   *  Only notifies on a change — every alt-tab calls this, and the answer is
   *  almost always the one already held. */
  async refreshInputPermission() {
    const next = await getInputPermission();
    if (next === this.inputPermission) return;
    this.inputPermission = next;
    this.notify();
  }

  /** Startup errors are re-read on every reload: the tray clears them when the
   *  hotkeys come back, so a fixed conflict must stop being reported. */
  async refreshStartupErrors() {
    this.startupErrors = await getStartupErrors();
    this.notify();
  }

  /** One row's credential status: exactly what a key saved, removed, tested or
   *  a row re-added under an old id can have changed. Re-reading the whole map
   *  costs a credential-store round trip per configured endpoint to learn
   *  nothing about the other N. */
  async refreshKey(providerId: string) {
    this.keyStatuses = { ...this.keyStatuses, [providerId]: await getKeyStatus(providerId) };
    this.notify();
  }

  /**
   * One row's model list.
   *
   * One argument rather than two booleans, because only three of their four
   * combinations mean anything — announcing a fetch that never went to the
   * network is not a state:
   *
   *  - `prime` takes the offline answer only, touching neither the network nor
   *    the credential store, which is what lets the whole table be primed on
   *    reveal while only the row about to be read is actually asked.
   *  - `live` goes to the network, quietly.
   *  - `asked` is the same trip made on a gesture, so a fallback gets said out
   *    loud rather than leaving a person waiting on an answer.
   *
   * The offline answer is always taken first, because the live fetch is
   * deliberately unbounded (no HTTP timeout, by design) and a dropdown holding
   * only its own current value meanwhile is the regression this prevents. A
   * refresh keeps the list already on screen.
   */
  async refreshModels(providerId: string, mode: RefreshMode = "live") {
    const live = mode !== "prime";
    if (live) {
      this.modelsLoading = new Set(this.modelsLoading).add(providerId);
      this.notify();
    }
    try {
      if (!this.models[providerId]) {
        this.#putModels(providerId, await getModels(providerId, false), mode);
      }
      if (live) {
        const catalog = await getModels(providerId, true);
        this.#putModels(providerId, catalog, mode);
        // `get_models` never fails — it answers with the offline list and says
        // why (ADR-0024). Which is right for a fetch nobody asked for, and
        // silent for the one gesture that *was* a question: pressing Refresh
        // with no key stored left the same list on screen and no answer
        // anywhere. `asked` is how a caller says a person is waiting on it.
        //
        // Both arms, not just the failing one: a refresh that succeeds and
        // returns the list it already had also changes nothing on screen, so it
        // went unanswered for exactly the same reason. The count is what makes
        // an unchanged list an answer rather than a shrug.
        if (mode === "asked") {
          const strings = i18n.strings;
          if (catalog.source !== "live" && catalog.fallback) {
            toasts.show(
              "danger",
              describeFailure(
                catalog.fallback,
                strings,
                strings.settings.connection.listUnavailable,
              ),
            );
          } else if (catalog.source === "live") {
            toasts.show(
              "ok",
              strings.settings.connection.listRefreshed(catalog.options.length),
            );
          }
        }
      }
    } catch (error) {
      // The command is infallible by design; if it ever is not, say so as the
      // outcome of the gesture it was. Not `configSlot.error`: that is where a
      // failed *write* goes, and the status bar renders it as "Not saved" —
      // which is the wrong sentence about a list that could not be read.
      const strings = i18n.strings;
      toasts.show(
        "danger",
        describeFailure(describeError(error), strings, strings.settings.connection.listUnavailable),
      );
    } finally {
      if (live) {
        const loading = new Set(this.modelsLoading);
        loading.delete(providerId);
        this.modelsLoading = loading;
        this.notify();
      }
    }
  }

  #putModels(providerId: string, catalog: ModelCatalog, mode: RefreshMode) {
    this.models = { ...this.models, [providerId]: catalog };
    this.notify();
    this.#adoptOnlyModel(providerId, catalog, mode);
  }

  /**
   * One offer is not a choice.
   *
   * A row ships no model, so a local server with a single model loaded would
   * otherwise sit on "none chosen" until the first hotkey press failed — and
   * picking the only thing on the list cannot be the wrong pick. Two or more
   * stays the user's, because which model to spend money on is not a decision
   * to make on somebody's behalf.
   *
   * Here rather than on the endpoint screen, which is where it started: a rule
   * about a row must not depend on whether anyone opened that row's screen, and
   * this is the one place that knows a catalog just arrived — for any row, from
   * any caller.
   *
   * Said out loud, because nothing asked for it: this writes `config.toml` with
   * no gesture behind it, which makes it the one write that has to announce
   * itself rather than the one that may stay quiet. `prime` is the exception,
   * and only because of when it runs — `refreshAll` primes every row on reveal,
   * so a fresh install with several single-model local rows would open onto a
   * stack of them. The adoption still happens there; only the sentence waits for
   * a trip somebody was actually watching.
   */
  #adoptOnlyModel(providerId: string, catalog: ModelCatalog, mode: RefreshMode) {
    if (catalog.source === "none" || catalog.options.length !== 1) return;
    const row = this.config?.api.providers.find((one) => one.id === providerId);
    if (!row || row.model !== "") return;
    const model = catalog.options[0].id;
    this.editConfig((draft) => {
      const target = draft.api.providers.find((one) => one.id === providerId);
      if (target) target.model = model;
    }, true);
    if (mode !== "prime") {
      toasts.show("ok", i18n.strings.settings.connection.modelAdopted(model));
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
    this.test = {};
    this.editingProvider = null;
    this.configSlot.error = null;
    toasts.clear();
    this.notify();
  }
}

export const settings = new SettingsStore();
