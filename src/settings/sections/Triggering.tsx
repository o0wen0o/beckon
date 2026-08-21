import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { HotkeyInput } from "@/components/HotkeyInput";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { PaneHeader } from "@/components/PaneHeader";
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

      {config ? (
        <FieldGroup title="Summoning">
          <Field
            label="Launcher hotkey"
            hint="If the combination is already taken it goes red and is not saved."
          >
            {() => <HotkeyInput value={config.launcher_hotkey} onChange={setLauncherHotkey} />}
          </Field>

          <Field
            label="Start with Windows"
            hint="Beckon lives in the tray; starting with Windows is the point."
          >
            {({ id, describedBy }) => (
              <OnOffSwitch
                id={id}
                describedBy={describedBy}
                label="Start with Windows"
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
