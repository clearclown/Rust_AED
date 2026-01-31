//! プロンプトビルダー
//!
//! OCR タスク用のプロンプトを組み立てます。
//!
//! # Example
//!
//! ```rust
//! use rust_aed::prompt::builder::{PromptBuilder, OutputFormat};
//! use rust_aed::TextDirection;
//!
//! let prompt = PromptBuilder::new()
//!     .direction(TextDirection::Vertical)
//!     .language("ja")
//!     .output_format(OutputFormat::PlainText)
//!     .build();
//!
//! println!("{}", prompt);
//! ```

use crate::presets::OcrPreset;
use crate::types::TextDirection;
use super::templates;

/// 出力フォーマット
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// プレーンテキスト
    #[default]
    PlainText,
    /// Markdown
    Markdown,
    /// JSON
    Json,
    /// CSV（表向け）
    Csv,
}

impl OutputFormat {
    /// フォーマットに対応するプロンプト文字列を取得
    pub fn prompt(&self) -> &'static str {
        match self {
            OutputFormat::PlainText => templates::format::PLAIN_TEXT,
            OutputFormat::Markdown => templates::format::MARKDOWN,
            OutputFormat::Json => templates::format::JSON,
            OutputFormat::Csv => templates::format::CSV,
        }
    }
}

/// 品質モード
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QualityMode {
    /// 標準（デフォルト）
    #[default]
    Standard,
    /// 高精度
    HighAccuracy,
    /// 高速
    Fast,
}

impl QualityMode {
    /// モードに対応するプロンプト文字列を取得
    pub fn prompt(&self) -> Option<&'static str> {
        match self {
            QualityMode::Standard => None,
            QualityMode::HighAccuracy => Some(templates::quality::HIGH_ACCURACY),
            QualityMode::Fast => Some(templates::quality::FAST),
        }
    }
}

/// プロンプトビルダー
///
/// さまざまなオプションを組み合わせて OCR プロンプトを構築します。
#[derive(Debug, Clone, Default)]
pub struct PromptBuilder {
    preset: Option<OcrPreset>,
    direction: TextDirection,
    language: Option<String>,
    output_format: OutputFormat,
    quality_mode: QualityMode,
    custom_instructions: Vec<String>,
    include_unclear_handling: bool,
    is_handwritten: bool,
    is_historical: bool,
    is_low_quality: bool,
}

impl PromptBuilder {
    /// 新しいビルダーを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// プリセットを設定
    pub fn preset(mut self, preset: OcrPreset) -> Self {
        self.preset = Some(preset);
        self
    }

