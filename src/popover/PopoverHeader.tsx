// The Popover's title bar: which Action is loaded, which model it goes to, and
// the way out. It is also the drag region, so nothing in it may take focus
// except the button.
//
// What is *happening* is deliberately not here — a running turn reports itself
// in the ledger row it belongs to, and again in the bar along the bottom where
// Stop is. A status in the title bar would be a third place saying it.
import { XIcon } from "lucide-react";
import { BrandMark } from "@/components/BrandMark";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useT } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import type { ModelParams } from "@/lib/types";

/** The title bar's box, shared with the preview layer that covers it
 *  (ADR-0017): the preview's close button only lands on top of the one
 *  underneath it while the two bars are the same height and padding. */
export const TITLE_BAR = "flex h-8.5 flex-none items-center gap-2 border-b pr-1 pl-3";

interface PopoverHeaderProps {
  actionName: string;
  model: ModelParams | null;
  onClose: () => void;
}

export function PopoverHeader({ actionName, model, onClose }: PopoverHeaderProps) {
  const t = useT();

  return (
    <header
      data-tauri-drag-region
      className={cn(TITLE_BAR, "cursor-default select-none")}
    >
      <BrandMark className="text-brand size-3.5 flex-none" />
      {/* An Action's name, at the weight every other list gives it. The
          tracked uppercase eyebrow this replaced is the group-head register,
          and a group head is quieter than its contents — the wrong thing to
          say about the one line naming what the window is for. */}
      <span className="truncate font-medium">{actionName}</span>
      <span className="flex-1" />
      {model ? (
        <span className="text-muted-quiet font-mono text-meta">{model.model}</span>
      ) : null}
      {/* Thinking being on is a capability in use, not a condition to act on,
          so it is an outlined chip rather than a warning colour. */}
      {model?.thinking ? (
        <Badge variant="outline" className="text-muted-foreground text-meta font-normal">
          {t.popover.thinking}
        </Badge>
      ) : null}
      {/* `icon-xs`, not `icon-sm`: a 32px hit target in a 34px bar leaves one
          pixel above and below it, so the hover fill reads as a band across the
          whole title bar rather than as a button in it. */}
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={t.popover.close}
        title={t.popover.close}
        onClick={onClose}
      >
        <XIcon className="size-3.5" />
      </Button>
    </header>
  );
}
