//! OpenAI プロバイダー実装

use std::time::{Duration, Instant};

use async_trait::async_trait;
use tracing::{debug, info, warn};

use super::config::OpenAIConfig;
use super::messages::{ChatCompletionRequest, ChatCompletionResponse};
use crate::api::vision::ImageData;
use crate::error::{AedError, Result};
use crate::provider::{LlmProvider, ProviderType, RawResponse};
use crate::types::{ExtractionResult, TextBlock, TextDirection, TokenUsage};

/// OpenAI プロバイダー
///
/// OpenAI GPT-4o Vision API を使用したテキスト抽出
pub struct OpenAIProvider {
    /// HTTP クライアント
    http_client: reqwest::Client,
    /// 設定
    config: OpenAIConfig,
}

impl OpenAIProvider {
    /// 環境変数から設定を読み込んで作成
    pub fn from_env() -> Result<Self> {
        let config = OpenAIConfig::from_env();
        Self::new(config)
    }

    /// 設定を指定して作成
    pub fn new(config: OpenAIConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(AedError::missing_api_key("OPENAI_API_KEY"));
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
        let config = OpenAIConfig::default().with_api_key(api_key);
        Self::new(config)
    }

    /// 画像を data URI 形式に変換
    fn to_data_uri(image: &ImageData) -> String {
        format!("data:{};base64,{}", image.media_type, image.base64)
    }

    /// リトライ付き API 呼び出し
    async fn call_with_retry<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            attempt += 1;
            debug!("API 呼び出し試行 {}/{}", attempt, self.config.max_retries + 1);

            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if e.is_retryable() && attempt <= self.config.max_retries => {
                    warn!(
                        "API エラー (リトライ可能): {}. {}ms 後に再試行",
                        e,
                        delay.as_millis()
                    );

                    if let AedError::RateLimited { retry_after } = &e {
                        delay = *retry_after;
                    }

                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(Duration::from_millis((delay.as_millis() as f64 * 2.0) as u64), max_delay);
                }
                Err(e) => {
                    warn!("API エラー (リトライ不可または最大試行回数超過): {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// 単一の API 呼び出し
    async fn call_api(&self, request: &ChatCompletionRequest) -> Result<(ChatCompletionResponse, Duration)> {
        let start_time = Instant::now();

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| AedError::missing_api_key("OPENAI_API_KEY"))?;

        debug!("リクエスト送信: model={}", request.model);

        let mut req = self
            .http_client
            .post(&self.config.base_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org_id) = self.config.organization_id {
            req = req.header("OpenAI-Organization", org_id);
        }
        if let Some(ref project_id) = self.config.project_id {
            req = req.header("OpenAI-Project", project_id);
        }

        let response = req
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AedError::Timeout {
                        timeout: Duration::from_secs(self.config.timeout_secs),
                    }
                } else {
                    AedError::HttpError(e)
                }
            })?;

        let status = response.status();
        debug!("レスポンスステータス: {}", status);

        if !status.is_success() {
            return Err(self.handle_error_response(response).await);
        }

        let api_response: ChatCompletionResponse =
            response.json().await.map_err(AedError::HttpError)?;

        let latency = start_time.elapsed();
        Ok((api_response, latency))
    }

    /// エラーレスポンスを処理
    async fn handle_error_response(&self, response: reqwest::Response) -> AedError {
        let status = response.status().as_u16();

        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        let error_body = response.text().await.unwrap_or_default();

        match status {
            401 => AedError::invalid_api_key("OpenAI"),
            429 => AedError::RateLimited {
                retry_after: retry_after.unwrap_or(Duration::from_secs(60)),
            },
            _ => AedError::ApiError {
                status,
                message: error_body,
            },
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn provider_id(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn extract_text(
        &self,
        image: &ImageData,
        prompt: &str,
        direction: TextDirection,
    ) -> Result<ExtractionResult> {
        let full_prompt = build_extraction_prompt(prompt, direction);
        let data_uri = Self::to_data_uri(image);

        let request = ChatCompletionRequest::new(&self.config.model)
            .with_max_tokens(self.config.max_tokens)
            .add_user_message_with_image(&full_prompt, &data_uri);

        let (response, latency) = self
            .call_with_retry(|| self.call_api(&request))
            .await?;

        info!(
            "API 呼び出し成功: {} トークン使用",
            response.usage.total_tokens
        );

        let text = response
            .text()
            .ok_or_else(|| {
                AedError::ExtractionFailed("API からテキストが返されませんでした".to_string())
            })?
            .to_string();

        Ok(ExtractionResult {
            text: text.clone(),
            confidence: 1.0,
            language: "unknown".to_string(),
            direction,
            blocks: vec![TextBlock {
                text,
                bbox: None,
                confidence: 1.0,
                direction,
                font_size: None,
            }],
            processing_time: latency,
            model: response.model.clone(),
            tokens_used: TokenUsage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
            },
        })
    }

    async fn call_vision_raw(&self, image: &ImageData, prompt: &str) -> Result<RawResponse> {
        let data_uri = Self::to_data_uri(image);

        let request = ChatCompletionRequest::new(&self.config.model)
            .with_max_tokens(self.config.max_tokens)
            .add_user_message_with_image(prompt, &data_uri);

        let (response, latency) = self
            .call_with_retry(|| self.call_api(&request))
            .await?;

        let text = response
            .text()
            .ok_or_else(|| {
                AedError::ExtractionFailed("API からテキストが返されませんでした".to_string())
            })?
            .to_string();

        let finish_reason = response.finish_reason();

        Ok(RawResponse {
            text,
            usage: TokenUsage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
            },
            model: response.model.clone(),
            latency,
            finish_reason,
        })
    }

    async fn extract_structured(
        &self,
        image: &ImageData,
        prompt: &str,
        schema: &str,
    ) -> Result<String> {
        let structured_prompt = format!(
            "{}\n\n以下の JSON スキーマに従って出力してください:\n```json\n{}\n```\n\nJSON のみを出力してください。",
            prompt, schema
        );
        let response = self.call_vision_raw(image, &structured_prompt).await?;
        Ok(response.text)
    }

    async fn health_check(&self) -> Result<bool> {
        if self.config.api_key.is_none() {
            return Ok(false);
        }
        Ok(true)
    }
}

/// 抽出用プロンプトを構築
fn build_extraction_prompt(base_prompt: &str, direction: TextDirection) -> String {
    let mut prompt = String::new();

    match direction {
        TextDirection::Vertical => {
            prompt.push_str(
                "This image contains Japanese vertical text. \
                 Please extract the text reading from right to left, top to bottom.\n\n",
            );
        }
        TextDirection::Horizontal => {
            prompt.push_str(
                "This image contains horizontal text. \
                 Please extract the text reading from left to right, top to bottom.\n\n",
            );
        }
        _ => {}
    }

    prompt.push_str(base_prompt);
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_extraction_prompt() {
        let prompt = build_extraction_prompt("Extract text", TextDirection::Vertical);
        assert!(prompt.contains("vertical"));
        assert!(prompt.contains("right to left"));
    }

    #[test]
    fn test_provider_creation_without_api_key() {
        let config = OpenAIConfig::default();
        let result = OpenAIProvider::new(config);
        assert!(matches!(result, Err(AedError::MissingApiKey { .. })));
    }

    #[test]
    fn test_data_uri() {
        let image = ImageData {
            base64: "abc123".to_string(),
            media_type: "image/png".to_string(),
            original_size: 100,
        };
        let uri = OpenAIProvider::to_data_uri(&image);
        assert_eq!(uri, "data:image/png;base64,abc123");
    }
}
