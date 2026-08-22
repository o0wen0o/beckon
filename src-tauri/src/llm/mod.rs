//! The OpenAI-compatible LLM layer.
//!
//! `sse` is a pure frame parser, `wire` the response shapes and the pure
//! functions over them, `error` the one error type, `request` the one place a
//! divergence between endpoints lives, `models` the catalog both `request` and
//! the Settings dropdown read, and `client` does the request. Nothing here knows
//! about windows.

pub mod client;
mod error;
pub mod models;
pub mod request;
pub mod sse;
mod wire;

pub use self::error::LlmError;
pub use self::wire::StreamEvent;

use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// One image on the wire, as a `data:` URL.
///
/// The provider also accepts an external URL and a Files API handle; a Capture
/// exists only in memory (ADR-0004, ADR-0016), so this is the only form Beckon
/// can send.
///
/// `Arc<str>` rather than `String`, and it is the only field in this module that
/// is: the history is resent untruncated on every follow-up (ADR-0004), so the
/// per-turn clone of it would otherwise `memcpy` every attached image again —
/// megabytes per turn, growing with the conversation. It serialises as the same
/// bare string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: Arc<str>,
}

/// One element of a multimodal message, in the documented tagged shape:
/// `{"type":"text","text":…}` / `{"type":"image_url","image_url":{"url":…}}`
/// (<https://api-docs.deepseek.com/guides/vision/>).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

/// What a message carries.
///
/// `untagged`, so a text-only message still serialises as a bare string: that
/// is the shape every OpenAI-compatible endpoint accepts, and a `base_url` may
/// point at one that has never heard of content parts (ADR-0016). Parts are
/// sent only when there is actually an image to send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Parts(Vec<Part>),
}

impl Content {
    /// Text plus the Captures attached to the turn, in the order they were taken
    /// (ADR-0017) — the provider reads a parts array in order, so "the first
    /// one" in the note means the first tile in the composer.
    ///
    /// The text part is dropped when empty rather than sent blank — an Action
    /// whose template renders to nothing is asking about the images alone. No
    /// images at all falls back to a bare string, because that is the shape an
    /// endpoint that has never heard of parts accepts.
    pub fn with_images(
        text: impl Into<String>,
        data_urls: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> Self {
        let text = text.into();
        let images: Vec<Part> = data_urls
            .into_iter()
            .map(|url| Part::ImageUrl {
                image_url: ImageUrl { url: url.into() },
            })
            .collect();
        if images.is_empty() {
            return Content::Text(text);
        }
        let mut parts = Vec::with_capacity(images.len() + 1);
        if !text.is_empty() {
            parts.push(Part::Text { text });
        }
        parts.extend(images);
        Content::Parts(parts)
    }
}

impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Content::Text(text.to_string())
    }
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Content::Text(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Content::Text(content.into()),
        }
    }
    pub fn user(content: impl Into<Content>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Content::Text(content.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text message must keep serialising as a bare string: the parts array
    /// is for images only, and an endpoint that predates it still has to work.
    #[test]
    fn text_content_serialises_as_a_bare_string() {
        let json = serde_json::to_value(Message::user("hi")).unwrap();
        assert_eq!(json["content"], serde_json::json!("hi"));
    }

    #[test]
    fn an_image_serialises_in_the_documented_parts_shape() {
        let message = Message::user(Content::with_images(
            "what is this?",
            ["data:image/png;base64,AA"],
        ));
        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(
            json["content"],
            serde_json::json!([
                { "type": "text", "text": "what is this?" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AA" } }
            ])
        );
    }

    /// Several Captures are several parts after the one text part, in the order
    /// they were taken (ADR-0017): a note saying "the second one" is only true
    /// if the wire keeps the composer's order.
    #[test]
    fn several_images_follow_the_text_in_order() {
        let content = Content::with_images("which is wrong?", ["data:one", "data:two"]);
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                { "type": "text", "text": "which is wrong?" },
                { "type": "image_url", "image_url": { "url": "data:one" } },
                { "type": "image_url", "image_url": { "url": "data:two" } }
            ])
        );
    }

    /// Nothing attached is not an empty parts array: it is the bare string an
    /// endpoint that predates content parts accepts (ADR-0016).
    #[test]
    fn no_images_stays_a_bare_string() {
        let content = Content::with_images("just words", Vec::<Arc<str>>::new());
        assert_eq!(content, Content::Text("just words".to_string()));
    }

    /// An Action whose template renders to nothing is asking about the image
    /// alone, so no empty text part is sent beside it.
    #[test]
    fn an_empty_prompt_sends_the_image_alone() {
        let content = Content::with_images("", ["data:image/png;base64,AA"]);
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AA" } }
            ])
        );
    }
}
