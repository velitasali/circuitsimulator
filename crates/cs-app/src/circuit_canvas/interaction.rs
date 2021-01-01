//! Mouse, wheel, keyboard, and hover interaction helper algorithms.

use cs_engine::canvas::{Canvas, Point};

pub(super) fn hover_tooltip_at(canvas: &Canvas, x: f64, y: f64) -> Option<String> {
    let scene = canvas.viewport().map_to_circuit(Point::new(x, y));
    canvas.hover_tooltip(scene)
}

pub(super) fn drop_scene_point(canvas: &Canvas, x: f64, y: f64) -> Point {
    canvas.viewport().map_to_circuit(Point::new(x, y))
}
