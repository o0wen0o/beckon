// The pane's entrance, once. Keyed by the shell on whatever identifies the view
// on screen — the route, plus the file when an Action is open — so it re-runs on
// every view change and fires exactly once per change.
//
// Opacity and a 4px offset only: the content is already laid out, so nothing
// waits on it. The offset is *vertical* because the pane is `overflow-y-auto`,
// which per spec computes `overflow-x` to `auto` too, so a horizontal transform
// flashes a scrollbar.
import type * as React from "react";

export function PaneEnter({ children }: { children: React.ReactNode }) {
  return (
    <div className="animate-in fade-in-0 slide-in-from-bottom-1 duration-200 ease-out motion-reduce:animate-none">
      {children}
    </div>
  );
}
