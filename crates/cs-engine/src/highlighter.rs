//! Port of C++ `Highlighter` over the custom `.syntax` files. Output is HTML
//! for a QML overlay (`Text.RichText`); qt-bridge cannot host
//! `QSyntaxHighlighter`.

use crate::compat::simulide;
use crate::theme::{ColorId, ColorTheme};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanStyle {
    pub fg: u32,
    pub bg: Option<u32>,
    pub bold: bool,
    pub italic: bool,
}

impl SpanStyle {
    fn default_text(dark: bool) -> Self {
        Self {
            fg: ColorTheme::rgb_u32(ColorId::EditorText, dark),
            bg: None,
            bold: false,
            italic: false,
        }
    }

    fn css(self) -> String {
        let mut s = format!("color:#{:06x}", self.fg);
        if let Some(bg) = self.bg {
            s.push_str(&format!(";background-color:#{bg:06x}"));
        }
        if self.bold {
            s.push_str(";font-weight:bold");
        }
        if self.italic {
            s.push_str(";font-style:italic");
        }
        s
    }
}

enum Rule {
    Keywords(HashMap<String, SpanStyle>),
    Regex {
        re: Regex,
        style: SpanStyle,
        on_lower: bool,
    },
}

struct Syntax {
    rules: Vec<Rule>,
    multi_start: Option<Regex>,
    multi_end: Option<Regex>,
    multi_style: SpanStyle,
}

fn syntax_map() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    MAP.get_or_init(|| {
        HashMap::from([
            (
                "cpp.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/cpp.syntax"),
            ),
            (
                "xml.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/xml.syntax"),
            ),
            (
                "js.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/js.syntax"),
            ),
            (
                "hex.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/hex.syntax"),
            ),
            (
                "avrasm.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/avrasm.syntax"),
            ),
            (
                "pic14asm.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/pic14asm.syntax"),
            ),
            (
                "i51asm.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/i51asm.syntax"),
            ),
            (
                "z80asm.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/z80asm.syntax"),
            ),
            (
                "6502asm.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/6502asm.syntax"),
            ),
            (
                "gcbasic.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/gcbasic.syntax"),
            ),
            (
                "makef.syntax",
                include_str!("../../../resources/data/codeeditor/syntax/makef.syntax"),
            ),
        ])
    })
}

/// Choose a `.syntax` file from a path, matching `CodeEditor` extension tests.
pub fn syntax_for_path(path: &str) -> Option<&'static str> {
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path);
    if name.eq_ignore_ascii_case("makefile") {
        return Some("makef.syntax");
    }
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if simulide::is_circuit_extension(&ext) {
        return Some("xml.syntax");
    }
    match ext.as_str() {
        "cpp" | "c" | "h" | "hpp" | "cc" | "hh" | "ino" | "as" => Some("cpp.syntax"),
        "xml" | "html" | "package" | "mcu" | "simu" => Some("xml.syntax"),
        "js" => Some("js.syntax"),
        "hex" | "ihx" => Some("hex.syntax"),
        "gcb" => Some("gcbasic.syntax"),
        "a51" => Some("i51asm.syntax"),
        "s" => Some("avrasm.syntax"),
        "asm" => Some(guess_asm(path)),
        _ => None,
    }
}

fn guess_asm(path: &str) -> &'static str {
    let Ok(text) = std::fs::read_to_string(path) else {
        return "avrasm.syntax";
    };
    let lower = text.to_ascii_lowercase();
    let pic = ["incf", "decf", "bcf", "bsf", "clrf", "movwf"]
        .iter()
        .filter(|w| lower.contains(*w))
        .count();
    let avr = ["ldi", "sbi", "cbi", "rjmp", "rcall", "out "]
        .iter()
        .filter(|w| lower.contains(*w))
        .count();
    let i51 = ["sjmp", "acall", "ajmp", "cpl", "setb"]
        .iter()
        .filter(|w| lower.contains(*w))
        .count();
    if pic > avr && pic > i51 {
        "pic14asm.syntax"
    } else if i51 > avr && i51 > pic {
        "i51asm.syntax"
    } else {
        "avrasm.syntax"
    }
}

