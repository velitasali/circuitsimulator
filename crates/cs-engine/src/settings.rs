//! App-wide defaults and per-circuit settings. Persistence is serde JSON via
//! `dirs` (not QSettings). CircuitDefaults in C++ live here too.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::units::{join_time, split_time};

/// C++ `CircuitDefaults` hardcoded fallbacks.
pub const DEFAULT_CANVAS_WIDTH: i32 = 3200;
pub const DEFAULT_CANVAS_HEIGHT: i32 = 2400;
pub const DEFAULT_STEP_SIZE: u64 = 1_000_000;
pub const DEFAULT_STEPS_PER_SEC: u64 = 1_000_000;
pub const DEFAULT_NL_STEPS: u32 = 100_000;
pub const DEFAULT_REACT_STEP: u64 = 1_000_000;
pub const DEFAULT_FPS: u64 = 20;
pub const DEFAULT_UNDO_STEPS: i32 = 100;
pub const DEFAULT_EDITOR_FONT: &str = "Ubuntu Mono";
pub const DEFAULT_EDITOR_FONT_SIZE: i32 = 14;
pub const DEFAULT_EDITOR_TAB_SIZE: i32 = 4;
pub const MAX_RECENT_CIRCUITS: usize = 20;
pub const MAX_RECENT_FILES: usize = 10;
pub const MAX_RECENT_PROJECTS: usize = 5;
pub const MAX_RECENT_COMPONENTS: usize = 10;

pub const TIME_UNITS: [&str; 5] = ["ps", "ns", "µs", "ms", "s"];

pub const LANGUAGES: &[(&str, &str)] = &[("en", "English"), ("tr", "Türkçe")];

pub const DEFAULT_WINDOW_WIDTH: i32 = 1200;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 800;
pub const MIN_WINDOW_WIDTH: i32 = 800;
pub const MIN_WINDOW_HEIGHT: i32 = 550;

fn default_window_width() -> i32 {
    DEFAULT_WINDOW_WIDTH
}

fn default_window_height() -> i32 {
    DEFAULT_WINDOW_HEIGHT
}

