// What every surface does before it paints. Written once so a second pre-mount
// step — and the reasoning for awaiting it — cannot land in two of the three
// entry points and not the third.
import { createRoot } from "react-dom/client";
import type { ReactElement } from "react";
import "../globals.css";
import { startTheme } from "./theme";

/**
 * Apply the stored theme, then render `element` into `#app`.
 *
 * The theme is awaited so the window never paints one palette and then flips.
 * Launcher and Popover are created hidden at startup (ADR-0007), so that wait
 * is paid once at launch and never on the hot path.
 */
export async function mountSurface(element: ReactElement) {
  await startTheme();
  const root = createRoot(document.getElementById("app")!);
  root.render(element);
  return root;
}
