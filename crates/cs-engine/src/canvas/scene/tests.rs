use super::*;
use crate::canvas::{Point, Rect};

#[test]
fn hit_body_and_miss() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    // Resistor body is [-11.0, -4.5, 22.0, 9.0] (x in -11..11, y in -4.5..4.5)
    assert_eq!(s.hit(Point::zero()), Some(0));
    assert_eq!(s.hit(Point::new(10.0, 0.0)), Some(0)); // within body
    assert_eq!(s.hit(Point::new(-16.0, 0.0)), None); // outside body width (along pin stem)
    assert_eq!(s.hit(Point::new(0.0, 6.0)), None); // outside body height (above/below)
    assert_eq!(s.hit(Point::new(100.0, 100.0)), None);
}

#[test]
fn selection_rect_body_margin_excludes_pins() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    let res = &s.items()[0];
    // Resistor body is [-11.0, -4.5, 22.0, 9.0]. With 8px margin it is [-19.0, -12.5, 38.0, 25.0].
    assert_eq!(res.body_rect(), Rect::new(-11.0, -4.5, 22.0, 9.0));
    assert_eq!(
        res.local_selection_rect(),
        Rect::new(-19.0, -12.5, 38.0, 25.0)
    );
    assert_eq!(res.selection_rect(), Rect::new(-19.0, -12.5, 38.0, 25.0));

    // Subcircuit with pins extending outside body + 8px margin
    let pkg = crate::package::Package {
        name: "TEST_DIP".into(),
        width: 4,  // 32px
        height: 4, // 32px
        pins: vec![crate::package::PkgPin {
            id: "p1".into(),
            label: "P1".into(),
            xpos: -16, // pin extends 16px to the left of the body origin (0, 0)
            ypos: 16,
            length: 16,
            space: 0,
            angle: 180,
            pin_type: "normal".into(),
        }],
        ..Default::default()
    };
    s.items.push(Item::subcircuit(
        "sub_1", 100.0, 100.0, "sub_1", pkg, "", None, false,
    ));
    let chip = &s.items()[1];
    // Body is [0.0, 0.0, 32.0, 32.0]. With 8px margin: [-8.0, -8.0, 48.0, 48.0].
    assert_eq!(chip.body_rect(), Rect::new(0.0, 0.0, 32.0, 32.0));
    assert_eq!(
        chip.local_selection_rect(),
        Rect::new(-8.0, -8.0, 48.0, 48.0)
    );
    assert_eq!(chip.selection_rect(), Rect::new(92.0, 92.0, 48.0, 48.0));

    // A rubber band hitting the distant pin tip at x=84 (100 - 16) but outside selection_rect (x: 92..140)
    s.select_intersecting(Rect::new(80.0, 110.0, 8.0, 10.0));
    assert!(
        !s.items()[1].selected,
        "Rubber band on extended pin outside body+margin must NOT select chip"
    );

    // A rubber band touching the selection box (e.g. at x=95, y=100)
    s.select_intersecting(Rect::new(90.0, 95.0, 10.0, 10.0));
    assert!(
        s.items()[1].selected,
        "Rubber band touching selection rect MUST select chip"
    );
}

#[test]
fn stepper_body_rect_and_pins() {
    let mut s = Scene::new();
    s.add_stepper(100.0, 200.0);
    let stepper = &s.items()[0];
    assert_eq!(stepper.body_rect(), Rect::new(-40.0, -40.0, 80.0, 80.0));
    assert_eq!(
        stepper.local_selection_rect(),
        Rect::new(-48.0, -48.0, 96.0, 96.0)
    );
    assert_eq!(stepper.selection_rect(), Rect::new(52.0, 152.0, 96.0, 96.0));

    let pins = stepper.pins();
    assert_eq!(pins.len(), 5);
    for p in &pins {
        assert_eq!(p.local.x, -48.0);
        assert_eq!(p.angle, 180);
        assert_eq!(p.length, 8.0);
        // Stem ends at -48.0 + 8.0 = -40.0 (exact body border)
        assert_eq!(p.local.x + p.length, -40.0);
    }
}