fn default_neg_one() -> i32 {
    -1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub language: String,
    pub theme: String,
    pub font_name: String,
    pub user_path: String,
    pub auto_update: bool,
    pub draw_grid: bool,
    pub show_scroll: bool,
    pub animate_logic: bool,
    pub animate_curr: bool,
    pub ansi_symbols: bool,
    pub canvas_width: i32,
    pub canvas_height: i32,
    pub fps: u64,
    pub undo_steps: i32,
    pub step_size: u64,
    pub steps_per_sec: u64,
    pub max_nl_steps: u32,
    pub react_step: u64,
    pub slope_steps: i32,
    pub editor_font_family: String,
    pub editor_font_size: i32,
    pub editor_tab_size: i32,
    pub editor_space_tabs: bool,
    pub editor_show_spaces: bool,
    pub editor_show_lsp_debug: bool,
    pub editor_close_paren: bool,
    pub editor_close_braces: bool,
    pub editor_close_brackets: bool,
    pub editor_close_quotes: bool,
    pub editor_close_squotes: bool,
    pub repaint_overlay: bool,
    pub show_component_rects: bool,
    pub recent_circuits: Vec<String>,
    pub recent_files: Vec<String>,
    pub recent_projects: Vec<String>,
    pub recent_components: Vec<String>,
    #[serde(default)]
    pub last_project_dir: Option<String>,
    #[serde(default = "default_window_width")]
    pub window_width: i32,
    #[serde(default = "default_window_height")]
    pub window_height: i32,
    #[serde(default = "default_neg_one")]
    pub window_x: i32,
    #[serde(default = "default_neg_one")]
    pub window_y: i32,
    #[serde(default)]
    pub window_maximized: bool,
    #[serde(default)]
    pub compiler_tool_paths: BTreeMap<String, String>,
    #[serde(default)]
    pub compiler_incl_paths: BTreeMap<String, String>,
    #[serde(default)]
    pub compiler_boards: BTreeMap<String, String>,
    #[serde(default)]
    pub compiler_custom_boards: BTreeMap<String, String>,
    #[serde(default)]
    pub installed_components: BTreeMap<String, i64>,
    #[serde(default)]
    pub project_sessions: BTreeMap<String, crate::project::ProjectSession>,
    #[serde(default)]
    pub shortcuts: BTreeMap<String, String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: String::new(),
            theme: "System".into(),
            // C++ MainWindow defaults to "Ubuntu" (registered from resources).
            font_name: "Ubuntu".into(),
            user_path: String::new(),
            auto_update: true,
            draw_grid: true,
            show_scroll: false,
            animate_logic: true,
            animate_curr: true,
            ansi_symbols: false,
            canvas_width: DEFAULT_CANVAS_WIDTH,
            canvas_height: DEFAULT_CANVAS_HEIGHT,
            fps: DEFAULT_FPS,
            undo_steps: DEFAULT_UNDO_STEPS,
            step_size: DEFAULT_STEP_SIZE,
            steps_per_sec: DEFAULT_STEPS_PER_SEC,
            max_nl_steps: DEFAULT_NL_STEPS,
            react_step: DEFAULT_REACT_STEP,
            slope_steps: 0,
            editor_font_family: DEFAULT_EDITOR_FONT.into(),
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
            editor_tab_size: DEFAULT_EDITOR_TAB_SIZE,
            editor_space_tabs: true,
            editor_show_spaces: false,
            editor_show_lsp_debug: false,
            editor_close_paren: false,
            editor_close_braces: false,
            editor_close_brackets: false,
            editor_close_quotes: false,
            editor_close_squotes: false,
            repaint_overlay: false,
            show_component_rects: false,
            recent_circuits: Vec::new(),
            recent_files: Vec::new(),
            recent_projects: Vec::new(),
            recent_components: Vec::new(),
            last_project_dir: None,
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            window_x: -1,
            window_y: -1,
            window_maximized: false,
            compiler_tool_paths: BTreeMap::new(),
            compiler_incl_paths: BTreeMap::new(),
            compiler_boards: BTreeMap::new(),
            compiler_custom_boards: BTreeMap::new(),
            installed_components: BTreeMap::new(),
            project_sessions: BTreeMap::new(),
            shortcuts: BTreeMap::new(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            let mut s = Self::default();
            if s.language.is_empty() {
                s.language = detect_locale();
            }
            return s;
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(mut s) => {
                if s.language.is_empty() {
                    s.language = detect_locale();
                }
                s.clamp();
                s
            }
            Err(_) => {
                let mut s = Self::default();
                s.language = detect_locale();
                s
            }
        }
    }

    pub fn save(&self) {
        self.save_to(&default_path());
    }

    pub fn save_to(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    fn clamp(&mut self) {
        self.canvas_width = self.canvas_width.clamp(1, 10_000);
        self.canvas_height = self.canvas_height.clamp(1, 10_000);
        self.undo_steps = self.undo_steps.clamp(5, 1000);
        self.editor_font_size = self.editor_font_size.clamp(6, 72);
        self.editor_tab_size = self.editor_tab_size.clamp(1, 16);
        if self.window_width < MIN_WINDOW_WIDTH {
            self.window_width = DEFAULT_WINDOW_WIDTH;
        }
        if self.window_height < MIN_WINDOW_HEIGHT {
            self.window_height = DEFAULT_WINDOW_HEIGHT;
        }
        if self.fps == 0 {
            self.fps = DEFAULT_FPS;
        }
        if self.step_size == 0 {
            self.step_size = DEFAULT_STEP_SIZE;
        }
        if self.steps_per_sec == 0 {
            self.steps_per_sec = DEFAULT_STEPS_PER_SEC;
        }
        if self.max_nl_steps == 0 {
            self.max_nl_steps = DEFAULT_NL_STEPS;
        }
        if self.react_step == 0 {
            self.react_step = DEFAULT_REACT_STEP;
        }
        if !LANGUAGES.iter().any(|(c, _)| *c == self.language) {
            self.language = "en".into();
        }
        if !["System", "Light", "Dark"].contains(&self.theme.as_str()) {
            self.theme = "System".into();
        }
        trim_recent(&mut self.recent_circuits, MAX_RECENT_CIRCUITS);
        trim_recent(&mut self.recent_files, MAX_RECENT_FILES);
        trim_recent(&mut self.recent_projects, MAX_RECENT_PROJECTS);
        trim_recent(&mut self.recent_components, MAX_RECENT_COMPONENTS);
    }

    pub fn update_window_geometry(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        maximized: bool,
    ) {
        if width >= MIN_WINDOW_WIDTH {
            self.window_width = width;
        }
        if height >= MIN_WINDOW_HEIGHT {
            self.window_height = height;
        }
        if !maximized {
            self.window_x = x;
            self.window_y = y;
        }
        self.window_maximized = maximized;
    }

    pub fn language_index(&self) -> i32 {
        LANGUAGES
            .iter()
            .position(|(c, _)| *c == self.language)
            .unwrap_or(0) as i32
    }

    pub fn set_language_index(&mut self, index: i32) {
        if let Some((code, _)) = LANGUAGES.get(index as usize) {
            self.language = (*code).into();
        }
    }

    pub fn language_names(&self) -> Vec<String> {
        LANGUAGES.iter().map(|(_, n)| (*n).to_string()).collect()
    }

    pub fn to_circ(&self) -> CircSettings {
        CircSettings {
            width: self.canvas_width,
            height: self.canvas_height,
            animate_logic: self.animate_logic,
            animate_curr: self.animate_curr,
            ansi: self.ansi_symbols,
            step_size: self.step_size,
            steps_ps: self.steps_per_sec,
            nl_steps: self.max_nl_steps,
            react_step_ps: self.react_step,
        }
    }

    pub fn push_recent_circuit(&mut self, path: String) {
        push_recent(&mut self.recent_circuits, path, MAX_RECENT_CIRCUITS);
    }

    pub fn push_recent_file(&mut self, path: String) {
        push_recent(&mut self.recent_files, path, MAX_RECENT_FILES);
    }

    pub fn push_recent_project(&mut self, path: String) {
        if !path.is_empty() {
            self.last_project_dir = Some(path.clone());
        }
        push_recent(&mut self.recent_projects, path, MAX_RECENT_PROJECTS);
    }

    pub fn push_recent_component(&mut self, spec: String) {
        push_recent(&mut self.recent_components, spec, MAX_RECENT_COMPONENTS);
    }

    pub fn clear_recent_components(&mut self) {
        self.recent_components.clear();
    }
}

