//! DeepSeek プロバイダー実装
//!
//! 注意: DeepSeek は現在 Vision をサポートしていません。
//! このプロバイダーは Hybrid モードでテキストのみの後処理に使用されます。

use std::time::Duration;

use async_trait::async_trait;

use super::config::DeepSeekConfig;
use crate::api::vision::ImageData;
use crate::error::{AedError, Result};
use crate::provider::{LlmProvider, ProviderType, RawResponse};
use crate::types::{ExtractionResult, TextDirection, TokenUsage};

/// DeepSeek プロバイダー
///
/// DeepSeek API を使用（Vision 未サポート）
pub struct DeepSeekProvider {
    /// HTTP クライアント
    #[allow(dead_code)]
    http_client: reqwest::Client,
    /// 設定
    config: DeepSeekConfig,
}

impl DeepSeekProvider {
    /// 環境変数から設定を読み込んで作成
    pub fn from_env() -> Result<Self> {
        let config = DeepSeekConfig::from_env();
        Self::new(config)
    }

    /// 設定を指定して作成
    pub fn new(config: DeepSeekConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(AedError::missing_api_key("DEEPSEEK_API_KEY"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(AedError::HttpError)?;

        Ok(Self {
            http_client,
            config,
        })
    }

    /// API キーを指定して作成
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self> {
        let config = DeepSeekConfig::default().with_api_key(api_key);
        Self::new(config)
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    fn provider_id(&self) -> ProviderType {
        ProviderType::DeepSeek
    }

    fn supports_vision(&self) -> bool {
        false // DeepSeek は Vision 未サポート
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn extract_text(
        &self,
        _image: &ImageData,
        _prompt: &str,
        _direction: TextDirection,
    ) -> Result<ExtractionResult> {
        Err(AedError::VisionNotSupported {
            provider: "DeepSeek",
        })
    }

    async fn call_vision_raw(&self, _image: &ImageData, _prompt: &str) -> Result<RawResponse> {
        Err(AedError::VisionNotSupported {
            provider: "DeepSeek",
        })
    }

    async fn extract_structured(
        &self,
        _image: &ImageData,
        _prompt: &str,
        _schema: &str,
    ) -> Result<String> {
        Err(AedError::VisionNotSupported {
            provider: "DeepSeek",
        })
    }

    async fn health_check(&self) -> Result<bool> {
        if self.config.api_key.is_none() {
            return Ok(false);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation_without_api_key() {
        let config = DeepSeekConfig::default();
        let result = DeepSeekProvider::new(config);
        assert!(matches!(result, Err(AedError::MissingApiKey { .. })));
    }

    #[test]
    fn test_supports_vision() {
        let config = DeepSeekConfig::default().with_api_key("test");
        let provider = DeepSeekProvider::new(config).unwrap();
        assert!(!provider.supports_vision());
    }
}