#[test]
fn topmost_wins() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(4.0, 0.0);
    assert_eq!(s.hit(Point::new(4.0, 0.0)), Some(1));
}

#[test]
fn exclusive_and_toggle_select() {
    let mut s = Scene::new();
    s.add_default_resistor(-40.0, 0.0);
    s.add_default_resistor(40.0, 0.0);
    assert!(s.select_only(0));
    assert!(s.items()[0].selected && !s.items()[1].selected);
    assert!(s.toggle_selected(1));
    assert!(s.items()[0].selected && s.items()[1].selected);
    assert!(s.clear_selection());
    assert!(!s.any_selected());
}

#[test]
fn rubber_band_intersects() {
    let mut s = Scene::new();
    s.add_default_resistor(-80.0, 0.0);
    s.add_default_resistor(80.0, 0.0);
    s.select_intersecting(Rect::new(-20.0, -20.0, 40.0, 40.0));
    assert!(!s.items()[0].selected && !s.items()[1].selected);
    s.select_intersecting(Rect::new(-100.0, -20.0, 40.0, 40.0));
    assert!(s.items()[0].selected && !s.items()[1].selected);
}

#[test]
fn delete_selected() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(40.0, 0.0);
    s.select_only(0);
    assert!(s.delete_selected());
    assert_eq!(s.items().len(), 1);
    assert_eq!(s.items()[0].id, "Resistor-2");
}

#[test]
fn pin_hit_on_resistor() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    let p = s.hit_pin(Point::new(-16.0, 0.0)).unwrap();
    assert_eq!(p.id, "Resistor-1-lPin");
    assert!(s.hit_pin(Point::new(0.0, 0.0)).is_none());
}

#[test]
fn opamp_follower_solves() {
    let mut s = Scene::new();
    s.add_fixed_volt(-80.0, 0.0, 2.5);
    s.add_opamp(0.0, 0.0);
    let vin = s.pin_scene("Fixed Voltage-1-outnod").unwrap();
    let plus = s.pin_scene("opAmp-1-inputNinv").unwrap();
    let minus = s.pin_scene("opAmp-1-inputInv").unwrap();
    let out = s.pin_scene("opAmp-1-output").unwrap();
    s.connect_pins("Fixed Voltage-1-outnod", vin, "opAmp-1-inputNinv", plus);
    s.connect_pins("opAmp-1-output", out, "opAmp-1-inputInv", minus);
    let mut c = s.to_circuit();
    c.solve().unwrap();
    let vo = c.pin_voltage("opAmp-1-output").unwrap();
    assert!((vo - 2.5).abs() < 0.02, "follower {vo}");
}

#[test]
fn jfet_idss_solves() {
    let mut s = Scene::new();
    s.add_fixed_volt(-80.0, 0.0, 12.0);
    s.add_resistor(0.0, 0.0, 100.0);
    s.add_jfet(80.0, 0.0);
    s.add_ground(112.0, 16.0);
    let vdd = s.pin_scene("Fixed Voltage-1-outnod").unwrap();
    let r1l = s.pin_scene("Resistor-1-lPin").unwrap();
    let r1r = s.pin_scene("Resistor-1-rPin").unwrap();
    let drain = s.pin_scene("Jfet-1-Dren").unwrap();
    let source = s.pin_scene("Jfet-1-Sour").unwrap();
    let gate = s.pin_scene("Jfet-1-Gate").unwrap();
    let gnd = s.pin_scene("Ground-1-Gnd").unwrap();
    s.connect_pins("Fixed Voltage-1-outnod", vdd, "Resistor-1-lPin", r1l);
    s.connect_pins("Resistor-1-rPin", r1r, "Jfet-1-Dren", drain);
    s.connect_pins("Jfet-1-Sour", source, "Ground-1-Gnd", gnd);
    s.connect_pins("Jfet-1-Gate", gate, "Ground-1-Gnd", gnd);
    let mut c = s.to_circuit();
    c.solve().unwrap();
    let i = c.resistor_current("Resistor-1").unwrap();
    assert!(i > 0.04, "Id {i}");
}

