//! The IPC surface. Thin on purpose: every command validates, delegates, and
//! lets the reload path broadcast the result.
//!
//! One file per thing being commanded — the config, Actions, the credential,
//! the model catalog, the platform's permissions, the windows. The re-exports
//! below keep the surface flat, so `generate_handler!` in `main.rs` and the
//! wrappers in `src/lib/ipc.ts` still name `commands::get_config` and never
//! learn which file it lives in.

mod actions;
mod config;
mod models;
mod platform;
mod secrets;
mod windows;

pub use self::actions::*;
pub use self::config::*;
pub use self::models::*;
pub use self::platform::*;
pub use self::secrets::*;
pub use self::windows::*;

/// Re-exported so a command still returns `commands::Failure`: it lives at the
/// crate root because `platform::capture` produces one too (ADR-0016), and one
/// shape is what makes `describeFailure` a single reader.
pub use crate::failure::Failure;

/// Refuse a request that has no model to name on the wire.
///
/// Here rather than in either caller, and for the reason `require_api_key` is
/// one function: two of them spelled the emptiness test and the `"no-model"`
/// kind twice, and that kind is a contract with `src/lib/i18n`'s `errors` table.
/// Callers supply only the sentence that varies — "open Settings" for a turn,
/// "press Refresh models" for the pane that is already open.
///
/// A row carries no model of its own, so an empty one is the ordinary state of a
/// row nobody has picked in yet, not a corruption. Refused in Beckon's own words
/// because the alternative is the vendor's `400` for `"model": ""`, which points
/// at nothing — and on the Connection pane reads as a rejected key.
pub(crate) fn require_model(model: &str, when_missing: &str) -> Result<(), Failure> {
    if model.trim().is_empty() {
        return Err(Failure::new("no-model", when_missing));
    }
    Ok(())
}
