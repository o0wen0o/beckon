//! The OpenAI-compatible LLM layer.
//!
//! `sse` is a pure frame parser, `wire` the response shapes and the pure
//! functions over them, `error` the one error type, `deepseek` the one place
//! provider quirks live, `models` the catalog both `deepseek` and the Settings
//! dropdown read, and `client` does the request. Nothing here knows about
//! windows or Actions.

pub mod client;
pub mod deepseek;
mod error;
pub mod models;
pub mod sse;
mod wire;

pub use self::error::LlmError;
pub use self::wire::StreamEvent;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
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
    /// Text plus one Capture. The text part is dropped when empty rather than
    /// sent blank — an Action whose template renders to nothing is asking about
    /// the image alone.
    pub fn with_image(text: impl Into<String>, data_url: impl Into<String>) -> Self {
        let text = text.into();
        let mut parts = Vec::with_capacity(2);
        if !text.is_empty() {
            parts.push(Part::Text { text });
        }
        parts.push(Part::ImageUrl {
            image_url: ImageUrl {
                url: data_url.into(),
            },
        });
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
        let message = Message::user(Content::with_image(
            "what is this?",
            "data:image/png;base64,AA",
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

    /// An Action whose template renders to nothing is asking about the image
    /// alone, so no empty text part is sent beside it.
    #[test]
    fn an_empty_prompt_sends_the_image_alone() {
        let content = Content::with_image("", "data:image/png;base64,AA");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AA" } }
            ])
        );
    }

    /// Text-only content stays a bare string on the wire, which is what an
    /// endpoint that has never heard of content parts accepts.
    #[test]
    fn plain_text_serialises_as_a_string() {
        let json = serde_json::to_value(Content::from("plain")).unwrap();
        assert_eq!(json, serde_json::json!("plain"));
    }
}
