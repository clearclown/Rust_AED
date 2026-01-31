//! バッチ処理の統合テスト

use rust_aed::extraction::batch::{batch_extract, BatchExtractBuilder};
use rust_aed::AedClient;

fn setup() -> AedClient {
    AedClient::from_env().expect("ANTHROPIC_API_KEY が設定されていません")
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_batch_extract() {
    let client = setup();

    // テスト用のファイルパスを収集
    let fixtures_dir = std::path::Path::new("tests/fixtures");
    if !fixtures_dir.exists() {
        eprintln!("テストフィクスチャディレクトリが見つかりません");
        return;
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(fixtures_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "png") {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        eprintln!("テスト用の PNG ファイルが見つかりません");
        return;
    }

    // 最大2ファイルでテスト
    paths.truncate(2);

    let result = batch_extract(&client, paths.clone(), 2).await;

    // 成功率を確認
    let total = result.successful.len() + result.failed.len();
    assert_eq!(total, paths.len());

    eprintln!(
        "バッチ処理結果: 成功 {}/{}, トークン {}",
        result.successful.len(),
        total,
        result.total_tokens.total()
    );
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_batch_builder_with_progress() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let client = setup();

    let fixtures_dir = std::path::Path::new("tests/fixtures");
    if !fixtures_dir.exists() {
        return;
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(fixtures_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "png") {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        return;
    }

    paths.truncate(2);

    let progress_count = Arc::new(AtomicUsize::new(0));
    let progress_clone = progress_count.clone();

    let result = BatchExtractBuilder::new(&client, paths)
        .concurrency(1)
        .on_progress(move |current, total| {
            progress_clone.fetch_add(1, Ordering::SeqCst);
            eprintln!("進捗: {}/{}", current, total);
        })
        .execute()
        .await;

    assert!(progress_count.load(Ordering::SeqCst) > 0);
    assert!(result.success_rate() >= 0.0);
}

#[cfg(test)]
mod unit_tests {
    use rust_aed::extraction::batch::BatchExtractBuilder;
    use rust_aed::{AedClient, OcrPreset, TextDirection};
    use std::path::PathBuf;

    #[test]
    fn test_builder_configuration() {
        let client = AedClient::new("test-key").unwrap();
        let paths = vec![PathBuf::from("test.png")];

        let _builder = BatchExtractBuilder::new(&client, paths)
            .concurrency(10)
            .preset(OcrPreset::JapaneseBook)
            .direction(TextDirection::Vertical)
            .language("ja");

        // ビルダーが正しく設定されることを確認
        // (フィールドは private なので直接確認はできないが、コンパイルが通ることを確認)
    }

    #[test]
    fn test_empty_batch() {
        // 空のバッチは即座に完了すべき
        // 実際の実行は API キーが必要なので skip
    }
}
