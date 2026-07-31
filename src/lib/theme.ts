// The theme is config-derived state like everything else (ADR-0003): Rust owns
// the setting, this module only maps it onto the document. Nothing here is
// remembered locally — no localStorage, no per-window copy — so the three
// surfaces cannot drift apart.

import { getConfig, onConfigChanged } from "./ipc";
import type { Theme } from "./types";

/** What the stylesheet understands. `system` is resolved before it gets there. */
type Painted = "light" | "dark";

const SYSTEM_IS_DARK = "(prefers-color-scheme: dark)";

/** Live only while the theme is `system`; see [applyTheme]. */
let following: ((event: MediaQueryListEvent) => void) | null = null;

function paint(scheme: Painted) {
  document.documentElement.dataset.theme = scheme;
}

/**
 * Put `theme` on the document, and keep it there.
 *
 * The Windows app theme is consulted **only** for `system`. That is the whole
 * reason app.css carries no bare `prefers-color-scheme` rule: a machine set to
 * dark must still get the light default until the user asks for otherwise.
 */
export function applyTheme(theme: Theme) {
  const query = window.matchMedia(SYSTEM_IS_DARK);
  if (following) {
    query.removeEventListener("change", following);
    following = null;
  }
  if (theme !== "system") {
    paint(theme);
    return;
  }
  following = (event) => paint(event.matches ? "dark" : "light");
  query.addEventListener("change", following);
  paint(query.matches ? "dark" : "light");
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
