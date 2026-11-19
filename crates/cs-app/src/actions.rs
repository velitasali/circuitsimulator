//! Central registry of application actions and keyboard shortcuts.
//!
//! All default shortcuts, legacy ID migrations, chord normalization, conflict
//! checks, and platform-specific shortcut collection live here so the menu bar,
//! Command Center, context menus, and macOS `NSMenu` never drift apart.

use std::collections::{BTreeMap, HashSet};

use serde_json::{Map, Value, json};

use crate::ui_icon::UiIcon;

/// One application action known to the menu bar and/or Command Center.
#[derive(Clone, Copy, Debug)]
pub struct ActionDef {
    pub id: &'static str,
    pub label: &'static str,
    pub default_shortcut: &'static str,
    pub icon: UiIcon,
    pub in_palette: bool,
}

const fn act(
    id: &'static str,
    label: &'static str,
    default_shortcut: &'static str,
    icon: UiIcon,
) -> ActionDef {
    ActionDef {
        id,
        label,
        default_shortcut,
        icon,
        in_palette: true,
    }
}

const fn menu_only(
    id: &'static str,
    label: &'static str,
    default_shortcut: &'static str,
    icon: UiIcon,
) -> ActionDef {
    ActionDef {
        id,
        label,
        default_shortcut,
        icon,
        in_palette: false,
    }
}

