//! DeepSeek プロバイダー
//!
//! DeepSeek API を使用したテキスト抽出プロバイダー
//! 注意: DeepSeek は現在 Vision をサポートしていません

mod client;
mod config;

pub use client::DeepSeekProvider;
pub use config::DeepSeekConfig;
