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

/// The version Anthropic's native routes require. A constant rather than a row
/// field: it names *their* API's revision, which is not something a user of this
/// program knows or should be asked.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Attach the credential, or deliberately none.
///
/// `None` is a real answer, not a missing one: a local endpoint wants no
/// `Authorization` header at all, and sending `Bearer ` with nothing after it is
/// a 401 from anything that reads the header (ADR-0021). Every request goes
/// through here so "no key means no header" is one rule.
///
/// **Both credential schemes go on every keyed request**, which is what lets the
/// provider table stay free of the `auth` field ADR-0021 refuses. Anthropic is
/// the one vendor whose compatibility layer is not the whole story: its native
/// `GET /v1/models` reads an `Authorization: Bearer` as an *OAuth* token and
/// rejects an API key outright, while `x-api-key` authenticates on both that
/// route and `chat/completions`. Where both headers are present Anthropic
/// validates `x-api-key` — probed 2026-08-25, against the live host.
///
/// This is not the "unknown field is a 400, not a courtesy" case the request
/// layer is built around, and the difference is header versus body: a recipient
/// is required to ignore a header field it does not recognise, and a JSON body
/// field it does not recognise is what it rejects. Probed the same day against
/// DeepSeek, OpenAI and MiniMax: the response is byte-identical with the extra
/// headers and without them.
///
/// The cost, stated: the key appears twice in whatever request log the vendor
/// keeps. Same request, same connection, same recipient — no new party reads it.
fn signed(request: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    match api_key {
        Some(key) => request
            .bearer_auth(key)
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION),
        None => request,
    }
}

/// The redirect limit reqwest's own default policy uses, restated because a
/// custom policy replaces that default rather than extending it.
const MAX_REDIRECTS: usize = 10;

/// What to do with one redirect. Named so the rule below can be tested without
/// a network or a `reqwest::redirect::Attempt`, which only reqwest can build.
#[derive(Debug, PartialEq)]
enum Redirect {
    Follow,
    Stop,
    TooMany,
}

/// Follow within one host, refuse to leave it.
///
/// `previous` is every URL already visited, last one first-from; `next` is where
/// the 3xx points. See [`build_http_client`] for why leaving the host is refused
/// rather than followed with the credential stripped.
fn redirect_verdict(previous: &[reqwest::Url], next: &reqwest::Url) -> Redirect {
    // No previous hop is not a redirect at all; treated as same-host so the
    // arm below is the only one that can stop a request.
    let same_host = previous
        .last()
        .map_or(true, |from| from.host_str() == next.host_str());
    if !same_host {
        Redirect::Stop
    } else if previous.len() > MAX_REDIRECTS {
        Redirect::TooMany
    } else {
        Redirect::Follow
    }
}

