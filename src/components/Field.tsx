// One row of the ledger, and the only layout a labelled control gets: a fixed
// right-aligned label against a value column, closed by a hairline. Centralised
// so a field added later cannot invent its own spacing or forget
// `aria-describedby`.
//
// The explanation is a permanent line under the control, not a bubble — a
// settings pane nobody can read without hovering is the worse failure. There is
// no exception to that any more: an Action's `[model]` overrides were the one
// place without room for a standing line, and they are now rows like every
// other (see `override`).
import * as React from "react";
import { RotateCcwIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";

/**
 * The two measures a control may take, named rather than numbered, so a control
 * that has to cap itself (`ModelSelect`, `Temperature`) reaches the same number
 * this row does. `wide` is for a control sharing its line with buttons.
 */
const MEASURE = { field: "max-w-control", wide: "max-w-control-wide" } as const;

/**
 * A row whose value may be inherited rather than owned — an Action's `[model]`
 * keys, absent from the file until overridden (ADR-0011).
 *
 * The control is live either way and shows the *effective* value, so touching
 * it is what overrides: one gesture for a select, a switch and a slider alike.
 * All this adds is which side of the default the row is on, in the least that
 * still tells the truth — a dot in the label's gutter, and a revert control
 * that names the default in its own label. A sentence under every control said
 * the same thing three times over, and a value nobody is departing from is not
 * news.
 */
interface FieldOverride {
  overridden: boolean;
  /** How the inherited value reads, e.g. "deepseek-v4-flash" or "off". */
  defaultReading: string;
  onRevert: () => void;
}

interface FieldProps {
  label: string;
  /** Constrains the control only — never the hint underneath it. Ignored on an
   *  override row, whose slot is the control measure by definition. */
  measure?: keyof typeof MEASURE;
  /** The permanent explanation under the control. */
  hint?: string;
  /** Red, and replaces the hint while it is present. */
  error?: string | null;
  /** Amber. Not a failure — something worth knowing. Coexists with the hint. */
  warning?: string | null;
  /** Present only on a row that can inherit its value. */
  override?: FieldOverride;
  children: (args: { id: string; describedBy: string | undefined }) => React.ReactNode;
}

export function Field({
  label,
  measure,
  hint,
  error = null,
  warning = null,
  override,
  children,
}: FieldProps) {
  const id = React.useId();
  const descriptionId = `${id}-description`;
  const hintId = `${id}-hint`;
  // Described by whatever is loudest, plus the hint whenever there is one.
  const describedBy =
    [error || warning ? descriptionId : null, hint ? hintId : null].filter(Boolean).join(" ") ||
    undefined;
  const revertLabel = override ? `Use the default (${override.defaultReading})` : "";

  return (
    <div className="flex items-baseline gap-ledger-gap border-b py-3.25">
      <Label
        htmlFor={id}
        className="text-muted-foreground w-ledger-label flex-none justify-end gap-1.5 text-right text-quiet font-normal"
      >
        {override ? (
          // The gutter is reserved on every override row, marked or not: one
          // that exists only when it is filled shifts the label sideways the
          // moment the row is overridden.
          <span
            aria-hidden
            className={`size-1 flex-none rounded-full ${
              override.overridden ? "bg-foreground" : "bg-transparent"
            }`}
          />
        ) : null}
        {label}
        {/* The dot is a mark, so the word goes to the accessibility tree — the
            revert control alone would leave the state to be inferred from which
            buttons exist. */}
        {override?.overridden ? <span className="sr-only">(overridden)</span> : null}
      </Label>
      <div className="flex min-w-0 flex-1 flex-col gap-1.25">
        {override ? (
          // The slot is the control measure whether the control fills it or
          // not, so a group of these lines its revert controls up in a column
          // rather than trailing three different control widths.
          <div className="flex items-center gap-1.5">
            <div className="max-w-control w-full min-w-0">{children({ id, describedBy })}</div>
            {override.overridden ? (
              <Button
                variant="ghost"
                size="icon-sm"
                className="text-muted-quiet flex-none"
                title={revertLabel}
                aria-label={revertLabel}
                onClick={override.onRevert}
              >
                <RotateCcwIcon className="size-3.5" />
              </Button>
            ) : null}
          </div>
        ) : measure ? (
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
