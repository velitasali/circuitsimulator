//! Simulator and compiler log panes. Two singletons: one engine cannot host
//! two instances of the same type, and the QML already takes `ctx` per pane.

use cs_engine::logging::LogLevel;
use cs_engine::theme::{ColorId, ColorTheme};
use qtbridge::qobject;
use regex::Regex;
use std::sync::OnceLock;

static LOC_REGEX: OnceLock<Regex> = OnceLock::new();
static SCRIPT_LOC_REGEX: OnceLock<Regex> = OnceLock::new();

/// Match a file and line reference from a log text line.
///
/// Matches C++ `OutPanelText::locationAt` regex:
/// `((?:[A-Za-z]:[\\/])?[^\s:()]+\.[A-Za-z0-9_+]+):(\d+)`
/// plus AngelScript `file.as line: \d+` format.
pub fn parse_location_from_line(line: &str) -> Option<(String, i32)> {
    let re = LOC_REGEX.get_or_init(|| {
        Regex::new(r"((?:[A-Za-z]:[\\/])?[^\s:()]+\.[A-Za-z0-9_+]+):(\d+)").expect("valid regex")
    });
    if let Some(caps) = re.captures(line) {
        if let (Some(f), Some(l)) = (caps.get(1), caps.get(2)) {
            if let Ok(line_num) = l.as_str().parse::<i32>() {
                if line_num > 0 {
                    return Some((f.as_str().to_string(), line_num));
                }
            }
        }
    }

    let script_re = SCRIPT_LOC_REGEX.get_or_init(|| {
        Regex::new(r"([^\s:()]+\.[A-Za-z0-9_+]+)\s+line:\s*(\d+)").expect("valid script regex")
    });
    if let Some(caps) = script_re.captures(line) {
        if let (Some(f), Some(l)) = (caps.get(1), caps.get(2)) {
            if let Ok(line_num) = l.as_str().parse::<i32>() {
                if line_num > 0 {
                    return Some((f.as_str().to_string(), line_num));
                }
            }
        }
    }

    None
}

/// Find a file:line location at character position `pos` in `text`.
pub fn find_location_in_text(text: &str, pos: i32) -> Option<(String, i32)> {
    if pos < 0 || text.is_empty() {
        return None;
    }
    let pos = pos as usize;
    let mut current_char_idx = 0;
    let mut line_start = 0;
    let mut target_line = None;

    for (byte_offset, ch) in text.char_indices() {
        if ch == '\n' {
            if current_char_idx >= pos {
                target_line = Some(&text[line_start..byte_offset]);
                break;
            }
            line_start = byte_offset + 1;
        }
        current_char_idx += ch.len_utf16();
    }
    if target_line.is_none() && pos <= current_char_idx {
        target_line = Some(&text[line_start..]);
    }

    let line_str = target_line?;
    parse_location_from_line(line_str)
}

/// Automatically classify a log line to determine its visual severity or stream category.
pub fn classify_line(line: &str) -> LogLevel {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return LogLevel::Stdout;
    }

    if line.starts_with("[stderr]") || line.starts_with("[clangd stderr]") {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error:") || lower.contains("fatal error:") {
            return LogLevel::Error;
        }
        if lower.contains("warning:") {
            return LogLevel::Warning;
        }
        return LogLevel::Stderr;
    }

    let lower = trimmed.to_ascii_lowercase();

    // 1. Error conditions (compiler error lines, crash notices, runtime errors)
    if lower.contains(": error:")
        || lower.contains(": fatal error:")
        || lower.starts_with("error:")
        || lower.starts_with("fatal error:")
        || lower.starts_with("error ")
        || lower.contains(" error: ")
        || lower.contains(" error ")
        || lower.contains("error: ")
        || lower.starts_with("crash:")
        || lower.contains("crash:")
        || lower.starts_with("failed to ")
        || lower.contains("no such file or directory")
        || (lower.contains("line:") && lower.contains("error"))
    {
        return LogLevel::Error;
    }

    // 2. Warning conditions (compiler warnings, overload notices)
    if lower.contains(": warning:")
        || lower.starts_with("warning:")
        || lower.starts_with("warning ")
        || lower.contains(" warning: ")
        || lower.contains(" warning ")
        || lower.contains("warning: ")
        || lower.starts_with("[circuit] overload")
        || lower.contains("overload on ")
        || (lower.contains("line:") && lower.contains("warning"))
    {
        return LogLevel::Warning;
    }

    // 3. Success / completion conditions
    if lower.starts_with("compilation finished")
        || lower.starts_with("firmware uploaded")
        || lower.starts_with("uploading done")
        || lower.contains("successfully")
        || lower.ends_with("done.")
        || lower.ends_with("succeeded.")
    {
        return LogLevel::Success;
    }

    // 4. Info / toolchain / step notices
    if lower.starts_with("compiling ")
        || lower.starts_with("linking ")
        || lower.starts_with("archiving ")
        || lower.starts_with("generating ")
        || lower.starts_with("starting simulation")
        || lower.starts_with("simulation started")
        || lower.starts_with("simulation stopped")
        || lower.starts_with("simulation paused")
        || lower.starts_with("starting debug session")
        || lower.starts_with("debug session started")
        || lower.starts_with("reset")
        || lower.starts_with("---")
        || lower.starts_with("===")
        || lower.contains(": note:")
        || lower.starts_with("note:")
        || lower.starts_with("info:")
    {
        return LogLevel::Info;
    }

    LogLevel::Stdout
}

