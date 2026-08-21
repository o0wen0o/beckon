// The ledger's horizontal rule: a tracked micro-label with a hairline under it,
// and the rows that belong to it.
//
// This is where the pane's air lives. Rows inside a group are tight — the
// hairline is enough to separate them — and the gap between groups is what says
// "different subject". Spacing every field apart by the same amount instead
// produces a list with no structure in it, which is the failure this component
// exists to correct.
import type * as React from "react";

interface FieldGroupProps {
  /** Omitted for a pane with only one group in it, where a head would only
   *  repeat the heading directly above it. */
  title?: string;
  children: React.ReactNode;
}

export function FieldGroup({ title, children }: FieldGroupProps) {
  return (
    <section className="mb-8.5 last:mb-0">
      {title ? (
        <h2 className="text-muted-quiet border-b pb-2 text-micro font-semibold tracking-eyebrow uppercase">
          {title}
        </h2>
      ) : null}
      {children}
    </section>
  );
}
