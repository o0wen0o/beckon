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
  SparklesIcon,
  TextCursorInputIcon,
  TextSelectIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { InputSource } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";
import { ActionEditor } from "./ActionEditor";
import { RawFileEditor } from "./RawFileEditor";

const SOURCE_ICON = {
  selection: TextSelectIcon,
  prompt: TextCursorInputIcon,
  auto: SparklesIcon,
};

/** Title case for display; the value itself stays the CONTEXT.md term. */
function sourceLabel(source: InputSource) {
  return source.charAt(0).toUpperCase() + source.slice(1);
}

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
      <>
        <header className="mb-6 flex items-center gap-3">
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
            <span className="font-display text-xl font-semibold">{title}</span>
            {/* The filename is the identity (ADR-0003): renaming never moves it.
                In the raw editor the heading *is* the filename, so repeating it
                here would print the same string twice, one line apart. */}
            <span className="text-muted-foreground font-small truncate text-2xs">
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
            <p className="text-muted-foreground font-small text-xs">That Action is gone.</p>
          )}
        </div>
      </>
    );
  }

  return (
    <>
      <header className="flex items-baseline justify-between gap-3">
        <h1 className="font-display mb-6 text-xl font-semibold">Actions</h1>
        <Button onClick={() => void store.create()}>
          <PlusIcon className="size-3.5" /> New Action
        </Button>
      </header>

      <ul className="flex list-none flex-col gap-0.5 p-0">
        {snapshot.actions.map((action) => {
          const conflict = snapshot.hotkey_errors[action.id];
          const SourceIcon = SOURCE_ICON[action.input_source];
          return (
            <li key={action.id}>
              <button
                type="button"
                onClick={() => store.open(action.file_name)}
                className="group hover:bg-accent focus-visible:ring-ring/50 flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors focus-visible:ring-[3px] focus-visible:outline-none"
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate">{action.name || action.file_name}</span>
                  {action.description ? (
                    <span className="text-muted-foreground font-small truncate text-xs">
                      {action.description}
                    </span>
                  ) : null}
                </span>
                {/* Two fixed slots, not a shrink-to-fit row: with the hotkey
                    chip optional, an ordinary flex row parks each Input Source
                    pill at a different x and the column reads as ragged. */}
                <span className="flex items-center gap-2">
                  {/* An outline pill drew a box around a word. This is a
                      property of the Action, not something to press, so it
                      reads as the label it is. */}
                  <span
                    title={`Input Source: ${sourceLabel(action.input_source)}`}
                    className="text-muted-foreground font-small flex items-center gap-1 text-2xs"
                  >
                    <SourceIcon className="size-3" />
                    {sourceLabel(action.input_source)}
                  </span>
                  {/* Reserved whether or not this Action has a hotkey, so the
                      pills line up. */}
                  <span className="flex min-w-24 flex-none justify-end">
                    {conflict ? (
                      // Outlined like the working hotkey chip beside it, in the
                      // danger colour: it is still the Action's hotkey, just an
                      // inactive one, and a solid red pill reads as a button.
                      <Badge
                        variant="outline"
                        title={conflict}
                        className="border-destructive/50 text-destructive gap-1 font-normal"
                      >
                        <TriangleAlertIcon className="size-3" /> {action.hotkey}
                      </Badge>
                    ) : action.hotkey ? (
                      <kbd className="bg-muted text-muted-foreground font-mono rounded border px-1.5 py-0.5 text-2xs">
                        {action.hotkey}
                      </kbd>
                    ) : null}
                  </span>
                </span>
                {/* The rows are the only way into the editor, and a name over a
                    description reads as a list of facts unless something says it
                    opens. */}
                <span className="text-muted-foreground group-hover:text-primary group-focus-visible:text-primary flex flex-none transition-all group-hover:translate-x-0.5">
                  <ChevronRightIcon className="size-4" />
                </span>
              </button>
            </li>
          );
        })}

        {/* A file that does not parse is reported, never dropped (ADR-0003), and
            the way back is the raw editor this row opens. */}
        {snapshot.errors.map((error) => (
          <li key={error.file_name}>
            <button
              type="button"
              onClick={() => void store.openRaw(error.file_name)}
              className="group hover:bg-accent focus-visible:ring-ring/50 flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors focus-visible:ring-[3px] focus-visible:outline-none"
            >
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="font-mono truncate text-xs">{error.file_name}</span>
                {/* The parse error itself, not a tooltip holding it: this row is
                    the only report that the file exists, and a `title` is
                    invisible to everything except a resting mouse. */}
                <span className="text-destructive font-small truncate text-xs">{error.message}</span>
              </span>
              <span className="flex items-center gap-2">
                <Badge
                  variant="outline"
                  className="border-destructive/50 text-destructive gap-1 font-normal"
                >
                  <TriangleAlertIcon className="size-3" /> Repair
                </Badge>
              </span>
              <span className="text-muted-foreground group-hover:text-primary flex flex-none transition-all group-hover:translate-x-0.5">
                <ChevronRightIcon className="size-4" />
              </span>
            </button>
          </li>
        ))}

        {snapshot.actions.length === 0 && snapshot.errors.length === 0 ? (
          <li className="text-muted-foreground py-6">
            <p className="m-0">No Actions yet. One Action is one prompt, stored as its own file.</p>
          </li>
        ) : null}
      </ul>
    </>
  );
}
