//! プロンプトテンプレート
//!
//! OCR タスク用のプロンプトテンプレートを定義します。

/// テキスト方向に関するプロンプト
pub mod direction {
    /// 縦書きテキスト読み取りのプロンプト
    pub const VERTICAL: &str = r#"この画像には日本語の縦書きテキストが含まれています。
以下のルールに従ってテキストを抽出してください：

1. 右から左、上から下の読み順で読む
2. 各列を改行で区切る
3. ルビ（振り仮名）は「漢字《ふりがな》」形式で表記
4. 傍点は「﹅」で表記
5. 句読点・記号はそのまま保持"#;

    /// 横書きテキスト読み取りのプロンプト
    pub const HORIZONTAL: &str = r#"この画像には横書きテキストが含まれています。
以下のルールに従ってテキストを抽出してください：

1. 左から右、上から下の読み順で読む
2. 段落は空行で区切る
3. 表やリストの構造を保持
4. 句読点・記号はそのまま保持"#;

    /// 混在テキスト読み取りのプロンプト
    pub const MIXED: &str = r#"この画像には縦書きと横書きのテキストが混在しています。
以下のルールに従ってテキストを抽出してください：

1. 主要なテキスト方向を判断して適切な順序で読む
2. 縦書き部分は右から左に読む
3. 横書き部分は左から右に読む
4. セクション間は空行で区切る"#;
}

/// 出力フォーマットに関するプロンプト
pub mod format {
    /// プレーンテキスト出力
    pub const PLAIN_TEXT: &str = "読み取ったテキストをそのまま出力してください。";

    /// Markdown 出力
    pub const MARKDOWN: &str = r#"読み取ったテキストを Markdown 形式で出力してください：
- 見出しは # を使用
- リストは - または 1. を使用
- 表は Markdown テーブル形式
- コードは ``` で囲む"#;

    /// JSON 出力
    pub const JSON: &str = r#"読み取った情報を JSON 形式で出力してください。
説明文は不要、JSON のみを出力してください。"#;

    /// CSV 出力（表向け）
    pub const CSV: &str = r#"読み取った表データを CSV 形式で出力してください：
- 最初の行はヘッダー
- 値はカンマで区切る
- 値に特殊文字がある場合は "" で囲む"#;
}

/// 言語に関するプロンプト
pub mod language {
    /// 日本語特有の処理
    pub const JAPANESE: &str = r#"日本語テキストの抽出ルール：
- 漢字、ひらがな、カタカナを正確に識別
- 旧字体は新字体に変換
- ルビは「漢字《ふりがな》」形式
- 半角/全角は元の表記を保持"#;

    /// 英語特有の処理
    pub const ENGLISH: &str = r#"English text extraction rules:
- Preserve capitalization
- Keep punctuation as written
- Maintain paragraph structure"#;

    /// 中国語（簡体字）特有の処理
    pub const CHINESE_SIMPLIFIED: &str = "简体中文文本提取，保持原有格式。";

    /// 中国語（繁体字）特有の処理
    pub const CHINESE_TRADITIONAL: &str = "繁體中文文本提取，保持原有格式。";

    /// 韓国語特有の処理
    pub const KOREAN: &str = "한국어 텍스트 추출, 원래 형식 유지.";
}

/// 品質・制約に関するプロンプト
pub mod quality {
    /// 高精度モード
    pub const HIGH_ACCURACY: &str = r#"最高精度でテキストを抽出してください：
- 1文字も漏らさない
- 誤読の可能性がある文字は [?] でマーク
- 不明瞭な部分は [不明瞭] と注記"#;

    /// 高速モード
    pub const FAST: &str = "主要なテキストのみを素早く抽出してください。";

    /// 不明瞭文字の処理
    pub const UNCLEAR_HANDLING: &str = r#"読み取れない・不明瞭な文字の処理：
- 完全に読めない: [?]
- 推測可能: 推測文字[?]
- 複数候補: [候補1/候補2]"#;
}

/// 特殊文書タイプに関するプロンプト
pub mod document_type {
    /// 手書き文書
    pub const HANDWRITTEN: &str = r#"手書き文字を読み取ってください：
- 崩し字・癖字に注意
- 文脈から推測して補完
- 判読不能は [?] でマーク"#;

    /// 古文書・歴史的文書
    pub const HISTORICAL: &str = r#"歴史的文書を読み取ってください：
- 旧仮名遣いをそのまま保持
- 変体仮名は現代仮名に変換
- 旧字体は注記付きで変換"#;

    /// スキャン品質が低い文書
    pub const LOW_QUALITY_SCAN: &str = r#"低品質スキャン画像からテキストを抽出：
- ノイズを無視
- コントラストの低い文字に注意
- 欠損部分は文脈から推測"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_not_empty() {
        assert!(!direction::VERTICAL.is_empty());
        assert!(!direction::HORIZONTAL.is_empty());
        assert!(!format::PLAIN_TEXT.is_empty());
        assert!(!format::JSON.is_empty());
        assert!(!language::JAPANESE.is_empty());
    }
}
