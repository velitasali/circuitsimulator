//! Per-item / per-wire dirty tracking for canvas rasterization.
//!
//! Analog voltage noise does not dirty a resistor: pin stems only change
//! colour on a logic-level crossing, and the body only changes when the
//! part itself is edited. Live parts (LEDs, meters, scopes, current
//! chevrons) mark only their own ids.

use rustc_hash::FxHashSet;

use super::Canvas;
use super::geom::{Point, Rect, map_local};
use super::pin::Pin;
use super::scene::wires::pin_belongs;
use super::scene::{Item, Part};
use super::wire::Wire;

/// Pin stem / wire logic colour threshold. Matches `paint_pin` / `paint_wire`.
pub const PIN_LOGIC_HIGH_V: f64 = 2.5;

#[inline]
pub fn pin_logic_high(v: f64) -> bool {
    v > PIN_LOGIC_HIGH_V
}

#[derive(Clone, Debug, Default)]
pub struct DirtySet {
    pub items: FxHashSet<String>,
    pub wires: FxHashSet<String>,
    /// Geometry snapshots (old + new bounds) so a move/delete still erases
    /// pixels after the scene item has already moved or been removed.
    pub rects: Vec<Rect>,
    pub full: bool,
}

impl DirtySet {
    pub fn is_empty(&self) -> bool {
        !self.full && self.items.is_empty() && self.wires.is_empty() && self.rects.is_empty()
    }

    pub fn mark_item(&mut self, id: &str) {
        self.items.insert(id.to_string());
    }

    pub fn mark_wire(&mut self, id: &str) {
        self.wires.insert(id.to_string());
    }

    pub fn mark_rect(&mut self, r: Rect) {
        if r.is_empty() {
            return;
        }
        self.rects.push(r);
    }

    pub fn mark_full(&mut self) {
        self.full = true;
    }

    pub fn merge(&mut self, other: DirtySet) {
        if other.full || self.full {
            self.mark_full();
            return;
        }
        self.items.extend(other.items);
        self.wires.extend(other.wires);
        self.rects.extend(other.rects);
    }

    /// Scene-space rects that must be re-rastered. Empty when `full`.
    pub fn scene_rects(&self, canvas: &Canvas) -> Vec<Rect> {
        if self.full {
            return Vec::new();
        }
        let mut rects = Vec::with_capacity(self.items.len() + self.wires.len() + self.rects.len());
        rects.extend_from_slice(&self.rects);
        for id in &self.items {
            if let Some(it) = canvas.scene().item_by_id(id) {
                rects.push(item_dirty_rect(it));
            }
        }
        for id in &self.wires {
            if let Some(w) = canvas.scene().wires().iter().find(|w| w.id == *id) {
                rects.push(wire_dirty_rect(w));
            }
        }
        rects
    }
}

/// Scene-space patch for `it`, including glow / horn overflow from
/// `visual_rect`, labels, and a few pixels of AA pad.
pub fn item_dirty_rect(it: &Item) -> Rect {
    let mut r = it.total_bounding_rect();
    if it.show_id && !it.label.is_empty() {
        let (lx, ly) = it.label_pos();
        let tw = crate::canvas::export::text::text_width(&it.label, 9.0, false).max(16.0);
        let approx_h = 14.0;
        let rad = (it.label_rot as f64).to_radians();
        let (s, c) = (rad.sin(), rad.cos());
        let pts = [
            Point::new(lx, ly),
            Point::new(lx + tw * c, ly + tw * s),
            Point::new(lx + tw * c - approx_h * s, ly + tw * s + approx_h * c),
            Point::new(lx - approx_h * s, ly + approx_h * c),
        ];
        for pt in pts {
            let sp = map_local(it.position(), pt, it.rotation, it.hflip, it.vflip);
            r = r.united(Rect::new(sp.x - 4.0, sp.y - 4.0, 8.0, 8.0));
        }
    }
    if it.show_val {
        let val_text = it.val_label_text();
        if !val_text.is_empty() {
            let (vx, vy) = it.val_pos();
            let tw = crate::canvas::export::text::text_width(&val_text, 9.0, false).max(16.0);
            let approx_h = 14.0;
            let rad = (it.val_rot as f64).to_radians();
            let (s, c) = (rad.sin(), rad.cos());
            let pts = [
                Point::new(vx, vy),
                Point::new(vx + tw * c, vy + tw * s),
                Point::new(vx + tw * c - approx_h * s, vy + tw * s + approx_h * c),
                Point::new(vx - approx_h * s, vy + approx_h * c),
            ];
            for pt in pts {
                let sp = map_local(it.position(), pt, it.rotation, it.hflip, it.vflip);
                r = r.united(Rect::new(sp.x - 4.0, sp.y - 4.0, 8.0, 8.0));
            }
        }
    }
    r.adjust(-8.0, -8.0, 8.0, 8.0)
}

