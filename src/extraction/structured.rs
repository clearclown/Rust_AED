//! 構造化抽出
//!
//! 画像から構造化データ（JSON）を抽出します。
//!
//! # Example
//!
//! ```rust,no_run
//! use rust_aed::AedClient;
//! use rust_aed::extraction::structured::extract_as;
//! use serde::Deserialize;
//! use schemars::JsonSchema;
//!
//! #[derive(Debug, Deserialize, JsonSchema)]
//! struct Invoice {
//!     invoice_number: String,
//!     date: String,
//!     total: f64,
//! }
//!
//! # async fn example() -> Result<(), rust_aed::AedError> {
//! let client = AedClient::from_env()?;
//! let invoice: Invoice = extract_as(&client, "invoice.png").await?;
//! println!("請求書番号: {}", invoice.invoice_number);
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use crate::api::vision;
use crate::client::AedClient;
use crate::error::{AedError, Result};

/// 構造化データを抽出
///
/// 画像から指定された型に従って構造化データを抽出します。
/// 型は `serde::Deserialize` と `schemars::JsonSchema` を実装している必要があります。
pub async fn extract_as<T, P>(client: &AedClient, path: P) -> Result<T>
where
    T: DeserializeOwned + JsonSchema,
    P: AsRef<Path>,
{
    let path = path.as_ref();

    // 画像読み込み
    let image_data = vision::load_image(path)?;

    // JSON Schema 生成
    let schema = schemars::schema_for!(T);
    let schema_json = serde_json::to_string_pretty(&schema)
        .map_err(|e| AedError::ExtractionFailed(format!("スキーマ生成失敗: {}", e)))?;

    // プロンプト構築
    let prompt = build_structured_prompt(&schema_json);

    // API 呼び出し
    let result = client.call_vision_for_structured(&image_data, &prompt).await?;

    // JSON パース
    let extracted: T = parse_json_response(&result)?;

    Ok(extracted)
}

/// 構造化データ抽出用プロンプトを構築
fn build_structured_prompt(schema: &str) -> String {
    format!(
        r#"画像から情報を抽出し、以下の JSON Schema に従って JSON 形式で出力してください。

## JSON Schema
```json
{}
```

## ルール
1. 画像内の該当する情報をすべて抽出
2. 読み取れない項目は null を設定
3. 数値は半角数字に変換
4. 日付は ISO 8601 形式（YYYY-MM-DD）に変換
5. 金額は通貨記号を除いた数値のみ

## 出力
JSON のみを出力してください。説明文は不要です。"#,
        schema
    )
}

/// JSON レスポンスをパース
fn parse_json_response<T: DeserializeOwned>(text: &str) -> Result<T> {
    // JSON ブロックを抽出（```json ... ``` で囲まれている場合）
    let json_str = extract_json_block(text);

    serde_json::from_str(json_str).map_err(|e| {
        AedError::ExtractionFailed(format!(
            "JSON パース失敗: {}. レスポンス: {}",
            e,
            &json_str[..json_str.len().min(500)]
        ))
    })
}

/// テキストから JSON ブロックを抽出
fn extract_json_block(text: &str) -> &str {
    // ```json ... ``` パターン
    if let Some(start) = text.find("```json") {
        let content_start = start + 7;
        if let Some(end) = text[content_start..].find("```") {
            return text[content_start..content_start + end].trim();
        }
    }

    // ``` ... ``` パターン（json 指定なし）
    if let Some(start) = text.find("```") {
        let content_start = start + 3;
        // 最初の改行後から開始
        if let Some(newline) = text[content_start..].find('\n') {
            let actual_start = content_start + newline + 1;
            if let Some(end) = text[actual_start..].find("```") {
                return text[actual_start..actual_start + end].trim();
            }
        }
    }

    // { で始まる JSON を探す
    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
    {
        return text[start..=end].trim();
    }

    // [ で始まる JSON 配列を探す
    if let Some(start) = text.find('[')
        && let Some(end) = text.rfind(']')
    {
        return text[start..=end].trim();
    }

    text.trim()
}

/// 構造化抽出用のビルダー
pub struct StructuredExtractBuilder<'a, T> {
    client: &'a AedClient,
    path: std::path::PathBuf,
    custom_prompt: Option<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T> StructuredExtractBuilder<'a, T>
where
    T: DeserializeOwned + JsonSchema,
{
    /// 新しいビルダーを作成
    pub fn new(client: &'a AedClient, path: impl AsRef<Path>) -> Self {
        Self {
            client,
            path: path.as_ref().to_path_buf(),
            custom_prompt: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// カスタムプロンプトを追加
    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.custom_prompt = Some(prompt.to_string());
        self
    }

    /// 抽出を実行
    pub async fn execute(self) -> Result<T> {
        let image_data = vision::load_image(&self.path)?;

        let schema = schemars::schema_for!(T);
        let schema_json = serde_json::to_string_pretty(&schema)
            .map_err(|e| AedError::ExtractionFailed(format!("スキーマ生成失敗: {}", e)))?;

        let prompt = if let Some(custom) = &self.custom_prompt {
            format!(
                "{}\n\n{}",
                custom,
                build_structured_prompt(&schema_json)
            )
        } else {
            build_structured_prompt(&schema_json)
        };

        let result = self.client.call_vision_for_structured(&image_data, &prompt).await?;
        parse_json_response(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block_with_markers() {
        let text = r#"Here is the data:
```json
{"name": "test", "value": 42}
```
"#;
        let json = extract_json_block(text);
        assert_eq!(json, r#"{"name": "test", "value": 42}"#);
    }

    #[test]
    fn test_extract_json_block_without_markers() {
        let text = r#"The result is {"name": "test", "value": 42} as shown."#;
        let json = extract_json_block(text);
        assert_eq!(json, r#"{"name": "test", "value": 42}"#);
    }

    #[test]
    fn test_extract_json_block_array() {
        let text = r#"Here is the array: [1, 2, 3]"#;
        let json = extract_json_block(text);
        assert_eq!(json, "[1, 2, 3]");
    }

    #[test]
    fn test_parse_json_response() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct TestStruct {
            name: String,
            value: i32,
        }

        let text = r#"{"name": "test", "value": 42}"#;
        let result: TestStruct = parse_json_response(text).unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.value, 42);
    }

    #[test]
    fn test_build_structured_prompt() {
        let schema = r#"{"type": "object"}"#;
        let prompt = build_structured_prompt(schema);
        assert!(prompt.contains("JSON Schema"));
        assert!(prompt.contains(schema));
    }
}