fn push_recent(list: &mut Vec<String>, path: String, cap: usize) {
    if path.is_empty() {
        return;
    }
    list.retain(|p| p != &path);
    list.insert(0, path);
    trim_recent(list, cap);
}

fn trim_recent(list: &mut Vec<String>, cap: usize) {
    if list.len() > cap {
        list.truncate(cap);
    }
}

pub fn data_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("Circuit Simulator");
    dir
}

fn default_path() -> PathBuf {
    let mut dir = data_dir();
    dir.push("settings.json");
    dir
}

fn detect_locale() -> String {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let code = raw.split('.').next().unwrap_or("").replace('-', "_");
    if code.starts_with("tr") {
        return "tr".into();
    }
    "en".into()
}

static STATE: OnceLock<Mutex<AppSettings>> = OnceLock::new();

fn cell() -> &'static Mutex<AppSettings> {
    STATE.get_or_init(|| Mutex::new(AppSettings::load()))
}

pub fn get() -> AppSettings {
    cell().lock().expect("settings lock").clone()
}

pub fn with<R>(f: impl FnOnce(&AppSettings) -> R) -> R {
    f(&cell().lock().expect("settings lock"))
}

pub fn replace(s: AppSettings) {
    let mut s = s;
    s.clamp();
    s.save();
    *cell().lock().expect("settings lock") = s;
}

pub fn edit(f: impl FnOnce(&mut AppSettings)) {
    let mut g = cell().lock().expect("settings lock");
    f(&mut g);
    g.clamp();
    g.save();
}

/// Per-circuit values stored in the `.sim1` header. C++ `Circuit` + `Simulator`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CircSettings {
    pub width: i32,
    pub height: i32,
    pub animate_logic: bool,
    pub animate_curr: bool,
    pub ansi: bool,
    pub step_size: u64,
    pub steps_ps: u64,
    pub nl_steps: u32,
    pub react_step_ps: u64,
}

impl Default for CircSettings {
    fn default() -> Self {
        Self {
            width: DEFAULT_CANVAS_WIDTH,
            height: DEFAULT_CANVAS_HEIGHT,
            animate_logic: true,
            animate_curr: true,
            ansi: false,
            step_size: DEFAULT_STEP_SIZE,
            steps_ps: DEFAULT_STEPS_PER_SEC,
            nl_steps: DEFAULT_NL_STEPS,
            react_step_ps: DEFAULT_REACT_STEP,
        }
    }
}

impl CircSettings {
    pub fn analog_dt(&self) -> f64 {
        self.react_step_ps as f64 / 1e12
    }

