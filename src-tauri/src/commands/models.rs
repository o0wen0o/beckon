//! The model dropdown's one command. Its whole job is to never leave the
//! dropdown empty, and to say by cause why the list is not the live one.

use serde::Serialize;
use tauri::{AppHandle, Manager};

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

/// The models the user may pick from.
///
/// **Never fails.** A dropdown that goes empty because the machine is offline
/// would be worse than the problem it reports, so a failed fetch downgrades to
/// the officially documented list and says why. Whatever the configuration
/// already names is always among the options — a value we cannot vouch for gets
/// surfaced, never quietly rewritten.
///
/// `live = false` answers from the catalog alone, touching neither the network
/// nor the Credential Manager. Settings asks for that first: the fetch is
/// deliberately unbounded (client.rs has no timeout), so the dropdown has to be
/// usable before it is attempted.
#[tauri::command]
pub async fn get_models(app: AppHandle, live: bool) -> ModelCatalog {
    // Locks are dropped before the await (state.rs).
    let (base_url, configured, http) = {
        let state = app.state::<AppState>();
        let config = state.config.read().expect("config lock");
        let registry = state.registry.read().expect("registry lock");
        let mut configured = vec![config.defaults.model.clone()];
        configured.extend(
            registry
                .actions
                .iter()
                .filter_map(|action| action.file.model.model.clone()),
        );
        (config.api.base_url.clone(), configured, state.http.clone())
    };

    // One decision, one literal: `live` cannot then disagree with the options.
    let (fetched, fallback) = if !live {
        (None, None)
    } else {
        match fetch_model_ids(&http, &base_url).await {
            Ok(ids) => (Some(ids), None),
            Err(failure) => (None, Some(failure)),
        }
    };

    ModelCatalog {
        options: models::options(fetched.as_deref(), &configured),
        live: fetched.is_some(),
        fallback,
    }
}

async fn fetch_model_ids(http: &reqwest::Client, base_url: &str) -> Result<Vec<String>, Failure> {
    // The cause only — the UI adds "so the documented list is shown", because
    // that consequence is the dropdown's to explain, not this layer's.
    let key = require_api_key("Store one to list the models your endpoint actually serves.")?;

    let ids = client::list_models(http, base_url, &key)
        .await
        .map_err(Failure::from)?;
    if ids.is_empty() {
        // An endpoint that serves nothing would leave the user unable to change
        // model at all. Treat it as no answer.
        return Err(Failure::new("empty", "Its list came back empty."));
    }
    Ok(ids)
}
