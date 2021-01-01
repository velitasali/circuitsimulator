//! Canvas event dispatch: mouse, keyboard, wheel, pinch-zoom, and cursor management.

use crate::canvas::drag::{Drag, WireEditMode};
use crate::canvas::geom::to_grid as snap_point;
use crate::canvas::scene::Part;
use crate::canvas::wire::WireHit;
use crate::canvas::{Canvas, Point, Rect};
use crate::components::ComponentChange;

pub const BUTTON_LEFT: u32 = 1;
pub const BUTTON_RIGHT: u32 = 2;
pub const BUTTON_MIDDLE: u32 = 4;

/// Qt keyboard modifiers.
pub const MOD_SHIFT: u32 = 0x0200_0000;
pub const MOD_CTRL: u32 = 0x0400_0000;
pub const MOD_ALT: u32 = 0x0800_0000;

/// Qt key codes used by the canvas.
pub const KEY_ESCAPE: i32 = 0x0100_0000;
pub const KEY_BACKSPACE: i32 = 0x0100_0003;
pub const KEY_DELETE: i32 = 0x0100_0007;
pub const KEY_PLUS: i32 = 0x2b;
pub const KEY_MINUS: i32 = 0x2d;
pub const KEY_0: i32 = 0x30;
pub const KEY_EQUAL: i32 = 0x3d;
pub const KEY_A: i32 = 0x41;
pub const KEY_C: i32 = 0x43;
pub const KEY_L: i32 = 0x4c;
pub const KEY_R: i32 = 0x52;
pub const KEY_V: i32 = 0x56;
pub const KEY_LEFT: i32 = 0x0100_0012;
pub const KEY_UP: i32 = 0x0100_0013;
pub const KEY_RIGHT: i32 = 0x0100_0014;
pub const KEY_DOWN: i32 = 0x0100_0015;
pub const KEY_X: i32 = 0x58;
pub const KEY_Y: i32 = 0x59;
pub const KEY_Z: i32 = 0x5a;

/// Qt `MetaModifier` (Command on macOS).
pub const MOD_META: u32 = 0x1000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorKind {
    Arrow,
    OpenHand,
    ClosedHand,
    Cross,
    SplitH,
    SplitV,
    SizeAll,
}

impl CursorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::OpenHand => "openhand",
            Self::ClosedHand => "closedhand",
            Self::Cross => "cross",
            Self::SplitH => "splith",
            Self::SplitV => "splitv",
            Self::SizeAll => "sizeall",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Change {
    pub zoom: bool,
    pub center: bool,
    pub viewport: bool,
    pub band: bool,
    /// QML `CircuitCanvas.items` JSON / selection notify. Not pixel invalidation.
    pub items: bool,
    pub wires: bool,
    pub cursor: bool,
    pub sim: bool,
    pub anim: bool,
    pub history: bool,
    pub props: bool,
    pub open_props: bool,
    pub plots: bool,
    pub settings: bool,
    pub open_scope: bool,
    pub open_la: bool,
    pub open_terminal: bool,
    pub open_serial_mon: Option<(String, String)>,
    pub hovered_pin: bool,
    pub pause_sim: bool,
}

impl Change {
    pub fn merge(&mut self, other: Change) {
        self.zoom |= other.zoom;
        self.center |= other.center;
        self.viewport |= other.viewport;
        self.band |= other.band;
        self.items |= other.items;
        self.wires |= other.wires;
        self.cursor |= other.cursor;
        self.sim |= other.sim;
        self.anim |= other.anim;
        self.history |= other.history;
        self.props |= other.props;
        self.open_props |= other.open_props;
        self.plots |= other.plots;
        self.settings |= other.settings;
        self.open_scope |= other.open_scope;
        self.open_la |= other.open_la;
        self.open_terminal |= other.open_terminal;
        if other.open_serial_mon.is_some() {
            self.open_serial_mon = other.open_serial_mon;
        }
        self.hovered_pin |= other.hovered_pin;
        self.pause_sim |= other.pause_sim;
    }

