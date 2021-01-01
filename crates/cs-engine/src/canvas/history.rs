//! Canvas history, undo/redo snapshots, and backup persistence.

use crate::canvas::drag::Drag;
use crate::canvas::events::Change;
use crate::canvas::{Canvas, Scene};
use crate::components::ComponentChange;

#[derive(Clone, Debug)]
pub(crate) struct Snapshot {
    pub(crate) sim1: String,
    pub(crate) selected_items: Vec<String>,
    pub(crate) selected_wires: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct History {
    pub(crate) undo: Vec<Snapshot>,
    pub(crate) redo: Vec<Snapshot>,
}

fn snap_to_backup(s: &Snapshot) -> crate::backup::HistorySnap {
    crate::backup::HistorySnap {
        sim1: s.sim1.clone(),
        selected_items: s.selected_items.clone(),
        selected_wires: s.selected_wires.clone(),
    }
}

fn snap_from_backup(s: crate::backup::HistorySnap) -> Snapshot {
    Snapshot {
        sim1: s.sim1,
        selected_items: s.selected_items,
        selected_wires: s.selected_wires,
    }
}

impl History {
    pub(crate) fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

impl Canvas {
    pub fn export_history(&self) -> crate::backup::CircuitHistory {
        crate::backup::CircuitHistory {
            undo: self.history.undo.iter().map(snap_to_backup).collect(),
            redo: self.history.redo.iter().map(snap_to_backup).collect(),
        }
    }

    pub fn import_history(&mut self, history: crate::backup::CircuitHistory) {
        self.history.undo = history.undo.into_iter().map(snap_from_backup).collect();
        self.history.redo = history.redo.into_iter().map(snap_from_backup).collect();
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            sim1: self.scene.to_sim1(),
            selected_items: self
                .scene
                .items()
                .iter()
                .filter(|it| it.selected)
                .map(|it| it.id.clone())
                .collect(),
            selected_wires: self
                .scene
                .wires()
                .iter()
                .filter(|w| w.selected)
                .map(|w| w.id.clone())
                .collect(),
        }
    }

    pub(crate) fn restore_snapshot(&mut self, snap: Snapshot) -> Change {
        match Scene::from_sim1_with(&snap.sim1, &self.search()) {
            Ok(mut scene) => {
                for it in scene.items_mut() {
                    it.selected = snap.selected_items.iter().any(|id| id == &it.id);
                }
                for w in scene.wires_mut() {
                    w.selected = snap.selected_wires.iter().any(|id| id == &w.id);
                }
                self.scene = scene;
                self.drag = Drag::None;
                self.banding = false;
                self.pending = None;
                self.dirty.mark_full();
                let w = self.scene.settings().width as f64;
                let h = self.scene.settings().height as f64;
                let mut c = Change::edit();
                if self.viewport.set_scene_size(w, h) {
                    c.viewport = true;
                }
                c.merge(self.refresh_sim());
                c.merge(self.refresh_props());
                c
            }
            Err(_) => Change::default(),
        }
    }

    pub(crate) fn max_undo(&self) -> usize {
        crate::settings::get().undo_steps.clamp(5, 1000) as usize
    }

    pub(crate) fn push_undo(&mut self) {
        self.history.undo.push(self.snapshot());
        let cap = self.max_undo();
        while self.history.undo.len() > cap {
            self.history.undo.remove(0);
        }
        self.history.redo.clear();
    }

    /// Snapshot (if `spec.undo`), run `mutate`, then apply view / props /
    /// history (`modified`) / sim so those sides cannot be forgotten.
    pub fn apply_component_edit<F>(&mut self, spec: ComponentChange, mutate: F) -> Change
    where
        F: FnOnce(&mut Scene) -> bool,
    {
        if spec.undo {
            self.push_undo();
        }
        if spec.view.rasters_item() {
            self.dirty_item_now(&spec.uid);
        }
        if !mutate(&mut self.scene) {
            if spec.undo {
                self.history.undo.pop();
            }
            return Change::default();
        }
        self.finish_component_change(spec)
    }

    /// Apply flags after a mutation that already happened. Does not snapshot.
    pub fn apply_component_change(&mut self, spec: ComponentChange) -> Change {
        self.finish_component_change(spec)
    }

    pub(crate) fn finish_component_change(&mut self, spec: ComponentChange) -> Change {
        if spec.structural {
            self.scene.drop_wires_to_missing_pins();
            self.scene.sync_wires_for(std::slice::from_ref(&spec.uid));
        }
        let mut c = Change::from_component(&spec);
        if spec.structural {
            self.dirty.mark_full();
        } else if spec.view.rasters_item() {
            self.dirty_item_now(&spec.uid);
        }
        if spec.sim {
            if spec.structural {
                c.merge(self.refresh_sim());
            } else if spec.view.rebuilds_qml_item() {
                c.merge(self.refresh_sim_component(&spec.uid));
            } else {
                self.merge_sim_raster(&mut c, &spec.uid);
            }
        }
        let dialog_open = self.prop_uid.as_deref() == Some(spec.uid.as_str())
            || self.open_prop_uids.iter().any(|id| id == &spec.uid);
        if dialog_open || (spec.saved && spec.view.rebuilds_qml_item()) {
            c.merge(self.refresh_props());
        }
        c
    }

    pub(crate) fn merge_sim_raster(&mut self, c: &mut Change, uid: &str) {
        c.merge(self.refresh_sim_component(uid).without_qml_model());
    }

    pub(crate) fn commit_pending(&mut self) -> bool {
        let Some(pending) = self.pending.take() else {
            return false;
        };
        if pending.sim1 == self.scene.to_sim1() {
            return false;
        }
        self.history.undo.push(pending);
        let cap = self.max_undo();
        while self.history.undo.len() > cap {
            self.history.undo.remove(0);
        }
        self.history.redo.clear();
        true
    }

    pub fn undo(&mut self) -> Change {
        let Some(snap) = self.history.undo.pop() else {
            return Change::default();
        };
        let current = self.snapshot();
        let mut c = self.restore_snapshot(snap);
        self.history.redo.push(current);
        c.history = true;
        c
    }

    pub fn redo(&mut self) -> Change {
        let Some(snap) = self.history.redo.pop() else {
            return Change::default();
        };
        let current = self.snapshot();
        let mut c = self.restore_snapshot(snap);
        self.history.undo.push(current);
        c.history = true;
        c
    }
}
