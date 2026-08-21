---
status: accepted
---

# The Settings pane is cards, not a ledger

[ADR-0008](0008-settings-is-react-with-shadcn-ui-and-tailwind.md) made the pane a **ledger**: a
fixed right-aligned label column against a value column, a hairline per row, groups under tracked
micro-labels. That is replaced. One configuration is now one **card** — the name and its
explanation on the left, the control at the card's right edge — and the pane draws no hairlines at
all. Everything else in ADR-0008 stands: the tokens, the inversion accent, the two faces, the type
scale, the 150–200ms curve, and the rule that a component names no colour, size or duration of its
own.

[ADR-0011](0011-action-model-overrides-are-ledger-rows.md) also stands, with its rows now cards.

## Why

The ledger drew a hairline to close every row *and* one under every group head, so the line count
was roughly rows + groups. On the Action editor — ten rows in three groups — that was thirteen
hairlines in a pane 712px wide, a line every forty pixels down the whole surface. The pane read as
ruled paper with settings on it.

Four cheaper fixes were prototyped against the real panes before this one:

- **drop the group's closing rule** (the 34px group gap already closes it): 13 lines to 10;
- **head + tail only**: 13 to 6, but a one-row group becomes a box around a single row, and rows
  with nothing between them lose the label-to-value read-across the ledger existed for;
- **head rule only**: 13 to 3, and the group head is then the only line on the pane;
- **one card per group** with rows flush inside it: fewest enclosures, but the tracked eyebrow
  inside a box reads as a table header and wants a different type register.

All four are still ruled surfaces with fewer rules. The card is the one that removes the rule
rather than rationing it: an edge that encloses a configuration says what a hairline was being
asked to say, and says it once.

## The card

- `rounded-lg border px-4.5 py-3.75`, 10px between cards, written once in `Field` and re-exported
  for `NavCard`. Nothing else may name that geometry.
- **No fill.** A tinted card was prototyped and rejected: ink-fill is the pane's one accent
  (ADR-0008), and a filled card puts the Segmented control's selected fill and the switch's on-fill
  inside a filled box. The edge carries the card.
- **No shadow**, for the same reason the frameless windows have none (ADR-0009): the surface is one
  plane.
- **No bleed.** Cards sit inside the pane's padding rather than reaching out through it, so a
  card's name starts one card-padding right of the pane title. The alternative — negative margins
  that pull the card edge into the pane's gutter to keep the names on the title's x — was
  rejected: it buys alignment by putting the card's edge 10px from the window frame.
- **Radius is `--radius`**, the same 8px the windows are rounded at. A tighter radius was tried and
  dropped; two radii on one plane read as two materials.
- **One weight of edge.** `--input` is now `--border`: a control's outline is the same line as the
  card that holds it. The split ADR-0008 made — `--input` darkened to clear the 3:1 WCAG 1.4.11
  boundary — was drawn when a field sat on the bare pane; inside a card it made a light box around
  a darker one, and the pane read as two materials again. What identifies a field now is its
  unfilled box, and on focus a border that goes to `--ring`.
- **Hover strengthens the edge, and that is the whole state.** `--border-strong` (the value `--input`
  used to hold) is the hover colour, exported from `Field` as `CARD_HOVER` so a card and the
  `NavCard` cannot hover differently. Still no fill: a fill is the pane's inversion accent.
- **The focus halo is 2px at `ring/25`.** It was 3px at `ring/50`, tuned against a field whose rest
  border was already dark; against `--border` the border-to-`--ring` jump is the affordance and the
  halo only has to be noticed. Applied to `input`, `textarea`, `select` and `NavCard` alike.
- **The name carries `font-medium`**, shadcn's `Label` default, which the ledger row used to cancel.
  With no hairline closing a row, weight is what separates the name from the explanation under it.

## What the card costs, and what pays for it

The label column is gone, so nothing reads *down* the pane any more: `--spacing-ledger-label` and
`--spacing-ledger-gap` are deleted, and a name is as wide as the name. What replaces the read-across
is the card's right edge — every control on a pane parks there, which is the same alignment claim in
the other direction. The controls that have no intrinsic width still take `--container-control` via
`Field`'s `measure`, so they park there at one width.

