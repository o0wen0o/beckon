// A tracked micro-label and the cards under it. The head carries no hairline
// (ADR-0012): every card already has an edge, so a rule under the label would be
// the only line on the pane that closes nothing.
//
// This is where the pane's air lives — cards inside a group are 10px apart, and
// the gap between groups is what says "different subject".
import type * as React from "react";

interface FieldGroupProps {
  /** Omitted for a pane with only one group in it, where a head would only
   *  repeat the heading directly above it. */
  title?: string;
  /** One standing statement about the whole group, set quiet at the far end of
   *  the head. For the statement whose only alternative is repeating itself on
   *  every card in the group. */
  note?: string;
  children: React.ReactNode;
}

export function FieldGroup({ title, note, children }: FieldGroupProps) {
  return (
    <section className="mb-7.5 last:mb-0">
      {title ? (
        <div className="flex items-baseline gap-3 pb-2">
          <h2 className="text-muted-quiet text-micro font-semibold tracking-eyebrow uppercase">
            {title}
          </h2>
          {note ? <span className="text-muted-quiet ml-auto text-meta">{note}</span> : null}
        </div>
      ) : null}
      <div className="flex flex-col gap-2.5">{children}</div>
    </section>
  );
}
