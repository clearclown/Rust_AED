//! プロバイダー設定
//!
//! 各プロバイダー共通の設定 trait と基本設定構造体を定義します。

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::ProviderType;

/// プロバイダー設定 trait
///
/// 全てのプロバイダー設定が実装する共通インターフェース
pub trait ProviderConfig: Send + Sync {
    /// プロバイダータイプを取得
    fn provider_type(&self) -> ProviderType;

    /// API キーを取得
    fn api_key(&self) -> Option<&str>;

    /// ベース URL を取得
    fn base_url(&self) -> &str;

    /// モデル名を取得
    fn model(&self) -> &str;

    /// 最大トークン数を取得
    fn max_tokens(&self) -> u32;

    /// タイムアウトを取得
    fn timeout(&self) -> Duration;

    /// リトライ回数を取得
    fn max_retries(&self) -> u32 {
        3
    }

    /// 初期リトライ遅延を取得
    fn initial_retry_delay(&self) -> Duration {
        Duration::from_secs(1)
    }

    /// 最大リトライ遅延を取得
    fn max_retry_delay(&self) -> Duration {
        Duration::from_secs(60)
    }
}

/// 基本プロバイダー設定
///
/// 多くのプロバイダーで共通して使用できる設定構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseProviderConfig {
    /// API キー
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// ベース URL
    pub base_url: String,

    /// モデル名
    pub model: String,

    /// 最大トークン数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// タイムアウト（秒）
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// 最大リトライ回数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// 初期リトライ遅延（秒）
    #[serde(default = "default_initial_retry_delay_secs")]
    pub initial_retry_delay_secs: u64,

    /// 最大リトライ遅延（秒）
    #[serde(default = "default_max_retry_delay_secs")]
    pub max_retry_delay_secs: u64,
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout_secs() -> u64 {
    120
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_retry_delay_secs() -> u64 {
    1
}

fn default_max_retry_delay_secs() -> u64 {
    60
}

impl BaseProviderConfig {
    /// 環境変数から設定を構築
    pub fn from_env_with_defaults(
        api_key_env: &str,
        base_url_env: Option<&str>,
        default_base_url: &str,
        default_model: &str,
    ) -> Self {
        let api_key = std::env::var(api_key_env).ok();

        let base_url = base_url_env
            .and_then(|env| std::env::var(env).ok())
            .unwrap_or_else(|| default_base_url.to_string());

        Self {
            api_key,
            base_url,
            model: default_model.to_string(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            initial_retry_delay_secs: default_initial_retry_delay_secs(),
            max_retry_delay_secs: default_max_retry_delay_secs(),
        }
    }

    /// ビルダーパターンでモデルを設定
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// ビルダーパターンで最大トークン数を設定
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// ビルダーパターンでタイムアウトを設定
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_secs = timeout.as_secs();
        self
    }

    /// ビルダーパターンでベース URL を設定
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// タイムアウトを Duration として取得
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

impl Default for BaseProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: String::new(),
            model: String::new(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            initial_retry_delay_secs: default_initial_retry_delay_secs(),
            max_retry_delay_secs: default_max_retry_delay_secs(),
        }
    }
}

/// OpenAI 互換 API の設定
///
/// OpenAI, LM Studio, その他 OpenAI 互換サービスで使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatibleConfig {
    /// 基本設定
    #[serde(flatten)]
    pub base: BaseProviderConfig,

    /// Organization ID (OpenAI のみ)
    pub organization_id: Option<String>,

    /// Project ID (OpenAI のみ)
    pub project_id: Option<String>,
}

impl OpenAICompatibleConfig {
    /// OpenAI 用のデフォルト設定
    pub fn openai_defaults() -> Self {
        Self {
            base: BaseProviderConfig::from_env_with_defaults(
                "OPENAI_API_KEY",
                Some("OPENAI_BASE_URL"),
                "https://api.openai.com/v1",
                "gpt-4o",
            ),
            organization_id: std::env::var("OPENAI_ORG_ID").ok(),
            project_id: std::env::var("OPENAI_PROJECT_ID").ok(),
        }
    }

    /// LM Studio 用のデフォルト設定
    pub fn lmstudio_defaults() -> Self {
        let base_url = std::env::var("LMSTUDIO_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());

        Self {
            base: BaseProviderConfig {
                api_key: Some("lm-studio".to_string()), // LM Studio は任意のキーを受け付ける
                base_url,
                model: "local-model".to_string(),
                ..Default::default()
            },
            organization_id: None,
            project_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_config_defaults() {
        let config = BaseProviderConfig::default();
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout_secs, 120);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_base_config_builder() {
        let config = BaseProviderConfig::default()
            .with_model("test-model")
            .with_max_tokens(8192)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.model, "test-model");
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_openai_compatible_lmstudio_defaults() {
        let config = OpenAICompatibleConfig::lmstudio_defaults();
        assert!(config.base.base_url.contains("localhost"));
        assert_eq!(config.base.api_key, Some("lm-studio".to_string()));
    }
}