#[test]
fn comparator_high_solves() {
    let mut s = Scene::new();
    s.add_fixed_volt(-80.0, 0.0, 5.0);
    s.add_comparator(0.0, 0.0);
    s.add_resistor(80.0, 0.0, 10_000.0);
    s.add_ground(120.0, 0.0);
    let vin = s.pin_scene("Fixed Voltage-1-outnod").unwrap();
    let plus = s.pin_scene("Comparator-1-in0").unwrap();
    let minus = s.pin_scene("Comparator-1-in1").unwrap();
    let out = s.pin_scene("Comparator-1-out").unwrap();
    let rl = s.pin_scene("Resistor-1-lPin").unwrap();
    let rr = s.pin_scene("Resistor-1-rPin").unwrap();
    let gnd = s.pin_scene("Ground-1-Gnd").unwrap();
    s.connect_pins("Fixed Voltage-1-outnod", vin, "Comparator-1-in0", plus);
    s.connect_pins("Comparator-1-in1", minus, "Ground-1-Gnd", gnd);
    s.connect_pins("Comparator-1-out", out, "Resistor-1-lPin", rl);
    s.connect_pins("Resistor-1-rPin", rr, "Ground-1-Gnd", gnd);
    let mut c = s.to_circuit();
    c.solve().unwrap();
    let vo = c.pin_voltage("Comparator-1-out").unwrap();
    assert!(vo > 4.5, "comparator high {vo}");
}

#[test]
fn volt_reg_solves() {
    let mut s = Scene::new();
    s.add_fixed_volt(-80.0, 0.0, 12.0);
    s.add_volt_reg(0.0, 0.0);
    s.add_resistor(80.0, 0.0, 100.0);
    s.add_ground(120.0, 16.0);
    let vin = s.pin_scene("Fixed Voltage-1-outnod").unwrap();
    let i_pin = s.pin_scene("VoltReg-1-input").unwrap();
    let o_pin = s.pin_scene("VoltReg-1-output").unwrap();
    let r_pin = s.pin_scene("VoltReg-1-ref").unwrap();
    let rl = s.pin_scene("Resistor-1-lPin").unwrap();
    let rr = s.pin_scene("Resistor-1-rPin").unwrap();
    let gnd = s.pin_scene("Ground-1-Gnd").unwrap();
    s.connect_pins("Fixed Voltage-1-outnod", vin, "VoltReg-1-input", i_pin);
    s.connect_pins("VoltReg-1-output", o_pin, "Resistor-1-lPin", rl);
    s.connect_pins("Resistor-1-rPin", rr, "Ground-1-Gnd", gnd);
    s.connect_pins("VoltReg-1-ref", r_pin, "Ground-1-Gnd", gnd);
    let mut c = s.to_circuit();
    c.solve().unwrap();
    let vo = c.pin_voltage("VoltReg-1-output").unwrap();
    assert!((vo - 1.2).abs() < 0.05, "regulated {vo}");
}

#[test]
fn wire_and_solve_divider() {
    let mut s = Scene::new();
    s.add_demo_divider();
    assert_eq!(s.items().len(), 4);
    assert_eq!(s.wires().len(), 3);
    let mut c = s.to_circuit();
    c.solve().unwrap();
    let mid = c.pin_voltage("Resistor-1-rPin").unwrap();
    assert!((mid - 2.5).abs() < 1e-6, "mid {mid}");
}

#[test]
fn delete_item_drops_wires() {
    let mut s = Scene::new();
    s.add_demo_divider();
    s.select_only(1); // Resistor-1
    assert!(s.delete_selected());
    assert_eq!(s.items().len(), 3);
    assert_eq!(s.wires().len(), 1); // only R2–GND remains
}

