// The theme is config-derived state like everything else (ADR-0003): Rust owns
// the setting, this module only maps it onto the document. Nothing here is
// remembered locally — no localStorage, no per-window copy — so the three
// surfaces cannot drift apart.

import { getConfig, onConfigChanged } from "./ipc";
import type { Theme } from "./types";

// One MediaQueryList for the module: `matchMedia` hands back a new object every
// call, so a per-call listener could never be removed again.
const systemIsDark = window.matchMedia("(prefers-color-scheme: dark)");

/** The theme currently on the document, or `null` before the first apply. */
let applied: Theme | null = null;

// Registered once and left there — it is a no-op unless the theme is `system`,
// and the windows outlive any reason to detach it (ADR-0007).
systemIsDark.addEventListener("change", () => {
  if (applied === "system") paint("system");
});

/**
 * Put `theme` on the document.
 *
 * The Windows app theme is consulted **only** for `system`. That is the whole
 * reason app.css carries no bare `prefers-color-scheme` rule: a machine set to
 * dark must still get the light default until the user asks for otherwise.
 */
function paint(theme: Theme) {
  const resolved = theme === "system" ? (systemIsDark.matches ? "dark" : "light") : theme;
  // Two stamps for one setting, for as long as the surfaces disagree about
  // their design system: `data-theme` is what src/app.css keys the Svelte
  // surfaces off, and `.dark` is the class shadcn/ui's own `dark` variant
  // matches. Both are written here so the resolution stays in one place.
  document.documentElement.dataset.theme = resolved;
  document.documentElement.classList.toggle("dark", resolved === "dark");
}

function applyTheme(theme: Theme) {
  // `config-changed` fires for every setting, not just this one; re-stamping
  // the root element would invalidate the whole document's style for nothing.
  if (theme === applied) return;
  applied = theme;
  paint(theme);
}

/**
 * Read the stored theme, apply it, and re-apply on every `config-changed`.
 *
 * Awaited before a surface mounts so the window never paints one palette and
 * then flips. The subscription is never disposed on purpose: the windows live
 * as long as the process does (ADR-0007).
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
