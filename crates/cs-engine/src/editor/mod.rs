//! Code editor support: compiler XML, `.cfg` sidecars, find/replace,
//! word completion, and a clangd LSP client. No Qt.

pub mod arduino;
pub mod ascript;
pub mod cfg;
pub mod compiler;
pub mod complete;
pub mod find;
pub mod format;
pub mod lsp;
pub mod newline;
pub mod watcher;

pub use arduino::{
    BoardEntry, arduino_include_paths, board_names_for_ui, default_boards,
    device_for_board_or_fqbn, list_arduino_boards, match_board, parse_board_list,
    resolve_board_fqbn,
};
pub use cfg::{FileConfig, load_cfg, save_cfg};
pub use compiler::{
    CompileCtx, CompileEvent, CompileJob, CompileResult, CompilerSpec, check_tool_path, compile,
    compiler_names, extract_ino_prototypes, find_executable, load_compilers, tool_path_candidates,
    write_compile_flags,
};
pub use complete::{
    Completion, CompletionEdit, calculate_completion, document_words, filter_completions,
    identifier_prefix,
};
pub use find::{FindOpts, Match, find_all, find_next, replace_all};
pub use format::{detect_spacing_style, format_code};
pub use lsp::{
    DiagnosticItem, LspClient, LspEvent, SignatureHelp, find_clangd, format_hover_markdown,
    format_signature_html, language_id_for_path,
};
pub use newline::{NewlineEdit, calculate_newline};
pub use watcher::{FileWatcher, WatchEvent};

/// 0-based `(line, character)` for a byte offset, matching LSP / QTextCursor.
pub fn line_col(text: &str, pos: usize) -> (i32, i32) {
    let mut safe_pos = pos.min(text.len());
    while safe_pos > 0 && !text.is_char_boundary(safe_pos) {
        safe_pos -= 1;
    }
    let mut line = 0i32;
    let mut last = 0usize;
    for (i, _) in text[..safe_pos].match_indices('\n') {
        line += 1;
        last = i + 1;
    }
    let col = text[last..safe_pos].chars().count() as i32;
    (line, col)
}

/// Byte offset of the start of a 1-based line.
pub fn line_start(text: &str, line_1: i32) -> usize {
    if line_1 <= 1 {
        return 0;
    }
    let mut n = 1i32;
    for (i, _) in text.match_indices('\n') {
        n += 1;
        if n == line_1 {
            return (i + 1).min(text.len());
        }
    }
    text.len()
}

/// 1-based line count (an empty buffer is one line).
pub fn line_count(text: &str) -> i32 {
    if text.is_empty() {
        1
    } else {
        text.bytes().filter(|b| *b == b'\n').count() as i32 + 1
    }
}

/// Byte offset from 0-based line and character (UTF-16-ish: we count chars).
pub fn offset_at(text: &str, line: i32, character: i32) -> usize {
    let start = line_start(text, line + 1);
    let rest = &text[start..];
    let line_end = rest.find('\n').map(|i| start + i).unwrap_or(text.len());
    let mut chars = 0i32;
    for (i, c) in text[start..line_end].char_indices() {
        if chars >= character {
            return start + i;
        }
        chars += 1;
        let _ = c;
    }
    line_end
}

use crate::theme::{ColorId, ColorTheme};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticRangeItem {
    pub start: usize,
    pub end: usize,
    pub severity: i32,
    pub color: String,
    pub message: String,
}

/// Convert a UTF-16 code unit offset into a UTF-8 byte offset.
pub fn utf16_to_byte_offset(text: &str, utf16_pos: usize) -> usize {
    let mut count = 0usize;
    for (idx, ch) in text.char_indices() {
        if count >= utf16_pos {
            return idx;
        }
        count += ch.len_utf16();
    }
    text.len()
}

/// Convert a UTF-8 byte offset into a UTF-16 code unit offset.
pub fn byte_to_utf16_offset(text: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(text.len());
    let mut safe = clamped;
    while safe > 0 && !text.is_char_boundary(safe) {
        safe -= 1;
    }
    text[..safe].encode_utf16().count()
}

/// Port of C++ `CodeEditor::onDiagnosticsReceived` word expansion:
/// If `start == end`, moves to the end of the word (`EndOfWord`),
/// or next character if at punctuation / boundary.
pub fn expand_token_if_empty(text: &str, start_byte: usize, mut end_byte: usize) -> usize {
    if start_byte != end_byte || start_byte >= text.len() {
        return end_byte;
    }
    let rest = &text[start_byte..];
    let mut chars = rest.chars();
    if let Some(first) = chars.next() {
        if first.is_alphanumeric() || first == '_' {
            let mut word_len = first.len_utf8();
            for ch in chars {
                if ch.is_alphanumeric() || ch == '_' {
                    word_len += ch.len_utf8();
                } else {
                    break;
                }
            }
            end_byte = start_byte + word_len;
        } else {
            end_byte = start_byte + first.len_utf8();
        }
    }
    end_byte
}

