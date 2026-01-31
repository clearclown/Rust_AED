//! xAI プロバイダー
//!
//! xAI Grok Vision API を使用したテキスト抽出プロバイダー

mod client;
mod config;

pub use client::XAIProvider;
pub use config::XAIConfig;

// xAI は OpenAI 互換 API を使用するため、メッセージ型は openai モジュールを再利用
