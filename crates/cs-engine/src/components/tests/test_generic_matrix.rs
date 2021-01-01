//! Generic matrix sweep testing every component type in the library.

use super::recorder::{DrawRecorder, record_part_paint};
use crate::canvas::Canvas;
use crate::canvas::draw::{PaintCtx, Palette};
use crate::components::harness::CASES;
use std::collections::HashSet;

#[test]
fn test_all_components_default_geometry_and_drawing() {
    let canvas = Canvas::new();
    let pal = Palette::light();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: "test",
    };

    for case in CASES {
        let part = (case.make)();
        let type_id = part.type_id();
        assert_eq!(type_id, case.itemtype, "Type ID mismatch for case");

        // 1. Verify body and visual_rect
        let body = part.body();
        assert!(
            body.x.is_finite() && body.y.is_finite() && body.w.is_finite() && body.h.is_finite(),
            "{type_id}: body rect has non-finite values: {body:?}"
        );
        assert!(
            body.w >= 0.0 && body.h >= 0.0,
            "{type_id}: body dimensions must be non-negative"
        );

        let visual = part.visual_rect();
        assert!(
            visual.x.is_finite()
                && visual.y.is_finite()
                && visual.w.is_finite()
                && visual.h.is_finite(),
            "{type_id}: visual rect has non-finite values: {visual:?}"
        );

        // 2. Verify pin geometry
        let pins = part.pin_geoms();
        let mut suffixes = HashSet::new();
        for (i, pin) in pins.iter().enumerate() {
            assert!(
                pin.local.x.is_finite() && pin.local.y.is_finite(),
                "{type_id} pin #{i} ({}) local coord is non-finite: {:?}",
                pin.suffix,
                pin.local
            );
            assert!(
                pin.length.is_finite() && pin.length >= 0.0,
                "{type_id} pin #{i} ({}) length is invalid: {}",
                pin.suffix,
                pin.length
            );
            assert!(
                suffixes.insert(pin.suffix.clone()),
                "{type_id} contains duplicate pin suffix: {}",
                pin.suffix
            );
        }

        // 3. Verify drawing
        let mut recorder = DrawRecorder::new();
        let _ = part.paint(&mut recorder, &ctx);
        assert!(
            recorder.is_all_finite(),
            "{type_id} emitted non-finite draw coordinates: {:#?}",
            recorder.ops
        );
    }
}

#[test]
fn test_all_components_extra_props_and_paint_effects() {
    for case in CASES {
        for &(prop, val1, val2) in case.extras {
            let mut p1 = (case.make)();
            let mut p2 = (case.make)();

            p1.set_prop_text(prop, val1)
                .unwrap_or_else(|e| panic!("{}: failed to set {prop}={val1}: {e}", case.itemtype));
            p2.set_prop_text(prop, val2)
                .unwrap_or_else(|e| panic!("{}: failed to set {prop}={val2}: {e}", case.itemtype));

            // Verify prop reflection
            let got1 = p1.get_prop_text(prop).unwrap();
            let got2 = p2.get_prop_text(prop).unwrap();
            assert_ne!(
                got1, got2,
                "{}: distinct values for {prop} ('{val1}' vs '{val2}') resulted in identical text '{got1}'",
                case.itemtype
            );

            // Verify geometry sanity after setting prop
            for p in [&p1, &p2] {
                let body = p.body();
                assert!(
                    body.x.is_finite()
                        && body.y.is_finite()
                        && body.w.is_finite()
                        && body.h.is_finite()
                );
                for pin in p.pin_geoms() {
                    assert!(pin.local.x.is_finite() && pin.local.y.is_finite());
                    assert!(pin.length.is_finite() && pin.length >= 0.0);
                }
            }

            // Verify drawing execution and sanity
            let rec1 = record_part_paint(&p1);
            let rec2 = record_part_paint(&p2);
            assert!(
                rec1.is_all_finite(),
                "{}: draw ops non-finite for {prop}={val1}",
                case.itemtype
            );
            assert!(
                rec2.is_all_finite(),
                "{}: draw ops non-finite for {prop}={val2}",
                case.itemtype
            );
        }
    }
}
