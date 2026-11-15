//! The HTTP side: one streaming call, one connection probe, one model list.
//!
//! Only the requests live here. The error type is in [`super::error`] and every
//! response shape in [`super::wire`], so nothing in this file has to be tested
//! against a network to be trusted.
//!
//! **No timeout on a streaming turn, deliberately** (README): a dead network
//! must surface as an immediate error in the Popover rather than a spinner that
//! never resolves, and a long thinking pause must not be mistaken for a hang.
//!
//! Two bounds that are not that timeout, and do not contradict it. *Connecting*
//! is bounded ([`CONNECT_TIMEOUT`]): a host that never completes a handshake is
//! not thinking, and without a bound it is exactly the spinner that never
//! resolves. And the three unary calls Settings makes — Test connection, the
//! model list, dialect detection — are bounded whole ([`UNARY_TIMEOUT`]): each
//! is one small exchange with a button waiting on it, and none of them streams.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::error::{chain, status_error, LlmError};
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
/// a 401 from anything that reads the header. Every request goes
/// through here so "no key means no header" is one rule.
///
/// **Both credential schemes go on every keyed request**, which is what lets the
/// provider table stay free of an `auth` field. Anthropic is
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

/// Follow within one origin, refuse to leave it.
///
/// `previous` is every URL already visited, last one first-from; `next` is where
/// the 3xx points. See [`build_http_client`] for why leaving is refused rather
/// than followed with the credential stripped.
///
/// The *origin*, not the host. A `3xx` from `https://api.example.com` to
/// `http://api.example.com` names the same host and puts the key on the wire in
/// plaintext; one to another port names the same host and reaches a different
/// service. The one crossing still followed is the upgrade — `http` to `https`
/// on the scheme's own port — because that is a server moving a request to the
/// safer of its own two doors.
fn redirect_verdict(previous: &[reqwest::Url], next: &reqwest::Url) -> Redirect {
    // No previous hop is not a redirect at all; treated as same-origin so the
    // arm below is the only one that can stop a request.
    let same_host = previous.last().is_none_or(|from| {
        let same_origin = from.scheme() == next.scheme()
            && from.port_or_known_default() == next.port_or_known_default();
        let upgrade = from.scheme() == "http" && next.scheme() == "https" && next.port().is_none();
        from.host_str() == next.host_str() && (same_origin || upgrade)
    });
    if !same_host {
        Redirect::Stop
    } else if previous.len() > MAX_REDIRECTS {
        Redirect::TooMany
    } else {
        Redirect::Follow
    }
}

