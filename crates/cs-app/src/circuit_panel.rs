//! Overlay chrome façade. Geometry lives in `cs_engine::overlays`.

use std::path::Path;

use crate::macos_host;
use crate::ui_icon::UiIcon;
use cs_engine::canvas::scene::ProgrammableDevice;
use cs_engine::overlays::{OverlayEntry, OverlaySnapshot, Overlays};
use cs_engine::subcircuit::SubcTreeNode;
use cs_engine::theme::{ColorId, ColorTheme};
use qtbridge::qobject;
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq)]
struct NavEntry {
    file_path: String,
    label: String,
}

#[derive(Clone, Debug)]
struct FlattenedNavRow {
    text: String,
    depth: i32,
    current: bool,
    file_chain: Vec<String>,
    label_chain: Vec<String>,
}

fn normalize_nav_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut s = path.replace('\\', "/");
    while let Some(idx) = s.get(1..).and_then(|rest| rest.find("//")) {
        s.replace_range(idx + 1..idx + 3, "/");
    }
    #[cfg(windows)]
    {
        s = s.to_ascii_lowercase();
    }
    s
}

fn nav_paths_equal(a: &str, b: &str) -> bool {
    if a.is_empty() && b.is_empty() {
        return true;
    }
    normalize_nav_path(a) == normalize_nav_path(b)
}

fn nav_chains_equal(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| nav_paths_equal(x, y))
}

fn display_circuit_label(path: &str, label: &str) -> String {
    if !label.is_empty() {
        return label.to_string();
    }
    let default_name = cs_engine::i18n::tr("New Circuit");
    if path.is_empty() {
        return default_name;
    }
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or(&default_name)
        .to_string()
}

fn find_subc_child_idx(nodes: &[SubcTreeNode], path: &str, label: &str) -> Option<usize> {
    if !label.is_empty() {
        if let Some(i) = nodes
            .iter()
            .position(|n| nav_paths_equal(&n.file_path, path) && n.label == label)
        {
            return Some(i);
        }
    }
    nodes
        .iter()
        .position(|n| !n.file_path.is_empty() && nav_paths_equal(&n.file_path, path))
}

fn children_at_stack<'a>(
    roots: &'a [SubcTreeNode],
    stack_tail: &[NavEntry],
) -> Option<&'a [SubcTreeNode]> {
    let mut cur = roots;
    for entry in stack_tail {
        let idx = find_subc_child_idx(cur, &entry.file_path, &entry.label)?;
        cur = &cur[idx].children;
    }
    Some(cur)
}

fn ensure_stack_in_tree<'a>(
    roots: &'a mut Vec<SubcTreeNode>,
    stack_tail: &[NavEntry],
) -> Option<&'a mut SubcTreeNode> {
    let (head, rest) = stack_tail.split_first()?;
    let idx = match find_subc_child_idx(roots, &head.file_path, &head.label) {
        Some(i) => i,
        None => {
            roots.push(SubcTreeNode {
                label: head.label.clone(),
                file_path: head.file_path.clone(),
                children: Vec::new(),
            });
            roots.len() - 1
        }
    };
    if rest.is_empty() {
        Some(&mut roots[idx])
    } else {
        ensure_stack_in_tree(&mut roots[idx].children, rest)
    }
}

pub struct CircuitPanel {
    overlays: Overlays,
    coords_x: i32,
    coords_y: i32,
    zoom_text: String,
    message_text: String,
    message_bg: String,
    message_color: String,
    warnings_text: String,
    warnings_crashed: bool,
    overload_panel_visible: bool,
    canvas_overflow_text: String,
    canvas_overflow_panel_visible: bool,
    zoom_menu_open: bool,
    running: bool,
    paused: bool,
    sim_info_enabled: bool,
    subc_stack: Vec<NavEntry>,
    root_subc_children: Vec<SubcTreeNode>,
    subc_popup_open: bool,
    devices: Vec<ProgrammableDevice>,
    active_device_id: Option<String>,
    device_popup_open: bool,
}

impl Default for CircuitPanel {
    fn default() -> Self {
        let dark = macos_host::current_theme_dark();
        Self {
            overlays: Overlays::new(),
            coords_x: 0,
            coords_y: 0,
            zoom_text: "100%".into(),
            message_text: String::new(),
            message_bg: ColorTheme::get_hex(ColorId::MsgOkBg, dark),
            message_color: ColorTheme::get_hex(ColorId::MsgOkText, dark),
            warnings_text: String::new(),
            warnings_crashed: false,
            overload_panel_visible: false,
            canvas_overflow_text: String::new(),
            canvas_overflow_panel_visible: false,
            zoom_menu_open: false,
            running: false,
            paused: false,
            sim_info_enabled: true,
            subc_stack: Vec::new(),
            root_subc_children: Vec::new(),
            subc_popup_open: false,
            devices: Vec::new(),
            active_device_id: None,
            device_popup_open: false,
        }
    }
}

fn snapshot_json(s: &OverlaySnapshot) -> Value {
    let mut map = serde_json::Map::new();
    for (k, v) in &s.entries {
        let val = match v {
            OverlayEntry::Rect(r) => json!({ "x": r.x, "y": r.y, "w": r.w, "h": r.h }),
            OverlayEntry::Flag(b) => json!(b),
            OverlayEntry::Int(i) => json!(i),
        };
        map.insert(k.clone(), val);
    }
    Value::Object(map)
}

