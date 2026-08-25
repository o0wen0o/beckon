//! The last model list each endpoint gave us, kept beside the config (ADR-0024).
//!
//! A provider row carries where to fetch and how to connect, never what to run
//! (`docs/register-audit-2026-08-25.md`), so the list a user picks from comes
//! from the endpoint and nowhere else. Without somewhere to keep it, every fresh
//! process starts with an empty dropdown on every row but the default one — and
//! the fetch is deliberately unbounded (`llm/client.rs` has no timeout), so
//! "just ask again" is not free.
//!
//! ## Not config, and not an Exchange
//!
//! ADR-0003 makes `config.toml` and `actions/` the user's, watched, and
//! broadcast whole through `reload.rs`. This file is none of that: the user did
//! not write it, editing it by hand would be meaningless, and a fetched list
//! landing on the broadcast path would echo back at the window that caused the
//! fetch and fight the save protocol. ADR-0004 says there is no storage layer;
//! ADR-0024 is the carve-out, and it is narrow on purpose — ids, and the URL they
//! were fetched from.
//!
//! The watcher never reports it: it is recursive over the config root, but
//! `is_ignored` drops anything that is not a `.toml`, which covers both this file
//! and the `.models.json.beckon-tmp` `write_atomic` publishes it from. So there
//! is nothing for `SelfWrites` to suppress, and registering the path would be
//! cargo cult.
//!
//! ## One writer
//!
//! [`crate::atomic::write_atomic`] builds a **fixed** temp path per target, so
//! two concurrent writers would interleave into one `.models.json.beckon-tmp`
//! and both rename — publishing a splice, atomically. That is reachable: opening
//! Settings primes every row at once. So this lives behind the `Mutex` on
//! [`AppState`](crate::state::AppState) and every write happens under it, which
//! is also why the whole document is rewritten rather than patched.
//!
//! ## What invalidates an entry
//!
//! The **built** `models_url`, not the row's `base_url`: the two stopped being
//! interchangeable when `client::api_url` learned that a version segment can sit
//! anywhere in the path, so an entry written before that change must not still
//! look valid. A row whose endpoint moved therefore has no entry, not a wrong one.
//!
//! Two callers drop an entry outright — `delete_api_key`, because a list fetched
//! with a key that is gone has stopped being anything the endpoint vouches for,
//! and `save_config`'s removed-row loop, which already deletes that row's
//! credential so a row re-added under the same id cannot inherit it (ADR-0021).
//!
//! TODO(register): a key *replaced* by one on a different org or tier cannot be
//! detected — `base_url` is unchanged and Beckon never sees the outgoing key — so
//! the entry can name a model the new key may not call. Same for a local row
//! after an `ollama pull`, where the list changes and nothing about the row does.
//! Refresh models is the answer to both; the audit's §4.5 records the open
//! question of whether a cached entry should say how old it is.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;

/// Bumped when the shape changes. An unrecognised version reads as empty rather
/// than as an error: the file is a convenience, and the endpoint is still there.
const VERSION: u32 = 1;

/// `PartialEq` is load-bearing: it is what lets [`ModelsCache::store`] tell a
/// list that changed from the same list fetched again, and skip the write.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Entry {
    /// The URL the ids came from, not the row's `base_url` — see the module docs.
    models_url: String,
    ids: Vec<String>,
}

/// The on-disk shape, versioned so a future change can refuse an old file
/// instead of misreading it.
///
/// Held by [`ModelsCache`] rather than built per write: the version is a
/// constant, so reconstructing it each time bought nothing and cost a clone of
/// every entry.
#[derive(Debug, Serialize, Deserialize)]
struct Document {
    version: u32,
    /// Keyed by [`Provider::id`](crate::config::Provider::id), which is also the
    /// credential account — so the two are dropped together or not at all.
    providers: HashMap<String, Entry>,
}

pub struct ModelsCache {
    path: PathBuf,
    document: Document,
}

