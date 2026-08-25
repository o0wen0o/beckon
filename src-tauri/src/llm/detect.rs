//! Asking an endpoint which thinking dialect it speaks.
//!
//! Endpoint knowledge, so it lives beside the request layer that owns the
//! dialects rather than in the command that happens to trigger it: the
//! sequencing rules below are facts about wire behaviour, and a second caller —
//! re-detecting a whole table, say — must get the same answer as the button
//! does. Nothing here knows about windows.

use crate::config::{Provider, Reasoning};

use super::{client, request};

/// Which thinking dialect an endpoint speaks, asked rather than typed.
///
/// A wrong [`Reasoning`] is a `400` on every turn, and it is the one field on a
/// row that a person has no way to look up — so a hand-made row used to ship a
/// control whose right answer the user was expected to know. The endpoint knows
/// it, and answering costs one small request per candidate.
///
/// Two guards keep this from inventing an answer:
///
///  - **A permissive endpoint is detected first.** One probe carries a field
///    nobody could recognise; an endpoint that takes it takes anything, so no
///    later answer would mean what it appears to, and the five requests are
///    skipped. Local servers and lenient proxies land here.
///  - **Ambiguity is not a result.** If more than one dialect is accepted, the
///    answer is `None`. Two arms overlap on purpose — MiniMax's probe is
///    DeepSeek's plus a field — so a MiniMax-compatible host is expected to come
///    back ambiguous, and reporting `None` costs the thinking switch where a
///    coin-flip would cost every turn.
///
/// This is a floor and not an oracle: `None` leaves the row exactly as the user
/// set it, and `config.toml` is still where a person who knows better says so.
pub async fn reasoning(
    http: &reqwest::Client,
    provider: &Provider,
    key: Option<&str>,
) -> Option<Reasoning> {
    // A preset already carries the answer, read off the vendor's own docs. Not
    // worth six requests to reconfirm, and detection is strictly the weaker
    // source — it could only talk us out of something true.
    if crate::config::is_preset(&provider.id) {
        return None;
    }

    let url = &provider.base_url;
    let permissive = request::build_permissiveness_probe(&provider.model);
    match client::accepts_body(http, url, key, &permissive).await {
        // Takes a field that does not exist, so it would take all five.
        Ok(true) => return None,
        Ok(false) => {}
        // A rejected key or a dead network: the caller already reported it.
        Err(_) => return None,
    }

    let mut accepted = None;
    for &candidate in request::DETECTABLE {
        let Some(body) = request::build_dialect_probe(candidate, &provider.model) else {
            continue;
        };
        match client::accepts_body(http, url, key, &body).await {
            Ok(true) if accepted.is_some() => return None, // Ambiguous.
            Ok(true) => accepted = Some(candidate),
            Ok(false) => {}
            Err(_) => return None,
        }
    }
    accepted
}
