//! Gemini GenerateContent API
//!
//! Gemini GenerateContent API のリクエスト・レスポンス型定義

use serde::{Deserialize, Serialize};

/// GenerateContent API リクエスト
#[derive(Debug, Serialize)]
pub struct GenerateContentRequest {
    /// コンテンツ配列
    pub contents: Vec<Content>,
    /// 生成設定
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

/// コンテンツ
#[derive(Debug, Serialize)]
pub struct Content {
    /// ロール（user/model）
    pub role: String,
    /// パーツ
    pub parts: Vec<Part>,
}

/// コンテンツパート
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Part {
    /// テキストパート
    Text { text: String },
    /// 画像パート
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
}

/// インラインデータ
#[derive(Debug, Serialize)]
pub struct InlineData {
    /// MIME タイプ
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Base64 エンコードデータ
    pub data: String,
}

/// 生成設定
#[derive(Debug, Serialize)]
pub struct GenerationConfig {
    /// 最大出力トークン数
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    pub max_output_tokens: Option<u32>,
    /// 温度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// GenerateContent API レスポンス
#[derive(Debug, Deserialize)]
pub struct GenerateContentResponse {
    /// 候補
    pub candidates: Vec<Candidate>,
    /// 使用量メタデータ
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// 候補
#[derive(Debug, Deserialize)]
pub struct Candidate {
    /// コンテンツ
    pub content: ResponseContent,
    /// 終了理由
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

/// レスポンスコンテンツ
#[derive(Debug, Deserialize)]
pub struct ResponseContent {
    /// パーツ
    pub parts: Vec<ResponsePart>,
}

/// レスポンスパート
#[derive(Debug, Deserialize)]
pub struct ResponsePart {
    /// テキスト
    pub text: Option<String>,
}

/// 使用量メタデータ
#[derive(Debug, Deserialize)]
pub struct UsageMetadata {
    /// プロンプトトークン数
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: Option<u32>,
    /// 候補トークン数
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: Option<u32>,
    /// 合計トークン数
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<u32>,
}

impl GenerateContentRequest {
    /// 新しいリクエストを作成
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            generation_config: None,
        }
    }

    /// 最大トークン数を設定
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.generation_config = Some(GenerationConfig {
            max_output_tokens: Some(max_tokens),
            temperature: None,
        });
        self
    }

    /// ユーザーメッセージを追加（テキストのみ）
    pub fn add_user_message(mut self, text: &str) -> Self {
        self.contents.push(Content {
            role: "user".to_string(),
            parts: vec![Part::Text {
                text: text.to_string(),
            }],
        });
        self
    }

    /// ユーザーメッセージを追加（画像付き）
    pub fn add_user_message_with_image(mut self, text: &str, mime_type: &str, data: &str) -> Self {
        self.contents.push(Content {
            role: "user".to_string(),
            parts: vec![
                Part::InlineData {
                    inline_data: InlineData {
                        mime_type: mime_type.to_string(),
                        data: data.to_string(),
                    },
                },
                Part::Text {
                    text: text.to_string(),
                },
            ],
        });
        self
    }
}

impl Default for GenerateContentRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerateContentResponse {
    /// 最初の候補のテキストを取得
    pub fn text(&self) -> Option<&str> {
        self.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.as_deref())
    }

    /// 終了理由を FinishReason に変換
    pub fn finish_reason(&self) -> Option<crate::provider::FinishReason> {
        self.candidates.first().and_then(|c| {
            c.finish_reason.as_ref().map(|reason| match reason.as_str() {
                "STOP" => crate::provider::FinishReason::EndTurn,
                "MAX_TOKENS" => crate::provider::FinishReason::MaxTokens,
                "SAFETY" => crate::provider::FinishReason::ContentFilter,
                _ => crate::provider::FinishReason::EndTurn,
            })
        })
    }

    /// 入力トークン数を取得
    pub fn input_tokens(&self) -> u32 {
        self.usage_metadata
            .as_ref()
            .and_then(|u| u.prompt_token_count)
            .unwrap_or(0)
    }

    /// 出力トークン数を取得
    pub fn output_tokens(&self) -> u32 {
        self.usage_metadata
            .as_ref()
            .and_then(|u| u.candidates_token_count)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let request = GenerateContentRequest::new()
            .with_max_tokens(1024)
            .add_user_message("Hello!");

        assert_eq!(request.contents.len(), 1);
        assert!(request.generation_config.is_some());
    }

    #[test]
    fn test_multimodal_request() {
        let request = GenerateContentRequest::new()
            .add_user_message_with_image("What's this?", "image/png", "base64data");

        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.contents[0].parts.len(), 2);
    }

    #[test]
    fn test_request_serialization() {
        let request = GenerateContentRequest::new()
            .add_user_message("Test");

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Test\""));
    }
}
