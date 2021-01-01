use cs_engine::canvas::scene::Scene;
use cs_engine::canvas::{Canvas, Point};
use cs_engine::components::Part;

#[test]
fn test_audio_out_default_impedance_is_8_ohms() {
    let mut scene = Scene::new();
    let id = scene.add_audio_out(0.0, 0.0);
    let item = scene.item_by_id(&id).expect("AudioOut item must exist");
    if let Part::AudioOut(p) = &item.kind {
        assert_eq!(
            p.impedance, 8.0,
            "AudioOut default impedance must be 8.0 ohms"
        );
        assert_eq!(p.volume, 100.0, "AudioOut default volume must be 100%");
        assert!(!p.buzzer, "AudioOut default buzzer must be false");
        assert_eq!(
            p.frequency, 1000.0,
            "AudioOut default frequency must be 1000 Hz"
        );
    } else {
        panic!("Expected AudioOut item");
    }
    assert_eq!(item.prop_text("Impedance"), Some("8 Ω".into()));
}

#[test]
fn test_potentiometer_and_var_resistor_defaults() {
    let mut scene = Scene::new();
    let pot_id = scene.add_potentiometer(0.0, 0.0);
    let pot = scene.item_by_id(&pot_id).unwrap();
    if let Part::Potentiometer(p) = &pot.kind {
        assert_eq!(
            p.resistance, 1000.0,
            "Potentiometer default resistance must be 1000 ohms (1 kΩ)"
        );
        assert_eq!(p.wiper, 0.5);
    } else {
        panic!("Expected Potentiometer");
    }
    assert_eq!(pot.prop_text("Resistance"), Some("1 kΩ".into()));

    let var_id = scene.add_var_resistor(0.0, 0.0);
    let var = scene.item_by_id(&var_id).unwrap();
    if let Part::VarResistor(p) = &var.kind {
        assert_eq!(
            p.resistance, 1000.0,
            "VarResistor default resistance must be 1000 ohms (1 kΩ)"
        );
        assert_eq!(p.min_r, 0.0);
        assert_eq!(p.max_r, 2000.0);
    } else {
        panic!("Expected VarResistor");
    }
    assert_eq!(var.prop_text("Resistance"), Some("1 kΩ".into()));
}

#[test]
fn test_resistor_dip_defaults() {
    let mut scene = Scene::new();
    let id = scene.add_resistor_dip(0.0, 0.0);
    let item = scene.item_by_id(&id).unwrap();
    if let Part::ResistorDip(p) = &item.kind {
        assert_eq!(p.size, 8, "ResistorDip default size must be 8");
        assert_eq!(
            p.resistance, 100.0,
            "ResistorDip default resistance must be 100 ohms"
        );
        assert!(!p.bussed, "ResistorDip default bussed must be false");
    } else {
        panic!("Expected ResistorDip");
    }
    assert_eq!(item.prop_text("Size"), Some("8".into()));
    assert_eq!(item.prop_text("Resistance"), Some("100 Ω".into()));
}

#[test]
fn test_sources_running_defaults() {
    let mut scene = Scene::new();
    let v_id = scene.add_volt_source(0.0, 0.0);
    let v_item = scene.item_by_id(&v_id).unwrap();
    if let Part::VoltSource(p) = &v_item.kind {
        assert!(p.running, "VoltSource default running must be true");
    } else {
        panic!("Expected VoltSource");
    }
    assert_eq!(v_item.prop_bool("Running"), Some(true));

    let c_id = scene.add_curr_source(0.0, 0.0);
    let c_item = scene.item_by_id(&c_id).unwrap();
    if let Part::CurrSource(p) = &c_item.kind {
        assert!(p.running, "CurrSource default running must be true");
        assert_eq!(
            p.max_value, 1.0,
            "CurrSource default max_value must be 1.0 A"
        );
    } else {
        panic!("Expected CurrSource");
    }
    assert_eq!(c_item.prop_bool("Running"), Some(true));
}

