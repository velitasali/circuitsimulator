use cs_engine::canvas::scene::Scene;
use cs_engine::{CERO_DOUB, Circuit};

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn test_potentiometer_circuit() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 10.0);
    c.add_ground("GND");
    c.add_comp(cs_engine::elements::Comp {
        id: "Pot1".into(),
        kind: cs_engine::elements::Kind::Potentiometer {
            resistance: 10_000.0,
            wiper: 0.25,
        },
    });
    c.add_resistor("RLoad", 1e9);
    c.connect("V1-outnod", "Pot1-rPin");
    c.connect("Pot1-lPin", "GND-Gnd");
    c.connect("Pot1-wPin", "RLoad-lPin");
    c.connect("RLoad-rPin", "GND-Gnd");
    c.solve().unwrap();

    approx(c.pin_voltage("Pot1-rPin").unwrap(), 10.0);
    approx(c.pin_voltage("Pot1-lPin").unwrap(), CERO_DOUB);
    approx(c.pin_voltage("Pot1-wPin").unwrap(), 2.5);
}

#[test]
fn test_push_switch_and_relay() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0);
    c.add_ground("GND");
    c.add_comp(cs_engine::elements::Comp {
        id: "Push1".into(),
        kind: cs_engine::elements::Kind::Push {
            closed: true,
            poles: 1,
        },
    });
    c.add_resistor("R1", 1000.0);
    c.connect("V1-outnod", "Push1-lPin0");
    c.connect("Push1-rPin0", "R1-lPin");
    c.connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();

    approx(c.pin_voltage("R1-lPin").unwrap(), 5.0);

    let mut c_open = Circuit::new();
    c_open.add_fixed_volt("V1", 5.0);
    c_open.add_ground("GND");
    c_open.add_comp(cs_engine::elements::Comp {
        id: "Push1".into(),
        kind: cs_engine::elements::Kind::Push {
            closed: false,
            poles: 1,
        },
    });
    c_open.add_resistor("R1", 1000.0);
    c_open.connect("V1-outnod", "Push1-lPin0");
    c_open.connect("Push1-rPin0", "R1-lPin");
    c_open.connect("R1-rPin", "GND-Gnd");
    c_open.solve().unwrap();

    approx(c_open.pin_voltage("R1-lPin").unwrap(), CERO_DOUB);
}

#[test]
fn test_switch_dip_and_resistor_dip() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0);
    c.add_ground("GND");
    c.add_comp(cs_engine::elements::Comp {
        id: "Dip1".into(),
        kind: cs_engine::elements::Kind::SwitchDip {
            size: 4,
            state: 0b0001, // Switch 0 closed, others open
            common_pin: false,
        },
    });
    c.add_comp(cs_engine::elements::Comp {
        id: "RDip1".into(),
        kind: cs_engine::elements::Kind::ResistorDip {
            size: 4,
            resistance: 1000.0,
            bussed: false,
        },
    });
    c.connect("V1-outnod", "Dip1-pin0");
    c.connect("Dip1-pin1", "RDip1-lPin0");
    c.connect("RDip1-rPin0", "GND-Gnd");
    c.solve().unwrap();

    approx(c.pin_voltage("RDip1-lPin0").unwrap(), 5.0);
}

#[test]
fn test_sensors_and_variable_passives() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 10.0);
    c.add_ground("GND");
    c.add_comp(cs_engine::elements::Comp {
        id: "LDR1".into(),
        kind: cs_engine::elements::Kind::Ldr {
            resistance: 2000.0,
            lux: 50.0,
        },
    });
    c.add_resistor("R1", 2000.0);
    c.connect("V1-outnod", "LDR1-lPin");
    c.connect("LDR1-rPin", "R1-lPin");
    c.connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();

    approx(c.pin_voltage("R1-lPin").unwrap(), 5.0);
}

#[test]
fn test_rail_and_lamp() {
    let mut c = Circuit::new();
    c.add_comp(cs_engine::elements::Comp {
        id: "Rail1".into(),
        kind: cs_engine::elements::Kind::Rail { voltage: 12.0 },
    });
    c.add_comp(cs_engine::elements::Comp {
        id: "Lamp1".into(),
        kind: cs_engine::elements::Kind::Lamp {
            voltage: 12.0,
            power: 24.0,
            r_cold: 6.0,
            resistance: 6.0,
        },
    });
    c.add_ground("GND");
    c.connect("Rail1-outnod", "Lamp1-lPin");
    c.connect("Lamp1-rPin", "GND-Gnd");
    c.solve().unwrap();

    approx(c.pin_voltage("Rail1-outnod").unwrap(), 12.0);
    approx(c.pin_voltage("Lamp1-lPin").unwrap(), 12.0);
}