pub fn build_http_client() -> reqwest::Client {
    // reqwest is built with `rustls-no-provider` (see `Cargo.toml`), under which
    // building a `Client` with no process-wide crypto provider is a panic rather
    // than an error. The updater plugin installs ring too, but lazily, on its
    // first check — long after `AppState` has built this client. An `Err` here
    // means a provider is already installed, which is the state being asked for.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // No `.timeout(..)` on purpose — see the module docs. The connect bound
    // is the one that is safe to put on every request, a streaming turn
    // included: it ends at the handshake, before there is a model to wait for.
    reqwest::Client::builder()
        .user_agent(concat!("beckon/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        // **A redirect to another host is refused, not followed.** reqwest strips
        // `Authorization` across hosts by itself, but its sensitive-header list
        // is a fixed set and `x-api-key` — which every keyed request carries
        // since Anthropic's native routes needed it — is not in it. Following
        // would hand the user's key in plaintext to whatever host the 3xx names,
        // which is a party the user never chose and the relay disclosure could
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

/// How long a connection may take to be established: DNS, TCP and TLS. Ten
/// seconds is several times a slow handshake over a bad link and still short
/// enough to read as an answer.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long one of Settings' unary calls may take from send to last byte. Wide
/// on purpose — Test connection against a local endpoint can be the request
/// that loads the model into memory — and it bounds a button, never a turn.
const UNARY_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the stream may go quiet before a consumer is told so
/// ([`StreamEvent::Idle`]). One frame: the same interval the Popover's deltas
/// are coalesced on, so a held-back tail goes out no later than the next tick
/// would have sent it.
const IDLE_AFTER: Duration = Duration::from_millis(16);

/// How much of a body that never parsed is kept to quote. An error message is a
/// sentence; `status_error` trims what it is handed to 300 characters anyway.
const UNPARSED_CAP: usize = 2048;

/// A successful status over a body with no events in it, as the failure it is.
///
/// Through `status_error`, so a JSON `error.message` is quoted in the vendor's
/// own words exactly as it would be under a `4xx`. An empty body has no words
/// to quote, and "HTTP 200:" followed by nothing would read as a rendering bug.
fn not_a_stream(status: u16, body: &[u8]) -> LlmError {
    let body = String::from_utf8_lossy(body);
    if body.trim().is_empty() {
        return status_error(
            status,
            "the endpoint closed the response without sending anything",
        );
    }
    status_error(status, &body)
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
        result = request.send() => result.map_err(|e| LlmError::Network(chain(&e)))?,
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
    // Whether anything reached the caller — answer or thinking. Not the same
    // as `received_any`, which counts frames: a role-only frame, a usage frame
    // and a frame that would not parse are all frames and none of them is
    // something to read.
    //
    // Atomics rather than `bool`s, though one task owns them: the wrapper
    // below writes both while the loop reads them, and it is held across every
    // `await` here — a `Cell` would make this future `!Send`, and it is spawned.
    let delivered = AtomicBool::new(false);
    // Text handed over since the caller was last told the stream went quiet.
    let undrained = AtomicBool::new(false);
    // The model's own word for how it ended, once it has said.
    let mut finish: Option<String> = None;
    let mut on_event = |event: StreamEvent| {
        delivered.store(true, Ordering::Relaxed);
        undrained.store(true, Ordering::Relaxed);
        on_event(event);
    };
    // The start of the body, kept only until the first event parses. A `200`
    // that is not a stream has to be quoted from somewhere, and by the time
    // that is known the bytes have been through the parser and dropped.
    let mut unparsed: Vec<u8> = Vec::new();

    'stream: loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(LlmError::Cancelled),
            next = stream.next() => next,
            // Armed only while the caller is holding text. A consumer that
            // batches flushes when the *next* token arrives, so the tail of a
            // burst — several frames in one network read, from a relay that
            // buffers or an endpoint that is simply fast — sat undrawn until
            // the following read, and the last thinking before a long pause
            // was not shown until the pause ended.
            _ = tokio::time::sleep(IDLE_AFTER), if undrained.load(Ordering::Relaxed) => {
                on_event(StreamEvent::Idle);
                undrained.store(false, Ordering::Relaxed);
                continue;
            }
        };

        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|e| {
            // A drop after partial output is an interruption, not a plain
            // network failure: the partial text is worth keeping.
            if received_any {
                LlmError::Interrupted(chain(&e))
            } else {
                LlmError::Network(chain(&e))
            }
        })?;

        if !received_any && unparsed.len() < UNPARSED_CAP {
            let room = UNPARSED_CAP - unparsed.len();
            unparsed.extend_from_slice(&chunk[..chunk.len().min(room)]);
        }

        for event in parser.push(&chunk) {
            match handle_event(event, &mut on_event)
                .map_err(|e| unanswered(e, delivered.load(Ordering::Relaxed)))?
            {
                Flow::Continue => received_any = true,
                Flow::Finished(reason) => {
                    received_any = true;
                    finish = Some(reason);
                }
                Flow::Done => {
                    saw_done = true;
                    break 'stream;
                }
            }
        }
    }

    if !saw_done {
        for event in parser.finish() {
            match handle_event(event, &mut on_event)
                .map_err(|e| unanswered(e, delivered.load(Ordering::Relaxed)))?
            {
                Flow::Continue => received_any = true,
                Flow::Finished(reason) => {
                    received_any = true;
                    finish = Some(reason);
                }
                Flow::Done => saw_done = true,
            }
        }
    }

    // Success on the status line and not one event in the body: a gateway that
    // answers `200` with a JSON error, an endpoint that ignored `stream: true`
    // and sent a whole completion, or a connection closed after the headers.
    // Falling through to `Ok` drew that as a finished, empty answer — the one
    // outcome that says nothing went wrong and shows nothing.
    if !saw_done && !received_any {
        return Err(not_a_stream(status.as_u16(), &unparsed));
    }

    // The server closed without `[DONE]` and without the model saying it was
    // finished: partial output, mark interrupted. With a `finish_reason` the
    // close *is* the ending — `[DONE]` is a convention some endpoints skip, and
    // a complete answer labelled "interrupted" is a false alarm on every turn.
    if !saw_done && finish.is_none() {
        return Err(LlmError::Interrupted(
            "the connection closed before the response finished".to_string(),
        ));
    }

    ending(
        finish.as_deref(),
        delivered.load(Ordering::Relaxed),
        status.as_u16(),
    )
}

/// What a stream that ended in an orderly way came to.
///
/// `Ok` only for an answer that is both complete and present. The two endings
/// that are not `stop` leave text on screen that is not the whole answer, and
/// say so the way a dropped connection does — the partial text is kept and
/// marked. And an ending with nothing delivered at all is a failure, not a
/// finished turn: it used to be drawn as a blank answer with a Copy button
/// under it, the one outcome that says nothing went wrong and shows nothing.
fn ending(finish: Option<&str>, delivered: bool, status: u16) -> Result<(), LlmError> {
    match finish {
        Some("length") => Err(LlmError::Interrupted(
            "the answer was cut off at the endpoint's output limit".to_string(),
        )),
        Some("content_filter") => Err(LlmError::Interrupted(
            "the endpoint's content filter stopped the answer".to_string(),
        )),
        _ if !delivered => Err(LlmError::Http {
            status,
            message: match finish {
                Some(reason) => {
                    format!("the endpoint finished without sending an answer ({reason})")
                }
                None => "the endpoint finished without sending an answer".to_string(),
            },
        }),
        _ => Ok(()),
    }
}

