---
status: accepted
---

# Fetched model lists are cached beside the config

Beckon keeps the last model list each endpoint served in `models.json`, a sidecar file in the config
directory. It is written after a successful `GET {base_url}/models`, read when that fetch is not
attempted or does not answer, and it is not the user's file: not watched, not broadcast, not
hand-editable in any meaningful sense.

This narrows ADR-0004's "there is no storage layer", which is edited to point here. It also
**supersedes [ADR-0021](0021-any-openai-compatible-endpoint-chosen-per-action.md) in part**: that
decision's documented-catalog fallback — `llm/models.rs` standing in as the list for DeepSeek's own
host — is gone entirely rather than narrowed, because a row carries no catalog at all now. The rest
of 0021 stands, including the table itself as the single source the request layer reads for
`thinking`, and 0021 is edited to point here.

## Why there is anything to cache

A provider row used to ship a `model`, and `CATALOG` in `llm/models.rs` used to stand in as a
documented list when no fetch had happened. Both are gone
([docs/register-audit-2026-08-25.md](../register-audit-2026-08-25.md)): a row carries where to fetch
and how to connect, never what to run, because a hand-kept model id rots silently and no gate in this
repository can see it happen — `glm-5.1` sat two generations behind GLM-5.3, resolving happily, on
green gates.

The consequence is that the endpoint's own list is now the *only* source of options. Without
somewhere to keep it, every fresh process starts with an empty dropdown on every row but the default
one, and the fetch is deliberately unbounded (`llm/client.rs` has no timeout), so asking again is not
free.

**What this buys is narrower than it first looks, and the ADR should say so.** The Settings webview
is created hidden at startup and never destroyed (ADR-0007), and `SettingsStore` is a module-level
singleton whose `models` map nothing clears on `settings:opened` — so closing Settings never lost the
list. Only a process restart did. This file is therefore one avoided round trip per row after a
restart, not a fix for a list that kept disappearing. That is worth a file; it is not worth a
subsystem, which is why the shape below is as small as it is.

## Why not `config.toml`

ADR-0003 makes `config.toml` and `actions/` the source of truth: the user writes them, a watcher
reads them, and every change is broadcast whole through `reload.rs`.

A fetched list is none of that. Putting it in `config.toml` would put it on the broadcast path, so it
would echo back at the window that caused the fetch and fight the save protocol `saveSlot.ts`
implements — a text field losing focus mid-edit because a dropdown finished loading. It would also
mean Beckon rewriting the user's file on its own, which ADR-0003 exists to prevent.

So it is its own file, and the watcher never sees it: the watcher is recursive over the config root,
but `is_ignored` drops anything that is not a `.toml`, which covers both `models.json` and the
`.models.json.beckon-tmp` that `atomic::write_atomic` publishes it from. There is nothing for
`SelfWrites` to suppress, and registering the path would be cargo cult.

## Why ADR-0004 survives

ADR-0004's subject is the **Exchange** — the conversation. Its argument is that persisting one drags
in a session list, search and a retention policy, which is a different product. All of that still
holds: nothing here records a prompt, an answer, a Capture or a turn.

What is narrowed is the flat sentence in its consequences, "There is no storage layer." There is now
one file that is neither the user's config nor an Exchange. It is kept as small as the job allows:
per provider id, the ids the endpoint listed and the URL they were listed from. No labels, no
descriptions, no ordering, no timestamps, no prompts.

## Decisions

- **One writer.** `atomic::write_atomic` builds a *fixed* temp path per target, so two concurrent
  writers would interleave into one `.models.json.beckon-tmp` and both rename — publishing a splice,
  atomically. That is reachable: opening Settings primes every row at once, and `get_models` is an
  async command. So the cache lives behind the `Mutex` on `AppState`, every write happens under it,
  and the whole document is rewritten rather than patched.
- **Validity is the built `models_url`, not the row's `base_url`.** The two stopped being
  interchangeable when `client::api_url` learned that a version segment can sit anywhere in the path,
  so an entry written before that change must not still look valid. A row whose endpoint moved has no
  entry, not a wrong one.
- **Two events drop an entry outright.** Removing a row's credential, because a list fetched with a
  key that is gone has stopped being something the endpoint vouches for; and removing the row, which
  already deletes that row's credential so a row re-added under the same id cannot inherit it
  (ADR-0021) — the list has to go the same way, for the same reason.
- **A cached list is not a live one.** `ModelCatalog` carries `cached` beside `live`, and `live` keeps
  its meaning: the endpoint answered *just now*. Folding a cache hit into `live` would have been the
  cheaper wiring and it would have destroyed a diagnostic — the Connection pane suppresses its
  fallback notice when `live` is set, so a rejected key, an unreadable credential store and an offline
  machine would all have rendered a full dropdown, the words "Listed by this endpoint", and no cause.
  ADR-0005 requires those three stay three things all the way to the UI; this is the field that lets a
  cache hit keep its own.
- **Unreadable is empty, never an error.** Missing, unparsable and wrong-version all mean "no
  entries". Nothing in the file is the user's work, so there is nothing to preserve by reporting, and
  the endpoint is still there to ask.

## Consequences

- The config directory holds a third file. It is Beckon's, not yours; deleting it costs one fetch.
  README's directory listing says so.
- A key *replaced* with one on a different organisation or tier cannot be detected: `base_url` is
  unchanged and Beckon never sees the outgoing key, so the entry may name a model the new key cannot
  call. Same for a local row after an `ollama pull`, where the list changes and nothing about the row
  does. Refresh models is the answer to both, and it is one click from where the wrong list is shown.
- Whether a cached entry should say *how old* it is remains open, and is deliberately not decided
  here. Recording a timestamp would need a dependency this crate does not declare, to write a field
  nothing yet reads; the audit's §4.5 carries the question, and the code carries a `TODO(register):`.
- The broker ADR the same audit calls for (§5.1) is therefore **ADR-0025**, not 0024.
