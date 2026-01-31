//! # Rust AED (Agentic Document Extraction)
//!
//! マルチプロバイダー対応 Rust 製 OCR・ドキュメント抽出ライブラリ
//!
//! ## 特徴
//!
//! - **マルチプロバイダー** - Claude, OpenAI, Gemini, xAI, 中国系 AI, ローカル LLM に対応
//! - **日本語縦書き対応** - プロンプトエンジニアリングによる縦書きテキストの正確な読み取り
//! - **構造化抽出** - 表、フォーム、請求書などを JSON 形式で直接出力
//! - **純 Rust 実装** - Python/GPU 依存なし
//!
//! ## サポートプロバイダー
//!
//! | カテゴリ | プロバイダー |
//! |---------|-------------|
//! | Cloud API | Claude, OpenAI, Gemini, xAI |
//! | 中国系 AI | Qwen, DeepSeek, GLM (Zhipu) |
//! | Local LLM | LM Studio (OpenAI 互換) |
//! | Hybrid | GCP Vision API + LLM |
//!
//! ## クイックスタート
//!
//! ```rust,no_run
//! use rust_aed::{AedClient, OcrPreset};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // クライアント初期化（環境変数から自動検出）
//!     let client = AedClient::from_env()?;
//!
//!     // 画像からテキスト抽出
//!     let result = client
//!         .extract_text("document.png")
//!         .preset(OcrPreset::JapaneseBook)
//!         .await?;
//!
//!     println!("{}", result.text);
//!     Ok(())
//! }
//! ```
//!
//! ## 縦書きテキストの抽出
//!
//! ```rust,no_run
//! use rust_aed::{AedClient, TextDirection};
//!
//! # async fn example() -> Result<(), rust_aed::AedError> {
//! let client = AedClient::from_env()?;
//!
//! let result = client
//!     .extract_text("vertical_novel.png")
//!     .direction(TextDirection::Vertical)
//!     .language("ja")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## プロバイダー指定
//!
//! ```rust,no_run
//! use rust_aed::provider::ProviderType;
//! // use rust_aed::AedClient;
//!
//! # async fn example() -> Result<(), rust_aed::AedError> {
//! // OpenAI を使用
//! // let client = AedClient::with_provider(ProviderType::OpenAI)?;
//!
//! // LM Studio（ローカル）を使用
//! // let client = AedClient::with_provider(ProviderType::LmStudio)?;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod cache;
pub mod client;
pub mod config;
pub mod error;
pub mod extraction;
pub mod presets;
pub mod prompt;
pub mod provider;
pub mod types;

// 公開 API
pub use client::AedClient;
pub use config::AedConfig;
pub use error::AedError;
pub use presets::OcrPreset;
pub use types::{BoundingBox, ExtractionResult, TextBlock, TextDirection};

// プロバイダー関連の再エクスポート
pub use provider::{LlmProvider, ProviderType, VisionBackend};

#[cfg(feature = "claude")]
pub use provider::claude::{ClaudeConfig, ClaudeProvider};

#[cfg(feature = "openai")]
pub use provider::openai::{OpenAIConfig, OpenAIProvider};

#[cfg(feature = "gemini")]
pub use provider::gemini::{GeminiConfig, GeminiProvider};

#[cfg(feature = "xai")]
pub use provider::xai::{XAIConfig, XAIProvider};

#[cfg(feature = "qwen")]
pub use provider::qwen::{QwenConfig, QwenProvider};

#[cfg(feature = "deepseek")]
pub use provider::deepseek::{DeepSeekConfig, DeepSeekProvider};

#[cfg(feature = "glm")]
pub use provider::glm::{GlmConfig, GlmProvider};

#[cfg(feature = "lmstudio")]
pub use provider::local::{LmStudioConfig, LmStudioProvider};

#[cfg(feature = "hybrid")]
pub use provider::hybrid::{GcpVisionBackend, GcpVisionConfig, HybridProvider};
