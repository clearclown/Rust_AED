//! テキスト処理ユーティリティ
//!
//! 抽出されたテキストの後処理・正規化を行います。

use crate::types::TextDirection;

/// 空白を正規化
///
/// 連続する空白を単一スペースに、不要な空行を削除します。
pub fn normalize_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_space = false;
    let mut prev_was_newline = false;
    let mut consecutive_newlines = 0;

    for ch in text.chars() {
        match ch {
            ' ' | '\t' | '\u{3000}' => {
                // 全角スペース、タブ、半角スペースを統一
                if !prev_was_space && !prev_was_newline {
                    result.push(' ');
                    prev_was_space = true;
                }
            }
            '\n' | '\r' => {
                if prev_was_newline {
                    consecutive_newlines += 1;
                    // 3つ以上の連続改行は2つにまとめる
                    if consecutive_newlines <= 1 {
                        result.push('\n');
                    }
                } else {
                    result.push('\n');
                    prev_was_newline = true;
                    consecutive_newlines = 0;
                }
                prev_was_space = false;
            }
            _ => {
                result.push(ch);
                prev_was_space = false;
                prev_was_newline = false;
                consecutive_newlines = 0;
            }
        }
    }

    result.trim().to_string()
}

/// テキスト方向を検出
///
/// テキストの文字配列パターンから縦書き/横書きを推定します。
/// 主に日本語テキストを対象としています。
pub fn detect_direction(text: &str) -> TextDirection {
    // 縦書き特有のパターンをチェック
    let vertical_indicators = [
        "︱", "︳", "︴", "︵", "︶", "︷", "︸", // 縦書き用括弧
        "﹅", "﹆", // 傍点
    ];

    let horizontal_indicators = [
        "「", "」", "『", "』", "(", ")", // 横書き用括弧
        "。", "、", // 横書き句読点
    ];

    let mut vertical_score = 0;
    let mut horizontal_score = 0;

    for indicator in &vertical_indicators {
        if text.contains(indicator) {
            vertical_score += 1;
        }
    }

    for indicator in &horizontal_indicators {
        if text.contains(indicator) {
            horizontal_score += 1;
        }
    }

    // 改行パターンの分析
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > 1 {
        // 短い行が多い場合は縦書きの可能性
        let avg_line_len: f64 = lines.iter().map(|l| l.chars().count()).sum::<usize>() as f64
            / lines.len() as f64;

        if avg_line_len < 20.0 {
            vertical_score += 1;
        } else {
            horizontal_score += 1;
        }
    }

    if vertical_score > horizontal_score {
        TextDirection::Vertical
    } else if horizontal_score > vertical_score {
        TextDirection::Horizontal
    } else {
        TextDirection::Auto
    }
}

/// ルビ（振り仮名）を抽出
///
/// 「漢字《ふりがな》」形式のルビを抽出します。
///
/// # Returns
///
/// (親文字, ふりがな) のタプルのベクター
pub fn extract_ruby_annotations(text: &str) -> Vec<(String, String)> {
    let mut annotations = Vec::new();
    let mut chars = text.chars().peekable();
    let mut parent = String::new();

    while let Some(ch) = chars.next() {
        if ch == '《' {
            // 《 の直前までが親文字
            if !parent.is_empty() {
                let ruby = collect_until(&mut chars, '》');
                if !ruby.is_empty() {
                    // 親文字から漢字部分のみ抽出
                    let parent_chars: String = parent
                        .chars()
                        .rev()
                        .take_while(|c| is_kanji(*c))
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();

                    if !parent_chars.is_empty() {
                        annotations.push((parent_chars, ruby));
                    }
                }
                parent.clear();
            }
        } else {
            parent.push(ch);
        }
    }

    annotations
}

/// ルビを除去
///
/// 「漢字《ふりがな》」形式のルビを除去し、親文字のみを残します。
pub fn remove_ruby_annotations(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_ruby = false;

    for ch in text.chars() {
        if ch == '《' {
            in_ruby = true;
        } else if ch == '》' {
            in_ruby = false;
        } else if !in_ruby {
            result.push(ch);
        }
    }

    result
}

