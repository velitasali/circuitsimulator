use cs_engine::canvas::Canvas;
use cs_engine::{CERO_DOUB, Circuit};

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-2, "{a} != {b}");
}

#[test]
fn test_ky023_joystick_simulation() {
    let mut c = Circuit::new();
    c.add_ground("gnd");
    c.add_resistor("r_x", 1e7);
    c.add_resistor("r_y", 1e7);
    c.add_resistor("r_sw", 1e7);
    c.add_ky023("joy", 15.0, -10.0, false);

    c.connect("joy-vrx", "r_x-lPin");
    c.connect("r_x-rPin", "gnd-Gnd");
    c.connect("joy-vry", "r_y-lPin");
    c.connect("r_y-rPin", "gnd-Gnd");
    c.connect("joy-sw", "r_sw-lPin");
    c.connect("r_sw-rPin", "gnd-Gnd");

    c.solve().unwrap();
    // stick_x = 15.0 -> norm = (15.0 + 25.0)/50.0 = 0.8 -> 5V * 0.8 = 4.0V
    approx(c.pin_voltage("joy-vrx").unwrap(), 4.0);
    // stick_y = -10.0 -> norm = (-10.0 + 25.0)/50.0 = 0.3 -> 5V * 0.3 = 1.5V
    approx(c.pin_voltage("joy-vry").unwrap(), 1.5);
    // SW button not pressed -> pulled up to 5V
    approx(c.pin_voltage("joy-sw").unwrap(), 5.0);

    // Now press button
    let mut c_down = Circuit::new();
    c_down.add_ground("gnd");
    c_down.add_resistor("r_sw", 1e7);
    c_down.add_ky023("joy", 0.0, 0.0, true);
    c_down.connect("joy-sw", "r_sw-lPin");
    c_down.connect("r_sw-rPin", "gnd-Gnd");
    c_down.solve().unwrap();
    // SW button pressed -> pulled down to GND
    approx(c_down.pin_voltage("joy-sw").unwrap(), CERO_DOUB);
}

#[test]
fn test_ky040_rotary_encoder_simulation() {
    let mut c = Circuit::new();
    c.add_ground("gnd");
    c.add_resistor("r_clk", 1e7);
    c.add_resistor("r_dt", 1e7);
    c.add_resistor("r_sw", 1e7);
    c.add_ky040("enc", 20, 1, false, true, false);

    c.connect("enc-clk", "r_clk-lPin");
    c.connect("r_clk-rPin", "gnd-Gnd");
    c.connect("enc-dt", "r_dt-lPin");
    c.connect("r_dt-rPin", "gnd-Gnd");
    c.connect("enc-sw", "r_sw-lPin");
    c.connect("r_sw-rPin", "gnd-Gnd");

    c.solve().unwrap();
    // state_a = true -> CLK=5V, state_b = false -> DT=0V
    approx(c.pin_voltage("enc-clk").unwrap(), 5.0);
    approx(c.pin_voltage("enc-dt").unwrap(), CERO_DOUB);
    approx(c.pin_voltage("enc-sw").unwrap(), 5.0);
}

