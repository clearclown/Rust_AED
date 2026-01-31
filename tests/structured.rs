//! 構造化抽出の統合テスト

use rust_aed::extraction::structured::extract_as;
use rust_aed::AedClient;
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

fn setup() -> AedClient {
    AedClient::from_env().expect("ANTHROPIC_API_KEY が設定されていません")
}

/// 請求書のサンプル構造
#[derive(Debug, Deserialize, JsonSchema)]
struct Invoice {
    /// 請求書番号
    invoice_number: Option<String>,
    /// 発行日
    date: Option<String>,
    /// 合計金額
    total: Option<f64>,
    /// 会社名
    company_name: Option<String>,
}

/// 名刺のサンプル構造
#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct BusinessCard {
    /// 氏名
    name: Option<String>,
    /// 会社名
    company: Option<String>,
    /// 役職
    title: Option<String>,
    /// メールアドレス
    email: Option<String>,
    /// 電話番号
    phone: Option<String>,
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_invoice_extraction() {
    let client = setup();
    let fixture_path = Path::new("tests/fixtures/invoice_sample.png");

    if !fixture_path.exists() {
        eprintln!("テストフィクスチャが見つかりません: {}", fixture_path.display());
        return;
    }

    let invoice: Invoice = extract_as(&client, fixture_path)
        .await
        .expect("請求書の抽出に失敗");

    // 何かしらの値が抽出されていることを確認
    let has_some_data = invoice.invoice_number.is_some()
        || invoice.date.is_some()
        || invoice.total.is_some()
        || invoice.company_name.is_some();

    assert!(has_some_data, "請求書から何も抽出できませんでした");
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_business_card_extraction() {
    let client = setup();
    let fixture_path = Path::new("tests/fixtures/business_card.png");

    if !fixture_path.exists() {
        eprintln!("テストフィクスチャが見つかりません: {}", fixture_path.display());
        return;
    }

    let card: BusinessCard = extract_as(&client, fixture_path)
        .await
        .expect("名刺の抽出に失敗");

    let has_some_data = card.name.is_some() || card.company.is_some() || card.email.is_some();

    assert!(has_some_data, "名刺から何も抽出できませんでした");
}

#[cfg(test)]
mod unit_tests {
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct SimpleStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = schemars::schema_for!(SimpleStruct);
        let schema_json = serde_json::to_string(&schema).unwrap();

        assert!(schema_json.contains("field1"));
        assert!(schema_json.contains("field2"));
    }
}
