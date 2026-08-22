// The Popover's eight resize grips (ADR-0018).
//
// The window is undecorated, so the OS draws no border to hit-test and no
// pointer gesture resizes it on its own: these strips are that border. Each one
// hands the press straight to the window manager, which owns the drag from
// there — nothing here follows the pointer or sets a size.
//
// Four pixels thick, and invisible: a window's edge is where a user already
// looks for this, and a visible frame drawn inside a frameless card would be a
// second edge beside the one the card already has. The cursor is the affordance.
//
// `aria-hidden`, and no keyboard equivalent: a drag has none, and eight nameless
// strips announced in a window driven by Esc and the arrows is worse than
// silence. The size a drag produces is remembered (ADR-0018), so this is a
// gesture performed once rather than on every summon.
import { startResizeDragging, type ResizeDirection } from "@/lib/ipc";

/** Edges before corners, and the corners last so they sit *over* the edges:
 *  the last 8px of an edge belongs to the corner it meets. */
const GRIPS: { direction: ResizeDirection; className: string }[] = [
  { direction: "North", className: "top-0 right-0 left-0 h-1 cursor-ns-resize" },
  { direction: "South", className: "right-0 bottom-0 left-0 h-1 cursor-ns-resize" },
  { direction: "West", className: "top-0 bottom-0 left-0 w-1 cursor-ew-resize" },
  { direction: "East", className: "top-0 right-0 bottom-0 w-1 cursor-ew-resize" },
  { direction: "NorthWest", className: "top-0 left-0 size-2 cursor-nwse-resize" },
  { direction: "NorthEast", className: "top-0 right-0 size-2 cursor-nesw-resize" },
  { direction: "SouthWest", className: "bottom-0 left-0 size-2 cursor-nesw-resize" },
  { direction: "SouthEast", className: "right-0 bottom-0 size-2 cursor-nwse-resize" },
];

export function ResizeGrips() {
  return (
    // Above the preview layer as well (ADR-0017 puts that at `z-50`): a window
    // is resizable while you are looking at a screenshot in it, which is when
    // the size matters most. `pointer-events-none` on the box so only the strips
    // themselves take the pointer — the rest of the card is still clickable
    // through it.
    <div aria-hidden className="pointer-events-none absolute inset-0 z-60">
      {GRIPS.map((grip) => (
        <div
          key={grip.direction}
          className={`pointer-events-auto absolute ${grip.className}`}
          // `onPointerDown`, not a click: the gesture is the press, and the
          // release happens inside the window manager's own drag loop.
          // `preventDefault` keeps the press off the text under it, which would
          // otherwise start a selection that never ends.
          onPointerDown={(event) => {
            event.preventDefault();
            void startResizeDragging(grip.direction);
          }}
        />
      ))}
    </div>
  );
}
