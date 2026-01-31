//! Vision API ユーティリティ
//!
//! 画像の読み込み、エンコード、バリデーション

use std::path::Path;

use crate::error::{AedError, Result};

/// サポートされている画像形式
pub const SUPPORTED_IMAGE_FORMATS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

/// サポートされている MIME タイプ
pub const SUPPORTED_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
];

/// 最大画像サイズ（5MB）
pub const MAX_IMAGE_SIZE: u64 = 5 * 1024 * 1024;

/// 最大 PDF サイズ（32MB）
pub const MAX_PDF_SIZE: u64 = 32 * 1024 * 1024;

/// 最大ページ数
pub const MAX_PAGES: u32 = 100;

/// 最小画像サイズ（ピクセル）
pub const MIN_IMAGE_DIMENSION: u32 = 200;

/// 推奨最大画像サイズ（ピクセル）
pub const RECOMMENDED_MAX_DIMENSION: u32 = 1568;

/// 画像データ
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Base64 エンコード済みデータ
    pub base64: String,
    /// MIME タイプ
    pub media_type: String,
    /// 元のファイルサイズ
    pub original_size: u64,
}

/// 画像を読み込んで Base64 エンコード
pub fn load_image<P: AsRef<Path>>(path: P) -> Result<ImageData> {
    let path = path.as_ref();

    // ファイル存在確認
    if !path.exists() {
        return Err(AedError::FileNotFound(path.to_path_buf()));
    }

    // 拡張子チェック
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| AedError::UnsupportedFormat("拡張子がありません".to_string()))?;

    if !SUPPORTED_IMAGE_FORMATS.contains(&extension.as_str()) {
        return Err(AedError::UnsupportedFormat(extension));
    }

    // ファイル読み込み
    let data = std::fs::read(path)?;
    let original_size = data.len() as u64;

    // サイズチェック
    if original_size > MAX_IMAGE_SIZE {
        return Err(AedError::FileTooLarge {
            size: original_size,
            max: MAX_IMAGE_SIZE,
        });
    }

    // MIME タイプ決定
    let media_type = extension_to_mime_type(&extension);

    // Base64 エンコード
    let base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &data,
    );

    Ok(ImageData {
        base64,
        media_type,
        original_size,
    })
}

/// PDF を読み込んで Base64 エンコード
pub fn load_pdf<P: AsRef<Path>>(path: P) -> Result<ImageData> {
    let path = path.as_ref();

    // ファイル存在確認
    if !path.exists() {
        return Err(AedError::FileNotFound(path.to_path_buf()));
    }

    // 拡張子チェック
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    if extension.as_deref() != Some("pdf") {
        return Err(AedError::UnsupportedFormat(
            extension.unwrap_or_default(),
        ));
    }

    // ファイル読み込み
    let data = std::fs::read(path)?;
    let original_size = data.len() as u64;

    // サイズチェック
    if original_size > MAX_PDF_SIZE {
        return Err(AedError::FileTooLarge {
            size: original_size,
            max: MAX_PDF_SIZE,
        });
    }

    // Base64 エンコード
    let base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &data,
    );

    Ok(ImageData {
        base64,
        media_type: "application/pdf".to_string(),
        original_size,
    })
}

/// 拡張子から MIME タイプを取得
fn extension_to_mime_type(extension: &str) -> String {
    match extension {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// トークン数を概算
///
/// 画像サイズから消費トークン数を概算します。
/// 計算式: tokens = (width * height) / 750
pub fn estimate_tokens(width: u32, height: u32) -> u32 {
    (width * height) / 750
}

/// 画像をリサイズすべきかを判定
pub fn should_resize(width: u32, height: u32) -> bool {
    width > RECOMMENDED_MAX_DIMENSION || height > RECOMMENDED_MAX_DIMENSION
}

/// リサイズ後のサイズを計算（アスペクト比維持）
pub fn calculate_resize_dimensions(width: u32, height: u32) -> (u32, u32) {
    if !should_resize(width, height) {
        return (width, height);
    }

    let ratio = (RECOMMENDED_MAX_DIMENSION as f64)
        / (width.max(height) as f64);

    let new_width = (width as f64 * ratio) as u32;
    let new_height = (height as f64 * ratio) as u32;

    (new_width, new_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_to_mime_type() {
        assert_eq!(extension_to_mime_type("png"), "image/png");
        assert_eq!(extension_to_mime_type("jpg"), "image/jpeg");
        assert_eq!(extension_to_mime_type("jpeg"), "image/jpeg");
        assert_eq!(extension_to_mime_type("gif"), "image/gif");
        assert_eq!(extension_to_mime_type("webp"), "image/webp");
        assert_eq!(extension_to_mime_type("pdf"), "application/pdf");
    }

    #[test]
    fn test_estimate_tokens() {
        // 1000x1000 = 1,000,000 / 750 = 1333
        assert_eq!(estimate_tokens(1000, 1000), 1333);

        // 1092x1092 = 1,192,464 / 750 = 1589 (整数除算)
        assert_eq!(estimate_tokens(1092, 1092), 1589);
    }

    #[test]
    fn test_should_resize() {
        assert!(!should_resize(1000, 1000));
        assert!(!should_resize(1568, 1568));
        assert!(should_resize(2000, 1000));
        assert!(should_resize(1000, 2000));
    }

    #[test]
    fn test_calculate_resize_dimensions() {
        // リサイズ不要
        let (w, h) = calculate_resize_dimensions(1000, 1000);
        assert_eq!((w, h), (1000, 1000));

        // 横長画像のリサイズ
        let (w, h) = calculate_resize_dimensions(3000, 2000);
        assert!(w <= RECOMMENDED_MAX_DIMENSION);
        assert!(h <= RECOMMENDED_MAX_DIMENSION);

        // 縦長画像のリサイズ
        let (w, h) = calculate_resize_dimensions(1000, 3000);
        assert!(w <= RECOMMENDED_MAX_DIMENSION);
        assert!(h <= RECOMMENDED_MAX_DIMENSION);
    }
}
