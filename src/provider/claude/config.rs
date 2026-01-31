//! Claude プロバイダー設定

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::provider::config::ProviderConfig;
use crate::provider::ProviderType;

/// Anthropic API のデフォルト URL
pub const DEFAULT_ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// API バージョン
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// デフォルトモデル
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// デフォルト最大トークン数
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// デフォルトタイムアウト（秒）
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Claude プロバイダー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    /// API キー
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// API ベース URL
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// 使用するモデル
    #[serde(default = "default_model")]
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

    /// 初期リトライ遅延（ミリ秒）
    #[serde(default = "default_initial_retry_delay_ms")]
    pub initial_retry_delay_ms: u64,

    /// 最大リトライ遅延（ミリ秒）
    #[serde(default = "default_max_retry_delay_ms")]
    pub max_retry_delay_ms: u64,

    /// バックオフ係数
    #[serde(default = "default_backoff_factor")]
    pub backoff_factor: f64,
}

fn default_base_url() -> String {
    DEFAULT_ANTHROPIC_API_URL.to_string()
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

fn default_max_tokens() -> u32 {
    DEFAULT_MAX_TOKENS
}

fn default_timeout_secs() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_retry_delay_ms() -> u64 {
    1000
}

fn default_max_retry_delay_ms() -> u64 {
    60000
}

fn default_backoff_factor() -> f64 {
    2.0
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_base_url(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            initial_retry_delay_ms: default_initial_retry_delay_ms(),
            max_retry_delay_ms: default_max_retry_delay_ms(),
            backoff_factor: default_backoff_factor(),
        }
    }
}

impl ClaudeConfig {
    /// 環境変数から設定を構築
    pub fn from_env() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_ANTHROPIC_API_URL.to_string());

        Self {
            api_key,
            base_url,
            ..Default::default()
        }
    }

    /// API キーを設定
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// モデルを設定
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 最大トークン数を設定
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// タイムアウトを設定
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_secs = timeout.as_secs();
        self
    }

    /// ベース URL を設定
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

impl ProviderConfig for ClaudeConfig {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Claude
    }

    fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    fn max_retries(&self) -> u32 {
        self.max_retries
    }

    fn initial_retry_delay(&self) -> Duration {
        Duration::from_millis(self.initial_retry_delay_ms)
    }

    fn max_retry_delay(&self) -> Duration {
        Duration::from_millis(self.max_retry_delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClaudeConfig::default();
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(config.base_url.contains("anthropic.com"));
    }

    #[test]
    fn test_config_builder() {
        let config = ClaudeConfig::default()
            .with_api_key("test-key")
            .with_model("claude-3-opus")
            .with_max_tokens(8192)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_provider_config_impl() {
        let config = ClaudeConfig::default().with_api_key("test-key");
        assert_eq!(config.provider_type(), ProviderType::Claude);
        assert_eq!(config.api_key(), Some("test-key"));
    }
}
