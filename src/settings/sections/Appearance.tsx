import { Field } from "@/components/Field";
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
  "Applies to the Launcher, the Popover and this window at once. Beckon starts light unless you say otherwise — “Follow Windows” is the only setting that reads the system preference.";

export function Appearance() {
  const store = useStore(settings);
  const config = store.config;

  return (
    <>
      <h1 className="font-display mb-6 text-xl font-semibold">Appearance</h1>

      {config ? (
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
      ) : null}
    </>
  );
}
