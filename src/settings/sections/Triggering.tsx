import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { HotkeyInput } from "@/components/HotkeyInput";
import { PaneHeader } from "@/components/PaneHeader";
import { Switch } from "@/components/ui/switch";
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
              // A switch rather than a checkbox: these settings take effect
              // immediately, and a checkbox reads as "will be applied when you
              // save" — which there is no way to do here (ADR-0003).
              <div className="flex items-center gap-2 self-start">
                <Switch
                  id={id}
                  aria-describedby={describedBy}
                  aria-label="Start with Windows"
                  checked={config.autostart}
                  onCheckedChange={(on) =>
                    store.editConfig((draft) => (draft.autostart = on), true)
                  }
                />
                {/* aria-hidden: the switch already announces checked, and a
                    screen reader reading "On" after "on" is noise. Fixed width,
                    or the row twitches every time it is thrown. */}
                <span
                  aria-hidden
                  className="text-muted-foreground min-w-5.5 text-left text-meta"
                >
                  {config.autostart ? "On" : "Off"}
                </span>
              </div>
            )}
          </Field>
        </FieldGroup>
      ) : null}
    </>
  );
}
