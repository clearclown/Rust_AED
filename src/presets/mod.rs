//! OCR プリセット
//!
//! 用途別に最適化されたプロンプトテンプレートを提供します。

use serde::{Deserialize, Serialize};

/// OCR プリセット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OcrPreset {
    /// 汎用（自動判定）
    General,
    /// 日本語書籍（縦書き対応）
    JapaneseBook,
    /// 漫画
    Manga,
    /// 請求書
    Invoice,
    /// 名刺
    BusinessCard,
    /// レシート
    Receipt,
    /// フォーム
    Form,
    /// 表
    Table,
    /// カスタム
    Custom(CustomPreset),
}

/// カスタムプリセット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPreset {
    /// 名前
    pub name: String,
    /// システムプロンプト
    pub system_prompt: String,
    /// 抽出プロンプト
    pub extraction_prompt: String,
}

impl OcrPreset {
    /// システムプロンプトを取得
    pub fn system_prompt(&self) -> String {
        match self {
            OcrPreset::General => GENERAL_PROMPT.to_string(),
            OcrPreset::JapaneseBook => JAPANESE_BOOK_PROMPT.to_string(),
            OcrPreset::Manga => MANGA_PROMPT.to_string(),
            OcrPreset::Invoice => INVOICE_PROMPT.to_string(),
            OcrPreset::BusinessCard => BUSINESS_CARD_PROMPT.to_string(),
            OcrPreset::Receipt => RECEIPT_PROMPT.to_string(),
            OcrPreset::Form => FORM_PROMPT.to_string(),
            OcrPreset::Table => TABLE_PROMPT.to_string(),
            OcrPreset::Custom(custom) => custom.system_prompt.clone(),
        }
    }

    /// カスタムプリセットビルダーを作成
    pub fn custom() -> CustomPresetBuilder {
        CustomPresetBuilder::default()
    }

    /// 日本語表示名を取得
    pub fn display_name_ja(&self) -> &'static str {
        match self {
            OcrPreset::General => "汎用",
            OcrPreset::JapaneseBook => "日本語書籍",
            OcrPreset::Manga => "漫画",
            OcrPreset::Invoice => "請求書",
            OcrPreset::BusinessCard => "名刺",
            OcrPreset::Receipt => "レシート",
            OcrPreset::Form => "フォーム",
            OcrPreset::Table => "表",
            OcrPreset::Custom(_) => "カスタム",
        }
    }
}

/// カスタムプリセットビルダー
#[derive(Debug, Default)]
pub struct CustomPresetBuilder {
    name: String,
    system_prompt: String,
    extraction_prompt: String,
}

impl CustomPresetBuilder {
    /// 名前を設定
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// システムプロンプトを設定
    pub fn system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = prompt.to_string();
        self
    }

    /// 抽出プロンプトを設定
    pub fn extraction_prompt(mut self, prompt: &str) -> Self {
        self.extraction_prompt = prompt.to_string();
        self
    }

    /// プリセットを構築
    pub fn build(self) -> OcrPreset {
        OcrPreset::Custom(CustomPreset {
            name: self.name,
            system_prompt: self.system_prompt,
            extraction_prompt: self.extraction_prompt,
        })
    }
}

// ============================================================
// プロンプトテンプレート
// ============================================================

const GENERAL_PROMPT: &str = r#"
あなたは高精度な OCR エンジンです。
画像内のすべてのテキストを正確に抽出してください。

ルール:
- テキストは読み取った順序で出力
- レイアウトをできるだけ保持
- 読み取れない文字は [?] で表記
"#;

const JAPANESE_BOOK_PROMPT: &str = r#"
あなたは日本語書籍専門の OCR エンジンです。
縦書き・横書きの日本語テキストを正確に抽出してください。

