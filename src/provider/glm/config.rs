//! GLM プロバイダー設定

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::provider::config::ProviderConfig;
use crate::provider::ProviderType;

/// Zhipu API のデフォルト URL
pub const DEFAULT_ZHIPU_API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

/// デフォルトモデル
pub const DEFAULT_MODEL: &str = "glm-4v";

/// GLM プロバイダー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmConfig {
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
}

fn default_base_url() -> String {
    DEFAULT_ZHIPU_API_URL.to_string()
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
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

impl Default for GlmConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_base_url(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
        }
    }
}

impl GlmConfig {
    /// 環境変数から設定を構築
    pub fn from_env() -> Self {
        let api_key = std::env::var("ZHIPU_API_KEY").ok();
        let base_url = std::env::var("ZHIPU_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_ZHIPU_API_URL.to_string());

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
}

impl ProviderConfig for GlmConfig {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GLM
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
        let config = GlmConfig::default();
        assert_eq!(config.model, DEFAULT_MODEL);
        assert!(config.base_url.contains("bigmodel.cn"));
    }
}
