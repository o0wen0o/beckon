// Radix portals its overlays to `document.body`. That is fine anywhere except
// inside Settings, whose whole save protocol is written in terms of one element:
// `saveSlot.textFocusHeld` asks whether focus is inside the pane, and the pane
// flushes the debounced write on its own `focusout`. An overlay rendered outside
// the pane makes opening a dropdown read as "the user left the form".
//
// So the pane publishes itself here, and the two portalling components Settings
// uses — `select` and `popover` — default their container to it. `alert-dialog`
// deliberately does not: the delete confirmation is hosted by the shell, outside
// the pane, exactly as the old native `<dialog>` was.
import * as React from "react";

const PaneContext = React.createContext<HTMLElement | null>(null);

export const PaneProvider = PaneContext.Provider;

/** The element Radix overlays should portal into, or `null` for the body. */
export function usePaneContainer(): HTMLElement | null {
  return React.useContext(PaneContext);
}
