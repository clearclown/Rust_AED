//! バッチ処理サンプル（出力ファイル保存版）
//!
//! 指定ディレクトリ内の画像を処理し、テキストファイルとして保存します。
//!
//! # 実行方法
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example batch_to_output -- <input_dir> [output_dir]
//! ```

use rust_aed::extraction::batch::BatchExtractBuilder;
use rust_aed::{AedClient, OcrPreset};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ログ初期化
    tracing_subscriber::fmt()
        .with_env_filter("rust_aed=info")
        .init();

    // コマンドライン引数から入力・出力ディレクトリを取得
    let args: Vec<String> = std::env::args().collect();
    let input_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("examples/pics")
    };
    let output_dir = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from("examples/outputs")
    };

    // 出力ディレクトリ作成
    std::fs::create_dir_all(&output_dir)?;

    // 画像ファイル収集
    let supported_extensions = ["png", "jpg", "jpeg", "gif", "webp"];
    let mut paths = Vec::new();

    for entry in std::fs::read_dir(&input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && supported_extensions.contains(&ext.to_lowercase().as_str())
        {
            paths.push(path);
        }
    }

    paths.sort();
    println!("処理対象ファイル: {} 件", paths.len());
    for path in &paths {
        println!("  - {}", path.display());
    }

    if paths.is_empty() {
        println!("処理するファイルがありません");
        return Ok(());
    }

    // クライアント初期化
    let client = AedClient::from_env()?;

    println!("\nバッチ処理を開始します...\n");

    // バッチ処理（並列度 3）
    let result = BatchExtractBuilder::new(&client, paths.clone())
        .concurrency(3)
        .preset(OcrPreset::JapaneseBook)
        .on_progress(|current, total| {
            println!("進捗: {}/{}", current, total);
        })
        .execute()
        .await;

    // 結果を保存
    println!("\n=== 結果を保存中 ===\n");

    for (i, extraction) in result.successful.iter().enumerate() {
        // 元のファイル名を取得
        let fallback_name = format!("{:04}", i);
        let original_name = paths.get(i)
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or(&fallback_name);

        let output_path = output_dir.join(format!("{}.txt", original_name));
        std::fs::write(&output_path, &extraction.text)?;
        println!("保存: {} ({} 文字)", output_path.display(), extraction.text.len());
    }

    // サマリー
    println!("\n=== 処理完了 ===");
    println!(
        "成功: {}/{} 件",
        result.successful.len(),
        result.successful.len() + result.failed.len()
    );
    println!("所要時間: {:?}", result.total_time);
    println!(
        "合計トークン: {} (入力: {}, 出力: {})",
        result.total_tokens.total(),
        result.total_tokens.input_tokens,
        result.total_tokens.output_tokens
    );
    println!(
        "推定コスト: ${:.4}",
        result.total_tokens.estimated_cost_usd()
    );

    if !result.failed.is_empty() {
        println!("\n=== 失敗したファイル ===");
        for (path, err) in &result.failed {
            println!("  {} : {}", path.display(), err);
        }
    }

    Ok(())
}
