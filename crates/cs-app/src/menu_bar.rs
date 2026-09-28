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
    /// QML `Shortcut` data for every menu row that carries one. The in-window
    /// menu bar only paints the keys as text, so non-macOS platforms register
    /// them from here; macOS leaves this empty because the NSMenu built from
    /// the same tree already owns the key equivalents.
    shortcuts: Vec<Value>,
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

/// Where an action shows up in the Command Center. The circuit palette and the
/// editor palette differ: Ctrl+S saves the circuit or the focused file.
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PaletteScope {
    Always,
    Editor,
    Circuit,
}

impl PaletteScope {
    pub fn includes(self, editor_focused: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Editor => editor_focused,
            Self::Circuit => !editor_focused,
        }
    }
}

/// One Command Center action. Its `shortcut` is the default the user's
/// custom binding overrides; `menu_bar` is where that binding is registered.
#[derive(Clone, Copy)]
pub struct PaletteAction {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub icon: &'static str,
}

const fn act(
    id: &'static str,
    label: &'static str,
    shortcut: &'static str,
    icon: &'static str,
) -> PaletteAction {
    PaletteAction {
        id,
        label,
        shortcut,
        icon,
    }
}

/// Shortcut-bindable actions, in Command Center order. `collect_shortcuts`
/// walks the menu tree instead of this list, so a row that is missing here
/// still keeps its menu shortcut.
#[rustfmt::skip]
pub const PALETTE_ACTIONS: &[(PaletteScope, PaletteAction)] = &[
    (PaletteScope::Always,  act("file.new", "New", "Ctrl+N", "add")),
    (PaletteScope::Always,  act("file.newCirc", "New Circuit", "", "account_tree")),
    (PaletteScope::Always,  act("file.newFile", "New File", "", "note_add")),
    (PaletteScope::Always,  act("file.newWindow", "New Window", "Ctrl+Shift+N", "open_in_new")),
    (PaletteScope::Always,  act("file.open", "Open...", "Ctrl+O", "folder_open")),
    (PaletteScope::Always,  act("file.save", "Save", "Ctrl+S", "save")),
    (PaletteScope::Always,  act("file.saveAs", "Save As...", "Ctrl+Shift+S", "save_as")),
    (PaletteScope::Always,  act("file.saveCirc", "Save Circuit", "", "save")),
    (PaletteScope::Always,  act("file.saveCircAs", "Save Circuit As...", "", "save_as")),
    (PaletteScope::Always,  act("file.saveFile", "Save File", "", "save")),
    (PaletteScope::Always,  act("file.saveFileAs", "Save File As...", "", "save_as")),
    (PaletteScope::Always,  act("file.saveAll", "Save All", "Ctrl+Alt+S", "save")),
    (PaletteScope::Always,  act("file.close", "Close", "Ctrl+W", "close")),
    (PaletteScope::Always,  act("file.closeCirc", "Close Circuit", "", "close")),
    (PaletteScope::Always,  act("file.closeFile", "Close File", "", "close")),
    (PaletteScope::Always,  act("file.openFolder", "Open Project", "", "folder_open")),
    (PaletteScope::Always,  act("file.closeProject", "Close Project", "", "folder_off")),
    (PaletteScope::Always,  act("file.saveImage", "Save Circuit as Image...", "", "image")),
    (PaletteScope::Always,  act("file.exportSub", "Export as Subcircuit...", "", "ios_share")),
    (PaletteScope::Always,  act("file.appSettings", "Application Settings...", "Ctrl+,", "settings")),
    (PaletteScope::Always,  act("file.quit", "Quit", "Ctrl+Q", "exit_to_app")),
    (PaletteScope::Always,  act("edit.undo", "Undo", "Ctrl+Z", "undo")),
    (PaletteScope::Always,  act("edit.redo", "Redo", "Ctrl+Y", "redo")),
    (PaletteScope::Always,  act("edit.cut", "Cut", "Ctrl+X", "content_cut")),
    (PaletteScope::Always,  act("edit.copy", "Copy", "Ctrl+C", "content_copy")),
    (PaletteScope::Always,  act("edit.paste", "Paste", "Ctrl+V", "content_paste")),
    (PaletteScope::Always,  act("edit.selectAll", "Select All", "Ctrl+A", "select_all")),
    (PaletteScope::Always,  act("edit.delete", "Delete Selection", "Delete", "delete")),
    (PaletteScope::Always,  act("edit.find", "Find", "Ctrl+F", "search")),
    (PaletteScope::Always,  act("edit.format", "Format Document", "Ctrl+Shift+I", "format_align_left")),
    (PaletteScope::Always,  act("edit.triggerCompletion", "Trigger Completion", "Ctrl+Space", "lightbulb")),
    (PaletteScope::Always,  act("edit.gotoDefinition", "Go to Definition", "Ctrl+F12", "search")),
    (PaletteScope::Always,  act("edit.rotateCw", "Rotate Clockwise", "Ctrl+R", "rotate_right")),
    (PaletteScope::Always,  act("edit.rotateCcw", "Rotate Counter-Clockwise", "Ctrl+Shift+R", "rotate_left")),
    (PaletteScope::Always,  act("edit.flipH", "Flip Horizontal", "Ctrl+L", "flip")),
    (PaletteScope::Always,  act("edit.flipV", "Flip Vertical", "Ctrl+Shift+L", "flip")),
    (PaletteScope::Always,  act("view.zoomIn", "Zoom In", "Ctrl++", "zoom_in")),
    (PaletteScope::Always,  act("view.zoomOut", "Zoom Out", "Ctrl+-", "zoom_out")),
    (PaletteScope::Always,  act("view.zoomFit", "Fit to View", "Ctrl+9", "fit_screen")),
    (PaletteScope::Always,  act("view.zoomSel", "Zoom to Selection", "", "fit_screen")),
    (PaletteScope::Always,  act("view.zoomOne", "Actual Size (1:1)", "Ctrl+0", "zoom_out_map")),
    (PaletteScope::Always,  act("view.showGrid", "Toggle Grid", "G", "grid_4x4")),
    (PaletteScope::Always,  act("view.showScroll", "Toggle Scrollbars", "", "swap_vert")),
    (PaletteScope::Always,  act("view.searchComp", "Search Components", "Ctrl+Shift+F", "search")),
    (PaletteScope::Always,  act("view.focusFiles", "Focus Files", "Ctrl+Shift+E", "folder")),
    (PaletteScope::Always,  act("view.focusEditor", "Focus Text Editor", "Ctrl+E", "code")),
    (PaletteScope::Always,  act("view.focusSimLog", "Focus Simulator Messages", "Ctrl+Shift+M", "terminal")),
    (PaletteScope::Always,  act("view.focusCompLog", "Focus Compiler Messages", "Ctrl+Shift+U", "build")),
    (PaletteScope::Always,  act("view.sidePanel", "Toggle Side Panel", "Ctrl+B", "dock_to_left")),
    (PaletteScope::Always,  act("view.editorPanel", "Toggle Editor Panel", "", "code")),
    (PaletteScope::Always,  act("view.toggleActivePanel", "Toggle Active Panel", "Ctrl+J", "dock_to_left")),
    (PaletteScope::Always,  act("view.libManager", "Library Manager", "", "local_library")),
    (PaletteScope::Always,  act("circ.power", "Start / Stop Simulation", "F8", "power_settings_new")),
    (PaletteScope::Always,  act("circ.pause", "Pause / Resume Simulation", "F9", "pause")),
    (PaletteScope::Always,  act("circ.step", "Step Simulation", "", "skip_next")),
    (PaletteScope::Always,  act("circ.settings", "Circuit Properties...", "", "tune")),
    (PaletteScope::Always,  act("circ.addAllComponents", "Add All Components", "", "grid_view")),
    (PaletteScope::Always,  act("sim.compile", "Compile", "F10", "build")),
    (PaletteScope::Always,  act("sim.load", "Upload", "F11", "upload")),
    (PaletteScope::Always,  act("sim.uploadRun", "Upload and Run", "", "play_arrow")),
    (PaletteScope::Always,  act("sim.debug", "Debug", "F12", "bug_report")),
    (PaletteScope::Always,  act("sim.run", "Run", "F5", "fast_forward")),
    (PaletteScope::Always,  act("sim.step", "Step", "F6", "step_into")),
    (PaletteScope::Always,  act("sim.stepOver", "Step Over", "F7", "step_over")),
    (PaletteScope::Always,  act("sim.debugPause", "Pause Debugger", "", "pause")),
    (PaletteScope::Always,  act("sim.stop", "Stop Debugger", "", "stop")),
    (PaletteScope::Always,  act("sim.reset", "Reset Debugger", "", "restart_alt")),
    (PaletteScope::Always,  act("sim.animateLogic", "Animate Logic", "", "motion_photos_on")),
    (PaletteScope::Always,  act("sim.animateCurr", "Animate Current", "", "waves")),
    (PaletteScope::Always,  act("help.info", "Simulation Info", "", "info")),
    (PaletteScope::Always,  act("help.about", "About Circuit Simulator", "", "help")),
    (PaletteScope::Always,  act("help.aboutQt", "About Qt", "", "help_outline")),
    (PaletteScope::Always,  act("debug.toggleRepaintOverlay", "Toggle Canvas Repaint Debug Overlay", "Ctrl+Alt+D", "bug_report")),
    (PaletteScope::Always,  act("debug.toggleComponentRects", "Show Component Rect", "Ctrl+Alt+C", "crop_free")),
];

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

