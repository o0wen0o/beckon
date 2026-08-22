---
status: accepted
---

# Temperature leaves the config and is pinned in the provider module

`temperature` was a `[defaults]` key in `config.toml` and an optional `[model]` key on every Action,
merged field-by-field by `ModelOverrides::merge_over`. Both are gone. The value that goes on the wire
is a constant in `llm/deepseek.rs`, still 1.3, and no longer anybody's setting.

## Why it was there

DeepSeek publishes a recommended temperature per use case — roughly 0 for code and data extraction,
1.3 for conversation and translation. That table is the whole argument for the key: it is *per use
case*, and an Action is a use case, so the Action file is where a per-use-case value belongs. It was
the one `[model]` key that changed the answer to the same prompt.

## Why it goes anyway

The argument assumes the user can tell which value is right, and Beckon gives them nothing to tell it
with. A Popover shows one answer to one prompt. There is no second panel, no re-roll at another
setting, no history to compare against — the Exchange is not even stored (ADR-0004). Every other
control on the pane can be judged from the product: a model has a name and a price, thinking mode is
visibly slower. A number between 0 and 2 that changes wording in a way you cannot see twice is a
setting whose feedback loop is outside the app.

What it cost to keep was not the merge arm. It was the pane: a slider, a three-word scale under it,
a paragraph of explanation, an override dot and a revert control on the Action editor — the widest
row in `[model]`, for the key nobody can evaluate. Model defaults is now a model and a switch, which
is what a defaults pane the user can reason about looks like.

## Why a constant, and why in `deepseek`

Two options once it stops being config: omit the field, or pin it.

Omitting hands the choice to the endpoint. That reads like neutrality and is not: `base_url` is
configurable (ADR-0013's provider work), so "the endpoint's default" is a different number on every
endpoint, and on DeepSeek specifically it is not the 1.3 their own guidance gives for the
conversational and translation shapes Beckon exists for. Removing a knob should not quietly change
the answers.

So it is pinned, and it lives in `llm/deepseek.rs`, which is the module CLAUDE.md already names as
the only home for provider quirks. 1.3 *is* a DeepSeek quirk — it is their recommendation, not a
general truth about language models — and `ModelParams` is the shape a request is built from, not a
place to keep a number the request layer already knows.

## Files that still name it

An Action file with `temperature = 0.2` under `[model]` keeps loading: `ActionFile` tolerates unknown
keys, so the value is ignored and dropped the next time the file is written. `config.toml` is the
same. Neither is an error and neither is reported — a diagnostic about a key whose meaning has been
withdrawn tells the reader to go do something there is nothing to do about.

## Consequences

- `DEFAULT_TEMPERATURE`, `ModelDefaults::temperature`, `ModelOverrides::temperature` and
  `ModelParams::temperature` are deleted. `merge_over` merges two fields.
- `src/components/Temperature.tsx` is deleted with its one and only pair of consumers.
  [ADR-0011](./0011-action-model-overrides-are-ledger-rows.md) argued the shape of an override row
  using the temperature row as its example, and
  [ADR-0012](./0012-settings-pane-is-cards-not-a-ledger.md) sized that control; both arguments stand
  for the rows that remain, and the temperature clauses in them are now history.
- The catalog keys `settings.defaults.temperature`, `temperatureHint` and the whole
  `controls.temperature` group leave both languages.
