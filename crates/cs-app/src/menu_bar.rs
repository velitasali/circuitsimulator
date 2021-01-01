//! Application menu tree. QML draws it in-window; macOS builds NSMenu from
//! the same JSON. No QMenuBar / QAction — qt-bridge cannot host Widgets.

use crate::macos_host;
use cs_engine::i18n;
use qtbridge::qobject;
use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone)]
struct Entry {
    id: String,
    text: String,
    shortcut: String,
    enabled: bool,
    visible: bool,
    checkable: bool,
    checked: bool,
    separator: bool,
    submenu: Option<MenuNode>,
}

#[derive(Clone)]
struct MenuNode {
    title: String,
    entries: Vec<Entry>,
}

pub struct AppMenuBar {
    menus: Vec<MenuNode>,
    json: Vec<Value>,
    running: bool,
    paused: bool,
    show_grid: bool,
    show_scroll: bool,
    animate_logic: bool,
    animate_curr: bool,
    can_undo: bool,
    can_redo: bool,
    has_selection: bool,
    can_paste: bool,
    has_project: bool,
}

fn t(s: &str) -> String {
    i18n::tr(s)
}

fn item(
    id: &str,
    text: &str,
    shortcut: &str,
    enabled: bool,
    checkable: bool,
    checked: bool,
) -> Entry {
    let custom_shortcut = cs_engine::settings::get().shortcuts.get(id).cloned();
    let sc = custom_shortcut.unwrap_or_else(|| shortcut.into());
    Entry {
        id: id.into(),
        text: text.into(),
        shortcut: sc,
        enabled,
        visible: true,
        checkable,
        checked,
        separator: false,
        submenu: None,
    }
}

fn sep() -> Entry {
    Entry {
        id: String::new(),
        text: String::new(),
        shortcut: String::new(),
        enabled: true,
        visible: true,
        checkable: false,
        checked: false,
        separator: true,
        submenu: None,
    }
}

fn sub(title: &str, entries: Vec<Entry>) -> Entry {
    Entry {
        id: String::new(),
        text: title.into(),
        shortcut: String::new(),
        enabled: true,
        visible: true,
        checkable: false,
        checked: false,
        separator: false,
        submenu: Some(MenuNode {
            title: title.into(),
            entries,
        }),
    }
}

fn empty_item(d: &Entry) -> Value {
    json!({
        "path": "",
        "text": d.text,
        "enabled": d.enabled,
        "visible": d.visible,
        "checkable": d.checkable,
        "checked": d.checked,
        "shortcut": d.shortcut,
        "separator": d.separator,
        "id": d.id,
    })
}

fn serialize_entries(entries: &[Entry], prefix: &str) -> Vec<Value> {
    let mut out = Vec::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        let path = if prefix.is_empty() {
            i.to_string()
        } else {
            format!("{prefix}/{i}")
        };
        if e.separator {
            let mut m = empty_item(e);
            m["path"] = json!(path);
            m["separator"] = json!(true);
            out.push(m);
            continue;
        }
        if let Some(sub) = &e.submenu {
            let mut m = empty_item(e);
            m["path"] = json!(path);
            m["text"] = json!(sub.title);
            m["submenu"] = json!(serialize_entries(&sub.entries, &path));
            out.push(m);
            continue;
        }
        let mut m = empty_item(e);
        m["path"] = json!(path);
        out.push(m);
    }
    out
}

fn serialize_menus(menus: &[MenuNode]) -> Vec<Value> {
    menus
        .iter()
        .enumerate()
        .map(|(i, m)| {
            json!({
                "title": m.title,
                "path": i.to_string(),
                "items": serialize_entries(&m.entries, &i.to_string()),
            })
        })
        .collect()
}

