// One key chip for the three surfaces: the Direct Hotkey in Settings, the same
// hotkey in the Launcher's list, and the Launcher's footer legend.
import type * as React from "react";
import { cn } from "@/lib/utils";

export function Kbd({ className, ...props }: React.ComponentProps<"kbd">) {
  return (
    <kbd
      // No size of its own: `font-mono` is 0.92em in the base layer, which is
      // the optical match for the sans beside it. Pinning an absolute size
      // makes the chip read as two different objects across the surfaces.
      //
      // `font-medium` for the same reason it is *not* a size: 0.92em of a 12px
      // row is the smallest text on any surface, and at 400 the mono strokes go
      // thinner than the sans they sit beside. Weight travels with the chip so
      // it stays one object; a per-surface weight would be the drift the size
      // rule above already refuses.
      className={cn(
        "bg-muted text-muted-foreground rounded border px-1.5 py-0.5 font-mono font-medium",
        className,
      )}
      {...props}
    />
  );
}