    pub(crate) fn edit() -> Self {
        Self {
            items: true,
            wires: true,
            history: true,
            props: true,
            settings: true,
            ..Self::default()
        }
    }

    /// Map a component mutation onto canvas flags. `history` notifies QML
    /// `CircuitCanvas.modified` (`to_sim1` vs `saved_sim1` remains the bool).
    /// `items` is the QML items JSON model, not DirtySet raster.
    pub fn from_component(cc: &ComponentChange) -> Self {
        Self {
            items: cc.view.rebuilds_qml_item(),
            wires: cc.structural,
            history: cc.undo,
            props: cc.saved && cc.view.rebuilds_qml_item(),
            ..Self::default()
        }
    }

    pub(crate) fn without_qml_model(mut self) -> Self {
        self.items = false;
        self.wires = false;
        self
    }

    pub(crate) fn viewport_all() -> Self {
        Self {
            zoom: true,
            center: true,
            viewport: true,
            ..Self::default()
        }
    }
}

impl Canvas {
    pub fn cmd(mods: u32) -> bool {
        mods & (MOD_CTRL | MOD_META) != 0
    }

    pub fn mouse_press(&mut self, button: u32, item_x: f64, item_y: f64, mods: u32) -> Change {
        let item = Point::new(item_x, item_y);
        let scene = self.viewport.map_to_circuit(item);
        self.last_item = item;
        self.last_scene = scene;
        self.cursor_moved = true;
        let hit = self.scene.hit(scene);
        let pin = self.scene.hit_pin(scene);
        let shift = mods & MOD_SHIFT != 0;
        let ctrl = mods & MOD_CTRL != 0;
        let mut c = Change::default();

        if self.scene.drawing() {
            if button == BUTTON_LEFT {
                if let Some(p) = pin {
                    if !p.unused {
                        let at = self.scene.pin_scene(&p.id).unwrap_or(scene);
                        if self.close_wire(&p, at) {
                            self.drag = Drag::None;
                            if self.commit_pending() {
                                c.history = true;
                            }
                            c.wires = true;
                            c.items = true;
                            c.merge(self.refresh_sim());
                            c.merge(self.update_hover(scene, mods));
                            return c;
                        }
                    }
                    return c;
                }
                if let Some(widx) = self.scene.hit_wire(scene) {
                    if self.splice_and_close(widx, snap_point(scene)) {
                        self.drag = Drag::None;
                        if self.commit_pending() {
                            c.history = true;
                        }
                        c.wires = true;
                        c.items = true;
                        c.merge(self.refresh_sim());
                        c.merge(self.update_hover(scene, mods));
                        return c;
                    }
                }
                // Empty click while drawing: corner is added on release.
                self.drag = Drag::Wire;
                c.wires = true;
            }
            return c;
        }

        let wire_hit = self.scene.hit_wire_detail(scene);
        let alt = mods & MOD_ALT != 0;

        if button == BUTTON_MIDDLE {
            if let Some((widx, hit)) = wire_hit {
                let seg = match hit {
                    WireHit::Segment(s) => s,
                    WireHit::Corner(v) => v.saturating_sub(1),
                };
                return self.begin_wire_edit(widx, WireEditMode::Segment(seg), scene, c);
            }
            self.drag = Drag::Pan { last_item: item };
            if self.cursor != CursorKind::ClosedHand {
                self.cursor = CursorKind::ClosedHand;
                c.cursor = true;
            }
            return c;
        }

        if button == BUTTON_LEFT
            && shift
            && !ctrl
            && hit.is_none()
            && pin.is_none()
            && wire_hit.is_none()
        {
            self.drag = Drag::Pan { last_item: item };
            if self.cursor != CursorKind::ClosedHand {
                self.cursor = CursorKind::ClosedHand;
                c.cursor = true;
            }
            return c;
        }

        let label_hit = self.scene.hit_label(scene);

        if button == BUTTON_RIGHT {
            // C++ ConnectorLine z = 100 sits above components. A right-click on a
            // wire that crosses a chip/board must select the wire — otherwise
            // Remove deletes the part underneath (e.g. mega328 / Arduino).
            // Connected pin tips share their point with a wire, so the wire wins
            // there too; unconnected pins keep the pin menu (Invert / Edit).
            let wire_idx = wire_hit.map(|(i, _)| i);
            let pin_menu = pin
                .as_ref()
                .filter(|p| wire_idx.is_none() || !self.scene.pin_connected(&p.id));
            if let Some(p) = pin_menu {
                self.hit_pin_id = Some(p.id.clone());
                self.hit_label_info = None;
            } else if let Some(widx) = wire_idx {
                self.hit_pin_id = None;
                self.hit_label_info = None;
                c.wires = self.select_only_wire(widx);
                c.items = true;
            } else if let Some((idx, is_val)) = label_hit {
                self.hit_pin_id = None;
                let uid = self.scene.items()[idx].id.clone();
                self.hit_label_info = Some((uid, is_val));
                if !self.scene.items()[idx].selected {
                    c.items = self.select_only(idx);
                    c.wires = true;
                }
            } else if let Some(idx) = hit {
                self.hit_pin_id = None;
                self.hit_label_info = None;
                if !self.scene.items()[idx].selected {
                    c.items = self.select_only(idx);
                    c.wires = true;
                }
            } else {
                self.hit_pin_id = None;
                self.hit_label_info = None;
            }
            return c;
        }

        self.hit_pin_id = None;
        self.hit_label_info = None;

        if button != BUTTON_LEFT {
            return c;
        }

        // C++ Node (wire junction) has z = 101, which sits above ConnectorLine (z = 100)
        // and other components (z = 0).
        // Clicking directly on a Node drags the junction (moving all connected wires).
        // Alt-clicking on a Node with an open pin starts a new wire branch.
        if let Some(idx) = hit.filter(|&i| self.scene.items()[i].is_node()) {
            let it = &self.scene.items()[idx];
            if alt {
                let free_pin = it
                    .pins()
                    .into_iter()
                    .find(|p| !self.scene.pin_connected(&p.id));
                if let Some(p) = free_pin {
                    let at = self.scene.pin_scene(&p.id).unwrap_or(scene);
                    let snap = self.snapshot();
                    if self.start_wire(&p, at) {
                        self.pending = Some(snap);
                        self.drag = Drag::Wire;
                        c.wires = true;
                        c.items = true;
                        c.merge(self.update_hover(scene, mods));
                        return c;
                    }
                }
            }
            if shift {
                c.items = self.toggle_selected(idx);
            } else if !self.scene.items()[idx].selected {
                c.items = self.select_only(idx);
            }
            let origin = snap_point(scene);
            self.pending = Some(self.snapshot());
            self.drag = Drag::Move {
                last_scene: origin,
                origin,
                index: idx,
                press_pos: scene,
            };
            c.merge(self.set_cursor(CursorKind::ClosedHand));
            c.items = true;
            c.wires = true;
            return c;
        }

        if let Some(p) = pin {
            if !p.unused && !self.scene.pin_connected(&p.id) {
                let at = self.scene.pin_scene(&p.id).unwrap_or(scene);
                let snap = self.snapshot();
                if self.start_wire(&p, at) {
                    self.pending = Some(snap);
                    self.drag = Drag::Wire;
                    c.wires = true;
                    c.items = true;
                    c.merge(self.update_hover(scene, mods));
                    return c;
                }
            }
        }

        // Wires sit above components (C++ ConnectorLine z = 100).
        if let Some((widx, hit)) = wire_hit {
            if alt {
                self.pending = Some(self.snapshot());
                if self.splice_and_start(widx, snap_point(scene)) {
                    self.drag = Drag::Wire;
                    c.wires = true;
                    c.items = true;
                    c.merge(self.update_hover(scene, mods));
                    return c;
                }
                self.pending = None;
            }
            if shift {
                c.wires = self.toggle_wire_selected(widx);
                return c;
            } else if !self.scene.wires()[widx].selected {
                c.wires = self.select_only_wire(widx);
                c.items = true;
            }
            let group = self.scene.wires()[widx].selected && self.scene.selected_count() > 1;
            let mode = if group {
                WireEditMode::Group
            } else {
                match hit {
                    WireHit::Segment(s) => WireEditMode::Segment(s),
                    WireHit::Corner(v) => WireEditMode::Corner(v),
                }
            };
            return self.begin_wire_edit(widx, mode, scene, c);
        }

        if let Some((idx, is_val)) = label_hit {
            let item = &self.scene.items()[idx];
            let (lx, ly) = if is_val {
                item.val_pos()
            } else {
                item.label_pos()
            };
            let start_pos = Point::new(lx, ly);
            self.pending = Some(self.snapshot());
            self.drag = Drag::MoveLabel {
                index: idx,
                is_val,
                origin: scene,
                start_pos,
                last_scene: scene,
            };
            c.merge(self.set_cursor(CursorKind::ClosedHand));
            c.items = true;
            return c;
        }

        if let Some(idx) = hit {
            let local = self.scene.items()[idx].map_scene(scene);
            let uid = self.scene.items()[idx].id.clone();
            if self.scene.items_mut()[idx].kind.interact_press(local) {
                self.dirty_item_now(&uid);
                self.drag = Drag::Interact { index: idx };
                self.merge_sim_raster(&mut c, &uid);
                return c;
            }
            if let Part::Push(_) = &self.scene.items()[idx].kind {
                let uid = self.scene.items()[idx].id.clone();
                self.set_push_down(&uid, true);
                self.pressed_push_id = Some(uid.clone());
                self.merge_sim_raster(&mut c, &uid);
            }
            if shift {
                c.items = self.toggle_selected(idx);
            } else if !self.scene.items()[idx].selected {
                c.items = self.select_only(idx);
            }
            let origin = snap_point(scene);
            self.pending = Some(self.snapshot());
            self.drag = Drag::Move {
                last_scene: origin,
                origin,
                index: idx,
                press_pos: scene,
            };
            c.items = true;
            c.wires = true;
        } else {
            self.banding = true;
            self.band_rect = Rect::new(scene.x, scene.y, 0.0, 0.0);
            self.drag = Drag::Band { origin: scene };
            c.items = self.clear_selection();
            c.wires = c.items;
            c.band = true;
        }
        c
    }

