//! Overlay geometry and drag/resize gestures.
//!
//! Matches `CircuitWidget::updateOverlayLayout` / `gestureBegin` / `gestureMove`
//! / `gestureEnd`. No Qt: the QML panel binds to the published rects.

use std::collections::BTreeMap;

pub const HANDLE_BTN: i32 = 24;
pub const HANDLE_GAP: i32 = 4;
pub const OVERLAY_MARGIN: i32 = 12;
pub const DEFAULT_SIDE_WIDTH: i32 = 250;
pub const DEFAULT_EDITOR_WIDTH: i32 = 350;
const MIN_PANEL: i32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayId {
    SidePanel,
    EditorPanel,
    Toolbar,
    InfoCard,
}

impl OverlayId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SidePanel => "sidePanel",
            Self::EditorPanel => "editorPanel",
            Self::Toolbar => "toolbar",
            Self::InfoCard => "infoCard",
        }
    }

    fn from_click(id: &str) -> Option<Self> {
        if id == "sidePanel" || id.starts_with("sidePanel") || id.starts_with("sp") {
            Some(Self::SidePanel)
        } else if id == "editorPanel" || id.starts_with("editorPanel") || id.starts_with("ep") {
            Some(Self::EditorPanel)
        } else if id == "toolbar" {
            Some(Self::Toolbar)
        } else if id == "infoCard" {
            Some(Self::InfoCard)
        } else {
            None
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

    pub fn center_x(self) -> i32 {
        self.x + self.w / 2
    }

    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayEntry {
    Rect(IRect),
    Flag(bool),
    Int(i32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OverlaySnapshot {
    pub entries: BTreeMap<String, OverlayEntry>,
}

impl OverlaySnapshot {
    fn put_rect(&mut self, key: &str, x: i32, y: i32, w: i32, h: i32) {
        self.entries
            .insert(key.to_string(), OverlayEntry::Rect(IRect::new(x, y, w, h)));
    }

    fn put_flag(&mut self, key: &str, v: bool) {
        self.entries.insert(key.to_string(), OverlayEntry::Flag(v));
    }

    fn put_int(&mut self, key: &str, v: i32) {
        self.entries.insert(key.to_string(), OverlayEntry::Int(v));
    }

    pub fn rect(&self, key: &str) -> IRect {
        match self.entries.get(key) {
            Some(OverlayEntry::Rect(r)) => *r,
            _ => IRect::default(),
        }
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(self.entries.get(key), Some(OverlayEntry::Flag(true)))
    }
}

#[derive(Clone, Debug, Default)]
struct Gesture {
    id: String,
    start_mouse: (i32, i32),
    start_pos: (i32, i32),
    start_w: i32,
    start_h: i32,
    side_right: bool,
    side_left: bool,
    side_bottom: bool,
    side_top: bool,
    editor_right: bool,
    editor_left: bool,
    editor_bottom: bool,
    editor_top: bool,
    drag_side: bool,
    drag_editor: bool,
    drag_toolbar: bool,
}

/// Stored overlay state. Layout is a pure function of this plus the last
/// published rects (for drag start).
#[derive(Clone, Debug)]
pub struct Overlays {
    view_w: i32,
    view_h: i32,
    toolbar_w: i32,
    toolbar_h: i32,
    info_w: i32,
    info_h: i32,
    side_shown: bool,
    editor_shown: bool,
    info_shown: bool,
    side_w: i32,
    side_h: i32,
    side_pos: (i32, i32),
    side_cluster_right: bool,
    editor_w: i32,
    editor_h: i32,
    editor_pos: (i32, i32),
    editor_cluster_right: bool,
    toolbar_pos: (i32, i32),
    order: Vec<OverlayId>,
    gesture: Gesture,
    snapshot: OverlaySnapshot,
}

impl Default for Overlays {
    fn default() -> Self {
        Self {
            view_w: 0,
            view_h: 0,
            toolbar_w: 0,
            toolbar_h: 0,
            info_w: 0,
            info_h: 0,
            side_shown: true,
            editor_shown: false,
            info_shown: false,
            side_w: DEFAULT_SIDE_WIDTH,
            side_h: -1,
            side_pos: (-1, -1),
            side_cluster_right: true,
            editor_w: DEFAULT_EDITOR_WIDTH,
            editor_h: -1,
            editor_pos: (-1, -1),
            editor_cluster_right: false,
            toolbar_pos: (-1, -1),
            order: vec![
                OverlayId::SidePanel,
                OverlayId::EditorPanel,
                OverlayId::Toolbar,
                OverlayId::InfoCard,
            ],
            gesture: Gesture::default(),
            snapshot: OverlaySnapshot::default(),
        }
    }
}

impl Overlays {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.relayout();
        s
    }

    pub fn snapshot(&self) -> &OverlaySnapshot {
        &self.snapshot
    }

    pub fn side_shown(&self) -> bool {
        self.side_shown
    }

    pub fn editor_shown(&self) -> bool {
        self.editor_shown
    }

    pub fn info_shown(&self) -> bool {
        self.info_shown
    }

    pub fn set_info_shown(&mut self, shown: bool) -> bool {
        if self.info_shown == shown {
            return false;
        }
        self.info_shown = shown;
        if shown {
            self.bring_to_front(OverlayId::InfoCard);
        }
        self.relayout();
        true
    }

    pub fn overlay_z(&self, id: OverlayId) -> i32 {
        self.order
            .iter()
            .position(|o| *o == id)
            .map(|i| i as i32 + 1)
            .unwrap_or(0)
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

    pub fn toggle_panel(&mut self, id: &str) -> bool {
        match id {
            "sidePanel" => {
                self.side_shown = !self.side_shown;
                if self.side_shown {
                    self.update_side_cluster();
                    self.bring_to_front(OverlayId::SidePanel);
                }
                self.relayout();
                true
            }
            "editorPanel" => {
                self.editor_shown = !self.editor_shown;
                if self.editor_shown {
                    self.update_editor_cluster();
                    self.bring_to_front(OverlayId::EditorPanel);
                }
                self.relayout();
                true
            }
            _ => false,
        }
    }

    pub fn set_side_shown(&mut self, shown: bool) -> bool {
        if self.side_shown == shown {
            return false;
        }
        self.side_shown = shown;
        if shown {
            self.update_side_cluster();
            self.bring_to_front(OverlayId::SidePanel);
        }
        self.relayout();
        true
    }

    pub fn set_editor_shown(&mut self, shown: bool) -> bool {
        if self.editor_shown == shown {
            return false;
        }
        self.editor_shown = shown;
        if shown {
            self.update_editor_cluster();
            self.bring_to_front(OverlayId::EditorPanel);
        }
        self.relayout();
        true
    }

    pub fn overlay_clicked(&mut self, id: &str) -> bool {
        if let Some(which) = OverlayId::from_click(id) {
            if self.bring_to_front(which) {
                self.relayout();
                return true;
            }
        }
        false
    }

    pub fn toggle_active_overlay(&mut self) -> bool {
        for &id in self.order.iter().rev() {
            match id {
                OverlayId::SidePanel => {
                    self.side_shown = !self.side_shown;
                    if self.side_shown {
                        self.update_side_cluster();
                        self.bring_to_front(OverlayId::SidePanel);
                    }
                    self.relayout();
                    return true;
                }
                OverlayId::EditorPanel => {
                    self.editor_shown = !self.editor_shown;
                    if self.editor_shown {
                        self.update_editor_cluster();
                        self.bring_to_front(OverlayId::EditorPanel);
                    }
                    self.relayout();
                    return true;
                }
                OverlayId::InfoCard => {
                    self.info_shown = !self.info_shown;
                    if self.info_shown {
                        self.bring_to_front(OverlayId::InfoCard);
                    }
                    self.relayout();
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub fn collapse_active_overlay(&mut self) -> bool {
        self.toggle_active_overlay()
    }

    pub fn reset_pos(&mut self, id: &str) -> bool {
        match id {
            "sidePanel" => self.side_pos = (-1, -1),
            "editorPanel" => self.editor_pos = (-1, -1),
            "toolbar" => self.toolbar_pos = (-1, -1),
            _ => return false,
        }
        self.relayout();
        true
    }

    pub fn reset_size(&mut self, id: &str) -> bool {
        match id {
            "sidePanel" => {
                self.side_w = DEFAULT_SIDE_WIDTH;
                self.side_h = -1;
            }
            "editorPanel" => {
                self.editor_w = DEFAULT_EDITOR_WIDTH;
                self.editor_h = -1;
            }
            _ => return false,
        }
        self.relayout();
        true
    }

    pub fn gesture_begin(&mut self, id: &str, x: i32, y: i32) {
        self.gesture = Gesture {
            id: id.to_string(),
            start_mouse: (x, y),
            ..Gesture::default()
        };

        if id.starts_with("sidePanel") {
            self.bring_to_front(OverlayId::SidePanel);
        } else if id.starts_with("editorPanel") {
            self.bring_to_front(OverlayId::EditorPanel);
        } else if id == "toolbar" {
            self.bring_to_front(OverlayId::Toolbar);
        } else if id == "infoCard" {
            self.bring_to_front(OverlayId::InfoCard);
        }

        let sp = self.snapshot.rect("sidePanel");
        let ep = self.snapshot.rect("editorPanel");
        let tb = self.snapshot.rect("toolbar");

        match id {
            "sidePanelRight" => {
                self.gesture.start_w = self.side_w;
                self.gesture.side_right = true;
            }
            "sidePanelBottom" => {
                self.gesture.start_h = if self.side_h > 0 { self.side_h } else { sp.h };
                self.gesture.side_bottom = true;
            }
            "sidePanelLeft" => {
                self.gesture.start_w = self.side_w;
                self.gesture.start_pos = sp.top_left();
                self.gesture.side_left = true;
            }
            "sidePanelTop" => {
                self.gesture.start_h = sp.h;
                self.gesture.start_pos = sp.top_left();
                self.gesture.side_top = true;
            }
            "sidePanelTL" | "sidePanelTR" | "sidePanelBL" | "sidePanelBR" => {
                self.gesture.start_w = self.side_w;
                self.gesture.start_pos = sp.top_left();
                if id.ends_with('L') {
                    self.gesture.side_left = true;
                } else {
                    self.gesture.side_right = true;
                }
                if id.starts_with("sidePanelT") {
                    self.gesture.start_h = sp.h;
                    self.gesture.side_top = true;
                } else {
                    self.gesture.start_h = if self.side_h > 0 { self.side_h } else { sp.h };
                    self.gesture.side_bottom = true;
                }
            }
            "sidePanel" => {
                self.gesture.start_pos = sp.top_left();
                self.gesture.drag_side = true;
            }
            "editorPanelLeft" => {
                self.gesture.start_w = self.editor_w;
                self.gesture.start_pos = ep.top_left();
                self.gesture.editor_left = true;
            }
            "editorPanelBottom" => {
                self.gesture.start_h = if self.editor_h > 0 {
                    self.editor_h
                } else {
                    ep.h
                };
                self.gesture.editor_bottom = true;
            }
            "editorPanelRight" => {
                self.gesture.start_w = self.editor_w;
                self.gesture.editor_right = true;
            }
            "editorPanelTop" => {
                self.gesture.start_h = ep.h;
                self.gesture.start_pos = ep.top_left();
                self.gesture.editor_top = true;
            }
            "editorPanelTL" | "editorPanelTR" | "editorPanelBL" | "editorPanelBR" => {
                self.gesture.start_w = self.editor_w;
                self.gesture.start_pos = ep.top_left();
                if id.ends_with('L') {
                    self.gesture.editor_left = true;
                } else {
                    self.gesture.editor_right = true;
                }
                if id.starts_with("editorPanelT") {
                    self.gesture.start_h = ep.h;
                    self.gesture.editor_top = true;
                } else {
                    self.gesture.start_h = if self.editor_h > 0 {
                        self.editor_h
                    } else {
                        ep.h
                    };
                    self.gesture.editor_bottom = true;
                }
            }
            "editorPanel" => {
                self.gesture.start_pos = ep.top_left();
                self.gesture.drag_editor = true;
            }
            "toolbar" => {
                self.gesture.start_pos = tb.top_left();
                self.gesture.drag_toolbar = true;
            }
            _ => {}
        }
        self.relayout();
    }

    pub fn gesture_move(&mut self, x: i32, y: i32) -> bool {
        if self.gesture.id.is_empty() {
            return false;
        }
        let dx = x - self.gesture.start_mouse.0;
        let dy = y - self.gesture.start_mouse.1;
        let margin = OVERLAY_MARGIN;
        let view_w = self.view_w;
        let view_h = self.view_h;
        let g = self.gesture.clone();

        let resize = |right: bool,
                      left: bool,
                      bottom: bool,
                      top: bool,
                      width: &mut i32,
                      height: &mut i32,
                      panel_pos: &mut (i32, i32)| {
            let mut p = g.start_pos;
            if right {
                let w = g.start_w + dx;
                if w > MIN_PANEL && w < view_w - MIN_PANEL {
                    *width = w;
                }
            } else if left {
                let w = g.start_w - dx;
                if w > MIN_PANEL && w < view_w - MIN_PANEL {
                    *width = w;
                    p.0 += dx;
                }
            }
            if bottom {
                let h = g.start_h + dy;
                if h > MIN_PANEL && h < view_h - margin {
                    *height = h;
                }
            } else if top {
                let h = g.start_h - dy;
                if h > MIN_PANEL && h < view_h - margin {
                    *height = h;
                    p.1 += dy;
                }
            }
            if left || top {
                *panel_pos = p;
            }
        };

        if g.side_right || g.side_left || g.side_bottom || g.side_top {
            resize(
                g.side_right,
                g.side_left,
                g.side_bottom,
                g.side_top,
                &mut self.side_w,
                &mut self.side_h,
                &mut self.side_pos,
            );
        } else if g.editor_right || g.editor_left || g.editor_bottom || g.editor_top {
            resize(
                g.editor_right,
                g.editor_left,
                g.editor_bottom,
                g.editor_top,
                &mut self.editor_w,
                &mut self.editor_h,
                &mut self.editor_pos,
            );
        } else if g.drag_side {
            self.side_pos = (g.start_pos.0 + dx, g.start_pos.1 + dy);
        } else if g.drag_editor {
            self.editor_pos = (g.start_pos.0 + dx, g.start_pos.1 + dy);
        } else if g.drag_toolbar {
            self.toolbar_pos = (g.start_pos.0 + dx, g.start_pos.1 + dy);
        } else {
            return false;
        }
        self.relayout();
        true
    }

    pub fn gesture_end(&mut self) {
        if self.gesture.id.starts_with("sidePanel") {
            self.update_side_cluster();
        } else if self.gesture.id.starts_with("editorPanel") {
            self.update_editor_cluster();
        }
        self.gesture = Gesture::default();
        self.relayout();
    }

    fn bring_to_front(&mut self, id: OverlayId) -> bool {
        if self.order.last() == Some(&id) {
            return false;
        }
        self.order.retain(|o| *o != id);
        self.order.push(id);
        true
    }

    fn update_side_cluster(&mut self) {
        if !self.side_shown {
            return;
        }
        self.side_cluster_right = self.snapshot.rect("sidePanel").center_x() <= self.view_w / 2;
    }

    fn update_editor_cluster(&mut self) {
        if !self.editor_shown {
            return;
        }
        self.editor_cluster_right = self.snapshot.rect("editorPanel").center_x() <= self.view_w / 2;
    }

    fn relayout(&mut self) {
        self.snapshot = layout(self);
    }
}

fn clamp_i(v: i32, lo: i32, hi: i32) -> i32 {
    if lo <= hi { v.clamp(lo, hi) } else { lo }
}

fn cluster_x(px: i32, w: i32, right: bool, gap: i32, btn: i32) -> i32 {
    if right { px + w + gap } else { px - gap - btn }
}

fn layout(s: &Overlays) -> OverlaySnapshot {
    let margin = OVERLAY_MARGIN;
    let btn = HANDLE_BTN;
    let gap = HANDLE_GAP;
    let bounds_h = (s.view_h - 2 * margin).max(0);

    let sp_default = s.side_pos.0 < 0;
    let ep_default = s.editor_pos.0 < 0;

    let sp_w = s.side_w;
    let sp_h = if s.side_h > 0 {
        s.side_h.min(bounds_h)
    } else {
        bounds_h
    };
    let (mut sp_px, mut sp_py) = (margin, margin);
    if !sp_default {
        let max_x = margin.max(s.view_w - margin - sp_w);
        let max_y = margin.max(s.view_h - margin - sp_h);
        sp_px = clamp_i(s.side_pos.0, margin, max_x);
        sp_py = clamp_i(s.side_pos.1, margin, max_y);
    }

    let ep_w = s.editor_w;
    let ep_h = if s.editor_h > 0 {
        s.editor_h.min(bounds_h)
    } else {
        bounds_h
    };
    let (mut ep_px, mut ep_py) = (s.view_w - margin - ep_w, margin);
    if !ep_default {
        let max_x = margin.max(s.view_w - margin - ep_w);
        let max_y = margin.max(s.view_h - margin - ep_h);
        ep_px = clamp_i(s.editor_pos.0, margin, max_x);
        ep_py = clamp_i(s.editor_pos.1, margin, max_y);
    }

    let tb_w = s.toolbar_w;
    let tb_h = s.toolbar_h;
    let (tb_x, tb_y) = if s.toolbar_pos.0 >= 0 {
        let max_x = margin.max(s.view_w - margin - tb_w);
        let max_y = margin.max(s.view_h - margin - tb_h);
        (
            clamp_i(s.toolbar_pos.0, margin, max_x),
            clamp_i(s.toolbar_pos.1, margin, max_y),
        )
    } else {
        ((s.view_w - tb_w) / 2, margin)
    };

    let ic_w = s.info_w;
    let ic_h = s.info_h;
    let mut ic_x = tb_x + tb_w - ic_w;
    let mut ic_y = tb_y + tb_h + 4;
    {
        let max_x = margin.max(s.view_w - margin - ic_w);
        let max_y = margin.max(s.view_h - margin - ic_h);
        ic_x = clamp_i(ic_x, margin, max_x);
        ic_y = clamp_i(ic_y, margin, max_y);
    }

    let mut rects = OverlaySnapshot::default();
    let put = |r: &mut OverlaySnapshot, k: &str, x: i32, y: i32, w: i32, h: i32| {
        r.put_rect(k, x, y, w, h);
    };
    put(&mut rects, "sidePanel", sp_px, sp_py, sp_w, sp_h);
    put(&mut rects, "editorPanel", ep_px, ep_py, ep_w, ep_h);
    put(&mut rects, "toolbar", tb_x, tb_y, tb_w, tb_h);
    put(&mut rects, "infoCard", ic_x, ic_y, ic_w, ic_h);

    put(
        &mut rects,
        "spResizeRight",
        sp_px + sp_w - 2,
        sp_py,
        5,
        sp_h,
    );
    put(
        &mut rects,
        "spResizeBottom",
        sp_px,
        sp_py + sp_h - 2,
        sp_w,
        5,
    );
    put(&mut rects, "spResizeLeft", sp_px - 3, sp_py, 5, sp_h);
    put(&mut rects, "spResizeTop", sp_px, sp_py - 3, sp_w, 5);
    put(&mut rects, "epResizeLeft", ep_px - 3, ep_py, 5, ep_h);
    put(
        &mut rects,
        "epResizeBottom",
        ep_px,
        ep_py + ep_h - 2,
        ep_w,
        5,
    );
    put(
        &mut rects,
        "epResizeRight",
        ep_px + ep_w - 2,
        ep_py,
        5,
        ep_h,
    );
    put(&mut rects, "epResizeTop", ep_px, ep_py - 3, ep_w, 5);

    let corner = 10;
    put(
        &mut rects,
        "spResizeTL",
        sp_px - 3,
        sp_py - 3,
        corner,
        corner,
    );
    put(
        &mut rects,
        "spResizeTR",
        sp_px + sp_w - 7,
        sp_py - 3,
        corner,
        corner,
    );
    put(
        &mut rects,
        "spResizeBL",
        sp_px - 3,
        sp_py + sp_h - 7,
        corner,
        corner,
    );
    put(
        &mut rects,
        "spResizeBR",
        sp_px + sp_w - 7,
        sp_py + sp_h - 7,
        corner,
        corner,
    );
    put(
        &mut rects,
        "epResizeTL",
        ep_px - 3,
        ep_py - 3,
        corner,
        corner,
    );
    put(
        &mut rects,
        "epResizeTR",
        ep_px + ep_w - 7,
        ep_py - 3,
        corner,
        corner,
    );
    put(
        &mut rects,
        "epResizeBL",
        ep_px - 3,
        ep_py + ep_h - 7,
        corner,
        corner,
    );
    put(
        &mut rects,
        "epResizeBR",
        ep_px + ep_w - 7,
        ep_py + ep_h - 7,
        corner,
        corner,
    );

    let sp_hx = cluster_x(sp_px, sp_w, s.side_cluster_right, gap, btn);
    put(&mut rects, "spDragHandle", sp_hx, sp_py, btn, btn);
    if s.side_shown {
        put(&mut rects, "spToggle", sp_hx, sp_py + btn + gap, btn, btn);
    } else {
        let x = if s.side_cluster_right {
            margin
        } else {
            s.view_w - margin - btn
        };
        put(&mut rects, "spToggle", x, margin, btn, btn);
    }

    let ep_hx = cluster_x(ep_px, ep_w, s.editor_cluster_right, gap, btn);
    put(&mut rects, "epDragHandle", ep_hx, ep_py, btn, btn);
    if s.editor_shown {
        put(&mut rects, "epToggle", ep_hx, ep_py + btn + gap, btn, btn);
    } else {
        let x = if s.editor_cluster_right {
            margin
        } else {
            s.view_w - margin - btn
        };
        put(&mut rects, "epToggle", x, margin, btn, btn);
    }

    rects.put_flag("sidePanelVisible", s.side_shown);
    rects.put_flag("editorPanelVisible", s.editor_shown);
    rects.put_flag("sidePanelClusterRight", s.side_cluster_right);
    rects.put_flag("editorPanelClusterRight", s.editor_cluster_right);
    rects.put_int("sidePanelZ", s.overlay_z(OverlayId::SidePanel));
    rects.put_int("editorPanelZ", s.overlay_z(OverlayId::EditorPanel));
    rects.put_int("toolbarZ", s.overlay_z(OverlayId::Toolbar));
    rects.put_int("infoCardZ", s.overlay_z(OverlayId::InfoCard));
    rects.put_int("margin", margin);
    rects.put_int("handleBtn", btn);
    rects.put_int("handleGap", gap);
    rects
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
    fn default_side_panel_is_left_anchored() {
        let o = sized();
        let sp = o.snapshot().rect("sidePanel");
        assert_eq!(sp.x, OVERLAY_MARGIN);
        assert_eq!(sp.y, OVERLAY_MARGIN);
        assert_eq!(sp.w, DEFAULT_SIDE_WIDTH);
        assert_eq!(sp.h, 800 - 2 * OVERLAY_MARGIN);
        assert!(o.snapshot().flag("sidePanelVisible"));
        assert!(!o.snapshot().flag("editorPanelVisible"));
    }

    #[test]
    fn default_editor_is_right_anchored_when_shown() {
        let mut o = sized();
        o.set_editor_shown(true);
        let ep = o.snapshot().rect("editorPanel");
        assert_eq!(ep.w, DEFAULT_EDITOR_WIDTH);
        assert_eq!(ep.x, 1200 - OVERLAY_MARGIN - DEFAULT_EDITOR_WIDTH);
        assert_eq!(ep.y, OVERLAY_MARGIN);
        assert!(o.snapshot().flag("editorPanelVisible"));
    }

    #[test]
    fn toolbar_centers_until_dragged() {
        let o = sized();
        let tb = o.snapshot().rect("toolbar");
        assert_eq!(tb.x, (1200 - 400) / 2);
        assert_eq!(tb.y, OVERLAY_MARGIN);
        assert_eq!(tb.w, 400);
    }

    #[test]
    fn click_raises_z() {
        let mut o = sized();
        let z0 = o.overlay_z(OverlayId::SidePanel);
        o.overlay_clicked("sidePanel");
        let z1 = o.overlay_z(OverlayId::SidePanel);
        assert!(z1 > z0);
        assert_eq!(z1, 4);
    }

    #[test]
    fn drag_side_panel_leaves_default_anchor() {
        let mut o = sized();
        o.gesture_begin("sidePanel", 20, 20);
        o.gesture_move(40, 50);
        o.gesture_end();
        let sp = o.snapshot().rect("sidePanel");
        // Full-height panel: maxY == margin, so Y cannot leave the default
        // anchor. X can. Matches CircuitWidget::updateOverlayLayout.
        assert_eq!(sp.x, OVERLAY_MARGIN + 20);
        assert_eq!(sp.y, OVERLAY_MARGIN);
    }

    #[test]
    fn resize_right_grows_width() {
        let mut o = sized();
        o.gesture_begin("sidePanelRight", 262, 100);
        o.gesture_move(312, 100);
        o.gesture_end();
        assert_eq!(o.snapshot().rect("sidePanel").w, DEFAULT_SIDE_WIDTH + 50);
    }

    #[test]
    fn reset_pos_restores_default_anchor() {
        let mut o = sized();
        o.gesture_begin("sidePanel", 20, 20);
        o.gesture_move(80, 80);
        o.gesture_end();
        o.reset_pos("sidePanel");
        let sp = o.snapshot().rect("sidePanel");
        assert_eq!(sp.x, OVERLAY_MARGIN);
        assert_eq!(sp.y, OVERLAY_MARGIN);
    }

    #[test]
    fn hidden_toggle_goes_to_margin() {
        let mut o = sized();
        o.toggle_panel("sidePanel");
        assert!(!o.side_shown());
        let t = o.snapshot().rect("spToggle");
        assert_eq!(t.w, HANDLE_BTN);
        assert_eq!(t.x, OVERLAY_MARGIN);
        assert_eq!(t.y, OVERLAY_MARGIN);
    }

    #[test]
    fn overlay_click_handle_ids() {
        let mut o = sized();
        assert_eq!(o.overlay_z(OverlayId::SidePanel), 1);
        assert_eq!(o.overlay_z(OverlayId::Toolbar), 3);

        // Clicking toolbar brings it to front
        o.overlay_clicked("toolbar");
        assert_eq!(o.overlay_z(OverlayId::Toolbar), 4);
        assert_eq!(o.overlay_z(OverlayId::SidePanel), 1);

        // Clicking spDragHandle brings SidePanel to front
        o.overlay_clicked("spDragHandle");
        assert_eq!(o.overlay_z(OverlayId::SidePanel), 4);
        assert_eq!(o.overlay_z(OverlayId::Toolbar), 3);

        // Clicking editorPanel brings EditorPanel to front
        o.set_editor_shown(true);
        o.overlay_clicked("editorPanel");
        assert_eq!(o.overlay_z(OverlayId::EditorPanel), 4);
    }

    #[test]
    fn set_side_shown_unhides_and_hides() {
        let mut o = sized();
        assert!(o.side_shown());
        assert!(o.snapshot().flag("sidePanelVisible"));

        // Hide side panel
        assert!(o.set_side_shown(false));
        assert!(!o.side_shown());
        assert!(!o.snapshot().flag("sidePanelVisible"));

        // Redundant hide returns false
        assert!(!o.set_side_shown(false));

        // Unhide side panel
        assert!(o.set_side_shown(true));
        assert!(o.side_shown());
        assert!(o.snapshot().flag("sidePanelVisible"));
        assert_eq!(o.overlay_z(OverlayId::SidePanel), 4);
    }

    #[test]
    fn test_toggle_active_overlay() {
        let mut o = sized();
        // Initially, side panel is shown (visible) and editor panel is hidden.
        assert!(o.side_shown());
        assert!(!o.editor_shown());

        // Side panel was interacted with last: toggle collapses it
        o.overlay_clicked("sidePanel");
        assert!(o.toggle_active_overlay());
        assert!(!o.side_shown());
        assert!(!o.snapshot().flag("sidePanelVisible"));

        // Toggle again: expands side panel back
        assert!(o.toggle_active_overlay());
        assert!(o.side_shown());
        assert!(o.snapshot().flag("sidePanelVisible"));

        // Open editor panel and click it (becomes top of MRU stack)
        o.set_editor_shown(true);
        o.overlay_clicked("editorPanel");
        assert!(o.toggle_active_overlay());
        assert!(!o.editor_shown());

        // Toggle expands editor panel back
        assert!(o.toggle_active_overlay());
        assert!(o.editor_shown());
    }
}
