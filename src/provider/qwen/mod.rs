//! Qwen プロバイダー
//!
//! Alibaba Qwen (DashScope) Vision API を使用したテキスト抽出プロバイダー

mod client;
mod config;

pub use client::QwenProvider;
pub use config::QwenConfig;
