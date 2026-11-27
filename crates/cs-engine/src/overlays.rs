//! 4-corner overlay layout engine, drag/resize gestures, and config persistence.
//!
//! Pure geometry (no Qt). Five independent overlays (`components`, `files`,
//! `editor`, `simLog`, `compLog`) can each be assigned to any of the four
//! corners (`tl`, `bl`, `tr`, `br`) and toggled individually between hover mode
//! (floating with shadow) and pinned / always-visible mode (docked flush with
//! the window margin).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const OVERLAY_MARGIN: i32 = 12;
pub const PANEL_GAP: i32 = 8;
pub const GROUP_GAP: i32 = 6;
pub const BTN_SIZE: i32 = 28;
pub const BTN_SPACING: i32 = 4;
pub const STRIP_PAD: i32 = 4;
pub const STRIP_W: i32 = BTN_SIZE + 2 * STRIP_PAD;
pub const SPLIT_HANDLE_H: i32 = 8;
pub const RESIZE_HIT: i32 = 6;
pub const DEFAULT_LEFT_WIDTH: i32 = 260;
pub const DEFAULT_RIGHT_WIDTH: i32 = 360;
pub const MIN_PANEL_W: i32 = 180;
pub const MIN_PANEL_H: i32 = 100;
pub const MIN_SPLIT_RATIO: f64 = 0.15;
pub const MAX_SPLIT_RATIO: f64 = 0.85;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Corner {
    Tl,
    Bl,
    Tr,
    Br,
}

impl Corner {
    pub const ALL: [Corner; 4] = [Corner::Tl, Corner::Bl, Corner::Tr, Corner::Br];

    pub fn index(self) -> usize {
        match self {
            Self::Tl => 0,
            Self::Bl => 1,
            Self::Tr => 2,
            Self::Br => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tl => "tl",
            Self::Bl => "bl",
            Self::Tr => "tr",
            Self::Br => "br",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "tl" => Some(Self::Tl),
            "bl" => Some(Self::Bl),
            "tr" => Some(Self::Tr),
            "br" => Some(Self::Br),
            _ => None,
        }
    }

    pub fn is_left(self) -> bool {
        matches!(self, Self::Tl | Self::Bl)
    }

    pub fn is_top(self) -> bool {
        matches!(self, Self::Tl | Self::Tr)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PanelId {
    Components,
    Files,
    Editor,
    SimLog,
    CompLog,
}

impl PanelId {
    pub const ALL: [PanelId; 5] = [
        PanelId::Components,
        PanelId::Files,
        PanelId::Editor,
        PanelId::SimLog,
        PanelId::CompLog,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Components => "components",
            Self::Files => "files",
            Self::Editor => "editor",
            Self::SimLog => "simLog",
            Self::CompLog => "compLog",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "components" => Some(Self::Components),
            "files" => Some(Self::Files),
            "editor" | "editorPanel" => Some(Self::Editor),
            "simLog" => Some(Self::SimLog),
            "compLog" => Some(Self::CompLog),
            _ => None,
        }
    }

    pub fn default_corner(self) -> Corner {
        match self {
            Self::Components | Self::Files => Corner::Tl,
            Self::Editor => Corner::Tr,
            Self::SimLog | Self::CompLog => Corner::Br,
        }
    }

    pub fn default_pinned(self) -> bool {
        match self {
            Self::Components | Self::Files => false,
            Self::Editor | Self::SimLog | Self::CompLog => true,
        }
    }

    pub fn default_width(self) -> i32 {
        match self {
            Self::Components | Self::Files => DEFAULT_LEFT_WIDTH,
            Self::Editor => 380,
            Self::SimLog | Self::CompLog => 340,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingReason {
    Hover,
    Focused,
}

impl FloatingReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Focused => "focused",
        }
    }
}

/// Serializable user configuration for the 4-corner overlay layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayLayoutConfig {
    pub corners: BTreeMap<String, String>,
    pub order: BTreeMap<String, Vec<String>>,
    pub pinned: BTreeMap<String, bool>,
    pub active_pinned: BTreeMap<String, String>,
    pub left_width: i32,
    pub right_width: i32,
    pub floating_widths: BTreeMap<String, i32>,
    pub left_split: f64,
    pub right_split: f64,
}

impl Default for OverlayLayoutConfig {
    fn default() -> Self {
        let mut corners = BTreeMap::new();
        let mut pinned = BTreeMap::new();
        let mut floating_widths = BTreeMap::new();
        for p in PanelId::ALL {
            corners.insert(p.as_str().into(), p.default_corner().as_str().into());
            pinned.insert(p.as_str().into(), p.default_pinned());
            floating_widths.insert(p.as_str().into(), p.default_width());
        }
        let mut order = BTreeMap::new();
        order.insert("tl".into(), vec!["components".into(), "files".into()]);
        order.insert("bl".into(), Vec::new());
        order.insert("tr".into(), vec!["editor".into()]);
        order.insert("br".into(), vec!["simLog".into(), "compLog".into()]);
        let mut active_pinned = BTreeMap::new();
        active_pinned.insert("tr".into(), "editor".into());
        active_pinned.insert("br".into(), "simLog".into());
        Self {
            corners,
            order,
            pinned,
            active_pinned,
            left_width: DEFAULT_LEFT_WIDTH,
            right_width: DEFAULT_RIGHT_WIDTH,
            floating_widths,
            left_split: 0.5,
            right_split: 0.5,
        }
    }
}

