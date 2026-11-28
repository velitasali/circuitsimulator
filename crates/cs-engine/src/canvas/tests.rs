//! Canvas unit and regression tests.

use super::*;
use crate::canvas::export::text;
use crate::canvas::geom::to_grid as snap_point;
use crate::canvas::scene::HitTarget;
use crate::components::ComponentChange;

#[test]
fn latch_trigger_none_drops_clock_wire() {
    let mut c = Canvas::empty();
    let id = c.scene.add_latch_d(0.0, 0.0);
    let rid = c.scene.add_default_resistor(80.0, 0.0);
    let clk = format!("{id}-clk");
    let rpin = format!("{rid}-lPin");
    let clk_at = c.scene.pin_scene(&clk).expect("clock pin");
    let r_at = c.scene.pin_scene(&rpin).expect("resistor pin");
    c.scene.connect_pins(&clk, clk_at, &rpin, r_at);
    assert_eq!(c.scene.wires().len(), 1);
    c.set_prop_text_for(&id, "Trigger".to_string(), "None".to_string());
    assert!(c.scene.wires().is_empty());
    let pins = c.scene.item_by_id(&id).expect("latch").pins();
    assert!(pins.iter().all(|p| p.id != clk));
}

#[test]
fn and_gate_num_inputs_drops_removed_pin_wire() {
    let mut c = Canvas::empty();
    let id = c.scene.add_and_gate(0.0, 0.0);
    let rid = c.scene.add_default_resistor(80.0, 0.0);
    let pin = format!("{id}-in1");
    let rpin = format!("{rid}-lPin");
    let pin_at = c.scene.pin_scene(&pin).expect("input pin");
    let r_at = c.scene.pin_scene(&rpin).expect("resistor pin");
    c.scene.connect_pins(&pin, pin_at, &rpin, r_at);
    assert_eq!(c.scene.wires().len(), 1);
    c.set_prop_text_for(&id, "NumInputs".to_string(), "1".to_string());
    assert!(c.scene.wires().is_empty());
}

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
    let scene = Scene::from_circ1(sim1).expect("valid sim1");
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
    let scene = Scene::from_circ1(sim1).expect("valid sim1");
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
fn zoom_to_fit_respects_available_area() {
    let mut c = Canvas::empty();
    c.set_view_size(1200.0, 800.0);
    c.set_fit_rect(56.0, 88.0, 720.0, 700.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.zoom_to_fit();
    let r = c
        .scene
        .items_bounding_rect()
        .adjust(-20.0, -20.0, 20.0, 20.0);
    let mapped = c.viewport.map_from_circuit(r.center());
    assert!((mapped.x - (56.0 + 360.0)).abs() < 1e-6);
    assert!((mapped.y - (88.0 + 350.0)).abs() < 1e-6);
}

#[test]
fn geometry_edits_keep_simulation_time() {
    let mut c = Canvas::with_demo_divider();
    c.set_view_size(800.0, 600.0);
    c.power_on();
    c.tick();
    let started = c.circ_time();
    assert!(started > 1, "tick should advance, t={started}");
    let mid = c.pin_voltage("Resistor-1-rPin").unwrap();

    let origin = c.scene.item_by_id("Resistor-1").unwrap().position();
    let from = item_of(&c, origin);
    c.mouse_press(BUTTON_LEFT, from.x, from.y, 0);
    let to = item_of(&c, Point::new(origin.x, origin.y + 16.0));
    c.mouse_move(to.x, to.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, to.x, to.y, 0);
    let moved = c.scene.item_by_id("Resistor-1").unwrap().position();
    assert!((moved.x - origin.x).abs() < 1e-6);
    assert!((moved.y - (origin.y + 16.0)).abs() < 1e-6);
    assert_eq!(c.circ_time(), started);
    assert!((c.pin_voltage("Resistor-1-rPin").unwrap() - mid).abs() < 1e-6);

    c.rotate_cw();
    c.flip_h();
    c.flip_v();
    let turned = c.scene.item_by_id("Resistor-1").unwrap();
    assert!((turned.rotation - 90.0).abs() < 1e-9);
    assert_eq!(turned.hflip, -1);
    assert_eq!(turned.vflip, -1);
    assert_eq!(c.circ_time(), started);
    assert!(c.sim_running());

    let mut wired = two_resistors_wired();
    wired.power_on();
    wired.tick();
    let wire_time = wired.circ_time();
    assert!(wire_time > 1, "tick should advance, t={wire_time}");
    let mid_w = item_of(&wired, Point::new(40.0, 0.0));
    wired.mouse_press(BUTTON_LEFT, mid_w.x, mid_w.y, 0);
    let down = item_of(&wired, Point::new(40.0, 16.0));
    wired.mouse_move(down.x, down.y, BUTTON_LEFT, 0);
    wired.mouse_release(BUTTON_LEFT, down.x, down.y, 0);
    assert_eq!(wired.circ_time(), wire_time);
    assert!(wired.scene.wires()[0].points.iter().any(|p| p.y == 16.0));

    c.tick();
    assert!(c.circ_time() > started);
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
fn new_circuit_stops_running_simulation() {
    let mut c = Canvas::with_demo_divider();
    c.power_on();
    assert!(c.sim_running());
    let change = c.new_circuit();
    assert!(!c.sim_running());
    assert!(change.sim);
}

#[test]
fn load_sim1_stops_running_simulation() {
    let mut c = Canvas::with_demo_divider();
    c.power_on();
    assert!(c.sim_running());
    let empty_sim1 = "<circuit version=\"1.0.0\"/>";
    let change = c.load_circ1(empty_sim1, None).unwrap();
    assert!(!c.sim_running());
    assert!(change.sim);
}

#[test]
fn failed_load_leaves_running_simulation() {
    let mut c = Canvas::with_demo_divider();
    c.power_on();
    c.pause_sim();
    let snapshot = c.to_circ1();
    let mid = c.pin_voltage("Resistor-1-rPin");
    assert!(mid.is_some());

    let err = c.load_circ1("not a circuit", None);
    assert!(err.is_err());
    assert!(c.sim_running());
    assert!(c.sim_paused());
    assert_eq!(c.pin_voltage("Resistor-1-rPin"), mid);
    assert_eq!(c.to_circ1(), snapshot);
}

#[test]
fn pause_and_resume_simulation() {
    let mut c = Canvas::with_demo_divider();
    c.power_on();
    assert!(c.sim_running());
    assert!(!c.sim_paused());

    let ch1 = c.pause_sim();
    assert!(c.sim_running());
    assert!(c.sim_paused());
    assert!(ch1.sim);

    let ch2 = c.resume_sim();
    assert!(c.sim_running());
    assert!(!c.sim_paused());
    assert!(ch2.sim);

    let ch3 = c.toggle_pause();
    assert!(c.sim_paused());
    assert!(ch3.sim);

    c.power_off();
    assert!(!c.sim_running());
    assert!(!c.sim_paused());
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
    let src = c.scene.to_circ1();
    assert!(src.contains("itemtype=\"Probe\""));
    assert!(src.contains("itemtype=\"Voltmeter\""));
    assert!(src.contains("itemtype=\"Ammeter\""));
    assert!(src.contains("itemtype=\"Oscope\""));
    assert!(src.contains("itemtype=\"LogicAnalyzer\""));
    let loaded = crate::canvas::Scene::from_circ1(&src).unwrap();
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
    c.load_circ1(src, Some("/tmp/divider.sim1".into())).unwrap();
    assert_eq!(c.scene.items().len(), 5);
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    assert!((c.pin_voltage("Node-5-0").unwrap() - 2.5).abs() < 1e-6);
    assert!(c.wire_current(&c.scene.wires()[0].id).abs() > 1e-4);
    let saved = c.to_circ1();
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
            circ1: String::new(),
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
    c.load_circ1(&src, Some(path.to_string_lossy().into_owned()))
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
    let saved = c.to_circ1();
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
    let saved = c.scene.to_circ1();
    assert!(saved.contains("itemtype=\"Bjt\""));
    assert!(saved.contains("itemtype=\"Mosfet\""));
    let s2 = Scene::from_circ1(&saved).unwrap();
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
    let saved = c.scene.to_circ1();
    assert!(saved.contains("itemtype=\"OpAmp\""));
    let s2 = Scene::from_circ1(&saved).unwrap();
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
    let saved = c.scene.to_circ1();
    assert!(saved.contains("itemtype=\"Jfet\""));
    assert!(saved.contains("LambdaInv="));
    let s2 = Scene::from_circ1(&saved).unwrap();
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
    let saved = c.scene.to_circ1();
    assert!(saved.contains("itemtype=\"Comparator\""));
    let s2 = Scene::from_circ1(&saved).unwrap();
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
    let saved = c.scene.to_circ1();
    assert!(saved.contains("itemtype=\"VoltReg\""));
    assert!(saved.contains("Voltage="));
    let s2 = Scene::from_circ1(&saved).unwrap();
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
    let saved = c.scene.to_circ1();
    assert!(saved.contains("rotation=\"90\""));
    let s2 = Scene::from_circ1(&saved).unwrap();
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
    assert_eq!(c.scene.items()[1].label, "Resistor-2");
    assert!((c.scene.items()[1].x - 40.0).abs() < 1e-9);
    c.undo();
    assert_eq!(c.scene.items().len(), 1);
}

#[test]
fn copy_paste_preserves_custom_label_and_updates_default_label() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.add_component_at("Resistor", Point::zero());
    c.scene.items_mut()[0].label = "R_bias".into();
    c.scene.select_only(0);
    assert!(c.copy_selection());
    c.paste_at(Point::new(40.0, 0.0));
    assert_eq!(c.scene.items().len(), 2);
    assert_eq!(c.scene.items()[1].id, "Resistor-2");
    assert_eq!(c.scene.items()[1].label, "R_bias");
}

#[test]
fn copy_paste_non_resistor_types_get_unique_ids_and_keep_wires() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    let clk = c.scene.add_clock(0.0, 0.0);
    let gate = c.scene.add_and_gate(80.0, 0.0);
    let bjt = c.scene.add_bjt(0.0, 64.0, false);
    let fv = c.scene.add_default_fixed_volt(80.0, 64.0);
    let zener = c.scene.add_diode(160.0, 64.0, true);
    let sp = c.scene.add_subpackage(240.0, 64.0);
    assert_eq!(clk, "Clock-1");
    assert_eq!(gate, "AndGate-1");
    assert_eq!(bjt, "BJT-1");
    assert_eq!(fv, "Fixed Voltage-1");
    assert_eq!(zener, "Zener-1");
    assert_eq!(sp, "SubPackage-1");

    let p_out = c.scene.pin_scene("Clock-1-outnod").unwrap();
    let p_in = c.scene.pin_scene("AndGate-1-in0").unwrap();
    c.scene
        .connect_pins("Clock-1-outnod", p_out, "AndGate-1-in0", p_in);

    c.scene.select_all();
    assert!(c.copy_selection());
    c.paste_at(Point::new(0.0, 160.0));

    assert_eq!(c.scene.items().len(), 12);
    let pasted_ids: Vec<&str> = c.scene.items()[6..]
        .iter()
        .map(|it| it.id.as_str())
        .collect();
    assert_eq!(
        pasted_ids,
        vec![
            "Clock-2",
            "AndGate-2",
            "BJT-2",
            "Fixed Voltage-2",
            "Zener-2",
            "SubPackage-2",
        ]
    );
    for it in &c.scene.items()[6..] {
        assert_eq!(it.label, it.id);
    }
    match &c.scene.items()[11].kind {
        Part::SubPackage(p) => assert_eq!(p.package.name, "SubPackage-2"),
        _ => panic!("expected SubPackage"),
    }
    assert_eq!(c.scene.wires().len(), 2);
    assert_eq!(c.scene.wires()[1].start_pin, "Clock-2-outnod");
    assert_eq!(c.scene.wires()[1].end_pin.as_deref(), Some("AndGate-2-in0"));
}

