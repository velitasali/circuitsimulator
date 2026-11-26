//! Editor panel façade. Documents are metadata JSON; the current buffer is
//! `currentText` so typing does not rebuild the tab Repeater.

pub mod compiler;
pub mod document;
pub mod lsp;
pub mod search;

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::mem;
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::path_util::strip_file_url;
use compiler::{PendingCompile, PendingSimAction, QueuedCompile};
use cs_engine::backup;
use cs_engine::debug::DebugSession;
use cs_engine::editor::{
    self, BoardEntry, CompilerSpec, FileWatcher, LspClient, LspEvent, WatchEvent,
    board_names_for_ui, calculate_newline, compiler_names, default_boards,
    device_for_board_or_fqbn, find_all, find_next, identifier_prefix, language_id_for_path,
    load_cfg, load_compilers, replace_all,
};
use cs_engine::highlighter;
use cs_engine::settings::AppSettings;
use cs_engine::theme::{ColorId, ColorTheme};
use document::Document;
use lsp::escape_html;
use qtbridge::qobject;
use serde_json::{Value, json};

pub struct EditorPanel {
    docs: Vec<Document>,
    next_doc_id: i32,
    current: i32,
    dark: bool,
    highlight_html: String,
    highlight_start_line: i32,
    highlight_viewport_start: usize,
    highlight_viewport_end: usize,
    font_family: String,
    font_size: i32,
    tab_size: i32,
    space_tabs: bool,
    show_spaces: bool,
    close_paren: bool,
    close_braces: bool,
    close_brackets: bool,
    close_quotes: bool,
    close_squotes: bool,
    focused: bool,
    cursor_pos: i32,
    completions: Value,
    completion_visible: bool,
    completion_index: i32,
    completion_prefix: String,
    completion_pending: bool,
    accepting_completion: bool,
    goto_line: i32,
    select_start: i32,
    select_end: i32,
    find_text: String,
    replace_text: String,
    find_case: bool,
    find_words: bool,
    find_regexp: bool,
    find_visible: bool,
    found_ranges: Value,
    diagnostic_ranges: Value,
    hover_html: String,
    pending_hover_diag: String,
    hover_gutter: bool,
    pending_hover_pos: i32,
    hover_link_start: i32,
    hover_link_end: i32,
    hover_link_color: String,
    file_settings_visible: bool,
    compiler_settings_visible: bool,
    tool_path_candidates: Vec<String>,
    tool_path_warning: String,
    lsp_ready: bool,
    edit_action: String,
    last_firmware: String,
    last_upload_target: String,
    firmware_mcus: BTreeMap<String, Vec<String>>,
    firmware_device_uids: BTreeMap<String, Vec<String>>,
    pending_log: Vec<String>,
    compiler_list: Vec<String>,
    specs: BTreeMap<String, CompilerSpec>,
    lsp: LspClient,
    show_lsp_debug: bool,
    file_watcher: FileWatcher,
    signature_html: String,
    signature_visible: bool,
    current_signature_label: String,
    current_signature_active_param: i32,
    current_parameters: Vec<String>,
    disk_prompt_path: String,
    disk_prompt_kind: String,
    disk_prompt_visible: bool,
    pending_flush: bool,
    text_color: String,
    current_line_color: String,
    debug_line_color: String,
    gutter_num_color: String,
    gutter_active_num_color: String,
    gutter_border_color: String,
    found_color: String,
    error_color: String,
    warn_color: String,
    arduino_boards: Vec<BoardEntry>,
    arduino_list_path: Option<String>,
    board_rx: Option<Receiver<(String, Vec<BoardEntry>)>>,
    board_loading_path: Option<String>,
    compiling: bool,
    compile_state: String,
    compile_file: String,
    compile_error_count: i32,
    compile_warning_count: i32,
    compile_cancel_requested: bool,
    compile_job: Option<PendingCompile>,
    compile_queue: VecDeque<QueuedCompile>,
    pending_sim_action: Option<PendingSimAction>,
    batch_log_opened: bool,
    batch_any_handled: bool,
    last_debug_line: i32,
    last_debug_state_gen: u64,
    pending_close_index: Option<usize>,
    pending_close_queue: Vec<usize>,
    project_path: String,
    initial_loaded: bool,
}

impl Default for EditorPanel {
    fn default() -> Self {
        let s = cs_engine::settings::get();
        let specs = load_compilers(&s);
        let compiler_list = compiler_names(&specs);
        let arduino_boards = default_boards();
        let mut panel = Self {
            docs: Vec::new(),
            next_doc_id: 1,
            current: -1,
            dark: false,
            highlight_html: String::new(),
            highlight_start_line: 0,
            highlight_viewport_start: 0,
            highlight_viewport_end: 200,
            font_family: s.editor_font_family.clone(),
            font_size: s.editor_font_size,
            tab_size: s.editor_tab_size,
            space_tabs: s.editor_space_tabs,
            show_spaces: s.editor_show_spaces,
            close_paren: s.editor_close_paren,
            close_braces: s.editor_close_braces,
            close_brackets: s.editor_close_brackets,
            close_quotes: s.editor_close_quotes,
            close_squotes: s.editor_close_squotes,
            focused: false,
            cursor_pos: 0,
            completions: json!([]),
            completion_visible: false,
            completion_index: 0,
            completion_prefix: String::new(),
            completion_pending: false,
            accepting_completion: false,
            goto_line: 0,
            select_start: -1,
            select_end: -1,
            find_text: String::new(),
            replace_text: String::new(),
            find_case: false,
            find_words: false,
            find_regexp: false,
            find_visible: false,
            found_ranges: json!([]),
            diagnostic_ranges: json!([]),
            hover_html: String::new(),
            pending_hover_diag: String::new(),
            hover_gutter: false,
            pending_hover_pos: -1,
            hover_link_start: -1,
            hover_link_end: -1,
            hover_link_color: String::new(),
            file_settings_visible: false,
            compiler_settings_visible: false,
            tool_path_candidates: Vec::new(),
            tool_path_warning: String::new(),
            lsp_ready: false,
            edit_action: String::new(),
            last_firmware: String::new(),
            last_upload_target: String::new(),
            firmware_mcus: BTreeMap::new(),
            firmware_device_uids: BTreeMap::new(),
            pending_log: Vec::new(),
            compiler_list,
            specs,
            lsp: LspClient::default(),
            show_lsp_debug: s.editor_show_lsp_debug,
            file_watcher: FileWatcher::new(),
            signature_html: String::new(),
            signature_visible: false,
            current_signature_label: String::new(),
            current_signature_active_param: -1,
            current_parameters: Vec::new(),
            disk_prompt_path: String::new(),
            disk_prompt_kind: String::new(),
            disk_prompt_visible: false,
            pending_flush: false,
            text_color: String::new(),
            current_line_color: String::new(),
            debug_line_color: String::new(),
            gutter_num_color: String::new(),
            gutter_active_num_color: String::new(),
            gutter_border_color: String::new(),
            found_color: String::new(),
            error_color: String::new(),
            warn_color: String::new(),
            arduino_boards,
            arduino_list_path: None,
            board_rx: None,
            board_loading_path: None,
            compiling: false,
            compile_state: "idle".to_string(),
            compile_file: String::new(),
            compile_error_count: 0,
            compile_warning_count: 0,
            compile_cancel_requested: false,
            compile_job: None,
            compile_queue: VecDeque::new(),
            pending_sim_action: None,
            batch_log_opened: false,
            batch_any_handled: false,
            last_debug_line: 0,
            last_debug_state_gen: 0,
            pending_close_index: None,
            pending_close_queue: Vec::new(),
            project_path: String::new(),
            initial_loaded: false,
        };
        panel.refresh_theme_colors();
        if let Some(path) = crate::pending::OPEN_EDITOR.get() {
            panel.open_path(path.clone());
        }
        panel
    }
}

impl EditorPanel {
    pub(super) fn current_doc(&self) -> Option<&Document> {
        self.docs.get(self.current as usize)
    }

    pub(super) fn current_doc_mut(&mut self) -> Option<&mut Document> {
        self.docs.get_mut(self.current as usize)
    }

    fn alloc_doc_id(&mut self) -> i32 {
        let id = self.next_doc_id;
        self.next_doc_id = self.next_doc_id.wrapping_add(1).max(1);
        id
    }

    fn clear_transient_navigation(&mut self) {
        self.goto_line = 0;
        self.select_start = -1;
        self.select_end = -1;
    }

    fn untitled_name(&self) -> String {
        let n = self.docs.iter().filter(|d| d.path.is_empty()).count() + 1;
        let base = cs_engine::i18n::tr("untitled");
        if n == 1 { base } else { format!("{base} {n}") }
    }

    fn refresh_theme_colors(&mut self) {
        let dark = self.dark;
        self.text_color = ColorTheme::get_hex(ColorId::EditorText, dark);
        self.current_line_color = ColorTheme::get_hex(ColorId::EditorCurrentLine, dark);
        self.debug_line_color = ColorTheme::get_hex(ColorId::EditorDebugLine, dark);
        self.gutter_num_color = ColorTheme::get_hex(ColorId::EditorGutterNum, dark);
        self.gutter_active_num_color = ColorTheme::get_hex(ColorId::EditorGutterActiveNum, dark);
        self.gutter_border_color = ColorTheme::get_hex(ColorId::EditorGutterBorder, dark);
        self.found_color = ColorTheme::get_hex(ColorId::EditorFound, dark);
        self.error_color = ColorTheme::get_hex(ColorId::EditorErrorUnderline, dark);
        self.warn_color = ColorTheme::get_hex(ColorId::EditorWarnUnderline, dark);
        self.hover_link_color = ColorTheme::get_hex(ColorId::EditorInfoUnderline, dark);
        for d in &mut self.docs {
            if !d.diagnostics.is_empty() {
                let ranges = editor::compute_diagnostic_ranges(&d.text, &d.diagnostics, dark);
                d.diagnostic_ranges = serde_json::to_value(&ranges).unwrap_or(json!([]));
            }
        }
        if let Some(d) = self.current_doc() {
            self.diagnostic_ranges = d.diagnostic_ranges.clone();
        }
    }

    fn render_highlight_window(&mut self) {
        let (start_line, html) = if let Some(d) = self.current_doc() {
            let total = d.highlight_cache.line_count();
            let (win_start, win_end) = if total <= 200 {
                (0, total)
            } else {
                let s = self.highlight_viewport_start.min(total.saturating_sub(1));
                let e = self.highlight_viewport_end.max(s + 1).min(total);
                (s, e)
            };
            d.highlight_cache.render_range(
                win_start,
                win_end,
                &self.font_family,
                self.font_size,
                self.dark,
            )
        } else {
            (
                0,
                highlighter::highlight_html(
                    "",
                    None,
                    self.dark,
                    self.show_spaces,
                    &self.font_family,
                    self.font_size,
                ),
            )
        };
        let start_i32 = start_line as i32;
        if self.highlight_start_line != start_i32 || self.highlight_html != html {
            self.highlight_start_line = start_i32;
            self.highlight_html = html;
            self.highlight_changed();
        }
    }

    fn rehighlight(&mut self) {
        let dark = self.dark;
        let show_spaces = self.show_spaces;
        if let Some(d) = self.current_doc_mut() {
            d.highlight_cache
                .update(&d.text, d.syntax.as_deref(), dark, show_spaces);
        }
        self.render_highlight_window();
    }

    fn apply_editor_settings(&mut self, s: &AppSettings) {
        self.font_family = s.editor_font_family.clone();
        self.font_size = s.editor_font_size;
        self.tab_size = s.editor_tab_size;
        self.space_tabs = s.editor_space_tabs;
        self.show_spaces = s.editor_show_spaces;
        self.close_paren = s.editor_close_paren;
        self.close_braces = s.editor_close_braces;
        self.close_brackets = s.editor_close_brackets;
        self.close_quotes = s.editor_close_quotes;
        self.close_squotes = s.editor_close_squotes;
        self.show_lsp_debug = s.editor_show_lsp_debug;
        self.editor_settings_changed();
        self.rehighlight();
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        let line = line.into();
        cs_engine::logging::log_compiler(&line);
        self.pending_log.push(line);
        self.log_changed();
    }

    pub fn push_log_buffered(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.pending_log.push(line);
    }

    pub fn open_path(&mut self, path: String) {
        self.open_path_inner(path, true);
    }

    fn open_path_inner(&mut self, path: String, record_in_session: bool) {
        let path = strip_file_url(&path);
        let norm_target = normalize_debug_path(Path::new(&path));
        if let Some((i, _)) = self.docs.iter().enumerate().find(|(_, d)| {
            !d.path.is_empty()
                && (d.path == path || normalize_debug_path(Path::new(&d.path)) == norm_target)
        }) {
            self.set_current_document(i as i32);
            self.request_show();
            self.request_focus();
            return;
        }
        let disk = fs::read_to_string(&path).unwrap_or_default();
        let (text, dirty) = match backup::get_file_for_path(&path, &self.project_path) {
            Some(draft) if draft.text != disk => (draft.text, true),
            Some(draft) => {
                backup::clear_file(&draft.id);
                (disk, false)
            }
            None => (disk, false),
        };
        let title = Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();
        let syntax = highlighter::syntax_for_path(&path).map(|s| s.to_string());
        let doc_id = self.alloc_doc_id();
        let mut doc = Document::untitled(doc_id, title);
        doc.path = path.clone();
        doc.backup_id = backup::id_for_path(&path, &self.project_path);
        doc.text = text;
        doc.dirty = dirty;
        doc.syntax = syntax;
        let cfg = load_cfg(&path);
        let settings = cs_engine::settings::get();
        doc.apply_cfg(cfg, &self.specs, &settings, &self.arduino_boards);
        if doc.syntax.is_none() && doc.compiler.eq_ignore_ascii_case("arduino") {
            doc.syntax = Some("c.syntax".to_string());
        }
        let initial_bps: Vec<usize> = doc
            .breakpoints
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| l as usize)
            .collect();
        self.docs.push(doc);
        self.current = self.docs.len() as i32 - 1;
        self.clear_transient_navigation();
        self.hide_completions();
        if !path.is_empty() {
            if !initial_bps.is_empty() {
                DebugSession::global().set_breakpoints_for_file(Path::new(&path), &initial_bps);
            }
            if record_in_session {
                cs_engine::project::update_session(&self.project_path, |s| {
                    if !s.editor_files.contains(&path) {
                        s.editor_files.push(path.clone());
                    }
                    s.active_editor_file = Some(path.clone());
                });
            }
        }
        self.refresh_arduino_boards();
        self.reapply_arduino_board();
        self.documents_changed();
        self.current_document_changed();
        self.current_file_changed();
        self.current_text_changed();
        self.marks_changed();
        self.refresh_tool_path_help();
        self.file_settings_changed();
        self.rehighlight();
        self.request_show();
        self.request_focus();
        if !path.is_empty() {
            self.file_watcher.watch(&path);
        }
        self.sync_lsp_open();
        self.update_lsp_ready();
    }

    fn is_inside_project(&self, file: &Path) -> bool {
        if self.project_path.is_empty() {
            return false;
        }
        let proj = normalize_debug_path(Path::new(&self.project_path));
        if proj.is_empty() {
            return false;
        }
        let norm = normalize_debug_path(file);
        norm == proj || norm.starts_with(&format!("{proj}/"))
    }

    fn activate_or_open_debug_file(&mut self, loc_file: &Path) -> bool {
        let norm_loc = normalize_debug_path(loc_file);
        // 1. Exact/normalized match among already-open tabs.
        if let Some((i, _)) = self.docs.iter().enumerate().find(|(_, d)| {
            !d.path.is_empty() && normalize_debug_path(Path::new(&d.path)) == norm_loc
        }) {
            if self.current != i as i32 {
                self.set_current_document(i as i32);
            }
            self.request_show();
            return true;
        }

        // 2. Existing file on disk (project header/source, Arduino core, or library source).
        if loc_file.is_file() {
            let resolved = display_clean_path(loc_file);
            let record_session = self.is_inside_project(loc_file);
            self.open_path_inner(resolved, record_session);
            return true;
        }

        // 3. Basename match among already-open tabs (prebuilt/moved ELF fixtures).
        if let Some(want_name) = loc_file.file_name().and_then(|n| n.to_str()) {
            if let Some((i, _)) = self.docs.iter().enumerate().find(|(_, d)| {
                !d.path.is_empty()
                    && Path::new(&d.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| file_names_equal(n, want_name))
            }) {
                if self.current != i as i32 {
                    self.set_current_document(i as i32);
                }
                self.request_show();
                return true;
            }

            // 4. Sibling file next to the active document or inside the project directory.
            let mut search_dirs = Vec::new();
            if let Some(d) = self.current_doc() {
                if let Some(parent) = Path::new(&d.path).parent() {
                    search_dirs.push(parent.to_path_buf());
                }
            }
            if !self.project_path.is_empty() {
                search_dirs.push(Path::new(&self.project_path).to_path_buf());
            }
            for dir in search_dirs {
                let cand = dir.join(want_name);
                if cand.is_file() {
                    let resolved = display_clean_path(&cand);
                    let record_session = self.is_inside_project(&cand);
                    self.open_path_inner(resolved, record_session);
                    return true;
                }
            }
        }

        false
    }

    fn sync_debug_location(&mut self) {
        let session = DebugSession::global();
        if !session.is_active() || !session.is_paused() {
            if self.last_debug_line != 0 {
                self.last_debug_line = 0;
                self.marks_changed();
            }
            return;
        }
        let state_gen = session.state_generation();
        if state_gen != self.last_debug_state_gen {
            self.last_debug_state_gen = state_gen;
            if let Some(loc) = session.active_location() {
                let target_line = loc.line as i32;
                let matched_tab = self.activate_or_open_debug_file(&loc.file);
                if matched_tab && target_line > 0 {
                    self.goto_1based(target_line);
                } else if !matched_tab {
                    cs_engine::logging::log_sim(format!(
                        "[Debug] Source file not found on disk: {}:{}",
                        loc.file.display(),
                        loc.line
                    ));
                }
            }
            self.last_debug_line = self.debug_line();
            self.marks_changed();
            return;
        }
        let cur_dl = self.debug_line();
        if cur_dl != self.last_debug_line {
            self.last_debug_line = cur_dl;
            self.marks_changed();
        }
    }
}

