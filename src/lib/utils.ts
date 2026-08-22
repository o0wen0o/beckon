// shadcn/ui's generated helper: merges a component's own classes with the ones
// a caller passes, so the caller's win without duplicating a Tailwind property.
import { clsx, type ClassValue } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

// tailwind-merge reads `text-<name>` against Tailwind's own font-size list and
// files anything else — our `--text-*` scale in globals.css — under text colour.
// Two wrong answers follow from that one guess: `text-note` stops cancelling the
// `text-sm` it was written to replace, and it cancels the `text-muted-foreground`
// beside it instead. Naming the scale here is what makes `cn("text-sm", "text-note")`
// mean 12px. Tailwind's own sizes stay registered — this list is added to them.
// It must mirror the `--text-*` block in globals.css; nothing links the two, so
// a token added there without a name added here regresses silently.
const twMerge = extendTailwindMerge({
  extend: {
    classGroups: {
      "font-size": [{ text: ["micro", "meta", "quiet", "note", "query", "title"] }],
    },
  },
});

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