#[test]
fn paste_wired_selection_into_empty_scene_keeps_wires() {
    let mut src_canvas = Canvas::empty();
    src_canvas.scene.add_clock(0.0, 0.0);
    src_canvas.scene.add_and_gate(80.0, 0.0);
    let p_out = src_canvas.scene.pin_scene("Clock-1-outnod").unwrap();
    let p_in = src_canvas.scene.pin_scene("AndGate-1-in0").unwrap();
    src_canvas
        .scene
        .connect_pins("Clock-1-outnod", p_out, "AndGate-1-in0", p_in);
    src_canvas.scene.select_all();
    let clip = src_canvas.scene.selection_to_circ1().unwrap();

    let mut dst = Scene::new();
    assert!(dst.paste_circ1(&clip, Point::new(40.0, 40.0)).unwrap());
    assert_eq!(dst.items().len(), 2);
    assert_eq!(dst.wires().len(), 1);
    assert_eq!(dst.wires()[0].start_pin, "Clock-1-outnod");
    assert_eq!(dst.wires()[0].end_pin.as_deref(), Some("AndGate-1-in0"));
}

#[test]
fn copy_paste_qemu_rebinds_uid_and_pins() {
    let mut c = Canvas::empty();
    let id = c
        .scene
        .add_qemu("Esp32", 0.0, 0.0, &crate::subcircuit::SubcSearch::default())
        .unwrap();
    assert_eq!(id, "Esp32-1");
    c.scene.select_only(0);
    assert!(c.copy_selection());
    c.paste_at(Point::new(120.0, 0.0));
    assert_eq!(c.scene.items().len(), 2);
    let pasted = &c.scene.items()[1];
    assert_eq!(pasted.id, "Esp32-2");
    assert_eq!(pasted.label, "Esp32-2");
    match &pasted.kind {
        Part::QemuDevice(q) => {
            assert_eq!(q.qemu.uid, "Esp32-2");
            for pin in &q.qemu.pins {
                assert!(
                    pin.id.starts_with("Esp32-2-"),
                    "unexpected pin id {}",
                    pin.id
                );
            }
        }
        _ => panic!("expected QemuDevice"),
    }
}

