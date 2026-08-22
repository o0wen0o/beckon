//! Running one turn: read the credential, build the body, stream it, and turn
//! the result into the Popover's events.

use std::time::Instant;

use tauri::{AppHandle, Manager};

use crate::llm::{client, request, LlmError, StreamEvent};
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

    // The row is read here rather than carried in the plan (ADR-0021): a
    // `base_url` corrected in Settings while a Popover is open has to reach the
    // next follow-up, and the Exchange only ever held the id.
    let (provider, language) = {
        let config = state.config.read().expect("config lock");
        (
            config.provider(Some(&plan.params.provider)).cloned(),
            config.language,
        )
    };
    let Some(provider) = provider else {
        emit_error(
            &app,
            &plan.exchange_id,
            "config",
            &crate::i18n::provider_missing(language, &plan.params.provider),
        );
        return;
    };

    // The credential split lives in `commands::require_api_key` once (ADR-0005,
    // ADR-0021): four outcomes, one of them "a local row wants no header". A
    // `Failure` is `{kind, message}`, which is exactly what `emit_error` takes,
    // so the Popover and Settings cannot come to disagree about the same row.
    let api_key = match crate::commands::require_api_key(
        &provider,
        &crate::i18n::turn_needs_key(language, &provider.label),
        language,
    ) {
        Ok(key) => key,
        Err(failure) => {
            emit_error(&app, &plan.exchange_id, &failure.kind, &failure.message);
            return;
        }
    };

    let body = match request::build_body(&provider, &plan.params, &plan.messages) {
        Ok(body) => body,
        Err(message) => {
            emit_error(&app, &plan.exchange_id, "config", &message);
            return;
        }
    };

    let mut sink = DeltaSink::new(app.clone(), plan.exchange_id.clone());
    let result = client::stream_chat(
        &state.http,
        &provider.base_url,
        api_key.as_deref(),
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
