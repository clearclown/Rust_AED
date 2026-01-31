//! # Rust AED (Agentic Document Extraction)
//!
//! Claude Vision API を活用した Rust 製 OCR・ドキュメント抽出ライブラリ
//!
//! ## 特徴
//!
//! - **日本語縦書き対応** - プロンプトエンジニアリングによる縦書きテキストの正確な読み取り
//! - **構造化抽出** - 表、フォーム、請求書などを JSON 形式で直接出力
//! - **純 Rust 実装** - Python/GPU 依存なし
//!
//! ## クイックスタート
//!
//! ```rust,no_run
//! use rust_aed::{AedClient, OcrPreset};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // クライアント初期化（環境変数 ANTHROPIC_API_KEY を使用）
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

pub mod api;
pub mod cache;
pub mod client;
pub mod config;
pub mod error;
pub mod extraction;
pub mod presets;
pub mod prompt;
pub mod types;

// 公開 API
pub use client::AedClient;
pub use config::AedConfig;
pub use error::AedError;
pub use presets::OcrPreset;
pub use types::{
    BoundingBox, ExtractionResult, TextBlock, TextDirection,
};
