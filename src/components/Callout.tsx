// A section-scoped message: something about this pane, not about the window.
// Window-level state lives in the status bar instead, so the two cannot pile up
// into a wall of coloured boxes.
//
// A rule and its text, never a card. The pane is ruled horizontally from top to
// bottom, so an outlined and rounded box in the middle of it is the only
// container on screen and reads as a different kind of thing — which is exactly
// backwards, because a callout is *about* the rows underneath it.
//
// The tone lives in the rule alone, and the prose stays the same muted grey as
// every other explanation on the pane: a paragraph set entirely in red says
// "everything here is the alarm", when what is actually the alarm is the one
// sentence in `<strong>`. That is also why there is no icon — the rule is the
// marker, and the meaning is carried by the words rather than by the colour.
import type * as React from "react";
import { cn } from "@/lib/utils";

interface CalloutProps {
  tone?: "info" | "warn" | "danger";
  /** For the one caller that is not a pane. `mb-6.5` is the ledger's rhythm
   *  below a callout; the Popover's scroller spaces its children with a `gap`,
   *  where that margin lands on top of the gap as a hole in the column. */
  className?: string;
  children: React.ReactNode;
}

const RULE: Record<NonNullable<CalloutProps["tone"]>, string> = {
  info: "border-l-primary",
  warn: "border-l-warning",
  danger: "border-l-destructive",
};

export function Callout({ tone = "info", className, children }: CalloutProps) {
  return (
    <div
      role={tone === "danger" ? "alert" : undefined}
      className={cn(
        "text-muted-foreground mb-6.5 grid max-w-measure gap-1.5 border-l-2 py-0.5 pl-3.5 text-sm",
        "[&_strong]:text-foreground [&_strong]:font-bold",
        "[&_ul]:list-disc [&_ul]:pl-5",
        RULE[tone],
        className,
      )}
    >
      {children}
    </div>
  );
}
