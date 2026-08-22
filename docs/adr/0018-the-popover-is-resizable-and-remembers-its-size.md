---
status: accepted
---

# The Popover is resizable, and the size it is left at is remembered

620×500 was a fixed rect. Every layout decision in the Popover was made against it — the card capped
at 4/5 of the width ([ADR-0010](./0010-popover-turns-are-sided-not-ruled.md)), the composer that must
not grow with each attachment, the rail rather than a row per Capture
([ADR-0017](./0017-a-turn-carries-several-captures-and-preview-is-a-layer.md)) — and it stays the
size those arguments are about. What changes is that it is now a *default* rather than the only
value: the window can be dragged to any size between 380×200 and 3840×2160, and the size it is left
at is what the next summon opens at.

## Why

A screenshot is the case that broke it. `ADR-0017`'s preview takes the whole window because the
window is all the screen Beckon owns — but 620×500 minus a title bar and a dot strip is a 604×428
box, and a full-screen snip fitted into it is a third of its real size. Reading anything in it means
opening the file somewhere else, which is the workflow the Capture existed to avoid. Nothing about a
fixed window makes that better; only a bigger one does.

The second case is prose. An Action pointed at a long paragraph produces an answer taller
than the window twice over, and a user who wants to read it beside the source has a scroll wheel and
nothing else.

## Why config and not window state

The window is created hidden at startup and re-sized on every trigger (ADR-0007). A size held only
by the window would therefore survive exactly until the next hotkey press, which is the same as not
remembering it. Remembering it at all means writing it to the file
[ADR-0003](./0003-rust-owns-state-webview-renders.md) makes authoritative — `[popover] width/height`
in `config.toml`, through `reload`'s one funnel like every other write, so the watcher swallows the
echo and every window hears the new snapshot.

Logical pixels, not physical: the same file has to mean the same window on a 100% monitor and a 150%
one, and logical is what `set_size` takes.

It is deliberately *not* in Settings' UI. The gesture that sets it is dragging the window, the number
is the result rather than the input, and a pane full of controls does not need a width field whose
only honest label is "the size you last left the Popover".

## Telling our own resize from the user's

Every resize reports itself, including the `set_size` at the start of each trigger. Persisting
reports indiscriminately would write our own summon back as if the user had dragged to it, so a
clamp or a rounding difference would walk the remembered size a pixel at a time. (When this was
written there was a louder version of the same bug: the `empty-selection` window was 220px on
purpose, and one of those would have shrunk every later Popover. That phase is gone under
[ADR-0020](0020-the-input-source-loses-its-selection-only-arm.md); the mechanism below is unchanged
and still necessary.)

So `AppState::popover_asked_size` records the size the window was last *told* to be, and
`remember_popover_size` drops any report that matches it (to the pixel: the round trip through
physical pixels at a fractional scale factor does not come back exact). What is left is a size the
user produced.

Two consequences worth naming:

- `MIN_POPOVER_H` is **200**, under the 220px hint height, not the 240 the composer would like. A
  floor above the shortest window the product shows itself is a floor `set_size` cannot meet, and the
  window manager clamping our own call would look exactly like a drag.
- the hint height is now a *ceiling* rather than a height: `min(remembered, 220)`. A Popover the user
  has already made shorter than the hint stays that short, because a hint is not a reason to make a
  window bigger than it was asked to be.

## Grips, because an undecorated window has no border

The Popover is frameless, so there is nothing for the OS to hit-test: `resizable: true` alone buys
nothing on either platform. Eight 4px strips inside the card's edge (`ResizeGrips`) hand the press to
`startResizeDragging`, and the window manager owns the drag from there — no pointer-following code
here, and no size set from the frontend at all.

They are invisible, and `aria-hidden`:

- a visible frame drawn inside a frameless card is a second edge beside the one the card has. The
  cursor over the edge is the affordance, which is where a user already looks for it;
- a drag has no keyboard equivalent to announce, and eight nameless strips read out in a window
  driven by Esc and the arrows is worse than silence. Unlike the rail's remove button
  ([ADR-0017](./0017-a-turn-carries-several-captures-and-preview-is-a-layer.md)), this is not a
  control with a keyboard path that hover was hiding — it is a pointer gesture, performed once,
  whose result is remembered.

The strips sit at `z-60`, above the preview layer: the window stays resizable while a screenshot is
being looked at in it, which is the case this ADR exists for.

## The write is debounced in the window, not in Rust

A drag reports every pixel. The Popover waits 400ms of quiet before it reports the size, so one drag
is one write. Rust stays a plain command with no timer in it — the debounce belongs beside the event
that needs it, and the command is idempotent either way (a size equal to the stored one returns
without writing).

## What this does not change

- **The default.** 620×500 out of the box, mirrored in `tauri.conf.json` so the first paint is not at
  the wrong size, and it is still the size every layout argument in the Popover is made against.
- **Cursor-adjacent placement.** `place_near_cursor` already took the size as an argument and clamps
  to the work area; a bigger window flips sooner and nothing else.
- **The Launcher.** Still fixed at 680×480 and still `resizable: false`: it is a picker whose height
  is its match list, and a taller one shows more of nothing.
- **Nothing is stored about an Exchange** (ADR-0004). A window size is not an Exchange.
