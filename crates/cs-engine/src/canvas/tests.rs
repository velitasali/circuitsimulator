//! Canvas unit and regression tests.

use super::*;
use crate::canvas::geom::to_grid as snap_point;
use crate::components::ComponentChange;

fn ready() -> Canvas {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(80.0, 0.0);
    c
}

fn item_of(c: &Canvas, scene: Point) -> Point {
    c.viewport.map_from_circuit(scene)
}

#[test]
fn click_selects_and_empty_clears() {
    let mut c = ready();
    let p = item_of(&c, Point::zero());
    c.mouse_press(BUTTON_LEFT, p.x, p.y, 0);
    c.mouse_release(BUTTON_LEFT, p.x, p.y, 0);
    assert!(c.scene.items()[0].selected);
    assert!(!c.scene.items()[1].selected);

    let empty = item_of(&c, Point::new(200.0, 200.0));
    c.mouse_press(BUTTON_LEFT, empty.x, empty.y, 0);
    c.mouse_release(BUTTON_LEFT, empty.x, empty.y, 0);
    assert!(!c.scene.any_selected());
    assert!(!c.banding());
}

#[test]
fn wheel_scroll_updates_last_scene_and_hover() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    let center_item = Point::new(400.0, 300.0);
    c.mouse_move(center_item.x, center_item.y, 0, 0);
    assert_eq!(c.last_scene().x.round(), 0.0);
    assert_eq!(c.last_scene().y.round(), 0.0);
    assert!(c.hover_tooltip(c.last_scene()).is_some());

    // Scroll down by 100 pixels in view space
    c.wheel(0.0, -100.0, 0.0, -100.0, center_item.x, center_item.y, 0);
    // The scene coordinates under the cursor should update
    assert_ne!(c.last_scene(), Point::new(0.0, 0.0));
    assert_eq!(c.last_scene(), c.viewport.map_to_circuit(center_item));
    // The resistor is no longer under the cursor
    assert!(c.hover_tooltip(c.last_scene()).is_none());

    // Scroll back
    c.wheel(0.0, 100.0, 0.0, 100.0, center_item.x, center_item.y, 0);
    assert_eq!(c.last_scene().x.round(), 0.0);
    assert_eq!(c.last_scene().y.round(), 0.0);
    assert!(c.hover_tooltip(c.last_scene()).is_some());
}