    pub fn mouse_move(&mut self, item_x: f64, item_y: f64, _buttons: u32, mods: u32) -> Change {
        let item = Point::new(item_x, item_y);
        let scene = self.viewport.map_to_circuit(item);
        self.last_item = item;
        self.last_scene = scene;
        self.cursor_moved = true;
        let mut c = Change::default();

        match self.drag {
            Drag::Pan { last_item } => {
                let delta = item.sub(last_item);
                let z = self.viewport.zoom();
                if z != 0.0 {
                    let cur = self.viewport.center();
                    let ch = self.set_center(cur.x - delta.x / z, cur.y - delta.y / z);
                    c.merge(ch);
                }
                self.drag = Drag::Pan { last_item: item };
            }
            Drag::Band { origin } => {
                self.band_rect =
                    Rect::new(origin.x, origin.y, scene.x - origin.x, scene.y - origin.y);
                c.items = self.select_intersecting(self.band_rect);
                c.wires = c.items;
                c.band = true;
            }
            Drag::Move {
                last_scene,
                origin,
                index,
                press_pos,
            } => {
                let now = snap_point(scene);
                let dx = now.x - last_scene.x;
                let dy = now.y - last_scene.y;
                if self.move_selected(dx, dy) {
                    if let Some(uid) = self.pressed_push_id.take() {
                        self.set_push_down(&uid, false);
                        self.merge_sim_raster(&mut c, &uid);
                    }
                    self.drag = Drag::Move {
                        last_scene: now,
                        origin,
                        index,
                        press_pos,
                    };
                    c.merge(self.refresh_sim().without_qml_model());
                }
            }
            Drag::MoveLabel {
                index,
                is_val,
                origin,
                start_pos,
                last_scene: _,
            } => {
                let _ = self.nudge_item_label(index, is_val, origin, start_pos, scene);
                self.drag = Drag::MoveLabel {
                    index,
                    is_val,
                    origin,
                    start_pos,
                    last_scene: scene,
                };
            }
            Drag::Wire => {
                let shift = mods & MOD_SHIFT != 0;
                if self.route_draft(scene, shift) {
                    c.wires = true;
                }
                c.merge(self.update_hover(scene, mods));
            }
            Drag::WireEdit {
                last_scene,
                origin,
                wire,
                mut mode,
            } => {
                let now = snap_point(scene);
                let dx = now.x - last_scene.x;
                let dy = now.y - last_scene.y;
                if dx != 0.0 || dy != 0.0 {
                    let delta = Point::new(dx, dy);
                    let free = mods & MOD_SHIFT != 0;
                    match mode {
                        WireEditMode::Group => {
                            if self.move_selected(dx, dy) {
                                c.merge(self.refresh_sim().without_qml_model());
                            }
                        }
                        WireEditMode::Segment(seg) => {
                            if let Some(new_seg) = self.drag_wire_segment(wire, seg, delta) {
                                mode = WireEditMode::Segment(new_seg);
                                c.wires = true;
                            }
                        }
                        WireEditMode::Corner(v) => {
                            if let Some(new_v) = self.drag_wire_corner(wire, v, delta, free) {
                                mode = WireEditMode::Corner(new_v);
                                c.wires = true;
                            }
                        }
                    }
                    self.drag = Drag::WireEdit {
                        last_scene: now,
                        origin,
                        wire,
                        mode,
                    };
                }
            }
            Drag::Interact { index } => {
                if let Some(item) = self.scene.items_mut().get_mut(index) {
                    let local = item.map_scene(scene);
                    let uid = item.id.clone();
                    if item.kind.interact_move(local) {
                        self.dirty_item_now(&uid);
                        self.merge_sim_raster(&mut c, &uid);
                    }
                }
            }
            Drag::None => {
                c.merge(self.update_hover(scene, mods));
            }
        }
        c
    }