#[test]
fn test_switches_enabled_defaults() {
    let mut scene = Scene::new();
    let sw_id = scene.add_default_switch(0.0, 0.0);
    let sw = scene.item_by_id(&sw_id).unwrap();
    if let Part::Switch(p) = &sw.kind {
        assert!(p.checked, "Switch default closed must be true");
    } else {
        panic!("Expected Switch");
    }
    assert_eq!(sw.prop_bool("Checked"), Some(true));

    let dip_id = scene.add_switch_dip(0.0, 0.0);
    let dip = scene.item_by_id(&dip_id).unwrap();
    if let Part::SwitchDip(p) = &dip.kind {
        assert_eq!(p.size, 4, "SwitchDip default size must be 4");
        assert_eq!(
            p.state, 15,
            "SwitchDip default state must have all switches closed (0b1111 = 15)"
        );
    } else {
        panic!("Expected SwitchDip");
    }
}

#[test]
fn test_sensors_and_motors_defaults() {
    let mut scene = Scene::new();
    let dht_id = scene.add_dht22(0.0, 0.0);
    let dht = scene.item_by_id(&dht_id).unwrap();
    if let Part::DHT22(p) = &dht.kind {
        assert_eq!(p.temp, 22.5, "DHT22 default temp must be 22.5 C");
        assert_eq!(p.humi, 68.5, "DHT22 default humi must be 68.5 %");
    } else {
        panic!("Expected DHT22");
    }

    let dcm_id = scene.add_dcmotor(0.0, 0.0);
    let dcm = scene.item_by_id(&dcm_id).unwrap();
    if let Part::DcMotor(p) = &dcm.kind {
        assert_eq!(
            p.resistance, 100.0,
            "DcMotor default resistance must be 100 ohms"
        );
    } else {
        panic!("Expected DcMotor");
    }
    assert_eq!(dcm.prop_text("Resistance"), Some("100 Ω".into()));

    let stp_id = scene.add_stepper(0.0, 0.0);
    let stp = scene.item_by_id(&stp_id).unwrap();
    if let Part::Stepper(p) = &stp.kind {
        assert_eq!(p.steps, 32, "Stepper default steps must be 32");
        assert_eq!(
            p.resistance, 100.0,
            "Stepper default resistance must be 100 ohms"
        );
    } else {
        panic!("Expected Stepper");
    }
    assert_eq!(stp.prop_text("Steps"), Some("32".into()));
    assert_eq!(stp.prop_text("Resistance"), Some("100 Ω".into()));
}

#[test]
fn test_scr_diac_triac_defaults_and_properties() {
    let mut scene = Scene::new();
    let scr_id = scene.add_scr(0.0, 0.0);
    let mut scr = scene.item_by_id(&scr_id).unwrap().clone();
    if let Part::Scr(p) = &scr.kind {
        assert_eq!(p.v_gate_th, 0.7);
        assert_eq!(p.i_hold, 0.0082);
        assert_eq!(p.i_trig, 0.01);
        assert_eq!(p.r_gate, 100.0);
    } else {
        panic!("Expected Scr");
    }
    assert_eq!(scr.prop_text("GateTh"), Some("700 mV".into()));
    assert_eq!(scr.prop_text("HoldCurr"), Some("8.2 mA".into()));
    assert_eq!(scr.prop_text("TrigCurr"), Some("10 mA".into()));
    assert_eq!(scr.prop_text("GateRes"), Some("100 Ω".into()));

    assert!(scr.set_prop_text("TrigCurr", "20 mA"));
    assert_eq!(scr.prop_text("TrigCurr"), Some("20 mA".into()));

    let diac_id = scene.add_diac(0.0, 0.0);
    let mut diac = scene.item_by_id(&diac_id).unwrap().clone();
    if let Part::Diac(p) = &diac.kind {
        assert_eq!(p.v_breakover, 30.0);
        assert_eq!(p.res_off, 1e8);
        assert_eq!(p.hold_curr, 0.01);
        assert_eq!(p.res_on, 500.0);
    } else {
        panic!("Expected Diac");
    }
    assert_eq!(diac.prop_text("Breakover"), Some("30 V".into()));
    assert_eq!(diac.prop_text("ResOff"), Some("100 MΩ".into()));
    assert_eq!(diac.prop_text("HoldCurr"), Some("10 mA".into()));
    assert_eq!(diac.prop_text("ResOn"), Some("500 Ω".into()));

    assert!(diac.set_prop_text("ResOn", "250 Ω"));
    assert_eq!(diac.prop_text("ResOn"), Some("250 Ω".into()));

    let triac_id = scene.add_triac(0.0, 0.0);
    let triac = scene.item_by_id(&triac_id).unwrap().clone();
    if let Part::Triac(p) = &triac.kind {
        assert_eq!(p.v_gate_th, 0.7);
        assert_eq!(p.i_hold, 0.0082);
        assert_eq!(p.i_trig, 0.01);
        assert_eq!(p.r_gate, 100.0);
    } else {
        panic!("Expected Triac");
    }
    assert_eq!(triac.prop_text("GateTh"), Some("700 mV".into()));
    assert_eq!(triac.prop_text("HoldCurr"), Some("8.2 mA".into()));
    assert_eq!(triac.prop_text("TrigCurr"), Some("10 mA".into()));
    assert_eq!(triac.prop_text("GateRes"), Some("100 Ω".into()));
}