    /// テキスト方向を設定
    pub fn direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    /// 言語を設定（ISO 639-1 コード）
    pub fn language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }

    /// 出力フォーマットを設定
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// 品質モードを設定
    pub fn quality_mode(mut self, mode: QualityMode) -> Self {
        self.quality_mode = mode;
        self
    }

    /// カスタム指示を追加
    pub fn add_instruction(mut self, instruction: &str) -> Self {
        self.custom_instructions.push(instruction.to_string());
        self
    }

    /// 不明瞭文字の処理ルールを含める
    pub fn include_unclear_handling(mut self, include: bool) -> Self {
        self.include_unclear_handling = include;
        self
    }

    /// 手書き文書フラグを設定
    pub fn handwritten(mut self, is_handwritten: bool) -> Self {
        self.is_handwritten = is_handwritten;
        self
    }

    /// 歴史的文書フラグを設定
    pub fn historical(mut self, is_historical: bool) -> Self {
        self.is_historical = is_historical;
        self
    }

    /// 低品質スキャンフラグを設定
    pub fn low_quality(mut self, is_low_quality: bool) -> Self {
        self.is_low_quality = is_low_quality;
        self
    }

    /// プロンプトを構築
    pub fn build(self) -> String {
        let mut sections = Vec::new();

        // 1. プリセットのシステムプロンプト
        if let Some(preset) = &self.preset {
            sections.push(preset.system_prompt());
        }

        // 2. テキスト方向
        match self.direction {
            TextDirection::Vertical => {
                sections.push(templates::direction::VERTICAL.to_string());
            }
            TextDirection::Horizontal => {
                sections.push(templates::direction::HORIZONTAL.to_string());
            }
            TextDirection::Mixed => {
                sections.push(templates::direction::MIXED.to_string());
            }
            TextDirection::Auto => {
                // 自動判定の場合は方向指示を含めない
            }
        }

        // 3. 言語固有の指示
        if let Some(ref lang) = self.language
            && let Some(lang_prompt) = self.get_language_prompt(lang)
        {
            sections.push(lang_prompt.to_string());
        }

        // 4. 品質モード
        if let Some(quality_prompt) = self.quality_mode.prompt() {
            sections.push(quality_prompt.to_string());
        }

        // 5. 特殊文書タイプ
        if self.is_handwritten {
            sections.push(templates::document_type::HANDWRITTEN.to_string());
        }
        if self.is_historical {
            sections.push(templates::document_type::HISTORICAL.to_string());
        }
        if self.is_low_quality {
            sections.push(templates::document_type::LOW_QUALITY_SCAN.to_string());
        }

        // 6. 不明瞭文字の処理
        if self.include_unclear_handling {
            sections.push(templates::quality::UNCLEAR_HANDLING.to_string());
        }

        // 7. カスタム指示
        for instruction in &self.custom_instructions {
            sections.push(instruction.clone());
        }

        // 8. 出力フォーマット
        sections.push(self.output_format.prompt().to_string());

        // 9. 最終指示
        sections.push("画像内のすべてのテキストを抽出してください。".to_string());

        sections.join("\n\n")
    }

    /// 言語コードに対応するプロンプトを取得
    fn get_language_prompt(&self, lang: &str) -> Option<&'static str> {
        match lang {
            "ja" => Some(templates::language::JAPANESE),
            "en" => Some(templates::language::ENGLISH),
            "zh" | "zh-CN" => Some(templates::language::CHINESE_SIMPLIFIED),
            "zh-TW" | "zh-HK" => Some(templates::language::CHINESE_TRADITIONAL),
            "ko" => Some(templates::language::KOREAN),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_builder() {
        let prompt = PromptBuilder::new().build();
        assert!(prompt.contains("読み取った"));
        assert!(prompt.contains("テキストを抽出"));
    }

    #[test]
    fn test_vertical_direction() {
        let prompt = PromptBuilder::new()
            .direction(TextDirection::Vertical)
            .build();
        assert!(prompt.contains("縦書き"));
        assert!(prompt.contains("右から左"));
    }

    #[test]
    fn test_japanese_language() {
        let prompt = PromptBuilder::new()
            .language("ja")
            .build();
        assert!(prompt.contains("漢字"));
        assert!(prompt.contains("ルビ"));
    }

    #[test]
    fn test_json_output_format() {
        let prompt = PromptBuilder::new()
            .output_format(OutputFormat::Json)
            .build();
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_high_accuracy_mode() {
        let prompt = PromptBuilder::new()
            .quality_mode(QualityMode::HighAccuracy)
            .build();
        assert!(prompt.contains("最高精度"));
    }

    #[test]
    fn test_custom_instruction() {
        let prompt = PromptBuilder::new()
            .add_instruction("数字は半角に統一してください")
            .build();
        assert!(prompt.contains("半角に統一"));
    }

    #[test]
    fn test_combined_options() {
        let prompt = PromptBuilder::new()
            .direction(TextDirection::Vertical)
            .language("ja")
            .output_format(OutputFormat::Markdown)
            .quality_mode(QualityMode::HighAccuracy)
            .handwritten(true)
            .build();

        assert!(prompt.contains("縦書き"));
        assert!(prompt.contains("日本語"));
        assert!(prompt.contains("Markdown"));
        assert!(prompt.contains("最高精度"));
        assert!(prompt.contains("手書き"));
    }

    #[test]
    fn test_preset_integration() {
        let prompt = PromptBuilder::new()
            .preset(OcrPreset::Invoice)
            .build();
        assert!(prompt.contains("請求書"));
    }
}
