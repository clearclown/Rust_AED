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
    #[error("API キーが設定されていません。環境変数 ANTHROPIC_API_KEY を設定してください")]
    MissingApiKey,

    /// API キーが無効です
    #[error("API キーが無効です")]
    InvalidApiKey,

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
            AedError::MissingApiKey
                | AedError::InvalidApiKey
                | AedError::UnsupportedFormat(_)
                | AedError::FileTooLarge { .. }
                | AedError::FileNotFound(_)
        )
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

        assert!(!AedError::MissingApiKey.is_retryable());
    }

    #[test]
    fn test_is_client_error() {
        assert!(AedError::MissingApiKey.is_client_error());
        assert!(AedError::InvalidApiKey.is_client_error());
        assert!(AedError::FileNotFound(PathBuf::from("/test")).is_client_error());

        assert!(!AedError::RateLimited {
            retry_after: Duration::from_secs(60)
        }
        .is_client_error());
    }

    #[test]
    fn test_error_display() {
        let err = AedError::MissingApiKey;
        assert!(err.to_string().contains("ANTHROPIC_API_KEY"));

        let err = AedError::FileTooLarge {
            size: 100_000_000,
            max: 32_000_000,
        };
        assert!(err.to_string().contains("100000000"));
    }
}
