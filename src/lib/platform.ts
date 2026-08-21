// What differs between the Windows build and the macOS one and is *not* prose:
// the test itself, the command modifier as an accelerator token, and how an
// accelerator is drawn. A per-component `isMac` test is how those drift.
//
// The platform-specific *words* moved to `lib/i18n/` when the second language
// arrived (ADR-0015): "Keychain" against "Windows Credential Manager" is one
// choice, but each of them then has a Chinese form, and a `const` can only hold
// one dimension. `IS_MAC` is exported for the catalogs to branch on.
//
// The test is the user agent rather than an IPC call: this decides labels that
// are rendered on the first frame, and the webview already knows the answer
// synchronously. Rust stays the source of truth for everything that is *state*
// (ADR-0013) — the input permission is a command, not a guess.

import type * as React from "react";

export const IS_MAC = /\bMac(intosh| OS X)\b/.test(navigator.userAgent);

/** The modifier the two window shortcuts use, as an accelerator token — the
 *  glyph is `formatAccelerator`'s to draw, so ⌘ stays in one place. A token, not
 *  a word: it is parsed, and it is the same in both languages. */
export const COMMAND_MODIFIER = IS_MAC ? "Cmd" : "Ctrl";

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
  const isModifier = (token: string) => GLYPHS.some(([pattern]) => pattern.test(token));

  // Walked in GLYPHS' order rather than the accelerator's, so "Shift+Cmd+T" and
  // "Cmd+Shift+T" draw alike without a second table to sort against.
  const modifiers = GLYPHS.filter(([pattern]) => tokens.some((token) => pattern.test(token))).map(
    ([, glyph]) => glyph,
  );
  const keys = tokens.filter((token) => token && !isModifier(token));
  return modifiers.join("") + keys.join("+");
}
