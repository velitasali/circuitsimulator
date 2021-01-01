use crate::macos_host;
use crate::path_util::strip_file_url;
use qtbridge::qobject;

pub struct App {
    window_title: String,
    native_menus: bool,
    dark_titlebar: bool,
    dark_theme: bool,
    version: String,
    compiled: String,
    qt_version: String,
    about_visible: bool,
    about_qt_visible: bool,
    osc_visible: bool,
    la_visible: bool,
    mcu_visible: bool,
    serial_mon_visible: bool,
    terminal_visible: bool,
    settings_visible: bool,
    circuit_settings_visible: bool,
    installer_visible: bool,
    side_panel_tab: i32,
    search_placeholder: String,
    font_family: String,
    font_mono_family: String,
    font_size: i32,
    i18n_tick: i32,
    window_width: i32,
    window_height: i32,
    window_x: i32,
    window_y: i32,
    window_maximized: bool,
    min_window_width: i32,
    min_window_height: i32,
}

impl Default for App {
    fn default() -> Self {
        let s = cs_engine::settings::get();
        cs_engine::i18n::set_locale(&s.language);
        let dark = macos_host::apply_theme(&s.theme);
        Self {
            window_title: "Circuit Simulator".to_string(),
            native_menus: cfg!(target_os = "macos"),
            dark_titlebar: dark,
            dark_theme: dark,
            version: format!("{} at Rev {}", env!("CARGO_PKG_VERSION"), env!("CS_REVNO")),
            compiled: format!("{} (dd-MM-yy)", env!("CS_BUILDDATE")),
            qt_version: crate::native_canvas::qt_version(),
            about_visible: std::env::args().any(|a| a == "--about"),
            about_qt_visible: std::env::args().any(|a| a == "--about-qt"),
            osc_visible: false,
            la_visible: false,
            mcu_visible: false,
            serial_mon_visible: false,
            terminal_visible: false,
            settings_visible: false,
            circuit_settings_visible: false,
            installer_visible: false,
            side_panel_tab: 0,
            search_placeholder: cs_engine::i18n::tr("Search components"),
            font_family: s.font_name.clone(),
            font_mono_family: s.editor_font_family.clone(),
            font_size: 13,
            i18n_tick: 0,
            window_width: s.window_width.max(cs_engine::settings::MIN_WINDOW_WIDTH),
            window_height: s.window_height.max(cs_engine::settings::MIN_WINDOW_HEIGHT),
            window_x: s.window_x,
            window_y: s.window_y,
            window_maximized: s.window_maximized,
            min_window_width: cs_engine::settings::MIN_WINDOW_WIDTH,
            min_window_height: cs_engine::settings::MIN_WINDOW_HEIGHT,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl App {
    qproperty!(
        "windowTitle",
        Read = window_title,
        Write = set_window_title,
        Notify = window_title_changed
    );
    qproperty!("nativeMenus", Read = native_menus, Constant);
    qproperty!(
        "darkTitlebar",
        Read = dark_titlebar,
        Write = set_dark_titlebar,
        Notify = dark_titlebar_changed
    );
    qproperty!("darkTheme", Read = dark_theme, Notify = theme_changed);
    qproperty!("version", Read = version, Constant);
    qproperty!("compiled", Read = compiled, Constant);
    qproperty!("qtVersion", Read = qt_version, Constant);
    qproperty!(
        "aboutVisible",
        Read = about_visible,
        Write = set_about_visible,
        Notify = about_visible_changed
    );
    qproperty!(
        "aboutQtVisible",
        Read = about_qt_visible,
        Write = set_about_qt_visible,
        Notify = about_qt_visible_changed
    );
    qproperty!(
        "oscVisible",
        Read = osc_visible,
        Write = set_osc_visible,
        Notify = osc_visible_changed
    );
    qproperty!(
        "laVisible",
        Read = la_visible,
        Write = set_la_visible,
        Notify = la_visible_changed
    );
    qproperty!(
        "mcuVisible",
        Read = mcu_visible,
        Write = set_mcu_visible,
        Notify = mcu_visible_changed
    );
    qproperty!(
        "serialMonVisible",
        Read = serial_mon_visible,
        Write = set_serial_mon_visible,
        Notify = serial_mon_visible_changed
    );
    qproperty!(
        "terminalVisible",
        Read = terminal_visible,
        Write = set_terminal_visible,
        Notify = terminal_visible_changed
    );
    qproperty!(
        "settingsVisible",
        Read = settings_visible,
        Write = set_settings_visible,
        Notify = settings_visible_changed
    );
    qproperty!(
        "circuitSettingsVisible",
        Read = circuit_settings_visible,
        Write = set_circuit_settings_visible,
        Notify = circuit_settings_visible_changed
    );
    qproperty!(
        "installerVisible",
        Read = installer_visible,
        Write = set_installer_visible,
        Notify = installer_visible_changed
    );
    qproperty!("i18nTick", Read = i18n_tick, Notify = i18n_tick_changed);
    qproperty!(
        "sidePanelTab",
        Read = side_panel_tab,
        Write = set_side_panel_tab,
        Notify = side_panel_tab_changed
    );
    qproperty!(
        "searchPlaceholder",
        Read = search_placeholder,
        Notify = search_placeholder_changed
    );
    qproperty!("fontFamily", Read = font_family, Notify = font_changed);
    qproperty!(
        "fontMonoFamily",
        Read = font_mono_family,
        Notify = font_changed
    );
    qproperty!("fontSize", Read = font_size, Notify = font_changed);
    qproperty!(
        "windowWidth",
        Read = window_width,
        Notify = window_geometry_changed
    );
    qproperty!(
        "windowHeight",
        Read = window_height,
        Notify = window_geometry_changed
    );
    qproperty!("windowX", Read = window_x, Notify = window_geometry_changed);
    qproperty!("windowY", Read = window_y, Notify = window_geometry_changed);
    qproperty!(
        "windowMaximized",
        Read = window_maximized,
        Notify = window_geometry_changed
    );
    qproperty!("minWindowWidth", Read = min_window_width, Constant);
    qproperty!("minWindowHeight", Read = min_window_height, Constant);

    #[qsignal]
    fn window_title_changed(&mut self);
    #[qsignal]
    fn dark_titlebar_changed(&mut self);
    #[qsignal]
    fn theme_changed(&mut self);
    #[qsignal]
    fn window_geometry_changed(&mut self);

    #[qsignal]
    fn about_visible_changed(&mut self);
    #[qsignal]
    fn about_qt_visible_changed(&mut self);
    #[qsignal]
    fn osc_visible_changed(&mut self);
    #[qsignal]
    fn la_visible_changed(&mut self);
    #[qsignal]
    fn mcu_visible_changed(&mut self);
    #[qsignal]
    fn serial_mon_visible_changed(&mut self);
    #[qsignal]
    fn terminal_visible_changed(&mut self);
    #[qsignal]
    fn settings_visible_changed(&mut self);
    #[qsignal]
    fn circuit_settings_visible_changed(&mut self);
    #[qsignal]
    fn installer_visible_changed(&mut self);
    #[qsignal]
    fn request_show_about(&mut self);
    #[qsignal]
    fn request_show_about_qt(&mut self);
    #[qsignal]
    fn request_show_osc(&mut self);
    #[qsignal]
    fn request_show_la(&mut self);
    #[qsignal]
    fn request_show_mcu(&mut self);
    #[qsignal]
    fn request_show_serial_mon(&mut self);
    #[qsignal]
    fn request_show_terminal(&mut self);
    #[qsignal]
    fn request_show_settings(&mut self);
    #[qsignal]
    fn request_show_circuit_settings(&mut self);
    #[qsignal]
    fn request_show_installer(&mut self);
    #[qsignal]
    fn i18n_tick_changed(&mut self);
    #[qsignal]
    fn side_panel_tab_changed(&mut self);
    #[qsignal]
    fn search_placeholder_changed(&mut self);
    #[qsignal]
    fn font_changed(&mut self);
    #[qsignal]
    fn focus_search_requested(&mut self);
    #[qsignal]
    fn flush_session(&mut self);

    fn window_title(&self) -> String {
        self.window_title.clone()
    }

    fn native_menus(&self) -> bool {
        self.native_menus
    }

    fn dark_titlebar(&self) -> bool {
        self.dark_titlebar
    }

    fn dark_theme(&self) -> bool {
        self.dark_theme
    }

    fn version(&self) -> String {
        self.version.clone()
    }

    fn compiled(&self) -> String {
        self.compiled.clone()
    }

    fn qt_version(&self) -> String {
        self.qt_version.clone()
    }

    fn about_visible(&self) -> bool {
        self.about_visible
    }

    fn about_qt_visible(&self) -> bool {
        self.about_qt_visible
    }

    fn osc_visible(&self) -> bool {
        self.osc_visible
    }

    fn la_visible(&self) -> bool {
        self.la_visible
    }

    fn mcu_visible(&self) -> bool {
        self.mcu_visible
    }

    fn serial_mon_visible(&self) -> bool {
        self.serial_mon_visible
    }

    fn terminal_visible(&self) -> bool {
        self.terminal_visible
    }

    fn settings_visible(&self) -> bool {
        self.settings_visible
    }

    fn circuit_settings_visible(&self) -> bool {
        self.circuit_settings_visible
    }

    fn installer_visible(&self) -> bool {
        self.installer_visible
    }

    fn side_panel_tab(&self) -> i32 {
        self.side_panel_tab
    }

    fn search_placeholder(&self) -> String {
        self.search_placeholder.clone()
    }

    fn font_family(&self) -> String {
        self.font_family.clone()
    }

    fn font_mono_family(&self) -> String {
        self.font_mono_family.clone()
    }

    fn font_size(&self) -> i32 {
        self.font_size
    }

    fn window_width(&self) -> i32 {
        self.window_width
    }

    fn window_height(&self) -> i32 {
        self.window_height
    }

    fn window_x(&self) -> i32 {
        self.window_x
    }

    fn window_y(&self) -> i32 {
        self.window_y
    }

    fn window_maximized(&self) -> bool {
        self.window_maximized
    }

    fn min_window_width(&self) -> i32 {
        self.min_window_width
    }

    fn min_window_height(&self) -> i32 {
        self.min_window_height
    }

    fn set_window_title(&mut self, title: String) {
        if self.window_title == title {
            return;
        }
        self.window_title = title;
        self.window_title_changed();
    }

    fn set_dark_titlebar(&mut self, dark: bool) {
        if self.dark_titlebar == dark {
            return;
        }
        self.dark_titlebar = dark;
        macos_host::set_titlebar_dark(dark);
        self.dark_titlebar_changed();
    }

    fn set_about_visible(&mut self, visible: bool) {
        if visible {
            if !self.about_visible {
                self.about_visible = true;
                self.about_visible_changed();
            }
            self.request_show_about();
        } else if self.about_visible {
            self.about_visible = false;
            self.about_visible_changed();
        }
    }

    fn set_about_qt_visible(&mut self, visible: bool) {
        if visible {
            if !self.about_qt_visible {
                self.about_qt_visible = true;
                self.about_qt_visible_changed();
            }
            self.request_show_about_qt();
        } else if self.about_qt_visible {
            self.about_qt_visible = false;
            self.about_qt_visible_changed();
        }
    }

    fn set_osc_visible(&mut self, visible: bool) {
        if visible {
            if !self.osc_visible {
                self.osc_visible = true;
                self.osc_visible_changed();
            }
            self.request_show_osc();
        } else if self.osc_visible {
            self.osc_visible = false;
            self.osc_visible_changed();
        }
    }

    fn set_la_visible(&mut self, visible: bool) {
        if visible {
            if !self.la_visible {
                self.la_visible = true;
                self.la_visible_changed();
            }
            self.request_show_la();
        } else if self.la_visible {
            self.la_visible = false;
            self.la_visible_changed();
        }
    }

    fn set_mcu_visible(&mut self, visible: bool) {
        if visible {
            if !self.mcu_visible {
                self.mcu_visible = true;
                self.mcu_visible_changed();
            }
            self.request_show_mcu();
        } else if self.mcu_visible {
            self.mcu_visible = false;
            self.mcu_visible_changed();
        }
    }

    fn set_serial_mon_visible(&mut self, visible: bool) {
        if visible {
            if !self.serial_mon_visible {
                self.serial_mon_visible = true;
                self.serial_mon_visible_changed();
            }
            self.request_show_serial_mon();
        } else if self.serial_mon_visible {
            self.serial_mon_visible = false;
            self.serial_mon_visible_changed();
        }
    }

    fn set_terminal_visible(&mut self, visible: bool) {
        if visible {
            if !self.terminal_visible {
                self.terminal_visible = true;
                self.terminal_visible_changed();
            }
            self.request_show_terminal();
        } else if self.terminal_visible {
            self.terminal_visible = false;
            self.terminal_visible_changed();
        }
    }

    fn set_settings_visible(&mut self, visible: bool) {
        if visible {
            if !self.settings_visible {
                self.settings_visible = true;
                self.settings_visible_changed();
            }
            self.request_show_settings();
        } else if self.settings_visible {
            self.settings_visible = false;
            self.settings_visible_changed();
        }
    }

    fn set_circuit_settings_visible(&mut self, visible: bool) {
        if visible {
            if !self.circuit_settings_visible {
                self.circuit_settings_visible = true;
                self.circuit_settings_visible_changed();
            }
            self.request_show_circuit_settings();
        } else if self.circuit_settings_visible {
            self.circuit_settings_visible = false;
            self.circuit_settings_visible_changed();
        }
    }

    fn set_installer_visible(&mut self, visible: bool) {
        if visible {
            if !self.installer_visible {
                self.installer_visible = true;
                self.installer_visible_changed();
            }
            self.request_show_installer();
        } else if self.installer_visible {
            self.installer_visible = false;
            self.installer_visible_changed();
        }
    }

    fn set_side_panel_tab(&mut self, index: i32) {
        if self.side_panel_tab == index {
            return;
        }
        self.side_panel_tab = index.clamp(0, 3);
        self.side_panel_tab_changed();
        self.search_placeholder = match self.side_panel_tab {
            0 => cs_engine::i18n::tr("Search components"),
            1 => cs_engine::i18n::tr("Search files"),
            _ => cs_engine::i18n::tr("Search"),
        };
        self.search_placeholder_changed();
    }

    #[qslot]
    fn show_about(&mut self) {
        self.set_about_visible(true);
    }

    #[qslot]
    fn show_about_qt(&mut self) {
        self.set_about_qt_visible(true);
    }

    #[qslot]
    fn show_osc(&mut self) {
        self.set_osc_visible(true);
    }

    #[qslot]
    fn show_la(&mut self) {
        self.set_la_visible(true);
    }

    #[qslot]
    fn show_mcu(&mut self) {
        self.set_mcu_visible(true);
    }

    #[qslot]
    fn show_serial_mon(&mut self) {
        self.set_serial_mon_visible(true);
    }

    #[qslot]
    fn show_terminal(&mut self) {
        self.set_terminal_visible(true);
    }

    #[qslot]
    fn show_settings(&mut self) {
        self.set_settings_visible(true);
    }

    #[qslot]
    fn show_circuit_settings(&mut self) {
        self.set_circuit_settings_visible(true);
    }

    #[qslot]
    fn show_installer(&mut self) {
        self.set_installer_visible(true);
    }

    #[qslot]
    fn new_window(&self) {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let _ = std::process::Command::new(exe).arg("-noproject").spawn();
    }

    fn i18n_tick(&self) -> i32 {
        self.i18n_tick
    }

    #[qslot]
    fn translate(&self, source: String) -> String {
        cs_engine::i18n::tr(&source)
    }

    #[qslot]
    fn tr(&self, source: String) -> String {
        cs_engine::i18n::tr(&source)
    }

    #[qslot]
    fn reload_theme(&mut self) {
        let s = cs_engine::settings::get();
        let is_dark = macos_host::apply_theme(&s.theme);
        if self.dark_theme != is_dark {
            self.dark_theme = is_dark;
            self.theme_changed();
        }
        if self.dark_titlebar != is_dark {
            self.dark_titlebar = is_dark;
            self.dark_titlebar_changed();
        }
    }

    #[qslot]
    fn reload_i18n(&mut self) {
        let s = cs_engine::settings::get();
        cs_engine::i18n::set_locale(&s.language);
        self.reload_theme();
        let mut font_changed = false;
        if self.font_family != s.font_name {
            self.font_family = s.font_name;
            font_changed = true;
        }
        if self.font_mono_family != s.editor_font_family {
            self.font_mono_family = s.editor_font_family;
            font_changed = true;
        }
        if font_changed {
            self.font_changed();
        }
        self.search_placeholder = match self.side_panel_tab {
            0 => cs_engine::i18n::tr("Search components"),
            1 => cs_engine::i18n::tr("Search files"),
            _ => cs_engine::i18n::tr("Search"),
        };
        self.search_placeholder_changed();
        self.i18n_tick = self.i18n_tick.wrapping_add(1);
        self.i18n_tick_changed();
    }

    #[qslot]
    fn set_search_filter(&mut self, filter: String) {
        // Forwarded from QML; ComponentList.search is the real filter.
        let _ = filter;
    }

    #[qslot]
    fn focus_search_component(&mut self) {
        self.set_side_panel_tab(0);
        self.focus_search_requested();
    }

    #[qslot]
    fn suggest_folder_url(&self, preferred_path: String) -> String {
        let p = strip_file_url(&preferred_path);
        if !p.is_empty() {
            let path = std::path::Path::new(&p);
            if path.is_dir() {
                return path_to_folder_url(path);
            }
            if let Some(parent) = path.parent() {
                if parent.is_dir() {
                    return path_to_folder_url(parent);
                }
            }
        }

        let s = cs_engine::settings::get();
        if let Some(root) = s.last_project_dir.as_ref() {
            if !root.is_empty() && std::path::Path::new(root).is_dir() {
                return path_to_folder_url(std::path::Path::new(root));
            }
        }
        if let Some(first) = s.recent_projects.first() {
            if !first.is_empty() && std::path::Path::new(first).is_dir() {
                return path_to_folder_url(std::path::Path::new(first));
            }
        }

        if let Some(home) = dirs::home_dir() {
            return path_to_folder_url(&home);
        }

        String::new()
    }

    #[qslot]
    fn suggest_file_url(&self, path: String) -> String {
        let p = strip_file_url(&path);
        if p.is_empty() {
            return String::new();
        }
        path_to_folder_url(std::path::Path::new(&p))
    }

    #[qslot]
    fn request_close(&mut self) -> bool {
        self.flush_session();
        cs_engine::backup::end_session();
        true
    }

    #[qslot]
    fn install_native_chrome(&mut self) {
        macos_host::apply_app_icon();
        macos_host::set_titlebar_dark(self.dark_titlebar);
        macos_host::setup_window();
    }

    #[qslot]
    fn save_window_state(&mut self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        if width < cs_engine::settings::MIN_WINDOW_WIDTH
            || height < cs_engine::settings::MIN_WINDOW_HEIGHT
        {
            return;
        }
        self.window_maximized = maximized;
        if !maximized {
            self.window_width = width;
            self.window_height = height;
            self.window_x = x;
            self.window_y = y;
        }
        cs_engine::settings::edit(|s| {
            s.update_window_geometry(x, y, width, height, maximized);
        });
    }

    #[qslot]
    fn parse_si(&self, val: String, unit: String) -> f64 {
        let trimmed = val.trim();
        if trimmed.is_empty() || !trimmed.chars().any(|c| c.is_ascii_digit()) {
            return f64::NAN;
        }
        cs_engine::units::parse_si(trimmed, &unit)
    }

    #[qslot]
    fn format_si_value(&self, val: f64, unit: String) -> String {
        let base_unit = cs_engine::units::base_unit_for(&unit);
        let s = cs_engine::units::format_si_precision(val, base_unit, 3);
        if !base_unit.is_empty() {
            s.strip_suffix(base_unit)
                .map(|r| r.trim().to_string())
                .unwrap_or(s)
        } else {
            s
        }
    }
}

fn path_to_folder_url(path: &std::path::Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    if p.starts_with('/') {
        format!("file://{p}")
    } else {
        format!("file:///{p}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_file_url() {
        assert_eq!(strip_file_url("/tmp/test.wav"), "/tmp/test.wav");
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(strip_file_url("file:///tmp/test.wav"), "/tmp/test.wav");
            assert_eq!(strip_file_url("   file:///tmp/foo   "), "/tmp/foo");
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(strip_file_url("file:///C:/tmp/test.wav"), "C:/tmp/test.wav");
            assert_eq!(strip_file_url("   file:///C:/tmp/foo   "), "C:/tmp/foo");
        }
    }

    #[test]
    fn test_path_to_folder_url() {
        let p = std::path::Path::new("/tmp/myproject");
        assert_eq!(path_to_folder_url(p), "file:///tmp/myproject");
    }

    #[test]
    fn test_suggest_file_url() {
        let app = App::default();
        assert_eq!(
            app.suggest_file_url("/tmp/board.sim2".into()),
            "file:///tmp/board.sim2"
        );
        assert_eq!(
            app.suggest_file_url("file:///tmp/main.c".into()),
            "file:///tmp/main.c"
        );
        assert_eq!(app.suggest_file_url("".into()), "");
    }

    #[test]
    fn test_suggest_folder_url_with_existing_path() {
        let app = App::default();
        let tmp = std::env::temp_dir();
        let url = app.suggest_folder_url(tmp.to_string_lossy().into_owned());
        assert!(url.starts_with("file://"));
    }

    #[test]
    fn test_parse_si_and_format_si_value() {
        let app = App::default();
        // Parsing tests
        assert!((app.parse_si("1k".into(), "V".into()) - 1000.0).abs() < 1e-12);
        assert!((app.parse_si("1 k".into(), "V".into()) - 1000.0).abs() < 1e-12);
        assert!((app.parse_si("1kV".into(), "V".into()) - 1000.0).abs() < 1e-12);
        assert!((app.parse_si("100m".into(), "s".into()) - 0.1).abs() < 1e-12);
        assert!((app.parse_si("100ms".into(), "s".into()) - 0.1).abs() < 1e-12);
        assert!((app.parse_si("10u".into(), "s".into()) - 1e-5).abs() < 1e-12);
        assert!((app.parse_si("10µs".into(), "s".into()) - 1e-5).abs() < 1e-12);
        assert!((app.parse_si("2.5".into(), "V".into()) - 2.5).abs() < 1e-12);
        assert!((app.parse_si("-500m".into(), "V".into()) - -0.5).abs() < 1e-12);
        assert!(app.parse_si("abc".into(), "V".into()).is_nan());
        assert!(app.parse_si("".into(), "V".into()).is_nan());

        // Formatting tests
        assert_eq!(app.format_si_value(1000.0, "V".into()), "1 k");
        assert_eq!(app.format_si_value(0.1, "s".into()), "100 m");
        assert_eq!(app.format_si_value(1e-5, "s".into()), "10 µ");
        assert_eq!(app.format_si_value(2.5, "V".into()), "2.5");
        assert_eq!(app.format_si_value(0.0, "V".into()), "0");
        assert_eq!(app.format_si_value(-0.5, "V".into()), "-500 m");
    }

    #[test]
    fn test_about_qt_initial_state() {
        let app = App::default();
        assert!(!app.about_qt_visible);
    }
}
