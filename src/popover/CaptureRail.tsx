// The Captures attached to the turn being composed, as one rail of equal square
// tiles (ADR-0017).
//
// Sideways rather than wrapped, and one line of prose about the *set* rather
// than a line per image: the composer must not grow taller as more is attached,
// or a fourth screenshot eats the conversation it is being asked about in a
// 500px window. What any one Capture actually is — its shape, its size — is one
// click away in the preview, which is the only place in this window where a
// full-screen snip is legible at all.
import * as React from "react";
import { XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT } from "@/lib/i18n";
import type { Capture } from "@/lib/types";
import { CaptureTile } from "./CaptureTile";
import { totalBytes } from "./exchange";

interface CaptureRailProps {
  captures: Capture[];
  onOpen: (index: number) => void;
  onRemove: (index: number) => void;
}

// Memoised: the draft lives in the composer, so every keystroke re-renders it
// while every one of these props stays the reference it was.
export const CaptureRail = React.memo(function CaptureRail({
  captures,
  onOpen,
  onRemove,
}: CaptureRailProps) {
  const t = useT();
  if (captures.length === 0) return null;

  return (
    <div className="flex flex-col gap-1.5">
      {/* The negative margin and the padding back are for the focus ring: an
          `overflow-x` box clips it, and the first tile's ring is on the edge. */}
      <div className="-mx-0.5 flex gap-1.5 overflow-x-auto px-0.5 pb-0.5">
        {captures.map((capture, index) => (
          // `group` per tile, so the remove button belongs to the tile it is on
          // rather than appearing on all of them at once.
          <div key={index} className="group relative flex-none">
            {/* `object-cover`: the tiles are a set and read as one only if they
                are the same shape, which means cropping rather than
                letterboxing four different aspect ratios. */}
            <CaptureTile
              capture={capture}
              onOpen={() => onOpen(index)}
              className="size-11"
              imageClassName="size-full object-cover"
            />
            {/* Kept out of the way until it is wanted, but never keyboard-only
                invisible: focus reveals it exactly as hover does. */}
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label={t.popover.removeCapture}
              title={t.popover.removeCapture}
              onClick={() => onRemove(index)}
              className="bg-background absolute -top-1 -right-1 size-4 rounded-full border opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
            >
              <XIcon className="size-2.5" />
            </Button>
          </div>
        ))}
      </div>
      <span className="text-muted-quiet text-meta">
        {t.popover.captureSet(captures.length, totalBytes(captures))}
      </span>
    </div>
  );
});
