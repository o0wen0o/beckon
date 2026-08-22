// A hotkey recorder that registers what it records **immediately**: if the
// combination is taken, it goes red on the spot and the value is refused
// (README). Nothing unregisterable can reach disk through this component.
import * as React from "react";
import { KeyboardIcon, XIcon } from "lucide-react";
import { Kbd } from "@/components/Kbd";
import { Button } from "@/components/ui/button";
import { describeError, probeHotkey } from "@/lib/ipc";
import { useT } from "@/lib/i18n";
import { formatAccelerator, IS_MAC } from "@/lib/platform";

interface HotkeyInputProps {
  value: string | null;
  /** Whether an empty value is allowed (Direct Hotkeys are optional). */
  clearable?: boolean;
  onChange: (accelerator: string | null) => void;
}

export function HotkeyInput({ value, clearable = false, onChange }: HotkeyInputProps) {
  const t = useT();
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

    const accelerator = toAccelerator(
      event,
      setError,
      t.controls.hotkey.needsModifier(t.words.modifierAdvice),
    );
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
      <div className="flex items-center gap-2 text-note">
        {/* The value is a chip and the affordance is a button beside it, rather
            than one bordered control doing both. A box holding a combination
            reads as a value someone typed into a field, and the thing that says
            how to change it should be the thing you press — which is also what
            keeps this row the same weight as the hotkey chips in the Actions
            list, instead of the widest control on the pane.

            `text-note` on the row rather than on each part: `Kbd` sizes itself
            at 0.92em of whatever it sits in, so the chip follows the labels down
            instead of having to be told twice. */}
        {value ? (
          <Kbd
            className={
              error !== null ? "border-destructive/60 text-destructive tabular-nums" : "tabular-nums"
            }
          >
            {formatAccelerator(value)}
          </Kbd>
        ) : null}
        <Button
          variant="ghost"
          size="xs"
          className={[
            // `font-medium` over `ghost`'s own 400: at 14px the light weight was
            // what kept a borderless button under the row label beside it, and
            // at 12px the size does that on its own.
            "min-w-20 flex-none justify-start font-medium",
            recording ? "text-primary" : "",
            error !== null && !recording ? "text-destructive" : "",
          ].join(" ")}
          onClick={() => setRecording((on) => !on)}
          onKeyDown={onKeyDown}
          onBlur={() => setRecording(false)}
        >
          <KeyboardIcon />
          {recording ? t.controls.hotkey.recording : value ? t.controls.hotkey.change : t.controls.hotkey.record}
        </Button>

        {clearable && value && !recording ? (
          <Button
            variant="ghost"
            size="xs"
            aria-label={t.controls.hotkey.clear}
            className="flex-none font-medium"
            onClick={() => {
              setError(null);
              onChange(null);
            }}
          >
            <XIcon /> {t.controls.hotkey.clearShort}
          </Button>
        ) : null}
      </div>

      {error ? <p className="text-destructive mt-1 text-note">{error}</p> : null}
    </>
  );
}

/** `noModifiers` is passed in rather than looked up: this is a pure function
 *  outside the tree, and the sentence is the catalog's (ADR-0015). */
function toAccelerator(
  event: React.KeyboardEvent,
  setError: (message: string) => void,
  noModifiers: string,
): string | null {
  const mods: string[] = [];
  if (event.ctrlKey) mods.push("Ctrl");
  if (event.altKey) mods.push("Alt");
  if (event.shiftKey) mods.push("Shift");
  // "Cmd" and "Super" are the same modifier to the parser on both platforms, so
  // this is only about which one a person reading `config.toml` expects to see.
  if (event.metaKey) mods.push(IS_MAC ? "Cmd" : "Super");

  const key = keyName(event.code);
  if (!key) return null;
  if (mods.length === 0) {
    setError(noModifiers);
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
