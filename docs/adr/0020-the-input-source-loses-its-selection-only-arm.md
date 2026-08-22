---
status: accepted
---

# The Input Source loses its selection-only arm

`input_source` had three values: `selection`, `prompt`, `auto`. It now has two. `selection` is gone,
`auto` is still the default, and `prompt` stays exactly as it was.

## Why the third arm went

`selection` and `auto` did the same thing whenever there *was* a Selection. They differed only on an
empty grab, and the difference was that `selection` produced a hint — "this Action works on a
Selection, and nothing was selected" — where `auto` produced an input box.

The hint is worse in every case it covers. Reaching it means the user pressed a hotkey and got a
sentence back, and the only way forward is to dismiss the window, select something, and press the
hotkey again. The input box in its place is a working window: they type, or they attach a screenshot
(ADR-0016, ADR-0017), and the Action runs. Nothing about an Action that *usually* reads the Selection
makes typed input wrong for it — the prompt template takes `{{input}}` either way and has no idea
where the string came from.

It also cost a phase. `PopoverPhase::EmptySelection` existed only to render that hint, and because a
hint window can never grow, it had a height rule of its own: `POPOVER_HINT_H` capped the remembered
size (ADR-0018) at 220px for this one phase. That is now three fewer moving parts — a phase, a
constant, and a branch on the summon path — for a state whose replacement is the state next door.

## Why `prompt` stays

`prompt` looks like the same kind of redundancy and is not. The grab happens **before the Action is
known** — that is ADR-0006's whole point, and it cannot be otherwise, because the copy shortcut has
to reach the foreground window before any Beckon window takes focus. So by the time an Action is
resolved there is always a Selection in hand, whether or not it has anything to do with what the user
was about to ask.

Under `auto`, that string becomes the request. An "ask anything" Action, fired from the Launcher while
some unrelated text happened to be highlighted, would send that text to the API instead of opening an
input box — with no step at which the user saw it happen. `prompt` is the declaration that this
Action ignores the grab, and there is nothing else in the config that can say so.

That asymmetry is the shape of the decision: the arm that only ever *refused* to do something was
worth keeping, and the arm that only ever *narrowed* `auto` was not.

## What a file that names it does

`selection` deserializes as `auto`, via a serde alias. Dropping the variant outright would make an
unknown-variant error out of an Action file written last week — and an unparsable Action is a
diagnostic in the Actions list, not a field that quietly reverts, so a user with three Actions would
find three of them broken by an update. The alias is one line and it is not a migration: nothing
rewrites the file, and the next save writes `auto` because that is the value in memory.

This is the same treatment `llm::models::CATALOG` gives a retired model id — recognised so an existing
config keeps working, never offered as a fresh choice.

## What this is not

Not a case for removing `input_source` altogether, which was the starting proposal. A two-value enum
still earns its place in CONTEXT.md as a named concept with a value on disk and a column in two
lists; what it does not earn is a third value. And not a contradiction of
[ADR-0002](./0002-selection-via-simulated-ctrl-c.md), which says an empty grab is a phase and never
an error — it still is, there is just one phase for it now instead of two.

## Consequences

- `InputSource::Selection` and `PopoverPhase::EmptySelection` are deleted, as is
  `trigger::window::POPOVER_HINT_H` and the height branch in `open_action` that read it. The summon
  always uses the remembered size.
- The `Notice` union in `src/popover/exchange.ts` loses `empty-selection`, and the Popover's notice
  slot loses its `Callout` — neither of the two notices left is an alarm.
- `MIN_POPOVER_H` is no longer a floor that has to sit under the hint height, since there is no
  height the product picks for itself any more. It stays 200px on its own merits.
- `t.popover.needsSelection`, `t.popover.selectAndRetry`, `t.words.emptyGrabCause`,
  `t.inputSource.selection` and `t.settings.actions.sourceHint.selection` leave both catalogs.
- The `translate.toml` seed now declares `auto`, which is what it always meant.
