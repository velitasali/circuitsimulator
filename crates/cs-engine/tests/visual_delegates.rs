use cs_engine::canvas::export::{Palette, svg_string};
use cs_engine::canvas::scene::{Item, Scene};
use cs_engine::canvas::{Canvas, Point, Rect};

#[test]
fn test_scene_item_delegate_accessors() {
    let mut scene = Scene::new();

    // 1. Logic Gate
    let and_gate = Item::gate(
        "And-1", 100.0, 100.0, "And", 3, false, true, false, false, false,
    );
    assert_eq!(and_gate.gate_kind(), "And");
    assert_eq!(and_gate.num_inputs(), 3);
    assert!(and_gate.invert_output());
    scene.add_saved_item(and_gate);

    // 2. Flip-Flop
    let ff = Item::flipflop("FF-1", 200.0, 100.0, "D", true, "Clock");
    assert_eq!(ff.ff_kind(), "D");
    assert!(ff.use_rs());
    assert_eq!(ff.trigger(), "Clock");
    scene.add_saved_item(ff);

    // 3. Switch DIP
    let dip = Item::switch_dip("Dip-1", 300.0, 100.0, 4, 0b0101, false);
    assert_eq!(dip.size(), 4);
    assert_eq!(dip.state(), 0b0101);
    assert!(!dip.exclusive());
    scene.add_saved_item(dip);

    // 4. Potentiometer
    let pot = Item::potentiometer("Pot-1", 400.0, 100.0, 50_000.0, 0.75);
    assert_eq!(pot.resistance(), 50_000.0);
    assert!((pot.wiper() - 0.75).abs() < 1e-5);
    scene.add_saved_item(pot);

    // 5. Dial
    let dial = Item::dial("Dial-1", 500.0, 100.0, 5.0);
    assert_eq!(dial.source_value(), 5.0);
    assert_eq!(dial.min_value(), 0.0);
    assert_eq!(dial.max_value(), 100.0);
    assert_eq!(dial.dial_step(), 1.0);
    scene.add_saved_item(dial);

    // 6. Push Button
    let push = Item::push("Push-1", 600.0, 100.0, false, 2);
    assert!(!push.pressed());
    assert_eq!(push.poles(), 2);
    assert!(!push.show_button());
    scene.add_saved_item(push);

    // 7. Seven Segment
    let seven_seg = Item::seven_segment("Seg-1", 700.0, 100.0, false);
    assert!(!seven_seg.common_anode());
    scene.add_saved_item(seven_seg);

    // 8. Tunnel
    let mut tunnel = Item::tunnel("Tun-1", 800.0, 100.0, "DATA_BUS");
    assert_eq!(tunnel.tunnel_name(), "DATA_BUS");
    assert!(!tunnel.is_bus());
    let pins = tunnel.pins();
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0].local.x, 0.0);
    assert_eq!(pins[0].local.y, 0.0);
    assert_eq!(pins[0].angle, 0);
    assert_eq!(pins[0].length, 4.0);
    assert!(!pins[0].is_bus);
    tunnel.set_prop_text("IsBus", "true");
    assert!(tunnel.is_bus());
    assert!(tunnel.pins()[0].is_bus);
    scene.add_saved_item(tunnel);

    // 9. Shape
    let mut rect_shape = Item::shape("Shape-1", 900.0, 100.0, "Rectangle");
    rect_shape.set_prop_text("Width", "80.0");
    rect_shape.set_prop_text("Height", "40.0");
    assert_eq!(rect_shape.shape_kind(), "Rectangle");
    assert_eq!(rect_shape.shape_width(), 80.0);
    assert_eq!(rect_shape.shape_height(), 40.0);
    scene.add_saved_item(rect_shape);

    assert_eq!(scene.items().len(), 9);
}

#[test]
fn test_canvas_interactive_manipulations() {
    let mut canvas = Canvas::default();

    // Add DIP Switch
    let dip = Item::switch_dip("Dip-1", 100.0, 100.0, 4, 0b0000, false);
    canvas.scene_mut().add_saved_item(dip);

    // Toggle DIP switch index 1
    canvas.toggle_dip_switch("Dip-1", 1);
    let dip_item = canvas.scene().item_by_id("Dip-1").unwrap();
    assert_eq!(dip_item.state(), 0b0010);

    // Set DIP switch index 3 ON
    canvas.set_dip_switch("Dip-1", 3, true);
    let dip_item = canvas.scene().item_by_id("Dip-1").unwrap();
    assert_eq!(dip_item.state(), 0b1010);

    // Add Push Button
    let push = Item::push("Push-1", 200.0, 100.0, false, 1);
    canvas.scene_mut().add_saved_item(push);

    canvas.set_push_state("Push-1", true);
    assert!(canvas.scene().item_by_id("Push-1").unwrap().pressed());

    // Add Potentiometer
    let pot = Item::potentiometer("Pot-1", 300.0, 100.0, 10_000.0, 0.5);
    canvas.scene_mut().add_saved_item(pot);

    canvas.set_pot_wiper("Pot-1", 0.85);
    assert!((canvas.scene().item_by_id("Pot-1").unwrap().wiper() - 0.85).abs() < 1e-5);

    // Add Dial
    let dial = Item::dial("Dial-1", 400.0, 100.0, 0.0);
    canvas.scene_mut().add_saved_item(dial);

    canvas.set_dial_val("Dial-1", 75.0);
    assert_eq!(
        canvas.scene().item_by_id("Dial-1").unwrap().source_value(),
        75.0
    );

    // Hit-testing toggle switch at position
    let sw = Item::switch("Sw-1", 500.0, 100.0, false);
    canvas.scene_mut().add_saved_item(sw);

    let sw_idx = canvas
        .scene()
        .items()
        .iter()
        .position(|it| it.id == "Sw-1")
        .unwrap();
    assert!(
        canvas
            .scene_mut()
            .toggle_switch_at(sw_idx, Point::new(500.0, 100.0))
            .is_some()
    );
    assert!(canvas.scene().item_by_id("Sw-1").unwrap().closed());

    // Add VoltSource (min: 0.0, max: 5.0, initial: 2.5)
    let vs = Item::volt_source("VoltSource-1", 600.0, 100.0, 2.5, true);
    canvas.scene_mut().add_saved_item(vs);

    // Knob center is at (600, 100 - 8) = (600, 92)
    // Vertical scroll up (+120) increases value by 1% of range (0.05)
    assert!(
        canvas
            .scene_mut()
            .wheel_at(Point::new(600.0, 92.0), 120.0)
            .is_some()
    );
    let val = canvas
        .scene()
        .item_by_id("VoltSource-1")
        .unwrap()
        .source_value();
    assert!((val - 2.55).abs() < 1e-4);

    // Horizontal scroll right (+120) also increases value in the same direction
    assert!(
        canvas
            .scene_mut()
            .wheel_at(Point::new(600.0, 92.0), 120.0)
            .is_some()
    );
    let val = canvas
        .scene()
        .item_by_id("VoltSource-1")
        .unwrap()
        .source_value();
    assert!((val - 2.60).abs() < 1e-4);

    // Vertical scroll down (-120) decreases value
    assert!(
        canvas
            .scene_mut()
            .wheel_at(Point::new(600.0, 92.0), -120.0)
            .is_some()
    );
    let val = canvas
        .scene()
        .item_by_id("VoltSource-1")
        .unwrap()
        .source_value();
    assert!((val - 2.55).abs() < 1e-4);

    // Canvas::wheel test at view coordinates mapping to the knob
    let view_pt = canvas.viewport().map_from_circuit(Point::new(600.0, 92.0));
    let ch = canvas.wheel(0.0, 0.0, 0.0, 120.0, view_pt.x, view_pt.y, 0);
    assert!(
        !ch.items,
        "Wheel on a source must not rebuild the QML items model"
    );
    assert!(
        canvas.take_dirty().items.contains("VoltSource-1"),
        "Wheel interaction must mark VoltSource-1 dirty"
    );
    assert!(canvas.take_last_wheel_item().is_some());
    let val = canvas
        .scene()
        .item_by_id("VoltSource-1")
        .unwrap()
        .source_value();
    assert!((val - 2.60).abs() < 1e-4);

    // Wheel event over button area (local y: 15 -> scene y: 115) also updates value
    let view_btn_pt = canvas.viewport().map_from_circuit(Point::new(600.0, 115.0));
    let ch_btn = canvas.wheel(0.0, 0.0, 0.0, 120.0, view_btn_pt.x, view_btn_pt.y, 0);
    assert!(!ch_btn.items);
    assert!(canvas.take_dirty().items.contains("VoltSource-1"));
    let val = canvas
        .scene()
        .item_by_id("VoltSource-1")
        .unwrap()
        .source_value();
    assert!((val - 2.65).abs() < 1e-4);

    // Clicking the button area via canvas mouse_press + mouse_release toggles running status
    let view_click_pt = canvas.viewport().map_from_circuit(Point::new(600.0, 119.0));
    canvas.mouse_press(
        cs_engine::canvas::BUTTON_LEFT,
        view_click_pt.x,
        view_click_pt.y,
        0,
    );
    let rel_ch = canvas.mouse_release(
        cs_engine::canvas::BUTTON_LEFT,
        view_click_pt.x,
        view_click_pt.y,
        0,
    );
    assert!(rel_ch.items, "Mouse release toggle must set Change::items");
    assert!(
        canvas.take_dirty().items.contains("VoltSource-1"),
        "Mouse release toggle must mark VoltSource-1 dirty"
    );
    assert!(
        !canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .source_running()
    );

    // Second click enables it back
    canvas.mouse_press(
        cs_engine::canvas::BUTTON_LEFT,
        view_click_pt.x,
        view_click_pt.y,
        0,
    );
    let rel_ch2 = canvas.mouse_release(
        cs_engine::canvas::BUTTON_LEFT,
        view_click_pt.x,
        view_click_pt.y,
        0,
    );
    assert!(rel_ch2.items);
    assert!(
        canvas.take_dirty().items.contains("VoltSource-1"),
        "Second mouse release toggle must mark VoltSource-1 dirty"
    );
    assert!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .source_running()
    );

    // Min / Max cross-adjustment tests:
    // Set min to 3V, max to 5V, value to 4V
    canvas
        .scene_mut()
        .item_by_id_mut("VoltSource-1")
        .unwrap()
        .set_prop_text("MinValue", "3 V");
    canvas
        .scene_mut()
        .item_by_id_mut("VoltSource-1")
        .unwrap()
        .set_prop_text("MaxValue", "5 V");
    canvas
        .scene_mut()
        .item_by_id_mut("VoltSource-1")
        .unwrap()
        .set_prop_text("Value", "4 V");
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .min_value(),
        3.0
    );
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .max_value(),
        5.0
    );
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .source_value(),
        4.0
    );

    // If min is 3V and we set MaxValue to 2V -> min becomes 2V and value clamps to 2V
    canvas
        .scene_mut()
        .item_by_id_mut("VoltSource-1")
        .unwrap()
        .set_prop_text("MaxValue", "2 V");
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .min_value(),
        2.0
    );
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .max_value(),
        2.0
    );
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .source_value(),
        2.0
    );

    // If max is 2V and we set MinValue to 5V -> max becomes 5V and value clamps to 5V
    canvas
        .scene_mut()
        .item_by_id_mut("VoltSource-1")
        .unwrap()
        .set_prop_text("MinValue", "5 V");
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .min_value(),
        5.0
    );
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .max_value(),
        5.0
    );
    assert_eq!(
        canvas
            .scene()
            .item_by_id("VoltSource-1")
            .unwrap()
            .source_value(),
        5.0
    );
}

#[test]
fn test_svg_export_rendering_all_visual_delegates() {
    let mut canvas = Canvas::default();

    canvas.scene_mut().add_saved_item(Item::gate(
        "G1", 0.0, 0.0, "And", 2, false, false, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "G2", 50.0, 0.0, "Or", 2, false, false, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "G3", 100.0, 0.0, "Xor", 2, false, false, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "G4", 150.0, 0.0, "Buffer", 1, false, true, false, false, false,
    ));
    canvas
        .scene_mut()
        .add_saved_item(Item::flipflop("FF1", 200.0, 0.0, "JK", true, "Clock"));
    canvas
        .scene_mut()
        .add_saved_item(Item::push("P1", 250.0, 0.0, true, 1));
    canvas
        .scene_mut()
        .add_saved_item(Item::switch_dip("DIP1", 300.0, 0.0, 4, 0b1100, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::potentiometer("POT1", 350.0, 0.0, 10_000.0, 0.3));
    canvas
        .scene_mut()
        .add_saved_item(Item::seven_segment("SEG1", 400.0, 0.0, true));
    canvas
        .scene_mut()
        .add_saved_item(Item::lamp("LAMP1", 450.0, 0.0, 12.0, 0.5, 1.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::tunnel("TUN1", 500.0, 0.0, "CLK_NET"));
    canvas
        .scene_mut()
        .add_saved_item(Item::dial("DIAL1", 550.0, 0.0, 50.0));

    let pal = Palette::light();
    let svg = svg_string(&canvas, &pal);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
    assert!(svg.contains("CLK_NET"));
}

#[test]
fn test_all_23_fixed_components() {
    let mut canvas = Canvas::default();
    let s = canvas.scene_mut();

    // 1. WaveGen
    s.add_saved_item(Item::wave_gen(
        "WG1", 0.0, 0.0, "Sine", 1000.0, 5.0, 0.0, 50.0,
    ));
    // 2. VoltageSource
    s.add_saved_item(Item::volt_source("VS1", 50.0, 0.0, 5.0, false));
    // 3. CurrentSource
    s.add_saved_item(Item::curr_source("CS1", 100.0, 0.0, 1.0, false));
    // 4. CSource
    s.add_saved_item(Item::csource(
        "CCS1", 150.0, 0.0, true, false, false, 2.0, 5.0, 1.0,
    ));
    // 5. Rail
    s.add_saved_item(Item::rail("R1", 200.0, 0.0, 5.0));
    // 6. SwitchDip
    s.add_saved_item(Item::switch_dip("SD1", 250.0, 0.0, 4, 0b1010, false));
    // 7. Relay
    s.add_saved_item(Item::relay(
        "RL1", 300.0, 0.0, false, false, 1, 12.0, 100.0, false,
    ));
    // 8. KeyPad
    s.add_saved_item(Item::keypad("KP1", 350.0, 0.0, 4, 4));
    // 9. ResistorDip
    s.add_saved_item(Item::resistor_dip("RD1", 400.0, 0.0, 4, 10_000.0, false));
    // 10. ElCapacitor
    s.add_saved_item(Item::el_capacitor("EC1", 450.0, 0.0, 100e-6));
    // 11. VarCapacitor
    s.add_saved_item(Item::var_capacitor("VC1", 500.0, 0.0, 10e-12));
    // 12. Scr
    s.add_saved_item(Item::scr("SCR1", 550.0, 0.0, 0.7, 0.01));
    // 13. Diac
    s.add_saved_item(Item::diac("DIAC1", 600.0, 0.0, 32.0));
    // 14. Triac
    s.add_saved_item(Item::triac("TRIAC1", 650.0, 0.0, 0.7, 0.01));
    // 15. AnalogMux
    s.add_saved_item(Item::analog_mux("MUX1", 700.0, 0.0, 8, 100.0));
    // 16. RGBLed
    s.add_saved_item(Item::rgb_led("RGB1", 750.0, 0.0, false));
    // 17. Or Gate
    s.add_saved_item(Item::gate(
        "OR1", 800.0, 0.0, "Or", 2, false, false, false, false, false,
    ));
    // 18. Lamp
    s.add_saved_item(Item::lamp("L1", 850.0, 0.0, 12.0, 0.5, 1.0));
    // 19. AudioOut
    s.add_saved_item(Item::audio_out("AO1", 900.0, 0.0, 8.0));
    // 20. WS2812
    s.add_saved_item(Item::ws2812("WS1", 950.0, 0.0, 8));
    // 21. Max72xx
    s.add_saved_item(Item::max72xx("M721", 1000.0, 0.0, 2));
    // 22. LedMatrix
    s.add_saved_item(Item::led_matrix("LM1", 1050.0, 0.0, 8, 8));
    // 23. LedBar
    s.add_saved_item(Item::led_bar("LB1", 1100.0, 0.0, 10));
    // 24. Latch
    s.add_saved_item(Item::latch(
        "LAT1", 1150.0, 0.0, 4, false, false, "Level", false,
    ));
    // 25. Hd44780
    s.add_saved_item(Item::hd44780("LCD1", 1200.0, 0.0, 2, 16));

    assert_eq!(canvas.scene().items().len(), 25);

    let pal = Palette::light();
    let svg = svg_string(&canvas, &pal);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));

    // Verify pin positions for components
    let sd = canvas.scene().item_by_id("SD1").unwrap();
    let sd_pins = sd.pins();
    assert_eq!(sd_pins.len(), 8);
    assert_eq!(sd_pins[0].local.y, -24.0); // lPin0
    assert_eq!(sd_pins[1].local.y, -24.0); // rPin0
    assert_eq!(sd_pins[2].local.y, -16.0); // lPin1

    let rd = canvas.scene().item_by_id("RD1").unwrap();
    let rd_pins = rd.pins();
    assert_eq!(rd_pins.len(), 8);
    assert_eq!(rd_pins[0].local.y, -24.0); // lPin0
    assert_eq!(rd_pins[1].local.y, -24.0); // rPin0
    assert_eq!(rd_pins[2].local.y, -16.0); // lPin1

    let lb = canvas.scene().item_by_id("LB1").unwrap();
    let lb_pins = lb.pins();
    assert_eq!(lb_pins.len(), 20);
    assert_eq!(lb_pins[0].local.y, -24.0); // lPin0
    assert_eq!(lb_pins[1].local.y, -24.0); // rPin0
    assert_eq!(lb_pins[2].local.y, -16.0); // lPin1

    let ec = canvas.scene().item_by_id("EC1").unwrap();
    assert_eq!(ec.kind.type_name(), "ElCapacitor");

    let lat = canvas.scene().item_by_id("LAT1").unwrap();
    let lat_pins = lat.pins();
    assert_eq!(lat_pins[0].local.x, -24.0);
    assert_eq!(lat_pins[0].length, 8.0);

    let mux = canvas.scene().item_by_id("MUX1").unwrap();
    let mux_pins = mux.pins();
    assert_eq!(mux_pins.len(), 13); // 8 channels + 3 address + En + Z
    let mux_rect = mux.body_rect();
    assert_eq!(mux_rect.x, -16.0);
    assert_eq!(mux_rect.y, 0.0);
    assert_eq!(mux_rect.w, 32.0);
    assert_eq!(mux_rect.h, 72.0);
}

