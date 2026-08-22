// The preview's viewport: one Capture, fitted or zoomed (ADR-0017).
//
// Two states, and what differs is which box the image is in:
//
// - **fitted** (`scale === null`) is the layout ADR-0017 argues for — capped on
//   both axes, whole, and re-fitted for nothing when the window is resized
//   (ADR-0018), because the fitted state names no number to recompute;
// - **zoomed** is an explicit pixel size in a scrolling box. Fit is what "the
//   whole image" means, but a full-screen snip fitted into a 620px window is a
//   third of its real size, and the text in it is usually the thing being asked
//   about.
//
// The interface is the wheel, a click and a drag, with no control drawn for any
// of them: the layer is already a title bar, two arrows and a dot strip over the
// picture, and a zoom widget on top of that is more chrome than image. The cursor
// says which gesture is live, and the title bar reads back the percentage.
//
// Pointer-only, like the window's own resize grips (ADR-0018): a wheel and a drag
// have no keyboard equivalent, and the preview is opened by clicking a thumbnail
// in the first place. Esc still closes the whole layer, which is what makes an
// unfamiliar zoom safe to try.
import * as React from "react";
import { useT } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import type { Capture } from "@/lib/types";

/** How far past the image's own pixels a zoom goes. Beyond this a screenshot is
 *  interpolation rather than information. */
const MAX_SCALE = 4;
/** One wheel notch. */
const STEP = 1.15;
/** Movement that turns a press into a pan rather than a click, so releasing a
 *  drag neither toggles the zoom nor closes the preview. */
const DRAG_SLOP = 4;

interface CaptureZoomProps {
  capture: Capture;
  alt: string;
  /** `null` is fitted; a number is that many image pixels per CSS pixel. */
  scale: number | null;
  /** The setter itself, updater form included: a trackpad delivers several wheel
   *  notches inside one frame, and reading `scale` out of the render closure
   *  would spend them all on the same starting value. */
  onScale: React.Dispatch<React.SetStateAction<number | null>>;
  /** A click on the box itself rather than on the image closes the preview
   *  (ADR-0017); the image has its own meaning for a click. */
  onScrim: (event: React.MouseEvent) => void;
}

