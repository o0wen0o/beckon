---
name: model-register
description: "Maintain Beckon's two hand-kept vendor tables, `presets()` and `CATALOG`, when a model or endpoint fact changes or must be re-checked: adding or retiring a DeepSeek model, adding a provider preset, adding a reasoning or web-search dialect, a moved base_url or key page, a listing endpoint that stopped answering, or auditing the register against what the vendors now publish. Examples: \"add the new DeepSeek model\", \"a provider row 400s\", \"add Cerebras as a preset\", \"does this endpoint support web search?\", \"is the register still current?\", \"the model dropdown is empty\", \"the key page link is dead\""
---

# Maintaining the model register

Beckon discovers no vendor fact. Two hand-kept tables carry them:

- **`presets()`** in [config.rs](../../../src-tauri/src/config.rs) — one `Provider` row per
  first-party endpoint: **where to fetch and how to connect, never what to run.** `base_url`,
  `key_page`, and the two fields a user cannot look up, `Reasoning` and `Search`. A wrong arm is a
  `400` on every turn that row serves — every turn for `Reasoning`, every *searching* turn for
  `Search` (ADR-0026).
- **`CATALOG`** in [llm/models.rs](../../../src-tauri/src/llm/models.rs) — DeepSeek's models and what
  each does with thinking mode. It feeds `thinking_wire` in
  [llm/request.rs](../../../src-tauri/src/llm/request.rs) and nothing else, so **only its `thinking`
  column is load-bearing**; label, description and order merely enrich ids the endpoint itself named.

**A row ships no model.** `presets()` leaves `model` empty on every row but `deepseek`; the list a
user picks from is the endpoint's own `GET {base_url}/models`, kept between launches in
[models_cache.rs](../../../src-tauri/src/models_cache.rs) (ADR-0024). Adding a `CATALOG` row puts the
model in nobody's dropdown — what the row buys is the `thinking` answer and a readable label. So the
highest-impact check is not "is this id current" but "does the listing endpoint still answer, and
with what credential".

Tests in both files guard shape, never currency — no assertion knows that a vendor renamed a model
yesterday. Read the current values out of the tables; **this file names no model id and no vendor
version**, because a stale example in a skill reads as a recommendation.

```bash
sed -n '/pub fn presets/,/^}/p' src-tauri/src/config.rs | grep -n '"'
grep -rn 'TODO(register)' src-tauri/src        # the open questions, held at their site
```

## Pick the branch

| Task | Read |
| --- | --- |
| Adding or retiring a DeepSeek model | [catalog.md](catalog.md) |
| Adding a provider preset | [preset.md](preset.md) |
| Adding a `Reasoning` arm | [dialect.md](dialect.md) |
| Adding a `Search` arm, or answering whether an endpoint can search | [search.md](search.md) |
| Auditing the register — quarterly, and after any report of a preset that `400`s | [audit.md](audit.md) |

