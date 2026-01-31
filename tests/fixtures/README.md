# テストフィクスチャ

統合テスト用のサンプル画像を配置してください。

## 必要なファイル

| ファイル名 | 説明 |
|-----------|------|
| `horizontal_text.png` | 横書きテキストの画像 |
| `vertical_text.png` | 縦書きテキストの画像 |
| `invoice_sample.png` | 請求書サンプル |
| `business_card.png` | 名刺サンプル |
| `manga_page.png` | 漫画ページ (オプション) |

## 画像要件

- 形式: PNG, JPG, JPEG, GIF, WebP
- 推奨サイズ: 1568px 以下
- 最大ファイルサイズ: 5MB

## 統合テストの実行

```bash
# API キーを設定
export ANTHROPIC_API_KEY="sk-ant-..."

# 統合テストを実行
cargo test --test '*' -- --ignored
```

## 注意

- テストは実際の Claude API を呼び出します
- API 使用量が発生します
- フィクスチャファイルは git に含めないことを推奨
