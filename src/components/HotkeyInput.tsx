// A hotkey recorder that registers what it records **immediately**: if the
// combination is taken, it goes red on the spot and the value is refused
// (README). Nothing unregisterable can reach disk through this component.
import * as React from "react";
import { KeyboardIcon, XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { describeError, probeHotkey } from "@/lib/ipc";

interface HotkeyInputProps {
  value: string | null;
  /** Whether an empty value is allowed (Direct Hotkeys are optional). */
  clearable?: boolean;
  onChange: (accelerator: string | null) => void;
}

export function HotkeyInput({ value, clearable = false, onChange }: HotkeyInputProps) {
  const [recording, setRecording] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  async function onKeyDown(event: React.KeyboardEvent) {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      setRecording(false);
      return;
    }

    const accelerator = toAccelerator(event, setError);
    if (!accelerator) return; // modifiers only so far — keep listening

    try {
      await probeHotkey(accelerator);
      setError(null);
      setRecording(false);
      onChange(accelerator);
    } catch (failure) {
      // Stay in recording mode: the user's next attempt should just work.
      setError(describeError(failure).message);
    }
  }

  return (
    <>
      <div className="flex items-center gap-2">
        {/* The value is a chip and the affordance is a button beside it, rather
            than one bordered control doing both. A box holding a combination
            reads as a value someone typed into a field, and the thing that says
            how to change it should be the thing you press — which is also what
            keeps this row the same weight as the hotkey chips in the Actions
            list, instead of the widest control on the pane. */}
        {value ? (
          <kbd
            className={`bg-muted font-mono rounded border px-1.5 py-0.5 text-meta tabular-nums ${
              error !== null ? "border-destructive/60 text-destructive" : "text-muted-foreground"
            }`}
          >
            {value}
          </kbd>
        ) : null}
        <Button
          variant="ghost"
          size="sm"
          className={[
            "min-w-24 flex-none justify-start",
            recording ? "text-primary" : "",
            error !== null && !recording ? "text-destructive" : "",
          ].join(" ")}
          onClick={() => setRecording((on) => !on)}
          onKeyDown={onKeyDown}
          onBlur={() => setRecording(false)}
        >
          <KeyboardIcon className="size-3.5" />
          {recording ? "Press keys…" : value ? "Change…" : "Record…"}
        </Button>

        {clearable && value && !recording ? (
          <Button
            variant="ghost"
            size="sm"
            aria-label="Clear the Direct Hotkey"
            className="flex-none"
            onClick={() => {
              setError(null);
              onChange(null);
            }}
          >
            <XIcon className="size-3.5" /> Clear
          </Button>
        ) : null}
      </div>

      {error ? <p className="text-destructive mt-1 text-note">{error}</p> : null}
    </>
  );
}

function toAccelerator(
  event: React.KeyboardEvent,
  setError: (message: string) => void,
): string | null {
  const mods: string[] = [];
  if (event.ctrlKey) mods.push("Ctrl");
  if (event.altKey) mods.push("Alt");
  if (event.shiftKey) mods.push("Shift");
  if (event.metaKey) mods.push("Super");

  const key = keyName(event.code);
  if (!key) return null;
  if (mods.length === 0) {
    setError("Add Ctrl, Alt or Shift — a bare key would fire everywhere.");
    return null;
  }
  return [...mods, key].join("+");
}

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (/^(Control|Shift|Alt|Meta|OS)(Left|Right)$/.test(code)) return null;
  const known = [
    "Space",
    "Enter",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Comma",
    "Period",
    "Slash",
    "Semicolon",
    "Quote",
    "Backquote",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Minus",
    "Equal",
  ];
  return known.includes(code) ? code : null;
}