#[test]
fn test_dcmotor_simulation() {
    let mut c = Circuit::new();
    c.add_fixed_volt("src", 10.0);
    c.add_ground("gnd");
    c.add_dcmotor("motor", 120, 5.0, 10.0);

    c.connect("src-outnod", "motor-lPin");
    c.connect("gnd-Gnd", "motor-rPin");

    c.solve().unwrap();
    approx(c.pin_voltage("motor-lPin").unwrap(), 10.0);
    approx(c.pin_voltage("motor-rPin").unwrap(), CERO_DOUB);
    // 10V across 10Ω coil = 1A
    let current = (10.0 - 0.0) / 10.0;
    approx(current, 1.0);

    // Speed ratio: 10V / 5V nominal = 2.0
    let comp = c
        .components()
        .iter()
        .find(|comp| comp.id == "motor")
        .unwrap();
    if let cs_engine::elements::Kind::DcMotor { speed, angle, .. } = comp.kind {
        approx(speed, 2.0);
        approx(angle, 0.0);
    } else {
        panic!("expected DcMotor");
    }

    // Run 125ms: 120 RPM * 2.0 speed = 4 rev/sec = 1440 deg/sec * 0.125s = 180 deg
    c.run_ps(125_000_000_000).unwrap();
    let comp = c
        .components()
        .iter()
        .find(|comp| comp.id == "motor")
        .unwrap();
    if let cs_engine::elements::Kind::DcMotor { speed, angle, .. } = comp.kind {
        approx(speed, 2.0);
        approx(angle, 180.0);
    } else {
        panic!("expected DcMotor");
    }

    // Verify pin lengths touch the border: (±40, 0) with length 12 -> reaches ±28
    let item = cs_engine::canvas::Item::dcmotor("motor", 0.0, 0.0, 120, 5.0, 10.0);
    let pins = item.pins();
    assert_eq!(pins.len(), 2);
    assert_eq!(pins[0].length, 12.0);
    assert_eq!(pins[1].length, 12.0);
}

#[test]
fn test_sensors_and_actuators_canvas_and_sim1_roundtrip() {
    let mut canvas = Canvas::new();

    assert!(canvas.scene_add_component_spec("KY023", cs_engine::canvas::Point::new(100.0, 100.0)));
    assert!(canvas.scene_add_component_spec("KY040", cs_engine::canvas::Point::new(200.0, 100.0)));
    assert!(canvas.scene_add_component_spec("SR04", cs_engine::canvas::Point::new(300.0, 100.0)));
    assert!(canvas.scene_add_component_spec("DHT22", cs_engine::canvas::Point::new(400.0, 100.0)));
    assert!(
        canvas.scene_add_component_spec("DS18B20", cs_engine::canvas::Point::new(500.0, 100.0))
    );
    assert!(canvas.scene_add_component_spec("DS1621", cs_engine::canvas::Point::new(600.0, 100.0)));
    assert!(canvas.scene_add_component_spec("DS1307", cs_engine::canvas::Point::new(700.0, 100.0)));
    assert!(
        canvas.scene_add_component_spec("DcMotor", cs_engine::canvas::Point::new(100.0, 300.0))
    );
    assert!(
        canvas.scene_add_component_spec("Stepper", cs_engine::canvas::Point::new(250.0, 300.0))
    );
    assert!(canvas.scene_add_component_spec("Servo", cs_engine::canvas::Point::new(400.0, 300.0)));
    assert!(canvas.scene_add_component_spec("SdCard", cs_engine::canvas::Point::new(550.0, 300.0)));
    assert!(canvas.scene_add_component_spec("Esp01", cs_engine::canvas::Point::new(700.0, 300.0)));

    // Verify all 12 items exist
    assert_eq!(canvas.scene().items().len(), 12);

    // Test XML export & re-import
    let xml = canvas.scene().to_sim1();
    assert!(xml.contains("itemtype=\"KY023\""));
    assert!(xml.contains("itemtype=\"KY040\""));
    assert!(xml.contains("itemtype=\"SR04\""));
    assert!(xml.contains("itemtype=\"DHT22\""));
    assert!(xml.contains("itemtype=\"DS18B20\""));
    assert!(xml.contains("itemtype=\"DS1621\""));
    assert!(xml.contains("itemtype=\"DS1307\""));
    assert!(xml.contains("itemtype=\"DcMotor\""));
    assert!(xml.contains("itemtype=\"Stepper\""));
    assert!(xml.contains("itemtype=\"Servo\""));
    assert!(xml.contains("itemtype=\"SdCard\""));
    assert!(xml.contains("itemtype=\"Esp01\""));

    let mut canvas2 = Canvas::new();
    canvas2.import_sim1(&xml);
    assert_eq!(canvas2.scene().items().len(), 12);
}