#[test]
fn delete_then_add_does_not_collide_ids() {
    let mut c = Canvas::empty();
    let g1 = c.scene.add_and_gate(0.0, 0.0);
    let g2 = c.scene.add_and_gate(80.0, 0.0);
    assert_eq!(g1, "AndGate-1");
    assert_eq!(g2, "AndGate-2");
    c.scene.select_only(0);
    c.delete_selected();
    let g3 = c.scene.add_and_gate(160.0, 0.0);
    assert_eq!(g3, "AndGate-3");
    assert_eq!(c.scene.items().len(), 2);
    assert_ne!(c.scene.items()[0].id, c.scene.items()[1].id);
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
    c.flip_h();
    assert_eq!(c.scene.items()[0].hflip, -1);
    let left = c.scene.pin_scene("Resistor-1-lPin").unwrap();
    assert!((left.x - 16.0).abs() < 1e-6);
    c.rotate_cw();
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
fn splice_node_lands_on_wire_when_clicking_off_center() {
    let mut c = two_resistors_wired();
    c.scene.add_default_resistor(40.0, 48.0);
    // Start a wire from R3's left pin (24, 48).
    let r3_l = item_of(&c, Point::new(24.0, 48.0));
    c.mouse_press(BUTTON_LEFT, r3_l.x, r3_l.y, 0);
    c.mouse_release(BUTTON_LEFT, r3_l.x, r3_l.y, 0);
    assert!(c.scene.drawing());

    // Click at (40, 3.0) — within WIRE_HIT (3.5) of the horizontal wire at y=0,
    // even though to_grid((40, 3.0)) is (40, 4.0) (4px off the wire!).
    let click = item_of(&c, Point::new(40.0, 3.0));
    c.mouse_move(click.x, click.y, 0, 0);
    // Live preview must snap onto the wire at y=0.
    let draft_end = *c
        .scene
        .wires()
        .iter()
        .find(|w| w.drawing())
        .unwrap()
        .points
        .last()
        .unwrap();
    assert_eq!(draft_end, Point::new(40.0, 0.0));

    c.mouse_press(BUTTON_LEFT, click.x, click.y, 0);
    assert!(!c.scene.drawing());
    let node = c
        .scene
        .items()
        .iter()
        .find(|it| it.is_node())
        .expect("junction node created");
    assert_eq!(node.position(), Point::new(40.0, 0.0));
    // All 3 wires connected to the node must meet at (40, 0).
    let node_id = node.id.clone();
    let attached = c.scene.wires().iter().filter(|w| {
        w.start_pin.starts_with(&node_id)
            || w.end_pin
                .as_deref()
                .is_some_and(|e| e.starts_with(&node_id))
    });
    assert_eq!(attached.clone().count(), 3);
    for w in attached {
        if w.start_pin.starts_with(&node_id) {
            assert_eq!(w.points[0], Point::new(40.0, 0.0));
        }
        if w.end_pin
            .as_deref()
            .is_some_and(|e| e.starts_with(&node_id))
        {
            assert_eq!(*w.points.last().unwrap(), Point::new(40.0, 0.0));
        }
    }
}

#[test]
fn splice_node_lands_and_stays_on_angled_wire() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(80.0, 24.0);
    c.scene.add_default_resistor(40.0, 64.0);

    // Draw an angled wire with Shift from R1 rPin (16, 0) to R2 lPin (64, 24).
    let a = item_of(&c, Point::new(16.0, 0.0));
    let b = item_of(&c, Point::new(64.0, 24.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_release(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_move(b.x, b.y, 0, MOD_SHIFT);
    c.mouse_press(BUTTON_LEFT, b.x, b.y, MOD_SHIFT);
    assert_eq!(
        c.scene.wires()[0].points,
        vec![Point::new(16.0, 0.0), Point::new(64.0, 24.0)]
    );

    // Connect R3 lPin (24, 64) onto the angled wire near its midpoint (40, 12) with slight offset.
    let r3 = item_of(&c, Point::new(24.0, 64.0));
    c.mouse_press(BUTTON_LEFT, r3.x, r3.y, 0);
    c.mouse_release(BUTTON_LEFT, r3.x, r3.y, 0);
    let splice_pt = item_of(&c, Point::new(40.0, 13.5));
    c.mouse_move(splice_pt.x, splice_pt.y, 0, 0);
    c.mouse_press(BUTTON_LEFT, splice_pt.x, splice_pt.y, 0);
    assert!(!c.scene.drawing());

    let node = c
        .scene
        .items()
        .iter()
        .find(|it| it.is_node())
        .expect("node created on angled wire");
    let np = node.position();
    // Line equation from (16, 0) to (64, 24): y = 0.5 * (x - 16)
    assert!(
        (np.y - 0.5 * (np.x - 16.0)).abs() < 1e-6,
        "node {np:?} is not on angled wire"
    );
    let node_id = node.id.clone();
    for w in c.scene.wires() {
        if w.start_pin.starts_with(&node_id) {
            assert_eq!(w.points[0], np);
        }
        if w.end_pin
            .as_deref()
            .is_some_and(|e| e.starts_with(&node_id))
        {
            assert_eq!(*w.points.last().unwrap(), np);
        }
    }
}

#[test]
fn draft_wire_snaps_live_to_pin_without_overflow() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(80.0, 0.0);
    let a = item_of(&c, Point::new(16.0, 0.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_release(BUTTON_LEFT, a.x, a.y, 0);

    // Move past R2's lPin (64, 0) onto (68, 2) — still within PIN_HIT_RADIUS (6.0).
    let overshoot = item_of(&c, Point::new(68.0, 2.0));
    c.mouse_move(overshoot.x, overshoot.y, 0, 0);
    let pts = &c.scene.wires()[0].points;
    assert_eq!(
        pts,
        &vec![Point::new(16.0, 0.0), Point::new(64.0, 0.0)],
        "draft wire must snap to pin tip (64, 0) without overflowing onto stem"
    );
}

#[test]
fn key_shift_toggles_draft_wire_angle_live() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    let a = item_of(&c, Point::new(16.0, 0.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_release(BUTTON_LEFT, a.x, a.y, 0);

    let dest = item_of(&c, Point::new(56.0, 24.0));
    c.mouse_move(dest.x, dest.y, 0, 0);
    assert_eq!(c.scene.wires()[0].points.len(), 3);

    // Pressing Shift toggles to angled immediately without mouse move.
    let ch = c.key_press_with_text(KEY_SHIFT, MOD_SHIFT, "", false);
    assert!(ch.wires);
    assert_eq!(
        c.scene.wires()[0].points,
        vec![Point::new(16.0, 0.0), Point::new(56.0, 24.0)]
    );

    // Releasing Shift snaps back to 90° immediately without mouse move.
    let ch = c.key_release(KEY_SHIFT, 0, "", false);
    assert!(ch.wires);
    assert_eq!(c.scene.wires()[0].points.len(), 3);
    for seg in c.scene.wires()[0].points.windows(2) {
        assert!(
            (seg[0].x - seg[1].x).abs() < 1e-6 || (seg[0].y - seg[1].y).abs() < 1e-6,
            "unexpected diagonal after Shift release: {:?}",
            c.scene.wires()[0].points
        );
    }
}

#[test]
fn rotated_component_wire_stays_orthogonal() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(0.0, 80.0);
    c.scene.select_only(0);
    c.rotate_selected(90.0);
    c.scene.select_only(1);
    c.rotate_selected(90.0);

    let r1_r = c.scene.pin_scene("Resistor-1-rPin").unwrap();
    let r2_l = c.scene.pin_scene("Resistor-2-lPin").unwrap();
    assert_eq!(r1_r, Point::new(0.0, 16.0));
    assert_eq!(r2_l, Point::new(0.0, 64.0));

    let a = item_of(&c, r1_r);
    let b = item_of(&c, r2_l);
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_release(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_move(b.x, b.y, 0, 0);
    c.mouse_press(BUTTON_LEFT, b.x, b.y, 0);
    assert!(c.scene.wires()[0].segment_is_vertical(0));

    // Move rotated R1 horizontally by 16px — wire must stay strictly orthogonal.
    let r1_body = item_of(&c, Point::new(0.0, 0.0));
    c.mouse_press(BUTTON_LEFT, r1_body.x, r1_body.y, 0);
    let moved = item_of(&c, Point::new(16.0, 0.0));
    c.mouse_move(moved.x, moved.y, BUTTON_LEFT, 0);
    for seg in c.scene.wires()[0].points.windows(2) {
        assert!(
            (seg[0].x - seg[1].x).abs() < 1e-6 || (seg[0].y - seg[1].y).abs() < 1e-6,
            "rotated component drag produced diagonal: {:?}",
            c.scene.wires()[0].points
        );
    }
    c.mouse_release(BUTTON_LEFT, moved.x, moved.y, 0);
}

#[test]
fn group_drag_keeps_wire_anchored_to_node_live() {
    let mut c = two_resistors_wired();
    c.scene.add_default_resistor(40.0, 48.0);
    let r3_l = item_of(&c, Point::new(24.0, 48.0));
    c.mouse_press(BUTTON_LEFT, r3_l.x, r3_l.y, 0);
    c.mouse_release(BUTTON_LEFT, r3_l.x, r3_l.y, 0);
    let mid = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, mid.x, mid.y, 0);

    let node_pos = c
        .scene
        .items()
        .iter()
        .find(|it| it.is_node())
        .unwrap()
        .position();
    assert_eq!(node_pos, Point::new(40.0, 0.0));

    // Select R1 and the wire from R1 to Node (without selecting Node), then drag R1.
    c.scene.clear_selection();
    c.scene.items_mut()[0].selected = true;
    c.scene.wires_mut()[0].selected = true;
    let r1 = item_of(&c, Point::new(0.0, 0.0));
    c.mouse_press(BUTTON_LEFT, r1.x, r1.y, 0);
    let down = item_of(&c, Point::new(0.0, 16.0));
    c.mouse_move(down.x, down.y, BUTTON_LEFT, 0);
    // Live during mouse_move (before mouse_release!), wire must stay anchored at Node (40, 0).
    assert_eq!(*c.scene.wires()[0].points.last().unwrap(), node_pos);
    c.mouse_release(BUTTON_LEFT, down.x, down.y, 0);
    assert_eq!(*c.scene.wires()[0].points.last().unwrap(), node_pos);
}

#[test]
fn drag_angled_wire_bends_and_keeps_pins_anchored() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(80.0, 24.0);
    let a = item_of(&c, Point::new(16.0, 0.0));
    let b = item_of(&c, Point::new(64.0, 24.0));
    c.mouse_press(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_release(BUTTON_LEFT, a.x, a.y, 0);
    c.mouse_move(b.x, b.y, 0, MOD_SHIFT);
    c.mouse_press(BUTTON_LEFT, b.x, b.y, MOD_SHIFT);
    assert_eq!(
        c.scene.wires()[0].points,
        vec![Point::new(16.0, 0.0), Point::new(64.0, 24.0)]
    );

    // Grab the angled wire near its midpoint and drag down 16px.
    let mid = item_of(&c, Point::new(40.0, 12.0));
    c.mouse_press(BUTTON_LEFT, mid.x, mid.y, 0);
    let dragged = item_of(&c, Point::new(40.0, 28.0));
    c.mouse_move(dragged.x, dragged.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, dragged.x, dragged.y, 0);
    let pts = &c.scene.wires()[0].points;
    assert_eq!(
        pts.len(),
        3,
        "angled wire should bend into 2 segments: {pts:?}"
    );
    assert_eq!(pts[0], Point::new(16.0, 0.0));
    assert_eq!(pts[2], Point::new(64.0, 24.0));
    assert_eq!(pts[1], Point::new(40.0, 28.0));
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
    let src = c.to_circ1();
    assert!(src.contains("width=\"1600\""));
    assert!(src.contains("height=\"900\""));
    assert!(src.contains("animate=\"1\""));
    assert!(src.contains("anicurr=\"0\""));
    assert!(src.contains("ansi=\"1\""));
    assert!(src.contains("NLsteps=\"42\""));
    assert!(src.contains("reaStep=\"2000\""));
    let mut loaded = Canvas::empty();
    loaded.load_circ1(&src, None).unwrap();
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
    c.load_circ1(sim1, Some(path.to_string_lossy().into_owned()))
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
    let saved = c.to_circ1();
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
    c.load_circ1(
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
    let saved = c.to_circ1();
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
    c.load_circ1(
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
fn click_on_component_over_wire_selects_component() {
    let mut c = ready();
    let p1 = c.scene.items()[0].pins()[1].id.clone();
    let p2 = c.scene.items()[1].pins()[0].id.clone();
    let p1_pos = c.scene.pin_scene(&p1).unwrap();
    let p2_pos = c.scene.pin_scene(&p2).unwrap();
    c.scene.connect_pins(&p1, p1_pos, &p2, p2_pos);

    // Route the wire through the first resistor body. The stroke is painted
    // under the part, so the part is what the cursor is on.
    let body_center = {
        let it = &c.scene.items()[0];
        let b = it.body_rect();
        Point::new(it.x + b.x + b.w / 2.0, it.y + b.y + b.h / 2.0)
    };
    c.scene.wires_mut()[0].points = vec![p1_pos, body_center, p2_pos];

    let click = item_of(&c, body_center);
    c.mouse_move(click.x, click.y, 0, 0);
    assert_eq!(c.cursor(), CursorKind::Arrow);
    let tip = c.hover_tooltip(body_center).unwrap_or_default();
    assert!(
        tip.contains("Component"),
        "hover on the part should describe the part, got {tip}"
    );

    c.mouse_press(BUTTON_LEFT, click.x, click.y, 0);
    assert!(c.scene.items()[0].selected);
    assert!(!c.scene.wires()[0].selected);

    c.mouse_press(BUTTON_RIGHT, click.x, click.y, 0);
    assert!(c.scene.items()[0].selected);
    assert!(!c.scene.wires()[0].selected);
    assert!(!c.has_pin_hit());

    // The same wire is still selectable where the part does not cover it.
    let exposed = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, exposed.x, exposed.y, 0);
    assert!(c.scene.wires()[0].selected);
    assert!(!c.scene.items()[0].selected);
}

#[test]
fn top_part_covers_pin_and_label_underneath() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);
    c.scene.add_default_resistor(0.0, 0.0);
    c.scene.add_default_resistor(16.0, 0.0);

    // The upper body is painted over the lower part's right pin.
    let click = item_of(&c, Point::new(16.0, 0.0));
    c.mouse_press(BUTTON_LEFT, click.x, click.y, 0);
    assert!(!c.scene.drawing());
    assert!(c.scene.wires().is_empty());
    assert!(c.scene.items()[1].selected);
    assert!(!c.scene.items()[0].selected);

    c.scene.clear_selection();
    {
        let it = &mut c.scene.items_mut()[0];
        it.show_id = true;
        it.custom_label_pos = true;
        it.label_x = 16.0;
        it.label_y = 0.0;
    }
    c.mouse_press(BUTTON_LEFT, click.x, click.y, 0);
    assert!(c.scene.items()[1].selected);
    assert!(!c.scene.items()[0].selected);
}

#[test]
fn draft_click_on_part_does_not_splice_wire_underneath() {
    let mut c = two_resistors_wired();
    c.scene.add_default_resistor(40.0, 0.0);
    let start = item_of(&c, Point::new(-16.0, 0.0));
    c.mouse_press(BUTTON_LEFT, start.x, start.y, 0);
    assert!(c.scene.drawing());
    let before = c.scene.wires().len();
    let covered = item_of(&c, Point::new(40.0, 0.0));
    c.mouse_press(BUTTON_LEFT, covered.x, covered.y, 0);
    assert!(c.scene.drawing());
    assert_eq!(c.scene.wires().len(), before);
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
fn test_label_cardinal_detents_hysteresis_and_reset() {
    let mut c = Canvas::empty();
    c.scene_mut().add_default_resistor(100.0, 100.0);
    c.scene_mut().items_mut()[0].show_id = true;
    c.scene_mut().items_mut()[0].show_val = true;

    let res = &c.scene().items()[0];
    let uid = res.id.clone();
    let top_slot = res.label_detent_slot(false, LabelDetent::Top);
    let left_slot = res.label_detent_slot(false, LabelDetent::Left);
    let right_slot = res.label_detent_slot(false, LabelDetent::Right);
    let bottom_slot = res.label_detent_slot(false, LabelDetent::Bottom);
    let val_bottom_slot = res.label_detent_slot(true, LabelDetent::Bottom);
    let val_right_slot = res.label_detent_slot(true, LabelDetent::Right);

    assert!(top_slot.is_default);
    assert!(!left_slot.is_default);
    assert!(val_bottom_slot.is_default);
    assert!(!val_right_slot.is_default);

    // Start dragging ID label from default Top detent.
    let grab_scene = res.map_local(Point::new(top_slot.anchor.x + 4.0, top_slot.anchor.y + 4.0));
    let grab_item = c.viewport().map_from_circuit(grab_scene);
    c.mouse_press(BUTTON_LEFT, grab_item.x, grab_item.y, 0);
    assert_eq!(c.active_label_detent(), None);

    // Small drag within release radius stays magnetically locked on Top and shows detent guide.
    c.mouse_move(grab_item.x + 3.0, grab_item.y - 2.0, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_detent(), Some((0, false, LabelDetent::Top)));
    assert!(!c.scene().items()[0].custom_label_pos);
    assert_eq!(
        c.scene().items()[0].label_pos(),
        (top_slot.anchor.x, top_slot.anchor.y)
    );

    // Releasing while never leaving the starting detent does not create an undo step.
    c.mouse_release(BUTTON_LEFT, grab_item.x + 3.0, grab_item.y - 2.0, 0);
    assert_eq!(c.active_label_detent(), None);
    assert!(!c.can_undo());

    // Drag past release radius into free space, then onto Left, Right, Bottom, and back to Top.
    c.mouse_press(BUTTON_LEFT, grab_item.x, grab_item.y, 0);
    c.mouse_move(grab_item.x + 30.0, grab_item.y - 30.0, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_detent(), None);
    assert!(c.scene().items()[0].custom_label_pos);

    for slot in [left_slot, right_slot, bottom_slot] {
        let delta = slot.anchor.sub(top_slot.anchor);
        let near_scene = Point::new(grab_scene.x + delta.x + 2.0, grab_scene.y + delta.y - 1.0);
        let near_item = c.viewport().map_from_circuit(near_scene);
        c.mouse_move(near_item.x, near_item.y, BUTTON_LEFT, 0);
        assert_eq!(c.active_label_detent(), Some((0, false, slot.kind)));
        let (lx, ly) = c.scene().items()[0].label_pos();
        assert!((lx - slot.anchor.x).abs() < 1e-6);
        assert!((ly - slot.anchor.y).abs() < 1e-6);
        assert!(c.scene().items()[0].custom_label_pos);
    }

    // Drag back near Top (default): snaps to Top and restores dynamic centering (custom_label_pos = false).
    let near_top = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x + 2.0, grab_scene.y - 2.0));
    c.mouse_move(near_top.x, near_top.y, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_detent(), Some((0, false, LabelDetent::Top)));
    assert!(!c.scene().items()[0].custom_label_pos);

    // Move to Right and release so it has a custom position for Reset Position testing.
    let delta_right = right_slot.anchor.sub(top_slot.anchor);
    let right_item = c.viewport().map_from_circuit(Point::new(
        grab_scene.x + delta_right.x,
        grab_scene.y + delta_right.y,
    ));
    c.mouse_move(right_item.x, right_item.y, BUTTON_LEFT, 0);
    c.mouse_release(BUTTON_LEFT, right_item.x, right_item.y, 0);
    assert_eq!(c.active_label_detent(), None);
    assert!(c.scene().items()[0].custom_label_pos);
    assert!(c.can_undo());

    // Right-click reports custom pos, and reset_item_label_pos restores dynamic default with undo.
    let right_hit_scene = c.scene().items()[0].map_local(Point::new(
        right_slot.anchor.x + 4.0,
        right_slot.anchor.y + 4.0,
    ));
    let right_hit_item = c.viewport().map_from_circuit(right_hit_scene);
    c.mouse_press(BUTTON_RIGHT, right_hit_item.x, right_hit_item.y, 0);
    assert!(c.has_label_hit());
    assert!(!c.hit_label_is_val());
    assert!(c.hit_label_has_custom_pos());

    let ch = c.reset_item_label_pos(&uid);
    assert!(ch.items);
    assert!(ch.history);
    assert!(!c.scene().items()[0].custom_label_pos);
    assert!(!c.hit_label_has_custom_pos());
    assert_eq!(
        c.scene().items()[0].label_pos(),
        (top_slot.anchor.x, top_slot.anchor.y)
    );
    // Resetting an already-default label is a no-op.
    assert!(!c.reset_item_label_pos(&uid).history);

    // Value label: snap to Right, then snap back to Bottom (default) clearing custom_val_pos,
    // and test reset_item_val_label_pos.
    let val_grab_scene = c.scene().items()[0].map_local(Point::new(
        val_bottom_slot.anchor.x + 4.0,
        val_bottom_slot.anchor.y + 4.0,
    ));
    let val_grab_item = c.viewport().map_from_circuit(val_grab_scene);
    c.mouse_press(BUTTON_LEFT, val_grab_item.x, val_grab_item.y, 0);
    let val_delta_right = val_right_slot.anchor.sub(val_bottom_slot.anchor);
    let val_right_item = c.viewport().map_from_circuit(Point::new(
        val_grab_scene.x + val_delta_right.x + 1.0,
        val_grab_scene.y + val_delta_right.y - 1.0,
    ));
    c.mouse_move(val_right_item.x, val_right_item.y, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_detent(), Some((0, true, LabelDetent::Right)));
    assert!(c.scene().items()[0].custom_val_pos);
    c.mouse_release(BUTTON_LEFT, val_right_item.x, val_right_item.y, 0);

    let ch = c.reset_item_val_label_pos(&uid);
    assert!(ch.history);
    assert!(!c.scene().items()[0].custom_val_pos);
    assert_eq!(
        c.scene().items()[0].val_pos(),
        (val_bottom_slot.anchor.x, val_bottom_slot.anchor.y)
    );
}

#[test]
fn test_esp32_label_detents_clear_pins_and_crossing_guides_activate_per_axis() {
    let mut c = Canvas::empty();
    assert!(c.scene_add_component_spec("Esp32,QemuDevice", Point::new(100.0, 100.0)));
    c.scene_mut().items_mut()[0].show_id = true;
    c.scene_mut().items_mut()[0].show_val = true;

    let esp = &c.scene().items()[0];
    let body = esp.body_rect();
    let clearance = esp.label_clearance_rect();

    // ESP32 has pins sticking out all 4 sides (-8 and 128 on a 120x120 body),
    // so clearance must extend beyond body_rect on all 4 sides including pin tip rings.
    assert!(clearance.left() <= body.left() - 10.0);
    assert!(clearance.right() >= body.right() + 10.0);
    assert!(clearance.top() <= body.top() - 10.0);
    assert!(clearance.bottom() >= body.bottom() + 10.0);

    // All 4 cardinal slots must sit strictly outside the pin clearance rect.
    for kind in [
        LabelDetent::Top,
        LabelDetent::Bottom,
        LabelDetent::Left,
        LabelDetent::Right,
    ] {
        let slot = esp.label_detent_slot(false, kind);
        let lbox = esp.label_local_box(false, slot.anchor);
        match kind {
            LabelDetent::Top => assert!(lbox.bottom() < clearance.top()),
            LabelDetent::Bottom => assert!(lbox.top() > clearance.bottom()),
            LabelDetent::Left => assert!(lbox.right() < clearance.left()),
            LabelDetent::Right => assert!(lbox.left() > clearance.right()),
        }
    }

    // Drag ID label straight up 40px above the Top detent: Y breaks free while X stays
    // locked to the vertical centerline, showing only the X guide.
    let top_slot = esp.label_detent_slot(false, LabelDetent::Top);
    let grab_scene = esp.map_local(Point::new(top_slot.anchor.x + 4.0, top_slot.anchor.y + 4.0));
    let grab_item = c.viewport().map_from_circuit(grab_scene);
    c.mouse_press(BUTTON_LEFT, grab_item.x, grab_item.y, 0);

    let high_center = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x + 2.0, grab_scene.y - 40.0));
    c.mouse_move(high_center.x, high_center.y, BUTTON_LEFT, 0);
    assert_eq!(
        c.active_label_guides(),
        Some(ActiveLabelGuides {
            index: 0,
            is_val: false,
            x: Some(LabelAxisX::Center),
            y: None,
        })
    );
    assert_eq!(c.active_label_detent(), None);
    let (lx, ly) = c.scene().items()[0].label_pos();
    assert!((lx - top_slot.anchor.x).abs() < 1e-6);
    assert!((ly - (top_slot.anchor.y - 40.0)).abs() <= 1.0);

    // Drag far to the right and cross the horizontal centerline: Y locks to Center while X is free.
    let axes = c.scene().items()[0].label_detent_axes(false);
    let dy_to_cy = axes.y_center - top_slot.anchor.y;
    let far_right_center = c.viewport().map_from_circuit(Point::new(
        grab_scene.x + 140.0,
        grab_scene.y + dy_to_cy + 1.5,
    ));
    c.mouse_move(far_right_center.x, far_right_center.y, BUTTON_LEFT, 0);
    assert_eq!(
        c.active_label_guides(),
        Some(ActiveLabelGuides {
            index: 0,
            is_val: false,
            x: None,
            y: Some(LabelAxisY::Center),
        })
    );
    assert_eq!(c.active_label_detent(), None);
    let (_, ly2) = c.scene().items()[0].label_pos();
    assert!((ly2 - axes.y_center).abs() < 1e-6);

    c.mouse_release(BUTTON_LEFT, far_right_center.x, far_right_center.y, 0);
    assert_eq!(c.active_label_guides(), None);
}

#[test]
fn test_label_hitbox_matches_text_bounds_and_does_not_block_body() {
    let mut c = Canvas::empty();
    c.scene_mut().add_default_resistor(100.0, 100.0);
    c.scene_mut().items_mut()[0].show_id = true;
    c.scene_mut().items_mut()[0].show_val = true;

    let res = &c.scene().items()[0];
    let body = res.body_rect();
    // Every vertical point inside the resistor body must hit the body, never its own labels.
    for dy in [body.top() + 0.5, 0.0, body.bottom() - 0.5] {
        let pt = res.map_local(Point::new(0.0, dy));
        assert_eq!(res.hit_label(pt), None);
        assert!(matches!(
            c.scene().hit_top(pt),
            Some(HitTarget::Body { index: 0 })
        ));
    }

    // Points just outside the ID label text box must not hit the label.
    let (lx, ly) = res.label_pos();
    let tw = text::text_width(&res.label, 9.0, false);
    assert_eq!(
        res.hit_label(res.map_local(Point::new(lx + tw * 0.5, ly + 5.0))),
        Some(false)
    );
    assert_eq!(
        res.hit_label(res.map_local(Point::new(lx - 2.5, ly + 5.0))),
        None
    );
    assert_eq!(
        res.hit_label(res.map_local(Point::new(lx + tw + 2.5, ly + 5.0))),
        None
    );
    assert_eq!(
        res.hit_label(res.map_local(Point::new(lx + tw * 0.5, ly - 2.0))),
        None
    );
    assert_eq!(
        res.hit_label(res.map_local(Point::new(lx + tw * 0.5, ly + 12.0))),
        None
    );
}

#[test]
fn test_label_guide_catches_fast_crossing_and_direction_reversal() {
    let mut c = Canvas::empty();
    c.scene_mut().add_default_resistor(100.0, 100.0);
    c.scene_mut().items_mut()[0].show_id = true;

    let res = &c.scene().items()[0];
    let top_slot = res.label_detent_slot(false, LabelDetent::Top);
    let grab_scene = res.map_local(Point::new(top_slot.anchor.x + 4.0, top_slot.anchor.y + 4.0));
    let grab_item = c.viewport().map_from_circuit(grab_scene);

    c.mouse_press(BUTTON_LEFT, grab_item.x, grab_item.y, 0);

    // 1. Pull past release_r (8.5px) to +9.5px (still < rearm_r = 12px) so Center X unlocks
    //    and enters suppressed state, then immediately reverse back to Center X: must re-capture.
    let pulled = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x + 9.5, grab_scene.y - 30.0));
    c.mouse_move(pulled.x, pulled.y, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_guides(), None);

    let reversed = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x + 1.0, grab_scene.y - 30.0));
    c.mouse_move(reversed.x, reversed.y, BUTTON_LEFT, 0);
    assert_eq!(
        c.active_label_guides(),
        Some(ActiveLabelGuides {
            index: 0,
            is_val: false,
            x: Some(LabelAxisX::Center),
            y: None,
        })
    );

    // 2. Move far left (-20px) so Center X is fully released, then jump across Center X in a
    //    single fast step from -7px to +7px (outside capture_r = 5px, within release_r = 8.5px):
    //    crossing detection must catch Center X instead of skipping over it.
    let far_left = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x - 20.0, grab_scene.y - 30.0));
    c.mouse_move(far_left.x, far_left.y, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_guides(), None);

    let approach = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x - 7.0, grab_scene.y - 30.0));
    c.mouse_move(approach.x, approach.y, BUTTON_LEFT, 0);
    assert_eq!(c.active_label_guides(), None);

    let fast_cross = c
        .viewport()
        .map_from_circuit(Point::new(grab_scene.x + 7.0, grab_scene.y - 30.0));
    c.mouse_move(fast_cross.x, fast_cross.y, BUTTON_LEFT, 0);
    assert_eq!(
        c.active_label_guides(),
        Some(ActiveLabelGuides {
            index: 0,
            is_val: false,
            x: Some(LabelAxisX::Center),
            y: None,
        })
    );
    assert!((c.scene().items()[0].label_pos().0 - top_slot.anchor.x).abs() < 1e-6);

    c.mouse_release(BUTTON_LEFT, fast_cross.x, fast_cross.y, 0);
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
        .load_circ1(sim1, Some(path.to_string_lossy().into_owned()))
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
fn test_canvas_create_firmware_preset_and_assign_links_source() {
    let dir = std::env::temp_dir().join(format!("cs-canvas-create-fw-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let circ = r#"<circuit version="1.0.0">
<item itemtype="QemuDevice" CircId="ESP32-1" label="ESP32-1" device="ESP32" Pos="0,0" />
</circuit>"#;
    let circ_path = dir.join("demo.circ1");
    std::fs::write(&circ_path, circ).unwrap();

    let mut canvas = Canvas::empty();
    canvas
        .load_circ1(circ, Some(circ_path.to_string_lossy().into_owned()))
        .unwrap();
    canvas.select_only(0);

    // Initially no firmware is set.
    assert!(!canvas.selected_has_firmware());

    // Preset resolves to Arduino + DOIT ESP32 DEVKIT V1.
    let preset = canvas.firmware_preset_for("ESP32-1").unwrap();
    assert_eq!(preset.compiler, "Arduino");
    assert_eq!(preset.board, "DOIT ESP32 DEVKIT V1");
    assert_eq!(preset.custom_board, "esp32:esp32:esp32doit-devkit-v1");
    assert_eq!(preset.extension, "ino");

    // Scaffold and assign to the MCU.
    let created = crate::editor::create_firmware_scaffold(&dir, "blink", &preset).unwrap();
    let ch =
        canvas.assign_firmware_path("ESP32-1", &created.linked_firmware_path.to_string_lossy());
    assert!(ch.items);
    assert!(ch.history);
    assert!(canvas.selected_has_firmware());
    assert_eq!(
        canvas.firmware_source_for("ESP32-1"),
        Some(created.source_path.clone())
    );

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
    let mut c = Canvas::empty();
    c.scene = Scene::from_circ1(sim1).unwrap();
    let ps_per_sec = c.scene().settings().ps_per_sec();
    c.power_on();
    c.tick_with_fps(20);
    let time_at_20fps = c.circ_time();
    assert_eq!(time_at_20fps, 1 + ps_per_sec / 20);

    // Test with 50 FPS
    let mut c2 = Canvas::empty();
    c2.scene = Scene::from_circ1(sim1).unwrap();
    c2.power_on();
    c2.tick_with_fps(50);
    let time_at_50fps = c2.circ_time();
    assert_eq!(time_at_50fps, 1 + ps_per_sec / 50);
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
    assert!(c.to_circ1().contains("Wiper=\"0.75\""), "{}", c.to_circ1());
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
    c.load_circ1(src, Some("/tmp/divider.sim1".into())).unwrap();

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
    c.load_circ1(src, Some("/tmp/divider.sim1".into())).unwrap();

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
        component_id_text: Color::rgb(0, 0, 139),
        pin_open_high: Color::rgb(0, 0, 0),
        meter_display: Color::rgb(0, 0, 0),
        msg_warn: Color::rgb(0, 0, 0),
        msg_error: Color::rgb(0, 0, 0),
        link_highlight: Color::rgb(37, 130, 235),
        exposed_highlight: Color::rgb(235, 190, 20),
        link_badge_text: Color::rgb(255, 255, 255),
        chip_label_text: Color::rgb(160, 160, 180),
    };

    let pctx = PaintCtx {
        canvas: &c,
        pal: &pal,
        scale: 1.0,
        item_id: &nid,
        on_board: false,
        active_item_id: None,
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
    let scene = Scene::from_circ1(sim1).expect("valid sim1");
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
        "higher Current Flow Animation Speed slider must move chevrons faster; slow={slow} fast={fast_step}"
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
    c.load_circ1(sim1, Some("/tmp/test_led_glow_dirty.sim1".into()))
        .unwrap();
    c.set_view_size(200.0, 200.0);
    c.zoom_one();
    let led_pos = c.scene.item_by_id("Led-1").expect("Led-1").position();
    c.set_center(led_pos.x, led_pos.y);
    let pal = Palette::dark();
    // Live canvas rasters with a transparent pixmap (GPU grid underneath).
    let Some(unlit) = render_viewport(&c, &pal, 200, 200, 1.0, false) else {
        eprintln!("skipping: vello renderer unavailable");
        return;
    };
    c.power_on();
    assert!(c.sim_error().is_none(), "{:?}", c.sim_error());
    for _ in 0..10 {
        c.tick();
    }
    let Some(lit) = render_viewport(&c, &pal, 200, 200, 1.0, false) else {
        eprintln!("skipping: vello renderer unavailable");
        return;
    };
    let led = c.scene.item_by_id("Led-1").expect("Led-1");
    let patch = super::dirty::item_dirty_rect(led);
    let Some(mut patched) = render_viewport(&c, &pal, 200, 200, 1.0, false) else {
        eprintln!("skipping: vello renderer unavailable");
        return;
    };
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

#[test]
fn right_click_subpackage_pin_notifies_items_and_resolves_ids() {
    let mut c = Canvas::empty();
    let uid = c.scene_mut().add_subpackage(100.0, 100.0);
    c.set_package_footprint_dip(&uid, "DIP-8", 8, Some(4));
    // Rename pin "1" to an ID containing a hyphen to verify item/local ID splitting.
    let mut pin = c.get_package_pin(&uid, "1").unwrap().clone();
    pin.id = "IN-1".to_string();
    pin.label = "IN-1".to_string();
    c.update_package_pin(&uid, "1", pin);

    let full_pin_id = format!("{uid}-IN-1");
    let pin_scene = c.scene().pin_scene(&full_pin_id).expect("pin scene pos");
    let pin_item = c.viewport().map_from_circuit(pin_scene);

    let ch = c.mouse_press(crate::canvas::BUTTON_RIGHT, pin_item.x, pin_item.y, 0);
    assert!(
        ch.items,
        "right-clicking a pin must set items=true so QML pinMenu properties refresh"
    );
    assert!(c.has_pin_hit());
    assert_eq!(c.hit_pin_id(), Some(full_pin_id.as_str()));
    assert_eq!(c.hit_pin_item_id(), Some(uid.as_str()));
    assert_eq!(c.hit_pin_local_id(), Some("IN-1"));

    // Right-clicking empty space clears hit_pin and notifies again.
    let empty_item = c.viewport().map_from_circuit(Point::new(-200.0, -200.0));
    let clear_ch = c.mouse_press(crate::canvas::BUTTON_RIGHT, empty_item.x, empty_item.y, 0);
    assert!(clear_ch.items);
    assert!(!c.has_pin_hit());
    assert_eq!(c.hit_pin_id(), None);
    assert_eq!(c.hit_pin_item_id(), None);
    assert_eq!(c.hit_pin_local_id(), None);
}

#[test]
fn can_insert_by_key_yields_to_component_keys_and_modal_modes() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // Neutral canvas allows bare-key component insertion.
    assert!(c.can_insert_by_key(KEY_R, 0, "r"));

    // A Push button owning 'r' preempts component insertion for 'r', but not 'c'.
    let push_id = c.scene.add_push(0.0, 0.0);
    c.scene
        .item_by_id_mut(&push_id)
        .unwrap()
        .set_prop_text("Key", "r");
    assert!(!c.can_insert_by_key(KEY_R, 0, "r"));
    assert!(c.can_insert_by_key(KEY_C, 0, "c"));

    // Wire drawing blocks component insertion for any key.
    let r_id = c.scene.add_default_resistor(80.0, 0.0);
    let pin_id = format!("{r_id}-lPin");
    let pin_scene = c.scene.pin_scene(&pin_id).unwrap();
    let pin_item = item_of(&c, pin_scene);
    c.mouse_press(BUTTON_LEFT, pin_item.x, pin_item.y, 0);
    assert!(c.scene.drawing());
    assert!(!c.can_insert_by_key(KEY_C, 0, "c"));
    c.key_press(KEY_ESCAPE, 0);
    assert!(!c.scene.drawing());
    assert!(c.can_insert_by_key(KEY_C, 0, "c"));
}

#[test]
fn tunnel_group_highlight_and_paint() {
    use crate::canvas::draw::{Align, Draw};

    let mut c = Canvas::empty();
    let t_clk_1 = c.scene.add_tunnel(0.0, 0.0);
    let t_clk_2 = c.scene.add_tunnel(80.0, 0.0);
    let t_vcc = c.scene.add_tunnel(160.0, 0.0);
    let t_unnamed = c.scene.add_tunnel(240.0, 0.0);

    c.scene
        .item_by_id_mut(&t_clk_1)
        .unwrap()
        .set_prop_text("Name", "CLK");
    c.scene
        .item_by_id_mut(&t_clk_2)
        .unwrap()
        .set_prop_text("Name", "CLK");
    c.scene
        .item_by_id_mut(&t_vcc)
        .unwrap()
        .set_prop_text("Name", "VCC");

    // Tunnels start unhighlighted.
    assert!(!c.scene.item_by_id(&t_clk_1).unwrap().tunnel_visible());

    // Highlighting an unnamed tunnel is a no-op.
    let ch_unnamed = c.set_tunnel_group_visible(&t_unnamed, true);
    assert!(!ch_unnamed.items);
    assert!(!c.scene.item_by_id(&t_unnamed).unwrap().tunnel_visible());

    // Show CLK group: highlights both CLK tunnels, does not push undo.
    let undo_before = c.history.undo.len();
    let ch_show = c.set_tunnel_group_visible(&t_clk_1, true);
    assert!(ch_show.items);
    assert!(!ch_show.history);
    assert_eq!(c.history.undo.len(), undo_before);
    assert!(c.scene.item_by_id(&t_clk_1).unwrap().tunnel_visible());
    assert!(c.scene.item_by_id(&t_clk_2).unwrap().tunnel_visible());
    assert!(!c.scene.item_by_id(&t_vcc).unwrap().tunnel_visible());

    // Verify Tunnel::paint emits a highlight fill_rect only when `show` is true.
    struct RectCounter {
        rects: Vec<(f64, f64, f64, f64, Color)>,
    }
    impl Draw for RectCounter {
        fn push(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64) {}
        fn pop(&mut self) {}
        fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color) {
            self.rects.push((x, y, w, h, c));
        }
        fn stroke_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_round_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color) {}
        fn stroke_round_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_circle(&mut self, _: f64, _: f64, _: f64, _: Color) {}
        fn stroke_circle(&mut self, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
        fn stroke_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn line(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn polyline(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
        fn fill_poly(&mut self, _: &[[f64; 2]], _: Color) {}
        fn stroke_poly(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
        fn text(&mut self, _: f64, _: f64, _: &str, _: f64, _: Color, _: Align) {}
        fn arc(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn grid_dots(&mut self, _: Rect, _: i32, _: Color) {}
    }

    let pal = Palette::light();
    let pctx = PaintCtx {
        canvas: &c,
        pal: &pal,
        scale: 1.0,
        item_id: &t_clk_1,
        on_board: false,
        active_item_id: None,
    };
    let mut shown_draw = RectCounter { rects: Vec::new() };
    c.scene
        .item_by_id(&t_clk_1)
        .unwrap()
        .kind
        .paint(&mut shown_draw, &pctx);
    assert_eq!(shown_draw.rects.len(), 1);
    assert_eq!(shown_draw.rects[0].4, pal.component_id_text.fade(0.4));

    let mut hidden_draw = RectCounter { rects: Vec::new() };
    c.scene
        .item_by_id(&t_vcc)
        .unwrap()
        .kind
        .paint(&mut hidden_draw, &pctx);
    assert!(hidden_draw.rects.is_empty());

    // Showing another group clears the previous group's highlight.
    let _ = c.set_tunnel_group_visible(&t_vcc, true);
    assert!(!c.scene.item_by_id(&t_clk_1).unwrap().tunnel_visible());
    assert!(!c.scene.item_by_id(&t_clk_2).unwrap().tunnel_visible());
    assert!(c.scene.item_by_id(&t_vcc).unwrap().tunnel_visible());

    // Hide clears that group.
    let _ = c.set_tunnel_group_visible(&t_vcc, false);
    assert!(!c.scene.item_by_id(&t_vcc).unwrap().tunnel_visible());
}

#[test]
fn tunnel_group_rename_and_info() {
    let mut c = Canvas::empty();
    let t1 = c.scene.add_tunnel(0.0, 0.0);
    let t2 = c.scene.add_tunnel(80.0, 0.0);
    let t_vcc = c.scene.add_tunnel(160.0, 0.0);
    let t_blank_1 = c.scene.add_tunnel(240.0, 0.0);
    let t_blank_2 = c.scene.add_tunnel(320.0, 0.0);

    c.scene
        .item_by_id_mut(&t1)
        .unwrap()
        .set_prop_text("Name", "CLK");
    c.scene
        .item_by_id_mut(&t2)
        .unwrap()
        .set_prop_text("Name", "CLK");
    c.scene
        .item_by_id_mut(&t_vcc)
        .unwrap()
        .set_prop_text("Name", "VCC");

    // Inspect group info for a named group and an unnamed tunnel.
    let info = c.tunnel_group_info(&t1).expect("tunnel info");
    assert_eq!(info.name, "CLK");
    assert_eq!(info.group_count, 2);
    assert_eq!(info.existing_groups.get("CLK"), Some(&2));
    assert_eq!(info.existing_groups.get("VCC"), Some(&1));

    let blank_info = c.tunnel_group_info(&t_blank_1).expect("blank tunnel info");
    assert_eq!(blank_info.name, "");
    assert_eq!(blank_info.group_count, 1);

    // Renaming an unnamed tunnel only renames that single tunnel, not other unnamed tunnels.
    let _ = c.rename_tunnel_group(&t_blank_1, "  RST  ");
    assert_eq!(c.scene.item_by_id(&t_blank_1).unwrap().tunnel_name(), "RST");
    assert_eq!(c.scene.item_by_id(&t_blank_2).unwrap().tunnel_name(), "");

    // No-op rename (same name or whitespace-only) does not push undo.
    let undo_len = c.history.undo.len();
    let noop = c.rename_tunnel_group(&t1, "CLK");
    assert!(!noop.history);
    assert_eq!(c.history.undo.len(), undo_len);
    let blank_noop = c.rename_tunnel_group(&t1, "   ");
    assert!(!blank_noop.history);
    assert_eq!(c.history.undo.len(), undo_len);

    // Highlight CLK, then rename/merge into VCC: all three become VCC and share highlight state.
    let _ = c.set_tunnel_group_visible(&t1, true);
    let ch = c.rename_tunnel_group(&t1, "VCC");
    assert!(ch.items);
    assert!(ch.history);
    assert_eq!(c.scene.item_by_id(&t1).unwrap().tunnel_name(), "VCC");
    assert_eq!(c.scene.item_by_id(&t2).unwrap().tunnel_name(), "VCC");
    assert_eq!(c.scene.item_by_id(&t_vcc).unwrap().tunnel_name(), "VCC");
    assert!(c.scene.item_by_id(&t1).unwrap().tunnel_visible());
    assert!(c.scene.item_by_id(&t2).unwrap().tunnel_visible());
    assert!(c.scene.item_by_id(&t_vcc).unwrap().tunnel_visible());
}

#[test]
fn set_show_grid_and_component_rect_mark_full_dirty() {
    let mut c = Canvas::empty();
    let _ = c.take_dirty();

    assert!(!c.set_show_grid(true));
    assert!(c.take_dirty().is_empty());

    assert!(c.set_show_grid(false));
    assert!(c.take_dirty().full);

    assert!(c.set_show_grid(true));
    assert!(c.take_dirty().full);

    assert!(!c.set_show_component_rect(false));
    assert!(c.take_dirty().is_empty());

    assert!(c.set_show_component_rect(true));
    assert!(c.take_dirty().full);
}

#[test]
fn paste_dirties_previously_selected_items() {
    let mut c = Canvas::empty();
    let r1 = c.scene.add_default_resistor(0.0, 0.0);
    c.scene.select_only(0);
    assert!(c.copy_selection());
    let _ = c.take_dirty();

    let ch = c.paste_at(Point::new(160.0, 0.0));
    assert!(ch.items);
    let dirty = c.take_dirty();
    assert!(
        dirty.items.contains(&r1),
        "deselected source item must be marked dirty on paste"
    );
    assert_eq!(c.scene.items().len(), 2);
    let r2 = c.scene.items()[1].id.clone();
    assert!(
        dirty.items.contains(&r2),
        "newly pasted item must be marked dirty on paste"
    );
}

#[test]
fn center_select_overflowing_import_pin_invert_and_label_pos_mark_dirty() {
    let mut c = Canvas::empty();
    let r1 = c.scene.add_default_resistor(120.0, 80.0);
    let _ = c.take_dirty();

    let ch = c.center_circuit();
    assert!(ch.items);
    assert!(c.take_dirty().full);

    let pin_id = format!("{r1}-lPin");
    let ch = c.toggle_pin_inverted(&pin_id);
    assert!(ch.items);
    assert!(c.take_dirty().items.contains(&r1));

    let ch = c.set_pin_inverted(&pin_id, false);
    assert!(ch.items);
    assert!(c.take_dirty().items.contains(&r1));

    let ch = c.set_item_label_pos(&r1, 12.0, -24.0);
    assert!(ch.items);
    assert!(c.take_dirty().items.contains(&r1));

    let ch = c.set_item_val_label_pos(&r1, 12.0, 24.0);
    assert!(ch.items);
    assert!(c.take_dirty().items.contains(&r1));

    // Place an item outside canvas bounds while r1 is selected, then select overflowing.
    c.scene.select_only(0);
    let r_out = c.scene.add_default_resistor(5000.0, 5000.0);
    let _ = c.take_dirty();
    let ch = c.select_overflowing_items();
    assert!(ch.items);
    let dirty = c.take_dirty();
    assert!(dirty.items.contains(&r1));
    assert!(dirty.items.contains(&r_out));

    // Import circuit marks full dirty.
    let src = c.to_circ1();
    let _ = c.take_dirty();
    let ch = c.import_circ1(&src);
    assert!(ch.items);
    assert!(c.take_dirty().full);
}

#[test]
fn meter_reading_change_on_non_third_tick_marks_dirty() {
    let mut c = Canvas::empty();
    let v_id = c.scene.add_fixed_volt(0.0, 0.0, 0.0);
    let vm_id = c.scene.add_voltmeter(80.0, 0.0);
    let gnd_id = c.scene.add_ground(80.0, 48.0);
    let p_out = c.scene.item_by_id(&v_id).unwrap().pins()[0].id.clone();
    let vm_pins = c.scene.item_by_id(&vm_id).unwrap().pins();
    let p_l = vm_pins[0].id.clone();
    let p_r = vm_pins[1].id.clone();
    let p_gnd = c.scene.item_by_id(&gnd_id).unwrap().pins()[0].id.clone();
    let p_out_pos = c.scene.pin_scene(&p_out).unwrap();
    let p_l_pos = c.scene.pin_scene(&p_l).unwrap();
    let p_r_pos = c.scene.pin_scene(&p_r).unwrap();
    let p_gnd_pos = c.scene.pin_scene(&p_gnd).unwrap();
    c.scene.connect_pins(&p_out, p_out_pos, &p_l, p_l_pos);
    c.scene.connect_pins(&p_r, p_r_pos, &p_gnd, p_gnd_pos);

    let _ = c.power_on();
    let _ = c.take_dirty();

    // Advance to a non-multiple-of-3 tick, change source voltage, and verify the
    // voltmeter is dirtied immediately rather than dropped between % 3 ticks.
    c.sim_tick = 1;
    let _ = c.set_prop_text_for(&v_id, "Voltage".into(), "5 V".into());
    let dirty = c.take_dirty();
    assert!(
        dirty.full || dirty.items.contains(&vm_id),
        "voltmeter must be marked dirty when its reading changes on a non-%3 tick"
    );
}

#[test]
fn paused_scope_and_la_setters_update_on_non_third_tick() {
    let mut c = Canvas::empty();
    let osc_id = c.scene.add_oscope(0.0, 0.0);
    let la_id = c.scene.add_lanalizer(200.0, 0.0);
    let _ = c.power_on();
    let _ = c.tick(); // sim_tick == 1 (non-%3)
    let _ = c.pause_sim();
    let _ = c.take_dirty();

    let _ = c.set_scope_time_div(2e-3);
    assert!(c.take_dirty().items.contains(&osc_id));

    let _ = c.set_scope_time_pos(5e-4);
    assert!(c.take_dirty().items.contains(&osc_id));

    let _ = c.set_scope_trigger(0);
    assert!(c.take_dirty().items.contains(&osc_id));

    let _ = c.set_scope_trig_level(1.5);
    assert!(c.take_dirty().items.contains(&osc_id));

    let _ = c.set_la_time_div(2e-3);
    assert!(c.take_dirty().items.contains(&la_id));

    let _ = c.set_la_time_pos(5e-4);
    assert!(c.take_dirty().items.contains(&la_id));

    let _ = c.set_la_trigger(1);
    assert!(c.take_dirty().items.contains(&la_id));

    let _ = c.set_la_thresholds(2.0, 1.0);
    assert!(c.take_dirty().items.contains(&la_id));
}

#[test]
fn led_matrix_sub_logic_threshold_change_marks_dirty() {
    let mut c = Canvas::empty();
    let v_id = c.scene.add_fixed_volt(0.0, 0.0, 0.0);
    let mat_id = c.scene.add_led_matrix(120.0, 0.0);
    let gnd_id = c.scene.add_ground(120.0, 120.0);
    // Row 0 -> first row pin, Col 0 -> first col pin. Default Yellow threshold = 2.4 V (paint > 1.92 V).
    let p_out = c.scene.item_by_id(&v_id).unwrap().pins()[0].id.clone();
    let mat_pins = c.scene.item_by_id(&mat_id).unwrap().pins();
    let p_r0 = mat_pins[0].id.clone();
    let p_c0 = mat_pins[8].id.clone();
    let p_gnd = c.scene.item_by_id(&gnd_id).unwrap().pins()[0].id.clone();
    let p_out_pos = c.scene.pin_scene(&p_out).unwrap();
    let p_r0_pos = c.scene.pin_scene(&p_r0).unwrap();
    let p_c0_pos = c.scene.pin_scene(&p_c0).unwrap();
    let p_gnd_pos = c.scene.pin_scene(&p_gnd).unwrap();
    c.scene.connect_pins(&p_out, p_out_pos, &p_r0, p_r0_pos);
    c.scene.connect_pins(&p_c0, p_c0_pos, &p_gnd, p_gnd_pos);

    let _ = c.power_on();
    let _ = c.take_dirty();

    // Raise voltage to 2.2 V: below PIN_LOGIC_HIGH_V (2.5 V) so pin logic state
    // does not change, but above LedMatrix paint threshold (2.4 * 0.8 = 1.92 V).
    let _ = c.set_prop_text_for(&v_id, "Voltage".into(), "2.2 V".into());
    let dirty = c.take_dirty();
    assert!(
        dirty.full || dirty.items.contains(&mat_id),
        "LedMatrix must be marked dirty when a pixel lights below 2.5 V"
    );
}

#[test]
fn zoom_scope_and_la_at_anchor_updates_time_pos_and_marks_dirty() {
    let mut c = Canvas::empty();
    let scope_id = c.scene.add_oscope(0.0, 0.0);
    let la_id = c.scene.add_lanalizer(160.0, 0.0);
    // 480 px wide => 480 / 48 = 10 horizontal scope divisions; 400x320 => 400*8/320 = 10 LA divisions.
    c.set_scope_view(480.0, 480.0);
    c.set_la_view(400.0, 320.0);
    let _ = c.set_scope_time_div(1e-3);
    let _ = c.set_scope_time_pos(0.0);
    let _ = c.set_la_time_div(1e-3);
    let _ = c.set_la_time_pos(0.0);
    let _ = c.take_dirty();

    // Zoom in 2x anchored at 25% from the left (untriggered):
    // old_window = 10 ms, new_window = 5 ms, delta_pos = (1 - 0.25) * (10 - 5) ms = 3.75 ms.
    let _ = c.zoom_scope_at(5e-4, 0.25);
    assert!((c.scope_time_div() - 5e-4).abs() < 1e-12);
    assert!((c.scope_time_pos() - 3.75e-3).abs() < 1e-12);
    assert!(c.take_dirty().items.contains(&scope_id));

    // Zoom back out to 1 ms at the same anchor restores time_pos = 0.
    let _ = c.zoom_scope_at(1e-3, 0.25);
    assert!((c.scope_time_div() - 1e-3).abs() < 1e-12);
    assert!(c.scope_time_pos().abs() < 1e-12);

    // Zoom LA anchored at right edge (1.0): time_pos stays 0.
    let _ = c.zoom_la_at(5e-4, 1.0);
    assert!((c.la_time_div() - 5e-4).abs() < 1e-12);
    assert!(c.la_time_pos().abs() < 1e-12);
    assert!(c.take_dirty().items.contains(&la_id));

    // Zoom LA anchored at left edge (0.0) from 0.5 ms -> 0.25 ms:
    // old_window = 5 ms, new_window = 2.5 ms, delta_pos = 1.0 * 2.5 ms = 2.5 ms.
    let _ = c.zoom_la_at(2.5e-4, 0.0);
    assert!((c.la_time_div() - 2.5e-4).abs() < 1e-12);
    assert!((c.la_time_pos() - 2.5e-3).abs() < 1e-12);
}

#[test]
fn upload_firmware_to_explicit_missing_target_does_not_overwrite_other_mcu() {
    let dir = std::env::temp_dir().join("cs_test_upload_explicit_target");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let hex1 = dir.join("fw1.hex");
    let hex2 = dir.join("fw2.hex");
    std::fs::write(&hex1, ":00000001FF\n").unwrap();
    std::fs::write(&hex2, ":00000001FF\n").unwrap();
    let circ_path = dir.join("test.circ1");

    let circ = r#"<circuit version="1.0.0">
  <item itemtype="MCU" CircId="mega328-1" label="MCU1" Pos="0,0" Program="fw1.hex"/>
  <item itemtype="MCU" CircId="mega328-2" label="MCU2" Pos="200,0" Program="fw2.hex"/>
</circuit>"#;
    std::fs::write(&circ_path, circ).unwrap();

    let mut canvas = Canvas::empty();
    canvas
        .load_circ1(circ, Some(circ_path.to_string_lossy().into_owned()))
        .unwrap();

    // Uploading with an explicit nonexistent target must NOT fall through and overwrite MCU1.
    let ch = canvas.upload_firmware_to(&hex2.to_string_lossy(), Some("mega328-missing"));
    assert!(!ch.items);
    assert_eq!(canvas.firmware_path_for("mega328-1"), Some(hex1.clone()));
    assert_eq!(canvas.firmware_path_for("mega328-2"), Some(hex2.clone()));

    // Uploading with an explicit valid target only updates that target.
    let ch2 = canvas.upload_firmware_to(&hex2.to_string_lossy(), Some("mega328-2"));
    assert!(ch2.items);
    assert_eq!(canvas.firmware_path_for("mega328-1"), Some(hex1));
    assert_eq!(canvas.firmware_path_for("mega328-2"), Some(hex2));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn power_on_sets_debug_target_mcu_to_active_device() {
    let circ = r#"<circuit version="1.0.0">
  <item itemtype="MCU" CircId="mega328-1" label="MCU1" Pos="0,0"/>
  <item itemtype="MCU" CircId="mega328-2" label="MCU2" Pos="200,0"/>
</circuit>"#;
    let mut canvas = Canvas::empty();
    canvas.load_circ1(circ, None).unwrap();
    canvas.set_active_device_id(Some("mega328-2".to_string()));
    canvas.power_on();
    assert_eq!(
        canvas
            .running
            .as_ref()
            .and_then(|c| c.debug_target_mcu.as_deref()),
        Some("mega328-2")
    );

    // Switching active device while running updates the debug target immediately.
    canvas.set_active_device_id(Some("mega328-1".to_string()));
    assert_eq!(
        canvas
            .running
            .as_ref()
            .and_then(|c| c.debug_target_mcu.as_deref()),
        Some("mega328-1")
    );
    canvas.power_off();
}

#[test]
fn phase6_two_scopes_independent_dirty_and_blank_channel() {
    let circ = r#"<circuit version="1.0.0">
  <item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="0,0" />
  <item itemtype="Clock" CircId="CLK1" Freq="1000 Hz" Voltage="5 V" Running="true" Pos="0,100" />
  <item itemtype="Oscope" CircId="S1" Pos="200,0" />
  <item itemtype="Oscope" CircId="S2" Pos="200,200" />
  <item itemtype="Connector" uid="w1" startpinid="V1-outnod" endpinid="S1-Pin0" />
  <item itemtype="Connector" uid="w2" startpinid="CLK1-outnod" endpinid="S2-Pin0" />
</circuit>"#;
    let mut canvas = Canvas::empty();
    canvas.load_circ1(circ, None).unwrap();
    canvas.power_on();
    // Run a few ticks so initial settle finishes and clear dirty set.
    for _ in 0..3 {
        canvas.tick();
    }
    let _ = canvas.take_dirty();

    // Advance enough ticks for CLK1 to toggle while V1 stays completely flat.
    let mut s2_dirtied = false;
    for _ in 0..9 {
        canvas.tick();
        let d = canvas.take_dirty();
        if d.items.contains("S2") {
            s2_dirtied = true;
            break;
        }
    }
    assert!(
        s2_dirtied,
        "S2 on a clock must mark dirty independently even while S1 is on a static rail"
    );

    // Changing S1's tunnel blanks channel 0 and must mark S1 dirty.
    let _ = canvas.take_dirty();
    let _ = canvas.set_prop_text_for("S1", "Tunnel0".into(), "NONEXISTENT_NET".into());
    let d = canvas.take_dirty();
    assert!(
        d.full || d.items.contains("S1"),
        "blank_channel must mark the blanked scope dirty"
    );
    canvas.power_off();
}

#[test]
fn phase6_visual_rects_cover_probe_text_shape_stroke_touchpad_and_ds1621() {
    use crate::canvas::dirty::item_dirty_rect;
    use crate::canvas::geom::map_local;
    use crate::components::{Component, DS1621, Probe, Shape, TouchPad};

    // Probe at default -45° rotation: voltage text at local (10, -6)..(45, 4)
    // must lie completely inside `item_dirty_rect`.
    let mut probe_item = Item::probe("P1", 100.0, 100.0, 2.5, false);
    assert!((probe_item.rotation - (-45.0)).abs() < 1e-6);
    let dirty = item_dirty_rect(&probe_item);
    for local_pt in [
        Point::new(10.0, -6.0),
        Point::new(45.0, -6.0),
        Point::new(45.0, 4.0),
        Point::new(10.0, 4.0),
    ] {
        let sp = map_local(
            probe_item.position(),
            local_pt,
            probe_item.rotation,
            probe_item.hflip,
            probe_item.vflip,
        );
        assert!(
            dirty.contains_point(sp),
            "probe dirty rect {dirty:?} must contain rotated text point {sp:?} (local {local_pt:?})"
        );
    }

    // With ShowVolt = false, Probe visual_rect shrinks back to body.
    let p_no_volt = Probe {
        show_volt: false,
        ..Probe::default()
    };
    assert_eq!(p_no_volt.visual_rect(), p_no_volt.body());
    probe_item.kind = Part::Probe(p_no_volt);
    assert_eq!(probe_item.kind.visual_rect(), probe_item.kind.body());

    // Shape with 20 px border expands visual_rect by half-stroke (10 px) on each side.
    let mut rect_shape = Shape::default();
    rect_shape.width = 40.0;
    rect_shape.height = 20.0;
    rect_shape.border = 20;
    let body = rect_shape.body();
    let vr = rect_shape.visual_rect();
    assert!(vr.x <= body.x - 10.0 + 1e-6);
    assert!(vr.y <= body.y - 10.0 + 1e-6);
    assert!(vr.x + vr.w >= body.x + body.w + 10.0 - 1e-6);
    assert!(vr.y + vr.h >= body.y + body.h + 10.0 - 1e-6);

    // TouchPad visual_rect covers ±6 px crosshair beyond left/top/right pad edges.
    let tp = TouchPad::default();
    let tp_b = tp.body();
    let tp_v = tp.visual_rect();
    assert!(tp_v.x <= tp_b.x - 6.0);
    assert!(tp_v.y <= tp_b.y - 6.0);
    assert!(tp_v.x + tp_v.w >= tp_b.x + tp_b.w + 6.0);

    // DS1621 visual_rect covers the top orientation notch reaching y = -32.
    let ds = DS1621::default();
    assert!(ds.visual_rect().y <= -32.0);
}

#[test]
fn phase6_led_bar_segment31_threshold_rgb_overload_and_lamp_bucket() {
    use crate::canvas::dirty::analog_visual_bucket_for_part;

    // 1. 32-segment LED bar with threshold = 3.0 V: segment 31 stays off at 2.0 V, turns on at 4.0 V.
    let circ_low = r#"<circuit version="1.0.0">
  <item itemtype="FixedVolt" CircId="V1" Voltage="2 V" Pos="0,0" />
  <item itemtype="LedBar" CircId="BAR1" Size="32" Grounded="true" Threshold="3 V" Pos="100,0" />
  <item itemtype="Connector" uid="w1" startpinid="V1-outnod" endpinid="BAR1-lPin31" />
</circuit>"#;
    let mut c_low = Canvas::empty();
    c_low.load_circ1(circ_low, None).unwrap();
    c_low.power_on();
    let r_low = c_low.readings().get("BAR1").expect("BAR1 reading");
    assert_eq!(r_low.text.len(), 32);
    assert_eq!(
        r_low.text.as_bytes()[31],
        b'0',
        "segment 31 at 2V must be off when threshold=3V"
    );
    let b_low = analog_visual_bucket_for_part(
        &c_low.scene().item_by_id("BAR1").unwrap().kind,
        "BAR1",
        c_low.running.as_ref().unwrap(),
    );

    let circ_high = circ_low.replace("Voltage=\"2 V\"", "Voltage=\"4 V\"");
    let mut c_high = Canvas::empty();
    c_high.load_circ1(&circ_high, None).unwrap();
    c_high.power_on();
    let r_high = c_high.readings().get("BAR1").expect("BAR1 reading");
    assert_eq!(
        r_high.text.as_bytes()[31],
        b'1',
        "segment 31 at 4V must be on when threshold=3V"
    );
    let b_high = analog_visual_bucket_for_part(
        &c_high.scene().item_by_id("BAR1").unwrap().kind,
        "BAR1",
        c_high.running.as_ref().unwrap(),
    );
    assert_ne!(
        b_low, b_high,
        "segment 31 crossing threshold must change dirty bucket"
    );
    c_low.power_off();
    c_high.power_off();

    // 2. RGB LED per-channel max current: 10 mA max on red with ~20 mA must trigger overload warning.
    let circ_rgb = r#"<circuit version="1.0.0">
  <item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="0,0" />
  <item itemtype="Resistor" CircId="R1" Resistance="150 Ω" Pos="50,0" />
  <item itemtype="RgbLed" CircId="RGB1" MaxCurrent_R="10 mA" Threshold_R="2 V" Pos="120,0" />
  <item itemtype="Ground" CircId="G1" Pos="160,0" />
  <item itemtype="Connector" uid="w1" startpinid="V1-outnod" endpinid="R1-lPin" />
  <item itemtype="Connector" uid="w2" startpinid="R1-rPin" endpinid="RGB1-rPin" />
  <item itemtype="Connector" uid="w3" startpinid="RGB1-cPin" endpinid="G1-Gnd" />
</circuit>"#;
    let mut c_rgb = Canvas::empty();
    c_rgb.load_circ1(circ_rgb, None).unwrap();
    c_rgb.power_on();
    c_rgb.tick();
    let ov = c_rgb
        .item_overload_state("RGB1")
        .expect("RGB1 overload state");
    assert!(
        ov.warning,
        "RGB LED red die at ~20 mA with 10 mA max must warn (reason: {})",
        ov.reason
    );
    c_rgb.power_off();

    // 3. High-resistance Lamp (47 kΩ) at 2 V produces non-zero analog dirty bucket.
    let circ_lamp = r#"<circuit version="1.0.0">
  <item itemtype="FixedVolt" CircId="V1" Voltage="2 V" Pos="0,0" />
  <item itemtype="Lamp" CircId="L1" Resistance="47 kΩ" Voltage="12 V" Power="2.4 W" Pos="80,0" />
  <item itemtype="Ground" CircId="G1" Pos="160,0" />
  <item itemtype="Connector" uid="w1" startpinid="V1-outnod" endpinid="L1-lPin" />
  <item itemtype="Connector" uid="w2" startpinid="L1-rPin" endpinid="G1-Gnd" />
</circuit>"#;
    let mut c_lamp = Canvas::empty();
    c_lamp.load_circ1(circ_lamp, None).unwrap();
    c_lamp.power_on();
    let lamp_bucket = analog_visual_bucket_for_part(
        &c_lamp.scene().item_by_id("L1").unwrap().kind,
        "L1",
        c_lamp.running.as_ref().unwrap(),
    )
    .unwrap();
    assert!(
        lamp_bucket > 0,
        "47 kΩ lamp at 2 V must have non-zero dirty bucket, got {lamp_bucket}"
    );
    c_lamp.power_off();
}