/// `Canvas::key_press` already implements these by hand. The canvas marks every
/// key it receives as handled, so a QML `Shortcut` bound to the same chord would
/// fire the action a second time — the menu keeps printing the hint, but only
/// the canvas reacts to the key.
const CANVAS_SHORTCUTS: &[&str] = &[
    "Ctrl+A",
    "Ctrl+C",
    "Ctrl+V",
    "Ctrl+X",
    "Ctrl+Y",
    "Ctrl+Z",
    "Delete",
    "Backspace",
];

fn canvas_owns(shortcut: &str) -> bool {
    CANVAS_SHORTCUTS
        .iter()
        .any(|s| s.eq_ignore_ascii_case(shortcut))
}

/// Actions and chords that are hardcoded with dedicated `Shortcut` components
/// in QML (`Ctrl+Shift+P` for Command Palette, `Ctrl+P` for Jump to Reference,
/// `Ctrl+Tab` for Tab Switcher).
///
/// They must not be registered dynamically by `menuShortcutInstantiator` in
/// `Main.qml`, otherwise Qt Quick detects an ambiguous shortcut overload and
/// suppresses both from firing.
const HARDCODED_ACTIONS: &[&str] = &["view.cmdPalette", "view.jumpRef", "view.tabSwitch"];

const HARDCODED_CHORDS: &[&str] = &["Ctrl+Shift+P", "Ctrl+P", "Ctrl+Tab", "Ctrl+Shift+Tab"];

