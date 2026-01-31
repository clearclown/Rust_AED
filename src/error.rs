//! エラー型定義
//!
//! Rust AED で発生する可能性のあるエラーを定義します。

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Rust AED のエラー型
#[derive(Debug, Error)]
pub enum AedError {
    // ============================================================
    // 認証エラー
    // ============================================================
    /// API キーが設定されていません
    #[error("API キーが設定されていません。環境変数 {env_var} を設定してください")]
    MissingApiKey {
        /// 必要な環境変数名
        env_var: &'static str,
    },

    /// API キーが無効です
    #[error("{provider} の API キーが無効です")]
    InvalidApiKey {
        /// プロバイダー名
        provider: &'static str,
    },

    // ============================================================
    // プロバイダーエラー
    // ============================================================
    /// プロバイダーが利用できません
    #[error("プロバイダー {provider} は利用できません: {reason}")]
    ProviderUnavailable {
        /// プロバイダー名
        provider: &'static str,
        /// 理由
        reason: String,
    },

    /// プロバイダーが Vision をサポートしていません
    #[error("プロバイダー {provider} は Vision をサポートしていません")]
    VisionNotSupported {
        /// プロバイダー名
        provider: &'static str,
    },

    /// プロバイダー固有のエラー
    #[error("{provider} エラー: {message}")]
    ProviderError {
        /// プロバイダー名
        provider: &'static str,
        /// エラーメッセージ
        message: String,
        /// 元のエラーコード（あれば）
        code: Option<String>,
    },

    /// 利用可能なプロバイダーがありません
    #[error("利用可能なプロバイダーが見つかりません。API キーを設定してください")]
    NoProviderAvailable,

    // ============================================================
    // API エラー
    // ============================================================
    /// API からエラーレスポンスを受信
    #[error("API エラー: {status} - {message}")]
    ApiError {
        /// HTTP ステータスコード
        status: u16,
        /// エラーメッセージ
        message: String,
    },

    /// レート制限に達しました
    #[error("レート制限に達しました。{retry_after:?} 後に再試行してください")]
    RateLimited {
        /// 再試行までの待機時間
        retry_after: Duration,
    },

    /// リクエストタイムアウト
    #[error("リクエストがタイムアウトしました（{timeout:?}）")]
    Timeout {
        /// タイムアウト時間
        timeout: Duration,
    },

    // ============================================================
    // 入力エラー
    // ============================================================
    /// サポートされていないファイル形式
    #[error("サポートされていないファイル形式です: {0}")]
    UnsupportedFormat(String),

    /// ファイルサイズが上限を超えています
    #[error("ファイルサイズが上限を超えています: {size} バイト（上限: {max} バイト）")]
    FileTooLarge {
        /// 実際のファイルサイズ
        size: u64,
        /// 上限サイズ
        max: u64,
    },

    /// ページ数が上限を超えています
    #[error("ページ数が上限を超えています: {pages} ページ（上限: {max} ページ）")]
    PageLimitExceeded {
        /// 実際のページ数
        pages: u32,
        /// 上限ページ数
        max: u32,
    },

    /// 画像が小さすぎます
    #[error("画像が小さすぎます: {width}x{height}（最小: 200x200）")]
    ImageTooSmall {
        /// 画像の幅
        width: u32,
        /// 画像の高さ
        height: u32,
    },

    // ============================================================
    // 処理エラー
    // ============================================================
    /// テキスト抽出に失敗しました
    #[error("テキスト抽出に失敗しました: {0}")]
    ExtractionFailed(String),

