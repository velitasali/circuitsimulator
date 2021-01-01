//! Exhaustive property coverage and multi-pole output verification for the Switch component.

use cs_engine::canvas::Canvas;
use cs_engine::canvas::scene::Scene;
use cs_engine::components::{Component, PropValue, Switch};
use cs_engine::elements::{Comp, Kind};
use cs_engine::simulation::Circuit;

#[test]
fn test_switch_all_props_reflection_and_aliases() {
    let mut sw = Switch::default();

    // 1. Initial defaults
    assert!(sw.checked, "Default switch should be checked (closed)");
    assert!(!sw.norm_close, "Default switch NormClose should be false");
    assert!(
        !sw.double_throw,
        "Default switch DoubleThrow should be false"
    );
    assert_eq!(sw.poles, 1, "Default switch Poles should be 1");
    assert_eq!(sw.key, "", "Default switch Key should be empty");
    assert!(!sw.show_button, "Default switch ShowButton should be false");

    // 2. Props metadata inspection
    let props = Switch::props();
    let prop_names: Vec<&str> = props.iter().map(|p| p.id).collect();
    assert!(prop_names.contains(&"NormClose"));
    assert!(prop_names.contains(&"DoubleThrow"));
    assert!(prop_names.contains(&"Poles"));
    assert!(prop_names.contains(&"Key"));
    assert!(prop_names.contains(&"ShowButton"));
    assert!(prop_names.contains(&"Checked"));

    // 3. Getters and Setters via Component trait
    assert_eq!(sw.get_prop_text("NormClose").as_deref(), Some("false"));
    assert_eq!(sw.get_prop_text("DoubleThrow").as_deref(), Some("false"));
    assert_eq!(sw.get_prop_text("Poles").as_deref(), Some("1"));
    assert_eq!(sw.get_prop_text("Key").as_deref(), Some(""));
    assert_eq!(sw.get_prop_text("ShowButton").as_deref(), Some("false"));
    assert_eq!(sw.get_prop_text("Checked").as_deref(), Some("true"));

    // Test aliases via get_prop_text_alias
    assert_eq!(
        sw.get_prop_text_alias("Norm_Close").as_deref(),
        Some("false")
    );
    assert_eq!(
        sw.get_prop_text_alias("Double_Throw").as_deref(),
        Some("false")
    );
    assert_eq!(
        sw.get_prop_text_alias("Show_Button").as_deref(),
        Some("false")
    );
    assert_eq!(
        sw.get_prop_text_alias("show_button").as_deref(),
        Some("false")
    );

    // 4. Modifying properties via set_prop_text and set_prop
    assert!(sw.set_prop_text("Poles", "3").is_ok());
    assert_eq!(sw.poles, 3);
    assert_eq!(sw.get_prop_text("Poles").as_deref(), Some("3"));

    // Poles clamping: min 1, max 4
    assert!(sw.set_prop_text("Poles", "0").is_ok());
    assert_eq!(sw.poles, 1);
    assert!(sw.set_prop_text("Poles", "10").is_ok());
    assert_eq!(sw.poles, 4);

    assert!(sw.set_prop("DoubleThrow", PropValue::Bool(true)).is_ok());
    assert!(sw.double_throw);

    assert!(sw.set_prop("NormClose", PropValue::Bool(true)).is_ok());
    assert!(sw.norm_close);

    assert!(sw.set_prop("Checked", PropValue::Bool(false)).is_ok());
    assert!(!sw.checked);

    assert!(sw.set_prop("ShowButton", PropValue::Bool(true)).is_ok());
    assert!(sw.show_button);

    assert!(sw.set_prop_text("Key", "K").is_ok());
    assert_eq!(sw.key, "K");
}

