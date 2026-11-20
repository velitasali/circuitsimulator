//! "Edit Board Layout" mode on a `SubPackage`: arrange visual/interactive components
//! on the package face without disturbing their schematic positions or wiring.

use std::collections::HashMap;

use crate::canvas::drag::Drag;
use crate::canvas::events::Change;
use crate::canvas::scene::{Item, Part, Scene};
use crate::canvas::{Canvas, Point, Rect, map_rect};
use crate::circ1::BoardPlacement;
use crate::package::SubcType;

/// Saved schematic transform of one item while editing a `SubPackage`'s board face.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SavedCircTransform {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub hflip: i32,
    pub vflip: i32,
}

/// Active "Edit Board Layout" session for a `SubPackage`.
#[derive(Clone, Debug)]
pub(crate) struct BoardLayoutSession {
    /// `CircId` of the `SubPackage` being laid out.
    pub pkg_uid: String,
    /// Original schematic transforms of every item before entering board layout mode.
    pub saved: HashMap<String, SavedCircTransform>,
}

/// Returns `true` if `part` can be mounted on a Board/Shield package face.
pub fn is_boardable_part(part: &Part) -> bool {
    !matches!(
        part,
        Part::SubPackage(_) | Part::Node(_) | Part::Tunnel(_) | Part::Bus(_)
    )
}

impl Canvas {
    pub fn is_board_layout(&self) -> bool {
        self.board_layout.is_some()
    }

    pub fn board_layout_pkg(&self) -> Option<&str> {
        self.board_layout.as_ref().map(|s| s.pkg_uid.as_str())
    }

    pub fn is_item_visible_in_board_layout(&self, it: &Item) -> bool {
        let Some(session) = &self.board_layout else {
            return true;
        };
        it.id == session.pkg_uid || is_boardable_part(&it.kind)
    }

    /// Package face rectangle `(0, 0, w, h)` for the active board layout session.
    pub fn board_layout_pkg_rect(&self) -> Option<Rect> {
        let session = self.board_layout.as_ref()?;
        let pkg_item = self.scene.item_by_id(&session.pkg_uid)?;
        let Part::SubPackage(sp) = &pkg_item.kind else {
            return None;
        };
        Some(Rect::new(
            0.0,
            0.0,
            sp.package.body_w(),
            sp.package.body_h(),
        ))
    }

    /// Returns `true` if `it` is currently positioned on the package face during board layout mode.
    pub fn is_item_mounted_in_board_layout(&self, it: &Item) -> bool {
        let Some(session) = &self.board_layout else {
            return false;
        };
        if it.id == session.pkg_uid || !is_boardable_part(&it.kind) {
            return false;
        }
        let Some(pkg_rect) = self.board_layout_pkg_rect() else {
            return false;
        };
        let item_rect = map_rect(
            Point::new(it.x, it.y),
            it.body_rect(),
            it.rotation,
            it.hflip,
            it.vflip,
        );
        pkg_rect.intersects(&item_rect)
    }

    /// Enter "Edit Board Layout" mode for the `SubPackage` identified by `pkg_uid`.
    /// Calling again for the same `pkg_uid` exits cleanly.
    pub fn enter_board_layout(&mut self, pkg_uid: &str) -> Change {
        if self.board_layout.as_ref().map(|s| s.pkg_uid.as_str()) == Some(pkg_uid) {
            return self.exit_board_layout();
        }
        let mut c = Change::default();
        if self.board_layout.is_some() {
            c.merge(self.exit_board_layout());
        }
        if self.link_mode.is_some() {
            c.merge(self.stop_linking());
        }
        if self.scene.drawing() {
            self.cancel_wire();
        }
        if self.sim_running {
            c.merge(self.power_off());
        }

        let Some(pkg_item) = self.scene.item_by_id(pkg_uid) else {
            return c;
        };
        if !matches!(pkg_item.kind, Part::SubPackage(_)) {
            return c;
        }

        // Snapshot before potentially switching SubcType to Board so the transition is undoable.
        let promoted = if let Some(it) = self.scene.item_by_id(pkg_uid) {
            matches!(
                &it.kind,
                Part::SubPackage(sp)
                    if !matches!(sp.package.subc_type, SubcType::Board | SubcType::Shield)
                        || sp.package.logic_symbol
            )
        } else {
            false
        };
        if promoted {
            self.push_undo();
            if let Some(it) = self.scene.item_by_id_mut(pkg_uid) {
                if let Part::SubPackage(sp) = &mut it.kind {
                    if !matches!(sp.package.subc_type, SubcType::Board | SubcType::Shield) {
                        sp.package.subc_type = SubcType::Board;
                    }
                    sp.package.logic_symbol = false;
                }
            }
        }

        let (pkg_w, pkg_h) = self.apply_board_layout_transforms(pkg_uid);

        self.scene.clear_selection();
        self.drag = Drag::None;
        self.banding = false;
        self.pending = None;
        self.dirty.mark_full();

        c.items = true;
        c.wires = true;
        c.props = true;
        c.history |= promoted;
        c.merge(self.set_center(pkg_w * 0.5 + 32.0, pkg_h * 0.5));
        c
    }

    /// Exit "Edit Board Layout" mode, committing `board` placements and restoring schematic positions.
    pub fn exit_board_layout(&mut self) -> Change {
        let Some(session) = self.board_layout.take() else {
            return Change::default();
        };
        let pkg_rect = self
            .scene
            .item_by_id(&session.pkg_uid)
            .and_then(|it| match &it.kind {
                Part::SubPackage(sp) => Some(Rect::new(
                    0.0,
                    0.0,
                    sp.package.body_w(),
                    sp.package.body_h(),
                )),
                _ => None,
            })
            .unwrap_or_default();

        commit_board_placements_and_restore(&mut self.scene, &session, pkg_rect);
        self.scene.clear_selection();
        self.drag = Drag::None;
        self.banding = false;
        self.pending = None;
        self.dirty.mark_full();

        Change {
            items: true,
            wires: true,
            props: true,
            history: true,
            ..Change::default()
        }
    }

