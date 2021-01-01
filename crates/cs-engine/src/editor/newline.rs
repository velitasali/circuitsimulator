//! Context-aware newline indentation and smart bracket expansion.
//! Parity with C++ `CodeEditor::handleNewline`.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::format::detect_spacing_style;

/// Represents the text replacement and cursor update to execute a newline edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewlineEdit {
    pub valid: bool,
    pub replace_start: usize,
    pub replace_end: usize,
    pub insert_text: String,
    pub new_cursor_pos: usize,
}

impl Default for NewlineEdit {
    fn default() -> Self {
        Self {
            valid: false,
            replace_start: 0,
            replace_end: 0,
            insert_text: String::new(),
            new_cursor_pos: 0,
        }
    }
}

/// Convert a UTF-16 code unit offset to a byte offset in `s`.
pub fn utf16_to_byte_offset(s: &str, utf16: usize) -> usize {
    let mut cur_utf16 = 0;
    for (byte_offset, ch) in s.char_indices() {
        if cur_utf16 >= utf16 {
            return byte_offset;
        }
        cur_utf16 += ch.len_utf16();
    }
    s.len()
}

/// Convert a byte offset in `s` to a UTF-16 code unit offset.
pub fn byte_to_utf16_offset(s: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(s.len());
    let mut safe = clamped;
    while safe > 0 && !s.is_char_boundary(safe) {
        safe -= 1;
    }
    s[..safe].encode_utf16().count()
}

/// Calculate the context-aware newline edit given the document text and UTF-16
/// cursor/selection offsets from the editor.
pub fn calculate_newline(
    text: &str,
    cursor_utf16: usize,
    sel_start_utf16: usize,
    sel_end_utf16: usize,
    fallback_space_tabs: bool,
    fallback_tab_size: i32,
) -> NewlineEdit {
    let min_sel = sel_start_utf16.min(sel_end_utf16);
    let max_sel = sel_start_utf16.max(sel_end_utf16);

    let sel_start_byte = utf16_to_byte_offset(text, min_sel);
    let sel_end_byte = utf16_to_byte_offset(text, max_sel);

    if sel_start_byte < sel_end_byte {
        // Selection active: remove selection first conceptually, then compute newline edit.
        let effective_text = format!("{}{}", &text[..sel_start_byte], &text[sel_end_byte..]);
        let raw_edit = calculate_newline_bytes(
            &effective_text,
            sel_start_byte,
            fallback_space_tabs,
            fallback_tab_size,
        );
        if !raw_edit.valid {
            return NewlineEdit::default();
        }

        let sel_byte_diff = sel_end_byte - sel_start_byte;
        let orig_replace_start = raw_edit.replace_start;
        let orig_replace_end = raw_edit.replace_end + sel_byte_diff;

        let modified_text = format!(
            "{}{}{}",
            &text[..orig_replace_start],
            &raw_edit.insert_text,
            &text[orig_replace_end..]
        );

        NewlineEdit {
            valid: true,
            replace_start: byte_to_utf16_offset(text, orig_replace_start),
            replace_end: byte_to_utf16_offset(text, orig_replace_end),
            insert_text: raw_edit.insert_text,
            new_cursor_pos: byte_to_utf16_offset(&modified_text, raw_edit.new_cursor_pos),
        }
    } else {
        let cursor_byte = utf16_to_byte_offset(text, cursor_utf16);
        let raw_edit =
            calculate_newline_bytes(text, cursor_byte, fallback_space_tabs, fallback_tab_size);
        if !raw_edit.valid {
            return NewlineEdit::default();
        }

        let modified_text = format!(
            "{}{}{}",
            &text[..raw_edit.replace_start],
            &raw_edit.insert_text,
            &text[raw_edit.replace_end..]
        );

        NewlineEdit {
            valid: true,
            replace_start: byte_to_utf16_offset(text, raw_edit.replace_start),
            replace_end: byte_to_utf16_offset(text, raw_edit.replace_end),
            insert_text: raw_edit.insert_text,
            new_cursor_pos: byte_to_utf16_offset(&modified_text, raw_edit.new_cursor_pos),
        }
    }
}