pub fn wire_dirty_rect(w: &Wire) -> Rect {
    w.bounds().adjust(-4.0, -4.0, 4.0, 4.0)
}

impl Canvas {
    pub(crate) fn dirty_item_now(&mut self, id: &str) {
        if let Some(it) = self.scene.item_by_id(id) {
            self.dirty.mark_rect(item_dirty_rect(it));
        }
        self.dirty.mark_item(id);
    }

    pub(crate) fn dirty_wire_now(&mut self, id: &str) {
        if let Some(w) = self.scene.wires().iter().find(|w| w.id == *id) {
            self.dirty.mark_rect(wire_dirty_rect(w));
        }
        self.dirty.mark_wire(id);
    }

    pub(crate) fn dirty_wire_index(&mut self, idx: usize) {
        if let Some(w) = self.scene.wires().get(idx) {
            let id = w.id.clone();
            self.dirty_wire_now(&id);
        }
    }

    pub(crate) fn dirty_draft_wire_now(&mut self) {
        if let Some(w) = self.scene.wires().iter().rev().find(|w| w.drawing()) {
            let id = w.id.clone();
            self.dirty_wire_now(&id);
        }
    }

    pub(crate) fn dirty_pin_now(&mut self, pin_id: &str) {
        let item_id = self.scene.items().iter().find_map(|it| {
            if pin_belongs(pin_id, std::slice::from_ref(&it.id)) {
                Some(it.id.clone())
            } else {
                None
            }
        });
        if let Some(id) = item_id {
            self.dirty_item_now(&id);
        }
    }

    /// Selected items plus every wire that would translate with them.
    pub(crate) fn dirty_move_footprint(&mut self) {
        let selected: Vec<String> = self
            .scene
            .items()
            .iter()
            .filter(|it| it.selected)
            .map(|it| it.id.clone())
            .collect();
        for id in &selected {
            self.dirty_item_now(id);
        }
        let wire_ids: Vec<String> = self
            .scene
            .wires()
            .iter()
            .filter(|w| {
                if w.drawing() {
                    return false;
                }
                let start_moved = pin_belongs(&w.start_pin, &selected);
                let end_moved = w
                    .end_pin
                    .as_ref()
                    .is_some_and(|e| pin_belongs(e, &selected));
                start_moved || end_moved || w.selected
            })
            .map(|w| w.id.clone())
            .collect();
        for id in wire_ids {
            self.dirty_wire_now(&id);
        }
    }

    pub(crate) fn draft_wire_ids(&self) -> Option<(String, String)> {
        self.scene
            .wires()
            .iter()
            .rev()
            .find(|w| w.drawing())
            .map(|w| (w.id.clone(), w.start_pin.clone()))
    }

    pub(crate) fn dirty_closed_wire(&mut self, draft: Option<(String, String)>, end_pin: &str) {
        if let Some((id, start)) = draft {
            self.dirty_wire_now(&id);
            self.dirty_pin_now(&start);
        }
        if !end_pin.is_empty() {
            self.dirty_pin_now(end_pin);
        }
    }

    pub(crate) fn dirty_selected_now(&mut self) {
        let item_ids: Vec<String> = self
            .scene
            .items()
            .iter()
            .filter(|it| it.selected)
            .map(|it| it.id.clone())
            .collect();
        for id in item_ids {
            self.dirty_item_now(&id);
        }
        let wire_ids: Vec<String> = self
            .scene
            .wires()
            .iter()
            .filter(|w| w.selected)
            .map(|w| w.id.clone())
            .collect();
        for id in wire_ids {
            self.dirty_wire_now(&id);
        }
    }

    pub(crate) fn move_selected(&mut self, dx: f64, dy: f64) -> bool {
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.dirty_move_footprint();
        let changed = self.scene.move_selected(dx, dy);
        if changed {
            self.dirty_move_footprint();
        }
        changed
    }

    pub(crate) fn start_wire(&mut self, pin: &Pin, at: Point) -> bool {
        if !self.scene.start_wire(pin, at) {
            return false;
        }
        self.dirty_draft_wire_now();
        self.dirty_pin_now(&pin.id);
        true
    }

    pub(crate) fn close_wire(&mut self, pin: &Pin, at: Point) -> bool {
        let draft = self.draft_wire_ids();
        self.dirty_draft_wire_now();
        if !self.scene.close_wire(pin, at) {
            return false;
        }
        self.dirty_closed_wire(draft, &pin.id);
        true
    }

