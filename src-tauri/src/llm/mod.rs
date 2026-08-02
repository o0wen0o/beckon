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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}
