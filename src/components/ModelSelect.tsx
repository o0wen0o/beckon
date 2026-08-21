// The one surface whose entire hazard is focus and re-render, so the two rules
// that stop a configured model being silently rewritten are both here.
import type { ModelOption } from "@/lib/types";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

/**
 * Radix refuses an item whose value is the empty string, and "" is how the
 * config expresses inherit. One sentinel, mapped at both edges, so nothing
 * outside this file has to know.
 */
const INHERIT = "__inherit__";

interface ModelSelectProps {
  /** "" means inherit, and is only legitimate when `inheritLabel` is given. */
  value: string;
  options: ModelOption[];
  inheritLabel?: string;
  id?: string;
  describedBy?: string;
  onChange: (id: string) => void;
}

export function ModelSelect({
  value,
  options,
  inheritLabel,
  id,
  describedBy,
  onChange,
}: ModelSelectProps) {
  const known = options.filter((option) => option.origin !== "configured");
  const configured = options.filter((option) => option.origin === "configured");

  /**
   * `value=` + `onValueChange`, never a two-way binding. A binding would write
   * back whatever the select settled on before the catalog arrived, which is
   * exactly how a configured model gets silently replaced.
   *
   * The guard is the other half of that rule: without an inherit option there
   * is no legitimate "", so an empty value can only be a render artefact from a
   * select momentarily holding a value not in its own list — and writing it
   * would blank the configured model.
   */
  function choose(next: string) {
    const resolved = next === INHERIT ? "" : next;
    if (resolved === "" && inheritLabel === undefined) return;
    onChange(resolved);
  }

  return (
    <Select value={value === "" ? INHERIT : value} onValueChange={choose}>
      {/* Held to the same measure as a text field rather than shrunk to its own
          content: shadcn's trigger is `w-fit`, which parks a twelve-character
          model id in a box narrower than everything else in the value column
          and breaks the one line the ledger draws. */}
      <SelectTrigger id={id} aria-describedby={describedBy} className="w-full max-w-85">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {inheritLabel !== undefined ? <SelectItem value={INHERIT}>{inheritLabel}</SelectItem> : null}
        {known.map((option) => (
          <SelectItem key={option.id} value={option.id}>
            {option.label}
          </SelectItem>
        ))}
        {configured.length > 0 ? (
          // Quarantined rather than mixed in: nothing vouches for these but the
          // configuration file that names them.
          <SelectGroup>
            <SelectLabel>Named by your configuration</SelectLabel>
            {configured.map((option) => (
              <SelectItem key={option.id} value={option.id}>
                {option.label}
              </SelectItem>
            ))}
          </SelectGroup>
        ) : null}
      </SelectContent>
    </Select>
  );
}
