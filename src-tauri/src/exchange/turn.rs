//! Running one turn: read the credential, build the body, stream it, and turn
//! the result into the Popover's events.

use std::time::Instant;

use tauri::{AppHandle, Manager};

use crate::llm::{client, deepseek, LlmError, StreamEvent};
use crate::state::AppState;

use super::events::{
    emit_delta, emit_done, emit_error, emit_first_token, emit_interrupted, DELTA_TICK,
};
use super::TurnPlan;

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
        Ok(()) => emit_done(&app, &plan.exchange_id),
        // Esc, hide, or a replacing trigger. The UI already knows; no error.
        Err(LlmError::Cancelled) => {}
        Err(LlmError::Interrupted(message)) => {
            emit_interrupted(&app, &plan.exchange_id, message);
        }
        Err(err) => {
            let kind = err.kind().to_string();
            emit_error(&app, &plan.exchange_id, &kind, &err.to_string());
        }
    }
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
            emit_first_token(&self.app, &self.exchange_id);
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
        emit_delta(
            &self.app,
            &self.exchange_id,
            std::mem::take(&mut self.content),
            std::mem::take(&mut self.reasoning),
        );
        self.last_flush = Instant::now();
    }
}
