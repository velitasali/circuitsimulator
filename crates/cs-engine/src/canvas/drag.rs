//! Canvas drag state machine.

use crate::canvas::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WireEditMode {
    Segment(usize),
    Corner(usize),
    Group,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Drag {
    None,
    Pan {
        last_item: Point,
    },
    Band {
        origin: Point,
    },
    Move {
        last_scene: Point,
        origin: Point,
        index: usize,
        press_pos: Point,
    },
    MoveLabel {
        index: usize,
        is_val: bool,
        origin: Point,
        start_pos: Point,
        last_scene: Point,
    },
    Wire,
    WireEdit {
        last_scene: Point,
        origin: Point,
        wire: usize,
        mode: WireEditMode,
    },
    Interact {
        index: usize,
    },
}
