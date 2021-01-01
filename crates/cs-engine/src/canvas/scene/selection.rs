//! Scene selection, hit-testing, transformations, bounding boxes, and canvas overflow.

use super::Scene;
use super::item::CanvasOverflowInfo;
use super::wires::pin_belongs;
use crate::canvas::geom::{Point, Rect};

impl Scene {
    /// Topmost first, same as `QGraphicsScene::items`.
    pub fn hit(&self, scene: Point) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .rev()
            .find(|(_, it)| it.contains(scene))
            .map(|(i, _)| i)
    }

    /// Check if a scene point hits any component label (ID label or value label).
    /// Returns `Some((item_index, is_val_label))`.
    pub fn hit_label(&self, scene: Point) -> Option<(usize, bool)> {
        self.items
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, it)| it.hit_label(scene).map(|is_val| (i, is_val)))
    }

    pub fn rotate_item_label(&mut self, uid: &str, angle_deg: i32) -> bool {
        if let Some(it) = self.item_by_id_mut(uid) {
            it.label_rot = (it.label_rot + angle_deg * it.hflip * it.vflip).rem_euclid(360);
            return true;
        }
        false
    }

    pub fn rotate_item_val_label(&mut self, uid: &str, angle_deg: i32) -> bool {
        if let Some(it) = self.item_by_id_mut(uid) {
            it.val_rot = (it.val_rot + angle_deg * it.hflip * it.vflip).rem_euclid(360);
            return true;
        }
        false
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, it)| it.selected)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn any_selected(&self) -> bool {
        self.items.iter().any(|it| it.selected) || self.wires.iter().any(|w| w.selected)
    }

    pub fn clear_selection(&mut self) -> bool {
        let mut changed = false;
        for it in &mut self.items {
            if it.selected {
                it.selected = false;
                changed = true;
            }
        }
        for w in &mut self.wires {
            if w.selected {
                w.selected = false;
                changed = true;
            }
        }
        changed
    }

    pub fn select_only(&mut self, index: usize) -> bool {
        let mut changed = self.clear_wire_selection();
        for (i, it) in self.items.iter_mut().enumerate() {
            let want = i == index;
            if it.selected != want {
                it.selected = want;
                changed = true;
            }
        }
        changed
    }

    pub fn select_only_wire(&mut self, index: usize) -> bool {
        let mut changed = false;
        for it in &mut self.items {
            if it.selected {
                it.selected = false;
                changed = true;
            }
        }
        for (i, w) in self.wires.iter_mut().enumerate() {
            let want = i == index && w.closed();
            if w.selected != want {
                w.selected = want;
                changed = true;
            }
        }
        changed
    }

    fn clear_wire_selection(&mut self) -> bool {
        let mut changed = false;
        for w in &mut self.wires {
            if w.selected {
                w.selected = false;
                changed = true;
            }
        }
        changed
    }

    pub fn toggle_selected(&mut self, index: usize) -> bool {
        if let Some(it) = self.items.get_mut(index) {
            it.selected = !it.selected;
            return true;
        }
        false
    }

    pub fn toggle_wire_selected(&mut self, index: usize) -> bool {
        if let Some(w) = self.wires.get_mut(index) {
            if w.closed() {
                w.selected = !w.selected;
                return true;
            }
        }
        false
    }

    /// C++ rubber band: `Qt::IntersectsItemShape` / `ReplaceSelection`.
    pub fn select_intersecting(&mut self, band: Rect) -> bool {
        let band = band.normalized();
        let mut changed = false;
        for it in &mut self.items {
            let hit = it.selection_rect().intersects(&band);
            if it.selected != hit {
                it.selected = hit;
                changed = true;
            }
        }
        for w in &mut self.wires {
            if w.drawing() {
                continue;
            }
            let hit = w.bounds().intersects(&band);
            if w.selected != hit {
                w.selected = hit;
                changed = true;
            }
        }
        changed
    }

    pub fn select_all(&mut self) -> bool {
        let mut changed = false;
        for it in &mut self.items {
            if !it.selected {
                it.selected = true;
                changed = true;
            }
        }
        for w in &mut self.wires {
            if w.closed() && !w.selected {
                w.selected = true;
                changed = true;
            }
        }
        changed
    }

    pub fn delete_selected(&mut self) -> bool {
        let drop_items: Vec<String> = self
            .items
            .iter()
            .filter(|it| it.selected)
            .map(|it| it.id.clone())
            .collect();
        let before_items = self.items.len();
        let before_wires = self.wires.len();
        self.items.retain(|it| !it.selected);
        self.wires.retain(|w| {
            if w.drawing() {
                return true;
            }
            if w.selected {
                return false;
            }
            let start_gone = drop_items.iter().any(|id| {
                w.start_pin.starts_with(id)
                    && w.start_pin.as_bytes().get(id.len()).copied() == Some(b'-')
            });
            let end_gone = w.end_pin.as_ref().is_some_and(|e| {
                drop_items.iter().any(|id| {
                    e.starts_with(id) && e.as_bytes().get(id.len()).copied() == Some(b'-')
                })
            });
            !start_gone && !end_gone
        });
        let cleaned = self.cleanup_nodes();
        self.items.len() != before_items || self.wires.len() != before_wires || cleaned
    }

    pub fn move_selected(&mut self, dx: f64, dy: f64) -> bool {
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        let mut moved_ids = Vec::new();
        let mut changed = false;
        for it in &mut self.items {
            if it.selected {
                it.x += dx;
                it.y += dy;
                moved_ids.push(it.id.clone());
                changed = true;
            }
        }
        let mut moved_wires = Vec::new();
        for (i, w) in self.wires.iter().enumerate() {
            if w.drawing() {
                continue;
            }
            let start_moved = pin_belongs(&w.start_pin, &moved_ids);
            let end_moved = w
                .end_pin
                .as_ref()
                .is_some_and(|e| pin_belongs(e, &moved_ids));
            if start_moved || end_moved || w.selected {
                moved_wires.push((i, start_moved, end_moved, w.selected));
            }
        }
        for (i, start_moved, end_moved, wire_sel) in moved_wires {
            if (start_moved && end_moved) || wire_sel {
                self.wires[i].translate(dx, dy);
                changed = true;
                continue;
            }
            let start_id = self.wires[i].start_pin.clone();
            let end_id = self.wires[i].end_pin.clone();
            if start_moved {
                if let Some(at) = self.pin_scene(&start_id) {
                    self.wires[i].set_start(at);
                    changed = true;
                }
            }
            if end_moved {
                if let Some(end) = end_id {
                    if let Some(at) = self.pin_scene(&end) {
                        self.wires[i].set_end(at);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    pub fn scene_rect(&self) -> Rect {
        let w = self.settings.width as f64;
        let h = self.settings.height as f64;
        Rect::new(-w * 0.5, -h * 0.5, w, h)
    }

    pub fn canvas_overflow_info(&self) -> CanvasOverflowInfo {
        let sr = self.scene_rect();
        let ibr = self.items_bounding_rect();
        let cur_w = self.settings.width;
        let cur_h = self.settings.height;

        if ibr.is_null() || (self.items.is_empty() && self.wires.is_empty()) {
            return CanvasOverflowInfo {
                has_overflow: false,
                overflowing_count: 0,
                required_width: cur_w,
                required_height: cur_h,
                current_width: cur_w,
                current_height: cur_h,
                items_bounds: Rect::default(),
            };
        }

        let mut overflowing_count = 0;
        for it in &self.items {
            let r = it.full_rect();
            if r.left() < sr.left() - 1e-4
                || r.right() > sr.right() + 1e-4
                || r.top() < sr.top() - 1e-4
                || r.bottom() > sr.bottom() + 1e-4
            {
                overflowing_count += 1;
            }
        }
        for w in &self.wires {
            if !w.closed() {
                continue;
            }
            let wb = w.bounds();
            if wb.left() < sr.left() - 1e-4
                || wb.right() > sr.right() + 1e-4
                || wb.top() < sr.top() - 1e-4
                || wb.bottom() > sr.bottom() + 1e-4
            {
                overflowing_count += 1;
            }
        }

        let has_overflow = ibr.left() < sr.left() - 1e-4
            || ibr.right() > sr.right() + 1e-4
            || ibr.top() < sr.top() - 1e-4
            || ibr.bottom() > sr.bottom() + 1e-4;

        let max_abs_x = ibr.left().abs().max(ibr.right().abs());
        let max_abs_y = ibr.top().abs().max(ibr.bottom().abs());
        let margin = 200.0;

        let req_w_raw = ((max_abs_x + margin) * 2.0).ceil() as i32;
        let req_h_raw = ((max_abs_y + margin) * 2.0).ceil() as i32;

        let required_width = (((req_w_raw + 99) / 100) * 100)
            .max(cur_w)
            .clamp(100, 10_000);
        let required_height = (((req_h_raw + 99) / 100) * 100)
            .max(cur_h)
            .clamp(100, 10_000);

        CanvasOverflowInfo {
            has_overflow,
            overflowing_count,
            required_width,
            required_height,
            current_width: cur_w,
            current_height: cur_h,
            items_bounds: ibr,
        }
    }

    pub fn select_overflowing(&mut self) -> bool {
        let sr = self.scene_rect();
        let mut changed = false;
        for it in &mut self.items {
            let r = it.full_rect();
            let overflows = r.left() < sr.left() - 1e-4
                || r.right() > sr.right() + 1e-4
                || r.top() < sr.top() - 1e-4
                || r.bottom() > sr.bottom() + 1e-4;
            if it.selected != overflows {
                it.selected = overflows;
                changed = true;
            }
        }
        for w in &mut self.wires {
            let wb = w.bounds();
            let overflows = w.closed()
                && (wb.left() < sr.left() - 1e-4
                    || wb.right() > sr.right() + 1e-4
                    || wb.top() < sr.top() - 1e-4
                    || wb.bottom() > sr.bottom() + 1e-4);
            if w.selected != overflows {
                w.selected = overflows;
                changed = true;
            }
        }
        changed
    }

    pub fn translate_all(&mut self, dx: f64, dy: f64) -> bool {
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        for it in &mut self.items {
            it.x += dx;
            it.y += dy;
        }
        for w in &mut self.wires {
            w.translate(dx, dy);
        }
        true
    }

    pub fn center_all(&mut self) -> (f64, f64) {
        let ibr = self.items_bounding_rect();
        if ibr.is_null() || (self.items.is_empty() && self.wires.is_empty()) {
            return (0.0, 0.0);
        }
        let center = ibr.center();
        let dx = (-center.x / 8.0).round() * 8.0;
        let dy = (-center.y / 8.0).round() * 8.0;
        if dx == 0.0 && dy == 0.0 {
            return (0.0, 0.0);
        }
        self.translate_all(dx, dy);
        (dx, dy)
    }

    pub fn items_bounding_rect(&self) -> Rect {
        let mut r = Rect::default();
        for it in &self.items {
            r = r.united(it.full_rect());
        }
        for w in &self.wires {
            if w.closed() {
                r = r.united(w.bounds());
            }
        }
        r
    }

    pub fn selected_rect(&self) -> Rect {
        let mut r = Rect::default();
        for it in &self.items {
            if it.selected {
                r = r.united(it.full_rect());
            }
        }
        for w in &self.wires {
            if w.selected {
                r = r.united(w.bounds());
            }
        }
        r
    }

    /// C++ `rotateAngle`: add `degrees * hflip * vflip` to selected items.
    pub fn rotate_selected(&mut self, degrees: f64) -> bool {
        let mut ids = Vec::new();
        for it in &mut self.items {
            if it.selected {
                it.rotation += degrees * it.hflip as f64 * it.vflip as f64;
                ids.push(it.id.clone());
            }
        }
        if ids.is_empty() {
            return false;
        }
        self.sync_wires_for(&ids);
        true
    }

    pub fn flip_h_selected(&mut self) -> bool {
        let mut ids = Vec::new();
        for it in &mut self.items {
            if it.selected {
                it.hflip = -it.hflip;
                ids.push(it.id.clone());
            }
        }
        if ids.is_empty() {
            return false;
        }
        self.sync_wires_for(&ids);
        true
    }

    pub fn flip_v_selected(&mut self) -> bool {
        let mut ids = Vec::new();
        for it in &mut self.items {
            if it.selected {
                it.vflip = -it.vflip;
                ids.push(it.id.clone());
            }
        }
        if ids.is_empty() {
            return false;
        }
        self.sync_wires_for(&ids);
        true
    }

    pub fn sync_wires_for(&mut self, item_ids: &[String]) {
        let mut updates = Vec::new();
        for (i, w) in self.wires.iter().enumerate() {
            if w.drawing() {
                continue;
            }
            let start = pin_belongs(&w.start_pin, item_ids);
            let end = w.end_pin.as_ref().is_some_and(|e| pin_belongs(e, item_ids));
            if start || end {
                updates.push((i, start, end, w.start_pin.clone(), w.end_pin.clone()));
            }
        }
        for (i, start, end, start_id, end_id) in updates {
            if start {
                if let Some(at) = self.pin_scene(&start_id) {
                    self.wires[i].set_start(at);
                }
            }
            if end {
                if let Some(end) = end_id {
                    if let Some(at) = self.pin_scene(&end) {
                        self.wires[i].set_end(at);
                    }
                }
            }
        }
    }

    pub fn drop_wires_to_missing_pins(&mut self) {
        let pins: std::collections::HashSet<String> = self
            .items
            .iter()
            .flat_map(|it| it.pins().into_iter().map(|p| p.id))
            .collect();
        self.wires.retain(|w| {
            if w.drawing() {
                return true;
            }
            pins.contains(&w.start_pin) && w.end_pin.as_ref().is_some_and(|e| pins.contains(e))
        });
        self.cleanup_nodes();
    }
}