#[test]
fn test_scene_catalog_roundtrip() {
    let mut s = Scene::new();
    s.add_clock(10.0, 10.0);
    s.add_rail(20.0, 20.0);
    s.add_wave_gen(30.0, 30.0);
    s.add_volt_source(40.0, 40.0);
    s.add_curr_source(50.0, 50.0);
    s.add_csource(60.0, 60.0);
    s.add_push(70.0, 70.0);
    s.add_switch_dip(80.0, 80.0);
    s.add_relay(90.0, 90.0);
    s.add_keypad(100.0, 100.0);
    s.add_potentiometer(110.0, 110.0);
    s.add_var_resistor(120.0, 120.0);
    s.add_resistor_dip(130.0, 130.0);
    s.add_ldr(140.0, 140.0);
    s.add_thermistor(150.0, 150.0);
    s.add_rtd(160.0, 160.0);
    s.add_strain(170.0, 170.0);
    s.add_var_capacitor(180.0, 180.0);
    s.add_var_inductor(190.0, 190.0);
    s.add_transformer(200.0, 200.0);
    s.add_scr(210.0, 210.0);
    s.add_diac(220.0, 220.0);
    s.add_triac(230.0, 230.0);
    s.add_analog_mux(240.0, 240.0);
    s.add_rgb_led(250.0, 250.0);
    s.add_led_bar(260.0, 260.0);
    s.add_seven_segment(270.0, 270.0);
    s.add_led_matrix(280.0, 280.0);
    s.add_max72xx(290.0, 290.0);
    s.add_ws2812(300.0, 300.0);
    s.add_audio_out(310.0, 310.0);
    s.add_lamp(320.0, 320.0);
    s.add_tunnel(330.0, 330.0);
    s.add_bus(340.0, 340.0);
    s.add_socket(350.0, 350.0);
    s.add_header(360.0, 360.0);
    s.add_serial_port(370.0, 370.0);
    s.add_serial_term(380.0, 380.0);
    s.add_dial(390.0, 390.0);
    s.add_and_gate(400.0, 400.0);
    s.add_or_gate(410.0, 410.0);
    s.add_xor_gate(420.0, 420.0);
    s.add_buffer(430.0, 430.0);
    s.add_flipflop_d(440.0, 440.0);
    s.add_flipflop_jk(450.0, 450.0);
    s.add_flipflop_rs(460.0, 460.0);
    s.add_flipflop_t(470.0, 470.0);
    s.add_latch_d(480.0, 480.0);
    s.add_test_unit(490.0, 490.0);
    s.add_shape("Rectangle", 500.0, 500.0);

    let count_before = s.items().len();
    let sim1 = s.to_sim1();
    assert!(sim1.contains("Clock"));
    assert!(sim1.contains("Rail"));
    assert!(sim1.contains("Potentiometer"));
    assert!(sim1.contains("Relay"));
    assert!(sim1.contains("AndGate"));

    let loaded = Scene::from_sim1(&sim1).unwrap();
    assert_eq!(loaded.items().len(), count_before);
    let kp = loaded
        .items()
        .iter()
        .find(|it| it.id.starts_with("KeyPad"))
        .expect("KeyPad item present");
    assert!(
        matches!(
            &kp.kind,
            cs_engine::components::Part::KeyPad(p) if p.rows == 4 && p.cols == 4
        ),
        "KeyPad item should remain KeyPad after loading from sim1, but got {:?}",
        kp.kind
    );
    let lm = loaded
        .items()
        .iter()
        .find(|it| it.id.starts_with("LedMatrix"))
        .expect("LedMatrix item present");
    assert!(
        matches!(
            &lm.kind,
            cs_engine::components::Part::LedMatrix(p) if p.rows == 8 && p.cols == 8
        ),
        "LedMatrix item should remain LedMatrix after loading from sim1, but got {:?}",
        lm.kind
    );
    let max = loaded
        .items()
        .iter()
        .find(|it| it.id.starts_with("Max72xx"))
        .expect("Max72xx item present");
    assert!(
        matches!(
            &max.kind,
            cs_engine::components::Part::Max72xx(p) if p.modules == 1
        ),
        "Max72xx item should remain Max72xx after loading from sim1, but got {:?}",
        max.kind
    );
    let ws = loaded
        .items()
        .iter()
        .find(|it| it.id.starts_with("WS2812"))
        .expect("WS2812 item present");
    assert!(
        matches!(
            &ws.kind,
            cs_engine::components::Part::Ws2812(p) if p.count == 8
        ),
        "WS2812 item should remain Ws2812 after loading from sim1, but got {:?}",
        ws.kind
    );
    let dial = loaded
        .items()
        .iter()
        .find(|it| it.id.starts_with("Dial"))
        .expect("Dial item present");
    assert!(
        matches!(&dial.kind, cs_engine::components::Part::Dial(_)),
        "Dial item should remain Dial after loading from sim1, but got {:?}",
        dial.kind
    );
    let shape = loaded
        .items()
        .iter()
        .find(|it| it.id.starts_with("Rectangle"))
        .expect("Rectangle shape item present");
    assert!(
        matches!(&shape.kind, cs_engine::components::Part::Shape(p) if p.shape_kind == cs_engine::components::ShapeKind::Rectangle),
        "Rectangle shape should remain Shape after loading from sim1, but got {:?}",
        shape.kind
    );
}
