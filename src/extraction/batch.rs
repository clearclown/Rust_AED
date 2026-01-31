//! バッチ処理
//!
//! 複数ファイルを並列処理します。
//!
//! # Example
//!
//! ```rust,no_run
//! use rust_aed::AedClient;
//! use rust_aed::extraction::batch::batch_extract;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), rust_aed::AedError> {
//! let client = AedClient::from_env()?;
//! let paths = vec![
//!     PathBuf::from("doc1.png"),
//!     PathBuf::from("doc2.png"),
//!     PathBuf::from("doc3.png"),
//! ];
//!
//! let result = batch_extract(&client, paths, 5).await;
//! println!("成功: {}, 失敗: {}", result.successful.len(), result.failed.len());
//! # Ok(())
//! # }
//! ```

use std::path::PathBuf;
use std::time::Instant;

use futures::stream::{self, StreamExt};

use crate::client::AedClient;
use crate::error::{AedError, Result};
use crate::presets::OcrPreset;
use crate::types::{BatchResult, ExtractionResult, TextDirection, TokenUsage};

/// バッチ抽出を実行
///
/// 複数の画像ファイルを並列処理してテキスト抽出します。
///
/// # Arguments
///
/// * `client` - AED クライアント
/// * `paths` - 処理するファイルパスのリスト
/// * `concurrency` - 同時処理数（推奨: 5-10）
///
/// # Returns
///
/// 成功/失敗を含む `BatchResult`
pub async fn batch_extract(
    client: &AedClient,
    paths: Vec<PathBuf>,
    concurrency: usize,
) -> BatchResult {
    let start_time = Instant::now();
    let total_count = paths.len();

    tracing::info!("バッチ処理開始: {} ファイル, 並列度: {}", total_count, concurrency);

    // 並列処理
    let results: Vec<(PathBuf, Result<ExtractionResult>)> = stream::iter(paths)
        .map(|path| {
            let client = client.clone();
            async move {
                let result = client.extract_text(&path).await;
                (path, result)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    // 結果を分類
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    let mut total_tokens = TokenUsage::default();

    for (path, result) in results {
        match result {
            Ok(extraction) => {
                total_tokens.input_tokens += extraction.tokens_used.input_tokens;
                total_tokens.output_tokens += extraction.tokens_used.output_tokens;
                successful.push(extraction);
            }
            Err(e) => {
                tracing::warn!("ファイル {} の処理に失敗: {}", path.display(), e);
                failed.push((path, e));
            }
        }
    }

    let total_time = start_time.elapsed();

    tracing::info!(
        "バッチ処理完了: 成功 {}/{}, 所要時間 {:?}",
        successful.len(),
        total_count,
        total_time
    );

    BatchResult {
        successful,
        failed,
        total_time,
        total_tokens,
    }
}

/// バッチ抽出ビルダー
pub struct BatchExtractBuilder<'a> {
    client: &'a AedClient,
    paths: Vec<PathBuf>,
    concurrency: usize,
    preset: Option<OcrPreset>,
    direction: TextDirection,
    language: Option<String>,
    on_progress: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
}

impl<'a> BatchExtractBuilder<'a> {
    /// 新しいビルダーを作成
    pub fn new(client: &'a AedClient, paths: Vec<PathBuf>) -> Self {
        Self {
            client,
            paths,
            concurrency: 5,
            preset: None,
            direction: TextDirection::Auto,
            language: None,
            on_progress: None,
        }
    }

    /// 並列度を設定
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.clamp(1, 20); // 1-20 に制限
        self
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

    /// 言語を設定
    pub fn language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }

    /// 進捗コールバックを設定
    pub fn on_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, usize) + Send + Sync + 'static,
    {
        self.on_progress = Some(Box::new(callback));
        self
    }

    /// 抽出を実行
    pub async fn execute(self) -> BatchResult {
        let start_time = Instant::now();
        let total_count = self.paths.len();
        let processed = std::sync::atomic::AtomicUsize::new(0);

        tracing::info!(
            "バッチ処理開始: {} ファイル, 並列度: {}, プリセット: {:?}",
            total_count,
            self.concurrency,
            self.preset.as_ref().map(|p| p.display_name_ja())
        );

        let results: Vec<(PathBuf, Result<ExtractionResult>)> = stream::iter(self.paths)
            .map(|path| {
                let client = self.client.clone();
                let preset = self.preset.clone();
                let direction = self.direction;
                let language = self.language.clone();
                let on_progress = &self.on_progress;
                let processed = &processed;
                let total = total_count;

                async move {
                    let mut builder = client.extract_text(&path)
                        .direction(direction);

                    if let Some(p) = preset {
                        builder = builder.preset(p);
                    }
                    if let Some(ref lang) = language {
                        builder = builder.language(lang);
                    }

                    let result = builder.await;

                    // 進捗通知
                    let current = processed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if let Some(callback) = on_progress {
                        callback(current, total);
                    }

                    (path, result)
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        // 結果を分類
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut total_tokens = TokenUsage::default();

        for (path, result) in results {
            match result {
                Ok(extraction) => {
                    total_tokens.input_tokens += extraction.tokens_used.input_tokens;
                    total_tokens.output_tokens += extraction.tokens_used.output_tokens;
                    successful.push(extraction);
                }
                Err(e) => {
                    failed.push((path, e));
                }
            }
        }

        BatchResult {
            successful,
            failed,
            total_time: start_time.elapsed(),
            total_tokens,
        }
    }
}

/// ディレクトリ内の画像を一括抽出
///
/// 指定されたディレクトリ内のサポートされている画像ファイルをすべて処理します。
pub async fn batch_extract_dir(
    client: &AedClient,
    dir: &std::path::Path,
    concurrency: usize,
) -> Result<BatchResult> {
    if !dir.is_dir() {
        return Err(AedError::FileNotFound(dir.to_path_buf()));
    }

    let supported_extensions = ["png", "jpg", "jpeg", "gif", "webp"];
    let mut paths = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && supported_extensions.contains(&ext.to_lowercase().as_str())
        {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        tracing::warn!("ディレクトリ {} に画像ファイルが見つかりません", dir.display());
    }

    // ファイル名でソート
    paths.sort();

    Ok(batch_extract(client, paths, concurrency).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_result_success_rate() {
        use std::time::Duration;

        let result = BatchResult {
            successful: vec![],
            failed: vec![],
            total_time: Duration::from_secs(1),
            total_tokens: TokenUsage::default(),
        };
        assert_eq!(result.success_rate(), 0.0);

        // 空でない結果のテストは実際のデータが必要
    }

    #[test]
    fn test_builder_concurrency_limits() {
        let client = AedClient::new("test-key").unwrap();
        let builder = BatchExtractBuilder::new(&client, vec![])
            .concurrency(100);
        assert_eq!(builder.concurrency, 20); // 上限

        let builder = BatchExtractBuilder::new(&client, vec![])
            .concurrency(0);
        assert_eq!(builder.concurrency, 1); // 下限
    }
}
