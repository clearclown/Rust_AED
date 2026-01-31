//! LM Studio プロバイダー
//!
//! LM Studio (OpenAI 互換 API) を使用したテキスト抽出プロバイダー

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::api::vision::ImageData;
use crate::error::{AedError, Result};
use crate::provider::config::ProviderConfig;
use crate::provider::{LlmProvider, ProviderType, RawResponse};
use crate::types::{ExtractionResult, TextBlock, TextDirection, TokenUsage};

/// LM Studio デフォルト URL
pub const DEFAULT_LMSTUDIO_URL: &str = "http://localhost:1234/v1/chat/completions";

/// デフォルトモデル
pub const DEFAULT_MODEL: &str = "local-model";

/// LM Studio 設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LmStudioConfig {
    /// ベース URL
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// モデル名（LM Studio でロードしたモデル）
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
    DEFAULT_LMSTUDIO_URL.to_string()
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout_secs() -> u64 {
    300 // ローカル LLM は遅い場合があるので長めに
}

fn default_max_retries() -> u32 {
    3
}

impl Default for LmStudioConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
        }
    }
}

impl LmStudioConfig {
    /// 環境変数から設定を構築
    pub fn from_env() -> Self {
        let base_url = std::env::var("LMSTUDIO_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_LMSTUDIO_URL.to_string());
        let model =
            std::env::var("LMSTUDIO_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self {
            base_url,
            model,
            ..Default::default()
        }
    }

    /// ベース URL を設定
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// モデルを設定
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl ProviderConfig for LmStudioConfig {
    fn provider_type(&self) -> ProviderType {
        ProviderType::LmStudio
    }

    fn api_key(&self) -> Option<&str> {
        // LM Studio は API キー不要（任意の文字列を受け付ける）
        Some("lm-studio")
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

// OpenAI 互換のリクエスト/レスポンス型
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: MessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// LM Studio プロバイダー
///
/// ローカルで動作する LLM を使用したテキスト抽出
pub struct LmStudioProvider {
    /// HTTP クライアント
    http_client: reqwest::Client,
    /// 設定
    config: LmStudioConfig,
}

impl LmStudioProvider {
    /// 環境変数から設定を読み込んで作成
    pub fn from_env() -> Result<Self> {
        let config = LmStudioConfig::from_env();
        Self::new(config)
    }

    /// 設定を指定して作成
    pub fn new(config: LmStudioConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(AedError::HttpError)?;

        Ok(Self {
            http_client,
            config,
        })
    }

    /// デフォルト設定で作成
    pub fn localhost() -> Result<Self> {
        Self::new(LmStudioConfig::default())
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
    async fn call_api(&self, request: &ChatCompletionRequest) -> Result<(ChatCompletionResponse, Duration)> {
        let start_time = Instant::now();

        debug!("リクエスト送信: model={}", request.model);

        let response = self
            .http_client
            .post(&self.config.base_url)
            .header("Authorization", "Bearer lm-studio")
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AedError::Timeout {
                        timeout: Duration::from_secs(self.config.timeout_secs),
                    }
                } else if e.is_connect() {
                    AedError::ProviderUnavailable {
                        provider: "LM Studio",
                        reason: "ローカルサーバーに接続できません。LM Studio が起動しているか確認してください。".to_string(),
                    }
                } else {
                    AedError::HttpError(e)
                }
            })?;

        let status = response.status();
        debug!("レスポンスステータス: {}", status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AedError::ApiError {
                status: status.as_u16(),
                message: error_body,
            });
        }

        let api_response: ChatCompletionResponse =
            response.json().await.map_err(AedError::HttpError)?;

        let latency = start_time.elapsed();
        Ok((api_response, latency))
    }

    /// リクエストを構築
    fn build_request(&self, prompt: &str, image: &ImageData) -> ChatCompletionRequest {
        let data_uri = Self::to_data_uri(image);

        ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Parts(vec![
                    ContentPart::ImageUrl {
                        image_url: ImageUrl { url: data_uri },
                    },
                    ContentPart::Text {
                        text: prompt.to_string(),
                    },
                ]),
            }],
            max_tokens: Some(self.config.max_tokens),
        }
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    fn provider_id(&self) -> ProviderType {
        ProviderType::LmStudio
    }

    fn supports_vision(&self) -> bool {
        // モデルに依存するが、Vision 対応モデルがロードされている前提
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

        let usage = response.usage.as_ref();
        info!(
            "API 呼び出し成功: {} トークン使用",
            usage.map(|u| u.prompt_tokens + u.completion_tokens).unwrap_or(0)
        );

        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
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
                input_tokens: usage.map(|u| u.prompt_tokens).unwrap_or(0),
                output_tokens: usage.map(|u| u.completion_tokens).unwrap_or(0),
            },
        })
    }

    async fn call_vision_raw(&self, image: &ImageData, prompt: &str) -> Result<RawResponse> {
        let request = self.build_request(prompt, image);

        let (response, latency) = self.call_with_retry(|| self.call_api(&request)).await?;

        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .ok_or_else(|| {
                AedError::ExtractionFailed("API からテキストが返されませんでした".to_string())
            })?
            .to_string();

        let finish_reason = response.choices.first().and_then(|c| {
            c.finish_reason.as_ref().map(|r| match r.as_str() {
                "stop" => crate::provider::FinishReason::EndTurn,
                "length" => crate::provider::FinishReason::MaxTokens,
                _ => crate::provider::FinishReason::EndTurn,
            })
        });

        let usage = response.usage.as_ref();

        Ok(RawResponse {
            text,
            usage: TokenUsage {
                input_tokens: usage.map(|u| u.prompt_tokens).unwrap_or(0),
                output_tokens: usage.map(|u| u.completion_tokens).unwrap_or(0),
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
        // サーバーへの接続テスト
        match self
            .http_client
            .get(self.config.base_url.replace("/chat/completions", "/models"))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
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
    fn test_default_config() {
        let config = LmStudioConfig::default();
        assert!(config.base_url.contains("localhost"));
        assert!(config.base_url.contains("1234"));
    }

    #[test]
    fn test_data_uri() {
        let image = ImageData {
            base64: "abc123".to_string(),
            media_type: "image/png".to_string(),
            original_size: 100,
        };
        let uri = LmStudioProvider::to_data_uri(&image);
        assert_eq!(uri, "data:image/png;base64,abc123");
    }
}
