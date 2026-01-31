//! GCP Vision バックエンド
//!
//! Google Cloud Vision API を使用した OCR バックエンド

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::vision::ImageData;
use crate::error::{AedError, Result};
use crate::provider::{BlockInfo, BlockType, DocumentTextResult, PageInfo, VisionBackend, VisionResult};

/// GCP Vision API のデフォルト URL
pub const DEFAULT_GCP_VISION_URL: &str =
    "https://vision.googleapis.com/v1/images:annotate";

/// GCP Vision 設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpVisionConfig {
    /// API キー（API キー認証を使用する場合）
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// サービスアカウント認証を使用するかどうか
    #[serde(default)]
    pub use_service_account: bool,

    /// タイムアウト（秒）
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_timeout_secs() -> u64 {
    60
}

impl Default for GcpVisionConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            use_service_account: false,
            timeout_secs: default_timeout_secs(),
        }
    }
}

impl GcpVisionConfig {
    /// 環境変数から設定を構築
    pub fn from_env() -> Self {
        let api_key = std::env::var("GOOGLE_API_KEY").ok();
        let use_service_account =
            std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok();

        Self {
            api_key,
            use_service_account,
            ..Default::default()
        }
    }

    /// API キーを設定
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self.use_service_account = false;
        self
    }
}

/// GCP Vision API リクエスト
#[derive(Debug, Serialize)]
struct AnnotateImageRequest {
    requests: Vec<ImageRequest>,
}

#[derive(Debug, Serialize)]
struct ImageRequest {
    image: Image,
    features: Vec<Feature>,
}

#[derive(Debug, Serialize)]
struct Image {
    content: String,
}

#[derive(Debug, Serialize)]
struct Feature {
    #[serde(rename = "type")]
    feature_type: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxResults")]
    max_results: Option<u32>,
}

/// GCP Vision API レスポンス
#[derive(Debug, Deserialize)]
struct AnnotateImageResponse {
    responses: Vec<ImageAnnotationResponse>,
}

#[derive(Debug, Deserialize)]
struct ImageAnnotationResponse {
    #[serde(rename = "textAnnotations")]
    text_annotations: Option<Vec<TextAnnotation>>,
    #[serde(rename = "fullTextAnnotation")]
    full_text_annotation: Option<FullTextAnnotation>,
    error: Option<GcpError>,
}