#[test]
fn test_interactive_canvas_mutations() {
    let mut canvas = Canvas::new();
    canvas.scene_add_component_spec("KY023", cs_engine::canvas::Point::new(100.0, 100.0));
    canvas.scene_add_component_spec("KY040", cs_engine::canvas::Point::new(200.0, 100.0));
    canvas.scene_add_component_spec("SR04", cs_engine::canvas::Point::new(300.0, 100.0));
    canvas.scene_add_component_spec("DHT22", cs_engine::canvas::Point::new(400.0, 100.0));

    let ky023_id = canvas.scene().items()[0].id.clone();
    let ky040_id = canvas.scene().items()[1].id.clone();
    let sr04_id = canvas.scene().items()[2].id.clone();
    let dht22_id = canvas.scene().items()[3].id.clone();

    // Test joystick manipulation
    canvas.set_joystick_pos(&ky023_id, 0.75, -0.5);
    canvas.set_joystick_button(&ky023_id, true);
    approx(
        canvas.scene().item_by_id(&ky023_id).unwrap().stick_x(),
        0.75,
    );
    approx(
        canvas.scene().item_by_id(&ky023_id).unwrap().stick_y(),
        -0.5,
    );
    assert!(canvas.scene().item_by_id(&ky023_id).unwrap().btn_down());

    // Test rotary encoder dial
    canvas.set_rotary_dial(&ky040_id, 5);
    canvas.set_rotary_button(&ky040_id, true);
    assert_eq!(canvas.scene().item_by_id(&ky040_id).unwrap().dial_val(), 5);
    assert!(canvas.scene().item_by_id(&ky040_id).unwrap().btn_closed());

    // Test ultrasonic distance
    canvas.set_sr04_distance(&sr04_id, 1.25);
    approx(
        canvas.scene().item_by_id(&sr04_id).unwrap().distance(),
        1.25,
    );

    // Test temperature step
    canvas.step_sensor_temp(&dht22_id, true);
    approx(canvas.scene().item_by_id(&dht22_id).unwrap().temp(), 23.0);
    canvas.step_sensor_humi(&dht22_id, false);
    approx(canvas.scene().item_by_id(&dht22_id).unwrap().humi(), 63.5);
}

#[test]
fn test_dcmotor_canvas_simulation_tick() {
    let sim1_xml = r#"<circuit version="1.0.0">
<item itemtype="FixedVolt" CircId="Fixed-1" Voltage="5 V" />
<item itemtype="DcMotor" CircId="DcMotor-1" RpmNominal="60" VoltNominal="5 V" Resistance="10 Ω" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="c1" startpinid="Fixed-1-outnod" endpinid="DcMotor-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="DcMotor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>"#;

    let mut canvas = Canvas::new();
    canvas.load_sim1(sim1_xml, None).unwrap();

    // Power on simulation
    canvas.power_on();
    assert!(canvas.sim_running());

    // Before ticks: speed is 1.0 (5V / 5V nominal = 1.0)
    let it = canvas.scene().item_by_id("DcMotor-1").unwrap();
    approx(it.speed(), 1.0);

    // Run tick: 60 RPM at speed 1.0 rotates 360 deg/sec
    let change = canvas.tick();
    assert!(change.anim || change.items);

    let it = canvas.scene().item_by_id("DcMotor-1").unwrap();
    approx(it.speed(), 1.0);
    assert!(it.angle() > 0.0);

    // Power off: speed resets to 0.0
    canvas.power_off();
    let it = canvas.scene().item_by_id("DcMotor-1").unwrap();
    approx(it.speed(), 0.0);
}

