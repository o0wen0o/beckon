//! One error type for the whole LLM layer, and the one place an HTTP status
//! becomes one.

use super::wire::ApiErrorFrame;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    /// The key was rejected. Distinct from every "cannot read the key" case in
    /// [`crate::secrets`].
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

/// An error and everything under it, as one sentence.
///
/// reqwest's own `Display` is the outermost layer only — "error sending request
/// for url (…)" — and the part a person can act on is in `source()`: the name
/// that did not resolve, the connection that was refused, the certificate that
/// did not verify. Quoting the top alone made a wrong host, an Ollama that is
/// not running and a corporate TLS proxy read as the same failure, which they
/// are not, and each has a different fix.
///
/// A layer that only repeats what the one above it already said is skipped:
/// hyper and reqwest both like to restate their cause.
pub(super) fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        if !cause_text.is_empty() && !text.contains(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

/// The one place a status code becomes an [`LlmError`]. "Key rejected" has to
/// stay distinguishable from every other failure, so the 401/403
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

    /// The cause is the part a person can act on, and it is under the top.
    #[test]
    fn a_chain_reads_down_to_the_cause_and_says_nothing_twice() {
        #[derive(Debug, thiserror::Error)]
        #[error("error sending request")]
        struct Outer(#[source] Middle);
        #[derive(Debug, thiserror::Error)]
        #[error("client error (Connect)")]
        struct Middle(#[source] std::io::Error);

        let refused = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        assert_eq!(
            chain(&Outer(Middle(refused))),
            "error sending request: client error (Connect): refused"
        );

        // A layer that only restates its cause is not repeated.
        #[derive(Debug, thiserror::Error)]
        #[error("tls: {0}")]
        struct Restates(#[source] std::io::Error);
        let inner = std::io::Error::other("bad certificate");
        assert_eq!(chain(&Restates(inner)), "tls: bad certificate");
    }

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