fn html_escape(s: &str) -> String {
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

fn color_for_level(level: LogLevel, dark: bool) -> String {
    let color_id = match level {
        LogLevel::Default | LogLevel::Stdout => ColorId::LogStdout,
        LogLevel::Stderr => ColorId::LogStderr,
        LogLevel::Error => ColorId::LogErrorText,
        LogLevel::Warning => ColorId::LogWarnText,
        LogLevel::Info => ColorId::LogInfoText,
        LogLevel::Success => ColorId::LogSuccessText,
    };
    ColorTheme::get_hex(color_id, dark)
}

fn render_entry_html(line: &str, level: LogLevel, dark: bool) -> String {
    let hex = color_for_level(level, dark);
    let escaped = html_escape(line);
    format!("<span style=\"color: {hex};\">{escaped}</span>")
}

pub struct SimulatorLog {
    text: String,
    raw_text: String,
    entries: Vec<(String, LogLevel)>,
    dark: bool,
    placeholder: String,
    text_color: String,
    base_color: String,
    font_family: String,
    font_size: i32,
    location_file: String,
    location_line: i32,
}

impl Default for SimulatorLog {
    fn default() -> Self {
        let s = cs_engine::settings::get();
        Self {
            text: String::new(),
            raw_text: String::new(),
            entries: Vec::new(),
            dark: false,
            placeholder: cs_engine::i18n::tr("Simulator messages"),
            text_color: String::new(),
            base_color: String::new(),
            font_family: s.editor_font_family,
            font_size: s.editor_font_size,
            location_file: String::new(),
            location_line: 0,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl SimulatorLog {
    qproperty!("text", Read = text, Notify = text_changed);
    qproperty!("placeholder", Read = placeholder, Notify = style_changed);
    qproperty!("textColor", Read = text_color, Notify = style_changed);
    qproperty!("baseColor", Read = base_color, Notify = style_changed);
    qproperty!("fontFamily", Read = font_family, Notify = style_changed);
    qproperty!("fontSize", Read = font_size, Notify = style_changed);
    qproperty!(
        "locationFile",
        Read = location_file,
        Notify = location_clicked
    );
    qproperty!(
        "locationLine",
        Read = location_line,
        Notify = location_clicked
    );

    #[qsignal]
    fn text_changed(&mut self);
    #[qsignal]
    fn style_changed(&mut self);
    #[qsignal]
    fn appended(&mut self);
    #[qsignal]
    fn copy_requested(&mut self);
    #[qsignal]
    fn select_all_requested(&mut self);
    #[qsignal]
    fn location_clicked(&mut self);

    fn text(&self) -> String {
        self.text.clone()
    }

    fn placeholder(&self) -> String {
        self.placeholder.clone()
    }

    fn text_color(&self) -> String {
        self.text_color.clone()
    }

    fn base_color(&self) -> String {
        self.base_color.clone()
    }

    fn font_family(&self) -> String {
        self.font_family.clone()
    }

    fn font_size(&self) -> i32 {
        self.font_size
    }

    fn location_file(&self) -> String {
        self.location_file.clone()
    }

    fn location_line(&self) -> i32 {
        self.location_line
    }

    #[qslot]
    fn reload_settings(&mut self) {
        let s = cs_engine::settings::get();
        self.placeholder = cs_engine::i18n::tr("Simulator messages");
        self.set_editor_font(s.editor_font_family, s.editor_font_size);
    }

    #[qslot]
    fn set_dark(&mut self, dark: bool) {
        if self.dark != dark {
            self.dark = dark;
            self.rebuild_html();
        }
    }

    fn rebuild_html(&mut self) {
        let mut html = String::new();
        for (i, (line, level)) in self.entries.iter().enumerate() {
            if i > 0 {
                html.push_str("<br/>");
            }
            html.push_str(&render_entry_html(line, *level, self.dark));
        }
        self.text = html;
        self.text_changed();
    }

    #[qslot]
    fn set_editor_font(&mut self, family: String, size: i32) {
        if self.font_family != family || self.font_size != size {
            self.font_family = family;
            self.font_size = size;
            self.style_changed();
        }
    }

    #[qslot]
    fn append_line(&mut self, line: String) {
        let level = classify_line(&line);
        self.append_line_with_level(line, level);
    }

    pub fn append_line_with_level(&mut self, line: String, level: LogLevel) {
        if !self.raw_text.is_empty() && !self.raw_text.ends_with('\n') {
            self.raw_text.push('\n');
        }
        self.raw_text.push_str(&line);

        if !self.text.is_empty() {
            self.text.push_str("<br/>");
        }
        self.text
            .push_str(&render_entry_html(&line, level, self.dark));
        self.entries.push((line, level));

        self.text_changed();
        self.appended();
    }

    #[qslot]
    fn clear(&mut self) {
        if self.text.is_empty() && self.raw_text.is_empty() {
            return;
        }
        self.entries.clear();
        self.raw_text.clear();
        self.text.clear();
        self.text_changed();
    }

    #[qslot]
    fn is_location_at(&self, pos: i32) -> bool {
        find_location_in_text(&self.raw_text, pos).is_some()
    }

    #[qslot]
    fn activate_location_at(&mut self, pos: i32) {
        if let Some((file, line)) = find_location_in_text(&self.raw_text, pos) {
            self.location_file = file;
            self.location_line = line;
            self.location_clicked();
        }
    }

    #[qslot]
    fn show_context_menu(&self, _has_selection: bool) {}
}

pub struct CompilerLog {
    text: String,
    raw_text: String,
    entries: Vec<(String, LogLevel)>,
    dark: bool,
    placeholder: String,
    text_color: String,
    base_color: String,
    font_family: String,
    font_size: i32,
    location_file: String,
    location_line: i32,
}

impl Default for CompilerLog {
    fn default() -> Self {
        let s = cs_engine::settings::get();
        Self {
            text: String::new(),
            raw_text: String::new(),
            entries: Vec::new(),
            dark: false,
            placeholder: cs_engine::i18n::tr("Compiler messages"),
            text_color: String::new(),
            base_color: String::new(),
            font_family: s.editor_font_family,
            font_size: s.editor_font_size,
            location_file: String::new(),
            location_line: 0,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl CompilerLog {
    qproperty!("text", Read = text, Notify = text_changed);
    qproperty!("placeholder", Read = placeholder, Notify = style_changed);
    qproperty!("textColor", Read = text_color, Notify = style_changed);
    qproperty!("baseColor", Read = base_color, Notify = style_changed);
    qproperty!("fontFamily", Read = font_family, Notify = style_changed);
    qproperty!("fontSize", Read = font_size, Notify = style_changed);
    qproperty!(
        "locationFile",
        Read = location_file,
        Notify = location_clicked
    );
    qproperty!(
        "locationLine",
        Read = location_line,
        Notify = location_clicked
    );

    #[qsignal]
    fn text_changed(&mut self);
    #[qsignal]
    fn style_changed(&mut self);
    #[qsignal]
    fn appended(&mut self);
    #[qsignal]
    fn copy_requested(&mut self);
    #[qsignal]
    fn select_all_requested(&mut self);
    #[qsignal]
    fn location_clicked(&mut self);

    fn text(&self) -> String {
        self.text.clone()
    }

    fn placeholder(&self) -> String {
        self.placeholder.clone()
    }

    fn text_color(&self) -> String {
        self.text_color.clone()
    }

    fn base_color(&self) -> String {
        self.base_color.clone()
    }

    fn font_family(&self) -> String {
        self.font_family.clone()
    }

    fn font_size(&self) -> i32 {
        self.font_size
    }

    fn location_file(&self) -> String {
        self.location_file.clone()
    }

    fn location_line(&self) -> i32 {
        self.location_line
    }

    #[qslot]
    fn reload_settings(&mut self) {
        let s = cs_engine::settings::get();
        self.placeholder = cs_engine::i18n::tr("Compiler messages");
        self.set_editor_font(s.editor_font_family, s.editor_font_size);
    }

    #[qslot]
    fn set_dark(&mut self, dark: bool) {
        if self.dark != dark {
            self.dark = dark;
            self.rebuild_html();
        }
    }

    fn rebuild_html(&mut self) {
        let mut html = String::new();
        for (i, (line, level)) in self.entries.iter().enumerate() {
            if i > 0 {
                html.push_str("<br/>");
            }
            html.push_str(&render_entry_html(line, *level, self.dark));
        }
        self.text = html;
        self.text_changed();
    }

    #[qslot]
    fn set_editor_font(&mut self, family: String, size: i32) {
        if self.font_family != family || self.font_size != size {
            self.font_family = family;
            self.font_size = size;
            self.style_changed();
        }
    }

    #[qslot]
    fn append_line(&mut self, line: String) {
        let level = classify_line(&line);
        self.append_line_with_level(line, level);
    }

    pub fn append_line_with_level(&mut self, line: String, level: LogLevel) {
        if !self.raw_text.is_empty() && !self.raw_text.ends_with('\n') {
            self.raw_text.push('\n');
        }
        self.raw_text.push_str(&line);

        if !self.text.is_empty() {
            self.text.push_str("<br/>");
        }
        self.text
            .push_str(&render_entry_html(&line, level, self.dark));
        self.entries.push((line, level));

        self.text_changed();
        self.appended();
    }

    #[qslot]
    fn clear(&mut self) {
        if self.text.is_empty() && self.raw_text.is_empty() {
            return;
        }
        self.entries.clear();
        self.raw_text.clear();
        self.text.clear();
        self.text_changed();
    }

    #[qslot]
    fn is_location_at(&self, pos: i32) -> bool {
        find_location_in_text(&self.raw_text, pos).is_some()
    }

    #[qslot]
    fn activate_location_at(&mut self, pos: i32) {
        if let Some((file, line)) = find_location_in_text(&self.raw_text, pos) {
            self.location_file = file;
            self.location_line = line;
            self.location_clicked();
        }
    }

    #[qslot]
    fn show_context_menu(&self, _has_selection: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gcc_error() {
        let line = "main.c:15:2: error: expected ';' before '}' token";
        assert_eq!(
            parse_location_from_line(line),
            Some(("main.c".to_string(), 15))
        );
    }

    #[test]
    fn parse_unix_path() {
        let line = "/home/user/project/test.cpp:123: warning: unused variable";
        assert_eq!(
            parse_location_from_line(line),
            Some(("/home/user/project/test.cpp".to_string(), 123))
        );
    }

    #[test]
    fn parse_windows_path() {
        let line = r"C:\project\src\main.cpp:42: error: syntax error";
        assert_eq!(
            parse_location_from_line(line),
            Some((r"C:\project\src\main.cpp".to_string(), 42))
        );
    }

    #[test]
    fn parse_script_error() {
        let line = "script.as line: 8 1 ERROR No matching signatures to 'print()'";
        assert_eq!(
            parse_location_from_line(line),
            Some(("script.as".to_string(), 8))
        );
    }

    #[test]
    fn find_location_in_multiline() {
        let text = "Compiling...\nmain.c:10: error: bad token\nDone.\n";
        // Position inside "main.c:10: error: bad token"
        let pos = 15;
        assert_eq!(
            find_location_in_text(text, pos),
            Some(("main.c".to_string(), 10))
        );
        // Position inside "Compiling..."
        assert_eq!(find_location_in_text(text, 5), None);
    }

    #[test]
    fn test_simulator_and_compiler_log_editor_font() {
        let sim_log = SimulatorLog::default();
        let comp_log = CompilerLog::default();
        let s = cs_engine::settings::get();
        assert_eq!(sim_log.font_family, s.editor_font_family);
        assert_eq!(sim_log.font_size, s.editor_font_size);
        assert_eq!(comp_log.font_family, s.editor_font_family);
        assert_eq!(comp_log.font_size, s.editor_font_size);
    }
}
