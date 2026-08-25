# Adding and retiring a DeepSeek model

`CATALOG` in [llm/models.rs](../../../src-tauri/src/llm/models.rs). Take every fact from the vendor
per *Checking a vendor's facts* in [SKILL.md](SKILL.md), and finish at *Not done until* there.

## Adding

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

## Retiring

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