#[test]
fn test_switch_multipole_spst_simulation_all_outputs() {
    // Test 1, 2, 3, 4 poles in Single Throw (SPST / DPST / 3PST / 4PST)
    for num_poles in 1..=4 {
        let mut circ = Circuit::new();
        circ.add_ground("GND");

        // Add multi-pole switch
        circ.add_switch_with("SW1", false, num_poles, false);

        // Connect independent voltage source and load resistor for each pole
        for i in 0..num_poles {
            let v_src = format!("V_{i}");
            let r_load = format!("R_{i}");
            let v_val = (i as f64 + 1.0) * 3.0; // 3V, 6V, 9V, 12V
            let r_val = 1000.0;

            circ.add_fixed_volt(&v_src, v_val);
            circ.add_resistor(&r_load, r_val);

            // Input to pole i common terminal (-pinP{i})
            circ.connect(format!("{v_src}-outnod"), format!("SW1-pinP{i}"));
            // Output from pole i contact (-switch{i}pinN)
            circ.connect(format!("SW1-switch{i}pinN"), format!("{r_load}-lPin"));
            circ.connect(format!("{r_load}-rPin"), "GND-Gnd");
        }

        // --- Case A: Switch is OPEN (closed = false, norm_close = false) ---
        circ.solve().unwrap();
        for i in 0..num_poles {
            let r_load_pin = format!("R_{i}-lPin");
            let v_out = circ.pin_voltage(&r_load_pin).expect("Voltage on R_i");
            assert!(
                v_out.abs() < 1e-4,
                "Pole {i}/{num_poles} output must be 0V when open, got {v_out}"
            );

            let sw_out_pin = format!("SW1-switch{i}pinN");
            let i_out = circ.current_out_of_pin(&sw_out_pin).unwrap_or(0.0);
            assert!(
                i_out.abs() < 1e-4,
                "Pole {i}/{num_poles} current must be 0A when open, got {i_out}"
            );
        }

        // --- Case B: Switch is CLOSED (closed = true) ---
        assert!(circ.set_switch("SW1", true));
        circ.solve().unwrap();
        for i in 0..num_poles {
            let expected_v = (i as f64 + 1.0) * 3.0;
            let expected_i = expected_v / 1000.0;

            let r_load_pin = format!("R_{i}-lPin");
            let v_out = circ.pin_voltage(&r_load_pin).expect("Voltage on R_i");
            assert!(
                (v_out - expected_v).abs() < 0.05,
                "Pole {i}/{num_poles} output must deliver ~{expected_v}V when closed, got {v_out}"
            );

            let sw_out_pin = format!("SW1-switch{i}pinN");
            let i_out = circ
                .current_out_of_pin(&sw_out_pin)
                .expect("Current on switch out pin");
            assert!(
                (i_out.abs() - expected_i).abs() < 1e-3,
                "Pole {i}/{num_poles} current must be ~{expected_i}A when closed, got {i_out}"
            );
        }

        // --- Case C: Switch reopened ---
        assert!(circ.set_switch("SW1", false));
        circ.solve().unwrap();
        for i in 0..num_poles {
            let r_load_pin = format!("R_{i}-lPin");
            let v_out = circ.pin_voltage(&r_load_pin).expect("Voltage on R_i");
            assert!(
                v_out.abs() < 1e-4,
                "Pole {i}/{num_poles} output must return to 0V when reopened, got {v_out}"
            );
        }
    }
}

