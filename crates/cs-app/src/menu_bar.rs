//! Application menu tree. QML draws it in-window; macOS builds NSMenu from
//! the same JSON. No QMenuBar / QAction — qt-bridge cannot host Widgets.

use std::collections::BTreeMap;
use std::path::Path;

use cs_engine::settings::AppSettings;
use cs_engine::{catalog, i18n, project, settings};
use qtbridge::qobject;
use serde_json::{Map, Value, json};

use crate::actions::{self, MenuLeaf};
use crate::{macos_host, path_util, pending};

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
    /// QML `Shortcut` registrations consumed by `Main.qml`. On non-macOS
    /// platforms this covers both menu items and palette-only actions; on
    /// macOS `NSMenu` owns modifier/function menu shortcuts, so this contains
    /// only non-menu shortcuts and bare-key shortcuts (`G`).
    shortcuts: Vec<Value>,
    /// Effective Qt shortcut string for every canonical action ID (including
    /// canvas-owned and native-menu shortcuts) for context-menu labels.
    shortcut_map: Map<String, Value>,
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
    has_mcu: bool,
    circuit_path: String,
    project_dir: String,
}

const DEBUGGER_MENU_IDS: &[&str] = &[
    "sim.debug",
    "sim.run",
    "sim.step",
    "sim.stepOver",
    "sim.debugPause",
    "sim.stop",
    "sim.reset",
];

fn t(s: &str) -> String {
    i18n::tr(s)
}

fn item(
    id: &str,
    text: &str,
    enabled: bool,
    checkable: bool,
    checked: bool,
    custom: &BTreeMap<String, String>,
) -> Entry {
    let sc = if id.is_empty() {
        String::new()
    } else {
        actions::effective_shortcut(id, custom)
    };
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

fn collect_menu_leaves<'a>(entries: &'a [Entry], out: &mut Vec<MenuLeaf<'a>>) {
    for e in entries {
        if let Some(sub) = &e.submenu {
            collect_menu_leaves(&sub.entries, out);
            continue;
        }
        if e.separator || e.id.is_empty() {
            continue;
        }
        out.push(MenuLeaf {
            id: &e.id,
            shortcut: &e.shortcut,
            enabled: e.enabled,
        });
    }
}

fn qml_shortcuts_for(
    menus: &[MenuNode],
    native_menus: bool,
    custom: &BTreeMap<String, String>,
) -> Vec<Value> {
    let mut leaves = Vec::new();
    for m in menus {
        collect_menu_leaves(&m.entries, &mut leaves);
    }
    actions::collect_qml_shortcuts(&leaves, native_menus, custom)
}

fn recent_entries(
    prefix: &str,
    clear_id: &str,
    clear_text: &str,
    paths: &[String],
    custom: &BTreeMap<String, String>,
) -> Vec<Entry> {
    let mut entries = Vec::new();
    for (i, p) in paths.iter().enumerate() {
        let name = Path::new(p)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(p);
        entries.push(item(
            &format!("{prefix}.{i}"),
            &format!("{}. {name}", i + 1),
            true,
            false,
            false,
            custom,
        ));
    }
    if entries.is_empty() {
        entries.push(item("", &t("(None)"), false, false, false, custom));
    }
    entries.push(sep());
    entries.push(item(
        clear_id,
        clear_text,
        !paths.is_empty(),
        false,
        false,
        custom,
    ));
    entries
}

fn local_subcircuit_names(circuit_path: &str, project_dir: &str) -> Vec<String> {
    let cat = catalog::local_subcircuits(
        (!circuit_path.is_empty()).then_some(circuit_path),
        (!project_dir.is_empty()).then_some(project_dir),
    );
    cat.items()
        .iter()
        .filter(|i| i.typ == "Subcircuit")
        .map(|i| i.name.clone())
        .collect()
}

