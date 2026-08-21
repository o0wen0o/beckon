---
status: accepted
---

# The Launcher's rows are cards on a well, and hover no longer selects

[ADR-0009](0009-launcher-and-popover-ported-to-react-svelte-removed.md) gave the Launcher a
full-bleed hairline row and made the pointer move the keyboard cursor. Both change here. A row is
now a **card** — paper, framed, inset in a gutter that is itself a `--muted` **well** — the pointer
under it strengthens its edge, and the ink fill belongs to the keyboard cursor alone.

[ADR-0012](0012-settings-pane-is-cards-not-a-ledger.md) named this decision and deferred it:
"carding the list means carding the Launcher, which is a separate decision about a window whose
whole job is one keypress." This is that decision, taken for the Launcher only. Settings' Actions
list stays ruled — see *What this costs*.

Everything else in ADR-0009 stands: the four fixed columns via `ActionCells`, the frameless window,
the keyboard as the primary path, the window dying with its focus.

## Why

The Launcher's body is nothing but this list, so the hairline row had no other structure to belong
to: five Actions read as four rules across an otherwise empty window, and the only thing that ever
looked like an object was the selected row. A frame makes each Action a thing you can point at
before you have chosen it.

A frame alone did not finish the job. Carded rows on the same paper as the window around them left
the whole surface one value from edge to edge — the complaint that opened this was simply *too
white* — and a card that shares its ground with everything else is only an outline. The window has
three parts and the body is the one that holds objects, so the body is the part that gets a ground.

The hover state is the reason this had to be more than a colour change. `ActionRow` bound
`onMouseMove={onSelect}`, so the hovered row *was* the selected row: a hover background could not be
added, because the ink fill was always over it. Any pointer affordance in this window starts by
separating the two.

## What was rejected

Four treatments were prototyped against the real window before this one:

- **ruled rows, softer fill** — keep the geometry and the mouse-selects binding, and soften the
  selected fill from ink to `--muted`. One class, no ADR. Rejected because it buys a quieter window
  by spending the cursor: the ink fill is the product's one fill and this window's one instance of
  it, and a keyboard-first picker cannot have the weakest mark on screen be its cursor.
- **cards, no inversion** — cards, with the cursor as `--muted` plus a `--border-strong` edge. Same
  objection, worse: hover and cursor then differ by one edge weight, and Enter runs the one you did
  not point at.
- **cards, no frame** — nothing drawn until the pointer arrives. Quietest at rest and closest to
  today, but a card that only exists under the pointer is a hover state, not a card; the list still
  reads as a field of text.
- **cards, frame, and hover moves the edge too** — `--muted` ground *and* `--border-strong` edge, as
  Settings' `CARD_HOVER` plus a fill. Rejected as two properties moving to say one thing, with the
  ink fill still one step above it. Hover moves one property; which one it is fell out of the ground
  decision below.

Then three treatments of the ground, prototyped against the same window in both themes:

- **tint the chrome** — header and footer take `--muted`, the list stays paper. The Spotlight
  reading, and the first thing anyone tries. Rejected because it moves the white rather than
  spending it: the body is 400 of the window's 480 pixels, so the flat expanse survives untouched
  and the two parts that were never the problem are the two that change. It also draws each
  boundary twice, tint *and* hairline — the argument `--sidebar` already carries in
  [globals.css](../../src/globals.css) for not tinting the navigation column.
- **tint the footer only** — the least intervention: the query bar is the window's subject and stays
  paper, the status strip gets a ground. Same objection, one third the effect.
- **tint the body — chosen.** The `--muted` ground goes under the list, where the cards are, and the
  chrome stays paper with its hairlines. It is the only one of the three that gives the cards
  something to be cards on, and the only one that puts the value change where the complaint was.

## The well

- **`bg-muted` on the `<ul>`, and nowhere else.** The gutter that insets the cards is the ground they
  stand on, so the tint arrives with no element of its own: one class on the list the window already
  had. The chrome above and below it keeps its hairline and takes no tint — a tint plus a border says
  one thing twice.
- **The cards go to `bg-background`.** They were transparent, which on paper was invisible and on the
  well would have been a hole. Paper on `--muted` is the same figure-and-ground the Popover's input
  card already uses, run the other way up.
- **In dark mode the well is *lighter* than the chrome** — `--muted` is 0.269 against a 0.145
  background — and the card on it is darker than both. That is not an inversion of the light-mode
  reading, it is the same one: the body is separated from the chrome, and the card is separated from
  the body. Which direction each step goes is the theme's business.
- **The empty state now has a floor.** The "no Actions yet" and "nothing matches" panels are centred
  in the list, so what used to be a void with text in it is a well with text in it.

## The row