#[test]
fn shift_click_toggles() {
    let mut c = ready();
    let a = item_of(&c, Point::zero());
    let b = item_of(&c, Point::new(80.0, 0.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_release(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_press(BUTTON_LEFT, b.x, b.y, MOD_SHIFT);
    c.mouse_release(BUTTON_LEFT, b.x, b.y, MOD_SHIFT);
    assert!(c.scene.items()[0].selected && c.scene.items()[1].selected);
}

#[test]
fn rubber_band_selects() {
    let mut c = ready();
    let a = item_of(&c, Point::new(-30.0, -30.0));
    let b = item_of(&c, Point::new(30.0, 30.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_move(b.x, b.y, BUTTON_LEFT, 0);
    assert!(c.banding());
    assert!(c.scene.items()[0].selected);
    assert!(!c.scene.items()[1].selected);
    c.mouse_release(BUTTON_LEFT, b.x, b.y, 0);
    assert!(!c.banding());
    assert!(c.scene.items()[0].selected);
}

#[test]
fn drag_snaps_to_grid4() {
    let mut c = ready();
    let start = item_of(&c, Point::zero());
    c.mouse_press(BUTTON_LEFT, start.x, start.y, 0);
    let mid = item_of(&c, Point::new(6.0, 0.0));
    c.mouse_move(mid.x, mid.y, BUTTON_LEFT, 0);
    // toGrid(0)=0, toGrid(6)=8, delta 8.
    assert_eq!(c.scene.items()[0].x, 8.0);
    c.mouse_release(BUTTON_LEFT, mid.x, mid.y, 0);
}

#[test]
fn moving_a_component_dirties_old_and_new_rects_not_full() {
    let mut c = ready();
    let start = item_of(&c, Point::zero());
    c.mouse_press(BUTTON_LEFT, start.x, start.y, 0);
    let _ = c.take_dirty();
    let old = c.scene.items()[0].total_bounding_rect();
    let mid = item_of(&c, Point::new(16.0, 0.0));
    let ch = c.mouse_move(mid.x, mid.y, BUTTON_LEFT, 0);
    assert!(
        !ch.items,
        "dragging must dirty-region raster without rebuilding the QML items model"
    );
    let dirty = c.peek_dirty().clone();
    assert!(
        !dirty.full,
        "dragging a part must not mark the whole canvas dirty"
    );
    assert!(
        dirty.items.contains(&c.scene.items()[0].id),
        "moved item should be in the dirty set, got {:?}",
        dirty.items
    );
    let rects = dirty.scene_rects(&c);
    assert!(
        !rects.is_empty(),
        "move must produce dirty scene rects for a region raster"
    );
    let new = c.scene.items()[0].total_bounding_rect();
    assert!(
        rects.iter().any(|r| r.intersects(&old)),
        "dirty rects must cover the old position so the ghost is erased"
    );
    assert!(
        rects.iter().any(|r| r.intersects(&new)),
        "dirty rects must cover the new position"
    );
}

#[test]
fn pan_middle_button() {
    let mut c = ready();
    let p0 = item_of(&c, Point::zero());
    c.mouse_press(BUTTON_MIDDLE, p0.x, p0.y, 0);
    assert_eq!(c.cursor(), CursorKind::ClosedHand);
    c.mouse_move(p0.x + 40.0, p0.y, BUTTON_MIDDLE, 0);
    assert!((c.viewport.center().x + 40.0).abs() < 1e-9);
    c.mouse_release(BUTTON_MIDDLE, p0.x + 40.0, p0.y, 0);
    assert_eq!(c.cursor(), CursorKind::Arrow);
}

#[test]
fn ctrl_wheel_zooms_at_cursor() {
    let mut c = ready();
    let anchor = item_of(&c, Point::new(40.0, 10.0));
    let scene_before = c.viewport.map_to_circuit(anchor);
    c.wheel(0.0, 0.0, 0.0, 700.0, anchor.x, anchor.y, MOD_CTRL);
    assert!((c.viewport.zoom() - 2.0).abs() < 1e-9);
    let scene_after = c.viewport.map_to_circuit(anchor);
    assert!((scene_after.x - scene_before.x).abs() < 1e-9);
    assert!((scene_after.y - scene_before.y).abs() < 1e-9);
}

#[test]
fn wheel_on_volt_source_dirties_item_without_items_model() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let id = c.scene.add_volt_source(0.0, 0.0);
    let knob = item_of(&c, Point::new(0.0, -8.0));
    let _ = c.take_dirty();
    let before = c.scene.item_by_id(&id).unwrap().source_value();
    let ch = c.wheel(0.0, 0.0, 0.0, -120.0, knob.x, knob.y, 0);
    assert!(
        !ch.items,
        "knob wheel must not set Change.items (QML JSON rebuild)"
    );
    assert!(!ch.history);
    let dirty = c.take_dirty();
    assert!(
        dirty.items.contains(&id),
        "knob wheel must dirty the source for a region raster, got {:?}",
        dirty.items
    );
    assert!(!dirty.full);
    assert!(c.take_last_wheel_item().is_some());
    let after = c.scene.item_by_id(&id).unwrap().source_value();
    assert_ne!(after, before);
}

#[test]
fn set_dial_val_does_not_rebuild_qml_or_push_undo() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let id = c.scene.add_volt_source(0.0, 0.0);
    c.mark_saved();
    let _ = c.take_dirty();
    let ch = c.set_dial_val(&id, 2.5);
    assert!(!ch.items);
    assert!(!c.can_undo(), "knob ticks must not snapshot undo");
    assert!(c.is_modified());
    assert!(c.take_dirty().items.contains(&id));
}

#[test]
fn sim_tick_dirties_without_qml_items_rebuild() {
    let mut c = Canvas::with_demo_divider();
    c.scene.settings_mut().animate_logic = true;
    c.power_on();
    let _ = c.take_dirty();
    c.set_pin_voltage("Resistor-1-lPin", 0.0);
    let ch = c.tick();
    assert!(!ch.items, "sim ticks must not rebuild CircuitCanvas.items");
    assert!(c.peek_dirty().items.contains("Resistor-1"));
}

#[test]
fn wheel_pans_pixel_and_angle_delta() {
    let mut c = ready();
    let initial_center = c.viewport.center();

    // Pixel delta panning (e.g. trackpad)
    c.wheel(50.0, -30.0, 0.0, 0.0, 0.0, 0.0, 0);
    assert!((c.viewport.center().x - (initial_center.x - 50.0)).abs() < 1e-9);
    assert!((c.viewport.center().y - (initial_center.y + 30.0)).abs() < 1e-9);

    // Reset center
    c.set_center(0.0, 0.0);

    // Angle delta panning (e.g. mouse wheel)
    // 120 angle delta = 24 px
    c.wheel(0.0, 0.0, 120.0, -240.0, 0.0, 0.0, 0);
    assert!((c.viewport.center().x - (-24.0)).abs() < 1e-9);
    assert!((c.viewport.center().y - 48.0).abs() < 1e-9);
}

#[test]
fn delete_and_escape() {
    let mut c = ready();
    let p = item_of(&c, Point::zero());
    c.mouse_press(BUTTON_LEFT, p.x, p.y, 0);
    c.mouse_release(BUTTON_LEFT, p.x, p.y, 0);
    c.key_press(KEY_ESCAPE, 0);
    assert!(!c.scene.any_selected());
    c.mouse_press(BUTTON_LEFT, p.x, p.y, 0);
    c.mouse_release(BUTTON_LEFT, p.x, p.y, 0);
    assert!(c.has_item_selection());
    assert!(!c.has_wire_selection());
    c.key_press(KEY_DELETE, 0);
    assert_eq!(c.scene.items().len(), 1);
    assert_eq!(c.scene.items()[0].id, "Resistor-2");
}

#[test]
fn delete_selected_method_and_rotate_180() {
    let mut c = ready();
    let p = item_of(&c, Point::zero());
    c.mouse_press(BUTTON_LEFT, p.x, p.y, 0);
    c.mouse_release(BUTTON_LEFT, p.x, p.y, 0);
    assert!(c.has_selection());
    let change_rot = c.rotate_selected(180.0);
    assert!(change_rot.items);
    assert_eq!(c.scene.items()[0].rotation, 180.0);

    let change_del = c.delete_selected();
    assert!(change_del.items);
    assert_eq!(c.scene.items().len(), 1);
    assert!(!c.has_selection());
}

#[test]
fn test_wire_current_from_start_or_end_pin() {
    let sim1 = r#"<circuit version="1.0.0">
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Pos="0,0" Voltage="5 V" />
<item itemtype="Resistor" CircId="Resistor-1" Pos="80,0" Resistance="1000 Ω" />
<item itemtype="Ground" CircId="Ground-1" Pos="160,0" />
<item itemtype="Connector" CircId="Wire-1" startpinid="Fixed Voltage-1-outnod" endpinid="Resistor-1-lPin" pointList="0,0,80,0" />
<item itemtype="Connector" CircId="Wire-2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" pointList="80,0,160,0" />
</circuit>"#;
    let scene = Scene::from_sim1(sim1).expect("valid sim1");
    let mut c = Canvas::empty();
    c.scene = scene;
    c.power_on();
    c.tick();
    let w1_curr = c.wire_current("Wire-1");
    let w2_curr = c.wire_current("Wire-2");
    assert!(
        w1_curr > 0.004 && w1_curr < 0.006,
        "w1 current was {w1_curr}"
    );
    assert!(
        w2_curr > 0.004 && w2_curr < 0.006,
        "w2 current was {w2_curr}"
    );
}

#[test]
fn test_wire_current_various_sources() {
    let sim1 = r#"<circuit version="1.0.0">
<item itemtype="Rail" CircId="Rail-1" Pos="0,0" Voltage="12 V" />
<item itemtype="Resistor" CircId="Resistor-1" Pos="80,0" Resistance="2400 Ω" />
<item itemtype="Ground" CircId="Ground-1" Pos="160,0" />
<item itemtype="Connector" CircId="Wire-1" startpinid="Rail-1-outnod" endpinid="Resistor-1-lPin" pointList="0,0,80,0" />
<item itemtype="Connector" CircId="Wire-2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" pointList="80,0,160,0" />
</circuit>"#;
    let scene = Scene::from_sim1(sim1).expect("valid sim1");
    let mut c = Canvas::empty();
    c.scene = scene;
    c.power_on();
    c.tick();
    let w1_curr = c.wire_current("Wire-1");
    let w2_curr = c.wire_current("Wire-2");
    assert!((w1_curr - 0.005).abs() < 1e-4, "w1 current was {w1_curr}");
    assert!((w2_curr - 0.005).abs() < 1e-4, "w2 current was {w2_curr}");
}

#[test]
fn zoom_to_fit_centers_items() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.zoom_to_fit();
    let r = c
        .scene
        .items_bounding_rect()
        .adjust(-20.0, -20.0, 20.0, 20.0);
    assert!((c.viewport.center().x - r.center().x).abs() < 1e-6);
    assert!((c.viewport.center().y - r.center().y).abs() < 1e-6);
    assert!(c.viewport.zoom() > 1.0);
}

#[test]
fn power_on_solves_demo_divider() {
    let mut c = Canvas::with_demo_divider();
    c.power_on();
    assert!(c.sim_running());
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    let mid = c.pin_voltage("Resistor-1-rPin").unwrap();
    assert!((mid - 2.5).abs() < 1e-6, "mid {mid}");
    let vout = c.pin_voltage("Fixed Voltage-1-outnod").unwrap();
    assert!((vout - 5.0).abs() < 1e-6, "vout {vout}");
    c.power_off();
    assert!(!c.sim_running());
    assert!(c.pin_voltage("Resistor-1-rPin").is_none());
}

#[test]
fn place_and_roundtrip_instruments() {
    let mut c = Canvas::empty();
    c.add_component_at("Probe", Point::zero());
    c.add_component_at("Voltmeter", Point::new(40.0, 0.0));
    c.add_component_at("Ammeter", Point::new(80.0, 0.0));
    c.add_component_at("Oscope", Point::new(160.0, 0.0));
    c.add_component_at("LogicAnalyzer", Point::new(400.0, 0.0));
    assert_eq!(c.scene.items().len(), 5);
    let src = c.scene.to_sim1();
    assert!(src.contains("itemtype=\"Probe\""));
    assert!(src.contains("itemtype=\"Voltmeter\""));
    assert!(src.contains("itemtype=\"Ammeter\""));
    assert!(src.contains("itemtype=\"Oscope\""));
    assert!(src.contains("itemtype=\"LogicAnalyzer\""));
    let loaded = crate::canvas::Scene::from_sim1(&src).unwrap();
    assert_eq!(loaded.items().len(), 5);
}

#[test]
fn click_pins_draws_a_wire() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(80.0, 0.0);
    let a = item_of(&c, Point::new(-16.0, 0.0));
    let b = item_of(&c, Point::new(64.0, 0.0)); // Resistor-2 lPin
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    assert!(c.scene.drawing());
    let _ = c.take_dirty();
    let ch = c.mouse_move(b.x, b.y, BUTTON_LEFT, 0);
    assert!(ch.wires, "routing a draft wire must request a wire repaint");
    let dirty = c.peek_dirty().clone();
    assert!(
        !dirty.full,
        "drawing a wire must not mark the whole canvas dirty"
    );
    assert!(
        !dirty.wires.is_empty() || !dirty.rects.is_empty(),
        "draft wire must be in the dirty set so apply() region-rasters it, got items={:?} wires={:?} rects={}",
        dirty.items,
        dirty.wires,
        dirty.rects.len()
    );
    assert!(
        !dirty.scene_rects(&c).is_empty(),
        "draft wire must produce dirty scene rects"
    );
    c.mouse_press(BUTTON_LEFT, b.x, b.y, 0);
    assert!(!c.scene.drawing());
    assert_eq!(c.scene.wires().len(), 1);
    assert_eq!(c.scene.wires()[0].start_pin, "Resistor-1-lPin");
    assert_eq!(
        c.scene.wires()[0].end_pin.as_deref(),
        Some("Resistor-2-lPin")
    );
}

#[test]
fn escape_cancels_wire() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    let a = item_of(&c, Point::new(-16.0, 0.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.key_press(KEY_ESCAPE, 0);
    assert!(!c.scene.drawing());
    assert!(c.scene.wires().is_empty());
}

#[test]
fn load_sim1_divider_onto_canvas() {
    let src = include_str!("../../tests/fixtures/divider.sim1");
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_sim1(src, Some("/tmp/divider.sim1".into())).unwrap();
    assert_eq!(c.scene.items().len(), 5);
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    assert!((c.pin_voltage("Node-5-0").unwrap() - 2.5).abs() < 1e-6);
    assert!(c.wire_current(&c.scene.wires()[0].id).abs() > 1e-4);
    let saved = c.to_sim1();
    assert!(saved.contains("itemtype=\"Resistor\""));
    assert!(saved.contains("pointList="));
}

#[test]
fn load_recovered_keeps_modified_against_disk() {
    let disk = include_str!("../../tests/fixtures/divider.sim1");
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_recovered(disk, Some("/tmp/divider.sim1".into()), "")
        .unwrap();
    assert!(c.is_modified());
    assert!(!c.can_undo());
    let hist = crate::backup::CircuitHistory {
        undo: vec![crate::backup::HistorySnap {
            sim1: String::new(),
            selected_items: vec![],
            selected_wires: vec![],
        }],
        redo: vec![],
    };
    c.import_history(hist);
    assert!(c.can_undo());
    assert_eq!(c.export_history().undo.len(), 1);
}

#[test]
fn load_subcircuit_divider_onto_canvas() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packages");
    let path = dir.join("parent.sim1");
    let src = std::fs::read_to_string(&path).unwrap();
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_sim1(&src, Some(path.to_string_lossy().into_owned()))
        .unwrap();
    assert_eq!(c.scene.items().len(), 3);
    let sub = c
        .scene
        .items()
        .iter()
        .find(|i| i.kind.type_name() == "Subcircuit")
        .expect("subcircuit item");
    let pins: Vec<_> = sub.pins().iter().map(|p| p.id.clone()).collect();
    assert!(pins.contains(&"vdiv-1-in".to_string()), "{pins:?}");
    assert!(pins.contains(&"vdiv-1-out".to_string()), "{pins:?}");
    assert!(pins.contains(&"vdiv-1-gnd".to_string()), "{pins:?}");
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    assert!((c.pin_voltage("vdiv-1-out").unwrap() - 2.5).abs() < 1e-6);
    let saved = c.to_sim1();
    assert!(saved.contains("itemtype=\"Subcircuit\""));
    assert!(saved.contains("CircId=\"vdiv-1\""));
    assert!(!saved.contains("itemtype=\"Tunnel\""));
}

#[test]
fn add_battery_ground_fixed() {
    let mut c = Canvas::empty();
    c.add_component_at("Battery", Point::zero());
    c.add_component_at("Ground", Point::new(40.0, 0.0));
    c.add_component_at("FixedVolt", Point::new(80.0, 0.0));
    assert_eq!(c.scene.items().len(), 3);
    assert_eq!(c.scene.items()[0].kind.type_name(), "Battery");
    assert_eq!(c.scene.items()[1].kind.type_name(), "Ground");
    assert_eq!(c.scene.items()[2].kind.type_name(), "FixedVolt");
}

#[test]
fn add_bjt_and_mosfet() {
    let mut c = Canvas::empty();
    c.add_component_at("Bjt", Point::zero());
    c.add_component_at("Mosfet", Point::new(40.0, 0.0));
    assert_eq!(c.scene.items().len(), 2);
    assert_eq!(c.scene.items()[0].kind.type_name(), "Bjt");
    assert_eq!(c.scene.items()[1].kind.type_name(), "Mosfet");
    assert_eq!(c.scene.items()[0].pins()[0].id, "BJT-1-collector");
    assert_eq!(c.scene.items()[1].pins()[0].id, "Mosfet-1-Dren");
    let saved = c.scene.to_sim1();
    assert!(saved.contains("itemtype=\"Bjt\""));
    assert!(saved.contains("itemtype=\"Mosfet\""));
    let s2 = Scene::from_sim1(&saved).unwrap();
    assert_eq!(s2.items().len(), 2);
}

#[test]
fn add_opamp() {
    let mut c = Canvas::empty();
    c.add_component_at("OpAmp", Point::zero());
    assert_eq!(c.scene.items().len(), 1);
    assert_eq!(c.scene.items()[0].kind.type_name(), "OpAmp");
    assert_eq!(c.scene.items()[0].pins()[0].id, "opAmp-1-inputNinv");
    assert_eq!(c.scene.items()[0].pins()[2].id, "opAmp-1-output");
    assert_eq!(c.scene.items()[0].pins().len(), 3);
    let saved = c.scene.to_sim1();
    assert!(saved.contains("itemtype=\"OpAmp\""));
    let s2 = Scene::from_sim1(&saved).unwrap();
    assert_eq!(s2.items().len(), 1);
    assert_eq!(s2.items()[0].kind.type_name(), "OpAmp");
}

#[test]
fn add_jfet() {
    let mut c = Canvas::empty();
    c.add_component_at("Jfet", Point::zero());
    assert_eq!(c.scene.items().len(), 1);
    assert_eq!(c.scene.items()[0].kind.type_name(), "Jfet");
    assert_eq!(c.scene.items()[0].pins()[0].id, "Jfet-1-Dren");
    assert_eq!(c.scene.items()[0].pins()[1].id, "Jfet-1-Sour");
    assert_eq!(c.scene.items()[0].pins()[2].id, "Jfet-1-Gate");
    let saved = c.scene.to_sim1();
    assert!(saved.contains("itemtype=\"Jfet\""));
    assert!(saved.contains("LambdaInv="));
    let s2 = Scene::from_sim1(&saved).unwrap();
    assert_eq!(s2.items().len(), 1);
    assert_eq!(s2.items()[0].kind.type_name(), "Jfet");
}

#[test]
fn add_comparator() {
    let mut c = Canvas::empty();
    c.add_component_at("Comparator", Point::zero());
    assert_eq!(c.scene.items().len(), 1);
    assert_eq!(c.scene.items()[0].kind.type_name(), "Comparator");
    assert_eq!(c.scene.items()[0].pins()[0].id, "Comparator-1-in0");
    assert_eq!(c.scene.items()[0].pins()[1].id, "Comparator-1-in1");
    assert_eq!(c.scene.items()[0].pins()[2].id, "Comparator-1-out");
    let saved = c.scene.to_sim1();
    assert!(saved.contains("itemtype=\"Comparator\""));
    let s2 = Scene::from_sim1(&saved).unwrap();
    assert_eq!(s2.items().len(), 1);
    assert_eq!(s2.items()[0].kind.type_name(), "Comparator");
}

#[test]
fn add_volt_reg() {
    let mut c = Canvas::empty();
    c.add_component_at("VoltReg", Point::zero());
    assert_eq!(c.scene.items().len(), 1);
    assert_eq!(c.scene.items()[0].kind.type_name(), "VoltReg");
    assert_eq!(c.scene.items()[0].pins()[0].id, "VoltReg-1-input");
    assert_eq!(c.scene.items()[0].pins()[1].id, "VoltReg-1-output");
    assert_eq!(c.scene.items()[0].pins()[2].id, "VoltReg-1-ref");
    let saved = c.scene.to_sim1();
    assert!(saved.contains("itemtype=\"VoltReg\""));
    assert!(saved.contains("Voltage="));
    let s2 = Scene::from_sim1(&saved).unwrap();
    assert_eq!(s2.items().len(), 1);
    assert_eq!(s2.items()[0].kind.type_name(), "VoltReg");
}

#[test]
fn rotate_moves_pin_and_undoes() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.add_component_at("Resistor", Point::zero());
    let left = c.scene.pin_scene("Resistor-1-lPin").unwrap();
    assert!((left.x + 16.0).abs() < 1e-9);
    c.scene.select_only(0);
    c.rotate_cw();
    let left = c.scene.pin_scene("Resistor-1-lPin").unwrap();
    assert!((left.x).abs() < 1e-6);
    assert!((left.y + 16.0).abs() < 1e-6);
    assert!(c.can_undo());
    c.undo();
    let left = c.scene.pin_scene("Resistor-1-lPin").unwrap();
    assert!((left.x + 16.0).abs() < 1e-6);
    assert!(left.y.abs() < 1e-6);
    c.redo();
    assert!((c.scene.items()[0].rotation - 90.0).abs() < 1e-9);
    let saved = c.scene.to_sim1();
    assert!(saved.contains("rotation=\"90\""));
    let s2 = Scene::from_sim1(&saved).unwrap();
    assert!((s2.items()[0].rotation - 90.0).abs() < 1e-9);
}

