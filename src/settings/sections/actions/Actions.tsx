// One section, two views over the same list: every Action in the pane, and the
// one being edited. Authoring lives here rather than in the Launcher — the
// Launcher is summoned by a hotkey to pick something and get out of the way, and
// a form is the opposite of that.
//
// There is no Save button and there must never be one (ADR-0003); the write is
// scheduled by the store as the fields change.
import * as React from "react";
import { ArrowLeftIcon, ChevronRightIcon, PlusIcon } from "lucide-react";
import { HotkeyCell, RepairCell, SourceCell } from "@/components/ActionCells";
import { Button } from "@/components/ui/button";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";
import { ActionEditor } from "./ActionEditor";
import { ActionDefinition, DEFINITION_SCREEN } from "./ActionDefinition";
import { RawFileEditor } from "./RawFileEditor";

/** The ledger row, shared by the Actions and the files that will not parse: the
 *  same four columns, so a broken file sits in the list rather than beside it.
 *  The two columns it ends with are `ActionCells`, which the Launcher's list
 *  draws too. */
const ROW =
  "group hover:bg-accent focus-visible:ring-ring/50 flex w-full items-center gap-3.5 border-b px-2 py-2.5 text-left transition-colors duration-150 ease-out focus-visible:ring-[3px] focus-visible:outline-none motion-reduce:transition-none";

/** The rows are the only way into the editor, and a name over a description
 *  reads as a list of facts unless something says it opens. One string, because
 *  as two the error row lost the colour change and kept the nudge. */
const CHEVRON =
  "text-muted-quiet group-hover:text-foreground group-focus-visible:text-foreground flex flex-none transition-[transform,color] duration-150 ease-out group-hover:translate-x-0.5 group-focus-visible:translate-x-0.5 motion-reduce:transition-none";

export function Actions() {
  const store = useStore(actionStore);
  const form = React.useRef<HTMLDivElement | null>(null);

  // The store tests focus against this element to decide whether adopting an
  // incoming snapshot would fight the user.
  React.useEffect(() => {
    store.form = form.current;
    return () => {
      store.form = null;
    };
  }, [store, store.editing]);

  const snapshot = store.snapshot;
  const editing = store.editing;

  if (editing) {
    /** The display name, or the bare file name for one that does not parse. */
    const name = editing.kind === "raw" ? editing.file : store.draft?.name || editing.file;
    const onDefinition = editing.kind === "action" && editing.screen === "definition";
    // On the Definition screen the heading is the screen and the Action's name is
    // the line under it — otherwise the name is the heading and its filename,
    // which is the identity (ADR-0003), sits underneath.
    const title = onDefinition ? DEFINITION_SCREEN : name;
    const under = onDefinition
      ? name
      : editing.kind === "raw"
        ? "does not parse — edited as text"
        : editing.file;

    return (
      // The list and the editor are two views of one pane; the entrance belongs
      // to the shell, which keys `PaneEnter` on the open screen as well as the
      // file, so this swap animates once rather than twice.
      <div>
        {/* Outside the form element below, so that pressing it moves focus out
            of whatever was being typed — and `showScreen` flushes as well, for
            the keyboard path where focus was already elsewhere. */}
        <header className="mb-6.5 flex items-center gap-3">
          <Button
            variant="ghost"
            size="icon"
            aria-label={onDefinition ? `Back to ${name}` : "Back to Actions"}
            className="flex-none"
            onClick={() => (onDefinition ? store.showScreen("main") : store.close())}
          >
            <ArrowLeftIcon className="size-3.5" />
          </Button>
          <div className="flex min-w-0 flex-col">
            <span className="font-display text-title font-semibold tracking-title">{title}</span>
            <span className="text-muted-foreground truncate text-meta">{under}</span>
          </div>
        </header>

        {/* Focus leaving the form is the last chance to write whatever was
            typed; the store then adopts the snapshot it held back. */}
        <div ref={form} onBlur={() => store.flush()}>
          {editing.kind === "raw" ? (
            <RawFileEditor />
          ) : !store.selected ? (
            <p className="text-muted-foreground text-meta">That Action is gone.</p>
          ) : onDefinition ? (
            <ActionDefinition />
          ) : (
            <ActionEditor action={store.selected} />
          )}
        </div>
      </div>
    );
  }

  return (
    <div>
      <PaneHeader
        title="Actions"
        action={
          <Button onClick={() => void store.create()}>
            <PlusIcon className="size-3.5" /> New Action
          </Button>
        }
      >
        One Action is one prompt, stored as its own file. The filename is its identity; the name is
        only what you see.
      </PaneHeader>

      {snapshot.actions.length > 0 ? (
        <FieldGroup
          title={`${snapshot.actions.length} ${snapshot.actions.length === 1 ? "Action" : "Actions"}`}
        >
          <ul className="flex list-none flex-col p-0">
            {snapshot.actions.map((action) => {
              const conflict = snapshot.hotkey_errors[action.id];
              return (
                <li key={action.id}>
                  <button
                    type="button"
                    onClick={() => store.open(action.file_name)}
                    className={ROW}
                  >
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate font-medium">{action.name || action.file_name}</span>
                      <span className="text-muted-foreground truncate text-meta">
                        {/* A description if there is one, the filename if not:
                            the second line is where the row's identity goes,
                            and an empty one leaves the name floating. */}
                        {action.description || (
                          <span className="font-mono">{action.file_name}</span>
                        )}
                      </span>
                    </span>
                    <SourceCell source={action.input_source} />
                    <HotkeyCell hotkey={action.hotkey} conflict={conflict} />
                    <span className={CHEVRON}>
                      <ChevronRightIcon className="size-4" />
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </FieldGroup>
      ) : null}

      {/* A file that does not parse is reported, never dropped (ADR-0003), and
          the way back is the raw editor this row opens. */}
      {snapshot.errors.length > 0 ? (
        <FieldGroup title="Will not parse">
          <ul className="flex list-none flex-col p-0">
            {snapshot.errors.map((error) => (
              <li key={error.file_name}>
                <button
                  type="button"
                  onClick={() => void store.openRaw(error.file_name)}
                  className={ROW}
                >
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="font-mono truncate font-medium">{error.file_name}</span>
                    {/* The parse error itself, not a tooltip holding it: this
                        row is the only report that the file exists, and a
                        `title` is invisible to everything except a resting
                        mouse. */}
                    <span className="text-destructive truncate text-meta">
                      {error.message}
                    </span>
                  </span>
                  <SourceCell />
                  <RepairCell />
                  <span className={CHEVRON}>
                    <ChevronRightIcon className="size-4" />
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </FieldGroup>
      ) : null}

      {snapshot.actions.length === 0 && snapshot.errors.length === 0 ? (
        <p className="text-muted-foreground py-6">
          No Actions yet. One Action is one prompt, stored as its own file.
        </p>
      ) : null}
    </div>
  );
}
