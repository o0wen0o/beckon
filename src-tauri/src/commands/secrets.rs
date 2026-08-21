//! Credential commands, plus the one place "no credential", "read error" and
//! "key rejected" are turned into three distinguishable [`Failure`]s (ADR-0005).

use tauri::{AppHandle, Manager};

use crate::config::Language;
use crate::i18n;
use crate::llm::client;
use crate::secrets::{self, KeyStatus};
use crate::state::AppState;

use super::Failure;

#[tauri::command]
pub fn get_key_status() -> KeyStatus {
    secrets::status()
}

#[tauri::command]
pub fn set_api_key(key: String) -> Result<KeyStatus, String> {
    secrets::write(&key)?;
    Ok(secrets::status())
}

#[tauri::command]
pub fn delete_api_key() -> Result<KeyStatus, String> {
    secrets::delete()?;
    Ok(secrets::status())
}

/// The stored key, or why there is none. ADR-0005 needs "no credential" and
/// "read error" to stay two different things all the way to the UI, so the
/// split lives here once; callers supply only the sentence that varies.
pub(super) fn require_api_key(when_missing: &str, language: Language) -> Result<String, Failure> {
    match secrets::read() {
        Ok(Some(key)) => Ok(key),
        Ok(None) => Err(Failure::new("no-credential", when_missing)),
        Err(message) => Err(Failure::new(
            "read-error",
            i18n::credential_unreadable(language, &message),
        )),
    }
}

/// "Test connection": one minimal request, reporting a rejected key separately
/// from an unreachable API (ADR-0005).
#[tauri::command]
pub async fn test_connection(app: AppHandle) -> Result<(), Failure> {
    let (base_url, model, http, language) = {
        let state = app.state::<AppState>();
        let config = state.config_snapshot();
        (
            config.api.base_url,
            config.defaults.model,
            state.http.clone(),
            config.language,
        )
    };

    let key = require_api_key(i18n::test_needs_key(language), language)?;

    client::test_connection(&http, &base_url, &key, &model)
        .await
        .map_err(Failure::from)
}
