---
name: model-register
description: "Use when a Beckon model or endpoint fact must change or be re-checked: adding or retiring a DeepSeek model, a moved base_url or key page, a listing endpoint that stopped answering, a new provider preset, a new reasoning dialect, or auditing the register against what the vendors now publish. Examples: \"add the new DeepSeek model\", \"a provider row 400s\", \"add Cerebras as a preset\", \"is the register still current?\", \"the model dropdown is empty\", \"the key page link is dead\""
---

# Maintaining the model register

Beckon discovers no vendor fact. Two hand-kept tables carry them, plus the mirror sites listed below:

- **`presets()`** in [config.rs](../../../src-tauri/src/config.rs) — one `Provider` row per
  first-party endpoint: **where to fetch and how to connect, never what to run.** `base_url`,
  `key_page`, and the one field a user cannot look up, `Reasoning`. A wrong arm is a `400` on every
  turn that row serves.
- **`CATALOG`** in [llm/models.rs](../../../src-tauri/src/llm/models.rs) — DeepSeek's models and what
  each does with thinking mode. It feeds `thinking_wire` in
  [llm/request.rs](../../../src-tauri/src/llm/request.rs) and nothing else, so **only its `thinking`
  column is load-bearing**; label, description and order merely enrich ids the endpoint itself named.

**A row ships no model.** `presets()` leaves `model` empty on every row but `deepseek`, and the list a
user picks from is the endpoint's own `GET {base_url}/models`, kept between launches in
[models_cache.rs](../../../src-tauri/src/models_cache.rs) (ADR-0024). So the highest-impact check is
not "is this id current" but "does the listing endpoint still answer, and with what credential".

Tests in both files guard shape, never currency — no assertion knows that a vendor renamed a model
yesterday. Read the current values out of the tables; **this file names no model id and no vendor
version**, because a stale example in a skill reads as a recommendation.

```bash
sed -n '/pub fn presets/,/^}/p' src-tauri/src/config.rs | grep -n '"'
grep -rn 'TODO(register)' src-tauri/src        # the open questions, held at their site
```

## Every hand-kept site

