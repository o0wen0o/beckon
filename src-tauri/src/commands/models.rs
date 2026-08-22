//! The model dropdown's one command. Its whole job is to never leave the
//! dropdown empty, and to say by cause why the list is not the live one.
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

/// What the model dropdown renders.
#[derive(Debug, Clone, Serialize)]
pub struct ModelCatalog {
    pub options: Vec<ModelOption>,
    /// `true` when the options came from the endpoint's own list.
    pub live: bool,
    /// Why the documented list is being shown instead, by cause: ADR-0005 needs
    /// "no credential", "read error" and "key rejected" to stay three different
    /// things, and this is the last hop before the UI.
    pub fallback: Option<Failure>,
}

/// The models the user may pick from, for one provider row.
///
/// **Never fails.** A dropdown that goes empty because the machine is offline
/// would be worse than the problem it reports, so a failed fetch downgrades to
/// whatever can be vouched for and says why. Whatever the configuration already
/// names is always among the options — a value we cannot vouch for gets
/// surfaced, never quietly rewritten.
///
/// `live = false` answers without touching the network or the credential store.
/// Settings asks for that first: the fetch is deliberately unbounded (client.rs
/// has no timeout), so the dropdown has to be usable before it is attempted.
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
                live: false,
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

    // One decision, one literal: `live` cannot then disagree with the options.
    let (fetched, fallback) = if !live {
        (None, None)
    } else {
        match fetch_model_ids(&http, &provider, language).await {
            Ok(ids) => (Some(ids), None),
            Err(failure) => (None, Some(failure)),
        }
    };

    ModelCatalog {
        options: models::options(
            fetched.as_deref(),
            // The catalog is DeepSeek's own list, so it stands in only for
            // DeepSeek's own host (ADR-0021).
            provider.is_deepseek_host(),
            &configured,
            language,
        ),
        live: fetched.is_some(),
        fallback,
    }
}

async fn fetch_model_ids(
    http: &reqwest::Client,
    provider: &Provider,
    language: Language,
) -> Result<Vec<String>, Failure> {
    // The cause only — the UI adds "so the documented list is shown", because
    // that consequence is the dropdown's to explain, not this layer's.
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