fn display_clean_path(p: &Path) -> String {
    let raw = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let s = raw.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn normalize_debug_path(p: &Path) -> String {
    let clean = display_clean_path(p).replace('\\', "/");
    let clean = clean.trim_end_matches('/').to_string();
    #[cfg(windows)]
    {
        clean.to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        clean
    }
}

fn file_names_equal(a: &str, b: &str) -> bool {
    #[cfg(windows)]
    {
        a.eq_ignore_ascii_case(b)
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

fn debug_paths_match(doc_path: &str, loc_file: &Path) -> bool {
    if doc_path.is_empty() {
        return false;
    }
    let dp = Path::new(doc_path);
    if normalize_debug_path(dp) == normalize_debug_path(loc_file) {
        return true;
    }
    // Only fall back to basename matching when the DWARF path does not exist on disk
    // (e.g. moved projects or prebuilt ELF fixtures), so `sketch/main.cpp` never
    // collides with `arduino-core/main.cpp`.
    if !loc_file.is_file() {
        if let (Some(a), Some(b)) = (
            dp.file_name().and_then(|n| n.to_str()),
            loc_file.file_name().and_then(|n| n.to_str()),
        ) {
            return file_names_equal(a, b);
        }
    }
    false
}

#[qobject(Singleton, ConvertToCamelCase)]
impl EditorPanel {
    qproperty!(
        "projectPath",
        Read = project_path,
        Write = set_project_path,
        Notify = project_path_changed
    );
    qproperty!("documents", Read = documents, Notify = documents_changed);
    qproperty!(
        "currentDocument",
        Read = current_document,
        Write = set_current_document,
        Notify = current_document_changed
    );
    qproperty!(
        "currentDocId",
        Read = current_doc_id,
        Notify = current_document_changed
    );
    qproperty!(
        "currentText",
        Read = current_text,
        Write = set_current_text,
        Notify = current_text_changed
    );
    qproperty!(
        "currentTitle",
        Read = current_title,
        Notify = current_file_changed
    );
    qproperty!(
        "currentPath",
        Read = current_path,
        Notify = current_file_changed
    );
    qproperty!(
        "highlightHtml",
        Read = highlight_html,
        Notify = highlight_changed
    );
    qproperty!(
        "highlightStartLine",
        Read = highlight_start_line,
        Notify = highlight_changed
    );
    qproperty!(
        "dark",
        Read = dark,
        Write = set_dark,
        Notify = highlight_changed
    );
    qproperty!("textColor", Read = text_color, Notify = highlight_changed);
    qproperty!(
        "currentLineColor",
        Read = current_line_color,
        Notify = highlight_changed
    );
    qproperty!(
        "debugLineColor",
        Read = debug_line_color,
        Notify = highlight_changed
    );
    qproperty!(
        "gutterNumColor",
        Read = gutter_num_color,
        Notify = highlight_changed
    );
    qproperty!(
        "gutterActiveNumColor",
        Read = gutter_active_num_color,
        Notify = highlight_changed
    );
    qproperty!(
        "gutterBorderColor",
        Read = gutter_border_color,
        Notify = highlight_changed
    );
    qproperty!("foundColor", Read = found_color, Notify = highlight_changed);
    qproperty!("errorColor", Read = error_color, Notify = highlight_changed);
    qproperty!("warnColor", Read = warn_color, Notify = highlight_changed);
    qproperty!(
        "fontFamily",
        Read = font_family,
        Notify = editor_settings_changed
    );
    qproperty!(
        "fontSize",
        Read = font_size,
        Notify = editor_settings_changed
    );
    qproperty!("tabSize", Read = tab_size, Notify = editor_settings_changed);
    qproperty!(
        "spaceTabs",
        Read = space_tabs,
        Notify = editor_settings_changed
    );
    qproperty!(
        "showSpaces",
        Read = show_spaces,
        Notify = editor_settings_changed
    );
    qproperty!(
        "closeParen",
        Read = close_paren,
        Notify = editor_settings_changed
    );
    qproperty!(
        "closeBraces",
        Read = close_braces,
        Notify = editor_settings_changed
    );
    qproperty!(
        "closeBrackets",
        Read = close_brackets,
        Notify = editor_settings_changed
    );
    qproperty!(
        "closeQuotes",
        Read = close_quotes,
        Notify = editor_settings_changed
    );
    qproperty!(
        "closeSquotes",
        Read = close_squotes,
        Notify = editor_settings_changed
    );
    qproperty!(
        "focused",
        Read = focused,
        Write = set_focused,
        Notify = focused_changed
    );
    qproperty!(
        "cursorPos",
        Read = cursor_pos,
        Write = set_cursor_pos,
        Notify = cursor_changed
    );
    qproperty!(
        "completions",
        Read = completions,
        Notify = completion_changed
    );
    qproperty!(
        "completionVisible",
        Read = completion_visible,
        Notify = completion_changed
    );
    qproperty!(
        "completionIndex",
        Read = completion_index,
        Notify = completion_changed
    );
    qproperty!(
        "completionPrefix",
        Read = completion_prefix,
        Notify = completion_changed
    );
    qproperty!("gotoLine", Read = goto_line, Notify = goto_changed);
    qproperty!("selectStart", Read = select_start, Notify = goto_changed);
    qproperty!("selectEnd", Read = select_end, Notify = goto_changed);
    qproperty!(
        "findText",
        Read = find_text,
        Write = set_find_text,
        Notify = find_changed
    );
    qproperty!(
        "replaceText",
        Read = replace_text,
        Write = set_replace_text,
        Notify = find_changed
    );
    qproperty!(
        "findCaseSensitive",
        Read = find_case_sensitive,
        Write = set_find_case,
        Notify = find_changed
    );
    qproperty!(
        "findWholeWords",
        Read = find_whole_words,
        Write = set_find_words,
        Notify = find_changed
    );
    qproperty!(
        "findRegexp",
        Read = find_regexp,
        Write = set_find_regexp,
        Notify = find_changed
    );
    qproperty!(
        "findVisible",
        Read = find_visible,
        Write = set_find_visible,
        Notify = find_changed
    );
    qproperty!("foundRanges", Read = found_ranges, Notify = find_changed);
    qproperty!(
        "diagnosticRanges",
        Read = diagnostic_ranges,
        Notify = marks_changed
    );
    qproperty!("hoverHtml", Read = hover_html, Notify = hover_changed);
    qproperty!(
        "hoverLinkStart",
        Read = hover_link_start,
        Notify = hover_link_changed
    );
    qproperty!(
        "hoverLinkEnd",
        Read = hover_link_end,
        Notify = hover_link_changed
    );
    qproperty!(
        "hoverLinkColor",
        Read = hover_link_color,
        Notify = highlight_changed
    );
    qproperty!("breakpoints", Read = breakpoints, Notify = marks_changed);
    qproperty!("errors", Read = errors, Notify = marks_changed);
    qproperty!("warnings", Read = warnings, Notify = marks_changed);
    qproperty!("debugLine", Read = debug_line, Notify = marks_changed);
    qproperty!("scrollMarks", Read = scroll_marks, Notify = marks_changed);
    qproperty!(
        "compilerName",
        Read = compiler_name,
        Write = set_compiler_name,
        Notify = file_settings_changed
    );
    qproperty!(
        "compilerNames",
        Read = compiler_names,
        Notify = file_settings_changed
    );
    qproperty!(
        "device",
        Read = device,
        Write = set_device,
        Notify = file_settings_changed
    );
    qproperty!(
        "family",
        Read = family,
        Write = set_family,
        Notify = file_settings_changed
    );
    qproperty!(
        "extraArgs",
        Read = extra_args,
        Write = set_extra_args,
        Notify = file_settings_changed
    );
    qproperty!(
        "toolPath",
        Read = tool_path,
        Write = set_tool_path,
        Notify = file_settings_changed
    );
    qproperty!(
        "inclPath",
        Read = incl_path,
        Write = set_incl_path,
        Notify = file_settings_changed
    );
    qproperty!(
        "loadCompiler",
        Read = load_compiler,
        Write = set_load_compiler,
        Notify = file_settings_changed
    );
    qproperty!(
        "loadBreakp",
        Read = load_breakp,
        Write = set_load_breakp,
        Notify = file_settings_changed
    );
    qproperty!(
        "openFiles",
        Read = open_files,
        Write = set_open_files,
        Notify = file_settings_changed
    );
    qproperty!(
        "usesDevice",
        Read = uses_device,
        Notify = file_settings_changed
    );
    qproperty!(
        "usesFamily",
        Read = uses_family,
        Notify = file_settings_changed
    );
    qproperty!(
        "usesExtraArgs",
        Read = uses_extra_args,
        Notify = file_settings_changed
    );
    qproperty!(
        "usesInclPath",
        Read = uses_incl_path,
        Notify = file_settings_changed
    );
    qproperty!(
        "hasCompiler",
        Read = has_compiler,
        Notify = file_settings_changed
    );
    qproperty!(
        "fileSettingsVisible",
        Read = file_settings_visible,
        Write = set_file_settings_visible,
        Notify = file_settings_changed
    );
    qproperty!(
        "compilerSettingsVisible",
        Read = compiler_settings_visible,
        Write = set_compiler_settings_visible,
        Notify = file_settings_changed
    );
    qproperty!(
        "toolPathSuggestions",
        Read = tool_path_suggestions,
        Notify = file_settings_changed
    );
    qproperty!(
        "toolPathWarning",
        Read = tool_path_warning,
        Notify = file_settings_changed
    );
    qproperty!(
        "isArduino",
        Read = is_arduino,
        Notify = file_settings_changed
    );
    qproperty!(
        "board",
        Read = board,
        Write = set_board,
        Notify = file_settings_changed
    );
    qproperty!(
        "customBoard",
        Read = custom_board,
        Write = set_custom_board,
        Notify = file_settings_changed
    );
    qproperty!(
        "boardList",
        Read = board_list,
        Notify = file_settings_changed
    );
    qproperty!(
        "isCustomBoard",
        Read = is_custom_board,
        Notify = file_settings_changed
    );
    qproperty!(
        "usesBoard",
        Read = uses_board,
        Notify = file_settings_changed
    );
    qproperty!("lspReady", Read = lsp_ready, Notify = lsp_ready_changed);
    qproperty!(
        "editAction",
        Read = edit_action,
        Notify = edit_action_changed
    );
    qproperty!(
        "lastFirmware",
        Read = last_firmware,
        Notify = firmware_changed
    );
    qproperty!(
        "lastUploadTarget",
        Read = last_upload_target,
        Notify = firmware_changed
    );
    qproperty!("compiling", Read = compiling, Notify = compiling_changed);
    qproperty!(
        "compileStatus",
        Read = compile_status,
        Notify = compiling_changed
    );
    qproperty!(
        "boardLoading",
        Read = board_loading,
        Notify = compiling_changed
    );
    qproperty!(
        "signatureHtml",
        Read = signature_html,
        Notify = signature_changed
    );
    qproperty!(
        "signatureVisible",
        Read = signature_visible,
        Notify = signature_changed
    );
    qproperty!(
        "diskPromptPath",
        Read = disk_prompt_path,
        Notify = disk_prompt_changed
    );
    qproperty!(
        "diskPromptKind",
        Read = disk_prompt_kind,
        Notify = disk_prompt_changed
    );
    qproperty!(
        "diskPromptVisible",
        Read = disk_prompt_visible,
        Notify = disk_prompt_changed
    );

    #[qsignal]
    pub fn project_path_changed(&mut self);
    #[qsignal]
    pub fn documents_changed(&mut self);
    #[qsignal]
    pub fn current_document_changed(&mut self);
    #[qsignal]
    pub fn current_file_changed(&mut self);
    #[qsignal]
    pub fn current_text_changed(&mut self);
    #[qsignal]
    pub fn signature_changed(&mut self);
    #[qsignal]
    pub fn disk_prompt_changed(&mut self);
    #[qsignal]
    pub fn request_show(&mut self);
    #[qsignal]
    pub fn request_hide(&mut self);
    #[qsignal]
    pub fn request_focus(&mut self);
    #[qsignal]
    pub fn highlight_changed(&mut self);
    #[qsignal]
    pub fn editor_settings_changed(&mut self);
    #[qsignal]
    pub fn request_save_as(&mut self);
    #[qsignal]
    pub fn focused_changed(&mut self);
    #[qsignal]
    pub fn cursor_changed(&mut self);
    #[qsignal]
    pub fn completion_changed(&mut self);
    #[qsignal]
    pub fn goto_changed(&mut self);
    #[qsignal]
    pub fn find_changed(&mut self);
    #[qsignal]
    pub fn request_show_find(&mut self);
    #[qsignal]
    pub fn hover_changed(&mut self);
    #[qsignal]
    pub fn hover_link_changed(&mut self);
    #[qsignal]
    pub fn marks_changed(&mut self);
    #[qsignal]
    pub fn file_settings_changed(&mut self);
    #[qsignal]
    pub fn request_show_file_settings(&mut self);
    #[qsignal]
    pub fn request_show_compiler_settings(&mut self);
    #[qsignal]
    pub fn lsp_ready_changed(&mut self);
    #[qsignal]
    pub fn edit_action_changed(&mut self);
    #[qsignal]
    pub fn firmware_changed(&mut self);
    #[qsignal]
    pub fn compiling_changed(&mut self);
    #[qsignal]
    pub fn log_changed(&mut self);
    #[qsignal]
    pub fn request_compiler_log(&mut self);
    #[qsignal]
    pub fn request_upload(&mut self);
    #[qsignal]
    pub fn request_upload_to(&mut self, path: String, target_id: String);
    #[qsignal]
    pub fn request_power_on(&mut self);
    #[qsignal]
    pub fn request_restart_power_on(&mut self);
    #[qsignal]
    pub fn request_debug_run(&mut self);
    #[qsignal]
    pub fn debug_state_changed(&mut self);
    #[qsignal]
    pub fn request_save_before_close(&mut self);
    #[qsignal]
    pub fn request_save_as_then_close(&mut self);

    pub fn documents(&self) -> Value {
        let arr: Vec<Value> = self
            .docs
            .iter()
            .map(|d| {
                let mcu_names = if d.path.is_empty() {
                    String::new()
                } else {
                    let norm = normalize_debug_path(Path::new(&d.path));
                    self.firmware_mcus
                        .get(&norm)
                        .map(|names| names.join(", "))
                        .unwrap_or_default()
                };
                json!({
                    "id": d.id,
                    "title": d.title,
                    "path": d.path,
                    "dirty": d.dirty,
                    "hasSettings": d.has_settings(),
                    "hasCompiler": d.has_compiler(),
                    "mcuNames": mcu_names,
                })
            })
            .collect();
        Value::Array(arr)
    }

    pub fn completions(&self) -> Value {
        self.completions.clone()
    }

    pub fn found_ranges(&self) -> Value {
        self.found_ranges.clone()
    }

    pub fn diagnostic_ranges(&self) -> Value {
        self.diagnostic_ranges.clone()
    }

    pub fn compiler_names(&self) -> Vec<String> {
        self.compiler_list.clone()
    }

    pub fn highlight_html(&self) -> String {
        self.highlight_html.clone()
    }
    pub fn highlight_start_line(&self) -> i32 {
        self.highlight_start_line
    }
    pub fn dark(&self) -> bool {
        self.dark
    }
    pub fn text_color(&self) -> String {
        self.text_color.clone()
    }
    pub fn current_line_color(&self) -> String {
        self.current_line_color.clone()
    }
    pub fn debug_line_color(&self) -> String {
        self.debug_line_color.clone()
    }
    pub fn gutter_num_color(&self) -> String {
        self.gutter_num_color.clone()
    }
    pub fn gutter_active_num_color(&self) -> String {
        self.gutter_active_num_color.clone()
    }
    pub fn gutter_border_color(&self) -> String {
        self.gutter_border_color.clone()
    }
    pub fn found_color(&self) -> String {
        self.found_color.clone()
    }
    pub fn error_color(&self) -> String {
        self.error_color.clone()
    }
    pub fn warn_color(&self) -> String {
        self.warn_color.clone()
    }
    pub fn font_family(&self) -> String {
        self.font_family.clone()
    }
    pub fn font_size(&self) -> i32 {
        self.font_size
    }
    pub fn tab_size(&self) -> i32 {
        self.tab_size
    }
    pub fn space_tabs(&self) -> bool {
        self.space_tabs
    }
    pub fn show_spaces(&self) -> bool {
        self.show_spaces
    }
    pub fn close_paren(&self) -> bool {
        self.close_paren
    }
    pub fn close_braces(&self) -> bool {
        self.close_braces
    }
    pub fn close_brackets(&self) -> bool {
        self.close_brackets
    }
    pub fn close_quotes(&self) -> bool {
        self.close_quotes
    }
    pub fn close_squotes(&self) -> bool {
        self.close_squotes
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn cursor_pos(&self) -> i32 {
        self.cursor_pos
    }
    pub fn completion_visible(&self) -> bool {
        self.completion_visible
    }
    pub fn completion_index(&self) -> i32 {
        self.completion_index
    }
    pub fn completion_prefix(&self) -> String {
        self.completion_prefix.clone()
    }
    pub fn goto_line(&self) -> i32 {
        self.goto_line
    }
    pub fn select_start(&self) -> i32 {
        self.select_start
    }
    pub fn select_end(&self) -> i32 {
        self.select_end
    }
    pub fn find_text(&self) -> String {
        self.find_text.clone()
    }
    pub fn replace_text(&self) -> String {
        self.replace_text.clone()
    }
    pub fn find_case_sensitive(&self) -> bool {
        self.find_case
    }
    pub fn find_whole_words(&self) -> bool {
        self.find_words
    }
    pub fn find_regexp(&self) -> bool {
        self.find_regexp
    }
    pub fn find_visible(&self) -> bool {
        self.find_visible
    }
    pub fn hover_html(&self) -> String {
        self.hover_html.clone()
    }
    pub fn hover_link_start(&self) -> i32 {
        self.hover_link_start
    }
    pub fn hover_link_end(&self) -> i32 {
        self.hover_link_end
    }
    pub fn hover_link_color(&self) -> String {
        self.hover_link_color.clone()
    }
    pub fn file_settings_visible(&self) -> bool {
        self.file_settings_visible
    }
    pub fn compiler_settings_visible(&self) -> bool {
        self.compiler_settings_visible
    }
    pub fn tool_path_warning(&self) -> String {
        self.tool_path_warning.clone()
    }
    pub fn lsp_ready(&self) -> bool {
        self.lsp_ready
    }
    pub fn edit_action(&self) -> String {
        self.edit_action.clone()
    }
    pub fn last_firmware(&self) -> String {
        self.last_firmware.clone()
    }
    pub fn last_upload_target(&self) -> String {
        self.last_upload_target.clone()
    }
    pub fn compiling(&self) -> bool {
        self.compiling
    }
    pub fn compile_status(&self) -> Value {
        json!({
            "state": self.compile_state,
            "file": self.compile_file,
            "errors": self.compile_error_count,
            "warnings": self.compile_warning_count,
        })
    }
    pub fn board_loading(&self) -> bool {
        self.board_rx.is_some()
    }
    pub fn signature_html(&self) -> String {
        self.signature_html.clone()
    }
    pub fn signature_visible(&self) -> bool {
        self.signature_visible
    }
    pub fn disk_prompt_path(&self) -> String {
        self.disk_prompt_path.clone()
    }
    pub fn disk_prompt_kind(&self) -> String {
        self.disk_prompt_kind.clone()
    }
    pub fn disk_prompt_visible(&self) -> bool {
        self.disk_prompt_visible
    }

    pub fn current_document(&self) -> i32 {
        self.current
    }

    pub fn current_doc_id(&self) -> i32 {
        self.current_doc().map(|d| d.id).unwrap_or(-1)
    }

    pub fn set_current_document(&mut self, index: i32) {
        if index == self.current {
            return;
        }
        if index < 0 || index as usize >= self.docs.len() {
            return;
        }
        self.current = index;
        self.clear_transient_navigation();
        self.diagnostic_ranges = self
            .current_doc()
            .map(|d| d.diagnostic_ranges.clone())
            .unwrap_or(json!([]));
        self.pending_hover_pos = -1;
        self.pending_hover_diag.clear();
        self.hover_gutter = false;
        if !self.hover_html.is_empty() {
            self.hover_html.clear();
            self.hover_changed();
        }
        self.clear_hover_link();
        self.hide_completions();
        self.current_document_changed();
        self.current_file_changed();
        self.current_text_changed();
        self.marks_changed();
        self.refresh_arduino_boards();
        self.refresh_tool_path_help();
        self.file_settings_changed();
        self.rehighlight();
        self.sync_lsp_open();
        self.update_lsp_ready();
        if let Some(path) = self
            .current_doc()
            .map(|d| d.path.clone())
            .filter(|p| !p.is_empty())
        {
            cs_engine::project::update_session(&self.project_path, |s| {
                s.active_editor_file = Some(path);
            });
        }
    }

    pub fn project_path(&self) -> String {
        self.project_path.clone()
    }

    pub fn set_project_path(&mut self, new_project: String) {
        let new_project = cs_engine::project::normalize_path(&new_project);
        if self.initial_loaded && self.project_path == new_project {
            return;
        }
        let is_first = !self.initial_loaded;
        self.initial_loaded = true;

        if !is_first {
            self.flush_drafts();
            self.close_all_docs_internal();
        }

        let from_cli = crate::pending::OPEN_EDITOR.get().is_some() && is_first;

        self.project_path = new_project.clone();
        self.project_path_changed();

        if from_cli {
            let editor_files: Vec<String> = self
                .docs
                .iter()
                .filter(|d| !d.path.is_empty())
                .map(|d| d.path.clone())
                .collect();
            let active_editor_file = self
                .current_doc()
                .filter(|d| !d.path.is_empty())
                .map(|d| d.path.clone());
            cs_engine::project::update_session(&new_project, |s| {
                s.editor_files = editor_files;
                s.active_editor_file = active_editor_file;
            });
        } else if !crate::pending::no_project() {
            if let Some(mut sess) = cs_engine::project::get_session(&new_project) {
                cs_engine::project::cleanup_missing_files(&mut sess);
                for path in sess.editor_files {
                    self.open_path(path);
                }
                if let Some(active) = sess.active_editor_file {
                    if let Some((idx, _)) =
                        self.docs.iter().enumerate().find(|(_, d)| d.path == active)
                    {
                        self.set_current_document(idx as i32);
                    }
                }
            }
        }
    }

    fn close_all_docs_internal(&mut self) {
        for doc in &self.docs {
            if !doc.path.is_empty() {
                self.file_watcher.unwatch(&doc.path);
            }
        }
        self.docs.clear();
        self.current = -1;
        self.clear_transient_navigation();
        self.hide_completions();
        self.pending_close_index = None;
        self.pending_close_queue.clear();
        self.diagnostic_ranges = json!([]);
        self.pending_hover_diag.clear();
        self.documents_changed();
        self.current_document_changed();
        self.current_file_changed();
        self.current_text_changed();
        self.marks_changed();
        self.file_settings_changed();
        self.rehighlight();
        self.request_hide();
    }

    pub fn current_text(&self) -> String {
        self.current_doc()
            .map(|d| d.text.clone())
            .unwrap_or_default()
    }

    pub fn set_current_text(&mut self, text: String) {
        self.apply_text_edit(text);
    }

    fn apply_text_edit(&mut self, text: String) {
        let mut changed = false;
        let mut dirty_toggled = false;
        let mut marks_need_update = false;
        if let Some(d) = self.current_doc_mut() {
            if d.text != text {
                let old_lines = editor::line_count(&d.text);
                let new_lines = editor::line_count(&text);
                let had_marks =
                    !d.breakpoints.is_empty() || !d.errors.is_empty() || !d.warnings.is_empty();
                marks_need_update = had_marks && old_lines != new_lines;
                dirty_toggled = !d.dirty;
                d.text = text;
                d.dirty = true;
                self.pending_hover_pos = -1;
                self.pending_hover_diag.clear();
                self.clear_hover_link();
                changed = true;
            }
        }
        if changed {
            self.pending_flush = true;
            self.current_text_changed();
            if dirty_toggled {
                self.documents_changed();
            }
            self.rehighlight();
            if marks_need_update {
                self.marks_changed();
            }
            if let Some(d) = self.current_doc() {
                if !d.path.is_empty() {
                    let path = d.path.clone();
                    let text = d.text.clone();
                    self.lsp.change_file(&path, &text);
                }
            }
            if !self.accepting_completion {
                let text = self.current_text();
                let cursor_byte =
                    editor::utf16_to_byte_offset(&text, self.cursor_pos.max(0) as usize);
                let prefix = identifier_prefix(&text, cursor_byte).to_string();
                let member_trigger = lsp::is_member_trigger(&text, cursor_byte);
                if prefix.len() >= 2 || (member_trigger && self.lsp.is_initialized()) {
                    if self.completion_visible
                        && !member_trigger
                        && prefix.len() > self.completion_prefix.len()
                        && prefix
                            .get(..self.completion_prefix.len())
                            .is_some_and(|h| h.eq_ignore_ascii_case(&self.completion_prefix))
                    {
                        if let Some(arr) = self.completions.as_array() {
                            let narrowed: Vec<Value> = arr
                                .iter()
                                .filter(|item| {
                                    item.get("text")
                                        .and_then(|v| v.as_str())
                                        .and_then(|t| t.get(..prefix.len()))
                                        .is_some_and(|h| h.eq_ignore_ascii_case(&prefix))
                                })
                                .cloned()
                                .collect();
                            if !narrowed.is_empty() {
                                self.set_completion_items(narrowed, &prefix);
                            }
                        }
                    }
                    self.complete_at(false);
                } else {
                    self.hide_completions();
                }
                self.update_signature_for_cursor();
            }
        }
    }

    pub fn current_title(&self) -> String {
        self.current_doc()
            .map(|d| d.title.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub fn current_path(&self) -> String {
        self.current_doc()
            .map(|d| d.path.clone())
            .unwrap_or_default()
    }

    fn set_dark(&mut self, dark: bool) {
        if self.dark == dark {
            return;
        }
        self.dark = dark;
        self.refresh_theme_colors();
        self.rehighlight();
        self.marks_changed();
    }

    fn set_focused(&mut self, focused: bool) {
        if self.focused == focused {
            return;
        }
        self.focused = focused;
        if !focused {
            self.hide_completions();
        }
        self.focused_changed();
    }

    fn update_signature_for_cursor(&mut self) {
        let (path, text) = match self.current_doc() {
            Some(d) => (d.path.clone(), d.text.clone()),
            None => return,
        };
        let cursor_utf16 = self.cursor_pos.max(0) as usize;
        let param_idx = editor::get_param_index_at_cursor(&text, cursor_utf16);
        if param_idx >= 0 {
            self.current_signature_active_param = param_idx;
            if !self.current_signature_label.is_empty() {
                let html = editor::format_signature_html(&editor::lsp::SignatureHelp {
                    label: self.current_signature_label.clone(),
                    active_parameter: param_idx,
                    parameters: self.current_parameters.clone(),
                });
                if self.signature_html != html || !self.signature_visible {
                    self.signature_html = html;
                    self.signature_visible = !self.signature_html.is_empty();
                    self.signature_changed();
                }
            }
            if !path.is_empty() && self.lsp.is_running() {
                let cursor_byte = editor::utf16_to_byte_offset(&text, cursor_utf16);
                let (line, col) = editor::line_col(&text, cursor_byte);
                self.lsp.request_signature_help(&path, line, col);
            }
        } else {
            if !self.current_signature_label.is_empty() || self.signature_visible {
                self.current_signature_label.clear();
                self.current_parameters.clear();
                self.current_signature_active_param = -1;
                self.signature_visible = false;
                self.signature_html.clear();
                self.signature_changed();
            }
        }
    }

    fn set_cursor_pos(&mut self, pos: i32) {
        if self.cursor_pos == pos {
            return;
        }
        self.cursor_pos = pos.max(0);
        self.cursor_changed();
        if !self.accepting_completion {
            if self.completion_visible {
                let text = self.current_text();
                let cursor_byte =
                    editor::utf16_to_byte_offset(&text, self.cursor_pos.max(0) as usize);
                let prefix = identifier_prefix(&text, cursor_byte);
                if prefix.is_empty() && !lsp::is_member_trigger(&text, cursor_byte) {
                    self.hide_completions();
                }
            }
            self.update_signature_for_cursor();
        }
    }

    pub fn breakpoints(&self) -> Value {
        json!(
            self.current_doc()
                .map(|d| d.breakpoints.clone())
                .unwrap_or_default()
        )
    }

    pub fn errors(&self) -> Value {
        json!(
            self.current_doc()
                .map(|d| d.errors.clone())
                .unwrap_or_default()
        )
    }

    pub fn warnings(&self) -> Value {
        json!(
            self.current_doc()
                .map(|d| d.warnings.clone())
                .unwrap_or_default()
        )
    }

    pub fn debug_line(&self) -> i32 {
        let session = DebugSession::global();
        if !session.is_active() || !session.is_paused() {
            return 0;
        }
        if let Some(loc) = session.active_location() {
            if let Some(d) = self.current_doc() {
                if debug_paths_match(&d.path, &loc.file) {
                    return loc.line as i32;
                }
            }
        }
        self.current_doc().map(|d| d.debug_line).unwrap_or(0)
    }

    pub fn scroll_marks(&self) -> Value {
        let Some(d) = self.current_doc() else {
            return json!([]);
        };
        let n = editor::line_count(&d.text).max(1) as f64;
        let mut marks = Vec::new();
        let push = |marks: &mut Vec<Value>, lines: &[i32], color: &str, weight: i32| {
            for line in lines {
                let pos = ((*line as f64 - 1.0) / (n - 1.0).max(1.0)).clamp(0.0, 1.0);
                marks.push(json!({ "pos": pos, "color": color, "weight": weight }));
            }
        };
        let err_color = ColorTheme::get_hex(ColorId::EditorErrorUnderline, self.dark);
        let warn_color = ColorTheme::get_hex(ColorId::EditorWarnUnderline, self.dark);
        push(&mut marks, &d.breakpoints, "#ffe014", 3);
        push(&mut marks, &d.errors, &err_color, 2);
        push(&mut marks, &d.warnings, &warn_color, 2);
        Value::Array(marks)
    }

    pub fn compiler_name(&self) -> String {
        self.current_doc()
            .map(|d| {
                if d.compiler.is_empty() || d.compiler == "None" {
                    let lower = d.path.to_ascii_lowercase();
                    if lower.ends_with(".ino") || lower.ends_with(".pde") {
                        "Arduino".into()
                    } else if lower.ends_with(".as") {
                        "AScript".into()
                    } else {
                        "None".into()
                    }
                } else {
                    d.compiler.clone()
                }
            })
            .unwrap_or_else(|| "None".into())
    }

    pub fn set_compiler_name(&mut self, name: String) {
        if let Some(d) = self.current_doc_mut() {
            if d.compiler == name {
                return;
            }
            d.compiler = name.clone();
            let settings = cs_engine::settings::get();
            d.tool_path = settings
                .compiler_tool_paths
                .get(&name)
                .cloned()
                .unwrap_or_default();
            if d.tool_path.is_empty() && name.eq_ignore_ascii_case("arduino") {
                if let Some(cand) = cs_engine::editor::arduino::tool_path_candidates().first() {
                    d.tool_path = cand.clone();
                }
            }
            d.incl_path = settings
                .compiler_incl_paths
                .get(&name)
                .cloned()
                .unwrap_or_default();
        }
        if !self.has_compiler() && self.compiler_settings_visible {
            self.compiler_settings_visible = false;
        }
        self.refresh_arduino_boards();
        self.reapply_arduino_board();
        self.refresh_tool_path_help();
        self.file_settings_changed();
        self.documents_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn device(&self) -> String {
        self.current_doc()
            .map(|d| d.device.clone())
            .unwrap_or_default()
    }

    pub fn set_device(&mut self, v: String) {
        if let Some(d) = self.current_doc_mut() {
            d.device = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn family(&self) -> String {
        self.current_doc()
            .map(|d| d.family.clone())
            .unwrap_or_default()
    }

    pub fn set_family(&mut self, v: String) {
        if let Some(d) = self.current_doc_mut() {
            d.family = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
    }

    pub fn extra_args(&self) -> String {
        self.current_doc()
            .map(|d| d.extra_args.clone())
            .unwrap_or_default()
    }

    pub fn set_extra_args(&mut self, v: String) {
        if let Some(d) = self.current_doc_mut() {
            d.extra_args = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn tool_path(&self) -> String {
        self.current_doc()
            .map(|d| d.tool_path.clone())
            .unwrap_or_default()
    }

    pub fn set_tool_path(&mut self, v: String) {
        if let Some(d) = self.current_doc_mut() {
            d.tool_path = v;
        }
        self.refresh_tool_path_help();
        self.refresh_arduino_boards();
        self.file_settings_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn incl_path(&self) -> String {
        self.current_doc()
            .map(|d| d.incl_path.clone())
            .unwrap_or_default()
    }

    pub fn set_incl_path(&mut self, v: String) {
        let v = strip_file_url(&v);
        if let Some(d) = self.current_doc_mut() {
            d.incl_path = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn load_compiler(&self) -> bool {
        self.current_doc().map(|d| d.load_compiler).unwrap_or(false)
    }

    pub fn set_load_compiler(&mut self, v: bool) {
        if let Some(d) = self.current_doc_mut() {
            d.load_compiler = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
    }

    pub fn load_breakp(&self) -> bool {
        self.current_doc().map(|d| d.load_breakp).unwrap_or(false)
    }

    pub fn set_load_breakp(&mut self, v: bool) {
        if let Some(d) = self.current_doc_mut() {
            d.load_breakp = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
    }

    pub fn open_files(&self) -> bool {
        self.current_doc().map(|d| d.open_files).unwrap_or(false)
    }

    pub fn set_open_files(&mut self, v: bool) {
        if let Some(d) = self.current_doc_mut() {
            d.open_files = v;
        }
        self.file_settings_changed();
        self.persist_cfg();
    }

    pub fn uses_device(&self) -> bool {
        self.current_doc()
            .and_then(|d| self.specs.get(&d.compiler))
            .is_some_and(|s| s.uses_device())
    }

    pub fn uses_family(&self) -> bool {
        self.current_doc()
            .and_then(|d| self.specs.get(&d.compiler))
            .is_some_and(|s| s.uses_family())
    }

    pub fn uses_extra_args(&self) -> bool {
        self.current_doc()
            .and_then(|d| self.specs.get(&d.compiler))
            .is_some_and(|s| s.uses_extra_args())
    }

    pub fn uses_incl_path(&self) -> bool {
        self.current_spec().is_some_and(|s| s.uses_incl_path())
    }

    pub fn has_compiler(&self) -> bool {
        self.current_doc().is_some_and(|d| d.has_compiler())
    }

    pub fn is_arduino(&self) -> bool {
        self.current_spec().is_some_and(|s| s.is_arduino())
    }

    pub fn board(&self) -> String {
        self.current_doc()
            .map(|d| {
                if d.board.is_empty() {
                    "Uno".to_string()
                } else {
                    d.board.clone()
                }
            })
            .unwrap_or_else(|| "Uno".into())
    }

    pub fn set_board(&mut self, v: String) {
        if let Some(d) = self.current_doc_mut() {
            if d.board == v {
                return;
            }
            d.raw_board = v.clone();
            if !v.eq_ignore_ascii_case("custom") {
                d.raw_custom_board.clear();
            }
            d.board = v;
            if d.compiler.eq_ignore_ascii_case("arduino") {
                let dev = device_for_board_or_fqbn(&d.board, &d.custom_board);
                if !dev.is_empty() {
                    d.device = dev;
                }
            }
        }
        self.file_settings_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn custom_board(&self) -> String {
        self.current_doc()
            .map(|d| d.custom_board.clone())
            .unwrap_or_default()
    }

    pub fn set_custom_board(&mut self, v: String) {
        if let Some(d) = self.current_doc_mut() {
            if d.custom_board == v {
                return;
            }
            d.raw_custom_board = v.clone();
            d.custom_board = v;
            if d.compiler.eq_ignore_ascii_case("arduino") {
                let dev = device_for_board_or_fqbn(&d.board, &d.custom_board);
                if !dev.is_empty() {
                    d.device = dev;
                }
            }
        }
        self.file_settings_changed();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    pub fn board_list(&self) -> Vec<String> {
        board_names_for_ui(&self.arduino_boards)
    }

    pub fn is_custom_board(&self) -> bool {
        self.current_doc()
            .is_some_and(|d| d.board.eq_ignore_ascii_case("custom"))
    }

    pub fn uses_board(&self) -> bool {
        self.is_arduino()
    }

    pub fn tool_path_suggestions(&self) -> Vec<String> {
        let mut list = self.tool_path_candidates.clone();
        let current = self.tool_path();
        if !current.is_empty() && !list.contains(&current) {
            list.insert(0, current);
        }
        list
    }

    pub fn set_compiler_settings_visible(&mut self, v: bool) {
        if v {
            if !self.has_compiler() {
                return;
            }
            if self.is_arduino() {
                self.refresh_arduino_boards();
            }
            self.refresh_tool_path_help();
            if !self.compiler_settings_visible {
                self.compiler_settings_visible = true;
                self.file_settings_changed();
            }
            self.request_show_compiler_settings();
        } else if self.compiler_settings_visible {
            self.compiler_settings_visible = false;
            self.file_settings_changed();
        }
    }

    pub fn set_file_settings_visible(&mut self, v: bool) {
        if v {
            if !self.file_settings_visible {
                self.file_settings_visible = true;
                self.file_settings_changed();
            }
            self.request_show_file_settings();
        } else if self.file_settings_visible {
            self.file_settings_visible = false;
            self.file_settings_changed();
        }
    }

    pub fn set_find_text(&mut self, v: String) {
        self.find_text = v;
        self.find_changed();
    }

    pub fn set_replace_text(&mut self, v: String) {
        self.replace_text = v;
        self.find_changed();
    }

    pub fn set_find_case(&mut self, v: bool) {
        self.find_case = v;
        self.find_changed();
    }

    pub fn set_find_words(&mut self, v: bool) {
        self.find_words = v;
        self.find_changed();
    }

    pub fn set_find_regexp(&mut self, v: bool) {
        self.find_regexp = v;
        self.find_changed();
    }

    pub fn set_find_visible(&mut self, v: bool) {
        if v {
            if !self.find_visible {
                self.find_visible = true;
                self.find_changed();
            }
            self.request_show_find();
        } else if self.find_visible {
            self.find_visible = false;
            self.found_ranges = json!([]);
            self.find_changed();
        }
    }

    fn close_documents(&mut self, indices: Vec<usize>) {
        self.pending_close_index = None;
        self.pending_close_queue.clear();
        for idx in indices {
            if idx < self.docs.len() && !self.pending_close_queue.contains(&idx) {
                self.pending_close_queue.push(idx);
            }
        }
        self.process_close_queue();
    }

    fn process_close_queue(&mut self) {
        while !self.pending_close_queue.is_empty() {
            let idx = self.pending_close_queue.remove(0);
            if idx >= self.docs.len() {
                continue;
            }
            if self.docs[idx].dirty {
                let index = idx as i32;
                self.hide_completions();
                if self.current != index {
                    self.current = index;
                    self.clear_transient_navigation();
                    self.diagnostic_ranges = self
                        .current_doc()
                        .map(|d| d.diagnostic_ranges.clone())
                        .unwrap_or(json!([]));
                    self.pending_hover_diag.clear();
                    self.current_document_changed();
                    self.current_file_changed();
                    self.current_text_changed();
                    self.marks_changed();
                    self.file_settings_changed();
                    self.rehighlight();
                }
                self.pending_close_index = Some(idx);
                self.request_save_before_close();
                return;
            }
            self.close_document_now(idx);
        }
    }

    #[qslot]
    fn close_document(&mut self, index: i32) {
        if index < 0 || index as usize >= self.docs.len() {
            return;
        }
        self.close_documents(vec![index as usize]);
    }

    #[qslot]
    fn close_all_documents(&mut self) {
        let indices: Vec<usize> = (0..self.docs.len()).collect();
        self.close_documents(indices);
    }

    #[qslot]
    fn close_other_documents(&mut self, index: i32) {
        let indices = close_indices_for_others(self.docs.len(), index);
        self.close_documents(indices);
    }

    #[qslot]
    fn close_documents_to_left(&mut self, index: i32) {
        let indices = close_indices_to_left(self.docs.len(), index);
        self.close_documents(indices);
    }

    #[qslot]
    fn close_documents_to_right(&mut self, index: i32) {
        let indices = close_indices_to_right(self.docs.len(), index);
        self.close_documents(indices);
    }

    fn close_document_now(&mut self, index: usize) {
        if index >= self.docs.len() {
            return;
        }
        self.hide_completions();
        let prev_doc_id = self.current_doc_id();
        let doc = self.docs.remove(index);
        adjust_close_queue_after_remove(&mut self.pending_close_queue, index);
        if doc.path.is_empty() {
            backup::clear_file(&doc.backup_id);
        } else {
            backup::clear_file_for_path(&doc.path, &self.project_path);
            self.file_watcher.unwatch(&doc.path);
            cs_engine::project::update_session(&self.project_path, |s| {
                s.editor_files.retain(|p| p != &doc.path);
                if s.active_editor_file.as_ref() == Some(&doc.path) {
                    s.active_editor_file = s.editor_files.last().cloned();
                }
            });
        }
        if self.docs.is_empty() {
            self.current = -1;
            self.request_hide();
        } else if self.current >= self.docs.len() as i32 {
            self.current = self.docs.len() as i32 - 1;
        } else if (index as i32) < self.current {
            self.current -= 1;
        }
        if self.current_doc_id() != prev_doc_id {
            self.clear_transient_navigation();
        }
        self.documents_changed();
        self.diagnostic_ranges = self
            .current_doc()
            .map(|d| d.diagnostic_ranges.clone())
            .unwrap_or(json!([]));
        self.pending_hover_diag.clear();
        self.current_document_changed();
        self.current_file_changed();
        self.current_text_changed();
        self.marks_changed();
        self.file_settings_changed();
        self.rehighlight();
        if !self.docs.is_empty() {
            self.request_focus();
        }
    }

    #[qslot]
    fn confirm_close_save(&mut self) {
        let Some(idx) = self.pending_close_index else {
            return;
        };
        if idx >= self.docs.len() {
            self.pending_close_index = None;
            self.process_close_queue();
            return;
        }
        let doc = &self.docs[idx];
        if doc.path.is_empty() {
            self.request_save_as_then_close();
            return;
        }
        let path = doc.path.clone();
        let text = doc.text.clone();
        if let Err(e) = std::fs::write(&path, &text) {
            let err_prefix = cs_engine::i18n::tr("Error saving file");
            self.push_log(format!("{err_prefix} {path}: {e}"));
            return;
        }
        backup::clear_file_for_path(&path, &self.project_path);
        self.file_watcher.sync(&path);
        self.pending_close_index = None;
        self.close_document_now(idx);
        self.process_close_queue();
    }

    #[qslot]
    fn confirm_close_discard(&mut self) {
        let Some(idx) = self.pending_close_index.take() else {
            return;
        };
        if idx >= self.docs.len() {
            self.process_close_queue();
            return;
        }
        let doc = &self.docs[idx];
        if doc.path.is_empty() {
            backup::clear_file(&doc.backup_id);
        } else {
            backup::clear_file_for_path(&doc.path, &self.project_path);
        }
        self.close_document_now(idx);
        self.process_close_queue();
    }

    #[qslot]
    fn confirm_close_cancel(&mut self) {
        self.pending_close_index = None;
        self.pending_close_queue.clear();
        if !self.docs.is_empty() {
            self.request_focus();
        }
    }

    #[qslot]
    fn finish_pending_close(&mut self) {
        if let Some(idx) = self.pending_close_index.take() {
            self.close_document_now(idx);
            self.process_close_queue();
        }
    }

    #[qslot]
    fn cancel_pending_close(&mut self) {
        self.pending_close_index = None;
        self.pending_close_queue.clear();
        if !self.docs.is_empty() {
            self.request_focus();
        }
    }

    #[qslot]
    fn close_current(&mut self) {
        let idx = self.current;
        self.close_document(idx);
    }

    #[qslot]
    fn set_firmware_devices(&mut self, devices: Value) {
        let has_arduino_file = devices.as_array().is_some_and(|arr| {
            arr.iter().any(|item| {
                let src = item
                    .get("firmwareSource")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let lower = src.to_ascii_lowercase();
                (lower.ends_with(".ino") || lower.ends_with(".pde")) && Path::new(src).is_file()
            })
        });
        let next_uids = build_firmware_device_uid_map(&devices);
        let next = build_firmware_mcu_map(devices);
        self.firmware_device_uids = next_uids;
        if self.firmware_mcus != next {
            self.firmware_mcus = next;
            self.documents_changed();
        }
        if has_arduino_file {
            self.refresh_arduino_boards();
        }
    }

    #[qslot]
    fn focus_open_file(&mut self, path: String) -> bool {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return false;
        }
        let norm_target = normalize_debug_path(Path::new(&path));
        if let Some((i, _)) = self.docs.iter().enumerate().find(|(_, d)| {
            !d.path.is_empty()
                && (d.path == path || normalize_debug_path(Path::new(&d.path)) == norm_target)
        }) {
            self.set_current_document(i as i32);
            return true;
        }
        false
    }

    #[qslot]
    fn compile_for_sim(&mut self, src_path: String, target_device_id: String) -> bool {
        let src_path = strip_file_url(&src_path);
        self.compile_path_for_sim(src_path, target_device_id)
    }

    #[qslot]
    fn compile_devices_for_sim(&mut self, devices: Value) -> bool {
        self.compile_devices_for_sim_inner(devices, false)
    }

    #[qslot]
    fn debug_devices(&mut self, devices: Value) -> bool {
        self.request_show();
        self.compile_devices_for_sim_inner(devices, true)
    }

    #[qslot]
    fn run_devices(&mut self, devices: Value) -> bool {
        if DebugSession::global().is_active() {
            self.debug_start();
            self.request_power_on();
            return true;
        }
        self.compile_devices_for_sim_inner(devices, false)
    }

    #[qslot]
    fn upload_doc_to_device(&mut self, index: i32, target_device_id: String) {
        if index < 0 {
            return;
        }
        self.upload_doc_to_device_inner(index as usize, target_device_id);
    }

    #[qslot]
    fn restore_drafts(&mut self) {
        if crate::pending::no_project() {
            return;
        }
        let orphans = backup::orphan_untitled_files();
        if orphans.is_empty() {
            return;
        }
        let had_docs = !self.docs.is_empty();
        let mut added = false;
        for draft in orphans {
            if self.docs.iter().any(|d| d.backup_id == draft.id) {
                continue;
            }
            let title = if draft.title.is_empty() {
                "untitled".into()
            } else {
                draft.title
            };
            let doc_id = self.alloc_doc_id();
            let mut doc = Document::untitled(doc_id, title);
            doc.backup_id = draft.id;
            doc.text = draft.text;
            doc.dirty = true;
            self.docs.push(doc);
            added = true;
        }
        if !added {
            return;
        }
        if !had_docs {
            self.current = 0;
            self.clear_transient_navigation();
            self.hide_completions();
            self.diagnostic_ranges = json!([]);
            self.pending_hover_diag.clear();
            self.documents_changed();
            self.current_document_changed();
            self.current_file_changed();
            self.current_text_changed();
            self.marks_changed();
            self.file_settings_changed();
            self.rehighlight();
            self.request_show();
            self.request_focus();
        } else {
            self.documents_changed();
        }
    }

    fn flush_dirty_drafts(&mut self) {
        self.pending_flush = false;
        for d in &self.docs {
            if d.dirty {
                let rec_id = d.recovery_id(&self.project_path);
                backup::put_file(&rec_id, &d.path, &self.project_path, &d.title, &d.text);
            }
        }
    }

    #[qslot]
    fn flush_drafts(&mut self) {
        self.flush_dirty_drafts();
        let editor_files: Vec<String> = self
            .docs
            .iter()
            .filter(|d| !d.path.is_empty())
            .map(|d| d.path.clone())
            .collect();
        let active_editor_file = self
            .current_doc()
            .filter(|d| !d.path.is_empty())
            .map(|d| d.path.clone());
        cs_engine::project::update_session(&self.project_path, |s| {
            s.editor_files = editor_files;
            s.active_editor_file = active_editor_file;
        });
    }

    #[qslot]
    fn focus_editor(&mut self) {
        self.request_focus();
    }

    #[qslot]
    fn new_file(&mut self) {
        let title = self.untitled_name();
        let doc_id = self.alloc_doc_id();
        self.docs.push(Document::untitled(doc_id, title));
        self.current = self.docs.len() as i32 - 1;
        self.clear_transient_navigation();
        self.hide_completions();
        self.diagnostic_ranges = json!([]);
        self.pending_hover_diag.clear();
        self.documents_changed();
        self.current_document_changed();
        self.current_file_changed();
        self.current_text_changed();
        self.marks_changed();
        self.file_settings_changed();
        self.rehighlight();
        self.request_show();
        self.request_focus();
    }

    #[qslot]
    fn load_file(&mut self, path: String) {
        let path = strip_file_url(&path);
        self.open_path(path);
    }

    #[qslot]
    fn open_file(&mut self, path: String) {
        let path = strip_file_url(&path);
        self.open_path(path);
    }

    #[qslot]
    fn navigate_to_source(&mut self, file: String, line: i32) {
        let file = strip_file_url(&file);
        let path = if std::path::Path::new(&file).exists() {
            file
        } else if let Some(active_doc) = self.current_doc() {
            let active_dir = std::path::Path::new(&active_doc.path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let file_name = std::path::Path::new(&file).file_name().unwrap_or_default();
            let candidate = active_dir.join(file_name);
            if candidate.exists() {
                candidate.to_string_lossy().into_owned()
            } else {
                file
            }
        } else {
            file
        };
        if std::path::Path::new(&path).exists() {
            self.open_path(path);
            if line > 0 {
                self.goto_1based(line);
            }
        }
    }

    #[qslot]
    fn save(&mut self) {
        let path = self.current_path();
        if path.is_empty() {
            self.request_save_as();
            return;
        }
        self.write_current(&path);
    }

    #[qslot]
    fn save_all(&mut self) -> bool {
        let mut untitled: Option<i32> = None;
        let snapshot: Vec<(usize, String, bool)> = self
            .docs
            .iter()
            .enumerate()
            .map(|(i, d)| (i, d.path.clone(), d.dirty))
            .collect();
        for (i, path, dirty) in snapshot {
            if !dirty {
                continue;
            }
            if path.is_empty() {
                if untitled.is_none() {
                    untitled = Some(i as i32);
                }
                continue;
            }
            self.write_doc(i, &path);
        }
        self.documents_changed();
        if let Some(idx) = untitled {
            self.set_current_document(idx);
            self.request_show();
            self.request_save_as();
            return false;
        }
        true
    }

    #[qslot]
    fn save_path(&mut self, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let old_path = self
            .current_doc()
            .map(|d| d.path.clone())
            .unwrap_or_default();
        let old_id = self
            .current_doc()
            .map(|d| d.recovery_id(&self.project_path))
            .unwrap_or_default();
        if !old_path.is_empty() && old_path != path {
            self.file_watcher.unwatch(&old_path);
            cs_engine::project::update_session(&self.project_path, |s| {
                s.editor_files.retain(|p| p != &old_path);
            });
        }
        if !old_id.is_empty() {
            backup::clear_file(&old_id);
        }
        self.file_watcher.watch(&path);
        self.write_current(&path);
        let backup_id = backup::id_for_path(&path, &self.project_path);
        if let Some(d) = self.current_doc_mut() {
            d.path = path.clone();
            d.backup_id = backup_id;
            d.title = Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();
            d.syntax = highlighter::syntax_for_path(&path).map(|s| s.to_string());
        }
        if !path.is_empty() {
            if let Some(d) = self.current_doc() {
                if !d.breakpoints.is_empty() {
                    let bp_lines: Vec<usize> = d
                        .breakpoints
                        .iter()
                        .filter(|&&l| l > 0)
                        .map(|&l| l as usize)
                        .collect();
                    DebugSession::global().set_breakpoints_for_file(Path::new(&path), &bp_lines);
                }
            }
            cs_engine::project::update_session(&self.project_path, |s| {
                if !s.editor_files.contains(&path) {
                    s.editor_files.push(path.clone());
                }
                s.active_editor_file = Some(path.clone());
            });
        }
        self.documents_changed();
        self.current_file_changed();
        self.current_text_changed();
        self.rehighlight();
        self.persist_cfg();
        self.sync_lsp_open();
    }

    #[qslot]
    fn reload_settings(&mut self) {
        let s = cs_engine::settings::get();
        self.apply_editor_settings(&s);
    }

    #[qslot]
    fn file_props(&mut self) {
        self.set_file_settings_visible(true);
    }

    #[qslot]
    fn compiler_props(&mut self) {
        self.set_compiler_settings_visible(true);
    }

    #[qslot]
    fn commit_tool_path(&mut self, path: String) {
        let path = strip_file_url(&path);
        let path = lsp::ensure_trailing_slash(&path);
        let warning = match self.current_spec() {
            Some(spec) if !spec.is_none() => editor::check_tool_path(spec, &path),
            _ => String::new(),
        };
        if warning.is_empty() {
            if let Some(d) = self.current_doc_mut() {
                d.tool_path = path;
            }
            if self.is_arduino() {
                self.refresh_arduino_boards();
                self.reapply_arduino_board();
            }
            self.persist_cfg();
            self.refresh_tool_path_help();
            self.sync_lsp_open();
        } else {
            self.tool_path_warning = warning;
        }
        self.file_settings_changed();
    }

    #[qslot]
    fn check_tool_path(&mut self, path: String) {
        let path = strip_file_url(&path);
        let path = lsp::ensure_trailing_slash(&path);
        self.tool_path_warning = match self.current_spec() {
            Some(spec) if !spec.is_none() => editor::check_tool_path(spec, &path),
            _ => String::new(),
        };
        self.file_settings_changed();
    }

    #[qslot]
    fn goto_definition(&mut self) {
        let path = self.current_path();
        let text = self.current_text();
        if path.is_empty() || language_id_for_path(&path).is_none() {
            return;
        }
        let cursor_byte = editor::utf16_to_byte_offset(&text, self.cursor_pos.max(0) as usize);
        let (line, col) = editor::line_col(&text, cursor_byte);
        self.lsp.request_definition(&path, line, col);
    }

    #[qslot]
    fn reload_document(&mut self) {
        let path = self.current_path();
        if path.is_empty() {
            return;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let recovery_id = self
                    .current_doc()
                    .map(|d| d.recovery_id(&self.project_path))
                    .unwrap_or_default();
                if let Some(d) = self.current_doc_mut() {
                    d.text = text;
                    d.dirty = false;
                }
                if !recovery_id.is_empty() {
                    backup::clear_file(&recovery_id);
                }
                self.file_watcher.sync(&path);
                self.hide_completions();
                self.current_text_changed();
                self.documents_changed();
                self.rehighlight();
                self.sync_lsp_open();
                let reload_prefix = cs_engine::i18n::tr("Reloaded from disk:");
                self.push_log(format!("{reload_prefix} {path}"));
            }
            Err(e) => {
                let err_prefix = cs_engine::i18n::tr("Could not reload");
                self.push_log(format!("{err_prefix} {path}: {e}"));
            }
        }
    }

    #[qslot]
    fn gutter_clicked(&mut self, line: i32, _button: i32) {
        if line < 1 {
            return;
        }
        if let Some(d) = self.current_doc_mut() {
            let path = Path::new(&d.path);
            if let Some(i) = d.breakpoints.iter().position(|l| *l == line) {
                d.breakpoints.remove(i);
                cs_engine::debug::DebugSession::global().remove_breakpoint(path, line as usize);
            } else {
                d.breakpoints.push(line);
                d.breakpoints.sort();
                cs_engine::debug::DebugSession::global().add_breakpoint(path, line as usize);
            }
        }
        self.marks_changed();
        self.persist_cfg();
    }

    #[qslot]
    fn complete(&mut self, manual: bool) {
        self.complete_at(manual);
    }

    #[qslot]
    fn hide_completion(&mut self) {
        self.hide_completions();
    }

    #[qslot]
    fn move_completion(&mut self, delta: i32) {
        let n = self
            .completions
            .as_array()
            .map(|a| a.len() as i32)
            .unwrap_or(0);
        if n <= 0 {
            return;
        }
        self.completion_index = (self.completion_index + delta).rem_euclid(n);
        self.completion_changed();
    }

    #[qslot]
    fn accept_completion(&mut self, index: i32, cursor_pos: i32) -> Value {
        let Some(arr) = self.completions.as_array() else {
            return json!({ "valid": false });
        };
        let Some(item) = arr.get(index as usize) else {
            return json!({ "valid": false });
        };
        let insert = item
            .get("insertText")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("text").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let detail = item
            .get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let kind = item.get("kind").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let word = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let prefix = self.completion_prefix.clone();
        self.hide_completions();

        let current_text = self.current_text();
        let cursor_utf16 = if cursor_pos >= 0 {
            cursor_pos as usize
        } else {
            self.cursor_pos.max(0) as usize
        };

        let edit = editor::calculate_completion(
            &current_text,
            cursor_utf16,
            &prefix,
            &insert,
            &word,
            &detail,
            kind,
        );

        if !edit.valid {
            return json!({ "valid": false });
        }

        self.accepting_completion = true;

        if edit.is_function {
            if !edit.signature_label.is_empty() {
                self.current_signature_label = edit.signature_label.clone();
                self.current_parameters.clear();
                self.current_signature_active_param = 0;
                let html = editor::format_signature_html(&editor::lsp::SignatureHelp {
                    label: edit.signature_label.clone(),
                    active_parameter: 0,
                    parameters: Vec::new(),
                });
                self.signature_html = html;
                self.signature_visible = !self.signature_html.is_empty();
                self.signature_changed();
            }
        } else {
            self.current_signature_label.clear();
            self.current_parameters.clear();
            self.current_signature_active_param = -1;
            self.signature_visible = false;
            self.signature_html.clear();
            self.signature_changed();
        }

        self.cursor_pos = edit.new_cursor_pos as i32;

        json!({
            "valid": true,
            "replaceStart": edit.replace_start,
            "replaceEnd": edit.replace_end,
            "insertText": edit.insert_text,
            "newCursorPos": edit.new_cursor_pos,
        })
    }

    #[qslot]
    fn finish_completion(&mut self) {
        self.accepting_completion = false;
        self.update_signature_for_cursor();
    }

    #[qslot]
    fn compile(&mut self) {
        self.compile_inner(false, false, false);
    }

    #[qslot]
    fn upload(&mut self) {
        self.compile_inner(false, true, false);
    }

    #[qslot]
    fn upload_run(&mut self) {
        self.compile_inner(false, true, true);
    }

    #[qslot]
    fn run(&mut self) {
        if cs_engine::debug::DebugSession::global().is_active() {
            self.debug_start();
            self.request_power_on();
        } else if self.current_doc().is_some() {
            self.upload_run();
        } else {
            self.request_power_on();
        }
    }

    #[qslot]
    fn debug(&mut self) {
        self.request_show();
        self.compile_inner(true, true, true);
    }

    #[qslot]
    fn cancel_compile(&mut self) {
        self.clear_compile_batch();
        if let Some(pending) = &mut self.compile_job {
            self.compile_cancel_requested = true;
            pending.job.cancel();
        }
    }

    #[qslot]
    fn clear_compile_status(&mut self) {
        if self.compiling || self.compile_state == "idle" {
            return;
        }
        self.compile_state = "idle".to_string();
        self.compile_file.clear();
        self.compile_error_count = 0;
        self.compile_warning_count = 0;
        self.compiling_changed();
    }

    #[qslot]
    fn find_dialog(&mut self) {
        self.set_find_visible(true);
        self.request_show();
    }

    #[qslot]
    fn find_next(&mut self) {
        let text = self.current_text();
        let cursor_byte = editor::utf16_to_byte_offset(&text, self.cursor_pos.max(0) as usize);
        match find_next(&text, cursor_byte, &self.find_opts(), true) {
            Ok(Some(m)) => {
                self.found_ranges = json!([]);
                self.apply_match(&text, m);
                self.find_changed();
            }
            Ok(None) => {}
            Err(e) => self.push_log(e),
        }
    }

    #[qslot]
    fn find_prev(&mut self) {
        let text = self.current_text();
        let cursor_byte = editor::utf16_to_byte_offset(&text, self.cursor_pos.max(0) as usize);
        match find_next(&text, cursor_byte, &self.find_opts(), false) {
            Ok(Some(m)) => {
                self.found_ranges = json!([]);
                self.apply_match(&text, m);
                self.find_changed();
            }
            Ok(None) => {}
            Err(e) => self.push_log(e),
        }
    }

    #[qslot]
    fn find_all(&mut self) {
        let text = self.current_text();
        match find_all(&text, &self.find_opts()) {
            Ok(all) => {
                let found_fmt = cs_engine::i18n::tr("Found %1 occurrence(s)")
                    .replace("%1", &all.len().to_string());
                self.push_log(found_fmt);
                self.found_ranges = json!(
                    all.iter()
                        .map(|m| json!({
                            "start": editor::byte_to_utf16_offset(&text, m.start),
                            "end": editor::byte_to_utf16_offset(&text, m.end),
                        }))
                        .collect::<Vec<_>>()
                );
                self.find_changed();
            }
            Err(e) => self.push_log(e),
        }
    }

    #[qslot]
    fn replace(&mut self) {
        let start = self.select_start;
        let end = self.select_end;
        let repl = self.replace_text.clone();
        if start >= 0 && end > start {
            let mut new_cursor = None;
            if let Some(d) = self.current_doc_mut() {
                let s_utf16 = start as usize;
                let e_utf16 = end as usize;
                let s_byte = editor::utf16_to_byte_offset(&d.text, s_utf16);
                let e_byte = editor::utf16_to_byte_offset(&d.text, e_utf16);
                if s_byte <= d.text.len() && e_byte <= d.text.len() && s_byte <= e_byte {
                    d.text.replace_range(s_byte..e_byte, &repl);
                    d.dirty = true;
                    let new_cursor_byte = s_byte + repl.len();
                    new_cursor =
                        Some(editor::byte_to_utf16_offset(&d.text, new_cursor_byte) as i32);
                }
            }
            if let Some(pos) = new_cursor {
                self.pending_flush = true;
                self.cursor_pos = pos;
            }
            self.select_start = -1;
            self.select_end = -1;
            self.current_text_changed();
            self.documents_changed();
            self.rehighlight();
        }
    }

    #[qslot]
    fn replace_and_find(&mut self) {
        self.replace();
        self.find_next();
    }

    #[qslot]
    fn replace_all(&mut self) {
        let text = self.current_text();
        match replace_all(&text, &self.find_opts(), &self.replace_text) {
            Ok((out, n)) => {
                if let Some(d) = self.current_doc_mut() {
                    d.text = out;
                    d.dirty = true;
                    self.pending_flush = true;
                }
                let repl_fmt =
                    cs_engine::i18n::tr("Replaced %1 occurrence(s)").replace("%1", &n.to_string());
                self.push_log(repl_fmt);
                self.found_ranges = json!([]);
                self.current_text_changed();
                self.documents_changed();
                self.rehighlight();
                self.find_changed();
            }
            Err(e) => self.push_log(e),
        }
    }

    #[qslot]
    fn close_find(&mut self) {
        self.find_visible = false;
        self.found_ranges = json!([]);
        self.find_changed();
        self.request_focus();
    }

    #[qslot]
    fn undo(&mut self) {
        self.fire_edit("undo");
    }
    #[qslot]
    fn redo(&mut self) {
        self.fire_edit("redo");
    }
    #[qslot]
    fn cut(&mut self) {
        self.fire_edit("cut");
    }
    #[qslot]
    fn copy(&mut self) {
        self.fire_edit("copy");
    }
    #[qslot]
    fn paste(&mut self) {
        self.fire_edit("paste");
    }
    #[qslot]
    fn select_all(&mut self) {
        self.fire_edit("selectAll");
    }

    #[qslot]
    fn take_log_line(&mut self) -> String {
        if self.pending_log.is_empty() {
            String::new()
        } else {
            self.pending_log.remove(0)
        }
    }

    #[qslot]
    fn take_log_lines(&mut self) -> Vec<String> {
        mem::take(&mut self.pending_log)
    }

    #[qslot]
    fn show_hover(&mut self, pos: i32) {
        let path = self.current_path();
        let text = self.current_text();
        if path.is_empty() || language_id_for_path(&path).is_none() {
            return;
        }
        self.hover_gutter = false;
        self.pending_hover_pos = pos;
        self.pending_hover_diag.clear();

        let byte_pos = editor::utf16_to_byte_offset(&text, pos.max(0) as usize);
        let (line, col) = editor::line_col(&text, byte_pos);

        // Check for diagnostics covering this line/col
        let mut diag_tooltips = Vec::new();
        if let Some(d) = self.current_doc() {
            for diag in &d.diagnostics {
                let diag_start_line = diag.start_line;
                let diag_end_line = diag.end_line;
                if line >= diag_start_line && line <= diag_end_line {
                    let mut matches_col = true;
                    if diag_start_line == diag_end_line && diag.end_col > diag.start_col {
                        matches_col = col >= diag.start_col && col <= diag.end_col;
                    }
                    if matches_col {
                        let (color, title) = match diag.severity {
                            1 => (
                                ColorTheme::get_hex(ColorId::EditorErrorUnderline, self.dark),
                                "Error",
                            ),
                            2 => (
                                ColorTheme::get_hex(ColorId::EditorWarnUnderline, self.dark),
                                "Warning",
                            ),
                            _ => (
                                ColorTheme::get_hex(ColorId::EditorInfoUnderline, self.dark),
                                "Info",
                            ),
                        };
                        let escaped = escape_html(&diag.message);
                        diag_tooltips.push(format!(
                            "<b style='color:{color};'>{title} (Line {}):</b> {escaped}",
                            line + 1
                        ));
                    }
                }
            }

            if diag_tooltips.is_empty() {
                let line_1based = line + 1;
                if d.errors.contains(&line_1based) {
                    let color = ColorTheme::get_hex(ColorId::EditorErrorUnderline, self.dark);
                    diag_tooltips.push(format!(
                        "<b style='color:{color};'>Error (Line {line_1based})</b>"
                    ));
                } else if d.warnings.contains(&line_1based) {
                    let color = ColorTheme::get_hex(ColorId::EditorWarnUnderline, self.dark);
                    diag_tooltips.push(format!(
                        "<b style='color:{color};'>Warning (Line {line_1based})</b>"
                    ));
                }
            }
        }

        if !diag_tooltips.is_empty() {
            let combined = diag_tooltips.join("<br>");
            self.pending_hover_diag = combined.clone();
            self.hover_html = combined;
            self.hover_changed();
        }

        self.lsp.request_hover(&path, line, col);
    }

    #[qslot]
    fn update_text(&mut self, text: String, cursor_pos: i32) {
        let clamped = cursor_pos.max(0);
        if self.cursor_pos != clamped {
            self.cursor_pos = clamped;
            self.cursor_changed();
        }
        self.apply_text_edit(text);
    }

    #[qslot]
    fn set_highlight_viewport(&mut self, start_line: i32, end_line: i32) {
        let s = start_line.max(0) as usize;
        let e = end_line.max(start_line + 1) as usize;
        if self.highlight_viewport_start == s && self.highlight_viewport_end == e {
            return;
        }
        self.highlight_viewport_start = s;
        self.highlight_viewport_end = e;
        if let Some(d) = self.current_doc() {
            if d.highlight_cache.line_count() == 0 && !d.text.is_empty() {
                self.rehighlight();
                return;
            }
        }
        self.render_highlight_window();
    }

    #[qslot]
    fn calculate_newline_for_current(
        &self,
        cursor_pos: i32,
        sel_start: i32,
        sel_end: i32,
    ) -> Value {
        let text = self.current_doc().map(|d| d.text.as_str()).unwrap_or("");
        let edit = calculate_newline(
            text,
            cursor_pos.max(0) as usize,
            sel_start.max(0) as usize,
            sel_end.max(0) as usize,
            self.space_tabs,
            self.tab_size,
        );
        json!({
            "valid": edit.valid,
            "replaceStart": edit.replace_start,
            "replaceEnd": edit.replace_end,
            "insertText": edit.insert_text,
            "newCursorPos": edit.new_cursor_pos,
        })
    }

    #[qslot]
    fn calculate_newline(
        &self,
        text: String,
        cursor_pos: i32,
        sel_start: i32,
        sel_end: i32,
    ) -> Value {
        let edit = calculate_newline(
            &text,
            cursor_pos.max(0) as usize,
            sel_start.max(0) as usize,
            sel_end.max(0) as usize,
            self.space_tabs,
            self.tab_size,
        );
        json!({
            "valid": edit.valid,
            "replaceStart": edit.replace_start,
            "replaceEnd": edit.replace_end,
            "insertText": edit.insert_text,
            "newCursorPos": edit.new_cursor_pos,
        })
    }

    #[qslot]
    fn hide_hover(&mut self) {
        self.pending_hover_pos = -1;
        self.pending_hover_diag.clear();
        self.hover_gutter = false;
        if self.hover_html.is_empty() {
            return;
        }
        self.hover_html.clear();
        self.hover_changed();
    }

    #[qslot]
    fn show_gutter_hover(&mut self, line: i32) {
        let tip = self.gutter_tooltip(line);
        self.pending_hover_pos = -1;
        self.pending_hover_diag.clear();
        self.hover_gutter = true;
        if !tip.is_empty() {
            self.hover_html = tip;
            self.hover_changed();
        } else if !self.hover_html.is_empty() {
            self.hover_html.clear();
            self.hover_changed();
        }
    }

    #[qslot]
    fn gutter_tooltip(&self, line: i32) -> String {
        let mut parts = Vec::new();
        if let Some(d) = self.current_doc() {
            for diag in &d.diagnostics {
                let start_1 = diag.start_line + 1;
                let end_1 = diag.end_line + 1;
                if line >= start_1 && line <= end_1 {
                    let (color, title) = match diag.severity {
                        1 => (
                            ColorTheme::get_hex(ColorId::EditorErrorUnderline, self.dark),
                            "Error",
                        ),
                        2 => (
                            ColorTheme::get_hex(ColorId::EditorWarnUnderline, self.dark),
                            "Warning",
                        ),
                        _ => (
                            ColorTheme::get_hex(ColorId::EditorInfoUnderline, self.dark),
                            "Info",
                        ),
                    };
                    let escaped = escape_html(&diag.message);
                    parts.push(format!(
                        "<b style='color:{color};'>{title} (Line {line}):</b> {escaped}"
                    ));
                }
            }

            if parts.is_empty() {
                if d.errors.contains(&line) {
                    let color = ColorTheme::get_hex(ColorId::EditorErrorUnderline, self.dark);
                    parts.push(format!("<b style='color:{color};'>Error (Line {line})</b>"));
                } else if d.warnings.contains(&line) {
                    let color = ColorTheme::get_hex(ColorId::EditorWarnUnderline, self.dark);
                    parts.push(format!(
                        "<b style='color:{color};'>Warning (Line {line})</b>"
                    ));
                }
            }

            if d.breakpoints.contains(&line) {
                parts.push(format!("<b>Breakpoint (Line {line})</b>"));
            }
            if line == self.debug_line() {
                let color = ColorTheme::get_hex(ColorId::EditorWarnUnderline, self.dark);
                parts.push(format!(
                    "<b style='color:{color};'>Paused at Line {line}</b>"
                ));
            }
        }
        parts.join("<br>")
    }

    #[qslot]
    fn text_hovered(&mut self, pos: i32, modifiers: i32) -> bool {
        const MOD_CTRL: i32 = 0x0400_0000;
        const MOD_META: i32 = 0x0800_0000;
        if modifiers & (MOD_CTRL | MOD_META) == 0 {
            self.clear_hover_link();
            return false;
        }
        let path = self.current_path();
        let text = self.current_text();
        if path.is_empty() || language_id_for_path(&path).is_none() {
            self.clear_hover_link();
            return false;
        }
        if let Some((start, end, _token)) = editor::find_ident_at_pos(&text, pos.max(0) as usize) {
            let s = start as i32;
            let e = end as i32;
            if self.hover_link_start != s || self.hover_link_end != e {
                self.hover_link_start = s;
                self.hover_link_end = e;
                self.hover_link_changed();
            }
            true
        } else {
            self.clear_hover_link();
            false
        }
    }

    #[qslot]
    fn clear_hover_link(&mut self) {
        if self.hover_link_start >= 0 || self.hover_link_end >= 0 {
            self.hover_link_start = -1;
            self.hover_link_end = -1;
            self.hover_link_changed();
        }
    }

    #[qslot]
    fn text_clicked(&mut self, pos: i32, modifiers: i32) -> bool {
        const MOD_CTRL: i32 = 0x0400_0000;
        const MOD_META: i32 = 0x0800_0000;
        if modifiers & (MOD_CTRL | MOD_META) == 0 {
            return false;
        }
        let path = self.current_path();
        let text = self.current_text();
        if path.is_empty() || language_id_for_path(&path).is_none() {
            return false;
        }
        let pos_u = pos.max(0) as usize;
        if editor::find_ident_at_pos(&text, pos_u).is_none() {
            return false;
        }
        let byte_pos = editor::utf16_to_byte_offset(&text, pos_u);
        let (line, col) = editor::line_col(&text, byte_pos);
        self.lsp.request_definition(&path, line, col);
        true
    }

    #[qslot]
    fn format_document(&mut self) {
        let (text, path) = match self.current_doc() {
            Some(d) => (d.text.clone(), d.path.clone()),
            None => return,
        };
        let p_opt = if path.is_empty() {
            None
        } else {
            Some(path.as_str())
        };
        match editor::format_code(&text, p_opt, self.space_tabs, self.tab_size) {
            Ok(formatted) => {
                if formatted != text {
                    if let Some(d) = self.current_doc_mut() {
                        d.text = formatted;
                        d.dirty = true;
                        self.pending_flush = true;
                    }
                    self.current_text_changed();
                    self.documents_changed();
                    self.rehighlight();
                    self.cursor_pos = self
                        .cursor_pos
                        .min(self.current_text().encode_utf16().count() as i32);
                    self.cursor_changed();
                    self.push_log(cs_engine::i18n::tr("Formatted document with clang-format"));
                }
            }
            Err(e) => {
                let err_prefix = cs_engine::i18n::tr("clang-format error:");
                self.push_log(format!("{err_prefix} {e}"));
            }
        }
    }

    #[qslot]
    fn request_signature(&mut self, pos: i32) {
        self.cursor_pos = pos.max(0);
        self.cursor_changed();
        self.update_signature_for_cursor();
    }

    #[qslot]
    fn dismiss_signature(&mut self) {
        self.signature_visible = false;
        self.signature_changed();
    }

    #[qslot]
    fn disk_prompt_reload(&mut self) {
        let path = self.disk_prompt_path.clone();
        self.disk_prompt_visible = false;
        self.disk_prompt_changed();
        if path.is_empty() {
            return;
        }
        if let Some((i, doc)) = self
            .docs
            .iter_mut()
            .enumerate()
            .find(|(_, d)| d.path == path)
        {
            if let Ok(new_text) = std::fs::read_to_string(&path) {
                doc.text = new_text;
                doc.dirty = false;
                let recovery_id = doc.recovery_id(&self.project_path);
                if !recovery_id.is_empty() {
                    backup::clear_file(&recovery_id);
                }
                self.file_watcher.sync(&path);
                if self.current == i as i32 {
                    self.hide_completions();
                    self.current_text_changed();
                    self.rehighlight();
                }
                self.documents_changed();
                let reload_prefix = cs_engine::i18n::tr("Reloaded from disk:");
                self.push_log(format!("{reload_prefix} {path}"));
            }
        }
    }

    #[qslot]
    fn disk_prompt_keep(&mut self) {
        let path = self.disk_prompt_path.clone();
        let kind = self.disk_prompt_kind.clone();
        self.disk_prompt_visible = false;
        self.disk_prompt_changed();
        if path.is_empty() {
            return;
        }
        self.file_watcher.acknowledge(&path);
        if kind == "deleted" {
            if let Some(doc) = self.docs.iter_mut().find(|d| d.path == path) {
                doc.dirty = true;
                self.pending_flush = true;
                self.documents_changed();
            }
        }
    }

    #[qslot]
    fn disk_prompt_close(&mut self) {
        let path = self.disk_prompt_path.clone();
        self.disk_prompt_visible = false;
        self.disk_prompt_changed();
        if path.is_empty() {
            return;
        }
        if let Some((i, _)) = self.docs.iter().enumerate().find(|(_, d)| d.path == path) {
            self.close_document_now(i);
        }
    }

    #[qslot]
    fn poll_lsp(&mut self) {
        if let Some(rx) = &self.board_rx {
            match rx.try_recv() {
                Ok((tp, boards)) => {
                    self.arduino_list_path = Some(tp);
                    self.arduino_boards = boards;
                    self.board_loading_path = None;
                    self.board_rx = None;
                    self.compiling_changed();
                    self.reapply_arduino_board();
                    self.file_settings_changed();
                    self.sync_lsp_open();
                }
                Err(TryRecvError::Disconnected) => {
                    self.board_loading_path = None;
                    self.board_rx = None;
                    self.compiling_changed();
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        self.drain_compile_job();
        self.sync_debug_location();
        if self.pending_flush {
            self.flush_dirty_drafts();
        }
        self.update_lsp_ready();
        // Drain file watcher events
        let watch_events = self.file_watcher.poll();
        for event in watch_events {
            match event {
                WatchEvent::Changed(path) => {
                    if let Some((i, doc)) = self
                        .docs
                        .iter_mut()
                        .enumerate()
                        .find(|(_, d)| d.path == path)
                    {
                        if !doc.dirty {
                            if let Ok(new_text) = std::fs::read_to_string(&path) {
                                doc.text = new_text;
                                doc.dirty = false;
                                self.file_watcher.sync(&path);
                                if self.current == i as i32 {
                                    self.current_text_changed();
                                    self.rehighlight();
                                }
                                self.documents_changed();
                                let reload_prefix = cs_engine::i18n::tr("Reloaded from disk:");
                                self.push_log(format!("{reload_prefix} {path}"));
                            }
                        } else if !self.disk_prompt_visible {
                            self.disk_prompt_path = path;
                            self.disk_prompt_kind = "changed".into();
                            self.disk_prompt_visible = true;
                            self.disk_prompt_changed();
                        }
                    }
                }
                WatchEvent::Deleted(path) => {
                    if self.docs.iter().any(|d| d.path == path) && !self.disk_prompt_visible {
                        self.disk_prompt_path = path;
                        self.disk_prompt_kind = "deleted".into();
                        self.disk_prompt_visible = true;
                        self.disk_prompt_changed();
                    }
                }
            }
        }

        let events = self.lsp.take_events();
        if events.is_empty() {
            return;
        }
        let path = self.current_path();
        let prefix = self.completion_prefix.clone();
        let show_debug = self.show_lsp_debug;
        let mut need_local = false;
        let mut logged_debug = false;
        for ev in events {
            match ev {
                LspEvent::Completions(items) => {
                    if !self.completion_pending || self.current_doc().is_none() {
                        continue;
                    }
                    self.completion_pending = false;
                    if items.is_empty() {
                        need_local = !prefix.is_empty();
                        continue;
                    }
                    let filtered: Vec<Value> = items
                        .into_iter()
                        .filter(|c| {
                            prefix.is_empty()
                                || c.label
                                    .get(..prefix.len())
                                    .is_some_and(|h| h.eq_ignore_ascii_case(&prefix))
                        })
                        .take(100)
                        .map(|c| {
                            json!({
                                "text": c.label,
                                "insertText": if c.insert_text.is_empty() { c.label } else { c.insert_text },
                                "detail": c.detail,
                                "kind": c.kind,
                            })
                        })
                        .collect();
                    if filtered.is_empty() {
                        need_local = !prefix.is_empty();
                    } else {
                        let p = prefix.clone();
                        self.set_completion_items(filtered, &p);
                    }
                }
                LspEvent::Diagnostics { uri, items } => {
                    let uri_path = strip_file_url(&editor::lsp::uri_to_path(&uri));
                    let cur_path = strip_file_url(&path);
                    let same_path = if cfg!(windows) {
                        uri_path.eq_ignore_ascii_case(&cur_path)
                    } else {
                        uri_path == cur_path
                    };
                    if !cur_path.is_empty()
                        && (same_path
                            || Path::new(&uri_path).file_name() == Path::new(&cur_path).file_name())
                    {
                        let dark = self.dark;
                        let mut new_ranges = None;
                        if let Some(d) = self.current_doc_mut() {
                            d.diagnostics = items.clone();
                            d.errors = items
                                .iter()
                                .filter(|i| i.severity <= 1)
                                .map(|i| i.start_line + 1)
                                .collect();
                            d.warnings = items
                                .iter()
                                .filter(|i| i.severity == 2)
                                .map(|i| i.start_line + 1)
                                .collect();
                            d.errors.sort();
                            d.errors.dedup();
                            d.warnings.sort();
                            d.warnings.dedup();

                            let ranges =
                                editor::compute_diagnostic_ranges(&d.text, &d.diagnostics, dark);
                            let val = serde_json::to_value(&ranges).unwrap_or(json!([]));
                            d.diagnostic_ranges = val.clone();
                            new_ranges = Some(val);
                        }
                        if let Some(val) = new_ranges {
                            self.diagnostic_ranges = val;
                        }
                        self.marks_changed();
                    }
                }
                LspEvent::Hover(text) => {
                    if self.hover_gutter || self.pending_hover_pos < 0 {
                        // Gutter hover is active or hover was dismissed; ignore late response
                    } else {
                        let formatted = editor::format_hover_markdown(&text, self.dark);
                        let combined = if !self.pending_hover_diag.is_empty()
                            && !formatted.is_empty()
                        {
                            let border = if self.dark { "#3a3a44" } else { "#d0d0d8" };
                            format!(
                                "{}<hr style='border: 0; border-top: 1px solid {border}; margin: 4px 0;'>{}",
                                self.pending_hover_diag, formatted
                            )
                        } else if !self.pending_hover_diag.is_empty() {
                            self.pending_hover_diag.clone()
                        } else if !formatted.is_empty() {
                            formatted
                        } else {
                            String::new()
                        };

                        if !combined.is_empty() {
                            if self.hover_html != combined {
                                self.hover_html = combined;
                                self.hover_changed();
                            }
                        } else if self.pending_hover_diag.is_empty() {
                            if !self.hover_html.is_empty() {
                                self.hover_html.clear();
                                self.hover_changed();
                            }
                            self.pending_hover_pos = -1;
                        }
                    }
                }
                LspEvent::Definition {
                    uri,
                    line,
                    character: _,
                } => {
                    let p = editor::lsp::uri_to_path(&uri);
                    if !p.is_empty() && p != path {
                        self.open_path(p);
                    }
                    self.goto_1based(line + 1);
                }
                LspEvent::SignatureHelp(sig) => {
                    if let Some(s) = sig {
                        let calc_param = self.current_doc().map_or(-1, |d| {
                            editor::get_param_index_at_cursor(&d.text, self.cursor_pos as usize)
                        });
                        let active_param = if calc_param >= 0 {
                            calc_param
                        } else {
                            s.active_parameter
                        };
                        self.current_signature_label = s.label.clone();
                        self.current_parameters = s.parameters.clone();
                        self.current_signature_active_param = active_param;
                        self.signature_html =
                            editor::format_signature_html(&editor::lsp::SignatureHelp {
                                label: s.label,
                                active_parameter: active_param,
                                parameters: s.parameters,
                            });
                        self.signature_visible = !self.signature_html.is_empty();
                    } else {
                        let in_params = self.current_doc().map_or(false, |d| {
                            editor::get_param_index_at_cursor(&d.text, self.cursor_pos as usize)
                                >= 0
                        });
                        if !in_params {
                            self.current_signature_label.clear();
                            self.current_parameters.clear();
                            self.current_signature_active_param = -1;
                            self.signature_html.clear();
                            self.signature_visible = false;
                        }
                    }
                    self.signature_changed();
                }
                LspEvent::Log(line) => {
                    if show_debug {
                        let line = if line.len() > 1024 {
                            let mut cut = 1024;
                            while cut > 0 && !line.is_char_boundary(cut) {
                                cut -= 1;
                            }
                            format!("{}… ({} bytes)", &line[..cut], line.len())
                        } else {
                            line
                        };
                        self.push_log_buffered(line);
                        logged_debug = true;
                    }
                }
            }
        }
        if logged_debug {
            self.log_changed();
        }
        if need_local && self.current_doc().is_some() {
            let p = prefix.clone();
            self.local_complete(&p);
        }
    }

    #[qslot]
    fn debug_start(&mut self) {
        // Sync breakpoints from all open documents before starting debug.
        let session = DebugSession::global();
        for doc in &self.docs {
            if !doc.path.is_empty() {
                let bp_lines: Vec<usize> = doc
                    .breakpoints
                    .iter()
                    .filter(|&&l| l > 0)
                    .map(|&l| l as usize)
                    .collect();
                session.set_breakpoints_for_file(Path::new(&doc.path), &bp_lines);
            }
        }
        session.resume();
        self.debug_state_changed();
    }

    #[qslot]
    fn debug_pause(&mut self) {
        DebugSession::global().pause();
        self.marks_changed();
        self.debug_state_changed();
    }

    #[qslot]
    fn debug_step(&mut self) {
        DebugSession::global().step_into();
        self.marks_changed();
        self.debug_state_changed();
    }

    #[qslot]
    fn debug_step_over(&mut self) {
        DebugSession::global().step_over();
        self.marks_changed();
        self.debug_state_changed();
    }

    #[qslot]
    fn debug_step_out(&mut self) {
        DebugSession::global().step_out();
        self.marks_changed();
        self.debug_state_changed();
    }

    #[qslot]
    fn debug_stop(&mut self) {
        DebugSession::global().stop();
        self.last_debug_line = 0;
        self.last_debug_state_gen = 0;
        self.marks_changed();
        self.debug_state_changed();
    }

    #[qslot]
    fn debug_reset(&mut self) {
        let session = DebugSession::global();
        if session.is_active() {
            session.pause();
            self.marks_changed();
        }
        self.request_debug_run();
    }

    fn fire_edit(&mut self, action: &str) {
        self.edit_action = action.into();
        self.edit_action_changed();
    }
}

fn build_firmware_device_uid_map(devices: &Value) -> BTreeMap<String, Vec<String>> {
    let mut next: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Value::Array(arr) = devices {
        for item in arr {
            let src = item
                .get("firmwareSource")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let uid = item
                .get("uid")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("id").and_then(|v| v.as_str()))
                .unwrap_or("");
            if src.is_empty() || uid.is_empty() {
                continue;
            }
            let norm = normalize_debug_path(Path::new(src));
            if norm.is_empty() {
                continue;
            }
            let entry = next.entry(norm).or_default();
            if !entry.iter().any(|existing| existing == uid) {
                entry.push(uid.to_string());
            }
        }
    }
    next
}

fn build_firmware_mcu_map(devices: Value) -> BTreeMap<String, Vec<String>> {
    let mut next: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Value::Array(arr) = devices {
        for item in arr {
            let src = item
                .get("firmwareSource")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let label = item.get("label").and_then(|v| v.as_str()).unwrap_or("");
            if src.is_empty() || label.is_empty() {
                continue;
            }
            let norm = normalize_debug_path(Path::new(src));
            if norm.is_empty() {
                continue;
            }
            let entry = next.entry(norm).or_default();
            if !entry.iter().any(|existing| existing == label) {
                entry.push(label.to_string());
            }
        }
    }
    next
}

fn adjust_close_queue_after_remove(queue: &mut Vec<usize>, removed: usize) {
    queue.retain_mut(|q| {
        if *q == removed {
            false
        } else {
            if *q > removed {
                *q -= 1;
            }
            true
        }
    });
}

fn close_indices_for_others(len: usize, keep: i32) -> Vec<usize> {
    if keep < 0 || keep as usize >= len {
        return Vec::new();
    }
    let keep = keep as usize;
    (0..len).filter(|&i| i != keep).collect()
}

fn close_indices_to_left(len: usize, index: i32) -> Vec<usize> {
    if index <= 0 || index as usize >= len {
        return Vec::new();
    }
    (0..index as usize).collect()
}

fn close_indices_to_right(len: usize, index: i32) -> Vec<usize> {
    if index < 0 || (index as usize) + 1 >= len {
        return Vec::new();
    }
    ((index as usize + 1)..len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QObjectHolder;

    fn doc_with_path(id: i32, title: &str, path: &str) -> Document {
        let mut d = Document::untitled(id, title.to_string());
        d.path = path.to_string();
        d
    }

    #[test]
    fn documents_include_mcu_names_for_matching_firmware_paths() {
        let mut panel = EditorPanel::default();
        panel.docs = vec![
            doc_with_path(1, "blink.ino", "/tmp/proj/blink/blink.ino"),
            doc_with_path(2, "sensor.ino", "/tmp/proj/sensor/sensor.ino"),
            doc_with_path(3, "notes.txt", "/tmp/proj/notes.txt"),
            Document::untitled(4, "untitled".to_string()),
        ];

        panel.firmware_mcus = build_firmware_mcu_map(json!([
            { "label": "mega328-1", "firmwareSource": "/tmp/proj/blink/blink.ino" },
            { "label": "Uno-2 / mega328-1", "firmwareSource": "/tmp/proj/blink/blink.ino" },
            { "label": "mega328-1", "firmwareSource": "/tmp/proj/blink/blink.ino" },
            { "label": "Esp32-1", "firmwareSource": "/tmp/proj/sensor/sensor.ino" },
            { "label": "Empty-1", "firmwareSource": "" },
        ]));

        let docs = panel.documents();
        let arr = docs.as_array().unwrap();
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0]["mcuNames"], "mega328-1, Uno-2 / mega328-1");
        assert_eq!(arr[1]["mcuNames"], "Esp32-1");
        assert_eq!(arr[2]["mcuNames"], "");
        assert_eq!(arr[3]["mcuNames"], "");
    }

    #[test]
    fn batch_close_index_helpers_and_queue_adjustment() {
        assert_eq!(close_indices_for_others(4, 1), vec![0, 2, 3]);
        assert_eq!(close_indices_for_others(1, 0), Vec::<usize>::new());
        assert_eq!(close_indices_for_others(4, -1), Vec::<usize>::new());
        assert_eq!(close_indices_for_others(4, 4), Vec::<usize>::new());

        assert_eq!(close_indices_to_left(4, 0), Vec::<usize>::new());
        assert_eq!(close_indices_to_left(4, 2), vec![0, 1]);
        assert_eq!(close_indices_to_left(4, 3), vec![0, 1, 2]);
        assert_eq!(close_indices_to_left(4, 4), Vec::<usize>::new());

        assert_eq!(close_indices_to_right(4, 3), Vec::<usize>::new());
        assert_eq!(close_indices_to_right(4, 1), vec![2, 3]);
        assert_eq!(close_indices_to_right(4, 0), vec![1, 2, 3]);
        assert_eq!(close_indices_to_right(4, -1), Vec::<usize>::new());

        let mut q = vec![0, 2, 3];
        let first = q.remove(0);
        assert_eq!(first, 0);
        adjust_close_queue_after_remove(&mut q, first);
        assert_eq!(q, vec![1, 2]);
        let second = q.remove(0);
        assert_eq!(second, 1);
        adjust_close_queue_after_remove(&mut q, second);
        assert_eq!(q, vec![1]);
    }

    #[test]
    fn compile_path_for_sim_saves_dirty_file_and_returns_false_without_compiler() {
        let dir = std::env::temp_dir().join("cs_test_compile_path_for_sim_no_compiler");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let asm_path = dir.join("prog.asm");
        let hdr_path = dir.join("defs.inc");
        fs::write(&asm_path, "; old\n").unwrap();
        fs::write(&hdr_path, "; old hdr\n").unwrap();
        let path_str = asm_path.to_string_lossy().into_owned();
        let hdr_str = hdr_path.to_string_lossy().into_owned();

        let mut panel = EditorPanel::default();
        let mut doc = doc_with_path(1, "prog.asm", &path_str);
        doc.text = "; updated\n".to_string();
        doc.dirty = true;
        doc.compiler = "None".to_string();
        let mut sib = doc_with_path(2, "defs.inc", &hdr_str);
        sib.text = "; updated hdr\n".to_string();
        sib.dirty = true;
        panel.docs.push(doc);
        panel.docs.push(sib);
        panel.current = 0;

        assert!(!panel.compile_path_for_sim(String::new(), String::new()));
        assert!(!panel.compile_path_for_sim(
            dir.join("missing.ino").to_string_lossy().into_owned(),
            String::new()
        ));
        let (any_saved, saved_current) = panel.save_dirty_docs_for_target(Some(0), &path_str);
        assert!(any_saved);
        assert!(saved_current);
        assert!(!panel.docs[0].dirty);
        assert!(!panel.docs[1].dirty);
        assert_eq!(fs::read_to_string(&asm_path).unwrap(), "; updated\n");
        assert_eq!(fs::read_to_string(&hdr_path).unwrap(), "; updated hdr\n");
        // Now clean and compiler is "None" -> returns false so caller runs powerOn() directly.
        assert!(!panel.compile_path_for_sim(path_str, "mcu-1".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_device_compile_targets_groups_shared_firmware_and_orders_active_last() {
        let devices = json!([
            { "uid": "mega328-1", "firmwareSource": "/tmp/proj/master/master.ino", "isActive": true },
            { "uid": "mega328-2", "firmwareSource": "/tmp/proj/slave/slave.ino", "isActive": false },
            { "uid": "Uno-3/mega328-1", "firmwareSource": "/tmp/proj/master/master.ino", "isActive": false },
            { "uid": "mega328-4", "firmwareSource": "", "isActive": false },
        ]);

        let sim_targets = compiler::collect_device_compile_targets(devices.clone(), false);
        assert_eq!(sim_targets.len(), 2);
        assert_eq!(sim_targets[0].src_path, "/tmp/proj/slave/slave.ino");
        assert_eq!(sim_targets[0].target_device_ids, vec!["mega328-2"]);
        assert!(!sim_targets[0].debug);
        assert!(!sim_targets[0].load_debug_symbols);

        assert_eq!(sim_targets[1].src_path, "/tmp/proj/master/master.ino");
        assert_eq!(
            sim_targets[1].target_device_ids,
            vec!["mega328-1", "Uno-3/mega328-1"]
        );
        assert!(!sim_targets[1].debug);
        assert!(sim_targets[1].load_debug_symbols);

        let dbg_targets = compiler::collect_device_compile_targets(devices, true);
        assert_eq!(dbg_targets.len(), 2);
        assert!(!dbg_targets[0].debug);
        assert!(!dbg_targets[0].load_debug_symbols);
        assert!(dbg_targets[1].debug);
        assert!(dbg_targets[1].load_debug_symbols);
    }

    #[test]
    fn compile_devices_for_sim_saves_all_mcu_dirty_docs_and_returns_false_without_compilers() {
        let dir = std::env::temp_dir().join("cs_test_compile_devices_for_sim");
        let _ = fs::remove_dir_all(&dir);
        let dir1 = dir.join("mcu1");
        let dir2 = dir.join("mcu2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();
        let p1 = dir1.join("fw1.asm");
        let p2 = dir2.join("fw2.asm");
        fs::write(&p1, "; old1\n").unwrap();
        fs::write(&p2, "; old2\n").unwrap();
        let s1 = p1.to_string_lossy().into_owned();
        let s2 = p2.to_string_lossy().into_owned();

        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        let mut d1 = doc_with_path(1, "fw1.asm", &s1);
        d1.text = "; new1\n".to_string();
        d1.dirty = true;
        d1.compiler = "None".to_string();
        let mut d2 = doc_with_path(2, "fw2.asm", &s2);
        d2.text = "; new2\n".to_string();
        d2.dirty = true;
        d2.compiler = "None".to_string();
        panel.docs.push(d1);
        panel.docs.push(d2);
        panel.current = 0;

        let devices = json!([
            { "uid": "mega328-1", "firmwareSource": s1, "isActive": true },
            { "uid": "mega328-2", "firmwareSource": s2, "isActive": false },
        ]);
        assert!(!panel.compile_devices_for_sim_inner(devices, false));
        assert!(!panel.docs[0].dirty);
        assert!(!panel.docs[1].dirty);
        assert_eq!(fs::read_to_string(&p1).unwrap(), "; new1\n");
        assert_eq!(fs::read_to_string(&p2).unwrap(), "; new2\n");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn doc_ids_stay_monotonic_and_stable_across_switches_and_closes() {
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        assert_eq!(panel.current_doc_id(), -1);

        panel.new_file();
        let id1 = panel.current_doc_id();
        panel.new_file();
        let id2 = panel.current_doc_id();
        panel.new_file();
        let id3 = panel.current_doc_id();
        assert!(id1 > 0 && id2 > id1 && id3 > id2);

        // Documents JSON includes matching stable IDs.
        let docs = panel.documents();
        let arr = docs.as_array().unwrap();
        assert_eq!(arr[0]["id"], id1);
        assert_eq!(arr[1]["id"], id2);
        assert_eq!(arr[2]["id"], id3);

        // Switching tabs clears stale find/goto selection state.
        panel.goto_line = 12;
        panel.select_start = 4;
        panel.select_end = 9;
        panel.set_current_document(1);
        assert_eq!(panel.current_doc_id(), id2);
        assert_eq!(panel.goto_line, 0);
        assert_eq!(panel.select_start, -1);
        assert_eq!(panel.select_end, -1);

        // Closing a tab to the left shifts index of the active doc but keeps its ID and nav state.
        panel.select_start = 2;
        panel.select_end = 5;
        panel.close_document_now(0);
        assert_eq!(panel.current, 0);
        assert_eq!(panel.current_doc_id(), id2);
        assert_eq!(panel.select_start, 2);
        assert_eq!(panel.select_end, 5);

        // Creating another file never reuses a previously closed ID.
        panel.new_file();
        let id4 = panel.current_doc_id();
        assert!(id4 > id3);
        assert_eq!(panel.select_start, -1);

        // Closing all clears currentDocId back to -1.
        panel.close_all_docs_internal();
        assert_eq!(panel.current_doc_id(), -1);
    }

    #[test]
    fn completions_hide_on_close_focus_loss_and_buffer_change() {
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        panel.new_file();
        panel.new_file();

        let show_sample_completions = |p: &mut EditorPanel| {
            p.completion_pending = true;
            p.set_completion_items(
                vec![json!({
                    "text": "digitalWrite",
                    "insertText": "digitalWrite",
                    "detail": "void(uint8_t, uint8_t)",
                    "kind": 3,
                })],
                "digi",
            );
            assert!(p.completion_visible());
            assert_eq!(p.completion_prefix(), "digi");
        };

        // Losing editor focus hides completions and clears pending state.
        panel.focused = true;
        show_sample_completions(&mut panel);
        panel.set_focused(false);
        assert!(!panel.completion_visible());
        assert!(!panel.completion_pending);
        assert!(panel.completion_prefix().is_empty());

        // Closing the active tab when another remains hides completions.
        show_sample_completions(&mut panel);
        let cur = panel.current as usize;
        panel.close_document_now(cur);
        assert!(!panel.completion_visible());
        assert!(!panel.completion_pending);

        // Creating a new buffer hides completions.
        show_sample_completions(&mut panel);
        panel.new_file();
        assert!(!panel.completion_visible());

        // Closing a dirty file (prompting save before close) hides completions.
        let cur = panel.current;
        panel.docs[cur as usize].dirty = true;
        show_sample_completions(&mut panel);
        panel.close_document(cur);
        assert!(!panel.completion_visible());
        assert!(panel.pending_close_index.is_some());
        panel.confirm_close_cancel();

        // Closing all docs hides completions.
        show_sample_completions(&mut panel);
        panel.close_all_docs_internal();
        assert!(!panel.completion_visible());
        assert_eq!(panel.current_doc_id(), -1);

        // Accepting a completion still uses the stored prefix before hide clears it.
        panel.new_file();
        panel.docs[0].text = "digi".to_string();
        panel.cursor_pos = 4;
        show_sample_completions(&mut panel);
        let edit = panel.accept_completion(0, 4);
        assert_eq!(edit["valid"], true);
        assert_eq!(edit["replaceStart"], 0);
        assert_eq!(edit["replaceEnd"], 4);
        assert!(!panel.completion_visible());
        assert!(panel.completion_prefix().is_empty());
    }

    #[test]
    fn firmware_device_uid_map_resolves_assigned_mcus_per_source() {
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        panel.set_firmware_devices(json!([
            { "uid": "mega328-1", "label": "MasterMCU", "firmwareSource": "/tmp/proj/master/master.ino", "isActive": true },
            { "uid": "mega328-2", "label": "SlaveMCU", "firmwareSource": "/tmp/proj/slave/slave.ino", "isActive": false },
            { "uid": "Uno-3/mega328-1", "label": "Uno-3 / mega328-1", "firmwareSource": "/tmp/proj/slave/slave.ino", "isActive": false },
            { "uid": "mega328-4", "label": "EmptyMCU", "firmwareSource": "", "isActive": false },
        ]));

        let norm_master = normalize_debug_path(Path::new("/tmp/proj/master/master.ino"));
        let norm_slave = normalize_debug_path(Path::new("/tmp/proj/slave/slave.ino"));
        let norm_unassigned = normalize_debug_path(Path::new("/tmp/proj/new/new.ino"));

        assert_eq!(
            panel.firmware_device_uids.get(&norm_master),
            Some(&vec!["mega328-1".to_string()])
        );
        assert_eq!(
            panel.firmware_device_uids.get(&norm_slave),
            Some(&vec![
                "mega328-2".to_string(),
                "Uno-3/mega328-1".to_string()
            ])
        );
        assert_eq!(panel.firmware_device_uids.get(&norm_unassigned), None);
    }

    #[test]
    fn document_to_cfg_preserves_raw_board_before_board_discovery() {
        let panel = EditorPanel::default_with_attached_qobject();
        let panel = panel.borrow();
        let settings = cs_engine::settings::AppSettings::default();
        let mut doc = Document::untitled(1, "esp32_uart.ino".into());
        doc.path = "/tmp/esp32_uart/esp32_uart.ino".into();
        let cfg = cs_engine::editor::FileConfig {
            compiler: "Arduino".into(),
            board: "DOIT ESP32 DEVKIT V1".into(),
            device: "esp32".into(),
            ..Default::default()
        };
        // Apply with only default_boards() (simulating before async board discovery finishes).
        doc.apply_cfg(cfg, &panel.specs, &settings, &default_boards());
        assert_eq!(doc.board, "Custom");
        assert_eq!(doc.custom_board, "DOIT ESP32 DEVKIT V1");
        assert_eq!(doc.raw_board, "DOIT ESP32 DEVKIT V1");

        // Persisting config before discovery must not overwrite Board with Custom.
        let saved = doc.to_cfg();
        assert_eq!(saved.board, "DOIT ESP32 DEVKIT V1");
        assert_eq!(saved.custom_board, "");
    }

    #[test]
    fn resolve_arduino_fqbn_uses_raw_board_and_local_packages_on_cold_start() {
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();

        // When arduino_boards already contains the board, raw_board resolves even if
        // board/custom_board were temporarily mapped to Custom before discovery.
        panel.arduino_boards.push(BoardEntry {
            name: "DOIT ESP32 DEVKIT V1".into(),
            fqbn: "esp32:esp32:esp32doit-devkit-v1".into(),
        });
        let fqbn = panel.resolve_arduino_fqbn(
            "Custom",
            "DOIT ESP32 DEVKIT V1",
            "DOIT ESP32 DEVKIT V1",
            "",
            "esp32",
            "",
        );
        assert_eq!(fqbn, "esp32:esp32:esp32doit-devkit-v1");

        // When the machine has Arduino15 ESP32 installed, a completely cold panel
        // (only default_boards()) still resolves DOIT ESP32 DEVKIT V1 to its FQBN
        // without opening the firmware file first.
        if cs_engine::editor::arduino::boards_from_arduino15()
            .iter()
            .any(|b| b.name.eq_ignore_ascii_case("DOIT ESP32 DEVKIT V1"))
        {
            panel.arduino_boards = default_boards();
            panel.arduino_list_path = None;
            let cold_fqbn = panel.resolve_arduino_fqbn(
                "Custom",
                "DOIT ESP32 DEVKIT V1",
                "DOIT ESP32 DEVKIT V1",
                "",
                "esp32",
                "",
            );
            assert_eq!(cold_fqbn, "esp32:esp32:esp32doit-devkit-v1");
        }
    }

    #[test]
    fn cold_start_compile_devices_for_sim_uses_cached_esp32_firmware_without_recompiling() {
        if !cs_engine::editor::arduino::boards_from_arduino15()
            .iter()
            .any(|b| b.name.eq_ignore_ascii_case("DOIT ESP32 DEVKIT V1"))
        {
            return;
        }

        let dir = std::env::temp_dir().join("cs_test_cold_start_esp32_sim_run");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let ino = dir.join("esp32_uart.ino");
        fs::write(&ino, "void setup() {}\nvoid loop() {}\n").unwrap();
        let ino_str = ino.to_string_lossy().into_owned();
        let cfg = cs_engine::editor::FileConfig {
            compiler: "Arduino".into(),
            board: "DOIT ESP32 DEVKIT V1".into(),
            device: "esp32".into(),
            ..Default::default()
        };
        cs_engine::editor::save_cfg(&ino_str, &cfg).unwrap();

        let canon = fs::canonicalize(&ino)
            .map(|p| p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
            .unwrap_or_else(|_| ino_str.clone());
        let build_dir = cs_engine::editor::arduino::resolve_arduino_build_dir(
            &canon,
            "esp32:esp32:esp32doit-devkit-v1",
        );
        let _ = fs::remove_dir_all(&build_dir);
        fs::create_dir_all(&build_dir).unwrap();
        let payload = b"cached-esp32-merged-binary";
        fs::write(build_dir.join("esp32_uart.ino.merged.bin"), payload).unwrap();
        fs::write(dir.join("esp32_uart.bin"), payload).unwrap();

        // Cold EditorPanel: no open docs, only default_boards() loaded.
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        assert_eq!(panel.docs.len(), 0);
        assert!(panel.arduino_list_path.is_none());

        let devices = json!([
            { "uid": "esp32-1", "firmwareSource": ino_str, "isActive": true },
        ]);
        assert!(panel.compile_devices_for_sim_inner(devices, false));
        assert!(
            panel.compile_job.is_none(),
            "expected up-to-date cached firmware hit without spawning arduino-cli compile"
        );
        assert_ne!(panel.compile_state, "error");
        assert!(
            panel.last_firmware.ends_with("esp32_uart.bin"),
            "unexpected last_firmware: {}",
            panel.last_firmware
        );

        let _ = fs::remove_dir_all(&build_dir);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_text_keeps_completions_visible_while_typing_prefix() {
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        panel.new_file();
        panel.docs[0].text = "void displaySetup() {}\nvoid dispatch() {}\ndis".to_string();
        panel.cursor_pos = panel.docs[0].text.len() as i32;
        panel.set_completion_items(
            vec![
                json!({ "text": "displaySetup", "insertText": "displaySetup", "detail": "", "kind": 3 }),
                json!({ "text": "dispatch", "insertText": "dispatch", "detail": "", "kind": 3 }),
            ],
            "dis",
        );
        assert!(panel.completion_visible());

        // Typing 'p' atomically updates text and cursor together without hiding completions.
        let next = "void displaySetup() {}\nvoid dispatch() {}\ndisp".to_string();
        let next_pos = next.len() as i32;
        panel.update_text(next, next_pos);
        assert!(panel.completion_visible());
        assert_eq!(panel.completion_prefix(), "disp");

        // Typing 'l' narrows smoothly to displaySetup (excluding dispatch) without hiding completions.
        let next2 = "void displaySetup() {}\nvoid dispatch() {}\ndispl".to_string();
        let next2_pos = next2.len() as i32;
        panel.update_text(next2, next2_pos);
        assert!(panel.completion_visible());
        assert_eq!(panel.completion_prefix(), "displ");
        let items = panel.completions();
        let arr = items.as_array().unwrap();
        assert!(arr.iter().any(|v| v["text"] == "displaySetup"), "{arr:?}");
        assert!(!arr.iter().any(|v| v["text"] == "dispatch"), "{arr:?}");
    }

    #[test]
    fn highlight_viewport_windows_large_documents() {
        let panel = EditorPanel::default_with_attached_qobject();
        let mut panel = panel.borrow_mut();
        panel.new_file();
        let lines: Vec<String> = (1..=434)
            .map(|i| format!("int line_{i} = {i}; // marker_{i}"))
            .collect();
        panel.docs[0].syntax = Some("cpp.syntax".to_string());
        panel.docs[0].text = lines.join("\n");
        panel.rehighlight();

        assert_eq!(panel.highlight_start_line(), 0);
        assert!(panel.highlight_html().contains("marker_1"));
        assert!(!panel.highlight_html().contains("marker_400"));

        panel.set_highlight_viewport(350, 434);
        assert_eq!(panel.highlight_start_line(), 350);
        assert!(panel.highlight_html().contains("marker_400"));
        assert!(!panel.highlight_html().contains("marker_1\n"));
    }
}