fn insert_subcircuit_entries(names: &[String], custom: &BTreeMap<String, String>) -> Vec<Entry> {
    if names.is_empty() {
        return vec![item("", &t("(None)"), false, false, false, custom)];
    }
    names
        .iter()
        .map(|name| {
            item(
                &format!("insertSubc:{name}"),
                name,
                true,
                false,
                false,
                custom,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_menus(
    st: &AppSettings,
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
    has_mcu: bool,
    local_subcircuits: &[String],
) -> Vec<MenuNode> {
    let c = &st.shortcuts;
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
                        item("file.newFile", &t("&File"), true, false, false, c),
                        item("file.newCirc", &t("&Circuit"), true, false, false, c),
                        item("file.newWindow", &t("&Window"), true, false, false, c),
                    ],
                ),
                item("file.open", &t("&Open..."), true, false, false, c),
                item(
                    "file.examples",
                    &t("Example &Circuits..."),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item(
                    "file.openFolder",
                    &t("Open &Project"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "file.closeProject",
                    &t("Close &Project"),
                    has_project,
                    false,
                    false,
                    c,
                ),
                sub(
                    &t("Recent Projects"),
                    recent_entries(
                        "file.recentProj",
                        "file.clearRecentProjects",
                        &t("Clear Recent Projects"),
                        &st.recent_projects,
                        c,
                    ),
                ),
                sep(),
                item("file.save", &t("&Save"), true, false, false, c),
                item("file.saveAs", &t("Save &As..."), true, false, false, c),
                item("file.saveAll", &t("Save A&ll"), true, false, false, c),
                sep(),
                sub(
                    &t("Recent Circuits"),
                    recent_entries(
                        "file.recentCirc",
                        "file.clearRecentCircuits",
                        &t("Clear Recent Circuits"),
                        &st.recent_circuits,
                        c,
                    ),
                ),
                item(
                    "file.exportSpice",
                    &t("Export &SPICE Netlist..."),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "file.exportKicad",
                    &t("Export &KiCad Netlist..."),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "file.exportSub",
                    &t("&Export as Subcircuit..."),
                    true,
                    false,
                    false,
                    c,
                ),
                sub(
                    &t("&Insert Subcircuit"),
                    insert_subcircuit_entries(local_subcircuits, c),
                ),
                sep(),
                sub(
                    &t("Recent Files"),
                    recent_entries(
                        "file.recentFile",
                        "file.clearRecentFiles",
                        &t("Clear Recent Files"),
                        &st.recent_files,
                        c,
                    ),
                ),
                item("file.close", &t("&Close"), true, false, false, c),
                item("file.closeFile", &t("Close File"), true, false, false, c),
                item("file.closeCirc", &t("Close Circuit"), true, false, false, c),
                sep(),
                item("file.settings", &t("Settings"), true, false, false, c),
                sep(),
                item("file.quit", &t("&Quit"), true, false, false, c),
            ],
        },
        MenuNode {
            title: t("&Edit"),
            entries: vec![
                item("edit.undo", &t("&Undo"), can_undo, false, false, c),
                item("edit.redo", &t("&Redo"), can_redo, false, false, c),
                sep(),
                item("edit.cut", &t("Cu&t"), has_selection, false, false, c),
                item("edit.copy", &t("&Copy"), has_selection, false, false, c),
                item("edit.paste", &t("&Paste"), can_paste, false, false, c),
                item("edit.find", &t("Find"), true, false, false, c),
                item("edit.format", &t("Format Document"), true, false, false, c),
                item(
                    "edit.triggerCompletion",
                    &t("Trigger Completion"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "edit.gotoDefinition",
                    &t("Go to Definition"),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item(
                    "edit.rotateCw",
                    &t("Rotate Clockwise"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "edit.rotateCcw",
                    &t("Rotate Counter-Clockwise"),
                    true,
                    false,
                    false,
                    c,
                ),
                item("edit.flipH", &t("Flip Horizontal"), true, false, false, c),
                item("edit.flipV", &t("Flip Vertical"), true, false, false, c),
                item(
                    "edit.properties",
                    &t("Component Properties..."),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item(
                    "circ.addAllComponents",
                    &t("Add All Components"),
                    true,
                    false,
                    false,
                    c,
                ),
            ],
        },
        MenuNode {
            title: t("&View"),
            entries: vec![
                item("view.zoomIn", &t("Zoom In"), true, false, false, c),
                item("view.zoomOut", &t("Zoom Out"), true, false, false, c),
                item("view.zoomFit", &t("Zoom to Fit"), true, false, false, c),
                item(
                    "view.zoomSel",
                    &t("Zoom to Selection"),
                    true,
                    false,
                    false,
                    c,
                ),
                item("view.zoomOne", &t("Reset Zoom"), true, false, false, c),
                sep(),
                item("view.showGrid", &t("Show Grid"), true, true, show_grid, c),
                item(
                    "view.showScroll",
                    &t("Show Scrollbars"),
                    true,
                    true,
                    show_scroll,
                    c,
                ),
                sep(),
                item(
                    "view.cmdPalette",
                    &t("Command Center"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "view.jumpRef",
                    &t("Jump to Reference"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "view.tabSwitch",
                    &t("Switch Tab/Circuit"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "view.searchComp",
                    &t("Search Components"),
                    true,
                    false,
                    false,
                    c,
                ),
                item("view.focusFiles", &t("Focus Files"), true, false, false, c),
                item(
                    "view.focusEditor",
                    &t("Focus Text Editor"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "view.focusSimLog",
                    &t("Focus Simulator Messages"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "view.focusCompLog",
                    &t("Focus Compiler Messages"),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item("view.sidePanel", &t("Side Panel"), true, false, false, c),
                item(
                    "view.editorPanel",
                    &t("Editor Panel"),
                    true,
                    false,
                    false,
                    c,
                ),
                item(
                    "view.toggleActivePanel",
                    &t("Toggle Active Panel"),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item(
                    "view.libManager",
                    &t("Library Manager"),
                    true,
                    false,
                    false,
                    c,
                ),
            ],
        },
        MenuNode {
            title: t("&Simulation"),
            entries: vec![
                item("sim.power", &power_text, true, false, false, c),
                item("sim.pause", &pause_text, running, false, false, c),
                item("sim.bode", &t("AC &Sweep..."), true, false, false, c),
                item(
                    "sim.dcSweep",
                    &t("&DC Parameter Sweep..."),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item("sim.compile", &t("Compile"), true, false, false, c),
                item("sim.load", &t("Upload"), true, false, false, c),
                sep(),
                item("sim.debug", &t("Debug"), has_mcu, false, false, c),
                item("sim.run", &t("Run"), has_mcu, false, false, c),
                item("sim.step", &t("Step"), has_mcu, false, false, c),
                item("sim.stepOver", &t("Step Over"), has_mcu, false, false, c),
                item(
                    "sim.debugPause",
                    &t("Pause Debugger"),
                    has_mcu,
                    false,
                    false,
                    c,
                ),
                item("sim.stop", &t("Stop Debugger"), has_mcu, false, false, c),
                item("sim.reset", &t("Reset Debugger"), has_mcu, false, false, c),
                sep(),
                item(
                    "sim.animateLogic",
                    &t("Animate Logic"),
                    true,
                    true,
                    animate_logic,
                    c,
                ),
                item(
                    "sim.animateCurr",
                    &t("Animate Current"),
                    true,
                    true,
                    animate_curr,
                    c,
                ),
            ],
        },
        MenuNode {
            title: t("&Help"),
            entries: vec![
                item("help.info", &t("Simulation Info"), true, false, false, c),
                item(
                    "help.checkUpdates",
                    &t("Check for &Updates..."),
                    true,
                    false,
                    false,
                    c,
                ),
                sep(),
                item(
                    "help.about",
                    &t("&About Circuit Simulator"),
                    true,
                    false,
                    false,
                    c,
                ),
            ],
        },
    ]
}

impl Default for AppMenuBar {
    fn default() -> Self {
        let st = settings::get();
        let has_project = project_open_from_settings(&st);
        let project_dir = if pending::no_project() {
            String::new()
        } else {
            st.last_project_dir
                .clone()
                .or_else(|| st.recent_projects.first().cloned())
                .map(|p| project::normalize_path(&p))
                .unwrap_or_default()
        };
        let local_subcs = local_subcircuit_names("", &project_dir);
        let menus = build_menus(
            &st,
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
            false,
            &local_subcs,
        );
        let json = serialize_menus(&menus);
        let shortcuts = qml_shortcuts_for(&menus, cfg!(target_os = "macos"), &st.shortcuts);
        let shortcut_map = actions::effective_shortcut_map(&st.shortcuts);
        Self {
            menus,
            json,
            shortcuts,
            shortcut_map,
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
            has_mcu: false,
            circuit_path: String::new(),
            project_dir,
        }
    }
}

fn project_open_from_settings(st: &AppSettings) -> bool {
    if pending::no_project() {
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
    let st = settings::get();
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
        let custom = settings::with(|s| s.shortcuts.clone());
        let next_shortcuts = qml_shortcuts_for(&self.menus, cfg!(target_os = "macos"), &custom);
        let next_map = actions::effective_shortcut_map(&custom);
        let shortcuts_changed = self.shortcuts != next_shortcuts;
        let map_changed = self.shortcut_map != next_map;

        self.json = serialize_menus(&self.menus);
        self.shortcuts = next_shortcuts;
        self.shortcut_map = next_map;
        macos_host::set_native_menu_json(&Value::Array(self.json.clone()).to_string());
        self.menus_changed();
        if shortcuts_changed {
            self.shortcuts_changed();
        }
        if map_changed {
            self.shortcut_map_changed();
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl AppMenuBar {
    qproperty!("menus", Read = menus, Notify = menus_changed);
    qproperty!("shortcuts", Read = shortcuts, Notify = shortcuts_changed);
    qproperty!(
        "shortcutMap",
        Read = shortcut_map,
        Notify = shortcut_map_changed
    );
    qproperty!("canUndo", Read = can_undo, Notify = edit_state_changed);
    qproperty!("canRedo", Read = can_redo, Notify = edit_state_changed);

    #[qsignal]
    fn menus_changed(&mut self);
    #[qsignal]
    fn shortcuts_changed(&mut self);
    #[qsignal]
    fn shortcut_map_changed(&mut self);
    #[qsignal]
    fn edit_state_changed(&mut self);
    #[qsignal]
    fn toggle_popup_requested(&mut self);
    #[qsignal]
    fn action(&mut self, id: String);

    fn menus(&self) -> Vec<Value> {
        self.json.clone()
    }

    fn shortcuts(&self) -> Vec<Value> {
        self.shortcuts.clone()
    }

    fn shortcut_map(&self) -> Value {
        Value::Object(self.shortcut_map.clone())
    }

    fn can_undo(&self) -> bool {
        self.can_undo
    }

    fn can_redo(&self) -> bool {
        self.can_redo
    }

    #[qslot]
    fn trigger(&mut self, path: String) {
        if let Some(e) = find_entry_mut(&mut self.menus, &path) {
            if !e.enabled || e.separator || e.submenu.is_some() || e.id.is_empty() {
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
    fn set_circuit_path(&mut self, path: String) {
        let clean = path_util::strip_file_url(&path);
        if self.circuit_path != clean {
            self.circuit_path = clean;
            self.rebuild();
        }
    }

    #[qslot]
    fn set_project_dir(&mut self, dir: String) {
        let clean = project::normalize_path(&path_util::strip_file_url(&dir));
        if self.project_dir != clean {
            self.project_dir = clean;
            self.rebuild();
        }
    }

    #[qslot]
    fn rebuild(&mut self) {
        let st = settings::get();
        let local_subcs = local_subcircuit_names(&self.circuit_path, &self.project_dir);
        self.menus = build_menus(
            &st,
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
            self.has_mcu,
            &local_subcs,
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
        let history_changed = self.can_undo != can_undo || self.can_redo != can_redo;
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
        if history_changed {
            self.edit_state_changed();
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
    fn set_has_mcu(&mut self, has: bool) {
        self.has_mcu = has;
        let mut changed = false;
        for id in DEBUGGER_MENU_IDS {
            for m in &mut self.menus {
                if let Some(e) = find_by_id_mut(&mut m.entries, id) {
                    if e.enabled != has {
                        e.enabled = has;
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
    fn set_alt_menu_open(&mut self, open: bool) {
        macos_host::set_alt_menu_open(open);
    }

    #[qslot]
    fn note_alt_toggled(&mut self, toggled: bool) {
        macos_host::note_alt_toggled(toggled);
    }

    #[qslot]
    fn request_toggle_popup(&mut self) {
        macos_host::note_alt_toggled(false);
        self.toggle_popup_requested();
    }

    #[qslot]
    fn install_native_menus(&mut self) {
        macos_host::install_native_menu(self);
        macos_host::set_native_menu_json(&Value::Array(self.json.clone()).to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(id: &str, shortcut: &str) -> Entry {
        Entry {
            id: id.into(),
            text: id.into(),
            shortcut: shortcut.into(),
            enabled: true,
            visible: true,
            checkable: false,
            checked: false,
            separator: false,
            submenu: None,
        }
    }

    fn shortcut_of(list: &[Value], id: &str) -> Option<String> {
        list.iter()
            .find(|e| e["id"] == id)
            .map(|e| e["shortcut"].as_str().unwrap_or_default().to_string())
    }

    fn default_menus() -> Vec<MenuNode> {
        let st = AppSettings::default();
        build_menus(
            &st,
            false,
            false,
            true,
            false,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            &[],
        )
    }

    #[test]
    fn collect_shortcuts_walks_submenus_and_skips_keyless_rows() {
        let node = MenuNode {
            title: "File".into(),
            entries: vec![
                leaf("file.save", "Ctrl+S"),
                leaf("file.closeProject", ""),
                sep(),
                Entry {
                    id: String::new(),
                    text: "Recent".into(),
                    shortcut: String::new(),
                    enabled: true,
                    visible: true,
                    checkable: false,
                    checked: false,
                    separator: false,
                    submenu: Some(MenuNode {
                        title: "Recent".into(),
                        entries: vec![leaf("file.recentFile.0", "Ctrl+1")],
                    }),
                },
            ],
        };
        let list = qml_shortcuts_for(&[node], false, &BTreeMap::new());
        assert_eq!(shortcut_of(&list, "file.save").as_deref(), Some("Ctrl+S"));
        assert_eq!(
            shortcut_of(&list, "file.recentFile.0").as_deref(),
            Some("Ctrl+1"),
            "submenu leaves must be registered too"
        );
        assert!(
            list.iter().all(|e| e["id"] != "file.closeProject"),
            "a keyless row must not be registered"
        );
    }

    #[test]
    fn collect_shortcuts_omits_chords_the_canvas_handles_itself() {
        let node = MenuNode {
            title: "Edit".into(),
            entries: vec![
                leaf("edit.undo", "Ctrl+Z"),
                leaf("edit.delete", "Delete"),
                leaf("edit.find", "Ctrl+F"),
            ],
        };
        let list = qml_shortcuts_for(&[node], false, &BTreeMap::new());
        assert_eq!(shortcut_of(&list, "edit.undo"), None);
        assert_eq!(shortcut_of(&list, "edit.delete"), None);
        assert_eq!(shortcut_of(&list, "edit.find").as_deref(), Some("Ctrl+F"));
    }

    #[test]
    fn default_menu_bar_publishes_expected_shortcuts_for_non_macos_and_macos() {
        let menus = default_menus();
        let empty = BTreeMap::new();

        // Non-macOS registers menu + non-menu shortcuts.
        let non_mac = qml_shortcuts_for(&menus, false, &empty);
        for (id, expected) in [
            ("file.save", "Ctrl+S"),
            ("file.saveAs", "Ctrl+Shift+S"),
            ("file.close", "Ctrl+W"),
            ("file.new", "Ctrl+N"),
            ("file.newWindow", "Ctrl+Shift+N"),
            ("file.settings", "Ctrl+,"),
            ("edit.rotateCw", "Ctrl+R"),
            ("edit.rotateCcw", "Ctrl+Shift+R"),
            ("edit.triggerCompletion", "Ctrl+Space"),
            ("edit.gotoDefinition", "Ctrl+F12"),
            ("edit.flipH", "Ctrl+L"),
            ("edit.flipV", "Ctrl+Shift+L"),
            ("edit.properties", "Alt+Return"),
            ("view.zoomIn", "Ctrl++"),
            ("view.zoomOut", "Ctrl+-"),
            ("view.zoomFit", "Ctrl+9"),
            ("view.zoomOne", "Ctrl+0"),
            ("view.showGrid", "G"),
            ("view.sidePanel", "Ctrl+B"),
            ("sim.power", "F8"),
            ("sim.pause", "F9"),
            ("debug.toggleRepaintOverlay", "Ctrl+Alt+D"),
            ("debug.toggleComponentRects", "Ctrl+Alt+C"),
        ] {
            assert_eq!(
                shortcut_of(&non_mac, id).as_deref(),
                Some(expected),
                "non-macOS QML list must include {id}"
            );
        }
        for hardcoded in ["view.cmdPalette", "view.jumpRef", "view.tabSwitch"] {
            assert_eq!(
                shortcut_of(&non_mac, hardcoded),
                None,
                "{hardcoded} is hardcoded in Main.qml; must not be registered dynamically"
            );
        }

        // macOS skips NSMenu-owned chords (Ctrl+S, F8, Ctrl+B, etc.) while keeping
        // non-menu shortcuts (Ctrl+N, Ctrl+Alt+D/C) and bare-key menu items (G).
        let mac = qml_shortcuts_for(&menus, true, &empty);
        assert_eq!(shortcut_of(&mac, "file.save"), None);
        assert_eq!(shortcut_of(&mac, "edit.properties"), None);
        assert_eq!(shortcut_of(&mac, "sim.power"), None);
        assert_eq!(shortcut_of(&mac, "view.sidePanel"), None);
        assert_eq!(shortcut_of(&mac, "view.showGrid").as_deref(), Some("G"));
        assert_eq!(shortcut_of(&mac, "file.new").as_deref(), Some("Ctrl+N"));
        assert_eq!(
            shortcut_of(&mac, "debug.toggleRepaintOverlay").as_deref(),
            Some("Ctrl+Alt+D")
        );
        assert_eq!(
            shortcut_of(&mac, "debug.toggleComponentRects").as_deref(),
            Some("Ctrl+Alt+C")
        );
    }

    #[test]
    fn menu_tree_uses_canonical_shortcuts_from_registry() {
        let menus = default_menus();
        let json = serialize_menus(&menus);
        let find_menu_shortcut = |id: &str| -> Option<String> {
            for m in &json {
                if let Some(items) = m["items"].as_array() {
                    for it in items {
                        if it["id"] == id {
                            return it["shortcut"].as_str().map(String::from);
                        }
                    }
                }
            }
            None
        };
        assert_eq!(
            find_menu_shortcut("file.settings").as_deref(),
            Some("Ctrl+,")
        );
        assert_eq!(
            find_menu_shortcut("edit.properties").as_deref(),
            Some("Alt+Return")
        );
        assert_eq!(find_menu_shortcut("sim.power").as_deref(), Some("F8"));
        assert_eq!(find_menu_shortcut("sim.pause").as_deref(), Some("F9"));
        assert_eq!(find_menu_shortcut("view.showGrid").as_deref(), Some("G"));
        assert_eq!(
            find_menu_shortcut("view.sidePanel").as_deref(),
            Some("Ctrl+B")
        );
    }

    #[test]
    fn debugger_menu_items_disabled_without_mcu_and_enabled_with_mcu() {
        let st = AppSettings::default();
        let without_mcu = build_menus(
            &st,
            false,
            false,
            true,
            false,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            &[],
        );
        let with_mcu = build_menus(
            &st,
            false,
            false,
            true,
            false,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            true,
            &[],
        );
        let find_enabled = |menus: &[MenuNode], id: &str| -> Option<bool> {
            let json = serialize_menus(menus);
            for m in &json {
                if let Some(items) = m["items"].as_array() {
                    for it in items {
                        if it["id"] == id {
                            return it["enabled"].as_bool();
                        }
                    }
                }
            }
            None
        };
        for id in DEBUGGER_MENU_IDS {
            assert_eq!(
                find_enabled(&without_mcu, id),
                Some(false),
                "{id} must be disabled when no MCU is present"
            );
            assert_eq!(
                find_enabled(&with_mcu, id),
                Some(true),
                "{id} must be enabled when an MCU is present"
            );
        }
        // Shortcuts follow the enabled state as well.
        let empty = BTreeMap::new();
        let sc_without = qml_shortcuts_for(&without_mcu, false, &empty);
        let sc_with = qml_shortcuts_for(&with_mcu, false, &empty);
        for id in ["sim.debug", "sim.run", "sim.step", "sim.stepOver"] {
            assert_eq!(
                sc_without
                    .iter()
                    .find(|e| e["id"] == id)
                    .and_then(|e| e["enabled"].as_bool()),
                Some(false),
                "shortcut for {id} must be disabled without MCU"
            );
            assert_eq!(
                sc_with
                    .iter()
                    .find(|e| e["id"] == id)
                    .and_then(|e| e["enabled"].as_bool()),
                Some(true),
                "shortcut for {id} must be enabled with MCU"
            );
        }
    }
}
