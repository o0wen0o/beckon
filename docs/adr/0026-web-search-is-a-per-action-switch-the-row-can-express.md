---
status: accepted
---

# Web search is a per-Action switch, and the row says whether it can be expressed

Every answer Beckon streams comes out of what the model was trained on. For most Actions that is the
right trade: a translation, a rewrite and a summary of a Selection have no use for the live web, and
paying for a search on each of them would make the fastest thing in the product slower and dearer for
nothing.

Some Actions are the other case — "what changed in this API", "is this still true" — and for those the
training cut-off is the whole failure. So **searching is a switch, per Action, off unless asked**, and
it lands as `[model] web_search` beside `thinking`.

## The shape is `thinking`'s, with the polarity reversed

Nothing new was built for this. It is `Option<bool>` on `ModelOverrides`, a `bool` on the `Provider`
row it inherits from, one more field in `ModelParams`, and one more mapping in `llm/request.rs`. The
override chain ADR-0021 designed already answers the hard question: overriding `provider` moves what
`web_search` inherits, exactly as it moves `model` and `thinking`.

What differs is the direction, and it changes two things:

- **`Reasoning` names off-switches; `Search` names on-switches.** Thinking is what endpoints do unless
  stopped, so the interesting direction there is `false` and `Reasoning::None` means "cannot be
  stopped". Nobody searches the web unless asked, so the interesting direction here is `true` and
  `Search::None` means "cannot be asked". Off is *silence* on every arm but one — xAI documents `on`
  as the default of its `search_parameters` object, so that row is told `off` in as many words, for
  the same reason DeepSeek is told `disabled`.
- **The default is off and stays off.** `false` on every preset row, `false` when an Action names a
  provider id that matches nothing, `false` in `Provider::default`. A search is billed per request on
  top of the tokens, so the switch is the consent, and a test — `no_preset_searches_until_it_is_asked_to`
  — holds it.

## The dialect is the endpoint's, which is ADR-0021 again

`Search` is a field on the row, prefilled by its preset, for the reason ADR-0021 gave about
`Reasoning`: which field an endpoint reads cannot be derived from a model id, and an unknown field is
a `400` on a strict endpoint rather than a courtesy. The arms are the endpoints whose search is **one
field on the same body and one round trip** — xAI's `search_parameters`, DashScope's `enable_search`,
OpenRouter's `plugins: [{"id": "web"}]`.

Three consequences worth stating, because each one is a "why is this not doing more":

- **A built-in tool is not an arm.** Moonshot's `$web_search` is declared as a `builtin_function` and
  answered with a `tool_calls` frame the caller must echo back before any answer arrives. That is a
  second request, and `exchange/turn.rs` streams one. Those hosts are `Search::None` until a tool loop
  is a feature rather than a field — which would be its own ADR, since it changes what a turn *is*.
- **No per-model list, and no error.** `thinking_wire` consults the model three times because a wrong
  answer there costs the turn. Here a host serving a model that ignores its own search field — a tier
  behind DashScope the field was never documented for — costs the *feature*, and a per-model list
  would rot in exchange for a wrong nothing. So `search_wire` reads only the row, and asking a
  `Search::None` endpoint to search is amber in Settings and silence on the wire, never a refusal.
  That is the trade ADR-0021 already made for thinking in the on-direction.

  **Superseded in part by ADR-0027**, and only for what the *control* offers: a pairing the vendor
  documents as excluded — DashScope's Max tiers, whose search is on an API Beckon does not post to —
  greys the switch out rather than accepting a `true` that reaches nothing. The wire half of this
  bullet is untouched: `search_wire` still reads only the row and still never fails.
- **Nothing detects it.** `llm/detect.rs` certifies a thinking dialect with a one-token probe; the
  same probe for search would run a real search and be billed for it. So a preset states its arm and a
  hand-made row is *asked* — the one place where this field is a control where `reasoning` is a
  statement.

## What is not decided here

- **Citations.** The named arms fold their results into the same completion, and `llm/wire.rs` reads
  `content` and the reasoning field. Where an endpoint also streams sources as annotations, Beckon
  drops them today; rendering them is a separate change to the Popover and to `wire.rs`, and this ADR
  does not pretend to have made it. DashScope's compatible mode does not return sources at all, which
  is a fact about that endpoint rather than a gap here.
- **Whether a given host is worth trusting with the query.** A search sends the question onward from
  the endpoint, and OpenRouter's runs at a broker (ADR-0025). The disclosure that row already carries
  is the answer; this switch adds a reason to read it.