#[test]
fn test_ky040_quadrature_gray_code_and_joystick_normalized() {
    let sim1_xml = r#"<circuit>
<item itemtype="Ground" CircId="gnd" />
<item itemtype="Resistor" CircId="r_vx" Resistance="10 MΩ" />
<item itemtype="Resistor" CircId="r_vy" Resistance="10 MΩ" />
<item itemtype="Resistor" CircId="r_jsw" Resistance="10 MΩ" />
<item itemtype="KY023" CircId="joy" />
<item itemtype="Connector" uid="c1" startpinid="joy-vrx" endpinid="r_vx-lPin" />
<item itemtype="Connector" uid="c2" startpinid="r_vx-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c3" startpinid="joy-vry" endpinid="r_vy-lPin" />
<item itemtype="Connector" uid="c4" startpinid="r_vy-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c5" startpinid="joy-sw" endpinid="r_jsw-lPin" />
<item itemtype="Connector" uid="c6" startpinid="r_jsw-rPin" endpinid="gnd-Gnd" />

<item itemtype="Resistor" CircId="r_clk" Resistance="10 MΩ" />
<item itemtype="Resistor" CircId="r_dt" Resistance="10 MΩ" />
<item itemtype="Resistor" CircId="r_esw" Resistance="10 MΩ" />
<item itemtype="KY040" CircId="enc" />
<item itemtype="Connector" uid="c7" startpinid="enc-clk" endpinid="r_clk-lPin" />
<item itemtype="Connector" uid="c8" startpinid="r_clk-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c9" startpinid="enc-dt" endpinid="r_dt-lPin" />
<item itemtype="Connector" uid="c10" startpinid="r_dt-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c11" startpinid="enc-sw" endpinid="r_esw-lPin" />
<item itemtype="Connector" uid="c12" startpinid="r_esw-rPin" endpinid="gnd-Gnd" />
</circuit>"#;

    let mut canvas = Canvas::new();
    canvas.load_sim1(sim1_xml, None).unwrap();

    canvas.power_on();

    // Test joystick normalized voltage outputs (center = 2.5V, right = 5.0V, left = 0.0V)
    canvas.set_joystick_pos("joy", 0.0, 0.0);
    approx(canvas.pin_voltage("joy-vrx").unwrap(), 2.5);
    approx(canvas.pin_voltage("joy-vry").unwrap(), 2.5);

    canvas.set_joystick_pos("joy", 1.0, -1.0);
    approx(canvas.pin_voltage("joy-vrx").unwrap(), 5.0);
    approx(canvas.pin_voltage("joy-vry").unwrap(), 0.0);

    // Test button
    canvas.set_joystick_button("joy", true);
    approx(canvas.pin_voltage("joy-sw").unwrap(), 0.0);
    canvas.set_joystick_button("joy", false);
    approx(canvas.pin_voltage("joy-sw").unwrap(), 5.0);

    // Test KY-040 rotary encoder quadrature Gray code sequence
    // k = val.rem_euclid(4)
    // k = 0 (val = 0): CLK = 0, DT = 0
    canvas.set_rotary_dial("enc", 0);
    approx(canvas.pin_voltage("enc-clk").unwrap(), 0.0);
    approx(canvas.pin_voltage("enc-dt").unwrap(), 0.0);

    // k = 1 (val = 1): CLK = 1, DT = 0
    canvas.set_rotary_dial("enc", 1);
    approx(canvas.pin_voltage("enc-clk").unwrap(), 5.0);
    approx(canvas.pin_voltage("enc-dt").unwrap(), 0.0);

    // k = 2 (val = 2): CLK = 1, DT = 1
    canvas.set_rotary_dial("enc", 2);
    approx(canvas.pin_voltage("enc-clk").unwrap(), 5.0);
    approx(canvas.pin_voltage("enc-dt").unwrap(), 5.0);

    // k = 3 (val = 3): CLK = 0, DT = 1
    canvas.set_rotary_dial("enc", 3);
    approx(canvas.pin_voltage("enc-clk").unwrap(), 0.0);
    approx(canvas.pin_voltage("enc-dt").unwrap(), 5.0);

    // Push button
    canvas.set_rotary_button("enc", true);
    approx(canvas.pin_voltage("enc-sw").unwrap(), 0.0);
    canvas.set_rotary_button("enc", false);
    approx(canvas.pin_voltage("enc-sw").unwrap(), 5.0);
}