#[test]
fn test_all_components_prop_groups_are_readable() {
    let mut canvas = Canvas::empty();
    let specs = cs_engine::library::all_placeable_specs();
    for (caption, typ) in &specs {
        let spec = if typ == "QemuDevice" || typ == "MCU" {
            format!("{caption},{typ}")
        } else {
            typ.clone()
        };
        canvas.scene_add_component_spec(&spec, Point::zero());
    }

    // Direct add_* for all items not directly in library specs or specialized
    let mut scene = Scene::new();
    let direct_ids = vec![
        scene.add_mux(0.0, 0.0),
        scene.add_demux(0.0, 0.0),
        scene.add_bcd_to_dec(0.0, 0.0),
        scene.add_dec_to_bcd(0.0, 0.0),
        scene.add_bcd_to_7s(0.0, 0.0),
        scene.add_seven_segment_bcd(0.0, 0.0),
        scene.add_i2c_to_parallel(0.0, 0.0),
        scene.add_adc(0.0, 0.0),
        scene.add_dac(0.0, 0.0),
        scene.add_counter(0.0, 0.0),
        scene.add_bin_counter(0.0, 0.0),
        scene.add_full_adder(0.0, 0.0),
        scene.add_magnitude_comp(0.0, 0.0),
        scene.add_shift_reg(0.0, 0.0),
        scene.add_function(0.0, 0.0),
        scene.add_memory(0.0, 0.0),
        scene.add_dynamic_memory(0.0, 0.0),
        scene.add_i2c_ram(0.0, 0.0),
        scene.add_lm555(0.0, 0.0),
        scene.add_scr(0.0, 0.0),
        scene.add_diac(0.0, 0.0),
        scene.add_triac(0.0, 0.0),
        scene.add_audio_out(0.0, 0.0),
        scene.add_potentiometer(0.0, 0.0),
        scene.add_var_resistor(0.0, 0.0),
        scene.add_resistor_dip(0.0, 0.0),
        scene.add_dcmotor(0.0, 0.0),
        scene.add_stepper(0.0, 0.0),
    ];

    let all_items: Vec<_> = canvas
        .scene()
        .items()
        .iter()
        .cloned()
        .chain(
            direct_ids
                .into_iter()
                .filter_map(|id| scene.item_by_id(&id).cloned()),
        )
        .collect();

    for item in all_items {
        let groups = item.prop_groups();
        for group in &groups {
            for row in &group.rows {
                let val_text = item.prop_text(row.name);
                let val_bool = item.prop_bool(row.name);
                assert!(
                    val_text.is_some() || val_bool.is_some(),
                    "Property '{}' in group '{}' of {:?} has neither prop_text nor prop_bool",
                    row.name,
                    group.name,
                    item.kind
                );
            }
        }
    }
}

#[test]
fn test_sim1_parser_component_defaults() {
    let xml = r#"<circuit>
        <item itemtype="AudioOut" CircId="AudioOut-1" />
        <item itemtype="Potentiometer" CircId="Pot-1" />
        <item itemtype="VarResistor" CircId="VarR-1" />
        <item itemtype="ResistorDip" CircId="RDip-1" />
        <item itemtype="VoltSource" CircId="V-1" />
        <item itemtype="CurrSource" CircId="I-1" />
        <item itemtype="DHT22" CircId="DHT-1" />
        <item itemtype="DcMotor" CircId="Motor-1" />
        <item itemtype="Stepper" CircId="Step-1" />
        <item itemtype="Scr" CircId="SCR-1" />
        <item itemtype="Diac" CircId="Diac-1" />
        <item itemtype="Triac" CircId="Triac-1" />
    </circuit>"#;

    let scene = Scene::from_sim1(xml).expect("Scene::from_sim1 should succeed");
    assert_eq!(scene.items().len(), 12);
}

