//! Document model and per-file configuration state for the code editor.

use cs_engine::backup;
use cs_engine::editor::{self, BoardEntry, CompilerSpec, FileConfig, match_board};
use cs_engine::settings::AppSettings;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn extract_in_file_value(text: &str, key: &str) -> String {
    let Some(first_line) = text.lines().next() else {
        return String::new();
    };
    let line_lower = first_line.to_ascii_lowercase();
    let key_lower = key.to_ascii_lowercase();
    if !line_lower.contains(&key_lower) {
        return String::new();
    }
    let normalized = first_line.replace(['=', ':'], " ");
    let mut words = normalized.split_whitespace();
    while let Some(word) = words.next() {
        if word.to_ascii_lowercase().contains(&key_lower) {
            if let Some(val) = words.next() {
                let trimmed = val.trim_matches(|c: char| matches!(c, ',' | '.' | ';' | '"' | '\''));
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    String::new()
}

pub struct Document {
    pub title: String,
    pub path: String,
    pub backup_id: String,
    pub text: String,
    pub dirty: bool,
    pub syntax: Option<String>,
    pub breakpoints: Vec<i32>,
    pub errors: Vec<i32>,
    pub warnings: Vec<i32>,
    pub diagnostics: Vec<editor::DiagnosticItem>,
    pub diagnostic_ranges: Value,
    pub compiler: String,
    pub board: String,
    pub custom_board: String,
    pub device: String,
    pub family: String,
    pub extra_args: String,
    pub tool_path: String,
    pub incl_path: String,
    pub load_compiler: bool,
    pub load_breakp: bool,
    pub open_files: bool,
    pub circuit: String,
    pub file_list: String,
    pub debug_line: i32,
}

impl Document {
    pub fn untitled(title: String) -> Self {
        Self {
            title,
            path: String::new(),
            backup_id: backup::new_untitled_id(),
            text: String::new(),
            dirty: false,
            syntax: None,
            breakpoints: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            diagnostics: Vec::new(),
            diagnostic_ranges: json!([]),
            compiler: "None".into(),
            board: "Uno".into(),
            custom_board: String::new(),
            device: String::new(),
            family: String::new(),
            extra_args: String::new(),
            tool_path: String::new(),
            incl_path: String::new(),
            load_compiler: false,
            load_breakp: false,
            open_files: false,
            circuit: String::new(),
            file_list: String::new(),
            debug_line: 0,
        }
    }

    pub fn has_compiler(&self) -> bool {
        if !self.compiler.is_empty() && self.compiler != "None" {
            return true;
        }
        let lower = self.path.to_ascii_lowercase();
        lower.ends_with(".ino")
            || lower.ends_with(".pde")
            || lower.ends_with(".as")
            || lower.ends_with(".gcb")
    }

    pub fn has_settings(&self) -> bool {
        !self.path.to_ascii_lowercase().ends_with(".cfg")
    }

    pub fn recovery_id(&self, project: &str) -> String {
        if self.path.is_empty() {
            self.backup_id.clone()
        } else {
            backup::id_for_path(&self.path, project)
        }
    }

    pub fn to_cfg(&self) -> FileConfig {
        FileConfig {
            compiler: self.compiler.clone(),
            load_compiler: self.load_compiler,
            load_breakp: self.load_breakp,
            open_files: self.open_files,
            circuit: self.circuit.clone(),
            file_list: self.file_list.clone(),
            breakpoints: self.breakpoints.clone(),
            board: self.board.clone(),
            custom_board: self.custom_board.clone(),
            device: self.device.clone(),
            family: self.family.clone(),
            extra_args: self.extra_args.clone(),
            tool_path: self.tool_path.clone(),
            incl_path: self.incl_path.clone(),
        }
    }

    pub fn apply_cfg(
        &mut self,
        cfg: FileConfig,
        specs: &BTreeMap<String, CompilerSpec>,
        settings: &AppSettings,
        arduino_boards: &[BoardEntry],
    ) {
        let default_compiler = {
            let lower = self.path.to_ascii_lowercase();
            if lower.ends_with(".ino") || lower.ends_with(".pde") {
                "Arduino"
            } else if lower.ends_with(".as") {
                "AScript"
            } else if lower.ends_with(".gcb") {
                "GcBasic"
            } else {
                "None"
            }
        };

        if !cfg.compiler.is_empty() && cfg.compiler != "None" {
            self.compiler = cfg.compiler;
        } else if default_compiler != "None" {
            self.compiler = default_compiler.to_string();
        } else {
            self.compiler = "None".to_string();
        }
        self.load_compiler = cfg.load_compiler;
        self.load_breakp = cfg.load_breakp;
        self.open_files = cfg.open_files;
        self.circuit = cfg.circuit;
        self.file_list = cfg.file_list;
        self.breakpoints = cfg.breakpoints;
        self.family = cfg.family;
        self.extra_args = cfg.extra_args;
        self.tool_path = if cfg.tool_path.is_empty() {
            settings
                .compiler_tool_paths
                .get(&self.compiler)
                .cloned()
                .unwrap_or_default()
        } else {
            cfg.tool_path
        };
        if self.tool_path.is_empty() && self.compiler.eq_ignore_ascii_case("arduino") {
            if let Some(cand) = cs_engine::editor::arduino::tool_path_candidates().first() {
                self.tool_path = cand.clone();
            }
        }
        self.incl_path = if cfg.incl_path.is_empty() {
            settings
                .compiler_incl_paths
                .get(&self.compiler)
                .cloned()
                .unwrap_or_default()
        } else {
            cfg.incl_path
        };
        if let Some(spec) = specs.get(&self.compiler) {
            if self.tool_path.is_empty() {
                self.tool_path = settings
                    .compiler_tool_paths
                    .get(&self.compiler)
                    .cloned()
                    .unwrap_or_default();
            }
            if let Some(syn) = &spec.syntax {
                self.syntax = Some(format!("{syn}.syntax"));
            }
        }

        // Match board settings for Arduino or when board / device is saved
        let header_board = extract_in_file_value(&self.text, "board");
        let header_device = extract_in_file_value(&self.text, "device");
        let user_board = settings
            .compiler_boards
            .get(&self.compiler)
            .cloned()
            .unwrap_or_default();
        let user_custom = settings
            .compiler_custom_boards
            .get(&self.compiler)
            .cloned()
            .unwrap_or_default();

        let query_board = if !cfg.board.is_empty() {
            cfg.board
        } else if !header_board.is_empty() {
            header_board
        } else {
            user_board
        };
        let query_custom = if !cfg.custom_board.is_empty() {
            cfg.custom_board
        } else {
            user_custom
        };
        let query_device = if !cfg.device.is_empty() {
            cfg.device
        } else {
            header_device
        };

        let is_arduino = self.compiler.eq_ignore_ascii_case("arduino");
        if is_arduino || !query_board.is_empty() || !query_custom.is_empty() {
            let (b, cb, dev) =
                match_board(&query_board, &query_custom, &query_device, arduino_boards);
            self.board = b;
            self.custom_board = cb;
            self.device = if !dev.is_empty() {
                dev
            } else if !query_device.is_empty() {
                query_device
            } else {
                self.device.clone()
            };
        } else {
            self.board = query_board;
            self.custom_board = query_custom;
            self.device = query_device;
        }
    }
}
