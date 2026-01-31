//! 共通型定義
//!
//! Rust AED で使用する共通の型を定義します。

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// テキスト抽出結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// 抽出されたテキスト
    pub text: String,

    /// 信頼度スコア (0.0-1.0)
    pub confidence: f32,

    /// 検出された言語（ISO 639-1 コード）
    pub language: String,

    /// テキスト方向
    pub direction: TextDirection,

    /// テキストブロック（位置情報付き）
    pub blocks: Vec<TextBlock>,

    /// 処理時間
    #[serde(with = "duration_serde")]
    pub processing_time: Duration,

    /// 使用したモデル
    pub model: String,

    /// 使用したトークン数
    pub tokens_used: TokenUsage,
}

/// テキストブロック
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// テキスト内容
    pub text: String,

    /// バウンディングボックス（オプション）
    pub bbox: Option<BoundingBox>,

    /// 信頼度スコア (0.0-1.0)
    pub confidence: f32,

    /// テキスト方向
    pub direction: TextDirection,

    /// フォントサイズ推定（ポイント）
    pub font_size: Option<f32>,
}

/// バウンディングボックス
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    /// 左上 X 座標
    pub x: u32,
    /// 左上 Y 座標
    pub y: u32,
    /// 幅
    pub width: u32,
    /// 高さ
    pub height: u32,
}

impl BoundingBox {
    /// 新しいバウンディングボックスを作成
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// 面積を計算
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// 中心座標を取得
    pub fn center(&self) -> (u32, u32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    /// 2つのバウンディングボックスが重なっているかを判定
    pub fn overlaps(&self, other: &BoundingBox) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

/// テキスト方向
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    /// 自動判定
    #[default]
    Auto,
    /// 横書き（左から右、上から下）
    Horizontal,
    /// 縦書き（右から左、上から下）
    Vertical,
    /// 混在
    Mixed,
}

impl TextDirection {
    /// 日本語表示名を取得
    pub fn display_name_ja(&self) -> &'static str {
        match self {
            TextDirection::Auto => "自動",
            TextDirection::Horizontal => "横書き",
            TextDirection::Vertical => "縦書き",
            TextDirection::Mixed => "混在",
        }
    }
}

/// トークン使用量
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 入力トークン数
    pub input_tokens: u32,
    /// 出力トークン数
    pub output_tokens: u32,
}

impl TokenUsage {
    /// 合計トークン数
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// コスト概算（USD）
    ///
    /// Sonnet 4.5 の料金: $3/1M input, $15/1M output
    pub fn estimated_cost_usd(&self) -> f64 {
        let input_cost = (self.input_tokens as f64) * 3.0 / 1_000_000.0;
        let output_cost = (self.output_tokens as f64) * 15.0 / 1_000_000.0;
        input_cost + output_cost
    }
}

/// バッチ処理結果
#[derive(Debug)]
pub struct BatchResult {
    /// 成功した結果
    pub successful: Vec<ExtractionResult>,
    /// 失敗したファイルとエラー
    pub failed: Vec<(std::path::PathBuf, crate::error::AedError)>,
    /// 合計処理時間
    pub total_time: Duration,
    /// 合計トークン使用量
    pub total_tokens: TokenUsage,
}

impl BatchResult {
    /// 成功率を計算
    pub fn success_rate(&self) -> f64 {
        let total = self.successful.len() + self.failed.len();
        if total == 0 {
            0.0
        } else {
            self.successful.len() as f64 / total as f64
        }
    }
}

/// Duration のシリアライズ/デシリアライズ
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_area() {
        let bbox = BoundingBox::new(10, 20, 100, 50);
        assert_eq!(bbox.area(), 5000);
    }

    #[test]
    fn test_bounding_box_center() {
        let bbox = BoundingBox::new(0, 0, 100, 100);
        assert_eq!(bbox.center(), (50, 50));
    }

    #[test]
    fn test_bounding_box_overlaps() {
        let box1 = BoundingBox::new(0, 0, 100, 100);
        let box2 = BoundingBox::new(50, 50, 100, 100);
        let box3 = BoundingBox::new(200, 200, 50, 50);

        assert!(box1.overlaps(&box2));
        assert!(!box1.overlaps(&box3));
    }

    #[test]
    fn test_token_usage_cost() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
        };

        // $3/1M input + $15/1M output
        // = 1000 * 3 / 1M + 500 * 15 / 1M
        // = 0.003 + 0.0075
        // = 0.0105
        let cost = usage.estimated_cost_usd();
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_text_direction_display() {
        assert_eq!(TextDirection::Vertical.display_name_ja(), "縦書き");
        assert_eq!(TextDirection::Horizontal.display_name_ja(), "横書き");
    }
}
