// The one surface whose entire hazard is focus and re-render, so the two rules
// that stop a configured model being silently rewritten are both here.
import * as React from "react";
import { useT } from "@/lib/i18n";
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
  /**
   * "" means inherit where `inheritLabel` is given, and "nothing chosen"
   * where it is not — which is the ordinary state of a provider row whose
   * endpoint has never answered.
   */
  value: string;
  options: ModelOption[];
  inheritLabel?: string;
  /**
   * Shown in the trigger for a bare "". Only reachable **without**
   * `inheritLabel`: with one, "" is mapped to the sentinel before Radix sees it,
   * so the two labels are alternatives for the same empty value rather than
   * things to pass together — see `shown` below.
   */
  placeholder?: string;
  id?: string;
  describedBy?: string;
  onChange: (id: string) => void;
}

export function ModelSelect({
  value,
  options,
  inheritLabel,
  placeholder,
  id,
  describedBy,
  onChange,
}: ModelSelectProps) {
  const t = useT();

  // Split once per list rather than twice per render: the panes this sits in
  // re-render on every keystroke, and the split is the same both times.
  const [known, configured] = React.useMemo(
    () => [
      options.filter((option) => option.origin !== "configured"),
      options.filter((option) => option.origin === "configured"),
    ],
    [options],
  );

  /**
   * `value=` + `onValueChange`, never a two-way binding: a binding writes back
   * whatever the select settled on before the catalog arrived.
   *
   * The guard is the other half — without an inherit option there is no
   * legitimate "", so an empty value is a render artefact from a select briefly
   * holding a value not in its own list, and writing it would blank the model.
   */
  function choose(next: string) {
    const resolved = next === INHERIT ? "" : next;
    if (resolved === "" && inheritLabel === undefined) return;
    onChange(resolved);
  }

  // The sentinel stands in for "" only where there is an inherit option to map
  // it back to. Without one, "" has to reach Radix as "" — that is the single
  // value it will paint a placeholder for, and a row whose endpoint has never
  // answered is exactly the case that needs one. `choose` already refuses to
  // write "" in that situation, so nothing new can be written here.
  const shown = inheritLabel !== undefined && value === "" ? INHERIT : value;

  return (
    <Select value={shown} onValueChange={choose}>
      {/* Sized to its own content, with a floor. Stretched to the control
          measure the chevron parks 200px from the value it belongs to, and the
          control reads as an empty box with a marker in the far corner; the
          floor is what that measure was really protecting against — a
          twelve-character model id in a box visibly narrower than every other
          control on the pane. The card right-aligns it either way, so the edge
          the column keeps is the trigger's right one. The ceiling is still
          `Field`'s own token. (ADR-0011, ADR-0012.)

          The floor came down with the type: the 12px register is the control's
          own, in ui/select.tsx, and at 12px a twelve-character id and the
          chevron clear 160px — which is what that floor was really protecting.
          Width is the only thing decided here, because it is the only thing
          about this select that is a Field-column argument rather than the
          control's. */}
      <SelectTrigger id={id} aria-describedby={describedBy} className="max-w-control w-fit min-w-40">
        <SelectValue placeholder={placeholder} />
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
            <SelectLabel>{t.controls.model.configuredGroup}</SelectLabel>
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
