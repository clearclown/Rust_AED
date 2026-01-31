//! OpenAI Chat Completions API
//!
//! OpenAI Chat Completions API のリクエスト・レスポンス型定義

use serde::{Deserialize, Serialize};

/// Chat Completions API リクエスト
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    /// モデル ID
    pub model: String,
    /// メッセージ配列
    pub messages: Vec<ChatMessage>,
    /// 最大トークン数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 温度パラメータ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// チャットメッセージ
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    /// ロール（system/user/assistant）
    pub role: String,
    /// コンテンツ
    pub content: MessageContent,
}

/// メッセージコンテンツ
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// 単純なテキスト
    Text(String),
    /// マルチモーダルコンテンツ
    Parts(Vec<ContentPart>),
}

/// コンテンツパート
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    /// テキストパート
    #[serde(rename = "text")]
    Text { text: String },
    /// 画像パート
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

/// 画像 URL
#[derive(Debug, Serialize)]
pub struct ImageUrl {
    /// URL または data URI
    pub url: String,
    /// 詳細度（low/high/auto）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Chat Completions API レスポンス
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    /// レスポンス ID
    pub id: String,
    /// オブジェクトタイプ
    pub object: String,
    /// 作成日時
    pub created: u64,
    /// モデル
    pub model: String,
    /// 選択肢
    pub choices: Vec<Choice>,
    /// 使用量
    pub usage: Usage,
}

/// 選択肢
#[derive(Debug, Deserialize)]
pub struct Choice {
    /// インデックス
    pub index: u32,
    /// メッセージ
    pub message: ResponseMessage,
    /// 終了理由
    pub finish_reason: Option<String>,
}

/// レスポンスメッセージ
#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    /// ロール
    pub role: String,
    /// コンテンツ
    pub content: Option<String>,
}

/// 使用量
#[derive(Debug, Deserialize)]
pub struct Usage {
    /// プロンプトトークン数
    pub prompt_tokens: u32,
    /// 完了トークン数
    pub completion_tokens: u32,
    /// 合計トークン数
    pub total_tokens: u32,
}

impl ChatCompletionRequest {
    /// 新しいリクエストを作成
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// 最大トークン数を設定
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// システムメッセージを追加
    pub fn add_system_message(mut self, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role: "system".to_string(),
            content: MessageContent::Text(content.to_string()),
        });
        self
    }

    /// ユーザーメッセージを追加（テキストのみ）
    pub fn add_user_message(mut self, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(content.to_string()),
        });
        self
    }

    /// ユーザーメッセージを追加（マルチモーダル）
    pub fn add_user_message_with_image(mut self, text: &str, image_data_uri: &str) -> Self {
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Parts(vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: image_data_uri.to_string(),
                        detail: Some("high".to_string()),
                    },
                },
                ContentPart::Text {
                    text: text.to_string(),
                },
            ]),
        });
        self
    }
}

impl ChatCompletionResponse {
    /// 最初の選択肢のテキストを取得
    pub fn text(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.message.content.as_deref())
    }

    /// 終了理由を FinishReason に変換
    pub fn finish_reason(&self) -> Option<crate::provider::FinishReason> {
        self.choices.first().and_then(|c| {
            c.finish_reason.as_ref().map(|reason| match reason.as_str() {
                "stop" => crate::provider::FinishReason::EndTurn,
                "length" => crate::provider::FinishReason::MaxTokens,
                "content_filter" => crate::provider::FinishReason::ContentFilter,
                "tool_calls" => crate::provider::FinishReason::ToolUse,
                _ => crate::provider::FinishReason::EndTurn,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let request = ChatCompletionRequest::new("gpt-4o")
            .with_max_tokens(1024)
            .add_system_message("You are a helpful assistant.")
            .add_user_message("Hello!");

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.messages.len(), 2);
    }

    #[test]
    fn test_multimodal_message() {
        let request = ChatCompletionRequest::new("gpt-4o")
            .add_user_message_with_image("What's in this image?", "data:image/png;base64,abc123");

        assert_eq!(request.messages.len(), 1);
        if let MessageContent::Parts(parts) = &request.messages[0].content {
            assert_eq!(parts.len(), 2);
        } else {
            panic!("Expected Parts content");
        }
    }

    #[test]
    fn test_request_serialization() {
        let request = ChatCompletionRequest::new("gpt-4o")
            .add_user_message("Hello");

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"gpt-4o\""));
        assert!(json.contains("\"role\":\"user\""));
    }
}