Every branch uses the two checks below and ends at [Not done until](#not-done-until).

## Checking a vendor's facts

Take every id and every URL from an authority, in this order. Memory is not an authority, and neither
is the next value in an apparent pattern.

1. **The vendor's own docs** — model list, pricing page, and the deprecation or changelog page, the
   only place a shutdown date exists. Where a vendor publishes a deprecation table, use its named
   replacement over your own judgement of the nearest tier.
2. **The vendor's overview page**, for whether a value is still their current one. Their ordering is
   their recommendation, so a row sitting several entries below the top is superseded whether or not
   it still answers. This is the only check behind audit.md's **Live but superseded** verdict.
3. **The URLs themselves.** Fetch every `key_page` and `base_url` and record the status code. A `301`
   is drift, not a pass. Mechanical, so this is the one part of an audit that is verified rather than
   trusted:

   ```bash
   curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' "$URL"
   ```

   Compare the answer against the `base_url` as `fold_legacy` leaves it, not as it was written:
   `normalise_base_url` supplies a missing scheme (`http` for loopback and the private ranges,
   `https` otherwise), drops trailing slashes, and strips a pasted `/chat/completions` or
   `/completions` tail. It deliberately strips no `models` segment, which can legitimately be part of
   a compatibility root. A row you "fixed" by deleting a slash changed nothing.
4. **The endpoint itself** — [the listing check](#checking-that-a-row-can-be-filled-at-all). Two
   requests, not one.

DeepSeek's three pages are cited by name and date in the `models.rs` module doc — follow those links
rather than re-deriving them.

Bump a table's checked date even when the check confirmed it was already right: that date is the only
record of when the list was last known good.

## Checking that a row can be filled at all

A row's whole offer is `GET {base_url}/models`, so that request is the check — **two results per row**,
recorded separately:

```bash
curl -s -o /dev/null -w 'anon  %{http_code}\n' "$BASE_URL/models"
curl -s -w '\nauth  %{http_code}\n' "$BASE_URL/models" -H "Authorization: Bearer $KEY" | head -40
```

The authenticated ids are also what settles an exact id string when some other question needs one.
Run the anonymous one too: `fetch_model_ids` calls `require_api_key` **before** any HTTP, so a
non-local row with no key stored never asks — whether or not the endpoint would have answered, and
whether that gate should stay cannot be settled without this column.

Outcomes:

- **Both answer** — the row is fine, and could be filled before a key is stored if Beckon asked.
- **Only the authenticated one answers** — the row is fine, and the current gate is correct for it.
- **Neither answers** — the row cannot be filled. On a **new** row that is a block, not a caveat: ship
  nothing rather than a row on which no model can ever be chosen. On an existing row, record it with
  what would settle it and mark the site with a `TODO(register):`.
- **The listing endpoint is outside the vendor's documented compatibility surface** — **Unverified**
  until a real key says otherwise. A native endpoint may want headers `llm/client::signed` cannot
  send: it has exactly two branches, bearer or nothing.

## Every hand-kept site

| What | Where | Changes when |
| --- | --- | --- |
| `CATALOG` rows — `id`, `thinking`, `retired` | `llm/models.rs` | DeepSeek ships, renames, or withdraws a model |
| `label`, `description` + `description_zh` per row | same rows | with the row; both languages live here, not in `src/lib/i18n/`. Enrichment only — a stale one mislabels an id the endpoint served, it does not offer a wrong model |
| Provenance dates in the module doc | `llm/models.rs` header | every re-check of the DeepSeek docs |
| `DEFAULT_MODEL` | `config.rs` | the default is retired or renamed. The **one** shipped model id outside a dialect's per-model list |
| `DEEPSEEK_TEMPERATURE` | `config.rs` | DeepSeek changes its own guidance (ADR-0019 → ADR-0021) |
| `DEFAULT_BASE_URL`, `DEFAULT_PROVIDER_ID`, `DEFAULT_PROVIDER_LABEL`, `DEFAULT_KEY_PAGE` | `config.rs` | DeepSeek moves a host or a key page |
| `presets()` rows — `base_url`, `key_page`, `reasoning`, `search`, and the **checked date** in the doc comment | `config.rs` | any vendor moves a host or a key page, or a new endpoint is added; the date on every audit. **Not `model`** |
| The version rule in `api_url`, and its mirror `chatUrl` | [llm/client.rs](../../../src-tauri/src/llm/client.rs), [providers.ts](../../../src/lib/providers.ts) | a vendor's compatibility path is versioned in a shape the rule misreads. Move the two together: nothing links them, and a pane on the old rule draws a URL Beckon does not post to |
| `BROKERS`, and its mirror in `providers.ts` | `config.rs`, [providers.ts](../../../src/lib/providers.ts) | a broker is added as a preset, or a new relaying host appears (ADR-0025). Move the two together: the pane draws its disclosure from the TS half |
| A `Reasoning` arm and its seven mirrors — `DETECTABLE` and `build_dialect_probe` in `llm/request.rs`, [types.ts](../../../src/lib/types.ts), `REASONINGS` in [Connection.tsx](../../../src/settings/sections/Connection.tsx), `reasoningName`/`reasoningHint` in [en.ts](../../../src/lib/i18n/en.ts) + `zh.ts`, the legal-values comment in [README.md](../../../README.md) | `config.rs` + those files | a new wire dialect. Only the first site is compiler-checked; [dialect.md](dialect.md) walks all eight. `Reasoning::guess` is frozen — legacy files only |
| A `Search` arm and its six mirrors — `types.ts`, `SEARCH_WIRES` in `Connection.tsx`, `searchName`/`searchHint` in `en.ts` + `zh.ts`, the legal-values comment in `README.md` | `config.rs` + those files | a vendor documents or withdraws a one-field web search on `/chat/completions`. [search.md](search.md) walks all nine sites; nothing detects a `Search` arm, so the dropdown is the only way a hand-made row reaches one |
| `Search::supports_model` — the model families each arm searches with, and the vendor's own exclusions | `config.rs` | a vendor moves web search onto or off a tier (ADR-0027). Families, never ids; **silence is `None`, not `false`**, because `false` greys the switch out in Settings |
| `EFFORT_NONE_FAMILIES` + its checked date | `llm/request.rs` | OpenAI ships or withdraws a family documenting `reasoning_effort: "none"`. An **allow**-list |
| `ALWAYS_THINKING_MINIMAX` + its checked date | same file | MiniMax ships or withdraws a family it documents as unable to stop thinking. A **deny**-list where the row above is an allow-list — read each with its polarity in hand |
| Model ids in the config examples | [README.md](../../../README.md) | an id in an example no longer exists |
| Model ids in the manual test checklist | [docs/macos-testing.md](../../../docs/macos-testing.md) | same; the vision and non-vision rows each name one |

Leave model ids quoted inside `docs/adr/` alone. An ADR records what was decided when, so a rotted id
there is history — fix it only if the ADR is being superseded anyway.

## Comments at the site

A comment arguing for a value the row no longer holds is worse than none. Replace it with the one fact
a user cannot look up — a mainland / international host split, a compatibility path versioned in an
unusual shape, why this arm and not `None`.

## Not done until

The gates pass:

```bash
npx tsc --noEmit
(cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
```

`cargo test models::` and `cargo test config::` cover this work specifically, but the frontend mirror
of `Reasoning` is only checked by `tsc` — run both. If a toolchain is missing in the environment you
are working in, say the gates did not run; never infer them from inspection.

**…and** the report states what the gates cannot see, rather than reporting green as current:

- **A `key_page` or `base_url` that moved.** The `301` answers, so nothing fails.
- **A `base_url` whose compatibility path `api_url` misreads.** The URL is built deterministically and
  no test reaches the network, so the row `404`s and reads to the user as their own key or their own
  network. `zhipu` shipped that way.
- **`chatUrl` in `providers.ts` drifting from `api_url`.** There is no JS test runner, so the pane can
  render a URL Beckon does not post to and every gate stays green.
- **A listing endpoint that does not answer**, leaving that row's dropdown unfillable.
- **A description that is stale but present.** Both language fields being non-empty and different is
  the whole assertion.
- **A `supports_model` family that ages into a wrong `false`.** Nothing fails: the switch is greyed,
  Settings says the model does not take the field, and the user reads a vendor claim Beckon made up.
  Re-read it whenever a vendor's tier names change.
- **A relaying row that discloses nothing** — a broker whose host `relays()` does not recognise, or a
  `BROKERS` list edited in `config.rs` without its `providers.ts` mirror. Both type fine; the pane
  simply stays quiet, which is the failure the old ban was really guarding against (ADR-0025).
- **An arm missing from `DETECTABLE` or `build_dialect_probe`.** Test connection never proposes it,
  and nothing anywhere reports that it did not.
- **A wrong `reasoning`** on a preset row, including one that was right when written and was made
  wrong by the vendor adding a switch. It types fine and fails as a `400` on the user's first turn.
- **A wrong or missing `search`.** Missing costs the feature silently — the Action runs, the endpoint
  never hears the question, and only the amber line in Settings says so. Wrong costs a `400`, but only
  on the turns that asked to search, so it survives every gate and most use.
- **A row that ships `web_search = true`.** `no_preset_searches_until_it_is_asked_to` catches a preset;
  nothing catches one written into a fixture or a seed. It is billed per request.
- **A withdrawn model still in a cached list.** `models.json` holds what an endpoint said last time;
  nothing expires it, and only Refresh models replaces it.
- **A family missing from `EFFORT_NONE_FAMILIES`** — suppression is silently dropped, so the turn runs
  slower and nothing says why — or **missing from `ALWAYS_THINKING_MINIMAX`**, the opposite polarity
  at the same cost: the field is sent, MiniMax accepts it and thinks anyway, and the refusal that
  would have told the user never fires.
- **A stale provenance date, or a row nobody read.** Both are why an audit's artifact is the per-row
  verdict table rather than the bumped date.

## Not part of this flow

- **A user's own model choice.** A configured id is surfaced, never rewritten, even when nothing
  vouches for it (`Origin::Configured`). Leave a config file's id as the user wrote it.
- **The live list, and the cache behind it.** `get_models` offers what the endpoint serves and
  `models_cache.rs` remembers it (ADR-0024). There is no documented fallback: adding a model to
  `CATALOG` is not how *any* endpoint's models appear, DeepSeek's included.
- **Temperature as a control.** ADR-0019 decided there is no temperature UI; ADR-0021 moved the pinned
  DeepSeek value onto that vendor's row. A preset may carry one; nothing else may.
- **Per-model capability gates.** The catalog records no image column — nothing is gated on one
  (ADR-0016), and `search_wire` consults no model either: a model ignoring its host's search field
  costs the feature, not the turn (ADR-0026).
- **A search that takes a tool loop.** Kimi's `$web_search` and its kind need a second round trip; a
  turn is one request. That is an ADR, not a register entry ([search.md](search.md)).
- **Contradicting an ADR.** A change needing the register to *work* differently rather than to *say*
  something different needs a new ADR first (see [CLAUDE.md](../../../CLAUDE.md)). Adding a
  `Reasoning` arm is *not* one of those **when the row alone decides it** — ADR-0021 made the dialect
  a property of the row precisely so such an arm is data. An arm needing a **per-model list** is the
  other case: 0021 named per-model knowledge as the thing putting the dialect on the row got rid of,
  so each further list narrows it and gets recorded in 0021's footnote or a new ADR.