/// All canonical application actions, in Command Center display order.
#[rustfmt::skip]
pub const ACTIONS: &[ActionDef] = &[
    act("file.new", "New", "Ctrl+N", UiIcon::Add),
    act("file.newCirc", "New Circuit", "", UiIcon::AccountTree),
    act("file.newFile", "New File", "", UiIcon::NoteAdd),
    act("file.newWindow", "New Window", "Ctrl+Shift+N", UiIcon::OpenInNew),
    act("file.open", "Open...", "Ctrl+O", UiIcon::FolderOpen),
    act("file.examples", "Example Circuits...", "", UiIcon::LibraryBooks),
    act("file.save", "Save", "Ctrl+S", UiIcon::Save),
    act("file.saveAs", "Save As...", "Ctrl+Shift+S", UiIcon::SaveAs),
    act("file.saveCirc", "Save Circuit", "", UiIcon::Save),
    act("file.saveCircAs", "Save Circuit As...", "", UiIcon::SaveAs),
    act("file.saveFile", "Save File", "", UiIcon::Save),
    act("file.saveFileAs", "Save File As...", "", UiIcon::SaveAs),
    act("file.saveAll", "Save All", "Ctrl+Alt+S", UiIcon::Save),
    act("file.close", "Close", "Ctrl+W", UiIcon::Close),
    act("file.closeCirc", "Close Circuit", "", UiIcon::Close),
    act("file.closeFile", "Close File", "", UiIcon::Close),
    act("file.openFolder", "Open Project", "", UiIcon::FolderOpen),
    act("file.closeProject", "Close Project", "", UiIcon::FolderOff),
    act("file.saveImage", "Save Circuit as Image...", "", UiIcon::Image),
    act("file.exportSpice", "Export SPICE Netlist...", "", UiIcon::Description),
    act("file.exportKicad", "Export KiCad Netlist...", "", UiIcon::AccountTree),
    act("file.exportSub", "Export as Subcircuit...", "", UiIcon::IosShare),
    act("file.settings", "Application Settings...", "Ctrl+,", UiIcon::Settings),
    act("file.quit", "Quit", "Ctrl+Q", UiIcon::ExitToApp),
    act("edit.undo", "Undo", "Ctrl+Z", UiIcon::Undo),
    act("edit.redo", "Redo", "Ctrl+Y", UiIcon::Redo),
    act("edit.cut", "Cut", "Ctrl+X", UiIcon::ContentCut),
    act("edit.copy", "Copy", "Ctrl+C", UiIcon::ContentCopy),
    act("edit.paste", "Paste", "Ctrl+V", UiIcon::ContentPaste),
    act("edit.selectAll", "Select All", "Ctrl+A", UiIcon::SelectAll),
    act("edit.delete", "Delete Selection", "Delete", UiIcon::Delete),
    act("edit.find", "Find", "Ctrl+F", UiIcon::Search),
    act("edit.format", "Format Document", "Ctrl+Shift+I", UiIcon::FormatAlignLeft),
    act("edit.triggerCompletion", "Trigger Completion", "Ctrl+Space", UiIcon::Lightbulb),
    act("edit.gotoDefinition", "Go to Definition", "Ctrl+F12", UiIcon::Search),
    act("edit.rotateCw", "Rotate Clockwise", "Ctrl+R", UiIcon::RotateRight),
    act("edit.rotateCcw", "Rotate Counter-Clockwise", "Ctrl+Shift+R", UiIcon::RotateLeft),
    act("edit.flipH", "Flip Horizontal", "Ctrl+L", UiIcon::Flip),
    act("edit.flipV", "Flip Vertical", "Ctrl+Shift+L", UiIcon::Flip),
    act("view.zoomIn", "Zoom In", "Ctrl++", UiIcon::ZoomIn),
    act("view.zoomOut", "Zoom Out", "Ctrl+-", UiIcon::ZoomOut),
    act("view.zoomFit", "Fit to View", "Ctrl+9", UiIcon::FitScreen),
    act("view.zoomSel", "Zoom to Selection", "", UiIcon::FitScreen),
    act("view.zoomOne", "Actual Size (1:1)", "Ctrl+0", UiIcon::ZoomOutMap),
    act("view.showGrid", "Toggle Grid", "G", UiIcon::Grid4x4),
    act("view.showScroll", "Toggle Scrollbars", "", UiIcon::SwapVert),
    menu_only("view.cmdPalette", "Command Center", "Ctrl+Shift+P", UiIcon::Search),
    menu_only("view.jumpRef", "Jump to Reference", "Ctrl+P", UiIcon::Search),
    menu_only(
        "view.tabSwitch",
        "Switch Tab/Circuit",
        if cfg!(target_os = "macos") {
            "Meta+Tab"
        } else {
            "Ctrl+Tab"
        },
        UiIcon::SwapVert,
    ),
    act("view.searchComp", "Search Components", "Ctrl+Shift+F", UiIcon::Search),
    act("view.focusFiles", "Focus Files", "Ctrl+Shift+E", UiIcon::Folder),
    act("view.focusEditor", "Focus Text Editor", "Ctrl+E", UiIcon::Code),
    act("view.focusSimLog", "Focus Simulator Messages", "Ctrl+Shift+M", UiIcon::Terminal),
    act("view.focusCompLog", "Focus Compiler Messages", "Ctrl+Shift+U", UiIcon::Build),
    act("view.sidePanel", "Toggle Side Panel", "Ctrl+B", UiIcon::DockToLeft),
    act("view.editorPanel", "Toggle Editor Panel", "", UiIcon::Code),
    act("view.toggleActivePanel", "Toggle Active Panel", "Ctrl+J", UiIcon::DockToLeft),
    act("view.libManager", "Library Manager", "", UiIcon::LocalLibrary),
    act("sim.power", "Start / Stop Simulation", "F8", UiIcon::PowerSettingsNew),
    act("sim.pause", "Pause / Resume Simulation", "F9", UiIcon::Pause),
    act("circ.step", "Step Simulation", "", UiIcon::SkipNext),
    act("circ.settings", "Circuit Properties...", "", UiIcon::Tune),
    act("circ.addAllComponents", "Add All Components", "", UiIcon::GridView),
    act("sim.compile", "Compile", "F10", UiIcon::Build),
    act("sim.load", "Upload", "F11", UiIcon::Upload),
    act("sim.uploadRun", "Upload and Run", "", UiIcon::PlayArrow),
    act("sim.debug", "Debug", "F12", UiIcon::BugReport),
    act("sim.run", "Run", "F5", UiIcon::FastForward),
    act("sim.step", "Step", "F6", UiIcon::StepInto),
    act("sim.stepOver", "Step Over", "F7", UiIcon::StepOver),
    act("sim.debugPause", "Pause Debugger", "", UiIcon::Pause),
    act("sim.stop", "Stop Debugger", "", UiIcon::Stop),
    act("sim.reset", "Reset Debugger", "", UiIcon::RestartAlt),
    act("sim.animateLogic", "Animate Logic", "", UiIcon::MotionPhotosOn),
    act("sim.animateCurr", "Animate Current", "", UiIcon::Waves),
    act("sim.bode", "AC Sweep...", "", UiIcon::ShowChart),
    act("sim.dcSweep", "DC Parameter Sweep...", "", UiIcon::Insights),
    act("help.info", "Simulation Info", "", UiIcon::Info),
    act("help.about", "About Circuit Simulator", "", UiIcon::Help),
    act("debug.toggleRepaintOverlay", "Toggle Canvas Repaint Debug Overlay", "Ctrl+Alt+D", UiIcon::BugReport),
    act("debug.toggleComponentRects", "Show Component Rect", "Ctrl+Alt+C", UiIcon::CropFree),
];

