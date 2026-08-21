import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { HotkeyInput } from "@/components/HotkeyInput";
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
      <h1 className="font-display mb-6 text-xl font-semibold">Triggering</h1>

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
        <>
          <Field
            label="Launcher hotkey"
            hint="Recorded hotkeys are registered immediately — if the combination is taken it goes red and is not saved."
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
                  className="text-muted-foreground font-small min-w-9 text-left text-xs"
                >
                  {config.autostart ? "On" : "Off"}
                </span>
              </div>
            )}
          </Field>
        </>
      ) : null}
    </>
  );
}
