---
name: model-register
description: "Use when a model or endpoint fact has to change in Beckon — adding or retiring a DeepSeek model, fixing a rotted preset model id, adding a provider preset, adding a reasoning dialect, or auditing the register for drift against what the vendors now publish. Examples: \"add the new DeepSeek model\", \"a preset model was renamed\", \"add Cerebras as a preset\", \"is the model catalog still current?\", \"the preset model 400s\""
---

# Maintaining the model register

Nothing in Beckon discovers a vendor's facts. Two hand-kept tables carry them, each read by two
consumers — which is the whole reason each is a single table:

- **`CATALOG`** in [llm/models.rs](../../../src-tauri/src/llm/models.rs) — DeepSeek's own models and
  what each does with thinking mode. It feeds the Settings dropdown *and* `thinking_wire` in
  [llm/request.rs](../../../src-tauri/src/llm/request.rs), so a dropdown built from a second list
  would offer models the request layer then refuses.
- **`presets()`** in [config.rs](../../../src-tauri/src/config.rs) — one filled-in `Provider` row per
  first-party endpoint, including the one field a user cannot look up: `Reasoning`. A wrong arm there
  is a `400` on every turn that row serves.

The tests in both files guard shape, never currency: no assertion can know that a vendor renamed a
model yesterday. Read the current values out of the two tables — **this file names no model id and no
vendor version**, because those are exactly the facts it is about, and a stale example in a skill
reads as a recommendation.

## Every hand-kept site