export function CaptureZoom({ capture, alt, scale, onScale, onScrim }: CaptureZoomProps) {
  const t = useT();
  const box = React.useRef<HTMLDivElement | null>(null);
  /** Where the view was centred before the scale changed, as a fraction of the
   *  image. Applied after the new size has been laid out. */
  const centre = React.useRef<{ x: number; y: number } | null>(null);
  /** The listener that eats the click a pan ends with, while it is armed. */
  const swallow = React.useRef<((event: MouseEvent) => void) | null>(null);

  /** A Capture whose dimensions did not survive normalisation (ADR-0016) has no
   *  fit to compute and no pixel size to state — a `width` of 0 would collapse
   *  it — so it stays fitted, which for the caps below means "as big as fits". */
  const zoomable = capture.width > 0 && capture.height > 0;

  /** The scale at which the image is exactly fitted — the floor every zoom is
   *  clamped to. Measured rather than remembered: the window is resizable
   *  (ADR-0018), so this changes without anything here being told. Measuring is
   *  also why the scrollbar is hidden rather than merely unwanted: a bar that
   *  took layout space would make the fit while zoomed differ from the fit the
   *  fitted state lays out.
   *
   *  Capped at 1: fitting is a way of making a big image whole, never a reason to
   *  enlarge a small one, and the fitted layout's `max-*` caps do not upscale
   *  either. */
  const fitScale = () => {
    const view = box.current;
    if (!view) return 1;
    return Math.min(1, view.clientWidth / capture.width, view.clientHeight / capture.height);
  };

  /** Remember the middle of the view before the scale changes, so a zoom grows
   *  around what is being looked at rather than around the top-left corner. */
  const rememberCentre = () => {
    const view = box.current;
    if (!view) return;
    centre.current = {
      x: (view.scrollLeft + view.clientWidth / 2) / view.scrollWidth,
      y: (view.scrollTop + view.clientHeight / 2) / view.scrollHeight,
    };
  };

  // Put that middle back once the new size is laid out but before it is painted,
  // so the image never appears at the new scale in the old scroll position.
  React.useLayoutEffect(() => {
    const view = box.current;
    const keep = centre.current;
    centre.current = null;
    if (!view || !keep) return;
    view.scrollLeft = keep.x * view.scrollWidth - view.clientWidth / 2;
    view.scrollTop = keep.y * view.scrollHeight - view.clientHeight / 2;
  }, [scale]);

  // A native listener, because React's own `onWheel` is registered passively at
  // the root and cannot cancel the scroll it would otherwise perform.
  React.useEffect(() => {
    const view = box.current;
    if (!view || !zoomable) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const fit = fitScale();
      const factor = event.deltaY < 0 ? STEP : 1 / STEP;
      rememberCentre();
      onScale((from) => {
        const to = (from ?? fit) * factor;
        // Snapping back to `null` rather than to the fit number keeps one
        // statement true: fitted is a state, not a coincidence of arithmetic.
        return to <= fit ? null : Math.min(to, MAX_SCALE);
      });
    };
    view.addEventListener("wheel", onWheel, { passive: false });
    return () => view.removeEventListener("wheel", onWheel);
    // `fitScale` and `rememberCentre` read the DOM and this `capture`, so the
    // listener is rebound when the Capture changes and not when the scale does.
  }, [onScale, zoomable, capture.width, capture.height]);

  const disarm = React.useCallback(() => {
    if (!swallow.current) return;
    window.removeEventListener("click", swallow.current, true);
    swallow.current = null;
  }, []);

  React.useEffect(() => disarm, [disarm]);

  /** After a pan, eat the click the release produces — the whole click, not just
   *  the image's own handler for it. A press that starts on the image often ends
   *  off it, and then the click is delivered to the nearest common ancestor,
   *  which is scrim: the drag would close the preview it was panning. Capture
   *  phase for the same reason, and a `once` listener is not enough — a gesture
   *  that produces no click at all would leave it armed for the next one. */
  const arm = () => {
    disarm();
    const eat = (event: MouseEvent) => {
      event.stopPropagation();
      disarm();
    };
    swallow.current = eat;
    window.addEventListener("click", eat, true);
  };

  const zoomed = scale !== null;

  const pan = (event: React.PointerEvent) => {
    const view = box.current;
    // A gesture that starts has not panned yet, zoomed or not: leaving a stale
    // swallow armed would eat the click that zooms back in.
    disarm();
    if (!view || !zoomed) return;
    let panned = false;
    const fromX = event.clientX;
    const fromY = event.clientY;
    const atLeft = view.scrollLeft;
    const atTop = view.scrollTop;
    const move = (moved: PointerEvent) => {
      const byX = moved.clientX - fromX;
      const byY = moved.clientY - fromY;
      if (Math.abs(byX) + Math.abs(byY) > DRAG_SLOP) panned = true;
      view.scrollLeft = atLeft - byX;
      view.scrollTop = atTop - byY;
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      if (panned) arm();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
  };

  return (
    // `overflow-auto` only while zoomed: at fit nothing overflows, and a box that
    // can scroll when it has nothing to scroll draws a gutter for no reason.
    //
    // The bar itself is hidden in both states — `scrollbar-width` for a current
    // engine, the `-webkit-` pseudo-element for an older WKWebView. Two reasons,
    // and only one of them is looks: a classic bar (which is what WebView2 draws)
    // takes layout space, so it would both shrink the picture the moment it is
    // zoomed and make `fitScale` measure a box narrower than the fitted one.
    // Panning is the way around a zoomed image anyway; the bar was never the
    // control.
    //
    // The image is centred with `m-auto` rather than by `justify-center`, which
    // is the one that survives overflowing: centring a child larger than its
    // scroll container pushes the overflow off the *start* edge, where no amount
    // of scrolling reaches it.
    <div
      ref={box}
      onClick={onScrim}
      className={cn(
        "flex h-full min-h-0 min-w-0 flex-1 scrollbar-none [&::-webkit-scrollbar]:hidden",
        zoomed ? "overflow-auto" : "overflow-hidden",
      )}
    >
      {/* Fitted, the caps are on both axes — a cap on the height alone fits a
          tall snip and lets a wide one overflow — and they keep the aspect ratio
          without an `object-fit`: the box is the image's own shape, scaled down
          until it fits. Zoomed, the size is stated outright and `max-w-none`
          retires the cap that would otherwise undo it. */}
      <img
        src={capture.data_url}
        alt={alt}
        title={zoomed ? t.popover.zoomOutHint : t.popover.zoomHint}
        draggable={false}
        width={scale === null ? undefined : Math.round(capture.width * scale)}
        height={scale === null ? undefined : Math.round(capture.height * scale)}
        onPointerDown={pan}
        onClick={() => {
          if (!zoomable) return;
          // Out of fit lands on the image's own pixels, which is the size its
          // text was rendered at. For an image already smaller than the box that
          // *is* the fit, so the step doubles instead of doing nothing.
          onScale(zoomed ? null : Math.max(1, fitScale() * 2));
        }}
        className={cn(
          "m-auto select-none",
          zoomed
            ? "max-w-none cursor-grab active:cursor-grabbing"
            : cn("max-h-full max-w-full", zoomable && "cursor-zoom-in"),
        )}
      />
    </div>
  );
}