impl ModelsCache {
    /// Read the file, or start empty.
    ///
    /// **Never fails.** Missing, unreadable, unparsable and wrong-version all
    /// mean "no entries" — the same posture `config.rs` takes towards a missing
    /// file, and for a stronger reason: nothing here is the user's work, so there
    /// is nothing to preserve by reporting.
    pub fn load(path: &Path) -> Self {
        let providers = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Document>(&text).ok())
            .filter(|document| document.version == VERSION)
            .map(|document| document.providers)
            .unwrap_or_default();
        Self {
            path: path.to_path_buf(),
            document: Document {
                version: VERSION,
                providers,
            },
        }
    }

    /// The ids this endpoint last served, if the entry was fetched from the URL
    /// we would ask now.
    pub fn get(&self, provider_id: &str, models_url: &str) -> Option<&[String]> {
        self.document
            .providers
            .get(provider_id)
            .filter(|entry| entry.models_url == models_url)
            .map(|entry| entry.ids.as_slice())
    }

    /// Record what an endpoint just served.
    ///
    /// An unchanged list is not written. Almost every call is one: opening
    /// Settings, opening a row, pressing Refresh models and storing a key all
    /// re-fetch a list that has not moved, and [`write_atomic`] ends in
    /// `sync_all` — which is the expensive thing in this module by two orders of
    /// magnitude, and the one worth not doing four times for nothing.
    pub fn store(&mut self, provider_id: &str, models_url: &str, ids: Vec<String>) {
        let entry = Entry {
            models_url: models_url.to_string(),
            ids,
        };
        if self.document.providers.get(provider_id) == Some(&entry) {
            return;
        }
        self.document
            .providers
            .insert(provider_id.to_string(), entry);
        self.persist();
    }

    /// Drop one row's entry: its key is gone, or the row is.
    pub fn forget(&mut self, provider_id: &str) {
        self.forget_all(std::iter::once(provider_id));
    }

    /// Drop several rows' entries in **one** write.
    ///
    /// Takes an iterator rather than being called in a loop, because
    /// [`persist`](Self::persist) ends in `sync_all`: N removed rows through
    /// `forget` would be N fsyncs for what the user did as one edit, which is
    /// the batching `save_config`'s single guard is there to make possible.
    /// Nothing is written when nothing was there to remove.
    pub fn forget_all<'a>(&mut self, provider_ids: impl IntoIterator<Item = &'a str>) {
        let mut removed = false;
        for id in provider_ids {
            removed |= self.document.providers.remove(id).is_some();
        }
        if removed {
            self.persist();
        }
    }

    /// Logged rather than returned. The caller is a dropdown that has already
    /// answered, and there is nothing a user could do about it — the list is
    /// still correct in memory, it just will not survive the process.
    fn persist(&self) {
        match serde_json::to_string_pretty(&self.document) {
            Ok(text) => {
                if let Err(err) = write_atomic(&self.path, &text) {
                    log::warn!("could not write the model cache: {err}");
                }
            }
            Err(err) => log::warn!("could not serialise the model cache: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(of: &[&str]) -> Vec<String> {
        of.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn an_entry_survives_a_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");

        let mut cache = ModelsCache::load(&path);
        assert!(cache
            .get("openai", "https://api.openai.com/v1/models")
            .is_none());
        cache.store(
            "openai",
            "https://api.openai.com/v1/models",
            ids(&["gpt-5.6-terra", "gpt-5.6-sol"]),
        );

        let again = ModelsCache::load(&path);
        assert_eq!(
            again.get("openai", "https://api.openai.com/v1/models"),
            Some(ids(&["gpt-5.6-terra", "gpt-5.6-sol"]).as_slice())
        );
    }

    /// The URL is the validity check, and it is the *built* one — which is what
    /// makes an entry written before `api_url` changed shape unusable rather than
    /// wrong. A row repointed at another endpoint has no list, not somebody
    /// else's.
    #[test]
    fn an_entry_is_ignored_once_the_url_moves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");

        let mut cache = ModelsCache::load(&path);
        cache.store(
            "zhipu",
            "https://open.bigmodel.cn/api/paas/v4/v1/models",
            ids(&["glm-5.1"]),
        );
        assert!(cache
            .get("zhipu", "https://open.bigmodel.cn/api/paas/v4/models")
            .is_none());
        assert!(cache
            .get("zhipu", "https://open.bigmodel.cn/api/paas/v4/v1/models")
            .is_some());
    }

    #[test]
    fn forgetting_a_row_reaches_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");

        let mut cache = ModelsCache::load(&path);
        cache.store("a", "u", ids(&["one"]));
        cache.store("b", "u", ids(&["two"]));
        cache.forget("a");

        let again = ModelsCache::load(&path);
        assert!(again.get("a", "u").is_none());
        assert!(again.get("b", "u").is_some());
    }

    /// Several rows in one write, and no write at all for ids that were never
    /// here — the same `sync_all` argument as `storing_an_unchanged_list`, and
    /// checked the same way: a write would repair the corrupted file.
    #[test]
    fn forgetting_several_rows_is_one_write_and_none_is_no_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");

        let mut cache = ModelsCache::load(&path);
        cache.store("a", "u", ids(&["one"]));
        cache.store("b", "u", ids(&["two"]));
        cache.store("c", "u", ids(&["three"]));

        cache.forget_all(["a", "b"]);
        let again = ModelsCache::load(&path);
        assert!(again.get("a", "u").is_none());
        assert!(again.get("b", "u").is_none());
        assert!(again.get("c", "u").is_some());

        fs::write(&path, "{not json").unwrap();
        cache.forget_all(["a", "nobody"]);
        assert_eq!(fs::read_to_string(&path).unwrap(), "{not json");
    }

    /// Every unreadable shape is "no entries", never an error: the file is a
    /// convenience and the endpoint is still there to ask.
    #[test]
    fn a_file_we_cannot_read_is_simply_empty() {
        let dir = tempfile::tempdir().unwrap();

        let missing = dir.path().join("nothing.json");
        assert!(ModelsCache::load(&missing).get("a", "u").is_none());

        let corrupt = dir.path().join("corrupt.json");
        fs::write(&corrupt, "{not json").unwrap();
        assert!(ModelsCache::load(&corrupt).get("a", "u").is_none());

        // A version this build does not know: refused whole rather than read as
        // far as it happens to parse.
        let future = dir.path().join("future.json");
        fs::write(
            &future,
            r#"{"version":99,"providers":{"a":{"models_url":"u","ids":["one"]}}}"#,
        )
        .unwrap();
        assert!(ModelsCache::load(&future).get("a", "u").is_none());
    }

    /// Storing a list that has not moved touches nothing, which is what keeps
    /// four `sync_all`s out of one Settings open. Checked by corrupting the file
    /// behind the cache's back: a write would repair it, and a skip leaves it.
    #[test]
    fn storing_an_unchanged_list_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");

        let mut cache = ModelsCache::load(&path);
        cache.store("a", "u", ids(&["one"]));
        fs::write(&path, "{not json").unwrap();

        cache.store("a", "u", ids(&["one"]));
        assert_eq!(fs::read_to_string(&path).unwrap(), "{not json");

        // A list that *did* move writes, so the skip is a skip and not a stall.
        cache.store("a", "u", ids(&["two"]));
        assert_eq!(
            ModelsCache::load(&path).get("a", "u"),
            Some(ids(&["two"]).as_slice())
        );
    }

    /// A write over a document this build wrote must replace it, not merge with
    /// it — the whole file is rewritten under one lock for exactly that reason.
    #[test]
    fn a_second_write_replaces_the_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");

        let mut cache = ModelsCache::load(&path);
        cache.store("a", "u", ids(&["one"]));
        cache.store("a", "u", ids(&["two"]));

        assert_eq!(
            ModelsCache::load(&path).get("a", "u"),
            Some(ids(&["two"]).as_slice())
        );
    }
}
