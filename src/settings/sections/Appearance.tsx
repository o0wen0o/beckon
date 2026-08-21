import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { Segmented } from "@/components/Segmented";
import { useT } from "@/lib/i18n";
import type { Language, Theme } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

export function Appearance() {
  const t = useT();
  const store = useStore(settings);
  const config = store.config;

  const themes = [
    { value: "light", label: t.settings.appearance.light, icon: SunIcon },
    { value: "dark", label: t.settings.appearance.dark, icon: MoonIcon },
    { value: "system", label: t.settings.appearance.system, icon: MonitorIcon },
  ] satisfies { value: Theme; label: string; icon: typeof SunIcon }[];

  // No icon set: `Segmented`'s icons are all-or-nothing, and there is no second
  // glyph that means "Chinese" the way a moon means dark. Each language names
  // itself in itself instead, which is what a reader who cannot read the
  // current one needs (ADR-0015).
  const languages = [
    { value: "en", label: t.settings.appearance.english },
    { value: "zh", label: t.settings.appearance.chinese },
  ] satisfies { value: Language; label: string }[];

  return (
    <>
      <PaneHeader title={t.settings.appearance.title}>{t.settings.appearance.lede}</PaneHeader>

      {config ? (
        <FieldGroup>
          <Field
            label={t.settings.appearance.theme}
            hint={t.settings.appearance.themeHint(t.words.systemAppearance)}
          >
            {({ id, describedBy }) => (
              <Segmented
                id={id}
                describedBy={describedBy}
                label={t.settings.appearance.theme}
                value={config.theme}
                options={themes}
                onChange={(theme) => store.editConfig((draft) => (draft.theme = theme), true)}
              />
            )}
          </Field>

          {/* Written immediately, like the theme beside it: the whole window
              re-renders in the other language, and a debounce would leave the
              two halves of one gesture visibly apart. */}
          <Field
            label={t.settings.appearance.language}
            hint={t.settings.appearance.languageHint}
          >
            {({ id, describedBy }) => (
              <Segmented
                id={id}
                describedBy={describedBy}
                label={t.settings.appearance.language}
                value={config.language}
                options={languages}
                onChange={(language) =>
                  store.editConfig((draft) => (draft.language = language), true)
                }
              />
            )}
          </Field>
        </FieldGroup>
      ) : null}
    </>
  );
}
