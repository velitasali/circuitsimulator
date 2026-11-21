use cs_engine::canvas::Canvas;
use cs_engine::canvas::export::{Palette, svg_string};
use cs_engine::{CERO_DOUB, Circuit};

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-2, "{a} != {b}");
}

#[test]
fn test_touchpad_4wire_reading_simulation() {
    // Standard 4-wire Touchpad X-read mode:
    // Drive XP to 5V, XM to GND.
    // Read voltage on YP (with 10 MΩ pull-down).

    // 1. Untouched state (-1, -1)
    let mut c_untouched = Circuit::new();
    c_untouched.add_fixed_volt("src_x", 5.0);
    c_untouched.add_ground("gnd");
    c_untouched.add_resistor("r_load", 1e7); // 10 MΩ meter load on YP
    c_untouched.add_touchpad("tp", 200, 100, 100.0, 500.0, 100.0, 500.0, -1, -1);

    c_untouched.connect("src_x-outnod", "tp-vrx_p");
    c_untouched.connect("gnd-Gnd", "tp-vrx_m");
    c_untouched.connect("tp-vry_p", "r_load-lPin");
    c_untouched.connect("r_load-rPin", "gnd-Gnd");

    c_untouched.solve().unwrap();
    approx(c_untouched.pin_voltage("tp-vrx_p").unwrap(), 5.0);
    approx(c_untouched.pin_voltage("tp-vrx_m").unwrap(), CERO_DOUB);
    // YP should be disconnected (0V via pull-down)
    approx(c_untouched.pin_voltage("tp-vry_p").unwrap(), CERO_DOUB);

    // 2. Touched at center (100, 50) of 200x100
    let mut c_center = Circuit::new();
    c_center.add_fixed_volt("src_x", 5.0);
    c_center.add_ground("gnd");
    c_center.add_resistor("r_load", 1e7);
    c_center.add_touchpad("tp", 200, 100, 100.0, 500.0, 100.0, 500.0, 100, 50);

    c_center.connect("src_x-outnod", "tp-vrx_p");
    c_center.connect("gnd-Gnd", "tp-vrx_m");
    c_center.connect("tp-vry_p", "r_load-lPin");
    c_center.connect("r_load-rPin", "gnd-Gnd");

    c_center.solve().unwrap();
    // Center of symmetrical 100..500Ω is 2.5V
    approx(c_center.pin_voltage("tp-vry_p").unwrap(), 2.5);

    // 3. Touched at quarter position (50, 25)
    // x_pos = 50 / 200 = 0.25
    // xResA = 100 + 400 * 0.25 = 200 Ω (between vrx_p and nodeX)
    // xResB = 600 - 200 = 400 Ω (between nodeX and vrx_m)
    // Voltage at nodeX = 5.0 * 400 / (200 + 400) = 3.333 V
    let mut c_quarter = Circuit::new();
    c_quarter.add_fixed_volt("src_x", 5.0);
    c_quarter.add_ground("gnd");
    c_quarter.add_resistor("r_load", 1e7);
    c_quarter.add_touchpad("tp", 200, 100, 100.0, 500.0, 100.0, 500.0, 50, 25);

    c_quarter.connect("src_x-outnod", "tp-vrx_p");
    c_quarter.connect("gnd-Gnd", "tp-vrx_m");
    c_quarter.connect("tp-vry_p", "r_load-lPin");
    c_quarter.connect("r_load-rPin", "gnd-Gnd");

    c_quarter.solve().unwrap();
    approx(c_quarter.pin_voltage("tp-vry_p").unwrap(), 3.333);
}

#[test]
fn test_touchpad_canvas_and_sim1_roundtrip() {
    let mut canvas = Canvas::new();
    let id = canvas.scene_mut().add_touchpad(100.0, 200.0);
    assert_eq!(id, "TouchPad-1");

    // Verify properties
    let item = canvas
        .scene()
        .item_by_id(&id)
        .expect("touchpad item exists");
    assert_eq!(item.prop_text("Width").as_deref(), Some("240"));
    assert_eq!(item.prop_text("Height").as_deref(), Some("320"));
    assert_eq!(item.prop_text("Transparent").as_deref(), Some("false"));

    // Modify properties
    let mut item_mut = item.clone();
    assert!(item_mut.set_prop_text("Width", "320"));
    assert!(item_mut.set_prop_text("Height", "480"));
    assert!(item_mut.set_prop_text("Transparent", "true"));
    assert!(item_mut.set_prop_text("RxMin", "200"));
    assert_eq!(item_mut.prop_text("Width").as_deref(), Some("320"));
    assert_eq!(item_mut.prop_text("Height").as_deref(), Some("480"));
    assert_eq!(item_mut.prop_text("Transparent").as_deref(), Some("true"));
    assert_eq!(item_mut.prop_bool("Transparent"), Some(true));

    // Interactive touchpad positioning
    canvas.set_touchpad_pos(&id, 50, 60);
    let item2 = canvas.scene().item_by_id(&id).unwrap();
    assert_eq!(item2.x_pos(), 50);
    assert_eq!(item2.y_pos(), 60);

    canvas.reset_touchpad(&id);
    let item3 = canvas.scene().item_by_id(&id).unwrap();
    assert_eq!(item3.x_pos(), -1);
    assert_eq!(item3.y_pos(), -1);

    // Sim1 serialization & export
    let sim1 = canvas.scene().to_sim1();
    assert!(sim1.contains("itemtype=\"TouchPad\""));
    assert!(sim1.contains("CircId=\"TouchPad-1\""));

    let pal = Palette::light();
    let svg = svg_string(&canvas, &pal);
    assert!(svg.contains("<svg"));
}