#[test]
fn test_joystick_and_encoder_canvas_mouse_interactions() {
    use cs_engine::canvas::{BUTTON_LEFT, Point};

    let sim1_xml = r#"<circuit>
<item itemtype="Ground" CircId="gnd" Pos="0,0" />
<item itemtype="Resistor" CircId="r_vx" Resistance="10 MΩ" Pos="0,0" />
<item itemtype="Resistor" CircId="r_vy" Resistance="10 MΩ" Pos="0,0" />
<item itemtype="Resistor" CircId="r_jsw" Resistance="10 MΩ" Pos="0,0" />
<item itemtype="KY023" CircId="joy" Pos="200,200" />
<item itemtype="Connector" uid="c1" startpinid="joy-vrx" endpinid="r_vx-lPin" />
<item itemtype="Connector" uid="c2" startpinid="r_vx-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c3" startpinid="joy-vry" endpinid="r_vy-lPin" />
<item itemtype="Connector" uid="c4" startpinid="r_vy-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c5" startpinid="joy-sw" endpinid="r_jsw-lPin" />
<item itemtype="Connector" uid="c6" startpinid="r_jsw-rPin" endpinid="gnd-Gnd" />

<item itemtype="Resistor" CircId="r_clk" Resistance="10 MΩ" Pos="0,0" />
<item itemtype="Resistor" CircId="r_dt" Resistance="10 MΩ" Pos="0,0" />
<item itemtype="Resistor" CircId="r_esw" Resistance="10 MΩ" Pos="0,0" />
<item itemtype="KY040" CircId="enc" Pos="400,200" />
<item itemtype="Connector" uid="c7" startpinid="enc-clk" endpinid="r_clk-lPin" />
<item itemtype="Connector" uid="c8" startpinid="r_clk-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c9" startpinid="enc-dt" endpinid="r_dt-lPin" />
<item itemtype="Connector" uid="c10" startpinid="r_dt-rPin" endpinid="gnd-Gnd" />
<item itemtype="Connector" uid="c11" startpinid="enc-sw" endpinid="r_esw-lPin" />
<item itemtype="Connector" uid="c12" startpinid="r_esw-rPin" endpinid="gnd-Gnd" />
</circuit>"#;

    let mut canvas = Canvas::new();
    canvas.load_sim1(sim1_xml, None).unwrap();
    canvas.power_on();

    // 1. Initial State
    // Joystick center: VRX = 2.5V, VRY = 2.5V, SW = 5V
    approx(canvas.pin_voltage("joy-vrx").unwrap(), 2.5);
    approx(canvas.pin_voltage("joy-vry").unwrap(), 2.5);
    approx(canvas.pin_voltage("joy-sw").unwrap(), 5.0);

    // 2. Click Pushbutton on Joystick
    // Button rect local: (8.0, 15.0, 10.0, 10.0) -> scene center is (213.0, 220.0)
    let screen_btn = canvas.viewport().map_from_circuit(Point::new(213.0, 220.0));
    let ch_press_btn = canvas.mouse_press(BUTTON_LEFT, screen_btn.x, screen_btn.y, 0);
    assert!(!ch_press_btn.items);
    assert!(canvas.take_dirty().items.contains("joy"));
    // SW pin should drop to 0V
    approx(canvas.pin_voltage("joy-sw").unwrap(), 0.0);

    // Release Pushbutton
    let ch_rel_btn = canvas.mouse_release(BUTTON_LEFT, screen_btn.x, screen_btn.y, 0);
    assert!(!ch_rel_btn.items);
    approx(canvas.pin_voltage("joy-sw").unwrap(), 5.0);

    // 3. Thumbstick Drag on Joystick
    // Thumbstick base center is at local (0.0, -8.0) -> scene (200.0, 192.0)
    let screen_stick = canvas.viewport().map_from_circuit(Point::new(200.0, 192.0));
    let ch_press_stick = canvas.mouse_press(BUTTON_LEFT, screen_stick.x, screen_stick.y, 0);
    assert!(!ch_press_stick.items);

    // Drag thumbstick to right (scene: 215.0, 192.0 -> local dx=15, clamped to max 13.0)
    let screen_right = canvas.viewport().map_from_circuit(Point::new(215.0, 192.0));
    let ch_move_stick = canvas.mouse_move(screen_right.x, screen_right.y, BUTTON_LEFT, 0);
    assert!(!ch_move_stick.items);
    assert!(canvas.take_dirty().items.contains("joy"));

    // stick_x clamped to 13.0 -> norm = (13.0+25.0)/50.0 = 0.76 -> 5.0 * 0.76 = 3.8V
    approx(canvas.pin_voltage("joy-vrx").unwrap(), 3.8);
    approx(canvas.pin_voltage("joy-vry").unwrap(), 2.5);

    // Release thumbstick -> should snap back to (0.0, 0.0)
    let ch_rel_stick = canvas.mouse_release(BUTTON_LEFT, screen_right.x, screen_right.y, 0);
    assert!(!ch_rel_stick.items);
    approx(canvas.pin_voltage("joy-vrx").unwrap(), 2.5);
    approx(canvas.pin_voltage("joy-vry").unwrap(), 2.5);

    // 4. Rotary Encoder Pushbutton
    // Knob center is at scene (400.0, 200.0)
    let screen_enc = canvas.viewport().map_from_circuit(Point::new(400.0, 200.0));
    let ch_press_enc = canvas.mouse_press(BUTTON_LEFT, screen_enc.x, screen_enc.y, 0);
    assert!(!ch_press_enc.items);
    assert!(canvas.take_dirty().items.contains("enc"));
    // enc-sw drops to 0V
    approx(canvas.pin_voltage("enc-sw").unwrap(), 0.0);

    let ch_rel_enc = canvas.mouse_release(BUTTON_LEFT, screen_enc.x, screen_enc.y, 0);
    assert!(!ch_rel_enc.items);
    approx(canvas.pin_voltage("enc-sw").unwrap(), 5.0);

    // 5. Rotary Encoder Wheel rotation
    // Scroll wheel over encoder knob
    canvas.wheel(0.0, 120.0, 0.0, 120.0, screen_enc.x, screen_enc.y, 0);
    assert!(canvas.take_dirty().items.contains("enc"));
    // Dial advanced by 1 step -> Gray code k=2 (CLK=5V, DT=5V)
    approx(canvas.pin_voltage("enc-clk").unwrap(), 5.0);
    approx(canvas.pin_voltage("enc-dt").unwrap(), 5.0);
}

