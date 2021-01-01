use cs_engine::canvas::scene::{Item, Scene};

#[test]
fn test_all_components_have_descriptions_and_props() {
    let mut scene = Scene::new();

    // Add all kinds of items
    let _r_id = scene.add_default_resistor(0.0, 0.0);
    let _b_id = scene.add_default_battery(10.0, 0.0);
    let gnd_id = scene.add_ground(20.0, 0.0);
    let _fv_id = scene.add_default_fixed_volt(30.0, 0.0);
    let node_id = scene.add_node(40.0, 0.0);
    let _cap_id = scene.add_default_capacitor(50.0, 0.0);
    let _elcap_id = scene.add_default_el_capacitor(60.0, 0.0);
    let _ind_id = scene.add_default_inductor(70.0, 0.0);
    let _sw_id = scene.add_switch(80.0, 0.0, false);
    let _diode_id = scene.add_diode(90.0, 0.0, false);
    let _zener_id = scene.add_diode(100.0, 0.0, true);
    let _led_id = scene.add_led(110.0, 0.0);
    let _bjt_id = scene.add_bjt(120.0, 0.0, false);
    let _mosfet_id = scene.add_mosfet(130.0, 0.0, false, false);
    let _opamp_id = scene.add_opamp(140.0, 0.0);
    let _jfet_id = scene.add_jfet(150.0, 0.0);
    let _comp_id = scene.add_comparator(160.0, 0.0);
    let _vr_id = scene.add_volt_reg(170.0, 0.0);
    let _probe_id = scene.add_probe(180.0, 0.0);
    let _vm_id = scene.add_voltmeter(190.0, 0.0);
    let _am_id = scene.add_ammeter(200.0, 0.0);
    let _fm_id = scene.add_freq_meter(210.0, 0.0);
    let _osc_id = scene.add_oscope(220.0, 0.0);
    let _la_id = scene.add_lanalizer(230.0, 0.0);
    let _clk_id = scene.add_clock(240.0, 0.0);
    let _rail_id = scene.add_rail(250.0, 0.0);
    let _wg_id = scene.add_wave_gen(260.0, 0.0);
    let _vs_id = scene.add_volt_source(270.0, 0.0);
    let _cs_id = scene.add_curr_source(280.0, 0.0);
    let _csrc_id = scene.add_csource(290.0, 0.0);
    let _push_id = scene.add_push(300.0, 0.0);
    let _dip_id = scene.add_switch_dip(310.0, 0.0);
    let _relay_id = scene.add_relay(320.0, 0.0);

    for it in scene.items() {
        assert!(
            !it.description().is_empty(),
            "Item {} description is empty",
            it.id
        );
        // Ground and Node have no editable properties, all others should have prop_rows
        if it.id == gnd_id || it.id == node_id {
            assert!(it.prop_rows().is_empty());
        } else {
            assert!(
                !it.prop_rows().is_empty(),
                "Item {} has no prop rows",
                it.id
            );
            for row in it.prop_rows() {
                assert!(!row.name.is_empty());
                assert!(!row.caption.is_empty());
                if row.kind == "bool" {
                    assert!(
                        it.prop_bool(row.name).is_some(),
                        "Item {} missing bool prop {}",
                        it.id,
                        row.name
                    );
                } else {
                    assert!(
                        it.prop_text(row.name).is_some(),
                        "Item {} missing text prop {}",
                        it.id,
                        row.name
                    );
                }
            }
        }
    }
}

