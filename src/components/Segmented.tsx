// Three-or-so mutually exclusive choices, shown rather than hidden behind a
// dropdown. shadcn/ui's ToggleGroup in `single` mode is Radix's roving-focus
// group, which is the same one-tab-stop-plus-arrows behaviour the hand-rolled
// radiogroup implemented.
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

interface SegmentedProps<T extends string> {
  value: T;
  options: { value: T; label: string }[];
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
      // The group's fill and the selected chip's fill trade places between
      // themes, because the chip has to sit *above* the group in both: in light
      // that is white on grey, in dark it is grey on the page colour. Keeping
      // `bg-muted` on the group in dark would have put a 0.145 chip inside a
      // 0.269 well, which reads as the one option that is switched off.
      className="bg-muted dark:bg-background self-start gap-0.5 rounded-md border p-0.5"
    >
      {options.map((option) => (
        <ToggleGroupItem
          key={option.value}
          value={option.value}
          // The selected item and the hovered one must not read the same, and
          // the stock variant gives both `hover:bg-accent`. `--accent` equals
          // `--muted` in dark and equals the group's own fill in light, so the
          // hover fill is either indistinguishable from the selected chip or
          // from nothing — it is cancelled outright. A fill means "selected"
          // and nothing else; hover brightens the label instead.
          className="text-muted-foreground data-[state=off]:hover:bg-transparent data-[state=off]:hover:text-foreground data-[state=on]:bg-background dark:data-[state=on]:bg-muted data-[state=on]:text-foreground rounded-sm px-3"
        >
          {option.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}
