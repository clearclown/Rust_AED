//! 請求書抽出サンプル
//!
//! 請求書から構造化データを抽出する例です。
//!
//! # 実行方法
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example invoice_extraction -- invoice.png
//! ```

use rust_aed::AedClient;
use schemars::JsonSchema;
use serde::Deserialize;
use std::env;

/// 請求書データ
#[derive(Debug, Deserialize, JsonSchema)]
struct Invoice {
    /// 請求書番号
    #[serde(rename = "請求書番号")]
    invoice_number: String,

    /// 発行日
    #[serde(rename = "発行日")]
    issue_date: String,

    /// 発行元会社名
    #[serde(rename = "発行元")]
    issuer: Company,

    /// 請求先会社名
    #[serde(rename = "請求先")]
    recipient: Company,

    /// 明細
    #[serde(rename = "明細")]
    items: Vec<InvoiceItem>,

    /// 小計
    #[serde(rename = "小計")]
    subtotal: i64,

    /// 消費税
    #[serde(rename = "消費税")]
    tax: i64,

    /// 合計
    #[serde(rename = "合計")]
    total: i64,
}

/// 会社情報
#[derive(Debug, Deserialize, JsonSchema)]
struct Company {
    /// 会社名
    #[serde(rename = "会社名")]
    name: String,

    /// 住所
    #[serde(rename = "住所")]
    address: Option<String>,
}

/// 明細項目
#[derive(Debug, Deserialize, JsonSchema)]
struct InvoiceItem {
    /// 品名
    #[serde(rename = "品名")]
    description: String,

    /// 数量
    #[serde(rename = "数量")]
    quantity: i32,

    /// 単価
    #[serde(rename = "単価")]
    unit_price: i64,

    /// 金額
    #[serde(rename = "金額")]
    amount: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("使用方法: {} <請求書画像>", args[0]);
        std::process::exit(1);
    }

    let image_path = &args[1];
    println!("請求書を読み込み中: {}", image_path);

    let client = AedClient::from_env()?;

    // 構造化抽出
    let invoice: Invoice = client
        .extract_structured(std::path::Path::new(image_path))
        .await?;

    // 結果表示
    println!("\n=== 請求書データ ===\n");
    println!("請求書番号: {}", invoice.invoice_number);
    println!("発行日: {}", invoice.issue_date);
    println!("\n発行元: {}", invoice.issuer.name);
    if let Some(addr) = &invoice.issuer.address {
        println!("  住所: {}", addr);
    }
    println!("\n請求先: {}", invoice.recipient.name);

    println!("\n--- 明細 ---");
    for (i, item) in invoice.items.iter().enumerate() {
        println!(
            "{}. {} × {} @ ¥{} = ¥{}",
            i + 1,
            item.description,
            item.quantity,
            item.unit_price,
            item.amount
        );
    }

    println!("\n--- 金額 ---");
    println!("小計:    ¥{}", invoice.subtotal);
    println!("消費税:  ¥{}", invoice.tax);
    println!("合計:    ¥{}", invoice.total);

    Ok(())
}
