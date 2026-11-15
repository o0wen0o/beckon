// What every surface does before it paints, written once so a second pre-mount
// step cannot land in two of the three entry points and not the third.
import { createRoot } from "react-dom/client";
import type { ReactElement } from "react";
import "../globals.css";
import { startLanguage } from "./i18n";
import { startTheme } from "./theme";

/**
 * A surface is an app window that happens to be drawn by a browser, and the
 * browser's own gestures still work in it: F5 and Ctrl/Cmd+R reload the page,
 * Ctrl/Cmd+P prints it, and a right-click offers Refresh and Save as. Reloading
 * is the one that costs something — the page comes back with no memory of the
 * Exchange Rust is still holding, so a finished answer is replaced by a turn
 * "waiting for a reply" that counts for ever, and the answer is gone.
 *
 * Only the default is prevented, in the capture phase, and nothing is stopped:
 * Ctrl+Shift+R is Regenerate in the Popover, and that handler still has to see
 * the key. The context menu stays wherever it is useful rather than merely
 * present — inside a field (paste, spelling) and over selected text (copy).
 *
 * Left alone under `vite dev`, where reloading the page is the workflow.
 */
function guardAgainstTheBrowser() {
  if (import.meta.env.DEV) return;

  window.addEventListener(
    "keydown",
    (event) => {
      const key = event.key.toLowerCase();
      const command = event.ctrlKey || event.metaKey;
      if (key === "f5" || (command && (key === "r" || key === "p"))) event.preventDefault();
    },
    true,
  );

  window.addEventListener("contextmenu", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    const editable = target?.closest("input, textarea, [contenteditable='true']") != null;
    const selecting = (window.getSelection()?.toString() ?? "") !== "";
    if (!editable && !selecting) event.preventDefault();
  });
}

/**
 * Apply the stored theme and language, then render `element` into `#app`.
 *
 * Both are awaited so the window never paints one palette — or one language —
 * and then flips. They are read concurrently because neither depends on the
 * other, and they are two reads of the same config file. Launcher and Popover
 * are created hidden at startup, so the wait is paid once at launch
 * and never on the hot path.
 */
export async function mountSurface(element: ReactElement) {
  guardAgainstTheBrowser();
  await Promise.all([startTheme(), startLanguage()]);
  const root = createRoot(document.getElementById("app")!);
  root.render(element);
  return root;
}
