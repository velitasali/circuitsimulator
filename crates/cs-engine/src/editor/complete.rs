//! Local word completion (C++ `CodeEditor::showCompletions` / `getDocumentWords`).

use regex::Regex;
use std::sync::OnceLock;

use super::newline::{byte_to_utf16_offset, utf16_to_byte_offset};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub text: String,
    pub insert_text: String,
    pub detail: String,
    pub kind: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CompletionEdit {
    pub valid: bool,
    pub replace_start: usize,
    pub replace_end: usize,
    pub insert_text: String,
    pub new_cursor_pos: usize,
    pub is_function: bool,
    pub signature_label: String,
}

/// Calculate the completion replacement range, text, and target cursor position.
/// Handles prefix deduction, function parentheses insertion, paren skipping,
/// and UTF-16 code unit offset translation for editor compatibility.
pub fn calculate_completion(
    text: &str,
    cursor_utf16: usize,
    completion_prefix: &str,
    insert: &str,
    word: &str,
    detail: &str,
    kind: i32,
) -> CompletionEdit {
    if word.is_empty() && insert.is_empty() {
        return CompletionEdit::default();
    }

    let is_function = kind == 2
        || kind == 3
        || kind == 4
        || detail.contains('(')
        || insert.contains('(')
        || word.contains('(');

    let effective_insert = if !insert.is_empty() { insert } else { word };
    let func_name = if let Some((name, _)) = effective_insert.split_once('(') {
        name.trim()
    } else {
        effective_insert.trim()
    };

    let cursor_byte = utf16_to_byte_offset(text, cursor_utf16).min(text.len());
    let id_prefix = identifier_prefix(text, cursor_byte);
    let prefix_len = if !id_prefix.is_empty() {
        id_prefix.len()
    } else {
        completion_prefix.len().min(cursor_byte)
    };
    let mut start_byte = cursor_byte.saturating_sub(prefix_len);
    while start_byte > 0 && !text.is_char_boundary(start_byte) {
        start_byte -= 1;
    }

    let mut insert_text = func_name.to_string();
    let new_cursor_byte;

    if is_function {
        let has_next_paren = text
            .get(cursor_byte..)
            .map_or(false, |s| s.starts_with('('));
        if !has_next_paren {
            insert_text.push_str("()");
            new_cursor_byte = start_byte + func_name.len() + 1; // between ( and )
        } else {
            new_cursor_byte = start_byte + func_name.len() + 1; // past (
        }
    } else {
        new_cursor_byte = start_byte + func_name.len();
    }

    let modified_text = format!(
        "{}{}{}",
        &text[..start_byte],
        &insert_text,
        &text[cursor_byte..]
    );

    let signature_label = if is_function {
        if detail.contains('(') {
            detail.to_string()
        } else if word.contains('(') {
            word.to_string()
        } else if insert.contains('(') {
            insert.to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    CompletionEdit {
        valid: true,
        replace_start: byte_to_utf16_offset(text, start_byte),
        replace_end: byte_to_utf16_offset(text, cursor_byte),
        insert_text,
        new_cursor_pos: byte_to_utf16_offset(&modified_text, new_cursor_byte),
        is_function,
        signature_label,
    }
}

fn ident_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").expect("ident"))
}

/// Identifier immediately before `cursor` (byte offset).
pub fn identifier_prefix(text: &str, cursor: usize) -> &str {
    let mut safe_cursor = cursor.min(text.len());
    while safe_cursor > 0 && !text.is_char_boundary(safe_cursor) {
        safe_cursor -= 1;
    }
    let bytes = text.as_bytes();
    let mut i = safe_cursor;
    while i > 0 {
        let b = bytes[i - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            i -= 1;
        } else {
            break;
        }
    }
    &text[i..safe_cursor]
}

pub fn document_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = ident_re()
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .filter(|w| w.len() > 2)
        .collect();
    words.sort();
    words.dedup();
    words
}