#[test]
fn load_divider_infers_positions_and_solves() {
    let src = include_str!("../../../tests/fixtures/divider.sim1");
    let s = Scene::from_sim1(src).unwrap();
    assert_eq!(s.items().len(), 5);
    assert_eq!(s.wires().len(), 4);
    assert!(s.items().iter().any(|i| i.kind.type_name() == "Node"));
    let r2 = s.items().iter().find(|i| i.id == "Resistor-2").unwrap();
    assert!((r2.x - 164.0).abs() < 1e-6, "R2 x {}", r2.x);
    let mut c = s.to_circuit();
    c.solve().unwrap();
    assert!((c.pin_voltage("Node-5-0").unwrap() - 2.5).abs() < 1e-6);
    let round = s.to_sim1();
    let s2 = Scene::from_sim1(&round).unwrap();
    assert_eq!(s2.items().len(), 5);
    assert_eq!(s2.wires().len(), 4);
    let mut c2 = s2.to_circuit();
    c2.solve().unwrap();
    assert!((c2.pin_voltage("Node-5-0").unwrap() - 2.5).abs() < 1e-6);
}

#[test]
fn start_and_cancel_wire() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(80.0, 0.0);
    let pin = s.hit_pin(Point::new(-16.0, 0.0)).unwrap();
    assert!(s.start_wire(&pin, Point::new(-16.0, 0.0)));
    assert!(s.drawing());
    s.route_draft(Point::new(40.0, 0.0), false);
    assert!(s.cancel_wire());
    assert!(!s.drawing());
    assert!(s.wires().is_empty());
}

#[test]
fn bus_mismatch_refuses_close() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(80.0, 0.0);
    let a = s.hit_pin(Point::new(-16.0, 0.0)).unwrap();
    assert!(s.start_wire(&a, Point::new(-16.0, 0.0)));
    s.wires_mut()[0].is_bus = true;
    let b = s.hit_pin(Point::new(64.0, 0.0)).unwrap();
    assert!(!s.close_wire(&b, Point::new(64.0, 0.0)));
    assert!(s.drawing());
}

#[test]
fn splice_bus_keeps_bus() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(80.0, 0.0);
    let a = s.hit_pin(Point::new(16.0, 0.0)).unwrap();
    let b = s.hit_pin(Point::new(64.0, 0.0)).unwrap();
    s.connect_pins(&a.id, Point::new(16.0, 0.0), &b.id, Point::new(64.0, 0.0));
    s.wires_mut()[0].is_bus = true;
    assert!(s.splice_and_start(0, Point::new(40.0, 0.0)));
    assert!(s.drawing());
    assert!(s.wires().iter().filter(|w| w.closed()).all(|w| w.is_bus));
    assert!(s.wires().iter().find(|w| w.drawing()).unwrap().is_bus);
    assert!(
        s.pin_is_bus(
            &s.items()
                .iter()
                .find(|it| it.kind.type_name() == "Node")
                .unwrap()
                .pins()[1]
                .id
        )
    );
}

#[test]
fn qemu_esp32_canvas_lifecycle() {
    let mut s = Scene::new();
    let id = s
        .add_qemu(
            "Esp32",
            100.0,
            100.0,
            &crate::subcircuit::SubcSearch::default(),
        )
        .expect("add esp32");
    assert_eq!(id, "Esp32-1");
    let it = s.items().iter().find(|it| it.id == id).expect("item found");
    assert_eq!(it.kind.type_name(), "QemuDevice");
    assert_eq!(it.pkg_w(), 15);
    assert_eq!(it.pkg_h(), 15);
    let pins = it.pins();
    assert!(pins.iter().any(|p| p.id == "Esp32-1-G00"));
    assert!(pins.iter().any(|p| p.id == "Esp32-1-Rst"));

    // Round-trip to sim1
    let sim1 = s.to_sim1();
    assert!(sim1.contains("itemtype=\"QemuDevice\""));
    assert!(sim1.contains("CircId=\"Esp32-1\""));

    let loaded = Scene::from_sim1(&sim1).expect("parse sim1");
    let it2 = loaded
        .items()
        .iter()
        .find(|it| it.id == "Esp32-1")
        .expect("item in loaded scene");
    assert_eq!(it2.kind.type_name(), "QemuDevice");
    assert_eq!(it2.pkg_w(), 15);

    // Convert to circuit
    let circ = loaded.to_circuit();
    assert!(circ.components().iter().any(|c| c.id == "Esp32-1"));
}