    /// JSON パースに失敗しました
    #[error("JSON パースに失敗しました: {0}")]
    JsonParseError(#[from] serde_json::Error),

    /// Base64 エンコードに失敗しました
    #[error("Base64 エンコードに失敗しました: {0}")]
    Base64Error(String),

    // ============================================================
    // I/O エラー
    // ============================================================
    /// ファイルが見つかりません
    #[error("ファイルが見つかりません: {0}")]
    FileNotFound(PathBuf),

    /// I/O エラー
    #[error("I/O エラー: {0}")]
    IoError(#[from] std::io::Error),

    /// HTTP クライアントエラー
    #[error("HTTP エラー: {0}")]
    HttpError(#[from] reqwest::Error),

    // ============================================================
    // 設定エラー
    // ============================================================
    /// 設定ファイルのパースに失敗しました
    #[error("設定ファイルのパースに失敗しました: {0}")]
    ConfigParseError(String),

    /// 無効な設定値
    #[error("無効な設定値: {key} = {value}")]
    InvalidConfigValue {
        /// 設定キー
        key: String,
        /// 設定値
        value: String,
    },
}

/// Result 型エイリアス
pub type Result<T> = std::result::Result<T, AedError>;

impl AedError {
    /// リトライ可能なエラーかどうかを判定
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AedError::RateLimited { .. }
                | AedError::Timeout { .. }
                | AedError::ApiError { status: 500..=599, .. }
        )
    }

    /// クライアント側のエラーかどうかを判定
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            AedError::MissingApiKey { .. }
                | AedError::InvalidApiKey { .. }
                | AedError::UnsupportedFormat(_)
                | AedError::FileTooLarge { .. }
                | AedError::FileNotFound(_)
        )
    }

    /// プロバイダー関連のエラーかどうかを判定
    pub fn is_provider_error(&self) -> bool {
        matches!(
            self,
            AedError::ProviderUnavailable { .. }
                | AedError::VisionNotSupported { .. }
                | AedError::ProviderError { .. }
                | AedError::NoProviderAvailable
        )
    }

    /// MissingApiKey エラーを作成するヘルパー
    pub fn missing_api_key(env_var: &'static str) -> Self {
        AedError::MissingApiKey { env_var }
    }

    /// InvalidApiKey エラーを作成するヘルパー
    pub fn invalid_api_key(provider: &'static str) -> Self {
        AedError::InvalidApiKey { provider }
    }

    /// ProviderError エラーを作成するヘルパー
    pub fn provider_error(provider: &'static str, message: impl Into<String>) -> Self {
        AedError::ProviderError {
            provider,
            message: message.into(),
            code: None,
        }
    }

    /// ProviderError エラーをコード付きで作成するヘルパー
    pub fn provider_error_with_code(
        provider: &'static str,
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        AedError::ProviderError {
            provider,
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// VisionNotSupported エラーを作成するヘルパー
    pub fn vision_not_supported(provider: &'static str) -> Self {
        AedError::VisionNotSupported { provider }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable() {
        assert!(AedError::RateLimited {
            retry_after: Duration::from_secs(60)
        }
        .is_retryable());

        assert!(AedError::Timeout {
            timeout: Duration::from_secs(30)
        }
        .is_retryable());

        assert!(AedError::ApiError {
            status: 503,
            message: "Service Unavailable".to_string()
        }
        .is_retryable());

        assert!(!AedError::missing_api_key("TEST_KEY").is_retryable());
    }

    #[test]
    fn test_is_client_error() {
        assert!(AedError::missing_api_key("TEST_KEY").is_client_error());
        assert!(AedError::invalid_api_key("TestProvider").is_client_error());
        assert!(AedError::FileNotFound(PathBuf::from("/test")).is_client_error());

        assert!(!AedError::RateLimited {
            retry_after: Duration::from_secs(60)
        }
        .is_client_error());
    }

    #[test]
    fn test_is_provider_error() {
        assert!(AedError::NoProviderAvailable.is_provider_error());
        assert!(AedError::ProviderUnavailable {
            provider: "Test",
            reason: "Test reason".to_string()
        }
        .is_provider_error());
        assert!(AedError::VisionNotSupported { provider: "Test" }.is_provider_error());
    }

    #[test]
    fn test_error_display() {
        let err = AedError::missing_api_key("ANTHROPIC_API_KEY");
        assert!(err.to_string().contains("ANTHROPIC_API_KEY"));

        let err = AedError::FileTooLarge {
            size: 100_000_000,
            max: 32_000_000,
        };
        assert!(err.to_string().contains("100000000"));

        let err = AedError::provider_error("Claude", "Test error");
        assert!(err.to_string().contains("Claude"));
        assert!(err.to_string().contains("Test error"));
    }

    #[test]
    fn test_provider_error_helpers() {
        let err = AedError::provider_error_with_code("OpenAI", "Rate limited", "rate_limit_exceeded");
        match err {
            AedError::ProviderError { provider, message, code } => {
                assert_eq!(provider, "OpenAI");
                assert_eq!(message, "Rate limited");
                assert_eq!(code, Some("rate_limit_exceeded".to_string()));
            }
            _ => panic!("Expected ProviderError"),
        }
    }
}
