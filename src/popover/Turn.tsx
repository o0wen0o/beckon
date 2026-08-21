// One turn, sided: what you asked is a card on the right, what came back runs
// left and bare. The sides are what say who spoke, which is why there is no
// label column here and no hairline between turns — the gap separates them.
//
// Every state a turn can be in is rendered here, so the ones that must not look
// alike sit side by side in one file. Output is plain text with preserved
// whitespace: acceptable for the MVP, and it cannot inject anything into the
// WebView.
import { CheckIcon, CopyIcon, RotateCcwIcon, TriangleAlertIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { describeFailure } from "@/lib/failures";
import { showSettings } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { exchange, settlesInSettings, type Turn } from "./exchange";

/** Long enough that the clamp is doing something, i.e. worth offering to undo. */
const CLAMP_AT = 160;

/**
 * The card is capped well short of the window: a bubble that reaches both edges
 * stops reading as one side of a conversation. The fill is `--muted`, the
 * quietest one there is — inversion is reserved for "current" everywhere else in
 * the product, and spending it here would make your own words the loudest thing
 * on screen.
 */
const CARD = "bg-muted max-w-4/5 rounded-lg border px-3 py-1.5 whitespace-pre-wrap text-quiet";

export function TurnView({ turn, index }: { turn: Turn; index: number }) {
  /**
   * The cause named first, then the provider's own words — the same sentence
   * Settings builds. Printing `note` bare handed the user a raw reqwest chain
   * for a `network` failure while Settings said "Could not reach the API".
   */
  const failure =
    turn.status === "error"
      ? describeFailure({ kind: turn.errorKind ?? "error", message: turn.note ?? "" })
      : null;

  const settled =
    turn.status === "done" || turn.status === "interrupted" || turn.status === "cancelled";
  const copied = exchange.copiedTurn === index;

  return (
    <div className="flex flex-col gap-2">
      {turn.question ? (
        // The toggle sits under the card rather than inside it: `ghost` hovers
        // to `--accent`, which is the same grey the card is filled with.
        <div className="flex flex-col items-end gap-1">
          {/* Clamped, not scrollable: a scroller inside the body's scroller is
              a trap. */}
          <div className={cn(CARD, turn.questionExpanded ? null : "line-clamp-2")}>
            {turn.question}
          </div>
          {turn.question.length > CLAMP_AT ? (
            <Button
              variant="ghost"
              size="xs"
              className="-mr-2"
              onClick={() => exchange.expandQuestion(turn)}
            >
              {turn.questionExpanded ? "Show less" : "Show all"}
            </Button>
          ) : null}
        </div>
      ) : null}

      <div className="flex min-w-0 flex-col items-start gap-1.5">
        {/* Nothing else on this side says the turn went wrong once the label
            column is gone, so the failure keeps a marker of its own. */}
        {failure ? (
          <span className="text-destructive text-meta font-medium">Failed</span>
        ) : null}

        {turn.reasoning ? (
          turn.reasoningOpen ? (
            <>
              <p className="text-muted-foreground max-h-40 overflow-y-auto whitespace-pre-wrap text-quiet">
                {turn.reasoning}
              </p>
              <Button
                variant="ghost"
                size="xs"
                className="-ml-2"
                onClick={() => exchange.toggleReasoning(turn)}
              >
                Hide
              </Button>
            </>
          ) : (
            <Button
              variant="ghost"
              size="xs"
              className="-ml-2"
              onClick={() => exchange.toggleReasoning(turn)}
            >
              Show what it thought
            </Button>
          )
        ) : null}

        {turn.status === "waiting-first-token" ? (
          // Two independent proofs the request is alive: a bar that pulses and
          // a counting integer. Under reduced motion the bar goes static — which
          // is still a bar, not a stalled one — and the counter carries it.
          <div className="flex w-full max-w-75 flex-col gap-2 py-1">
            <div className="bg-muted-foreground h-0.5 animate-pulse rounded-full motion-reduce:animate-none motion-reduce:opacity-60" />
            <span className="text-muted-quiet tabular-nums text-meta">
              Waiting for the first token
              {exchange.waitedSeconds > 0 ? ` · ${exchange.waitedSeconds}s` : ""}
            </span>
          </div>
        ) : null}

        {turn.answer ? (
          // The only prose in the product, so it gets its own leading.
          <p className="max-w-measure whitespace-pre-wrap break-words leading-relaxed">
            {turn.answer}
            {turn.status === "streaming" ? (
              // Not a blinking text caret — a blink says "type here". A steady
              // bar that breathes says "output is arriving".
              <span className="bg-foreground ml-0.5 inline-block h-4 w-1.5 animate-pulse rounded-xs align-text-bottom motion-reduce:animate-none" />
            ) : null}
          </p>
        ) : null}

        {turn.status === "interrupted" ? (
          <p className="text-warning flex items-center gap-1.5 text-note">
            <TriangleAlertIcon className="size-3 flex-none" />
            {turn.answer ? "Interrupted" : "Interrupted before any output"}
            {turn.note ? ` — ${turn.note}` : ""}
          </p>
        ) : null}

        {turn.status === "cancelled" ? (
          <p className="text-warning text-note">Cancelled.</p>
        ) : null}

        {failure ? (
          <>
            <p className="max-w-measure text-quiet">{failure}</p>
            <div className="mt-1 flex gap-2">
              <Button size="sm" onClick={() => void exchange.retry()}>
                <RotateCcwIcon /> Retry
              </Button>
              {settlesInSettings(turn.errorKind) ? (
                <Button variant="outline" size="sm" onClick={() => void showSettings()}>
                  Open Settings
                </Button>
              ) : null}
            </div>
          </>
        ) : null}

        {turn.answer && settled ? (
          // Quiet, and fixed-width so the Copied swap cannot reflow the block:
          // the answer is the content, not the button under it.
          <Button
            variant="ghost"
            size="xs"
            className="-ml-2 w-22 justify-start"
            onClick={() => void exchange.copy(turn.answer, index)}
          >
            {copied ? <CheckIcon /> : <CopyIcon />}
            {copied ? "Copied" : "Copy"}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