    pub(crate) fn cancel_wire(&mut self) -> bool {
        let draft = self.draft_wire_ids();
        self.dirty_draft_wire_now();
        if !self.scene.cancel_wire() {
            return false;
        }
        if let Some((_, start)) = draft {
            self.dirty_pin_now(&start);
        }
        true
    }

    pub(crate) fn route_draft(&mut self, cursor: Point, shift: bool) -> bool {
        self.dirty_draft_wire_now();
        if !self.scene.route_draft(cursor, shift) {
            return false;
        }
        self.dirty_draft_wire_now();
        true
    }

    pub(crate) fn inc_draft_corner(&mut self) -> bool {
        self.dirty_draft_wire_now();
        if !self.scene.inc_draft_corner() {
            return false;
        }
        self.dirty_draft_wire_now();
        true
    }

    pub(crate) fn splice_and_close(&mut self, wire_idx: usize, at: Point) -> bool {
        let draft = self.draft_wire_ids();
        self.dirty_draft_wire_now();
        self.dirty_wire_index(wire_idx);
        if !self.scene.splice_and_close(wire_idx, at) {
            return false;
        }
        self.dirty_closed_wire(draft, "");
        self.dirty_after_splice(wire_idx);
        true
    }

    pub(crate) fn splice_and_start(&mut self, wire_idx: usize, at: Point) -> bool {
        self.dirty_wire_index(wire_idx);
        if !self.scene.splice_and_start(wire_idx, at) {
            return false;
        }
        self.dirty_after_splice(wire_idx);
        self.dirty_draft_wire_now();
        true
    }

    fn dirty_after_splice(&mut self, wire_idx: usize) {
        self.dirty_wire_index(wire_idx);
        if let Some(w) = self.scene.wires().last() {
            let id = w.id.clone();
            self.dirty_wire_now(&id);
        }
        if let Some(it) = self.scene.items().iter().rev().find(|it| it.is_node()) {
            let id = it.id.clone();
            self.dirty_item_now(&id);
        }
    }

    pub(crate) fn select_only(&mut self, index: usize) -> bool {
        self.dirty_selected_now();
        let changed = self.scene.select_only(index);
        if changed {
            self.dirty_selected_now();
        }
        changed
    }

    pub(crate) fn select_only_wire(&mut self, index: usize) -> bool {
        self.dirty_selected_now();
        let changed = self.scene.select_only_wire(index);
        if changed {
            self.dirty_selected_now();
        }
        changed
    }

    pub(crate) fn toggle_selected(&mut self, index: usize) -> bool {
        let Some(id) = self.scene.items().get(index).map(|it| it.id.clone()) else {
            return false;
        };
        self.dirty_item_now(&id);
        let changed = self.scene.toggle_selected(index);
        self.dirty_item_now(&id);
        changed
    }

    pub(crate) fn toggle_wire_selected(&mut self, index: usize) -> bool {
        self.dirty_wire_index(index);
        let changed = self.scene.toggle_wire_selected(index);
        if changed {
            self.dirty_wire_index(index);
        }
        changed
    }

    pub(crate) fn clear_selection(&mut self) -> bool {
        if !self.scene.any_selected() {
            return false;
        }
        self.dirty_selected_now();
        self.scene.clear_selection()
    }

    pub(crate) fn select_intersecting(&mut self, band: Rect) -> bool {
        self.dirty_selected_now();
        let changed = self.scene.select_intersecting(band);
        self.dirty_selected_now();
        changed
    }

    pub(crate) fn drag_wire_segment(
        &mut self,
        idx: usize,
        seg: usize,
        delta: Point,
    ) -> Option<usize> {
        self.dirty_wire_index(idx);
        let new_seg = self.scene.drag_wire_segment(idx, seg, delta)?;
        self.dirty_wire_index(idx);
        Some(new_seg)
    }

    pub(crate) fn drag_wire_corner(
        &mut self,
        idx: usize,
        vertex: usize,
        delta: Point,
        free: bool,
    ) -> Option<usize> {
        self.dirty_wire_index(idx);
        let new_v = self.scene.drag_wire_corner(idx, vertex, delta, free)?;
        self.dirty_wire_index(idx);
        Some(new_v)
    }

    pub(crate) fn finish_wire_drag(&mut self, idx: usize) -> bool {
        self.dirty_wire_index(idx);
        let changed = self.scene.finish_wire_drag(idx);
        self.dirty_wire_index(idx);
        changed
    }

    pub(crate) fn sync_wires_for_selected(&mut self) {
        let moved_ids: Vec<String> = self
            .scene
            .items()
            .iter()
            .filter(|it| it.selected)
            .map(|it| it.id.clone())
            .collect();
        if moved_ids.is_empty() {
            return;
        }
        self.dirty_move_footprint();
        self.scene.sync_wires_for(&moved_ids);
        self.dirty_move_footprint();
    }