impl OverlayLayoutConfig {
    pub fn clamp(&mut self) {
        self.left_width = self.left_width.clamp(MIN_PANEL_W, 1200);
        self.right_width = self.right_width.clamp(MIN_PANEL_W, 1200);
        self.left_split = self.left_split.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
        self.right_split = self.right_split.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
        for p in PanelId::ALL {
            let key = p.as_str();
            let valid_corner = self
                .corners
                .get(key)
                .and_then(|c| Corner::parse(c))
                .unwrap_or_else(|| p.default_corner());
            self.corners
                .insert(key.into(), valid_corner.as_str().into());
            self.pinned
                .entry(key.into())
                .or_insert_with(|| p.default_pinned());
            let fw = self
                .floating_widths
                .get(key)
                .copied()
                .unwrap_or_else(|| p.default_width())
                .clamp(MIN_PANEL_W, 1200);
            self.floating_widths.insert(key.into(), fw);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn top_left(self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelLayout {
    pub visible: bool,
    pub floating: bool,
    pub pinned: bool,
    pub active_pinned: bool,
    pub corner: String,
    pub is_left: bool,
    pub rect: IRect,
    pub button: IRect,
    pub z: i32,
    pub resize: IRect,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CornerStrip {
    pub rect: IRect,
    pub pinned_rect: IRect,
    pub hover_rect: IRect,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HandleRect {
    pub visible: bool,
    pub rect: IRect,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlaySnapshot {
    pub margin: i32,
    pub top_min: i32,
    pub bottom_max: i32,
    pub available_area: IRect,
    pub fit_area: IRect,
    pub toolbar: IRect,
    pub toolbar_z: i32,
    pub info_card: IRect,
    pub info_card_z: i32,
    pub panels: BTreeMap<PanelId, PanelLayout>,
    pub corners: [CornerStrip; 4],
    pub left_split: HandleRect,
    pub right_split: HandleRect,
    pub left_resize: HandleRect,
    pub right_resize: HandleRect,
    pub floating_panel: Option<PanelId>,
    pub floating_reason: String,
}

#[derive(Clone, Debug, Default)]
struct Gesture {
    id: String,
    start_mouse: (i32, i32),
    start_pos: (i32, i32),
    start_w: i32,
    start_split: f64,
    avail_h: i32,
    floating_panel: Option<PanelId>,
    floating_is_left: bool,
    mutated_config: bool,
}

/// Stored overlay state. Layout is a pure function of this state.
#[derive(Clone, Debug)]
pub struct Overlays {
    view_w: i32,
    view_h: i32,
    top_inset: i32,
    toolbar_w: i32,
    toolbar_h: i32,
    toolbar_pos: (i32, i32),
    toolbar_raised: bool,
    info_w: i32,
    info_h: i32,
    info_shown: bool,
    corner_panels: [Vec<PanelId>; 4],
    pinned: BTreeMap<PanelId, bool>,
    active_pinned: [Option<PanelId>; 4],
    floating_panel: Option<PanelId>,
    floating_reason: FloatingReason,
    left_width: i32,
    right_width: i32,
    floating_widths: BTreeMap<PanelId, i32>,
    left_split: f64,
    right_split: f64,
    last_interacted: Option<PanelId>,
    gesture: Gesture,
    snapshot: OverlaySnapshot,
}

impl Default for Overlays {
    fn default() -> Self {
        Self::from_config(&OverlayLayoutConfig::default())
    }
}

impl Overlays {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_config(cfg: &OverlayLayoutConfig) -> Self {
        let mut clean = cfg.clone();
        clean.clamp();

        let mut corner_panels: [Vec<PanelId>; 4] = Default::default();
        let mut placed: [bool; 5] = [false; 5];

        // First place panels according to persisted per-corner order.
        for c in Corner::ALL {
            if let Some(list) = clean.order.get(c.as_str()) {
                for id_str in list {
                    if let Some(p) = PanelId::parse(id_str) {
                        let expected_corner = clean
                            .corners
                            .get(p.as_str())
                            .and_then(|s| Corner::parse(s))
                            .unwrap_or_else(|| p.default_corner());
                        let idx = panel_index(p);
                        if expected_corner == c && !placed[idx] {
                            corner_panels[c.index()].push(p);
                            placed[idx] = true;
                        }
                    }
                }
            }
        }

        // Place any remaining panels (e.g. from older/partial configs) in canonical order.
        for p in PanelId::ALL {
            let idx = panel_index(p);
            if !placed[idx] {
                let c = clean
                    .corners
                    .get(p.as_str())
                    .and_then(|s| Corner::parse(s))
                    .unwrap_or_else(|| p.default_corner());
                corner_panels[c.index()].push(p);
                placed[idx] = true;
            }
        }

        let mut pinned = BTreeMap::new();
        let mut floating_widths = BTreeMap::new();
        for p in PanelId::ALL {
            let is_pin = clean
                .pinned
                .get(p.as_str())
                .copied()
                .unwrap_or_else(|| p.default_pinned());
            pinned.insert(p, is_pin);
            let fw = clean
                .floating_widths
                .get(p.as_str())
                .copied()
                .unwrap_or_else(|| p.default_width())
                .clamp(MIN_PANEL_W, 1200);
            floating_widths.insert(p, fw);
        }

        let mut active_pinned: [Option<PanelId>; 4] = [None; 4];
        for c in Corner::ALL {
            active_pinned[c.index()] = match clean.active_pinned.get(c.as_str()) {
                // Explicitly persisted empty string means this docked corner is collapsed.
                Some(s) if s.is_empty() => None,
                Some(s) => PanelId::parse(s)
                    .filter(|p| {
                        corner_panels[c.index()].contains(p)
                            && pinned.get(p).copied().unwrap_or(false)
                    })
                    .or_else(|| {
                        corner_panels[c.index()]
                            .iter()
                            .copied()
                            .find(|p| pinned.get(p).copied().unwrap_or(false))
                    }),
                None => corner_panels[c.index()]
                    .iter()
                    .copied()
                    .find(|p| pinned.get(p).copied().unwrap_or(false)),
            };
        }

        let mut s = Self {
            view_w: 0,
            view_h: 0,
            top_inset: 0,
            toolbar_w: 0,
            toolbar_h: 0,
            toolbar_pos: (-1, -1),
            toolbar_raised: false,
            info_w: 0,
            info_h: 0,
            info_shown: false,
            corner_panels,
            pinned,
            active_pinned,
            floating_panel: None,
            floating_reason: FloatingReason::Hover,
            left_width: clean.left_width,
            right_width: clean.right_width,
            floating_widths,
            left_split: clean.left_split,
            right_split: clean.right_split,
            last_interacted: None,
            gesture: Gesture::default(),
            snapshot: OverlaySnapshot::default(),
        };
        s.relayout();
        s
    }

    pub fn to_config(&self) -> OverlayLayoutConfig {
        let mut corners = BTreeMap::new();
        let mut order = BTreeMap::new();
        let mut active_pinned = BTreeMap::new();
        for c in Corner::ALL {
            let ids: Vec<String> = self.corner_panels[c.index()]
                .iter()
                .map(|p| {
                    corners.insert(p.as_str().into(), c.as_str().into());
                    p.as_str().into()
                })
                .collect();
            order.insert(c.as_str().into(), ids);
            let has_pinned_in_corner = self.corner_panels[c.index()]
                .iter()
                .any(|&p| self.is_pinned(p));
            if let Some(ap) = self.active_pinned[c.index()] {
                active_pinned.insert(c.as_str().into(), ap.as_str().into());
            } else if has_pinned_in_corner {
                // Persist collapsed state so docked corners stay collapsed across restarts.
                active_pinned.insert(c.as_str().into(), String::new());
            }
        }
        let mut pinned = BTreeMap::new();
        let mut floating_widths = BTreeMap::new();
        for p in PanelId::ALL {
            pinned.insert(p.as_str().into(), self.is_pinned(p));
            floating_widths.insert(p.as_str().into(), self.floating_width(p));
        }
        OverlayLayoutConfig {
            corners,
            order,
            pinned,
            active_pinned,
            left_width: self.left_width,
            right_width: self.right_width,
            floating_widths,
            left_split: self.left_split,
            right_split: self.right_split,
        }
    }

    pub fn apply_config(&mut self, cfg: &OverlayLayoutConfig) {
        let view_w = self.view_w;
        let view_h = self.view_h;
        let top_inset = self.top_inset;
        let toolbar_w = self.toolbar_w;
        let toolbar_h = self.toolbar_h;
        let toolbar_pos = self.toolbar_pos;
        let info_w = self.info_w;
        let info_h = self.info_h;
        let info_shown = self.info_shown;

        *self = Self::from_config(cfg);
        self.view_w = view_w;
        self.view_h = view_h;
        self.top_inset = top_inset;
        self.toolbar_w = toolbar_w;
        self.toolbar_h = toolbar_h;
        self.toolbar_pos = toolbar_pos;
        self.info_w = info_w;
        self.info_h = info_h;
        self.info_shown = info_shown;
        self.relayout();
    }

    pub fn reset_layout(&mut self) {
        self.apply_config(&OverlayLayoutConfig::default());
        self.toolbar_pos = (-1, -1);
        self.relayout();
    }

    pub fn snapshot(&self) -> &OverlaySnapshot {
        &self.snapshot
    }

    pub fn info_shown(&self) -> bool {
        self.info_shown
    }

    pub fn is_pinned(&self, p: PanelId) -> bool {
        self.pinned.get(&p).copied().unwrap_or(false)
    }

    pub fn is_panel_visible(&self, p: PanelId) -> bool {
        self.snapshot
            .panels
            .get(&p)
            .map(|l| l.visible)
            .unwrap_or(false)
    }

    pub fn corner_of(&self, p: PanelId) -> Corner {
        for c in Corner::ALL {
            if self.corner_panels[c.index()].contains(&p) {
                return c;
            }
        }
        p.default_corner()
    }

    pub fn floating_panel(&self) -> Option<PanelId> {
        self.floating_panel
    }

    pub fn floating_reason(&self) -> FloatingReason {
        self.floating_reason
    }

    fn floating_width(&self, p: PanelId) -> i32 {
        self.floating_widths
            .get(&p)
            .copied()
            .unwrap_or_else(|| p.default_width())
    }

    /// Top Y bound for overlays: equal `OVERLAY_MARGIN` below `top_inset`
    /// whether or not a custom title bar is active.
    pub fn top_min(&self) -> i32 {
        self.top_inset + OVERLAY_MARGIN
    }

    pub fn set_info_shown(&mut self, shown: bool) -> bool {
        if self.info_shown == shown {
            return false;
        }
        self.info_shown = shown;
        if shown {
            self.toolbar_raised = true;
        }
        self.relayout();
        true
    }

    pub fn set_view_size(&mut self, w: i32, h: i32) -> bool {
        if self.view_w == w && self.view_h == h {
            return false;
        }
        self.view_w = w;
        self.view_h = h;
        self.relayout();
        true
    }

    pub fn set_top_inset(&mut self, inset: i32) -> bool {
        let inset = inset.max(0);
        if self.top_inset == inset {
            return false;
        }
        self.top_inset = inset;
        self.relayout();
        true
    }

    pub fn set_toolbar_size(&mut self, w: i32, h: i32) -> bool {
        if self.toolbar_w == w && self.toolbar_h == h {
            return false;
        }
        self.toolbar_w = w;
        self.toolbar_h = h;
        self.relayout();
        true
    }

    pub fn set_info_card_size(&mut self, w: i32, h: i32) -> bool {
        if self.info_w == w && self.info_h == h {
            return false;
        }
        self.info_w = w;
        self.info_h = h;
        self.relayout();
        true
    }

    /// Handles single-clicking an overlay's corner icon button.
    ///
    /// - If in hover mode: opens or closes the floating overlay temporarily.
    /// - If docked (always-visible mode) and currently expanded: collapses the
    ///   docked panel without switching it to hover mode.
    /// - If docked and collapsed or another docked panel in the same corner is
    ///   shown: expands / switches to show it.
    ///
    /// Returns `(layout_changed, config_changed)`.
    pub fn button_clicked(&mut self, panel: PanelId) -> (bool, bool) {
        let corner = self.corner_of(panel);
        let ci = corner.index();
        self.last_interacted = Some(panel);
        self.toolbar_raised = false;

        if !self.is_pinned(panel) {
            if self.floating_panel == Some(panel) {
                self.floating_panel = None;
            } else {
                self.floating_panel = Some(panel);
                self.floating_reason = FloatingReason::Focused;
            }
            self.relayout();
            return (true, false);
        }

        if self.active_pinned[ci] == Some(panel) {
            // Collapse the docked panel while keeping it in docked mode.
            self.active_pinned[ci] = None;
        } else {
            self.active_pinned[ci] = Some(panel);
            if self.floating_panel == Some(panel) {
                self.floating_panel = None;
            }
        }

        self.relayout();
        (true, true)
    }

    /// Handles double-clicking an overlay's corner icon button: toggles
    /// between always-visible (pinned) and hover mode.
    pub fn button_double_clicked(&mut self, panel: PanelId) -> bool {
        let next_pin = !self.is_pinned(panel);
        self.toolbar_raised = false;
        self.set_panel_pinned(panel, next_pin)
    }

    /// Explicitly sets a panel's pinned (always-visible) state.
    pub fn set_panel_pinned(&mut self, panel: PanelId, pin: bool) -> bool {
        let corner = self.corner_of(panel);
        let ci = corner.index();
        if self.is_pinned(panel) == pin {
            if pin && self.active_pinned[ci] != Some(panel) {
                self.active_pinned[ci] = Some(panel);
                self.relayout();
                return true;
            }
            return false;
        }
        self.pinned.insert(panel, pin);
        if pin {
            self.active_pinned[ci] = Some(panel);
            if self.floating_panel == Some(panel) {
                self.floating_panel = None;
            }
        } else if self.active_pinned[ci] == Some(panel) {
            self.active_pinned[ci] = self.corner_panels[ci]
                .iter()
                .copied()
                .find(|&p| p != panel && self.is_pinned(p));
        }
        self.last_interacted = Some(panel);
        self.relayout();
        true
    }

    /// Hover enter/leave on a corner icon button.
    /// Only non-pinned panels open on hover; hovering a pinned button closes
    /// any previous hover-only floating panel.
    pub fn set_hover_panel(&mut self, panel: Option<PanelId>) -> bool {
        match panel {
            Some(p) => {
                if self.is_pinned(p) {
                    if self.floating_panel.is_some()
                        && self.floating_reason == FloatingReason::Hover
                    {
                        self.floating_panel = None;
                        self.relayout();
                        return true;
                    }
                    return false;
                }
                if self.floating_panel == Some(p) && self.floating_reason == FloatingReason::Hover {
                    return false;
                }
                self.floating_panel = Some(p);
                self.floating_reason = FloatingReason::Hover;
                self.toolbar_raised = false;
                self.relayout();
                true
            }
            None => {
                if self.floating_panel.is_some() && self.floating_reason == FloatingReason::Hover {
                    self.floating_panel = None;
                    self.relayout();
                    return true;
                }
                false
            }
        }
    }

    /// Programmatically opens / focuses a panel (e.g. opening a file, shortcut,
    /// starting simulation, clicking compile status).
    ///
    /// - If the panel is pinned: activates it in its corner (returns `true` if
    ///   config/active tab changed).
    /// - If the panel is in hover mode: opens it as a temporary `Focused`
    ///   floating overlay until dismissed via Esc or clicking outside.
    ///
    /// Returns `(layout_changed, config_changed)`.
    pub fn open_temporary(&mut self, panel: PanelId) -> (bool, bool) {
        self.last_interacted = Some(panel);
        self.toolbar_raised = false;
        let corner = self.corner_of(panel);
        let ci = corner.index();

        if self.is_pinned(panel) {
            let tab_changed = self.active_pinned[ci] != Some(panel);
            let float_cleared = self.floating_panel.is_some();
            self.active_pinned[ci] = Some(panel);
            self.floating_panel = None;
            if tab_changed || float_cleared {
                self.relayout();
                return (true, tab_changed);
            }
            return (false, false);
        }

        let changed =
            self.floating_panel != Some(panel) || self.floating_reason != FloatingReason::Focused;
        self.floating_panel = Some(panel);
        self.floating_reason = FloatingReason::Focused;
        if changed {
            self.relayout();
        }
        (changed, false)
    }

    /// Dismisses any open floating overlay (whether hover or focused).
    pub fn dismiss_floating(&mut self) -> bool {
        if self.floating_panel.is_none() {
            return false;
        }
        self.floating_panel = None;
        self.relayout();
        true
    }

    /// Hides a panel if it is currently floating (used when e.g. editor closes
    /// all documents); pinned panels stay pinned.
    pub fn hide_if_floating(&mut self, panel: PanelId) -> bool {
        if self.floating_panel == Some(panel) {
            self.floating_panel = None;
            self.relayout();
            return true;
        }
        false
    }

    /// Moves a panel to another corner (`tl`, `bl`, `tr`, `br`), preserving its
    /// pinned/hover mode. Returns `true` if the corner changed.
    pub fn move_panel_to_corner(&mut self, panel: PanelId, target: Corner) -> bool {
        let old = self.corner_of(panel);
        if old == target {
            return false;
        }
        let oi = old.index();
        let ti = target.index();
        self.corner_panels[oi].retain(|&p| p != panel);
        self.corner_panels[ti].push(panel);

        if self.active_pinned[oi] == Some(panel) {
            self.active_pinned[oi] = self.corner_panels[oi]
                .iter()
                .copied()
                .find(|&p| self.is_pinned(p));
        }
        if self.is_pinned(panel) {
            self.active_pinned[ti] = Some(panel);
        }
        if self.floating_panel == Some(panel) {
            self.floating_panel = None;
        }
        self.last_interacted = Some(panel);
        self.relayout();
        true
    }

    pub fn overlay_clicked(&mut self, id: &str) -> bool {
        if id == "toolbar" || id == "infoCard" {
            if !self.toolbar_raised {
                self.toolbar_raised = true;
                self.relayout();
                return true;
            }
            return false;
        }
        if let Some(p) = PanelId::parse(id) {
            self.last_interacted = Some(p);
            if self.toolbar_raised {
                self.toolbar_raised = false;
                self.relayout();
                return true;
            }
        }
        false
    }

    /// Toggles the most recently interacted overlay (or dismisses a floating
    /// overlay if one is open). Returns `(layout_changed, config_changed)`.
    pub fn toggle_active_overlay(&mut self) -> (bool, bool) {
        if self.floating_panel.is_some() {
            self.floating_panel = None;
            self.relayout();
            return (true, false);
        }
        let target = self
            .last_interacted
            .or_else(|| {
                Corner::ALL
                    .iter()
                    .find_map(|c| self.active_pinned[c.index()])
            })
            .unwrap_or(PanelId::Components);
        self.button_double_clicked(target);
        (true, true)
    }

    pub fn reset_pos(&mut self, id: &str) -> bool {
        if id == "toolbar" {
            self.toolbar_pos = (-1, -1);
            self.relayout();
            return true;
        }
        false
    }

    pub fn reset_size(&mut self, id: &str) -> bool {
        match id {
            "left" | "leftResize" => {
                self.left_width = DEFAULT_LEFT_WIDTH;
                self.relayout();
                true
            }
            "right" | "rightResize" => {
                self.right_width = DEFAULT_RIGHT_WIDTH;
                self.relayout();
                true
            }
            "leftSplit" => {
                self.left_split = 0.5;
                self.relayout();
                true
            }
            "rightSplit" => {
                self.right_split = 0.5;
                self.relayout();
                true
            }
            "floatResize" => {
                if let Some(p) = self.floating_panel {
                    self.floating_widths.insert(p, p.default_width());
                    self.relayout();
                    return true;
                }
                false
            }
            other => {
                if let Some(p) = PanelId::parse(other) {
                    self.floating_widths.insert(p, p.default_width());
                    self.relayout();
                    return true;
                }
                false
            }
        }
    }

    pub fn gesture_begin(&mut self, id: &str, x: i32, y: i32) {
        let top_min = self.top_min();
        let bottom_max = self.view_h - OVERLAY_MARGIN;
        let avail_h = (bottom_max - top_min).max(1);

        let mut g = Gesture {
            id: id.to_string(),
            start_mouse: (x, y),
            avail_h,
            ..Gesture::default()
        };

        match id {
            "toolbar" => {
                self.toolbar_raised = true;
                g.start_pos = self.snapshot.toolbar.top_left();
            }
            "leftResize" => {
                g.start_w = self.left_width;
            }
            "rightResize" => {
                g.start_w = self.right_width;
            }
            "leftSplit" => {
                g.start_split = self.left_split;
            }
            "rightSplit" => {
                g.start_split = self.right_split;
            }
            "floatResize" => {
                if let Some(p) = self.floating_panel {
                    g.floating_panel = Some(p);
                    g.floating_is_left = self.corner_of(p).is_left();
                    g.start_w = self
                        .snapshot
                        .panels
                        .get(&p)
                        .map(|l| l.rect.w)
                        .unwrap_or_else(|| self.floating_width(p));
                }
            }
            _ => {}
        }

        self.gesture = g;
        self.relayout();
    }

    pub fn gesture_move(&mut self, x: i32, y: i32) -> bool {
        if self.gesture.id.is_empty() {
            return false;
        }
        let dx = x - self.gesture.start_mouse.0;
        let dy = y - self.gesture.start_mouse.1;
        let max_side_w = (self.view_w - 2 * OVERLAY_MARGIN - STRIP_W - PANEL_GAP - 80)
            .max(MIN_PANEL_W)
            .min(1200);

        match self.gesture.id.as_str() {
            "toolbar" => {
                self.toolbar_pos = (
                    (self.gesture.start_pos.0 + dx).max(0),
                    (self.gesture.start_pos.1 + dy).max(0),
                );
            }
            "leftResize" => {
                self.left_width = (self.gesture.start_w + dx).clamp(MIN_PANEL_W, max_side_w);
                self.gesture.mutated_config = true;
            }
            "rightResize" => {
                self.right_width = (self.gesture.start_w - dx).clamp(MIN_PANEL_W, max_side_w);
                self.gesture.mutated_config = true;
            }
            "leftSplit" => {
                let usable = (self.gesture.avail_h - SPLIT_HANDLE_H).max(1) as f64;
                self.left_split = (self.gesture.start_split + dy as f64 / usable)
                    .clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
                self.gesture.mutated_config = true;
            }
            "rightSplit" => {
                let usable = (self.gesture.avail_h - SPLIT_HANDLE_H).max(1) as f64;
                self.right_split = (self.gesture.start_split + dy as f64 / usable)
                    .clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
                self.gesture.mutated_config = true;
            }
            "floatResize" => {
                if let Some(p) = self.gesture.floating_panel {
                    let next_w = if self.gesture.floating_is_left {
                        self.gesture.start_w + dx
                    } else {
                        self.gesture.start_w - dx
                    };
                    self.floating_widths
                        .insert(p, next_w.clamp(MIN_PANEL_W, max_side_w));
                    self.gesture.mutated_config = true;
                }
            }
            _ => return false,
        }
        self.relayout();
        true
    }

    /// Ends the active gesture. Returns `true` if persistent layout config changed.
    pub fn gesture_end(&mut self) -> bool {
        let mutated = self.gesture.mutated_config;
        self.gesture = Gesture::default();
        self.relayout();
        mutated
    }

    fn relayout(&mut self) {
        self.snapshot = layout(self);
    }
}

fn panel_index(p: PanelId) -> usize {
    match p {
        PanelId::Components => 0,
        PanelId::Files => 1,
        PanelId::Editor => 2,
        PanelId::SimLog => 3,
        PanelId::CompLog => 4,
    }
}

fn clamp_i(v: i32, lo: i32, hi: i32) -> i32 {
    if lo <= hi { v.clamp(lo, hi) } else { lo }
}

/// Clamp a horizontal position for an item of `size` inside the tightest bounds
/// that can accommodate it: first the clear area inside corner strips, then the
/// space between docked columns, and finally the window margins.
fn clamp_horiz_to_available(
    raw: i32,
    size: i32,
    clear_lo: i32,
    clear_hi: i32,
    dock_lo: i32,
    dock_hi: i32,
    view_lo: i32,
    view_hi: i32,
) -> i32 {
    let clear_max = clear_hi - size;
    if clear_lo <= clear_max {
        return raw.clamp(clear_lo, clear_max);
    }
    let dock_max = dock_hi - size;
    if dock_lo <= dock_max {
        return raw.clamp(dock_lo, dock_max);
    }
    let view_max = view_lo.max(view_hi - size);
    clamp_i(raw, view_lo, view_max)
}

fn strip_height(count: usize) -> i32 {
    if count == 0 {
        return 0;
    }
    2 * STRIP_PAD + count as i32 * BTN_SIZE + (count as i32 - 1) * BTN_SPACING
}

/// Place a block 4px off the toolbar edge that faces it.
fn toolbar_anchored(
    opens_up: bool,
    tb_y: i32,
    tb_h: i32,
    block_h: i32,
    top_min: i32,
    bottom_limit: i32,
) -> (i32, i32) {
    let gap = 4;
    let block_h = block_h.max(0);
    if opens_up {
        let bottom = tb_y - gap;
        let room = (bottom - top_min).max(0);
        let h = block_h.min(room);
        (bottom - h, h)
    } else {
        let top = tb_y + tb_h + gap;
        let room = (bottom_limit - top).max(0);
        let h = block_h.min(room);
        (top, h)
    }
}

fn layout(s: &Overlays) -> OverlaySnapshot {
    let margin = OVERLAY_MARGIN;
    let top_min = s.top_min();
    let bottom_max = (s.view_h - margin).max(top_min);
    let avail_h = (bottom_max - top_min).max(0);

    let tl_active = s.active_pinned[Corner::Tl.index()];
    let bl_active = s.active_pinned[Corner::Bl.index()];
    let tr_active = s.active_pinned[Corner::Tr.index()];
    let br_active = s.active_pinned[Corner::Br.index()];

    let has_left_dock = tl_active.is_some() || bl_active.is_some();
    let has_right_dock = tr_active.is_some() || br_active.is_some();

    // Effective docked widths clamped cleanly to view width so left & right
    // docked columns never collide.
    let (eff_left_w, eff_right_w) = if has_left_dock && has_right_dock {
        let overhead = 2 * margin + 2 * (PANEL_GAP + STRIP_W) + 120;
        let max_total = (s.view_w - overhead).max(0);
        let raw_l = s.left_width.max(MIN_PANEL_W);
        let raw_r = s.right_width.max(MIN_PANEL_W);
        if raw_l + raw_r <= max_total {
            (raw_l, raw_r)
        } else if max_total == 0 {
            (0, 0)
        } else {
            let total = (raw_l + raw_r) as f64;
            let min_each = MIN_PANEL_W.min(max_total / 2);
            let l = ((max_total as f64 * (raw_l as f64 / total)).round() as i32)
                .clamp(min_each, (max_total - min_each).max(min_each));
            let r = (max_total - l).max(0);
            (l, r)
        }
    } else if has_left_dock {
        let max_one = (s.view_w - 2 * margin - PANEL_GAP - STRIP_W - 60).max(0);
        let w = clamp_i(s.left_width, MIN_PANEL_W.min(max_one), max_one);
        (w, 0)
    } else if has_right_dock {
        let max_one = (s.view_w - 2 * margin - PANEL_GAP - STRIP_W - 60).max(0);
        let w = clamp_i(s.right_width, MIN_PANEL_W.min(max_one), max_one);
        (0, w)
    } else {
        (0, 0)
    };

    // Button strip X positions: when a side has a docked panel, ALL corner
    // button strips on that side shift inward next to the docked panel's inner edge.
    let left_btn_x = if has_left_dock {
        margin + eff_left_w + PANEL_GAP
    } else {
        margin
    };
    let right_btn_x = if has_right_dock {
        (s.view_w - margin - eff_right_w - PANEL_GAP - STRIP_W).max(margin)
    } else {
        (s.view_w - margin - STRIP_W).max(margin)
    };

    // Corner button strips: pinned and hover buttons in the same corner sit in
    // separate pills separated by GROUP_GAP when both exist.
    let mut corners: [CornerStrip; 4] = Default::default();
    let mut button_rects: BTreeMap<PanelId, IRect> = BTreeMap::new();

    let corner_groups = |c: Corner| -> (Vec<PanelId>, Vec<PanelId>, i32, i32, i32) {
        let mut pinned_list = Vec::new();
        let mut hover_list = Vec::new();
        for &p in &s.corner_panels[c.index()] {
            if s.is_pinned(p) {
                pinned_list.push(p);
            } else {
                hover_list.push(p);
            }
        }
        let ph = strip_height(pinned_list.len());
        let hh = strip_height(hover_list.len());
        let total = if ph > 0 && hh > 0 {
            ph + GROUP_GAP + hh
        } else {
            ph + hh
        };
        (pinned_list, hover_list, ph, hh, total)
    };

    let (tl_pin, tl_hov, tl_ph, tl_hh, tl_total) = corner_groups(Corner::Tl);
    let (bl_pin, bl_hov, bl_ph, bl_hh, bl_total) = corner_groups(Corner::Bl);
    let (tr_pin, tr_hov, tr_ph, tr_hh, tr_total) = corner_groups(Corner::Tr);
    let (br_pin, br_hov, br_ph, br_hh, br_total) = corner_groups(Corner::Br);

    let bl_y = if tl_total > 0 {
        (bottom_max - bl_total).max(top_min + tl_total + PANEL_GAP)
    } else {
        (bottom_max - bl_total).max(top_min)
    };
    let br_y = if tr_total > 0 {
        (bottom_max - br_total).max(top_min + tr_total + PANEL_GAP)
    } else {
        (bottom_max - br_total).max(top_min)
    };

    let mut place_corner = |c: Corner,
                            strip_x: i32,
                            strip_y: i32,
                            total_h: i32,
                            pin_list: &[PanelId],
                            hov_list: &[PanelId],
                            ph: i32,
                            hh: i32| {
        let ci = c.index();
        let (pin_rect, hov_rect) = if c.is_top() {
            let pr = if ph > 0 {
                IRect::new(strip_x, strip_y, STRIP_W, ph)
            } else {
                IRect::default()
            };
            let hy = if ph > 0 {
                strip_y + ph + GROUP_GAP
            } else {
                strip_y
            };
            let hr = if hh > 0 {
                IRect::new(strip_x, hy, STRIP_W, hh)
            } else {
                IRect::default()
            };
            (pr, hr)
        } else {
            // Bottom corners: pinned pill hugs the bottom edge, hover pill sits above it.
            let hr = if hh > 0 {
                IRect::new(strip_x, strip_y, STRIP_W, hh)
            } else {
                IRect::default()
            };
            let py = if hh > 0 {
                strip_y + hh + GROUP_GAP
            } else {
                strip_y
            };
            let pr = if ph > 0 {
                IRect::new(strip_x, py, STRIP_W, ph)
            } else {
                IRect::default()
            };
            (pr, hr)
        };

        for (i, &p) in pin_list.iter().enumerate() {
            let by = pin_rect.y + STRIP_PAD + i as i32 * (BTN_SIZE + BTN_SPACING);
            button_rects.insert(
                p,
                IRect::new(pin_rect.x + STRIP_PAD, by, BTN_SIZE, BTN_SIZE),
            );
        }
        for (i, &p) in hov_list.iter().enumerate() {
            let by = hov_rect.y + STRIP_PAD + i as i32 * (BTN_SIZE + BTN_SPACING);
            button_rects.insert(
                p,
                IRect::new(hov_rect.x + STRIP_PAD, by, BTN_SIZE, BTN_SIZE),
            );
        }

        corners[ci] = CornerStrip {
            rect: IRect::new(
                strip_x,
                strip_y,
                if total_h > 0 { STRIP_W } else { 0 },
                total_h,
            ),
            pinned_rect: pin_rect,
            hover_rect: hov_rect,
        };
    };

    place_corner(
        Corner::Tl,
        left_btn_x,
        top_min,
        tl_total,
        &tl_pin,
        &tl_hov,
        tl_ph,
        tl_hh,
    );
    place_corner(
        Corner::Bl,
        left_btn_x,
        bl_y,
        bl_total,
        &bl_pin,
        &bl_hov,
        bl_ph,
        bl_hh,
    );
    place_corner(
        Corner::Tr,
        right_btn_x,
        top_min,
        tr_total,
        &tr_pin,
        &tr_hov,
        tr_ph,
        tr_hh,
    );
    place_corner(
        Corner::Br,
        right_btn_x,
        br_y,
        br_total,
        &br_pin,
        &br_hov,
        br_ph,
        br_hh,
    );

    // Initialize panel layouts (hidden by default).
    let mut panels: BTreeMap<PanelId, PanelLayout> = BTreeMap::new();
    for p in PanelId::ALL {
        let c = s.corner_of(p);
        panels.insert(
            p,
            PanelLayout {
                visible: false,
                floating: false,
                pinned: s.is_pinned(p),
                active_pinned: s.active_pinned[c.index()] == Some(p),
                corner: c.as_str().into(),
                is_left: c.is_left(),
                rect: IRect::default(),
                button: button_rects.get(&p).copied().unwrap_or_default(),
                z: 1,
                resize: IRect::default(),
            },
        );
    }

    // Docked panels on Left side (TL / BL).
    let mut left_split = HandleRect::default();
    let mut left_resize = HandleRect::default();
    if has_left_dock && eff_left_w > 0 && avail_h > 0 {
        left_resize = HandleRect {
            visible: true,
            rect: IRect::new(
                margin + eff_left_w - RESIZE_HIT / 2,
                top_min,
                RESIZE_HIT,
                avail_h,
            ),
        };
        match (tl_active, bl_active) {
            (Some(tl), Some(bl)) => {
                let usable = (avail_h - SPLIT_HANDLE_H).max(0);
                let min_pane = MIN_PANEL_H.min(usable / 2);
                let top_h = ((usable as f64 * s.left_split).round() as i32)
                    .clamp(min_pane, (usable - min_pane).max(min_pane));
                let bot_h = (usable - top_h).max(0);

                if let Some(l) = panels.get_mut(&tl) {
                    l.visible = true;
                    l.rect = IRect::new(margin, top_min, eff_left_w, top_h);
                    l.z = 1;
                }
                left_split = HandleRect {
                    visible: true,
                    rect: IRect::new(margin, top_min + top_h, eff_left_w, SPLIT_HANDLE_H),
                };
                if let Some(l) = panels.get_mut(&bl) {
                    l.visible = true;
                    l.rect =
                        IRect::new(margin, top_min + top_h + SPLIT_HANDLE_H, eff_left_w, bot_h);
                    l.z = 1;
                }
            }
            (Some(p), None) | (None, Some(p)) => {
                if let Some(l) = panels.get_mut(&p) {
                    l.visible = true;
                    l.rect = IRect::new(margin, top_min, eff_left_w, avail_h);
                    l.z = 1;
                }
            }
            (None, None) => {}
        }
    }

    // Docked panels on Right side (TR / BR).
    let mut right_split = HandleRect::default();
    let mut right_resize = HandleRect::default();
    if has_right_dock && eff_right_w > 0 && avail_h > 0 {
        let right_x = (s.view_w - margin - eff_right_w).max(margin);
        right_resize = HandleRect {
            visible: true,
            rect: IRect::new(right_x - RESIZE_HIT / 2, top_min, RESIZE_HIT, avail_h),
        };
        match (tr_active, br_active) {
            (Some(tr), Some(br)) => {
                let usable = (avail_h - SPLIT_HANDLE_H).max(0);
                let min_pane = MIN_PANEL_H.min(usable / 2);
                let top_h = ((usable as f64 * s.right_split).round() as i32)
                    .clamp(min_pane, (usable - min_pane).max(min_pane));
                let bot_h = (usable - top_h).max(0);

                if let Some(l) = panels.get_mut(&tr) {
                    l.visible = true;
                    l.rect = IRect::new(right_x, top_min, eff_right_w, top_h);
                    l.z = 1;
                }
                right_split = HandleRect {
                    visible: true,
                    rect: IRect::new(right_x, top_min + top_h, eff_right_w, SPLIT_HANDLE_H),
                };
                if let Some(l) = panels.get_mut(&br) {
                    l.visible = true;
                    l.rect = IRect::new(
                        right_x,
                        top_min + top_h + SPLIT_HANDLE_H,
                        eff_right_w,
                        bot_h,
                    );
                    l.z = 1;
                }
            }
            (Some(p), None) | (None, Some(p)) => {
                if let Some(l) = panels.get_mut(&p) {
                    l.visible = true;
                    l.rect = IRect::new(right_x, top_min, eff_right_w, avail_h);
                    l.z = 1;
                }
            }
            (None, None) => {}
        }
    }

    // Floating hover / focused overlay (shifted inside the button strip so it
    // never overlaps any always-visible docked panel or corner button strip).
    let mut active_floating = None;
    if let Some(fp) = s.floating_panel {
        if !s.is_pinned(fp) && avail_h > 0 {
            let corner = s.corner_of(fp);
            let pref_w = s.floating_width(fp);
            let fh = if corner.is_top() {
                avail_h
            } else {
                ((avail_h as f64 * 0.55).round() as i32).clamp(240.min(avail_h), avail_h)
            };
            let fy = if corner.is_top() {
                top_min
            } else {
                (bottom_max - fh).max(top_min)
            };

            let (fx, fw, resize_rect) = if corner.is_left() {
                let fx = left_btn_x + STRIP_W + PANEL_GAP;
                let avail_w = (s.view_w - margin - fx).max(0);
                let fw = clamp_i(pref_w, MIN_PANEL_W.min(avail_w), avail_w);
                let rr = IRect::new(fx + fw - RESIZE_HIT / 2, fy, RESIZE_HIT, fh);
                (fx, fw, rr)
            } else {
                let right_edge = right_btn_x - PANEL_GAP;
                let avail_w = (right_edge - margin).max(0);
                let fw = clamp_i(pref_w, MIN_PANEL_W.min(avail_w), avail_w);
                let fx = (right_edge - fw).max(margin);
                let rr = IRect::new(fx - RESIZE_HIT / 2, fy, RESIZE_HIT, fh);
                (fx, fw, rr)
            };

            if fw > 0 {
                active_floating = Some(fp);
                if let Some(l) = panels.get_mut(&fp) {
                    l.visible = true;
                    l.floating = true;
                    l.rect = IRect::new(fx, fy, fw, fh);
                    l.z = 7;
                    l.resize = resize_rect;
                }
            }
        }
    }

    // Available area between docked overlays (and clear of corner button strips).
    let has_left_strip = tl_total > 0 || bl_total > 0;
    let has_right_strip = tr_total > 0 || br_total > 0;
    let dock_left = if has_left_dock && eff_left_w > 0 {
        margin + eff_left_w + PANEL_GAP
    } else {
        margin
    };
    let dock_right = if has_right_dock && eff_right_w > 0 {
        (s.view_w - margin - eff_right_w - PANEL_GAP).max(dock_left)
    } else {
        (s.view_w - margin).max(dock_left)
    };
    let clear_left = if has_left_strip {
        left_btn_x + STRIP_W + PANEL_GAP
    } else {
        dock_left
    };
    let clear_right = if has_right_strip {
        (right_btn_x - PANEL_GAP).max(clear_left)
    } else {
        dock_right.max(clear_left)
    };
    let clear_w = (clear_right - clear_left).max(0);
    let dock_w = (dock_right - dock_left).max(0);
    let available_area = if s.view_w > 0 && s.view_h > 0 && avail_h > 0 {
        if clear_w > 0 {
            IRect::new(clear_left, top_min, clear_w, avail_h)
        } else if dock_w > 0 {
            IRect::new(dock_left, top_min, dock_w, avail_h)
        } else {
            IRect::default()
        }
    } else {
        IRect::default()
    };

    // Floating toolbar + simulation info card.
    let tb_w = s.toolbar_w;
    let tb_h = s.toolbar_h;
    let view_right = (s.view_w - margin).max(margin);
    let (tb_x, tb_y) = if s.toolbar_pos.0 >= 0 {
        let max_y = top_min.max(s.view_h - margin - tb_h);
        (
            clamp_horiz_to_available(
                s.toolbar_pos.0,
                tb_w,
                clear_left,
                clear_right,
                dock_left,
                dock_right,
                margin,
                view_right,
            ),
            clamp_i(s.toolbar_pos.1, top_min, max_y),
        )
    } else {
        let centered = if clear_w >= tb_w {
            clear_left + (clear_w - tb_w) / 2
        } else {
            dock_left + (dock_w - tb_w) / 2
        };
        (
            clamp_horiz_to_available(
                centered,
                tb_w,
                clear_left,
                clear_right,
                dock_left,
                dock_right,
                margin,
                view_right,
            ),
            top_min,
        )
    };

    // Fit area for zoom-to-fit / zoom-to-selection: same horizontal bounds as
    // `available_area`, and vertically clear of the overlay toolbar when the
    // toolbar sits near the top or bottom edge.
    let fit_area = if available_area.w > 0 && available_area.h > 0 {
        let (fit_top, fit_bottom) = if tb_h > 0 {
            if tb_y <= top_min + tb_h {
                ((tb_y + tb_h + PANEL_GAP).min(bottom_max), bottom_max)
            } else if tb_y + tb_h >= bottom_max - tb_h {
                (top_min, (tb_y - PANEL_GAP).max(top_min))
            } else {
                (top_min, bottom_max)
            }
        } else {
            (top_min, bottom_max)
        };
        let fit_h = (fit_bottom - fit_top).max(0);
        if fit_h > 0 {
            IRect::new(available_area.x, fit_top, available_area.w, fit_h)
        } else {
            available_area
        }
    } else {
        IRect::default()
    };

    let ic_w = s.info_w;
    let opens_up = tb_y + tb_h / 2 > (s.top_inset + s.view_h) / 2;
    let (ic_y, ic_h) = toolbar_anchored(opens_up, tb_y, tb_h, s.info_h, top_min, bottom_max);
    let ic_x = {
        let raw = tb_x + tb_w - ic_w;
        clamp_horiz_to_available(
            raw,
            ic_w,
            clear_left,
            clear_right,
            dock_left,
            dock_right,
            margin,
            view_right,
        )
    };

    let toolbar_z = if s.toolbar_raised { 8 } else { 5 };

    OverlaySnapshot {
        margin,
        top_min,
        bottom_max,
        available_area,
        fit_area,
        toolbar: IRect::new(tb_x, tb_y, tb_w, tb_h),
        toolbar_z,
        info_card: IRect::new(ic_x, ic_y, ic_w, ic_h),
        info_card_z: toolbar_z,
        panels,
        corners,
        left_split,
        right_split,
        left_resize,
        right_resize,
        floating_panel: active_floating,
        floating_reason: if active_floating.is_some() {
            s.floating_reason.as_str().into()
        } else {
            String::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sized() -> Overlays {
        let mut o = Overlays::new();
        o.set_view_size(1200, 800);
        o.set_toolbar_size(400, 34);
        o
    }

    #[test]
    fn defaults_place_left_on_hover_and_right_always_visible() {
        let o = sized();
        let snap = o.snapshot();

        assert_eq!(o.corner_of(PanelId::Components), Corner::Tl);
        assert_eq!(o.corner_of(PanelId::Files), Corner::Tl);
        assert_eq!(o.corner_of(PanelId::Editor), Corner::Tr);
        assert_eq!(o.corner_of(PanelId::SimLog), Corner::Br);
        assert_eq!(o.corner_of(PanelId::CompLog), Corner::Br);

        // Components and Files start in hover mode on TL.
        assert!(!o.is_pinned(PanelId::Components));
        assert!(!o.is_pinned(PanelId::Files));
        assert!(!snap.panels[&PanelId::Components].visible);
        assert!(!snap.panels[&PanelId::Files].visible);

        // Editor (TR) and SimLog + CompLog (BR) start always visible, sharing the right column.
        assert!(o.is_pinned(PanelId::Editor));
        assert!(o.is_pinned(PanelId::SimLog));
        assert!(o.is_pinned(PanelId::CompLog));
        assert!(snap.panels[&PanelId::Editor].visible);
        assert!(!snap.panels[&PanelId::Editor].floating);
        assert!(snap.panels[&PanelId::SimLog].visible);
        assert!(!snap.panels[&PanelId::SimLog].floating);
        assert!(!snap.panels[&PanelId::CompLog].visible);
        assert!(snap.right_split.visible);

        // Left corner strip hugs the left margin; right corner strips sit inside the right column.
        let tl = &snap.corners[Corner::Tl.index()];
        assert_eq!(tl.rect.x, OVERLAY_MARGIN);
        assert_eq!(tl.rect.y, OVERLAY_MARGIN);
        assert_eq!(tl.pinned_rect.h, 0);
        assert_eq!(tl.hover_rect.h, strip_height(2));
        assert_eq!(snap.panels[&PanelId::Components].button.w, BTN_SIZE);
        assert_eq!(snap.panels[&PanelId::Files].button.w, BTN_SIZE);

        let right_x = 1200 - OVERLAY_MARGIN - DEFAULT_RIGHT_WIDTH - PANEL_GAP - STRIP_W;
        let tr = &snap.corners[Corner::Tr.index()];
        assert_eq!(tr.rect.x, right_x);
        assert_eq!(tr.rect.y, OVERLAY_MARGIN);
        assert_eq!(tr.pinned_rect.h, strip_height(1));

        let br = &snap.corners[Corner::Br.index()];
        assert_eq!(br.rect.x, right_x);
        assert_eq!(br.rect.y + br.rect.h, 800 - OVERLAY_MARGIN);
        assert_eq!(br.pinned_rect.h, strip_height(2));
    }

    #[test]
    fn equal_padding_with_custom_titlebar() {
        let mut o = sized();
        assert!(o.set_top_inset(34));
        assert!(!o.set_top_inset(34));

        let expected_top = 34 + OVERLAY_MARGIN;
        assert_eq!(o.snapshot().top_min, expected_top);
        assert_eq!(o.snapshot().toolbar.y, expected_top);
        assert_eq!(
            o.snapshot().corners[Corner::Tl.index()].rect.y,
            expected_top
        );

        o.button_double_clicked(PanelId::Components);
        let comp = &o.snapshot().panels[&PanelId::Components];
        assert_eq!(comp.rect.x, OVERLAY_MARGIN);
        assert_eq!(comp.rect.y, expected_top);
        assert_eq!(comp.rect.h, 800 - expected_top - OVERLAY_MARGIN);
    }

    #[test]
    fn single_click_opens_closes_hover_and_double_click_toggles_pin() {
        let mut o = sized();

        // Single-click in hover mode opens, and clicking again closes (stays hover mode).
        assert_eq!(o.button_clicked(PanelId::Components), (true, false));
        assert!(!o.is_pinned(PanelId::Components));
        assert!(o.snapshot().panels[&PanelId::Components].visible);
        assert!(o.snapshot().panels[&PanelId::Components].floating);
        assert_eq!(o.button_clicked(PanelId::Components), (true, false));
        assert!(!o.snapshot().panels[&PanelId::Components].visible);

        // Double-click Components -> pinned & visible; TL splits into separate pinned & hover pills.
        assert!(o.button_double_clicked(PanelId::Components));
        assert!(o.is_pinned(PanelId::Components));
        assert!(o.snapshot().panels[&PanelId::Components].visible);
        assert!(!o.snapshot().panels[&PanelId::Components].floating);
        let tl_split = &o.snapshot().corners[Corner::Tl.index()];
        assert_eq!(tl_split.pinned_rect.h, strip_height(1));
        assert_eq!(tl_split.hover_rect.h, strip_height(1));
        assert_eq!(
            tl_split.hover_rect.y,
            tl_split.pinned_rect.y + tl_split.pinned_rect.h + GROUP_GAP
        );

        // Hover Files while Components is pinned -> Files floats INSIDE, leaving space for Components + buttons.
        o.set_hover_panel(Some(PanelId::Files));
        let comp = &o.snapshot().panels[&PanelId::Components];
        let files = &o.snapshot().panels[&PanelId::Files];
        let tl_strip = &o.snapshot().corners[Corner::Tl.index()].rect;
        assert!(comp.visible && !comp.floating);
        assert!(files.visible && files.floating);
        assert_eq!(tl_strip.x, comp.rect.x + comp.rect.w + PANEL_GAP);
        assert_eq!(files.rect.x, tl_strip.x + tl_strip.w + PANEL_GAP);

        // Double-click Files -> now BOTH Components and Files are pinned in TL, Files is active.
        assert!(o.button_double_clicked(PanelId::Files));
        assert!(o.is_pinned(PanelId::Components));
        assert!(o.is_pinned(PanelId::Files));
        assert!(!o.snapshot().panels[&PanelId::Components].visible);
        assert!(o.snapshot().panels[&PanelId::Files].visible);
        assert!(!o.snapshot().panels[&PanelId::Files].floating);

        // Single-click inactive pinned Components -> switches active pinned panel back to Components.
        assert_eq!(o.button_clicked(PanelId::Components), (true, true));
        assert!(o.is_pinned(PanelId::Components));
        assert!(o.is_pinned(PanelId::Files));
        assert!(o.snapshot().panels[&PanelId::Components].visible);
        assert!(!o.snapshot().panels[&PanelId::Files].visible);

        // Single-click active docked Components -> collapses it while keeping it docked (does NOT turn on hover).
        assert_eq!(o.button_clicked(PanelId::Components), (true, true));
        assert!(o.is_pinned(PanelId::Components));
        assert!(o.is_pinned(PanelId::Files));
        assert!(!o.snapshot().panels[&PanelId::Components].visible);
        assert!(!o.snapshot().panels[&PanelId::Files].visible);
        // Hovering a collapsed docked button does NOT pop it open.
        assert!(!o.set_hover_panel(Some(PanelId::Components)));
        assert!(!o.snapshot().panels[&PanelId::Components].visible);

        // Single-click collapsed docked Components -> expands it again.
        assert_eq!(o.button_clicked(PanelId::Components), (true, true));
        assert!(o.snapshot().panels[&PanelId::Components].visible);

        // Double-click switches Components and Files back to hover mode.
        assert!(o.button_double_clicked(PanelId::Components));
        assert!(!o.is_pinned(PanelId::Components));
        assert!(o.is_pinned(PanelId::Files));
        assert!(o.snapshot().panels[&PanelId::Files].visible);

        assert!(o.button_double_clicked(PanelId::Files));
        assert!(!o.is_pinned(PanelId::Files));
        assert!(!o.snapshot().panels[&PanelId::Files].visible);
    }

    #[test]
    fn bl_hover_button_moves_inside_when_tl_is_pinned() {
        let mut o = sized();
        o.move_panel_to_corner(PanelId::Files, Corner::Bl);
        o.button_double_clicked(PanelId::Components);

        let snap = o.snapshot();
        let comp = snap.panels[&PanelId::Components].rect;
        let tl_strip = snap.corners[Corner::Tl.index()].rect;
        let bl_strip = snap.corners[Corner::Bl.index()].rect;
        assert_eq!(comp.x, OVERLAY_MARGIN);
        assert_eq!(tl_strip.x, comp.x + comp.w + PANEL_GAP);
        assert_eq!(bl_strip.x, tl_strip.x);

        // Hovering BL panel opens it inside, to the right of the BL strip.
        o.set_hover_panel(Some(PanelId::Files));
        let files = &o.snapshot().panels[&PanelId::Files];
        assert!(files.visible && files.floating);
        assert_eq!(files.rect.x, bl_strip.x + bl_strip.w + PANEL_GAP);
        assert_eq!(files.rect.y + files.rect.h, 800 - OVERLAY_MARGIN);
    }

    #[test]
    fn both_tl_and_bl_pinned_share_column_with_draggable_split_handle() {
        let mut o = sized();
        o.move_panel_to_corner(PanelId::Files, Corner::Bl);
        o.button_double_clicked(PanelId::Components);
        o.button_double_clicked(PanelId::Files);

        let snap = o.snapshot();
        assert!(snap.left_split.visible);
        let tl = &snap.panels[&PanelId::Components].rect;
        let bl = &snap.panels[&PanelId::Files].rect;
        assert_eq!(tl.x, OVERLAY_MARGIN);
        assert_eq!(bl.x, OVERLAY_MARGIN);
        assert_eq!(tl.w, bl.w);
        assert_eq!(tl.y + tl.h, snap.left_split.rect.y);
        assert_eq!(snap.left_split.rect.y + SPLIT_HANDLE_H, bl.y);
        assert_eq!(bl.y + bl.h, 800 - OVERLAY_MARGIN);
        let initial_tl_h = tl.h;

        // Drag split handle down by 100px -> TL grows, BL shrinks.
        let hy = snap.left_split.rect.y + 4;
        o.gesture_begin("leftSplit", 50, hy);
        o.gesture_move(50, hy + 100);
        assert!(o.gesture_end());
        let tl2 = o.snapshot().panels[&PanelId::Components].rect;
        let bl2 = o.snapshot().panels[&PanelId::Files].rect;
        assert!(tl2.h > initial_tl_h + 80);
        assert_eq!(tl2.h + SPLIT_HANDLE_H + bl2.h, 800 - 2 * OVERLAY_MARGIN);
    }

    #[test]
    fn programmatic_open_uses_temporary_focused_floating_for_hover_panels() {
        let mut o = sized();
        let (changed, cfg_changed) = o.open_temporary(PanelId::Files);
        assert!(changed);
        assert!(!cfg_changed);
        assert!(!o.is_pinned(PanelId::Files));
        assert!(o.snapshot().panels[&PanelId::Files].visible);
        assert!(o.snapshot().panels[&PanelId::Files].floating);
        assert_eq!(o.floating_reason(), FloatingReason::Focused);

        // Mouse leave (hover = None) does NOT dismiss a Focused temporary overlay.
        assert!(!o.set_hover_panel(None));
        assert!(o.snapshot().panels[&PanelId::Files].visible);

        // Explicit dismiss (Esc / click canvas) closes it.
        assert!(o.dismiss_floating());
        assert!(!o.snapshot().panels[&PanelId::Files].visible);
    }

    #[test]
    fn config_roundtrip_preserves_corners_pins_widths_and_splits() {
        let mut o = sized();
        o.move_panel_to_corner(PanelId::Files, Corner::Bl);
        o.button_double_clicked(PanelId::Components);
        o.button_double_clicked(PanelId::Files);
        o.gesture_begin("leftResize", 272, 100);
        o.gesture_move(322, 100);
        assert!(o.gesture_end());
        o.gesture_begin("leftSplit", 50, 400);
        o.gesture_move(50, 320);
        assert!(o.gesture_end());

        let cfg = o.to_config();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let parsed: OverlayLayoutConfig = serde_json::from_str(&json).expect("deserialize");
        let mut restored = Overlays::from_config(&parsed);
        restored.set_view_size(1200, 800);

        assert_eq!(restored.corner_of(PanelId::Files), Corner::Bl);
        assert!(restored.is_pinned(PanelId::Components));
        assert!(restored.is_pinned(PanelId::Files));
        assert_eq!(
            restored.snapshot().panels[&PanelId::Components].rect,
            o.snapshot().panels[&PanelId::Components].rect
        );
        assert_eq!(
            restored.snapshot().panels[&PanelId::Files].rect,
            o.snapshot().panels[&PanelId::Files].rect
        );

        // Collapsed docked corner also roundtrips without auto-expanding.
        o.button_clicked(PanelId::Editor); // collapse TR Editor
        assert!(o.is_pinned(PanelId::Editor));
        assert!(!o.snapshot().panels[&PanelId::Editor].visible);
        let cfg2 = o.to_config();
        let parsed2: OverlayLayoutConfig =
            serde_json::from_str(&serde_json::to_string(&cfg2).unwrap()).unwrap();
        let mut restored2 = Overlays::from_config(&parsed2);
        restored2.set_view_size(1200, 800);
        assert!(restored2.is_pinned(PanelId::Editor));
        assert!(!restored2.snapshot().panels[&PanelId::Editor].visible);
    }

    #[test]
    fn info_card_flips_above_toolbar_in_lower_half() {
        let mut o = sized();
        o.set_info_card_size(220, 120);

        let tb_top = o.snapshot().toolbar;
        let ic_top = o.snapshot().info_card;
        assert_eq!(ic_top.y, tb_top.y + tb_top.h + 4);

        o.gesture_begin("toolbar", tb_top.x + 10, tb_top.y + 10);
        o.gesture_move(tb_top.x + 10, tb_top.y + 10 + 450);
        o.gesture_end();
        let tb_bot = o.snapshot().toolbar;
        let ic_bot = o.snapshot().info_card;
        assert_eq!(ic_bot.y, tb_bot.y - ic_bot.h - 4);
    }

    #[test]
    fn toolbar_and_fit_area_follow_docked_overlays() {
        let mut o = sized();

        // Unpin SimLog and CompLog so ONLY Editor is docked on the top right.
        o.set_panel_pinned(PanelId::SimLog, false);
        o.set_panel_pinned(PanelId::CompLog, false);
        assert!(o.is_pinned(PanelId::Editor));

        let snap_right = o.snapshot().clone();
        let expected_clear_left = OVERLAY_MARGIN + STRIP_W + PANEL_GAP;
        let expected_clear_right =
            1200 - OVERLAY_MARGIN - DEFAULT_RIGHT_WIDTH - PANEL_GAP - STRIP_W - PANEL_GAP;
        assert_eq!(snap_right.available_area.x, expected_clear_left);
        assert_eq!(
            snap_right.available_area.x + snap_right.available_area.w,
            expected_clear_right
        );

        // Toolbar is centered in the left-shifted available area (closer to left edge).
        let avail_center = snap_right.available_area.x + snap_right.available_area.w / 2;
        let tb_center = snap_right.toolbar.x + snap_right.toolbar.w / 2;
        assert!((tb_center - avail_center).abs() <= 1);
        assert!(
            tb_center < 600,
            "toolbar should sit left of window center when only right editor is docked"
        );

        // Fit area shares horizontal bounds and clears the top toolbar vertically.
        assert_eq!(snap_right.fit_area.x, snap_right.available_area.x);
        assert_eq!(snap_right.fit_area.w, snap_right.available_area.w);
        assert_eq!(
            snap_right.fit_area.y,
            snap_right.toolbar.y + snap_right.toolbar.h + PANEL_GAP
        );

        // Floating hover overlay on the left must NOT shift the toolbar or shrink available_area.
        o.set_hover_panel(Some(PanelId::Components));
        assert_eq!(o.snapshot().available_area, snap_right.available_area);
        assert_eq!(o.snapshot().toolbar, snap_right.toolbar);
        assert_eq!(o.snapshot().fit_area, snap_right.fit_area);
        o.set_hover_panel(None);

        // Collapsing the right docked Editor restores full-width available area and centers toolbar.
        o.button_clicked(PanelId::Editor);
        let snap_none = o.snapshot().clone();
        assert_eq!(
            snap_none.toolbar.x + snap_none.toolbar.w / 2,
            600,
            "toolbar should center in full window when no overlay is docked"
        );
        assert!(snap_none.available_area.w > snap_right.available_area.w);

        // Docking ONLY Components on the top left shifts toolbar closer to the right edge.
        o.set_panel_pinned(PanelId::Components, true);
        let snap_left = o.snapshot().clone();
        let left_tb_center = snap_left.toolbar.x + snap_left.toolbar.w / 2;
        assert!(
            left_tb_center > 600,
            "toolbar should sit right of window center when only left panel is docked"
        );
        assert_eq!(
            snap_left.available_area.x,
            OVERLAY_MARGIN + DEFAULT_LEFT_WIDTH + PANEL_GAP + STRIP_W + PANEL_GAP
        );
    }

    #[test]
    fn dragged_toolbar_clamps_out_of_docked_overlay() {
        let mut o = sized();
        o.set_panel_pinned(PanelId::Editor, false);
        o.set_panel_pinned(PanelId::SimLog, false);
        o.set_panel_pinned(PanelId::CompLog, false);

        // Drag toolbar far to the right while nothing is docked.
        let tb = o.snapshot().toolbar;
        o.gesture_begin("toolbar", tb.x + 10, tb.y + 10);
        o.gesture_move(tb.x + 10 + 500, tb.y + 10);
        o.gesture_end();
        let dragged_right_x = o.snapshot().toolbar.x;
        assert!(dragged_right_x > 650);

        // Docking Editor on the right clamps the dragged toolbar inside the available area.
        o.set_panel_pinned(PanelId::Editor, true);
        let snap = o.snapshot();
        assert!(
            snap.toolbar.x + snap.toolbar.w <= snap.available_area.x + snap.available_area.w,
            "dragged toolbar should not overlap right docked overlay"
        );
    }
}
