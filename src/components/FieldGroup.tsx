// The ledger's horizontal rule: a tracked micro-label with a hairline under it,
// and its rows. This is where the pane's air lives — rows inside a group are
// tight, and the gap between groups is what says "different subject".
import type * as React from "react";

interface FieldGroupProps {
  /** Omitted for a pane with only one group in it, where a head would only
   *  repeat the heading directly above it. */
  title?: string;
  /** One standing statement about the whole group, set quiet at the far end of
   *  the head. For the statement whose only alternative is repeating itself on
   *  every row in the group. */
  note?: string;
  children: React.ReactNode;
}

export function FieldGroup({ title, note, children }: FieldGroupProps) {
  return (
    <section className="mb-8.5 last:mb-0">
      {title ? (
        <div className="flex items-baseline gap-3 border-b pb-2">
          <h2 className="text-muted-quiet text-micro font-semibold tracking-eyebrow uppercase">
            {title}
          </h2>
          {note ? <span className="text-muted-quiet ml-auto text-meta">{note}</span> : null}
        </div>
      ) : null}
      {children}
    </section>
  );
}
