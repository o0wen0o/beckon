// One Capture as a clickable thumbnail (ADR-0017).
//
// The rail above the composer and a sent turn's card draw the same control at
// three sizes, and the affordance is the rule rather than the size: a bordered
// image whose edge strengthens on hover, labelled with what clicking does and
// tooltipped with what the image is. Sizing is the caller's — everything else
// is here, so the two surfaces cannot drift apart.
//
// `alt=""`: the button carries the label, and a screen reader reads that rather
// than anything inside it. The set's own prose sits under both surfaces.
import { useT } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import type { Capture } from "@/lib/types";

interface CaptureTileProps {
  capture: Capture;
  onOpen: () => void;
  /** The tile's box: its size, and its corner radius at that size. */
  className?: string;
  /** How the image sits in that box — `object-cover` for a set of tiles that
   *  must read as one, `object-contain` for a lone image shown whole. */
  imageClassName?: string;
}

export function CaptureTile({ capture, onOpen, className, imageClassName }: CaptureTileProps) {
  const t = useT();

  return (
    <button
      type="button"
      onClick={onOpen}
      aria-label={t.popover.viewCapture}
      title={t.popover.captureMeta(capture.width, capture.height, capture.bytes)}
      className={cn(
        "border-border hover:border-foreground/40 block overflow-hidden rounded-md border",
        className,
      )}
    >
      <img src={capture.data_url} alt="" className={imageClassName} />
    </button>
  );
}
