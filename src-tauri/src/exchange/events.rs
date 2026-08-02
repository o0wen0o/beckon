//! The Popover's state machine, as it goes over the wire.
//!
//! The window is driven by events, not return values: `first-token` fires once,
//! `delta` is coalesced onto [`DELTA_TICK`], and then exactly one of `done` /
//! `error` / `interrupted` — or silence on cancel, which the UI already knows
//! about. Names and payloads are mirrored in `src/lib/ipc.ts`.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

// Where the streamed events go: only the Popover renders an Exchange, and the
// label is owned by the layer that creates windows.
use crate::trigger::WINDOW_POPOVER as TARGET_WINDOW;

/// Deltas are coalesced onto this tick. Per-token IPC floods the WebView and
/// makes the Popover feel *slower* than the network.
pub const DELTA_TICK: Duration = Duration::from_millis(16);

pub const EVENT_FIRST_TOKEN: &str = "exchange:first-token";
pub const EVENT_DELTA: &str = "exchange:delta";
pub const EVENT_DONE: &str = "exchange:done";
pub const EVENT_ERROR: &str = "exchange:error";
pub const EVENT_INTERRUPTED: &str = "exchange:interrupted";

#[derive(Debug, Clone, Serialize)]
pub struct ExchangeIdPayload {
    pub exchange_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeltaPayload {
    pub exchange_id: String,
    /// Answer text since the last tick.
    pub content: String,
    /// Thinking text since the last tick.
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub exchange_id: String,
    /// Stable discriminant: `auth`, `http`, `network`, `no-credential`, …
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InterruptedPayload {
    pub exchange_id: String,
    pub message: String,
}

/// The request is alive. Fired once per turn, by the first delta of any kind.
pub fn emit_first_token(app: &AppHandle, exchange_id: &str) {
    emit_id(app, EVENT_FIRST_TOKEN, exchange_id);
}

pub fn emit_delta(app: &AppHandle, exchange_id: &str, content: String, reasoning: String) {
    let _ = app.emit_to(
        TARGET_WINDOW,
        EVENT_DELTA,
        DeltaPayload {
            exchange_id: exchange_id.to_string(),
            content,
            reasoning,
        },
    );
}

pub fn emit_done(app: &AppHandle, exchange_id: &str) {
    emit_id(app, EVENT_DONE, exchange_id);
}

pub fn emit_error(app: &AppHandle, exchange_id: &str, kind: &str, message: &str) {
    let _ = app.emit_to(
        TARGET_WINDOW,
        EVENT_ERROR,
        ErrorPayload {
            exchange_id: exchange_id.to_string(),
            kind: kind.to_string(),
            message: message.to_string(),
        },
    );
}

/// Partial output, then the stream died. The Popover keeps the text (README).
pub fn emit_interrupted(app: &AppHandle, exchange_id: &str, message: String) {
    let _ = app.emit_to(
        TARGET_WINDOW,
        EVENT_INTERRUPTED,
        InterruptedPayload {
            exchange_id: exchange_id.to_string(),
            message,
        },
    );
}

fn emit_id(app: &AppHandle, event: &str, exchange_id: &str) {
    let _ = app.emit_to(
        TARGET_WINDOW,
        event,
        ExchangeIdPayload {
            exchange_id: exchange_id.to_string(),
        },
    );
}