/// Compute diagnostic token ranges with UTF-16 offsets and theme colors,
/// splitting multi-line diagnostics per line so each segment stays on a single line.
pub fn compute_diagnostic_ranges(
    text: &str,
    items: &[DiagnosticItem],
    dark: bool,
) -> Vec<DiagnosticRangeItem> {
    let mut out = Vec::new();
    let total_lines = line_count(text);

    for diag in items {
        let color = match diag.severity {
            1 => ColorTheme::get_hex(ColorId::EditorErrorUnderline, dark),
            2 => ColorTheme::get_hex(ColorId::EditorWarnUnderline, dark),
            _ => ColorTheme::get_hex(ColorId::EditorInfoUnderline, dark),
        };

        let s_line = diag.start_line.clamp(0, (total_lines - 1).max(0));
        let e_line = diag.end_line.clamp(0, (total_lines - 1).max(0));

        if s_line == e_line {
            let start_byte = offset_at(text, s_line, diag.start_col);
            let mut end_byte = offset_at(text, e_line, diag.end_col);
            if start_byte == end_byte {
                end_byte = expand_token_if_empty(text, start_byte, end_byte);
            }
            let u16_start = byte_to_utf16_offset(text, start_byte);
            let u16_end = byte_to_utf16_offset(text, end_byte);
            if u16_end > u16_start {
                out.push(DiagnosticRangeItem {
                    start: u16_start,
                    end: u16_end,
                    severity: diag.severity,
                    color,
                    message: diag.message.clone(),
                });
            }
        } else {
            // Multi-line range: split per line
            for line in s_line..=e_line {
                let start_byte = if line == s_line {
                    offset_at(text, line, diag.start_col)
                } else {
                    line_start(text, line + 1)
                };
                let end_byte = if line == e_line {
                    offset_at(text, line, diag.end_col)
                } else {
                    let ls = line_start(text, line + 1);
                    let rest = &text[ls..];
                    rest.find('\n').map(|i| ls + i).unwrap_or(text.len())
                };

                let u16_start = byte_to_utf16_offset(text, start_byte);
                let u16_end = byte_to_utf16_offset(text, end_byte);
                if u16_end > u16_start {
                    out.push(DiagnosticRangeItem {
                        start: u16_start,
                        end: u16_end,
                        severity: diag.severity,
                        color: color.clone(),
                        message: diag.message.clone(),
                    });
                }
            }
        }
    }

    out
}

/// Port of C++ `CodeEditor::getParamIndexAtCursor`:
/// Walks backwards from cursor position (up to 500 characters) to determine
/// which parameter index (0-based) the cursor is currently on.
/// Handles nesting of `()`, `[]`, `{}` and stops on statement boundaries (`;`, `{`).
/// Returns `-1` if cursor is not inside an unclosed `(` parameter list.
pub fn get_param_index_at_cursor(text: &str, utf16_pos: usize) -> i32 {
    let byte_pos = utf16_to_byte_offset(text, utf16_pos);
    let slice = &text[..byte_pos];
    let mut nesting = 0i32;
    let mut comma_count = 0i32;
    let mut max_chars = 500;
    let mut found_open_paren = false;

    for ch in slice.chars().rev() {
        if max_chars <= 0 {
            break;
        }
        max_chars -= 1;

        match ch {
            ')' | ']' | '}' => nesting += 1,
            '(' => {
                if nesting > 0 {
                    nesting -= 1;
                } else {
                    found_open_paren = true;
                    break;
                }
            }
            '[' => {
                if nesting > 0 {
                    nesting -= 1;
                }
            }
            '{' => {
                if nesting > 0 {
                    nesting -= 1;
                } else {
                    break;
                }
            }
            ';' => break,
            ',' if nesting == 0 => comma_count += 1,
            _ => {}
        }
    }

    if !found_open_paren { -1 } else { comma_count }
}

/// C/C++ keywords that should not be treated as navigatable symbol definitions.
pub const C_CPP_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "atomic_cancel",
    "atomic_commit",
    "atomic_noexcept",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "const_cast",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "reflexpr",
    "register",
    "reinterpret_cast",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "synchronized",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
];