pub fn build_http_client() -> reqwest::Client {
    // No `.timeout(..)` on purpose — see the module docs.
    reqwest::Client::builder()
        .user_agent(concat!("beckon/", env!("CARGO_PKG_VERSION")))
        // **A redirect to another host is refused, not followed.** reqwest strips
        // `Authorization` across hosts by itself, but its sensitive-header list
        // is a fixed set and `x-api-key` — which every keyed request carries
        // since Anthropic's native routes needed it — is not in it. Following
        // would hand the user's key in plaintext to whatever host the 3xx names,
        // which is a party the user never chose and a disclosure ADR-0025 could
        // not have shown them. Stopping surfaces the 3xx as an ordinary status
        // error instead.
        //
        // Same-host redirects still follow: no new recipient, so no new reader
        // of the key, and a vendor moving `/v1` under its own host stays a
        // working row.
        .redirect(reqwest::redirect::Policy::custom(
            |attempt| match redirect_verdict(attempt.previous(), attempt.url()) {
                Redirect::Follow => attempt.follow(),
                Redirect::Stop => attempt.stop(),
                Redirect::TooMany => attempt.error("too many redirects"),
            },
        ))
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

/// Send one prepared non-streaming request and read it to the end.
///
/// The one place the three unary routes below turn a transport failure into
/// [`LlmError::Network`], so "unreachable" is worded once rather than three
/// times.
///
/// The body is read even where the status alone answers the question, because a
/// connection is returned to the pool only once its body has reached EOF, and
/// dialect detection fires up to six of these at the same host in a row — an
/// undrained response costs each of them a fresh TLS handshake. The bodies are
/// one-token completions and error envelopes, so reading them is cheap.
async fn send_read(
    request: reqwest::RequestBuilder,
) -> Result<(reqwest::StatusCode, String), LlmError> {
    let response = request
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;
    Ok((status, body))
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
    let (status, body) = send_read(
        signed(http.post(chat_url(base_url)), api_key)
            .json(&super::request::build_probe_body(model)),
    )
    .await?;

    if status.is_success() {
        return Ok(());
    }
    Err(status_error(status.as_u16(), &body))
}

/// Whether this endpoint accepts one candidate body.
///
/// `Ok(true)` for a 2xx and `Ok(false)` for a `400` — which is the whole
/// question, since a rejected field is exactly what a strict endpoint answers
/// with. Anything else is an error rather than a `false`: a `401` or a dead
/// network says nothing about which dialect is spoken, and reporting it as "not
/// this one" would let a bad key be read as a fact about the wire.
pub(super) async fn accepts_body(
    http: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    body: &Value,
) -> Result<bool, LlmError> {
    let (status, text) =
        send_read(signed(http.post(chat_url(base_url)), api_key).json(body)).await?;

    if status.is_success() {
        return Ok(true);
    }
    if status == reqwest::StatusCode::BAD_REQUEST {
        return Ok(false);
    }
    Err(status_error(status.as_u16(), &text))
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
    let (status, body) = send_read(signed(http.get(models_url(base_url)), api_key)).await?;

    if !status.is_success() {
        return Err(status_error(status.as_u16(), &body));
    }
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

    fn url(text: &str) -> reqwest::Url {
        text.parse().expect("url")
    }

    /// The reason the credential can ride on every keyed request: it never
    /// reaches a host the user did not name. reqwest strips `Authorization`
    /// across hosts on its own and does not know `x-api-key`, so this is the
    /// only thing standing between a hostile 3xx and the key in plaintext.
    #[test]
    fn a_redirect_off_the_host_is_refused() {
        assert_eq!(
            redirect_verdict(
                &[url("https://api.example.com/v1/models")],
                &url("https://collector.attacker.example/v1/models"),
            ),
            Redirect::Stop
        );
    }

    /// Within one host there is no new recipient, so a vendor moving its own
    /// path stays a working row.
    #[test]
    fn a_redirect_within_the_host_is_followed() {
        assert_eq!(
            redirect_verdict(
                &[url("https://api.example.com/v1/models")],
                &url("https://api.example.com/v2/models"),
            ),
            Redirect::Follow
        );
        assert_eq!(
            redirect_verdict(&[], &url("https://api.example.com/v1/models")),
            Redirect::Follow
        );
    }

    /// A custom policy replaces reqwest's default, limit included — so a
    /// same-host redirect loop has to be stopped here or it never is.
    #[test]
    fn a_same_host_loop_still_hits_a_limit() {
        let visited: Vec<_> = (0..=MAX_REDIRECTS)
            .map(|hop| url(&format!("https://api.example.com/{hop}")))
            .collect();
        assert_eq!(
            redirect_verdict(&visited, &url("https://api.example.com/again")),
            Redirect::TooMany
        );
    }

    /// Both schemes, so no provider row has to carry an `auth` field and no
    /// user has to answer a question about a header. Anthropic validates
    /// `x-api-key` where both are present; everyone else ignores it.
    #[test]
    fn a_keyed_request_carries_both_credential_schemes() {
        let request = signed(
            build_http_client().get("https://example.com"),
            Some("sk-test"),
        )
        .build()
        .expect("request");
        let headers = request.headers();
        assert_eq!(headers["authorization"], "Bearer sk-test");
        assert_eq!(headers["x-api-key"], "sk-test");
        assert_eq!(headers["anthropic-version"], ANTHROPIC_VERSION);
    }

    /// The other half of the same rule (ADR-0021): no key means no credential
    /// at all, because a local endpoint wants an ordinary unauthenticated
    /// request and `Bearer ` with nothing after it is a 401.
    #[test]
    fn an_unkeyed_request_carries_no_credential_header_at_all() {
        let request = signed(build_http_client().get("https://example.com"), None)
            .build()
            .expect("request");
        let headers = request.headers();
        assert!(headers.get("authorization").is_none());
        assert!(headers.get("x-api-key").is_none());
        // Not even the version: nothing about this request is Anthropic's.
        assert!(headers.get("anthropic-version").is_none());
    }
}
