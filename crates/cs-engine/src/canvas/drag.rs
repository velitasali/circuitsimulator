//! Canvas drag state machine.

use crate::canvas::Point;
use crate::canvas::scene::{LabelAxisX, LabelAxisY};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AxisDetentState<T: Copy + Eq> {
    pub locked: Option<T>,
    pub suppressed: Option<T>,
    pub last_raw: Option<f64>,
}

impl<T: Copy + Eq> Default for AxisDetentState<T> {
    fn default() -> Self {
        Self {
            locked: None,
            suppressed: None,
            last_raw: None,
        }
    }
}

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
        start_custom: bool,
        last_scene: Point,
        x_detent: AxisDetentState<LabelAxisX>,
        y_detent: AxisDetentState<LabelAxisY>,
    },
    Wire {
        add_corner_on_release: bool,
    },
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
