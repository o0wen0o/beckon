//! Credential commands, plus the one place "no credential", "read error" and
//! "key rejected" are turned into three distinguishable [`Failure`]s (ADR-0005).
//!
//! Every one of them names a provider (ADR-0021): with an endpoint per Action,
//! one row can be missing its key while every other one works, so a status is
//! never "the key" — it is always *whose*.

use std::collections::HashMap;

use tauri::{AppHandle, Manager};

use crate::config::{Language, Provider, Reasoning};
use crate::i18n;
use crate::llm::{client, detect};
use crate::secrets::{self, KeyStatus};
use crate::state::AppState;

use super::Failure;

/// Every configured row's credential status, in one read.
///
/// A map rather than one command per row: the Connection pane draws the whole
/// inventory at once, and N round trips to render one list is N chances for the
/// list to be drawn half-answered.
///
/// `async`, and the reads handed to a blocking task: this is N round trips
/// through `keyring`'s FFI, and a synchronous command runs them on the
/// event-loop thread — the same thread that serves the Popover.
#[tauri::command]
pub async fn get_key_statuses(app: AppHandle) -> HashMap<String, KeyStatus> {
    let ids: Vec<String> = app
        .state::<AppState>()
        .config_snapshot()
        .api
        .providers
        .into_iter()
        .map(|provider| provider.id)
        .collect();
    tauri::async_runtime::spawn_blocking(move || {
        ids.into_iter()
            .map(|id| {
                let status = secrets::status(&id);
                (id, status)
            })
            .collect()
    })
    .await
    .expect("credential status task")
}

/// One row's status, for the operations that change exactly one.
///
/// Beside [`get_key_statuses`] rather than instead of it: the inventory wants
/// the whole map in one answer, but a key saved, removed or tested on one row
/// says nothing about the other N — and paying N credential-store reads to learn
/// that is the cost this exists to avoid.
#[tauri::command]
pub async fn get_key_status(provider_id: String) -> KeyStatus {
    tauri::async_runtime::spawn_blocking(move || secrets::status(&provider_id))
        .await
        .expect("credential status task")
}

#[tauri::command]
pub fn set_api_key(provider_id: String, key: String) -> Result<KeyStatus, String> {
    secrets::write(&provider_id, &key)?;
    Ok(secrets::status(&provider_id))
}

/// Also drops this row's cached model list (ADR-0024): a list fetched with a key
/// that is gone has stopped being something the endpoint vouches for, and
/// offering it afterwards is the "the list has to stop being live when the key
/// that fetched it is gone" rule the audit's §4.3 states.
#[tauri::command]
pub fn delete_api_key(app: AppHandle, provider_id: String) -> Result<KeyStatus, String> {
    secrets::delete(&provider_id)?;
    app.state::<AppState>()
        .models_cache
        .lock()
        .expect("model cache lock")
        .forget(&provider_id);
    Ok(secrets::status(&provider_id))
}

/// The row a command was asked about, or a [`Failure`] naming what is missing.
pub(super) fn require_provider(app: &AppHandle, provider_id: &str) -> Result<Provider, Failure> {
    let state = app.state::<AppState>();
    let config = state.config_snapshot();
    config.api.find(provider_id).cloned().ok_or_else(|| {
        Failure::new(
            "config",
            i18n::provider_missing(config.language, provider_id),
        )
    })
}

/// The stored key for one row, or why there is none.
///
/// `Ok(None)` is a real answer: a local endpoint wants no `Authorization` header,
/// so nothing stored for one is a working setup rather than a fault (ADR-0021).
/// On a remote host the two failing outcomes stay two different things all the
/// way to the UI (ADR-0005), so the split lives here once; callers supply only
/// the sentence that varies.
pub(crate) fn require_api_key(
    provider: &Provider,
    when_missing: &str,
    language: Language,
) -> Result<Option<String>, Failure> {
    match secrets::read(&provider.id) {
        Ok(Some(key)) => Ok(Some(key)),
        Ok(None) if provider.is_local() => Ok(None),
        Ok(None) => Err(Failure::new("no-credential", when_missing)),
        Err(message) => Err(Failure::new(
            "read-error",
            i18n::credential_unreadable(language, &message),
        )),
    }
}

/// What one connection test learned.
///
/// A struct rather than `()` so the test can answer the second question a user
/// would otherwise have to answer themselves — see [`crate::llm::detect::reasoning`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionReport {
    /// The dialect this endpoint was observed to accept, when exactly one was.
    /// `None` means "keep whatever the row says": either nothing was learned, or
    /// there was nothing to learn because the row came from a preset.
    pub reasoning: Option<Reasoning>,
}

/// "Test connection": one minimal request to one row, reporting a rejected key
/// separately from an unreachable API (ADR-0005) — and, on a row no preset
/// filled in, learning its dialect while it is already talking to the endpoint.
#[tauri::command]
pub async fn test_connection(
    app: AppHandle,
    provider_id: String,
) -> Result<ConnectionReport, Failure> {
    let provider = require_provider(&app, &provider_id)?;
    let (http, language) = {
        let state = app.state::<AppState>();
        (state.http.clone(), state.config_snapshot().language)
    };

    let key = require_api_key(&provider, i18n::test_needs_key(language), language)?;

    // The probe needs a model like any other request, and a row ships none
    // (`docs/register-audit-2026-08-25.md`) — so on a fresh row this button is
    // reachable *before* one is chosen, and it sits above the model field on the
    // pane. The predicate and the kind live in `commands::require_model`, so
    // this refusal and `exchange/turn.rs`'s cannot come apart; only the sentence
    // is this caller's, and it names Refresh models rather than Settings because
    // the reader is already in Settings.
    super::require_model(&provider.model, i18n::test_needs_model(language))?;

    client::test_connection(&http, &provider.base_url, key.as_deref(), &provider.model)
        .await
        .map_err(Failure::from)?;

    // Only after the plain request succeeded: until then a `400` cannot be told
    // apart from the endpoint disliking something else about the body.
    Ok(ConnectionReport {
        reasoning: detect::reasoning(&http, &provider, key.as_deref()).await,
    })
}

/// Open the page one provider's keys come from.
///
/// The URL comes from the row's `key_page`, not from the webview: "open whatever
/// is asked for" is a far larger surface than this gap is worth. It must be
/// `https`, so a hand-edited config cannot turn this into a `file:` opener.
#[tauri::command]
pub fn open_key_page(app: AppHandle, provider_id: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let provider = require_provider(&app, &provider_id).map_err(|failure| failure.message)?;
    let url = provider
        .key_page
        .filter(|url| url.starts_with("https://"))
        .ok_or_else(|| format!("{} has no key page recorded", provider.label))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