    /// Build a schematic `Scene` snapshot with up-to-date `board` placements while in board layout mode.
    pub(crate) fn board_layout_scene_snapshot(&self) -> Scene {
        let mut cloned = self.scene.clone();
        if let Some(session) = &self.board_layout {
            let pkg_rect = self.board_layout_pkg_rect().unwrap_or_default();
            commit_board_placements_and_restore(&mut cloned, session, pkg_rect);
        }
        cloned
    }

    /// Re-apply board layout view after restoring a schematic undo/redo snapshot.
    pub(crate) fn reapply_board_layout_after_restore(&mut self) {
        let Some(pkg_uid) = self.board_layout.as_ref().map(|s| s.pkg_uid.clone()) else {
            return;
        };
        let exists = self
            .scene
            .item_by_id(&pkg_uid)
            .is_some_and(|it| matches!(it.kind, Part::SubPackage(_)));
        if !exists {
            self.board_layout = None;
            return;
        }
        self.apply_board_layout_transforms(&pkg_uid);
    }

    fn apply_board_layout_transforms(&mut self, pkg_uid: &str) -> (f64, f64) {
        let mut saved = HashMap::with_capacity(self.scene.items().len());
        let mut pkg_w = 32.0;
        let mut pkg_h = 32.0;
        for it in self.scene.items() {
            saved.insert(
                it.id.clone(),
                SavedCircTransform {
                    x: it.x,
                    y: it.y,
                    rotation: it.rotation,
                    hflip: it.hflip,
                    vflip: it.vflip,
                },
            );
            if it.id == pkg_uid {
                if let Part::SubPackage(sp) = &it.kind {
                    pkg_w = sp.package.body_w().max(8.0);
                    pkg_h = sp.package.body_h().max(8.0);
                }
            }
        }

        let mut park_x = ((pkg_w + 40.0) / 8.0).ceil() * 8.0;
        let mut park_y = 16.0;
        let max_col_h = pkg_h.max(200.0);
        let mut col_max_w = 48.0f64;

        for it in self.scene.items_mut() {
            if it.id == pkg_uid {
                it.x = 0.0;
                it.y = 0.0;
                it.rotation = 0.0;
                it.hflip = 1;
                it.vflip = 1;
                continue;
            }
            if !is_boardable_part(&it.kind) {
                continue;
            }
            if let Some(b) = it.board {
                it.x = b.x;
                it.y = b.y;
                it.rotation = b.rotation;
                it.hflip = b.hflip;
                it.vflip = b.vflip;
            } else {
                let body = it.body_rect();
                let step_h = (((body.h + 16.0).max(32.0)) / 8.0).ceil() * 8.0;
                let step_w = (((body.w + 24.0).max(48.0)) / 8.0).ceil() * 8.0;
                if park_y > 16.0 && park_y + step_h > max_col_h + 32.0 {
                    park_x += col_max_w;
                    park_y = 16.0;
                    col_max_w = 48.0;
                }
                it.x = park_x - body.x;
                it.y = park_y - body.y;
                park_y += step_h;
                col_max_w = col_max_w.max(step_w);
            }
        }

        self.board_layout = Some(BoardLayoutSession {
            pkg_uid: pkg_uid.to_string(),
            saved,
        });
        (pkg_w, pkg_h)
    }

    /// Hit-test only boardable components (topmost first) during board layout mode.
    pub(crate) fn hit_board_layout_item(&self, scene: Point) -> Option<usize> {
        let session = self.board_layout.as_ref()?;
        for (i, it) in self.scene.items().iter().enumerate().rev() {
            if it.id == session.pkg_uid || !is_boardable_part(&it.kind) {
                continue;
            }
            let local = it.map_scene(scene);
            if it.local_hit_rect().contains_point(local) {
                return Some(i);
            }
        }
        None
    }

    /// Hit-test any visible item (including the anchored `SubPackage`) during board layout mode.
    pub(crate) fn hit_board_layout_any(&self, scene: Point) -> Option<usize> {
        if let Some(i) = self.hit_board_layout_item(scene) {
            return Some(i);
        }
        let session = self.board_layout.as_ref()?;
        let idx = self
            .scene
            .items()
            .iter()
            .position(|it| it.id == session.pkg_uid)?;
        let pkg = &self.scene.items()[idx];
        let local = pkg.map_scene(scene);
        if pkg.local_hit_rect().contains_point(local) {
            Some(idx)
        } else {
            None
        }
    }
}

fn commit_board_placements_and_restore(
    scene: &mut Scene,
    session: &BoardLayoutSession,
    pkg_rect: Rect,
) {
    for it in scene.items_mut() {
        if it.id != session.pkg_uid && is_boardable_part(&it.kind) {
            let item_rect = map_rect(
                Point::new(it.x, it.y),
                it.body_rect(),
                it.rotation,
                it.hflip,
                it.vflip,
            );
            if pkg_rect.intersects(&item_rect) {
                it.board = Some(BoardPlacement {
                    x: it.x,
                    y: it.y,
                    rotation: it.rotation,
                    hflip: it.hflip,
                    vflip: it.vflip,
                });
            } else {
                it.board = None;
            }
        }
        if let Some(orig) = session.saved.get(&it.id) {
            it.x = orig.x;
            it.y = orig.y;
            it.rotation = orig.rotation;
            it.hflip = orig.hflip;
            it.vflip = orig.vflip;
        }
    }
}
