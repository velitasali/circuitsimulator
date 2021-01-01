//! Command Center / Command Palette / Jump to Reference.

use qtbridge::qobject;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub struct CommandCenter {
    search_text: String,
    current_row: i32,
    items: Vec<Value>,
    circuit_items: Vec<Value>,
    visible: bool,
    project_dir: String,
    editor_focused: bool,
    lib: cs_engine::library::Library,
}

impl Default for CommandCenter {
    fn default() -> Self {
        let s = cs_engine::settings::get();
        let proj = if crate::pending::no_project() {
            String::new()
        } else {
            s.last_project_dir
                .or_else(|| s.recent_projects.first().cloned())
                .unwrap_or_default()
        };
        let mut lib = cs_engine::library::Library::new();
        lib.add_catalog_mcus(&cs_engine::catalog::standard());
        let mut cc = Self {
            search_text: String::new(),
            current_row: 0,
            items: Vec::new(),
            circuit_items: Vec::new(),
            visible: false,
            project_dir: proj,
            editor_focused: false,
            lib,
        };
        cc.populate_jump_to_reference("");
        cc
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl CommandCenter {
    qproperty!(
        "searchText",
        Read = search_text,
        Write = set_search_text,
        Notify = search_text_changed
    );
    qproperty!(
        "currentRow",
        Read = current_row,
        Write = set_current_row,
        Notify = current_row_changed
    );
    qproperty!("items", Read = items, Notify = items_changed);
    qproperty!(
        "visible",
        Read = is_visible,
        Write = set_visible,
        Notify = visible_changed
    );
    qproperty!(
        "projectDir",
        Read = project_dir,
        Write = set_project_dir,
        Notify = project_dir_changed
    );
    qproperty!(
        "editorFocused",
        Read = editor_focused,
        Write = set_editor_focused,
        Notify = editor_focused_changed
    );

    #[qsignal]
    fn search_text_changed(&mut self);
    #[qsignal]
    fn current_row_changed(&mut self);
    #[qsignal]
    fn items_changed(&mut self);
    #[qsignal]
    fn visible_changed(&mut self);
    #[qsignal]
    fn project_dir_changed(&mut self);
    #[qsignal]
    fn editor_focused_changed(&mut self);
    #[qsignal]
    fn shortcuts_updated(&mut self);
    #[qsignal]
    fn execute_item(&mut self, typ: String, data: String, comp_type: String);

    fn items(&self) -> Value {
        Value::Array(self.items.clone())
    }

    fn search_text(&self) -> String {
        self.search_text.clone()
    }

    fn current_row(&self) -> i32 {
        self.current_row
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn project_dir(&self) -> String {
        self.project_dir.clone()
    }

    fn editor_focused(&self) -> bool {
        self.editor_focused
    }

    fn set_visible(&mut self, v: bool) {
        if self.visible != v {
            self.visible = v;
            self.visible_changed();
        }
    }

    fn set_project_dir(&mut self, dir: String) {
        if self.project_dir != dir {
            self.project_dir = dir;
            self.project_dir_changed();
            if !self.search_text.starts_with('>') {
                self.repopulate();
            }
        }
    }

    fn set_editor_focused(&mut self, focused: bool) {
        if self.editor_focused == focused {
            return;
        }
        self.editor_focused = focused;
        self.editor_focused_changed();
        if self.search_text.starts_with('>') {
            self.repopulate();
        }
    }

    fn set_search_text(&mut self, text: String) {
        if self.search_text == text {
            return;
        }
        self.search_text = text;
        self.search_text_changed();
        self.repopulate();
    }

    fn set_current_row(&mut self, row: i32) {
        let clamped = if self.items.is_empty() {
            -1
        } else {
            row.clamp(0, self.items.len() as i32 - 1)
        };
        if self.current_row != clamped {
            self.current_row = clamped;
            self.current_row_changed();
        }
    }

    #[qslot]
    fn open_command_palette(&mut self) {
        self.search_text = ">".into();
        self.search_text_changed();
        self.repopulate();
        self.set_visible(true);
    }

    #[qslot]
    fn open_jump_to_reference(&mut self) {
        self.search_text = String::new();
        self.search_text_changed();
        self.repopulate();
        self.set_visible(true);
    }

    #[qslot]
    fn set_circuit_items(&mut self, items: Value) {
        if let Value::Array(arr) = items {
            self.circuit_items = arr;
            if !self.search_text.starts_with('>') {
                self.repopulate();
            }
        }
    }

    #[qslot]
    fn move_selection(&mut self, delta: i32) {
        if self.items.is_empty() {
            self.set_current_row(-1);
            return;
        }
        let len = self.items.len() as i32;
        let next = (self.current_row + delta).rem_euclid(len);
        self.set_current_row(next);
    }

    #[qslot]
    fn toggle_palette_prefix(&mut self, palette: bool) {
        if palette && !self.search_text.starts_with('>') {
            let s = format!(">{}", self.search_text);
            self.set_search_text(s);
        } else if !palette && self.search_text.starts_with('>') {
            let s = self.search_text.trim_start_matches('>').to_string();
            self.set_search_text(s);
        }
    }

    #[qslot]
    fn accept(&mut self) {
        self.set_visible(false);
    }

    #[qslot]
    fn reject(&mut self) {
        self.set_visible(false);
    }

    #[qslot]
    fn activate_current(&mut self) {
        if self.current_row < 0 || self.current_row as usize >= self.items.len() {
            return;
        }
        let item = &self.items[self.current_row as usize];
        let typ = item["type"].as_str().unwrap_or_default().to_string();
        let data = item["data"].as_str().unwrap_or_default().to_string();
        let comp_type = item["compType"].as_str().unwrap_or_default().to_string();
        self.set_visible(false);
        self.execute_item(typ, data, comp_type);
    }

    #[qslot]
    fn set_shortcut(&mut self, id: String, shortcut: String) {
        cs_engine::settings::edit(|s| {
            if shortcut.is_empty() {
                s.shortcuts.remove(&id);
            } else {
                s.shortcuts.insert(id, shortcut);
            }
        });
        self.shortcuts_updated();
        self.repopulate();
    }

    fn repopulate(&mut self) {
        self.items.clear();
        if let Some(rest) = self.search_text.strip_prefix('>') {
            let filter = rest.trim().to_lowercase();
            self.populate_command_palette(&filter);
        } else {
            let filter = self.search_text.trim().to_lowercase();
            self.populate_jump_to_reference(&filter);
        }
        self.items_changed();
        self.set_current_row(if self.items.is_empty() { -1 } else { 0 });
    }

    fn populate_command_palette(&mut self, filter: &str) {
        self.items.clear();
        struct RankedEntry {
            rank: i32,
            item: Value,
        }
        let mut entries = Vec::new();

        let custom_shortcuts = cs_engine::settings::get().shortcuts;
        for action in palette_actions(self.editor_focused) {
            let effective_shortcut = custom_shortcuts
                .get(action.id)
                .map(|s| s.as_str())
                .unwrap_or(action.shortcut);
            let trans_label = cs_engine::i18n::tr(action.label);
            let label_rank = cs_engine::library::text_match_tier(action.label, filter)
                .or_else(|| cs_engine::library::text_match_tier(&trans_label, filter));
            let id_rank = cs_engine::library::text_match_tier(action.id, filter);
            let rank = match (label_rank, id_rank) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            if let Some(r) = rank {
                entries.push(RankedEntry {
                    rank: r,
                    item: json!({
                        "text": trans_label,
                        "shortcut": effective_shortcut,
                        "icon": action.icon,
                        "type": "action",
                        "data": action.id,
                        "compType": "",
                    }),
                });
            }
        }

        if !self.editor_focused {
            let insert_prefix = cs_engine::i18n::tr("Insert >");
            for item in self.lib.items() {
                if item.item_type != cs_engine::library::TYPE_COMPONENT {
                    continue;
                }
                let label = &item.caption;
                let trans_label = cs_engine::i18n::tr(label);
                let comp_type = &item.comp_type;
                let insert_text = format!("{insert_prefix} {trans_label}");

                let mut rank = item
                    .match_rank(filter)
                    .or_else(|| cs_engine::library::text_match_tier(&trans_label, filter));
                if rank.is_none() && !filter.is_empty() {
                    // Check if parent category matched
                    if let Some(parent_id) = item.parent {
                        if let Some(parent_item) = self.lib.item(parent_id) {
                            let trans_parent = cs_engine::i18n::tr(&parent_item.caption);
                            if cs_engine::library::text_match_tier(&parent_item.caption, filter)
                                .is_some()
                                || cs_engine::library::text_match_tier(&trans_parent, filter)
                                    .is_some()
                                || parent_item.caption.to_lowercase().contains(filter)
                                || trans_parent.to_lowercase().contains(filter)
                            {
                                rank = Some(8);
                            }
                        }
                    }
                }

                if let Some(r) = rank {
                    entries.push(RankedEntry {
                        rank: r,
                        item: json!({
                            "text": insert_text,
                            "shortcut": "",
                            "icon": "developer_board",
                            "type": "component",
                            "data": label,
                            "compType": comp_type,
                        }),
                    });
                }
            }
        }

        entries.sort_by_key(|e| e.rank);
        for e in entries {
            self.items.push(e.item);
        }
    }

    fn populate_jump_to_reference(&mut self, filter: &str) {
        self.items.clear();

        struct RankedEntry {
            rank: i32,
            item: Value,
        }

        // 1. Existing components on canvas
        let mut comp_entries = Vec::new();
        let focus_prefix = cs_engine::i18n::tr("Focus >");
        for comp in &self.circuit_items {
            let uid = comp["uid"].as_str().unwrap_or_default();
            let label = comp["label"].as_str().unwrap_or(uid);
            let kind = comp["kind"].as_str().unwrap_or_default();
            let display_label = if label.is_empty() { uid } else { label };

            if filter.is_empty() {
                let text = format!("{focus_prefix} {display_label}");
                let tooltip = if display_label != uid {
                    format!("{} ({})", display_label, uid)
                } else {
                    uid.to_string()
                };
                comp_entries.push(RankedEntry {
                    rank: 0,
                    item: json!({
                        "text": text,
                        "shortcut": "",
                        "icon": "developer_board",
                        "tooltip": tooltip,
                        "type": "jump_comp",
                        "data": uid,
                        "compType": kind,
                    }),
                });
            } else {
                let label_tier = cs_engine::library::text_match_tier(display_label, filter);
                let uid_tier = cs_engine::library::text_match_tier(uid, filter);
                let kind_tier = cs_engine::library::text_match_tier(kind, filter);
                let best_tier = [label_tier, uid_tier, kind_tier]
                    .into_iter()
                    .flatten()
                    .min();
                if let Some(tier) = best_tier {
                    let text = format!("{focus_prefix} {display_label}");
                    let tooltip = if display_label != uid {
                        format!("{} ({})", display_label, uid)
                    } else {
                        uid.to_string()
                    };
                    comp_entries.push(RankedEntry {
                        rank: tier,
                        item: json!({
                            "text": text,
                            "shortcut": "",
                            "icon": "developer_board",
                            "tooltip": tooltip,
                            "type": "jump_comp",
                            "data": uid,
                            "compType": kind,
                        }),
                    });
                }
            }
        }
        comp_entries.sort_by_key(|e| e.rank);
        for e in comp_entries {
            self.items.push(e.item);
        }

        // 2. Files in project directory
        if !self.project_dir.is_empty() {
            let dir_path = Path::new(&self.project_dir);
            if dir_path.is_dir() {
                let mut files = Vec::new();
                scan_dir_files(dir_path, &mut files, 200);

                for fpath in files {
                    let file_name = fpath
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    let rel_path = fpath
                        .strip_prefix(dir_path)
                        .ok()
                        .and_then(|p| p.to_str())
                        .unwrap_or(file_name);
                    let full_str = fpath.to_string_lossy().to_string();

                    if filter.is_empty()
                        || file_name.to_lowercase().contains(filter)
                        || rel_path.to_lowercase().contains(filter)
                    {
                        let ext = fpath
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let icon = match ext.as_str() {
                            "circ1" | "sim1" | "sim2" | "circ" => "schema",
                            "c" | "cpp" | "cc" | "h" | "hpp" | "rs" | "ino" | "py" | "asm"
                            | "s" => "code",
                            "txt" | "md" | "json" | "xml" | "csv" | "hex" | "bin" => "description",
                            _ => "draft",
                        };

                        self.items.push(json!({
                            "text": rel_path,
                            "shortcut": "",
                            "icon": icon,
                            "tooltip": full_str,
                            "type": "jump_file",
                            "data": full_str,
                            "compType": "",
                        }));
                    }
                }
            }
        }

        // 3. Quick jumps
        let quick_jumps = [
            ("Jump to Side Panel", "view.sidePanel", "dock_to_left"),
            ("Jump to Code Editor", "view.editorPanel", "code"),
            ("Jump to Simulator Messages", "view.simLog", "terminal"),
            ("Jump to Compiler Messages", "view.compLog", "pest_control"),
            (
                "Add All Components to Canvas",
                "circ.addAllComponents",
                "grid_view",
            ),
        ];
        for (label, id, icon) in quick_jumps {
            let trans_label = cs_engine::i18n::tr(label);
            if filter.is_empty()
                || label.to_lowercase().contains(filter)
                || trans_label.to_lowercase().contains(filter)
            {
                self.items.push(json!({
                    "text": trans_label,
                    "shortcut": "",
                    "icon": icon,
                    "type": "action",
                    "data": id,
                    "compType": "",
                }));
            }
        }
    }
}

#[derive(Clone, Copy)]
struct PaletteAction {
    id: &'static str,
    label: &'static str,
    shortcut: &'static str,
    icon: &'static str,
}

#[derive(Clone, Copy)]
enum PaletteScope {
    Always,
    Editor,
    Circuit,
}

impl PaletteScope {
    fn includes(self, editor_focused: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Editor => editor_focused,
            Self::Circuit => !editor_focused,
        }
    }
}

fn palette_actions(editor_focused: bool) -> impl Iterator<Item = PaletteAction> {
    PALETTE_ACTIONS
        .iter()
        .copied()
        .filter(move |(scope, _)| scope.includes(editor_focused))
        .map(|(_, action)| action)
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

#[rustfmt::skip]
const PALETTE_ACTIONS: &[(PaletteScope, PaletteAction)] = &[
    (PaletteScope::Editor,  act("file.newFile", "New File", "Ctrl+N", "note_add")),
    (PaletteScope::Circuit, act("file.new", "New Circuit", "Ctrl+N", "note_add")),
    (PaletteScope::Always,  act("file.open", "Open...", "Ctrl+O", "folder_open")),
    (PaletteScope::Always,  act("file.save", "Save", "Ctrl+S", "save")),
    (PaletteScope::Always,  act("file.saveAs", "Save As...", "Ctrl+Shift+S", "save_as")),
    (PaletteScope::Editor,  act("file.saveFile", "Save File", "", "save")),
    (PaletteScope::Editor,  act("file.saveFileAs", "Save File As...", "", "save_as")),
    (PaletteScope::Circuit, act("file.saveCirc", "Save Circuit", "", "save")),
    (PaletteScope::Circuit, act("file.saveCircAs", "Save Circuit As...", "", "save_as")),
    (PaletteScope::Always,  act("file.saveAll", "Save All", "Ctrl+Alt+S", "save")),
    (PaletteScope::Editor,  act("file.closeFile", "Close File", "Ctrl+W", "close")),
    (PaletteScope::Always,  act("file.openFolder", "Open Project", "", "folder_open")),
    (PaletteScope::Always,  act("file.closeProject", "Close Project", "", "folder_off")),
    (PaletteScope::Circuit, act("file.saveImage", "Save Circuit as Image...", "", "image")),
    (PaletteScope::Always,  act("file.appSettings", "Application Settings...", "Ctrl+,", "settings")),
    (PaletteScope::Always,  act("edit.undo", "Undo", "Ctrl+Z", "undo")),
    (PaletteScope::Always,  act("edit.redo", "Redo", "Ctrl+Y", "redo")),
    (PaletteScope::Always,  act("edit.cut", "Cut", "Ctrl+X", "content_cut")),
    (PaletteScope::Always,  act("edit.copy", "Copy", "Ctrl+C", "content_copy")),
    (PaletteScope::Always,  act("edit.paste", "Paste", "Ctrl+V", "content_paste")),
    (PaletteScope::Always,  act("edit.selectAll", "Select All", "Ctrl+A", "select_all")),
    (PaletteScope::Editor,  act("edit.find", "Find", "Ctrl+F", "search")),
    (PaletteScope::Editor,  act("edit.format", "Format Document", "Ctrl+Shift+I", "format_align_left")),
    (PaletteScope::Circuit, act("edit.delete", "Delete Selection", "Delete", "delete")),
    (PaletteScope::Circuit, act("edit.rotateCw", "Rotate Clockwise", "Ctrl+R", "rotate_right")),
    (PaletteScope::Circuit, act("edit.rotateCcw", "Rotate Counter-Clockwise", "Ctrl+Shift+R", "rotate_left")),
    (PaletteScope::Circuit, act("view.zoomIn", "Zoom In", "Ctrl++", "zoom_in")),
    (PaletteScope::Circuit, act("view.zoomOut", "Zoom Out", "Ctrl+-", "zoom_out")),
    (PaletteScope::Circuit, act("view.zoomFit", "Fit to View", "Ctrl+9", "fit_screen")),
    (PaletteScope::Circuit, act("view.zoomOne", "Actual Size (1:1)", "Ctrl+0", "zoom_out_map")),
    (PaletteScope::Circuit, act("view.showGrid", "Toggle Grid", "G", "grid_4x4")),
    (PaletteScope::Always,  act("view.sidePanel", "Toggle Side Panel", "Ctrl+B", "dock_to_left")),
    (PaletteScope::Always,  act("view.editorPanel", "Toggle Editor Panel", "Ctrl+E", "code")),
    (PaletteScope::Always,  act("view.toggleActivePanel", "Toggle Active Panel", "Ctrl+J", "dock_to_left")),
    (PaletteScope::Always,  act("view.libManager", "Library Manager", "", "local_library")),
    (PaletteScope::Always,  act("circ.power", "Start / Stop Simulation", "F8", "power_settings_new")),
    (PaletteScope::Always,  act("circ.pause", "Pause / Resume Simulation", "F9", "pause")),
    (PaletteScope::Always,  act("circ.step", "Step Simulation", "", "skip_next")),
    (PaletteScope::Circuit, act("circ.settings", "Circuit Properties...", "", "tune")),
    (PaletteScope::Circuit, act("circ.addAllComponents", "Add All Components", "", "grid_view")),
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
    (PaletteScope::Always,  act("debug.toggleRepaintOverlay", "Toggle Canvas Repaint Debug Overlay", "Ctrl+Alt+D", "bug_report")),
    (PaletteScope::Always,  act("debug.toggleComponentRects", "Show Component Rect", "Ctrl+Alt+C", "crop_free")),
];

fn scan_dir_files(dir: &Path, files: &mut Vec<PathBuf>, max: usize) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if !name.starts_with('.')
                    && name != "target"
                    && name != "build"
                    && name != "node_modules"
                    && name != ".git"
                {
                    scan_dir_files(&p, files, max);
                }
            } else if p.is_file() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if !name.starts_with('.') {
                    files.push(p);
                    if files.len() >= max {
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_palette_lists_electrolytic_capacitor() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("electrolytic");
        let found = cc.items.iter().any(|item| {
            item["text"] == "Insert > Electrolytic Capacitor"
                && item["compType"] == "ElCapacitor"
                && item["data"] == "Electrolytic Capacitor"
                && item["type"] == "component"
        });
        assert!(
            found,
            "Electrolytic Capacitor not found in command palette items: {:?}",
            cc.items
        );
    }

    #[test]
    fn command_palette_lists_all_library_components() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("");
        let comp_count = cc
            .items
            .iter()
            .filter(|item| item["type"] == "component")
            .count();
        // Standard library has 60+ components
        assert!(
            comp_count >= 50,
            "Expected at least 50 components, found {}",
            comp_count
        );

        let names_to_check = [
            "Resistor",
            "Capacitor",
            "Electrolytic Capacitor",
            "Variable Capacitor",
            "Inductor",
            "Diode",
            "Zener Diode",
            "BJT",
            "MOSFET",
            "JFET",
            "OpAmp",
            "Comparator",
            "Voltage Regulator",
            "LED",
            "RGB LED",
            "7-Segment Display",
            "And Gate",
            "Or Gate",
            "Flip-Flop D",
            "Voltmeter",
            "Ampmeter",
            "Oscilloscope",
            "Logic Analyzer",
            "Probe",
        ];
        for name in names_to_check {
            let label = format!("Insert > {}", name);
            let found = cc.items.iter().any(|item| {
                item["text"].as_str().map(|t| t.to_lowercase()) == Some(label.to_lowercase())
            });
            assert!(
                found,
                "Component '{}' should be listed in command palette",
                name
            );
        }
    }

    #[test]
    fn command_palette_searches_by_comp_type_or_label() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("elcapacitor");
        assert!(
            cc.items
                .iter()
                .any(|item| item["text"] == "Insert > Electrolytic Capacitor")
        );

        cc.populate_command_palette("varcapacitor");
        assert!(
            cc.items
                .iter()
                .any(|item| item["text"] == "Insert > Variable Capacitor")
        );
    }