#[test]
fn test_servo_motor_simulation() {
    let mut c = Circuit::new();
    c.add_fixed_volt("vcc", 5.0);
    c.add_ground("gnd");
    c.add_fixed_volt("sig", 0.0);
    c.add_servo("srv", 0.2, 1000.0, 2000.0);

    c.connect("vcc-outnod", "srv-PinV+");
    c.connect("gnd-Gnd", "srv-PinGnd");
    c.connect("sig-outnod", "srv-PinSig");

    c.solve().unwrap();

    let get_pos = |circ: &Circuit| -> (f64, f64) {
        let comp = circ.components().iter().find(|x| x.id == "srv").unwrap();
        if let cs_engine::elements::Kind::Servo {
            pos, target_pos, ..
        } = comp.kind
        {
            (pos, target_pos)
        } else {
            panic!("expected Kind::Servo");
        }
    };

    let set_sig = |circ: &mut Circuit, volt: f64| {
        circ.update_component_kind(
            "sig",
            &cs_engine::elements::Kind::FixedVolt { voltage: volt },
        );
        circ.re_solve().unwrap();
    };

    let (pos, target) = get_pos(&c);
    approx(pos, 90.0);
    approx(target, 90.0);

    // Pulse of 1000 µs (1 ms) -> Target = 0 deg
    // 1. Rising edge on Sig
    set_sig(&mut c, 5.0);
    c.run_ps(1_000_000_000).unwrap(); // 1 ms

    // 2. Falling edge on Sig
    set_sig(&mut c, 0.0);
    c.run_ps(19_000_000_000).unwrap(); // Rest of 20ms period

    let (_, target) = get_pos(&c);
    approx(target, 0.0);

    // Run for 0.4s to allow servo to rotate to 0 deg (speed is 0.2 s/60° -> 90° takes 0.3s)
    c.run_ps(400_000_000_000).unwrap();
    let (pos, _) = get_pos(&c);
    approx(pos, 0.0);

    // Pulse of 2000 µs (2 ms) -> Target = 180 deg
    // 1. Rising edge
    set_sig(&mut c, 5.0);
    c.run_ps(2_000_000_000).unwrap(); // 2 ms

    // 2. Falling edge
    set_sig(&mut c, 0.0);
    c.run_ps(18_000_000_000).unwrap();

    let (_, target) = get_pos(&c);
    approx(target, 180.0);

    // Run for 0.7s to rotate 180 deg (180° takes 0.6s at 0.2 s/60°)
    c.run_ps(700_000_000_000).unwrap();
    let (pos, _) = get_pos(&c);
    approx(pos, 180.0);
}