#[test]
fn mcu_canvas_lifecycle_and_relaunch_roundtrip() {
    let mut s = Scene::new();
    let search = crate::subcircuit::SubcSearch::from_circuit_path(None).with_standard_catalog();
    let id = s
        .add_mcu("mega328", 100.0, 100.0, &search)
        .expect("add mega328");
    assert_eq!(id, "mega328-1");
    let it = s.items().iter().find(|it| it.id == id).expect("item found");
    assert_eq!(it.kind.type_name(), "Mcu");
    let initial_w = it.pkg_w();
    let initial_h = it.pkg_h();
    let initial_pins_len = it.pins().len();
    assert!(initial_w > 0 && initial_h > 0);
    assert!(initial_pins_len > 0);

    // Round-trip to sim1
    let sim1 = s.to_sim1();
    assert!(sim1.contains("itemtype=\"Mcu\"") || sim1.contains("itemtype=\"MCU\""));
    assert!(sim1.contains("CircId=\"mega328-1\""));
    assert!(sim1.contains("Device=\"mega328\""));
    assert!(!sim1.contains("Device=\"mega328-1\""));

    let loaded = Scene::from_sim1_with(&sim1, &search).expect("parse sim1");
    let it2 = loaded
        .items()
        .iter()
        .find(|it| it.id == "mega328-1")
        .expect("item in loaded scene");
    assert_eq!(it2.kind.type_name(), "Mcu");
    assert_eq!(it2.pkg_w(), initial_w);
    assert_eq!(it2.pkg_h(), initial_h);
    assert_eq!(it2.pins().len(), initial_pins_len);
    assert!(!it2.pins().is_empty());

    // Legacy / corrupted sim1 with Device="mega328-1" should also heal gracefully
    let legacy_sim1 = r#"<circuit version="1.0.0">
<item itemtype="Mcu" CircId="mega328-1" Device="mega328-1" Pos="100,100" />
</circuit>"#;
    let legacy_loaded = Scene::from_sim1_with(legacy_sim1, &search).expect("parse legacy sim1");
    let it_legacy = legacy_loaded
        .items()
        .iter()
        .find(|it| it.id == "mega328-1")
        .expect("item in legacy scene");
    assert_eq!(it_legacy.pkg_w(), initial_w);
    assert_eq!(it_legacy.pkg_h(), initial_h);
    assert_eq!(it_legacy.pins().len(), initial_pins_len);
}

#[test]
fn canvas_add_component_spec_esp32() {
    let mut c = crate::canvas::Canvas::new();
    assert!(c.scene_add_component_spec("Esp32,QemuDevice", Point::new(40.0, 40.0)));
    assert_eq!(c.scene().items().len(), 1);
    assert_eq!(c.scene().items()[0].id, "Esp32-1");
}

