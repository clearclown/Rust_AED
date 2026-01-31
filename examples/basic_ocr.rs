//! 基本的な OCR サンプル
//!
//! 画像からテキストを抽出する基本的な例です。
//!
//! # 実行方法
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example basic_ocr -- image.png
//! ```

use rust_aed::{AedClient, OcrPreset};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ログ初期化
    tracing_subscriber::fmt::init();

    // コマンドライン引数から画像パスを取得
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("使用方法: {} <画像ファイル>", args[0]);
        eprintln!("例: {} document.png", args[0]);
        std::process::exit(1);
    }

    let image_path = &args[1];
    println!("画像を読み込み中: {}", image_path);

    // クライアント初期化
    let client = AedClient::from_env()?;

    // テキスト抽出
    println!("テキストを抽出中...");
    let result = client
        .extract_text(image_path)
        .preset(OcrPreset::General)
        .await?;

    // 結果表示
    println!("\n=== 抽出結果 ===");
    println!("テキスト:\n{}", result.text);
    println!("\n--- メタデータ ---");
    println!("信頼度: {:.2}%", result.confidence * 100.0);
    println!("言語: {}", result.language);
    println!("方向: {}", result.direction.display_name_ja());
    println!("処理時間: {:?}", result.processing_time);
    println!(
        "トークン使用量: {} (入力: {}, 出力: {})",
        result.tokens_used.total(),
        result.tokens_used.input_tokens,
        result.tokens_used.output_tokens
    );
    println!(
        "推定コスト: ${:.6}",
        result.tokens_used.estimated_cost_usd()
    );

    Ok(())
}