    pub fn mouse_release(&mut self, _button: u32, item_x: f64, item_y: f64, mods: u32) -> Change {
        let item = Point::new(item_x, item_y);
        self.last_item = item;
        self.last_scene = self.viewport.map_to_circuit(item);
        self.cursor_moved = true;
        let mut c = Change::default();
        match self.drag {
            Drag::Pan { .. } => {
                c.merge(self.update_hover(self.last_scene, mods));
            }
            Drag::Band { .. } => {
                self.banding = false;
                self.band_rect = Rect::default();
                c.band = true;
                self.drag = Drag::None;
            }
            Drag::Wire => {
                // C++ `incActLine` on left-release while a wire is being drawn.
                if self.scene.drawing() {
                    self.inc_draft_corner();
                    c.wires = true;
                    return c;
                }
                self.drag = Drag::None;
            }
            Drag::WireEdit {
                last_scene,
                origin,
                wire,
                mode,
            } => {
                match mode {
                    WireEditMode::Group => self.finish_selected_wires(),
                    _ => {
                        self.finish_wire_drag(wire);
                    }
                }
                c.wires = true;
                c.items = true;
                if last_scene == origin {
                    self.pending = None;
                } else if self.commit_pending() {
                    c.history = true;
                    c.merge(self.refresh_sim());
                }
                self.drag = Drag::None;
                c.merge(self.update_hover(self.last_scene, mods));
            }
            Drag::Interact { index } => {
                if let Some(item) = self.scene.items_mut().get_mut(index) {
                    let local = item.map_scene(self.last_scene);
                    let uid = item.id.clone();
                    if item.kind.interact_release(local) {
                        self.dirty_item_now(&uid);
                        self.merge_sim_raster(&mut c, &uid);
                    }
                }
                self.drag = Drag::None;
                c.merge(self.set_cursor(CursorKind::Arrow));
                c.merge(self.update_hover(self.last_scene, mods));
            }
            Drag::Move {
                last_scene,
                origin,
                index,
                press_pos,
            } => {
                if let Some(uid) = self.pressed_push_id.take() {
                    self.set_push_down(&uid, false);
                    self.merge_sim_raster(&mut c, &uid);
                }
                if last_scene == origin {
                    if let Some(id) = self.toggle_switch_at(index, press_pos) {
                        if self.commit_pending() {
                            c.history = true;
                        }
                        c.items = true;
                        if self.prop_uid.as_deref() == Some(&id)
                            || self.open_prop_uids.iter().any(|uid| uid == &id)
                        {
                            c.props = true;
                        }
                        if let Some(item) = self.scene.item_by_id(&id) {
                            match &item.kind {
                                Part::SerialPort(_) => {
                                    let title = format!("{id}-Serial");
                                    let mon_id = format!("{id}:Serial");
                                    c.open_serial_mon = Some((mon_id, title));
                                }
                                Part::SerialTerm(_) => {
                                    c.open_terminal = true;
                                }
                                _ => {}
                            }
                        }
                        c.merge(self.refresh_sim_component(&id));
                    } else {
                        self.pending = None;
                    }
                } else {
                    self.sync_wires_for_selected();
                    self.finish_selected_wires();
                    if self.commit_pending() {
                        c.history = true;
                    }
                    c.wires = true;
                    c.items = true;
                    c.merge(self.refresh_sim());
                }
                self.drag = Drag::None;
                c.merge(self.set_cursor(CursorKind::Arrow));
                c.merge(self.update_hover(self.last_scene, mods));
            }
            Drag::MoveLabel {
                last_scene, origin, ..
            } => {
                if last_scene != origin {
                    if self.commit_pending() {
                        c.history = true;
                    }
                } else {
                    self.pending = None;
                }
                self.drag = Drag::None;
                c.merge(self.set_cursor(CursorKind::Arrow));
                c.merge(self.update_hover(self.last_scene, mods));
            }
            Drag::None => {
                self.drag = Drag::None;
            }
        }
        if let Some(uid) = self.pressed_push_id.take() {
            self.set_push_down(&uid, false);
            self.merge_sim_raster(&mut c, &uid);
        }
        if !matches!(self.drag, Drag::Wire) {
            self.drag = Drag::None;
        }
        c
    }

