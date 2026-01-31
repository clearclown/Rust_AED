//! OpenAI プロバイダー設定

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::provider::config::ProviderConfig;
use crate::provider::ProviderType;

/// OpenAI API のデフォルト URL
pub const DEFAULT_OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// デフォルトモデル
pub const DEFAULT_MODEL: &str = "gpt-4o";

/// デフォルト最大トークン数
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// デフォルトタイムアウト（秒）
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// OpenAI プロバイダー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
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

    /// Organization ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,

    /// Project ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

fn default_base_url() -> String {
    DEFAULT_OPENAI_API_URL.to_string()
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

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_base_url(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            organization_id: None,
            project_id: None,
        }
    }
}

impl OpenAIConfig {
    /// 環境変数から設定を構築
    pub fn from_env() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_API_URL.to_string());
        let organization_id = std::env::var("OPENAI_ORG_ID").ok();
        let project_id = std::env::var("OPENAI_PROJECT_ID").ok();

        Self {
            api_key,
            base_url,
            organization_id,
            project_id,
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

impl ProviderConfig for OpenAIConfig {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OpenAIConfig::default();
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(config.base_url.contains("openai.com"));
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAIConfig::default()
            .with_api_key("test-key")
            .with_model("gpt-4o-mini")
            .with_max_tokens(8192);

        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.max_tokens, 8192);
    }
}