/// An error frame that arrived before anything else did is not an
/// interruption: there is no partial answer to keep, and `Interrupted` draws a
/// settled turn with a note under it and none of a failure's affordances. It is
/// the request being refused on a `200`, so it is reported as one — in the
/// vendor's words, with Retry beside it.
fn unanswered(error: LlmError, delivered: bool) -> LlmError {
    match error {
        LlmError::Interrupted(message) if !delivered => LlmError::Http {
            status: 200,
            message,
        },
        other => other,
    }
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
        .timeout(UNARY_TIMEOUT)
        .send()
        .await
        .map_err(|e| LlmError::Network(chain(&e)))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| LlmError::Network(chain(&e)))?;
    Ok((status, body))
}

/// "Test connection": the smallest request that proves the
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
/// stays distinguishable from an unreachable API even though the
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

    /// The same host is not the same recipient. A downgrade puts the key on
    /// the wire in plaintext, and another port is another service; neither
    /// `Authorization` stripping nor the host comparison caught `x-api-key`
    /// going to either.
    #[test]
    fn a_redirect_to_another_scheme_or_port_of_the_host_is_refused() {
        let from = [url("https://api.example.com/v1/models")];
        assert_eq!(
            redirect_verdict(&from, &url("http://api.example.com/v1/models")),
            Redirect::Stop
        );
        assert_eq!(
            redirect_verdict(&from, &url("https://api.example.com:8443/v1/models")),
            Redirect::Stop
        );
        // The explicit default port is the same origin.
        assert_eq!(
            redirect_verdict(&from, &url("https://api.example.com:443/v2/models")),
            Redirect::Follow
        );
    }

    /// The one crossing that is still followed: a server sending plain HTTP on
    /// to its own HTTPS door.
    #[test]
    fn an_upgrade_to_https_on_the_same_host_is_followed() {
        assert_eq!(
            redirect_verdict(
                &[url("http://api.example.com/v1/models")],
                &url("https://api.example.com/v1/models"),
            ),
            Redirect::Follow
        );
        assert_eq!(
            redirect_verdict(
                &[url("http://api.example.com/v1/models")],
                &url("https://api.example.com:8443/v1/models"),
            ),
            Redirect::Stop
        );
    }

    /// An orderly ending is `Ok` only when there is an answer and it is whole.
    #[test]
    fn an_ending_is_a_success_only_when_the_answer_is_whole_and_present() {
        assert_eq!(ending(Some("stop"), true, 200), Ok(()));
        // No `finish_reason` at all, but `[DONE]` and text: an endpoint that
        // does not send one is still an endpoint that answered.
        assert_eq!(ending(None, true, 200), Ok(()));
        assert!(matches!(
            ending(Some("length"), true, 200),
            Err(LlmError::Interrupted(_))
        ));
        assert!(matches!(
            ending(Some("content_filter"), false, 200),
            Err(LlmError::Interrupted(_))
        ));
        // Finished, and nothing to show for it.
        assert!(matches!(
            ending(Some("stop"), false, 200),
            Err(LlmError::Http { status: 200, .. })
        ));
        assert!(matches!(
            ending(None, false, 200),
            Err(LlmError::Http { .. })
        ));
    }

    /// A refusal on a `200` is a failure with Retry, not a settled empty turn.
    #[test]
    fn an_error_frame_before_any_text_is_a_failure_not_an_interruption() {
        let refused = LlmError::Interrupted("rate limited".to_string());
        assert_eq!(
            unanswered(refused.clone(), false),
            LlmError::Http {
                status: 200,
                message: "rate limited".to_string()
            }
        );
        // After text has arrived it is what it says: the text is kept.
        assert_eq!(unanswered(refused.clone(), true), refused);
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

    /// The other half of the same rule: no key means no credential
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

    /// A gateway that answers `200` with a JSON error body. The vendor's own
    /// sentence is what reaches the Popover, the way it would under a `4xx`.
    #[test]
    fn a_success_status_over_a_json_error_quotes_the_error() {
        let err = not_a_stream(200, br#"{"error":{"message":"quota exceeded"}}"#);
        assert_eq!(
            err,
            LlmError::Http {
                status: 200,
                message: "quota exceeded".to_string(),
            }
        );
    }

    /// Nothing to quote is said in words, not left as "HTTP 200:" and a blank.
    #[test]
    fn a_success_status_over_an_empty_body_still_says_something() {
        for body in [&b""[..], b"  \r\n"] {
            let LlmError::Http { status, message } = not_a_stream(200, body) else {
                panic!("an empty 200 is an HTTP failure");
            };
            assert_eq!(status, 200);
            assert!(!message.trim().is_empty());
        }
    }
}