/// Port of C++ `Highlighter::darkColor`: invert HSL lightness, floor at 140.
/// Qt `QColor::getHsl`/`setHsl` integers are h 0..=359 (or -1), s/l 0..=255.
fn dark_color(c: u32) -> u32 {
    let r = ((c >> 16) & 0xFF) as i32;
    let g = ((c >> 8) & 0xFF) as i32;
    let b = (c & 0xFF) as i32;
    let (h, s, mut l) = rgb_to_hsl_qt(r, g, b);
    if l < 150 {
        l = 255 - l;
    }
    if l < 140 {
        l = 140;
    }
    hsl_to_rgb_qt(h, s, l)
}

fn rgb_to_hsl_qt(r: i32, g: i32, b: i32) -> (i32, i32, i32) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let lightness = 0.5 * (max + min);
    let l = (lightness * 255.0).round() as i32;
    if delta.abs() < 1e-12 {
        return (-1, 0, l);
    }
    let saturation = if lightness < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };
    let mut hue = if (rf - max).abs() < 1e-12 {
        (gf - bf) / delta
    } else if (gf - max).abs() < 1e-12 {
        2.0 + (bf - rf) / delta
    } else {
        4.0 + (rf - gf) / delta
    };
    hue *= 60.0;
    if hue < 0.0 {
        hue += 360.0;
    }
    (hue.round() as i32, (saturation * 255.0).round() as i32, l)
}

fn hsl_to_rgb_qt(h: i32, s: i32, l: i32) -> u32 {
    let lf = l as f64 / 255.0;
    let sf = s as f64 / 255.0;
    let (rf, gf, bf) = if s == 0 || h < 0 {
        (lf, lf, lf)
    } else {
        let q = if lf < 0.5 {
            lf * (1.0 + sf)
        } else {
            lf + sf - lf * sf
        };
        let p = 2.0 * lf - q;
        let hf = f64::from(h.rem_euclid(360)) / 360.0;
        (
            hsl_helper(p, q, hf + 1.0 / 3.0),
            hsl_helper(p, q, hf),
            hsl_helper(p, q, hf - 1.0 / 3.0),
        )
    };
    let nr = (rf * 255.0).round().clamp(0.0, 255.0) as u32;
    let ng = (gf * 255.0).round().clamp(0.0, 255.0) as u32;
    let nb = (bf * 255.0).round().clamp(0.0, 255.0) as u32;
    (nr << 16) | (ng << 8) | nb
}

