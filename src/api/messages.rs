//! Claude Messages API
//!
//! Claude Messages API のリクエスト・レスポンス型定義

use serde::{Deserialize, Serialize};

/// Messages API リクエスト
#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    /// モデル ID
    pub model: String,
    /// 最大トークン数
    pub max_tokens: u32,
    /// メッセージ配列
    pub messages: Vec<Message>,
    /// システムプロンプト（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// メッセージ
#[derive(Debug, Serialize)]
pub struct Message {
    /// ロール（user/assistant）
    pub role: String,
    /// コンテンツ
    pub content: Vec<ContentBlock>,
}

/// コンテンツブロック
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// テキストブロック
    #[serde(rename = "text")]
    Text { text: String },
    /// 画像ブロック
    #[serde(rename = "image")]
    Image { source: ImageSource },
    /// ドキュメントブロック（PDF）
    #[serde(rename = "document")]
    Document { source: DocumentSource },
}

/// 画像ソース
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ImageSource {
    /// Base64 エンコード
    #[serde(rename = "base64")]
    Base64 {
        media_type: String,
        data: String,
    },
    /// URL
    #[serde(rename = "url")]
    Url { url: String },
}

/// ドキュメントソース
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum DocumentSource {
    /// Base64 エンコード
    #[serde(rename = "base64")]
    Base64 {
        media_type: String,
        data: String,
    },
    /// URL
    #[serde(rename = "url")]
    Url { url: String },
}

/// Messages API レスポンス
#[derive(Debug, Deserialize)]
pub struct MessagesResponse {
    /// レスポンス ID
    pub id: String,
    /// モデル
    pub model: String,
    /// 停止理由
    pub stop_reason: Option<String>,
    /// コンテンツ
    pub content: Vec<ResponseContentBlock>,
    /// 使用量
    pub usage: Usage,
}

/// レスポンスコンテンツブロック
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

/// 使用量
#[derive(Debug, Deserialize)]
pub struct Usage {
    /// 入力トークン数
    pub input_tokens: u32,
    /// 出力トークン数
    pub output_tokens: u32,
}

impl MessagesRequest {
    /// 新しいリクエストを作成
    pub fn new(model: &str, max_tokens: u32) -> Self {
        Self {
            model: model.to_string(),
            max_tokens,
            messages: Vec::new(),
            system: None,
        }
    }

    /// システムプロンプトを設定
    pub fn with_system(mut self, system: &str) -> Self {
        self.system = Some(system.to_string());
        self
    }

    /// ユーザーメッセージを追加
    pub fn add_user_message(mut self, content: Vec<ContentBlock>) -> Self {
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });
        self
    }
}

impl ContentBlock {
    /// テキストブロックを作成
    pub fn text(text: &str) -> Self {
        ContentBlock::Text {
            text: text.to_string(),
        }
    }

    /// Base64 画像ブロックを作成
    pub fn image_base64(media_type: &str, data: &str) -> Self {
        ContentBlock::Image {
            source: ImageSource::Base64 {
                media_type: media_type.to_string(),
                data: data.to_string(),
            },
        }
    }

    /// URL 画像ブロックを作成
    pub fn image_url(url: &str) -> Self {
        ContentBlock::Image {
            source: ImageSource::Url {
                url: url.to_string(),
            },
        }
    }
}

impl MessagesResponse {
    /// テキストコンテンツを取得
    pub fn text(&self) -> Option<&str> {
        for block in &self.content {
            if let ResponseContentBlock::Text { text } = block {
                return Some(text);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let request = MessagesRequest::new("claude-sonnet-4-5", 1024)
            .with_system("You are a helpful assistant.")
            .add_user_message(vec![ContentBlock::text("Hello!")]);

        assert_eq!(request.model, "claude-sonnet-4-5");
        assert_eq!(request.max_tokens, 1024);
        assert!(request.system.is_some());
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_content_block_serialization() {
        let text_block = ContentBlock::text("Hello");
        let json = serde_json::to_string(&text_block).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let image_block = ContentBlock::image_base64("image/png", "base64data");
        let json = serde_json::to_string(&image_block).unwrap();
        assert!(json.contains("\"type\":\"image\""));
        assert!(json.contains("\"type\":\"base64\""));
    }
}
