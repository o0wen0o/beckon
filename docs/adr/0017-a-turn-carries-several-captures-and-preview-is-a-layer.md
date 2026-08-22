---
status: accepted
---

# A turn carries up to four Captures, and looking at one is a layer over the Popover

[ADR-0016](./0016-captures-from-the-os-snip-tool-via-the-clipboard.md) attached exactly one Capture
to a turn. This extends that to a small ordered list and gives the Popover somewhere to *look* at
one: a rail of thumbnails in the composer, and a preview that covers the window.

It does not supersede 0016 — where the bytes come from, when they are sent, and why the clipboard is
not restored are all unchanged. What changes is the arity and the fact that a thumbnail is now a
button.

## Why more than one

The question people bring to a screenshot is often comparative — "which of these two dialogs is
wrong", "does this match the mock" — and with one slot per turn the only way to ask it is to snip a
region wide enough to hold both, at whatever resolution that leaves. Two Captures cost two parts in
one message and the provider reads them in order, so the note beside them can say "the first one".

## Four, and the ceiling is about the history, not the wire

`state::MAX_CAPTURES` is 4. The wire would take six — 8 MiB per image ([ADR-0016](./0016-captures-from-the-os-snip-tool-via-the-clipboard.md)'s
stricter-than-the-provider cap) inside a 48 MiB body — so the binding constraint is
[ADR-0004](./0004-exchanges-are-never-persisted.md) instead: the history is resent untruncated on
every follow-up, so each attachment is paid for again by every later turn in the Exchange.

A snip taken with the tray full is refused as `capture-too-many` and **keeps what is already
attached**. That is the same shape of answer as `capture-too-large`: bytes exist and cannot be sent,
so it is an error beside the tray rather than instead of it. A cancelled snip stays what it was — not
an error, and it leaves the tray alone.

## The list is Rust's, and the window names an index

The Captures live in `PopoverView`, for the reason the single one did (ADR-0003): the request is
built from those bytes and the thumbnails are drawn from the same ones, so one owner means the
picture on screen cannot differ from the picture that was sent.

A Capture has no identity of its own — it is bytes, and two snips of the same region are equal — so
the remove button names a **position**. That is safe because of two facts together: the list only
ever grows at the end, and only the Popover shrinks it. An index the window rendered a moment ago
therefore still names the tile the user clicked, and an out-of-range index is a no-op rather than a
panic (`PopoverView::remove_capture`).

## The rail, not a list of rows

The composer shows one sideways-scrolled rail of equal square tiles with a single line of prose
about the set ("3 screenshots · 785 KB total"). The alternative — one row per Capture, each with its
own thumbnail and size — was built and driven side by side with this, and it loses on the constraint
that matters: the Popover is 620×500 until the user drags it bigger
([ADR-0018](./0018-the-popover-is-resizable-and-remembers-its-size.md)), and the composer shares it
with the conversation the screenshots are being asked about. Rows grow the composer with every attachment; a rail does not.
The per-image size is one click away instead of on screen four times.

Consequences of the rail:

- tiles are `object-cover`, so the shape of any one Capture is not readable from the rail. That is
  the trade the preview pays back;
- the remove button appears on hover *and* on focus, because a rail whose only affordance is hover
  is unreachable from the keyboard the Popover is usually driven from.

## Preview is a layer over the window, not a route or a bigger thumbnail

A 1920×1080 snip is not legible at any size a 620px column can give it, and the window is all the
screen Beckon owns — so the preview takes the window: `absolute inset-0` over the Popover card,
title bar included, at 97% opacity so it still reads as something you are *inside*.
([ADR-0018](./0018-the-popover-is-resizable-and-remembers-its-size.md) is the other half of this
argument: the window itself can now be made bigger, which is the only thing that makes a full-screen
snip legible rather than merely whole.)

**The ground is grey, in both themes.** A screenshot is mostly pale and has no border of its own: on
the window's own background its edges are simply not there, and "the image ends here" is the one
thing a viewer has to say. So the layer paints `--scrim`, a token added for this and consumed only
here — light `0.9`, dark `0.205`, the same grey in the same role in both palettes rather than one
theme's colours borrowed by the other.

It is not `--muted`, the obvious candidate: that is `--accent` to the pixel in both modes, so a
preview painted with it would leave its own arrows with an invisible hover. It is not the dark
palette scoped to the layer either — that gets a grey and re-derives the chrome in one move, but it
means a light-themed window with a dark panel inside it, and the product has one palette at a time.
What the scrim does cost is two rungs of the type scale: `--muted-quiet` is 4.65:1 on the background
and clears no grey ground at all, so the preview's counter and its dimensions line sit one step
brighter than the same pair would anywhere else. `globals.css` carries the ratios.

**Clicking the ground closes it.** Esc is the way out for the keyboard the Popover is usually driven
from, but the preview is opened by clicking a thumbnail, and a layer opened by the pointer needs a
pointer way out that is not a 24px button in a corner. The scrim boxes name themselves — a click
closes only when it lands on one of them rather than on something inside it (`target ===
currentTarget`), so a control added later cannot silently become a second close button. The title bar
is deliberately *not* scrim: it is the row that holds the close button, and a bar that dismisses on a
near-miss is a bar you cannot aim at.

The alternatives, and why not:

