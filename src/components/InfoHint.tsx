// An explanation behind an icon, for the one place a permanent line does not
// fit: `OverrideField`'s collapsed rows. Everywhere else a field's explanation
// is a standing line under the control (see `Field`).
//
// The text stays in the accessibility tree at all times — the `sr-only` span is
// what `aria-describedby` points at — so the popover is a visual affordance
// only, and is `aria-hidden` to stop the sentence being read twice.
import * as React from "react";
import { InfoIcon } from "lucide-react";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

interface InfoHintProps {
  text: string;
  /** The id the described control points at. */
  id?: string;
}

export function InfoHint({ text, id }: InfoHintProps) {
  // Hover is the affordance; the click is for everyone hover does not serve —
  // a keyboard reaches it as a button, a touch or a trackpad tap pins it open.
  const [open, setOpen] = React.useState(false);

  return (
    <span
      className="inline-flex align-middle"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
    >
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger
          aria-label="More information"
          className="text-muted-foreground hover:text-primary focus-visible:text-primary focus-visible:ring-ring/50 cursor-help rounded-full transition-colors focus-visible:ring-[3px] focus-visible:outline-none"
        >
          <InfoIcon className="size-3.5" />
        </PopoverTrigger>
        <PopoverContent
          align="start"
          aria-hidden
          className="text-muted-foreground w-auto max-w-70 p-3 text-meta leading-relaxed"
        >
          {text}
        </PopoverContent>
      </Popover>
      <span id={id} className="sr-only">
        {text}
      </span>
    </span>
  );
}
