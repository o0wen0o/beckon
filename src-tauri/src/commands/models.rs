//! The model dropdown's one command. Its job is to offer whatever can be
//! vouched for, and to say by cause why the list is not the live one.
//!
//! It used to be "never leave the dropdown empty", by falling back to a
//! documented catalog. A provider row carries no catalog any more
//! (`docs/register-audit-2026-08-24.md`), so an empty answer is now a real
//! answer — the state of a row whose endpoint has never spoken — and the pane
//! renders it rather than a select rendering a blank box.
//!
//! Asked **per provider** since ADR-0021: there is one `base_url` and one key
//! per row, so "the model list" is a different list per endpoint. Which is also
//! why `configured` is gathered per provider — see [`get_models`].

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::config::{Language, Provider};

use crate::i18n;
use crate::llm::client;
use crate::llm::models::{self, ModelOption};
use crate::state::AppState;

use super::{secrets::require_api_key, Failure};

/// Where the list the dropdown is rendering came from.
///
/// One field rather than a `live` and a `cached` flag: those were two booleans
/// spelling one three-state, and `live && cached` — a fresh answer that is also
/// last time's — was a combination nothing but prose forbade. The distinction
/// itself is load-bearing, which is why it did not collapse into one flag: the
/// pane suppresses its fallback notice for a live list, so a cached list
/// reported as live would show a full dropdown, the words "listed by this
/// endpoint", and no sign that the key was rejected. ADR-0005's three causes
/// have to stay three things all the way to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelSource {
    /// The endpoint answered **just now**.
    Live,
    /// The list this endpoint served last time (ADR-0024), because it was not
    /// asked or did not answer.
    Cached,
    /// The endpoint has never answered. Whatever is on offer is only what the
    /// configuration names, and on a fresh row that is nothing at all.
    None,
}

/// What the model dropdown renders.
#[derive(Debug, Clone, Serialize)]
pub struct ModelCatalog {
    pub options: Vec<ModelOption>,
    pub source: ModelSource,
    /// Why the list is not the endpoint's own answer, by cause: ADR-0005 needs
    /// "no credential", "read error" and "key rejected" to stay three different
    /// things, and this is the last hop before the UI. Set alongside a
    /// [`ModelSource::Cached`] or [`ModelSource::None`] list when a fetch failed.
    pub fallback: Option<Failure>,
}

/// The models the user may pick from, for one provider row.
///
/// **Never fails.** A failed fetch downgrades to whatever can be vouched for —
/// the list this endpoint served last time, then whatever the configuration
/// names — and says why. A configured value is always among the options: it gets
/// surfaced, never quietly rewritten.
///
/// `live = false` answers without touching the network or the credential store.
/// Settings asks for that first: the fetch is deliberately unbounded (client.rs
/// has no timeout), so the dropdown has to be usable before it is attempted.
/// With a cache behind it, that first answer is now usually a real list rather
/// than just the row's own model.
///
/// An unknown `provider_id` still answers, with an empty option set rather than
/// an error: this command's contract is that the dropdown always renders, and
/// the missing row is reported by the pane that names it.
#[tauri::command]
pub async fn get_models(app: AppHandle, provider_id: String, live: bool) -> ModelCatalog {
    // Locks are dropped before the await (state.rs).
    let (provider, configured, http, language) = {
        let state = app.state::<AppState>();
        let config = state.config.read().expect("config lock");

        // Ahead of the gathering below rather than after it: an unknown row has
        // nothing to gather options *for*.
        let Some(provider) = config.api.find(&provider_id).cloned() else {
            return ModelCatalog {
                options: Vec::new(),
                source: ModelSource::None,
                fallback: None,
            };
        };
        let registry = state.registry.read().expect("registry lock");

        // Every model id this *row* is named alongside: its own, plus each
        // Action that resolves to it. Gathered per provider, because an Action
        // pinning `deepseek-v4-pro` while pointing at Ollama must still see its
        // own value in Ollama's dropdown — that value is what would go on the
        // wire, and a select whose value is missing silently rewrites it.
        //
        // Resolved through `Config::provider_id`, which is where "an Action
        // naming no provider is on the default row" lives: a second spelling of
        // that fallback is a dropdown that disagrees with the wire (ADR-0021).
        let mut configured: Vec<String> = vec![provider.model.clone()];
        configured.extend(registry.actions.iter().filter_map(|action| {
            (config.provider_id(action.file.model.provider.as_deref()) == provider_id)
                .then(|| action.file.model.model.clone())
                .flatten()
        }));

        (provider, configured, state.http.clone(), config.language)
    };

    // The validity key for a cached entry, and the URL a fetch would use: one
    // expression, so the two cannot disagree about which endpoint this is.
    let models_url = client::models_url(&provider.base_url);

    // One decision, one literal: `live` cannot then disagree with the options.
    let (fetched, fallback) = if !live {
        (None, None)
    } else {
        match fetch_model_ids(&http, &provider, language).await {
            Ok(ids) => (Some(ids), None),
            Err(failure) => (None, Some(failure)),
        }
    };

    // Stated before the store consumes `fetched`, so the source below is a fact
    // rather than something inferred back out of the options.
    let fresh = fetched.is_some();

    // The lock is taken after the await and dropped before the return (state.rs).
    // Stored and then read back through the same guard rather than kept in hand:
    // one path to the ids means neither arm clones the endpoint's whole list to
    // hand `options` a copy the cache already holds. `options` is pure — no
    // lock, no I/O — so building it here costs the guard nothing.
    let (options, has_ids) = {
        let state = app.state::<AppState>();
        let mut cache = state.models_cache.lock().expect("model cache lock");
        if let Some(ids) = fetched {
            cache.store(&provider_id, &models_url, ids);
        }
        // Not asked, or asked and refused: the last list this endpoint served is
        // the best thing that can be vouched for, and `fallback` above still
        // carries why it is not a fresh one.
        let ids = cache.get(&provider_id, &models_url);
        (models::options(ids, &configured, language), ids.is_some())
    };

    ModelCatalog {
        options,
        source: match (fresh, has_ids) {
            (true, _) => ModelSource::Live,
            // The endpoint's own ids, but not its answer today.
            (false, true) => ModelSource::Cached,
            (false, false) => ModelSource::None,
        },
        fallback,
    }
}

async fn fetch_model_ids(
    http: &reqwest::Client,
    provider: &Provider,
    language: Language,
) -> Result<Vec<String>, Failure> {
    // The cause only — the UI adds what is being shown instead, because that
    // consequence is the dropdown's to explain and not this layer's.
    let key = require_api_key(provider, i18n::models_need_key(language), language)?;

    let ids = client::list_models(http, &provider.base_url, key.as_deref())
        .await
        .map_err(Failure::from)?;
    if ids.is_empty() {
        // An endpoint that serves nothing would leave the user unable to change
        // model at all. Treat it as no answer.
        return Err(Failure::new("empty", i18n::models_empty(language)));
    }
    Ok(ids)
}