#[test]
fn copy_paste_new_ids() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.add_component_at("Resistor", Point::zero());
    c.scene.select_only(0);
    assert!(c.copy_selection());
    c.paste_at(Point::new(40.0, 0.0));
    assert_eq!(c.scene.items().len(), 2);
    assert_eq!(c.scene.items()[1].id, "Resistor-2");
    assert!((c.scene.items()[1].x - 40.0).abs() < 1e-9);
    c.undo();
    assert_eq!(c.scene.items().len(), 1);
}

#[test]
fn property_edit_resistance() {
    let mut c = Canvas::empty();
    c.add_component_at("Resistor", Point::zero());
    c.open_properties("Resistor-1");
    c.set_prop_text("Resistance".into(), "10 kΩ".into());
    match &c.scene.items()[0].kind {
        Part::Resistor(r) => {
            assert!((r.resistance - 10_000.0).abs() < 1e-6);
        }
        _ => panic!("expected resistor"),
    }
    c.undo();
    match &c.scene.items()[0].kind {
        Part::Resistor(r) => {
            assert!((r.resistance - 100.0).abs() < 1e-6);
        }
        _ => panic!("expected resistor"),
    }
}

#[test]
fn flip_h_and_ctrl_r() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.add_component_at("Resistor", Point::zero());
    c.scene.select_only(0);
    c.key_press(KEY_L, MOD_CTRL);
    assert_eq!(c.scene.items()[0].hflip, -1);
    let left = c.scene.pin_scene("Resistor-1-lPin").unwrap();
    assert!((left.x - 16.0).abs() < 1e-6);
    c.key_press(KEY_R, MOD_CTRL);
    // C++: rotation += 90 * hflip * vflip = 90 * -1 * 1 = -90
    assert!((c.scene.items()[0].rotation + 90.0).abs() < 1e-9);
}

fn two_resistors_wired() -> Canvas {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(80.0, 0.0);
    let a = item_of(&c, Point::new(16.0, 0.0));
    let b = item_of(&c, Point::new(64.0, 0.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_press(BUTTON_LEFT, b.x, b.y, 0);
    assert_eq!(c.scene.wires().len(), 1);
    c
}

#[test]
fn hover_wire_sets_split_cursor() {
    let mut c = two_resistors_wired();
    let mid = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_move(mid.x, mid.y, 0, 0);
    assert_eq!(c.cursor(), CursorKind::SplitV);
}

#[test]
fn drag_wire_segment_makes_u() {
    let mut c = two_resistors_wired();
    let mid = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, mid.x, mid.y, 0);
    assert_eq!(c.cursor(), CursorKind::SplitV);
    let down = item_of(&c, Point::new(40.0, 16.0));
    c.mouse_move(down.x, down.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, down.x, down.y, 0);
    let pts = &c.scene.wires()[0].points;
    assert_eq!(pts[0], Point::new(16.0, 0.0));
    assert_eq!(*pts.last().unwrap(), Point::new(64.0, 0.0));
    assert!(pts.iter().any(|p| p.y == 16.0), "{pts:?}");
    assert!(pts.len() >= 4, "{pts:?}");
}

#[test]
fn middle_on_wire_does_not_pan() {
    let mut c = two_resistors_wired();
    let mid = item_of(&c, Point::new(40.0, 0.0));
    let down = item_of(&c, Point::new(40.0, 16.0));
    let center = c.viewport.center();
    c.mouse_press(BUTTON_MIDDLE, mid.x, mid.y, 0);
    c.mouse_move(down.x, down.y, BUTTON_MIDDLE, 0);
    assert!((c.viewport.center().x - center.x).abs() < 1e-9);
    assert!((c.viewport.center().y - center.y).abs() < 1e-9);
    c.mouse_release(BUTTON_MIDDLE, down.x, down.y, 0);
    assert!(c.scene.wires()[0].points.iter().any(|p| p.y != 0.0));
}

#[test]
fn drag_wire_corner_keeps_square() {
    let mut c = two_resistors_wired();
    let mid = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, mid.x, mid.y, 0);
    let down = item_of(&c, Point::new(40.0, 16.0));
    c.mouse_move(down.x, down.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, down.x, down.y, 0);
    let corner = item_of(&c, Point::new(16.0, 16.0));
    c.mouse_move(corner.x, corner.y, 0, 0);
    assert_eq!(c.cursor(), CursorKind::SizeAll);
    c.mouse_press(BUTTON_LEFT, corner.x, corner.y, 0);
    let moved = item_of(&c, Point::new(24.0, 24.0));
    c.mouse_move(moved.x, moved.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, moved.x, moved.y, 0);
    let pts = &c.scene.wires()[0].points;
    for w in pts.windows(2) {
        assert!(
            w[0].x == w[1].x || w[0].y == w[1].y,
            "diagonal leftover {pts:?}"
        );
    }
}

#[test]
fn alt_click_starts_wire_from_splice() {
    let mut c = two_resistors_wired();
    let mid = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, mid.x, mid.y, MOD_ALT);
    assert!(c.scene.drawing());
    assert!(
        c.scene
            .items()
            .iter()
            .any(|it| it.kind.type_name() == "Node")
    );
    assert_eq!(c.scene.wires().len(), 3); // two halves + draft
}

#[test]
fn wire_drag_is_undoable() {
    let mut c = two_resistors_wired();
    let before = c.scene.wires()[0].points.clone();
    let mid = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, mid.x, mid.y, 0);
    let down = item_of(&c, Point::new(40.0, 16.0));
    c.mouse_move(down.x, down.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, down.x, down.y, 0);
    assert_ne!(c.scene.wires()[0].points, before);
    assert!(c.can_undo());
    c.undo();
    assert_eq!(c.scene.wires()[0].points, before);
}

#[test]
fn circuit_header_roundtrip() {
    let mut c = Canvas::empty();
    c.update_circ_settings(|s| {
        s.width = 1600;
        s.height = 900;
        s.animate_logic = true;
        s.animate_curr = false;
        s.ansi = true;
        s.nl_steps = 42;
        s.react_step_ps = 2000;
    });
    let src = c.to_sim1();
    assert!(src.contains("width=\"1600\""));
    assert!(src.contains("height=\"900\""));
    assert!(src.contains("animate=\"1\""));
    assert!(src.contains("anicurr=\"0\""));
    assert!(src.contains("ansi=\"1\""));
    assert!(src.contains("NLsteps=\"42\""));
    assert!(src.contains("reaStep=\"2000\""));
    let mut loaded = Canvas::empty();
    loaded.load_sim1(&src, None).unwrap();
    let s = loaded.circ_settings();
    assert_eq!(s.width, 1600);
    assert_eq!(s.height, 900);
    assert!(s.animate_logic);
    assert!(!s.animate_curr);
    assert!(s.ansi);
    assert_eq!(s.nl_steps, 42);
    assert_eq!(s.react_step_ps, 2000);
}

