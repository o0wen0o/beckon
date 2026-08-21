// One key chip for the three surfaces. An Action's Direct Hotkey in the
// Settings list, the same hotkey in the Launcher's list, and the key legend in
// the Launcher's footer are the same object at three sizes of importance, and
// the moment they are three class strings they drift.
import type * as React from "react";
import { cn } from "@/lib/utils";

export function Kbd({ className, ...props }: React.ComponentProps<"kbd">) {
  return (
    <kbd
      // No size of its own. `font-mono` is set at 0.92em in the base layer,
      // which is the optical match for the sans beside it — so a chip in the
      // footer's 11.5px legend and a chip in a 14px row each come out level
      // with their own neighbours. Pinning it to one absolute size is what
      // made the same object read as two different objects.
      className={cn(
        "bg-muted text-muted-foreground rounded border px-1.5 py-0.5 font-mono",
        className,
      )}
      {...props}
    />
  );
}
