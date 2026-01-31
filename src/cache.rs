//! キャッシュ管理
//!
//! API レスポンスのキャッシュを管理します。
//!
//! # Example
//!
//! ```rust,no_run
//! use rust_aed::cache::ResponseCache;
//! use rust_aed::config::CacheConfig;
//! use std::path::Path;
//!
//! # fn example() -> Result<(), rust_aed::AedError> {
//! let config = CacheConfig::default();
//! let cache = ResponseCache::new(&config)?;
//!
//! // キャッシュに保存
//! let key = cache.generate_key(Path::new("image.png"), "prompt text");
//! cache.store(&key, "extracted text content")?;
//!
//! // キャッシュから取得
//! if let Some(cached) = cache.get(&key)? {
//!     println!("キャッシュヒット: {}", cached);
//! }
//! # Ok(())
//! # }
//! ```

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::config::CacheConfig;
use crate::error::Result;

/// レスポンスキャッシュ
pub struct ResponseCache {
    /// キャッシュディレクトリ
    base_dir: PathBuf,
    /// TTL (時間)
    ttl: Duration,
    /// 最大サイズ (バイト) - 将来のサイズ制限機能用
    #[allow(dead_code)]
    max_size: u64,
    /// 有効フラグ
    enabled: bool,
}

impl ResponseCache {
    /// 新しいキャッシュを作成
    pub fn new(config: &CacheConfig) -> Result<Self> {
        let base_dir = PathBuf::from(&config.directory);

        if config.enabled {
            fs::create_dir_all(&base_dir)?;
        }

        Ok(Self {
            base_dir,
            ttl: Duration::from_secs(config.ttl_hours as u64 * 3600),
            max_size: config.max_size,
            enabled: config.enabled,
        })
    }

    /// キャッシュキーを生成
    ///
    /// 画像パスとプロンプトから SHA-256 ハッシュを生成します。
    pub fn generate_key(&self, image_path: &Path, prompt: &str) -> String {
        let mut hasher = Sha256::new();

        // ファイルパス（正規化）
        if let Ok(canonical) = image_path.canonicalize() {
            hasher.update(canonical.to_string_lossy().as_bytes());
        } else {
            hasher.update(image_path.to_string_lossy().as_bytes());
        }

        // プロンプト
        hasher.update(prompt.as_bytes());

        // ファイルの更新時刻
        if let Ok(metadata) = fs::metadata(image_path)
            && let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH)
        {
            hasher.update(duration.as_secs().to_le_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// キャッシュからデータを取得
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        if !self.enabled {
            return Ok(None);
        }

        let cache_path = self.cache_path(key);

        if !cache_path.exists() {
            return Ok(None);
        }

        // TTL チェック
        if self.is_expired(&cache_path)? {
            tracing::debug!("キャッシュ期限切れ: {}", key);
            fs::remove_file(&cache_path)?;
            return Ok(None);
        }

        // ファイル読み込み
        let mut file = fs::File::open(&cache_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        tracing::debug!("キャッシュヒット: {}", key);
        Ok(Some(content))
    }

    /// キャッシュにデータを保存
    pub fn store(&self, key: &str, data: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let cache_path = self.cache_path(key);

        // ディレクトリ作成
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // ファイル書き込み
        let mut file = fs::File::create(&cache_path)?;
        file.write_all(data.as_bytes())?;

        tracing::debug!("キャッシュ保存: {}", key);
        Ok(())
    }

    /// キャッシュを削除
    pub fn remove(&self, key: &str) -> Result<()> {
        let cache_path = self.cache_path(key);

        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
        }

        Ok(())
    }

    /// 全キャッシュをクリア
    pub fn clear(&self) -> Result<()> {
        if self.base_dir.exists() {
            fs::remove_dir_all(&self.base_dir)?;
            fs::create_dir_all(&self.base_dir)?;
        }
        Ok(())
    }

    /// 期限切れキャッシュを削除
    pub fn cleanup(&self) -> Result<CleanupStats> {
        let mut stats = CleanupStats::default();

        if !self.base_dir.exists() {
            return Ok(stats);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                stats.total_files += 1;
                stats.total_size += entry.metadata()?.len();

                if self.is_expired(&path)? {
                    fs::remove_file(&path)?;
                    stats.removed_files += 1;
                }
            }
        }

        Ok(stats)
    }

