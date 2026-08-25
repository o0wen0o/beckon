---
status: accepted
---

# A preset may relay, if the row says so

ADR-0021 shipped a rule about what may appear in the provider preset list: **the request has to
terminate at the company whose key it carries.** No aggregator, no gateway, no OpenRouter. A test,
`every_preset_goes_direct_to_its_own_vendor`, held it in place by name, because a broker satisfies
the `Provider` type and nothing else in the codebase would have noticed.

That rule is now a **disclosure** rather than a ban. A preset may point at a broker, and
`Provider::relays` is what makes the row say so — on the identity line, under the URL it is about,
before a key has been stored.

## What has not changed

The distinction ADR-0021 drew is still the right one and is still worth stating in full, because the
reason for it did not weaken.

It was never "does this company own the model". A hosted vLLM serving somebody else's open weights on
its own GPUs is not a broker: your key is theirs, the inference is theirs, and nothing is forwarded.
A broker is different in kind. It holds keys to *other* APIs and relays your request to one of them,
so a third party ends up inside a relationship that was between you and a vendor — and Beckon cannot
tell you what that third party does with the text, because it has no way to know.

All of that is still true of OpenRouter today. Nothing about the risk changed.

## What changed is who decides

The ban answered a question on the user's behalf that was theirs to answer. Someone who wants one key
that reaches forty models, or who wants a model no first-party row can serve, has a real reason to
accept a relay — and the old rule did not make that unavailable, it only made it undocumented. They
could still type `https://openrouter.ai/api/v1` into a blank row, and the product would say nothing
at all about what that meant. The ban therefore protected the users who were never at risk and left
the ones who went there anyway with **less** information than a preset would have given them.

A disclosure inverts that. The row that relays says so, whether it came from the preset list or from
a person typing a URL, and the choice stays with the person whose text it is.

## Why the disclosure is derived, not a field

`Provider::relays` matches the host against `BROKERS`. It is not a field on the row, and the
difference matters: a field would have to be filled in by whoever created the row, which means the
hand-typed OpenRouter URL — the one case where the user is least likely to already know — would
disclose nothing.

This is a host guess, which ADR-0021 refuses everywhere else, and the asymmetry is deliberate.
`Reasoning::guess` is banned because a wrong dialect is a `400` on every turn, so the row has to
*state* what it speaks. A wrong broker match costs a warning nobody needed. The two are not the same
kind of wrong, and the rule that governs one should not govern the other.

## Consequences

- `BROKERS` moves out of the test and into `config.rs` as the list both the rule and the test read.
- `every_preset_goes_direct_to_its_own_vendor` becomes `every_relaying_preset_says_so`. It asserts the
  weaker but load-bearing thing: a preset may name a broker, but not one `relays()` fails to
  recognise. A row that relayed *silently* is the failure the ban was really guarding against, and
  that is now what the test guards.
- `src/lib/providers.ts` mirrors the list, the way it already mirrors `is_local`'s. The two halves
  move together or a relaying row stops disclosing.
- OpenRouter joins the preset list, with `Reasoning::Openrouter` for its dialect.

## What this does not open

The list is still curated, and "it discloses" is not a reason to add anything. A row still has to be
an endpoint someone would deliberately choose, and the disclosure is the price of admission for a
broker rather than a licence for the category. Nothing here says a *default* may relay: `deepseek`
remains what a fresh install gets, and ADR-0021's "no active row, one per Action" is untouched.
