// One row of the ledger, and the only layout a labelled control gets: a fixed
// right-aligned label against a value column, closed by a hairline. Centralised
// so a field added later cannot invent its own spacing or forget
// `aria-describedby`.
//
// The explanation is a permanent line under the control, not a bubble — a
// settings pane nobody can read without hovering is the worse failure.
// `InfoHint` survives only where the room is not there: `OverrideField`.
import * as React from "react";
import { Label } from "@/components/ui/label";

/**
 * The two measures a control may take, named rather than numbered, so a control
 * that has to cap itself (`ModelSelect`, `Temperature`) reaches the same number
 * this row does. `wide` is for a control sharing its line with buttons.
 */
const MEASURE = { field: "max-w-control", wide: "max-w-control-wide" } as const;

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
  // Described by whatever is loudest, plus the hint whenever there is one.
  const describedBy =
    [error || warning ? descriptionId : null, hint ? hintId : null].filter(Boolean).join(" ") ||
    undefined;

  return (
    <div className="flex items-baseline gap-ledger-gap border-b py-3.25">
      <Label
        htmlFor={id}
        className="text-muted-foreground w-ledger-label flex-none justify-end text-right text-quiet font-normal"
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