fn hsl_helper(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn parse_color(tok: &str) -> Option<u32> {
    if tok == "default" {
        return None;
    }
    u32::from_str_radix(tok.trim_start_matches('#'), 16).ok()
}

fn rem_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn compile_re(pat: &str) -> Option<Regex> {
    Regex::new(pat).ok()
}

fn parse_syntax(src: &str, dark: bool, show_spaces: bool) -> Syntax {
    let mut rules_names: Vec<String> = Vec::new();
    let mut styles: HashMap<String, SpanStyle> = HashMap::new();
    let mut out = Syntax {
        rules: Vec::new(),
        multi_start: None,
        multi_end: None,
        multi_style: SpanStyle::default_text(dark),
    };

    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("rules:") {
            rules_names = rest.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        for rule in &rules_names {
            let first = words[0];
            if !first.starts_with(rule.as_str()) {
                continue;
            }
            if first.ends_with("-style:") {
                let mut w = words.iter().skip(1);
                let mut style = SpanStyle::default_text(dark);
                if let Some(fg) = w.next() {
                    if let Some(mut c) = parse_color(fg) {
                        if dark {
                            c = dark_color(c);
                        }
                        style.fg = c;
                    }
                }
                if let Some(bg) = w.next() {
                    if let Some(mut c) = parse_color(bg) {
                        if dark {
                            c = dark_color(c);
                        }
                        style.bg = Some(c);
                    }
                }
                if let Some(b) = w.next() {
                    style.bold = b.eq_ignore_ascii_case("true");
                }
                if let Some(i) = w.next() {
                    style.italic = i.eq_ignore_ascii_case("true");
                }
                styles.insert(rule.clone(), style);
            } else {
                let style = styles
                    .get(rule.as_str())
                    .copied()
                    .unwrap_or_else(|| SpanStyle::default_text(dark));
                if first.contains("multiLineComment") {
                    out.multi_style = style;
                    let rest: Vec<&str> = words.iter().skip(1).copied().collect();
                    if rest.len() >= 2 {
                        let start = rem_quotes(rest[0]).replace("\\\\", "\\");
                        let end = rem_quotes(rest[1]).replace("\\\\", "\\");
                        out.multi_start = compile_re(&start);
                        out.multi_end = compile_re(&end);
                    }
                } else {
                    for exp in words.iter().skip(1) {
                        if !exp.starts_with('"')
                            && !exp.is_empty()
                            && exp.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        {
                            let kw = exp.to_ascii_lowercase();
                            if let Some(Rule::Keywords(map)) = out.rules.last_mut() {
                                map.insert(kw, style);
                            } else {
                                let mut map = HashMap::new();
                                map.insert(kw, style);
                                out.rules.push(Rule::Keywords(map));
                            }
                            continue;
                        }
                        let pat = if exp.starts_with('"') {
                            rem_quotes(exp).to_string()
                        } else {
                            format!(r"\b{exp}\b")
                        };
                        if let Some(re) = compile_re(&pat) {
                            out.rules.push(Rule::Regex {
                                re,
                                style,
                                on_lower: true,
                            });
                        }
                    }
                }
            }
            break;
        }
    }

    if show_spaces {
        let space = SpanStyle {
            fg: ColorTheme::rgb_u32(ColorId::EditorSpace, dark),
            bg: None,
            bold: false,
            italic: false,
        };
        if let Some(re) = compile_re(" ") {
            out.rules.push(Rule::Regex {
                re,
                style: space,
                on_lower: true,
            });
        }
        if let Some(re) = compile_re("\t") {
            out.rules.push(Rule::Regex {
                re,
                style: space,
                on_lower: true,
            });
        }
    }

    out
}

#[inline]
fn is_ascii_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_word_boundary_left(hay: &str, start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let prev = hay.as_bytes()[start - 1];
    if prev < 0x80 {
        !is_ascii_word(prev)
    } else {
        hay[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    }
}

fn is_word_boundary_right(hay: &str, end: usize) -> bool {
    if end >= hay.len() {
        return true;
    }
    let next = hay.as_bytes()[end];
    if next < 0x80 {
        !is_ascii_word(next)
    } else {
        hay[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    }
}

fn apply_keywords(hay: &str, map: &HashMap<String, SpanStyle>, fmt: &mut [Option<SpanStyle>]) {
    let bytes = hay.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if is_ascii_word(bytes[i]) {
            let start = i;
            i += 1;
            while i < len && is_ascii_word(bytes[i]) {
                i += 1;
            }
            if is_word_boundary_left(hay, start) && is_word_boundary_right(hay, i) {
                if let Some(&style) = map.get(&hay[start..i]) {
                    let s = start.min(fmt.len());
                    let e = i.min(fmt.len());
                    for slot in fmt.iter_mut().take(e).skip(s) {
                        *slot = Some(style);
                    }
                }
            }
        } else {
            i += 1;
        }
    }
}

fn apply_rule(text: &str, hay: &str, re: &Regex, style: SpanStyle, fmt: &mut [Option<SpanStyle>]) {
    for m in re.find_iter(hay) {
        let s = m.start().min(fmt.len());
        let e = m.end().min(fmt.len());
        for slot in fmt.iter_mut().take(e).skip(s) {
            *slot = Some(style);
        }
        let _ = text;
    }
}

/// Unquoted tokens longer than 2 chars, matching C++ `Highlighter::readSyntaxFile`
/// keyword collection for local completion.
pub fn keywords_for_syntax(syntax_file: Option<&str>) -> Vec<String> {
    let Some(name) = syntax_file else {
        return Vec::new();
    };
    let Some(src) = syntax_map().get(name) else {
        return Vec::new();
    };
    let mut rules_names: Vec<String> = Vec::new();
    let mut words = Vec::new();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("rules:") {
            rules_names = rest.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.is_empty() {
            continue;
        }
        for rule in &rules_names {
            let first = toks[0];
            if !first.starts_with(rule.as_str()) {
                continue;
            }
            if first.ends_with("-style:") || first.contains("multiLineComment") {
                break;
            }
            for exp in toks.iter().skip(1) {
                if !exp.starts_with('"') && exp.len() > 2 {
                    words.push((*exp).to_string());
                }
            }
            break;
        }
    }
    words.sort();
    words.dedup();
    words
}

fn sanitize_font_family(family: &str) -> String {
    let cleaned: String = family
        .chars()
        .filter(|c| !matches!(*c, '\'' | '"' | ';' | '<' | '>'))
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "Ubuntu Mono".into()
    } else {
        cleaned.to_string()
    }
}

fn pre_open(font_family: &str, font_size: i32, dark: bool) -> String {
    // Qt's rich-text default stylesheet sets `pre { font-family: fixed }`,
    // which ignores Text.font. C++ never hit this: QSyntaxHighlighter painted
    // the same QTextDocument whose defaultFont was the user's editor QFont.
    // Trailing '\n' is consumed by QTextHtmlParser's `<pre>\n` stripping rule
    // so a leading newline in the document itself is never eaten.
    let family = sanitize_font_family(font_family);
    let size = font_size.clamp(6, 72);
    let fg = ColorTheme::rgb_u32(ColorId::EditorText, dark);
    format!("<pre style=\"margin:0;font-family:'{family}';font-size:{size}px;color:#{fg:06x};\">\n")
}

fn cached_syntax(name: &str, src: &str, dark: bool, show_spaces: bool) -> Arc<Syntax> {
    static CACHE: OnceLock<Mutex<HashMap<(String, bool, bool), Arc<Syntax>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (name.to_string(), dark, show_spaces);
    if let Ok(g) = cache.lock() {
        if let Some(syn) = g.get(&key) {
            return Arc::clone(syn);
        }
    }
    let compiled = Arc::new(parse_syntax(src, dark, show_spaces));
    if let Ok(mut g) = cache.lock() {
        g.insert(key, Arc::clone(&compiled));
    }
    compiled
}

fn highlight_single_line(line: &str, syn: &Syntax, in_multi: &mut bool) -> String {
    let mut fmt = vec![None; line.len()];
    let lower = if line.is_ascii() {
        line.to_ascii_lowercase()
    } else {
        line.to_lowercase()
    };
    for rule in &syn.rules {
        match rule {
            Rule::Keywords(map) => apply_keywords(lower.as_str(), map, &mut fmt),
            Rule::Regex {
                re,
                style,
                on_lower,
            } => {
                let hay = if *on_lower { lower.as_str() } else { line };
                apply_rule(line, hay, re, *style, &mut fmt);
            }
        }
    }
    if let (Some(start), Some(end)) = (&syn.multi_start, &syn.multi_end) {
        let mut start_index = if *in_multi {
            Some(0usize)
        } else {
            start.find(line).map(|m| m.start())
        };
        while let Some(from) = start_index {
            let from = from.min(line.len());
            let (comment_end, closed) = match end.find(&line[from..]) {
                Some(m) => (from + m.end(), true),
                None => (line.len(), false),
            };
            for slot in fmt.iter_mut().take(comment_end).skip(from) {
                *slot = Some(syn.multi_style);
            }
            *in_multi = !closed;
            if !closed {
                break;
            }
            start_index = start
                .find(&line[comment_end..])
                .map(|m| comment_end + m.start());
        }
    }
    spans_to_html(line, &fmt)
}

/// Per-document incremental line highlight cache.
/// Re-highlights only edited lines (plus any trailing lines whose multiline
/// comment entry state changed) and renders arbitrary line windows in O(window).
#[derive(Default)]
pub struct HighlightCache {
    syntax: Option<String>,
    dark: bool,
    show_spaces: bool,
    lines: Vec<String>,
    html_lines: Vec<String>,
    in_multi_after: Vec<bool>,
}

impl HighlightCache {
    pub fn line_count(&self) -> usize {
        self.html_lines.len()
    }

    pub fn update(&mut self, text: &str, syntax_file: Option<&str>, dark: bool, show_spaces: bool) {
        let syn = syntax_file.and_then(|name| {
            syntax_map()
                .get(name)
                .map(|src| cached_syntax(name, src, dark, show_spaces))
        });
        let normalized_syntax = syn.as_ref().and_then(|_| syntax_file.map(str::to_string));

        let config_changed = self.syntax != normalized_syntax
            || self.dark != dark
            || self.show_spaces != show_spaces;

        let new_lines: Vec<&str> = if text.is_empty() {
            vec![""]
        } else {
            text.split('\n').collect()
        };

        if config_changed || self.lines.is_empty() {
            self.syntax = normalized_syntax;
            self.dark = dark;
            self.show_spaces = show_spaces;
            self.lines.clear();
            self.html_lines.clear();
            self.in_multi_after.clear();
            self.lines.reserve(new_lines.len());
            self.html_lines.reserve(new_lines.len());
            self.in_multi_after.reserve(new_lines.len());

            let mut in_multi = false;
            for line in new_lines {
                let html = match &syn {
                    Some(s) => highlight_single_line(line, s, &mut in_multi),
                    None => html_escape(line),
                };
                self.lines.push(line.to_string());
                self.html_lines.push(html);
                self.in_multi_after.push(in_multi);
            }
            return;
        }

        let old_len = self.lines.len();
        let new_len = new_lines.len();

        let mut prefix = 0usize;
        let min_len = old_len.min(new_len);
        while prefix < min_len && self.lines[prefix] == new_lines[prefix] {
            prefix += 1;
        }

        if prefix == old_len && prefix == new_len {
            return;
        }

        let mut suffix = 0usize;
        while suffix < (old_len - prefix)
            && suffix < (new_len - prefix)
            && self.lines[old_len - 1 - suffix] == new_lines[new_len - 1 - suffix]
        {
            suffix += 1;
        }

        let old_mid_end = old_len - suffix;
        let new_mid_end = new_len - suffix;

        let mut in_multi = if prefix > 0 {
            self.in_multi_after[prefix - 1]
        } else {
            false
        };

        let mut mid_lines = Vec::with_capacity(new_mid_end - prefix);
        let mut mid_html = Vec::with_capacity(new_mid_end - prefix);
        let mut mid_multi = Vec::with_capacity(new_mid_end - prefix);
        for line in &new_lines[prefix..new_mid_end] {
            let html = match &syn {
                Some(s) => highlight_single_line(line, s, &mut in_multi),
                None => html_escape(line),
            };
            mid_lines.push((*line).to_string());
            mid_html.push(html);
            mid_multi.push(in_multi);
        }

        let old_suffix_entry_multi = if old_mid_end > 0 {
            self.in_multi_after[old_mid_end - 1]
        } else {
            false
        };

        self.lines.splice(prefix..old_mid_end, mid_lines);
        self.html_lines.splice(prefix..old_mid_end, mid_html);
        self.in_multi_after.splice(prefix..old_mid_end, mid_multi);

        if syn.is_some() && in_multi != old_suffix_entry_multi {
            let s = syn.as_ref().unwrap();
            let mut idx = new_mid_end;
            while idx < self.lines.len() {
                let prev_after = self.in_multi_after[idx];
                self.html_lines[idx] = highlight_single_line(&self.lines[idx], s, &mut in_multi);
                self.in_multi_after[idx] = in_multi;
                if in_multi == prev_after {
                    break;
                }
                idx += 1;
            }
        }
    }

    pub fn render_range(
        &self,
        start_line: usize,
        end_line: usize,
        font_family: &str,
        font_size: i32,
        dark: bool,
    ) -> (usize, String) {
        let total = self.html_lines.len();
        if total == 0 {
            let mut html = pre_open(font_family, font_size, dark);
            html.push_str("\n</pre>");
            return (0, html);
        }
        let s = start_line.min(total - 1);
        let e = end_line.max(s + 1).min(total);
        let est: usize = self.html_lines[s..e]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + 96;
        let mut html = String::with_capacity(est);
        html.push_str(&pre_open(font_family, font_size, dark));
        for (idx, line_html) in self.html_lines[s..e].iter().enumerate() {
            if idx > 0 {
                html.push('\n');
            }
            html.push_str(line_html);
        }
        html.push_str("\n</pre>");
        (s, html)
    }
}

/// Highlight `text` to an HTML fragment suitable for `Text.RichText`.
pub fn highlight_html(
    text: &str,
    syntax_file: Option<&str>,
    dark: bool,
    show_spaces: bool,
    font_family: &str,
    font_size: i32,
) -> String {
    let Some(name) = syntax_file else {
        return escape_pre(text, font_family, font_size, dark);
    };
    let Some(src) = syntax_map().get(name) else {
        return escape_pre(text, font_family, font_size, dark);
    };
    let syn = cached_syntax(name, src, dark, show_spaces);
    let lines: Vec<&str> = if text.is_empty() {
        vec![""]
    } else {
        text.split('\n').collect()
    };
    let mut html = pre_open(font_family, font_size, dark);
    let mut in_multi = false;
    for (li, line) in lines.iter().enumerate() {
        html.push_str(&highlight_single_line(line, &syn, &mut in_multi));
        if li + 1 < lines.len() {
            html.push('\n');
        }
    }
    html.push_str("\n</pre>");
    html
}

fn spans_to_html(line: &str, fmt: &[Option<SpanStyle>]) -> String {
    if line.is_empty() {
        return String::new();
    }
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let style = fmt.get(i).copied().flatten();
        let mut j = i + 1;
        while j < bytes.len() && fmt.get(j).copied().flatten() == style {
            j += 1;
        }
        // Walk back so we don't split a UTF-8 sequence.
        while j < bytes.len() && (bytes[j] & 0xC0) == 0x80 {
            j += 1;
        }
        while j > i && (bytes[i] & 0xC0) == 0x80 {
            i -= 1;
        }
        let chunk = &line[i.min(line.len())..j.min(line.len())];
        let esc = html_escape(chunk);
        if let Some(st) = style {
            out.push_str(&format!("<span style=\"{}\">{esc}</span>", st.css()));
        } else {
            out.push_str(&esc);
        }
        i = j;
    }
    out
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_pre(text: &str, font_family: &str, font_size: i32, dark: bool) -> String {
    format!(
        "{}{}\n</pre>",
        pre_open(font_family, font_size, dark),
        html_escape(text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpp_keywords() {
        let html = highlight_html(
            "int main() { return 0; }\n",
            Some("cpp.syntax"),
            false,
            false,
            "Ubuntu Mono",
            14,
        );
        assert!(html.contains("int"), "{html}");
        assert!(html.contains("return"), "{html}");
        assert!(html.contains("span"), "{html}");
        assert!(
            html.contains("#202060") || html.contains("#300050"),
            "{html}"
        );
    }

    #[test]
    fn xml_tags() {
        let html = highlight_html(
            "<item itemtype=\"Resistor\" />\n",
            Some("xml.syntax"),
            false,
            false,
            "Ubuntu Mono",
            14,
        );
        assert!(html.contains("item"), "{html}");
        assert!(html.contains("span"), "{html}");
    }

    #[test]
    fn syntax_from_path() {
        assert_eq!(syntax_for_path("foo.cpp"), Some("cpp.syntax"));
        assert_eq!(syntax_for_path("a.sim1"), Some("xml.syntax"));
        assert_eq!(syntax_for_path("Makefile"), Some("makef.syntax"));
        assert_eq!(syntax_for_path("x.unknown"), None);
    }

    #[test]
    fn cpp_keyword_list() {
        let k = keywords_for_syntax(Some("cpp.syntax"));
        assert!(k.iter().any(|w| w == "return"), "{k:?}");
        assert!(k.iter().any(|w| w == "class"), "{k:?}");
        assert!(!k.iter().any(|w| w == "if"), "{k:?}"); // length <= 2 skipped
    }

    #[test]
    fn multiline_comment() {
        let src = "int a;\n/* hi\nthere */\nint b;\n";
        let html = highlight_html(src, Some("cpp.syntax"), false, false, "Ubuntu Mono", 14);
        assert!(html.contains("hi"), "{html}");
        assert!(html.contains("there"), "{html}");
    }

    #[test]
    fn html_uses_editor_font() {
        let html = highlight_html("x", None, false, false, "Menlo", 18);
        assert!(
            html.contains("font-family:'Menlo'") && html.contains("font-size:18px"),
            "{html}"
        );
    }

    #[test]
    fn html_uses_themed_default_text_color() {
        let light = highlight_html("x", None, false, false, "Menlo", 14);
        assert!(light.contains("color:#1e1e1e"), "{light}");
        let dark = highlight_html("x", None, true, false, "Menlo", 14);
        assert!(dark.contains("color:#dcdcdc"), "{dark}");
    }

    #[test]
    fn dark_color_inverts_hsl_lightness_like_qt() {
        // cpp.syntax keyword1 #202060 is a dark blue (L=64). C++ inverts to L=191
        // at the same hue, which is a light periwinkle — not a scaled-up RGB.
        assert_eq!(dark_color(0x202060), 0x9F9FDF);
        // Already-light salmon (L=170) is left above the invert threshold.
        let light = dark_color(0xFA5A5A);
        let lr = ((light >> 16) & 0xFF) as i32;
        let lg = ((light >> 8) & 0xFF) as i32;
        let lb = (light & 0xFF) as i32;
        assert!(lr > 200 && lg < 160 && lb < 160, "{light:#06x}");
    }

    #[test]
    fn dark_cpp_keywords_use_inverted_colors() {
        let html = highlight_html(
            "class Foo { return 0; }\n",
            Some("cpp.syntax"),
            true,
            false,
            "Ubuntu Mono",
            14,
        );
        assert!(!html.contains("#202060"), "{html}");
        assert!(html.contains("#9f9fdf"), "{html}");
        assert!(html.contains("color:#dcdcdc"), "{html}");
    }

    #[test]
    fn pre_framing_preserves_leading_and_trailing_newlines() {
        let highlighted = highlight_html(
            "\nint a;\n",
            Some("cpp.syntax"),
            false,
            false,
            "Ubuntu Mono",
            14,
        );
        // QTextHtmlParser strips one '\n' right after <pre...> and one before </pre>,
        // so the document's leading/trailing '\n' requires two '\n' at each boundary.
        assert!(highlighted.contains("\">\n\n<span"), "{highlighted}");
        assert!(highlighted.ends_with("a;\n\n</pre>"), "{highlighted}");

        let plain = highlight_html("\nhello\n", None, false, false, "Ubuntu Mono", 14);
        assert!(plain.contains("\">\n\nhello\n\n</pre>"), "{plain}");
    }

    #[test]
    fn highlight_cache_matches_full_highlight_across_edits_and_windows() {
        let mut cache = HighlightCache::default();
        let initial = "int a = 1;\n/* comment\nline2\n*/\nvoid loop() {\n  return;\n}\n";
        cache.update(initial, Some("cpp.syntax"), false, false);
        let (start, full) = cache.render_range(0, cache.line_count(), "Ubuntu Mono", 14, false);
        assert_eq!(start, 0);
        assert_eq!(
            full,
            highlight_html(initial, Some("cpp.syntax"), false, false, "Ubuntu Mono", 14)
        );

        // Insert a newline inside loop()
        let edited = "int a = 1;\n/* comment\nline2\n*/\nvoid loop() {\n\n  return;\n}\n";
        cache.update(edited, Some("cpp.syntax"), false, false);
        let (_, full_edited) = cache.render_range(0, cache.line_count(), "Ubuntu Mono", 14, false);
        assert_eq!(
            full_edited,
            highlight_html(edited, Some("cpp.syntax"), false, false, "Ubuntu Mono", 14)
        );

        // Remove closing `*/` so multiline comment propagates to remaining lines
        let unclosed = "int a = 1;\n/* comment\nline2\n\nvoid loop() {\n\n  return;\n}\n";
        cache.update(unclosed, Some("cpp.syntax"), false, false);
        let (_, full_unclosed) =
            cache.render_range(0, cache.line_count(), "Ubuntu Mono", 14, false);
        assert_eq!(
            full_unclosed,
            highlight_html(
                unclosed,
                Some("cpp.syntax"),
                false,
                false,
                "Ubuntu Mono",
                14
            )
        );

        // Window slice starts at line 4
        let (ws, window) = cache.render_range(4, 6, "Ubuntu Mono", 14, false);
        assert_eq!(ws, 4);
        assert!(window.contains("void"), "{window}");
        assert!(!window.contains("int a"), "{window}");
    }
}