const PIC14_MCU: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTA" addr="0x05" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
  </regblock>
  <regblock start="0x80" end="0x8F">
    <mapped addr="0x80" mapto="0x00"/>
    <mapped addr="0x82" mapto="0x02"/>
    <mapped addr="0x83" mapto="0x03"/>
    <mapped addr="0x84" mapto="0x04"/>
    <register name="OPTION" addr="0x81" reset="11111111"/>
    <register name="TRISA" addr="0x85" reset="11111111"/>
    <mapped addr="0x8A" mapto="0x0A"/>
  </regblock>
  <datablock start="0x0C" end="0x4F"/>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
</mcu>
"#;

#[test]
fn load_mcu_onto_canvas_feeds_monitor() {
    let dir = std::env::temp_dir().join(format!("cs-canvas-mcu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pic14test")).unwrap();
    std::fs::write(dir.join("pic14test").join("pic14test.mcu"), PIC14_MCU).unwrap();
    std::fs::write(
        dir.join("porta.hex"),
        ":0C0000008316850183120130850005285D\n:00000001FF\n",
    )
    .unwrap();
    let sim1 = r#"<circuit version="1.0.0" >
<item itemtype="Mcu" CircId="pic14test-1" Frequency="4 MHz" Program="porta.hex" AutoLoad="true" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="10 kΩ" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" CircId="c1" startpinid="pic14test-1-PORTA0" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" CircId="c2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>
"#;
    let path = dir.join("porta.sim1");
    std::fs::write(&path, sim1).unwrap();
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_sim1(sim1, Some(path.to_string_lossy().into_owned()))
        .unwrap();
    assert!(c.scene.hidden().is_empty(), "MCU is a canvas chip");
    let mcu = c
        .scene
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "Mcu")
        .expect("MCU on canvas");
    assert!(
        mcu.pins().iter().any(|p| p.id == "pic14test-1-PORTA0"),
        "GPIO pins {:?}",
        mcu.pins().iter().map(|p| p.id.clone()).collect::<Vec<_>>()
    );
    assert!(mcu.pkg_w() > 0 && mcu.pkg_h() > 0);
    assert!(
        c.scene
            .items()
            .iter()
            .any(|it| it.kind.type_name() == "Resistor")
    );
    let saved = c.to_sim1();
    assert!(saved.contains("itemtype=\"Mcu\""), "{saved}");
    assert!(saved.contains("CircId=\"pic14test-1\""), "{saved}");
    assert!(saved.contains("Pos="), "{saved}");
    c.sync_mcu();
    let snap = crate::mcu::monitor_snap();
    assert_eq!(snap.id, "pic14test-1");
    assert!(snap.has_status);
    assert_eq!(snap.ram.len(), 256);
    c.power_on();
    for _ in 0..20 {
        c.tick();
    }
    c.sync_mcu();
    let snap = crate::mcu::monitor_snap();
    assert_eq!(snap.ram[0x05], 1, "PORTA after run, pc={}", snap.pc);
    let _ = c.poke_mcu_ram(0x20, 0xAB);
    c.sync_mcu();
    assert_eq!(crate::mcu::monitor_snap().ram[0x20], 0xAB);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_mcu_package_places_gpio_pins() {
    let dir = std::env::temp_dir().join(format!("cs-canvas-mcu-pkg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pic14test")).unwrap();
    std::fs::write(dir.join("pic14test").join("pic14test.mcu"), PIC14_MCU).unwrap();
    std::fs::write(
        dir.join("pic14test").join("pic14test.package"),
        r#"<packageB name="DIP" width="4" height="6">
    <pin type="" xpos="-8" ypos="24" angle="180" length="8" id="RA0" label="RA0" />
    <pin type="" xpos="40" ypos="8" angle="0" length="8" id="RA1" label="RA1" />
</packageB>
"#,
    )
    .unwrap();
    let sim1 = r#"<circuit version="1.0.0" >
<item itemtype="Mcu" CircId="pic14test-1" Pos="16,8" />
</circuit>
"#;
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_sim1(
        sim1,
        Some(dir.join("x.sim1").to_string_lossy().into_owned()),
    )
    .unwrap();
    let mcu = c
        .scene
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "Mcu")
        .expect("MCU");
    let p0 = mcu
        .pins()
        .into_iter()
        .find(|p| p.id == "pic14test-1-PORTA0")
        .expect("PORTA0");
    assert_eq!(p0.local, Point::new(-8.0, 24.0));
    assert_eq!(p0.label, "RA0");
    let p1 = mcu
        .pins()
        .into_iter()
        .find(|p| p.id == "pic14test-1-PORTA1")
        .expect("PORTA1");
    assert_eq!(p1.local, Point::new(40.0, 8.0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn place_mcu_from_catalog_xml() {
    let dir = std::env::temp_dir().join(format!("cs-canvas-mcu-place-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("AVR")).unwrap();
    std::fs::write(
        dir.join("avr.xml"),
        r#"<itemlib>
            <itemset category="AVR/attiny" type="MCU">
                <item name="tiny13" package="AVR/tiny13" data="AVR/tiny13" />
            </itemset>
            </itemlib>"#,
    )
    .unwrap();
    std::fs::write(dir.join("AVR").join("tiny13.mcu"), PIC14_MCU).unwrap();
    let mut catalog = crate::catalog::Catalog::new();
    catalog.load_dir(&dir);
    let mut c = Canvas::empty();
    c.set_catalog(catalog);
    c.set_view_size(800.0, 600.0);
    let ch = c.add_component_at("tiny13,MCU", Point::new(16.0, 8.0));
    assert!(ch.items);
    let mcu = c
        .scene
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "Mcu")
        .expect("placed MCU");
    assert_eq!(mcu.id, "tiny13-1");
    assert!(
        mcu.pins().iter().any(|p| p.id == "tiny13-1-PORTA0"),
        "GPIO pins {:?}",
        mcu.pins().iter().map(|p| p.id.clone()).collect::<Vec<_>>()
    );
    let saved = c.to_sim1();
    assert!(saved.contains("itemtype=\"Mcu\""), "{saved}");
    assert!(saved.contains("CircId=\"tiny13-1\""), "{saved}");
    let ch = c.add_component_at("Mcu", Point::zero());
    assert!(!ch.items);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_sim1_mcu_via_catalog_not_circuit_dir() {
    let dir = std::env::temp_dir().join(format!("cs-canvas-mcu-catload-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let lib = dir.join("lib");
    let circ = dir.join("circ");
    std::fs::create_dir_all(lib.join("AVR")).unwrap();
    std::fs::create_dir_all(&circ).unwrap();
    std::fs::write(
        lib.join("avr.xml"),
        r#"<itemlib>
            <itemset category="AVR" type="MCU">
                <item name="tiny13" data="AVR/tiny13" />
            </itemset>
            </itemlib>"#,
    )
    .unwrap();
    std::fs::write(lib.join("AVR").join("tiny13.mcu"), PIC14_MCU).unwrap();
    let mut catalog = crate::catalog::Catalog::new();
    catalog.load_dir(&lib);
    let sim1 = r#"<circuit version="1.0.0" >
<item itemtype="Mcu" CircId="tiny13-1" Pos="16,8" />
</circuit>
"#;
    let mut c = Canvas::empty();
    c.set_catalog(catalog);
    c.set_view_size(800.0, 600.0);
    c.load_sim1(
        sim1,
        Some(circ.join("x.sim1").to_string_lossy().into_owned()),
    )
    .unwrap();
    let mcu = c
        .scene
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "Mcu")
        .expect("MCU from catalog");
    assert_eq!(mcu.id, "tiny13-1");
    assert!(mcu.pins().iter().any(|p| p.id == "tiny13-1-PORTA0"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_add_all_components_layout_and_spacing() {
    let mut c = Canvas::empty();
    c.set_view_size(1000.0, 800.0);

    let ch = c.add_all_components();
    assert!(ch.items);

    let items = c.scene.items();
    assert!(
        items.len() >= 70,
        "Expected at least 70 items from central catalog, got {}",
        items.len()
    );

    // Verify each item has positive dimensions and valid coordinates
    for it in items {
        let r = it.full_rect();
        assert!(r.w > 0.0, "Item {} has non-positive width: {}", it.id, r.w);
        assert!(r.h > 0.0, "Item {} has non-positive height: {}", it.id, r.h);
    }

    // Verify that no two placed components overlap
    for i in 0..items.len() {
        let r_i = items[i].full_rect();
        for j in (i + 1)..items.len() {
            let r_j = items[j].full_rect();
            // Test intersection with small epsilon margin
            let overlaps = r_i.x < r_j.x + r_j.w - 1.0
                && r_i.x + r_i.w > r_j.x + 1.0
                && r_i.y < r_j.y + r_j.h - 1.0
                && r_i.y + r_i.h > r_j.y + 1.0;
            assert!(
                !overlaps,
                "Components {} (bounds {:?}) and {} (bounds {:?}) overlap!",
                items[i].id, r_i, items[j].id, r_j
            );
        }
    }

    // Verify undo reverts all items
    c.undo();
    assert_eq!(c.scene.items().len(), 0);
}

#[test]
fn test_add_all_components_uses_cursor_position_reference() {
    let mut c = Canvas::empty();
    c.set_view_size(1000.0, 800.0);

    // Move cursor to specific coordinates
    let cursor_view = Point::new(650.0, 480.0);
    let expected_scene = c.viewport().map_to_circuit(cursor_view);
    c.mouse_move(cursor_view.x, cursor_view.y, 0, 0);

    let snapped_expected = snap_point(expected_scene);
    assert_eq!(c.last_scene(), expected_scene);

    let initial_center = c.viewport().center();
    let initial_zoom = c.viewport().zoom();

    // Add all components using default method (which should use last_scene)
    let ch = c.add_all_components();
    assert!(!c.scene.items().is_empty());
    assert!(
        !ch.viewport,
        "Viewport should not auto-scroll or change zoom"
    );
    assert_eq!(c.viewport().center(), initial_center);
    assert_eq!(c.viewport().zoom(), initial_zoom);

    let first = &c.scene.items()[0];
    let local_rect = first.local_hit_rect();
    let first_top_left_x = first.x + local_rect.x;
    let first_top_left_y = first.y + local_rect.y;

    assert_eq!(first_top_left_x, snapped_expected.x);
    assert_eq!(first_top_left_y, snapped_expected.y);

    // Test explicit starting point via add_all_components_at
    let mut c2 = Canvas::empty();
    c2.set_view_size(1000.0, 800.0);
    let custom_start = Point::new(240.0, 360.0);
    c2.add_all_components_at(custom_start);
    assert!(!c2.scene.items().is_empty());

    let first2 = &c2.scene.items()[0];
    let local_rect2 = first2.local_hit_rect();
    let first2_top_left_x = first2.x + local_rect2.x;
    let first2_top_left_y = first2.y + local_rect2.y;

    assert_eq!(first2_top_left_x, custom_start.x);
    assert_eq!(first2_top_left_y, custom_start.y);
}

#[test]
fn test_library_placeable_coverage() {
    let specs = crate::library::all_placeable_specs();
    assert!(
        !specs.is_empty(),
        "Library placeable specs should not be empty"
    );
    let mut c = Canvas::empty();
    let mut supported_count = 0;
    for (caption, typ) in &specs {
        let spec = if typ == "QemuDevice" || typ == "MCU" {
            format!("{caption},{typ}")
        } else {
            typ.clone()
        };
        if c.scene_add_component_spec(&spec, Point::zero()) {
            supported_count += 1;
        }
    }
    // At least all ported components (70+) must be supported
    assert!(
        supported_count >= 70,
        "Expected at least 70 supported placeable components from library, got {}",
        supported_count
    );

    // Verify each FlipFlop subtype specifically instantiates the expected kind
    let mut c_ff = Canvas::empty();
    assert!(c_ff.scene_add_component_spec("FlipFlopD", Point::zero()));
    assert!(c_ff.scene_add_component_spec("FlipFlopJK", Point::zero()));
    assert!(c_ff.scene_add_component_spec("FlipFlopRS", Point::zero()));
    assert!(c_ff.scene_add_component_spec("FlipFlopT", Point::zero()));
    assert!(c_ff.scene_add_component_spec("LatchD", Point::zero()));

    let items = c_ff.scene.items();
    assert_eq!(items.len(), 5);
    if let crate::canvas::Part::FlipFlop(ff) = &items[0].kind {
        assert_eq!(ff.ff_kind, crate::digital::FlipFlopKind::D);
    } else {
        panic!("expected FlipFlop D");
    }
    if let crate::canvas::Part::FlipFlop(ff) = &items[1].kind {
        assert_eq!(ff.ff_kind, crate::digital::FlipFlopKind::Jk);
    } else {
        panic!("expected FlipFlop JK");
    }
    if let crate::canvas::Part::FlipFlop(ff) = &items[2].kind {
        assert_eq!(ff.ff_kind, crate::digital::FlipFlopKind::Rs);
    } else {
        panic!("expected FlipFlop RS");
    }
    if let crate::canvas::Part::FlipFlop(ff) = &items[3].kind {
        assert_eq!(ff.ff_kind, crate::digital::FlipFlopKind::T);
    } else {
        panic!("expected FlipFlop T");
    }
    assert!(matches!(&items[4].kind, crate::canvas::Part::Latch(_)));
}

#[test]
fn test_focus_component() {
    let mut c = Canvas::empty();
    c.set_view_size(1000.0, 800.0);
    c.add_component_at("Resistor", Point::new(100.0, 200.0));
    c.add_component_at("Capacitor", Point::new(400.0, 500.0));

    let res_id = c.scene.items()[0].id.clone();
    let cap_id = c.scene.items()[1].id.clone();
    let cap_rect = c.scene.items()[1].full_rect();

    let ch = c.focus_component(&cap_id);
    assert!(ch.items);
    assert!(ch.center);
    assert!(ch.viewport);

    // Capacitor should be selected, resistor should not
    assert!(!c.scene.item_by_id(&res_id).unwrap().selected);
    assert!(c.scene.item_by_id(&cap_id).unwrap().selected);

    // Viewport center should be at capacitor center
    let vp_center = c.viewport().center();
    let cap_center = cap_rect.center();
    assert!((vp_center.x - cap_center.x).abs() < 1e-3);
    assert!((vp_center.y - cap_center.y).abs() < 1e-3);

    // Focusing non-existent component returns empty change
    let ch_none = c.focus_component("NonExistent-123");
    assert!(!ch_none.items);
    assert!(!ch_none.center);
}

#[test]
fn test_wire_click_selection() {
    let mut c = Canvas::empty();
    c.set_view_size(1000.0, 800.0);
    c.add_component_at("Resistor", Point::new(100.0, 100.0));
    c.add_component_at("Resistor", Point::new(200.0, 100.0));

    let p1 = c.scene.items()[0].pins()[1].id.clone();
    let p2 = c.scene.items()[1].pins()[0].id.clone();
    let p1_pos = c.scene.pin_scene(&p1).unwrap();
    let p2_pos = c.scene.pin_scene(&p2).unwrap();

    c.scene.connect_pins(&p1, p1_pos, &p2, p2_pos);

    assert_eq!(c.scene.wires().len(), 1);
    assert!(!c.scene.wires()[0].selected);

    // Click on the wire segment midpoint in viewport coordinates
    let mid_scene = Point::new((p1_pos.x + p2_pos.x) / 2.0, (p1_pos.y + p2_pos.y) / 2.0);
    let mid_item = c.viewport().map_from_circuit(mid_scene);
    let ch = c.mouse_press(BUTTON_LEFT, mid_item.x, mid_item.y, 0);
    assert!(ch.wires);
    assert!(c.scene.wires()[0].selected);
}

#[test]
fn right_click_wire_over_component_selects_wire_not_part() {
    let mut c = ready();
    let p1 = c.scene.items()[0].pins()[1].id.clone();
    let p2 = c.scene.items()[1].pins()[0].id.clone();
    let p1_pos = c.scene.pin_scene(&p1).unwrap();
    let p2_pos = c.scene.pin_scene(&p2).unwrap();
    c.scene.connect_pins(&p1, p1_pos, &p2, p2_pos);

    // Detour the wire through the first resistor body so a right-click hits
    // both the wire and the component — the mega328/Arduino board case.
    let body_center = {
        let it = &c.scene.items()[0];
        let b = it.body_rect();
        Point::new(it.x + b.x + b.w / 2.0, it.y + b.y + b.h / 2.0)
    };
    c.scene.wires_mut()[0].points = vec![p1_pos, body_center, p2_pos];

    // Pre-select the component, as after placing a chip and wiring it.
    c.scene.select_only(0);
    assert!(c.scene.items()[0].selected);

    let click = item_of(&c, body_center);
    c.mouse_press(BUTTON_RIGHT, click.x, click.y, 0);
    assert!(
        c.scene.wires()[0].selected,
        "right-click on a wire over a component should select the wire"
    );
    assert!(
        !c.scene.items()[0].selected,
        "the component under the wire must not stay selected"
    );
    assert!(!c.has_pin_hit());

    assert!(c.scene.delete_selected());
    assert_eq!(
        c.scene.items().len(),
        2,
        "removing the wire must not delete the component under it"
    );
    assert!(c.scene.wires().is_empty());
}

#[test]
fn test_cursor_position_tracking_and_component_placement() {
    let mut c = Canvas::empty();
    c.set_view_size(1000.0, 800.0);

    // Before any mouse move, last_scene falls back to viewport center (0.0, 0.0)
    assert!(!c.cursor_moved());
    assert_eq!(c.last_scene(), Point::zero());

    // Mouse moves to a specific point on canvas (e.g. view coordinates 650, 480)
    let cursor_view = Point::new(650.0, 480.0);
    let expected_scene = c.viewport().map_to_circuit(cursor_view);
    c.mouse_move(cursor_view.x, cursor_view.y, 0, 0);

    assert!(c.cursor_moved());
    assert!((c.last_scene().x - expected_scene.x).abs() < 1e-6);
    assert!((c.last_scene().y - expected_scene.y).abs() < 1e-6);

    // Insert component at last_scene
    c.add_component_at("Resistor", c.last_scene());
    assert_eq!(c.scene.items().len(), 1);
    let r1 = &c.scene.items()[0];
    let snapped_expected = snap_point(expected_scene);
    assert_eq!(Point::new(r1.x, r1.y), snapped_expected);

    // Move cursor to another location (e.g. view coordinates 200, 160)
    let new_cursor_view = Point::new(200.0, 160.0);
    let new_expected_scene = c.viewport().map_to_circuit(new_cursor_view);
    c.mouse_move(new_cursor_view.x, new_cursor_view.y, 0, 0);

    assert_eq!(c.last_scene(), new_expected_scene);

    // Insert another component at the new cursor location
    c.add_component_at("Capacitor", c.last_scene());
    assert_eq!(c.scene.items().len(), 2);
    let cap = &c.scene.items()[1];
    let new_snapped_expected = snap_point(new_expected_scene);
    assert_eq!(Point::new(cap.x, cap.y), new_snapped_expected);
}

#[test]
fn test_cmd_a_select_all_selects_items_and_wires() {
    let mut c = Canvas::empty();
    c.scene_mut().add_default_resistor(0.0, 0.0);
    c.scene_mut().add_default_resistor(80.0, 0.0);
    let r1_r = c.scene().hit_pin(Point::new(16.0, 0.0)).unwrap();
    let r2_l = c.scene().hit_pin(Point::new(64.0, 0.0)).unwrap();
    c.scene_mut().connect_pins(
        &r1_r.id,
        Point::new(16.0, 0.0),
        &r2_l.id,
        Point::new(64.0, 0.0),
    );

    assert!(!c.scene().items()[0].selected);
    assert!(!c.scene().items()[1].selected);
    assert!(!c.scene().wires()[0].selected);

    // Press Cmd + A (or Ctrl + A)
    let ch = c.key_press(crate::canvas::KEY_A, crate::canvas::MOD_META);
    assert!(ch.items, "Change.items must be true");
    assert!(
        ch.wires,
        "Change.wires must be true so canvas repaints wire selection"
    );
    assert!(c.scene().items()[0].selected);
    assert!(c.scene().items()[1].selected);
    assert!(c.scene().wires()[0].selected);
}

#[test]
fn test_pin_hover_and_unhover_transitions() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene_mut().add_default_resistor(0.0, 0.0);

    // Hover over the left pin at (-16, 0)
    let pin_item_pos = c.viewport().map_from_circuit(Point::new(-16.0, 0.0));
    let ch1 = c.mouse_move(pin_item_pos.x, pin_item_pos.y, 0, 0);
    assert!(
        ch1.hovered_pin,
        "Change.hovered_pin must be true when hovering a pin"
    );
    assert_eq!(c.hovered_pin(), Some("Resistor-1-lPin"));

    // Move away from the pin to empty space (e.g. at (100, 100))
    let empty_item_pos = c.viewport().map_from_circuit(Point::new(100.0, 100.0));
    let ch2 = c.mouse_move(empty_item_pos.x, empty_item_pos.y, 0, 0);
    assert!(
        ch2.hovered_pin,
        "Change.hovered_pin must be true when moving away from a pin"
    );
    assert_eq!(
        c.hovered_pin(),
        None,
        "Hovered pin must be None after moving away"
    );

    // Hover again over the right pin at (16, 0)
    let pin_r_item_pos = c.viewport().map_from_circuit(Point::new(16.0, 0.0));
    let ch3 = c.mouse_move(pin_r_item_pos.x, pin_r_item_pos.y, 0, 0);
    assert!(ch3.hovered_pin);
    assert_eq!(c.hovered_pin(), Some("Resistor-1-rPin"));

    // Call mouse_cancel (e.g. mouse leaves canvas)
    let ch4 = c.mouse_cancel();
    assert!(
        ch4.hovered_pin,
        "Change.hovered_pin must be true on mouse_cancel when a pin was hovered"
    );
    assert_eq!(
        c.hovered_pin(),
        None,
        "Hovered pin must be cleared on mouse_cancel"
    );
}

#[test]
fn test_unused_pin_canvas_interaction() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene_mut()
        .add_qemu(
            "Esp32",
            100.0,
            100.0,
            &crate::subcircuit::SubcSearch::default(),
        )
        .unwrap();

    let it = &c.scene().items()[0];
    let vdd2 = it
        .pins()
        .into_iter()
        .find(|p| p.id == "Esp32-1-Vdd2")
        .unwrap();
    assert!(vdd2.unused);
    let vdd_scene_pos = it.pin_scene_pos(&vdd2);
    let vdd_item_pos = c.viewport().map_from_circuit(vdd_scene_pos);

    // Hover over Vdd2: cursor should be Arrow (not Cross)
    c.mouse_move(vdd_item_pos.x, vdd_item_pos.y, 0, 0);
    assert_eq!(c.cursor(), CursorKind::Arrow);

    // Tooltip should describe the pin and indicate it is Unused
    let tooltip = c.hover_tooltip(vdd_scene_pos).expect("tooltip exists");
    assert!(
        tooltip.contains("Unused"),
        "tooltip should contain 'Unused': {tooltip}"
    );
    assert!(
        tooltip.contains("Positive supply voltage (Unused)"),
        "tooltip should say 'Positive supply voltage (Unused)': {tooltip}"
    );

    // Click on Vdd2: should NOT start drawing a wire
    c.mouse_press(
        crate::canvas::BUTTON_LEFT,
        vdd_item_pos.x,
        vdd_item_pos.y,
        0,
    );
    assert!(!c.scene().drawing());
}

#[test]
fn test_selected_item_uarts() {
    let mut c = Canvas::empty();
    c.scene_mut().add_default_resistor(0.0, 0.0);
    c.scene_mut().select_only(0);
    assert!(c.selected_item_uarts().is_empty(), "Resistor has no UARTs");

    let mut c2 = Canvas::empty();
    c2.scene_mut()
        .add_qemu(
            "Esp32",
            100.0,
            100.0,
            &crate::subcircuit::SubcSearch::default(),
        )
        .unwrap();
    c2.scene_mut().select_only(0);
    let uarts = c2.selected_item_uarts();
    assert_eq!(
        uarts,
        vec!["USART1", "USART2", "USART3", "SPI1", "SPI2", "I2C1", "I2C2"]
    );
}

#[test]
fn test_hit_label_and_drag_label_and_value_with_undo() {
    let mut c = Canvas::empty();
    c.scene_mut().add_default_resistor(100.0, 100.0);
    c.scene_mut().items_mut()[0].show_id = true;
    c.scene_mut().items_mut()[0].show_val = true;
    let res = &c.scene().items()[0];
    let (lx, ly) = res.label_pos();
    let (vx, vy) = res.val_pos();

    // Label scene position
    let label_scene = res.map_local(Point::new(lx + 4.0, ly + 4.0));
    let val_scene = res.map_local(Point::new(vx + 4.0, vy + 4.0));
    let label_item = c.viewport().map_from_circuit(label_scene);
    let val_item = c.viewport().map_from_circuit(val_scene);

    // Hover test
    let _hover_change = c.mouse_move(label_item.x, label_item.y, 0, 0);
    assert_eq!(c.cursor(), CursorKind::OpenHand);

    // Click and drag ID label
    c.mouse_press(crate::canvas::BUTTON_LEFT, label_item.x, label_item.y, 0);
    assert_eq!(c.cursor(), CursorKind::ClosedHand);

    c.mouse_move(
        label_item.x + 20.0,
        label_item.y - 15.0,
        crate::canvas::BUTTON_LEFT,
        0,
    );
    assert!(c.scene().items()[0].custom_label_pos);
    assert_ne!(c.scene().items()[0].label_x, lx);

    c.mouse_release(
        crate::canvas::BUTTON_LEFT,
        label_item.x + 20.0,
        label_item.y - 15.0,
        0,
    );
    assert_eq!(c.cursor(), CursorKind::OpenHand);
    assert!(c.can_undo());

    // Rotate label
    let uid = c.scene().items()[0].id.clone();
    c.rotate_item_label(&uid, 90);
    assert_eq!(c.scene().items()[0].label_rot, 90);

    // Undo rotation and drag
    c.undo();
    assert_eq!(c.scene().items()[0].label_rot, 0);
    c.undo();
    assert_eq!(c.scene().items()[0].label_x, lx);

    // Click and drag value label
    c.mouse_press(crate::canvas::BUTTON_LEFT, val_item.x, val_item.y, 0);
    assert_eq!(c.cursor(), CursorKind::ClosedHand);

    c.mouse_move(
        val_item.x + 10.0,
        val_item.y + 25.0,
        crate::canvas::BUTTON_LEFT,
        0,
    );
    assert!(c.scene().items()[0].custom_val_pos);
    assert_ne!(c.scene().items()[0].val_x, vx);

    c.mouse_release(
        crate::canvas::BUTTON_LEFT,
        val_item.x + 10.0,
        val_item.y + 25.0,
        0,
    );
    assert!(c.can_undo());
}

#[test]
fn test_canvas_firmware_source_resolution() {
    let dir = std::env::temp_dir().join(format!("cs-canvas-firmware-src-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pic14test")).unwrap();
    std::fs::write(dir.join("pic14test").join("pic14test.mcu"), PIC14_MCU).unwrap();

    let hex_path = dir.join("porta.hex");
    let ino_path = dir.join("porta.ino");
    std::fs::write(
        &hex_path,
        ":0C0000008316850183120130850005285D\n:00000001FF\n",
    )
    .unwrap();
    std::fs::write(&ino_path, "void setup() {} void loop() {}").unwrap();

    let sim1 = r#"<circuit version="1.0.0" >
<item itemtype="Mcu" CircId="pic14test-1" Frequency="4 MHz" Program="porta.hex" AutoLoad="true" />
</circuit>"#;
    let path = dir.join("porta.sim1");
    std::fs::write(&path, sim1).unwrap();

    let mut canvas = Canvas::empty();
    canvas
        .load_sim1(sim1, Some(path.to_string_lossy().into_owned()))
        .unwrap();

    // Path and source lookup by explicit UID
    assert_eq!(
        canvas.firmware_path_for("pic14test-1"),
        Some(hex_path.clone())
    );
    assert_eq!(
        canvas.firmware_source_for("pic14test-1"),
        Some(ino_path.clone())
    );

    // Empty UID falls back to the first MCU in the scene
    assert_eq!(canvas.firmware_path_for(""), Some(hex_path.clone()));
    assert_eq!(canvas.firmware_source_for(""), Some(ino_path.clone()));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_canvas_tick_respects_fps_setting() {
    let sim1 = r#"<circuit version="1.0.0">
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Pos="0,0" Voltage="5 V" />
<item itemtype="Resistor" CircId="Resistor-1" Pos="80,0" Resistance="1000 Ω" />
<item itemtype="Ground" CircId="Ground-1" Pos="160,0" />
<item itemtype="Connector" CircId="Wire-1" startpinid="Fixed Voltage-1-outnod" endpinid="Resistor-1-lPin" pointList="0,0,80,0" />
<item itemtype="Connector" CircId="Wire-2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" pointList="80,0,160,0" />
</circuit>"#;
    // Test with 20 FPS
    crate::settings::edit(|s| s.fps = 20);
    let mut c = Canvas::empty();
    c.scene = Scene::from_sim1(sim1).unwrap();
    let ps_per_sec = c.scene().settings().ps_per_sec();
    c.power_on();
    c.tick();
    let time_at_20fps = c.circ_time();
    assert_eq!(time_at_20fps, 1 + ps_per_sec / 20);

    // Test with 50 FPS
    crate::settings::edit(|s| s.fps = 50);
    let mut c2 = Canvas::empty();
    c2.scene = Scene::from_sim1(sim1).unwrap();
    c2.power_on();
    c2.tick();
    let time_at_50fps = c2.circ_time();
    assert_eq!(time_at_50fps, 1 + ps_per_sec / 50);

    // Restore default FPS
    crate::settings::edit(|s| s.fps = crate::settings::DEFAULT_FPS);
}

#[test]
fn from_component_sets_items_props_history() {
    let cc = ComponentChange::document("R-1");
    let c = Change::from_component(&cc);
    assert!(c.items);
    assert!(c.props);
    assert!(c.history);
    assert!(!c.wires);

    let live = Change::from_component(&ComponentChange::live("Push-1"));
    assert!(!live.items);
    assert!(!live.history);
    assert!(!live.props);

    let knob = Change::from_component(&ComponentChange::continuous("VoltSource-1"));
    assert!(!knob.items);
    assert!(!knob.history);
    assert!(!knob.props);
}

#[test]
fn component_change_bus_sets_items_props_history_and_modified() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let id = c.scene.add_default_resistor(0.0, 0.0);
    c.mark_saved();
    assert!(!c.is_modified());

    let change = c.apply_component_edit(ComponentChange::document(&id), |scene| {
        scene
            .item_by_id_mut(&id)
            .is_some_and(|it| it.set_prop_text("Resistance", "4.7 kΩ"))
    });
    assert!(change.items, "view must refresh");
    assert!(change.props, "prop dialog must refresh");
    assert!(change.history, "modified notify rides on history");
    assert!(c.is_modified());
    assert!(c.can_undo());
}

#[test]
fn pot_wiper_through_bus_marks_modified() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let id = c.scene.add_potentiometer(0.0, 0.0);
    c.mark_saved();
    assert!(!c.is_modified());

    let change = c.set_pot_wiper(&id, 0.75);
    assert!(!change.items, "wiper must not rebuild the QML items model");
    assert!(c.is_modified());
    assert!(c.to_sim1().contains("Wiper=\"0.75\""), "{}", c.to_sim1());
}

#[test]
fn live_push_does_not_mark_modified() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let id = c.scene.add_push(0.0, 0.0);
    c.mark_saved();
    let change = c.set_push_state(&id, true);
    assert!(
        !change.items,
        "live push must not rebuild the QML items model"
    );
    assert!(!change.history);
    assert!(!c.is_modified());
}

#[test]
fn test_canvas_overflow_detection_and_auto_fit() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let mut s = c.circ_settings().clone();
    s.width = 1000;
    s.height = 800;
    c.set_circ_settings(s);

    // Within bounds (scene_rect is [-500, -400, 1000, 800])
    c.scene.add_default_resistor(0.0, 0.0);
    let info = c.canvas_overflow_info();
    assert!(!info.has_overflow);
    assert_eq!(info.overflowing_count, 0);

    // Place component far to the right (x = 800, which exceeds 500)
    let _far_id = c.scene.add_default_resistor(800.0, 0.0);
    let info_overflow = c.canvas_overflow_info();
    assert!(info_overflow.has_overflow);
    assert!(info_overflow.overflowing_count >= 1);
    assert!(info_overflow.required_width > 1000);

    // Select overflowing
    let sel_change = c.select_overflowing_items();
    assert!(sel_change.items);
    assert!(c.scene.items().iter().any(|it| it.selected && it.x > 500.0));
    assert!(
        c.scene
            .items()
            .iter()
            .any(|it| !it.selected && it.x < 500.0)
    );

    // Auto-fit expands canvas
    let fit_change = c.auto_fit_canvas(200.0);
    assert!(fit_change.settings);
    assert!(fit_change.history);
    let info_after_fit = c.canvas_overflow_info();
    assert!(!info_after_fit.has_overflow);
    assert!(c.circ_settings().width >= info_overflow.required_width);

    // Test Undo reverts canvas expansion
    assert!(c.can_undo());
    c.undo();
    let info_reverted = c.canvas_overflow_info();
    assert!(info_reverted.has_overflow);
    assert_eq!(c.circ_settings().width, 1000);
}

#[test]
fn test_canvas_center_circuit() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let mut s = c.circ_settings().clone();
    s.width = 1000;
    s.height = 800;
    c.set_circ_settings(s);

    // Add resistors off-center at (600, 500) and (800, 500)
    c.scene.add_default_resistor(600.0, 500.0);
    c.scene.add_default_resistor(800.0, 500.0);
    let info_before = c.canvas_overflow_info();
    assert!(info_before.has_overflow);

    // Center circuit
    let center_change = c.center_circuit();
    assert!(center_change.items);
    assert!(center_change.history);

    let bounds = c.scene.items_bounding_rect();
    let center = bounds.center();
    assert!(
        center.x.abs() < 10.0,
        "center.x should be near 0, got {}",
        center.x
    );
    assert!(
        center.y.abs() < 10.0,
        "center.y should be near 0, got {}",
        center.y
    );

    // Since width is 1000 and items span ~200px, centering puts them inside the 1000x800 canvas!
    let info_after = c.canvas_overflow_info();
    assert!(!info_after.has_overflow);

    // Undo recentering
    assert!(c.can_undo());
    c.undo();
    let info_undone = c.canvas_overflow_info();
    assert!(info_undone.has_overflow);
}

#[test]
fn test_drag_node_moves_connected_wires() {
    let src = include_str!("../../tests/fixtures/divider.sim1");
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_sim1(src, Some("/tmp/divider.sim1".into())).unwrap();

    let node = c
        .scene
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "Node")
        .expect("Node must exist in divider.sim1");
    let node_id = node.id.clone();
    let initial_pos = node.position();

    // Click on the node and drag it by (0, 24)
    let p_item = item_of(&c, initial_pos);
    c.mouse_press(BUTTON_LEFT, p_item.x, p_item.y, 0);

    let target_scene = Point::new(initial_pos.x, initial_pos.y + 24.0);
    let target_item = item_of(&c, target_scene);
    c.mouse_move(target_item.x, target_item.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, target_item.x, target_item.y, 0);

    // Verify node moved to new position
    let moved_node = c.scene.item_by_id(&node_id).unwrap();
    assert_eq!(moved_node.position(), target_scene);

    // Verify connected wires have their endpoints updated
    for w in c.scene.wires() {
        if w.start_pin.starts_with(&node_id) {
            assert_eq!(w.points.first().copied(), Some(target_scene));
        }
        if w.end_pin.as_ref().is_some_and(|e| e.starts_with(&node_id)) {
            assert_eq!(w.points.last().copied(), Some(target_scene));
        }
    }

    // Verify undo restores original position
    assert!(c.can_undo());
    c.undo();
    let restored_node = c.scene.item_by_id(&node_id).unwrap();
    assert_eq!(restored_node.position(), initial_pos);
}

#[test]
fn test_node_hover_cursor_and_alt_start_wire() {
    let src = include_str!("../../tests/fixtures/divider.sim1");
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.load_sim1(src, Some("/tmp/divider.sim1".into())).unwrap();

    let node = c
        .scene
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "Node")
        .unwrap();
    let node_pos = node.position();
    let p_item = item_of(&c, node_pos);

    // Normal hover on node shows OpenHand cursor
    c.mouse_move(p_item.x, p_item.y, 0, 0);
    assert_eq!(c.cursor(), CursorKind::OpenHand);

    // Hover with Alt on node with free pin shows Cross cursor
    c.mouse_move(p_item.x, p_item.y, 0, MOD_ALT);
    assert_eq!(c.cursor(), CursorKind::Cross);

    // Alt-click starts drafting a new wire branch
    c.mouse_press(BUTTON_LEFT, p_item.x, p_item.y, MOD_ALT);
    assert!(c.scene.drawing());
    c.key_press(KEY_ESCAPE, 0);
    assert!(!c.scene.drawing());
}