#[test]
fn test_scr_triac_socket_header_serial_delegates() {
    let mut canvas = Canvas::default();
    let s = canvas.scene_mut();

    // SCR pin angles and lengths
    let scr = Item::scr("SCR_TEST", 0.0, 0.0, 0.7, 0.01);
    let scr_pins = scr.pins();
    assert_eq!(scr_pins.len(), 3);
    assert_eq!(scr_pins[2].angle, -40);
    assert_eq!(scr_pins[2].length, 12.0);
    s.add_saved_item(scr);

    // TRIAC pin angles and lengths
    let triac = Item::triac("TRIAC_TEST", 50.0, 0.0, 0.7, 0.01);
    let triac_pins = triac.pins();
    assert_eq!(triac_pins.len(), 3);
    assert_eq!(triac_pins[2].angle, -26);
    assert_eq!(triac_pins[2].length, 8.9);
    s.add_saved_item(triac);

    // Socket: 8 outer pins for wires (left) + 8 socket pins (center)
    let socket = Item::socket("SOCK_TEST", 100.0, 0.0, 8);
    assert_eq!(socket.kind.type_name(), "Socket");
    let sock_pins = socket.pins();
    assert_eq!(sock_pins.len(), 16);
    s.add_saved_item(socket);

    // Header: 6 outer pins for wires (left) + 6 header pins (center)
    let header = Item::header("HDR_TEST", 150.0, 0.0, 6);
    assert_eq!(header.kind.type_name(), "Header");
    let hdr_pins = header.pins();
    assert_eq!(hdr_pins.len(), 12);
    s.add_saved_item(header);

    // SerialPort
    let sp = Item::serial_port("SP_TEST", 200.0, 0.0, "ttyUSB0", 115200);
    assert_eq!(sp.kind.type_name(), "SerialPort");
    s.add_saved_item(sp);

    // SerialTerm
    let st = Item::serial_term("ST_TEST", 250.0, 0.0, 9600);
    assert_eq!(st.kind.type_name(), "SerialTerm");
    s.add_saved_item(st);

    // RGB LED
    let rgb = Item::rgb_led("RGB_TEST", 300.0, 0.0, true);
    assert_eq!(rgb.kind.type_name(), "RgbLed");
    s.add_saved_item(rgb);

    // SVG Export verification
    let pal = Palette::light();
    let svg = svg_string(&canvas, &pal);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn test_all_21_adjusted_component_bounding_rects() {
    use cs_engine::canvas::Rect;

    // 1. FreqMeter
    let freq = Item::freq_meter("FM-1", 0.0, 0.0, 1.0);
    assert_eq!(freq.body_rect(), Rect::new(-32.0, -10.0, 64.0, 20.0));

    // 2. SwitchDip
    let sw_dip = Item::switch_dip("SD-1", 0.0, 0.0, 4, 0, false);
    assert_eq!(sw_dip.body_rect(), Rect::new(-3.0, -28.0, 14.0, 32.0));

    // 3. ResistorDip
    let res_dip = Item::resistor_dip("RD-1", 0.0, 0.0, 4, 1000.0, false);
    assert_eq!(res_dip.body_rect(), Rect::new(-9.0, -28.0, 18.0, 32.0));

    // 4. KeyPad
    let keypad = Item::keypad("KP-1", 0.0, 0.0, 4, 3);
    assert_eq!(keypad.body_rect(), Rect::new(-12.0, -4.0, 56.0, 72.0));

    // 5. Relay
    let relay = Item::relay("RL-1", 0.0, 0.0, false, false, 1, 0.02, 0.01, false);
    assert_eq!(relay.body_rect(), Rect::new(-12.0, -28.0, 24.0, 36.0));

    // 6. Csource
    let csource = Item::csource("CS-1", 0.0, 0.0, true, false, false, 1.0, 0.0, 0.0);
    assert_eq!(csource.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));

    // 7. Diac
    let diac = Item::diac("DC-1", 0.0, 0.0, 30.0);
    assert_eq!(diac.body_rect(), Rect::new(-8.0, -16.0, 16.0, 32.0));

    // 8. Triac
    let triac = Item::triac("TR-1", 0.0, 0.0, 0.7, 0.01);
    assert_eq!(triac.body_rect(), Rect::new(-8.0, -16.0, 16.0, 32.0));

    // 8b. Scr
    let scr = Item::scr("SCR-1", 0.0, 0.0, 0.7, 0.01);
    assert_eq!(scr.body_rect(), Rect::new(-10.0, -8.0, 20.0, 16.0));

    // 9. Bjt
    let bjt = Item::bjt("BJT-1", 0.0, 0.0, false);
    assert_eq!(bjt.body_rect(), Rect::new(-12.0, -14.0, 28.0, 28.0));

    // 10. Mosfet
    let mosfet = Item::mosfet("MOS-1", 0.0, 0.0, false, false);
    assert_eq!(mosfet.body_rect(), Rect::new(-12.0, -14.0, 28.0, 28.0));

    // 11. Jfet
    let jfet = Item::jfet("JFET-1", 0.0, 0.0);
    assert_eq!(jfet.body_rect(), Rect::new(-12.0, -14.0, 28.0, 28.0));

    // 12. VoltReg
    let voltreg = Item::volt_reg("VR-1", 0.0, 0.0);
    assert_eq!(voltreg.body_rect(), Rect::new(-11.0, -8.0, 22.0, 16.0));

    // 13. Led
    let led = Item::led("LED-1", 0.0, 0.0);
    assert_eq!(led.body_rect(), Rect::new(-6.0, -8.0, 16.0, 16.0));

    // 14. RgbLed
    let rgb = Item::rgb_led("RGB-1", 0.0, 0.0, false);
    assert_eq!(rgb.body_rect(), Rect::new(-10.0, -10.0, 20.0, 20.0));
    let rgb_pins = rgb.pins();
    assert_eq!(rgb_pins.len(), 4);
    assert_eq!(rgb_pins[0].length, 6.0);
    assert_eq!(rgb_pins[3].length, 6.0);

    // 15. LedBar
    let led_bar = Item::led_bar("LB-1", 0.0, 0.0, 10);
    assert_eq!(led_bar.body_rect(), Rect::new(-8.0, -28.0, 16.0, 80.0));

    // 16. LedMatrix
    let matrix = Item::led_matrix("LM-1", 0.0, 0.0, 8, 8);
    assert_eq!(matrix.body_rect(), Rect::new(-8.0, -8.0, 72.0, 72.0));

    // 17. Max72xx
    let max = Item::max72xx("MX-1", 0.0, 0.0, 2);
    assert_eq!(max.body_rect(), Rect::new(-36.0, -44.0, 136.0, 88.0));
    let max_pins = max.pins();
    assert_eq!(max_pins[0].local.x, -44.0);
    assert_eq!(max_pins[0].length, 8.0);

    // 18. Ws2812
    let ws = Item::ws2812("WS-1", 0.0, 0.0, 8);
    assert_eq!(ws.body_rect(), Rect::new(-6.0, -6.0, 96.0, 12.0));

    // 19. Servo
    let servo = Item::servo("SV-1", 0.0, 0.0, 0.2, 1000.0, 2000.0);
    assert_eq!(servo.body_rect(), Rect::new(-40.0, -24.0, 80.0, 48.0));
    let servo_pins = servo.pins();
    assert_eq!(servo_pins[0].length, 8.0);

    // 20. AudioOut
    let audio = Item::audio_out("AO-1", 0.0, 0.0, 8.0);
    assert_eq!(audio.body_rect(), Rect::new(-10.0, -24.0, 20.0, 40.0));

    // 21. Lamp
    let lamp = Item::lamp("LP-1", 0.0, 0.0, 12.0, 5.0, 2.0);
    assert_eq!(lamp.body_rect(), Rect::new(-8.0, -8.0, 16.0, 16.0));

    // 22. TestUnit (width 32, dynamic height, pin stems contact border at x = ±16.0)
    let tu = Item::test_unit("TU-1", 0.0, 0.0, "O", "I0,I1", 1e-7, Vec::new());
    assert_eq!(tu.body_rect(), Rect::new(-16.0, -12.0, 32.0, 24.0));
    let tu_pins = tu.pins();
    assert_eq!(tu_pins.len(), 3);
    // Left pins (in0, in1): x=-24, length=8 -> base reaches x = -24 + 8 = -16.0 (left border)
    assert_eq!(tu_pins[0].local.x, -24.0);
    assert_eq!(tu_pins[0].length, 8.0);
    // Right pin (out0): x=24, length=8 -> base reaches x = 24 - 8 = 16.0 (right border)
    assert_eq!(tu_pins[2].local.x, 24.0);
    assert_eq!(tu_pins[2].length, 8.0);
}