/// Core newline calculation working on UTF-8 byte offsets.
fn calculate_newline_bytes(
    text: &str,
    cursor_byte: usize,
    fallback_space_tabs: bool,
    fallback_tab_size: i32,
) -> NewlineEdit {
    let cursor_byte = cursor_byte.min(text.len());

    // Find current line bounds [line_start, line_end]
    let line_start = text[..cursor_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[cursor_byte..]
        .find('\n')
        .map(|i| cursor_byte + i)
        .unwrap_or(text.len());

    let line_text = &text[line_start..line_end];
    let pos_in_line = cursor_byte - line_start;

    let text_before = &line_text[..pos_in_line];
    let text_after = &line_text[pos_in_line..];

    // Detect indentation style of current file
    let (use_tabs, indent_width) =
        detect_spacing_style(text, fallback_space_tabs, fallback_tab_size);
    let indent_step = if use_tabs {
        "\t".to_string()
    } else {
        " ".repeat(indent_width.max(1) as usize)
    };

    // Extract leading whitespace of current line
    let mut current_indent = String::new();
    for ch in line_text.chars() {
        if ch == ' ' || ch == '\t' {
            current_indent.push(ch);
        } else {
            break;
        }
    }

    // If cursor is positioned within leading whitespace, split the line at cursor
    if pos_in_line < current_indent.len() {
        return NewlineEdit {
            valid: true,
            replace_start: cursor_byte,
            replace_end: cursor_byte,
            insert_text: "\n".to_string(),
            new_cursor_pos: cursor_byte + 1,
        };
    }

    // Trim trailing whitespace from text_before (beyond current_indent)
    let mut trimmed_text_before = text_before;
    let mut trim_count = 0;
    while trimmed_text_before.len() > current_indent.len()
        && (trimmed_text_before.ends_with(' ') || trimmed_text_before.ends_with('\t'))
    {
        trimmed_text_before = &trimmed_text_before[..trimmed_text_before.len() - 1];
        trim_count += 1;
    }

    let trimmed_before = trimmed_text_before.trim();
    let trimmed_after = text_after.trim();
    let trimmed_line = line_text.trim();

    // Check 1: Inside multiline comment /* ... */
    let mut in_block_comment = false;
    let mut comment_prefix = "";
    if (trimmed_line.starts_with("/*") || trimmed_line.starts_with('*'))
        && !trimmed_before.contains("*/")
    {
        in_block_comment = true;
        if trimmed_line.starts_with("/*") {
            comment_prefix = " * ";
        } else if trimmed_line.starts_with("* ") {
            comment_prefix = "* ";
        } else if trimmed_line.starts_with('*') {
            comment_prefix = "* ";
        }
    }

    // Check 2: Matching enclosing brackets split: { | }, ( | ), [ | ]
    let matching_braces = trimmed_before.ends_with('{') && trimmed_after.starts_with('}');
    let matching_parens = trimmed_before.ends_with('(') && trimmed_after.starts_with(')');
    let matching_brackets = trimmed_before.ends_with('[') && trimmed_after.starts_with(']');

    if matching_braces || matching_parens || matching_brackets {
        let after_skip = text_after
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .count();
        let remaining_after = &text_after[after_skip..];

        let replace_start = cursor_byte - trim_count;
        let replace_end = line_end;

        let first_line_insert = format!("\n{}{}", current_indent, indent_step);
        let second_line_insert = format!("\n{}{}", current_indent, remaining_after);

        let new_cursor_pos = replace_start + first_line_insert.len();
        let insert_text = format!("{}{}", first_line_insert, second_line_insert);

        return NewlineEdit {
            valid: true,
            replace_start,
            replace_end,
            insert_text,
            new_cursor_pos,
        };
    }

    // Check 3: Check if indentation should increase
    let mut extra_indent = false;
    let unbraced_ctrl_re =
        Regex::new(r"^(if\s*\(.*\)|else|else\s+if\s*\(.*\)|for\s*\(.*\)|while\s*\(.*\)|do)$")
            .unwrap();

    if !in_block_comment {
        if trimmed_before.ends_with('{')
            || trimmed_before.ends_with('(')
            || trimmed_before.ends_with('[')
        {
            extra_indent = true;
        } else if trimmed_before.ends_with(':')
            && !trimmed_before.ends_with("::")
            && !trimmed_before.starts_with("//")
        {
            extra_indent = true;
        } else if !trimmed_before.ends_with(';')
            && !trimmed_before.ends_with('}')
            && !trimmed_before.contains("//")
            && unbraced_ctrl_re.is_match(trimmed_before)
        {
            extra_indent = true;
        }
    }

    // Check 4: Check if current statement is ending an unbraced block (Dedent)
    let mut target_indent = current_indent.clone();

    if !extra_indent && !in_block_comment && trimmed_before.ends_with(';') && line_start > 0 {
        let prev_text_slice = &text[..line_start - 1];
        let prev_line_start = prev_text_slice.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prev_line = &prev_text_slice[prev_line_start..];
        let prev_trimmed = prev_line.trim();
        if unbraced_ctrl_re.is_match(prev_trimmed)
            && !prev_trimmed.ends_with('{')
            && !prev_trimmed.ends_with(';')
        {
            let mut prev_indent = String::new();
            for ch in prev_line.chars() {
                if ch == ' ' || ch == '\t' {
                    prev_indent.push(ch);
                } else {
                    break;
                }
            }
            if current_indent.len() > prev_indent.len() {
                target_indent = prev_indent;
            }
        }
    }

    // Check 5: If cursor is right before a closing brace/bracket '}', ']', ')'
    if !extra_indent
        && !in_block_comment
        && (trimmed_after.starts_with('}')
            || trimmed_after.starts_with(']')
            || trimmed_after.starts_with(')'))
        && target_indent.ends_with(&indent_step)
    {
        target_indent.truncate(target_indent.len() - indent_step.len());
    }

    let new_line_indent = if in_block_comment {
        format!("{}{}", current_indent, comment_prefix)
    } else if extra_indent {
        format!("{}{}", current_indent, indent_step)
    } else {
        target_indent
    };

    let after_skip = text_after
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .count();
    let remaining_after = &text_after[after_skip..];

    let replace_start = cursor_byte - trim_count;
    let replace_end = line_end;

    let insert_prefix = format!("\n{}", new_line_indent);
    let new_cursor_pos = replace_start + insert_prefix.len();
    let insert_text = format!("{}{}", insert_prefix, remaining_after);

    NewlineEdit {
        valid: true,
        replace_start,
        replace_end,
        insert_text,
        new_cursor_pos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newline_leading_whitespace_split() {
        let text = "    let x = 1;";
        let edit = calculate_newline(text, 2, 2, 2, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 2);
        assert_eq!(edit.replace_end, 2);
        assert_eq!(edit.insert_text, "\n");
        assert_eq!(edit.new_cursor_pos, 3);
    }

    #[test]
    fn test_newline_same_indent() {
        let text = "    let x = 1;";
        let edit = calculate_newline(text, 14, 14, 14, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 14);
        assert_eq!(edit.replace_end, 14);
        assert_eq!(edit.insert_text, "\n    ");
        assert_eq!(edit.new_cursor_pos, 19);
    }

    #[test]
    fn test_newline_brace_expansion() {
        let text = "void loop() {}";
        let edit = calculate_newline(text, 13, 13, 13, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 13);
        assert_eq!(edit.replace_end, 14);
        assert_eq!(edit.insert_text, "\n    \n}");
        assert_eq!(edit.new_cursor_pos, 18);
    }

    #[test]
    fn test_newline_extra_indent_on_open_brace() {
        let text = "void setup() {";
        let edit = calculate_newline(text, 14, 14, 14, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.replace_start, 14);
        assert_eq!(edit.replace_end, 14);
        assert_eq!(edit.insert_text, "\n    ");
        assert_eq!(edit.new_cursor_pos, 19);
    }

    #[test]
    fn test_newline_extra_indent_on_unbraced_if() {
        let text = "if (x > 0)";
        let edit = calculate_newline(text, 10, 10, 10, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.insert_text, "\n    ");
        assert_eq!(edit.new_cursor_pos, 15);
    }

    #[test]
    fn test_newline_dedent_after_semicolon() {
        let text = "if (x > 0)\n    do_action();";
        let cursor = text.len();
        let edit = calculate_newline(text, cursor, cursor, cursor, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.insert_text, "\n");
        assert_eq!(edit.new_cursor_pos, cursor + 1);
    }

    #[test]
    fn test_newline_block_comment_continuation() {
        let text = "/*";
        let edit = calculate_newline(text, 2, 2, 2, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.insert_text, "\n * ");
        assert_eq!(edit.new_cursor_pos, 6);

        let text2 = " * hello";
        let edit2 = calculate_newline(text2, 8, 8, 8, true, 4);
        assert!(edit2.valid);
        assert_eq!(edit2.insert_text, "\n * ");
    }

    #[test]
    fn test_newline_with_selection() {
        let text = "void test() {\n    int a = 12345;\n}";
        // Select "123"
        let sel_start = text.find("123").unwrap();
        let sel_end = sel_start + 3;
        let edit = calculate_newline(text, sel_end, sel_start, sel_end, true, 4);
        assert!(edit.valid);
        // Trims trailing space after '=' before the selection
        assert_eq!(edit.replace_start, sel_start - 1);
        assert_eq!(
            edit.replace_end,
            text.find('\n').unwrap() + 1 + "    int a = 12345;".len()
        );
    }

    #[test]
    fn test_newline_tab_detected() {
        let text = "\t\tval = 42;";
        let cursor = text.len();
        let edit = calculate_newline(text, cursor, cursor, cursor, true, 4);
        assert!(edit.valid);
        assert_eq!(edit.insert_text, "\n\t\t");
    }
}
