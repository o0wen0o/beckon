//! Exchanges: in-memory only, one per Popover (ADR-0004).
//!
//! No storage layer exists anywhere in this file on purpose. Follow-up turns
//! resend the full history untruncated — a single Exchange is short-lived, so
//! the growth has a natural ceiling.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use crate::action::ModelParams;
use crate::llm::client::{self, LlmError, StreamEvent};
use crate::llm::{deepseek, Message};
use crate::state::AppState;
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

#[derive(Debug)]
pub struct Exchange {
    pub params: ModelParams,
    /// `messages[0]` is the system prompt; the rest is the turn history.
    pub messages: Vec<Message>,
}

#[derive(Debug)]
struct Entry {
    exchange: Exchange,
    cancel: CancellationToken,
}

/// One turn's worth of everything the runner needs, taken under the lock so
/// nothing is held across an await.
pub struct TurnPlan {
    pub exchange_id: String,
    pub params: ModelParams,
    pub messages: Vec<Message>,
    pub cancel: CancellationToken,
}

#[derive(Debug, Default)]
pub struct ExchangeManager {
    next_id: AtomicU64,
    inner: Mutex<HashMap<String, Entry>>,
}

impl ExchangeManager {
    /// Open an Exchange. The caller then starts its first turn.
    pub fn create(&self, system_prompt: &str, params: ModelParams) -> String {
        let id = format!("ex{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let exchange = Exchange {
            params,
            messages: vec![Message::system(system_prompt)],
        };
        self.inner.lock().expect("exchange lock").insert(
            id.clone(),
            Entry {
                exchange,
                cancel: CancellationToken::new(),
            },
        );
        id
    }

    /// Append a user turn and hand back what the runner needs. A fresh
    /// cancellation token is installed: a cancelled token stays cancelled, so
    /// the previous turn's token cannot be reused.
    pub fn begin_turn(&self, id: &str, user_text: &str) -> Option<TurnPlan> {
        let mut map = self.inner.lock().expect("exchange lock");
        let entry = map.get_mut(id)?;
        entry.exchange.messages.push(Message::user(user_text));
        entry.cancel = CancellationToken::new();
        Some(TurnPlan {
            exchange_id: id.to_string(),
            params: entry.exchange.params.clone(),
            messages: entry.exchange.messages.clone(),
            cancel: entry.cancel.clone(),
        })
    }

    /// Record what the assistant produced. Partial text from an interrupted
    /// turn is recorded too — it is what the user can see, so a follow-up must
    /// be consistent with it.
    pub fn commit_assistant(&self, id: &str, text: &str) {
        if text.is_empty() {
            return;
        }
        let mut map = self.inner.lock().expect("exchange lock");
        if let Some(entry) = map.get_mut(id) {
            entry.exchange.messages.push(Message::assistant(text));
        }
    }

    pub fn cancel(&self, id: &str) {
        let map = self.inner.lock().expect("exchange lock");
        if let Some(entry) = map.get(id) {
            entry.cancel.cancel();
        }
    }

    /// Cancel and forget everything. Hiding the Popover and a fresh trigger
    /// both end up here — the Exchange dies with the window (ADR-0004).
    pub fn discard_all(&self) {
        let mut map = self.inner.lock().expect("exchange lock");
        for entry in map.values() {
            entry.cancel.cancel();
        }
        map.clear();
    }

    /// The last thing the user sent. A retry resends exactly that — the turn
    /// that failed is the one worth repeating.
    pub fn last_user_message(&self, id: &str) -> Option<String> {
        let map = self.inner.lock().expect("exchange lock");
        map.get(id)?
            .exchange
            .messages
            .iter()
            .rev()
            .find(|message| message.role == crate::llm::Role::User)
            .map(|message| message.content.clone())
    }
}

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

/// Run one turn to completion, emitting the Popover's state machine as events.
///
/// Spawned rather than awaited by the caller: a trigger must return
/// immediately so the window can be shown while the request is in flight.
pub fn spawn_turn(app: AppHandle, plan: TurnPlan) {
    tauri::async_runtime::spawn(async move {
        run_turn(app, plan).await;
    });
}

