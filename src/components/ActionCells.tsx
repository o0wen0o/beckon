// The two fixed columns an Action's row ends with, in one place. The Launcher's
// list and the Actions list in Settings are meant to read as the same list, and
// `lib/inputSource.ts` and `Kbd` already carry the icon, the word and the chip —
// but the columns *around* them were a second copy, which is where the drift
// landed: the same conflict chip written out at four call sites, two of them
// carrying a size class and two of them not.
//
// The widths are fixed rather than shrink-to-fit because the hotkey chip is
// optional: an ordinary flex row parks every Input Source at a different x and
// the list reads as ragged.
//
// The inversion classes are unconditional. Only the Launcher marks a row
// `aria-selected`, so in Settings they never match — which is what lets one
// column serve a picker whose current row is ink-filled and a pane whose rows
// are not.
import { TriangleAlertIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Kbd } from "@/components/Kbd";
import { SOURCE_ICON, sourceLabel } from "@/lib/inputSource";
import type { InputSource } from "@/lib/types";

/** Outlined like the working hotkey chip beside it, in the danger colour: it is
 *  still the Action's hotkey, just an inactive one, and a solid red pill reads
 *  as a button. */
const DANGER_CHIP =
  "border-destructive/60 text-destructive group-aria-selected:border-current group-aria-selected:text-primary-foreground gap-1 font-normal";

/** The Input Source column. Without a source it is the spacer that keeps a file
 *  that will not parse in the list rather than beside it. */
export function SourceCell({ source }: { source?: InputSource }) {
  if (!source) return <span className="w-23 flex-none" />;

  const Icon = SOURCE_ICON[source];
  return (
    <span
      title={`Input Source: ${sourceLabel(source)}`}
      className="text-muted-quiet group-aria-selected:text-primary-foreground/65 flex w-23 flex-none items-center gap-1.5 text-meta"
    >
      <Icon className="size-3" />
      {sourceLabel(source)}
    </span>
  );
}

/** The Direct Hotkey column: the chip, the same chip in the danger colour when
 *  the combination could not be registered, or nothing. */
export function HotkeyCell({
  hotkey,
  conflict,
}: {
  hotkey: string | null | undefined;
  /** Why the combination is not registered, if it is not. */
  conflict?: string;
}) {
  return (
    <span className="flex w-28 flex-none justify-end">
      {conflict ? (
        // Mono, so it carries no size of its own: `font-mono` is 0.92em in the
        // base layer, which is what keeps it level with the registered chip
        // rather than a step above it.
        <Badge variant="outline" title={conflict} className={`${DANGER_CHIP} font-mono`}>
          <TriangleAlertIcon className="size-3" /> {hotkey}
        </Badge>
      ) : hotkey ? (
        <Kbd className="group-aria-selected:border-current/30 group-aria-selected:bg-transparent group-aria-selected:text-primary-foreground/70">
          {hotkey}
        </Kbd>
      ) : null}
    </span>
  );
}

/** What stands in the hotkey column for a file that does not parse: it cannot be
 *  run, so the row is a way to the raw editor instead. Set in the sans, so it
 *  takes a size — the mono chips above it get theirs from the base layer. */
export function RepairCell() {
  return (
    <span className="flex w-28 flex-none justify-end">
      <Badge variant="outline" className={`${DANGER_CHIP} text-meta`}>
        <TriangleAlertIcon className="size-3" /> Repair
      </Badge>
    </span>
  );
}
