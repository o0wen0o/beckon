// One row of the Launcher's list, in the same four columns the Actions list in
// Settings uses: name over description, the Input Source, the Direct Hotkey.
// The two lists are meant to read as one list, so the columns are fixed here
// too — with the hotkey chip optional, an ordinary flex row parks every Input
// Source at a different x and the list reads as ragged.
//
// The selected row is ink-filled with paper text. That inversion is the only
// fill in the window, which is what makes "selected" unmistakable without a
// tint, a rail or a hue.
import { HotkeyCell, RepairCell, SourceCell } from "@/components/ActionCells";
import { highlight } from "@/lib/fuzzy";
import type { Action } from "@/lib/types";

const ROW =
  "group flex h-13 w-full cursor-default items-center gap-3.5 border-b px-4 text-left transition-colors duration-150 ease-out aria-selected:bg-primary aria-selected:text-primary-foreground motion-reduce:transition-none";

/** A description is body copy about the row; the Input Source column beside it
 *  is a label about it, and carries the quieter of the two greys from
 *  `SourceCell`. Inside the selected row both become strengths of the paper text
 *  — the muted greys are tuned against the background and read as smudges on
 *  the fill. */
const DESC = "text-muted-foreground group-aria-selected:text-primary-foreground/80";

interface ActionRowProps {
  action: Action;
  /** Why this Action's Direct Hotkey is not registered, if it is not. */
  conflict?: string;
  query: string;
  selected: boolean;
  onSelect: () => void;
  onRun: () => void;
  ref?: React.Ref<HTMLLIElement>;
}

export function ActionRow({
  action,
  conflict,
  query,
  selected,
  onSelect,
  onRun,
  ref,
}: ActionRowProps) {
  return (
    // A listbox, not a menu: the query box keeps focus and the window drives
    // the list from the keyboard, so the mouse here is a convenience.
    <li
      ref={ref}
      role="option"
      aria-selected={selected}
      onMouseMove={onSelect}
      onClick={onRun}
      className={ROW}
    >
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
