//! OpenAI プロバイダー
//!
//! OpenAI GPT-4o Vision API を使用したテキスト抽出プロバイダー

mod client;
mod config;
mod messages;

pub use client::OpenAIProvider;
pub use config::OpenAIConfig;
pub use messages::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ContentPart, ImageUrl,
};
