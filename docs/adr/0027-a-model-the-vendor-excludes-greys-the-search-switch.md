---
status: accepted
---

# A model the vendor excludes greys the web search switch

ADR-0026 said the model is not consulted: `search_wire` reads the row, an endpoint that cannot be
asked to search is amber in Settings and silence on the wire, and no per-model list is kept. The
argument was that a wrong answer costs the *feature* rather than the turn, so a list would rot in
exchange for nothing.

Re-checking the arms against the vendors' own pages on 2026-08-25 turned up the case that argument
did not cover. Alibaba documents `enable_search` on the OpenAI-compatible endpoint for the Qwen Plus
and Flash tiers; the Max tiers take web search through their Responses API, which Beckon does not
post to. So a DashScope row set to `qwen3.7-max` offers a switch, bills nothing, changes nothing, and
says nothing — the user asked for a search, got an answer without one, and there is no way to tell
from the answer.

**A pairing the vendor excludes greys the switch out.** Not a refusal, not a wire change: the control
stops offering something its own vendor says would do nothing.

## What this narrows, and what it leaves alone

ADR-0026 is superseded **in part**, in one direction:

- **The wire is untouched.** `search_wire` still reads only the row, still sends the arm's field for
  any model, and still never fails. An Action file that says `web_search = true` against an excluded
  model runs exactly as it did — the field goes out, the endpoint ignores it, the turn is fine. Every
  reason 0026 gave for that stands, and none of them was about what a *control* should offer.
- **What the model can do is now a field, not a list.** `Search::supports_model` answers per arm,
  and the arms answer by family: xAI and OpenRouter say yes to everything, because the endpoint runs
  the search before or beside the model rather than through it; DashScope names Plus and Flash;
  `Search::None` says no to everything, which is the row's own fact restated. There is no table of
  ids, which is the thing 0026 refused to keep, and family matching ages at the speed the family
  does.
- **Silence is not a no.** An id no arm recognises is `None` — the switch stays offered. An arm that
  answered `false` for everything it had not heard of would grey out each new model on the day it
  shipped, which is the failure mode 0026 was actually guarding against.

## The switch is greyed in one direction only

Blocked means "cannot be turned **on** here". A setting that is already `true` — inherited from the
row, or written into an Action file before the model was changed — leaves the switch live, because
that switch is the only control that could clear it. Disabling it in both directions would trap the
value and then explain, in amber, that it does nothing.

The amber says which of the two facts applies: the endpoint has no search field at all (0026's
reading), or this model does not take the one it has (this ADR's).

## Where the answer comes from

Rust, and only Rust. `Search::supports_model` runs where the dropdown is built, so each `ModelOption`
carries `search: Option<bool>` beside the `thinking: Option<Thinking>` that has been there since
ADR-0021 — same tri-state, same meaning for `None`. The frontend reads the field and greys a control;
it derives nothing, which is what keeps `src/lib/providers.ts`'s rule ("state it twice only if it
cannot drift") from having to absorb a table of vendor model families.
