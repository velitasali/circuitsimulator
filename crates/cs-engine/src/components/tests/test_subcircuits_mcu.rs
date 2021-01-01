//! Tests for subcircuits, packages, MCU, QEMU devices, dials, and schematic shapes.

use super::recorder::{DrawOp, record_part_paint};
use crate::components::dial::Dial;
use crate::components::shape::Shape;
use crate::components::*;

#[test]
fn test_dial_properties_and_wiper() {
    let mut dial = Dial::default();
    dial.set_prop_text("MinVal", "0").unwrap();
    dial.set_prop_text("MaxVal", "200").unwrap();
    dial.set_prop_text("Value", "150").unwrap();

    assert_eq!(dial.min_val, 0.0);
    assert_eq!(dial.max_val, 200.0);
    assert_eq!(dial.value, 150.0);
    assert!((dial.wiper() - 0.75).abs() < 1e-6);

    let rec = record_part_paint(&Part::Dial(dial));
    assert!(rec.is_all_finite());
    assert!(rec.ops.iter().any(|op| matches!(op, DrawOp::Arc { .. })));
    assert!(rec.ops.iter().any(|op| matches!(op, DrawOp::Line { .. })));
}

#[test]
fn test_shape_rect_ellipse_text_drawing() {
    // Rectangle
    let mut shape_rect = Shape::default();
    shape_rect.set_prop_text("ShapeKind", "Rectangle").unwrap();
    shape_rect.set_prop_text("Width", "80").unwrap();
    shape_rect.set_prop_text("Height", "40").unwrap();

    let rec_rect = record_part_paint(&Part::Shape(shape_rect));
    assert!(rec_rect.is_all_finite());
    assert!(rec_rect.ops.iter().any(|op| matches!(
        op,
        DrawOp::FillRoundRect { .. }
            | DrawOp::StrokeRoundRect { .. }
            | DrawOp::FillRect { .. }
            | DrawOp::StrokeRect { .. }
    )));

    // Ellipse
    let mut shape_ellipse = Shape::default();
    shape_ellipse.set_prop_text("ShapeKind", "Ellipse").unwrap();
    shape_ellipse.set_prop_text("Width", "50").unwrap();
    shape_ellipse.set_prop_text("Height", "50").unwrap();

    let rec_ellipse = record_part_paint(&Part::Shape(shape_ellipse));
    assert!(rec_ellipse.is_all_finite());
    assert!(rec_ellipse.ops.iter().any(|op| matches!(
        op,
        DrawOp::FillEllipse { .. } | DrawOp::StrokeEllipse { .. } | DrawOp::StrokeCircle { .. }
    )));

    // Text
    let mut shape_text = Shape::default();
    shape_text.set_prop_text("ShapeKind", "Text").unwrap();
    shape_text.set_prop_text("Text", "Circuit Heading").unwrap();
    let rec_text = record_part_paint(&Part::Shape(shape_text));
    assert!(rec_text.is_all_finite());
    assert!(rec_text.contains_text("Circuit Heading"));
}

#[test]
fn test_subcircuit_and_subpackage() {
    let mut sub = Subcircuit::default();
    sub.set_prop_text("LogicSymbol", "true").unwrap();
    assert!(sub.logic_symbol);

    let rec_sub = record_part_paint(&Part::Subcircuit(sub));
    assert!(rec_sub.is_all_finite());

    let pkg = SubPackage::default();
    let rec_pkg = record_part_paint(&Part::SubPackage(pkg));
    assert!(rec_pkg.is_all_finite());
}

#[test]
fn test_mcu_and_qemu_device() {
    let mcu = Mcu::default();
    let rec_mcu = record_part_paint(&Part::Mcu(mcu));
    assert!(rec_mcu.is_all_finite());

    let qemu = QemuDevice::default();
    let rec_qemu = record_part_paint(&Part::QemuDevice(qemu));
    assert!(rec_qemu.is_all_finite());
}
