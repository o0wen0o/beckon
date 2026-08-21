// An Action's `[model]` values are all optional: absent means "inherit the
// global default". Expressing that as an "inherit" entry in a dropdown makes it
// look like a value somebody picked, and hides what is being inherited.
//
// So the row *is* the control: it reads as a value with its provenance, and
// opening it is what overrides — there is no separate button to press first. It
// closes again when focus leaves, which keeps a list of these readable as a
// summary of the Action rather than as a wall of open forms. Closing is
// presentation only; the override itself is on disk the moment it is made.
import * as React from "react";
import { Button } from "@/components/ui/button";
import { InfoHint } from "./InfoHint";

interface OverrideFieldProps {
  label: string;
  /** How the inherited value reads, e.g. "deepseek-v4-flash". */
  inherited: string;
  /** How the overriding value reads. Ignored while inheriting. */
  current: string;
  overridden: boolean;
  /** true → seed from the inherited value; false → write null. */
  onOverride: (on: boolean) => void;
  children: React.ReactNode;
  hint?: string;
  error?: string | null;
}

export function OverrideField({
  label,
  inherited,
  current,
  overridden,
  onOverride,
  children,
  hint,
  error = null,
}: OverrideFieldProps) {
  const [expanded, setExpanded] = React.useState(false);
  const root = React.useRef<HTMLDivElement | null>(null);
  // Focusing the control is also what arms the collapse: without focus inside,
  // `focusout` would never fire and the row would stay open. It has to wait for
  // the control to exist, so it runs in the effect after `expanded` flips.
  const focusOnOpen = React.useRef(false);

  React.useEffect(() => {
    if (!expanded || !focusOnOpen.current) return;
    focusOnOpen.current = false;
    root.current?.querySelector<HTMLElement>("input, select, textarea, button")?.focus();
  }, [expanded]);

  function open() {
    if (expanded) return;
    focusOnOpen.current = true;
    setExpanded(true);
    // The click itself is the override — nothing else to press.
    if (!overridden) onOverride(true);
  }

  return (
    <div
      ref={root}
      role="group"
      aria-label={label}
      className={[
        "flex flex-col gap-2 rounded-md border",
        overridden ? "border-input" : "border-border",
        expanded ? "bg-card p-3" : "p-1",
      ].join(" ")}
      onBlur={(event) => {
        const next = event.relatedTarget;
        if (next instanceof Node && root.current?.contains(next)) return;
        setExpanded(false);
      }}
      onKeyDown={(event) => {
        // Esc closes the row rather than the window it sits in.
        if (event.key === "Escape" && expanded) {
          event.stopPropagation();
          setExpanded(false);
        }
      }}
    >
      {expanded ? (
        <>
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground flex-none text-meta font-semibold">{label}</span>
            {hint ? <InfoHint text={hint} /> : null}
            <Button
              variant="link"
              size="sm"
              className="text-muted-foreground ml-auto h-auto p-0 text-meta underline"
              onClick={() => {
                onOverride(false);
                setExpanded(false);
              }}
            >
              Use the default
            </Button>
          </div>
          <div>{children}</div>
        </>
      ) : (
        <button
          type="button"
          onClick={open}
          className="hover:bg-accent flex w-full items-baseline gap-2 rounded-sm p-2 text-left"
        >
          <span className="text-muted-foreground flex-none text-meta font-semibold">{label}</span>
          {/* An inherited value is shown, but never as though it were this
              Action's. */}
          <span
            className={`font-mono min-w-0 flex-1 truncate text-meta ${
              overridden ? "text-foreground" : "text-muted-foreground"
            }`}
          >
            {overridden ? current : inherited}
          </span>
          <span className="text-muted-foreground flex-none text-meta">
            {overridden ? "overridden" : "from Model defaults"}
          </span>
        </button>
      )}

      {error ? <p className="text-destructive mx-2 mb-1 text-note">{error}</p> : null}
    </div>
  );
}
