// Three-or-so mutually exclusive choices, shown rather than hidden behind a
// dropdown. ToggleGroup in `single` mode gives Radix's roving focus.
import type { LucideIcon } from "lucide-react";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

interface SegmentedProps<T extends string> {
  value: T;
  /** The icon is optional and per set: a glyph earns its place where the
   *  choices have one each, and half a set of icons is worse than none. */
  options: { value: T; label: string; icon?: LucideIcon }[];
  label: string;
  id?: string;
  describedBy?: string;
  onChange: (value: T) => void;
}

export function Segmented<T extends string>({
  value,
  options,
  label,
  id,
  describedBy,
  onChange,
}: SegmentedProps<T>) {
  return (
    <ToggleGroup
      type="single"
      id={id}
      aria-label={label}
      aria-describedby={describedBy}
      value={value}
      // Radix clears a single-value group when its active item is pressed
      // again. There is no "no Input Source", so an empty value is dropped
      // rather than written.
      onValueChange={(next) => next && onChange(next as T)}
      size="sm"
      // A track holding separate segments, not one welded row: the group is an
      // edge and nothing else — no ground, since a fill is the pane's "selected"
      // and the selected segment already carries it — and the choices inside it
      // are rounded and set apart by a gap. A non-zero `spacing` is also what turns off shadcn's
      // welded-row rules (rounded-none plus a shared left border).
      spacing={1}
      className="self-start rounded-lg border p-1"
    >
      {options.map((option) => (
        <ToggleGroupItem
          key={option.value}
          value={option.value}
          // A segment draws no edge of its own — the group's is the only one —
          // so hover is a `--muted` ground under the label. Stock gives selected
          // and hovered the same `bg-accent`; here the two fills stay a register
          // apart, hover quiet and selected the pane's inversion.
          className="text-muted-foreground rounded-md data-[state=off]:hover:bg-muted data-[state=off]:hover:text-foreground data-[state=on]:bg-primary data-[state=on]:font-medium data-[state=on]:text-primary-foreground px-3 transition-colors duration-150 ease-out motion-reduce:transition-none"
        >
          {/* 14px, not the toggle base's 16: the glyph rides beside a 14px
              label and matching it keeps the pair one word. */}
          {option.icon ? <option.icon aria-hidden className="size-3.5" /> : null}
          {option.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}
