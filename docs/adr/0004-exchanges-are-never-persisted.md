# Exchanges are never persisted; closing the Popover destroys them

One Action trigger opens one Exchange, which supports multiple follow-up turns inside the Popover but is discarded when the window closes — nothing written to disk, no history list, no search.

This is a **deliberate scope decision**, not an unimplemented feature. The value of this tool is in the moment: translating a sentence, asking a passing question. Content like that has very little review value, and the moment you introduce persistence you drag in a storage layer, a session-list UI, search, and a retention policy — that is a different product, and the DeepSeek web app already does it.

## Consequences

- There is no storage layer. An Exchange is pure in-memory state, created and destroyed along with the Popover window.
- Follow-up turns resend the full history to the API, so tokens grow linearly with each turn. Because nothing is persisted, a single Exchange is inherently short-lived, so this growth has a natural ceiling and needs no extra truncation strategy.
- If a user wants to keep something, the path is "Copy" in the Popover — it is the only export mechanism, so it has to be good.