    pub(crate) fn set_push_down(&mut self, uid: &str, down: bool) {
        self.scene.set_push_state(uid, down);
        self.dirty_item_now(uid);
    }

    pub(crate) fn toggle_switch_at(&mut self, index: usize, press_pos: Point) -> Option<String> {
        let id = self.scene.toggle_switch_at(index, press_pos)?;
        self.dirty_item_now(&id);
        Some(id)
    }

    pub(crate) fn wheel_component_at(&mut self, scene: Point, delta: f64) -> Option<String> {
        let id = self.scene.wheel_at(scene, delta)?;
        self.dirty_item_now(&id);
        Some(id)
    }

    pub(crate) fn nudge_item_label(
        &mut self,
        index: usize,
        is_val: bool,
        origin: Point,
        start_pos: Point,
        scene: Point,
    ) -> bool {
        let Some(item) = self.scene.items().get(index) else {
            return false;
        };
        let id = item.id.clone();
        self.dirty_item_now(&id);
        let Some(item) = self.scene.items_mut().get_mut(index) else {
            return false;
        };
        let local_origin = item.map_scene(origin);
        let local_now = item.map_scene(scene);
        let dx = local_now.x - local_origin.x;
        let dy = local_now.y - local_origin.y;
        if !is_val {
            item.label_x = (start_pos.x + dx).round();
            item.label_y = (start_pos.y + dy).round();
            item.custom_label_pos = true;
        } else {
            item.val_x = (start_pos.x + dx).round();
            item.val_y = (start_pos.y + dy).round();
            item.custom_val_pos = true;
        }
        self.dirty_item_now(&id);
        true
    }

    pub(crate) fn drop_selected_visuals(&mut self) -> bool {
        if !self.scene.any_selected() {
            return false;
        }
        self.dirty_move_footprint();
        self.dirty_selected_now();
        self.scene.delete_selected()
    }
}

/// Quantized glow used to decide whether an analog-painted part changed.
pub fn analog_visual_bucket(it: &Item, circuit: &crate::Circuit) -> Option<i32> {
    fn glow_bucket(ratio: f64) -> i32 {
        (ratio.clamp(0.0, 2.0).sqrt() * 24.0).round() as i32
    }
    fn pin_i(circuit: &crate::Circuit, item_id: &str, suffix: &str) -> f64 {
        super::with_pin_id(item_id, suffix, |pin_id| {
            circuit.current_out_of_pin(pin_id).unwrap_or(0.0)
        })
    }
    fn pin_v(circuit: &crate::Circuit, item_id: &str, suffix: &str) -> f64 {
        super::with_pin_id(item_id, suffix, |pin_id| {
            circuit.pin_voltage(pin_id).unwrap_or(0.0)
        })
    }
    match &it.kind {
        Part::Led(p) => {
            let current = pin_i(circuit, &it.id, crate::elements::pins::PIN_LEFT).max(0.0);
            let max_i = p.max_current.max(0.001);
            let mut ratio = current / max_i;
            // Paint can light from Vf even when pin current is ~0; the
            // current-only bucket missed that until a full raster (scroll).
            if ratio <= 0.001 {
                let va = pin_v(circuit, &it.id, crate::elements::pins::PIN_LEFT);
                let vc = if p.grounded {
                    0.0
                } else {
                    pin_v(circuit, &it.id, crate::elements::pins::PIN_RIGHT)
                };
                let v_diff = (va - vc).max(0.0);
                if v_diff > p.threshold * 0.95 {
                    ratio = ((v_diff - p.threshold).max(0.0) / (p.resistance.max(0.1) * max_i))
                        .clamp(0.05, 1.5);
                }
            }
            Some(glow_bucket(ratio))
        }
        Part::RgbLed(_) => {
            let ir = pin_i(circuit, &it.id, crate::elements::pins::PIN_RGB_R).abs() / 0.03;
            let ig = pin_i(circuit, &it.id, crate::elements::pins::PIN_RGB_G).abs() / 0.03;
            let ib = pin_i(circuit, &it.id, crate::elements::pins::PIN_RGB_B).abs() / 0.03;
            Some((glow_bucket(ir) << 16) | (glow_bucket(ig) << 8) | glow_bucket(ib))
        }
        Part::Lamp(p) => {
            let current = pin_i(circuit, &it.id, crate::elements::pins::PIN_LEFT).abs();
            let max_i = if p.voltage > 1e-6 {
                p.power / p.voltage
            } else {
                1.0
            };
            Some(glow_bucket(current / max_i.max(0.001)))
        }
        _ => None,
    }
}
