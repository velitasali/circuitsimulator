//! Text search, find-and-replace, and navigation options for EditorPanel.

use super::EditorPanel;
use cs_engine::editor::{self, FindOpts};

impl EditorPanel {
    pub fn find_opts(&self) -> FindOpts {
        FindOpts {
            text: self.find_text.clone(),
            case_sensitive: self.find_case,
            whole_words: self.find_words,
            regexp: self.find_regexp,
        }
    }

    pub fn apply_match(&mut self, text: &str, m: editor::Match) {
        let u16_start = editor::byte_to_utf16_offset(text, m.start);
        let u16_end = editor::byte_to_utf16_offset(text, m.end);
        self.select_start = u16_start as i32;
        self.select_end = u16_end as i32;
        self.cursor_pos = u16_end as i32;
        self.goto_changed();
    }

    pub fn goto_1based(&mut self, line: i32) {
        self.goto_line = line;
        self.goto_changed();
    }
}
