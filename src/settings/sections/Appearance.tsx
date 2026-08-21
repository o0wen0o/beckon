import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { Segmented } from "@/components/Segmented";
import { SYSTEM_APPEARANCE } from "@/lib/platform";
import type { Theme } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

const THEMES = [
  { value: "light", label: "Light", icon: SunIcon },
  { value: "dark", label: "Dark", icon: MoonIcon },
  { value: "system", label: "System", icon: MonitorIcon },
] satisfies { value: Theme; label: string; icon: typeof SunIcon }[];

const THEME_HINT = `Beckon starts light unless you say otherwise. “System” is the only setting that reads the ${SYSTEM_APPEARANCE}, and it follows it live.`;

export function Appearance() {
  const store = useStore(settings);
  const config = store.config;

  return (
    <>
      <PaneHeader title="Appearance">
        Applies to the Launcher, the Popover and this window at once.
      </PaneHeader>

      {config ? (
        <FieldGroup>
          <Field label="Theme" hint={THEME_HINT}>
            {({ id, describedBy }) => (
              <Segmented
                id={id}
                describedBy={describedBy}
                label="Theme"
                value={config.theme}
                options={THEMES}
                onChange={(theme) => store.editConfig((draft) => (draft.theme = theme), true)}
              />
            )}
          </Field>
        </FieldGroup>
      ) : null}
    </>
  );
}