fn hardcoded_owns(id: &str, shortcut: &str) -> bool {
    HARDCODED_ACTIONS.contains(&id)
        || HARDCODED_CHORDS
            .iter()
            .any(|s| s.eq_ignore_ascii_case(shortcut))
}

/// Every leaf that carries a shortcut, flattened for QML `Shortcut` items.
/// All but macOS use this: the in-window menu bar only paints the keys as text,
/// so without it a reassigned shortcut would show up in the menu and the
/// Command Center yet never fire. Leaves with an empty shortcut are skipped —
/// that is both a parent/separator and a shortcut the user cleared.
fn collect_shortcuts(entries: &[Entry], out: &mut Vec<Value>) {
    for e in entries {
        if let Some(sub) = &e.submenu {
            collect_shortcuts(&sub.entries, out);
            continue;
        }
        if e.separator || e.id.is_empty() || e.shortcut.is_empty() {
            continue;
        }
        if canvas_owns(&e.shortcut) || hardcoded_owns(&e.id, &e.shortcut) {
            continue;
        }
        out.push(json!({
            "id": e.id,
            "shortcut": e.shortcut,
            "enabled": e.enabled,
        }));
    }
}

fn menu_shortcuts(menus: &[MenuNode]) -> Vec<Value> {
    let mut out = Vec::new();
    for m in menus {
        collect_shortcuts(&m.entries, &mut out);
    }
    let custom_shortcuts = cs_engine::settings::get().shortcuts;
    for (_, action) in PALETTE_ACTIONS {
        if hardcoded_owns(action.id, action.shortcut) || out.iter().any(|v| v["id"] == action.id) {
            continue;
        }
        let effective_shortcut = custom_shortcuts
            .get(action.id)
            .map(|s| s.as_str())
            .unwrap_or(action.shortcut);
        if effective_shortcut.is_empty()
            || canvas_owns(effective_shortcut)
            || hardcoded_owns(action.id, effective_shortcut)
            || out.iter().any(|v| {
                v["shortcut"]
                    .as_str()
                    .map(|s| s.eq_ignore_ascii_case(effective_shortcut))
                    .unwrap_or(false)
            })
        {
            continue;
        }
        out.push(json!({
            "id": action.id,
            "shortcut": effective_shortcut,
            "enabled": true,
        }));
    }
    out
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
                        item(
                            "file.newWindow",
                            &t("&Window"),
                            "Ctrl+Shift+N",
                            true,
                            false,
                            false,
                        ),
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
                item("file.close", &t("&Close"), "Ctrl+W", true, false, false),
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
                item(
                    "edit.format",
                    &t("Format Document"),
                    "Ctrl+Shift+I",
                    true,
                    false,
                    false,
                ),
                item(
                    "edit.triggerCompletion",
                    &t("Trigger Completion"),
                    "Ctrl+Space",
                    true,
                    false,
                    false,
                ),
                item(
                    "edit.gotoDefinition",
                    &t("Go to Definition"),
                    "Ctrl+F12",
                    true,
                    false,
                    false,
                ),
                sep(),
                item(
                    "edit.rotateCw",
                    &t("Rotate Clockwise"),
                    "Ctrl+R",
                    true,
                    false,
                    false,
                ),
                item(
                    "edit.rotateCcw",
                    &t("Rotate Counter-Clockwise"),
                    "Ctrl+Shift+R",
                    true,
                    false,
                    false,
                ),
                item(
                    "edit.flipH",
                    &t("Flip Horizontal"),
                    "Ctrl+L",
                    true,
                    false,
                    false,
                ),
                item(
                    "edit.flipV",
                    &t("Flip Vertical"),
                    "Ctrl+Shift+L",
                    true,
                    false,
                    false,
                ),
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
                item("view.zoomIn", &t("Zoom In"), "Ctrl++", true, false, false),
                item("view.zoomOut", &t("Zoom Out"), "Ctrl+-", true, false, false),
                item(
                    "view.zoomFit",
                    &t("Zoom to Fit"),
                    "Ctrl+9",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.zoomSel",
                    &t("Zoom to Selection"),
                    "",
                    true,
                    false,
                    false,
                ),
                item(
                    "view.zoomOne",
                    &t("Reset Zoom"),
                    "Ctrl+0",
                    true,
                    false,
                    false,
                ),
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
        let shortcuts = menu_shortcuts(&menus);
        Self {
            menus,
            json,
            shortcuts,
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
        self.shortcuts = menu_shortcuts(&self.menus);
        macos_host::set_native_menu_json(&Value::Array(self.json.clone()).to_string());
        self.menus_changed();
        self.shortcuts_changed();
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl AppMenuBar {
    qproperty!("menus", Read = menus, Notify = menus_changed);
    qproperty!("shortcuts", Read = shortcuts, Notify = shortcuts_changed);

    #[qsignal]
    fn menus_changed(&mut self);
    #[qsignal]
    fn shortcuts_changed(&mut self);
    #[qsignal]
    fn action(&mut self, id: String);

    fn menus(&self) -> Vec<Value> {
        self.json.clone()
    }

    fn shortcuts(&self) -> Vec<Value> {
        self.shortcuts.clone()
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
        let list = menu_shortcuts(&[node]);
        assert_eq!(shortcut_of(&list, "file.save").as_deref(), Some("Ctrl+S"));
        assert_eq!(
            shortcut_of(&list, "file.recentFile.0").as_deref(),
            Some("Ctrl+1"),
            "submenu leaves must be registered too"
        );
        assert!(
            list.iter().all(|e| e["id"] != "file.closeProject"),
            "a cleared shortcut must not be registered"
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
        let list = menu_shortcuts(&[node]);
        assert_eq!(shortcut_of(&list, "edit.undo"), None);
        assert_eq!(shortcut_of(&list, "edit.delete"), None);
        assert_eq!(shortcut_of(&list, "edit.find").as_deref(), Some("Ctrl+F"));
    }

    #[test]
    fn default_menu_bar_publishes_save_shortcuts_for_qml() {
        let bar = AppMenuBar::default();
        let list = bar.shortcuts();
        assert_eq!(
            shortcut_of(&list, "file.save").as_deref(),
            Some("Ctrl+S"),
            "QML registers Ctrl+S from this list on non-macOS platforms"
        );
        assert_eq!(
            shortcut_of(&list, "file.saveAs").as_deref(),
            Some("Ctrl+Shift+S")
        );
        assert_eq!(shortcut_of(&list, "file.close").as_deref(), Some("Ctrl+W"));
        assert_eq!(shortcut_of(&list, "file.new").as_deref(), Some("Ctrl+N"));
        assert_eq!(
            shortcut_of(&list, "file.newWindow").as_deref(),
            Some("Ctrl+Shift+N")
        );
        assert_eq!(
            shortcut_of(&list, "edit.rotateCw").as_deref(),
            Some("Ctrl+R")
        );
        assert_eq!(
            shortcut_of(&list, "edit.rotateCcw").as_deref(),
            Some("Ctrl+Shift+R")
        );
        assert_eq!(
            shortcut_of(&list, "edit.triggerCompletion").as_deref(),
            Some("Ctrl+Space")
        );
        assert_eq!(
            shortcut_of(&list, "edit.gotoDefinition").as_deref(),
            Some("Ctrl+F12")
        );
        assert_eq!(shortcut_of(&list, "edit.flipH").as_deref(), Some("Ctrl+L"));
        assert_eq!(
            shortcut_of(&list, "edit.flipV").as_deref(),
            Some("Ctrl+Shift+L")
        );
        assert_eq!(shortcut_of(&list, "view.zoomIn").as_deref(), Some("Ctrl++"));
        assert_eq!(
            shortcut_of(&list, "view.zoomOut").as_deref(),
            Some("Ctrl+-")
        );
        assert_eq!(
            shortcut_of(&list, "view.zoomFit").as_deref(),
            Some("Ctrl+9")
        );
        assert_eq!(
            shortcut_of(&list, "view.zoomOne").as_deref(),
            Some("Ctrl+0")
        );
        assert_eq!(
            shortcut_of(&list, "debug.toggleRepaintOverlay").as_deref(),
            Some("Ctrl+Alt+D")
        );
        assert_eq!(
            shortcut_of(&list, "view.cmdPalette"),
            None,
            "hardcoded in Main.qml; must not be registered dynamically"
        );
        assert_eq!(
            shortcut_of(&list, "view.jumpRef"),
            None,
            "hardcoded in Main.qml; must not be registered dynamically"
        );
        assert_eq!(
            shortcut_of(&list, "view.tabSwitch"),
            None,
            "hardcoded in Main.qml; must not be registered dynamically"
        );
    }
}