    #[test]
    fn command_palette_alias_search_transistor_finds_bjt_mosfet_jfet() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("transistor");
        let texts: Vec<&str> = cc
            .items
            .iter()
            .filter_map(|item| item["text"].as_str())
            .collect();
        assert!(
            texts.contains(&"Insert > BJT"),
            "Expected 'Insert > BJT' when searching 'transistor', found: {:?}",
            texts
        );
        assert!(
            texts.contains(&"Insert > MOSFET"),
            "Expected 'Insert > MOSFET' when searching 'transistor', found: {:?}",
            texts
        );
        assert!(
            texts.contains(&"Insert > JFET"),
            "Expected 'Insert > JFET' when searching 'transistor', found: {:?}",
            texts
        );
    }

    #[test]
    fn command_palette_exact_matches_rank_highest() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("bjt");
        assert_eq!(
            cc.items[0]["text"], "Insert > BJT",
            "Exact match on BJT must be top result, found: {:?}",
            cc.items
        );

        cc.populate_command_palette("undo");
        assert_eq!(
            cc.items[0]["text"], "Undo",
            "Exact match on action 'Undo' must be top result, found: {:?}",
            cc.items
        );
    }

    #[test]
    fn command_palette_alias_keywords() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("npn");
        assert!(cc.items.iter().any(|i| i["text"] == "Insert > BJT"));

        cc.populate_command_palette("pot");
        assert!(
            cc.items
                .iter()
                .any(|i| i["text"] == "Insert > Potentiometer")
        );

        cc.populate_command_palette("timer");
        assert!(cc.items.iter().any(|i| i["text"] == "Insert > LM555 Timer"));

        cc.populate_command_palette("gnd");
        assert!(
            cc.items
                .iter()
                .any(|i| i["text"] == "Insert > Ground (0 V)")
        );

        cc.populate_command_palette("neopixel");
        assert!(
            cc.items
                .iter()
                .any(|i| i["text"] == "Insert > WS2812 RGB LED")
        );
    }

    #[test]
    fn jump_to_reference_lists_and_filters_circuit_components() {
        let mut cc = CommandCenter::default();
        let comps = json!([
            { "uid": "Resistor-1", "label": "R1", "kind": "Resistor" },
            { "uid": "Capacitor-1", "label": "C_filter", "kind": "Capacitor" },
            { "uid": "Led-1", "label": "Led-1", "kind": "Led" },
            { "uid": "VoltReg-1", "label": "7805", "kind": "VoltReg" },
        ]);
        if let Value::Array(arr) = comps {
            cc.circuit_items = arr;
        }

        // 1. Empty filter lists all components
        cc.populate_jump_to_reference("");
        let comp_items: Vec<_> = cc
            .items
            .iter()
            .filter(|i| i["type"] == "jump_comp")
            .collect();
        assert_eq!(comp_items.len(), 4);
        assert_eq!(comp_items[0]["text"], "Focus > R1");
        assert_eq!(comp_items[0]["data"], "Resistor-1");
        assert_eq!(comp_items[1]["text"], "Focus > C_filter");
        assert_eq!(comp_items[1]["data"], "Capacitor-1");
        assert_eq!(comp_items[2]["text"], "Focus > Led-1");
        assert_eq!(comp_items[3]["text"], "Focus > 7805");

        // 2. Filter by label
        cc.populate_jump_to_reference("r1");
        assert_eq!(cc.items[0]["text"], "Focus > R1");
        assert_eq!(cc.items[0]["data"], "Resistor-1");
        assert_eq!(cc.items[0]["type"], "jump_comp");

        // 3. Filter by uid
        cc.populate_jump_to_reference("capacitor");
        assert_eq!(cc.items[0]["text"], "Focus > C_filter");
        assert_eq!(cc.items[0]["data"], "Capacitor-1");

        // 4. Filter by kind
        cc.populate_jump_to_reference("voltreg");
        assert_eq!(cc.items[0]["text"], "Focus > 7805");
        assert_eq!(cc.items[0]["data"], "VoltReg-1");
    }

    #[test]
    fn jump_to_reference_ranks_exact_matches_first() {
        let mut cc = CommandCenter::default();
        let comps = json!([
            { "uid": "Resistor-10", "label": "R10", "kind": "Resistor" },
            { "uid": "Resistor-1", "label": "R1", "kind": "Resistor" },
            { "uid": "Resistor-11", "label": "R11", "kind": "Resistor" },
        ]);
        if let Value::Array(arr) = comps {
            cc.circuit_items = arr;
        }

        cc.populate_jump_to_reference("r1");
        assert_eq!(
            cc.items[0]["text"], "Focus > R1",
            "Exact match on R1 must rank first, got: {:?}",
            cc.items
        );
    }

    #[test]
    fn command_palette_finds_repaint_debug_overlay() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("repaint");
        let item = cc
            .items
            .iter()
            .find(|i| i["data"] == "debug.toggleRepaintOverlay");
        assert!(
            item.is_some(),
            "Must find debug.toggleRepaintOverlay for 'repaint'"
        );
        let item = item.unwrap();
        assert_eq!(item["text"], "Toggle Canvas Repaint Debug Overlay");
        assert_eq!(item["shortcut"], "Ctrl+Alt+D");

        cc.populate_command_palette("overlay");
        let item2 = cc
            .items
            .iter()
            .find(|i| i["data"] == "debug.toggleRepaintOverlay");
        assert!(
            item2.is_some(),
            "Must find debug.toggleRepaintOverlay for 'overlay'"
        );
    }

    #[test]
    fn command_palette_finds_show_component_rect() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("rect");
        let item = cc
            .items
            .iter()
            .find(|i| i["data"] == "debug.toggleComponentRects");
        assert!(
            item.is_some(),
            "Must find debug.toggleComponentRects for 'rect'"
        );
        let item = item.unwrap();
        assert_eq!(item["text"], "Show Component Rect");
        assert_eq!(item["shortcut"], "Ctrl+Alt+C");

        cc.populate_command_palette("component");
        let item2 = cc
            .items
            .iter()
            .find(|i| i["data"] == "debug.toggleComponentRects");
        assert!(
            item2.is_some(),
            "Must find debug.toggleComponentRects for 'component'"
        );
    }

    #[test]
    fn command_palette_swaps_file_actions_when_editor_focused() {
        let mut cc = CommandCenter::default();
        cc.populate_command_palette("");
        let circuit_save = cc.items.iter().find(|i| i["data"] == "file.save");
        assert!(circuit_save.is_some(), "Circuit palette must list Save");
        assert_eq!(circuit_save.unwrap()["text"], "Save");
        assert_eq!(circuit_save.unwrap()["shortcut"], "Ctrl+S");
        assert!(
            cc.items
                .iter()
                .any(|i| i["data"] == "file.saveCirc" && i["shortcut"] == ""),
            "Circuit palette must list Save Circuit without shortcut"
        );
        assert!(
            cc.items
                .iter()
                .any(|i| i["data"] == "file.new" && i["text"] == "New Circuit"),
            "Circuit palette must list New Circuit"
        );
        assert!(
            cc.items.iter().any(|i| i["type"] == "component"),
            "Circuit palette must list insert-component entries"
        );
        assert!(
            !cc.items
                .iter()
                .any(|i| i["data"] == "file.saveFile" || i["data"] == "file.closeFile"),
            "Circuit palette must not list editor file actions"
        );
        assert!(
            cc.items
                .iter()
                .any(|i| i["data"] == "file.saveAll" && i["shortcut"] == "Ctrl+Alt+S"),
            "Circuit palette must list Save All"
        );

        cc.editor_focused = true;
        cc.populate_command_palette("");
        let generic_save = cc.items.iter().find(|i| i["data"] == "file.save");
        assert!(generic_save.is_some(), "Editor palette must list Save");
        assert_eq!(generic_save.unwrap()["text"], "Save");
        assert_eq!(generic_save.unwrap()["shortcut"], "Ctrl+S");

        let file_save = cc.items.iter().find(|i| i["data"] == "file.saveFile");
        assert!(file_save.is_some(), "Editor palette must list Save File");
        assert_eq!(file_save.unwrap()["text"], "Save File");
        assert_eq!(file_save.unwrap()["shortcut"], "");
        assert!(
            cc.items.iter().any(|i| i["data"] == "file.newFile"
                && i["text"] == "New File"
                && i["shortcut"] == "Ctrl+N"),
            "Editor palette must list New File with Ctrl+N"
        );
        let close = cc.items.iter().find(|i| i["data"] == "file.closeFile");
        assert!(close.is_some(), "Editor palette must list Close File");
        assert_eq!(close.unwrap()["text"], "Close File");
        assert_eq!(close.unwrap()["shortcut"], "Ctrl+W");
        assert!(
            cc.items.iter().any(|i| i["data"] == "edit.find"),
            "Editor palette must list Find"
        );
        assert!(
            cc.items.iter().any(|i| i["data"] == "edit.format"),
            "Editor palette must list Format Document"
        );
        assert!(
            !cc.items.iter().any(|i| i["type"] == "component"),
            "Editor palette must not list insert-component entries"
        );
        assert!(
            !cc.items
                .iter()
                .any(|i| i["data"] == "file.saveCirc" || i["data"] == "file.new"),
            "Editor palette must not list circuit file actions"
        );
        assert!(
            !cc.items
                .iter()
                .any(|i| i["data"] == "edit.rotateCw" || i["data"] == "view.zoomIn"),
            "Editor palette must not list canvas-only commands"
        );
        let save_all = cc.items.iter().find(|i| i["data"] == "file.saveAll");
        assert!(save_all.is_some(), "Editor palette must list Save All");
        assert_eq!(save_all.unwrap()["shortcut"], "Ctrl+Alt+S");

        assert!(
            cc.items.iter().any(|i| i["data"] == "sim.debug"
                && i["text"] == "Debug"
                && i["shortcut"] == "F12"),
            "Editor palette must list Debug with F12"
        );
        assert!(
            cc.items
                .iter()
                .any(|i| i["data"] == "sim.run" && i["text"] == "Run" && i["shortcut"] == "F5"),
            "Editor palette must list Run with F5"
        );

        cc.editor_focused = false;
        cc.populate_command_palette("");
        assert!(
            cc.items
                .iter()
                .any(|i| i["data"] == "sim.debug" && i["text"] == "Debug"),
            "Circuit palette must list Debug"
        );
        assert!(
            cc.items
                .iter()
                .any(|i| i["data"] == "sim.run" && i["text"] == "Run"),
            "Circuit palette must list Run"
        );
    }
}
