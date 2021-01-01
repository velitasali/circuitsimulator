//! Canvas selection clipboard, cut/copy/paste, and geometric transformations.

use crate::canvas::events::Change;
use crate::canvas::geom::to_grid as snap_point;
use crate::canvas::{Canvas, Point};
use crate::components::ComponentChange;

impl Canvas {
    pub fn copy_selection(&mut self) -> bool {
        let Some(src) = self.scene.selection_to_sim1() else {
            return false;
        };
        let origin = {
            let r = self.scene.selected_rect();
            if r.is_null() {
                self.last_scene()
            } else {
                r.center()
            }
        };
        self.clipboard = Some((src, snap_point(origin)));
        true
    }

    pub fn paste_at_cursor(&mut self) -> Change {
        self.paste_at(self.last_scene())
    }

    pub fn paste_at(&mut self, at: Point) -> Change {
        let Some((src, origin)) = self.clipboard.clone() else {
            return Change::default();
        };
        let delta = snap_point(at).sub(origin);
        self.push_undo();
        match self.scene.paste_sim1_with(&src, delta, &self.search()) {
            Ok(true) => {
                self.dirty_selected_now();
                let mut c = Change::edit();
                c.merge(self.refresh_sim());
                c
            }
            _ => {
                self.history.undo.pop();
                Change::default()
            }
        }
    }

    pub fn cut_selection(&mut self) -> Change {
        if !self.scene.any_selected() {
            return Change::default();
        }
        self.copy_selection();
        self.push_undo();
        let mut c = Change::edit();
        c.items = self.drop_selected_visuals();
        c.wires = true;
        c.merge(self.refresh_sim());
        c.merge(self.refresh_props());
        c
    }

    pub fn delete_selected(&mut self) -> Change {
        if !self.scene.any_selected() {
            return Change::default();
        }
        self.push_undo();
        let mut c = Change::edit();
        c.items = self.drop_selected_visuals();
        c.wires = true;
        c.merge(self.refresh_sim());
        c.merge(self.refresh_props());
        c
    }

    pub fn select_all(&mut self) -> Change {
        self.dirty_selected_now();
        let changed = self.scene.select_all();
        if changed {
            self.dirty_selected_now();
        }
        Change {
            items: changed,
            wires: changed,
            history: false,
            ..Change::default()
        }
    }

    pub fn rotate_cw(&mut self) -> Change {
        self.rotate_selected(90.0)
    }

    pub fn rotate_ccw(&mut self) -> Change {
        self.rotate_selected(-90.0)
    }

    pub fn rotate_selected(&mut self, degrees: f64) -> Change {
        if !self.scene.items().iter().any(|it| it.selected) {
            return Change::default();
        }
        self.push_undo();
        self.dirty_move_footprint();
        if !self.scene.rotate_selected(degrees) {
            self.history.undo.pop();
            return Change::default();
        }
        self.dirty_move_footprint();
        let mut c = Change::edit();
        c.merge(self.refresh_sim());
        c
    }

    pub fn flip_h(&mut self) -> Change {
        if !self.scene.items().iter().any(|it| it.selected) {
            return Change::default();
        }
        self.push_undo();
        self.dirty_move_footprint();
        if !self.scene.flip_h_selected() {
            self.history.undo.pop();
            return Change::default();
        }
        self.dirty_move_footprint();
        let mut c = Change::edit();
        c.merge(self.refresh_sim());
        c
    }

    pub fn flip_v(&mut self) -> Change {
        if !self.scene.items().iter().any(|it| it.selected) {
            return Change::default();
        }
        self.push_undo();
        self.dirty_move_footprint();
        if !self.scene.flip_v_selected() {
            self.history.undo.pop();
            return Change::default();
        }
        self.dirty_move_footprint();
        let mut c = Change::edit();
        c.merge(self.refresh_sim());
        c
    }

    pub fn rotate_item_label(&mut self, uid: &str, angle_deg: i32) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            scene.rotate_item_label(&uid_s, angle_deg)
        })
    }

    pub fn rotate_item_val_label(&mut self, uid: &str, angle_deg: i32) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            scene.rotate_item_val_label(&uid_s, angle_deg)
        })
    }
}