fn recent_entries(prefix: &str, clear_id: &str, clear_text: &str, paths: &[String]) -> Vec<Entry> {
    let mut entries = Vec::new();
    for (i, p) in paths.iter().enumerate() {
        let name = Path::new(p)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(p);
        entries.push(item(
            &format!("{prefix}.{i}"),
            &format!("{}. {name}", i + 1),
            "",
            true,
            false,
            false,
        ));
    }
    if entries.is_empty() {
        entries.push(item("", &t("(None)"), "", false, false, false));
    }
    entries.push(sep());
    entries.push(item(
        clear_id,
        clear_text,
        "",
        !paths.is_empty(),
        false,
        false,
    ));
    entries
}

fn build_menus(
    running: bool,
    paused: bool,
    show_grid: bool,
    show_scroll: bool,
    animate_logic: bool,
    animate_curr: bool,
    can_undo: bool,
    can_redo: bool,
    has_selection: bool,
    can_paste: bool,
    has_project: bool,
) -> Vec<MenuNode> {
    let st = cs_engine::settings::get();
    let power_text = if running {
        t("&Stop Simulation")
    } else {
        t("Start &Simulation")
    };
    let pause_text = if paused {
        t("&Resume Simulation")
    } else {
        t("&Pause Simulation")
    };

    vec![
        MenuNode {
            title: t("&File"),
            entries: vec![
                sub(
                    &t("&New"),
                    vec![
                        item("file.newFile", &t("&File"), "", true, false, false),
                        item("file.newCirc", &t("&Circuit"), "", true, false, false),
                        item("file.newWindow", &t("&Window"), "", true, false, false),
                    ],
                ),
                item("file.open", &t("&Open..."), "Ctrl+O", true, false, false),
                sep(),
                item(
                    "file.openFolder",
                    &t("Open &Project"),
                    "",
                    true,
                    false,
                    false,
                ),
                item(
                    "file.closeProject",
                    &t("Close &Project"),
                    "",
                    has_project,
                    false,
                    false,
                ),
                sub(
                    &t("Recent Projects"),
                    recent_entries(
                        "file.recentProj",
                        "file.clearRecentProjects",
                        &t("Clear Recent Projects"),
                        &st.recent_projects,
                    ),
                ),
                sep(),
                item("file.save", &t("&Save"), "Ctrl+S", true, false, false),
                item(
                    "file.saveAs",
                    &t("Save &As..."),
                    "Ctrl+Shift+S",
                    true,
                    false,
                    false,
                ),
                item(
                    "file.saveAll",
                    &t("Save A&ll"),
                    "Ctrl+Alt+S",
                    true,
                    false,
                    false,
                ),
                sep(),
                sub(
                    &t("Recent Circuits"),
                    recent_entries(
                        "file.recentCirc",
                        "file.clearRecentCircuits",
                        &t("Clear Recent Circuits"),
                        &st.recent_circuits,
                    ),
                ),
                item(
                    "file.exportSub",
                    &t("&Export as Subcircuit..."),
                    "",
                    true,
                    false,
                    false,
                ),
                sub(&t("&Insert Subcircuit"), vec![]),
                sep(),
                sub(
                    &t("Recent Files"),
                    recent_entries(
                        "file.recentFile",
                        "file.clearRecentFiles",
                        &t("Clear Recent Files"),
                        &st.recent_files,
                    ),
                ),
                item("file.closeFile", &t("Close File"), "", true, false, false),
                item(
                    "file.closeCirc",
                    &t("Close Circuit"),
                    "",
                    true,
                    false,
                    false,
                ),
                sep(),
                item(
                    "file.settings",
                    &t("Settings"),
                    "Ctrl+,",
                    true,
                    false,
                    false,
                ),
                sep(),
                item("file.quit", &t("&Quit"), "Ctrl+Q", true, false, false),
            ],
        },
        MenuNode {
            title: t("&Edit"),
            entries: vec![
                item("edit.undo", &t("&Undo"), "Ctrl+Z", can_undo, false, false),
                item("edit.redo", &t("&Redo"), "Ctrl+Y", can_redo, false, false),
                sep(),
                item(
                    "edit.cut",
                    &t("Cu&t"),
                    "Ctrl+X",
                    has_selection,
                    false,
                    false,
                ),
                item(
                    "edit.copy",
                    &t("&Copy"),
                    "Ctrl+C",
                    has_selection,
                    false,
                    false,
                ),
                item(
                    "edit.paste",
                    &t("&Paste"),
                    "Ctrl+V",
                    can_paste,
                    false,
                    false,
                ),
                item("edit.find", &t("Find"), "Ctrl+F", true, false, false),
                sep(),
                item(
                    "circ.addAllComponents",
                    &t("Add All Components"),
                    "",
                    true,
                    false,
                    false,
                ),
            ],
        },
        MenuNode {
            title: t("&View"),
            entries: vec![
                item("view.zoomIn", &t("Zoom In"), "Ctrl+=", true, false, false),
                item("view.zoomOut", &t("Zoom Out"), "Ctrl+-", true, false, false),
                item("view.zoomFit", &t("Zoom to Fit"), "", true, false, false),
                item(
                    "view.zoomSel",
                    &t("Zoom to Selection"),
                    "",
                    true,
                    false,
                    false,
                ),
                item("view.zoomOne", &t("Reset Zoom"), "", true, false, false),
                sep(),
                item("view.showGrid", &t("Show Grid"), "", true, true, show_grid),
                item(
                    "view.showScroll",
                    &t("Show Scrollbars"),
                    "",
                    true,
                    true,
                    show_scroll,
                ),
                sep(),
                item(
                    "view.cmdPalette",
                    &t("Command Center"),
                    "Ctrl+Shift+P",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.jumpRef",
                    &t("Jump to Reference"),
                    "Ctrl+P",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.tabSwitch",
                    &t("Switch Tab/Circuit"),
                    if cfg!(target_os = "macos") {
                        "Meta+Tab"
                    } else {
                        "Ctrl+Tab"
                    },
                    true,
                    false,
                    false,
                ),
                item(
                    "view.searchComp",
                    &t("Search Components"),
                    "Ctrl+Shift+F",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.focusFiles",
                    &t("Focus Files"),
                    "Ctrl+Shift+E",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.focusEditor",
                    &t("Focus Text Editor"),
                    "Ctrl+E",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.focusSimLog",
                    &t("Focus Simulator Messages"),
                    "Ctrl+Shift+M",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.focusCompLog",
                    &t("Focus Compiler Messages"),
                    "Ctrl+Shift+U",
                    true,
                    false,
                    false,
                ),
                sep(),
                item("view.sidePanel", &t("Side Panel"), "", true, false, false),
                item(
                    "view.editorPanel",
                    &t("Editor Panel"),
                    "",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.toggleActivePanel",
                    &t("Toggle Active Panel"),
                    "Ctrl+J",
                    true,
                    false,
                    false,
                ),
                sep(),
                item(
                    "view.libManager",
                    &t("Library Manager"),
                    "",
                    true,
                    false,
                    false,
                ),
            ],
        },
        MenuNode {
            title: t("&Simulation"),
            entries: vec![
                item("sim.power", &power_text, "", true, false, false),
                item("sim.pause", &pause_text, "", running, false, false),
                sep(),
                item("sim.compile", &t("Compile"), "F10", true, false, false),
                item("sim.load", &t("Upload"), "F11", true, false, false),
                sep(),
                item("sim.debug", &t("Debug"), "F12", true, false, false),
                item("sim.run", &t("Run"), "F5", true, false, false),
                item("sim.step", &t("Step"), "F6", true, false, false),
                item("sim.stepOver", &t("Step Over"), "F7", true, false, false),
                item(
                    "sim.debugPause",
                    &t("Pause Debugger"),
                    "",
                    true,
                    false,
                    false,
                ),
                item("sim.stop", &t("Stop Debugger"), "", true, false, false),
                item("sim.reset", &t("Reset Debugger"), "", true, false, false),
                sep(),
                item(
                    "sim.animateLogic",
                    &t("Animate Logic"),
                    "",
                    true,
                    true,
                    animate_logic,
                ),
                item(
                    "sim.animateCurr",
                    &t("Animate Current"),
                    "",
                    true,
                    true,
                    animate_curr,
                ),
            ],
        },
        MenuNode {
            title: t("&Help"),
            entries: vec![
                item("help.info", &t("Simulation Info"), "", true, false, false),
                item(
                    "help.about",
                    &t("&About Circuit Simulator"),
                    "",
                    true,
                    false,
                    false,
                ),
                item("help.aboutQt", &t("About Qt"), "", true, false, false),
            ],
        },
    ]
}