#[test]
fn test_servo_motor_canvas_sync() {
    let sim1 = r#"
<circuit version="1.0.0" width="1000" height="1000">
  <item itemtype="FixedVolt" CircId="vcc" Voltage="5 V" Pos="0,0" />
  <item itemtype="Ground" CircId="gnd" Pos="0,50" />
  <item itemtype="FixedVolt" CircId="sig" Voltage="0 V" Pos="50,0" />
  <item itemtype="Servo" CircId="srv" Speed="0.2" MinPulse="1000" MaxPulse="2000" Pos="100,0" />
  <item itemtype="Connector" startpinid="vcc-outnod" endpinid="srv-PinV+" />
  <item itemtype="Connector" startpinid="gnd-Gnd" endpinid="srv-PinGnd" />
  <item itemtype="Connector" startpinid="sig-outnod" endpinid="srv-PinSig" />
</circuit>
"#;
    let mut canvas = Canvas::new();
    canvas.load_sim1(sim1, None).unwrap();

    let ch_on = canvas.power_on();
    assert!(ch_on.sim);

    // Set Sig to 5V for 1.0ms
    if let Some(item) = canvas.scene_mut().item_by_id_mut("sig") {
        if let cs_engine::canvas::Part::FixedVolt(ref mut fv) = item.kind {
            fv.voltage = 5.0;
        }
    }
    canvas.refresh_sim_component("sig");

    // Advance 1ms
    for _ in 0..10 {
        canvas.tick();
    }

    // Set Sig to 0V
    if let Some(item) = canvas.scene_mut().item_by_id_mut("sig") {
        if let cs_engine::canvas::Part::FixedVolt(ref mut fv) = item.kind {
            fv.voltage = 0.0;
        }
    }
    canvas.refresh_sim_component("sig");

    // Advance 500ms
    for _ in 0..50 {
        canvas.tick();
    }

    let servo_item = canvas.scene().item_by_id("srv").unwrap();
    if let cs_engine::canvas::Part::Servo(ref s) = servo_item.kind {
        approx(s.pos, 180.0);
    } else {
        panic!("expected Part::Servo");
    }

    canvas.power_off();
}