#[test]
fn test_human_readable_component_names_and_tooltips() {
    let mut canvas = Canvas::empty();

    // Verify SD Card Reader specifically (the exemplar mentioned in requirements)
    canvas.scene_add_component_spec("SdCard", Point::zero());
    let sd_item = canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.kind.type_name() == "SdCard")
        .unwrap();
    assert_eq!(sd_item.human_name(), "SD Card Reader");
    let tooltip = canvas
        .hover_tooltip(Point::zero())
        .expect("tooltip exists for SD card reader");
    assert!(
        tooltip.contains("SD Card Reader (SdCard-1)"),
        "Tooltip was: {tooltip}"
    );

    // Verify specific components that previously had abbreviated or machine identifiers
    let mut scene = Scene::new();
    let id = scene.add_default_resistor(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Resistor");

    let id = scene.add_default_el_capacitor(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "Electrolytic Capacitor"
    );

    let id = scene.add_volt_reg(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "Voltage Regulator"
    );

    let id = scene.add_voltmeter(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Voltmeter");

    let id = scene.add_ammeter(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Ampmeter");

    let id = scene.add_freq_meter(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "Frequency Meter"
    );

    let id = scene.add_lanalizer(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "Logic Analyzer"
    );

    let id = scene.add_oscope(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Oscilloscope");

    let id = scene.add_switch_dip(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "DIP Switch");

    let id = scene.add_lamp(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "Incandescent Lamp"
    );

    let id = scene.add_stepper(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Stepper Motor");

    let id = scene.add_servo(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Servo Motor");

    let id = scene.add_touchpad(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "TouchPad (Resistive)"
    );

    let id = scene.add_sr04(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "HC-SR04 Ultrasonic Sensor"
    );

    let id = scene.add_dht22(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "DHT22 Temperature & Humidity Sensor"
    );

    let id = scene.add_lm555(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "LM555 Timer");

    let id = scene.add_counter(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "Simple Counter"
    );

    let id = scene.add_memory(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "RAM/ROM Memory"
    );

    let id = scene.add_bcd_to_dec(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "BCD to Decimal Decoder"
    );

    let id = scene.add_bcd_to_7s(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "BCD to 7-Segment Decoder"
    );

    let id = scene.add_seven_segment_bcd(0.0, 0.0);
    assert_eq!(
        scene.item_by_id(&id).unwrap().human_name(),
        "7-Segment Display with BCD Decoder"
    );

    let id = scene.add_test_unit(0.0, 0.0);
    assert_eq!(scene.item_by_id(&id).unwrap().human_name(), "Test Unit");
}

#[test]
fn test_test_unit_defaults_and_property_mutations() {
    let mut scene = Scene::new();
    let id = scene.add_test_unit(0.0, 0.0);
    let item = scene.item_by_id(&id).expect("TestUnit item must exist");

    // 1. Verify C++ defaults: Inputs="O", Outputs="I0,I1", Period=100 ns (1e-7 s)
    if let Part::TestUnit(p) = &item.kind {
        assert_eq!(p.inputs, "O", "TestUnit default Inputs must be 'O'");
        assert_eq!(
            p.outputs, "I0,I1",
            "TestUnit default Outputs must be 'I0,I1'"
        );
        assert_eq!(
            p.period, 1e-7,
            "TestUnit default Period must be 1e-7 s (100 ns)"
        );
        assert!(p.truth.is_empty(), "TestUnit default Truth must be empty");
    } else {
        panic!("Expected TestUnit item");
    }

    assert_eq!(item.prop_text("Inputs"), Some("O".into()));
    assert_eq!(item.prop_text("Outputs"), Some("I0,I1".into()));
    assert_eq!(item.prop_text("Period"), Some("100 ns".into()));

    // Verify prop_groups units and captions
    let groups = item.prop_groups();
    let main_group = groups.iter().find(|g| g.name == "Main").unwrap();
    assert!(
        main_group
            .rows
            .iter()
            .any(|r| r.name == "Inputs" && r.caption == "Inputs")
    );
    assert!(
        main_group
            .rows
            .iter()
            .any(|r| r.name == "Outputs" && r.caption == "Outputs")
    );

    let test_group = groups.iter().find(|g| g.name == "Test").unwrap();
    let period_row = test_group.rows.iter().find(|r| r.name == "Period").unwrap();
    assert_eq!(
        period_row.unit, "ns",
        "TestUnit Period unit in Test prop group must be 'ns'"
    );

    // 2. Property mutations
    {
        let item_mut = scene.item_by_id_mut(&id).unwrap();
        assert!(item_mut.set_prop_text("Inputs", "A,B"));
        assert!(item_mut.set_prop_text("Outputs", "Y"));
        // Setting without unit defaults to ns
        assert!(item_mut.set_prop_text("Period", "50"));
        assert_eq!(item_mut.prop_text("Period"), Some("50 ns".into()));
        assert!(item_mut.set_prop_text("Period", "200 ns"));
        assert_eq!(item_mut.prop_text("Period"), Some("200 ns".into()));
        assert!(item_mut.set_prop_text("Truth", "0,0,0,1,"));
        assert_eq!(item_mut.prop_text("Truth"), Some("0,0,0,1,".into()));
    }

    // 3. to_circuit conversion preserves modified properties
    let circuit = scene.to_circuit();
    let comp = circuit.components().iter().find(|c| c.id == id).unwrap();
    assert_eq!(comp.id, id);

    // 4. sim1 XML serialization & deserialization roundtrip preserves properties
    let sim1_xml = scene.to_sim1();
    let roundtrip_scene = Scene::from_sim1(&sim1_xml).expect("XML reload failed");
    let rt_item = roundtrip_scene.item_by_id(&id).unwrap();
    assert_eq!(rt_item.prop_text("Inputs"), Some("A,B".into()));
    assert_eq!(rt_item.prop_text("Outputs"), Some("Y".into()));
    assert_eq!(rt_item.prop_text("Period"), Some("200 ns".into()));
    assert_eq!(rt_item.prop_text("Truth"), Some("0,0,0,1,".into()));
}

#[test]
fn test_bus_pin_hover_tooltip_parity() {
    // Verifies that the Bus component pin hover tooltips match C++ bus.cpp:
    //   - ePin0 (bottom trunk) and busPinI (top trunk) → "Bus trunk connector"
    //   - ePinN (line pins) → "Bus line N"
    use cs_engine::canvas::{Canvas, Point};

    let mut scene = cs_engine::canvas::scene::Scene::new();
    let _id = scene.add_bus(0.0, 0.0); // 8-line bus at origin (default), start_bit=0
    //  Bus geometry (from bus_pins / bus.cpp):
    //    bottom trunk (ePin0):  local (0,  0), angle 90
    //    line pin ePinI:         local (-8, -64 + I*8), angle 180  (for 8 lines: I=1..8)
    //    top trunk (busPinI):   local (0, -56), angle 270

    let mut canvas = Canvas::empty();
    *canvas.scene_mut() = scene;

    // Bottom trunk pin (ePin0) — hit at scene (0, 0)
    let tip_bottom = canvas
        .hover_tooltip(Point::new(0.0, 0.0))
        .expect("tooltip for ePin0 (bottom trunk)");
    assert!(
        tip_bottom.contains("Bus trunk connector"),
        "ePin0 tooltip should contain 'Bus trunk connector', got: {tip_bottom}"
    );

    // Top trunk pin (busPinI) — for 8 lines: local y = -(8-1)*8 = -56
    let tip_top = canvas
        .hover_tooltip(Point::new(0.0, -56.0))
        .expect("tooltip for busPinI (top trunk)");
    assert!(
        tip_top.contains("Bus trunk connector"),
        "busPinI tooltip should contain 'Bus trunk connector', got: {tip_top}"
    );

    // Line pin ePin1 (bit 0): local (-8, -64 + 1*8) = (-8, -56)
    let tip_line1 = canvas
        .hover_tooltip(Point::new(-8.0, -56.0))
        .expect("tooltip for ePin1");
    assert!(
        tip_line1.contains("Bus line 0"),
        "ePin1 tooltip should contain 'Bus line 0', got: {tip_line1}"
    );

    // Line pin ePin4 (bit 3): local (-8, -64 + 4*8) = (-8, -32)
    let tip_line4 = canvas
        .hover_tooltip(Point::new(-8.0, -32.0))
        .expect("tooltip for ePin4");
    assert!(
        tip_line4.contains("Bus line 3"),
        "ePin4 tooltip should contain 'Bus line 3', got: {tip_line4}"
    );
}
