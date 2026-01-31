//! AED クライアント
//!
//! Claude Vision API を使用したドキュメント抽出クライアント

use std::path::Path;
use std::time::Duration;

use crate::config::AedConfig;
use crate::error::{AedError, Result};
use crate::presets::OcrPreset;
use crate::types::{ExtractionResult, TextDirection};

/// AED クライアント
///
/// Claude Vision API を使用してドキュメントからテキストを抽出します。
///
/// # Example
///
/// ```rust,no_run
/// use rust_aed::AedClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = AedClient::from_env()?;
///
///     let result = client
///         .extract_text("document.png")
///         .await?;
///
///     println!("{}", result.text);
///     Ok(())
/// }
/// ```
pub struct AedClient {
    /// HTTP クライアント
    http_client: reqwest::Client,
    /// 設定
    config: AedConfig,
}

impl Clone for AedClient {
    fn clone(&self) -> Self {
        Self {
            http_client: self.http_client.clone(),
            config: self.config.clone(),
        }
    }
}

impl AedClient {
    /// 環境変数から API キーを読み込んでクライアントを作成
    ///
    /// # Errors
    ///
    /// 環境変数 `ANTHROPIC_API_KEY` が設定されていない場合はエラーを返します。
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| AedError::MissingApiKey)?;

        Self::new(&api_key)
    }

    /// API キーを指定してクライアントを作成
    pub fn new(api_key: &str) -> Result<Self> {
        let config = AedConfig::default().with_api_key(api_key);
        Self::with_config(config)
    }

    /// 設定を指定してクライアントを作成
    pub fn with_config(config: AedConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(AedError::HttpError)?;

        Ok(Self {
            http_client,
            config,
        })
    }

    /// 設定ファイルからクライアントを作成
    pub fn from_config_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = AedConfig::from_file(path)?;
        Self::with_config(config)
    }

    /// テキスト抽出リクエストを開始
    pub fn extract_text<P: AsRef<Path>>(&self, path: P) -> ExtractTextBuilder<'_> {
        ExtractTextBuilder::new(self, path.as_ref().to_path_buf())
    }

    /// 構造化データ抽出
    ///
    /// 指定した型に従ってドキュメントから構造化データを抽出します。
    pub async fn extract_structured<T>(&self, _path: &Path) -> Result<T>
    where
        T: serde::de::DeserializeOwned + schemars::JsonSchema,
    {
        // TODO: 実装
        unimplemented!("構造化抽出は v0.2 で実装予定")
    }

    /// Claude Vision API を呼び出し
    async fn call_vision(
        &self,
        _image_data: &[u8],
        _prompt: &str,
    ) -> Result<ExtractionResult> {
        // TODO: 実際の API 呼び出し実装
        unimplemented!("API 呼び出しは実装予定")
    }
}

/// テキスト抽出リクエストビルダー
pub struct ExtractTextBuilder<'a> {
    client: &'a AedClient,
    path: std::path::PathBuf,
    preset: Option<OcrPreset>,
    direction: TextDirection,
    language: Option<String>,
}

impl<'a> ExtractTextBuilder<'a> {
    fn new(client: &'a AedClient, path: std::path::PathBuf) -> Self {
        Self {
            client,
            path,
            preset: None,
            direction: TextDirection::Auto,
            language: None,
        }
    }

    /// プリセットを指定
    pub fn preset(mut self, preset: OcrPreset) -> Self {
        self.preset = Some(preset);
        self
    }

    /// テキスト方向を指定
    pub fn direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    /// 言語を指定（ISO 639-1 コード）
    pub fn language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }

    /// 抽出を実行
    pub async fn await_result(self) -> Result<ExtractionResult> {
        // ファイル存在確認
        if !self.path.exists() {
            return Err(AedError::FileNotFound(self.path));
        }

        // ファイル読み込み
        let image_data = std::fs::read(&self.path)?;

        // プロンプト構築
        let prompt = self.build_prompt();

        // API 呼び出し
        self.client.call_vision(&image_data, &prompt).await
    }

    fn build_prompt(&self) -> String {
        let mut prompt = String::new();

        // プリセットのシステムプロンプト
        if let Some(ref preset) = self.preset {
            prompt.push_str(&preset.system_prompt());
            prompt.push('\n');
        }

        // 方向指定
        match self.direction {
            TextDirection::Vertical => {
                prompt.push_str(
                    "この画像には日本語の縦書きテキストが含まれています。\n\
                     右から左、上から下の読み順でテキストを抽出してください。\n",
                );
            }
            TextDirection::Horizontal => {
                prompt.push_str(
                    "この画像には横書きテキストが含まれています。\n\
                     左から右、上から下の読み順でテキストを抽出してください。\n",
                );
            }
            _ => {}
        }

        // 言語指定
        if let Some(ref lang) = self.language {
            prompt.push_str(&format!("言語: {}\n", lang));
        }

        prompt.push_str("\n画像内のすべてのテキストを抽出してください。");

        prompt
    }
}

// IntoFuture の代わりにメソッドチェーンで await を呼び出す
impl<'a> ExtractTextBuilder<'a> {
    /// 抽出を実行（await 可能）
    pub async fn send(self) -> Result<ExtractionResult> {
        self.await_result().await
    }
}

// `.await` で直接呼び出せるようにする
impl<'a> std::future::IntoFuture for ExtractTextBuilder<'a> {
    type Output = Result<ExtractionResult>;
    type IntoFuture = std::pin::Pin<
        Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>,
    >;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.await_result())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_missing_key() {
        // 環境変数を一時的にクリア
        // SAFETY: テスト環境でのみ使用、他のスレッドがこの環境変数にアクセスしない前提
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
        }

        let result = AedClient::from_env();
        assert!(matches!(result, Err(AedError::MissingApiKey)));
    }

    #[test]
    fn test_build_vertical_prompt() {
        let client = AedClient {
            http_client: reqwest::Client::new(),
            config: AedConfig::default(),
        };

        let builder = client
            .extract_text("test.png")
            .direction(TextDirection::Vertical)
            .language("ja");

        let prompt = builder.build_prompt();
        assert!(prompt.contains("縦書き"));
        assert!(prompt.contains("右から左"));
        assert!(prompt.contains("ja"));
    }
}
