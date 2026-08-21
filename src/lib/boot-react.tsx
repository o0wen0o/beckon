// What a React surface does before it paints, mirroring `boot.ts` for the
// Svelte ones. Written separately rather than branching inside one function
// because the two pull in different stylesheets, and a surface must not ship
// the other framework's design system.
import { createRoot } from "react-dom/client";
import type { ReactElement } from "react";
import "../globals.css";
import { startTheme } from "./theme";

/**
 * Apply the stored theme, then render `element` into `#app`.
 *
 * The theme is awaited so the window never paints one palette and then flips.
 */
export async function mountReactSurface(element: ReactElement) {
  await startTheme();
  const root = createRoot(document.getElementById("app")!);
  root.render(element);
  return root;
}