#[test]
fn test_switch_multipole_dpdt_simulation_all_throws() {
    // Test 1, 2, 3, 4 poles in Double Throw (SPDT / DPDT / 3PDT / 4PDT)
    for num_poles in 1..=4 {
        let mut circ = Circuit::new();
        circ.add_ground("GND");

        // Add multi-pole double-throw switch
        circ.add_switch_with("SW1", false, num_poles, true);

        // For each pole i:
        // Input: V_i = (i+1) * 2.5V connected to SW1-pinP{i}
        // Throw 0: connected to R0_{i} (500 ohms)
        // Throw 1: connected to R1_{i} (1000 ohms)
        for i in 0..num_poles {
            let v_src = format!("V_{i}");
            let r0 = format!("R0_{i}");
            let r1 = format!("R1_{i}");
            let v_val = (i as f64 + 1.0) * 2.5; // 2.5V, 5.0V, 7.5V, 10.0V

            circ.add_fixed_volt(&v_src, v_val);
            circ.add_resistor(&r0, 500.0);
            circ.add_resistor(&r1, 1000.0);

            // Pole input
            circ.connect(format!("{v_src}-outnod"), format!("SW1-pinP{i}"));
            // Throw 0: switch{2*i}pinN
            circ.connect(format!("SW1-switch{}pinN", 2 * i), format!("{r0}-lPin"));
            circ.connect(format!("{r0}-rPin"), "GND-Gnd");
            // Throw 1: switch{2*i+1}pinN
            circ.connect(format!("SW1-switch{}pinN", 2 * i + 1), format!("{r1}-lPin"));
            circ.connect(format!("{r1}-rPin"), "GND-Gnd");
        }

        // --- State A: closed = false (Resting position: Throw 1 conducts, Throw 0 disconnected) ---
        circ.solve().unwrap();
        for i in 0..num_poles {
            let v_val = (i as f64 + 1.0) * 2.5;

            let v_t0 = circ.pin_voltage(&format!("R0_{i}-lPin")).unwrap();
            let v_t1 = circ.pin_voltage(&format!("R1_{i}-lPin")).unwrap();

            assert!(
                v_t0.abs() < 1e-4,
                "Pole {i}/{num_poles} Throw 0 must be 0V when switch is open (false), got {v_t0}"
            );
            assert!(
                (v_t1 - v_val).abs() < 0.05,
                "Pole {i}/{num_poles} Throw 1 must be ~{v_val}V when switch is open (false), got {v_t1}"
            );

            // Current checks
            let i_t0 = circ
                .current_out_of_pin(&format!("SW1-switch{}pinN", 2 * i))
                .unwrap_or(0.0);
            let i_t1 = circ
                .current_out_of_pin(&format!("SW1-switch{}pinN", 2 * i + 1))
                .expect("Current T1");
            assert!(i_t0.abs() < 1e-4, "Throw 0 current should be 0");
            let expected_i1 = v_val / 1000.0;
            assert!(
                (i_t1.abs() - expected_i1).abs() < 1e-3,
                "Throw 1 current must be ~{expected_i1}A, got {i_t1}"
            );
        }

        // --- State B: closed = true (Active position: Throw 0 conducts, Throw 1 disconnected) ---
        assert!(circ.set_switch("SW1", true));
        circ.solve().unwrap();
        for i in 0..num_poles {
            let v_val = (i as f64 + 1.0) * 2.5;

            let v_t0 = circ.pin_voltage(&format!("R0_{i}-lPin")).unwrap();
            let v_t1 = circ.pin_voltage(&format!("R1_{i}-lPin")).unwrap();

            assert!(
                (v_t0 - v_val).abs() < 0.05,
                "Pole {i}/{num_poles} Throw 0 must be ~{v_val}V when switch is closed (true), got {v_t0}"
            );
            assert!(
                v_t1.abs() < 1e-4,
                "Pole {i}/{num_poles} Throw 1 must be 0V when switch is closed (true), got {v_t1}"
            );

            // Current checks
            let i_t0 = circ
                .current_out_of_pin(&format!("SW1-switch{}pinN", 2 * i))
                .expect("Current T0");
            let i_t1 = circ
                .current_out_of_pin(&format!("SW1-switch{}pinN", 2 * i + 1))
                .unwrap_or(0.0);
            let expected_i0 = v_val / 500.0;
            assert!(
                (i_t0.abs() - expected_i0).abs() < 1e-3,
                "Throw 0 current must be ~{expected_i0}A, got {i_t0}"
            );
            assert!(i_t1.abs() < 1e-4, "Throw 1 current should be 0");
        }
    }
}