/// `Canvas::key_press` (and Qt `TextArea` built-ins) handle these chords
/// directly. Registering a QML `Shortcut` for the same chord would double-fire
/// or steal keys from text inputs.
const CANVAS_SHORTCUTS: &[&str] = &[
    "Ctrl+A",
    "Ctrl+C",
    "Ctrl+V",
    "Ctrl+X",
    "Ctrl+Y",
    "Ctrl+Z",
    "Ctrl+Shift+Z",
    "Delete",
    "Backspace",
];

/// Actions owned by dedicated `Shortcut` elements in `Main.qml`.
const HARDCODED_ACTIONS: &[&str] = &["view.cmdPalette", "view.jumpRef", "view.tabSwitch"];

/// Chords reserved by the dedicated `Shortcut` elements in `Main.qml`.
const HARDCODED_CHORDS: &[(&str, &str)] = &[
    ("Ctrl+Shift+P", "Command Center"),
    ("Meta+Shift+P", "Command Center"),
    ("Ctrl+P", "Jump to Reference"),
    ("Meta+P", "Jump to Reference"),
    ("Ctrl+Tab", "Switch Tab/Circuit"),
    ("Meta+Tab", "Switch Tab/Circuit"),
    ("Ctrl+Shift+Tab", "Switch Tab/Circuit"),
    ("Meta+Shift+Tab", "Switch Tab/Circuit"),
];

/// Maps retired action IDs from older `settings.json` files to their current ID.
pub fn canonical_id(id: &str) -> &str {
    match id {
        "file.appSettings" => "file.settings",
        "circ.power" => "sim.power",
        "circ.pause" => "sim.pause",
        "view.simLog" => "view.focusSimLog",
        "view.compLog" => "view.focusCompLog",
        other => other,
    }
}

fn legacy_aliases(canonical: &str) -> &'static [&'static str] {
    match canonical {
        "file.settings" => &["file.appSettings"],
        "sim.power" => &["circ.power"],
        "sim.pause" => &["circ.pause"],
        "view.focusSimLog" => &["view.simLog"],
        "view.focusCompLog" => &["view.compLog"],
        _ => &[],
    }
}

pub fn find(id: &str) -> Option<&'static ActionDef> {
    let cid = canonical_id(id);
    ACTIONS.iter().find(|a| a.id == cid)
}

pub fn palette_actions() -> impl Iterator<Item = &'static ActionDef> {
    ACTIONS.iter().filter(|a| a.in_palette)
}

pub fn default_shortcut(id: &str) -> &'static str {
    find(id).map(|a| a.default_shortcut).unwrap_or("")
}