#[test]
fn test_esp32_unused_pins_non_connectible() {
    let mut s = Scene::new();
    let id = s
        .add_qemu(
            "Esp32",
            100.0,
            100.0,
            &crate::subcircuit::SubcSearch::default(),
        )
        .expect("add esp32");
    let it = s.items().iter().find(|it| it.id == id).expect("item found");
    let pins = it.pins();

    // Verify VDD / VddA pins are marked unused
    let vdd_pins: Vec<_> = pins.iter().filter(|p| p.label.starts_with("Vdd")).collect();
    assert!(!vdd_pins.is_empty());
    for vp in &vdd_pins {
        assert!(vp.unused, "pin {} should be unused", vp.id);
    }

    // Verify GPIO pins are NOT unused
    let g00 = pins
        .iter()
        .find(|p| p.id == "Esp32-1-G00")
        .cloned()
        .expect("G00");
    assert!(!g00.unused);
    let g00_pos = it.pin_scene_pos(&g00);

    let vdd2 = pins
        .iter()
        .find(|p| p.id == "Esp32-1-Vdd2")
        .cloned()
        .expect("Vdd2");
    let vdd_pos = it.pin_scene_pos(&vdd2);

    // Attempting to start a wire from an unused pin must fail
    assert!(!s.start_wire(&vdd2, vdd_pos));

    // Start a wire from G00 (valid)
    assert!(s.start_wire(&g00, g00_pos));
    assert!(s.drawing());

    // Attempting to close wire on unused pin must fail
    assert!(!s.close_wire(&vdd2, vdd_pos));
    assert!(s.drawing());

    // Cancel the draft wire
    assert!(s.cancel_wire());
    assert!(!s.drawing());
}

#[test]
fn test_node_removed_when_third_wire_deleted() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0); // R1
    s.add_default_resistor(80.0, 0.0); // R2
    s.add_default_resistor(40.0, 60.0); // R3

    let r1_r = s.hit_pin(Point::new(16.0, 0.0)).unwrap();
    let r2_l = s.hit_pin(Point::new(64.0, 0.0)).unwrap();
    let r3_l = s.hit_pin(Point::new(24.0, 60.0)).unwrap();

    // 1. Connect R1 to R2
    s.connect_pins(
        &r1_r.id,
        Point::new(16.0, 0.0),
        &r2_l.id,
        Point::new(64.0, 0.0),
    );
    assert_eq!(s.wires().len(), 1);

    // 2. Splice wire and connect 3rd branch to R3
    assert!(s.splice_and_start(0, Point::new(40.0, 0.0)));
    assert!(s.close_wire(&r3_l, Point::new(24.0, 60.0)));

    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        1
    );
    assert_eq!(s.wires().len(), 3);

    // 3. Select the 3rd wire (the one connected to R3) and delete it
    let branch_idx = s
        .wires()
        .iter()
        .position(|w| w.start_pin == r3_l.id || w.end_pin.as_deref() == Some(&r3_l.id))
        .expect("branch wire found");
    s.wires_mut()[branch_idx].selected = true;

    assert!(s.delete_selected());

    // Node must be removed
    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        0,
        "Node should be removed when 3rd wire is deleted"
    );
    // The remaining 2 wire segments must be joined into 1 wire connecting R1 and R2
    assert_eq!(
        s.wires().len(),
        1,
        "Remaining 2 wires should be merged into 1"
    );
    let merged = &s.wires()[0];
    let connects_r1_r2 = (merged.start_pin == r1_r.id
        && merged.end_pin.as_deref() == Some(&r2_l.id))
        || (merged.start_pin == r2_l.id && merged.end_pin.as_deref() == Some(&r1_r.id));
    assert!(connects_r1_r2, "Merged wire must connect R1 and R2");
}

