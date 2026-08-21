// Every pane opens the same way: the section's name at display size, one line
// saying what it is for, and the section's create action on the title's line.
import type * as React from "react";

interface PaneHeaderProps {
  title: string;
  /** The one-line description. */
  children?: React.ReactNode;
  /** The section's create action, if it has one. */
  action?: React.ReactNode;
}

export function PaneHeader({ title, children, action }: PaneHeaderProps) {
  return (
    <header className="mb-6.5 flex items-start justify-between gap-5">
      <div className="min-w-0">
        <h1 className="font-display text-title font-semibold tracking-title">{title}</h1>
        {children ? (
          <p className="text-muted-foreground mt-1 max-w-lede text-quiet text-pretty">{children}</p>
        ) : null}
      </div>
      {action ? <div className="flex-none">{action}</div> : null}
    </header>
  );
}
