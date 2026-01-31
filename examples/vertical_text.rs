//! 縦書きテキスト抽出サンプル
//!
//! 日本語の縦書きテキストを抽出する例です。
//!
//! # 実行方法
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example vertical_text -- novel.png
//! ```

use rust_aed::{AedClient, OcrPreset, TextDirection};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("使用方法: {} <縦書き画像>", args[0]);
        std::process::exit(1);
    }

    let image_path = &args[1];
    println!("縦書き画像を読み込み中: {}", image_path);

    let client = AedClient::from_env()?;

    // 日本語書籍プリセット + 縦書き指定
    let result = client
        .extract_text(image_path)
        .preset(OcrPreset::JapaneseBook)
        .direction(TextDirection::Vertical)
        .language("ja")
        .await?;

    println!("\n=== 縦書きテキスト抽出結果 ===\n");
    println!("{}", result.text);
    println!("\n--- 統計 ---");
    println!("検出方向: {}", result.direction.display_name_ja());
    println!("信頼度: {:.2}%", result.confidence * 100.0);

    Ok(())
}
