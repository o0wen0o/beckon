// A section-scoped message: about this pane, not about the window (window-level
// state goes in the status bar). A rule and its text, never a card.
//
// The tone lives in the rule alone and the prose stays muted — the alarm is the
// one sentence in `<strong>`. No icon: nothing here depends on colour alone.
import type * as React from "react";
import { cn } from "@/lib/utils";

interface CalloutProps {
  tone?: "info" | "warn" | "danger";
  /** For the one caller that is not a pane: the Popover's scroller spaces its
   *  children with a `gap`, where `mb-6.5` lands as a hole in the column. */
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