/// Parses a shortcut chord into `(ctrl, alt, shift, meta, key)`.
fn parse_chord(raw: &str) -> Option<(bool, bool, bool, bool, String)> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;

    // Split on '+' while preserving a literal '+' key (e.g. "Ctrl++", "+").
    let (prefix, literal_plus) = if s == "+" {
        ("", true)
    } else if let Some(p) = s.strip_suffix("++") {
        (p, true)
    } else {
        (s, false)
    };

    let mut tokens: Vec<&str> = if prefix.is_empty() {
        Vec::new()
    } else {
        prefix
            .split('+')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect()
    };

    let raw_key = if literal_plus { "+" } else { tokens.pop()? };

    for tok in tokens {
        match tok.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" | "command" => ctrl = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            "meta" | "control" => meta = true,
            _ => {}
        }
    }

    let key = normalize_key_token(raw_key, &mut shift);
    if key.is_empty() {
        return None;
    }
    Some((ctrl, alt, shift, meta, key))
}

fn normalize_key_token(raw: &str, shift: &mut bool) -> String {
    let lower = raw.to_ascii_lowercase();
    match lower.as_str() {
        "esc" | "escape" => return "Esc".into(),
        "tab" => return "Tab".into(),
        "backtab" => {
            *shift = true;
            return "Tab".into();
        }
        "backspace" => return "Backspace".into(),
        "return" | "enter" => return "Return".into(),
        "ins" | "insert" => return "Insert".into(),
        "del" | "delete" => return "Delete".into(),
        "pause" => return "Pause".into(),
        "print" => return "Print".into(),
        "home" => return "Home".into(),
        "end" => return "End".into(),
        "left" => return "Left".into(),
        "up" => return "Up".into(),
        "right" => return "Right".into(),
        "down" => return "Down".into(),
        "pgup" | "pageup" => return "PageUp".into(),
        "pgdown" | "pagedown" => return "PageDown".into(),
        "space" => return "Space".into(),
        _ => {}
    }
    if let Some(rest) = lower.strip_prefix('f') {
        if let Ok(n) = rest.parse::<u8>() {
            if (1..=24).contains(&n) {
                return format!("F{n}");
            }
        }
    }
    if raw.len() == 1 {
        let ch = raw.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return ch.to_ascii_uppercase().to_string();
        }
    }
    raw.to_string()
}

/// Normalizes a Qt shortcut string into canonical modifier order
/// (`Ctrl+Alt+Shift+Meta+<Key>`). Empty/whitespace input stays empty.
pub fn normalize_shortcut(raw: &str) -> String {
    let Some((ctrl, alt, shift, meta, key)) = parse_chord(raw) else {
        return String::new();
    };
    let mut parts: Vec<&str> = Vec::with_capacity(5);
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if meta {
        parts.push("Meta");
    }
    parts.push(&key);
    parts.join("+")
}

/// Returns whether two shortcut strings refer to the same key chord.
/// `Ctrl++` and `Ctrl+=` are treated as equivalent because `Main.qml` binds
/// both sequences together for zoom-in.
pub fn chords_equal(a: &str, b: &str) -> bool {
    let na = normalize_shortcut(a);
    let nb = normalize_shortcut(b);
    if na.is_empty() || nb.is_empty() {
        return false;
    }
    if na.eq_ignore_ascii_case(&nb) {
        return true;
    }
    matches!(
        (na.as_str(), nb.as_str()),
        ("Ctrl++", "Ctrl+=") | ("Ctrl+=", "Ctrl++")
    )
}

pub fn canvas_owns(shortcut: &str) -> bool {
    CANVAS_SHORTCUTS.iter().any(|s| chords_equal(s, shortcut))
}

pub fn hardcoded_owns(id: &str, shortcut: &str) -> bool {
    let cid = canonical_id(id);
    HARDCODED_ACTIONS.contains(&cid)
        || HARDCODED_CHORDS
            .iter()
            .any(|(chord, _)| chords_equal(chord, shortcut))
}

