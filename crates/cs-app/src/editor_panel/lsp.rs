//! LSP client orchestration, autocompletions, and language server diagnostics for EditorPanel.

use super::EditorPanel;
use cs_engine::editor::{
    self, CompileCtx, document_words, filter_completions, identifier_prefix, language_id_for_path,
    write_compile_flags,
};
use cs_engine::highlighter;
use serde_json::{Value, json};
use std::path::Path;

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn ensure_trailing_slash(path: &str) -> String {
    if path.is_empty() || path.ends_with('/') || path.ends_with('\\') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

impl EditorPanel {
    pub fn ensure_lsp(&mut self, path: &str) {
        if language_id_for_path(path).is_none() {
            return;
        }
        if !self.lsp.is_running() {
            if let Some(bin) = editor::find_clangd() {
                let root = Path::new(path)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ".".into());
                self.lsp.start(&bin, &root);
            }
        }
    }

    pub fn sync_lsp_open(&mut self) {
        let Some(d) = self.current_doc() else {
            return;
        };
        if d.path.is_empty() {
            return;
        }
        let path = d.path.clone();
        let text = d.text.clone();
        let compiler_name = if d.compiler.is_empty() || d.compiler == "None" {
            let lower = path.to_ascii_lowercase();
            if lower.ends_with(".ino") || lower.ends_with(".pde") {
                "Arduino".to_string()
            } else if lower.ends_with(".as") {
                "AScript".to_string()
            } else {
                String::new()
            }
        } else {
            d.compiler.clone()
        };
        let ctx = CompileCtx {
            file: path.clone(),
            board: d.board.clone(),
            custom_board: d.custom_board.clone(),
            device: d.device.clone(),
            family: d.family.clone(),
            extra_args: d.extra_args.clone(),
            incl_path: d.incl_path.clone(),
            tool_path: d.tool_path.clone(),
            debug: false,
            fqbn: String::new(),
        };
        let spec = self.specs.get(&compiler_name).cloned().unwrap_or_default();
        write_compile_flags(&ctx, &spec);
        self.ensure_lsp(&path);
        self.lsp.reopen_file(&path, &text);
    }

    pub fn hide_completions(&mut self) {
        if !self.completion_visible {
            return;
        }
        self.completion_visible = false;
        self.completions = json!([]);
        self.completion_changed();
    }

    pub fn local_complete(&mut self, prefix: &str) {
        let (text, syntax) = self
            .current_doc()
            .map(|d| (d.text.clone(), d.syntax.clone()))
            .unwrap_or_default();
        let mut source = highlighter::keywords_for_syntax(syntax.as_deref());
        source.extend(document_words(&text));
        source.sort();
        source.dedup();
        let items = filter_completions(&source, prefix);
        self.set_completion_items(
            items
                .into_iter()
                .map(|c| {
                    json!({
                        "text": c.text,
                        "insertText": c.insert_text,
                        "detail": c.detail,
                        "kind": c.kind,
                    })
                })
                .collect(),
            prefix,
        );
    }

    pub fn set_completion_items(&mut self, items: Vec<Value>, prefix: &str) {
        self.completion_prefix = prefix.to_string();
        self.completions = Value::Array(items);
        self.completion_index = 0;
        self.completion_visible = self.completions.as_array().is_some_and(|a| !a.is_empty());
        self.completion_changed();
    }

    pub fn complete_at(&mut self, manual: bool) {
        let text = self.current_text();
        let cursor_byte = editor::utf16_to_byte_offset(&text, self.cursor_pos.max(0) as usize);
        let prefix = identifier_prefix(&text, cursor_byte).to_string();
        if !manual && prefix.len() < 2 {
            self.hide_completions();
            return;
        }
        self.completion_prefix = prefix.clone();
        let path = self.current_path();
        if !path.is_empty() && language_id_for_path(&path).is_some() && self.lsp.is_running() {
            let (line, col) = editor::line_col(&text, cursor_byte);
            self.lsp.request_completion(&path, line, col);
            return;
        }
        self.local_complete(&prefix);
    }

    pub fn update_lsp_ready(&mut self) {
        let ready =
            self.lsp.is_initialized() && self.current_doc().is_some_and(|d| !d.path.is_empty());
        if ready != self.lsp_ready {
            self.lsp_ready = ready;
            self.lsp_ready_changed();
        }
    }
}
