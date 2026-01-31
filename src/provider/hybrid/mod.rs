//! Hybrid プロバイダー
//!
//! Vision バックエンド（GCP Vision など）+ LLM の組み合わせ
//!
//! # 使用方法
//!
//! ```rust,ignore
//! use rust_aed::provider::hybrid::{HybridProvider, GcpVisionBackend};
//! use rust_aed::provider::claude::ClaudeProvider;
//!
//! let vision = GcpVisionBackend::from_env()?;
//! let llm = ClaudeProvider::from_env()?;
//! let provider = HybridProvider::new(vision, llm);
//! ```

mod gcp_vision;
mod provider;

pub use gcp_vision::{GcpVisionBackend, GcpVisionConfig};
pub use provider::HybridProvider;