    /// キャッシュサイズを取得
    pub fn size(&self) -> Result<u64> {
        if !self.base_dir.exists() {
            return Ok(0);
        }

        // 再帰的にサイズを計算
        fn dir_size(path: &Path) -> std::io::Result<u64> {
            let mut total = 0u64;
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        total += dir_size(&path)?;
                    } else {
                        total += entry.metadata()?.len();
                    }
                }
            }
            Ok(total)
        }

        Ok(dir_size(&self.base_dir)?)
    }

    /// キャッシュファイルパスを取得
    fn cache_path(&self, key: &str) -> PathBuf {
        // キーの最初の2文字をサブディレクトリとして使用
        let subdir = &key[..2.min(key.len())];
        self.base_dir.join(subdir).join(format!("{}.cache", key))
    }

    /// 期限切れかどうかを判定
    fn is_expired(&self, path: &Path) -> Result<bool> {
        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?;

        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);

        Ok(age > self.ttl)
    }
}

/// クリーンアップ統計
#[derive(Debug, Default)]
pub struct CleanupStats {
    /// 処理したファイル数
    pub total_files: usize,
    /// 削除したファイル数
    pub removed_files: usize,
    /// 合計サイズ（削除前）
    pub total_size: u64,
}

/// キャッシュ付きクライアント拡張
pub trait CacheExt {
    /// キャッシュを使用してテキスト抽出
    fn extract_text_cached<'a>(
        &'a self,
        path: &'a Path,
        cache: &'a ResponseCache,
    ) -> CachedExtractBuilder<'a>;
}

/// キャッシュ付き抽出ビルダー
#[allow(dead_code)]
pub struct CachedExtractBuilder<'a> {
    client: &'a crate::client::AedClient,
    path: &'a Path,
    cache: &'a ResponseCache,
    prompt: String,
}

impl<'a> CachedExtractBuilder<'a> {
    /// プロンプトを設定
    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }
}

impl CacheExt for crate::client::AedClient {
    fn extract_text_cached<'a>(
        &'a self,
        path: &'a Path,
        cache: &'a ResponseCache,
    ) -> CachedExtractBuilder<'a> {
        CachedExtractBuilder {
            client: self,
            path,
            cache,
            prompt: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_cache() -> (ResponseCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            enabled: true,
            directory: temp_dir.path().to_string_lossy().to_string(),
            ttl_hours: 24,
            max_size: 1024 * 1024,
        };
        let cache = ResponseCache::new(&config).unwrap();
        (cache, temp_dir)
    }

    #[test]
    fn test_generate_key() {
        let (cache, _temp) = create_test_cache();
        let key1 = cache.generate_key(Path::new("test.png"), "prompt1");
        let key2 = cache.generate_key(Path::new("test.png"), "prompt2");
        let key3 = cache.generate_key(Path::new("other.png"), "prompt1");

        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_store_and_get() {
        let (cache, _temp) = create_test_cache();
        let key = "test_key_abc123";
        let data = "test data content";

        cache.store(key, data).unwrap();
        let retrieved = cache.get(key).unwrap();

        assert_eq!(retrieved, Some(data.to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let (cache, _temp) = create_test_cache();
        let result = cache.get("nonexistent_key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_remove() {
        let (cache, _temp) = create_test_cache();
        let key = "remove_test_key";

        cache.store(key, "data").unwrap();
        assert!(cache.get(key).unwrap().is_some());

        cache.remove(key).unwrap();
        assert!(cache.get(key).unwrap().is_none());
    }

    #[test]
    fn test_clear() {
        let (cache, _temp) = create_test_cache();

        cache.store("key1", "data1").unwrap();
        cache.store("key2", "data2").unwrap();

        cache.clear().unwrap();

        assert!(cache.get("key1").unwrap().is_none());
        assert!(cache.get("key2").unwrap().is_none());
    }

    #[test]
    fn test_disabled_cache() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            enabled: false,
            directory: temp_dir.path().to_string_lossy().to_string(),
            ttl_hours: 24,
            max_size: 1024 * 1024,
        };
        let cache = ResponseCache::new(&config).unwrap();

        cache.store("key", "data").unwrap();
        let result = cache.get("key").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_size() {
        let (cache, _temp) = create_test_cache();

        cache.store("key1", "short").unwrap();
        cache.store("key2", "a longer piece of content").unwrap();

        let size = cache.size().unwrap();
        assert!(size > 0);
    }
}
