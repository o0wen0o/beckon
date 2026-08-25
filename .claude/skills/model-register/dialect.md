# Adding a reasoning dialect

For the web-search half of the same idea, read [search.md](search.md) instead.

A `Reasoning` arm says how an endpoint is told **not** to think. Eight edits, and only the first is
compiler-checked. Take every fact from the vendor per *Checking a vendor's facts* in
[SKILL.md](SKILL.md), and finish at *Not done until* there.

1. The `Reasoning` arm in [config.rs](../../../src-tauri/src/config.rs), plus its match in
   `thinking_wire` and, if the dialect puts a new field on the wire, a `ThinkingWire` variant and its
   arm in `build_body`.
2. A wire test in [request.rs](../../../src-tauri/src/llm/request.rs) beside the existing per-arm
   ones, asserting the exact field in both directions. Every other arm has one; a dialect without one
   is a `400` waiting for a user to find.
3. `DETECTABLE` in `request.rs` — the arms Test connection probes for on a hand-made row. An arm
   absent from it is never proposed, and the user is back to knowing an answer they cannot look up.
4. `build_dialect_probe`'s match in the same file — the body that certifies the arm. Return `None`
   only for an arm no single request could distinguish, and say which one that is in the comment.
5. `Reasoning` in [types.ts](../../../src/lib/types.ts) — a string union, so a missing arm is silent.
6. `REASONINGS` in [Connection.tsx](../../../src/settings/sections/Connection.tsx) — the dropdown's
   order, hand-listed.
7. `reasoningName` and `reasoningHint` in **both** i18n catalogs. `en.ts` fixes the shape and `zh.ts`
   must satisfy it, so this pair is checked once step 5 is done.
8. The `reasoning = ...` legal-values comment in [README.md](../../../README.md), the only place a
   user reads what may go in the field.

## Reuse an existing arm only where every documented value matches

An arm whose off-value matches while its on-value is one the vendor does not document is the exact
failure the enum exists to prevent.

## Hold the new probe against every existing one

`detect::reasoning` answers `None` the moment two arms are accepted, so a probe that is a **superset**
of another's makes *both* undetectable — MiniMax's is DeepSeek's plus one field, and that pair is
expected back ambiguous. Ambiguity is a correct answer rather than a bug: `None` leaves the row
exactly as the user set it, where a coin-flip would cost every turn. But if the new arm makes an arm
that *was* detectable stop being one, say so in the report rather than shipping the loss quietly.

## Does every model behind that endpoint take the field?

A dialect is the endpoint's property (ADR-0021) and the row still selects it — but `api.deepseek.com`
still answers to legacy ids that picked the mode through the id itself, and `api.openai.com` serves
families that reject the effort floor. Where the host is mixed, have the arm consult a per-model list
beside it (`CATALOG`, `EFFORT_NONE_FAMILIES`, `ALWAYS_THINKING_MINIMAX`) and make the unmatched case
the safe direction: `Omit` where silence costs only the feature, a refusal only where silence would
cost invisible latency on every turn. Which polarity the list takes follows from what the wrong send
does — an allow-list where the field is a `400` on anything unlisted, a deny-list where the field is
accepted and ignored — and both arrive at the same rule about the case the list has not heard of.

Those per-model lists are the only shipped model ids this register permits, so a further one carries
the same burden: it answers *how to connect* rather than *what to run*, a wrong value is a `400`
rather than a worse model, and it holds ids only — no labels, no descriptions, no ordering, nothing a
user ever sees. A list wanted for any other reason belongs in the endpoint's own fetched list. A new
list also narrows ADR-0021 and gets recorded (its footnote, or a new ADR).

## Two things to leave alone

- Confirm nothing else keys off the arm by name — `src/lib/models.ts` asks only whether the row is
  `none`, so a new arm is treated as throwable without an edit. Check that this is still true rather
  than assuming it.
- `Reasoning::guess`. It runs on exactly one thing — a config file written before the provider table
  existed (ADR-0021) — and a new dialect has no legacy files to migrate.

A dialect only ever sent in one direction is normal — express the asymmetry in `thinking_wire`, where
it can carry a comment, rather than in `build_body`, which stays a dumb translation of the wire
variant it is handed.
