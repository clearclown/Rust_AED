//! Qwen プロバイダー実装

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::config::QwenConfig;
use crate::api::vision::ImageData;
use crate::error::{AedError, Result};
use crate::provider::{LlmProvider, ProviderType, RawResponse};
use crate::types::{ExtractionResult, TextBlock, TextDirection, TokenUsage};

/// DashScope API リクエスト
#[derive(Debug, Serialize)]
struct DashScopeRequest {
    model: String,
    input: Input,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Parameters>,
}

#[derive(Debug, Serialize)]
struct Input {
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Content {
    Text { text: String },
    Image { image: String },
}

#[derive(Debug, Serialize)]
struct Parameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// DashScope API レスポンス
#[derive(Debug, Deserialize)]
struct DashScopeResponse {
    output: Output,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Output {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Vec<ResponseContent>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Qwen プロバイダー
///
/// Alibaba Qwen (DashScope) Vision API を使用したテキスト抽出
pub struct QwenProvider {
    /// HTTP クライアント
    http_client: reqwest::Client,
    /// 設定
    config: QwenConfig,
}

impl QwenProvider {
    /// 環境変数から設定を読み込んで作成
    pub fn from_env() -> Result<Self> {
        let config = QwenConfig::from_env();
        Self::new(config)
    }

    /// 設定を指定して作成
    pub fn new(config: QwenConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(AedError::missing_api_key("DASHSCOPE_API_KEY"));
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
        let config = QwenConfig::default().with_api_key(api_key);
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
                    delay = std::cmp::min(
                        Duration::from_millis((delay.as_millis() as f64 * 2.0) as u64),
                        max_delay,
                    );
                }
                Err(e) => {
                    warn!("API エラー (リトライ不可または最大試行回数超過): {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// 単一の API 呼び出し
    async fn call_api(&self, request: &DashScopeRequest) -> Result<(DashScopeResponse, Duration)> {
        let start_time = Instant::now();

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| AedError::missing_api_key("DASHSCOPE_API_KEY"))?;

        debug!("リクエスト送信: model={}", request.model);

        let response = self
            .http_client
            .post(&self.config.base_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
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

        let api_response: DashScopeResponse =
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
            401 | 403 => AedError::invalid_api_key("Qwen"),
            429 => AedError::RateLimited {
                retry_after: retry_after.unwrap_or(Duration::from_secs(60)),
            },
            _ => AedError::ApiError {
                status,
                message: error_body,
            },
        }
    }

    /// リクエストを構築
    fn build_request(&self, prompt: &str, image: &ImageData) -> DashScopeRequest {
        let data_uri = Self::to_data_uri(image);

        DashScopeRequest {
            model: self.config.model.clone(),
            input: Input {
                messages: vec![Message {
                    role: "user".to_string(),
                    content: vec![
                        Content::Image { image: data_uri },
                        Content::Text {
                            text: prompt.to_string(),
                        },
                    ],
                }],
            },
            parameters: Some(Parameters {
                max_tokens: Some(self.config.max_tokens),
            }),
        }
    }
}

#[async_trait]
impl LlmProvider for QwenProvider {
    fn provider_id(&self) -> ProviderType {
        ProviderType::Qwen
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
        let request = self.build_request(&full_prompt, image);

        let (response, latency) = self.call_with_retry(|| self.call_api(&request)).await?;

        info!(
            "API 呼び出し成功: {} トークン使用",
            response.usage.input_tokens + response.usage.output_tokens
        );

        let text = response
            .output
            .choices
            .first()
            .and_then(|c| c.message.content.first())
            .and_then(|c| c.text.as_deref())
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
            model: self.config.model.clone(),
            tokens_used: TokenUsage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
            },
        })
    }

    async fn call_vision_raw(&self, image: &ImageData, prompt: &str) -> Result<RawResponse> {
        let request = self.build_request(prompt, image);

        let (response, latency) = self.call_with_retry(|| self.call_api(&request)).await?;

        let text = response
            .output
            .choices
            .first()
            .and_then(|c| c.message.content.first())
            .and_then(|c| c.text.as_deref())
            .ok_or_else(|| {
                AedError::ExtractionFailed("API からテキストが返されませんでした".to_string())
            })?
            .to_string();

        let finish_reason = response.output.choices.first().and_then(|c| {
            c.finish_reason.as_ref().map(|r| match r.as_str() {
                "stop" => crate::provider::FinishReason::EndTurn,
                "length" => crate::provider::FinishReason::MaxTokens,
                _ => crate::provider::FinishReason::EndTurn,
            })
        });

        Ok(RawResponse {
            text,
            usage: TokenUsage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
            },
            model: self.config.model.clone(),
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
                "这张图片包含日语竖排文字。请从右到左、从上到下的顺序提取文本。\n\n",
            );
        }
        TextDirection::Horizontal => {
            prompt.push_str(
                "这张图片包含横排文字。请从左到右、从上到下的顺序提取文本。\n\n",
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
    fn test_provider_creation_without_api_key() {
        let config = QwenConfig::default();
        let result = QwenProvider::new(config);
        assert!(matches!(result, Err(AedError::MissingApiKey { .. })));
    }

    #[test]
    fn test_data_uri() {
        let image = ImageData {
            base64: "abc123".to_string(),
            media_type: "image/png".to_string(),
            original_size: 100,
        };
        let uri = QwenProvider::to_data_uri(&image);
        assert_eq!(uri, "data:image/png;base64,abc123");
    }
}
