//! プロバイダー抽象化レイヤー
//!
//! 複数の LLM プロバイダーを統一的に扱うための trait とユーティリティを提供します。
//!
//! # サポートプロバイダー
//!
//! | カテゴリ | プロバイダー |
//! |---------|-------------|
//! | Cloud API | Claude, OpenAI, Gemini, xAI |
//! | 中国系AI | Qwen, DeepSeek, GLM (Zhipu) |
//! | Local LLM | LM Studio (OpenAI互換) |
//! | Hybrid | GCP Vision API + LLM |
//!
//! # 使用例
//!
//! ```rust,ignore
//! use rust_aed::provider::{LlmProvider, ProviderType};
//! use rust_aed::AedClient;
//!
//! // 自動検出（環境変数から）
//! let client = AedClient::from_env()?;
//!
//! // 明示的にプロバイダー指定
//! let client = AedClient::with_provider(ProviderType::OpenAI)?;
//! ```

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::api::vision::ImageData;
use crate::error::Result;
use crate::types::{ExtractionResult, TextDirection, TokenUsage};

// プロバイダーモジュール
#[cfg(feature = "claude")]
pub mod claude;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "gemini")]
pub mod gemini;

#[cfg(feature = "xai")]
pub mod xai;

#[cfg(feature = "qwen")]
pub mod qwen;

#[cfg(feature = "deepseek")]
pub mod deepseek;

#[cfg(feature = "glm")]
pub mod glm;

#[cfg(feature = "lmstudio")]
pub mod local;

#[cfg(feature = "hybrid")]
pub mod hybrid;

// 共通設定
pub mod config;

// Re-exports
pub use config::ProviderConfig;

/// プロバイダータイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ProviderType {
    /// Anthropic Claude
    Claude,
    /// OpenAI GPT-4o
    OpenAI,
    /// Google Gemini
    Gemini,
    /// xAI Grok
    XAI,
    /// Alibaba Qwen (DashScope)
    Qwen,
    /// DeepSeek
    DeepSeek,
    /// Zhipu GLM
    GLM,
    /// LM Studio (ローカル)
    LmStudio,
    /// Hybrid (Vision API + LLM)
    Hybrid,
}

impl ProviderType {
    /// 環境変数名を取得
    pub fn env_var_name(&self) -> &'static str {
        match self {
            ProviderType::Claude => "ANTHROPIC_API_KEY",
            ProviderType::OpenAI => "OPENAI_API_KEY",
            ProviderType::Gemini => "GEMINI_API_KEY",
            ProviderType::XAI => "XAI_API_KEY",
            ProviderType::Qwen => "DASHSCOPE_API_KEY",
            ProviderType::DeepSeek => "DEEPSEEK_API_KEY",
            ProviderType::GLM => "ZHIPU_API_KEY",
            ProviderType::LmStudio => "LMSTUDIO_BASE_URL",
            ProviderType::Hybrid => "GOOGLE_APPLICATION_CREDENTIALS",
        }
    }

    /// 表示名を取得
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::Claude => "Anthropic Claude",
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Gemini => "Google Gemini",
            ProviderType::XAI => "xAI Grok",
            ProviderType::Qwen => "Alibaba Qwen",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::GLM => "Zhipu GLM",
            ProviderType::LmStudio => "LM Studio",
            ProviderType::Hybrid => "Hybrid (Vision + LLM)",
        }
    }

    /// デフォルトモデルを取得
    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderType::Claude => "claude-sonnet-4-20250514",
            ProviderType::OpenAI => "gpt-4o",
            ProviderType::Gemini => "gemini-2.0-flash",
            ProviderType::XAI => "grok-2-vision-1212",
            ProviderType::Qwen => "qwen-vl-max",
            ProviderType::DeepSeek => "deepseek-chat",
            ProviderType::GLM => "glm-4v",
            ProviderType::LmStudio => "local-model",
            ProviderType::Hybrid => "claude-sonnet-4-20250514",
        }
    }

    /// Vision をサポートしているかどうか
    pub fn supports_vision(&self) -> bool {
        match self {
            ProviderType::Claude => true,
            ProviderType::OpenAI => true,
            ProviderType::Gemini => true,
            ProviderType::XAI => true,
            ProviderType::Qwen => true,
            ProviderType::DeepSeek => false, // DeepSeek は Vision 未サポート
            ProviderType::GLM => true,
            ProviderType::LmStudio => true, // モデル依存
            ProviderType::Hybrid => true,
        }
    }

    /// 環境変数から利用可能なプロバイダーを検出
    pub fn detect_from_env() -> Option<Self> {
        // 優先順位で検出
        let providers = [
            ProviderType::Claude,
            ProviderType::OpenAI,
            ProviderType::Gemini,
            ProviderType::XAI,
            ProviderType::Qwen,
            ProviderType::DeepSeek,
            ProviderType::GLM,
            ProviderType::LmStudio,
        ];

        for provider in providers {
            if std::env::var(provider.env_var_name()).is_ok() {
                return Some(provider);
            }
        }

        None
    }
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// LLM プロバイダー trait
///
/// Vision 対応の LLM プロバイダーを抽象化します。
/// `async_trait` を使用して dyn-compatible にしています。
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// プロバイダー ID を取得
    fn provider_id(&self) -> ProviderType;

    /// Vision をサポートしているかどうか
    fn supports_vision(&self) -> bool;

    /// 使用中のモデル名を取得
    fn model_name(&self) -> &str;

    /// テキスト抽出を実行
    ///
    /// 画像からテキストを抽出し、構造化された結果を返します。
    async fn extract_text(
        &self,
        image: &ImageData,
        prompt: &str,
        direction: TextDirection,
    ) -> Result<ExtractionResult>;

    /// 生の Vision API 呼び出し
    ///
    /// 画像とプロンプトを送信し、生のテキストレスポンスを返します。
    async fn call_vision_raw(&self, image: &ImageData, prompt: &str) -> Result<RawResponse>;

    /// 構造化データ抽出
    ///
    /// JSON スキーマを指定して構造化データを抽出します。
    async fn extract_structured(
        &self,
        image: &ImageData,
        prompt: &str,
        schema: &str,
    ) -> Result<String>;

    /// ヘルスチェック
    ///
    /// プロバイダーが利用可能かどうかを確認します。
    async fn health_check(&self) -> Result<bool>;
}

