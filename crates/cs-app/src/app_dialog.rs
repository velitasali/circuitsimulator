//! App Settings façade. Layout is QML (`AppDialog.qml`); this is the value
//! plumbing, matching C++ `AppDialog`.

use crate::path_util::strip_file_url;
use cs_engine::settings::{self, AppSettings, CircSettings, LANGUAGES, TIME_UNITS};
use qtbridge::qobject;

pub struct AppDialog {
    s: AppSettings,
}

impl Default for AppDialog {
    fn default() -> Self {
        Self { s: settings::get() }
    }
}

impl AppDialog {
    fn persist(&mut self) {
        let dialog_s = self.s.clone();
        settings::edit(|s| {
            s.language = dialog_s.language;
            s.theme = dialog_s.theme;
            s.font_name = dialog_s.font_name;
            s.user_path = dialog_s.user_path;
            s.auto_update = dialog_s.auto_update;
            s.draw_grid = dialog_s.draw_grid;
            s.show_scroll = dialog_s.show_scroll;
            s.animate_logic = dialog_s.animate_logic;
            s.animate_curr = dialog_s.animate_curr;
            s.ansi_symbols = dialog_s.ansi_symbols;
            s.canvas_width = dialog_s.canvas_width;
            s.canvas_height = dialog_s.canvas_height;
            s.fps = dialog_s.fps;
            s.undo_steps = dialog_s.undo_steps;
            s.step_size = dialog_s.step_size;
            s.steps_per_sec = dialog_s.steps_per_sec;
            s.max_nl_steps = dialog_s.max_nl_steps;
            s.react_step = dialog_s.react_step;
            s.slope_steps = dialog_s.slope_steps;
            s.editor_font_family = dialog_s.editor_font_family;
            s.editor_font_size = dialog_s.editor_font_size;
            s.editor_tab_size = dialog_s.editor_tab_size;
            s.editor_space_tabs = dialog_s.editor_space_tabs;
            s.editor_show_spaces = dialog_s.editor_show_spaces;
            s.editor_show_lsp_debug = dialog_s.editor_show_lsp_debug;
            s.editor_close_paren = dialog_s.editor_close_paren;
            s.editor_close_braces = dialog_s.editor_close_braces;
            s.editor_close_brackets = dialog_s.editor_close_brackets;
            s.editor_close_quotes = dialog_s.editor_close_quotes;
            s.editor_close_squotes = dialog_s.editor_close_squotes;
            s.repaint_overlay = dialog_s.repaint_overlay;
            s.show_component_rects = dialog_s.show_component_rects;
            s.recent_circuits = dialog_s.recent_circuits;
            s.recent_files = dialog_s.recent_files;
            s.recent_projects = dialog_s.recent_projects;
            s.last_project_dir = dialog_s.last_project_dir;
        });
        self.s = settings::get();
    }

    fn circ(&self) -> CircSettings {
        self.s.to_circ()
    }