#[test]
fn test_node_drawing_and_color_matching() {
    use crate::canvas::draw::{Align, Color, Draw, PaintCtx, Palette};

    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let nid = c.scene.add_node(0.0, 0.0);

    struct MockDraw {
        circles: Vec<(f64, f64, f64, Color)>,
    }
    impl Draw for MockDraw {
        fn fill_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _c: Color) {}
        fn stroke_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _c: Color, _width: f64) {}
        fn fill_round_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _r: f64, _c: Color) {}
        fn stroke_round_rect(
            &mut self,
            _x: f64,
            _y: f64,
            _w: f64,
            _h: f64,
            _r: f64,
            _c: Color,
            _width: f64,
        ) {
        }
        fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, color: Color) {
            self.circles.push((cx, cy, r, color));
        }
        fn stroke_circle(&mut self, _cx: f64, _cy: f64, _r: f64, _c: Color, _width: f64) {}
        fn fill_ellipse(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _c: Color) {}
        fn stroke_ellipse(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _c: Color, _width: f64) {}
        fn fill_poly(&mut self, _pts: &[[f64; 2]], _c: Color) {}
        fn stroke_poly(&mut self, _pts: &[[f64; 2]], _c: Color, _width: f64, _close: bool) {}
        fn polyline(&mut self, _pts: &[[f64; 2]], _c: Color, _width: f64, _dash: bool) {}
        fn line(&mut self, _x0: f64, _y0: f64, _x1: f64, _y1: f64, _c: Color, _width: f64) {}
        fn arc(&mut self, _cx: f64, _cy: f64, _r: f64, _a0: f64, _a1: f64, _c: Color, _width: f64) {
        }
        fn text(&mut self, _x: f64, _y: f64, _s: &str, _size: f64, _c: Color, _align: Align) {}
        fn grid_dots(&mut self, _scene: Rect, _step: i32, _c: Color) {}
        fn push(&mut self, _x: f64, _y: f64, _rot: f64, _sx: f64, _sy: f64) {}
        fn pop(&mut self) {}
    }

    let pal = Palette {
        canvas: Color::rgb(255, 255, 255),
        grid: Color::rgb(200, 200, 200),
        body: Color::rgb(240, 240, 240),
        border: Color::rgb(50, 50, 50),
        wire: Color::rgb(0, 150, 0),
        pin_high: Color::rgb(255, 0, 0),
        pin_low: Color::rgb(0, 0, 255),
        band: Color::rgb(255, 200, 0),
        tunnel_color1: Color::rgb(0, 0, 0),
        tunnel_color2: Color::rgb(0, 0, 0),
        tunnel_color3: Color::rgb(0, 0, 0),
        pin_open_high: Color::rgb(0, 0, 0),
        meter_display: Color::rgb(0, 0, 0),
        msg_warn: Color::rgb(0, 0, 0),
        msg_error: Color::rgb(0, 0, 0),
    };

    let pctx = PaintCtx {
        canvas: &c,
        pal: &pal,
        scale: 1.0,
        item_id: &nid,
    };

    let mut md = MockDraw { circles: vec![] };
    let node_item = c.scene.item_by_id(&nid).unwrap();
    node_item.kind.paint(&mut md, &pctx);

    assert_eq!(md.circles.len(), 1);
    let (_, _, r, color) = md.circles[0];
    assert_eq!(r, 1.5, "Node radius should be shrunk to 1.5px");
    assert_eq!(color, pal.wire, "Idle node color must match wire color");
}