    pub fn mouse_cancel(&mut self) -> Change {
        let mut c = Change::default();
        if let Some(uid) = self.pressed_push_id.take() {
            self.set_push_down(&uid, false);
            self.merge_sim_raster(&mut c, &uid);
        }
        match self.drag {
            Drag::Pan { .. } => {
                c.merge(self.set_cursor(CursorKind::Arrow));
            }
            Drag::Band { .. } => {
                self.banding = false;
                self.band_rect = Rect::default();
                c.band = true;
            }
            Drag::Wire => {
                self.pending = None;
                if self.cancel_wire() {
                    c.wires = true;
                    c.items = true;
                }
            }
            Drag::WireEdit { wire, mode, .. } => {
                match mode {
                    WireEditMode::Group => self.finish_selected_wires(),
                    _ => {
                        self.finish_wire_drag(wire);
                    }
                }
                c.wires = true;
            }
            Drag::Interact { index } => {
                if let Some(item) = self.scene.items_mut().get_mut(index) {
                    let uid = item.id.clone();
                    if item.kind.interact_cancel() {
                        self.dirty_item_now(&uid);
                        self.merge_sim_raster(&mut c, &uid);
                    }
                }
            }
            _ => {}
        }
        self.drag = Drag::None;
        if self.hovered_pin.take().is_some() {
            c.hovered_pin = true;
        }
        c.merge(self.set_cursor(CursorKind::Arrow));
        c
    }

