// One layout for every labelled control: label (plus its explanation behind an
// info icon), control, then whichever of warning / error applies. Centralised so
// a field added later cannot invent its own spacing or forget to wire
// `aria-describedby`.
//
// The hint lives in the icon's bubble, never inline: warnings and errors are the
// only prose that earns a permanent line, because those are conditions the user
// has to act on rather than background.
import * as React from "react";
import { Label } from "@/components/ui/label";
import { InfoHint } from "./InfoHint";

interface FieldProps {
  label: string;
  /** Shown on hover/focus of the info icon; always in the a11y tree. */
  hint?: string;
  /** Red, and replaces the hint while it is present. */
  error?: string | null;
  /** Amber. Not a failure — something worth knowing. Coexists with the hint. */
  warning?: string | null;
  /**
   * Which way the hint bubble hangs. A field in a right-hand column passes
   * `"end"`, so the bubble hangs leftwards instead of off the pane's edge.
   */
  hintAlign?: "start" | "end";
  children: (args: { id: string; describedBy: string | undefined }) => React.ReactNode;
}

export function Field({
  label,
  hint,
  error = null,
  warning = null,
  hintAlign = "start",
  children,
}: FieldProps) {
  const id = React.useId();
  const descriptionId = `${id}-description`;
  const hintId = `${id}-hint`;
  // The control is described by whatever is loudest, and by the hint whenever
  // there is one — the hint is invisible most of the time, so dropping it from
  // the description is the one place it would be lost outright.
  const describedBy =
    [error || warning ? descriptionId : null, hint ? hintId : null].filter(Boolean).join(" ") ||
    undefined;

  return (
    <div className="mb-6 flex flex-col gap-1.5">
      <div className="flex items-center gap-2">
        <Label htmlFor={id} className="text-muted-foreground text-xs font-semibold">
          {label}
        </Label>
        {hint ? <InfoHint text={hint} id={hintId} align={hintAlign} /> : null}
      </div>
      {children({ id, describedBy })}

      {error ? (
        <p id={descriptionId} className="text-destructive font-small m-0 text-xs">
          {error}
        </p>
      ) : warning ? (
        <p id={descriptionId} className="font-small m-0 text-warning text-xs">
          {warning}
        </p>
      ) : null}
    </div>
  );
}
