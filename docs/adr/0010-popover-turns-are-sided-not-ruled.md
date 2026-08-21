---
status: accepted
---

# The Popover's turns are sided, not ruled

[ADR-0009](0009-launcher-and-popover-ported-to-react-svelte-removed.md) applied the Settings
ledger to all three windows, the Popover included: a fixed label column holding `You`, the
model's name, `Thinking` and `Failed`, against a content column, one hairline per row. That part
of it is replaced. Everything else in ADR-0009 stands.

## Why the ledger was the wrong fit here, specifically

The ledger's label column earns its space by naming a **value** — a setting, an Action's Input
Source, a hotkey — because the alternative is a stack of forms with nothing telling one row from
the next. A conversation has something a pane and a picker do not: **two speakers.** And the
label column could only name one of them badly.

- `You` is the window addressing the user in the first person, which nothing else in the product
  does.
- The other side had no good word either. Naming it after the model printed a fact the header
  already carries — every turn in one Exchange goes to the same model, so the column repeated one
  string down the whole window and truncated it doing so, since `deepseek-reasoner` does not fit a
  96px column. Naming it after the content (`Answer`) meant the two labels described different
  kinds of thing: one a speaker, one a payload.

Sides carry the same information with no words at all, and they are what every reader of a chat
already knows how to read.

## The shape

Your input is a card on the right, capped at 80% of the window; the answer runs left and bare to
`--container-measure`. There are no hairlines between turns — the gap separates them, which is
what makes the two sides visible as sides.

**The card is filled with `--muted`, the quietest fill there is.** The alternative considered and
rejected was the inverted fill: `--primary` with paper text, which is unmistakable and is what a
chat app would reach for. It is refused because inversion means "current" everywhere else in this
product — the selected nav item, the selected segment, the selected row in the Launcher — and
spending it here would both dilute that meaning and make the user's own words the loudest thing in
the window. An outlined card with a rule down the model's side was also mocked; it is the most
consistent of the three with the rest of the app and it was passed over for being the least legible
as a conversation, which is the one thing this window is.

## What the label column was carrying, and where it went

- **The failure marker.** Nothing else on that side says a turn went wrong once the label is gone,
  so a `Failed` line in `--destructive` sits over the failure sentence. It is a marker, not a
  heading: the sentence beneath it is still the muted prose `describeFailure` builds.
- **The notices.** `Idle`, the empty-Selection block and the "type what you want to send" hint were
  label rows, and a notice has no side. The one that is an alarm — an Action that needs a Selection,
  with nothing selected — is a `Callout`, a rule and its text, the same marker the panes use. The
  other two are ordinary prose.
- **The composer's label.** Gone. A single labelled row under a window of unlabelled turns would be
  the only label column left in it.

## Consequences

- `Row.tsx` and its exported `ROW_LABEL` are deleted; `TurnRow.tsx` becomes `Turn.tsx` exporting
  `TurnView`, since it no longer renders a row.
- **The ledger is now two windows, not three,** and the design system is unaffected either way: the
  Popover still names no colour, size or duration of its own, and this change added no token, no
  keyframe and no CSS.
- The header is the only place the model is named. That was already true of the status; it is now
  true of the model too, which is the version of that header the ADR-0009 prototype argued for and
  the label column quietly undid.
- Prose in the Popover is still measured — `max-w-measure` on the answer. Sided layout without a
  measure is how a chat window ends up with 90-character lines.