impl Default for AppMenuBar {
    fn default() -> Self {
        let st = cs_engine::settings::get();
        let has_project = project_open_from_settings(&st);
        let menus = build_menus(
            false,
            false,
            st.draw_grid,
            st.show_scroll,
            st.animate_logic,
            st.animate_curr,
            false,
            false,
            false,
            false,
            has_project,
        );
        let json = serialize_menus(&menus);
        Self {
            menus,
            json,
            running: false,
            paused: false,
            show_grid: st.draw_grid,
            show_scroll: st.show_scroll,
            animate_logic: st.animate_logic,
            animate_curr: st.animate_curr,
            can_undo: false,
            can_redo: false,
            has_selection: false,
            can_paste: false,
            has_project,
        }
    }
}

fn project_open_from_settings(st: &cs_engine::settings::AppSettings) -> bool {
    if crate::pending::no_project() {
        return false;
    }
    st.last_project_dir
        .as_ref()
        .filter(|p| Path::new(p).is_dir())
        .is_some()
        || st
            .recent_projects
            .first()
            .is_some_and(|p| Path::new(p).is_dir())
}

fn find_entry_mut<'a>(menus: &'a mut [MenuNode], path: &str) -> Option<&'a mut Entry> {
    let idxs: Vec<usize> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().ok())
        .collect::<Option<_>>()?;
    if idxs.len() < 2 {
        return None;
    }
    let mut entries = &mut menus.get_mut(idxs[0])?.entries;
    for (n, &i) in idxs[1..].iter().enumerate() {
        if n + 2 == idxs.len() {
            return entries.get_mut(i);
        }
        entries = &mut entries.get_mut(i)?.submenu.as_mut()?.entries;
    }
    None
}

