//! AED クライアント
//!
//! マルチプロバイダー対応のドキュメント抽出クライアント
//!
//! # サポートプロバイダー
//!
//! | カテゴリ | プロバイダー |
//! |---------|-------------|
//! | Cloud API | Claude, OpenAI, Gemini, xAI |
//! | 中国系AI | Qwen, DeepSeek, GLM (Zhipu) |
//! | Local LLM | LM Studio (OpenAI互換) |
//! | Hybrid | GCP Vision API + LLM |

use std::path::Path;
use std::sync::Arc;

use tracing::{debug, info};

use crate::api::vision::{self, ImageData};
use crate::config::AedConfig;
use crate::error::{AedError, Result};
use crate::presets::OcrPreset;
use crate::provider::{LlmProvider, ProviderType};
use crate::types::{ExtractionResult, TextDirection};

/// AED クライアント
///
/// マルチプロバイダー対応のドキュメント抽出クライアントです。
/// 任意の LLM プロバイダーを使用してテキスト抽出を行います。
///
/// # Example
///
/// ```rust,no_run
/// use rust_aed::AedClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // 環境変数から自動検出
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
///
/// # プロバイダー指定
///
/// ```rust,no_run
/// use rust_aed::{AedClient, ProviderType};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // 明示的にプロバイダーを指定
/// let client = AedClient::with_provider(ProviderType::OpenAI)?;
/// # Ok(())
/// # }
/// ```
pub struct AedClient {
    /// LLM プロバイダー
    provider: Arc<dyn LlmProvider>,
    /// 設定
    config: AedConfig,
}

impl Clone for AedClient {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
            config: self.config.clone(),
        }
    }
}

impl AedClient {
    /// 環境変数から自動検出してクライアントを作成
    ///
    /// 以下の順序で環境変数を確認し、最初に見つかったプロバイダーを使用します:
    /// 1. `ANTHROPIC_API_KEY` → Claude
    /// 2. `OPENAI_API_KEY` → OpenAI
    /// 3. `GEMINI_API_KEY` → Gemini
    /// 4. `XAI_API_KEY` → xAI
    /// 5. `DASHSCOPE_API_KEY` → Qwen
    /// 6. `DEEPSEEK_API_KEY` → DeepSeek
    /// 7. `ZHIPU_API_KEY` → GLM
    /// 8. `LMSTUDIO_BASE_URL` → LM Studio
    ///
    /// # Errors
    ///
    /// いずれの環境変数も設定されていない場合はエラーを返します。
    pub fn from_env() -> Result<Self> {
        let provider_type = ProviderType::detect_from_env()
            .ok_or(AedError::NoProviderAvailable)?;

        info!("プロバイダー自動検出: {}", provider_type);
        Self::with_provider(provider_type)
    }

    /// 指定したプロバイダータイプでクライアントを作成
    ///
    /// プロバイダー固有の環境変数から設定を読み込みます。
    pub fn with_provider(provider_type: ProviderType) -> Result<Self> {
        let provider = create_provider_from_env(provider_type)?;
        let config = AedConfig::default();

        Ok(Self { provider, config })
    }

