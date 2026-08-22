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