fn map_recent_action(id: &str) -> Option<String> {
    let st = cs_engine::settings::get();
    let (prefix, list) = if let Some(rest) = id.strip_prefix("file.recentCirc.") {
        (rest, &st.recent_circuits)
    } else if let Some(rest) = id.strip_prefix("file.recentFile.") {
        (rest, &st.recent_files)
    } else if let Some(rest) = id.strip_prefix("file.recentProj.") {
        (rest, &st.recent_projects)
    } else {
        return None;
    };
    let idx: usize = prefix.parse().ok()?;
    let path = list.get(idx)?;
    let kind = if id.starts_with("file.recentCirc.") {
        "openCirc"
    } else if id.starts_with("file.recentFile.") {
        "openFile"
    } else {
        "openProj"
    };
    Some(format!("{kind}:{path}"))
}

fn find_by_id_mut<'a>(entries: &'a mut [Entry], id: &str) -> Option<&'a mut Entry> {
    for e in entries {
        if e.id == id {
            return Some(e);
        }
        if let Some(sub) = &mut e.submenu {
            if let Some(hit) = find_by_id_mut(&mut sub.entries, id) {
                return Some(hit);
            }
        }
    }
    None
}

impl AppMenuBar {
    fn republish(&mut self) {
        self.json = serialize_menus(&self.menus);
        macos_host::set_native_menu_json(&Value::Array(self.json.clone()).to_string());
        self.menus_changed();
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl AppMenuBar {
    qproperty!("menus", Read = menus, Notify = menus_changed);

    #[qsignal]
    fn menus_changed(&mut self);
    #[qsignal]
    fn action(&mut self, id: String);

    fn menus(&self) -> Vec<Value> {
        self.json.clone()
    }

    #[qslot]
    fn trigger(&mut self, path: String) {
        if let Some(e) = find_entry_mut(&mut self.menus, &path) {
            if e.separator || e.submenu.is_some() || e.id.is_empty() {
                return;
            }
            let id = e.id.clone();
            if let Some(mapped) = map_recent_action(&id) {
                self.action(mapped);
                return;
            }
            self.action(id);
        }
    }

    #[qslot]
    fn trigger_action(&mut self, id: String) {
        if let Some(mapped) = map_recent_action(&id) {
            self.action(mapped);
            return;
        }
        self.action(id);
    }

    #[qslot]
    fn set_custom_shortcut(&mut self, id: String, shortcut: String) {
        cs_engine::settings::edit(|s| {
            if shortcut.is_empty() {
                s.shortcuts.remove(&id);
            } else {
                s.shortcuts.insert(id, shortcut);
            }
        });
        self.rebuild();
    }

    #[qslot]
    fn rebuild(&mut self) {
        self.menus = build_menus(
            self.running,
            self.paused,
            self.show_grid,
            self.show_scroll,
            self.animate_logic,
            self.animate_curr,
            self.can_undo,
            self.can_redo,
            self.has_selection,
            self.can_paste,
            self.has_project,
        );
        self.republish();
    }

    #[qslot]
    fn about_to_show(&mut self, _path: String) {}

    #[qslot]
    fn set_sim_state(&mut self, running: bool, paused: bool) {
        self.running = running;
        self.paused = paused;
        macos_host::set_sim_state(running, paused);
        let mut changed = false;
        let power_text = if running {
            t("&Stop Simulation")
        } else {
            t("Start &Simulation")
        };
        let pause_text = if paused {
            t("&Resume Simulation")
        } else {
            t("&Pause Simulation")
        };
        for m in &mut self.menus {
            if let Some(e) = find_by_id_mut(&mut m.entries, "sim.power") {
                if e.text != power_text {
                    e.text = power_text.clone();
                    changed = true;
                }
            }
            if let Some(e) = find_by_id_mut(&mut m.entries, "sim.pause") {
                if e.text != pause_text || e.enabled != running {
                    e.text = pause_text.clone();
                    e.enabled = running;
                    changed = true;
                }
            }
        }
        if changed {
            self.republish();
        }
    }

    #[qslot]
    fn set_edit_state(
        &mut self,
        can_undo: bool,
        can_redo: bool,
        has_selection: bool,
        can_paste: bool,
    ) {
        self.can_undo = can_undo;
        self.can_redo = can_redo;
        self.has_selection = has_selection;
        self.can_paste = can_paste;
        let mut changed = false;
        for (id, enabled) in [
            ("edit.undo", can_undo),
            ("edit.redo", can_redo),
            ("edit.cut", has_selection),
            ("edit.copy", has_selection),
            ("edit.paste", can_paste),
        ] {
            for m in &mut self.menus {
                if let Some(e) = find_by_id_mut(&mut m.entries, id) {
                    if e.enabled != enabled {
                        e.enabled = enabled;
                        changed = true;
                    }
                    break;
                }
            }
        }
        if changed {
            self.republish();
        }
    }

    #[qslot]
    fn set_has_project(&mut self, has: bool) {
        self.has_project = has;
        for m in &mut self.menus {
            if let Some(e) = find_by_id_mut(&mut m.entries, "file.closeProject") {
                if e.enabled != has {
                    e.enabled = has;
                    self.republish();
                }
                return;
            }
        }
    }

    #[qslot]
    fn set_checked(&mut self, id: String, checked: bool) {
        if id == "view.showGrid" {
            self.show_grid = checked;
        } else if id == "view.showScroll" {
            self.show_scroll = checked;
        } else if id == "sim.animateLogic" {
            self.animate_logic = checked;
        } else if id == "sim.animateCurr" {
            self.animate_curr = checked;
        }
        for m in &mut self.menus {
            if let Some(e) = find_by_id_mut(&mut m.entries, &id) {
                if e.checked != checked {
                    e.checked = checked;
                    self.republish();
                }
                return;
            }
        }
    }

    #[qslot]
    fn install_native_menus(&mut self) {
        macos_host::install_native_menu(self);
        macos_host::set_native_menu_json(&Value::Array(self.json.clone()).to_string());
    }
}
