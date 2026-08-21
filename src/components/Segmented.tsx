// Three-or-so mutually exclusive choices, shown rather than hidden behind a
// dropdown. ToggleGroup in `single` mode gives Radix's roving focus.
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
      className="bg-muted self-start gap-0.5 rounded-md border p-0.5"
    >
      {options.map((option) => (
        <ToggleGroupItem
          key={option.value}
          value={option.value}
          // A fill means "selected" and nothing else; hover brightens the
          // label. Stock gives selected and hovered the same `bg-accent`, and
          // `--accent` equals `--muted`, so hover would match the group's ground.
          className="text-muted-foreground data-[state=off]:hover:bg-transparent data-[state=off]:hover:text-foreground data-[state=on]:bg-primary data-[state=on]:text-primary-foreground data-[state=on]:font-medium rounded-sm px-3 transition-colors duration-150 ease-out motion-reduce:transition-none"
        >
          {option.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}