#[test]
fn test_property_editing_getters_and_setters() {
    let mut item = Item::resistor("R1", 0.0, 0.0, 1000.0);
    assert_eq!(item.prop_text("Resistance").as_deref(), Some("1 kΩ"));
    assert!(item.set_prop_text("Resistance", "4.7 kΩ"));
    assert_eq!(item.prop_text("Resistance").as_deref(), Some("4.7 kΩ"));

    // Diode
    let mut diode = Item::diode("D1", 0.0, 0.0, false);
    assert!(diode.set_prop_text("Threshold", "0.85 V"));
    assert!(diode.set_prop_text("MaxCurrent", "2.5 A"));
    assert!(diode.set_prop_text("Resistance", "0.25 Ω"));
    assert!(diode.set_prop_text("BrkDownV", "75 V"));
    assert_eq!(diode.prop_text("Threshold").as_deref(), Some("850 mV"));
    assert_eq!(diode.prop_text("MaxCurrent").as_deref(), Some("2.5 A"));
    assert_eq!(diode.prop_text("Resistance").as_deref(), Some("250 mΩ"));
    assert_eq!(diode.prop_text("BrkDownV").as_deref(), Some("75 V"));

    // BJT
    let mut bjt = Item::bjt("Q1", 0.0, 0.0, false);
    assert_eq!(diode.prop_bool("PNP"), None);
    assert_eq!(bjt.prop_bool("PNP"), Some(false));
    assert!(bjt.set_prop_bool("PNP", true));
    assert_eq!(bjt.prop_bool("PNP"), Some(true));
    assert!(bjt.set_prop_text("Gain", "250"));
    assert!(bjt.set_prop_text("Vcrit", "0.65 V"));
    assert_eq!(bjt.prop_text("Gain").as_deref(), Some("250"));
    assert_eq!(bjt.prop_text("Vcrit").as_deref(), Some("650 mV"));

    // MOSFET
    let mut mos = Item::mosfet("M1", 0.0, 0.0, false, false);
    assert_eq!(mos.prop_bool("PChannel"), Some(false));
    assert_eq!(mos.prop_bool("Depletion"), Some(false));
    assert!(mos.set_prop_bool("PChannel", true));
    assert!(mos.set_prop_bool("Depletion", true));
    assert_eq!(mos.prop_bool("PChannel"), Some(true));
    assert_eq!(mos.prop_bool("Depletion"), Some(true));
    assert!(mos.set_prop_text("RDSon", "0.05 Ω"));
    assert!(mos.set_prop_text("Threshold", "2.5 V"));
    assert_eq!(mos.prop_text("RDSon").as_deref(), Some("50 mΩ"));
    assert_eq!(mos.prop_text("Threshold").as_deref(), Some("2.5 V"));

    // FixedVolt & Probe
    let mut fv = Item::fixed_volt("V1", 0.0, 0.0, 5.0);
    assert_eq!(fv.prop_bool("Small"), Some(false));
    assert!(fv.set_prop_bool("Small", true));
    assert_eq!(fv.prop_bool("Small"), Some(true));

    let mut probe = Item::probe("P1", 0.0, 0.0, 2.5, false);
    assert_eq!(probe.prop_bool("ShowVolt"), Some(false));
    assert!(probe.set_prop_bool("ShowVolt", true));
    assert_eq!(probe.prop_bool("ShowVolt"), Some(true));

    // Relay
    let mut relay = Item::relay("RL1", 0.0, 0.0, false, false, 2, 0.03, 0.015, false);
    assert_eq!(relay.prop_bool("NormClose"), Some(false));
    assert_eq!(relay.prop_bool("DoubleThrow"), Some(false));
    assert_eq!(relay.prop_text("Poles").as_deref(), Some("2"));
    assert!(relay.set_prop_bool("DoubleThrow", true));
    assert!(relay.set_prop_text("Poles", "4"));
    assert!(relay.set_prop_text("IOn", "45 mA"));
    assert_eq!(relay.prop_bool("DoubleThrow"), Some(true));
    assert_eq!(relay.prop_text("Poles").as_deref(), Some("4"));
    assert_eq!(relay.prop_text("IOn").as_deref(), Some("45 mA"));

    // SwitchDip
    let mut dip = Item::switch_dip("DIP1", 0.0, 0.0, 8, 0, false);
    assert_eq!(dip.prop_text("Size").as_deref(), Some("8"));
    assert_eq!(dip.prop_bool("CommonPin"), Some(false));
    assert!(dip.set_prop_text("Size", "12"));
    assert!(dip.set_prop_bool("CommonPin", true));
    assert_eq!(dip.prop_text("Size").as_deref(), Some("12"));
    assert_eq!(dip.prop_bool("CommonPin"), Some(true));

    // WaveGen
    let mut wg = Item::wave_gen("WG1", 0.0, 0.0, "Sine", 1000.0, 5.0, 0.0, 0.5);
    assert_eq!(wg.prop_text("WaveType").as_deref(), Some("Sine"));
    assert!(wg.set_prop_text("WaveType", "Square"));
    assert!(wg.set_prop_text("Frequency", "2.5 kHz"));
    assert!(wg.set_prop_text("Amplitude", "12 V"));
    assert!(wg.set_prop_text("Duty", "75%"));
    assert_eq!(wg.prop_text("WaveType").as_deref(), Some("Square"));
    assert_eq!(wg.prop_text("Frequency").as_deref(), Some("2.5 kHz"));
    assert_eq!(wg.prop_text("Amplitude").as_deref(), Some("12 V"));
    assert_eq!(wg.prop_text("Duty").as_deref(), Some("75 %"));

    // Direct metric subunit inputs
    assert!(item.set_prop_text("Resistance", "4k7"));
    assert_eq!(item.prop_text("Resistance").as_deref(), Some("4.7 kΩ"));
    assert!(item.set_prop_text("Resistance", "100k"));
    assert_eq!(item.prop_text("Resistance").as_deref(), Some("100 kΩ"));
    assert!(item.set_prop_text("Resistance", "1.2M"));
    assert_eq!(item.prop_text("Resistance").as_deref(), Some("1.2 MΩ"));

    let mut cap = Item::capacitor("C1", 0.0, 0.0, 1e-6);
    assert!(cap.set_prop_text("Capacitance", "4.7u"));
    assert_eq!(cap.prop_text("Capacitance").as_deref(), Some("4.7 µF"));
    assert!(cap.set_prop_text("Capacitance", "2u2"));
    assert_eq!(cap.prop_text("Capacitance").as_deref(), Some("2.2 µF"));
    assert!(cap.set_prop_text("Capacitance", "100n"));
    assert_eq!(cap.prop_text("Capacitance").as_deref(), Some("100 nF"));
}

