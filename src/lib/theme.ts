// The theme is config-derived state like everything else (ADR-0003): Rust owns
// the setting, this module only maps it onto the document. Nothing is
// remembered locally, so the three surfaces cannot drift apart.

import { getConfig, onConfigChanged } from "./ipc";
import type { Theme } from "./types";

// One MediaQueryList for the module: `matchMedia` returns a new object every
// call, so a per-call listener could never be removed again.
const systemIsDark = window.matchMedia("(prefers-color-scheme: dark)");

/** The theme currently on the document, or `null` before the first apply. */
let applied: Theme | null = null;

// Registered once and left there: a no-op unless the theme is `system`, and the
// windows outlive any reason to detach it (ADR-0007). `prefers-color-scheme`
// is the webview's own signal on both platforms, so nothing here is per-OS.
systemIsDark.addEventListener("change", () => {
  if (applied === "system") paint("system");
});

/**
 * Put `theme` on the document.
 *
 * The OS appearance is consulted **only** for `system` — which is why
 * globals.css carries no bare `prefers-color-scheme` rule: a machine set to
 * dark still gets the light default until the user asks otherwise.
 */
function paint(theme: Theme) {
  const resolved = theme === "system" ? (systemIsDark.matches ? "dark" : "light") : theme;
  // `.dark` is the class shadcn/ui's own `dark` variant matches.
  document.documentElement.classList.toggle("dark", resolved === "dark");
}

function applyTheme(theme: Theme) {
  // `config-changed` fires for every setting; re-stamping the root element
  // would invalidate the whole document's style for nothing.
  if (theme === applied) return;
  applied = theme;
  paint(theme);
}

/**
 * Read the stored theme, apply it, and re-apply on every `config-changed`.
 *
 * Awaited before a surface mounts so the window never paints one palette and
 * then flips. The subscription is never disposed: the windows live as long as
 * the process does (ADR-0007).
 */
export async function startTheme(): Promise<void> {
  let theme: Theme = "light";
  try {
    theme = (await getConfig()).theme;
  } catch (error) {
    // Losing the theme must not stop a surface from mounting.
    console.warn("could not read the theme; falling back to light", error);
  }
  applyTheme(theme);
  void onConfigChanged((config) => applyTheme(config.theme));
}
