//! The kind-plus-message pair a failure reaches a window as.
//!
//! At the crate root rather than inside `commands/`, because two different
//! layers produce one: a command that refused, and a Capture that cannot be
//! sent (ADR-0016). Both are read by the same `describeFailure` in the
//! frontend, so they have to be the same shape — and a second struct shaped
//! *like* this one is how the two drift.
//!
//! The kind is a contract string the frontend catalogs key on; the message is
//! either Beckon's own sentence in the reader's language or a cause quoted
//! verbatim from something that does not speak it (ADR-0015).

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
    pub(crate) fn new(kind: &str, message: impl Into<String>) -> Self {
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
