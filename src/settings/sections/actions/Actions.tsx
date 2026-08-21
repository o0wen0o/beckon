// One section, two views over the same list: every Action in the pane, and the
// one being edited. Authoring lives here rather than in the Launcher — the
// Launcher is summoned by a hotkey to pick something and get out of the way, and
// a form is the opposite of that.
//
// There is no Save button and there must never be one (ADR-0003); the write is
// scheduled by the store as the fields change.
import * as React from "react";
import {
  ArrowLeftIcon,
  ChevronRightIcon,
  PlusIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Kbd } from "@/components/Kbd";
import { Button } from "@/components/ui/button";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { SOURCE_ICON, sourceLabel } from "@/lib/inputSource";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";
import { ActionEditor } from "./ActionEditor";
import { RawFileEditor } from "./RawFileEditor";

/** The ledger row, shared by the Actions and the files that will not parse: the
 *  same four columns, so a broken file sits in the list rather than beside it. */
const ROW =
  "group hover:bg-accent focus-visible:ring-ring/50 flex w-full items-center gap-3.5 border-b px-2 py-2.5 text-left transition-colors duration-150 ease-out focus-visible:ring-[3px] focus-visible:outline-none motion-reduce:transition-none";

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
    const title = editing.kind === "raw" ? editing.file : store.draft?.name || editing.file;

    return (
      // The list and the editor are two views of one pane, and each mounts
      // fresh when the other leaves, so the entrance runs without a key.
      <div className="animate-in fade-in-0 slide-in-from-bottom-1 duration-200 ease-out motion-reduce:animate-none">
        <header className="mb-6.5 flex items-center gap-3">
          <Button
            variant="ghost"
            size="icon"
            aria-label="Back to Actions"
            className="flex-none"
            onClick={() => store.close()}
          >
            <ArrowLeftIcon className="size-3.5" />
          </Button>
          <div className="flex min-w-0 flex-col">
            <span className="font-display text-title font-semibold tracking-title">{title}</span>
            {/* The filename is the identity (ADR-0003): renaming never moves it.
                In the raw editor the heading *is* the filename, so repeating it
                here would print the same string twice, one line apart. */}
            <span className="text-muted-foreground truncate text-meta">
              {editing.kind === "raw" ? "does not parse — edited as text" : editing.file}
            </span>
          </div>
        </header>

        {/* Focus leaving the form is the last chance to write whatever was
            typed; the store then adopts the snapshot it held back. */}
        <div ref={form} onBlur={() => store.flush()}>
          {editing.kind === "raw" ? (
            <RawFileEditor />
          ) : store.selected ? (
            <ActionEditor action={store.selected} />
          ) : (
            <p className="text-muted-foreground text-meta">That Action is gone.</p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="animate-in fade-in-0 slide-in-from-bottom-1 duration-200 ease-out motion-reduce:animate-none">
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
              const SourceIcon = SOURCE_ICON[action.input_source];
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
                    {/* Two fixed columns, not a shrink-to-fit row: with the
                        hotkey chip optional, an ordinary flex row parks each
                        Input Source at a different x and the list reads as
                        ragged. */}
                    <span
                      title={`Input Source: ${sourceLabel(action.input_source)}`}
                      className="text-muted-quiet flex w-23 flex-none items-center gap-1.5 text-meta"
                    >
                      <SourceIcon className="size-3" />
                      {sourceLabel(action.input_source)}
                    </span>
                    <span className="flex w-28 flex-none justify-end">
                      {conflict ? (
                        // Outlined like the working hotkey chip beside it, in
                        // the danger colour: it is still the Action's hotkey,
                        // just an inactive one, and a solid red pill reads as a
                        // button.
                        <Badge
                          variant="outline"
                          title={conflict}
                          className="border-destructive/60 text-destructive gap-1 font-mono font-normal"
                        >
                          <TriangleAlertIcon className="size-3" /> {action.hotkey}
                        </Badge>
                      ) : action.hotkey ? (
                        <Kbd>{action.hotkey}</Kbd>
                      ) : null}
                    </span>
                    {/* The rows are the only way into the editor, and a name
                        over a description reads as a list of facts unless
                        something says it opens. */}
                    <span className="text-muted-quiet group-hover:text-foreground group-focus-visible:text-foreground flex flex-none transition-[transform,color] duration-150 ease-out group-hover:translate-x-0.5 group-focus-visible:translate-x-0.5 motion-reduce:transition-none">
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
                  <span className="w-23 flex-none" />
                  <span className="flex w-28 flex-none justify-end">
                    <Badge
                      variant="outline"
                      className="border-destructive/60 text-destructive gap-1 text-meta font-normal"
                    >
                      <TriangleAlertIcon className="size-3" /> Repair
                    </Badge>
                  </span>
                  <span className="text-muted-quiet group-hover:text-foreground flex flex-none transition-[transform,color] duration-150 ease-out group-hover:translate-x-0.5 group-focus-visible:translate-x-0.5 motion-reduce:transition-none">
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
