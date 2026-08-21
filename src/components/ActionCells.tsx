// The two fixed columns an Action's row ends with, shared by the Launcher's list
// and the Actions list in Settings so the two cannot drift.
//
// The widths are fixed because the hotkey chip is optional: a shrink-to-fit row
// parks every Input Source at a different x. The inversion classes are
// unconditional — only the Launcher marks a row `aria-selected`.
import { TriangleAlertIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Kbd } from "@/components/Kbd";
import { SOURCE_ICON, sourceLabel } from "@/lib/inputSource";
import { formatAccelerator } from "@/lib/platform";
import type { InputSource } from "@/lib/types";

/** Outlined like the working chip beside it: still the Action's hotkey, just an
 *  inactive one. A solid red pill would read as a button. */
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
        // Mono, so it takes 0.92em from the base layer and stays level with
        // the registered chip rather than a step above it.
        <Badge variant="outline" title={conflict} className={`${DANGER_CHIP} font-mono`}>
          <TriangleAlertIcon className="size-3" /> {formatAccelerator(hotkey ?? "")}
        </Badge>
      ) : hotkey ? (
        <Kbd className="group-aria-selected:border-current/30 group-aria-selected:bg-transparent group-aria-selected:text-primary-foreground/70">
          {formatAccelerator(hotkey)}
        </Kbd>
      ) : null}
    </span>
  );
}

/** Stands in the hotkey column for a file that does not parse: it cannot be run,
 *  so the row is a way to the raw editor instead. Sans, so it needs a size. */
export function RepairCell() {
  return (
    <span className="flex w-28 flex-none justify-end">
      <Badge variant="outline" className={`${DANGER_CHIP} text-meta`}>
        <TriangleAlertIcon className="size-3" /> Repair
      </Badge>
    </span>
  );
}
