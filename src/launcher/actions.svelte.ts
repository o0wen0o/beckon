// Action authoring lives with the Actions themselves: the Launcher is the list,
// so it is also where one is added, edited and deleted. Settings keeps what is
// global — the key, the hotkey, the theme, the model defaults.
//
// Everything editable is held here rather than in the components (ADR-0003):
// a save is echoed back at this window as `actions-changed`, and spreading the
// defence against that across a dozen field components is how one clobbering
// bug becomes twelve. `src/lib/saveSlot.svelte.ts` carries the shared half of
// that reasoning.
//
// A module-level singleton, because there is exactly one Launcher window and it
// is never destroyed (ADR-0007).
import {
  createAction,
  deleteAction as deleteActionFile,
  describeError,
  getActions,
  getConfig,
  getModels,
  readActionRaw,
  saveAction,
  setLauncherModal,
  writeActionRaw,
} from "../lib/ipc";
import { SaveSlot, textFocusHeld } from "../lib/saveSlot.svelte";
import type {
  Action,
  ActionFile,
  Config,
  ModelCatalog,
  RegistrySnapshot,
} from "../lib/types";

/** What the Launcher is showing instead of its list. */
export type Editing =
  /** An Action being edited as a form. `file` is its identity. */
  | { kind: "action"; file: string }
  /** A file that does not parse, being repaired as text. */
  | { kind: "raw"; file: string };

class ActionStore {
  snapshot = $state<RegistrySnapshot>({ actions: [], errors: [], hotkey_errors: {} });
  /** Read-only here: the editor needs the defaults an override inherits from. */
  config = $state<Config | null>(null);
  models = $state<ModelCatalog | null>(null);

  editing = $state<Editing | null>(null);

  /** The editable copy of the selected Action. Never read field values from
   *  the snapshot: that is what the draft exists to prevent. */
  draft = $state<ActionFile | null>(null);
  raw = $state<{ file: string; text: string; error?: string } | null>(null);
  pendingDelete = $state<Action | null>(null);

  /** The editor element. Set by the component; the suppression test needs it. */
  form: HTMLElement | null = null;

  #deferred = false;
  #modelsInFlight = false;

  readonly slot = new SaveSlot(
    () => this.#release(),
    () => void this.#resync(),
  );

  get suppressed() {
    return textFocusHeld(this.form) || this.slot.busy;
  }

  get selected(): Action | null {
    if (this.editing?.kind !== "action") return null;
    const file = this.editing.file;
    return this.snapshot.actions.find((action) => action.file_name === file) ?? null;
  }

  // --- adoption -----------------------------------------------------------

  /** The list itself is always safe: nothing types into it. */
  adoptActions(next: RegistrySnapshot) {
    this.snapshot = next;
    if (this.suppressed) {
      this.#deferred = true;
      return;
    }
    this.syncDraft();
  }

  adoptConfig(next: Config) {
    this.config = next;
  }

  /** A snapshot refused while the user was typing is held, not dropped —
   *  otherwise an external edit leaves the form permanently stale. */
  #release() {
    if (this.suppressed || !this.#deferred) return;
    this.#deferred = false;
    this.syncDraft();
  }

  async #resync() {
    this.snapshot = await getActions();
    this.syncDraft();
  }

  // --- editing ------------------------------------------------------------

  editDraft(mutate: (draft: ActionFile) => void, immediate = false) {
    if (!this.draft || this.editing?.kind !== "action") return;
    mutate(this.draft);
    // Identity is the filename, so that — not the display name — is what
    // decides which file this write lands in.
    const fileName = this.editing.file;
    const next = $state.snapshot(this.draft);
    this.slot.schedule(() => saveAction(fileName, next), immediate);
  }

  /**
   * Rebuild the draft from the snapshot. Called explicitly, never from an
   * effect on `snapshot`: an effect would re-derive it on every arrival,
   * including the echo of the user's own keystroke.
   */
  syncDraft() {
    const selected = this.selected;
    if (!selected) {
      this.draft = null;
      return;
    }
    this.draft = {
      name: selected.name,
      description: selected.description ?? null,
      input_source: selected.input_source,
      hotkey: selected.hotkey ?? null,
      prompt: { system: selected.prompt.system, user: selected.prompt.user ?? null },
      model: { ...selected.model },
    };
  }

  // --- opening and closing ------------------------------------------------

  async open(file: string) {
    this.#enterEditor({ kind: "action", file });
    this.syncDraft();
    void this.refreshModels();
  }

  async openRaw(file: string) {
    try {
      const text = await readActionRaw(file);
      this.#enterEditor({ kind: "raw", file });
      this.raw = { file, text };
      this.draft = null;
    } catch (error) {
      this.slot.error = describeError(error).message;
    }
  }

  async create() {
    try {
      const fileName = await createAction("New Action");
      this.snapshot = await getActions();
      void this.open(fileName);
    } catch (error) {
      this.slot.error = describeError(error).message;
    }
  }

  async saveRaw() {
    const current = this.raw;
    if (!current) return;
    try {
      await writeActionRaw(current.file, current.text);
      this.snapshot = await getActions();
      // It parses now, so it has become an Action again.
      void this.open(current.file);
    } catch (error) {
      this.raw = { ...current, error: describeError(error).message };
    }
  }

  async deleteAction(action: Action) {
    try {
      await deleteActionFile(action.file_name);
      this.snapshot = await getActions();
      this.close();
    } catch (error) {
      this.slot.error = describeError(error).message;
    } finally {
      this.pendingDelete = null;
    }
  }

  /** Back to the list. Whatever was typed is written first. Called on every
   *  summon as well, so it stays free when there was nothing open. */
  close() {
    if (!this.editing) return;
    this.flush();
    this.editing = null;
    this.draft = null;
    this.raw = null;
    this.pendingDelete = null;
    void setLauncherModal(false);
  }

  #enterEditor(editing: Editing) {
    this.editing = editing;
    this.raw = null;
    this.slot.error = null;
    // A Launcher dies with its focus; an editor must not. Rust holds the flag,
    // and clears it itself whenever the Launcher is hidden.
    void setLauncherModal(true);
  }

  // --- lifecycle ----------------------------------------------------------

  /** Focus left the form: whatever was being typed is finished. */
  flush() {
    this.slot.flush();
    // `focusout` fires before focus moves, so `activeElement` is still the
    // outgoing element here. Re-check once the browser has settled.
    queueMicrotask(() => this.#release());
  }

  async refresh() {
    this.adoptActions(await getActions());
    this.config = await getConfig();
  }

  /**
   * Asked for only once the editor is opened: this one can go to the network,
   * and the Launcher's job — show a list, pick a row — never needs it. The
   * dropdown renders from the current value until the catalog lands.
   *
   * A live list is kept; a fallback one is retried on the next open, because
   * the reason it fell back — no key yet, endpoint down — is usually the thing
   * the user has just gone and fixed.
   */
  async refreshModels() {
    if (this.models?.live || this.#modelsInFlight) return;
    this.#modelsInFlight = true;
    try {
      if (!this.models) this.models = await getModels(false);
      this.models = await getModels(true);
    } catch (error) {
      // The command is infallible by design; if it ever is not, keep whatever
      // list is already on screen rather than emptying the dropdown.
      this.slot.error = describeError(error).message;
    } finally {
      this.#modelsInFlight = false;
    }
  }
}

export const actions = new ActionStore();
