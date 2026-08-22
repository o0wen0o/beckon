// One Capture, full size, over the whole Popover (ADR-0017).
//
// Over the window rather than inside the scroller, because a 1920×1080 snip is
// not legible at any size a 620px column can give it — the window is all the
// screen Beckon owns, so the preview takes the window. It is a layer and not a
// route: the Exchange underneath is untouched, and closing puts the reader back
// where they were with the draft they were typing intact.
//
// Which set it is walking comes from the caller: the pending tray and a sent
// turn are different sets, and the arrows must not cross between them.
//
// The keys — Esc, ←, → — belong to the window, so they are `Popover.tsx`'s, in
// the one handler that already decides what Esc means. The *pointer* way out is
// here, because it is a fact about this layout: empty scrim closes it.
import * as React from "react";
import { ChevronLeftIcon, ChevronRightIcon, XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import type { Capture } from "@/lib/types";
import { CaptureZoom } from "./CaptureZoom";
import { TITLE_BAR } from "./PopoverHeader";

interface CapturePreviewProps {
  items: Capture[];
  index: number;
  onStep: (by: number) => void;
  onClose: () => void;
}

export function CapturePreview({ items, index, onStep, onClose }: CapturePreviewProps) {
  const t = useT();
  /**
   * How the Capture on screen is scaled — `null` is fitted, which is what every
   * image opens at (ADR-0017).
   *
   * Component state rather than the store's: a wheel notch is not something the
   * Exchange has an opinion about, and putting it in `ExchangeStore` would
   * publish to the whole window several times a second. It is reset by *stepping*
   * rather than by closing, because a new image is a new fit and the layer is
   * unmounted on close anyway.
   */
  const [scale, setScale] = React.useState<number | null>(null);
  React.useEffect(() => setScale(null), [index]);

  const capture = items[index];
  if (!capture) return null;
  const alone = items.length < 2;

  // Clicking the scrim closes it (ADR-0017): the way out that does not need the
  // keyboard, on the two thirds of the layer that are ground rather than
  // control.
  //
  // `target === currentTarget` rather than a `stopPropagation` on every control
  // inside: the boxes below name themselves as scrim, so a control added later
  // cannot silently become a second close button.
  const closeOnScrim = (event: React.MouseEvent) => {
    if (event.target === event.currentTarget) onClose();
  };

  return (
    // Grey, in both themes: a screenshot is mostly pale and has no border of
    // its own, so on the window's own background its edges are simply not
    // there. `--scrim` is that grey — a token, and this is its one consumer, so
    // the ratios it has to clear are written down in `globals.css` beside it
    // rather than reasoned about here.
    //
    // Still not quite opaque: a preview that hides the window entirely stops
    // reading as something you are inside and starts reading as somewhere you
    // went.
    <div className="bg-scrim/97 absolute inset-0 z-50 flex flex-col rounded-lg">
      {/* The title bar's own box, so the close button lands where the one
          underneath it was. Not scrim: it is the row that holds the close
          button, and a bar you can dismiss a window from by missing the button
          is a bar you cannot aim at. */}
      <div className={TITLE_BAR}>
        {/* Both lines sit one rung above where the same pair would sit on the
            window's own background, and keep the step between them: the quiet
            grey is 4.65:1 on `--background` and clears no grey ground at all,
            so on the scrim the value is `--foreground` and the note about it is
            `--muted-foreground` (14.68:1 and 4.96:1). */}
        <span className="tabular-nums text-meta">
          {index + 1} / {items.length}
        </span>
        <span className="text-muted-foreground truncate text-meta">
          {t.popover.captureMeta(capture.width, capture.height, capture.bytes)}
        </span>
        <span className="flex-1" />
        {/* Only while zoomed, and no percentage for the fitted state: "100%"
            beside an image that is not at 100% is worse than nothing, and a
            reading that is always there is a reading nobody reads. */}
        {scale === null ? null : (
          <span className="text-muted-foreground tabular-nums text-meta">
            {Math.round(scale * 100)}%
          </span>
        )}
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label={t.popover.closeCapture}
          title={t.popover.closeCapture}
          onClick={onClose}
        >
          <XIcon className="size-3.5" />
        </Button>
      </div>

      <div onClick={closeOnScrim} className="flex min-h-0 flex-1 items-center gap-1 px-2">
        {/* Both arrows stay mounted for a single Capture, disabled: dropping
            them would move the image sideways between one screenshot and two. */}
        <Button
          variant="ghost"
          size="icon"
          className="flex-none"
          aria-label={t.popover.previousCapture}
          disabled={alone}
          onClick={() => onStep(-1)}
        >
          <ChevronLeftIcon className="size-4" />
        </Button>
        {/* The image gets a box of its own, and the box is the flex item, so
            `min-w-0` can retire the automatic minimum width. Without it a
            replaced element takes one from its own aspect ratio: a 1920×1080
            snip in a 428px-tall row claims 761px of width, which is 141px wider
            than the whole window — the picture ran off the right-hand edge and
            took the next-Capture arrow with it.

            This is the one place a Capture is shown whole; the rail is where a
            set is cropped square to read as one. */}
        <CaptureZoom
          capture={capture}
          alt={t.popover.capturePosition(index + 1, items.length)}
          scale={scale}
          onScale={setScale}
          onScrim={closeOnScrim}
        />
        <Button
          variant="ghost"
          size="icon"
          className="flex-none"
          aria-label={t.popover.nextCapture}
          disabled={alone}
          onClick={() => onStep(1)}
        >
          <ChevronRightIcon className="size-4" />
        </Button>
      </div>

      {alone ? null : (
        // Dots rather than a filmstrip: the thumbnails are still on screen in
        // the rail or the card underneath, and a second strip of them here
        // would be the same set twice.
        <div
          onClick={closeOnScrim}
          className="flex h-9 flex-none items-center justify-center gap-1.5"
        >
          {items.map((_, at) => (
            <button
              key={at}
              type="button"
              // A dot is a step to a known place; `onStep`'s wrap is a no-op for
              // a delta that is already in range, so this needs no second way in.
              onClick={() => onStep(at - index)}
              aria-label={t.popover.capturePosition(at + 1, items.length)}
              aria-current={at === index}
              className={cn(
                "size-1.5 rounded-full",
                at === index
                  ? "bg-foreground"
                  : "bg-muted-foreground/40 hover:bg-muted-foreground",
              )}
            />
          ))}
        </div>
      )}
    </div>
  );
}