    /// C++ `wheelEvent`: Ctrl/Cmd zooms (`2^(angleDelta.y/700)`); else pan by
    /// pixelDelta, or `angleDelta/120*24` when pixelDelta is null.
    pub fn wheel(
        &mut self,
        pixel_dx: f64,
        pixel_dy: f64,
        angle_dx: f64,
        angle_dy: f64,
        item_x: f64,
        item_y: f64,
        mods: u32,
    ) -> Change {
        if item_x != 0.0 || item_y != 0.0 {
            self.last_item = Point::new(item_x, item_y);
            self.cursor_moved = true;
        }
        if Self::cmd(mods) {
            let factor = 2f64.powf(angle_dy / 700.0);
            return self.zoom_by(factor, self.last_item);
        }
        let scene = self.viewport.map_to_circuit(self.last_item);
        let delta = if angle_dy.abs() > 0.0 || angle_dx.abs() > 0.0 {
            if angle_dy.abs() >= angle_dx.abs() {
                angle_dy
            } else {
                angle_dx
            }
        } else if pixel_dy.abs() >= pixel_dx.abs() {
            pixel_dy
        } else {
            pixel_dx
        };
        if let Some(id) = self.wheel_component_at(scene, delta) {
            let (val, text, wiper) = if let Some(item) = self.scene.item_by_id(&id) {
                (item.source_value(), item.val_label_text(), item.wiper())
            } else {
                (0.0, String::new(), 0.0)
            };
            self.last_wheel_item = Some((id.clone(), val, text, wiper));
            return self.finish_component_change(ComponentChange::continuous(&id));
        }
        let (dx, dy) = if pixel_dx == 0.0 && pixel_dy == 0.0 {
            (angle_dx / 120.0 * 24.0, angle_dy / 120.0 * 24.0)
        } else {
            (pixel_dx, pixel_dy)
        };
        let z = self.viewport.zoom();
        if z == 0.0 {
            return Change::default();
        }
        let cur = self.viewport.center();
        self.set_center(cur.x - dx / z, cur.y - dy / z)
    }