| What | Where | Changes when |
| --- | --- | --- |
| `CATALOG` rows — `id`, `thinking`, `retired` | `llm/models.rs` | DeepSeek ships, renames, or withdraws a model |
| `label`, `description` + `description_zh` per row | same rows | with the row; both languages live here, not in `src/lib/i18n/`. Enrichment only — a stale one mislabels an id the endpoint served, it does not offer a wrong model |
| Provenance dates in the module doc | `llm/models.rs` header | every re-check of the DeepSeek docs |
| `DEFAULT_MODEL` | `config.rs` | the default is retired or renamed. The **one** shipped model id outside a dialect's per-model list |
| `DEEPSEEK_TEMPERATURE` | `config.rs` | DeepSeek changes its own guidance (ADR-0019 → ADR-0021) |
| `DEFAULT_BASE_URL`, `DEFAULT_PROVIDER_ID`, `DEFAULT_PROVIDER_LABEL`, `DEFAULT_KEY_PAGE` | `config.rs` | DeepSeek moves a host or a key page |
| `presets()` rows — `base_url`, `key_page`, `reasoning` | `config.rs` | any vendor moves a host or a key page, or a new endpoint is added. **Not `model`** |
| The version rule in `api_url`, and its mirror `chatUrl` | [llm/client.rs](../../../src-tauri/src/llm/client.rs), [providers.ts](../../../src/lib/providers.ts) | a vendor's compatibility path is versioned in a shape the rule misreads. Move the two together: nothing links them, and a pane on the old rule draws a URL Beckon does not post to |
| The **checked date** in the `presets()` doc comment | `config.rs` | every audit, including one that changes nothing |
| `Reasoning` arms and `Reasoning::guess` host substrings | `config.rs` | a new wire dialect (see [Adding a dialect](#adding-a-reasoning-dialect)); `guess` is frozen — legacy files only |
| The `Reasoning` union mirrored outside Rust | [types.ts](../../../src/lib/types.ts), [Connection.tsx](../../../src/settings/sections/Connection.tsx), [en.ts](../../../src/lib/i18n/en.ts) + `zh.ts` | with a new arm; no compiler link from the Rust enum reaches the TS union |
| The legal `reasoning` values in the config example | [README.md](../../../README.md) | with a new arm; the only user-facing list of them |
| `EFFORT_NONE_FAMILIES` + its checked date | [llm/request.rs](../../../src-tauri/src/llm/request.rs) | OpenAI ships or withdraws a family documenting `reasoning_effort: "none"` |
| Model ids in the config examples | [README.md](../../../README.md) | an id in an example no longer exists |
| Model ids in the manual test checklist | [docs/macos-testing.md](../../../docs/macos-testing.md) | same; the vision and non-vision rows each name one |

Leave model ids quoted inside `docs/adr/` alone. An ADR records what was decided when, so a rotted id
there is history — fix it only if the ADR is being superseded anyway.

Two field kinds rot with no gate anywhere, and both are connection facts:

- **`key_page` and `base_url`** — a vendor moves a page, the old URL keeps answering through a `301`,
  and nothing complains. `key_page` is the first link a new user clicks.
- **`reasoning`** — correct when written, made wrong by the vendor later documenting an off-switch
  (audit step 3).

## Checking a vendor's facts

Take every id and every URL from an authority, in this order. Memory is not an authority, and neither
is the next value in an apparent pattern.

1. **The vendor's own docs** — model list, pricing page, and the deprecation or changelog page, the
   only place a shutdown date exists. Where a vendor publishes a deprecation table, use its named
   replacement over your own judgement of the nearest tier.
2. **The vendor's overview page**, for whether a value is still their current one. Their ordering is
   their recommendation, so a row sitting several entries below the top is superseded whether or not
   it still answers. This is the method behind the **Live but superseded** verdict, which otherwise
   has an obvious question, no obvious check, and so gets skipped.
3. **The URLs themselves.** Fetch every `key_page` and `base_url` and record the status code. A `301`
   is drift, not a pass. Mechanical, so this is the one part of an audit that is verified rather than
   trusted:

```bash
curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' "$URL"
```

4. **The endpoint itself** — [Checking that a row can be filled at
   all](#checking-that-a-row-can-be-filled-at-all). Two requests, not one.

DeepSeek's three pages are cited by name and date in the `models.rs` module doc — follow those links
rather than re-deriving them.

Bump a table's checked date even when the check confirmed it was already right: that date is the only
record of when the list was last known good. It is not the audit's artifact, though — the per-row
verdict table is.

## Auditing the register

The one procedure to run on a schedule rather than on a trigger. Roughly quarterly, and after any
report of a preset that `400`s.

1. Print both tables. `CATALOG` and `presets()` are the whole surface.
2. Check each row against the vendor per [above](#checking-a-vendors-facts), and give it one verdict:
   - **Gone** — the id no longer serves. Fix now; every turn on that row is a `400`.
   - **Alias that has drifted** — the id resolves but the vendor points it at an older model or a
     lower effort. Worse than gone, because nothing complains.
   - **Live but superseded** — resolves, serves, generations behind. Applies to a `base_url` as much
     as to a model id: a vendor can publish a newer versioned path while the old one keeps answering.
   - **Moved** — a `key_page` or `base_url` answering only through a redirect. Follow it and carry the
     destination.
   - **Current** — record it and move on.
   - **Unverified** — the fact could not be reached at all: a login wall, a compatibility page
     documenting no listing endpoint, an examples page too stale to be evidence. Not a pass and not a
     failure. On a **new** row it blocks that row rather than shipping on a guess; on an existing row
     record it together with what would settle it. Never a guess, never silence.
3. Re-read each row's `reasoning` against what the vendor now documents. This check finds the most: a
   vendor adding an off-switch makes a previously correct `None` wrong and no gate can see it. Each
   arm's comment states why it was chosen — if that sentence is no longer true, neither is the arm.
   Then re-read any **per-model list an arm consults** — today `CATALOG` for the DeepSeek arm,
   `EFFORT_NONE_FAMILIES` for the OpenAI one. A family missing from the second is a silent loss of
   suppression rather than a `400`.
4. Bump the checked date in every doc comment carrying one — today `CATALOG`, `presets()` and
   `EFFORT_NONE_FAMILIES` — whatever you changed.

**Not done until** the audit has written **one line per row, carrying that row's verdict**, into a
dated table at `docs/register-audit-YYYY-MM-DD.md`. A row with no line is unchecked, not fine: a
bumped date alone cannot distinguish a row that was read from a row that was skipped, and one date
covering a dozen rows is how a superseded id survives an audit that "passed". A table under `docs/`
rather than a doc comment, so `config.rs` does not double in size and the next audit has a diff to
read. Questions the audit opens but may not settle stay at their site as `TODO(register):`.

## Adding a DeepSeek model

1. Add the `CatalogEntry` in the position it should appear in the dropdown — catalog order is
   meaningful, and `rank` sorts a live list by it.
2. Set `thinking` from the docs, not by analogy with a sibling model. `Switchable` means the
   documented `thinking` object works both directions; `AlwaysOn` makes `thinking = false` a refused
   turn pointing at `switchable_suggestion()`; `Never` means a `thinking` object is a `400`. **When
   the docs say nothing, `Never` is the answer** — refusing out loud beats sending a field hopefully.
3. Write both descriptions, one line each, the Chinese one taking its terms from the
   [CONTEXT.md](../../../CONTEXT.md) table. `the_catalog_is_translated_throughout` only checks that
   both exist and differ, so an untranslated copy-paste with one word changed would pass.
4. Note the provenance in the module doc — which page, which date — the way existing entries do.

Adding a row puts the model in nobody's dropdown; only the endpoint's own list does that. What the row
buys is the `thinking` answer and a readable label.

## Retiring a model

Keep the row: a config that still names a withdrawn model has to keep working and be explained
(ADR-0021).

1. Set `retired: true` and put the withdrawal date in both descriptions.
   `a_configured_retired_model_is_offered_and_explained` asserts the English one carries the date.
2. Leave `thinking` as it was. `thinking_wire` still consults it, and a live list may resurrect the
   model — retirement is a fact about DeepSeek's own host, not about every `base_url`.
3. Change the `label` to carry `(retired)`, as the already-retired rows do.
4. If the retired model was `DEFAULT_MODEL`, move that constant to a non-retired `Switchable` row.
   `the_default_model_is_a_live_catalog_entry` fails until you do, and `Provider::deepseek()` picks it
   up for free. This is the one place a retirement can break a fresh install.
5. Record the withdrawal in the module doc's provenance, with the date and the page that stated it.

## Checking that a row can be filled at all

A row's whole offer is `GET {base_url}/models`, so that request is the check — **two results per row**,
recorded separately:

1. **With no `Authorization` header.** Some listing endpoints are public. Record the status.
2. **With a key.** Record the status and the first few ids, which is also what settles an exact id
   string when some other question needs one.

```bash
curl -s -o /dev/null -w 'anon  %{http_code}\n' "$BASE_URL/models"
curl -s -w '\nauth  %{http_code}\n' "$BASE_URL/models" -H "Authorization: Bearer $KEY" | head -40
```

Run the anonymous one too. Beckon's own `fetch_model_ids` calls `require_api_key` **before** any HTTP,
so a non-local row with no key stored never asks — whether or not the endpoint would have answered,
and whether that gate should stay cannot be settled without this column.

Outcomes:

- **Both answer** — the row is fine, and could be filled before a key is stored if Beckon asked.
- **Only the authenticated one answers** — the row is fine, and the current gate is correct for it.
- **Neither answers** — the row cannot be filled. On a **new** row that is a block, not a caveat: ship
  nothing rather than a row on which no model can ever be chosen. On an existing row, record it with
  what would settle it and mark the site with a `TODO(register):`.
- **The listing endpoint is outside the vendor's documented compatibility surface** — **Unverified**
  until a real key says otherwise. A native endpoint may want headers `llm/client::signed` cannot
  send: it has exactly two branches, bearer or nothing.

A comment arguing for a value the row no longer holds is worse than none. Replace it with the one fact
a user cannot look up — a mainland / international host split, a compatibility path versioned in an
unusual shape, why this arm and not `None`.

## Adding a provider preset

Require that **the request terminates at the company whose key it carries**. An inference provider
serving somebody else's open weights on its own GPUs qualifies; a broker holding keys to other APIs
and relaying does not, because it puts a third party inside a relationship that was between the user
and a vendor — no OpenRouter, no LiteLLM, no gateway.
`every_preset_goes_direct_to_its_own_vendor` enforces this by name list, so a broker absent from
`BROKERS` satisfies every type and nothing else notices. Check the vendor by hand.

The same test requires a distinct `id` (it is the credential account, so a duplicate shares a key), a
non-empty `label`, `https://` unless `is_local()`, and a `key_page` for anything remote and none for
anything local. It says nothing about whether either URL still resolves, which is why a dead one
survives it.

Leave `model` empty — it is the `row` helper's only unconditional field. Confirm `api_url` builds the
URL you expect for this `base_url` before anything else: a compatibility path versioned unusually is a
row that cannot work however right its other fields are.

`reasoning` defaults to `None` in the `row` helper, and `None` is usually right — "nothing on the wire
either way" covers every plain OpenAI-compatible endpoint, including the reasoning models a user picks
deliberately. Depart from it only where the endpoint documents a switch that turns reasoning **off**,
and write the comment naming which switch and why, as the departing rows do. "Off" means off: a floor
that still reasons is not one, and claiming it would send a value meaning something the user did not
ask for.

A vendor whose compatibility layer documents no `/models` listing is a row whose dropdown may never
fill. Settle that with a real key before the row ships.

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
   user reads what may go in the field.

Reuse an existing arm only where **every** documented value matches. An arm whose off-value matches
while its on-value is one the vendor does not document is the exact failure the enum exists to
prevent.

Then answer the question the arm's name hides: **does every model behind that endpoint take the
field?** A dialect is the endpoint's property (ADR-0021) and the row still selects it — but
`api.deepseek.com` still answers to legacy ids that picked the mode through the id itself, and
`api.openai.com` serves families that reject the effort floor. Where the host is mixed, have the arm
consult a per-model list beside it (`CATALOG`, `EFFORT_NONE_FAMILIES`) and make the unmatched case the
safe direction: `Omit` where silence costs only the feature, a refusal only where silence would cost
invisible latency on every turn.

Those per-model lists are the only shipped model ids this register permits, so a third one carries the
same burden: it answers *how to connect* rather than *what to run*, a wrong value is a `400` rather
than a worse model, and it holds ids only — no labels, no descriptions, no ordering, nothing a user
ever sees. A list wanted for any other reason belongs in the endpoint's own fetched list.

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

- **A `key_page` or `base_url` that moved.** The `301` answers, so nothing fails.
- **A `base_url` whose compatibility path `api_url` misreads.** The URL is built deterministically and
  no test reaches the network, so the row `404`s and reads to the user as their own key or their own
  network. `zhipu` shipped that way.
- **`chatUrl` in `providers.ts` drifting from `api_url`.** There is no JS test runner, so the pane can
  render a URL Beckon does not post to and every gate stays green.
- **A listing endpoint that does not answer**, leaving that row's dropdown unfillable.
- **A description that is stale but present.** Both language fields being non-empty and different is
  the whole assertion.
- **A broker added as a preset** whose name is not in `BROKERS`.
- **A wrong `reasoning`** on a preset row, including one that was right when written and was made
  wrong by the vendor adding a switch. It types fine and fails as a `400` on the user's first turn.
- **A withdrawn model still in a cached list.** `models.json` holds what an endpoint said last time;
  nothing expires it, and only Refresh models replaces it.
- **A model family missing from `EFFORT_NONE_FAMILIES`.** Suppression is silently dropped for it — the
  turn runs, slower, and nothing says why.
- **A stale provenance date.** It is prose.
- **A row nobody read.** Which is why the per-row verdict table, not the bumped date, is the artifact.

An audit that says "all passing" without saying "checked against the vendor on this date", per row, has
reported the wrong thing.

## Not part of this flow

- **A user's own model choice.** A configured id is surfaced, never rewritten, even when nothing
  vouches for it (`Origin::Configured`). Do not "correct" a config file.
- **The live list, and the cache behind it.** `get_models` offers what the endpoint serves and
  `models_cache.rs` remembers it (ADR-0024). There is no documented fallback: adding a model to
  `CATALOG` is not how *any* endpoint's models appear, DeepSeek's included.
- **Temperature as a control.** ADR-0019 decided there is no temperature UI; ADR-0021 moved the pinned
  DeepSeek value onto that vendor's row. A preset may carry one; nothing else may.
- **Per-model capability gates.** The catalog records no image column — nothing is gated on one
  (ADR-0016).
- **Contradicting an ADR.** A change needing the register to *work* differently rather than to *say*
  something different needs a new ADR first (see [CLAUDE.md](../../../CLAUDE.md)). Adding a
  `Reasoning` arm is *not* one of those **when the row alone decides it**: ADR-0021 made the dialect a
  property of the row precisely so such an arm is data. An arm needing a **per-model list** is the
  other case — 0021 named per-model knowledge as the thing putting the dialect on the row got rid of,
  so a second list narrows it and gets recorded (0021's own footnote, or a new ADR). Moving the model
  list off the row into a cache was such a change and is now **ADR-0024**; admitting a broker is still
  open and would be **ADR-0025**. Neither is a fact this flow may settle on its own.