#[test]
fn test_touchpad_mouse_interaction_on_circuit_view() {
    use cs_engine::canvas::scene::Item;
    use cs_engine::canvas::{BUTTON_LEFT, Point};

    let mut canvas = Canvas::new();
    let p = Point::new(0.0, 0.0);

    // Add TouchPad 200x100 at (200.0, 200.0)
    let mut tp_item = Item::touchpad_with(
        "TouchPad-1",
        200.0,
        200.0,
        200,
        100,
        false,
        100.0,
        500.0,
        100.0,
        500.0,
    );
    // Reset to untouched
    if let cs_engine::components::Part::TouchPad(ref mut tp) = tp_item.kind {
        tp.x_pos = -1;
        tp.y_pos = -1;
    }
    canvas.scene_mut().add_saved_item(tp_item);

    // Add Fixed Voltage (5V), Ground, and load resistor (10 MΩ)
    canvas
        .scene_mut()
        .add_saved_item(Item::fixed_volt("FV-1", 100.0, 250.0, 5.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::ground("GND-1", 100.0, 300.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::resistor("R-LOAD", 300.0, 250.0, 1e7));

    // Connect:
    // FV-1 -> TouchPad-1-vrx_p
    // GND-1 -> TouchPad-1-vrx_m
    // TouchPad-1-vry_p -> R-LOAD-lPin
    // R-LOAD-rPin -> GND-1
    canvas
        .scene_mut()
        .connect_pins("FV-1-outnod", p, "TouchPad-1-vrx_p", p);
    canvas
        .scene_mut()
        .connect_pins("GND-1-Gnd", p, "TouchPad-1-vrx_m", p);
    canvas
        .scene_mut()
        .connect_pins("TouchPad-1-vry_p", p, "R-LOAD-lPin", p);
    canvas
        .scene_mut()
        .connect_pins("R-LOAD-rPin", p, "GND-1-Gnd", p);

    // Power on simulation
    let c = canvas.power_on();
    assert!(c.sim);

    // 1. Initially untouched: 0V on YP
    let v_yp_init = canvas.pin_voltage("TouchPad-1-vry_p").unwrap();
    approx(v_yp_init, 0.0);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().x_pos(), -1);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().y_pos(), -1);

    // 2. Mouse Press at center of TouchPad (200.0, 150.0)
    // Note: TouchPad is at (200.0, 200.0), width 200, height 100.
    // Body is x in [100.0, 300.0], y in [100.0, 200.0]. Center is (200.0, 150.0).
    let screen_center = canvas.viewport().map_from_circuit(Point::new(200.0, 150.0));
    let ch_press = canvas.mouse_press(BUTTON_LEFT, screen_center.x, screen_center.y, 0);

    assert!(
        !ch_press.items,
        "live touch must not rebuild the QML items model"
    );
    assert_eq!(
        canvas.scene().item_by_id("TouchPad-1").unwrap().x_pos(),
        100
    );
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().y_pos(), 50);
    assert!(canvas.take_dirty().items.contains("TouchPad-1"));

    // Real-time circuit simulation should immediately reflect the center touch (2.5V on YP)
    let v_yp_center = canvas.pin_voltage("TouchPad-1-vry_p").unwrap();
    approx(v_yp_center, 2.5);

    // Verify schematic position was NOT dragged/moved
    assert_eq!(
        canvas.scene().item_by_id("TouchPad-1").unwrap().position(),
        Point::new(200.0, 200.0)
    );

    // 3. Mouse Move / Drag to quarter point (150.0, 125.0) -> local (50, 25)
    let screen_quarter = canvas.viewport().map_from_circuit(Point::new(150.0, 125.0));
    let ch_move = canvas.mouse_move(screen_quarter.x, screen_quarter.y, BUTTON_LEFT, 0);
    assert!(!ch_move.items);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().x_pos(), 50);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().y_pos(), 25);

    // Simulation updates to ~3.333V
    let v_yp_quarter = canvas.pin_voltage("TouchPad-1-vry_p").unwrap();
    approx(v_yp_quarter, 3.333);

    // 4. Mouse Release -> Finger lifted
    let ch_rel = canvas.mouse_release(BUTTON_LEFT, screen_quarter.x, screen_quarter.y, 0);
    assert!(!ch_rel.items);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().x_pos(), -1);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().y_pos(), -1);

    // Simulation returns to 0V (untouched)
    let v_yp_rel = canvas.pin_voltage("TouchPad-1-vry_p").unwrap();
    approx(v_yp_rel, 0.0);

    // 5. Mouse Cancel test
    canvas.mouse_press(BUTTON_LEFT, screen_center.x, screen_center.y, 0);
    assert_eq!(
        canvas.scene().item_by_id("TouchPad-1").unwrap().x_pos(),
        100
    );
    canvas.mouse_cancel();
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().x_pos(), -1);
    assert_eq!(canvas.scene().item_by_id("TouchPad-1").unwrap().y_pos(), -1);
    let v_yp_cancel = canvas.pin_voltage("TouchPad-1-vry_p").unwrap();
    approx(v_yp_cancel, 0.0);
}