    pub fn pinch_zoom(&mut self, factor: f64, item_x: f64, item_y: f64) -> Change {
        if item_x != 0.0 || item_y != 0.0 {
            self.last_item = Point::new(item_x, item_y);
            self.cursor_moved = true;
        }
        self.zoom_by(factor, self.last_item)
    }

    pub fn key_press(&mut self, key: i32, mods: u32) -> Change {
        let cmd = Self::cmd(mods);
        let shift = mods & MOD_SHIFT != 0;
        if cmd && key == KEY_A {
            return self.select_all();
        }
        if cmd && key == KEY_Z {
            return if shift { self.redo() } else { self.undo() };
        }
        if cmd && key == KEY_Y {
            return self.redo();
        }
        if cmd && key == KEY_C {
            self.copy_selection();
            return Change {
                history: true,
                ..Change::default()
            };
        }
        if cmd && key == KEY_X {
            return self.cut_selection();
        }
        if cmd && key == KEY_V {
            return self.paste_at_cursor();
        }
        if cmd && key == KEY_R {
            return if shift {
                self.rotate_ccw()
            } else {
                self.rotate_cw()
            };
        }
        if cmd && key == KEY_L {
            return if shift { self.flip_v() } else { self.flip_h() };
        }
        if cmd && (key == KEY_PLUS || key == KEY_EQUAL) {
            return self.zoom_in();
        }
        if cmd && key == KEY_MINUS {
            return self.zoom_out();
        }
        if cmd && key == KEY_0 {
            return self.zoom_one();
        }
        if !cmd
            && (key == KEY_LEFT
                || key == KEY_DOWN
                || key == KEY_MINUS
                || key == KEY_RIGHT
                || key == KEY_UP
                || key == KEY_PLUS
                || key == KEY_EQUAL)
        {
            let increase = key == KEY_RIGHT || key == KEY_UP || key == KEY_PLUS || key == KEY_EQUAL;
            if let Some(idx) = self
                .scene
                .items()
                .iter()
                .position(|it| it.selected && matches!(it.kind, Part::Potentiometer(_)))
            {
                let id = self.scene.items()[idx].id.clone();
                let uid_s = id.clone();
                let c = self.apply_component_edit(ComponentChange::continuous(&id), |scene| {
                    let Some(it) = scene.item_by_id_mut(&uid_s) else {
                        return false;
                    };
                    let Part::Potentiometer(pot) = &mut it.kind else {
                        return false;
                    };
                    let step = (pot.dial.step / 100.0).clamp(0.01, 1.0);
                    let delta = if increase { step } else { -step };
                    pot.wiper = ((pot.wiper + delta) * 100.0).round() / 100.0;
                    pot.wiper = pot.wiper.clamp(0.0, 1.0);
                    true
                });
                if let Some(item) = self.scene.item_by_id(&id) {
                    self.last_wheel_item =
                        Some((id, item.source_value(), item.val_label_text(), item.wiper()));
                }
                return c;
            }
        }
        if key == KEY_ESCAPE {
            if self.scene.drawing() {
                self.pending = None;
                let cancelled = self.cancel_wire();
                let mut c = Change {
                    wires: cancelled,
                    items: true,
                    ..Change::default()
                };
                self.drag = Drag::None;
                c.merge(self.update_hover(self.last_scene, 0));
                return c;
            }
            return Change {
                items: self.clear_selection(),
                wires: true,
                ..Change::default()
            };
        }
        if key == KEY_DELETE || key == KEY_BACKSPACE {
            return self.delete_selected();
        }
        Change::default()
    }

