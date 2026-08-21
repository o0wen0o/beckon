// The pane's entrance, once. Keyed by the shell on whatever identifies the view
// on screen — the route, plus the file when an Action is open — so the animation
// re-runs on every view change and only fires once per change. Written out at
// each call site it was three copies of the recipe, and arriving at the Actions
// section ran two of them at the same time: the offsets added and the opacities
// multiplied, so the one section with an inner view swap entered differently
// from the other four.
//
// It animates opacity and a 4px offset only. The content is already laid out, so
// nothing waits on it — and the offset is *vertical* because the pane is
// `overflow-y-auto`, which per spec computes `overflow-x` to `auto` as well, so
// a horizontal transform flashes a scrollbar. `motion-reduce` stops it.
import type * as React from "react";

export function PaneEnter({ children }: { children: React.ReactNode }) {
  return (
    <div className="animate-in fade-in-0 slide-in-from-bottom-1 duration-200 ease-out motion-reduce:animate-none">
      {children}
    </div>
  );
}
