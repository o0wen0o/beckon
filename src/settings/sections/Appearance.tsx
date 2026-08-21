import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { Segmented } from "@/components/Segmented";
import type { Theme } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

const THEMES: { value: Theme; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "Follow Windows" },
];

const THEME_HINT =
  "Beckon starts light unless you say otherwise. “Follow Windows” is the only setting that reads the system preference, and it follows it live.";

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
