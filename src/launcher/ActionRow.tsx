// One row of the Launcher's list, in the same four columns Settings' Actions
// list uses: name over description, Input Source, Direct Hotkey. The columns are
// fixed here so every Input Source parks at the same x in both lists.
//
// The row is a card, not a ruled row (ADR-0014); the fill still belongs to the
// keyboard cursor alone. The treatment is on `ROW` below.
//
// Hover does *not* select. The pointer once drove the cursor, which made the two
// the same row and left no ground for a hover state to paint on; a click still
// runs the row it landed on, which is what `onRun` already did.
import { HotkeyCell, RepairCell, SourceCell } from "@/components/ActionCells";
import { highlight } from "@/lib/fuzzy";
import type { Action } from "@/lib/types";

/** The card carries `--background` because the list under it does not: it sits on
 *  a `--muted` well (ADR-0014), which is what makes a frame at rest read as an
 *  object rather than as a rule around nothing.
 *
 *  So hover is the *edge*, not the ground — `--muted` is the well now and a card
 *  hovering to it would sink into the list instead of lifting off it. Same edge
 *  as Settings' `CARD_HOVER`, through the same token, and still one property
 *  moving; the guard has to be written out because Tailwind reads class names
 *  out of the source text, so `not-aria-selected:` cannot be composed onto it.
 *
 *  The selected card's frame goes to the fill's own colour: a filled row still
 *  wearing a lighter outline reads as two marks rather than one block. */
const ROW =
  "group flex h-13 w-full cursor-default items-center gap-3.5 rounded-md border bg-background px-4 text-left transition-colors not-aria-selected:hover:border-border-strong aria-selected:border-primary aria-selected:bg-primary aria-selected:text-primary-foreground motion-reduce:transition-none";

/** A description is body copy about the row; the Input Source beside it is a
 *  label about it, and carries the quieter grey from `SourceCell`. On the
 *  selected row both become strengths of the paper text — the muted greys are
 *  tuned against the background and read as smudges on the fill. */
const DESC = "text-muted-foreground group-aria-selected:text-primary-foreground/80";

interface ActionRowProps {
  action: Action;
  /** Why this Action's Direct Hotkey is not registered, if it is not. */
  conflict?: string;
  query: string;
  selected: boolean;
  onRun: () => void;
  ref?: React.Ref<HTMLLIElement>;
}

export function ActionRow({ action, conflict, query, selected, onRun, ref }: ActionRowProps) {
  return (
    // A listbox, not a menu: the query box keeps focus and the window drives the
    // list from the keyboard, so the mouse here is a convenience.
    <li ref={ref} role="option" aria-selected={selected} onClick={onRun} className={ROW}>
      <span className="flex min-w-0 flex-1 flex-col">
        {/* The row's name, at the weight the Actions list gives it — so a
            matched run is one step up from it rather than two. */}
        <span className="truncate font-medium">
          {highlight(action.name, query).map((run, at) => (
            // Keyed by position, not content: "aXaX" matched by "aa" yields two
            // identical runs, and a content key would collide.
            <span key={at} className={run.hit ? "font-semibold" : undefined}>
              {run.text}
            </span>
          ))}
        </span>
        {action.description ? (
          <span className={`truncate text-meta ${DESC}`}>{action.description}</span>
        ) : null}
      </span>

      <SourceCell source={action.input_source} />
      <HotkeyCell hotkey={action.hotkey} conflict={conflict} />
    </li>
  );
}

/** A file that does not parse is reported, never dropped (ADR-0003). It cannot
 *  be run, and the raw editor that repairs it is in Settings — so this row is a
 *  way there rather than a dead entry. */
export function BrokenRow({ file, message, onOpen }: BrokenRowProps) {
  return (
    <li role="option" aria-selected={false} onClick={onOpen} className={ROW}>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate font-mono font-medium">{file}</span>
        {/* The parse error itself, not a tooltip holding it: a `title` is
            invisible to everything except a resting mouse. */}
        <span className="text-destructive truncate text-meta">{message}</span>
      </span>
      <SourceCell />
      <RepairCell />
    </li>
  );
}

interface BrokenRowProps {
  file: string;
  message: string;
  onOpen: () => void;
}
