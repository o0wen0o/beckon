//! One error type for the whole LLM layer, and the one place an HTTP status
//! becomes one.

use super::wire::ApiErrorFrame;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    /// The key was rejected. Distinct from every "cannot read the key" case in
    /// [`crate::secrets`] (ADR-0005).
    #[error("the API key was rejected ({status}): {message}")]
    Auth { status: u16, message: String },
    #[error("the API returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("could not reach the API: {0}")]
    Network(String),
    /// The stream died after partial output. The Popover keeps that text and
    /// marks it interrupted (README).
    #[error("the stream was interrupted: {0}")]
    Interrupted(String),
    #[error("cancelled")]
    Cancelled,
}

impl LlmError {
    /// Stable discriminant for the frontend state machine.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Auth { .. } => "auth",
            Self::Http { .. } => "http",
            Self::Network(_) => "network",
            Self::Interrupted(_) => "interrupted",
            Self::Cancelled => "cancelled",
        }
    }
}

/// The one place a status code becomes an [`LlmError`]. ADR-0005 needs "key
/// rejected" to stay distinguishable from every other failure, so the 401/403
/// boundary is decided here rather than at each call site.
pub(super) fn status_error(status: u16, body: &str) -> LlmError {
    let message = trim_error_body(body);
    if status == 401 || status == 403 {
        LlmError::Auth { status, message }
    } else {
        LlmError::Http { status, message }
    }
}

/// Error bodies are sometimes an HTML page. Prefer the JSON `error.message`,
/// and cap whatever is left so the Popover stays readable.
fn trim_error_body(body: &str) -> String {
    if let Ok(frame) = serde_json::from_str::<ApiErrorFrame>(body) {
        if let Some(error) = frame.error {
            return error.message;
        }
    }
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 300 {
        flat.chars().take(300).collect::<String>() + "…"
    } else {
        flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_bodies_prefer_the_json_message() {
        assert_eq!(
            trim_error_body(r#"{"error":{"message":"Invalid API key"}}"#),
            "Invalid API key"
        );
        let html = format!("<html>{}</html>", "x".repeat(500));
        assert!(trim_error_body(&html).chars().count() <= 301);
    }

    #[test]
    fn only_401_and_403_are_an_auth_failure() {
        assert!(matches!(status_error(401, ""), LlmError::Auth { .. }));
        assert!(matches!(status_error(403, ""), LlmError::Auth { .. }));
        assert!(matches!(status_error(429, ""), LlmError::Http { .. }));
    }

    #[test]
    fn error_kinds_are_stable() {
        assert_eq!(
            LlmError::Auth {
                status: 401,
                message: String::new()
            }
            .kind(),
            "auth"
        );
        assert_eq!(LlmError::Network("x".into()).kind(), "network");
        assert_eq!(LlmError::Interrupted("x".into()).kind(), "interrupted");
    }
}