ルール:
- 縦書きは右から左、上から下の順で読む
- 横書きは左から右、上から下の順で読む
- ルビ（振り仮名）は「漢字《ふりがな》」形式で表記
- 傍点は「﹅」で表記
- 改ページは「---」で区切る
- 章・節のタイトルは「# 」で開始

出力形式:
読み取ったテキストをそのまま出力してください。
"#;

const MANGA_PROMPT: &str = r#"
あなたは漫画専門の OCR エンジンです。
漫画ページからテキストを抽出してください。

ルール:
- 吹き出し内のセリフを優先
- 読み順は右上から左下
- 効果音（オノマトペ）は【】で囲む
- ナレーションは「」で囲まない
- 複数の吹き出しは改行で区切る

出力形式:
セリフ1
セリフ2
【効果音】
"#;

const INVOICE_PROMPT: &str = r#"
あなたは請求書専門の OCR エンジンです。
請求書から構造化データを抽出してください。

抽出項目:
- 請求書番号
- 発行日
- 支払期限
- 発行元（会社名、住所、電話番号）
- 請求先（会社名、住所）
- 明細（品名、数量、単価、金額）
- 小計、消費税、合計金額
- 振込先

出力形式: JSON
"#;

const BUSINESS_CARD_PROMPT: &str = r#"
あなたは名刺専門の OCR エンジンです。
名刺から連絡先情報を抽出してください。

抽出項目:
- 氏名（漢字、ふりがな）
- 会社名
- 部署・役職
- 電話番号
- FAX番号
- メールアドレス
- 住所
- URL

出力形式: JSON
"#;

const RECEIPT_PROMPT: &str = r#"
あなたはレシート専門の OCR エンジンです。
レシートから購入情報を抽出してください。

抽出項目:
- 店舗名
- 店舗住所
- 電話番号
- 購入日時
- 商品明細（品名、数量、金額）
- 小計、消費税、合計
- 支払方法
- レシート番号

出力形式: JSON
"#;

const FORM_PROMPT: &str = r#"
あなたはフォーム専門の OCR エンジンです。
入力済みフォームからラベルと値のペアを抽出してください。

ルール:
- ラベル: 値 の形式で出力
- チェックボックスは [x] または [ ] で表記
- 未記入欄は「（未記入）」と表記
- 手書き文字も可能な限り読み取る

出力形式:
ラベル1: 値1
ラベル2: 値2
"#;

const TABLE_PROMPT: &str = r#"
あなたは表専門の OCR エンジンです。
画像内の表を構造化データとして抽出してください。

ルール:
- ヘッダー行を識別
- セル結合を考慮
- 空セルは空文字列
- 数値は半角に統一

出力形式: CSV または JSON（複雑な表の場合）
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_display_names() {
        assert_eq!(OcrPreset::General.display_name_ja(), "汎用");
        assert_eq!(OcrPreset::JapaneseBook.display_name_ja(), "日本語書籍");
        assert_eq!(OcrPreset::Manga.display_name_ja(), "漫画");
    }

    #[test]
    fn test_custom_preset_builder() {
        let preset = OcrPreset::custom()
            .name("医療文書")
            .system_prompt("あなたは医療文書の専門家です")
            .extraction_prompt("診断名を抽出してください")
            .build();

        if let OcrPreset::Custom(custom) = preset {
            assert_eq!(custom.name, "医療文書");
            assert!(custom.system_prompt.contains("医療文書"));
        } else {
            panic!("Expected Custom preset");
        }
    }

    #[test]
    fn test_system_prompt_not_empty() {
        let presets = [
            OcrPreset::General,
            OcrPreset::JapaneseBook,
            OcrPreset::Manga,
            OcrPreset::Invoice,
            OcrPreset::BusinessCard,
            OcrPreset::Receipt,
            OcrPreset::Form,
            OcrPreset::Table,
        ];

        for preset in presets {
            let prompt = preset.system_prompt();
            assert!(!prompt.is_empty(), "{:?} has empty prompt", preset);
        }
    }
}