#[test]
fn test_switch_normally_closed_simulation_and_truth_table() {
    // Test Normally Closed (NormClose = true) across SPST and SPDT
    // 1. SPST NormClose = true
    let mut sw_spst = Switch::default();
    sw_spst.norm_close = true;
    sw_spst.checked = false;
    assert!(
        sw_spst.electrically_closed(),
        "NC switch unchecked must be electrically closed"
    );
    sw_spst.checked = true;
    assert!(
        !sw_spst.electrically_closed(),
        "NC switch checked must be electrically open"
    );

    // 2. SPDT NormClose = true simulation
    let mut circ = Circuit::new();
    circ.add_ground("GND");
    circ.add_fixed_volt("VIN", 5.0);
    circ.add_resistor("R0", 100.0);
    circ.add_resistor("R1", 100.0);

    let sw_comp = Comp {
        id: "SW1".to_string(),
        kind: Kind::Switch {
            closed: sw_spst.electrically_closed(),
            poles: 1,
            double_throw: true,
        },
    };
    circ.add_comp(sw_comp);

    circ.connect("VIN-outnod", "SW1-pinP0")
        .connect("SW1-switch0pinN", "R0-lPin")
        .connect("R0-rPin", "GND-Gnd")
        .connect("SW1-switch1pinN", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");

    // When sw_spst.checked = true (actuated), electrically_closed is false -> Throw 1 conducts
    circ.solve().unwrap();
    let v_r0 = circ.pin_voltage("R0-lPin").unwrap();
    let v_r1 = circ.pin_voltage("R1-lPin").unwrap();
    assert!(
        v_r0.abs() < 1e-4,
        "Throw 0 must be open when actuated on NC switch"
    );
    assert!(
        (v_r1 - 5.0).abs() < 0.05,
        "Throw 1 must conduct when actuated on NC switch"
    );
}

#[test]
fn test_switch_canvas_scene_mutations_and_sim1_roundtrip() {
    let mut scene = Scene::new();
    let sw_id = scene.add_switch(100.0, 150.0, true);

    // Initial item verification
    let item = scene.item_by_id(&sw_id).expect("Switch exists in scene");
    assert_eq!(item.prop_text("Poles").as_deref(), Some("1"));
    assert_eq!(item.prop_bool("DoubleThrow"), Some(false));
    assert_eq!(item.prop_bool("NormClose"), Some(false));
    assert_eq!(item.prop_bool("ShowButton"), Some(false));
    assert_eq!(item.prop_text("Key").as_deref(), Some(""));
    assert_eq!(item.prop_bool("Checked"), Some(true));

    // Mutate via scene item
    {
        let it = scene.item_by_id_mut(&sw_id).unwrap();
        it.set_prop_text("Poles", "3");
        it.set_prop_bool("DoubleThrow", true);
        it.set_prop_bool("NormClose", true);
        it.set_prop_bool("ShowButton", true);
        it.set_prop_text("Key", "S");
        it.set_prop_bool("Checked", false);
    }

    // Verify pin count and body rect after mutations
    let it = scene.item_by_id(&sw_id).unwrap();
    assert_eq!(it.poles(), 3);
    assert!(it.double_throw());
    assert!(it.norm_close());
    assert!(it.show_button());
    assert_eq!(it.key(), "S");
    assert_eq!(it.prop_bool("Checked"), Some(false));
    assert_eq!(it.pins().len(), 9, "3 poles * 3 pins per pole = 9 pins");

    // Export to SIM1 XML
    let sim1_xml = scene.to_sim1();
    assert!(sim1_xml.contains("Poles=\"3\""));
    assert!(sim1_xml.contains("DoubleThrow=\"true\"") || sim1_xml.contains("DT=\"true\""));
    assert!(sim1_xml.contains("NormClose=\"true\"") || sim1_xml.contains("Norm_Close=\"true\""));
    assert!(sim1_xml.contains("ShowButton=\"true\""));
    assert!(sim1_xml.contains("Key=\"S\""));
    assert!(sim1_xml.contains("Checked=\"false\""));

    // Reload from SIM1 XML
    let scene2 = Scene::from_sim1(&sim1_xml).expect("SIM1 deserialize success");
    let it2 = scene2.item_by_id(&sw_id).expect("Switch loaded from SIM1");
    assert_eq!(it2.poles(), 3);
    assert!(it2.double_throw());
    assert!(it2.norm_close());
    assert!(it2.show_button());
    assert_eq!(it2.key(), "S");
    assert_eq!(it2.prop_bool("Checked"), Some(false));
    assert_eq!(it2.pins().len(), 9);
}

#[test]
fn test_switch_canvas_toggle_and_circuit_lowering() {
    let mut canvas = Canvas::new();
    let sw_id = canvas.scene_mut().add_switch(0.0, 0.0, false);
    let fv_id = canvas.scene_mut().add_default_fixed_volt(0.0, 0.0);
    let r_id = canvas.scene_mut().add_resistor(0.0, 0.0, 1000.0);
    let gnd_id = canvas.scene_mut().add_ground(0.0, 0.0);

    let p = cs_engine::canvas::Point::new(0.0, 0.0);
    canvas
        .scene_mut()
        .connect_pins(&format!("{fv_id}-outnod"), p, &format!("{sw_id}-pinP0"), p);
    canvas.scene_mut().connect_pins(
        &format!("{sw_id}-switch0pinN"),
        p,
        &format!("{r_id}-lPin"),
        p,
    );
    canvas
        .scene_mut()
        .connect_pins(&format!("{r_id}-rPin"), p, &format!("{gnd_id}-Gnd"), p);

    // Initial state: switch is open (false)
    let mut circ = canvas.scene().to_circuit();
    circ.solve().unwrap();
    let v_open = circ.pin_voltage(&format!("{r_id}-lPin")).unwrap();
    assert!(v_open.abs() < 1e-4, "Initially open switch must read 0V");

    // Toggle switch via canvas toggle_item
    canvas.toggle_item(&sw_id);
    assert_eq!(
        canvas
            .scene()
            .item_by_id(&sw_id)
            .unwrap()
            .prop_bool("Checked"),
        Some(true)
    );

    // Circuit rebuilt from lowered scene
    let mut circ2 = canvas.scene().to_circuit();
    circ2.solve().unwrap();
    let v_closed = circ2.pin_voltage(&format!("{r_id}-lPin")).unwrap();
    assert!(
        (v_closed - 5.0).abs() < 0.05,
        "Closed switch must deliver 5V, got {v_closed}"
    );
}