#[test]
fn test_node_removed_when_connected_component_deleted() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0); // R1
    s.add_default_resistor(80.0, 0.0); // R2
    s.add_default_resistor(40.0, 60.0); // R3

    let r1_r = s.hit_pin(Point::new(16.0, 0.0)).unwrap();
    let r2_l = s.hit_pin(Point::new(64.0, 0.0)).unwrap();
    let r3_l = s.hit_pin(Point::new(24.0, 60.0)).unwrap();

    s.connect_pins(
        &r1_r.id,
        Point::new(16.0, 0.0),
        &r2_l.id,
        Point::new(64.0, 0.0),
    );
    assert!(s.splice_and_start(0, Point::new(40.0, 0.0)));
    assert!(s.close_wire(&r3_l, Point::new(24.0, 60.0)));

    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        1
    );
    assert_eq!(s.wires().len(), 3);

    // Select R3 and delete it
    let r3_item = s
        .items_mut()
        .iter_mut()
        .find(|it| it.id == "Resistor-3")
        .unwrap();
    r3_item.selected = true;

    assert!(s.delete_selected());

    // R3 gone, Node gone, wires joined back into 1
    assert!(s.items().iter().all(|it| it.id != "Resistor-3"));
    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        0
    );
    assert_eq!(s.wires().len(), 1);
    let merged = &s.wires()[0];
    let connects_r1_r2 = (merged.start_pin == r1_r.id
        && merged.end_pin.as_deref() == Some(&r2_l.id))
        || (merged.start_pin == r2_l.id && merged.end_pin.as_deref() == Some(&r1_r.id));
    assert!(connects_r1_r2);
}

#[test]
fn test_node_deleted_directly_drops_attached_wires() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(80.0, 0.0);
    s.add_default_resistor(40.0, 60.0);

    let r1_r = s.hit_pin(Point::new(16.0, 0.0)).unwrap();
    let r2_l = s.hit_pin(Point::new(64.0, 0.0)).unwrap();
    let r3_l = s.hit_pin(Point::new(24.0, 60.0)).unwrap();

    s.connect_pins(
        &r1_r.id,
        Point::new(16.0, 0.0),
        &r2_l.id,
        Point::new(64.0, 0.0),
    );
    assert!(s.splice_and_start(0, Point::new(40.0, 0.0)));
    assert!(s.close_wire(&r3_l, Point::new(24.0, 60.0)));

    let node_item = s
        .items_mut()
        .iter_mut()
        .find(|it| it.kind.type_name() == "Node")
        .unwrap();
    node_item.selected = true;

    assert!(s.delete_selected());

    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        0
    );
    assert_eq!(s.wires().len(), 0);
}

#[test]
fn test_cascading_node_removal() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0); // R1
    s.add_default_resistor(120.0, 0.0); // R2
    s.add_default_resistor(30.0, 60.0); // R3
    s.add_default_resistor(90.0, 60.0); // R4

    let r1_r = s.hit_pin(Point::new(16.0, 0.0)).unwrap();
    let r2_l = s.hit_pin(Point::new(104.0, 0.0)).unwrap();
    let r3_l = s.hit_pin(Point::new(14.0, 60.0)).unwrap();
    let r4_l = s.hit_pin(Point::new(74.0, 60.0)).unwrap();

    // Connect R1 to R2
    s.connect_pins(
        &r1_r.id,
        Point::new(16.0, 0.0),
        &r2_l.id,
        Point::new(104.0, 0.0),
    );

    // First splice for R3
    assert!(s.splice_and_start(0, Point::new(30.0, 0.0)));
    assert!(s.close_wire(&r3_l, Point::new(14.0, 60.0)));

    // Find wire between Node-1 and R2 to splice second time for R4
    let w_idx = s
        .wires()
        .iter()
        .position(|w| w.start_pin == r2_l.id || w.end_pin.as_deref() == Some(&r2_l.id))
        .unwrap();
    assert!(s.splice_and_start(w_idx, Point::new(90.0, 0.0)));
    assert!(s.close_wire(&r4_l, Point::new(74.0, 60.0)));

    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        2
    );

    // Delete both branch resistors (R3 and R4)
    for it in s.items_mut() {
        if it.id == "Resistor-3" || it.id == "Resistor-4" {
            it.selected = true;
        }
    }
    assert!(s.delete_selected());

    // Both nodes should be cleanly removed and all remaining segments merged into 1 wire connecting R1 to R2
    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        0
    );
    assert_eq!(s.wires().len(), 1);
    let merged = &s.wires()[0];
    let connects_r1_r2 = (merged.start_pin == r1_r.id
        && merged.end_pin.as_deref() == Some(&r2_l.id))
        || (merged.start_pin == r2_l.id && merged.end_pin.as_deref() == Some(&r1_r.id));
    assert!(connects_r1_r2);
}

