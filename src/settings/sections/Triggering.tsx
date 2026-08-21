import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { HotkeyInput } from "@/components/HotkeyInput";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { PaneHeader } from "@/components/PaneHeader";
import { Button } from "@/components/ui/button";
import { useT } from "@/lib/i18n";
import { openInputPermissionSettings } from "@/lib/ipc";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

export function Triggering() {
  const t = useT();
  const store = useStore(settings);
  const config = store.config;
  const autostartLabel = t.words.autostart;

  function setLauncherHotkey(accelerator: string | null) {
    if (!accelerator) return;
    store.editConfig((draft) => (draft.launcher_hotkey = accelerator), true);
  }

  return (
    <>
      <PaneHeader title={t.settings.triggering.title}>{t.settings.triggering.lede}</PaneHeader>

      {store.startupErrors.length > 0 ? (
        <Callout tone="danger">
          <p>
            <strong>{t.settings.triggering.hotkeyDeadLead}</strong>
          </p>
          <ul>
            {store.startupErrors.map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
          <p>{t.settings.triggering.hotkeyDeadBody}</p>
        </Callout>
      ) : null}

      {/* The one thing a hotkey can be registered for and still do nothing:
          macOS drops the synthetic copy silently without Accessibility trust,
          so an Action reading the Selection returns an empty grab and looks
          broken (ADR-0013). `not-required` — every Windows run — says nothing
          at all, and neither does `null`, which is only "not asked yet". */}
      {store.inputPermission === "denied" ? (
        <Callout tone="danger">
          <p>
            <strong>{t.settings.triggering.permissionLead}</strong>
            {t.settings.triggering.permissionBody}
          </p>
          <p>{t.settings.triggering.permissionStillWorks}</p>
          <p>
            <Button
              variant="link"
              onClick={() => void openInputPermissionSettings()}
            >
              {t.settings.triggering.openAccessibility}
            </Button>
          </p>
        </Callout>
      ) : null}

      {config ? (
        <FieldGroup title={t.settings.triggering.summoning}>
          <Field
            label={t.settings.triggering.launcherHotkey}
            hint={t.settings.triggering.launcherHotkeyHint}
          >
            {() => <HotkeyInput value={config.launcher_hotkey} onChange={setLauncherHotkey} />}
          </Field>

          <Field label={autostartLabel} hint={t.settings.triggering.autostartHint(t.words.tray)}>
            {({ id, describedBy }) => (
              <OnOffSwitch
                id={id}
                describedBy={describedBy}
                label={autostartLabel}
                checked={config.autostart}
                onChange={(on) => store.editConfig((draft) => (draft.autostart = on), true)}
              />
            )}
          </Field>
        </FieldGroup>
      ) : null}
    </>
  );
}
