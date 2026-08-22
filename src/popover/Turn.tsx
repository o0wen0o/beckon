// One turn, sided: what you asked is a card on the right, what came back runs
// left and bare. The sides are what say who spoke, which is why there is no
// label column here and no hairline between turns — the gap separates them.
//
// Every state a turn can be in is rendered here, so the ones that must not look
// alike sit side by side in one file. Output is plain text with preserved
// whitespace: acceptable for the MVP, and it cannot inject anything into the
// WebView.
import { CheckIcon, CopyIcon, RotateCcwIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { describeFailure } from "@/lib/failures";
import { useT } from "@/lib/i18n";
import { showSettings } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { CaptureTile } from "./CaptureTile";
import { exchange, isSettled, settlesInSettings, totalBytes, type Turn } from "./exchange";

/** Long enough that the clamp is doing something, i.e. worth offering to undo. */
const CLAMP_AT = 160;

/**
 * Capped well short of the window: a bubble reaching both edges stops reading as
 * one side of a conversation. The fill is `--muted`, the quietest there is —
 * inversion means "current" everywhere else, and spending it here would make
 * your own words the loudest thing on screen.
 */
const CARD = "bg-muted max-w-4/5 rounded-lg border px-3 py-1.5 whitespace-pre-wrap text-quiet";

/**
 * The three quiet buttons under a turn — reasoning disclosure, clamp toggle,
 * Copy — one step quieter than `ghost`'s grey, because they are labels *about*
 * the turn rather than part of it. The negative margin pulls each flush with
 * the text it belongs to; hover still takes them to full ink.
 */
const QUIET = "text-muted-quiet";

export function TurnView({ turn, index }: { turn: Turn; index: number }) {
  const t = useT();
  const captures = turn.captures;
  /** More than one is a grid of cropped tiles with one line of prose about the
   *  set; exactly one is the image itself, described by its own dimensions. */
  const many = captures.length > 1;

  /** The cause named first, then the provider's own words — the same sentence
   *  Settings builds, rather than a raw reqwest chain. */
  const failure =
    turn.status === "error"
      ? describeFailure({ kind: turn.errorKind ?? "error", message: turn.note ?? "" }, t)
      : null;

  const settled = isSettled(turn.status);
  const copied = exchange.copiedTurn === index;

  return (
    <div className="flex flex-col gap-2">
      {turn.question || captures.length > 0 ? (
        // The toggle sits under the card rather than inside it: `ghost` hovers
        // to `--accent`, which is the same grey the card is filled with.
        <div className="flex flex-col items-end gap-1">
          {/* The Captures go in the same card the words do, above them
              (ADR-0016): what you sent is one object, and the images are the
              larger half of it. `line-clamp` cannot be on the card once there
              is an image in it — it would clip the picture rather than the
              prose — so the clamp moves onto the text. */}
          <div className={cn(CARD, "flex flex-col gap-1.5")}>
            {captures.length > 0 ? (
              <>
                {/* Two-up once there is more than one, and cropped to a common
                    height: a column of whole images makes a two-screenshot
                    question taller than the answer to it, which inverts which
                    side of the turn reads as the subject. Whole is what the
                    preview is for (ADR-0017). */}
                <div className={many ? "grid grid-cols-2 gap-1.5" : "contents"}>
                  {captures.map((capture, at) => (
                    <CaptureTile
                      key={at}
                      capture={capture}
                      onOpen={() => exchange.openPreview(index, at)}
                      imageClassName={
                        many ? "h-24 w-full object-cover" : "max-h-40 w-full object-contain"
                      }
                    />
                  ))}
                </div>
                <span className={cn(QUIET, "text-meta")}>
                  {many
                    ? t.popover.captureSet(captures.length, totalBytes(captures))
                    : t.popover.captureMeta(
                        captures[0].width,
                        captures[0].height,
                        captures[0].bytes,
                      )}
                </span>
              </>
            ) : null}
            {turn.question ? (
              <span className={turn.questionExpanded ? undefined : "line-clamp-2"}>
                {turn.question}
              </span>
            ) : null}
          </div>
          {turn.question.length > CLAMP_AT ? (
            <Button
              variant="ghost"
              size="xs"
              className={cn(QUIET, "-mr-1.5")}
              onClick={() => exchange.expandQuestion(turn)}
            >
              {turn.questionExpanded ? t.popover.showLess : t.popover.showAll}
            </Button>
          ) : null}
        </div>
      ) : null}

      {/* Capped as a proportion, the way the card opposite is: `--container-measure`
          is the *pane's* prose measure at 980px, and in a 620px window it wrapped
          the answer some 100px short of the edge — narrower than the question
          above it, which inverted which side of the turn looked like the subject. */}
      <div className="flex min-w-0 max-w-11/12 flex-col items-start gap-1.5">
        {/* Nothing else on this side says the turn went wrong once the label
            column is gone, so the failure keeps a marker of its own. */}
        {failure ? (
          <span className="text-destructive text-meta font-medium">{t.popover.failed}</span>
        ) : null}

        {turn.reasoning ? (
          <>
            {turn.reasoningOpen ? (
              <p className="text-muted-foreground max-h-30 overflow-y-auto whitespace-pre-wrap text-quiet">
                {turn.reasoning}
              </p>
            ) : null}
            {/* One button with two labels, not one per arm: the affordance is
                the same object either way. */}
            <Button
              variant="ghost"
              size="xs"
              className={cn(QUIET, "-ml-1.5")}
              onClick={() => exchange.toggleReasoning(turn)}
            >
              {turn.reasoningOpen ? t.popover.hideThinking : t.popover.showThinking}
            </Button>
          </>
        ) : null}

        {turn.status === "waiting-first-token" ? (
          // Two independent proofs the request is alive: a pulsing bar and a
          // counting integer. Under reduced motion the counter carries it.
          <div className="flex w-full max-w-75 flex-col gap-2">
            <div className="bg-muted-foreground h-0.5 animate-pulse rounded-full motion-reduce:animate-none motion-reduce:opacity-60" />
            <span className="text-muted-quiet tabular-nums text-meta">
              {t.popover.waiting}
              {exchange.waitedSeconds > 0 ? ` · ${exchange.waitedSeconds}s` : ""}
            </span>
          </div>
        ) : null}

        {turn.answer ? (
          // The only prose in the product, so it gets its own leading.
          <p className="whitespace-pre-wrap wrap-break-word leading-relaxed">
            {turn.answer}
            {turn.status === "streaming" ? (
              // Not a blinking caret: a blink says "type here", a breathing
              // bar says "output is arriving".
              <span className="bg-foreground ml-0.5 inline-block h-4 w-1.5 animate-pulse rounded-xs align-text-bottom motion-reduce:animate-none" />
            ) : null}
          </p>
        ) : null}

        {/* No icon, for the reason a `Callout` has none: the words carry the
            meaning and the colour is not the only thing saying it. A warning
            triangle beside one line of prose also made this the only glyph in
            the scroller, which read as the loudest state rather than the
            mildest one. */}
        {turn.status === "interrupted" ? (
          <p className="text-warning text-note">
            {turn.answer ? t.popover.interrupted : t.popover.interruptedEmpty}
            {turn.note ? ` — ${turn.note}` : ""}
          </p>
        ) : null}

        {turn.status === "cancelled" ? (
          <p className="text-warning text-note">{t.popover.cancelled}</p>
        ) : null}

        {failure ? (
          <>
            <p className="text-quiet">{failure}</p>
            <div className="mt-1 flex gap-2">
              <Button size="sm" onClick={() => void exchange.retry()}>
                <RotateCcwIcon className="size-3.5" /> {t.popover.retry}
              </Button>
              {settlesInSettings(turn.errorKind) ? (
                <Button variant="outline" size="sm" onClick={() => void showSettings()}>
                  {t.popover.openSettings}
                </Button>
              ) : null}
            </div>
          </>
        ) : null}

        {turn.answer && settled ? (
          // Shrink-to-fit like every other quiet button here. A fixed width to
          // stop the Copied swap reflowing is unnecessary — it is the last
          // child of a left-aligned column — and leaves empty button under the
          // pointer.
          <Button
            variant="ghost"
            size="xs"
            className={cn(QUIET, "-ml-1.5")}
            onClick={() => void exchange.copy(turn.answer, index)}
          >
            {copied ? <CheckIcon /> : <CopyIcon />}
            {copied ? t.popover.copied : t.popover.copy}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