#[qobject(Singleton, ConvertToCamelCase)]
impl CircuitPanel {
    qproperty!(
        "overlayRects",
        Read = overlay_rects,
        Notify = overlay_layout_changed
    );
    qproperty!(
        "toolbarItems",
        Read = toolbar_items,
        Notify = toolbar_changed
    );
    qproperty!("messageText", Read = message_text, Notify = toolbar_changed);
    qproperty!("messageBg", Read = message_bg, Notify = toolbar_changed);
    qproperty!(
        "messageColor",
        Read = message_color,
        Notify = toolbar_changed
    );
    qproperty!("coordsX", Read = coords_x, Notify = coords_changed);
    qproperty!("coordsY", Read = coords_y, Notify = coords_changed);
    qproperty!("zoomText", Read = zoom_text, Notify = toolbar_changed);
    qproperty!(
        "warningsText",
        Read = warnings_text,
        Notify = toolbar_changed
    );
    qproperty!(
        "warningsCrashed",
        Read = warnings_crashed,
        Notify = toolbar_changed
    );
    qproperty!(
        "overloadPanelVisible",
        Read = overload_panel_visible,
        Notify = overload_panel_visible_changed
    );
    qproperty!(
        "canvasOverflowText",
        Read = canvas_overflow_text,
        Notify = toolbar_changed
    );
    qproperty!(
        "canvasOverflowPanelVisible",
        Read = canvas_overflow_panel_visible,
        Notify = canvas_overflow_panel_visible_changed
    );
    qproperty!(
        "zoomMenuOpen",
        Read = zoom_menu_open,
        Notify = zoom_menu_open_changed
    );
    qproperty!(
        "infoVisible",
        Read = info_visible,
        Notify = info_visible_changed
    );
    qproperty!("gripIcon", Read = grip_icon, Constant);
    qproperty!("subcNav", Read = subc_nav, Notify = subc_nav_changed);
    qproperty!("deviceSel", Read = device_sel, Notify = device_sel_changed);
    qproperty!("simRunning", Read = sim_running, Notify = toolbar_changed);
    qproperty!("simPaused", Read = sim_paused, Notify = toolbar_changed);

    #[qsignal]
    fn overlay_layout_changed(&mut self);
    #[qsignal]
    fn toolbar_changed(&mut self);
    #[qsignal]
    fn subc_nav_changed(&mut self);
    #[qsignal]
    fn device_sel_changed(&mut self);
    #[qsignal]
    fn coords_changed(&mut self);
    #[qsignal]
    fn info_visible_changed(&mut self);
    #[qsignal]
    fn overload_panel_visible_changed(&mut self);
    #[qsignal]
    fn canvas_overflow_panel_visible_changed(&mut self);
    #[qsignal]
    fn zoom_menu_open_changed(&mut self);
    #[qsignal]
    fn active_device_changed(&mut self, id: String);
    #[qsignal]
    fn load_circuit_requested(&mut self, path: String);
    #[qsignal]
    fn action(&mut self, id: String);

    fn message_text(&self) -> String {
        self.message_text.clone()
    }
    fn message_bg(&self) -> String {
        self.message_bg.clone()
    }
    fn message_color(&self) -> String {
        self.message_color.clone()
    }
    fn coords_x(&self) -> i32 {
        self.coords_x
    }
    fn coords_y(&self) -> i32 {
        self.coords_y
    }
    fn zoom_text(&self) -> String {
        self.zoom_text.clone()
    }
    fn warnings_text(&self) -> String {
        self.warnings_text.clone()
    }
    fn warnings_crashed(&self) -> bool {
        self.warnings_crashed
    }
    fn canvas_overflow_text(&self) -> String {
        self.canvas_overflow_text.clone()
    }
    fn zoom_menu_open(&self) -> bool {
        self.zoom_menu_open
    }
    fn sim_running(&self) -> bool {
        self.running
    }
    fn sim_paused(&self) -> bool {
        self.paused
    }

    fn overlay_rects(&self) -> Value {
        snapshot_json(self.overlays.snapshot())
    }

    fn info_visible(&self) -> bool {
        self.overlays.info_shown()
    }

    fn overload_panel_visible(&self) -> bool {
        self.overload_panel_visible
    }

    fn canvas_overflow_panel_visible(&self) -> bool {
        self.canvas_overflow_panel_visible
    }

    fn grip_icon(&self) -> String {
        String::new()
    }

    fn subc_nav_visible(&self) -> bool {
        self.subc_stack.len() > 1 || !self.root_subc_children.is_empty()
    }