- `rounded-md border px-4 h-13`, in a list that is `p-1.5` with `gap-1.25`. The gutter insets every
  frame from the window's own edge; the 5px gap is what keeps two adjacent frames from doubling into
  a single 2px line.
- **Hover is one property, and the well decides which.** `not-aria-selected:hover:border-border-strong`
  — the edge strengthens, the ground does not, and the description keeps its grey. `--muted` is what
  the card is standing on now, so hovering *to* `--muted` would sink the card into the list instead
  of lifting it off; the edge is the only property left that can move without a second value. It is
  the same token Settings' `CARD_HOVER` takes, which is the pane's one hover state. The `:not` is
  load-bearing: without it a pointer resting on the cursor would repaint the ink fill's frame with
  the hover edge.
- **The selected card's frame goes to `--primary`**, the fill's own colour. A filled row still
  wearing a lighter outline reads as two marks rather than one block.
- The fill itself is unchanged from ADR-0009: ink ground, paper text, and the muted greys in the
  Input Source and description columns lifted to strengths of the paper (`ActionCells`).
- `BrokenRow` is the same card. It cannot be run, but it can be clicked through to the raw editor,
  so the hover edge says the right thing about it.

## Hover no longer selects

`onMouseMove={onSelect}` is gone, and with it `ActionRow`'s `onSelect` prop. The pointer never moves
the cursor; `onClick={onRun}` already ran the row it landed on, so nothing replaces it. Hovering is
looking, clicking is picking, and the two states are now visible at once.

This costs one thing: arrowing to a row, then moving the mouse, no longer drags the cursor along.
That was never a gesture worth having in a window summoned by a hotkey and dismissed by Escape.

## The cursor is hidden until the window is touched

`wanted` is `null` on every summon rather than `0`, and no row draws the fill until the user does
something: the first arrow, or the first character typed. The ink fill says "Enter runs this", and on
a window nobody has touched yet there is no *this* — while hover selected, the fill was always under
the pointer and never had to answer for itself; standing still on row one, it does.

Two details make it behave:

- **The first arrow reveals the cursor where it already was**, rather than moving one past it: Down
  lands on the top match, Up on the last. Arrowing from a hidden cursor to row two would skip a row
  that was never on screen.
- **Typing reveals it too.** Typing re-ranks the list, so the top match becomes what Enter will run,
  and an answer Enter acts on has to be visible. Type-then-Enter is the window's main gesture.

What stays: Enter always runs `selected`, which is the top match while the cursor is hidden. So Enter
on an untouched window with an empty query runs the first Action without having marked it first. That
is the picker's contract — Enter means go — but it is the one keystroke this change leaves blind.

## What "a fill means selected" now means

ADR-0008's rule was that a fill is the pane's inversion accent and nothing else. This adds the same
exception `Segmented` already carries, and for the same reason: a `--muted` ground one register below
the inversion, scoped to the inside of an enclosure. `Segmented` is a track holding segments; the
Launcher's list is a well holding cards. In both, the muted value is the *inside of the enclosure*
rather than a mark on anything — it says "these belong together", not "this one is chosen". The rule
reads as *the inversion is the only ink fill*, and it is still the only fill the Launcher paints on a
row.

Nothing in the window hovers to a fill any more, which is what keeps this honest: the one muted
ground on screen is static, so it cannot be mistaken for a state.

## What this costs

- **One fewer Action fits** at the same window height: 52px row plus a 5px gap plus a 12px gutter.
  Shrinking the row is a separate decision, not a correction to this one.
- **The two lists are no longer one row.** ADR-0009's parity was geometry *and* columns; it is now
  columns only. `ActionCells` still owns the Input Source and Direct Hotkey cells and their widths,
  so the two lists cannot drift in what they show or where it sits — but the Launcher's row is
  carded and Settings' Actions row is still ruled. ADR-0012's argument for keeping that list ruled
  is untouched by this: it is a list of records inside a pane of cards, and carding it would put a
  card inside a pane whose one enclosure is already the card.
- **A second line runs down the window.** The card edge sits 6px inside the window border for the
  full height of the list. The well is what makes that legible rather than doubled: the two lines
  now separate different pairs of surfaces — window against desktop, card against ground — instead of
  being two rules across one continuous paper. It is also why hover can take the edge at all. The
  standing objection was that darkening the frame turns the pair into a double rule, and it still
  holds for a *resting* frame; hover darkens one card at a time, under the pointer, which is not a
  line down the window.
- **The Launcher and Settings disagree about grounds now.** Settings' pane is paper holding carded
  configurations and draws no well (ADR-0012: no fill, no bleed); the Launcher's body is a well
  holding carded records. That is the same split the two lists already carry — one is a pane, one is
  a picker whose whole body is the list — but it is one more way the two surfaces are not the same
  surface, and `ActionCells` remains the only thing holding them together.