#[test]
fn test_audio_out_power_cycle_lifecycle() {
    let sim1 = r#"<circuit version="1.0.0">
<item itemtype="WaveGen" CircId="WaveGen-1" Pos="0,0" Frequency="1000 Hz" Amplitude="5 V" />
<item itemtype="AudioOut" CircId="AudioOut-1" Pos="80,0" Impedance="8 Ω" Volume="100" />
<item itemtype="Connector" CircId="Wire-1" startpinid="WaveGen-1-outnod" endpinid="AudioOut-1-lPin" pointList="0,0,80,0" />
</circuit>"#;
    let scene = Scene::from_sim1(sim1).expect("valid sim1");
    let mut c = Canvas::empty();
    c.scene = scene;

    let recorder = std::sync::Arc::new(std::sync::Mutex::new(crate::audio::Recorder::default()));
    c.set_audio_sink(recorder.clone());

    // First power cycle
    c.power_on();
    for _ in 0..5 {
        c.tick();
    }
    let count1 = recorder.lock().unwrap().samples.len();
    assert!(count1 > 0, "Samples should be produced in first run");

    // Power off
    c.power_off();

    // Second power cycle (must resume sink and continue recording samples)
    c.power_on();
    for _ in 0..5 {
        c.tick();
    }
    let count2 = recorder.lock().unwrap().samples.len();
    assert!(
        count2 > count1,
        "Audio samples must continue to be recorded after power cycle (got {count2} vs {count1})"
    );

    c.power_off();
}

