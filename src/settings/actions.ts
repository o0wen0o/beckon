// Action authoring lives in Settings (ADR-0003), the Launcher is the picker and
// nothing more, so running one and authoring one do not share a gesture.
//
// Everything editable is held here rather than in the components: a save is
// echoed back at this window as `actions-changed`, and spreading that defence
// across a dozen fields turns one clobbering bug into twelve. The write
// machinery is `src/lib/saveSlot.ts`; the defaults an override inherits from
// and the model catalog are read off `store.ts` rather than fetched again.
//
// A module-level singleton: one Settings window, never destroyed (ADR-0007).
import {
  createAction,
  deleteAction as deleteActionFile,
  describeError,
  getActions,
  readActionRaw,
  saveAction,
  writeActionRaw,
} from "../lib/ipc";
import { SaveSlot, textFocusHeld } from "../lib/saveSlot";
import { Notifier } from "../lib/store";
import type { Action, ActionFile, RegistrySnapshot } from "../lib/types";

/**
 * Which screen of an Action's editor is open (ADR-0012). The four text fields
 * are one card that opens its own screen, so the editor is two views, not one
 * long form — and which one is showing is state like any other, not a `useState`
 * inside a component the shell re-keys.
 */
export type ActionScreen = "main" | "definition";

/** What the Actions section is showing instead of its list. */
export type Editing =
  /** An Action being edited as a form. `file` is its identity. */
  | { kind: "action"; file: string; screen: ActionScreen }
  /** A file that does not parse, being repaired as text. */
  | { kind: "raw"; file: string };

class ActionStore extends Notifier {
  snapshot: RegistrySnapshot = { actions: [], errors: [], hotkey_errors: {} };

  editing: Editing | null = null;

  /** The editable copy of the selected Action. Never read field values from
   *  the snapshot: that is what the draft exists to prevent. */
  draft: ActionFile | null = null;
  raw: { file: string; text: string; error?: string } | null = null;
  pendingDelete: Action | null = null;

  /** The editor element. Set by the component; the suppression test needs it. */
  form: HTMLElement | null = null;

  #deferred = false;

  readonly slot = new SaveSlot(
    () => this.#release(),
    () => void this.#resync(),
    () => this.notify(),
  );

  get suppressed() {
    return textFocusHeld(this.form) || this.slot.busy;
  }

  get selected(): Action | null {
    if (this.editing?.kind !== "action") return null;
    const file = this.editing.file;
    return this.snapshot.actions.find((action) => action.file_name === file) ?? null;
  }

  /** Something in this section needs attention: a file that does not parse, or
   *  a Direct Hotkey that could not be registered. */
  get flagged() {
    return this.snapshot.errors.length > 0 || Object.keys(this.snapshot.hotkey_errors).length > 0;
  }

  // --- adoption -----------------------------------------------------------

  /** The list itself is always safe: nothing types into it. */
  adoptActions(next: RegistrySnapshot) {
    this.snapshot = next;
    if (this.suppressed) {
      this.#deferred = true;
      this.notify();
      return;
    }
    this.syncDraft();
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
    const next = structuredClone(this.draft);
    this.notify();
    this.slot.schedule(() => saveAction(fileName, next), immediate);
  }

  /** The raw text of a file that does not parse. Written with a button, not on
   *  every keystroke — a file mid-edit almost never parses. */
  editRaw(text: string) {
    if (!this.raw) return;
    this.raw = { ...this.raw, text };
    this.notify();
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
      this.notify();
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
    this.notify();
  }

  // --- opening and closing ------------------------------------------------

  open(file: string) {
    this.#enterEditor({ kind: "action", file, screen: "main" });
    this.syncDraft();
  }

  /** Move between the editor's screens. The write is flushed first: a card that
   *  navigates has to end a pending edit exactly as leaving the section does. */
  showScreen(screen: ActionScreen) {
    if (this.editing?.kind !== "action" || this.editing.screen === screen) return;
    this.flush();
    this.editing = { ...this.editing, screen };
    this.notify();
  }

  async openRaw(file: string) {
    try {
      const text = await readActionRaw(file);
      this.#enterEditor({ kind: "raw", file });
      this.raw = { file, text };
      this.draft = null;
      this.notify();
    } catch (error) {
      this.slot.error = describeError(error).message;
    }
  }

  async create() {
    try {
      const fileName = await createAction("New Action");
      this.snapshot = await getActions();
      this.open(fileName);
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
      this.open(current.file);
    } catch (error) {
      this.raw = { ...current, error: describeError(error).message };
      this.notify();
    }
  }

  askDelete(action: Action | null) {
    this.pendingDelete = action;
    this.notify();
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
      this.notify();
    }
  }

  /** Back to the list. Whatever was typed is written first. Called on every
   *  open of the window too, so it stays free when there was nothing open. */
  close() {
    if (!this.editing) return;
    this.flush();
    this.editing = null;
    this.draft = null;
    this.raw = null;
    this.pendingDelete = null;
    this.notify();
  }

  #enterEditor(editing: Editing) {
    this.editing = editing;
    this.raw = null;
    this.slot.error = null;
    this.notify();
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
  }
}

export const actionStore = new ActionStore();
