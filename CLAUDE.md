# CLAUDE.md

このファイルは Claude Code がこのリポジトリで作業する際のガイダンスを提供します。

## プロジェクト概要

**Rust AED (Agentic Document Extraction)** は、Anthropic Claude Vision API を活用した Rust 製の OCR・ドキュメント抽出ライブラリです。

- **目的**: 画像・PDF からのテキスト抽出と構造化データ出力
- **特徴**: 日本語縦書き対応、構造化抽出、純Rust実装
- **ライセンス**: MIT

---

## クイックスタート

```bash
cd rust-aed

# ビルド
cargo build --release

# テスト
cargo test

# サンプル実行（API キー設定後）
export ANTHROPIC_API_KEY="sk-ant-..."
cargo run --example basic_ocr
```

---

## アーキテクチャ

### ディレクトリ構造

```
rust-aed/
├── Cargo.toml
├── src/
│   ├── lib.rs              # ライブラリエントリーポイント
│   ├── client.rs           # AedClient 実装
│   ├── api/
│   │   ├── mod.rs
│   │   ├── messages.rs     # Claude Messages API
│   │   └── vision.rs       # Vision リクエスト構築
│   ├── extraction/
│   │   ├── mod.rs
│   │   ├── text.rs         # テキスト抽出
│   │   ├── structured.rs   # 構造化抽出
│   │   └── batch.rs        # バッチ処理
│   ├── presets/
│   │   ├── mod.rs
│   │   ├── japanese_book.rs
│   │   ├── invoice.rs
│   │   └── ...
│   ├── prompt/
│   │   ├── mod.rs
│   │   ├── builder.rs      # プロンプト構築
│   │   └── templates.rs    # テンプレート
│   ├── cache.rs            # レスポンスキャッシュ
│   ├── config.rs           # 設定管理
│   ├── error.rs            # エラー型定義
│   └── types.rs            # 共通型定義
├── examples/
│   ├── basic_ocr.rs
│   ├── vertical_text.rs
│   ├── invoice_extraction.rs
│   └── batch_processing.rs
├── tests/
│   ├── integration/
│   └── fixtures/
└── docs/
    ├── 企画書.md
    └── 要件定義書.md
```

### 主要モジュール

| モジュール | 責務 |
|-----------|------|
| `client.rs` | API クライアント、認証、リトライ処理 |
| `api/vision.rs` | base64エンコード、マルチパートリクエスト構築 |
| `extraction/text.rs` | テキスト抽出ロジック |
| `extraction/structured.rs` | JSON スキーマ生成、型安全な抽出 |
| `presets/` | 用途別プロンプトテンプレート |
| `prompt/builder.rs` | プロンプトエンジニアリング |

---

## 開発ガイドライン

### TDD ワークフロー

```
1. docs/ に仕様を記述
2. tests/ にテストケース作成 (Red)
3. src/ に実装 (Green)
4. リファクタリング
5. cargo test --all-features
```

### コードスタイル

```bash
# フォーマット
cargo fmt

# リント
cargo clippy -- -D warnings

# ドキュメント生成
cargo doc --open
```

### コミット規約

```
feat(extraction): 縦書きテキスト抽出を追加
fix(api): タイムアウト処理を修正
docs: README に使用例を追加
test: 請求書抽出のテストケース追加
```

---

## 主要アルゴリズム

### 1. 縦書きテキスト抽出

```rust
// presets/japanese_book.rs
pub fn vertical_text_prompt() -> String {
    r#"
    この画像には日本語の縦書きテキストが含まれています。
    以下のルールに従って抽出してください：

    1. 右から左、上から下の読み順で読む
    2. 各列を改行で区切る
    3. ルビは「漢字《ふりがな》」形式
    4. 句読点・記号はそのまま保持

    出力: 読み取ったテキストのみ
    "#.to_string()
}
```

### 2. 構造化抽出

```rust
// extraction/structured.rs
pub async fn extract_as<T: DeserializeOwned>(
    client: &AedClient,
    image_path: &Path,
) -> Result<T> {
    // 1. 型から JSON Schema 生成
    let schema = generate_schema::<T>()?;

    // 2. スキーマを含むプロンプト構築
    let prompt = format!(
        "画像から以下のJSON形式でデータを抽出:\n{}",
        serde_json::to_string_pretty(&schema)?
    );

    // 3. Claude API 呼び出し
    let response = client.call_vision(image_path, &prompt).await?;

    // 4. JSON パース
    serde_json::from_str(&response.text)
}
```

### 3. バッチ処理

```rust
// extraction/batch.rs
pub async fn batch_extract(
    client: &AedClient,
    paths: Vec<PathBuf>,
    concurrency: usize,
) -> Vec<Result<ExtractionResult>> {
    futures::stream::iter(paths)
        .map(|path| client.extract_text(&path))
        .buffer_unordered(concurrency)
        .collect()
        .await
}
```

---

## API 仕様

### Claude Messages API

```rust
// api/messages.rs
#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}
```

### エラーハンドリング

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum AedError {
    #[error("API キーが設定されていません")]
    MissingApiKey,

    #[error("API エラー: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("レート制限: {retry_after}秒後に再試行")]
    RateLimited { retry_after: u64 },

    #[error("画像読み込みエラー: {0}")]
    ImageLoadError(#[from] std::io::Error),

    #[error("JSON パースエラー: {0}")]
    JsonError(#[from] serde_json::Error),
}
```

---

## テスト

### 単体テスト

```bash
# 全テスト
cargo test

# 特定モジュール
cargo test extraction::text

# 詳細出力
cargo test -- --nocapture
```

### 統合テスト

```bash
# API キー必要
export ANTHROPIC_API_KEY="sk-ant-..."
cargo test --test integration -- --ignored
```

### テストフィクスチャ

`tests/fixtures/` に以下のサンプル画像を配置：

- `horizontal_text.png` - 横書きテキスト
- `vertical_text.png` - 縦書きテキスト
- `invoice_sample.png` - 請求書サンプル
- `manga_page.png` - 漫画ページ

---

## 依存クレート

| クレート | 用途 |
|---------|------|
| `reqwest` | HTTP クライアント |
| `tokio` | 非同期ランタイム |
| `serde` / `serde_json` | シリアライズ |
| `base64` | 画像エンコード |
| `thiserror` | エラー定義 |
| `tracing` | ログ・トレース |
| `schemars` | JSON Schema 生成 |

---

## パフォーマンス考慮事項

1. **画像サイズ**: 1568px 以下に事前リサイズ推奨
2. **バッチ処理**: 並列度 5-10 程度を推奨（レート制限対策）
3. **キャッシュ**: 同一画像の再処理を回避
4. **タイムアウト**: デフォルト 120秒、大きなPDFは延長

---

## トラブルシューティング

| 問題 | 対処法 |
|------|--------|
| `MissingApiKey` | `ANTHROPIC_API_KEY` 環境変数を設定 |
| `RateLimited` | 並列度を下げる、リトライ待機 |
| 縦書きが横書きで出力 | `TextDirection::Vertical` を明示指定 |
| 文字化け | 画像品質確認、解像度を上げる |

---

## 参考資料

- [Claude Vision ドキュメント](https://platform.claude.com/docs/en/build-with-claude/vision)
- [Claude PDF サポート](https://platform.claude.com/docs/en/docs/build-with-claude/pdf-support)
- [clust (Rust SDK)](https://github.com/mochi-neko/clust)
