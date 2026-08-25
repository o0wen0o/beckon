# Adding a provider preset

A row in `presets()` in [config.rs](../../../src-tauri/src/config.rs). Take every URL from the vendor
per *Checking a vendor's facts* in [SKILL.md](SKILL.md), and finish at *Not done until* there.

## Ask first where the request terminates

An inference provider serving somebody else's open weights on its own GPUs terminates at itself: the
key is theirs, the inference is theirs, nothing is forwarded. A broker holds keys to *other* APIs and
relays to one of them, so a third party ends up inside a relationship that was between the user and a
vendor, and Beckon cannot say what it does with the text.

That used to be a ban. Since **ADR-0025** it is a **disclosure**: a broker may be a preset, provided
`Provider::relays` recognises its host, so the row says so on the identity line before any key is
stored. `every_relaying_preset_says_so` asserts that weaker, load-bearing thing — not that no row
relays, but that none relays *silently*. So a broker whose host is missing from `BROKERS` satisfies
every type and passes every test while disclosing nothing. Answer the question by hand, and when the
answer is "it relays", add the host to `BROKERS` in `config.rs` **and** to the mirror in
[providers.ts](../../../src/lib/providers.ts), which is what the pane actually draws from.

"It discloses" is not itself a reason to add a row. The list stays curated: a row is still an
endpoint someone would deliberately choose, and nothing here lets a *default* relay.

## The fields

The same test requires a distinct `id` (it is the credential account, so a duplicate shares a key), a
non-empty `label`, `https://` unless `is_local()`, and a `key_page` for anything remote and none for
anything local. It says nothing about whether either URL still resolves, which is why a dead one
survives it.

Confirm `api_url` builds the URL you expect for this `base_url` before anything else: a compatibility
path versioned unusually is a row that cannot work however right its other fields are.

Leave `model` empty — it is the `row` helper's only unconditional field.

`reasoning` defaults to `None` in the `row` helper, and `None` is usually right — "nothing on the wire
either way" covers every plain OpenAI-compatible endpoint, including the reasoning models a user picks
deliberately. Depart from it only where the endpoint documents a switch that turns reasoning **off**,
and write the comment naming which switch and why, as the departing rows do. "Off" means off: a floor
that still reasons is not one, and claiming it would send a value meaning something the user did not
ask for. A dialect the register has no arm for is [dialect.md](dialect.md).

`search` defaults to `None` in the `row` helper too, and the bar for departing from it — one field on
the same body, one round trip, the endpoint runs the search, and `web_search` still `false` — is in
[search.md](search.md).

Do not expect Test connection to settle either field. It reports a detected dialect, but
`detect::reasoning` returns early on `is_preset`: detection is strictly the weaker source and could
only ever talk the register out of something read off the vendor's own docs. It answers for a row a
user typed; it never audits this table. For `search` it says nothing at all — nothing probes for one,
so the vendor's docs are the only source there is.

## Before the row ships

Run *Checking that a row can be filled at all* in [SKILL.md](SKILL.md) with a real key. A vendor whose
compatibility layer documents no `/models` listing is a row whose dropdown may never fill — on a new
row, **Neither answers** is a block, not a caveat.