| What | Where | Changes when |
| --- | --- | --- |
| `CATALOG` rows — `id`, `label`, `thinking`, `retired` | `llm/models.rs` | DeepSeek ships, renames, or withdraws a model |
| `description` + `description_zh` per row | same rows | with the row; both languages live here, not in `src/lib/i18n/` |
| Provenance dates in the module doc | `llm/models.rs` header | every time the DeepSeek docs are re-checked |
| `DOCUMENTED` test const | `llm/models.rs` tests | any non-retired row added, removed, or reordered |
| `DEFAULT_MODEL` | `config.rs` | the default is retired or renamed |
| `DEEPSEEK_TEMPERATURE` | `config.rs` | DeepSeek changes its own guidance (ADR-0019 → ADR-0021) |
| `DEFAULT_BASE_URL`, `DEEPSEEK_HOST`, `DEFAULT_PROVIDER_ID`, `DEFAULT_PROVIDER_LABEL`, `DEFAULT_KEY_PAGE` | `config.rs` | DeepSeek moves a host or a key page |
| `presets()` rows — `base_url`, starting `model`, `key_page`, `reasoning` | `config.rs` | any vendor renames a model, moves a host, or a new endpoint is added |
| The **checked date** in the `presets()` doc comment | `config.rs` | every audit, including one that changes nothing |
| `Reasoning` arms and `Reasoning::guess` host substrings | `config.rs` | a new wire dialect (see [Adding a dialect](#adding-a-reasoning-dialect)); `guess` is frozen — legacy files only |
| The `Reasoning` union mirrored outside Rust | [types.ts](../../../src/lib/types.ts), [Connection.tsx](../../../src/settings/sections/Connection.tsx), [en.ts](../../../src/lib/i18n/en.ts) + `zh.ts` | with a new arm; there is no compiler link from the Rust enum to the TS union |
| The legal `reasoning` values in the config example | [README.md](../../../README.md) | with a new arm; it is the only user-facing list of them |
| `EFFORT_NONE_FAMILIES` + its checked date | [llm/request.rs](../../../src-tauri/src/llm/request.rs) | OpenAI ships a family that documents `reasoning_effort: "none"`, or withdraws one |
| Model ids in the config examples | [README.md](../../../README.md) | when an id in an example no longer exists |
| Model ids in the manual test checklist | [docs/macos-testing.md](../../../docs/macos-testing.md) | same; the vision and non-vision rows each name one |

Leave model ids quoted inside `docs/adr/` alone. An ADR records what was decided when, so a rotted id
there is history rather than a defect — fix it only if the ADR is being superseded anyway.

Highest rot rate is the `model` field of `presets()`: every value is a vendor-side name that can
change without a single test failing. Print the current set before reasoning about it:

```bash
sed -n '/pub fn presets/,/^}/p' src-tauri/src/config.rs | grep -n '"'
```

## Checking a vendor's facts

Take every id from an authority, in this order — never from memory, and never by inferring the next
one in a pattern:

1. **The vendor's own docs** — the model list, the pricing page, and the deprecation or changelog
   page, which is the only place a shutdown date exists. Where a vendor publishes a deprecation
   table, use its named replacement over your own judgement of the nearest tier.
2. **The endpoint itself**, which settles an exact id string:

```bash
curl -s "$BASE_URL/models" -H "Authorization: Bearer $KEY" | head -40
```

DeepSeek's three pages are cited by name and date in the `models.rs` module doc — follow those links
rather than re-deriving them.

Bump a table's checked date even when the check confirmed it was already right: that date is the only
record of when the list was last known good, and an audit that changes nothing is exactly the one
whose result is otherwise invisible.

## Auditing the register

The one procedure to run on a schedule rather than on a trigger, because nothing else surfaces rot.
Roughly quarterly, and after any report of a preset that `400`s.

1. Print both tables. `CATALOG` and `presets()` are the whole surface.
2. Check each row against the vendor per [above](#checking-a-vendors-facts), and sort what you find:
   - **Gone** — the id no longer serves. Fix now; every turn on that row is a `400`.
   - **Alias that has drifted** — the id resolves but the vendor points it at an older model or a
     lower effort. Worse than gone, because nothing complains.
   - **Live but superseded** — resolves, serves, generations behind. See
     [Fixing a rotted preset model](#fixing-a-rotted-preset-model) for which outcome applies.
   - **Current** — record it and move on.
3. Re-read each row's `reasoning` against what the vendor now documents. This check finds the most,
   because a vendor adding an off-switch makes a previously correct `None` wrong and no gate can see
   it. Each arm's comment states why it was chosen — if that sentence is no longer true, neither is
   the arm. Then re-read any **per-model list an arm consults** — today `CATALOG` for the DeepSeek
   arm, `EFFORT_NONE_FAMILIES` for the OpenAI one. A family missing from the second is a silent loss
   of suppression rather than a `400`.
4. Bump the checked date in every doc comment that carries one — today `CATALOG`, `presets()` and
   `EFFORT_NONE_FAMILIES` — whatever you changed.

Done when every row has one of the four verdicts, every checked date is bumped, and the report names
the date the vendor was checked rather than the gate results (see [Not done until](#not-done-until)).

## Adding a DeepSeek model

1. Add the `CatalogEntry` in the position it should appear in the dropdown — catalog order is
   meaningful, and `rank` sorts a live list by it.
2. Set `thinking` from the docs, not by analogy with a sibling model. `Switchable` means the
   documented `thinking` object works both directions; `AlwaysOn` makes `thinking = false` a refused
   turn with a message pointing at `switchable_suggestion()`; `Never` means a `thinking` object is a
   `400`. **When the docs say nothing, `Never` is the answer** — refusing out loud beats sending a
   field hopefully.
3. Write both descriptions, one line each, the Chinese one taking its terms from the
   [CONTEXT.md](../../../CONTEXT.md) table. `the_catalog_is_translated_throughout` only checks that
   both exist and differ, so an untranslated copy-paste with one word changed would pass.
4. Update `DOCUMENTED` in the tests to the new non-retired set, in catalog order.
5. Note the provenance in the module doc — which page, which date — the way existing entries do.

## Retiring a model

Keep the row: a config that still names a withdrawn model has to keep working and be explained
(ADR-0021).

1. Set `retired: true` and put the withdrawal date in both descriptions.
   `a_configured_retired_model_is_offered_and_explained` asserts the English one carries the date.
2. Leave `thinking` as it was. `thinking_wire` still consults it, and a live list may resurrect the
   model — retirement is a fact about DeepSeek's own host, not about every `base_url`.
3. Change the `label` to carry `(retired)`, as the already-retired rows do.
4. Update `DOCUMENTED`, which now has one fewer entry.
5. If the retired model was `DEFAULT_MODEL`, move that constant to a non-retired `Switchable` row.
   `the_default_model_is_a_live_catalog_entry` fails until you do, and `Provider::deepseek()` picks it
   up for free.
6. Record the withdrawal in the module doc's provenance, with the date and the page that stated it.

## Fixing a rotted preset model

A preset's `model` is a starting value, not a claim. Three correct outcomes, in order of preference:

- **An evergreen alias**, where the vendor publishes one — a name with no version in it that they
  repoint themselves. Best, because it cannot rot. Verify it actually tracks the current model: an
  alias quietly frozen to an old generation is the worst of the three, since it works and is wrong.
- **The current pinned id.** What most vendors leave you. It will rot again, which is what the checked
  date is for.
- **Empty**, where their ids carry dated or `-preview` suffixes that rot fast. Empty sends the user to
  the dropdown, which is where the endpoint's own list lands anyway; a rotted id is a `400` they have
  to decode instead.

Say in a dated one-liner comment what the value was and why it moved — that is what makes the next
audit cheap.

## Adding a provider preset

Require that **the request terminates at the company whose key it carries**. An inference provider
serving somebody else's open weights on its own GPUs qualifies; a broker holding keys to other APIs
and relaying does not, because it puts a third party inside a relationship that was between the user
and a vendor — no OpenRouter, no LiteLLM, no gateway.
`every_preset_goes_direct_to_its_own_vendor` enforces this by name list, so a broker absent from
`BROKERS` satisfies every type and nothing else notices. Check the vendor by hand.

The same test requires a distinct `id` (it is the credential account, so a duplicate shares a key), a
non-empty `label`, `https://` unless `is_local()`, and a `key_page` for anything remote and none for
anything local. It says nothing about `model`, which is why a dead one survives it.

`reasoning` defaults to `None` in the `row` helper, and `None` is usually right — it means "nothing on
the wire either way", which covers every plain OpenAI-compatible endpoint including the reasoning
models you pick deliberately. Depart from it only where the endpoint documents a switch that turns
reasoning **off**, and write the comment naming which switch and why, as the departing rows do. "Off"
means off: a floor that still reasons is not one, and claiming it would send a value meaning something
the user did not ask for.

## Adding a reasoning dialect

Six edits, and only the first is compiler-checked:

1. The `Reasoning` arm in `config.rs`, plus its match in `thinking_wire` and, if the dialect puts a
   new field on the wire, a `ThinkingWire` variant and its arm in `build_body`.
2. A wire test in `request.rs` beside the existing per-arm ones, asserting the exact field in both
   directions. Every other arm has one; a dialect without one is a `400` waiting for a user to find.
3. `Reasoning` in [types.ts](../../../src/lib/types.ts) — a string union, so a missing arm is silent.
4. `REASONINGS` in [Connection.tsx](../../../src/settings/sections/Connection.tsx) — the dropdown's
   order, hand-listed.
5. `reasoningName` and `reasoningHint` in **both** i18n catalogs. `en.ts` fixes the shape and `zh.ts`
   must satisfy it, so this pair is checked once step 3 is done.
6. The `reasoning = ...` legal-values comment in [README.md](../../../README.md), the only place a
   user reads what may go in the field. An arm the README omits is an arm nobody outside the code
   knows exists.

Then answer the question the arm's name hides: **does every model behind that endpoint take the
field?** A dialect is the endpoint's property (ADR-0021) and the row still selects it — but
`api.deepseek.com` still answers to legacy ids that picked the mode through the id itself, so the
`thinking` object contradicts one of them, and `api.openai.com` serves `gpt-4o` and `o3`, which reject
the effort floor. Where the host is mixed, have the arm consult
a per-model list beside it (`CATALOG`, `EFFORT_NONE_FAMILIES`) and make the unmatched case the safe
direction: `Omit` where silence costs only the feature, a refusal only where silence would cost
invisible latency on every turn.

Confirm nothing else keys off the arm by name — `src/lib/models.ts` asks only whether the row is
`none`, so a new arm is treated as throwable without an edit. Check that this is still true rather
than assuming it.

A dialect only ever sent in one direction is normal — express the asymmetry in `thinking_wire`, where
it can carry a comment, rather than in `build_body`, which stays a dumb translation of the wire
variant it is handed.

Leave `Reasoning::guess` alone. It is the only host guess in the codebase and it runs on exactly one
thing: a config file written before the provider table existed (ADR-0021). A new dialect has no legacy
files to migrate.

## Not done until

The gates pass:

```bash
npx tsc --noEmit
(cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
```

`cargo test models::` and `cargo test config::` cover this work specifically, but the frontend mirror
of `Reasoning` is only checked by `tsc` — run both. If a toolchain is missing in the environment you
are working in, say the gates did not run; never infer them from inspection.

…**and** the report states what the gates cannot see, rather than reporting green as current:

- **A renamed or withdrawn vendor model.** No test reaches the network. Only the docs check does.
- **An alias that has drifted** to an older model or a lower effort. It resolves, it serves, it is
  wrong, and nothing anywhere says so.
- **A description that is stale but present.** Both language fields being non-empty and different is
  the whole assertion.
- **A broker added as a preset** whose name is not in `BROKERS`.
- **A wrong `reasoning`** on a preset row, including one that was right when written and was made
  wrong by the vendor adding a switch. It types fine and fails as a `400` on the user's first turn.
- **A model family missing from `EFFORT_NONE_FAMILIES`.** Suppression is silently dropped for it — the
  turn runs, slower, and nothing says why.
- **A stale provenance date.** It is prose.

An audit that says "all passing" without saying "checked against the vendor on this date" has reported
the wrong thing.

## Not part of this flow

- **A user's own model choice.** A configured id is surfaced, never rewritten, even when nothing
  vouches for it (`Origin::Configured`). Do not "correct" a config file.
- **The live list.** `get_models` prefers what the endpoint serves; the catalog is the fallback, and
  only for DeepSeek's own host. Adding a model to `CATALOG` is not how a non-DeepSeek endpoint's
  models appear.
- **Temperature as a control.** ADR-0019 decided there is no temperature UI; ADR-0021 moved the pinned
  DeepSeek value onto that vendor's row. A preset may carry one; nothing else may.
- **Per-model capability gates.** The catalog records no image column — nothing is gated on one
  (ADR-0016).
- **Contradicting an ADR.** A change needing the register to work differently rather than to say
  something different needs a new ADR first (see [CLAUDE.md](../../../CLAUDE.md)). Adding a
  `Reasoning` arm is *not* one of those **when the row alone decides it**: ADR-0021 made the dialect a
  property of the row precisely so such an arm is data. An arm that needs a **per-model list** is the
  other case — 0021 named per-model knowledge as the thing putting the dialect on the row got rid of,
  so a second list narrows it and gets recorded (0021's own footnote, or a new ADR).