/// Vision バックエンド trait
///
/// GCP Vision API などの純粋な OCR バックエンドを抽象化します。
#[async_trait]
pub trait VisionBackend: Send + Sync {
    /// バックエンド ID を取得
    fn backend_id(&self) -> &'static str;

    /// 画像から生テキストを抽出
    async fn extract_raw_text(&self, image: &ImageData) -> Result<VisionResult>;

    /// ドキュメントテキスト検出（段落・ブロック情報付き）
    async fn detect_document_text(&self, image: &ImageData) -> Result<DocumentTextResult>;
}

/// 生のレスポンス
#[derive(Debug, Clone)]
pub struct RawResponse {
    /// レスポンステキスト
    pub text: String,
    /// トークン使用量
    pub usage: TokenUsage,
    /// モデル名
    pub model: String,
    /// 処理時間
    pub latency: Duration,
    /// 終了理由
    pub finish_reason: Option<FinishReason>,
}

/// 終了理由
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// 正常終了
    EndTurn,
    /// 最大トークン到達
    MaxTokens,
    /// Stop シーケンス
    StopSequence,
    /// コンテンツフィルター
    ContentFilter,
    /// ツール使用
    ToolUse,
}

/// Vision バックエンドの結果
#[derive(Debug, Clone)]
pub struct VisionResult {
    /// 抽出されたテキスト
    pub text: String,
    /// 信頼度スコア (0.0-1.0)
    pub confidence: f32,
    /// 検出された言語
    pub detected_language: Option<String>,
}

/// ドキュメントテキスト検出結果
#[derive(Debug, Clone)]
pub struct DocumentTextResult {
    /// 全テキスト
    pub full_text: String,
    /// ページ情報
    pub pages: Vec<PageInfo>,
}

/// ページ情報
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// ページ番号（1始まり）
    pub page_number: u32,
    /// ページ内のブロック
    pub blocks: Vec<BlockInfo>,
}

/// ブロック情報
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// テキスト
    pub text: String,
    /// ブロックタイプ
    pub block_type: BlockType,
    /// 信頼度
    pub confidence: f32,
}

/// ブロックタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// テキスト
    Text,
    /// テーブル
    Table,
    /// 図
    Figure,
    /// 数式
    Formula,
    /// バーコード
    Barcode,
    /// 不明
    Unknown,
}

/// プロバイダーを作成するファクトリー関数の型
pub type ProviderFactory = fn(&dyn ProviderConfig) -> Result<Arc<dyn LlmProvider>>;

/// 利用可能なプロバイダー一覧を取得
pub fn available_providers() -> Vec<ProviderType> {
    let mut providers = Vec::new();

    #[cfg(feature = "claude")]
    providers.push(ProviderType::Claude);

    #[cfg(feature = "openai")]
    providers.push(ProviderType::OpenAI);

    #[cfg(feature = "gemini")]
    providers.push(ProviderType::Gemini);

    #[cfg(feature = "xai")]
    providers.push(ProviderType::XAI);

    #[cfg(feature = "qwen")]
    providers.push(ProviderType::Qwen);

    #[cfg(feature = "deepseek")]
    providers.push(ProviderType::DeepSeek);

    #[cfg(feature = "glm")]
    providers.push(ProviderType::GLM);

    #[cfg(feature = "lmstudio")]
    providers.push(ProviderType::LmStudio);

    #[cfg(feature = "hybrid")]
    providers.push(ProviderType::Hybrid);

    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_env_var() {
        assert_eq!(ProviderType::Claude.env_var_name(), "ANTHROPIC_API_KEY");
        assert_eq!(ProviderType::OpenAI.env_var_name(), "OPENAI_API_KEY");
        assert_eq!(ProviderType::Gemini.env_var_name(), "GEMINI_API_KEY");
    }

    #[test]
    fn test_provider_type_default_model() {
        assert!(ProviderType::Claude.default_model().contains("claude"));
        assert!(ProviderType::OpenAI.default_model().contains("gpt"));
        assert!(ProviderType::Gemini.default_model().contains("gemini"));
    }

    #[test]
    fn test_provider_supports_vision() {
        assert!(ProviderType::Claude.supports_vision());
        assert!(ProviderType::OpenAI.supports_vision());
        assert!(!ProviderType::DeepSeek.supports_vision());
    }

    #[test]
    fn test_available_providers() {
        let providers = available_providers();
        // feature flags に応じて変わるので、空でないことだけ確認
        // (デフォルトでは claude が有効)
        #[cfg(feature = "claude")]
        assert!(providers.contains(&ProviderType::Claude));
    }
}
