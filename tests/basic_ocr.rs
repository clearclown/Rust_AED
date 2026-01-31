//! 基本 OCR 統合テスト
//!
//! これらのテストは実際の API を呼び出すため、
//! `--ignored` フラグを付けて実行します。
//!
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo test --test basic_ocr -- --ignored
//! ```

use rust_aed::{AedClient, OcrPreset, TextDirection};
use std::path::Path;

fn setup() -> AedClient {
    // 環境変数から API キーを取得
    AedClient::from_env().expect("ANTHROPIC_API_KEY が設定されていません")
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_basic_text_extraction() {
    let client = setup();
    let fixture_path = Path::new("tests/fixtures/horizontal_text.png");

    if !fixture_path.exists() {
        eprintln!("テストフィクスチャが見つかりません: {}", fixture_path.display());
        return;
    }

    let result = client.extract_text(fixture_path).await.unwrap();

    assert!(!result.text.is_empty());
    assert!(result.tokens_used.total() > 0);
    assert!(!result.model.is_empty());
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_vertical_text_extraction() {
    let client = setup();
    let fixture_path = Path::new("tests/fixtures/vertical_text.png");

    if !fixture_path.exists() {
        eprintln!("テストフィクスチャが見つかりません: {}", fixture_path.display());
        return;
    }

    let result = client
        .extract_text(fixture_path)
        .direction(TextDirection::Vertical)
        .language("ja")
        .await
        .unwrap();

    assert!(!result.text.is_empty());
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_preset_japanese_book() {
    let client = setup();
    let fixture_path = Path::new("tests/fixtures/vertical_text.png");

    if !fixture_path.exists() {
        eprintln!("テストフィクスチャが見つかりません: {}", fixture_path.display());
        return;
    }

    let result = client
        .extract_text(fixture_path)
        .preset(OcrPreset::JapaneseBook)
        .await
        .unwrap();

    assert!(!result.text.is_empty());
}

#[tokio::test]
#[ignore = "API キーが必要"]
async fn test_file_not_found() {
    let client = setup();

    let result = client.extract_text("nonexistent_file.png").await;

    assert!(result.is_err());
}

#[cfg(test)]
mod unit_tests {
    use rust_aed::{AedClient, TextDirection};

    #[test]
    fn test_client_creation() {
        let result = AedClient::new("test-api-key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_direction_variants() {
        assert_eq!(TextDirection::Auto.display_name_ja(), "自動");
        assert_eq!(TextDirection::Vertical.display_name_ja(), "縦書き");
        assert_eq!(TextDirection::Horizontal.display_name_ja(), "横書き");
        assert_eq!(TextDirection::Mixed.display_name_ja(), "混在");
    }
}