#[test]
fn dc_resistor_is_not_dirty_after_settle() {
    let mut c = Canvas::with_demo_divider();
    c.scene.settings_mut().animate_curr = false;
    c.scene.settings_mut().animate_logic = false;
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    let _ = c.take_dirty();
    for _ in 0..12 {
        c.tick();
    }
    let dirty = c.take_dirty();
    assert!(
        !dirty.full,
        "DC divider should not mark the whole canvas dirty after settle"
    );
    for it in c.scene.items() {
        if matches!(&it.kind, Part::Resistor(_)) {
            assert!(
                !dirty.items.contains(&it.id),
                "resistor {} should not redraw every tick",
                it.id
            );
        }
    }
}

#[test]
fn animate_curr_dirties_wires_not_resistors() {
    let mut c = Canvas::with_demo_divider();
    c.scene.settings_mut().animate_curr = true;
    c.scene.settings_mut().animate_logic = false;
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    let _ = c.take_dirty();
    let mut saw_wire = false;
    for _ in 0..12 {
        c.tick();
        let dirty = c.take_dirty();
        for it in c.scene.items() {
            if matches!(&it.kind, Part::Resistor(_)) {
                assert!(
                    !dirty.items.contains(&it.id),
                    "resistor {} dirtied by current chevrons",
                    it.id
                );
            }
        }
        if !dirty.wires.is_empty() {
            saw_wire = true;
        }
    }
    assert!(
        saw_wire,
        "current animation should dirty current-carrying wires"
    );
}

