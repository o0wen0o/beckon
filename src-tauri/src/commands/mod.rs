//! The IPC surface. Thin on purpose: every command validates, delegates, and
//! lets the reload path broadcast the result.
//!
//! One file per thing being commanded — the config, Actions, the credential,
//! the model catalog, the windows. The re-exports below keep the surface flat,
//! so `generate_handler!` in `main.rs` and the wrappers in `src/lib/ipc.ts`
//! still name `commands::get_config` and never learn which file it lives in.

mod actions;
mod config;
mod models;
mod secrets;
mod windows;

pub use self::actions::*;
pub use self::config::*;
pub use self::models::*;
pub use self::secrets::*;
pub use self::windows::*;

use serde::Serialize;

use crate::llm::LlmError;

/// An error the UI has to react to differently depending on cause, rather than
/// just print.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub kind: String,
    pub message: String,
}

impl Failure {
    pub(super) fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
        }
    }
}

impl From<LlmError> for Failure {
    fn from(error: LlmError) -> Self {
        Failure::new(error.kind(), error.to_string())
    }
}