- **a bigger thumbnail in the card** — the card is capped at 4/5 of a 620px window on purpose
  ([ADR-0009](./0009-launcher-and-popover-ported-to-react-svelte-removed.md)'s register argument, restated in
  [ADR-0014](./0014-launcher-rows-are-cards.md)): a bubble reaching both edges stops reading as one
  side of a conversation;
- **replacing the scroller** — the conversation disappearing is a stronger claim than "look at this
  for a second", and it puts the composer's own tiles on screen beside the preview's, which is the
  same set twice;
- **a new window** — a second window to place, size and re-focus, for something that closes on Esc.

The preview is frontend-only state (`ExchangeStore.preview`), like `reasoningOpen`: nothing about
looking at an image changes what would be sent, so Rust has no opinion about it. It carries the *set*
it is walking, because the pending tray and a sent turn are different sets and the arrows must not
cross between them.

## Shown whole means capped on both axes

The image is capped at `max-h-full max-w-full` inside a `min-w-0` wrapper, which is one line of CSS
with a bug behind it: a replaced element used directly as a flex item takes an automatic minimum
width from its own aspect ratio, so a 1920×1080 snip in a 428px-tall row claimed 761px of width — 141
more than the whole window — and hung off the right-hand edge with the next-Capture arrow behind it.
`object-contain` did not save it: the *box* was oversized, and contain only fits the picture inside
the box. The wrapper is the flex item now, and the image is a plain in-flow element whose two caps
give it the window's shape or its own, whichever is smaller.

## Fit is what it opens at; zoom is the other state

Fit answers "is the whole thing there". It does not answer "what does that line say" — a 1920×1080
snip fitted into the window is a third of its real size, and the text in it is usually the reason the
screenshot was taken. So the viewport has two states, and which one it is in decides which box the
image is in:

- **fitted** is `scale === null`, the layout above, in an `overflow-hidden` box. `null` rather than
  the fit ratio is what makes a window resize free
  ([ADR-0018](./0018-the-popover-is-resizable-and-remembers-its-size.md)): the fitted state names no
  number, so there is nothing to recompute;
- **zoomed** is a stated pixel size in an `overflow-auto` box, floored at fit and capped at 4× the
  image's own pixels — past that a screenshot is interpolation rather than information.

The scrolling box has **no scrollbar** (`scrollbar-width: none`, plus the `-webkit-` pseudo-element
for a WKWebView too old to honour it). Not a cosmetic choice: WebView2 draws classic bars, which take
layout space, and the box measured 509×413 with them against 524×428 without — so the picture would
shrink the moment it was zoomed, and `fitScale` would measure a box 15px narrower than the one the
fitted state lays out, putting the zoom floor 3% below the actual fit. Panning is how a zoomed image
is moved around; the bar was never the control.

The gestures are the wheel (continuous, about the middle of the view), a click (fit ↔ the image's own
pixels, so one click is the size the text was rendered at), and a drag to pan. Nothing is drawn for
any of them: the layer is already a title bar, two arrows and a dot strip over the picture, and a
zoom widget on top of that is more chrome than image. The cursor says which gesture is live, and the
title bar reads the percentage back — **only while zoomed**, because "100%" beside an image that is
not at 100% is worse than saying nothing.

Two details that are bugs if you get them wrong:

- the image centres with `m-auto`, not `justify-center`. Centring a child larger than its scroll
  container pushes the overflow off the *start* edge, where no amount of scrolling reaches it;
- the wheel listener is attached natively rather than as `onWheel`. React registers that one
  passively at the root, where it cannot cancel the scroll it would otherwise perform;
- the wheel reads the scale it is changing through the setter's *updater* form, not out of the render
  closure. A trackpad delivers several notches inside one frame and React has not re-rendered between
  them, so a closure read spends them all on the same starting value;
- the click a pan ends with is swallowed at the window in the capture phase, not merely ignored by the
  image's own handler. A press that starts on the image usually ends off it, and the click is then
  delivered to the nearest common ancestor — which is scrim, so the drag closed the preview it was
  panning. Measured both ways: unguarded, a drag released outside the image closed the layer.

Zoom is component state, not `ExchangeStore`'s: a wheel notch is not something the Exchange has an
opinion about, and it would publish to the whole window several times a second. It resets on
*stepping* to another Capture — a different image is a different fit — and needs no reset on close,
because the layer is unmounted.

Pointer-only, and deliberately: a wheel and a drag have no keyboard equivalent to bind, the preview
is opened by clicking a thumbnail in the first place, and Esc still closes the whole layer, which is
what makes an unfamiliar gesture safe to try. This is the same trade as the resize grips (ADR-0018).

## Esc now means three things, in order

The Popover's one key handler decides: close the preview, else cancel a live request, else close the
window. Nearest-layer-first is the only order in which each of the three is reachable, and it is why
the preview does not register a handler of its own — two handlers for one key is how the order
drifts.

`←` / `→` walk the set, and are guarded on the preview being up: unguarded they are the caret's keys
in the composer below.

## What this does not change

- **Sending still consumes the tray.** All of it, at once, so a follow-up cannot resend it
  (ADR-0016). A retry replays the exact message that failed, images included.
- **The wire shape.** `Content::with_images` emits the one text part followed by the images in order;
  no images at all is still a bare string, which is what an endpoint that predates content parts
  accepts.
- **Nothing about which models read images.** Still the endpoint's answer to give.
- **Nothing is stored.** The Captures die with the window (ADR-0004), preview included.