    fn set_circ(&mut self, c: CircSettings) {
        self.s.step_size = c.step_size;
        self.s.steps_per_sec = c.steps_ps;
        self.s.react_step = c.react_step_ps;
        self.s.max_nl_steps = c.nl_steps;
        self.persist();
        self.sim_changed();
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl AppDialog {
    qproperty!("languages", Read = languages, Constant);
    qproperty!(
        "language",
        Read = language,
        Write = set_language,
        Notify = app_changed
    );
    qproperty!("themes", Read = themes, Constant);
    qproperty!(
        "theme",
        Read = theme,
        Write = set_theme,
        Notify = app_changed
    );
    qproperty!(
        "fontName",
        Read = font_name,
        Write = set_font_name,
        Notify = app_changed
    );
    qproperty!(
        "userPath",
        Read = user_path,
        Write = set_user_path,
        Notify = app_changed
    );
    qproperty!(
        "userPathPlaceholder",
        Read = user_path_placeholder,
        Constant
    );
    qproperty!(
        "autoUpdate",
        Read = auto_update,
        Write = set_auto_update,
        Notify = app_changed
    );

    qproperty!(
        "drawGrid",
        Read = draw_grid,
        Write = set_draw_grid,
        Notify = circuit_changed
    );
    qproperty!(
        "showScroll",
        Read = show_scroll,
        Write = set_show_scroll,
        Notify = circuit_changed
    );
    qproperty!(
        "animateLogic",
        Read = animate_logic,
        Write = set_animate_logic,
        Notify = circuit_changed
    );
    qproperty!(
        "animateCurr",
        Read = animate_curr,
        Write = set_animate_curr,
        Notify = circuit_changed
    );
    qproperty!(
        "ansiSymbols",
        Read = ansi_symbols,
        Write = set_ansi_symbols,
        Notify = circuit_changed
    );
    qproperty!(
        "canvasWidth",
        Read = canvas_width,
        Write = set_canvas_width,
        Notify = circuit_changed
    );
    qproperty!(
        "canvasHeight",
        Read = canvas_height,
        Write = set_canvas_height,
        Notify = circuit_changed
    );
    qproperty!("fps", Read = fps, Write = set_fps, Notify = circuit_changed);
    qproperty!(
        "undoSteps",
        Read = undo_steps,
        Write = set_undo_steps,
        Notify = circuit_changed
    );

    qproperty!("timeUnits", Read = time_units, Constant);
    qproperty!(
        "speedPercent",
        Read = speed_percent,
        Write = set_speed_percent,
        Notify = sim_changed
    );
    qproperty!("speedLabel", Read = speed_label, Notify = sim_changed);
    qproperty!(
        "simStep",
        Read = sim_step,
        Write = set_sim_step,
        Notify = sim_changed
    );
    qproperty!(
        "simStepUnit",
        Read = sim_step_unit,
        Write = set_sim_step_unit,
        Notify = sim_changed
    );
    qproperty!(
        "nlSteps",
        Read = nl_steps,
        Write = set_nl_steps,
        Notify = sim_changed
    );
    qproperty!(
        "reactStep",
        Read = react_step,
        Write = set_react_step,
        Notify = sim_changed
    );
    qproperty!(
        "reactStepUnit",
        Read = react_step_unit,
        Write = set_react_step_unit,
        Notify = sim_changed
    );
    qproperty!(
        "slopeSteps",
        Read = slope_steps,
        Write = set_slope_steps,
        Notify = sim_changed
    );

    qproperty!("hasEditor", Read = has_editor, Constant);
    qproperty!(
        "editorFontFamily",
        Read = editor_font_family,
        Write = set_editor_font_family,
        Notify = editor_changed
    );
    qproperty!(
        "editorFontSize",
        Read = editor_font_size,
        Write = set_editor_font_size,
        Notify = editor_changed
    );
    qproperty!(
        "editorTabSize",
        Read = editor_tab_size,
        Write = set_editor_tab_size,
        Notify = editor_changed
    );
    qproperty!(
        "editorSpaceTabs",
        Read = editor_space_tabs,
        Write = set_editor_space_tabs,
        Notify = editor_changed
    );
    qproperty!(
        "editorShowSpaces",
        Read = editor_show_spaces,
        Write = set_editor_show_spaces,
        Notify = editor_changed
    );
    qproperty!(
        "editorShowLspDebug",
        Read = editor_show_lsp_debug,
        Write = set_editor_show_lsp_debug,
        Notify = editor_changed
    );
    qproperty!(
        "editorCloseParenthesis",
        Read = editor_close_paren,
        Write = set_editor_close_paren,
        Notify = editor_changed
    );
    qproperty!(
        "editorCloseBraces",
        Read = editor_close_braces,
        Write = set_editor_close_braces,
        Notify = editor_changed
    );
    qproperty!(
        "editorCloseBrackets",
        Read = editor_close_brackets,
        Write = set_editor_close_brackets,
        Notify = editor_changed
    );
    qproperty!(
        "editorCloseQuotes",
        Read = editor_close_quotes,
        Write = set_editor_close_quotes,
        Notify = editor_changed
    );
    qproperty!(
        "editorCloseSquotes",
        Read = editor_close_squotes,
        Write = set_editor_close_squotes,
        Notify = editor_changed
    );
    qproperty!(
        "repaintOverlayEnabled",
        Read = repaint_overlay,
        Write = set_repaint_overlay,
        Notify = debug_changed
    );
    qproperty!(
        "showComponentRects",
        Read = show_component_rects,
        Write = set_show_component_rects,
        Notify = debug_changed
    );

    #[qsignal]
    fn app_changed(&mut self);
    #[qsignal]
    fn circuit_changed(&mut self);
    #[qsignal]
    fn sim_changed(&mut self);
    #[qsignal]
    fn editor_changed(&mut self);
    #[qsignal]
    fn debug_changed(&mut self);
    #[qsignal]
    fn request_browse_user_path(&mut self);
    #[qsignal]
    fn request_reset(&mut self);

    fn languages(&self) -> Vec<String> {
        LANGUAGES.iter().map(|(_, n)| (*n).to_string()).collect()
    }
    fn language(&self) -> i32 {
        self.s.language_index()
    }
    fn set_language(&mut self, index: i32) {
        if index == self.language() {
            return;
        }
        self.s.set_language_index(index);
        cs_engine::i18n::set_locale(&self.s.language);
        self.persist();
        self.app_changed();
    }
    fn themes(&self) -> Vec<String> {
        vec!["System".into(), "Light".into(), "Dark".into()]
    }
    fn theme(&self) -> String {
        self.s.theme.clone()
    }
    fn set_theme(&mut self, theme: String) {
        if self.s.theme == theme {
            return;
        }
        self.s.theme = theme;
        self.persist();
        crate::macos_host::apply_theme(&self.s.theme);
        self.app_changed();
    }
    fn font_name(&self) -> String {
        self.s.font_name.clone()
    }
    fn set_font_name(&mut self, name: String) {
        if self.s.font_name == name {
            return;
        }
        self.s.font_name = name;
        self.persist();
        self.app_changed();
    }
    fn user_path(&self) -> String {
        self.s.user_path.clone()
    }
    fn set_user_path(&mut self, path: String) {
        self.s.user_path = strip_file_url(&path);
        self.persist();
        self.app_changed();
    }
    fn user_path_placeholder(&self) -> String {
        dirs_fallback()
    }
    fn auto_update(&self) -> bool {
        self.s.auto_update
    }
    fn set_auto_update(&mut self, v: bool) {
        self.s.auto_update = v;
        self.persist();
        self.app_changed();
    }

    fn draw_grid(&self) -> bool {
        self.s.draw_grid
    }
    fn set_draw_grid(&mut self, v: bool) {
        self.s.draw_grid = v;
        self.persist();
        self.circuit_changed();
    }
    fn show_scroll(&self) -> bool {
        self.s.show_scroll
    }
    fn set_show_scroll(&mut self, v: bool) {
        self.s.show_scroll = v;
        self.persist();
        self.circuit_changed();
    }
    fn animate_logic(&self) -> bool {
        self.s.animate_logic
    }
    fn set_animate_logic(&mut self, v: bool) {
        self.s.animate_logic = v;
        self.persist();
        self.circuit_changed();
    }
    fn animate_curr(&self) -> bool {
        self.s.animate_curr
    }
    fn set_animate_curr(&mut self, v: bool) {
        self.s.animate_curr = v;
        self.persist();
        self.circuit_changed();
    }
    fn ansi_symbols(&self) -> bool {
        self.s.ansi_symbols
    }
    fn set_ansi_symbols(&mut self, v: bool) {
        self.s.ansi_symbols = v;
        self.persist();
        self.circuit_changed();
    }
    fn canvas_width(&self) -> i32 {
        self.s.canvas_width
    }
    fn set_canvas_width(&mut self, w: i32) {
        self.s.canvas_width = w.clamp(1, 10_000);
        self.persist();
        self.circuit_changed();
    }
    fn canvas_height(&self) -> i32 {
        self.s.canvas_height
    }
    fn set_canvas_height(&mut self, h: i32) {
        self.s.canvas_height = h.clamp(1, 10_000);
        self.persist();
        self.circuit_changed();
    }
    fn fps(&self) -> i32 {
        self.s.fps as i32
    }
    fn set_fps(&mut self, fps: i32) {
        self.s.fps = fps.clamp(1, 100) as u64;
        self.persist();
        self.circuit_changed();
    }
    fn undo_steps(&self) -> i32 {
        self.s.undo_steps
    }
    fn set_undo_steps(&mut self, steps: i32) {
        self.s.undo_steps = steps.clamp(5, 1000);
        self.persist();
        self.circuit_changed();
    }

    fn time_units(&self) -> Vec<String> {
        TIME_UNITS.iter().map(|s| (*s).to_string()).collect()
    }
    fn speed_percent(&self) -> f64 {
        self.circ().speed_percent()
    }
    fn set_speed_percent(&mut self, p: f64) {
        let mut c = self.circ();
        c.set_speed_percent(p);
        self.set_circ(c);
    }
    fn speed_label(&self) -> String {
        self.circ().speed_label()
    }
    fn sim_step(&self) -> i32 {
        self.s.steps_per_sec as i32
    }
    fn set_sim_step(&mut self, step: i32) {
        let mut c = self.circ();
        c.set_step(step.max(1) as u64);
        self.set_circ(c);
    }
    fn sim_step_unit(&self) -> i32 {
        self.circ().step_unit()
    }
    fn set_sim_step_unit(&mut self, unit: i32) {
        let mut c = self.circ();
        c.set_step_unit(unit);
        self.set_circ(c);
    }
    fn nl_steps(&self) -> i32 {
        self.s.max_nl_steps as i32
    }
    fn set_nl_steps(&mut self, n: i32) {
        self.s.max_nl_steps = n.max(0) as u32;
        self.persist();
        self.sim_changed();
    }
    fn react_step(&self) -> i32 {
        self.circ().react_step_value() as i32
    }
    fn set_react_step(&mut self, n: i32) {
        let mut c = self.circ();
        let unit = c.react_step_unit();
        c.set_react(n.max(1) as u64, unit);
        self.set_circ(c);
    }
    fn react_step_unit(&self) -> i32 {
        self.circ().react_step_unit()
    }
    fn set_react_step_unit(&mut self, unit: i32) {
        let mut c = self.circ();
        let v = c.react_step_value();
        c.set_react(v, unit);
        self.set_circ(c);
    }
    fn slope_steps(&self) -> i32 {
        self.s.slope_steps
    }
    fn set_slope_steps(&mut self, n: i32) {
        self.s.slope_steps = n.clamp(0, 100);
        self.persist();
        self.sim_changed();
    }

    fn has_editor(&self) -> bool {
        true
    }
    fn editor_font_family(&self) -> String {
        self.s.editor_font_family.clone()
    }
    fn set_editor_font_family(&mut self, v: String) {
        self.s.editor_font_family = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_font_size(&self) -> i32 {
        self.s.editor_font_size
    }
    fn set_editor_font_size(&mut self, v: i32) {
        self.s.editor_font_size = v.clamp(6, 72);
        self.persist();
        self.editor_changed();
    }
    fn editor_tab_size(&self) -> i32 {
        self.s.editor_tab_size
    }
    fn set_editor_tab_size(&mut self, v: i32) {
        self.s.editor_tab_size = v.clamp(1, 16);
        self.persist();
        self.editor_changed();
    }
    fn editor_space_tabs(&self) -> bool {
        self.s.editor_space_tabs
    }
    fn set_editor_space_tabs(&mut self, v: bool) {
        self.s.editor_space_tabs = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_show_spaces(&self) -> bool {
        self.s.editor_show_spaces
    }
    fn set_editor_show_spaces(&mut self, v: bool) {
        self.s.editor_show_spaces = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_show_lsp_debug(&self) -> bool {
        self.s.editor_show_lsp_debug
    }
    fn set_editor_show_lsp_debug(&mut self, v: bool) {
        self.s.editor_show_lsp_debug = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_close_paren(&self) -> bool {
        self.s.editor_close_paren
    }
    fn set_editor_close_paren(&mut self, v: bool) {
        self.s.editor_close_paren = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_close_braces(&self) -> bool {
        self.s.editor_close_braces
    }
    fn set_editor_close_braces(&mut self, v: bool) {
        self.s.editor_close_braces = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_close_brackets(&self) -> bool {
        self.s.editor_close_brackets
    }
    fn set_editor_close_brackets(&mut self, v: bool) {
        self.s.editor_close_brackets = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_close_quotes(&self) -> bool {
        self.s.editor_close_quotes
    }
    fn set_editor_close_quotes(&mut self, v: bool) {
        self.s.editor_close_quotes = v;
        self.persist();
        self.editor_changed();
    }
    fn editor_close_squotes(&self) -> bool {
        self.s.editor_close_squotes
    }
    fn set_editor_close_squotes(&mut self, v: bool) {
        self.s.editor_close_squotes = v;
        self.persist();
        self.editor_changed();
    }
    fn repaint_overlay(&self) -> bool {
        self.s.repaint_overlay
    }
    fn set_repaint_overlay(&mut self, v: bool) {
        self.s.repaint_overlay = v;
        self.persist();
        self.debug_changed();
    }
    fn show_component_rects(&self) -> bool {
        self.s.show_component_rects
    }
    fn set_show_component_rects(&mut self, v: bool) {
        self.s.show_component_rects = v;
        self.persist();
        self.debug_changed();
    }

    #[qslot]
    fn browse_user_path(&mut self) {
        self.request_browse_user_path();
    }

    #[qslot]
    fn reset_settings(&mut self) {
        self.request_reset();
    }

    #[qslot]
    fn confirm_reset(&mut self) {
        self.s = AppSettings::default();
        self.s.language = "en".into();
        cs_engine::i18n::set_locale("en");
        self.persist();
        self.app_changed();
        self.circuit_changed();
        self.sim_changed();
        self.editor_changed();
        self.debug_changed();
    }

    #[qslot]
    fn add_recent_circuit(&mut self, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        settings::edit(|s| s.push_recent_circuit(path));
        self.s = settings::get();
    }

    #[qslot]
    fn add_recent_file(&mut self, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        settings::edit(|s| s.push_recent_file(path));
        self.s = settings::get();
    }

    #[qslot]
    fn add_recent_project(&mut self, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        settings::edit(|s| s.push_recent_project(path));
        self.s = settings::get();
    }

    #[qslot]
    fn clear_recent_circuits(&mut self) {
        settings::edit(|s| s.recent_circuits.clear());
        self.s = settings::get();
        self.app_changed();
    }

    #[qslot]
    fn clear_recent_files(&mut self) {
        settings::edit(|s| s.recent_files.clear());
        self.s = settings::get();
        self.app_changed();
    }

    #[qslot]
    fn clear_recent_projects(&mut self) {
        settings::edit(|s| {
            s.recent_projects.clear();
            s.last_project_dir = None;
        });
        self.s = settings::get();
        self.app_changed();
    }
}

fn dirs_fallback() -> String {
    let mut p = dirs::data_dir().unwrap_or_else(|| ".".into());
    p.push("Circuit Simulator");
    p.to_string_lossy().into_owned()
}