/// Whether macOS `NSMenu` should own a menu item's key equivalent.
/// Bare non-function keys (such as `G` for Toggle Grid) must stay out of
/// `NSMenu` so AppKit does not hijack typing in text fields; QML registers
/// them instead where `TextInput` accepts `ShortcutOverride`.
pub fn macos_menu_owns_shortcut(shortcut: &str) -> bool {
    let Some((ctrl, alt, _shift, meta, key)) = parse_chord(shortcut) else {
        return false;
    };
    if ctrl || alt || meta {
        return true;
    }
    if let Some(rest) = key.strip_prefix('F') {
        if let Ok(n) = rest.parse::<u8>() {
            return (1..=24).contains(&n);
        }
    }
    false
}

/// Resolves the active shortcut for `id` against user overrides.
///
/// - Absent key: built-in default from `ACTIONS`.
/// - Present empty string `""`: explicitly unbound by the user.
/// - Present non-empty string: normalized custom binding.
pub fn effective_shortcut(id: &str, custom: &BTreeMap<String, String>) -> String {
    let cid = canonical_id(id);
    if let Some(val) = custom.get(cid) {
        return normalize_shortcut(val);
    }
    for alias in legacy_aliases(cid) {
        if let Some(val) = custom.get(*alias) {
            return normalize_shortcut(val);
        }
    }
    normalize_shortcut(default_shortcut(cid))
}

/// Map of every canonical action ID to its effective Qt shortcut string.
pub fn effective_shortcut_map(custom: &BTreeMap<String, String>) -> Map<String, Value> {
    let mut map = Map::with_capacity(ACTIONS.len());
    for action in ACTIONS {
        let sc = effective_shortcut(action.id, custom);
        map.insert(action.id.to_string(), Value::String(sc));
    }
    map
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conflict {
    None,
    Reserved {
        reason: &'static str,
    },
    Action {
        id: &'static str,
        label: &'static str,
    },
}

/// Checks whether assigning `raw_shortcut` to `action_id` conflicts with a
/// reserved chord or another action's effective binding.
pub fn check_conflict(
    action_id: &str,
    raw_shortcut: &str,
    custom: &BTreeMap<String, String>,
) -> Conflict {
    let norm = normalize_shortcut(raw_shortcut);
    if norm.is_empty() {
        return Conflict::None;
    }
    let cid = canonical_id(action_id);

    if let Some((_, owner_label)) = HARDCODED_CHORDS
        .iter()
        .find(|(chord, _)| chords_equal(chord, &norm))
    {
        return Conflict::Reserved {
            reason: owner_label,
        };
    }

    if canvas_owns(&norm) && !chords_equal(default_shortcut(cid), &norm) {
        return Conflict::Reserved {
            reason: "Canvas editing",
        };
    }

    for action in ACTIONS {
        if action.id == cid {
            continue;
        }
        let other = effective_shortcut(action.id, custom);
        if chords_equal(&other, &norm) {
            return Conflict::Action {
                id: action.id,
                label: action.label,
            };
        }
    }

    Conflict::None
}

fn unbind_action(custom: &mut BTreeMap<String, String>, cid: &str) {
    for alias in legacy_aliases(cid) {
        custom.remove(*alias);
    }
    if default_shortcut(cid).is_empty() {
        custom.remove(cid);
    } else {
        custom.insert(cid.to_string(), String::new());
    }
}

fn unbind_chord_owners(custom: &mut BTreeMap<String, String>, except_cid: &str, norm_chord: &str) {
    if norm_chord.is_empty() {
        return;
    }
    let conflicting: Vec<&'static str> = ACTIONS
        .iter()
        .filter(|a| a.id != except_cid)
        .filter(|a| chords_equal(&effective_shortcut(a.id, custom), norm_chord))
        .map(|a| a.id)
        .collect();
    for other_id in conflicting {
        unbind_action(custom, other_id);
    }
}

/// Applies a user shortcut edit to `custom`.
///
/// - Passing `""` explicitly unbinds the action (even if it has a default).
/// - Passing the action's default shortcut removes the custom override.
/// - Passing a chord already assigned to another action clears the old owner.
/// - Returns `false` without mutating `custom` if the chord is reserved.
pub fn set_shortcut(
    custom: &mut BTreeMap<String, String>,
    action_id: &str,
    raw_shortcut: &str,
) -> bool {
    let cid = canonical_id(action_id);
    let norm = normalize_shortcut(raw_shortcut);

    if matches!(
        check_conflict(cid, &norm, custom),
        Conflict::Reserved { .. }
    ) {
        return false;
    }

    for alias in legacy_aliases(cid) {
        custom.remove(*alias);
    }

    if !norm.is_empty() {
        unbind_chord_owners(custom, cid, &norm);
    }

    let def = normalize_shortcut(default_shortcut(cid));
    if norm.is_empty() {
        unbind_action(custom, cid);
    } else if chords_equal(&norm, &def) {
        custom.remove(cid);
    } else {
        custom.insert(cid.to_string(), norm);
    }

    true
}

/// Resets an action's shortcut back to its built-in default, clearing any
/// conflicting custom assignment on another action.
pub fn reset_shortcut(custom: &mut BTreeMap<String, String>, action_id: &str) {
    let cid = canonical_id(action_id);
    for alias in legacy_aliases(cid) {
        custom.remove(*alias);
    }
    let def = normalize_shortcut(default_shortcut(cid));
    if !def.is_empty() {
        unbind_chord_owners(custom, cid, &def);
    }
    custom.remove(cid);
}

/// Flattened leaf entry from `AppMenuBar`'s menu tree.
pub struct MenuLeaf<'a> {
    pub id: &'a str,
    pub shortcut: &'a str,
    pub enabled: bool,
}

