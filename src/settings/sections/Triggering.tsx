import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { HotkeyInput } from "@/components/HotkeyInput";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { PaneHeader } from "@/components/PaneHeader";
import { Button } from "@/components/ui/button";
import { openInputPermissionSettings } from "@/lib/ipc";
import { AUTOSTART_LABEL, TRAY } from "@/lib/platform";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

export function Triggering() {
  const store = useStore(settings);
  const config = store.config;

  function setLauncherHotkey(accelerator: string | null) {
    if (!accelerator) return;
    store.editConfig((draft) => (draft.launcher_hotkey = accelerator), true);
  }

  return (
    <>
      <PaneHeader title="Triggering">
        How Beckon is summoned. Every hotkey is registered the moment you record it.
      </PaneHeader>

      {store.startupErrors.length > 0 ? (
        <Callout tone="danger">
          <p>
            <strong>A hotkey is not active.</strong>
          </p>
          <ul>
            {store.startupErrors.map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
          <p>Record a different combination below; it is registered the moment you record it.</p>
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
            <strong>Beckon cannot read the Selection.</strong> Grabbing it means sending a Cmd+C to
            whatever is in front, and macOS allows that only for an app you have trusted under
            Privacy &amp; Security → Accessibility.
          </p>
          <p>
            Hotkeys still fire and Actions that ask you to type still work. Turn Beckon on in the
            list, then come back to this window.
          </p>
          <p>
            <Button
              variant="link"
              className="h-auto p-0 underline"
              onClick={() => void openInputPermissionSettings()}
            >
              Open Accessibility settings
            </Button>
          </p>
        </Callout>
      ) : null}

      {config ? (
        <FieldGroup title="Summoning">
          <Field
            label="Launcher hotkey"
            hint="If the combination is already taken it goes red and is not saved."
          >
            {() => <HotkeyInput value={config.launcher_hotkey} onChange={setLauncherHotkey} />}
          </Field>

          <Field
            label={AUTOSTART_LABEL}
            hint={`Beckon lives in the ${TRAY}; starting with the machine is the point.`}
          >
            {({ id, describedBy }) => (
              <OnOffSwitch
                id={id}
                describedBy={describedBy}
                label={AUTOSTART_LABEL}
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