/// 数字を半角に正規化
pub fn normalize_numbers(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '０' => '0',
            '１' => '1',
            '２' => '2',
            '３' => '3',
            '４' => '4',
            '５' => '5',
            '６' => '6',
            '７' => '7',
            '８' => '8',
            '９' => '9',
            _ => c,
        })
        .collect()
}

/// アルファベットを半角に正規化
pub fn normalize_alphabet(text: &str) -> String {
    text.chars()
        .map(|c| {
            let code = c as u32;
            // 全角英大文字 (Ａ-Ｚ: U+FF21-U+FF3A)
            if (0xFF21..=0xFF3A).contains(&code) {
                char::from_u32(code - 0xFF21 + 0x41).unwrap_or(c)
            // 全角英小文字 (ａ-ｚ: U+FF41-U+FF5A)
            } else if (0xFF41..=0xFF5A).contains(&code) {
                char::from_u32(code - 0xFF41 + 0x61).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// テキストから行を抽出
pub fn split_lines(text: &str) -> Vec<&str> {
    text.lines().filter(|line| !line.trim().is_empty()).collect()
}

/// 指定文字まで収集
fn collect_until(chars: &mut std::iter::Peekable<std::str::Chars>, until: char) -> String {
    let mut result = String::new();
    while let Some(&ch) = chars.peek() {
        chars.next();
        if ch == until {
            break;
        }
        result.push(ch);
    }
    result
}

/// 漢字かどうかを判定
fn is_kanji(c: char) -> bool {
    let code = c as u32;
    // CJK統合漢字
    (0x4E00..=0x9FFF).contains(&code)
        // CJK統合漢字拡張A
        || (0x3400..=0x4DBF).contains(&code)
        // CJK統合漢字拡張B-F
        || (0x20000..=0x2A6DF).contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("a  b   c"), "a b c");
        assert_eq!(normalize_whitespace("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(normalize_whitespace("  hello  "), "hello");
        assert_eq!(normalize_whitespace("日本語　テスト"), "日本語 テスト");
    }

    #[test]
    fn test_extract_ruby() {
        let text = "漢字《かんじ》を読む";
        let annotations = extract_ruby_annotations(text);
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0], ("漢字".to_string(), "かんじ".to_string()));
    }

    #[test]
    fn test_extract_multiple_ruby() {
        let text = "東京《とうきょう》と大阪《おおさか》";
        let annotations = extract_ruby_annotations(text);
        assert_eq!(annotations.len(), 2);
        assert_eq!(annotations[0].0, "東京");
        assert_eq!(annotations[1].0, "大阪");
    }

    #[test]
    fn test_remove_ruby() {
        let text = "漢字《かんじ》を読む";
        let result = remove_ruby_annotations(text);
        assert_eq!(result, "漢字を読む");
    }

    #[test]
    fn test_normalize_numbers() {
        assert_eq!(normalize_numbers("１２３４５"), "12345");
        assert_eq!(normalize_numbers("価格：￥１，０００"), "価格：￥1，000");
    }

    #[test]
    fn test_normalize_alphabet() {
        assert_eq!(normalize_alphabet("Ａｂｃ"), "Abc");
        assert_eq!(normalize_alphabet("ＨＥＬＬＯ"), "HELLO");
    }

    #[test]
    fn test_detect_direction_vertical() {
        // 短い行が多い場合
        let text = "あ\nい\nう\nえ\nお";
        let direction = detect_direction(text);
        assert_eq!(direction, TextDirection::Vertical);
    }

    #[test]
    fn test_detect_direction_horizontal() {
        // 長い行がある場合
        let text = "これは横書きの文章です。普通の日本語の文章として書かれています。";
        let direction = detect_direction(text);
        assert_eq!(direction, TextDirection::Horizontal);
    }

    #[test]
    fn test_split_lines() {
        let text = "line1\n\nline2\n  \nline3";
        let lines = split_lines(text);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
    }

    #[test]
    fn test_is_kanji() {
        assert!(is_kanji('漢'));
        assert!(is_kanji('字'));
        assert!(!is_kanji('あ'));
        assert!(!is_kanji('a'));
    }
}
