//! The HTTP side: one streaming call, one connection probe, one model list.
//!
//! Only the requests live here. The error type is in [`super::error`] and every
//! response shape in [`super::wire`], so nothing in this file has to be tested
//! against a network to be trusted.
//!
//! **No timeout, deliberately** (README): a dead network must surface as an
//! immediate error in the Popover rather than a spinner that never resolves,
//! and a long thinking pause must not be mistaken for a hang.

use futures_util::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::error::{status_error, LlmError};
use super::sse::SseParser;
use super::wire::{handle_event, parse_model_list, Flow, StreamEvent};

/// `{base_url}/v1/{path}`, tolerating a `base_url` that already carries a
/// version segment. Every endpoint goes through here so the tolerance is one
/// rule and not one per route.
///
/// The rule is **any** path segment that looks like a version, not the last one:
/// this used to test `ends_with("/v1")`, which was right for the endpoints the
/// provider table grew up on and wrong for every vendor who versions their path
/// differently. `open.bigmodel.cn/api/paas/v4` became `…/v4/v1/chat/completions`
/// and Google's `…/v1beta/openai/` became `…/openai/v1/models` — the first a
/// shipped row, and no test here reaches the network, so both read to a user as
/// their own key or their own network.
///
/// "Any" rather than "the last", because a version can come first: Cloudflare's
/// AI Gateway compat root is `gateway.ai.cloudflare.com/v1/{account}/{gateway}/openai`,
/// which has to be taken literally. And not "any path at all is already
/// complete", which would score better on the table above at the cost of every
/// hand-typed `proxy.example.com/openai` that works today *because* `/v1` is
/// appended.
///
/// Two limits, stated rather than hidden. A path segment like `/v2ray/` is a
/// knowing false positive — cheaper than the alternative, and a user who has one
/// can spell the whole path. And a lexical test cannot know whether a versioned
/// path *is* the compatibility root: `…/api/paas/v4` is, `…/client/v4/accounts/x/ai`
/// would not be. The durable answer is for `base_url` to mean the compat root
/// outright, which is a migration rather than a fix to this function.
fn api_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if has_version_segment(base) {
        format!("{base}/{path}")
    } else {
        format!("{base}/v1/{path}")
    }
}

/// Whether any path segment of `base` is a version: `v` followed by a digit.
///
/// The authority is skipped, so a host like `v2.example.com` is not read as a
/// version — the scheme separator is what tells them apart, and a base with no
/// scheme is treated as authority-first the way [`crate::config`]'s `host_of`
/// does.
fn has_version_segment(base: &str) -> bool {
    base.split_once("://")
        .map_or(base, |(_, rest)| rest)
        .split('/')
        // The authority, which is never a version segment.
        .skip(1)
        // Deliberately the same shape as the `/^v\d/i` its mirror in
        // `src/lib/providers.ts` tests with, so the two read as one rule.
        .any(|segment| matches!(segment.as_bytes(), [b'v' | b'V', digit, ..] if digit.is_ascii_digit()))
}

/// `POST {base_url}/v1/chat/completions`.
pub fn chat_url(base_url: &str) -> String {
    api_url(base_url, "chat/completions")
}

/// `GET {base_url}/v1/models`, the OpenAI-compatible list endpoint.
pub fn models_url(base_url: &str) -> String {
    api_url(base_url, "models")
}

/// Attach the credential, or deliberately none.
///
/// `None` is a real answer, not a missing one: a local endpoint wants no
/// `Authorization` header at all, and sending `Bearer ` with nothing after it is
/// a 401 from anything that reads the header (ADR-0021). Every request goes
/// through here so "no key means no header" is one rule.
fn signed(request: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    match api_key {
        Some(key) => request.bearer_auth(key),
        None => request,
    }
}

