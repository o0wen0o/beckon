// The ledger's horizontal rule: a tracked micro-label with a hairline under it,
// and its rows. This is where the pane's air lives — rows inside a group are
// tight, and the gap between groups is what says "different subject".
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