/// Find an identifier token at the given UTF-16 code unit offset.
/// Returns `Some((start_utf16, end_utf16, token_slice))` if the offset points to
/// a valid identifier (`[A-Za-z_][A-Za-z0-9_]*`) that is not a reserved keyword.
pub fn find_ident_at_pos(text: &str, pos_utf16: usize) -> Option<(usize, usize, &str)> {
    if text.is_empty() {
        return None;
    }
    let byte_pos = utf16_to_byte_offset(text, pos_utf16);
    if byte_pos >= text.len() {
        return None;
    }

    let bytes = text.as_bytes();
    let cur = bytes[byte_pos];
    // Must be resting on an identifier character
    if !cur.is_ascii_alphanumeric() && cur != b'_' {
        return None;
    }

    // Expand backwards
    let mut start = byte_pos;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    // Expand forwards
    let mut end = byte_pos;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if start >= end {
        return None;
    }

    // First char cannot be a digit
    if bytes[start].is_ascii_digit() {
        return None;
    }

    let token = &text[start..end];
    if C_CPP_KEYWORDS.contains(&token) {
        return None;
    }

    let start_utf16 = byte_to_utf16_offset(text, start);
    let end_utf16 = byte_to_utf16_offset(text, end);
    Some((start_utf16, end_utf16, token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_basic() {
        let t = "ab\ncd\ne";
        assert_eq!(line_col(t, 0), (0, 0));
        assert_eq!(line_col(t, 2), (0, 2));
        assert_eq!(line_col(t, 3), (1, 0));
        assert_eq!(line_col(t, 5), (1, 2));
        assert_eq!(line_start(t, 1), 0);
        assert_eq!(line_start(t, 2), 3);
        assert_eq!(line_count(t), 3);
        assert_eq!(offset_at(t, 1, 1), 4);
    }

    #[test]
    fn utf16_and_byte_conversions() {
        let t = "a🦀b\nc";
        // 'a' = 1 byte, 1 utf16
        // '🦀' = 4 bytes, 2 utf16
        // 'b' = 1 byte, 1 utf16
        assert_eq!(byte_to_utf16_offset(t, 0), 0);
        assert_eq!(byte_to_utf16_offset(t, 1), 1);
        assert_eq!(byte_to_utf16_offset(t, 5), 3); // after crab
        assert_eq!(byte_to_utf16_offset(t, 6), 4); // after 'b'
        assert_eq!(utf16_to_byte_offset(t, 0), 0);
        assert_eq!(utf16_to_byte_offset(t, 1), 1);
        assert_eq!(utf16_to_byte_offset(t, 3), 5);
        assert_eq!(utf16_to_byte_offset(t, 4), 6);
    }

    #[test]
    fn token_expansion() {
        let t = "int foo_bar = 123;";
        // Point diagnostic at start of "foo_bar" (offset 4)
        assert_eq!(expand_token_if_empty(t, 4, 4), 11);
        assert_eq!(&t[4..11], "foo_bar");

        // Already has non-empty range
        assert_eq!(expand_token_if_empty(t, 4, 7), 7);

        // Punctuation ';' at offset 17
        assert_eq!(expand_token_if_empty(t, 17, 17), 18);
    }

    #[test]
    fn diagnostic_ranges_computation() {
        let t = "void setup() {\n  int val = bad_var;\n}";
        let diags = vec![
            DiagnosticItem {
                start_line: 1,
                start_col: 12,
                end_line: 1,
                end_col: 19,
                severity: 1,
                message: "use of undeclared identifier 'bad_var'".into(),
            },
            DiagnosticItem {
                start_line: 1,
                start_col: 6,
                end_line: 1,
                end_col: 6, // 0-width, should expand "val"
                severity: 2,
                message: "unused variable 'val'".into(),
            },
        ];

        let ranges = compute_diagnostic_ranges(t, &diags, false);
        assert_eq!(ranges.len(), 2);
        // "bad_var": line 1 starts at byte 15. col 12 is 27. col 19 is 34.
        assert_eq!(ranges[0].start, 27);
        assert_eq!(ranges[0].end, 34);
        assert_eq!(ranges[0].severity, 1);
        assert_eq!(ranges[0].color, "#f03232");

        // "val": line 1, col 6 is 21. Expanded to 24 ("val").
        assert_eq!(ranges[1].start, 21);
        assert_eq!(ranges[1].end, 24);
        assert_eq!(ranges[1].severity, 2);
        assert_eq!(ranges[1].color, "#ffa500");
    }

    #[test]
    fn multiline_diagnostic_splits_per_line() {
        let t = "line0\nline1\nline2";
        let diags = vec![DiagnosticItem {
            start_line: 0,
            start_col: 2,
            end_line: 2,
            end_col: 3,
            severity: 1,
            message: "multiline error".into(),
        }];

        let ranges = compute_diagnostic_ranges(t, &diags, true);
        assert_eq!(ranges.len(), 3);
        // Line 0: from col 2 (offset 2) to end of line 0 (offset 5)
        assert_eq!(ranges[0].start, 2);
        assert_eq!(ranges[0].end, 5);
        // Line 1: from col 0 (offset 6) to end of line 1 (offset 11)
        assert_eq!(ranges[1].start, 6);
        assert_eq!(ranges[1].end, 11);
        // Line 2: from col 0 (offset 12) to col 3 (offset 15)
        assert_eq!(ranges[2].start, 12);
        assert_eq!(ranges[2].end, 15);
        assert_eq!(ranges[0].color, "#ff5050");
    }

    #[test]
    fn test_param_index_basic() {
        let text = "digitalWrite(13, HIGH)";
        // Before '('
        assert_eq!(get_param_index_at_cursor(text, 11), -1);
        // Right after '('
        assert_eq!(get_param_index_at_cursor(text, 13), 0);
        // At "13"
        assert_eq!(get_param_index_at_cursor(text, 15), 0);
        // After comma
        assert_eq!(get_param_index_at_cursor(text, 16), 1);
        // At "HIGH"
        assert_eq!(get_param_index_at_cursor(text, 20), 1);
        // After ')'
        assert_eq!(get_param_index_at_cursor(text, 22), -1);
    }

    #[test]
    fn test_param_index_nested() {
        let text = "digitalWrite(readPin(4, 5), HIGH)";
        // Inside readPin, param 0
        let pos_inside_readpin_0 = text.find("readPin(").unwrap() + 8;
        assert_eq!(get_param_index_at_cursor(text, pos_inside_readpin_0), 0);
        // Inside readPin, param 1
        let pos_inside_readpin_1 = text.find("5").unwrap();
        assert_eq!(get_param_index_at_cursor(text, pos_inside_readpin_1), 1);
        // After readPin(...) call, at "HIGH"
        let pos_high = text.find("HIGH").unwrap();
        assert_eq!(get_param_index_at_cursor(text, pos_high), 1);
    }

    #[test]
    fn test_param_index_outside_and_boundaries() {
        assert_eq!(get_param_index_at_cursor("int a = 5;", 8), -1);
        assert_eq!(get_param_index_at_cursor("func(); foo", 10), -1);
        assert_eq!(get_param_index_at_cursor("{ func(1); }", 11), -1);
    }

    #[test]
    fn test_param_index_brackets_and_multiline() {
        let text = "draw({\n  1,\n  2\n}, [a, b], 42)";
        let pos_after_bracket = text.find("42").unwrap();
        assert_eq!(get_param_index_at_cursor(text, pos_after_bracket), 2);
    }

    #[test]
    fn test_find_ident_at_pos_basic() {
        let text = "void delay(unsigned long ms);";
        // 'delay' is at 5..10
        for i in 5..10 {
            let res = find_ident_at_pos(text, i);
            assert_eq!(res, Some((5, 10, "delay")), "at pos {i}");
        }

        // 'void' is a keyword
        assert_eq!(find_ident_at_pos(text, 0), None);
        assert_eq!(find_ident_at_pos(text, 2), None);

        // 'unsigned' and 'long' are keywords
        assert_eq!(find_ident_at_pos(text, 11), None);
        assert_eq!(find_ident_at_pos(text, 20), None);

        // 'ms' is an identifier at 25..27
        assert_eq!(find_ident_at_pos(text, 25), Some((25, 27, "ms")));
        assert_eq!(find_ident_at_pos(text, 26), Some((25, 27, "ms")));

        // Space and punctuation
        assert_eq!(find_ident_at_pos(text, 4), None); // space
        assert_eq!(find_ident_at_pos(text, 10), None); // '('
        assert_eq!(find_ident_at_pos(text, 27), None); // ')'
        assert_eq!(find_ident_at_pos(text, 28), None); // ';'
        assert_eq!(find_ident_at_pos(text, 100), None); // out of bounds
    }

    #[test]
    fn test_find_ident_at_pos_unicode() {
        let text = "// 🦀\nfoo_bar(123);";
        // '🦀' has 4 bytes and 2 utf16 units
        let foo_start_utf16 = byte_to_utf16_offset(text, text.find("foo_bar").unwrap());
        let res = find_ident_at_pos(text, foo_start_utf16 + 2);
        assert_eq!(res, Some((foo_start_utf16, foo_start_utf16 + 7, "foo_bar")));

        // Numbers should not match
        let num_start_utf16 = byte_to_utf16_offset(text, text.find("123").unwrap());
        assert_eq!(find_ident_at_pos(text, num_start_utf16), None);
    }
}