    pub fn mouse_double_click(
        &mut self,
        button: u32,
        item_x: f64,
        item_y: f64,
        _mods: u32,
    ) -> Change {
        let item = Point::new(item_x, item_y);
        let scene = self.viewport.map_to_circuit(item);
        self.last_item = item;
        self.last_scene = scene;
        self.cursor_moved = true;
        if button != BUTTON_LEFT {
            return Change::default();
        }
        if let Some(idx) = self.scene.hit(scene) {
            let it = &self.scene.items()[idx];
            if matches!(it.kind, Part::Oscope(_)) {
                return Change {
                    open_scope: true,
                    ..Change::default()
                };
            } else if matches!(it.kind, Part::LogicAnalyzer(_)) {
                return Change {
                    open_la: true,
                    ..Change::default()
                };
            }
            let id = it.id.clone();
            return self.open_properties(&id);
        }
        Change::default()
    }

    pub(crate) fn set_cursor(&mut self, want: CursorKind) -> Change {
        if self.cursor == want {
            return Change::default();
        }
        self.cursor = want;
        Change {
            cursor: true,
            ..Change::default()
        }
    }

    pub(crate) fn update_hover(&mut self, scene: Point, mods: u32) -> Change {
        let alt = mods & MOD_ALT != 0;
        let is_node_hover = self
            .scene
            .hit(scene)
            .is_some_and(|i| self.scene.items()[i].is_node());
        let pin = if is_node_hover && !alt && !self.scene.drawing() {
            None
        } else {
            self.scene.hit_pin(scene)
        };
        let id = pin.as_ref().map(|p| p.id.clone());
        let mut c = Change::default();
        if self.hovered_pin != id {
            self.hovered_pin = id;
            c.hovered_pin = true;
        }
        let want = if self.scene.drawing() {
            CursorKind::Cross
        } else if is_node_hover {
            if alt
                && self
                    .scene
                    .hit_pin(scene)
                    .is_some_and(|p| !p.unused && !self.scene.pin_connected(&p.id))
            {
                CursorKind::Cross
            } else {
                CursorKind::OpenHand
            }
        } else if let Some(p) = pin {
            if !p.unused && !self.scene.pin_connected(&p.id) {
                CursorKind::Cross
            } else {
                CursorKind::Arrow
            }
        } else if let Some((widx, hit)) = self.scene.hit_wire_detail(scene) {
            if alt {
                CursorKind::Cross
            } else {
                match hit {
                    WireHit::Corner(_) => CursorKind::SizeAll,
                    WireHit::Segment(seg) => {
                        let w = &self.scene.wires()[widx];
                        if w.segment_is_horizontal(seg) {
                            CursorKind::SplitV
                        } else if w.segment_is_vertical(seg) {
                            CursorKind::SplitH
                        } else {
                            CursorKind::SizeAll
                        }
                    }
                }
            }
        } else if self.scene.hit_label(scene).is_some() {
            CursorKind::OpenHand
        } else {
            CursorKind::Arrow
        };
        c.merge(self.set_cursor(want));
        c
    }
}