Two consequences follow from right-aligning:

- `Temperature` takes a width rather than a ceiling (`w-control max-w-full`). Shrink-to-fit, its
  `flex-1` track collapsed to the number input beside it.
- `ModelSelect` keeps `w-fit min-w-48 max-w-control` from ADR-0011, and the edge its column now
  keeps is the trigger's right one rather than its left.

## Typed fields stack

A text field cannot right-align against its own name: at the window's minimum width (780px, so a
pane of 512px) a 420px control leaves 32px for the label. So `Field` takes `stacked` — name, control
at its measure, explanation — and every typed configuration uses it: the API key line, the Base URL,
and all four of an Action's text fields. The pane therefore has two card shapes, and which one a
card takes is decided by whether its value is *written* or *chosen*.

## The Action editor is two screens

Four stacked cards ahead of the Input Source buried every choice in the editor under the prompts.
So the four typed fields moved to a screen of their own — **Definition** — opened by one `NavCard`
above the first group head, and the main screen holds only choices: Trigger (Input Source, Direct
Hotkey), Model overrides, and the file's own delete card. The split the two names carry is what the
Action *is* against how it fires and what it fires at.

**That screen is one card, not four.** The name, the description and the two prompts are one
configuration — they are the Action's definition — and four boxes inside a screen already reached
through a box enclose that one thing four times. Inside it no field takes a `measure`: a measure
exists so controls chosen from a set park at one x down a pane of mixed cards, and on a screen of one
card there is nothing to line up against — while the system prompt and the user template are the
longest strings in the app. So every field there runs the card's full width. `Field` takes `bare` for it: no edge, no
padding, no hover, the fields spaced by the air a card would have put between them. It is also the
one card on the pane that does not respond to the pointer, and correctly so — there is nothing else
on the screen to move to.

- **Which screen is open is store state** (`Editing.screen`), not a `useState` in a component the
  shell re-keys. `showScreen` flushes the save slot before it moves, so a card that navigates ends
  a pending edit exactly as leaving the section does (ADR-0003). The back control also sits outside
  the form element, which is the same protocol the section nav uses.
- **`PaneEnter` is keyed on the screen** as well as the route and the file, so the drill-in animates
  once — the existing 200ms, 4px vertical entrance, not a new one.
- **A warning on the far side of a click is carried onto the card that opens it.** `NavCard` takes a
  `warning`, and the editor passes the first of the text screen's own — an Action with no name, a
  user template with no `{{input}}`. A field's problem must survive being one screen away, which is
  the same reason the navigation column flags a section.
- The group that was `Action` is now `Trigger`: with the name and description gone, what is left in
  it is the Input Source and the Direct Hotkey.
- `Delete Action` is a card under a `This file` head. With no hairlines left there is no divider for
  it to sit above, and a group head is what now says "this is not one of the settings". The
  treatment is unchanged: `destructive-outline` at rest, solid red only in the confirmation.

## What stays ruled

The **Actions list** keeps its hairline rows. It is a list of records, not of configurations, and
its row is the Launcher's row at pane density — the same four columns at the same fixed widths, via
`ActionCells`, so the two lists cannot drift (ADR-0009). Parity with the Launcher outranks
consistency with the pane; carding the list means carding the Launcher, which is a separate
decision about a window whose whole job is one keypress.

`SettingsNav`'s divider and the `StatusBar`'s top border also stay: both are window chrome, not
pane rules.

## Open

`Definition` — the screen's name, held in one constant so the card and the heading cannot disagree —
is not in [CONTEXT.md](../../CONTEXT.md)'s vocabulary. The ubiquitous language has *Action* and
*Prompt* but no word for the four fields together. Two names were tried and dropped first:
`Name and prompt`, because a conjunction in a screen's name says it holds two things, and `Wording`,
because it names the medium rather than the meaning. *Definition* claims the split with the main
screen: what the Action is, against how it is triggered and what model runs it. The counter-argument
is that an Input Source is arguably part of a definition too. If a better word arrives, that constant
is the only place it lands.