#[test]
fn test_all_23_digital_logic_components() {
    // 1. Counter
    let ctr = Item::counter("CTR", 0.0, 0.0, 4, 16);
    assert_eq!(ctr.body_rect(), Rect::new(-12.0, -12.0, 24.0, 24.0));
    let ctr_pins = ctr.pins();
    assert_eq!(ctr_pins[0].local.x, -20.0); // clk left pin touches x = -12
    assert_eq!(ctr_pins[2].local.x, 20.0); // Q right pin touches x = 12

    // 2. BinCounter
    let bc = Item::bin_counter("BC", 0.0, 0.0, false);
    assert_eq!(bc.body_rect(), Rect::new(-16.0, -24.0, 32.0, 48.0));
    let bc_pins = bc.pins();
    assert_eq!(bc_pins[0].local.x, -24.0); // clk touches x = -16
    assert_eq!(bc_pins[2].local.x, 24.0); // q0 touches x = 16

    // 3. FullAdder
    let fa = Item::full_adder("FA", 0.0, 0.0, 4);
    assert_eq!(fa.body_rect(), Rect::new(-12.0, -40.0, 24.0, 80.0));
    let fa_pins = fa.pins();
    assert_eq!(fa_pins[0].local.x, -20.0); // A0 touches x = -12
    assert_eq!(fa_pins[8].local.x, 20.0); // Ci touches x = 12
    assert_eq!(fa_pins[9].local.x, 20.0); // S0 touches x = 12

    // 4. MagnitudeComp
    let mc = Item::magnitude_comp("MC", 0.0, 0.0, 4);
    assert_eq!(mc.body_rect(), Rect::new(-16.0, -48.0, 32.0, 96.0));
    let mc_pins = mc.pins();
    assert_eq!(mc_pins[0].local.x, -24.0); // iA>B touches x = -16
    assert_eq!(mc_pins[11].local.x, 24.0); // A>B touches x = 16

    // 5. ShiftReg
    let sr = Item::shift_reg("SR", 0.0, 0.0, 8);
    assert_eq!(sr.body_rect(), Rect::new(-16.0, -36.0, 32.0, 72.0));
    let sr_pins = sr.pins();
    assert_eq!(sr_pins[0].local.x, -24.0); // DI touches x = -16
    assert_eq!(sr_pins[4].local.x, 24.0); // Q0 touches x = 16

    // 6. Function
    let func = Item::function("FN", 0.0, 0.0, 4, "A & B");
    assert_eq!(func.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    let func_pins = func.pins();
    assert_eq!(func_pins[0].local.x, -24.0); // I0 touches x = -16
    assert_eq!(func_pins[4].local.x, 24.0); // Out touches x = 16

    // 7. FlipFlop D
    let ffd = Item::flipflop("FFD", 0.0, 0.0, "D", true, "Clock");
    assert_eq!(ffd.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    let ffd_pins = ffd.pins();
    assert_eq!(ffd_pins[0].local.x, -24.0); // D touches x = -16
    assert_eq!(ffd_pins[2].local.x, 24.0); // Q touches x = 16
    assert_eq!(ffd_pins[4].local.y, -24.0); // S touches y = -16
    assert_eq!(ffd_pins[5].local.y, 24.0); // R touches y = 16

    // 8. FlipFlop JK
    let ffjk = Item::flipflop("FFJK", 0.0, 0.0, "JK", true, "Clock");
    assert_eq!(ffjk.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    let ffjk_pins = ffjk.pins();
    assert_eq!(ffjk_pins[0].local.x, -24.0); // J touches x = -16
    assert_eq!(ffjk_pins[3].local.x, 24.0); // Q touches x = 16

    // 9. FlipFlop RS
    let ffrs = Item::flipflop("FFRS", 0.0, 0.0, "RS", false, "Clock");
    assert_eq!(ffrs.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    let ffrs_pins = ffrs.pins();
    assert_eq!(ffrs_pins[0].local.x, -24.0); // S touches x = -16
    assert_eq!(ffrs_pins[3].local.x, 24.0); // Q touches x = 16

    // 10. FlipFlop T
    let fft = Item::flipflop("FFT", 0.0, 0.0, "T", true, "Clock");
    assert_eq!(fft.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    let fft_pins = fft.pins();
    assert_eq!(fft_pins[0].local.x, -24.0); // T touches x = -16
    assert_eq!(fft_pins[2].local.x, 24.0); // Q touches x = 16
    // 11. SevenSegmentBCD
    let ssbcd = Item::seven_segment_bcd("7SBCD", 0.0, 0.0, "Red", false);
    assert_eq!(ssbcd.body_rect(), Rect::new(-16.0, -24.0, 32.0, 48.0));
    let ssbcd_pins = ssbcd.pins();
    assert_eq!(ssbcd_pins[0].local.y, 32.0); // BCD 1 touches y = 24
    assert_eq!(ssbcd_pins[0].label, "1");

    // 12. LM555
    let lm = Item::lm555("555", 0.0, 0.0);
    assert_eq!(lm.body_rect(), Rect::new(-16.0, -20.0, 32.0, 40.0));
    let lm_pins = lm.pins();
    assert_eq!(lm_pins[0].local.x, -24.0); // Gnd touches x = -16
    assert_eq!(lm_pins[4].local.x, 24.0); // CV touches x = 16

    // 13. DAC
    let dac = Item::dac("DAC", 0.0, 0.0, 8, 5.0);
    assert_eq!(dac.body_rect(), Rect::new(-16.0, -36.0, 32.0, 72.0));
    let dac_pins = dac.pins();
    assert_eq!(dac_pins[0].local.x, -24.0); // 0 touches x = -16
    assert_eq!(dac_pins[8].local.x, 24.0); // Out touches x = 16

    // 14. ADC
    let adc = Item::adc("ADC", 0.0, 0.0, 8, 5.0, 0.0);
    assert_eq!(adc.body_rect(), Rect::new(-16.0, -36.0, 32.0, 72.0));
    let adc_pins = adc.pins();
    assert_eq!(adc_pins[0].local.x, -24.0); // In touches x = -16
    assert_eq!(adc_pins[1].local.x, 24.0); // 0 touches x = 16

    // 15. I2CToParallel
    let i2cp = Item::i2c_to_parallel("I2CP", 0.0, 0.0, 0x20);
    assert_eq!(i2cp.body_rect(), Rect::new(-16.0, -32.0, 32.0, 72.0));
    let i2cp_pins = i2cp.pins();
    assert_eq!(i2cp_pins[0].local.x, -24.0); // SDA touches x = -16
    assert_eq!(i2cp_pins[0].label, "SDA");
    assert_eq!(i2cp_pins[1].label, "SCL");
    assert_eq!(i2cp_pins[2].label, "INT");
    assert_eq!(i2cp_pins[6].local.x, 24.0); // D0 touches x = 16
    assert_eq!(i2cp_pins[6].label, "D0");

    // 16. BcdTo7S
    let b7s = Item::bcd_to_7s("B7S", 0.0, 0.0, false);
    assert_eq!(b7s.body_rect(), Rect::new(-16.0, -32.0, 32.0, 64.0));
    let b7s_pins = b7s.pins();
    assert_eq!(b7s_pins[0].local.x, -24.0); // S0 touches x = -16
    assert_eq!(b7s_pins[0].label, "S0");
    assert_eq!(b7s_pins[4].local.y, -40.0); // OE at top
    assert_eq!(b7s_pins[4].label, "OE");
    assert_eq!(b7s_pins[5].local.x, 24.0); // a touches x = 16
    assert_eq!(b7s_pins[5].label, "a");

    // 17. DecToBcd
    let d2b = Item::dec_to_bcd("D2B", 0.0, 0.0, false, false);
    assert_eq!(d2b.body_rect(), Rect::new(-16.0, -44.0, 32.0, 88.0));
    let d2b_pins = d2b.pins();
    assert_eq!(d2b_pins[0].local.x, -24.0); // D1 touches x = -16
    assert_eq!(d2b_pins[9].local.y, -52.0); // OE top pin
    assert_eq!(d2b_pins[10].local.x, 24.0); // A touches x = 16

    // 18. BcdToDec
    let b2d = Item::bcd_to_dec("B2D", 0.0, 0.0, false, false);
    assert_eq!(b2d.body_rect(), Rect::new(-16.0, -44.0, 32.0, 88.0));
    let b2d_pins = b2d.pins();
    assert_eq!(b2d_pins[0].local.x, -24.0); // S0 touches x = -16
    assert_eq!(b2d_pins[4].local.y, -52.0); // OE top pin
    assert_eq!(b2d_pins[5].local.x, 24.0); // 0 touches x = 16

    // 19. Demux
    let dmx = Item::demux("DMX", 0.0, 0.0, 3, false);
    assert_eq!(dmx.body_rect(), Rect::new(-16.0, -46.0, 32.0, 92.0));
    let dmx_pins = dmx.pins();
    assert_eq!(dmx_pins[0].local.x, -24.0); // D touches x = -16
    assert_eq!(dmx_pins[1].local.y, 48.0); // S0 on line y = 48
    assert_eq!(dmx_pins[2].local.y, 48.0); // S1 on line y = 48
    assert_eq!(dmx_pins[3].local.y, 48.0); // S2 on line y = 48
    assert_eq!(dmx_pins[5].local.x, 24.0); // D0 touches x = 16
    assert_eq!(dmx_pins[5].label, "D0");

    // 20. Mux
    let mux = Item::mux("MUX", 0.0, 0.0, 3);
    assert_eq!(mux.body_rect(), Rect::new(-16.0, -46.0, 32.0, 92.0));
    let mux_pins = mux.pins();
    assert_eq!(mux_pins[0].local.x, -24.0); // D0 touches x = -16
    assert_eq!(mux_pins[0].label, "D0");
    assert_eq!(mux_pins[8].local.y, 48.0); // S0 on line y = 48
    assert_eq!(mux_pins[9].local.y, 48.0); // S1 on line y = 48
    assert_eq!(mux_pins[10].local.y, 48.0); // S2 on line y = 48
    assert_eq!(mux_pins[12].local.x, 24.0); // Y touches x = 16

    // 21. I2CRam
    let i2cr = Item::i2c_ram("I2CR", 0.0, 0.0, 256, 0x50, Vec::new());
    assert_eq!(i2cr.body_rect(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    let i2cr_pins = i2cr.pins();
    assert_eq!(i2cr_pins[0].local.x, -24.0); // SDA touches x = -16
    assert_eq!(i2cr_pins[2].local.x, 24.0); // A0 touches x = 16

    // 22. DynamicMemory
    let dram = Item::dynamic_memory("DRAM", 0.0, 0.0, 8, Vec::new());
    assert_eq!(dram.body_rect(), Rect::new(-16.0, -40.0, 32.0, 88.0));
    let dram_pins = dram.pins();
    assert_eq!(dram_pins[0].local.x, -24.0); // in0 touches x = -16
    assert_eq!(dram_pins[10].local.x, 24.0); // out0 touches x = 16

    // 23. Memory
    let ram = Item::memory("RAM", 0.0, 0.0, 8, 8, false, Vec::new());
    assert_eq!(ram.body_rect(), Rect::new(-16.0, -40.0, 32.0, 88.0));
    let ram_pins = ram.pins();
    assert_eq!(ram_pins[0].local.x, -24.0); // in0 touches x = -16
    assert_eq!(ram_pins[10].local.x, 24.0); // out0 touches x = 16
}

#[test]
fn test_rotated_logic_component_labels_and_orientation() {
    let mut canvas = Canvas::default();
    canvas
        .scene_mut()
        .add_saved_item(Item::shift_reg("SR1", 0.0, 0.0, 8));
    canvas
        .scene_mut()
        .add_saved_item(Item::magnitude_comp("MC1", 100.0, 0.0, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::function("FN1", 200.0, 0.0, 4, "A & B"));
    canvas
        .scene_mut()
        .add_saved_item(Item::bin_counter("BC1", 300.0, 0.0, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::full_adder("FA1", 400.0, 0.0, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::counter("CTR1", 500.0, 0.0, 4, 16));

    let pal = Palette::light();
    let svg = svg_string(&canvas, &pal);

    assert!(svg.contains("rotate(-90)"));
    assert!(svg.contains(">SHIFT REG<"));
    assert!(svg.contains(">MAG COMP<"));
    assert!(svg.contains(">FUNC<"));
    assert!(svg.contains(">BIN CTR<"));
    assert!(svg.contains(">ADDER<"));
    assert!(svg.contains(">CTR<"));
    assert!(!svg.contains(">COUNTER<"));
    assert!(!svg.contains(">SHIFT\nREG<"));
    assert!(!svg.contains(">MAG\nCOMP<"));
    assert!(!svg.contains(">BIN\nCTR<"));
}

#[test]
fn test_dht22_ds1621_ds18b20_sdcard_geometry_and_pin_contact() {
    use cs_engine::canvas::{Item, Rect};

    // 1. DHT22
    let dht = Item::dht22("DHT-1", 0.0, 0.0, "DHT22", 22.5, 50.0, 0.5, 1.0);
    assert_eq!(dht.body_rect(), Rect::new(-20.0, -40.0, 40.0, 68.0));
    // Bottom edge is at y = -40.0 + 68.0 = 28.0
    let dht_pins = dht.pins();
    assert_eq!(dht_pins.len(), 4);
    for pin in &dht_pins {
        assert_eq!(pin.local.y, 32.0);
        assert_eq!(pin.angle, 270);
        assert_eq!(pin.length, 4.0);
        // Pin tip at 32.0; with angle 270 (stem extends upwards along -Y by 4.0), base touches y = 28.0
    }

    // 2. DS1621
    let ds1621 = Item::ds1621("DS1621-1", 0.0, 0.0, 25.0, 0.5);
    assert_eq!(ds1621.body_rect(), Rect::new(-20.0, -28.0, 40.0, 56.0));
    // Left edge at x = -20.0, right edge at x = 20.0
    let ds1621_pins = ds1621.pins();
    assert_eq!(ds1621_pins.len(), 8);
    for pin in &ds1621_pins[0..4] {
        // Left pins (angle 180): tip at x = -24.0, length 4.0, stem extends to x = -20.0
        assert_eq!(pin.local.x, -24.0);
        assert_eq!(pin.angle, 180);
        assert_eq!(pin.length, 4.0);
    }
    for pin in &ds1621_pins[4..8] {
        // Right pins (angle 0): tip at x = 24.0, length 4.0, stem extends to x = 20.0
        assert_eq!(pin.local.x, 24.0);
        assert_eq!(pin.angle, 0);
        assert_eq!(pin.length, 4.0);
    }

    // 3. DS18B20
    let ds18 = Item::ds18b20("DS18-1", 0.0, 0.0, "28FF2B450000", 25.0, 0.5);
    assert_eq!(ds18.body_rect(), Rect::new(-20.0, -20.0, 40.0, 40.0));
    // Bottom edge is at y = -20.0 + 40.0 = 20.0
    let ds18_pins = ds18.pins();
    assert_eq!(ds18_pins.len(), 3);
    for pin in &ds18_pins {
        assert_eq!(pin.local.y, 24.0);
        assert_eq!(pin.angle, 270);
        assert_eq!(pin.length, 4.0);
        // Pin tip at 24.0; with angle 270 (stem extends upwards along -Y by 4.0), base touches y = 20.0
    }

    // 4. SdCard
    let sd = Item::sdcard("SD-1", 0.0, 0.0, "");
    assert_eq!(sd.body_rect(), Rect::new(-24.0, -16.0, 56.0, 40.0));
    // Left edge is at x = -24.0
    let sd_pins = sd.pins();
    assert_eq!(sd_pins.len(), 4);
    for pin in &sd_pins {
        assert_eq!(pin.local.x, -32.0);
        assert_eq!(pin.angle, 180);
        assert_eq!(pin.length, 8.0);
        // Pin tip at -32.0; with angle 180 (stem extends rightwards along +X by 8.0), base touches x = -24.0
    }
}

#[test]
fn test_potentiometer_props_and_arrow_keys() {
    let mut pot = Item::potentiometer("POT-1", 0.0, 0.0, 1000.0, 0.5);
    assert_eq!(pot.prop_text("Value_Ohm"), Some("500 Ω".to_string()));
    pot.set_prop_text("Value_Ohm", "250 Ω");
    assert!((pot.wiper() - 0.25).abs() < 1e-6);
    assert_eq!(pot.prop_text("Value_Ohm"), Some("250 Ω".to_string()));

    let mut canvas = cs_engine::canvas::Canvas::new();
    canvas.scene_mut().add_saved_item(pot);
    canvas.scene_mut().select_only(0);

    use cs_engine::canvas::{KEY_LEFT, KEY_RIGHT};
    // Arrow right should increase wiper
    canvas.key_press(KEY_RIGHT, 0);
    assert!((canvas.scene().items()[0].wiper() - 0.26).abs() < 1e-6);

    // Arrow left should decrease wiper
    canvas.key_press(KEY_LEFT, 0);
    assert!((canvas.scene().items()[0].wiper() - 0.25).abs() < 1e-6);
}

#[test]
fn test_voltreg_pin_contact() {
    use cs_engine::canvas::{Item, Rect};
    let reg = Item::volt_reg("VR-1", 0.0, 0.0);
    let body = reg.body_rect();
    assert_eq!(body, Rect::new(-11.0, -8.0, 22.0, 16.0));
    // The bottom edge of the body is at y = 8.0
    let pins = reg.pins();
    let ref_pin = pins.iter().find(|p| p.id.ends_with("-ref")).unwrap();
    assert_eq!(ref_pin.local.x, 0.0);
    assert_eq!(ref_pin.local.y, 16.0);
    assert_eq!(ref_pin.angle, 270);
    assert_eq!(ref_pin.length, 8.0);
    // Pin stem: starts at y = 16.0 and extends upwards by length 8.0 to y = 8.0 (exactly touches body bottom border)
    assert_eq!(ref_pin.local.y - ref_pin.length, body.y + body.h);
}

#[test]
fn test_ws2812_geometry_and_pin_contact() {
    use cs_engine::canvas::{Item, Rect};
    let mut ws = Item::ws2812("WS-1", 0.0, 0.0, 8);
    assert_eq!(ws.rows(), 1);
    assert_eq!(ws.cols(), 8);
    let body = ws.body_rect();
    assert_eq!(body, Rect::new(-6.0, -6.0, 96.0, 12.0));

    let pins = ws.pins();
    let din = pins.iter().find(|p| p.id.ends_with("-din")).unwrap();
    let vcc = pins.iter().find(|p| p.id.ends_with("-vcc")).unwrap();
    let dout = pins.iter().find(|p| p.id.ends_with("-dout")).unwrap();
    let gnd = pins.iter().find(|p| p.id.ends_with("-gnd")).unwrap();

    // DIN and VCC on left at x = -16.0, angle 180, length 10.0 touching body left border at x = -6.0
    assert_eq!(din.local.x, -16.0);
    assert_eq!(din.angle, 180);
    assert_eq!(din.length, 10.0);
    assert_eq!(din.local.x + din.length, body.x);

    assert_eq!(vcc.local.x, -16.0);
    assert_eq!(vcc.angle, 180);
    assert_eq!(vcc.length, 10.0);
    assert_eq!(vcc.local.x + vcc.length, body.x);

    // DOUT and GND on right at x = 96.0, angle 0, length 6.0 touching body right border at x = 90.0 (-6 + 96)
    assert_eq!(dout.local.x, 96.0);
    assert_eq!(dout.angle, 0);
    assert_eq!(dout.length, 6.0);
    assert_eq!(dout.local.x - dout.length, body.x + body.w);

    assert_eq!(gnd.local.x, 96.0);
    assert_eq!(gnd.angle, 0);
    assert_eq!(gnd.length, 6.0);
    assert_eq!(gnd.local.x - gnd.length, body.x + body.w);

    // Update rows and cols
    assert!(ws.set_prop_text("Rows", "2"));
    assert!(ws.set_prop_text("Cols", "16"));
    assert_eq!(ws.rows(), 2);
    assert_eq!(ws.cols(), 16);
    let body2 = ws.body_rect();
    assert_eq!(body2, Rect::new(-6.0, -6.0, 192.0, 24.0));
    let pins2 = ws.pins();
    let dout2 = pins2.iter().find(|p| p.id.ends_with("-dout")).unwrap();
    assert_eq!(dout2.local.x, 192.0);
    assert_eq!(dout2.local.x - dout2.length, body2.x + body2.w);
}

#[test]
fn test_show_button_property_aliases() {
    use cs_engine::canvas::Item;

    // Push
    let mut push = Item::push("PUSH-1", 0.0, 0.0, false, 1);
    assert_eq!(push.prop_bool("ShowButton"), Some(false));
    assert_eq!(push.prop_bool("show_button"), Some(false));
    assert_eq!(push.prop_bool("Show_Button"), Some(false));
    assert_eq!(push.prop_text("ShowButton"), Some("false".into()));
    assert_eq!(push.prop_text("show_button"), Some("false".into()));

    assert!(push.set_prop_bool("show_button", true));
    assert_eq!(push.prop_bool("ShowButton"), Some(true));
    assert_eq!(push.prop_bool("show_button"), Some(true));

    assert!(push.set_prop_text("Show_Button", "false"));
    assert_eq!(push.prop_bool("ShowButton"), Some(false));

    // Switch
    let mut sw = Item::switch("SW-1", 0.0, 0.0, false);
    assert_eq!(sw.prop_bool("ShowButton"), Some(false));
    assert_eq!(sw.prop_bool("show_button"), Some(false));
    assert_eq!(sw.prop_bool("Show_Button"), Some(false));

    assert!(sw.set_prop_bool("Show_Button", true));
    assert_eq!(sw.prop_bool("ShowButton"), Some(true));
    assert_eq!(sw.prop_bool("show_button"), Some(true));

    assert!(sw.set_prop_text("show_button", "0"));
    assert_eq!(sw.prop_bool("ShowButton"), Some(false));
    assert!(!sw.show_button());

    assert!(sw.set_prop_bool("ShowButton", true));
    assert!(sw.show_button());
}

#[test]
fn test_push_button_momentary_and_norm_close() {
    use cs_engine::canvas::Canvas;
    use cs_engine::canvas::scene::Item;

    let mut push = Item::push("PB-1", 0.0, 0.0, false, 1);
    assert!(!push.pressed());

    // Set norm_close
    assert!(push.set_prop_bool("Norm_Close", true));

    // Toggle switch should not latch a push button
    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(push);
    let idx = canvas.scene().items().len() - 1;
    assert!(
        !canvas.scene_mut().toggle_switch(idx),
        "Push button must not latch on toggle_switch"
    );

    // Momentary press via set_push_state
    canvas.set_push_state("PB-1", true);
    assert!(canvas.scene().items()[idx].pressed());
    canvas.set_push_state("PB-1", false);
    assert!(!canvas.scene().items()[idx].pressed());

    // Canvas mouse press and release
    let p = canvas
        .viewport()
        .map_from_circuit(cs_engine::canvas::Point::zero());
    canvas.mouse_press(1, p.x, p.y, 0);
    assert!(
        canvas.scene().items()[idx].pressed(),
        "Mouse press must activate push button"
    );
    canvas.mouse_release(1, p.x, p.y, 0);
    assert!(
        !canvas.scene().items()[idx].pressed(),
        "Mouse release must release push button"
    );

    // Sim1 with Checked="true" must not load as pressed
    let sim1 = "<circuit>\n<item itemtype=\"Push\" CircId=\"Push-1\" Poles=\"1\" Norm_Close=\"false\" Checked=\"true\" Pos=\"0,0\" />\n</circuit>";
    let s = cs_engine::canvas::scene::Scene::from_sim1(sim1).expect("sim1 parse");
    let loaded_push = s.item_by_id("Push-1").expect("Push-1 must exist");
    assert!(
        !loaded_push.pressed(),
        "Push button must never load in pressed state even if Checked=true was in file"
    );
}

#[test]
fn test_potentiometer_simulation_and_pin_aliases() {
    use cs_engine::canvas::scene::{Item, Scene};
    use cs_engine::canvas::{Canvas, Point};

    let pot = Item::potentiometer("POT-1", 0.0, 0.0, 10_000.0, 0.5);
    assert_eq!(pot.source_value(), 5_000.0);
    assert_eq!(pot.prop_text("Value_Ohm"), Some("5 kΩ".into()));

    // Test with canvas scene pin naming (-lPin, -wPin, -rPin)
    let mut scene = Scene::new();
    let p = Point::new(0.0, 0.0);
    scene.add_saved_item(Item::fixed_volt("FV-1", 0.0, 0.0, 10.0));
    scene.add_saved_item(Item::potentiometer("POT-1", 0.0, 0.0, 10_000.0, 0.5));
    scene.add_saved_item(Item::resistor("R-LOAD", 0.0, 0.0, 1_000_000.0));
    scene.add_saved_item(Item::ground("GND-1", 0.0, 0.0));
    scene.connect_pins("FV-1-outnod", p, "POT-1-lPin", p);
    scene.connect_pins("GND-1-Gnd", p, "POT-1-rPin", p);
    scene.connect_pins("POT-1-wPin", p, "R-LOAD-lPin", p);
    scene.connect_pins("R-LOAD-rPin", p, "GND-1-Gnd", p);
    let mut c = scene.to_circuit();
    let _ = c.solve();
    let vw = c.pin_voltage("POT-1-wPin").unwrap_or(0.0);
    assert!(
        (vw - 5.0).abs() < 0.1,
        "Wiper at 50% should divide 10V to ~5V, got {vw}"
    );

    // Test Canvas set_pot_wiper
    let mut canvas = Canvas::new();
    canvas
        .scene_mut()
        .add_saved_item(Item::potentiometer("POT-1", 0.0, 0.0, 10_000.0, 0.5));
    canvas.set_pot_wiper("POT-1", 0.8);
    let item = canvas.scene().item_by_id("POT-1").unwrap();
    assert!((item.wiper() - 0.8).abs() < 1e-5);
    assert_eq!(item.source_value(), 8_000.0);
}

#[test]
fn test_keypad_simulation_connectivity() {
    use cs_engine::canvas::scene::{Item, Scene};
    use cs_engine::canvas::{Canvas, Point};

    let mut scene = Scene::new();
    let p = Point::new(0.0, 0.0);
    scene.add_saved_item(Item::fixed_volt("FV-1", 0.0, 0.0, 5.0));
    let mut kp = Item::keypad("KP-1", 0.0, 0.0, 4, 4);
    if let cs_engine::canvas::Part::KeyPad(ref mut keypad) = kp.kind {
        keypad.pressed = Some((1, 2)); // Row 1, Col 2 -> Pin1 to Pin6
    }
    scene.add_saved_item(kp);
    scene.add_saved_item(Item::resistor("R-1", 0.0, 0.0, 1_000.0));
    scene.add_saved_item(Item::ground("GND-1", 0.0, 0.0));
    scene.connect_pins("FV-1-outnod", p, "KP-1-Pin1", p);
    scene.connect_pins("KP-1-Pin6", p, "R-1-lPin", p);
    scene.connect_pins("R-1-rPin", p, "GND-1-gndnod", p);
    let mut c = scene.to_circuit();
    let _ = c.solve();
    let v_col = c.pin_voltage("KP-1-Pin6").unwrap_or(0.0);
    assert!(
        (v_col - 5.0).abs() < 0.1,
        "Closed keypad key should pass 5V, got {v_col}"
    );

    // Canvas set_keypad_pressed
    let mut canvas = Canvas::new();
    let kp = Item::keypad("KP-1", 0.0, 0.0, 4, 4);
    canvas.scene_mut().add_saved_item(kp);
    canvas.set_keypad_pressed("KP-1", 2, 3, true);
    if let cs_engine::canvas::Part::KeyPad(ref keypad) =
        canvas.scene().item_by_id("KP-1").unwrap().kind
    {
        assert_eq!(keypad.pressed, Some((2, 3)));
    } else {
        panic!("Expected KeyPad");
    }
    canvas.set_keypad_pressed("KP-1", 2, 3, false);
    if let cs_engine::canvas::Part::KeyPad(ref keypad) =
        canvas.scene().item_by_id("KP-1").unwrap().kind
    {
        assert_eq!(keypad.pressed, None);
    }
}

#[test]
fn test_jfet_source_pin_length_and_contact() {
    use cs_engine::canvas::Item;

    let jfet = Item::jfet("JFET-1", 0.0, 0.0);
    let pins = jfet.pins();
    let sour = pins
        .iter()
        .find(|p| p.id.ends_with("-Sour"))
        .expect("Source pin must exist");
    assert_eq!(
        sour.length, 8.0,
        "JFET source pin length must be 8.0 to touch the internal lead"
    );
}

#[test]
fn test_ws2812_pin_names_are_hidden() {
    use cs_engine::canvas::Item;

    let ws = Item::ws2812("WS-1", 0.0, 0.0, 8);
    for pin in ws.pins() {
        assert!(
            pin.label.is_empty(),
            "WS2812 pin label must be empty so text does not appear on component body, got {:?}",
            pin.label
        );
    }
}

#[test]
fn test_ds1307_esp01_ds1621_pin_names_are_hidden() {
    use cs_engine::canvas::Item;

    let ds1307 = Item::ds1307("DS1307-1", 0.0, 0.0, true);
    for pin in ds1307.pins() {
        assert!(
            pin.label.is_empty(),
            "DS1307 pin label must be empty so text does not appear on canvas, got {:?}",
            pin.label
        );
    }

    let esp01 = Item::esp01("ESP-1", 0.0, 0.0, 115200, false);
    for pin in esp01.pins() {
        assert!(
            pin.label.is_empty(),
            "ESP01 pin label must be empty so text does not appear on canvas, got {:?}",
            pin.label
        );
    }

    let ds1621 = Item::ds1621("DS1621-1", 0.0, 0.0, 25.0, 0.5);
    for pin in ds1621.pins() {
        assert!(
            pin.label.is_empty(),
            "DS1621 pin label must be empty so text does not appear on canvas, got {:?}",
            pin.label
        );
    }
}

#[test]
fn test_switch_and_push_multipole_geometry_and_selection_rect() {
    use cs_engine::canvas::Rect;
    use cs_engine::canvas::scene::Item;

    for poles in 1..=4 {
        let push = Item::push("PB", 0.0, 0.0, false, poles);
        let sw = {
            let mut it = Item::switch("SW", 0.0, 0.0, false);
            it.set_prop_text("Poles", &poles.to_string());
            it
        };

        let expected_h = 16.0 * (poles as f64);
        let expected_y = 8.0 - expected_h;
        let expected_rect = Rect::new(-12.0, expected_y, 24.0, expected_h);

        assert_eq!(
            push.body_rect(),
            expected_rect,
            "Push {poles} poles body_rect must match C++ upward expansion"
        );
        assert_eq!(
            sw.body_rect(),
            expected_rect,
            "Switch {poles} poles body_rect must match C++ upward expansion"
        );

        // Verify selection rect contains all pin contact points
        let push_sel = push.local_selection_rect();
        let sw_sel = sw.local_selection_rect();
        for i in 0..poles {
            let pin_y = -16.0 * (i as f64);
            assert!(
                push_sel.contains_point(cs_engine::canvas::Point::new(0.0, pin_y)),
                "Push selection rect must contain pole {i} at y={pin_y}"
            );
            assert!(
                sw_sel.contains_point(cs_engine::canvas::Point::new(0.0, pin_y)),
                "Switch selection rect must contain pole {i} at y={pin_y}"
            );
        }
    }
}

#[test]
fn test_switch_multipole_pins_and_double_throw() {
    use cs_engine::canvas::scene::Item;

    let mut sw = Item::switch("SW-1", 0.0, 0.0, false);
    assert_eq!(sw.pins().len(), 2, "1-pole SPST switch must have 2 pins");
    assert_eq!(sw.pins()[0].id, "SW-1-pinP0");
    assert_eq!(sw.pins()[1].id, "SW-1-switch0pinN");

    // Change poles to 3
    assert!(sw.set_prop_text("Poles", "3"));
    assert_eq!(sw.poles(), 3);
    assert_eq!(sw.pins().len(), 6, "3-pole SPST switch must have 6 pins");
    assert_eq!(sw.pins()[0].id, "SW-1-pinP0");
    assert_eq!(sw.pins()[1].id, "SW-1-switch0pinN");
    assert_eq!(sw.pins()[2].id, "SW-1-pinP1");
    assert_eq!(sw.pins()[3].id, "SW-1-switch1pinN");
    assert_eq!(sw.pins()[4].id, "SW-1-pinP2");
    assert_eq!(sw.pins()[5].id, "SW-1-switch2pinN");

    // Enable double-throw on 3-pole switch (3-pole DPDT -> 3 * 3 = 9 pins)
    assert!(sw.set_prop_bool("Double_Throw", true));
    assert!(sw.double_throw());
    assert_eq!(sw.pins().len(), 9, "3-pole DPDT switch must have 9 pins");
    assert_eq!(sw.pins()[0].id, "SW-1-pinP0");
    assert_eq!(sw.pins()[1].id, "SW-1-switch0pinN");
    assert_eq!(sw.pins()[2].id, "SW-1-switch1pinN");
    assert_eq!(sw.pins()[3].id, "SW-1-pinP1");
    assert_eq!(sw.pins()[4].id, "SW-1-switch2pinN");
    assert_eq!(sw.pins()[5].id, "SW-1-switch3pinN");
    assert_eq!(sw.pins()[6].id, "SW-1-pinP2");
    assert_eq!(sw.pins()[7].id, "SW-1-switch4pinN");
    assert_eq!(sw.pins()[8].id, "SW-1-switch5pinN");
}

#[test]
fn test_switch_and_push_property_reactivity() {
    use cs_engine::canvas::Canvas;

    let mut canvas = Canvas::new();
    let sw_id = canvas.scene_mut().add_switch(0.0, 0.0, false);
    let pb_id = canvas.scene_mut().add_push(100.0, 0.0);

    // Test ShowButton toggle on Push
    canvas.open_properties(&pb_id);
    assert!(!canvas.scene().item_by_id(&pb_id).unwrap().show_button());
    canvas.set_prop_bool("ShowButton".to_string(), true);
    assert!(canvas.scene().item_by_id(&pb_id).unwrap().show_button());
    canvas.set_prop_bool("ShowButton".to_string(), false);
    assert!(!canvas.scene().item_by_id(&pb_id).unwrap().show_button());

    // Test Norm_Close on Push
    assert!(!canvas.scene().item_by_id(&pb_id).unwrap().norm_close());
    canvas.set_prop_bool("Norm_Close".to_string(), true);
    assert!(canvas.scene().item_by_id(&pb_id).unwrap().norm_close());
    canvas.set_prop_bool("Norm_Close".to_string(), false);
    assert!(!canvas.scene().item_by_id(&pb_id).unwrap().norm_close());

    // Test Poles on Push
    assert_eq!(canvas.scene().item_by_id(&pb_id).unwrap().poles(), 1);
    canvas.set_prop_text("Poles".to_string(), "4".to_string());
    assert_eq!(canvas.scene().item_by_id(&pb_id).unwrap().poles(), 4);
    assert_eq!(canvas.scene().item_by_id(&pb_id).unwrap().pins().len(), 8);

    // Test ShowButton and Poles on Switch
    canvas.open_properties(&sw_id);
    assert!(!canvas.scene().item_by_id(&sw_id).unwrap().show_button());
    canvas.set_prop_bool("ShowButton".to_string(), true);
    assert!(canvas.scene().item_by_id(&sw_id).unwrap().show_button());
    canvas.set_prop_bool("ShowButton".to_string(), false);
    assert!(!canvas.scene().item_by_id(&sw_id).unwrap().show_button());

    assert_eq!(canvas.scene().item_by_id(&sw_id).unwrap().poles(), 1);
    canvas.set_prop_text("Poles".to_string(), "2".to_string());
    assert_eq!(canvas.scene().item_by_id(&sw_id).unwrap().poles(), 2);
    assert_eq!(canvas.scene().item_by_id(&sw_id).unwrap().pins().len(), 4);
}

#[test]
fn test_switch_multipole_simulation() {
    use cs_engine::Circuit;
    use cs_engine::elements::Comp;

    let mut circ = Circuit::new();
    circ.add_comp(Comp::fixed_volt("V1", 10.0))
        .add_comp(Comp::resistor("R1", 100.0))
        .add_comp(Comp::fixed_volt("V2", 5.0))
        .add_comp(Comp::resistor("R2", 100.0))
        .add_comp(Comp::ground("GND"))
        .add_comp(Comp::switch_with_poles("SW", false, 2));

    circ.connect("V1-outnod", "SW-pinP0")
        .connect("SW-switch0pinN", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");

    circ.connect("V2-outnod", "SW-pinP1")
        .connect("SW-switch1pinN", "R2-lPin")
        .connect("R2-rPin", "GND-Gnd");

    circ.solve().unwrap();

    assert!(
        circ.pin_voltage("R1-lPin").unwrap().abs() < 1e-4,
        "Open switch pole 0 must not conduct"
    );
    assert!(
        circ.pin_voltage("R2-lPin").unwrap().abs() < 1e-4,
        "Open switch pole 1 must not conduct"
    );

    circ.set_switch("SW", true);
    circ.solve().unwrap();

    let v1 = circ.pin_voltage("R1-lPin").unwrap();
    let v2 = circ.pin_voltage("R2-lPin").unwrap();
    assert!(
        (v1 - 10.0).abs() < 0.1,
        "Pole 0 should deliver ~10V to R1, got {v1}"
    );
    assert!(
        (v2 - 5.0).abs() < 0.1,
        "Pole 1 should deliver ~5V to R2, got {v2}"
    );
}

#[test]
fn test_double_throw_switch_simulation_and_norm_close() {
    use cs_engine::Circuit;
    use cs_engine::elements::Comp;

    // 1. Standard SPDT (NormClose = false)
    let mut circ = Circuit::new();
    circ.add_comp(Comp::fixed_volt("V1", 5.0))
        .add_comp(Comp::resistor("R0", 100.0))
        .add_comp(Comp::resistor("R1", 100.0))
        .add_comp(Comp::ground("GND"))
        .add_comp(Comp::switch_full("SW", false, 1, true)); // open (checked=false), 1 pole, DT=true

    // Connect V1 to common pole P0
    circ.connect("V1-outnod", "SW-pinP0")
        // Throw 0 to R0
        .connect("SW-switch0pinN", "R0-lPin")
        .connect("R0-rPin", "GND-Gnd")
        // Throw 1 to R1
        .connect("SW-switch1pinN", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");

    circ.solve().unwrap();

    // When switch is open (checked = false):
    // Throw 0 should be disconnected (0 V).
    // Throw 1 should conduct current (5 V).
    let vr0_open = circ.pin_voltage("R0-lPin").unwrap();
    let vr1_open = circ.pin_voltage("R1-lPin").unwrap();
    assert!(
        vr0_open.abs() < 1e-4,
        "Throw 0 must not conduct when switch is open, got {vr0_open}"
    );
    assert!(
        (vr1_open - 5.0).abs() < 0.1,
        "Throw 1 must conduct when switch is open, got {vr1_open}"
    );

    let i_t0_open = circ.current_out_of_pin("SW-switch0pinN").unwrap();
    let i_t1_open = circ.current_out_of_pin("SW-switch1pinN").unwrap();
    assert!(
        i_t0_open.abs() < 1e-4,
        "Current on throw 0 should be 0 when open"
    );
    assert!(
        (i_t1_open.abs() - 0.05).abs() < 0.005,
        "Current on throw 1 should be ~50mA when open, got {i_t1_open}"
    );

    // Now close switch (checked = true)
    circ.set_switch("SW", true);
    circ.solve().unwrap();

    // When switch is closed (checked = true):
    // Throw 0 conducts (5 V).
    // Throw 1 is disconnected (0 V).
    let vr0_closed = circ.pin_voltage("R0-lPin").unwrap();
    let vr1_closed = circ.pin_voltage("R1-lPin").unwrap();
    assert!(
        (vr0_closed - 5.0).abs() < 0.1,
        "Throw 0 must conduct when switch is closed, got {vr0_closed}"
    );
    assert!(
        vr1_closed.abs() < 1e-4,
        "Throw 1 must not conduct when switch is closed, got {vr1_closed}"
    );

    let i_t0_closed = circ.current_out_of_pin("SW-switch0pinN").unwrap();
    let i_t1_closed = circ.current_out_of_pin("SW-switch1pinN").unwrap();
    assert!(
        (i_t0_closed.abs() - 0.05).abs() < 0.005,
        "Current on throw 0 should be ~50mA when closed"
    );
    assert!(
        i_t1_closed.abs() < 1e-4,
        "Current on throw 1 should be 0 when closed"
    );

    // 2. Normally Closed SPDT via Scene
    use cs_engine::canvas::Canvas;
    let mut canvas = Canvas::new();
    let sw_id = canvas.scene_mut().add_switch(0.0, 0.0, false);
    canvas.open_properties(&sw_id);
    canvas.set_prop_bool("Double_Throw".to_string(), true);
    canvas.set_prop_bool("Norm_Close".to_string(), true);

    let sw_item = canvas.scene().item_by_id(&sw_id).unwrap();
    assert!(sw_item.double_throw());
    assert!(sw_item.norm_close());

    // In resting state (checked = false), Normally Closed means electrically closed (throw 0 conducts)
    let comp = sw_item.to_element_kind().unwrap();
    if let cs_engine::elements::Kind::Switch {
        closed,
        double_throw,
        poles,
    } = comp
    {
        assert!(
            closed,
            "Normally closed switch must be electrically closed at rest"
        );
        assert!(double_throw);
        assert_eq!(poles, 1);
    } else {
        panic!("Expected Kind::Switch");
    }
}

#[test]
fn test_switch_visual_open_blade_angles_up_and_pins_do_not_protrude() {
    use cs_engine::canvas::scene::Item;
    let sw = Item::switch("SW-1", 0.0, 0.0, false);

    // All pin stems must be length 6.0 so they meet dots at x = ±10 without protruding
    for pin in sw.pins() {
        assert_eq!(
            pin.length, 6.0,
            "Pin length must be 6.0 (start ±16 -> dot at ±10) to avoid protruding into switch interior"
        );
    }
}

#[test]
fn test_switch_dip_and_keypad_pin_names_are_hidden() {
    use cs_engine::canvas::Item;

    let sw_dip = Item::switch_dip("DIP-1", 0.0, 0.0, 8, 0, false);
    for pin in sw_dip.pins() {
        assert!(
            pin.label.is_empty(),
            "DIP switch pin label must be empty so text does not appear on canvas, got {:?}",
            pin.label
        );
    }

    let sw_dip_com = Item::switch_dip("DIP-COM", 0.0, 0.0, 8, 0, true);
    for pin in sw_dip_com.pins() {
        assert!(
            pin.label.is_empty(),
            "DIP switch with common pin label must be empty so text does not appear on canvas, got {:?}",
            pin.label
        );
    }

    let kp = Item::keypad("KP-1", 0.0, 0.0, 4, 4);
    for pin in kp.pins() {
        assert!(
            pin.label.is_empty(),
            "Keypad pin label must be empty so text does not appear on canvas, got {:?}",
            pin.label
        );
    }
}

#[test]
fn test_potentiometer_dial_knob_and_wheel() {
    use cs_engine::canvas::{Canvas, Item, Point, Rect};

    let pot = Item::potentiometer("POT-1", 0.0, 0.0, 10_000.0, 0.5);
    // Bounding rect must include the dial knob (y: -33 to 12)
    assert_eq!(pot.body_rect(), Rect::new(-14.0, -33.0, 28.0, 45.0));

    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(pot);

    // Wheel event right on the dial knob center (0, -19)
    let initial_wiper = canvas.scene().items()[0].wiper();
    assert_eq!(initial_wiper, 0.5);

    // Scroll up (positive delta) on the dial knob
    let changed = canvas.scene_mut().wheel_at(Point::new(0.0, -19.0), 120.0);
    assert_eq!(changed.as_deref(), Some("POT-1"));
    assert!((canvas.scene().items()[0].wiper() - 0.51).abs() < 1e-4);

    // Scroll down (negative delta) on the dial knob
    let changed = canvas.scene_mut().wheel_at(Point::new(0.0, -19.0), -120.0);
    assert_eq!(changed.as_deref(), Some("POT-1"));
    assert!((canvas.scene().items()[0].wiper() - 0.50).abs() < 1e-4);
}

#[test]
fn test_variable_components_dial_geometry_and_interaction() {
    use cs_engine::canvas::{Canvas, Item, Point, Rect};

    // 1. Variable Resistor
    let vr = Item::var_resistor("VR-1", 0.0, 0.0, 1000.0);
    assert_eq!(vr.body_rect(), Rect::new(-14.0, -33.0, 28.0, 42.0));
    assert!((vr.wiper() - 0.5).abs() < 1e-5);

    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(vr);

    // Wheel at knob center (0, -19)
    let changed = canvas.scene_mut().wheel_at(Point::new(0.0, -19.0), 120.0);
    assert_eq!(changed.as_deref(), Some("VR-1"));
    assert!(canvas.scene().items()[0].wiper() > 0.5);

    // set_dial_val
    canvas.set_dial_val("VR-1", 1500.0);
    let item = canvas.scene().item_by_id("VR-1").unwrap();
    assert_eq!(item.source_value(), 1500.0);
    assert!((item.wiper() - 0.75).abs() < 1e-5);

    // 2. Variable Capacitor
    let vc = Item::var_capacitor("VC-1", 0.0, 0.0, 10e-6);
    assert_eq!(vc.body_rect(), Rect::new(-14.0, -33.0, 28.0, 42.0));
    assert!((vc.wiper() - 0.5).abs() < 1e-5);

    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(vc);

    let changed = canvas.scene_mut().wheel_at(Point::new(0.0, -19.0), 120.0);
    assert_eq!(changed.as_deref(), Some("VC-1"));
    assert!(canvas.scene().items()[0].wiper() > 0.5);

    canvas.set_dial_val("VC-1", 5e-6);
    let item = canvas.scene().item_by_id("VC-1").unwrap();
    assert_eq!(item.source_value(), 5e-6);
    assert!((item.wiper() - 0.25).abs() < 1e-5);

    // 3. Variable Inductor
    let vi = Item::var_inductor("VI-1", 0.0, 0.0, 1e-3);
    assert_eq!(vi.body_rect(), Rect::new(-14.0, -33.0, 28.0, 42.0));
    assert!((vi.wiper() - 0.5).abs() < 1e-5);

    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(vi);

    let changed = canvas.scene_mut().wheel_at(Point::new(0.0, -19.0), 120.0);
    assert_eq!(changed.as_deref(), Some("VI-1"));
    assert!(canvas.scene().items()[0].wiper() > 0.5);

    canvas.set_dial_val("VI-1", 1.5e-3);
    let item = canvas.scene().item_by_id("VI-1").unwrap();
    assert_eq!(item.source_value(), 1.5e-3);
    assert!((item.wiper() - 0.75).abs() < 1e-5);

    // 4. KY-040 Rotary Encoder
    let ky = Item::ky040("KY-1", 0.0, 0.0, 20);
    assert_eq!(ky.body_rect(), Rect::new(-20.0, -28.0, 40.0, 56.0));
    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(ky);

    let changed = canvas.scene_mut().wheel_at(Point::new(0.0, 0.0), 120.0);
    assert_eq!(changed.as_deref(), Some("KY-1"));
    assert_eq!(canvas.scene().items()[0].dial_val(), 2);
}

#[test]
fn test_potentiometer_stationary_wiper_and_dial_rotation() {
    use cs_engine::canvas::export::{Palette, svg_string};
    use cs_engine::canvas::{Canvas, Item, Point};

    let pot = Item::potentiometer("POT-1", 100.0, 100.0, 10_000.0, 0.25);
    let mut canvas = Canvas::new();
    canvas.scene_mut().add_saved_item(pot);

    // Clicking the resistor body must NOT change the wiper
    let toggled = canvas
        .scene_mut()
        .toggle_switch_at(0, Point::new(105.0, 100.0));
    assert!(toggled.is_none(), "Clicking body should not slide wiper");
    assert_eq!(canvas.scene().items()[0].wiper(), 0.25);

    // Clicking the knob (100, 100 - 19 = 81) steps the wiper
    let toggled_knob = canvas
        .scene_mut()
        .toggle_switch_at(0, Point::new(100.0, 81.0));
    assert!(toggled_knob.is_some(), "Clicking knob should step wiper");
    assert!((canvas.scene().items()[0].wiper() - 0.26).abs() < 1e-4);

    // Svg export: wiper stem must be stationary at x=0
    let svg = svg_string(&canvas, &Palette::light());
    assert!(
        svg.contains(r#"<line x1="0" y1="10" x2="0" y2="4.5""#),
        "Wiper line must be stationary at center, got: {}",
        svg
    );
}

#[test]
fn test_serial_port_and_serial_term_geometry_and_pin_contact() {
    use cs_engine::canvas::{Item, Rect};

    // SerialPort (body is [-32, -16, 160, 32], left border at x = -32)
    let sp = Item::serial_port("SP-1", 0.0, 0.0, "ttyUSB0", 115200);
    let sp_body = sp.body_rect();
    assert_eq!(sp_body, Rect::new(-32.0, -16.0, 160.0, 32.0));

    let sp_pins = sp.pins();
    assert_eq!(sp_pins.len(), 4);
    let rx = sp_pins.iter().find(|p| p.id.ends_with("-rx")).unwrap();
    let tx = sp_pins.iter().find(|p| p.id.ends_with("-tx")).unwrap();
    let gnd = sp_pins.iter().find(|p| p.id.ends_with("-gnd")).unwrap();
    let vcc = sp_pins.iter().find(|p| p.id.ends_with("-vcc")).unwrap();

    // Left pins (rx, tx): tip at x = -40.0, angle 180, length 8.0 -> base touches x = -32.0 (left border)
    assert_eq!(rx.local.x, -40.0);
    assert_eq!(rx.angle, 180);
    assert_eq!(rx.length, 8.0);
    assert_eq!(rx.local.x + rx.length, sp_body.x);

    assert_eq!(tx.local.x, -40.0);
    assert_eq!(tx.angle, 180);
    assert_eq!(tx.length, 8.0);
    assert_eq!(tx.local.x + tx.length, sp_body.x);

    // Right pins (gnd, vcc): tip at x = 136.0, angle 0, length 8.0 -> base touches x = 128.0 (right border)
    assert_eq!(gnd.local.x, 136.0);
    assert_eq!(gnd.angle, 0);
    assert_eq!(gnd.length, 8.0);
    assert_eq!(gnd.local.x - gnd.length, sp_body.x + sp_body.w);

    assert_eq!(vcc.local.x, 136.0);
    assert_eq!(vcc.angle, 0);
    assert_eq!(vcc.length, 8.0);
    assert_eq!(vcc.local.x - vcc.length, sp_body.x + sp_body.w);

    // SerialTerm (body is [-16, -16, 72, 32], left border at x = -16)
    let st = Item::serial_term("ST-1", 0.0, 0.0, 9600);
    let st_body = st.body_rect();
    assert_eq!(st_body, Rect::new(-16.0, -16.0, 72.0, 32.0));

    let st_pins = st.pins();
    assert_eq!(st_pins.len(), 2);
    let st_rx = st_pins.iter().find(|p| p.id.ends_with("-rx")).unwrap();
    let st_tx = st_pins.iter().find(|p| p.id.ends_with("-tx")).unwrap();

    // Left pins (rx, tx): tip at x = -24.0, angle 180, length 8.0 -> base touches x = -16.0 (left border)
    assert_eq!(st_rx.local.x, -24.0);
    assert_eq!(st_rx.angle, 180);
    assert_eq!(st_rx.length, 8.0);
    assert_eq!(st_rx.local.x + st_rx.length, st_body.x);

    assert_eq!(st_tx.local.x, -24.0);
    assert_eq!(st_tx.angle, 180);
    assert_eq!(st_tx.length, 8.0);
    assert_eq!(st_tx.local.x + st_tx.length, st_body.x);
}

#[test]
fn test_clock_wavegen_rail_probe_geometry_and_pin_contact() {
    // 1. Clock (circle r=8.0 at (0,0); border at x = 8.0)
    let clk = Item::clock("CLK-1", 0.0, 0.0, 5.0, 1.0);
    let clk_pins = clk.pins();
    assert_eq!(clk_pins.len(), 1);
    assert_eq!(clk_pins[0].local.x, 16.0);
    assert_eq!(clk_pins[0].local.y, 0.0);
    assert_eq!(clk_pins[0].angle, 0);
    assert_eq!(clk_pins[0].length, 8.0);
    assert_eq!(clk_pins[0].local.x - clk_pins[0].length, 8.0);

    // 2. WaveGen
    // 2a. Unipolar (circle r=8.0 at (0,0); border at x = 8.0)
    let wg_uni = Item::wave_gen("WG-1", 0.0, 0.0, "Sine", 1000.0, 5.0, 0.0, 50.0);
    let uni_pins = wg_uni.pins();
    assert_eq!(uni_pins.len(), 1);
    assert_eq!(uni_pins[0].local.x, 16.0);
    assert_eq!(uni_pins[0].local.y, 0.0);
    assert_eq!(uni_pins[0].angle, 0);
    assert_eq!(uni_pins[0].length, 8.0);
    assert_eq!(uni_pins[0].local.x - uni_pins[0].length, 8.0);

    // 2b. Bipolar (circle r=8.0 at (0,0); pins at y = -4.0 and y = 4.0; border at x = sqrt(48) ≈ 6.9282)
    let mut wg_bi = Item::wave_gen("WG-2", 0.0, 0.0, "Sine", 1000.0, 5.0, 0.0, 50.0);
    assert!(wg_bi.set_prop_bool("Bipolar", true));
    let bi_pins = wg_bi.pins();
    assert_eq!(bi_pins.len(), 2);
    let border_x = (8.0_f64.powi(2) - 4.0_f64.powi(2)).sqrt();
    for pin in &bi_pins {
        assert_eq!(pin.local.x, 16.0);
        assert_eq!(pin.angle, 0);
        assert_eq!(pin.length, 9.07);
        let contact_x = pin.local.x - pin.length;
        assert!((contact_x - border_x).abs() < 0.01);
    }

    // 3. Rail (polygon right border at x = 9.0)
    let rail = Item::rail("Rail-1", 0.0, 0.0, 5.0);
    let rail_pins = rail.pins();
    assert_eq!(rail_pins.len(), 1);
    assert_eq!(rail_pins[0].local.x, 16.0);
    assert_eq!(rail_pins[0].local.y, 0.0);
    assert_eq!(rail_pins[0].angle, 0);
    assert_eq!(rail_pins[0].length, 7.0);
    assert_eq!(rail_pins[0].local.x - rail_pins[0].length, 9.0);

    // 4. Probe
    // 4a. Normal probe (circular body at [-8, -8, 16, 16], left border at x = -8.0)
    let probe = Item::probe("Probe-1", 0.0, 0.0, 2.5, false);
    let probe_pins = probe.pins();
    assert_eq!(probe_pins.len(), 1);
    assert_eq!(probe_pins[0].local.x, -24.0);
    assert_eq!(probe_pins[0].local.y, 0.0);
    assert_eq!(probe_pins[0].angle, 180);
    assert_eq!(probe_pins[0].length, 16.0);
    assert_eq!(probe_pins[0].local.x + probe_pins[0].length, -8.0);

    // 4b. Small probe (circular body at [-16, -4, 8, 8], left border at x = -16.0)
    let probe_sm = Item::probe("Probe-2", 0.0, 0.0, 2.5, true);
    let sm_pins = probe_sm.pins();
    assert_eq!(sm_pins.len(), 1);
    assert_eq!(sm_pins[0].local.x, -24.0);
    assert_eq!(sm_pins[0].local.y, 0.0);
    assert_eq!(sm_pins[0].angle, 180);
    assert_eq!(sm_pins[0].length, 8.0);
    assert_eq!(sm_pins[0].local.x + sm_pins[0].length, -16.0);
}

#[test]
fn test_seven_segment_gates_triac_led_matrix_alignments() {
    // 1. Seven Segment display:
    // Left border: x = -18.0, Right border: x = 18.0, Top border: y = -28.0, Bottom border: y = 28.0
    let ss = Item::seven_segment("SS-1", 0.0, 0.0, false);
    let ss_rect = ss.body_rect();
    assert_eq!(ss_rect.x, -18.0);
    assert_eq!(ss_rect.y, -28.0);
    assert_eq!(ss_rect.w, 36.0);
    assert_eq!(ss_rect.h, 56.0);

    let pins = ss.pins();
    assert_eq!(pins.len(), 9);
    for p in &pins[..7] {
        // Pins a..g on left
        assert_eq!(p.local.x, -24.0);
        assert_eq!(p.angle, 180);
        assert_eq!(p.length, 6.0);
        assert_eq!(
            p.local.x + p.length,
            -18.0,
            "Left pins must meet left border"
        );
    }
    // Dot pin on bottom
    let dot = &pins[7];
    assert_eq!(dot.local.x, -8.0);
    assert_eq!(dot.local.y, 32.0);
    assert_eq!(dot.angle, 270);
    assert_eq!(dot.length, 4.0);
    assert_eq!(
        dot.local.y - dot.length,
        28.0,
        "Dot pin must meet bottom border"
    );

    // COM pin on bottom
    let com = &pins[8];
    assert_eq!(com.local.x, 8.0);
    assert_eq!(com.local.y, 32.0);
    assert_eq!(com.angle, 270);
    assert_eq!(com.length, 4.0);
    assert_eq!(
        com.local.y - com.length,
        28.0,
        "COM pin must meet bottom border"
    );

    // 2. AND Gate:
    // Input pins from -16.0 with length 8.0 reach x = -8.0
    // Output pin from 16.0 with length 8.0 reaches x = 8.0
    let and_gate = Item::gate(
        "Gate-1", 0.0, 0.0, "And", 2, false, false, false, false, false,
    );
    let g_pins = and_gate.pins();
    assert_eq!(g_pins.len(), 3);
    for p in &g_pins[..2] {
        assert_eq!(p.local.x, -16.0);
        assert_eq!(p.angle, 180);
        assert_eq!(p.length, 8.0);
        assert!(p.label.is_empty(), "Gate input pins must have empty labels");
        assert_eq!(
            p.local.x + p.length,
            -8.0,
            "Gate input pins must meet back border"
        );
    }
    let out_pin = &g_pins[2];
    assert_eq!(out_pin.local.x, 16.0);
    assert_eq!(out_pin.angle, 0);
    assert_eq!(out_pin.length, 8.0);
    assert!(
        out_pin.label.is_empty(),
        "Gate output pin must have empty label"
    );
    assert_eq!(
        out_pin.local.x - out_pin.length,
        8.0,
        "Gate out pin must meet front tip"
    );

    // 3. TRIAC Gate Pin:
    // Terminal vertical line is at x = 8.0. Gate pin starts at (16.0, 12.0) with angle -26 deg.
    let triac = Item::triac("TRIAC-1", 0.0, 0.0, 0.7, 0.01);
    let t_pins = triac.pins();
    assert_eq!(t_pins.len(), 3);
    let g_pin = &t_pins[2];
    assert_eq!(g_pin.local.x, 16.0);
    assert_eq!(g_pin.local.y, 12.0);
    assert_eq!(g_pin.angle, -26);
    assert_eq!(g_pin.length, 8.9);
    let end_x = g_pin.local.x - g_pin.length * (g_pin.angle as f64).to_radians().cos();
    assert!(
        (end_x - 8.0).abs() < 0.05,
        "TRIAC gate pin must reach x=8.0 border"
    );

    // 4. LED Matrix pins:
    // Pin names must be empty
    let lm = Item::led_matrix("LM-1", 0.0, 0.0, 8, 8);
    let lm_pins = lm.pins();
    assert_eq!(lm_pins.len(), 16);
    for p in lm_pins {
        assert!(p.label.is_empty(), "LED Matrix pin label should be empty");
    }
}

#[test]
fn test_logic_gates_pin_labels_empty_and_bus_geometry() {
    // 1. AND, OR, XOR gates have empty pin labels
    for kind in ["And", "Or", "Xor"] {
        let gate = Item::gate(
            format!("{kind}-1"),
            0.0,
            0.0,
            kind,
            3,
            false,
            false,
            false,
            false,
            false,
        );
        for p in gate.pins() {
            assert!(
                p.label.is_empty(),
                "{kind} gate pin {} should have empty label, got '{}'",
                p.id,
                p.label
            );
        }
    }

    // 2. Bus component geometry
    let bus = Item::bus("Bus-1", 0.0, 0.0, 8);
    let brect = bus.body_rect();
    // 8 lines -> busHeight = 7 * 8 = 56.0
    // Body rect covers trunk connector area: y from -58.0 to 2.0 (height 60.0), width 6.0 centered at 0.0
    assert_eq!(brect, Rect::new(-3.0, -58.0, 6.0, 60.0));

    let b_pins = bus.pins();
    assert_eq!(b_pins.len(), 10); // 1 bottom trunk + 8 line pins + 1 top trunk
    // Bottom trunk pin ePin0
    assert_eq!(b_pins[0].id, "Bus-1-ePin0");
    assert!(b_pins[0].is_bus);
    assert_eq!(b_pins[0].local.x, 0.0);
    assert_eq!(b_pins[0].local.y, 0.0);
    assert_eq!(b_pins[0].length, 1.0);

    // Top trunk pin busPinI
    assert_eq!(b_pins[9].id, "Bus-1-busPinI");
    assert!(b_pins[9].is_bus);
    assert_eq!(b_pins[9].local.x, 0.0);
    assert_eq!(b_pins[9].local.y, -56.0);
    assert_eq!(b_pins[9].length, 1.0);

    // Line pins ePin1..ePin8: meet spine at x = 0.0
    for i in 1..=8 {
        let p = &b_pins[i];
        assert!(!p.is_bus);
        assert_eq!(p.local.x, -8.0);
        assert_eq!(p.length, 8.0);
        assert_eq!(p.angle, 180);
        assert_eq!(
            p.local.x + p.length,
            0.0,
            "Line pin stem must meet bus spine at x=0"
        );
    }
}

#[test]
fn test_seven_segment_multidisplay_and_vertical_pins() {
    let mut ss2 = Item::seven_segment("SS-2", 0.0, 0.0, false);
    ss2.set_prop_text("NumDisplays", "2");
    let rect2 = ss2.body_rect();
    assert_eq!(rect2.x, -18.0);
    assert_eq!(rect2.y, -28.0);
    assert_eq!(rect2.w, 68.0);
    assert_eq!(rect2.h, 56.0);

    let pins2 = ss2.pins();
    // 7 segment pins + 1 dot pin + 2 common pins = 10 pins
    assert_eq!(pins2.len(), 10);
    assert_eq!(pins2[8].local.x, 8.0); // Common pin 0
    assert_eq!(pins2[9].local.x, 40.0); // Common pin 1
    assert_eq!(pins2[8].local.y - pins2[8].length, 28.0);
    assert_eq!(pins2[9].local.y - pins2[9].length, 28.0);

    // Test vertical layout
    let mut ss_vert = Item::seven_segment("SS-V", 0.0, 0.0, true);
    ss_vert.set_prop_bool("Vertical_Pins", true);
    let pins_v = ss_vert.pins();
    assert_eq!(pins_v.len(), 9);
    // Top 5 pins (a..e) at y = -32, length 4 meet top border at y = -28
    for p in &pins_v[..5] {
        assert_eq!(p.local.y, -32.0);
        assert_eq!(p.angle, 90);
        assert_eq!(p.length, 4.0);
        assert_eq!(p.local.y + p.length, -28.0, "Top pins must meet top border");
    }
    // Bottom pins: f, g, dot, com meet bottom border at y = 28
    for p in &pins_v[5..] {
        assert_eq!(p.local.y, 32.0);
        assert_eq!(p.angle, 270);
        assert_eq!(p.length, 4.0);
        assert_eq!(
            p.local.y - p.length,
            28.0,
            "Bottom pins must meet bottom border"
        );
    }
}

#[test]
fn test_add_all_components_type_id_coverage() {
    let mut canvas = Canvas::default();
    canvas.add_all_components();
    let items = canvas.scene().items();
    assert!(
        !items.is_empty(),
        "add_all_components must place components"
    );

    for it in items {
        let type_name = it.kind.type_name();
        assert!(!type_name.is_empty(), "Item {} has empty type_name", it.id);
    }
}

#[test]
fn test_mcu_item_properties_for_qml() {
    let mut canvas = Canvas::default();
    canvas.add_all_components();
    let items = canvas.scene().items();
    let mcu_items: Vec<_> = items
        .iter()
        .filter(|it| it.kind.type_name() == "Mcu" || it.kind.type_name() == "QemuDevice")
        .collect();
    assert!(
        items.len() >= 180,
        "Expected at least 180 items placed, got {}",
        items.len()
    );
    assert!(
        mcu_items.len() >= 70,
        "Expected at least 70 MCU/Qemu items, got {}",
        mcu_items.len()
    );

    for it in &mcu_items {
        let rect = it.body_rect();
        assert!(
            rect.w > 0.0 && rect.h > 0.0,
            "MCU {} must have positive body rect",
            it.id
        );
        assert!(
            it.pkg_w() > 0 && it.pkg_h() > 0,
            "MCU {} must have positive package dimensions",
            it.id
        );
        let pins = it.pins();
        assert!(!pins.is_empty(), "MCU {} must have pins", it.id);
        for p in &pins {
            assert!(!p.id.is_empty(), "Pin on {} must have non-empty ID", it.id);
            assert!(p.length > 0.0, "Pin on {} must have positive length", it.id);
        }
    }

    // Verify ChipBody and ChipLabelText colors in ColorTheme
    use cs_engine::theme::{ColorId, ColorTheme};
    let chip_body_dark = ColorTheme::get_rgba(ColorId::ChipBody, true);
    let chip_body_light = ColorTheme::get_rgba(ColorId::ChipBody, false);
    assert_eq!(chip_body_dark, (20, 30, 60, 255));
    assert_eq!(chip_body_light, (20, 30, 60, 255));

    let chip_label_dark = ColorTheme::get_rgba(ColorId::ChipLabelText, true);
    let chip_label_light = ColorTheme::get_rgba(ColorId::ChipLabelText, false);
    assert_eq!(chip_label_dark, (160, 160, 180, 255));
    assert_eq!(chip_label_light, (160, 160, 180, 255));

    // Verify scripted MCUs place successfully
    for script_mcu in ["6520", "6522", "6532"] {
        let mut c = Canvas::default();
        let spec = format!("{script_mcu},MCU");
        assert!(
            c.scene_add_component_spec(&spec, Point::zero()),
            "Scripted MCU {script_mcu} must place"
        );
        let placed = c.scene().items();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].kind.type_name(), "Mcu");
        assert!(placed[0].pins().len() > 0);
    }
}

#[test]
fn test_drawable_resistor_led_push_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Resistor rendering
    let res = Item::resistor("R-1", -60.0, -30.0, 4700.0);
    canvas.scene_mut().add_saved_item(res);

    // 2. LED rendering
    let mut led = Item::led("LED-1", 0.0, -30.0);
    led.set_prop_text("Color", "Green");
    canvas.scene_mut().add_saved_item(led);

    // 3. Push button rendering
    let push = Item::push("PUSH-1", 60.0, -30.0, false, 1);
    canvas.scene_mut().add_saved_item(push);

    let pm_idle = render_viewport(&canvas, &pal, 500, 300, 1.0, true).expect("render idle");
    assert_eq!(pm_idle.width(), 500);
    assert_eq!(pm_idle.height(), 300);

    // 4. Test Grounded LED removes cathode pin
    let mut led_gnd = Item::led("LED-GND", -60.0, 30.0);
    assert_eq!(led_gnd.pins().len(), 2);
    led_gnd.set_prop_bool("Grounded", true);
    assert_eq!(led_gnd.pins().len(), 1);
    assert!(led_gnd.pins()[0].id.ends_with("-lPin"));
    canvas.scene_mut().add_saved_item(led_gnd);

    // 5. Test Push Button with ShowButton enabled and pressed
    let mut push_box = Item::push("PUSH-BOX", 60.0, 30.0, false, 1);
    push_box.set_prop_bool("ShowButton", true);
    push_box.set_prop_bool("Checked", true);
    canvas.scene_mut().add_saved_item(push_box);

    let pm_active = render_viewport(&canvas, &pal, 500, 300, 1.0, true).expect("render active");
    assert_ne!(pm_idle.data(), pm_active.data());

    // 6. Test simulation running lit LED glow
    canvas.set_sim_running(true);
    canvas.set_pin_voltage("LED-1-lPin", 5.0);
    canvas.set_pin_voltage("LED-1-rPin", 0.0);
    let pm_sim = render_viewport(&canvas, &pal, 500, 300, 1.0, true).expect("render sim");
    assert_ne!(pm_active.data(), pm_sim.data());
}

#[test]
fn test_drawable_batch2_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Sources & Passive components
    let b = Item::battery("BAT-1", -100.0, -100.0, 9.0, 0.1);
    canvas.scene_mut().add_saved_item(b);

    let g = Item::ground("GND-1", -50.0, -100.0);
    canvas.scene_mut().add_saved_item(g);

    let mut fv = Item::fixed_volt("FV-1", 0.0, -100.0, 5.0);
    canvas.scene_mut().add_saved_item(fv.clone());
    fv.set_prop_bool("Small", true);
    fv.id = "FV-SMALL".to_string();
    fv.x = 20.0;
    canvas.scene_mut().add_saved_item(fv);

    let r = Item::rail("RAIL-1", 50.0, -100.0, 12.0);
    canvas.scene_mut().add_saved_item(r);

    let mut clk = Item::clock("CLK-1", 100.0, -100.0, 5.0, 1000.0);
    canvas.scene_mut().add_saved_item(clk.clone());
    clk.set_prop_bool("Small", true);
    clk.id = "CLK-SMALL".to_string();
    clk.x = 120.0;
    canvas.scene_mut().add_saved_item(clk);

    let cap = Item::capacitor("C-1", -100.0, -50.0, 1e-6);
    canvas.scene_mut().add_saved_item(cap);

    let elcap = Item::el_capacitor("EC-1", -50.0, -50.0, 10e-6);
    canvas.scene_mut().add_saved_item(elcap);

    let ind = Item::inductor("L-1", 0.0, -50.0, 1e-3);
    canvas.scene_mut().add_saved_item(ind);

    // 2. Switches & Diodes
    let sw_open = Item::switch("SW-OPEN", 50.0, -50.0, false);
    canvas.scene_mut().add_saved_item(sw_open);

    let sw_closed = Item::switch("SW-CLOSED", 100.0, -50.0, true);
    canvas.scene_mut().add_saved_item(sw_closed);

    let mut sw_btn = Item::switch("SW-BTN", 150.0, -50.0, true);
    sw_btn.set_prop_bool("ShowButton", true);
    canvas.scene_mut().add_saved_item(sw_btn);

    let diode = Item::diode("D-1", -100.0, 0.0, false);
    canvas.scene_mut().add_saved_item(diode);

    let zener = Item::diode("ZD-1", -50.0, 0.0, true);
    canvas.scene_mut().add_saved_item(zener);

    // 3. OpAmp, Comparator, VoltReg
    let opamp = Item::opamp("OP-1", 0.0, 0.0);
    canvas.scene_mut().add_saved_item(opamp);

    let comp = Item::comparator("COMP-1", 50.0, 0.0);
    canvas.scene_mut().add_saved_item(comp);

    let vr = Item::volt_reg("VR-1", 100.0, 0.0);
    canvas.scene_mut().add_saved_item(vr);

    // 4. Transistors
    let bjt_npn = Item::bjt("Q-NPN", -100.0, 50.0, false);
    canvas.scene_mut().add_saved_item(bjt_npn);

    let bjt_pnp = Item::bjt("Q-PNP", -50.0, 50.0, true);
    canvas.scene_mut().add_saved_item(bjt_pnp);

    let mosfet_n = Item::mosfet("M-N", 0.0, 50.0, false, false);
    canvas.scene_mut().add_saved_item(mosfet_n);

    let mosfet_p = Item::mosfet("M-P", 50.0, 50.0, true, false);
    canvas.scene_mut().add_saved_item(mosfet_p);

    let jfet = Item::jfet("J-1", 100.0, 50.0);
    canvas.scene_mut().add_saved_item(jfet);

    // Rasterize viewport using preview canvas renderer
    let pm = render_viewport(&canvas, &pal, 800, 600, 1.0, true).expect("render batch2");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 600);

    // Verify non-empty rasterized pixels
    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 500,
        "Rendered pixmap must contain painted elements"
    );
}

#[test]
fn test_drawable_batch3_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Logic gates
    canvas.scene_mut().add_saved_item(Item::gate(
        "AND-1", -150.0, -100.0, "And", 3, false, false, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "OR-1", -75.0, -100.0, "Or", 2, false, false, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "XOR-1", 0.0, -100.0, "Xor", 2, false, false, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "NOT-1", 75.0, -100.0, "Not", 1, false, true, false, false, false,
    ));
    canvas.scene_mut().add_saved_item(Item::gate(
        "BUF-1", 150.0, -100.0, "Buffer", 1, false, false, false, true, false,
    ));

    // 2. Variable passives
    canvas
        .scene_mut()
        .add_saved_item(Item::var_resistor("VR-1", -150.0, -30.0, 5000.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::potentiometer("POT-1", -75.0, -30.0, 10000.0, 0.6));
    canvas
        .scene_mut()
        .add_saved_item(Item::var_capacitor("VC-1", 0.0, -30.0, 10e-12));
    canvas
        .scene_mut()
        .add_saved_item(Item::var_inductor("VL-1", 75.0, -30.0, 2e-3));

    // 3. Meters & Probes
    canvas
        .scene_mut()
        .add_saved_item(Item::probe("PRB-1", -150.0, 40.0, 2.5, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::voltmeter("VM-1", -75.0, 40.0, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::ammeter("AM-1", 0.0, 40.0, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::freq_meter("FM-1", 75.0, 40.0, 2.5));

    // 4. Displays & Actuators
    canvas
        .scene_mut()
        .add_saved_item(Item::seven_segment("SS-1", -150.0, 120.0, false));
    canvas.scene_mut().add_saved_item(Item::seven_segment_bcd(
        "SSB-1", -75.0, 120.0, "Green", false,
    ));
    canvas
        .scene_mut()
        .add_saved_item(Item::lamp("L-1", 0.0, 120.0, 12.0, 24.0, 0.6));
    canvas.scene_mut().add_saved_item(Item::relay(
        "RLY-1", 75.0, 120.0, false, false, 1, 0.03, 0.01, false,
    ));
    canvas
        .scene_mut()
        .add_saved_item(Item::transformer("TR-1", 150.0, 120.0, 1.0, 1.0, 0.99));
    canvas
        .scene_mut()
        .add_saved_item(Item::switch_dip("DIP-1", -75.0, 190.0, 4, 0b1010, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::keypad("KEY-1", 75.0, 190.0, 4, 4));

    // Rasterize viewport using preview canvas renderer
    let pm = render_viewport(&canvas, &pal, 800, 600, 1.0, true).expect("render batch3");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 600);

    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 1000,
        "Rendered pixmap must contain painted elements, got {non_bg}"
    );
}

#[test]
fn test_drawable_batch4_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Semiconductors & Mux
    canvas
        .scene_mut()
        .add_saved_item(Item::scr("SCR-1", -150.0, -120.0, 0.7, 0.0082));
    canvas
        .scene_mut()
        .add_saved_item(Item::diac("DIAC-1", -75.0, -120.0, 30.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::triac("TRIAC-1", 0.0, -120.0, 0.7, 0.0082));
    canvas
        .scene_mut()
        .add_saved_item(Item::analog_mux("AMUX-1", 75.0, -120.0, 4, 50.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::resistor_dip("RDIP-1", 150.0, -120.0, 4, 100.0, false));

    // 2. Transducers, Displays & Controls
    canvas
        .scene_mut()
        .add_saved_item(Item::audio_out("AUD-1", -150.0, -40.0, 8.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::tunnel("TUN-1", -75.0, -40.0, "CLK"));
    canvas
        .scene_mut()
        .add_saved_item(Item::led_bar("LBAR-1", 0.0, -40.0, 8));
    canvas
        .scene_mut()
        .add_saved_item(Item::led_matrix("LMAT-1", 75.0, -40.0, 4, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::rgb_led("RGB-1", 150.0, -40.0, false));

    // 3. Addressable LEDs, Modules & Sensors
    canvas
        .scene_mut()
        .add_saved_item(Item::ws2812("WS-1", -150.0, 50.0, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::max72xx("MAX-1", -75.0, 50.0, 1));
    canvas
        .scene_mut()
        .add_saved_item(Item::ky040("ENC-1", 0.0, 50.0, 20));
    canvas
        .scene_mut()
        .add_saved_item(Item::ky023("JOY-1", 75.0, 50.0));
    canvas.scene_mut().add_saved_item(Item::touchpad(
        "TP-1", 150.0, 50.0, 120, 80, false, 100.0, 500.0, 100.0, 500.0,
    ));

    // 4. Sources & Generators
    canvas
        .scene_mut()
        .add_saved_item(Item::volt_source("VS-1", -150.0, 150.0, 5.0, true));
    canvas
        .scene_mut()
        .add_saved_item(Item::curr_source("CS-1", -75.0, 150.0, 0.02, true));
    canvas.scene_mut().add_saved_item(Item::csource(
        "CCS-1", 0.0, 150.0, true, false, false, 2.0, 0.0, 0.0,
    ));
    canvas.scene_mut().add_saved_item(Item::wave_gen(
        "WG-1", 75.0, 150.0, "Sine", 1000.0, 5.0, 0.0, 0.5,
    ));
    canvas
        .scene_mut()
        .add_saved_item(Item::dial("DL-1", 150.0, 150.0, 50.0));

    // Rasterize viewport using preview canvas renderer
    let pm = render_viewport(&canvas, &pal, 800, 600, 1.0, true).expect("render batch4");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 600);

    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 1000,
        "Rendered pixmap must contain painted elements, got {non_bg}"
    );
}

#[test]
fn test_drawable_batch5_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Sensors & Digital Storage
    canvas
        .scene_mut()
        .add_saved_item(Item::ldr("LDR-1", -150.0, -120.0, 1000.0, 100.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::thermistor("TH-1", -75.0, -120.0, 1000.0, 25.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::rtd("RTD-1", 0.0, -120.0, 100.0, 25.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::strain("STR-1", 75.0, -120.0, 120.0, 0.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::flipflop("FF-1", 150.0, -120.0, "D", false, "pos"));

    // 2. Logic & Instruments
    canvas.scene_mut().add_saved_item(Item::latch(
        "LAT-1", -150.0, -40.0, 4, false, false, "pos", false,
    ));
    canvas.scene_mut().add_saved_item(Item::test_unit(
        "TU-1",
        -75.0,
        -40.0,
        "in0,in1",
        "out0",
        1e-7,
        vec![],
    ));
    canvas
        .scene_mut()
        .add_saved_item(Item::lanalizer("LA-1", 0.0, -40.0, true));
    canvas
        .scene_mut()
        .add_saved_item(Item::oscope("OSC-1", 75.0, -40.0, true));
    canvas
        .scene_mut()
        .add_saved_item(Item::bus("BUS-1", 150.0, -40.0, 8));

    // 3. Connectors & Serial Interfaces
    canvas
        .scene_mut()
        .add_saved_item(Item::socket("SOC-1", -150.0, 50.0, 8));
    canvas
        .scene_mut()
        .add_saved_item(Item::header("HDR-1", -75.0, 50.0, 8));
    canvas
        .scene_mut()
        .add_saved_item(Item::serial_port("SP-1", 0.0, 50.0, "COM1", 9600));
    canvas
        .scene_mut()
        .add_saved_item(Item::serial_term("ST-1", 75.0, 50.0, 9600));
    canvas
        .scene_mut()
        .add_saved_item(Item::stepper("STP-1", 150.0, 50.0, false, 32, 100.0));

    // 4. Motors & Displays
    canvas
        .scene_mut()
        .add_saved_item(Item::dcmotor("DCM-1", -150.0, 150.0, 60, 5.0, 100.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::servo("SRV-1", -75.0, 150.0, 0.2, 1000.0, 2000.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::hd44780("LCD-1", 0.0, 150.0, 2, 16));
    canvas.scene_mut().add_saved_item(Item::ssd1306(
        "OLED-1", 75.0, 150.0, 128, 64, 0x3C, "White", true, 100.0,
    ));

    // Rasterize viewport using preview canvas renderer
    let pm = render_viewport(&canvas, &pal, 800, 600, 1.0, true).expect("render batch5");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 600);

    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 1000,
        "Rendered pixmap must contain painted elements, got {non_bg}"
    );
}

#[test]
fn test_drawable_batch6_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Multiplexers & Decoders
    canvas
        .scene_mut()
        .add_saved_item(Item::mux("MUX-1", -150.0, -120.0, 2));
    canvas
        .scene_mut()
        .add_saved_item(Item::demux("DEMUX-1", -75.0, -120.0, 2, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::bcd_to_dec("B2D-1", 0.0, -120.0, false, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::dec_to_bcd("D2B-1", 75.0, -120.0, false, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::bcd_to_7s("B27-1", 150.0, -120.0, false));

    // 2. Arithmetic & Registers
    canvas
        .scene_mut()
        .add_saved_item(Item::full_adder("FA-1", -150.0, -40.0, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::half_adder("HA-1", -75.0, -40.0, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::counter("CTR-1", 0.0, -40.0, 4, 15));
    canvas
        .scene_mut()
        .add_saved_item(Item::bin_counter("BCTR-1", 75.0, -40.0, false));
    canvas
        .scene_mut()
        .add_saved_item(Item::shift_reg("SR-1", 150.0, -40.0, 8));

    // 3. Converters & Interfaces
    canvas
        .scene_mut()
        .add_saved_item(Item::magnitude_comp("MAG-1", -150.0, 50.0, 4));
    canvas
        .scene_mut()
        .add_saved_item(Item::adc("ADC-1", -75.0, 50.0, 8, 5.0, 0.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::dac("DAC-1", 0.0, 50.0, 8, 5.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::i2c_to_parallel("I2CP-1", 75.0, 50.0, 0x20));
    canvas
        .scene_mut()
        .add_saved_item(Item::lm555("555-1", 150.0, 50.0));

    // 4. Memories & Functions
    canvas
        .scene_mut()
        .add_saved_item(Item::memory("RAM-1", -150.0, 150.0, 8, 8, false, vec![]));
    canvas
        .scene_mut()
        .add_saved_item(Item::dynamic_memory("DRAM-1", -75.0, 150.0, 8, vec![]));
    canvas
        .scene_mut()
        .add_saved_item(Item::i2c_ram("I2CR-1", 0.0, 150.0, 256, 0x50, vec![]));
    canvas
        .scene_mut()
        .add_saved_item(Item::function("FUNC-1", 75.0, 150.0, 3, "I0 & I1"));

    // Rasterize viewport using preview canvas renderer
    let pm = render_viewport(&canvas, &pal, 800, 600, 1.0, true).expect("render batch6");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 600);

    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 1000,
        "Rendered pixmap must contain painted elements, got {non_bg}"
    );
}

#[test]
fn test_drawable_batch7_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::new();
    let pal = Palette::light();

    // 1. Shapes & Packages
    canvas
        .scene_mut()
        .add_saved_item(Item::shape("SHAPE-1", -150.0, -150.0, "Rectangle"));
    canvas.scene_mut().add_saved_item(Item::subcircuit(
        "SUB-1",
        -75.0,
        -150.0,
        "74HC00",
        cs_engine::package::Package::default(),
        "",
        None,
        false,
    ));
    canvas.scene_mut().add_saved_item(Item::subpackage(
        "SPKG-1",
        0.0,
        -150.0,
        cs_engine::package::Package::default(),
    ));
    let mcu_view = cs_engine::mcu::McuView {
        mcu: cs_engine::mcu::McuComp::default(),
        package: cs_engine::package::Package::default(),
        packages: std::collections::BTreeMap::new(),
        logic_symbol: false,
    };
    canvas
        .scene_mut()
        .add_saved_item(Item::mcu("MCU-1", 75.0, -150.0, mcu_view));
    let qemu_view = cs_engine::qemu::QemuView {
        qemu: cs_engine::qemu::QemuComp::default(),
        package: cs_engine::package::Package::default(),
        packages: std::collections::BTreeMap::new(),
        logic_symbol: false,
    };
    canvas
        .scene_mut()
        .add_saved_item(Item::qemu_device("QEMU-1", 150.0, -150.0, qemu_view));

    // 2. Sensors & Modules
    canvas
        .scene_mut()
        .add_saved_item(Item::sr04("SR04-1", -150.0, -50.0, 1.0, true));
    canvas.scene_mut().add_saved_item(Item::dht22(
        "DHT-1", -75.0, -50.0, "DHT22", 25.0, 50.0, 0.5, 1.0,
    ));
    canvas
        .scene_mut()
        .add_saved_item(Item::ds18b20("DS18-1", 0.0, -50.0, "28FF", 25.0, 0.5));
    canvas
        .scene_mut()
        .add_saved_item(Item::ds1621("DS16-1", 75.0, -50.0, 25.0, 0.5));
    canvas
        .scene_mut()
        .add_saved_item(Item::ds1307("DS13-1", 150.0, -50.0, true));

    // 3. Storage & Wireless
    canvas
        .scene_mut()
        .add_saved_item(Item::sdcard("SD-1", -150.0, 50.0, "card.img"));
    canvas
        .scene_mut()
        .add_saved_item(Item::esp01("ESP-1", -75.0, 50.0, 115200, false));

    // 4. Displays
    canvas
        .scene_mut()
        .add_saved_item(Item::tft_display("TFT-1", 0.0, 50.0, "ILI9341", 80, 60));
    canvas
        .scene_mut()
        .add_saved_item(Item::pcd8544("PCD-1", 75.0, 50.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::sh1107("SH-1", 150.0, 50.0, 64, 64));
    canvas
        .scene_mut()
        .add_saved_item(Item::ks0108("KS-1", -100.0, 150.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::pcf8833("PCF-1", 0.0, 150.0));
    canvas
        .scene_mut()
        .add_saved_item(Item::aip31068("AIP-1", 100.0, 150.0, 2, 16));

    // Rasterize viewport using preview canvas renderer
    let pm = render_viewport(&canvas, &pal, 800, 600, 1.0, true).expect("render batch7");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 600);

    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 1000,
        "Rendered pixmap must contain painted elements, got {non_bg}"
    );
}

#[test]
fn test_oscilloscope_and_logic_analyzer_visual_rendering() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};

    let mut canvas = Canvas::default();
    let pal = Palette::dark();

    // 1. Oscilloscope with custom tunnels
    let mut osc = Item::oscope("Osc-1", 0.0, 0.0, true);
    osc.set_prop_text("Tunnel1", "SIGNAL_IN");
    osc.set_prop_text("Tunnel2", "CLK_OUT");
    canvas.scene_mut().add_saved_item(osc);

    // 2. Logic Analyzer with custom tunnels
    let mut la = Item::lanalizer("LA-1", 300.0, 0.0, true);
    la.set_prop_text("Tunnel1", "D0");
    la.set_prop_text("Tunnel2", "D1");
    canvas.scene_mut().add_saved_item(la);

    // Verify SVG export contains expected elements
    let svg = cs_engine::canvas::export::svg_string(&canvas, &pal);
    assert!(
        svg.contains("SIGNAL_IN"),
        "SVG must contain custom tunnel name SIGNAL_IN"
    );
    assert!(
        svg.contains("CLK_OUT"),
        "SVG must contain custom tunnel name CLK_OUT"
    );
    assert!(svg.contains("GND"), "SVG must contain GND label");
    assert!(
        svg.contains("D0"),
        "SVG must contain Logic Analyzer channel label D0"
    );
    assert!(
        svg.contains("#ffff00"),
        "SVG must contain Ch0 yellow swatch/trace"
    );

    // Verify viewport rasterization
    let pm = render_viewport(&canvas, &pal, 800, 400, 1.0, true).expect("render oscope");
    assert_eq!(pm.width(), 800);
    assert_eq!(pm.height(), 400);

    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 2000,
        "Rendered pixmap must contain rich painted instrument elements, got {non_bg}"
    );
}

#[test]
fn test_inductor_and_var_inductor_geometry_and_pin_contact() {
    use cs_engine::canvas::{Canvas, Palette, render_viewport};
    use cs_engine::components::Component;

    // 1. Inductor component & item pins
    let ind = Item::inductor("L-1", 0.0, 0.0, 1e-3);
    let ind_pins = ind.pins();
    assert_eq!(ind_pins.len(), 2);
    assert_eq!(ind_pins[0].local.x, -16.0);
    assert_eq!(ind_pins[0].local.y, 0.0);
    assert_eq!(ind_pins[0].length, 4.0);
    assert_eq!(ind_pins[0].local.x + ind_pins[0].length, -12.0);

    assert_eq!(ind_pins[1].local.x, 16.0);
    assert_eq!(ind_pins[1].local.y, 0.0);
    assert_eq!(ind_pins[1].length, 4.0);
    assert_eq!(ind_pins[1].local.x - ind_pins[1].length, 12.0);

    let comp_ind = cs_engine::components::Inductor::default();
    let comp_ind_pins = comp_ind.pin_geoms();
    assert_eq!(comp_ind_pins[0].length, 4.0);
    assert_eq!(comp_ind_pins[1].length, 4.0);

    // 2. VarInductor component & item pins (must use 4.0 to meet coil at x = ±12.0)
    let var_ind = Item::var_inductor("VL-1", 100.0, 0.0, 2e-3);
    let var_pins = var_ind.pins();
    assert_eq!(var_pins.len(), 2);
    assert_eq!(var_pins[0].local.x, -16.0);
    assert_eq!(var_pins[0].local.y, 0.0);
    assert_eq!(var_pins[0].length, 4.0);
    assert_eq!(var_pins[0].local.x + var_pins[0].length, -12.0);

    assert_eq!(var_pins[1].local.x, 16.0);
    assert_eq!(var_pins[1].local.y, 0.0);
    assert_eq!(var_pins[1].length, 4.0);
    assert_eq!(var_pins[1].local.x - var_pins[1].length, 12.0);

    let comp_var = cs_engine::components::VarInductor::default();
    let comp_var_pins = comp_var.pin_geoms();
    assert_eq!(comp_var_pins[0].length, 4.0);
    assert_eq!(comp_var_pins[1].length, 4.0);

    // 3. Canvas rendering & SVG export
    let mut canvas = Canvas::default();
    let pal = Palette::dark();
    canvas.scene_mut().add_saved_item(ind);
    canvas.scene_mut().add_saved_item(var_ind);

    let svg = cs_engine::canvas::export::svg_string(&canvas, &pal);
    assert!(!svg.is_empty(), "SVG string must not be empty");

    let pm = render_viewport(&canvas, &pal, 400, 300, 1.0, true).expect("render inductors");
    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 500,
        "Rendered pixmap must contain painted inductor elements, got {non_bg}"
    );
}

#[test]
fn test_transformer_geometry_and_pin_contact() {
    use cs_engine::canvas::{Canvas, Palette, Rect, render_viewport};
    use cs_engine::components::Component;

    // 1. Transformer component and item pin checks
    let tr = Item::transformer("TR-1", 0.0, 0.0, 1.0, 1.0, 0.99);
    let pins = tr.pins();
    assert_eq!(pins.len(), 4);

    // Primary top pin: (-16, 0), length 6 -> contacts (-10, 0)
    assert_eq!(pins[0].local.x, -16.0);
    assert_eq!(pins[0].local.y, 0.0);
    assert_eq!(pins[0].angle, 180);
    assert_eq!(pins[0].length, 6.0);
    assert_eq!(pins[0].local.x + pins[0].length, -10.0);

    // Primary bottom pin: (-16, 24), length 6 -> contacts (-10, 24)
    assert_eq!(pins[1].local.x, -16.0);
    assert_eq!(pins[1].local.y, 24.0);
    assert_eq!(pins[1].angle, 180);
    assert_eq!(pins[1].length, 6.0);
    assert_eq!(pins[1].local.x + pins[1].length, -10.0);

    // Secondary top pin: (16, 0), length 6 -> contacts (10, 0)
    assert_eq!(pins[2].local.x, 16.0);
    assert_eq!(pins[2].local.y, 0.0);
    assert_eq!(pins[2].angle, 0);
    assert_eq!(pins[2].length, 6.0);
    assert_eq!(pins[2].local.x - pins[2].length, 10.0);

    // Secondary bottom pin: (16, 24), length 6 -> contacts (10, 24)
    assert_eq!(pins[3].local.x, 16.0);
    assert_eq!(pins[3].local.y, 24.0);
    assert_eq!(pins[3].angle, 0);
    assert_eq!(pins[3].length, 6.0);
    assert_eq!(pins[3].local.x - pins[3].length, 10.0);

    let comp_tr = cs_engine::components::Transformer::default();
    assert_eq!(comp_tr.body(), Rect::new(-12.0, 0.0, 24.0, 24.0));
    let comp_pins = comp_tr.pin_geoms();
    assert_eq!(comp_pins.len(), 4);
    assert_eq!(comp_pins[0].local.y, 0.0);
    assert_eq!(comp_pins[1].local.y, 24.0);
    assert_eq!(comp_pins[2].local.y, 0.0);
    assert_eq!(comp_pins[3].local.y, 24.0);

    // 2. Canvas viewport rendering
    let mut canvas = Canvas::default();
    let pal = Palette::light();
    canvas.scene_mut().add_saved_item(tr);

    let pm = render_viewport(&canvas, &pal, 400, 300, 1.0, true).expect("render transformer");
    let non_bg = pm.data().iter().filter(|&&b| b != 255).count();
    assert!(
        non_bg > 200,
        "Rendered pixmap must contain painted transformer, got {non_bg}"
    );
}

#[test]
fn test_switch_smart_pin_aliases_and_tooltips() {
    use cs_engine::canvas::Canvas;
    use cs_engine::simulation::Circuit;

    let mut canvas = Canvas::new();
    let sw_id = canvas.scene_mut().add_switch(0.0, 0.0, true);

    let get_tip = |canvas: &Canvas, sw_id: &str, suffix: &str| -> String {
        let pin_id = format!("{sw_id}{suffix}");
        let it = canvas.scene().item_by_id(sw_id).unwrap();
        let p = it.pins().into_iter().find(|p| p.id == pin_id).unwrap();
        let pt = it.map_local(p.local);
        canvas.hover_tooltip(pt).unwrap()
    };

    // 1. Tooltips for single-throw SPST: Terminal A & B
    assert!(get_tip(&canvas, &sw_id, "-pinP0").contains("Switch terminal A"));
    assert!(get_tip(&canvas, &sw_id, "-switch0pinN").contains("Switch terminal B"));

    // 2. Tooltips for 2-pole DPST (single-throw): Pole 0/1 Terminal A/B
    canvas.open_properties(&sw_id);
    canvas.set_prop_text("Poles".to_string(), "2".to_string());
    assert!(get_tip(&canvas, &sw_id, "-pinP0").contains("Pole 0 \u{2013} terminal A"));
    assert!(get_tip(&canvas, &sw_id, "-switch0pinN").contains("Pole 0 \u{2013} terminal B"));
    assert!(get_tip(&canvas, &sw_id, "-pinP1").contains("Pole 1 \u{2013} terminal A"));
    assert!(get_tip(&canvas, &sw_id, "-switch1pinN").contains("Pole 1 \u{2013} terminal B"));

    // 3. Tooltips for 2-pole DPDT (double-throw): Pole 0/1 Common (COM), NC, NO
    canvas.set_prop_bool("DoubleThrow".to_string(), true);
    assert!(get_tip(&canvas, &sw_id, "-pinP0").contains("Pole 0 common terminal (COM)"));
    assert!(get_tip(&canvas, &sw_id, "-switch0pinN").contains("Pole 0, throw 0 (NC)"));
    assert!(get_tip(&canvas, &sw_id, "-switch1pinN").contains("Pole 0, throw 1 (NO)"));
    assert!(get_tip(&canvas, &sw_id, "-pinP1").contains("Pole 1 common terminal (COM)"));
    assert!(get_tip(&canvas, &sw_id, "-switch2pinN").contains("Pole 1, throw 0 (NC)"));
    assert!(get_tip(&canvas, &sw_id, "-switch3pinN").contains("Pole 1, throw 1 (NO)"));

    // 4. Pin alias connectivity in Circuit: connecting via lPin/rPin aliases works
    let mut circ = Circuit::new();
    circ.add_fixed_volt("V1", 5.0)
        .add_switch_with("SW", true, 2, false)
        .add_resistor("R1", 1000.0)
        .add_ground("GND");

    // Connect pole 0 using lPin0 / rPin0 aliases
    circ.connect("V1-outnod", "SW-lPin0")
        .connect("SW-rPin0", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");

    let _ = circ.solve();
    let v_r = circ.pin_voltage("R1-lPin").expect("R1 voltage");
    assert!(
        (v_r - 5.0).abs() < 1e-3,
        "Switch connected via lPin0/rPin0 must conduct 5V, got {v_r}"
    );

    let i_sw = circ
        .current_out_of_pin("SW-lPin0")
        .expect("SW lPin0 current");
    assert!(
        (i_sw - (-0.005)).abs() < 1e-4,
        "Current through lPin0 must be -5mA, got {i_sw}"
    );
}

#[test]
fn test_ssd1306_visual_pixel_rendering() {
    let mut canvas = Canvas::new();
    canvas.scene_add_component_spec("SSD1306", Point::new(200.0, 200.0));
    let id = canvas.scene().items()[0].id.clone();

    // Default off/blank
    let pal = Palette::light();
    let svg_blank = svg_string(&canvas, &pal);
    assert!(svg_blank.contains("fill=\"#324664\"")); // Casing color

    // Set reading with a single dot in page 0, col 10 (0x01)
    let hex = "00".repeat(128 * 8);
    let mut hex_vec: Vec<u8> = hex.into_bytes();
    hex_vec[20] = b'0';
    hex_vec[21] = b'1';
    let hex_str = String::from_utf8(hex_vec).unwrap();

    canvas.set_reading(
        &id,
        cs_engine::instruments::ReadingView {
            text: hex_str,
            max_text: String::new(),
            avg_text: String::new(),
            extra: String::new(),
            high: false,
            low: false,
            hz_text: String::new(),
        },
    );

    let svg_active = svg_string(&canvas, &pal);
    // Should contain foreground pixel fill #f5f5f5 (245, 245, 245)
    assert!(
        svg_active.contains("fill=\"#f5f5f5\""),
        "SVG must contain drawn OLED pixel with color #f5f5f5"
    );
}
