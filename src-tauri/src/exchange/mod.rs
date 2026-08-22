//! Exchanges: in-memory only, one per Popover (ADR-0004).
//!
//! No storage layer exists anywhere in this module on purpose. Follow-up turns
//! resend the full history untruncated — a single Exchange is short-lived, so
//! the growth has a natural ceiling.
//!
//! This file is the bookkeeping: what an Exchange holds and how a turn is
//! started against it. [`events`] is the wire to the Popover, [`turn`] is the
//! task that runs one turn and drives that wire.

pub mod events;
mod turn;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

use crate::action::ModelParams;
use crate::llm::{Content, Message};

pub use self::turn::spawn_turn;

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
    ///
    /// `content` rather than `&str` because a turn may carry a Capture
    /// (ADR-0016); the image travels with the history like any other message.
    pub fn begin_turn(&self, id: &str, content: impl Into<Content>) -> Option<TurnPlan> {
        let mut map = self.inner.lock().expect("exchange lock");
        let entry = map.get_mut(id)?;
        entry.exchange.messages.push(Message::user(content.into()));
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

    /// The last thing the user sent, Capture included. A retry resends exactly
    /// that — the turn that failed is the one worth repeating, and repeating it
    /// without its image would answer a different question.
    pub fn last_user_message(&self, id: &str) -> Option<Content> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ModelParams {
        ModelParams {
            model: "deepseek-v4-flash".into(),
            thinking: false,
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
        assert_eq!(second.messages[1].content, Content::from("hello"));
        assert_eq!(second.messages[2].content, Content::from("你好"));
        assert_eq!(second.messages[3].content, Content::from("again, politely"));
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

    /// A Capture rides in the history like any other content, so a follow-up
    /// still has the image the first answer was about (ADR-0004: untruncated).
    #[test]
    fn a_capture_stays_in_the_history_for_the_next_turn() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(
            &id,
            Content::with_images("what is this?", ["data:image/png;base64,AA"]),
        );
        manager.commit_assistant(&id, "a dialog");

        let second = manager.begin_turn(&id, "and the button?").unwrap();
        assert_eq!(
            second.messages[1].content,
            Content::with_images("what is this?", ["data:image/png;base64,AA"])
        );
        // The retry path resends this one verbatim, so it is the follow-up and
        // carries no image of its own.
        assert_eq!(
            manager.last_user_message(&id),
            Some(Content::from("and the button?"))
        );
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
        assert_eq!(manager.last_user_message(&id), Some(Content::from("two")));
    }
}