#[test]
fn test_splice_and_cancel_wire_restores_original_wire() {
    let mut s = Scene::new();
    s.add_default_resistor(0.0, 0.0);
    s.add_default_resistor(80.0, 0.0);

    let r1_r = s.hit_pin(Point::new(16.0, 0.0)).unwrap();
    let r2_l = s.hit_pin(Point::new(64.0, 0.0)).unwrap();

    s.connect_pins(
        &r1_r.id,
        Point::new(16.0, 0.0),
        &r2_l.id,
        Point::new(64.0, 0.0),
    );
    assert_eq!(s.wires().len(), 1);

    assert!(s.splice_and_start(0, Point::new(40.0, 0.0)));
    assert!(s.drawing());
    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        1
    );

    // Cancel the draft wire
    assert!(s.cancel_wire());
    assert!(!s.drawing());

    // Node must be removed and wire merged back into 1
    assert_eq!(
        s.items()
            .iter()
            .filter(|it| it.kind.type_name() == "Node")
            .count(),
        0
    );
    assert_eq!(s.wires().len(), 1);
    let merged = &s.wires()[0];
    let connects_r1_r2 = (merged.start_pin == r1_r.id
        && merged.end_pin.as_deref() == Some(&r2_l.id))
        || (merged.start_pin == r2_l.id && merged.end_pin.as_deref() == Some(&r1_r.id));
    assert!(connects_r1_r2);
}

#[test]
fn test_dynamic_label_and_val_positioning_based_on_clickable_area() {
    let mut r = Item::resistor("R1", 0.0, 0.0, 1000.0);
    let rect = r.body_rect();
    let cx = rect.x + rect.w * 0.5;

    // Auto position: label above clickable area, horizontally centered
    let (lx, ly) = r.label_pos();
    assert!(
        ly < rect.y,
        "label Y ({}) should be above component top ({})",
        ly,
        rect.y
    );
    let tw = crate::canvas::export::text::text_width(&r.label, 9.0, false);
    let label_cx = lx + tw * 0.5;
    assert!(
        (label_cx - cx).abs() < 1e-6,
        "label should be horizontally centered on clickable area"
    );

    // Auto position: value below clickable area, horizontally centered
    let (vx, vy) = r.val_pos();
    assert!(
        vy > rect.y + rect.h,
        "value Y ({}) should be below component bottom ({})",
        vy,
        rect.y + rect.h
    );
    let val_text = r.val_label_text();
    let val_tw = crate::canvas::export::text::text_width(&val_text, 9.0, false);
    let val_cx = vx + val_tw * 0.5;
    assert!(
        (val_cx - cx).abs() < 1e-6,
        "val should be horizontally centered on clickable area"
    );

    // Gate size dynamic expansion: 2 inputs vs 8 inputs
    let gate2 = Item::gate("G2", 0.0, 0.0, "AND", 2, false, false, false, false, false);
    let gate8 = Item::gate("G8", 0.0, 0.0, "AND", 8, false, false, false, false, false);
    assert!(gate8.body_rect().h > gate2.body_rect().h);
    assert!(
        gate8.label_pos().1 < gate2.label_pos().1,
        "8-input gate label must be higher than 2-input gate label"
    );
    assert!(
        gate8.val_pos().1 > gate2.val_pos().1,
        "8-input gate value must be lower than 2-input gate value"
    );

    // Setting custom position
    r.label_x = 42.0;
    r.label_y = -50.0;
    r.custom_label_pos = true;
    assert_eq!(r.label_pos(), (42.0, -50.0));

    r.val_x = 42.0;
    r.val_y = 50.0;
    r.custom_val_pos = true;
    assert_eq!(r.val_pos(), (42.0, 50.0));
}
