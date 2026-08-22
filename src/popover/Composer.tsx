// The follow-up box. It owns its own text and nothing else — the height is
// `field-sizing-content` with a floor and a ceiling, which is the browser doing
// what a resize handler used to. It is mounted only when there is something to
// type into, so a fresh mount is also the reset: the window is reused
// (ADR-0007) and a draft must not survive into the next trigger.
//
// No label. The turns above it are sided rather than labelled, so a label
// column here would be the only one in the window.
//
// The screenshot button lives here rather than in the title bar (ADR-0016): a
// Capture is something you attach to what you are about to send, so it belongs
// beside the box you are typing it in — and Send stays the one thing that sends.
import * as React from "react";
import { CameraIcon, SendIcon, TrashIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { describeFailure } from "@/lib/failures";
import { useT } from "@/lib/i18n";
import { COMMAND_MODIFIER, formatAccelerator } from "@/lib/platform";
import type { Capture, Failure } from "@/lib/types";
import { sendable } from "./exchange";

/** The Popover-local shortcut for a screenshot, drawn once in the platform's
 *  own spelling. Not a global hotkey: it only means anything with this window
 *  up, and `Popover.tsx` is what listens for the chord. */
const CAPTURE_ACCELERATOR = formatAccelerator(`${COMMAND_MODIFIER}+Shift+S`);

interface ComposerProps {
  placeholder: string;
  capture: Capture | null;
  /** The last snip produced nothing. */
  captureCancelled: boolean;
  /** A screenshot was taken and cannot be sent, by cause. */
  captureError: Failure | null;
  onSend: (text: string) => void;
  onCapture: () => void;
  onDiscardCapture: () => void;
}

export function Composer({
  placeholder,
  capture,
  captureCancelled,
  captureError,
  onSend,
  onCapture,
  onDiscardCapture,
}: ComposerProps) {
  const t = useT();
  const [draft, setDraft] = React.useState("");
  const box = React.useRef<HTMLTextAreaElement | null>(null);

  // The Popover is summoned by a hotkey; reaching for the mouse to click into
  // the one box on screen is the thing that would make it not worth summoning.
  React.useEffect(() => {
    box.current?.focus();
  }, []);

  // The window was hidden while the snip tool had the screen, so focus comes
  // back to the window rather than to the box inside it.
  React.useEffect(() => {
    if (capture) box.current?.focus();
  }, [capture]);

  const text = draft.trim();
  const canSend = sendable(text, capture);

  const send = () => {
    if (!canSend) return;
    setDraft("");
    onSend(text);
  };

  return (
    <div className="flex flex-none flex-col gap-2 border-t px-3.5 py-2.5">
      {/* What the last snip did, said next to the button that ran it rather
          than at the top of the scroller: the scroller holds the conversation
          and the standing notice for a Popover with nothing in it yet, and by
          the time a screenshot is taken the user is reading the bottom of it. */}
      {captureError ? (
        // A screenshot that cannot be sent is worth the marker the product
        // gives an alarm: something *was* captured and is being dropped.
        <p className="text-warning text-note">{describeFailure(captureError, t)}</p>
      ) : captureCancelled ? (
        // Not an alarm: nothing was captured, so nothing was sent.
        <p className="text-muted-quiet text-note">
          {t.popover.captureCancelled} {t.popover.captureRetry(CAPTURE_ACCELERATOR)}
        </p>
      ) : null}

      {/* The pending Capture sits above the box, not inside it: a thumbnail in
          the field would move the caret, and the field stays a text field. */}
      {capture ? (
        <div className="bg-muted flex items-center gap-2 self-start rounded-md border py-1 pr-1 pl-2">
          <img src={capture.data_url} alt="" className="h-8 w-12 rounded border object-cover" />
          <span className="text-muted-foreground text-quiet">
            {t.popover.captureMeta(capture.width, capture.height, capture.bytes)}
          </span>
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label={t.popover.removeCapture}
            title={t.popover.removeCapture}
            onClick={onDiscardCapture}
          >
            <TrashIcon className="size-3.5" />
          </Button>
        </div>
      ) : null}

      <div className="flex items-end gap-2">
        {/* Outlined, not filled: Send is the one filled control in the window,
            and two of them would make "attach" look like a second way out. */}
        <Button
          variant="outline"
          size="icon"
          className="flex-none"
          aria-label={t.popover.captureTooltip(CAPTURE_ACCELERATOR)}
          title={t.popover.captureTooltip(CAPTURE_ACCELERATOR)}
          onClick={onCapture}
        >
          <CameraIcon className="size-3.5" />
        </Button>
        <Textarea
          ref={box}
          rows={1}
          value={draft}
          placeholder={capture ? t.popover.captureNote : placeholder}
          aria-label={capture ? t.popover.captureNote : placeholder}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              send();
            }
          }}
          className="max-h-30 min-h-9 resize-none py-1.5"
        />
        {/* Full height, so it lines up with the empty box beside it rather than
            with the last line of a grown one — and a 14px glyph, because 16px
            against 14px text is the one size that reads as an icon set too big. */}
        <Button className="flex-none" disabled={!canSend} onClick={send}>
          <SendIcon className="size-3.5" /> {t.popover.send}
        </Button>
      </div>
    </div>
  );
}
