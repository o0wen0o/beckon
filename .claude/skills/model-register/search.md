# Adding a web-search dialect

A `Search` arm says how an endpoint is **asked** to search the web (ADR-0026). The mirror of
[dialect.md](dialect.md) with the polarity flipped, and shorter by three steps, because nothing
probes for one. Finish at *Not done until* in [SKILL.md](SKILL.md).

1. The `Search` arm in [config.rs](../../../src-tauri/src/config.rs), with the comment naming the
   exact field, whether the endpoint searches unless told not to, and the date it was checked.
2. A `SearchWire` variant and its arm in `apply_search` and `search_wire` in
   [request.rs](../../../src-tauri/src/llm/request.rs).
3. The arm in `Search::supports_model` in [config.rs](../../../src-tauri/src/config.rs) (ADR-0027) —
   `Some(true)` where the endpoint searches with everything it serves, and a family match only where
   the vendor documents an exclusion. `None` for anything the vendor is silent about: that keeps the
   switch offered, and a `false` you guessed greys a working pairing out.
4. A wire test beside the existing ones asserting the exact field. `a_row_that_can_search_says_nothing_until_asked`
   already loops the arms it knows — add yours to that array, so the off-direction is covered too.
5. The preset row's `search:` in `presets()`, if a shipped row speaks it.
6. `Search` in [types.ts](../../../src/lib/types.ts) — a string union, so a missing arm is silent.
7. `SEARCH_WIRES` in [Connection.tsx](../../../src/settings/sections/Connection.tsx) — the dropdown's
   order, hand-listed. Unlike `reasoning`, this one **is** a control on a hand-made row: nothing
   detects it, so an arm missing here can never be chosen by anyone who did not use a preset.
8. `searchName` and `searchHint` in **both** i18n catalogs, and the hint names the model families
   where `supports_model` names any. `en.ts` fixes the shape, `zh.ts` must
   satisfy it.
9. The `search = ...` legal-values comment in [README.md](../../../README.md).

## What qualifies as an arm

**One field on the same body, one round trip, and the endpoint runs the search.** Everything else is
`None`, and the three ways a host fails that test are each worth recognising by name:

- **A built-in tool the caller must answer.** Moonshot's `$web_search` replies with a `tool_calls`
  frame the caller echoes back before any answer arrives. That is a second request; `exchange/turn.rs`
  streams one. Not an arm, and not a `TODO(register)` either — it needs an ADR, because it changes
  what a turn is.
- **A field that is only real on some models.** OpenAI documents web search on `/chat/completions`
  for its search-specialised models, which search on every turn with no field to ask them to. The
  switch there is the model id, which the dropdown already offers. Not an arm.
- **A field whose required value you cannot pin down.** Zhipu's `web_search` tool is server-run in one
  round trip — it would qualify — but its `search_engine` field is required and their mainland and
  international docs name different values for it. A wrong value is a `400` on every searching turn,
  so it stays `None` behind a `TODO(register)` until a real key on that host settles it.

## Off has to be expressible too

Ask the vendor's docs one extra question every time: **does this endpoint search when the field is
absent?** Most do not, and for those, off is silence and the arm carries no direction. xAI documents
`on` as its object's default, so `Search::Xai` sends `{"mode": "off"}` explicitly — the same
insurance `Reasoning::Deepseek` buys, and for the same reason: a default nobody chose is not consent,
and here it is billed.

Where the on-direction has more than one documented value, pick the one that matches what the switch
promises. `auto` over `on` at xAI, because `on` searches every source on every turn whether the
question needed one or not, and the user asked for "search the web", not "search always".

## The model gates the control, never the wire

`thinking_wire` consults the model three times because a wrong answer there costs the turn. A model
that ignores its host's search field costs the *feature*, so `search_wire` still reads only the row
and no arm may be a hard error. What ADR-0027 added is one step earlier: `Search::supports_model`
answers whether the vendor documents this pairing, the dropdown carries the answer, and Settings
greys the switch where the answer is `false`.

Three rules when you write an arm's answer:

- **Families, not ids.** The vendor documents tiers; match the tier. A list of exact model names is
  what ADR-0026 refused to keep and ADR-0027 did not bring back.
- **Silence is `None`, not `false`.** Only the vendor's own exclusion greys a switch. An arm that
  says no to every id it has not heard of greys out each new model on the day it ships.
- **The wire stays lenient.** Never make `search_wire` read the model. An Action file written before
  a model changed must keep running — the field goes out, the endpoint ignores it, the turn is fine.

## What ships off

`web_search` is `false` on every preset and `no_preset_searches_until_it_is_asked_to` holds it. A
search is billed per request on top of the tokens, so the arm is the capability and the `false` is
the consent. A row that arrives on with no user having asked is the one failure this whole branch is
guarding against.
