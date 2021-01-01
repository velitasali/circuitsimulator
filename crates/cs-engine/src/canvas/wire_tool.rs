//! Interactive wire editing, corner/segment dragging, and routing operations.

use crate::canvas::drag::{Drag, WireEditMode};
use crate::canvas::events::{Change, CursorKind};
use crate::canvas::geom::to_grid as snap_point;
use crate::canvas::{Canvas, Point};

impl Canvas {
    pub(crate) fn begin_wire_edit(
        &mut self,
        wire: usize,
        mode: WireEditMode,
        scene: Point,
        mut c: Change,
    ) -> Change {
        let origin = snap_point(scene);
        self.pending = Some(self.snapshot());
        self.drag = Drag::WireEdit {
            last_scene: origin,
            origin,
            wire,
            mode,
        };
        let want = match mode {
            WireEditMode::Group => CursorKind::SizeAll,
            WireEditMode::Corner(_) => CursorKind::SizeAll,
            WireEditMode::Segment(seg) => {
                let w = &self.scene.wires()[wire];
                if w.segment_is_horizontal(seg) {
                    CursorKind::SplitV
                } else if w.segment_is_vertical(seg) {
                    CursorKind::SplitH
                } else {
                    CursorKind::SizeAll
                }
            }
        };
        c.merge(self.set_cursor(want));
        self.dirty_wire_index(wire);
        c.wires = true;
        c
    }

    pub(crate) fn finish_selected_wires(&mut self) {
        self.dirty_move_footprint();
        let idxs: Vec<usize> = self
            .scene
            .wires()
            .iter()
            .enumerate()
            .filter(|(_, w)| w.selected && w.closed())
            .map(|(i, _)| i)
            .collect();
        for i in idxs {
            self.scene.finish_wire_drag(i);
        }
        self.dirty_move_footprint();
    }
}
