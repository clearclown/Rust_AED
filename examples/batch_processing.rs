//! バッチ処理サンプル
//!
//! 複数ファイルを並列処理する例です。
//!
//! # 実行方法
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example batch_processing -- images/*.png
//! ```

use rust_aed::{AedClient, OcrPreset};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("使用方法: {} <画像ファイル...>", args[0]);
        eprintln!("例: {} page1.png page2.png page3.png", args[0]);
        std::process::exit(1);
    }

    let image_paths: Vec<PathBuf> = args[1..].iter().map(PathBuf::from).collect();
    println!("{}個のファイルを処理します", image_paths.len());

    let client = AedClient::from_env()?;
    let start = std::time::Instant::now();

    // 並列処理
    let mut handles = Vec::new();

    for path in image_paths {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let result = client_clone
                .extract_text(&path)
                .preset(OcrPreset::General)
                .await;

            (path, result)
        });
        handles.push(handle);
    }

    // 結果収集
    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await {
            Ok((path, Ok(result))) => {
                println!("\n✓ {}", path.display());
                println!("  文字数: {}", result.text.len());
                println!("  信頼度: {:.2}%", result.confidence * 100.0);
                success_count += 1;
            }
            Ok((path, Err(e))) => {
                println!("\n✗ {} - エラー: {}", path.display(), e);
                error_count += 1;
            }
            Err(e) => {
                println!("\n✗ タスクエラー: {}", e);
                error_count += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    println!("\n=== 処理完了 ===");
    println!("成功: {}", success_count);
    println!("失敗: {}", error_count);
    println!("処理時間: {:?}", elapsed);
    println!(
        "平均: {:?}/ファイル",
        elapsed / (success_count + error_count) as u32
    );

    Ok(())
}

