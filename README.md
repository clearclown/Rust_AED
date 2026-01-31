# Rust AED (Agentic Document Extraction)

Claude Vision API を活用した Rust 製ドキュメント抽出ライブラリ

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)

---

## 概要

**Rust AED** は、Anthropic Claude の Vision API を利用して、画像やPDFからテキストを抽出する Rust ライブラリです。従来のOCRと異なり、LLM の理解力を活かして以下を実現します：

- **日本語縦書き対応** - プロンプトエンジニアリングによる縦書きテキストの正確な読み取り
- **構造化抽出** - 表、フォーム、請求書などを JSON 形式で直接出力
- **レイアウト理解** - 図、チャート、複雑なレイアウトの意味理解
- **多言語対応** - 日本語、英語、中国語など同時処理

---

## 特徴

| 機能 | 説明 |
|------|------|
| **純 Rust 実装** | Python/GPU 依存なし、どこでも動作 |
| **非同期対応** | `tokio` ベースの高効率処理 |
| **バッチ処理** | 大量ドキュメントの並列処理 |
| **キャッシュ** | API 呼び出しの最適化 |
| **プリセット** | 書籍、請求書、名刺など用途別テンプレート |

---

## クイックスタート

### インストール

```toml
[dependencies]
rust-aed = "0.1"
tokio = { version = "1", features = ["full"] }
```

### 基本的な使い方

```rust
use rust_aed::{AedClient, OcrPreset};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // クライアント初期化（環境変数 ANTHROPIC_API_KEY を使用）
    let client = AedClient::from_env()?;

    // 画像からテキスト抽出
    let result = client
        .extract_text("document.png")
        .preset(OcrPreset::JapaneseBook)  // 日本語書籍向けプリセット
        .await?;

    println!("{}", result.text);
    Ok(())
}
```

### 縦書きテキストの抽出

```rust
use rust_aed::{AedClient, TextDirection};

let result = client
    .extract_text("vertical_novel.png")
    .direction(TextDirection::Vertical)  // 縦書き指定
    .language("ja")
    .await?;
```

### 構造化データの抽出

```rust
use rust_aed::AedClient;
use serde::Deserialize;

#[derive(Deserialize)]
struct Invoice {
    company_name: String,
    total_amount: i64,
    items: Vec<InvoiceItem>,
}

let invoice: Invoice = client
    .extract_structured("invoice.pdf")
    .await?;

println!("会社名: {}", invoice.company_name);
println!("合計: {}円", invoice.total_amount);
```

---

## 対応フォーマット

### 入力

| 形式 | 拡張子 | 備考 |
|------|--------|------|
| 画像 | `.png`, `.jpg`, `.gif`, `.webp` | 最大 5MB |
| PDF | `.pdf` | 最大 32MB、100ページ |

### 出力

- プレーンテキスト
- JSON（構造化データ）
- Markdown
- hOCR（座標付き）

---

## プリセット一覧

| プリセット | 用途 | 特徴 |
|-----------|------|------|
| `JapaneseBook` | 日本語書籍 | 縦書き対応、ルビ検出 |
| `Manga` | 漫画 | 吹き出し認識、効果音対応 |
| `Invoice` | 請求書 | 金額・明細の構造化抽出 |
| `BusinessCard` | 名刺 | 連絡先情報の抽出 |
| `Receipt` | レシート | 日付・店舗・金額抽出 |
| `Form` | フォーム | 入力欄と値のペア抽出 |
| `Table` | 表 | CSV/JSON 形式での表抽出 |
| `General` | 汎用 | 自動判定 |

---

## 設定

### 環境変数

```bash
# 必須
export ANTHROPIC_API_KEY="sk-ant-..."

# オプション
export AED_MODEL="claude-sonnet-4-5"      # 使用モデル
export AED_MAX_TOKENS="4096"               # 最大トークン数
export AED_TIMEOUT_SECS="120"              # タイムアウト秒数
```

### 設定ファイル（`aed.toml`）

```toml
[api]
model = "claude-sonnet-4-5"
max_tokens = 4096
timeout_secs = 120

[cache]
enabled = true
directory = ".aed_cache"
ttl_hours = 24

[defaults]
language = "ja"
direction = "auto"
```

---

## API リファレンス

詳細は [docs.rs](https://docs.rs/rust-aed) を参照してください。

### 主要な型

```rust
// クライアント
pub struct AedClient { ... }

// 抽出結果
pub struct ExtractionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub direction: TextDirection,
    pub blocks: Vec<TextBlock>,
}

// テキストブロック
pub struct TextBlock {
    pub text: String,
    pub bbox: Option<BoundingBox>,
    pub confidence: f32,
}
```

---

## ベンチマーク

| 処理内容 | 時間 | 備考 |
|---------|------|------|
| A4文書 1ページ | ~2秒 | Sonnet 4.5 |
| 書籍 10ページ | ~15秒 | バッチ処理 |
| 請求書構造化 | ~3秒 | JSON出力 |

※ネットワーク環境により変動

---

## ライセンス

MIT License

---

## 関連プロジェクト

- [superbook-pdf](https://github.com/example/superbook-pdf) - 書籍PDF変換ツール
- [clust](https://github.com/mochi-neko/clust) - Claude API Rustクライアント
- [kreuzberg](https://github.com/Goldziher/kreuzberg) - 多形式ドキュメント処理

---

## 貢献

Issue や Pull Request を歓迎します。詳細は [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。
