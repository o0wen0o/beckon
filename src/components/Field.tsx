// One configuration, one card (ADR-0012). The name and its explanation on the
// left, the control at the card's right edge; the card's own edge is what closes
// the row, so nothing on the pane draws a hairline any more. Centralised so a
// field added later cannot invent its own spacing or forget `aria-describedby`.
//
// The explanation is a permanent line under the name, not a bubble — a settings
// pane nobody can read without hovering is the worse failure. There is no
// exception to that: an Action's `[model]` overrides were the one place without
// room for a standing line, and they are cards like every other (see `override`).
import * as React from "react";
import { RotateCcwIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";

/**
 * The card, and the only place its geometry is written. No fill and no shadow:
 * the edge is the whole card, so "selected" stays the pane's one inversion.
 */
export const CARD = "rounded-lg border px-4.5 py-3.75";

/**
 * The card's one state under the pointer: the edge strengthens, and nothing
 * fills — a fill is still the pane's inversion accent and nothing else. Shared
 * with `NavCard`, which is a card you can actually press.
 */
export const CARD_HOVER =
  "transition-colors duration-150 ease-out hover:border-border-strong motion-reduce:transition-none";

/**
 * The two measures a control may take, named rather than numbered, so a control
 * that has to cap itself (`ModelSelect`, `Temperature`) reaches the same number
 * this card does. `wide` is for a control sharing its line with buttons.
 */
const MEASURE = { field: "max-w-control", wide: "max-w-control-wide" } as const;

/**
 * A row whose value may be inherited rather than owned — an Action's `[model]`
 * keys, absent from the file until overridden (ADR-0011).
 *
 * The control is live either way and shows the *effective* value, so touching
 * it is what overrides: one gesture for a select, a switch and a slider alike.
 * All this adds is which side of the default the row is on, in the least that
 * still tells the truth — a dot hung in the card's padding beside the name,
 * and a revert control that names the default in its own label.
 */
interface FieldOverride {
  overridden: boolean;
  /** How the inherited value reads, e.g. "deepseek-v4-flash" or "off". */
  defaultReading: string;
  onRevert: () => void;
}

interface FieldProps {
  label: string;
  /** Caps the control. On a stacked card the control also fills up to it. */
  measure?: keyof typeof MEASURE;
  /**
   * Text entry: the control goes under the name at its measure instead of at
   * the card's right edge. A field cannot right-align against its own label —
   * at the window's minimum width there is no room left for the name.
   */
  stacked?: boolean;
  /**
   * No card of its own: no edge, no padding, no hover. For the one screen that
   * holds a single card of related fields, where four boxes inside a box would
   * be four enclosures of the same thing (ADR-0012).
   */
  bare?: boolean;
  /** The permanent explanation under the name. */
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
  stacked = false,
  bare = false,
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
  const described = Boolean(error || warning || hint);

  const control = measure ? (
    <div className={`${MEASURE[measure]} ${stacked ? "w-full" : ""} min-w-0`}>
      {children({ id, describedBy })}
    </div>
  ) : (
    children({ id, describedBy })
  );

  const shell = bare ? "" : `${CARD} ${CARD_HOVER}`;

  return (
    <div
      className={
        stacked
          ? `${shell} grid grid-cols-1 gap-y-2`
          : `${shell} grid grid-cols-[1fr_auto] items-center gap-x-6`
      }
    >
      {/* A shade heavier than the prose under it — shadcn's own `font-medium`,
          which this row used to cancel. The name is the one thing on a card
          that has to be findable while scanning past it. */}
      <Label htmlFor={id} className="relative col-start-1 row-start-1 gap-1.5">
        {override?.overridden ? (
          // The mark hangs in the card's own padding rather than taking a
          // column of its own: reserved in the flow it indented every name in
          // the group past every other name on the pane, and out of the flow it
          // still cannot shift the one row that carries it.
          <span
            aria-hidden
            className="bg-foreground absolute top-1/2 -left-2.5 size-1 -translate-y-1/2 rounded-full"
          />
        ) : null}
        {label}
        {/* The dot is a mark, so the word goes to the accessibility tree — the
            revert control alone would leave the state to be inferred from which
            buttons exist. */}
        {override?.overridden ? <span className="sr-only">(overridden)</span> : null}
      </Label>

      <div
        className={
          stacked
            ? "col-start-1 row-start-2"
            : `col-start-2 row-start-1 min-w-0 justify-self-end ${described ? "row-span-2" : ""}`
        }
      >
        {override ? (
          // The revert slot is held open on every override row, filled or not,
          // so the controls above it stay aligned at the card's right edge
          // rather than stepping left on the rows that have one.
          <div className="flex items-center gap-1.5">
            {control}
            <span className="flex size-7 flex-none items-center justify-center">
              {override.overridden ? (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-quiet"
                  title={revertLabel}
                  aria-label={revertLabel}
                  onClick={override.onRevert}
                >
                  <RotateCcwIcon className="size-3.5" />
                </Button>
              ) : null}
            </span>
          </div>
        ) : (
          control
        )}
      </div>

      {described ? (
        <div
          className={`col-start-1 flex max-w-measure flex-col gap-0.5 ${
            stacked ? "row-start-3" : "row-start-2 mt-0.75"
          }`}
        >
          {error ? (
            <p id={descriptionId} className="text-destructive m-0 text-note">
              {error}
            </p>
          ) : warning ? (
            <p id={descriptionId} className="m-0 text-warning text-note">
              {warning}
            </p>
          ) : null}

          {hint ? (
            <p id={hintId} className="text-muted-foreground m-0 text-meta">
              {hint}
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
