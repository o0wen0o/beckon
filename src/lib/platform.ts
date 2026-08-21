// The handful of things that differ between the Windows build and the macOS
// one, in one place — the same reason `inputSource.ts` exists. Three surfaces
// render these words and a hotkey chip appears in two lists; a per-component
// `isMac` test is how they drift.
//
// The test is the user agent rather than an IPC call: this decides labels that
// are rendered on the first frame, and the webview already knows the answer
// synchronously. Rust stays the source of truth for everything that is *state*
// (ADR-0013) — the input permission below is a command, not a guess.

import type * as React from "react";

export const IS_MAC = /\bMac(intosh| OS X)\b/.test(navigator.userAgent);

/** What the OS calls the place the API key is kept (ADR-0005). */
export const CREDENTIAL_STORE = IS_MAC ? "Keychain" : "Windows Credential Manager";

/** Where Beckon sits when no window is open. */
export const TRAY = IS_MAC ? "menu bar" : "tray";

/** The autostart switch's name, which is the platform's own phrase for it. */
export const AUTOSTART_LABEL = IS_MAC ? "Start at login" : "Start with Windows";

/** What `theme = "system"` follows. */
export const SYSTEM_APPEARANCE = IS_MAC ? "macOS appearance" : "Windows preference";

/** The modifier the two window shortcuts use: Cmd on macOS, Ctrl elsewhere. */
export const COMMAND_KEY = IS_MAC ? "⌘" : "Ctrl+";

/** True when the event carries this platform's command modifier. */
export function hasCommandModifier(event: KeyboardEvent | React.KeyboardEvent) {
  return IS_MAC ? event.metaKey : event.ctrlKey;
}

/** In macOS order — Control, Option, Shift, Command — which is not the order
 *  an accelerator string is written in. */
const GLYPHS: [RegExp, string][] = [
  [/^(control|ctrl)$/i, "⌃"],
  [/^(alt|option|opt)$/i, "⌥"],
  [/^shift$/i, "⇧"],
  [/^(cmd|command|super|meta|win)$/i, "⌘"],
];

/**
 * How a stored accelerator is drawn. On Windows that is verbatim — the string
 * in `config.toml` is already what the platform writes on a menu. macOS spells
 * the same combination in glyphs with no separators, and in its own order, so
 * the chip has to be rebuilt rather than substituted into.
 */
export function formatAccelerator(accelerator: string): string {
  if (!IS_MAC) return accelerator;

  const tokens = accelerator.split("+").map((token) => token.trim());
  const modifiers: string[] = [];
  const keys: string[] = [];

  for (const token of tokens) {
    const glyph = GLYPHS.find(([pattern]) => pattern.test(token));
    if (glyph) modifiers.push(glyph[1]);
    else if (token) keys.push(token);
  }

  // Sorted by GLYPHS' own order, so "Shift+Cmd+T" and "Cmd+Shift+T" draw alike.
  const order = GLYPHS.map(([, glyph]) => glyph);
  modifiers.sort((a, b) => order.indexOf(a) - order.indexOf(b));
  return modifiers.join("") + keys.join("+");
}
