# Auditing the register

The one procedure to run on a schedule rather than on a trigger. Roughly quarterly, and after any
report of a preset that `400`s.

1. Print both tables. `CATALOG` and `presets()` are the whole surface.
2. Check each row against the vendor per *Checking a vendor's facts* in [SKILL.md](SKILL.md), and give
   it one verdict:
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
   Then re-read every **per-model list an arm consults** — today `CATALOG` for the DeepSeek arm,
   `EFFORT_NONE_FAMILIES` for the OpenAI one, `ALWAYS_THINKING_MINIMAX` for the MiniMax one — each
   with its polarity in hand, the allow-list and the deny-list failing from opposite directions to the
   same invisible latency on every turn.
4. Re-read each row's `search` the same way, and ask the one question that has no gate: **does this
   endpoint document a one-field web search on `/chat/completions` today?** The polarity is the
   opposite of `reasoning`'s, so the drift is too — a vendor *adding* a search field leaves a correct
   `None` merely stale, costing the feature, while a vendor changing a field's name or its required
   values turns a working arm into a `400` on every searching turn. Both survive the gates; only the
   second is loud, and only for the users who turned it on. Read any `TODO(register)` about a search
   field while you are there — one is open on `zhipu` (see [search.md](search.md)).
5. Re-read `Search::supports_model` against the same pages: which tiers the vendor documents the field
   for, and which it excludes (ADR-0027). A family name that aged out fails silently *and* visibly —
   the switch is greyed and Settings tells the user their model cannot search, on Beckon's word
   rather than the vendor's — so a stale `Some(false)` is the worse half of this row to leave.
6. Bump the checked date in every doc comment carrying one — today `CATALOG`, `presets()`,
   `EFFORT_NONE_FAMILIES` and `ALWAYS_THINKING_MINIMAX` — whatever you changed.

**Not done until** the audit has written **one line per row, carrying that row's verdict**, into a
dated table at `docs/register-audit-YYYY-MM-DD.md`, and the gates in [SKILL.md](SKILL.md) have run. A
row with no line is unchecked, not fine: a bumped date alone cannot distinguish a row that was read
from a row that was skipped, and one date covering a dozen rows is how a superseded id survives an
audit that "passed". A table under `docs/` rather than a doc comment, so `config.rs` does not double
in size and the next audit has a diff to read. Questions the audit opens but may not settle stay at
their site as `TODO(register):`.

An audit that says "all passing" without saying "checked against the vendor on this date", per row,
has reported the wrong thing.
