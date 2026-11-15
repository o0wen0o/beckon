//! Exchanges: in-memory only, one per Popover.
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
    /// `content` rather than `&str` because a turn may carry a Capture;
    /// the image travels with the history like any other message.
    ///
    /// A turn that follows one nobody answered — it failed, or was stopped
    /// before the first token, and the user typed on instead of pressing Retry —
    /// joins that message rather than following it. `commit_assistant` records
    /// nothing for an empty answer, so pushing would send `[…, user, user]`:
    /// the same `400` from every strict chat template that `retry_turn` exists
    /// to avoid, reached by the other button.
    pub fn begin_turn(&self, id: &str, content: impl Into<Content>) -> Option<TurnPlan> {
        let mut map = self.inner.lock().expect("exchange lock");
        let entry = map.get_mut(id)?;
        let content = content.into();
        match entry.exchange.messages.last_mut() {
            Some(last) if last.role == crate::llm::Role::User => last.content.append(content),
            _ => entry.exchange.messages.push(Message::user(content)),
        }
        Some(Self::plan(id, entry))
    }

    /// Run the last user turn again, verbatim — the same words and the same
    /// Captures — because the turn that failed is the one worth repeating, and
    /// repeating it without its image would answer a different question.
    ///
    /// Nothing is appended: the message is already in the history. Appending it
    /// again sent `[…, user X, user X]`, once more per Retry — the tokens and
    /// the Captures twice, and a `400` from every chat template that insists on
    /// alternating roles, which made Retry the one button that could not work
    /// against a local model.
    ///
    /// Whatever the failed attempt left *after* that message is dropped. A
    /// stream that died with an error may have committed partial text, and the
    /// Popover clears that answer when Retry is pressed; the history has to
    /// match what is on screen, for the reason `commit_assistant` records
    /// partial text in the first place.
    pub fn retry_turn(&self, id: &str) -> Option<TurnPlan> {
        let mut map = self.inner.lock().expect("exchange lock");
        let entry = map.get_mut(id)?;
        let last_user = entry
            .exchange
            .messages
            .iter()
            .rposition(|message| message.role == crate::llm::Role::User)?;
        entry.exchange.messages.truncate(last_user + 1);
        Some(Self::plan(id, entry))
    }

    /// What a runner needs for the history as it now stands. The fresh
    /// cancellation token is installed here, once, for both ways a turn starts.
    fn plan(id: &str, entry: &mut Entry) -> TurnPlan {
        entry.cancel = CancellationToken::new();
        TurnPlan {
            exchange_id: id.to_string(),
            params: entry.exchange.params.clone(),
            messages: entry.exchange.messages.clone(),
            cancel: entry.cancel.clone(),
        }
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
    /// both end up here — the Exchange dies with the window.
    pub fn discard_all(&self) {
        let mut map = self.inner.lock().expect("exchange lock");
        for entry in map.values() {
            entry.cancel.cancel();
        }
        map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ModelParams {
        ModelParams {
            provider: "deepseek".into(),
            model: "deepseek-v4-flash".into(),
            thinking: false,
            web_search: false,
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
    /// still has the image the first answer was about (untruncated).
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
        // The follow-up carries no image of its own: the Capture belongs to the
        // turn it was attached to.
        assert_eq!(second.messages[3].content, Content::from("and the button?"));
    }

    #[test]
    fn discarding_cancels_and_forgets() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        let plan = manager.begin_turn(&id, "one").unwrap();

        manager.discard_all();
        assert!(plan.cancel.is_cancelled());
        assert!(manager.retry_turn(&id).is_none());
        assert!(manager.begin_turn(&id, "two").is_none());
    }

    #[test]
    fn empty_assistant_text_is_not_recorded() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(&id, "one").unwrap();
        manager.commit_assistant(&id, "");
        // Still just system + "one" — no assistant message was pushed for the
        // empty answer, so the next turn joins the unanswered one rather than
        // following it (`a_follow_up_after_an_unanswered_turn_joins_it`).
        let next = manager.begin_turn(&id, "two").unwrap();
        assert_eq!(next.messages.len(), 2);
        assert!(next
            .messages
            .iter()
            .all(|message| message.role != crate::llm::Role::Assistant));
    }

    /// A turn that failed before any text came back is still the last message.
    /// Retrying must send the history as it stands — pushing the same message
    /// again is what made every retry one copy longer than the last.
    #[test]
    fn a_retry_resends_the_turn_rather_than_appending_it() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(&id, "one").unwrap();
        manager.commit_assistant(&id, "answer");
        let failed = manager.begin_turn(&id, "two").unwrap();

        let first = manager.retry_turn(&id).unwrap();
        let second = manager.retry_turn(&id).unwrap();
        assert_eq!(first.messages, failed.messages);
        assert_eq!(second.messages, failed.messages);
        assert_eq!(second.messages[3].content, Content::from("two"));
    }

    /// The Capture rides along, because it is part of the message being resent.
    #[test]
    fn a_retry_keeps_the_captures_of_the_turn_it_repeats() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(
            &id,
            Content::with_images("what is this?", ["data:image/png;base64,AA"]),
        );

        let retried = manager.retry_turn(&id).unwrap();
        assert_eq!(retried.messages.len(), 2);
        assert_eq!(
            retried.messages[1].content,
            Content::with_images("what is this?", ["data:image/png;base64,AA"])
        );
    }

    /// An error after partial output commits that text, and the Popover clears
    /// it when Retry is pressed — so the history drops it too.
    #[test]
    fn a_retry_drops_the_partial_answer_it_replaces() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(&id, "one").unwrap();
        manager.commit_assistant(&id, "half an ans");

        let retried = manager.retry_turn(&id).unwrap();
        assert_eq!(retried.messages.len(), 2);
        assert_eq!(retried.messages[1].content, Content::from("one"));
    }

    /// Roles alternate whatever the user does after a turn that got no
    /// answer: a follow-up typed instead of Retry joins the unanswered message.
    #[test]
    fn a_follow_up_after_an_unanswered_turn_joins_it() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager.begin_turn(&id, "one").unwrap();
        // Nothing came back: `commit_assistant` records no empty answer.
        manager.commit_assistant(&id, "");

        let plan = manager.begin_turn(&id, "two").unwrap();
        let roles: Vec<_> = plan.messages.iter().map(|m| m.role.clone()).collect();
        assert_eq!(
            roles,
            vec![crate::llm::Role::System, crate::llm::Role::User]
        );
        assert_eq!(plan.messages[1].content, Content::Text("one\n\ntwo".into()));

        // An answered turn is followed, not joined.
        manager.commit_assistant(&id, "answer");
        let plan = manager.begin_turn(&id, "three").unwrap();
        assert_eq!(plan.messages.len(), 4);
    }

    /// The Captures of both halves survive the join, in the order they were sent.
    #[test]
    fn joining_keeps_the_captures_of_both_messages() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        manager
            .begin_turn(
                &id,
                Content::with_images("look", ["data:image/png;base64,AA"]),
            )
            .unwrap();

        let plan = manager.begin_turn(&id, "and this").unwrap();
        let Content::Parts(parts) = &plan.messages[1].content else {
            panic!("an image keeps the message in parts");
        };
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[2], crate::llm::Part::Text { text } if text == "and this"));
    }

    #[test]
    fn a_retry_gets_a_fresh_cancellation_token() {
        let manager = ExchangeManager::default();
        let id = manager.create("s", params());
        let first = manager.begin_turn(&id, "one").unwrap();
        manager.cancel(&id);
        assert!(first.cancel.is_cancelled());

        assert!(!manager.retry_turn(&id).unwrap().cancel.is_cancelled());
    }
}