pub fn build_http_client() -> reqwest::Client {
    // No `.timeout(..)` on purpose — see the module docs.
    reqwest::Client::builder()
        .user_agent(concat!("beckon/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP client")
}

/// Stream one completion. `on_event` is called on the calling task, so it may
/// emit Tauri events directly.
pub async fn stream_chat(
    http: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    body: &Value,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(StreamEvent),
) -> Result<(), LlmError> {
    let request = signed(http.post(chat_url(base_url)), api_key)
        .header("accept", "text/event-stream")
        .json(body);

    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(LlmError::Cancelled),
        result = request.send() => result.map_err(|e| LlmError::Network(e.to_string()))?,
    };

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|e| format!("<no response body: {e}>"));
        return Err(status_error(status.as_u16(), &body));
    }

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();
    let mut received_any = false;
    let mut saw_done = false;

    'stream: loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(LlmError::Cancelled),
            next = stream.next() => next,
        };

        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|e| {
            // A drop after partial output is an interruption, not a plain
            // network failure: the partial text is worth keeping.
            if received_any {
                LlmError::Interrupted(e.to_string())
            } else {
                LlmError::Network(e.to_string())
            }
        })?;

        for event in parser.push(&chunk) {
            match handle_event(event, &mut on_event)? {
                Flow::Continue => received_any = true,
                Flow::Done => {
                    saw_done = true;
                    break 'stream;
                }
            }
        }
    }

    if !saw_done {
        for event in parser.finish() {
            if let Flow::Done = handle_event(event, &mut on_event)? {
                saw_done = true;
            }
        }
    }

    // The server closed without `[DONE]`: partial output, mark interrupted.
    if !saw_done && received_any {
        return Err(LlmError::Interrupted(
            "the connection closed before the response finished".to_string(),
        ));
    }

    Ok(())
}

/// "Test connection" (Phase 2 / ADR-0005): the smallest request that proves the
/// key and `base_url` work, reporting auth failure separately from a network
/// failure.
pub async fn test_connection(
    http: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
) -> Result<(), LlmError> {
    let response = signed(http.post(chat_url(base_url)), api_key)
        .json(&super::request::build_probe_body(model))
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    Err(status_error(
        status.as_u16(),
        &response.text().await.unwrap_or_default(),
    ))
}

/// The ids the endpoint says it serves.
///
/// Every failure comes back as an ordinary [`LlmError`], so a rejected key
/// stays distinguishable from an unreachable API (ADR-0005) even though the
/// caller's response to both is the same: fall back to the documented list.
pub async fn list_models(
    http: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<String>, LlmError> {
    let response = signed(http.get(models_url(base_url)), api_key)
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(status_error(
            status.as_u16(),
            &response.text().await.unwrap_or_default(),
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;
    parse_model_list(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerates_a_base_url_that_already_carries_the_version() {
        for (base, expected) in [
            ("https://api.deepseek.com", "https://api.deepseek.com/v1/x"),
            ("https://api.deepseek.com/", "https://api.deepseek.com/v1/x"),
            ("http://localhost:11434/v1", "http://localhost:11434/v1/x"),
            (
                "  https://example.com/api/  ",
                "https://example.com/api/v1/x",
            ),
        ] {
            assert_eq!(api_url(base, "x"), expected, "base {base}");
        }
    }

    /// The shapes the `ends_with("/v1")` rule got wrong. Both are rows in
    /// `presets()`, and one of them shipped — so this test is the record that
    /// the two URLs a user could not have debugged are now the documented ones.
    #[test]
    fn a_version_segment_anywhere_in_the_path_means_the_base_is_complete() {
        for (base, expected) in [
            // Zhipu: versioned last, but not `v1`.
            (
                "https://open.bigmodel.cn/api/paas/v4",
                "https://open.bigmodel.cn/api/paas/v4/x",
            ),
            // Google's compatibility layer: versioned, then a suffix.
            (
                "https://generativelanguage.googleapis.com/v1beta/openai/",
                "https://generativelanguage.googleapis.com/v1beta/openai/x",
            ),
            // Cloudflare's AI Gateway: versioned *first*, which is why the rule
            // reads every segment rather than the last one.
            (
                "https://gateway.ai.cloudflare.com/v1/acct/gw/openai",
                "https://gateway.ai.cloudflare.com/v1/acct/gw/openai/x",
            ),
            // Anthropic's trailing slash, trimmed before the test.
            (
                "https://api.anthropic.com/v1/",
                "https://api.anthropic.com/v1/x",
            ),
        ] {
            assert_eq!(api_url(base, "x"), expected, "base {base}");
        }
    }

    /// The authority is not a path segment, so a versioned *host* still gets the
    /// version appended — and the one false positive the rule knowingly accepts
    /// is a path segment that merely starts like a version.
    #[test]
    fn the_version_test_reads_the_path_and_not_the_host() {
        assert_eq!(
            api_url("https://v2.example.com", "x"),
            "https://v2.example.com/v1/x"
        );
        // Knowingly wrong, and cheaper than the alternative: a user with this
        // path can spell the whole thing.
        assert_eq!(
            api_url("https://example.com/v2ray", "x"),
            "https://example.com/v2ray/x"
        );
    }

    #[test]
    fn names_the_two_endpoints() {
        assert_eq!(
            chat_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            models_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/models"
        );
    }
}
