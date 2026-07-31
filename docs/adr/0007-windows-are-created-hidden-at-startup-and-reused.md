---
status: accepted
---

# The hot-path windows are created hidden at startup and reused, never created per trigger

The Launcher and the Popover are created — hidden — while Beckon starts. Triggering shows and
positions an existing window; closing hides it. Neither is ever created or destroyed in response to a
hotkey.

Creating a WebView costs on the order of 100ms plus process setup. A tool whose entire proposition
is "press a hotkey and the answer appears next to your cursor" cannot pay that on the hot path, and
it would pay it *twice* on the Launcher path (Launcher, then Popover).

**Settings is the exception**: it is built on first use and kept afterwards. Nothing about opening a
settings window is latency-sensitive, and a live WebView is the most expensive thing a resident tool
carries — see the measurement below.

## This appears to contradict ADR-0004, and does not

[ADR-0004](./0004-exchanges-are-never-persisted.md) says "closing the Popover destroys the
Exchange", and [CONTEXT.md](../../CONTEXT.md) says of the Popover: "Closing it destroys it." Read
naively, that asks for window destruction. What those statements are actually about is the
**lifetime of the conversation**: no history, no storage layer, nothing retained after the window
goes away.

That guarantee is about the Exchange, so it is met by destroying the Exchange. Hiding the Popover
cancels the in-flight request, drops the Exchange from the map, and clears the view state; what
remains is an empty WebView showing nothing. Nothing survives that a destroyed window would have
taken with it.

## Consequences

- **Hiding must clear state explicitly.** With a destroyed window this was free; now it is code that
  can be forgotten. The single hide path (`trigger::hide_popover`) is what makes this safe: cancel,
  discard the Exchange, clear the view, hide, restore focus — in that order, in one place.
- `CloseRequested` is intercepted for every window and turned into a hide. A user clicking the
  Settings window's X must not be able to destroy a window the app expects to still exist.
- Only one Popover exists, so a trigger while one is open cannot open a second. It cancels the
  in-flight request and replaces the contents — the docs left this undefined; this is the decision.
- **Resident memory is well over ADR-0001's ~30MB expectation, and this is why.** Measured on the
  release build with all three surfaces alive (first run, so Settings had opened): the Rust process
  is 30MB working set — the figure ADR-0001 quotes — but the WebView2 process group brings the tree
  to ~490MB working set / ~285MB private bytes across 9 processes. Working set overstates it (the
  WebView2 runtime is shared with any other WebView2 app on the machine), private bytes do not.
  ADR-0001's number described the Tauri process, not the browser it drives. Building Settings lazily
  removes one surface from the steady state; measuring the two-surface steady state needs a stored
  API key, so it is on the manual checklist rather than recorded here.
- The windows outlive any single trigger, so a shown window cannot rely on `mount` to learn what it
  is displaying. State arrives as an event plus a `get_popover_view` command, which the window
  re-reads every time it is revealed.