pub fn filter_completions(source: &[String], prefix: &str) -> Vec<Completion> {
    let mut out = Vec::new();
    for w in source {
        let clean = w.strip_prefix('.').unwrap_or(w);
        if prefix.is_empty() || clean.len() >= prefix.len() && starts_with_ci(clean, prefix) {
            out.push(Completion {
                kind: if clean.contains('(') { 2 } else { 6 },
                text: clean.to_string(),
                insert_text: clean.to_string(),
                detail: String::new(),
            });
        }
        if out.len() >= 80 {
            break;
        }
    }
    out
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.get(..prefix.len())
        .is_some_and(|h| h.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_and_filter() {
        let text = "int main() { digitalWrite(1, HIGH); digi";
        let cur = text.len();
        assert_eq!(identifier_prefix(text, cur), "digi");
        let words = document_words(text);
        assert!(words.iter().any(|w| w == "digitalWrite"), "{words:?}");
        let c = filter_completions(&words, "digi");
        assert!(c.iter().any(|x| x.text == "digitalWrite"), "{c:?}");
    }

    #[test]
    fn function_completion_without_following_paren() {
        let text = "void loop() {\n    del\n}";
        // "void loop() {\n    del" has length 21. "del" starts at 18.
        let cursor = 21;
        let edit = calculate_completion(
            text,
            cursor,
            "del",
            "delay",
            "delay",
            "void delay(unsigned long ms)",
            3,
        );
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 18);
        assert_eq!(edit.replace_end, 21);
        assert_eq!(edit.insert_text, "delay()");
        // Cursor placed inside parens: 18 + 5 ("delay") + 1 ("(") = 24.
        assert_eq!(edit.new_cursor_pos, 24);
        assert!(edit.is_function);
        assert_eq!(edit.signature_label, "void delay(unsigned long ms)");
    }

    #[test]
    fn function_completion_with_following_paren() {
        let text = "void loop() {\n    del(100);\n}";
        let cursor = 21;
        let edit = calculate_completion(
            text,
            cursor,
            "del",
            "delay",
            "delay",
            "void delay(unsigned long ms)",
            3,
        );
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 18);
        assert_eq!(edit.replace_end, 21);
        assert_eq!(edit.insert_text, "delay");
        // Cursor moved past existing paren: 18 + 5 + 1 = 24.
        assert_eq!(edit.new_cursor_pos, 24);
        assert!(edit.is_function);
    }

    #[test]
    fn variable_completion() {
        let text = "int counter = 0;\nvoid loop() {\n    cou = 1;\n}";
        // "int counter = 0;\nvoid loop() {\n    cou" has length: 17 + 14 + 7 = 38
        // "cou" starts at 35.
        let cursor = 38;
        let edit = calculate_completion(
            text,
            cursor,
            "cou",
            "counter",
            "counter",
            "int counter",
            6, // variable
        );
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 35);
        assert_eq!(edit.replace_end, 38);
        assert_eq!(edit.insert_text, "counter");
        assert_eq!(edit.new_cursor_pos, 42); // 35 + 7
        assert!(!edit.is_function);
    }

    #[test]
    fn unicode_offset_preservation() {
        let text = "// Café ☕\nvoid loop() {\n    del\n}";
        // "// Café ☕\n" is 12 UTF-16 code units (or check exact):
        // UTF-16: '/' '/' ' ' 'C' 'a' 'f' 'é' ' ' '☕' '\n' = 10 chars, all BMP = 10 code units
        // "void loop() {\n    del" = 21 code units.
        // Total cursor UTF-16: 10 + 21 = 31.
        let cursor_utf16 = byte_to_utf16_offset(text, text.find("del").unwrap() + 3);
        let edit = calculate_completion(
            text,
            cursor_utf16,
            "del",
            "delay",
            "delay",
            "void delay(unsigned long ms)",
            3,
        );
        assert!(edit.valid);
        assert_eq!(edit.insert_text, "delay()");
        let expected_start = byte_to_utf16_offset(text, text.find("del").unwrap());
        assert_eq!(edit.replace_start, expected_start);
        assert_eq!(edit.replace_end, cursor_utf16);
        assert_eq!(edit.new_cursor_pos, expected_start + 6);
    }

    #[test]
    fn identifier_prefix_non_char_boundary_safety() {
        let text = "// ±\nint foo_bar = 1;";
        let pm_pos = text.find('±').unwrap();
        // Byte index inside '±' (which is 2 bytes long: pm_pos..pm_pos+2)
        let inside_char = pm_pos + 1;
        assert!(!text.is_char_boundary(inside_char));
        // Calling with an offset inside a multi-byte character must not panic
        assert_eq!(identifier_prefix(text, inside_char), "");

        let foo_end = text.find("foo_bar").unwrap() + 7;
        assert_eq!(identifier_prefix(text, foo_end), "foo_bar");

        // Cursor beyond string length must not panic
        assert_eq!(identifier_prefix(text, text.len() + 100), "");
    }
}