#[derive(Debug, Deserialize)]
struct TextAnnotation {
    description: String,
    #[allow(dead_code)]
    locale: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FullTextAnnotation {
    text: String,
    pages: Option<Vec<Page>>,
}

#[derive(Debug, Deserialize)]
struct Page {
    blocks: Option<Vec<Block>>,
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Block {
    paragraphs: Option<Vec<Paragraph>>,
    #[serde(rename = "blockType")]
    block_type: Option<String>,
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Paragraph {
    words: Option<Vec<Word>>,
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Word {
    symbols: Option<Vec<Symbol>>,
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Symbol {
    text: String,
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct GcpError {
    code: i32,
    message: String,
}

/// GCP Vision バックエンド
pub struct GcpVisionBackend {
    /// HTTP クライアント
    http_client: reqwest::Client,
    /// 設定
    config: GcpVisionConfig,
}

impl GcpVisionBackend {
    /// 環境変数から設定を読み込んで作成
    pub fn from_env() -> Result<Self> {
        let config = GcpVisionConfig::from_env();
        Self::new(config)
    }

    /// 設定を指定して作成
    pub fn new(config: GcpVisionConfig) -> Result<Self> {
        if config.api_key.is_none() && !config.use_service_account {
            return Err(AedError::missing_api_key("GOOGLE_API_KEY"));
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
        let config = GcpVisionConfig::default().with_api_key(api_key);
        Self::new(config)
    }

    /// API リクエストを送信
    async fn send_request(&self, request: &AnnotateImageRequest) -> Result<AnnotateImageResponse> {
        let url = if let Some(ref api_key) = self.config.api_key {
            format!("{}?key={}", DEFAULT_GCP_VISION_URL, api_key)
        } else {
            // サービスアカウント認証は別途実装が必要
            return Err(AedError::provider_error(
                "GCP Vision",
                "サービスアカウント認証は未実装です",
            ));
        };

        debug!("GCP Vision API リクエスト送信");

        let response = self
            .http_client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(AedError::HttpError)?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AedError::ApiError {
                status: status.as_u16(),
                message: error_body,
            });
        }

        let api_response: AnnotateImageResponse =
            response.json().await.map_err(AedError::HttpError)?;

        // エラーチェック
        if let Some(response) = api_response.responses.first() {
            if let Some(ref error) = response.error {
                return Err(AedError::provider_error_with_code(
                    "GCP Vision",
                    &error.message,
                    error.code.to_string(),
                ));
            }
        }

        Ok(api_response)
    }
}

#[async_trait]
impl VisionBackend for GcpVisionBackend {
    fn backend_id(&self) -> &'static str {
        "gcp-vision"
    }

    async fn extract_raw_text(&self, image: &ImageData) -> Result<VisionResult> {
        let request = AnnotateImageRequest {
            requests: vec![ImageRequest {
                image: Image {
                    content: image.base64.clone(),
                },
                features: vec![Feature {
                    feature_type: "TEXT_DETECTION".to_string(),
                    max_results: None,
                }],
            }],
        };

        let response = self.send_request(&request).await?;

        let annotation = response
            .responses
            .first()
            .ok_or_else(|| AedError::ExtractionFailed("レスポンスが空です".to_string()))?;

        let text = annotation
            .text_annotations
            .as_ref()
            .and_then(|annotations| annotations.first())
            .map(|a| a.description.clone())
            .unwrap_or_default();

        // 言語検出
        let detected_language = annotation
            .text_annotations
            .as_ref()
            .and_then(|annotations| annotations.first())
            .and_then(|a| a.locale.clone());

        Ok(VisionResult {
            text,
            confidence: 0.9, // GCP Vision は全体の信頼度を返さないのでデフォルト値
            detected_language,
        })
    }

    async fn detect_document_text(&self, image: &ImageData) -> Result<DocumentTextResult> {
        let request = AnnotateImageRequest {
            requests: vec![ImageRequest {
                image: Image {
                    content: image.base64.clone(),
                },
                features: vec![Feature {
                    feature_type: "DOCUMENT_TEXT_DETECTION".to_string(),
                    max_results: None,
                }],
            }],
        };

        let response = self.send_request(&request).await?;

        let annotation = response
            .responses
            .first()
            .ok_or_else(|| AedError::ExtractionFailed("レスポンスが空です".to_string()))?;

        let full_text = annotation
            .full_text_annotation
            .as_ref()
            .map(|fta| fta.text.clone())
            .unwrap_or_default();

        // ページ情報を構築
        let pages = if let Some(ref fta) = annotation.full_text_annotation {
            fta.pages
                .as_ref()
                .map(|pages| {
                    pages
                        .iter()
                        .enumerate()
                        .map(|(i, page)| {
                            let blocks = page
                                .blocks
                                .as_ref()
                                .map(|blocks| {
                                    blocks
                                        .iter()
                                        .map(|block| {
                                            let text = extract_block_text(block);
                                            let block_type = block
                                                .block_type
                                                .as_ref()
                                                .map(|t| match t.as_str() {
                                                    "TEXT" => BlockType::Text,
                                                    "TABLE" => BlockType::Table,
                                                    "PICTURE" => BlockType::Figure,
                                                    "BARCODE" => BlockType::Barcode,
                                                    _ => BlockType::Unknown,
                                                })
                                                .unwrap_or(BlockType::Text);
                                            let confidence = block.confidence.unwrap_or(0.9);

                                            BlockInfo {
                                                text,
                                                block_type,
                                                confidence,
                                            }
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            PageInfo {
                                page_number: (i + 1) as u32,
                                blocks,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(DocumentTextResult {
            full_text,
            pages,
        })
    }
}

/// ブロックからテキストを抽出
fn extract_block_text(block: &Block) -> String {
    let mut text = String::new();

    if let Some(ref paragraphs) = block.paragraphs {
        for paragraph in paragraphs {
            if let Some(ref words) = paragraph.words {
                for word in words {
                    if let Some(ref symbols) = word.symbols {
                        for symbol in symbols {
                            text.push_str(&symbol.text);
                        }
                    }
                    text.push(' ');
                }
            }
            text.push('\n');
        }
    }

    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GcpVisionConfig::default();
        assert!(config.api_key.is_none());
        assert!(!config.use_service_account);
    }

    #[test]
    fn test_config_with_api_key() {
        let config = GcpVisionConfig::default().with_api_key("test-key");
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_backend_creation_without_credentials() {
        let config = GcpVisionConfig::default();
        let result = GcpVisionBackend::new(config);
        assert!(matches!(result, Err(AedError::MissingApiKey { .. })));
    }
}
