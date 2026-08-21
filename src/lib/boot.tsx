// What every surface does before it paints, written once so a second pre-mount
// step cannot land in two of the three entry points and not the third.
import { createRoot } from "react-dom/client";
import type { ReactElement } from "react";
import "../globals.css";
import { startLanguage } from "./i18n";
import { startTheme } from "./theme";

/**
 * Apply the stored theme and language, then render `element` into `#app`.
 *
 * Both are awaited so the window never paints one palette — or one language —
 * and then flips. They are read concurrently because neither depends on the
 * other, and they are two reads of the same config file. Launcher and Popover
 * are created hidden at startup (ADR-0007), so the wait is paid once at launch
 * and never on the hot path.
 */
export async function mountSurface(element: ReactElement) {
  await Promise.all([startTheme(), startLanguage()]);
  const root = createRoot(document.getElementById("app")!);
  root.render(element);
  return root;
}