    pub fn set_analog_dt(&mut self, dt: f64) {
        self.react_step_ps = (dt * 1e12).round().max(1.0) as u64;
    }

    pub fn ps_per_sec(&self) -> u64 {
        self.step_size.saturating_mul(self.steps_ps)
    }

    pub fn speed_percent(&self) -> f64 {
        100.0 * self.ps_per_sec() as f64 / 1e12
    }

    pub fn speed_label(&self) -> String {
        format!(" {:.2}%", self.speed_percent())
    }

    pub fn step_unit(&self) -> i32 {
        split_time(self.step_size).1
    }

    pub fn react_step_unit(&self) -> i32 {
        split_time(self.react_step_ps).1
    }

    pub fn react_step_value(&self) -> u64 {
        split_time(self.react_step_ps).0
    }

    /// C++ `CircuitDialog::updtSpeed`.
    pub fn set_ps_per_sec(&mut self, mut ps: u64) {
        if ps == 0 {
            ps = 1;
        }
        if ps > 1_000_000_000_000 {
            ps = 1_000_000_000_000;
        }
        if self.step_size == 0 {
            self.step_size = 1;
        }
        let mut steps = ps / self.step_size;
        let mut size = self.step_size;
        while steps >= 1_000_000_000 && size <= u64::MAX / 1000 {
            steps /= 1000;
            size *= 1000;
        }
        while steps == 0 && size >= 1000 {
            size /= 1000;
            steps = ps / size;
        }
        if steps == 0 {
            steps = 1;
        }
        self.step_size = size;
        self.steps_ps = steps;
    }

    pub fn set_speed_percent(&mut self, mut percent: f64) {
        if percent == 0.0 {
            percent = 1.0;
        }
        if percent > 100.0 {
            percent = 100.0;
        }
        let ps = (percent * 1e12 / 100.0).round() as u64;
        self.set_ps_per_sec(ps);
    }

    pub fn set_step(&mut self, step: u64) {
        self.steps_ps = step.max(1);
        let mut ps = self.ps_per_sec();
        if ps > 1_000_000_000_000 {
            ps = 1_000_000_000_000;
        }
        self.set_ps_per_sec(ps);
    }

    pub fn set_step_unit(&mut self, unit: i32) {
        self.step_size = join_time(1, unit).max(1);
        let mut ps = self.ps_per_sec();
        if ps > 1_000_000_000_000 {
            ps = 1_000_000_000_000;
        }
        self.set_ps_per_sec(ps);
    }

    pub fn set_react(&mut self, value: u64, unit: i32) {
        self.react_step_ps = join_time(value.max(1), unit).max(1);
    }

    pub fn header_attrs(&self) -> String {
        format!(
            "stepSize=\"{}\" stepsPS=\"{}\" NLsteps=\"{}\" reaStep=\"{}\" animate=\"{}\" anicurr=\"{}\" ansi=\"{}\" width=\"{}\" height=\"{}\"",
            self.step_size,
            self.steps_ps,
            self.nl_steps,
            self.react_step_ps,
            if self.animate_logic { 1 } else { 0 },
            if self.animate_curr { 1 } else { 0 },
            if self.ansi { 1 } else { 0 },
            self.width,
            self.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let dir = std::env::temp_dir().join(format!("cs-settings-{}.json", std::process::id()));
        let mut s = AppSettings::default();
        s.language = "tr".into();
        s.canvas_width = 2000;
        s.push_recent_circuit("/tmp/a.sim1".into());
        s.push_recent_circuit("/tmp/b.sim1".into());
        s.push_recent_circuit("/tmp/a.sim1".into());
        assert_eq!(s.recent_circuits, vec!["/tmp/a.sim1", "/tmp/b.sim1"]);
        s.save_to(&dir);
        let loaded = AppSettings::load_from(&dir);
        assert_eq!(loaded.language, "tr");
        assert_eq!(loaded.canvas_width, 2000);
        assert_eq!(loaded.recent_circuits[0], "/tmp/a.sim1");
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn speed_renorm() {
        let mut c = CircSettings::default();
        c.set_speed_percent(50.0);
        assert!((c.speed_percent() - 50.0).abs() < 0.01);
        c.set_step_unit(2); // µs
        assert_eq!(c.step_size, 1_000_000);
        c.set_react(10, 1); // 10 ns
        assert_eq!(c.react_step_ps, 10_000);
        assert!((c.analog_dt() - 1e-8).abs() < 1e-20);
    }
}
