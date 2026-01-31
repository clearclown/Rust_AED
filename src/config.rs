//! 設定管理
//!
//! Rust AED の設定を管理します。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AedError, Result};

/// AED 設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AedConfig {
    /// API キー
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// 使用するモデル
    #[serde(default = "default_model")]
    pub model: String,

    /// 最大トークン数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// タイムアウト（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// キャッシュ設定
    #[serde(default)]
    pub cache: CacheConfig,

    /// リトライ設定
    #[serde(default)]
    pub retry: RetryConfig,
}

fn default_model() -> String {
    "claude-sonnet-4-5".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout() -> u64 {
    120
}

impl Default for AedConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout(),
            cache: CacheConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}

impl AedConfig {
    /// 設定ファイルから読み込み
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        toml::from_str(&content)
            .map_err(|e| AedError::ConfigParseError(e.to_string()))
    }

    /// API キーを設定
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = Some(api_key.to_string());
        self
    }

    /// モデルを設定
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// 最大トークン数を設定
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// タイムアウトを設定
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

/// キャッシュ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// キャッシュ有効化
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    /// キャッシュディレクトリ
    #[serde(default = "default_cache_dir")]
    pub directory: String,

    /// TTL（時間）
    #[serde(default = "default_cache_ttl")]
    pub ttl_hours: u32,

    /// 最大サイズ（バイト）
    #[serde(default = "default_cache_max_size")]
    pub max_size: u64,
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_dir() -> String {
    ".aed_cache".to_string()
}

fn default_cache_ttl() -> u32 {
    24
}

fn default_cache_max_size() -> u64 {
    1024 * 1024 * 1024 // 1GB
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_cache_enabled(),
            directory: default_cache_dir(),
            ttl_hours: default_cache_ttl(),
            max_size: default_cache_max_size(),
        }
    }
}

/// リトライ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大リトライ回数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// 初期待機時間（ミリ秒）
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// 最大待機時間（ミリ秒）
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,

    /// バックオフ係数
    #[serde(default = "default_backoff_factor")]
    pub backoff_factor: f64,
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_delay() -> u64 {
    1000
}

fn default_max_delay() -> u64 {
    60000
}

fn default_backoff_factor() -> f64 {
    2.0
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            backoff_factor: default_backoff_factor(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AedConfig::default();

        assert_eq!(config.model, "claude-sonnet-4-5");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout_secs, 120);
        assert!(config.cache.enabled);
    }

    #[test]
    fn test_with_api_key() {
        let config = AedConfig::default().with_api_key("test-key");

        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_config_parse() {
        let toml_content = r#"
            model = "claude-haiku-3"
            max_tokens = 2048
            timeout_secs = 60

            [cache]
            enabled = false

            [retry]
            max_retries = 5
        "#;

        let config: AedConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.model, "claude-haiku-3");
        assert_eq!(config.max_tokens, 2048);
        assert!(!config.cache.enabled);
        assert_eq!(config.retry.max_retries, 5);
    }
}