async fn run_turn(app: AppHandle, plan: TurnPlan) {
    let state = app.state::<AppState>();

    let base_url = {
        let config = state.config.read().expect("config lock");
        config.api.base_url.clone()
    };

    let api_key = match crate::secrets::read() {
        Ok(Some(key)) => key,
        Ok(None) => {
            emit_error(
                &app,
                &plan.exchange_id,
                "no-credential",
                "No API key is stored. Open Settings to add one.",
            );
            return;
        }
        Err(message) => {
            emit_error(
                &app,
                &plan.exchange_id,
                "read-error",
                &format!("The API key could not be read from the Credential Manager: {message}"),
            );
            return;
        }
    };

    let body = match deepseek::build_body(&plan.params, &plan.messages) {
        Ok(body) => body,
        Err(message) => {
            emit_error(&app, &plan.exchange_id, "config", &message);
            return;
        }
    };

    let mut sink = DeltaSink::new(app.clone(), plan.exchange_id.clone());
    let result = client::stream_chat(
        &state.http,
        &base_url,
        &api_key,
        &body,
        &plan.cancel,
        |event| sink.push(event),
    )
    .await;

    sink.flush();
    state
        .exchanges
        .commit_assistant(&plan.exchange_id, &sink.answer);

    match result {
        Ok(()) => {
            let _ = app.emit_to(
                TARGET_WINDOW,
                EVENT_DONE,
                ExchangeIdPayload {
                    exchange_id: plan.exchange_id.clone(),
                },
            );
        }
        // Esc, hide, or a replacing trigger. The UI already knows; no error.
        Err(LlmError::Cancelled) => {}
        Err(LlmError::Interrupted(message)) => {
            let _ = app.emit_to(
                TARGET_WINDOW,
                EVENT_INTERRUPTED,
                InterruptedPayload {
                    exchange_id: plan.exchange_id.clone(),
                    message,
                },
            );
        }
        Err(err) => {
            let kind = err.kind().to_string();
            emit_error(&app, &plan.exchange_id, &kind, &err.to_string());
        }
    }
}

fn emit_error(app: &AppHandle, exchange_id: &str, kind: &str, message: &str) {
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

/// Coalesces deltas onto [`DELTA_TICK`] and fires `first-token` exactly once.
struct DeltaSink {
    app: AppHandle,
    exchange_id: String,
    content: String,
    reasoning: String,
    /// The complete answer, for the Exchange history.
    answer: String,
    last_flush: Instant,
    announced_first: bool,
}

impl DeltaSink {
    fn new(app: AppHandle, exchange_id: String) -> Self {
        Self {
            app,
            exchange_id,
            content: String::new(),
            reasoning: String::new(),
            answer: String::new(),
            last_flush: Instant::now(),
            announced_first: false,
        }
    }

    fn push(&mut self, event: StreamEvent) {
        // Thinking text counts as a first token: it is proof the request is
        // alive, which is the distinction the README asks the UI to make.
        if !self.announced_first {
            self.announced_first = true;
            let _ = self.app.emit_to(
                TARGET_WINDOW,
                EVENT_FIRST_TOKEN,
                ExchangeIdPayload {
                    exchange_id: self.exchange_id.clone(),
                },
            );
        }

        match event {
            StreamEvent::Content(text) => {
                self.answer.push_str(&text);
                self.content.push_str(&text);
            }
            StreamEvent::Reasoning(text) => self.reasoning.push_str(&text),
        }

        if self.last_flush.elapsed() >= DELTA_TICK {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.content.is_empty() && self.reasoning.is_empty() {
            return;
        }
        let _ = self.app.emit_to(
            TARGET_WINDOW,
            EVENT_DELTA,
            DeltaPayload {
                exchange_id: self.exchange_id.clone(),
                content: std::mem::take(&mut self.content),
                reasoning: std::mem::take(&mut self.reasoning),
            },
        );
        self.last_flush = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ModelParams {
        ModelParams {
            model: "deepseek-v4-flash".into(),
            thinking: false,
            temperature: 1.3,
        }
    }

    #[test]
    fn a_turn_carries_the_full_history_untruncated() {
        let manager = ExchangeManager::default();
        let id = manager.create("you are a translator", params());

        let first = manager.begin_turn(&id, "hello").unwrap();
        assert_eq!(first.messages.len(), 2);
        manager.commit_assistant(&id, "你好");

        let second = manager.begin_turn(&id, "again, politely").unwrap();
        assert_eq!(second.messages.len(), 4);
        assert_eq!(second.messages[1].content, "hello");
        assert_eq!(second.messages[2].content, "你好");
        assert_eq!(second.messages[3].content, "again, politely");
    }

    #[test]
    fn each_turn_gets_a_fresh_cancellation_token() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());

        let first = manager.begin_turn(&id, "one").unwrap();
        manager.cancel(&id);
        assert!(first.cancel.is_cancelled());

        let second = manager.begin_turn(&id, "two").unwrap();
        assert!(!second.cancel.is_cancelled());
    }

    #[test]
    fn discarding_cancels_and_forgets() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        let plan = manager.begin_turn(&id, "one").unwrap();

        manager.discard_all();
        assert!(plan.cancel.is_cancelled());
        assert!(manager.last_user_message(&id).is_none());
        assert!(manager.begin_turn(&id, "two").is_none());
    }

    #[test]
    fn empty_assistant_text_is_not_recorded() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(&id, "one").unwrap();
        manager.commit_assistant(&id, "");
        // Still just system + "one": the next turn is only the third message.
        let next = manager.begin_turn(&id, "two").unwrap();
        assert_eq!(next.messages.len(), 3);
    }

    #[test]
    fn a_retry_resends_the_last_user_message() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(&id, "one").unwrap();
        manager.commit_assistant(&id, "answer");
        manager.begin_turn(&id, "two").unwrap();
        assert_eq!(manager.last_user_message(&id).as_deref(), Some("two"));
    }
}