    fn subc_nav(&self) -> Value {
        let visible = self.subc_nav_visible();
        let default_circuit_name = cs_engine::i18n::tr("New Circuit");
        let button_text = self
            .subc_stack
            .last()
            .map(|e| e.label.clone())
            .unwrap_or_else(|| default_circuit_name);
        let button_tooltip = if self.subc_stack.is_empty() {
            cs_engine::i18n::tr("Subcircuit Navigator")
        } else {
            self.subc_stack
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>()
                .join(" / ")
        };

        let rows = self.build_subc_rows();
        let rows_json: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "text": r.text,
                    "depth": r.depth,
                    "current": r.current,
                })
            })
            .collect();

        json!({
            "visible": visible,
            "popupOpen": self.subc_popup_open,
            "buttonText": button_text,
            "buttonTooltip": button_tooltip,
            "iconLigature": UiIcon::AccountTree.as_str(),
            "iconUri": "",
            "rows": rows_json,
        })
    }

    fn build_subc_rows(&self) -> Vec<FlattenedNavRow> {
        let mut rows = Vec::new();
        let root_entry = self.subc_stack.first().cloned().unwrap_or(NavEntry {
            file_path: String::new(),
            label: cs_engine::i18n::tr("New Circuit"),
        });
        let current_file_chain: Vec<String> = self
            .subc_stack
            .iter()
            .map(|e| e.file_path.clone())
            .collect();
        let current_label_chain: Vec<String> =
            self.subc_stack.iter().map(|e| e.label.clone()).collect();
        let root_file_chain = vec![root_entry.file_path.clone()];
        let root_label_chain = vec![root_entry.label.clone()];

        rows.push(FlattenedNavRow {
            text: root_entry.label.clone(),
            depth: 0,
            current: self.subc_stack.len() <= 1,
            file_chain: root_file_chain.clone(),
            label_chain: root_label_chain.clone(),
        });

        self.flatten_subc_nodes(
            &self.root_subc_children,
            1,
            &root_file_chain,
            &root_label_chain,
            &mut rows,
        );

        if self.subc_stack.len() > 1 {
            let exact_idx = rows.iter().position(|r| {
                nav_chains_equal(&r.file_chain, &current_file_chain)
                    && r.label_chain == current_label_chain
            });
            let matched_idx = exact_idx.or_else(|| {
                rows.iter()
                    .position(|r| nav_chains_equal(&r.file_chain, &current_file_chain))
            });
            if let Some(idx) = matched_idx {
                rows[idx].current = true;
            } else {
                // Defensive fallback: splice any active stack entries missing from the tree
                // so the current hierarchy is always visible and navigable in the dropdown.
                let mut prefix_len = 1usize;
                let mut insert_after = 0usize;
                for len in (1..self.subc_stack.len()).rev() {
                    if let Some(pos) = rows
                        .iter()
                        .position(|r| nav_chains_equal(&r.file_chain, &current_file_chain[..len]))
                    {
                        prefix_len = len;
                        insert_after = pos;
                        break;
                    }
                }
                let mut extra = Vec::new();
                for len in (prefix_len + 1)..=self.subc_stack.len() {
                    let entry = &self.subc_stack[len - 1];
                    extra.push(FlattenedNavRow {
                        text: entry.label.clone(),
                        depth: (len - 1) as i32,
                        current: len == self.subc_stack.len(),
                        file_chain: current_file_chain[..len].to_vec(),
                        label_chain: current_label_chain[..len].to_vec(),
                    });
                }
                let insert_pos = insert_after + 1;
                rows.splice(insert_pos..insert_pos, extra);
            }
        }

        rows
    }

    fn flatten_subc_nodes(
        &self,
        nodes: &[SubcTreeNode],
        depth: i32,
        file_chain: &[String],
        label_chain: &[String],
        out: &mut Vec<FlattenedNavRow>,
    ) {
        for node in nodes {
            let mut child_file_chain = file_chain.to_vec();
            child_file_chain.push(node.file_path.clone());
            let mut child_label_chain = label_chain.to_vec();
            child_label_chain.push(node.label.clone());

            out.push(FlattenedNavRow {
                text: node.label.clone(),
                depth,
                current: false,
                file_chain: child_file_chain.clone(),
                label_chain: child_label_chain.clone(),
            });

            self.flatten_subc_nodes(
                &node.children,
                depth + 1,
                &child_file_chain,
                &child_label_chain,
                out,
            );
        }
    }

    fn device_sel_visible(&self) -> bool {
        !self.devices.is_empty()
    }

    fn device_sel(&self) -> Value {
        let visible = self.device_sel_visible();
        let active = self.devices.iter().find(|d| d.is_active);
        let button_text = if let Some(d) = active {
            d.label.clone()
        } else {
            cs_engine::i18n::tr("No device")
        };
        let button_tooltip = if let Some(d) = active {
            let prefix =
                cs_engine::i18n::tr("Active device: firmware uploads and debugging go here");
            format!("{prefix}\n{}", d.display_text)
        } else {
            cs_engine::i18n::tr("No programmable device selected")
        };
        let rows_json: Vec<Value> = self
            .devices
            .iter()
            .map(|d| {
                json!({
                    "text": d.display_text,
                    "depth": 0,
                    "current": d.is_active,
                })
            })
            .collect();

        json!({
            "visible": visible,
            "popupOpen": self.device_popup_open,
            "buttonText": button_text,
            "buttonTooltip": button_tooltip,
            "iconLigature": UiIcon::Memory.as_str(),
            "iconUri": "",
            "rows": rows_json,
        })
    }

    fn toolbar_items(&self) -> Value {
        let subc_vis = self.subc_nav_visible();
        let dev_vis = self.device_sel_visible();
        let debug_vis = cs_engine::debug::DebugSession::global().is_active();
        let power_text = if self.running {
            cs_engine::i18n::tr("Stop Simulation")
        } else {
            cs_engine::i18n::tr("Start Simulation")
        };
        let pause_text = if self.paused {
            cs_engine::i18n::tr("Resume Simulation")
        } else {
            cs_engine::i18n::tr("Pause Simulation")
        };
        let mut items = vec![
            widget("subcNav", subc_vis),
            separator(subc_vis),
            widget("deviceSel", dev_vis),
            separator(dev_vis),
            action(
                "zoomIn",
                &cs_engine::i18n::tr("Zoom In"),
                UiIcon::ZoomIn,
                true,
                false,
                false,
            ),
            action(
                "zoomOut",
                &cs_engine::i18n::tr("Zoom Out"),
                UiIcon::ZoomOut,
                true,
                false,
                false,
            ),
            widget("zoom", true),
            separator(true),
            action(
                "power",
                &power_text,
                if self.running {
                    UiIcon::Stop
                } else {
                    UiIcon::PlayArrow
                },
                true,
                false,
                false,
            ),
            action(
                "pause",
                &pause_text,
                UiIcon::Pause,
                self.running,
                true,
                self.paused,
            ),
            separator(false),
            action(
                "step",
                &cs_engine::i18n::tr("Step Debugger"),
                UiIcon::StepInto,
                debug_vis,
                false,
                false,
            ),
            action(
                "stepOver",
                &cs_engine::i18n::tr("Step Over"),
                UiIcon::StepOver,
                debug_vis,
                false,
                false,
            ),
            action(
                "reset",
                &cs_engine::i18n::tr("Reset Debugger"),
                UiIcon::RestartAlt,
                debug_vis,
                false,
                false,
            ),
            action(
                "watch",
                &cs_engine::i18n::tr("Variables"),
                UiIcon::Visibility,
                debug_vis,
                false,
                false,
            ),
            action(
                "stop",
                &cs_engine::i18n::tr("Stop Debugger"),
                UiIcon::Stop,
                debug_vis,
                false,
                false,
            ),
            separator(false),
            widget("msg", !self.message_text.is_empty()),
            widget("coords", true),
            separator(true),
            widget("warnings", !self.warnings_text.is_empty()),
            widget("canvasOverflow", !self.canvas_overflow_text.is_empty()),
            separator(false),
            action(
                "settCircuit",
                &cs_engine::i18n::tr("Circuit Settings"),
                UiIcon::Tune,
                true,
                false,
                false,
            ),
            action(
                "simInfo",
                &cs_engine::i18n::tr("Simulation Info"),
                UiIcon::Info,
                self.running,
                true,
                self.sim_info_enabled,
            ),
        ];
        collapse_separators(&mut items);
        Value::Array(items)
    }

    #[qslot]
    fn set_view_size(&mut self, w: f64, h: f64) {
        if self
            .overlays
            .set_view_size(w.round() as i32, h.round() as i32)
        {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn set_top_inset(&mut self, inset: f64) {
        if self.overlays.set_top_inset(inset.round() as i32) {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn set_toolbar_size(&mut self, w: f64, h: f64) {
        if self
            .overlays
            .set_toolbar_size(w.round() as i32, h.round() as i32)
        {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn set_info_card_size(&mut self, w: f64, h: f64) {
        if self
            .overlays
            .set_info_card_size(w.round() as i32, h.round() as i32)
        {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn overlay_clicked(&mut self, id: String) {
        if self.overlays.overlay_clicked(&id) {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn toggle_panel(&mut self, id: String) {
        if self.overlays.toggle_panel(&id) {
            self.overlay_layout_changed();
            self.toolbar_changed();
        }
    }

    #[qslot]
    fn toggle_active_overlay(&mut self) -> bool {
        if self.overlays.toggle_active_overlay() {
            self.overlay_layout_changed();
            self.toolbar_changed();
            self.info_visible_changed();
            true
        } else {
            false
        }
    }

    #[qslot]
    fn collapse_active_overlay(&mut self) -> bool {
        self.toggle_active_overlay()
    }

    #[qslot]
    fn show_side_panel(&mut self) {
        if self.overlays.set_side_shown(true) {
            self.overlay_layout_changed();
        } else if self.overlays.overlay_clicked("sidePanel") {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn hide_side_panel(&mut self) {
        if self.overlays.set_side_shown(false) {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn show_editor(&mut self) {
        if self.overlays.set_editor_shown(true) {
            self.overlay_layout_changed();
        } else if self.overlays.overlay_clicked("editorPanel") {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn hide_editor(&mut self) {
        if self.overlays.set_editor_shown(false) {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn reset_overlay_pos(&mut self, id: String) {
        if self.overlays.reset_pos(&id) {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn reset_overlay_size(&mut self, id: String) {
        if self.overlays.reset_size(&id) {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn gesture_begin(&mut self, id: String, x: f64, y: f64) {
        self.overlays
            .gesture_begin(&id, x.round() as i32, y.round() as i32);
        self.overlay_layout_changed();
    }

    #[qslot]
    fn gesture_move(&mut self, x: f64, y: f64) {
        if self
            .overlays
            .gesture_move(x.round() as i32, y.round() as i32)
        {
            self.overlay_layout_changed();
        }
    }

    #[qslot]
    fn gesture_end(&mut self) {
        self.overlays.gesture_end();
        self.overlay_layout_changed();
    }

    #[qslot]
    fn toolbar_triggered(&mut self, index: i32) {
        if self.overlays.overlay_clicked("toolbar") {
            self.overlay_layout_changed();
        }
        let items = self.toolbar_items();
        let Some(Value::Array(arr)) = Some(items).filter(|v| v.is_array()) else {
            return;
        };
        let Some(item) = arr.get(index as usize) else {
            return;
        };
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
        match id {
            "power" => self.power_circ(),
            "pause" => self.pause_circ(),
            "simInfo" => self.toggle_info(),
            "step" => {
                cs_engine::debug::DebugSession::global().step_into();
            }
            "stepOver" => {
                cs_engine::debug::DebugSession::global().step_over();
            }
            "reset" => {
                cs_engine::debug::DebugSession::global().stop();
                self.toolbar_changed();
            }
            "stop" => {
                cs_engine::debug::DebugSession::global().stop();
                self.toolbar_changed();
            }
            other if !other.is_empty() => self.action(other.to_string()),
            _ => {}
        }
    }

    #[qslot]
    fn show_zoom_menu(&mut self, _x: f64, _y: f64) {
        self.zoom_menu_open = true;
        self.zoom_menu_open_changed();
    }

    #[qslot]
    fn zoom_menu_closed(&mut self) {
        if self.zoom_menu_open {
            self.zoom_menu_open = false;
            self.zoom_menu_open_changed();
        }
    }

    #[qslot]
    fn show_overloads(&mut self, _x: f64, _y: f64) {}

    #[qslot]
    fn set_warnings(&mut self, text: String, crashed: bool) {
        if self.warnings_text == text && self.warnings_crashed == crashed {
            return;
        }
        self.warnings_text = text;
        self.warnings_crashed = crashed;
        self.toolbar_changed();
    }

    #[qslot]
    fn set_overload_panel_visible(&mut self, vis: bool) {
        if self.overload_panel_visible != vis {
            self.overload_panel_visible = vis;
            self.overload_panel_visible_changed();
        }
    }

    #[qslot]
    fn set_canvas_overflow(&mut self, text: String) {
        if self.canvas_overflow_text == text {
            return;
        }
        self.canvas_overflow_text = text;
        self.toolbar_changed();
    }

    #[qslot]
    fn set_canvas_overflow_panel_visible(&mut self, vis: bool) {
        if self.canvas_overflow_panel_visible != vis {
            self.canvas_overflow_panel_visible = vis;
            self.canvas_overflow_panel_visible_changed();
        }
    }

    fn set_info_hidden(&mut self) {
        self.sim_info_enabled = false;
        let _ = self.overlays.set_info_shown(false);
    }

    #[qslot]
    fn hide_info(&mut self) {
        self.set_info_hidden();
        self.info_visible_changed();
        self.overlay_layout_changed();
        self.toolbar_changed();
    }

    #[qslot]
    fn set_coords(&mut self, x: i32, y: i32) {
        if self.coords_x == x && self.coords_y == y {
            return;
        }
        self.coords_x = x;
        self.coords_y = y;
        self.coords_changed();
    }

    #[qslot]
    fn set_zoom_percent(&mut self, zoom: f64) {
        let text = format!("{}%", (zoom * 100.0).round() as i32);
        if self.zoom_text == text {
            return;
        }
        self.zoom_text = text;
        self.toolbar_changed();
    }

    #[qslot]
    fn set_message(&mut self, text: String, bg: String, color: String) {
        let changed = self.message_text != text
            || (!bg.is_empty() && self.message_bg != bg)
            || (!color.is_empty() && self.message_color != color);
        if !changed {
            return;
        }
        self.message_text = text;
        if !bg.is_empty() {
            self.message_bg = bg;
        }
        if !color.is_empty() {
            self.message_color = color;
        }
        self.toolbar_changed();
    }

    fn set_sim_running_state(&mut self, running: bool) {
        self.running = running;
        self.paused = false;
        if self.running {
            let _ = self.overlays.set_info_shown(self.sim_info_enabled);
            let _ = self.overlays.set_side_shown(true);
            let _ = self.overlays.overlay_clicked("sidePanel");
        } else {
            let _ = self.overlays.set_info_shown(false);
        }
    }

    #[qslot]
    fn reload_i18n(&mut self) {
        if self.running {
            let dark = macos_host::current_theme_dark();
            self.message_text = if self.paused {
                cs_engine::i18n::tr("Paused")
            } else {
                cs_engine::i18n::tr("Running")
            };
            let (bg_id, text_id) = if self.paused {
                (ColorId::MsgWarnBg, ColorId::MsgWarnText)
            } else {
                (ColorId::MsgOkBg, ColorId::MsgOkText)
            };
            self.message_bg = ColorTheme::get_hex(bg_id, dark);
            self.message_color = ColorTheme::get_hex(text_id, dark);
        }
        self.toolbar_changed();
        self.subc_nav_changed();
        self.device_sel_changed();
    }

    fn sync_sim_state_data(&mut self, running: bool, paused: bool) -> bool {
        if self.running == running
            && self.paused == paused
            && (!running || !self.message_text.is_empty())
        {
            return false;
        }
        // Opening the side panel and the info card belongs to a start or stop.
        // Pause and resume only change the banner.
        if self.running != running {
            self.set_sim_running_state(running);
        }
        self.paused = paused;
        let dark = macos_host::current_theme_dark();
        if !self.running {
            self.message_text.clear();
        } else if self.paused {
            self.message_text = cs_engine::i18n::tr("Paused");
            self.message_bg = ColorTheme::get_hex(ColorId::MsgWarnBg, dark);
            self.message_color = ColorTheme::get_hex(ColorId::MsgWarnText, dark);
        } else {
            self.message_text = cs_engine::i18n::tr("Running");
            self.message_bg = ColorTheme::get_hex(ColorId::MsgOkBg, dark);
            self.message_color = ColorTheme::get_hex(ColorId::MsgOkText, dark);
        }
        macos_host::set_sim_state(self.running, self.paused);
        true
    }

    #[qslot]
    fn sync_sim_state(&mut self, running: bool, paused: bool) {
        if self.sync_sim_state_data(running, paused) {
            self.info_visible_changed();
            self.overlay_layout_changed();
            self.toolbar_changed();
        }
    }

    #[qslot]
    fn power_circ(&mut self) {
        self.action("powerCirc".into());
    }

    #[cfg(test)]
    fn stop_circ_data(&mut self) -> bool {
        self.sync_sim_state_data(false, false)
    }

    #[qslot]
    fn stop_circ(&mut self) {
        self.action("powerOff".into());
        self.sync_sim_state(false, false);
    }

    #[qslot]
    fn pause_circ(&mut self) {
        self.action("pauseCirc".into());
    }

    #[qslot]
    fn install_native_chrome(&mut self) {
        macos_host::install_touchbar(self);
        macos_host::set_sim_state(self.running, self.paused);
    }

    fn select_subc_index(&mut self, index: usize) -> Option<String> {
        let rows = self.build_subc_rows();
        let row = rows.get(index)?;
        let target = row.file_chain.last()?.clone();
        // A non-root row with no resolved file path cannot be opened; keep current context.
        if row.depth > 0 && target.is_empty() {
            self.subc_popup_open = false;
            return None;
        }
        if row.current {
            self.subc_popup_open = false;
            return None;
        }
        self.subc_stack = row
            .file_chain
            .iter()
            .zip(&row.label_chain)
            .map(|(f, l)| NavEntry {
                file_path: f.clone(),
                label: l.clone(),
            })
            .collect();
        self.subc_popup_open = false;
        Some(target)
    }

    #[qslot]
    fn activate_subc_nav(&mut self, index: i32) {
        let was_vis = self.subc_nav_visible();
        if let Some(target) = self.select_subc_index(index as usize) {
            self.load_circuit_requested(target);
            self.subc_nav_changed();
            if self.subc_nav_visible() != was_vis {
                self.toolbar_changed();
            }
        }
    }

    #[qslot]
    fn activate_device(&mut self, index: i32) {
        if let Some(dev) = self.devices.get(index as usize) {
            let id = dev.id.clone();
            self.set_active_device(id);
        }
        self.set_device_popup_open(false);
    }

    #[qslot]
    fn set_active_device(&mut self, id: String) {
        self.active_device_id = Some(id.clone());
        for d in &mut self.devices {
            d.is_active = d.id == id || d.uid == id;
        }
        self.active_device_changed(id);
        self.device_sel_changed();
    }

    #[qslot]
    fn set_subc_popup_open(&mut self, open: bool) {
        self.subc_popup_open = open;
    }

    #[qslot]
    fn set_device_popup_open(&mut self, open: bool) {
        self.device_popup_open = open;
    }

    fn reset_subcircuits_data(&mut self, root_path: String, root_label: String, tree: Value) {
        let children: Vec<SubcTreeNode> = serde_json::from_value(tree).unwrap_or_default();
        let display_label = display_circuit_label(&root_path, &root_label);
        self.subc_stack = vec![NavEntry {
            file_path: root_path,
            label: display_label,
        }];
        self.root_subc_children = children;
    }

    #[qslot]
    fn reset_subcircuits(&mut self, root_path: String, root_label: String, tree: Value) {
        let was_vis = self.subc_nav_visible();
        self.reset_subcircuits_data(root_path, root_label, tree);
        self.subc_nav_changed();
        if self.subc_nav_visible() != was_vis {
            self.toolbar_changed();
        }
    }

    fn sync_subcircuits_data(
        &mut self,
        current_path: String,
        current_label: String,
        tree: Value,
    ) -> (bool, bool) {
        let was_vis = self.subc_nav_visible();
        let children: Vec<SubcTreeNode> = serde_json::from_value(tree).unwrap_or_default();
        let changed = if self.subc_stack.len() <= 1 {
            let display_label = display_circuit_label(&current_path, &current_label);
            let next_root = NavEntry {
                file_path: current_path,
                label: display_label,
            };
            let root_changed = self.subc_stack.first() != Some(&next_root);
            let tree_changed = self.root_subc_children != children;
            if root_changed {
                self.subc_stack = vec![next_root];
            }
            if tree_changed {
                self.root_subc_children = children;
            }
            root_changed || tree_changed
        } else if let Some(node) =
            ensure_stack_in_tree(&mut self.root_subc_children, &self.subc_stack[1..])
        {
            if node.children != children {
                node.children = children;
                true
            } else {
                false
            }
        } else {
            false
        };
        let vis_changed = self.subc_nav_visible() != was_vis;
        (changed, vis_changed)
    }

    #[qslot]
    fn sync_subcircuits(&mut self, current_path: String, current_label: String, tree: Value) {
        let (changed, vis_changed) = self.sync_subcircuits_data(current_path, current_label, tree);
        if changed {
            self.subc_nav_changed();
        }
        if vis_changed {
            self.toolbar_changed();
        }
    }

    fn navigate_into_subcircuit_data(&mut self, path: String, label: String) {
        let entry_label = if label.is_empty() {
            Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|n| !n.is_empty())
                .unwrap_or("Subcircuit")
                .to_string()
        } else {
            label
        };

        if self.subc_stack.is_empty() {
            self.subc_stack.push(NavEntry {
                file_path: path,
                label: entry_label,
            });
            return;
        }
        if self
            .subc_stack
            .last()
            .is_some_and(|e| nav_paths_equal(&e.file_path, &path))
        {
            return;
        }

        // Prefer matching a direct child of the current stack position so navigating
        // deeper never jumps out of the current ancestor chain.
        let matched_child = children_at_stack(&self.root_subc_children, &self.subc_stack[1..])
            .and_then(|children| {
                let idx = find_subc_child_idx(children, &path, &entry_label)?;
                let child = &children[idx];
                Some(NavEntry {
                    file_path: child.file_path.clone(),
                    label: child.label.clone(),
                })
            });

        let next_entry = matched_child.unwrap_or(NavEntry {
            file_path: path,
            label: entry_label,
        });
        self.subc_stack.push(next_entry);
        ensure_stack_in_tree(&mut self.root_subc_children, &self.subc_stack[1..]);
    }

    #[qslot]
    fn navigate_into_subcircuit(&mut self, path: String, label: String) {
        let was_vis = self.subc_nav_visible();
        self.navigate_into_subcircuit_data(path, label);
        self.subc_nav_changed();
        if self.subc_nav_visible() != was_vis {
            self.toolbar_changed();
        }
    }

    #[qslot]
    fn push_subcircuit_hop(&mut self, path: String, label: String) {
        self.navigate_into_subcircuit(path, label);
    }

    #[qslot]
    fn update_devices(&mut self, devices_val: Value) {
        if let Ok(mut devs) = serde_json::from_value::<Vec<ProgrammableDevice>>(devices_val) {
            let was_vis = self.device_sel_visible();
            if let Some(aid) = &self.active_device_id {
                for d in &mut devs {
                    d.is_active = d.id == *aid || d.uid == *aid;
                }
            }
            if !devs.is_empty() && !devs.iter().any(|d| d.is_active) {
                devs[0].is_active = true;
                self.active_device_id = Some(devs[0].id.clone());
            }
            self.devices = devs;
            self.device_sel_changed();
            if self.device_sel_visible() != was_vis {
                self.toolbar_changed();
            }
        }
    }

    fn toggle_info_state(&mut self) {
        if !self.running {
            return;
        }
        self.sim_info_enabled = !self.sim_info_enabled;
        let _ = self.overlays.set_info_shown(self.sim_info_enabled);
    }

    #[qslot]
    fn toggle_info(&mut self) {
        if !self.running {
            return;
        }
        self.toggle_info_state();
        self.info_visible_changed();
        self.overlay_layout_changed();
        self.toolbar_changed();
    }
}

fn action(
    id: &str,
    tooltip: &str,
    icon: UiIcon,
    visible: bool,
    checkable: bool,
    checked: bool,
) -> Value {
    json!({
        "kind": "action",
        "id": id,
        "text": tooltip,
        "tooltip": tooltip,
        "enabled": true,
        "visible": visible,
        "checkable": checkable,
        "checked": checked,
        "iconLigature": icon.as_str(),
        "icon": "",
        "tint": "",
        "separator": false,
        "widget": ""
    })
}

fn separator(visible: bool) -> Value {
    json!({
        "kind": "separator",
        "id": "",
        "separator": true,
        "widget": "",
        "visible": visible,
        "enabled": true
    })
}

fn widget(name: &str, visible: bool) -> Value {
    json!({
        "kind": "widget",
        "id": name,
        "separator": false,
        "widget": name,
        "visible": visible,
        "enabled": true
    })
}

fn collapse_separators(items: &mut [Value]) {
    let mut seen_visible = false;
    let mut pending: Option<usize> = None;
    for i in 0..items.len() {
        let is_sep = items[i].get("separator").and_then(|v| v.as_bool()) == Some(true);
        if is_sep {
            if let Some(obj) = items[i].as_object_mut() {
                obj.insert("visible".into(), json!(false));
            }
            if seen_visible {
                pending = Some(i);
            }
        } else if items[i].get("visible").and_then(|v| v.as_bool()) == Some(true) {
            if let Some(p) = pending {
                if let Some(obj) = items[p].as_object_mut() {
                    obj.insert("visible".into(), json!(true));
                }
                pending = None;
            }
            seen_visible = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subcircuit_navigation_remembers_parent_circuit() {
        let mut panel = CircuitPanel::default();

        let tree = json!([
            {
                "label": "Subcircuit Alpha",
                "file_path": "/path/to/sub_a.sim1",
                "children": [
                    {
                        "label": "Child Gamma",
                        "file_path": "/path/to/sub_c.sim1",
                        "children": []
                    }
                ]
            },
            {
                "label": "Subcircuit Beta",
                "file_path": "/path/to/sub_b.sim1",
                "children": []
            }
        ]);

        // 1. Reset on loading root circuit
        panel.reset_subcircuits_data("/path/to/main.sim1".into(), "Main Schematic".into(), tree);

        let nav = panel.subc_nav();
        assert_eq!(nav["visible"], true);
        assert_eq!(nav["buttonText"], "Main Schematic");
        assert_eq!(nav["buttonTooltip"], "Main Schematic");

        let rows = panel.build_subc_rows();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].text, "Main Schematic");
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[0].current, true);
        assert_eq!(rows[1].text, "Subcircuit Alpha");
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[1].current, false);
        assert_eq!(rows[2].text, "Child Gamma");
        assert_eq!(rows[2].depth, 2);
        assert_eq!(rows[2].current, false);
        assert_eq!(rows[3].text, "Subcircuit Beta");
        assert_eq!(rows[3].depth, 1);
        assert_eq!(rows[3].current, false);

        // 2. Navigate into Subcircuit Alpha
        panel
            .navigate_into_subcircuit_data("/path/to/sub_a.sim1".into(), "Subcircuit Alpha".into());

        let nav = panel.subc_nav();
        assert_eq!(nav["visible"], true);
        assert_eq!(nav["buttonText"], "Subcircuit Alpha");
        assert_eq!(nav["buttonTooltip"], "Main Schematic / Subcircuit Alpha");

        let rows = panel.build_subc_rows();
        assert_eq!(rows[0].current, false);
        assert_eq!(rows[1].current, true);
        assert_eq!(rows[2].current, false);

        // 3. Navigate deeper into Child Gamma
        panel.navigate_into_subcircuit_data("/path/to/sub_c.sim1".into(), "Child Gamma".into());

        let nav = panel.subc_nav();
        assert_eq!(nav["visible"], true);
        assert_eq!(nav["buttonText"], "Child Gamma");
        assert_eq!(
            nav["buttonTooltip"],
            "Main Schematic / Subcircuit Alpha / Child Gamma"
        );

        let rows = panel.build_subc_rows();
        assert_eq!(rows[0].current, false);
        assert_eq!(rows[1].current, false);
        assert_eq!(rows[2].current, true);

        // 4. Use select_subc_index(0) to jump back to root (Main Schematic)
        let target = panel.select_subc_index(0);
        assert_eq!(target.as_deref(), Some("/path/to/main.sim1"));

        let nav = panel.subc_nav();
        assert_eq!(nav["visible"], true);
        assert_eq!(nav["buttonText"], "Main Schematic");
        assert_eq!(nav["buttonTooltip"], "Main Schematic");

        let rows = panel.build_subc_rows();
        assert_eq!(rows[0].current, true);
        assert_eq!(rows[1].current, false);
        assert_eq!(rows[2].current, false);
    }

    #[test]
    fn subcircuit_sync_updates_tree_without_losing_navigation_context() {
        let mut panel = CircuitPanel::default();

        // 1. Start with an unsaved empty circuit -> dropdown hidden.
        panel.reset_subcircuits_data(String::new(), String::new(), json!([]));
        assert!(!panel.subc_nav_visible());

        // 2. Place a subcircuit on root -> sync_subcircuits reveals the dropdown with hierarchy.
        let root_tree = json!([
            {
                "label": "BoardA",
                "file_path": "C:/proj/BoardA/BoardA.circ1",
                "children": [
                    {
                        "label": "LED_PKG",
                        "file_path": "C:/proj/LED_PKG/LED_PKG.circ1",
                        "children": []
                    }
                ]
            },
            {
                "label": "LED_PKG",
                "file_path": "C:/proj/LED_PKG/LED_PKG.circ1",
                "children": []
            }
        ]);
        let (changed, vis_changed) =
            panel.sync_subcircuits_data(String::new(), String::new(), root_tree.clone());
        assert!(changed);
        assert!(vis_changed);
        assert!(panel.subc_nav_visible());
        assert_eq!(panel.build_subc_rows().len(), 4);

        // 3. Save the unsaved root circuit -> root path & label update cleanly.
        panel.sync_subcircuits_data("C:/proj/main.circ1".into(), "main.circ1".into(), root_tree);
        assert_eq!(panel.build_subc_rows()[0].text, "main.circ1");
        assert_eq!(
            panel.build_subc_rows()[0].file_chain,
            vec!["C:/proj/main.circ1"]
        );

        // 4. Navigate into BoardA using Windows backslash path variant.
        panel.navigate_into_subcircuit_data(r"C:\proj\BoardA\BoardA.circ1".into(), "BoardA".into());
        let nav = panel.subc_nav();
        assert_eq!(nav["buttonTooltip"], "main.circ1 / BoardA");
        let rows = panel.build_subc_rows();
        assert!(!rows[0].current);
        assert!(rows[1].current);

        // 5. Open LED_PKG from inside BoardA -> must stay under BoardA, NOT jump to top-level LED_PKG!
        panel.navigate_into_subcircuit_data(
            r"C:\proj\LED_PKG\LED_PKG.circ1".into(),
            "LED_PKG".into(),
        );
        let nav = panel.subc_nav();
        assert_eq!(nav["buttonTooltip"], "main.circ1 / BoardA / LED_PKG");
        let rows = panel.build_subc_rows();
        assert!(rows[2].current, "nested BoardA/LED_PKG must be current");
        assert!(!rows[3].current, "top-level LED_PKG must not be current");

        // 6. Syncing while inside LED_PKG preserves stack and updates LED_PKG's children.
        let inner_tree = json!([
            {
                "label": "DiodeCore",
                "file_path": "C:/proj/DiodeCore/DiodeCore.circ1",
                "children": []
            }
        ]);
        panel.sync_subcircuits_data(
            "C:/proj/LED_PKG/LED_PKG.circ1".into(),
            "LED_PKG.circ1".into(),
            inner_tree,
        );
        let nav = panel.subc_nav();
        assert_eq!(nav["buttonTooltip"], "main.circ1 / BoardA / LED_PKG");
        let rows = panel.build_subc_rows();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[3].text, "DiodeCore");
        assert_eq!(rows[3].depth, 3);
        assert!(rows[2].current);
    }

    #[test]
    fn subcircuit_navigation_shows_dynamic_hops_and_guards_unresolved_rows() {
        let mut panel = CircuitPanel::default();
        panel.reset_subcircuits_data("/proj/main.circ1".into(), "main.circ1".into(), json!([]));

        // Navigating into a subcircuit before root_subc_children was populated still lists hierarchy.
        panel.navigate_into_subcircuit_data("/proj/sub_a.circ1".into(), "SubA".into());
        panel.navigate_into_subcircuit_data("/proj/sub_b.circ1".into(), "SubB".into());
        let rows = panel.build_subc_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].text, "main.circ1");
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[1].text, "SubA");
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[2].text, "SubB");
        assert_eq!(rows[2].depth, 2);
        assert!(rows[2].current);

        // Clicking an already-current row returns None (no redundant reload).
        assert_eq!(panel.select_subc_index(2), None);

        // Unresolved non-root subcircuit (empty file_path) must not corrupt stack.
        panel.reset_subcircuits_data(
            "/proj/main.circ1".into(),
            "main.circ1".into(),
            json!([{ "label": "MissingSub", "file_path": "", "children": [] }]),
        );
        assert_eq!(panel.select_subc_index(1), None);
        assert_eq!(panel.subc_stack.len(), 1);
        assert_eq!(panel.subc_stack[0].file_path, "/proj/main.circ1");
    }

    #[test]
    fn test_simulation_info_remembers_state_across_runs() {
        let mut panel = CircuitPanel::default();
        assert!(panel.sim_info_enabled);
        assert!(!panel.info_visible());

        // 1. Start simulation: info card should be shown because sim_info_enabled defaults to true.
        panel.set_sim_running_state(true);
        assert!(panel.running);
        assert!(panel.sim_info_enabled);
        assert!(panel.info_visible());

        // 2. Hide info (simulating clicking close button on InfoCard).
        panel.set_info_hidden();
        assert!(!panel.sim_info_enabled);
        assert!(!panel.info_visible());

        // 3. Stop simulation.
        panel.set_sim_running_state(false);
        assert!(!panel.running);
        assert!(!panel.sim_info_enabled);
        assert!(!panel.info_visible());

        // 4. Start simulation again: info card should remember it was hidden.
        panel.set_sim_running_state(true);
        assert!(panel.running);
        assert!(!panel.sim_info_enabled);
        assert!(!panel.info_visible());

        // 5. Toggle info back on.
        panel.toggle_info_state();
        assert!(panel.sim_info_enabled);
        assert!(panel.info_visible());

        // 6. Stop and start simulation again: info card should remember it was shown.
        panel.set_sim_running_state(false);
        assert!(!panel.running);
        assert!(!panel.info_visible());
        panel.set_sim_running_state(true);
        assert!(panel.running);
        assert!(panel.sim_info_enabled);
        assert!(panel.info_visible());
    }

    #[test]
    fn test_stop_circ_resets_simulation_and_ui_state() {
        let mut panel = CircuitPanel::default();
        panel.set_sim_running_state(true);
        panel.message_text = "Running".into();
        assert!(panel.running);
        assert!(panel.info_visible());

        let stopped = panel.stop_circ_data();
        assert!(stopped);
        assert!(!panel.running);
        assert!(!panel.paused);
        assert!(panel.message_text.is_empty());
        assert!(!panel.info_visible());

        // Calling again should return false (already stopped)
        assert!(!panel.stop_circ_data());
    }

    #[test]
    fn test_sync_sim_state_paused() {
        let mut panel = CircuitPanel::default();
        panel.sync_sim_state_data(true, true);
        assert!(panel.running);
        assert!(panel.paused);
        assert_eq!(panel.message_text, "Paused");

        panel.sync_sim_state_data(false, false);
        assert!(!panel.running);
        assert!(!panel.paused);
        assert!(panel.message_text.is_empty());
    }

    #[test]
    fn test_sync_start_shows_side_panel_and_info() {
        let mut panel = CircuitPanel::default();
        assert!(panel.overlays.set_side_shown(false));
        assert!(!panel.info_visible());

        assert!(panel.sync_sim_state_data(true, false));
        assert!(panel.running);
        assert!(!panel.paused);
        assert_eq!(panel.message_text, "Running");
        assert!(panel.overlays.side_shown());
        assert!(panel.info_visible());
    }

    #[test]
    fn test_sync_pause_leaves_overlays_alone() {
        let mut panel = CircuitPanel::default();
        panel.sync_sim_state_data(true, false);
        panel.set_info_hidden();
        assert!(panel.overlays.set_side_shown(false));
        assert!(!panel.info_visible());
        assert!(!panel.overlays.side_shown());

        assert!(panel.sync_sim_state_data(true, true));
        assert!(panel.paused);
        assert_eq!(panel.message_text, "Paused");
        assert!(!panel.info_visible());
        assert!(!panel.overlays.side_shown());

        assert!(panel.sync_sim_state_data(true, false));
        assert!(!panel.paused);
        assert_eq!(panel.message_text, "Running");
        assert!(!panel.info_visible());
        assert!(!panel.overlays.side_shown());
    }
}