/// Produces the QML `Shortcut` registration list for `Main.qml`.
///
/// - When `native_menus` is `false` (Windows/Linux): registers all menu and
///   non-menu shortcuts (excluding canvas-owned and hardcoded chords).
/// - When `native_menus` is `true` (macOS): skips chords owned by native
///   `NSMenu` items so they do not double-fire, while still registering
///   non-menu actions and bare-key menu shortcuts (`G`).
/// - Deduplicates chords so Qt never encounters an ambiguous overload.
pub fn collect_qml_shortcuts(
    menu_leaves: &[MenuLeaf<'_>],
    native_menus: bool,
    custom: &BTreeMap<String, String>,
) -> Vec<Value> {
    let mut out = Vec::new();
    let mut menu_ids: HashSet<&str> = HashSet::new();
    let mut claimed_chords: Vec<String> = Vec::new();

    for leaf in menu_leaves {
        if leaf.id.is_empty() {
            continue;
        }
        let cid = canonical_id(leaf.id);
        menu_ids.insert(cid);

        let norm = normalize_shortcut(leaf.shortcut);
        if norm.is_empty() {
            continue;
        }

        if native_menus && macos_menu_owns_shortcut(&norm) {
            claimed_chords.push(norm);
            continue;
        }

        if canvas_owns(&norm)
            || hardcoded_owns(cid, &norm)
            || claimed_chords.iter().any(|c| chords_equal(c, &norm))
        {
            continue;
        }

        claimed_chords.push(norm.clone());
        out.push(json!({
            "id": cid,
            "shortcut": norm,
            "enabled": leaf.enabled,
        }));
    }

    for action in ACTIONS {
        if menu_ids.contains(action.id) || hardcoded_owns(action.id, action.default_shortcut) {
            continue;
        }
        let norm = effective_shortcut(action.id, custom);
        if norm.is_empty()
            || canvas_owns(&norm)
            || hardcoded_owns(action.id, &norm)
            || claimed_chords.iter().any(|c| chords_equal(c, &norm))
        {
            continue;
        }
        claimed_chords.push(norm.clone());
        out.push(json!({
            "id": action.id,
            "shortcut": norm,
            "enabled": true,
        }));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_ids_and_default_shortcuts_are_unique() {
        let mut ids = HashSet::new();
        let mut chords: Vec<(&str, String)> = Vec::new();

        for a in ACTIONS {
            assert!(ids.insert(a.id), "duplicate action id '{}'", a.id);
            let norm = normalize_shortcut(a.default_shortcut);
            if !norm.is_empty() {
                if let Some((prev_id, _)) = chords.iter().find(|(_, c)| chords_equal(c, &norm)) {
                    panic!(
                        "duplicate default shortcut '{}' on '{}' and '{}'",
                        norm, prev_id, a.id
                    );
                }
                chords.push((a.id, norm));
            }
        }
    }

    #[test]
    fn normalize_handles_plus_backtab_and_modifier_order() {
        assert_eq!(normalize_shortcut("Ctrl++"), "Ctrl++");
        assert_eq!(normalize_shortcut("Shift+Ctrl++"), "Ctrl+Shift++");
        assert_eq!(normalize_shortcut("+"), "+");
        assert_eq!(normalize_shortcut("Shift+Backtab"), "Shift+Tab");
        assert_eq!(normalize_shortcut("Backtab"), "Shift+Tab");
        assert_eq!(normalize_shortcut("shift+alt+ctrl+s"), "Ctrl+Alt+Shift+S");
        assert_eq!(normalize_shortcut("cmd+shift+r"), "Ctrl+Shift+R");
        assert_eq!(normalize_shortcut("del"), "Delete");
        assert_eq!(normalize_shortcut("escape"), "Esc");
        assert_eq!(normalize_shortcut("f12"), "F12");
        assert_eq!(normalize_shortcut("  "), "");
    }

    #[test]
    fn clear_unbinds_default_and_reset_restores_it() {
        let mut custom = BTreeMap::new();
        assert_eq!(effective_shortcut("file.save", &custom), "Ctrl+S");

        // Clearing an action with a default stores "" so it stays unbound.
        assert!(set_shortcut(&mut custom, "file.save", ""));
        assert_eq!(effective_shortcut("file.save", &custom), "");
        assert_eq!(custom.get("file.save").map(String::as_str), Some(""));

        // Reset restores the default and removes the override entry.
        reset_shortcut(&mut custom, "file.save");
        assert_eq!(effective_shortcut("file.save", &custom), "Ctrl+S");
        assert!(!custom.contains_key("file.save"));
    }

    #[test]
    fn rebinding_to_another_actions_chord_unbinds_previous_owner() {
        let mut custom = BTreeMap::new();
        assert_eq!(
            check_conflict("file.open", "Ctrl+S", &custom),
            Conflict::Action {
                id: "file.save",
                label: "Save"
            }
        );

        assert!(set_shortcut(&mut custom, "file.open", "Ctrl+S"));
        assert_eq!(effective_shortcut("file.open", &custom), "Ctrl+S");
        assert_eq!(effective_shortcut("file.save", &custom), "");
        assert_eq!(
            check_conflict("file.open", "Ctrl+S", &custom),
            Conflict::None
        );

        // Resetting file.save reclaims Ctrl+S and clears file.open.
        reset_shortcut(&mut custom, "file.save");
        assert_eq!(effective_shortcut("file.save", &custom), "Ctrl+S");
        assert_eq!(effective_shortcut("file.open", &custom), "");
    }

    #[test]
    fn reserved_chords_are_rejected() {
        let mut custom = BTreeMap::new();
        assert!(matches!(
            check_conflict("file.save", "Ctrl+P", &custom),
            Conflict::Reserved { .. }
        ));
        assert!(matches!(
            check_conflict("file.save", "Ctrl+Z", &custom),
            Conflict::Reserved { .. }
        ));
        assert!(!set_shortcut(&mut custom, "file.save", "Ctrl+P"));
        assert!(!set_shortcut(&mut custom, "file.save", "Ctrl+Z"));
        assert_eq!(effective_shortcut("file.save", &custom), "Ctrl+S");

        // Canvas action itself can still be set back to its own default.
        assert_eq!(
            check_conflict("edit.undo", "Ctrl+Z", &custom),
            Conflict::None
        );
        assert!(set_shortcut(&mut custom, "edit.undo", "Ctrl+U"));
        assert_eq!(effective_shortcut("edit.undo", &custom), "Ctrl+U");
        assert!(set_shortcut(&mut custom, "edit.undo", "Ctrl+Z"));
        assert_eq!(effective_shortcut("edit.undo", &custom), "Ctrl+Z");
    }

    #[test]
    fn legacy_ids_resolve_and_migrate_on_write() {
        let mut custom = BTreeMap::new();
        custom.insert("file.appSettings".into(), "Ctrl+Alt+,".into());
        custom.insert("circ.power".into(), "F4".into());

        assert_eq!(effective_shortcut("file.settings", &custom), "Ctrl+Alt+,");
        assert_eq!(effective_shortcut("sim.power", &custom), "F4");

        assert!(set_shortcut(&mut custom, "file.settings", "Ctrl+Shift+,"));
        assert!(!custom.contains_key("file.appSettings"));
        assert_eq!(effective_shortcut("file.settings", &custom), "Ctrl+Shift+,");
    }

    #[test]
    fn collect_qml_shortcuts_handles_native_menus_and_deduplication() {
        let custom = BTreeMap::new();
        let leaves = vec![
            MenuLeaf {
                id: "file.save",
                shortcut: "Ctrl+S",
                enabled: true,
            },
            MenuLeaf {
                id: "view.showGrid",
                shortcut: "G",
                enabled: true,
            },
            MenuLeaf {
                id: "sim.power",
                shortcut: "F8",
                enabled: true,
            },
            // Duplicate chord should be deduplicated.
            MenuLeaf {
                id: "file.saveCirc",
                shortcut: "Ctrl+S",
                enabled: true,
            },
        ];

        // Non-macOS: registers menu items + non-menu palette actions, deduplicating Ctrl+S.
        let non_mac = collect_qml_shortcuts(&leaves, false, &custom);
        assert!(
            non_mac
                .iter()
                .any(|v| v["id"] == "file.save" && v["shortcut"] == "Ctrl+S")
        );
        assert!(!non_mac.iter().any(|v| v["id"] == "file.saveCirc"));
        assert!(
            non_mac
                .iter()
                .any(|v| v["id"] == "view.showGrid" && v["shortcut"] == "G")
        );
        assert!(
            non_mac
                .iter()
                .any(|v| v["id"] == "sim.power" && v["shortcut"] == "F8")
        );
        assert!(
            non_mac
                .iter()
                .any(|v| v["id"] == "file.new" && v["shortcut"] == "Ctrl+N")
        );

        // macOS: NSMenu owns Ctrl+S and F8, while QML still owns bare 'G' and non-menu Ctrl+N.
        let mac = collect_qml_shortcuts(&leaves, true, &custom);
        assert!(!mac.iter().any(|v| v["id"] == "file.save"));
        assert!(!mac.iter().any(|v| v["id"] == "sim.power"));
        assert!(!mac.iter().any(|v| v["id"] == "file.saveCirc"));
        assert!(
            mac.iter()
                .any(|v| v["id"] == "view.showGrid" && v["shortcut"] == "G")
        );
        assert!(
            mac.iter()
                .any(|v| v["id"] == "file.new" && v["shortcut"] == "Ctrl+N")
        );
        assert!(
            mac.iter()
                .any(|v| v["id"] == "debug.toggleRepaintOverlay" && v["shortcut"] == "Ctrl+Alt+D")
        );
    }

    #[test]
    fn effective_shortcut_map_includes_canvas_shortcuts() {
        let custom = BTreeMap::new();
        let map = effective_shortcut_map(&custom);
        assert_eq!(map.get("edit.copy").and_then(Value::as_str), Some("Ctrl+C"));
        assert_eq!(map.get("edit.undo").and_then(Value::as_str), Some("Ctrl+Z"));
        assert_eq!(
            map.get("edit.delete").and_then(Value::as_str),
            Some("Delete")
        );
        assert_eq!(map.get("sim.power").and_then(Value::as_str), Some("F8"));
    }
}
