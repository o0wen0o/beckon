// One row of the ledger, and the only layout a labelled control gets: a
// right-aligned label against a value column, closed by a hairline. Centralised
// so a field added later cannot invent its own spacing or forget to wire
// `aria-describedby`.
//
// The label column is fixed and the labels are flush to it, so the controls all
// start at the same x and the pane can be read as a column of values rather than
// as a stack of forms. That column is what makes the density legible: rows are
// tight, and the air goes between `FieldGroup`s instead.
//
// The explanation is a permanent line under the control. It used to live behind
// an info icon, on the grounds that two lines of prose per control reads as
// documentation — true when the label sat directly above the control and the
// hint pushed them apart, but in the value column the hint is beside the label
// rather than between them, and a settings pane nobody can read without
// hovering is the worse failure. `InfoHint` survives where the room genuinely
// is not there: `OverrideField`'s collapsed rows.
import * as React from "react";
import { Label } from "@/components/ui/label";

/**
 * The two measures a control is allowed to take. A text field stretched across
 * the pane reads as an empty box with a cursor in the corner, so the value
 * column holds the control to a measure and lets the explanation under it run
 * to the prose width instead. `wide` is for a control that shares its line with
 * buttons.
 */
const MEASURE = { field: "max-w-85", wide: "max-w-105" } as const;

interface FieldProps {
  label: string;
  /** Constrains the control only — never the hint underneath it. */
  measure?: keyof typeof MEASURE;
  /** The permanent explanation under the control. */
  hint?: string;
  /** Red, and replaces the hint while it is present. */
  error?: string | null;
  /** Amber. Not a failure — something worth knowing. Coexists with the hint. */
  warning?: string | null;
  children: (args: { id: string; describedBy: string | undefined }) => React.ReactNode;
}

export function Field({
  label,
  measure,
  hint,
  error = null,
  warning = null,
  children,
}: FieldProps) {
  const id = React.useId();
  const descriptionId = `${id}-description`;
  const hintId = `${id}-hint`;
  // The control is described by whatever is loudest, and by the hint whenever
  // there is one.
  const describedBy =
    [error || warning ? descriptionId : null, hint ? hintId : null].filter(Boolean).join(" ") ||
    undefined;

  return (
    <div className="flex items-baseline gap-5 border-b py-3.25">
      <Label
        htmlFor={id}
        className="text-muted-foreground w-42 flex-none justify-end text-right text-quiet font-normal"
      >
        {label}
      </Label>
      <div className="flex min-w-0 flex-1 flex-col gap-1.25">
        {measure ? (
          <div className={`${MEASURE[measure]} min-w-0`}>{children({ id, describedBy })}</div>
        ) : (
          children({ id, describedBy })
        )}

        {error ? (
          <p id={descriptionId} className="text-destructive m-0 max-w-measure text-note">
            {error}
          </p>
        ) : warning ? (
          <p id={descriptionId} className="m-0 max-w-measure text-warning text-note">
            {warning}
          </p>
        ) : null}

        {hint ? (
          <p id={hintId} className="text-muted-foreground m-0 max-w-measure text-meta">
            {hint}
          </p>
        ) : null}
      </div>
    </div>
  );
}
