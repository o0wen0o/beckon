// Radix portals its overlays to `document.body`, which breaks Settings' save
// protocol: `saveSlot.textFocusHeld` asks whether focus is inside the pane, and
// the pane flushes the debounced write on its own `focusout`, so an overlay
// outside it makes opening a dropdown read as "the user left the form".
//
// The pane publishes itself here and `select` / `popover` default to it.
// `alert-dialog` deliberately does not: the delete confirmation is hosted by
// the shell, outside the pane.
import * as React from "react";

const PaneContext = React.createContext<HTMLElement | null>(null);

export const PaneProvider = PaneContext.Provider;

/** The element Radix overlays should portal into, or `null` for the body. */
export function usePaneContainer(): HTMLElement | null {
  return React.useContext(PaneContext);
}
