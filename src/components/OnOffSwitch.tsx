// A switch with its state written beside it, which is the only form a switch
// takes on this surface. Three panes drew it by hand and the third had already
// dropped `self-start` and `text-left`, so the fixed-width readout that exists to
// stop the row twitching was centred in one place and flush in two.
//
// A switch rather than a checkbox: these settings take effect immediately, and a
// checkbox reads as "will be applied when you save" — which there is no way to
// do here (ADR-0003).
import { Switch } from "@/components/ui/switch";

interface OnOffSwitchProps {
  checked: boolean;
  /** Named for the switch itself, since the readout beside it is not a label. */
  label: string;
  id?: string;
  describedBy?: string;
  onChange: (on: boolean) => void;
}

export function OnOffSwitch({ checked, label, id, describedBy, onChange }: OnOffSwitchProps) {
  return (
    <div className="flex items-center gap-2 self-start">
      <Switch
        id={id}
        aria-describedby={describedBy}
        aria-label={label}
        checked={checked}
        onCheckedChange={onChange}
      />
      {/* aria-hidden: the switch already announces checked, and a screen reader
          reading "On" after "on" is noise. Fixed width and flush left, or the
          row twitches every time it is thrown. */}
      <span aria-hidden className="text-muted-foreground min-w-5.5 text-left text-meta">
        {checked ? "On" : "Off"}
      </span>
    </div>
  );
}