#[test]
fn curr_speed_slider_matches_cpp_curve() {
    assert!((curr_speed_from_slider(150) - 1.0).abs() < 1e-12);
    assert!((curr_speed_from_slider(300) - 2.0_f64.powf(4.5)).abs() < 1e-9);
    assert!(curr_speed_from_slider(1) < curr_speed_from_slider(150));
    assert!((wrap_chevron_step(9.0) - 1.0).abs() < 1e-12);
    assert!((wrap_chevron_step(-1.0) - 7.0).abs() < 1e-12);
}

#[test]
fn chevron_step_advances_with_current_speed_slider() {
    let mut c = Canvas::with_demo_divider();
    c.scene.settings_mut().animate_curr = true;
    c.scene.settings_mut().animate_logic = false;
    c.set_curr_speed_slider(150);
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    let _ = c.take_dirty();
    let wire_id = c
        .scene
        .wires()
        .iter()
        .find(|w| c.wire_current(&w.id).abs() > 1e-12)
        .map(|w| w.id.clone())
        .expect("demo divider should carry current");

    c.tick();
    let slow = c.chevron_step(&wire_id);
    assert!(slow > 0.0, "chevrons must advance each frame, got {slow}");
    c.tick();
    let slow2 = c.chevron_step(&wire_id);
    let slow_delta = slow2 - slow;
    assert!(
        (slow_delta - slow).abs() < 1e-9,
        "per-frame step must accumulate (C++ m_step += speed*current), not tick*speed; slow={slow} slow2={slow2}"
    );

    let mut fast = Canvas::with_demo_divider();
    fast.scene.settings_mut().animate_curr = true;
    fast.scene.settings_mut().animate_logic = false;
    fast.set_curr_speed_slider(1000);
    fast.power_on();
    let _ = fast.take_dirty();
    fast.tick();
    let fast_step = fast.chevron_step(&wire_id);
    assert!(
        fast_step > slow * 2.0,
        "higher Current Speed slider must move chevrons faster; slow={slow} fast={fast_step}"
    );
}

#[test]
fn pin_logic_crossing_dirties_resistor() {
    let mut c = Canvas::with_demo_divider();
    c.scene.settings_mut().animate_curr = false;
    c.scene.settings_mut().animate_logic = true;
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    let _ = c.take_dirty();
    // Force a logic-level change on Resistor-1's left pin (was ~5 V).
    c.set_pin_voltage("Resistor-1-lPin", 0.0);
    // Re-publish by ticking; solver will restore 5 V, crossing 2.5 V.
    c.tick();
    let dirty = c.peek_dirty();
    assert!(
        dirty.items.contains("Resistor-1"),
        "resistor should dirty when a pin crosses the logic threshold, got {:?}",
        dirty.items
    );
}

#[test]
fn led_dirty_rect_covers_glow_halo() {
    let mut c = Canvas::empty();
    c.scene.add_led(0.0, 0.0);
    let dirty = super::dirty::item_dirty_rect(&c.scene.items()[0]);
    // Body is y ∈ [-8, 8]; warning halo around (2, 0) reaches ±22.
    assert!(
        dirty.contains_point(Point::new(2.0, 18.0)),
        "LED glow above the body must be in the dirty rect, got {dirty:?}"
    );
    assert!(
        dirty.contains_point(Point::new(2.0, -18.0)),
        "LED glow below the body must be in the dirty rect, got {dirty:?}"
    );
}

#[test]
fn servo_dirty_rect_covers_rotating_horn() {
    let mut c = Canvas::empty();
    c.scene.add_servo(0.0, 0.0);
    let dirty = super::dirty::item_dirty_rect(&c.scene.items()[0]);
    // Horn stadium about (16, 0): tips at 90° / 180° / 0° sit outside the
    // 80×48 enclosure (x ≤ 40, |y| ≤ 24).
    for (p, label) in [
        (Point::new(56.0, 0.0), "90° tip"),
        (Point::new(16.0, 40.0), "180° tip"),
        (Point::new(16.0, -40.0), "0° tip"),
    ] {
        assert!(
            dirty.contains_point(p),
            "servo {label} {p:?} must be in the dirty rect, got {dirty:?}"
        );
    }
}

#[test]
fn led_region_raster_updates_glow_outside_body() {
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="0,0" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="220" Pos="50,0" />
<item itemtype="Led" CircId="Led-1" Color="Yellow" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="150,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Resistor-1-lPin" pointList="0,0,50,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Resistor-1-rPin" endpinid="Led-1-lPin" pointList="50,0,100,0" />
<item itemtype="Connector" uid="conn-3" startpinid="Ground-1-Gnd" endpinid="Led-1-rPin" pointList="150,0,100,0" />
</circuit>"#;
    let mut c = Canvas::empty();
    c.set_view_size(200.0, 200.0);
    c.set_center(100.0, 0.0);
    c.load_sim1(sim1, Some("/tmp/test_led_glow_dirty.sim1".into()))
        .unwrap();
    c.set_view_size(200.0, 200.0);
    c.zoom_one();
    let led_pos = c.scene.item_by_id("Led-1").expect("Led-1").position();
    c.set_center(led_pos.x, led_pos.y);
    let pal = Palette::dark();
    // Live canvas rasters with a transparent pixmap (GPU grid underneath).
    let unlit = render_viewport(&c, &pal, 200, 200, 1.0, false).expect("unlit");
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    for _ in 0..10 {
        c.tick();
    }
    let lit = render_viewport(&c, &pal, 200, 200, 1.0, false).expect("lit");
    let led = c.scene.item_by_id("Led-1").expect("Led-1");
    let patch = super::dirty::item_dirty_rect(led);
    let mut patched = render_viewport(&c, &pal, 200, 200, 1.0, false).expect("base");
    patched.data_mut().copy_from_slice(unlit.data());
    assert!(render_viewport_regions_mut(
        &c,
        &pal,
        &mut patched,
        1.0,
        &[patch]
    ));
    // Local (2, -12): above the 16-tall body, inside the r=18 halo.
    let glow = Point::new(led_pos.x + 2.0, led_pos.y - 12.0);
    let s = c.viewport().zoom();
    let px = (100.0 + (glow.x - led_pos.x) * s).floor() as u32;
    let py = (100.0 + (glow.y - led_pos.y) * s).floor() as u32;
    let i = ((py * 200 + px) * 4) as usize;
    let glow_lit = &lit.data()[i..i + 4];
    let glow_patched = &patched.data()[i..i + 4];
    let glow_unlit = &unlit.data()[i..i + 4];
    assert_ne!(
        glow_lit, glow_unlit,
        "lit LED must paint glow above the body at pixel ({px},{py})"
    );
    assert_eq!(
        glow_patched, glow_lit,
        "dirty-region raster must update the glow outside the LED body"
    );
}

#[test]
fn test_circ_settings_discrete_edit_marks_modified_and_supports_undo() {
    let mut c = Canvas::empty();
    c.mark_saved();
    assert!(!c.is_modified());
    assert!(!c.can_undo());

    let old_w = c.circ_settings().width;
    let change = c.update_circ_settings(|s| s.width = old_w + 500);
    assert!(change.history, "history flag must be true");
    assert!(change.settings, "settings flag must be true");
    assert!(c.is_modified(), "canvas must be marked modified");
    assert!(c.can_undo(), "undo must be available");
    assert_eq!(c.circ_settings().width, old_w + 500);

    let undo_change = c.undo();
    assert!(undo_change.settings, "undo must flag settings change");
    assert_eq!(c.circ_settings().width, old_w);
    assert!(!c.is_modified(), "reverting setting clears modified");
    assert!(c.can_redo(), "redo must be available");

    let redo_change = c.redo();
    assert!(redo_change.settings, "redo must flag settings change");
    assert_eq!(c.circ_settings().width, old_w + 500);
    assert!(c.is_modified(), "redo restores modified");
}

#[test]
fn test_circ_settings_continuous_slider_drag_groups_single_undo() {
    let mut c = Canvas::empty();
    let initial_speed = c.circ_settings().speed_percent();
    c.mark_saved();
    assert!(!c.is_modified());

    c.begin_circ_settings_edit();

    let ch1 = c.update_circ_settings(|s| s.set_speed_percent(20.0));
    assert!(
        !ch1.history,
        "in-flight drag should not emit history / new undo step"
    );
    assert!(ch1.settings);

    let ch2 = c.update_circ_settings(|s| s.set_speed_percent(50.0));
    assert!(
        !ch2.history,
        "in-flight drag should not emit history / new undo step"
    );
    assert!(ch2.settings);

    let commit_change = c.commit_circ_settings_edit();
    assert!(commit_change.history, "commit must emit history flag");
    assert!(c.is_modified());
    assert!(c.can_undo());

    let undo_change = c.undo();
    assert!(undo_change.settings);
    assert!((c.circ_settings().speed_percent() - initial_speed).abs() < 1e-3);
    assert!(!c.is_modified());
    assert!(!c.can_undo());
}
