---
status: accepted
---

# An Action's model overrides are ledger rows

[ADR-0008](0008-settings-is-react-with-shadcn-ui-and-tailwind.md) made the pane a ledger and
gave every labelled control one row shape. The Action editor's `[model]` block was the exception:
three bordered boxes indented into the value column, each collapsed to a summary until clicked.
That exception is removed, and with it `OverrideField` and `InfoHint`. The rest of ADR-0008
stands.

**Amended by [ADR-0012](0012-settings-pane-is-cards-not-a-ledger.md):** the pane is cards now, not
a ledger, so "row" below reads as "card" throughout. Every decision in this ADR survives that —
the live control showing the effective value, the dot beside the name, the revert
control naming the default, the group head's note. What changed is where the revert control sits:
the card right-aligns its control, so the slot holding the revert is held open at the card's right
edge rather than after a slot the width of the control measure. The two ledger spacing tokens named
in the last bullet are deleted. The dot also left the flow: reserved as a column it indented every
name in the group past every other name on the pane, so it now hangs in the card's own padding,
which keeps the names aligned and still cannot shift the row that carries it.

## What the exception was buying, and what it cost

An Action's `[model]` keys are optional — absent means "inherit Model defaults" — so the row had a
third state no single control expresses. `OverrideField` answered that by making the *row* the
control: opening it overrode, and the collapsed line printed the inherited value with
`from Model defaults` beside it.

The cost was paid on every one of those goals.

- **You could not look without writing.** Opening the row to see what the select offered wrote the
  override to disk. There was no inspect gesture at all.
- **Saving read as discarding.** The row collapsed on `focusout`, which is the same gesture that
  commits a debounced write (ADR-0003). The one animation the user saw after editing was the form
  closing.
- **Reverting required overriding first.** `Use the default` lived inside the expanded row, so the
  way back was reachable only by taking the step you wanted to undo.
- **The explanation had to hide.** A collapsed row has no room for a standing line, so the
  temperature sentence went behind an `InfoHint` bubble — while the identical sentence stands as
  prose on Model defaults, two clicks away.
- **It was the only block on the pane out of line.** Being neither a row nor a group, it indented
  itself past `--spacing-ledger-label` plus `--spacing-ledger-gap` by hand to reach the column the
  controls above it sat in.

## The shape

The control is live whether the key is present or not, and it shows the **effective** value — what
a request would carry. Touching it is what overrides: choosing a model, throwing the switch,
dragging the slider. One gesture, three control types, and looking costs nothing.

What is left to say is which side of the default a row is on, and that is `Field`'s `override`
prop, deliberately the least that still tells the truth:

- a 4px ink dot beside the name, on an overridden row only — hung in the card's padding rather than
  given a column, since a gutter reserved in the flow indents the whole group and one that exists
  only when filled would shift the label sideways the moment the row is overridden;
- a revert control after the slot, on an overridden row only, naming the default in its own
  accessible label (`Use the default (off)`) — which is the one place the default's *value* is
  spelled out, on demand rather than standing;
- one quiet line in the group head, `Unmarked rows follow Model defaults`, for every row that is
  not marked.

The slot holding the control is the control measure whether the control fills it or not, so the
revert controls of a group line up in a column instead of trailing three different control widths.

Three per-row sentences, a `2 of 3 overridden` count and a `Reset all` were all tried and all
dropped: a value nobody is departing from is not news, and three rows do not need a bulk action.

## What follows

- `InfoHint` had one consumer and one justification — the collapsed rows — so it is deleted, and a
  field's explanation is now a permanent line with no exception. The `popover` primitive stays in
  `components/ui` as library source.
- The Model select is sized to its content with a floor (`w-fit min-w-48`, ceiling unchanged)
  rather than stretched to the measure. Stretched, its chevron sat 200px from the value it belongs
  to; the floor is what the measure was really protecting against. The line the column keeps is the control's
  right edge under ADR-0012, and was its left edge here; either way the sizing is unaffected.
- The temperature readout is 28px rather than an input's full height: it is a caption on the
  slider, and at 36px it stood taller than the track it annotates.
- `--spacing-ledger-label` and `--spacing-ledger-gap` now have exactly one consumer, `Field`. (Both are deleted under ADR-0012, which has no label column.)