#[test]
fn test_sim1_serialization_and_deserialization_roundtrip() {
    let mut scene = Scene::new();
    let bjt_id = scene.add_bjt(0.0, 0.0, true);
    scene
        .item_by_id_mut(&bjt_id)
        .unwrap()
        .set_prop_text("Gain", "320");
    scene
        .item_by_id_mut(&bjt_id)
        .unwrap()
        .set_prop_text("Vcrit", "0.68 V");

    let mos_id = scene.add_mosfet(10.0, 0.0, true, true);
    scene
        .item_by_id_mut(&mos_id)
        .unwrap()
        .set_prop_text("RDSon", "0.08 Ω");
    scene
        .item_by_id_mut(&mos_id)
        .unwrap()
        .set_prop_text("Threshold", "1.8 V");

    let diode_id = scene.add_diode(20.0, 0.0, false);
    scene
        .item_by_id_mut(&diode_id)
        .unwrap()
        .set_prop_text("Threshold", "0.85 V");
    scene
        .item_by_id_mut(&diode_id)
        .unwrap()
        .set_prop_text("MaxCurrent", "3.0 A");
    scene
        .item_by_id_mut(&diode_id)
        .unwrap()
        .set_prop_text("Resistance", "0.2 Ω");

    let xml = scene.to_sim1();
    assert!(xml.contains(r#"itemtype="Bjt""#));
    assert!(xml.contains(r#"Gain="320""#));
    assert!(xml.contains(r#"itemtype="Mosfet""#));
    assert!(xml.contains(r#"PChannel="true""#));
    assert!(xml.contains(r#"Depletion="true""#));
    assert!(xml.contains(r#"RDSon="80 mΩ""#));
    assert!(xml.contains(r#"itemtype="Diode""#));
    assert!(xml.contains(r#"Threshold="850 mV""#));
    assert!(xml.contains(r#"MaxCurrent="3 A""#));

    // Reload from XML
    let scene2 = Scene::from_sim1(&xml).expect("XML reload failed");

    let bjt2 = scene2
        .item_by_id(&bjt_id)
        .expect("BJT not found in reloaded scene");
    assert_eq!(bjt2.prop_bool("PNP"), Some(true));
    assert_eq!(bjt2.prop_text("Gain").as_deref(), Some("320"));

    let mos2 = scene2
        .item_by_id(&mos_id)
        .expect("Mosfet not found in reloaded scene");
    assert_eq!(mos2.prop_bool("PChannel"), Some(true));
    assert_eq!(mos2.prop_bool("Depletion"), Some(true));
    assert_eq!(mos2.prop_text("RDSon").as_deref(), Some("80 mΩ"));

    let d2 = scene2
        .item_by_id(&diode_id)
        .expect("Diode not found in reloaded scene");
    assert_eq!(d2.prop_text("Threshold").as_deref(), Some("850 mV"));
    assert_eq!(d2.prop_text("MaxCurrent").as_deref(), Some("3 A"));
    assert_eq!(d2.prop_text("Resistance").as_deref(), Some("200 mΩ"));
}

#[test]
fn test_circuit_simulation_model_creation() {
    let mut scene = Scene::new();
    let _r_id = scene.add_resistor(0.0, 0.0, 1000.0);
    let d_id = scene.add_diode(10.0, 0.0, false);
    scene
        .item_by_id_mut(&d_id)
        .unwrap()
        .set_prop_text("Threshold", "0.75 V");
    scene
        .item_by_id_mut(&d_id)
        .unwrap()
        .set_prop_text("MaxCurrent", "2.0 A");
    scene
        .item_by_id_mut(&d_id)
        .unwrap()
        .set_prop_text("Resistance", "0.15 Ω");

    let circuit = scene.to_circuit();
    assert_eq!(circuit.components().len(), 2);

    let comp_d = circuit
        .components()
        .iter()
        .find(|c| c.id == d_id)
        .expect("Diode comp found");
    if let cs_engine::elements::Kind::Diode { state, zener } = &comp_d.kind {
        assert!(!zener);
        assert!((state.threshold - 0.75).abs() < 1e-4);
        assert!((state.max_current - 2.0).abs() < 1e-4);
        assert!((state.series_r - 0.15).abs() < 1e-4);
    } else {
        panic!("Expected Diode element");
    }
}

#[test]
fn led_grounded_hides_cathode_and_stamps_to_gnd() {
    let mut scene = Scene::new();
    let led_id = scene.add_led(0.0, 0.0);
    {
        let led = scene.item_by_id_mut(&led_id).unwrap();
        assert_eq!(led.prop_bool("Grounded"), Some(false));
        assert_eq!(led.pins().len(), 2);
        assert!(led.set_prop_bool("Grounded", true));
        assert_eq!(led.prop_bool("Grounded"), Some(true));
        let pins = led.pins();
        assert_eq!(pins.len(), 1);
        assert!(pins[0].id.ends_with("-lPin"));
    }

    let xml = scene.to_sim1();
    assert!(xml.contains(r#"Grounded="true""#));
    let loaded = Scene::from_sim1(&xml).expect("reload grounded LED");
    let led2 = loaded.item_by_id(&led_id).expect("LED present");
    assert_eq!(led2.prop_bool("Grounded"), Some(true));
    assert_eq!(led2.pins().len(), 1);

    let mut scene = Scene::new();
    let fv = scene.add_default_fixed_volt(0.0, 0.0);
    let r = scene.add_resistor(80.0, 0.0, 330.0);
    let led_id = scene.add_led(160.0, 0.0);
    scene
        .item_by_id_mut(&led_id)
        .unwrap()
        .set_prop_bool("Grounded", true);
    let origin = cs_engine::canvas::Point::new(0.0, 0.0);
    let fv_pin = format!("{fv}-outnod");
    let r_l = format!("{r}-lPin");
    let r_r = format!("{r}-rPin");
    let led_a = format!("{led_id}-lPin");
    scene.connect_pins(&fv_pin, origin, &r_l, origin);
    scene.connect_pins(&r_r, origin, &led_a, origin);
    let mut c = scene.to_circuit();
    c.solve().unwrap();
    let vf = c.pin_voltage(&led_a).unwrap();
    assert!((vf - 2.4).abs() < 0.05, "grounded LED Vf {vf}");
}

#[test]
fn led_bar_colors_properties_and_simulation() {
    use cs_engine::canvas::Canvas;
    use cs_engine::theme::ColorTheme;

    let mut canvas = Canvas::new();
    let lb_id = canvas.scene_mut().add_led_bar(0.0, 0.0);
    {
        let lb = canvas.scene_mut().item_by_id_mut(&lb_id).unwrap();
        assert_eq!(lb.prop_text("Color").as_deref(), Some("Red"));
        assert_eq!(lb.prop_text("Segments").as_deref(), Some("10"));
        assert_eq!(lb.prop_bool("Grounded"), Some(false));
        assert_eq!(lb.pins().len(), 20); // 10 anodes + 10 cathodes

        // Change color to Yellow
        assert!(lb.set_prop_text("Color", "Yellow"));
        assert_eq!(lb.prop_text("Color").as_deref(), Some("Yellow"));

        // Verify ColorTheme gives parity colors for both lit and unlit
        let (lit, unlit) = ColorTheme::led_color_hex(lb.color_str());
        assert_eq!(lit, "#ffee22");
        assert_eq!(unlit, "#444411");

        // Change color to Green
        assert!(lb.set_prop_text("Color", "Green"));
        let (lit, unlit) = ColorTheme::led_color_hex(lb.color_str());
        assert_eq!(lit, "#22dd44");
        assert_eq!(unlit, "#114411");

        // Change color to Blue
        assert!(lb.set_prop_text("Color", "Blue"));
        let (lit, unlit) = ColorTheme::led_color_hex(lb.color_str());
        assert_eq!(lit, "#2288ff");
        assert_eq!(unlit, "#112244");
    }

    // Connect segment 0 to 5V and GND, segment 1 unconnected
    let fv = canvas.scene_mut().add_default_fixed_volt(0.0, 0.0);
    let gnd = canvas.scene_mut().add_ground(0.0, 0.0);
    let origin = cs_engine::canvas::Point::new(0.0, 0.0);
    canvas.scene_mut().connect_pins(
        &format!("{fv}-outnod"),
        origin,
        &format!("{lb_id}-lPin0"),
        origin,
    );
    canvas.scene_mut().connect_pins(
        &format!("{gnd}-Gnd"),
        origin,
        &format!("{lb_id}-rPin0"),
        origin,
    );

    canvas.power_on();
    for _ in 0..5 {
        canvas.tick();
    }

    let readings = canvas.readings();
    let rd = readings.get(&lb_id).expect("LedBar reading should exist");
    // Segment 0 should be lit ('1'), remaining segments unlit ('0')
    assert_eq!(rd.text.chars().next(), Some('1'));
    assert_eq!(rd.text.chars().nth(1), Some('0'));
    assert_eq!(rd.text, "1000000000");

    // Test Grounded mode
    canvas.power_off();
    {
        let lb = canvas.scene_mut().item_by_id_mut(&lb_id).unwrap();
        assert!(lb.set_prop_bool("Grounded", true));
        assert_eq!(lb.pins().len(), 10); // only anode pins
    }
    canvas.power_on();
    for _ in 0..5 {
        canvas.tick();
    }
    let rd_gnd = canvas.readings().get(&lb_id).expect("LedBar reading");
    assert_eq!(rd_gnd.text, "1000000000");
}

#[test]
fn test_led_color_dropdown_updates_threshold() {
    let mut item = Item::led("LED1", 0.0, 0.0);
    // Default is Yellow -> 2.4 V
    assert_eq!(item.prop_text("Color").as_deref(), Some("Yellow"));
    assert_eq!(item.prop_text("Threshold").as_deref(), Some("2.4 V"));

    // Change to Red -> 1.8 V
    assert!(item.set_prop_text("Color", "Red"));
    assert_eq!(item.prop_text("Threshold").as_deref(), Some("1.8 V"));

    // Change to Blue -> 3.6 V
    assert!(item.set_prop_text("Color", "Blue"));
    assert_eq!(item.prop_text("Threshold").as_deref(), Some("3.6 V"));

    // Change to White -> 4.0 V
    assert!(item.set_prop_text("Color", "White"));
    assert_eq!(item.prop_text("Threshold").as_deref(), Some("4 V"));
}

#[test]
fn test_wavegen_conditional_properties_and_tabs() {
    let mut wg = Item::wave_gen("WG1", 0.0, 0.0, "Sine", 1000.0, 5.0, 50.0, 0.0);
    let groups = wg.prop_groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "Main");
    assert_eq!(groups[1].name, "Electric");

    // By default Wave_Type is Sine: Duty and File not visible
    let duty_row = groups[0].rows.iter().find(|r| r.name == "Duty").unwrap();
    let file_row = groups[0].rows.iter().find(|r| r.name == "File").unwrap();
    assert!(!duty_row.visible);
    assert!(!file_row.visible);

    // Switch to Triangle: Duty is visible
    assert!(wg.set_prop_text("Wave_Type", "Triangle"));
    let groups_tri = wg.prop_groups();
    let duty_tri = groups_tri[0]
        .rows
        .iter()
        .find(|r| r.name == "Duty")
        .unwrap();
    assert!(duty_tri.visible);

    // Switch to Wav: File is visible
    assert!(wg.set_prop_text("Wave_Type", "Wav"));
    let groups_wav = wg.prop_groups();
    let file_wav = groups_wav[0]
        .rows
        .iter()
        .find(|r| r.name == "File")
        .unwrap();
    assert!(file_wav.visible);
}

#[test]
fn test_multitab_components_parity() {
    // OpAmp has Main and Supply
    let opamp = Item::opamp("OA1", 0.0, 0.0);
    let opamp_groups = opamp.prop_groups();
    assert_eq!(opamp_groups.len(), 2);
    assert_eq!(opamp_groups[0].name, "Main");
    assert_eq!(opamp_groups[1].name, "Supply");

    // Relay has Main, Electric, Coil
    let relay = Item::relay("RL1", 0.0, 0.0, false, false, 1, 0.02, 0.01, false);
    let relay_groups = relay.prop_groups();
    assert_eq!(relay_groups.len(), 3);
    assert_eq!(relay_groups[0].name, "Main");
    assert_eq!(relay_groups[1].name, "Electric");
    assert_eq!(relay_groups[2].name, "Coil");

    // Diode has Main, Electric, Advanced
    let diode = Item::diode("D1", 0.0, 0.0, false);
    let diode_groups = diode.prop_groups();
    assert_eq!(diode_groups.len(), 3);
    assert_eq!(diode_groups[0].name, "Main");
    assert_eq!(diode_groups[1].name, "Electric");
    assert_eq!(diode_groups[2].name, "Advanced");

    // MCU has Main and Config
    let mcu_item = Item::new(
        "MCU-1",
        0.0,
        0.0,
        cs_engine::canvas::Part::Mcu(cs_engine::components::Mcu::default()),
    );
    let mcu_groups = mcu_item.prop_groups();
    assert_eq!(mcu_groups.len(), 2);
    assert_eq!(mcu_groups[0].name, "Main");
    assert_eq!(mcu_groups[1].name, "Config");
}

#[test]
fn test_mcu_properties_tabs_and_main_comp_disabled_parity() {
    let mut scene = Scene::new();
    let search = cs_engine::subcircuit::SubcSearch::default().with_standard_catalog();
    let mcu_id = scene
        .add_mcu("mega328", 0.0, 0.0, &search)
        .expect("add mega328");
    let item = scene.item_by_id(&mcu_id).unwrap();

    let groups = item.prop_groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "Main");
    assert_eq!(groups[1].name, "Config");

    // Check Main tab properties
    let main_props: Vec<&str> = groups[0].rows.iter().map(|r| r.name).collect();
    assert!(main_props.contains(&"Package"));
    assert!(main_props.contains(&"LogicSymbol"));
    assert!(main_props.contains(&"Frequency"));
    assert!(main_props.contains(&"ForceFreq"));
    assert!(main_props.contains(&"Program"));
    assert!(main_props.contains(&"AutoLoad"));
    assert!(main_props.contains(&"SavePgm"));

    // Check Config tab properties
    let config_props: Vec<&str> = groups[1].rows.iter().map(|r| r.name).collect();
    assert_eq!(
        config_props,
        vec!["RstEnabled", "ExtOsc", "WdtEnabled", "ClkOut"]
    );

    // When standalone, all rows are enabled
    for g in &groups {
        for r in &g.rows {
            assert!(r.enabled, "Property {} should be enabled", r.name);
        }
    }

    // When item is marked as mainComp (e.g. inside an Arduino Uno/Mega subcircuit)
    let item_mut = scene.item_by_id_mut(&mcu_id).unwrap();
    assert!(item_mut.set_prop_bool("MainComp", true));

    let board_groups = item_mut.prop_groups();
    assert_eq!(board_groups.len(), 2);

    // Package is disabled on a board MCU
    let pkg_row = board_groups[0]
        .rows
        .iter()
        .find(|r| r.name == "Package")
        .unwrap();
    assert!(!pkg_row.enabled, "Package must be disabled for board MCU");

    // Firmware / Frequency options remain enabled
    let freq_row = board_groups[0]
        .rows
        .iter()
        .find(|r| r.name == "Frequency")
        .unwrap();
    assert!(freq_row.enabled, "Frequency must remain enabled");
    let pgm_row = board_groups[0]
        .rows
        .iter()
        .find(|r| r.name == "Program")
        .unwrap();
    assert!(pgm_row.enabled, "Program must remain enabled");

    // Config tab properties are all disabled for board MCU
    for r in &board_groups[1].rows {
        assert!(
            !r.enabled,
            "Config property {} must be disabled for board MCU",
            r.name
        );
    }
}

#[test]
fn test_mcu_package_dropdown_switching_and_parity() {
    let mut canvas = cs_engine::canvas::Canvas::new();
    let search = cs_engine::subcircuit::SubcSearch::default().with_standard_catalog();
    let mcu_id = canvas
        .scene_mut()
        .add_mcu("mega328", 0.0, 0.0, &search)
        .expect("add mega328");

    let item = canvas.scene().item_by_id(&mcu_id).unwrap();
    let groups = item.prop_groups();
    let pkg_row = groups[0]
        .rows
        .iter()
        .find(|r| r.name == "Package")
        .expect("Package property must exist");

    assert_eq!(pkg_row.kind, "enum");
    assert_eq!(
        pkg_row.options,
        vec!["1- mega328_DIP".to_string(), "2- mega328_LS".to_string()]
    );
    assert_eq!(item.prop_text("Package").as_deref(), Some("1- mega328_DIP"));
    assert_eq!(item.package().unwrap().name, "mega328");
    assert!(!item.logic_symbol());

    // Switch to LS package via Canvas property edit
    canvas.set_prop_text_for(&mcu_id, "Package".to_string(), "2- mega328_LS".to_string());
    let item_ls = canvas.scene().item_by_id(&mcu_id).unwrap();
    assert_eq!(
        item_ls.prop_text("Package").as_deref(),
        Some("2- mega328_LS")
    );
    assert!(item_ls.logic_symbol());
    assert!(item_ls.package().unwrap().logic_symbol);

    // Switch back to DIP package
    canvas.set_prop_text_for(&mcu_id, "Package".to_string(), "1- mega328_DIP".to_string());
    let item_dip = canvas.scene().item_by_id(&mcu_id).unwrap();
    assert_eq!(
        item_dip.prop_text("Package").as_deref(),
        Some("1- mega328_DIP")
    );
    assert!(!item_dip.logic_symbol());
    assert!(!item_dip.package().unwrap().logic_symbol);

    // Setting LogicSymbol to true syncs Package to LS
    canvas.set_prop_bool_for(&mcu_id, "LogicSymbol".to_string(), true);
    let item_ls2 = canvas.scene().item_by_id(&mcu_id).unwrap();
    assert_eq!(
        item_ls2.prop_text("Package").as_deref(),
        Some("2- mega328_LS")
    );
    assert!(item_ls2.logic_symbol());
}
