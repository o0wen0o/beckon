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
   Then re-read any **per-model list an arm consults** — today `CATALOG` for the DeepSeek arm,
   `EFFORT_NONE_FAMILIES` for the OpenAI one, `ALWAYS_THINKING_MINIMAX` for the MiniMax one. Read each
   with its polarity in hand, because they fail from opposite directions to the same place: a family
   missing from the **allow**-list `EFFORT_NONE_FAMILIES` silently drops suppression, while a family
   missing from the **deny**-list `ALWAYS_THINKING_MINIMAX` is sent `disabled` by a host that accepts
   it and thinks anyway. Neither is a `400`; both are invisible latency on every turn.
4. Bump the checked date in every doc comment carrying one — today `CATALOG`, `presets()`,
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