    /// LLM プロバイダーを直接指定してクライアントを作成
    ///
    /// カスタム設定のプロバイダーを使用する場合に便利です。
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rust_aed::AedClient;
    /// # #[cfg(feature = "claude")]
    /// use rust_aed::ClaudeProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "claude")]
    /// # {
    /// let provider = ClaudeProvider::from_env()?;
    /// let client = AedClient::from_provider(provider);
    /// # }
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_provider<P: LlmProvider + 'static>(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            config: AedConfig::default(),
        }
    }

    /// Arc で包まれたプロバイダーからクライアントを作成
    pub fn from_arc_provider(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            config: AedConfig::default(),
        }
    }

    /// 設定を指定してクライアントを作成
    pub fn with_config(mut self, config: AedConfig) -> Self {
        self.config = config;
        self
    }

    /// 設定ファイルからクライアントを作成
    ///
    /// 設定ファイルを読み込み、環境変数から自動検出したプロバイダーを使用します。
    pub fn from_config_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = AedConfig::from_file(path)?;
        let mut client = Self::from_env()?;
        client.config = config;
        Ok(client)
    }

    // ========================================
    // Claude 専用コンストラクタ (後方互換性)
    // ========================================

    /// API キーを指定して Claude クライアントを作成
    ///
    /// 後方互換性のために提供されます。
    /// 新しいコードでは `with_provider` の使用を推奨します。
    #[cfg(feature = "claude")]
    pub fn new(api_key: &str) -> Result<Self> {
        use crate::provider::claude::{ClaudeConfig, ClaudeProvider};

        let config = ClaudeConfig::default().with_api_key(api_key);
        let provider = ClaudeProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    // ========================================
    // プロバイダー固有のコンストラクタ
    // ========================================

    /// Claude プロバイダーでクライアントを作成
    #[cfg(feature = "claude")]
    pub fn with_claude(config: crate::provider::claude::ClaudeConfig) -> Result<Self> {
        use crate::provider::claude::ClaudeProvider;

        let provider = ClaudeProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// OpenAI プロバイダーでクライアントを作成
    #[cfg(feature = "openai")]
    pub fn with_openai(config: crate::provider::openai::OpenAIConfig) -> Result<Self> {
        use crate::provider::openai::OpenAIProvider;

        let provider = OpenAIProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// Gemini プロバイダーでクライアントを作成
    #[cfg(feature = "gemini")]
    pub fn with_gemini(config: crate::provider::gemini::GeminiConfig) -> Result<Self> {
        use crate::provider::gemini::GeminiProvider;

        let provider = GeminiProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// xAI プロバイダーでクライアントを作成
    #[cfg(feature = "xai")]
    pub fn with_xai(config: crate::provider::xai::XAIConfig) -> Result<Self> {
        use crate::provider::xai::XAIProvider;

        let provider = XAIProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// Qwen プロバイダーでクライアントを作成
    #[cfg(feature = "qwen")]
    pub fn with_qwen(config: crate::provider::qwen::QwenConfig) -> Result<Self> {
        use crate::provider::qwen::QwenProvider;

        let provider = QwenProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// DeepSeek プロバイダーでクライアントを作成
    #[cfg(feature = "deepseek")]
    pub fn with_deepseek(config: crate::provider::deepseek::DeepSeekConfig) -> Result<Self> {
        use crate::provider::deepseek::DeepSeekProvider;

        let provider = DeepSeekProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// GLM プロバイダーでクライアントを作成
    #[cfg(feature = "glm")]
    pub fn with_glm(config: crate::provider::glm::GlmConfig) -> Result<Self> {
        use crate::provider::glm::GlmProvider;

        let provider = GlmProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    /// LM Studio プロバイダーでクライアントを作成
    #[cfg(feature = "lmstudio")]
    pub fn with_lmstudio(config: crate::provider::local::LmStudioConfig) -> Result<Self> {
        use crate::provider::local::LmStudioProvider;

        let provider = LmStudioProvider::new(config)?;
        Ok(Self::from_provider(provider))
    }

    // ========================================
    // プロバイダー情報取得
    // ========================================

    /// 使用中のプロバイダータイプを取得
    pub fn provider_type(&self) -> ProviderType {
        self.provider.provider_id()
    }

    /// 使用中のモデル名を取得
    pub fn model_name(&self) -> &str {
        self.provider.model_name()
    }

    /// プロバイダーが Vision をサポートしているか
    pub fn supports_vision(&self) -> bool {
        self.provider.supports_vision()
    }

    /// プロバイダーへの参照を取得
    pub fn provider(&self) -> &dyn LlmProvider {
        &*self.provider
    }

    /// 設定への参照を取得
    pub fn config(&self) -> &AedConfig {
        &self.config
    }

    // ========================================
    // 抽出メソッド
    // ========================================

    /// テキスト抽出リクエストを開始
    pub fn extract_text<P: AsRef<Path>>(&self, path: P) -> ExtractTextBuilder<'_> {
        ExtractTextBuilder::new(self, path.as_ref().to_path_buf())
    }

    /// 画像データから直接テキスト抽出
    pub async fn extract_text_from_image(
        &self,
        image: &ImageData,
        prompt: &str,
        direction: TextDirection,
    ) -> Result<ExtractionResult> {
        if !self.supports_vision() {
            return Err(AedError::vision_not_supported(
                self.provider_type().display_name(),
            ));
        }

        debug!(
            "テキスト抽出開始: provider={}, model={}",
            self.provider_type(),
            self.model_name()
        );

        self.provider.extract_text(image, prompt, direction).await
    }

    /// 構造化データ抽出
    ///
    /// 指定した型に従ってドキュメントから構造化データを抽出します。
    pub async fn extract_structured<T>(&self, path: &Path) -> Result<T>
    where
        T: serde::de::DeserializeOwned + schemars::JsonSchema,
    {
        crate::extraction::structured::extract_as(self, path).await
    }

    /// 構造化抽出用 Vision API 呼び出し
    ///
    /// 画像とプロンプトを送信し、テキストレスポンスのみを返します。
    pub async fn call_vision_for_structured(
        &self,
        image_data: &ImageData,
        prompt: &str,
    ) -> Result<String> {
        if !self.supports_vision() {
            return Err(AedError::vision_not_supported(
                self.provider_type().display_name(),
            ));
        }

        let result = self.provider.call_vision_raw(image_data, prompt).await?;
        Ok(result.text)
    }

    /// ヘルスチェック
    ///
    /// プロバイダーが利用可能かどうかを確認します。
    pub async fn health_check(&self) -> Result<bool> {
        self.provider.health_check().await
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
        // Vision サポートチェック
        if !self.client.supports_vision() {
            return Err(AedError::vision_not_supported(
                self.client.provider_type().display_name(),
            ));
        }

        // 画像読み込み（バリデーション含む）
        let image_data = vision::load_image(&self.path)?;

        // プロンプト構築
        let prompt = self.build_prompt();

        // プロバイダー経由で抽出
        self.client
            .provider
            .extract_text(&image_data, &prompt, self.direction)
            .await
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

// ========================================
// プロバイダーファクトリー関数
// ========================================

/// 環境変数からプロバイダーを作成
fn create_provider_from_env(provider_type: ProviderType) -> Result<Arc<dyn LlmProvider>> {
    match provider_type {
        #[cfg(feature = "claude")]
        ProviderType::Claude => {
            use crate::provider::claude::ClaudeProvider;
            let provider = ClaudeProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "openai")]
        ProviderType::OpenAI => {
            use crate::provider::openai::OpenAIProvider;
            let provider = OpenAIProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "gemini")]
        ProviderType::Gemini => {
            use crate::provider::gemini::GeminiProvider;
            let provider = GeminiProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "xai")]
        ProviderType::XAI => {
            use crate::provider::xai::XAIProvider;
            let provider = XAIProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "qwen")]
        ProviderType::Qwen => {
            use crate::provider::qwen::QwenProvider;
            let provider = QwenProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "deepseek")]
        ProviderType::DeepSeek => {
            use crate::provider::deepseek::DeepSeekProvider;
            let provider = DeepSeekProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "glm")]
        ProviderType::GLM => {
            use crate::provider::glm::GlmProvider;
            let provider = GlmProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "lmstudio")]
        ProviderType::LmStudio => {
            use crate::provider::local::LmStudioProvider;
            let provider = LmStudioProvider::from_env()?;
            Ok(Arc::new(provider))
        }

        #[cfg(feature = "hybrid")]
        ProviderType::Hybrid => {
            // Hybrid プロバイダーは Vision バックエンドと LLM の両方が必要
            // デフォルトでは GCP Vision + Claude を使用
            Err(AedError::provider_error(
                "Hybrid",
                "Hybrid プロバイダーは with_hybrid() メソッドで明示的に構成してください",
            ))
        }

        #[allow(unreachable_patterns)]
        _ => Err(AedError::ProviderUnavailable {
            provider: provider_type.display_name(),
            reason: format!(
                "プロバイダー '{}' は現在のビルドでは利用できません。\
                 Cargo.toml で対応する feature を有効にしてください。",
                provider_type
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[test]
    fn test_from_env_no_provider() {
        // 全ての環境変数をクリア
        for provider in crate::provider::available_providers() {
            // SAFETY: テスト環境でのみ使用
            unsafe {
                std::env::remove_var(provider.env_var_name());
            }
        }

        let result = AedClient::from_env();
        assert!(matches!(result, Err(AedError::NoProviderAvailable)));
    }

    #[test]
    fn test_build_vertical_prompt() {
        // テスト用のダミープロバイダー
        struct DummyProvider;

        #[async_trait]
        impl LlmProvider for DummyProvider {
            fn provider_id(&self) -> ProviderType {
                ProviderType::Claude
            }

            fn supports_vision(&self) -> bool {
                true
            }

            fn model_name(&self) -> &str {
                "dummy"
            }

            async fn extract_text(
                &self,
                _image: &ImageData,
                _prompt: &str,
                _direction: TextDirection,
            ) -> Result<ExtractionResult> {
                unimplemented!()
            }

            async fn call_vision_raw(
                &self,
                _image: &ImageData,
                _prompt: &str,
            ) -> Result<crate::provider::RawResponse> {
                unimplemented!()
            }

            async fn extract_structured(
                &self,
                _image: &ImageData,
                _prompt: &str,
                _schema: &str,
            ) -> Result<String> {
                unimplemented!()
            }

            async fn health_check(&self) -> Result<bool> {
                Ok(true)
            }
        }

        let client = AedClient::from_provider(DummyProvider);

        let builder = client
            .extract_text("test.png")
            .direction(TextDirection::Vertical)
            .language("ja");

        let prompt = builder.build_prompt();
        assert!(prompt.contains("縦書き"));
        assert!(prompt.contains("右から左"));
        assert!(prompt.contains("ja"));
    }

    #[test]
    fn test_client_clone() {
        struct DummyProvider;

        #[async_trait]
        impl LlmProvider for DummyProvider {
            fn provider_id(&self) -> ProviderType {
                ProviderType::Claude
            }

            fn supports_vision(&self) -> bool {
                true
            }

            fn model_name(&self) -> &str {
                "dummy"
            }

            async fn extract_text(
                &self,
                _image: &ImageData,
                _prompt: &str,
                _direction: TextDirection,
            ) -> Result<ExtractionResult> {
                unimplemented!()
            }

            async fn call_vision_raw(
                &self,
                _image: &ImageData,
                _prompt: &str,
            ) -> Result<crate::provider::RawResponse> {
                unimplemented!()
            }

            async fn extract_structured(
                &self,
                _image: &ImageData,
                _prompt: &str,
                _schema: &str,
            ) -> Result<String> {
                unimplemented!()
            }

            async fn health_check(&self) -> Result<bool> {
                Ok(true)
            }
        }

        let client = AedClient::from_provider(DummyProvider);
        let cloned = client.clone();

        assert_eq!(client.provider_type(), cloned.provider_type());
    }
}
