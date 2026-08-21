// A 0–2 range with meaning at both ends deserves better than a bare spinner.
// The number input stays: it is the typable, screen-reader-friendly path, and
// the slider is the affordance.
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import { useT } from "@/lib/i18n";

interface TemperatureProps {
  value: number;
  id?: string;
  describedBy?: string;
  onChange: (value: number) => void;
}

export function Temperature({ value, id, describedBy, onChange }: TemperatureProps) {
  const t = useT();

  /**
   * `min`/`max` on a number input constrain the spinner, not typing: "9" is
   * accepted and would reach disk. Clamp on the way out, and drop anything
   * non-finite rather than writing it — the API refuses such a request.
   */
  function commit(raw: string | number) {
    const parsed = typeof raw === "number" ? raw : Number(String(raw).trim());
    if (!Number.isFinite(parsed)) return;
    onChange(Math.min(2, Math.max(0, Math.round(parsed * 10) / 10)));
  }

  return (
    // A width, not just a ceiling: the card right-aligns its control, and a
    // slider whose track is `flex-1` inside a shrink-to-fit box collapses to the
    // number input beside it. The measure is still `Field`'s own token.
    <div className="w-control max-w-full">
      {/* The scale is nested with the slider, not laid beside the number box:
          the ticks name positions *on the track*, so spanning the whole row put
          "2 · loose" under the number input and "1" left of the midpoint. */}
      <div className="flex items-start gap-3">
        <div className="flex min-w-0 flex-1 flex-col gap-1 pt-1.75">
          <Slider
            min={0}
            max={2}
            step={0.1}
            value={[value]}
            aria-label={t.settings.defaults.temperature}
            aria-describedby={describedBy}
            onValueChange={([next]) => commit(next)}
          />
          <div className="text-muted-foreground flex justify-between text-micro">
            <span>{t.controls.temperature.low}</span>
            <span>{t.controls.temperature.mid}</span>
            <span>{t.controls.temperature.high}</span>
          </div>
        </div>
        {/* A caption on the slider, not a field of its own: at an input's full
            height it stood taller than the whole track it annotates. */}
        <Input
          id={id}
          type="number"
          step={0.1}
          min={0}
          max={2}
          value={value}
          className="h-7 w-17 flex-none text-center tabular-nums"
          onChange={(event) => commit(event.currentTarget.value)}
        />
      </div>
    </div>
  );
}
