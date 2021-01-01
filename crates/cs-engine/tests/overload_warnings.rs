//! Integration tests for circuit component overload warning and crash detection.

use cs_engine::canvas::{Canvas, Point};

#[test]
fn test_el_capacitor_reverse_polarity_overload() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // Fixed Voltage (5V) -> C1 (elCapacitor) reverse polarity:
    // C1 positive pin is "lPin", negative pin is "rPin".
    // Connect Fixed Voltage to negative pin (rPin), and Ground to positive pin (lPin).
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="0,0" />
<item itemtype="ElCapacitor" CircId="elCapacitor-1" Capacitance="10 µF" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="200,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="elCapacitor-1-rPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="elCapacitor-1-lPin" pointList="200,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_cap.sim1".into()))
        .unwrap();
    c.power_on();
    assert!(c.sim_running(), "Sim must be running");
    assert!(c.sim_error().is_none(), "No sim error: {:?}", c.sim_error());

    // Run ticks
    for _ in 0..10 {
        c.tick();
    }

    println!("Overload states: {:?}", c.overload_states());
    println!("Overload log: {:?}", c.overload_log());

    // Check overload state
    let state = c.item_overload_state("elCapacitor-1");
    assert!(state.is_some(), "elCapacitor should have an overload state");
    let state = state.unwrap();
    assert!(
        state.crashed,
        "elCapacitor must be crashed due to reverse polarity"
    );
    assert!(
        state.reason.contains("Reverse polarity"),
        "Reason must mention reverse polarity: {}",
        state.reason
    );

    // Check overload log
    let log = c.overload_log();
    assert!(!log.is_empty(), "Overload log must contain an entry");
    assert_eq!(log[0].comp_uid, "elCapacitor-1");
    assert!(log[0].crashed);

    // Check hover tooltip
    let cap_pos = Point::new(100.0, -5.0);
    let tooltip = c.hover_tooltip(cap_pos);
    assert!(tooltip.is_some(), "Hover tooltip should exist");
    assert!(
        tooltip.unwrap().contains("Reverse polarity"),
        "Tooltip must display overload reason"
    );

    // Power off clears active overload state
    c.power_off();
    assert!(
        c.item_overload_state("elCapacitor-1").is_none(),
        "Power off must reset active overload state"
    );
    assert_eq!(
        c.overload_log().len(),
        1,
        "Power off should preserve log history"
    );

    // Clear log empties log
    c.clear_overload_log();
    assert!(
        c.overload_log().is_empty(),
        "clear_overload_log must empty the log"
    );
}

#[test]
fn test_led_overcurrent_overload() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // 10V directly across Led with max_current 20mA (0.02A) -> heavy overcurrent
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="10 V" Pos="0,0" />
<item itemtype="Led" CircId="Led-1" MaxCurrent="0.02" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="200,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Led-1-lPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="Led-1-rPin" pointList="200,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_led.sim1".into()))
        .unwrap();
    c.power_on();
    assert!(c.sim_running(), "Sim must be running");

    for _ in 0..10 {
        c.tick();
    }

    println!("LED overload states: {:?}", c.overload_states());
    println!("LED overload log: {:?}", c.overload_log());

    let state = c.item_overload_state("Led-1");
    assert!(state.is_some(), "LED should have an overload state");
    let state = state.unwrap();
    assert!(
        state.crashed || state.warning,
        "LED must be in warning or crashed state"
    );
    assert!(
        state.reason.contains("Overcurrent"),
        "Reason must mention Overcurrent: {}",
        state.reason
    );

    // Test select_and_center_item
    c.select_and_center_item("Led-1");
    let led_item = c
        .scene()
        .items()
        .iter()
        .find(|it| it.id == "Led-1")
        .unwrap();
    assert!(led_item.selected, "Item should be selected");
}

#[test]
fn test_voltmeter_overflow_overload() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // 1500V connected to Voltmeter (> 999V display limit)
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="1500 V" Pos="0,0" />
<item itemtype="Voltmeter" CircId="Voltimeter-1" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="200,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Voltimeter-1-lPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="Voltimeter-1-rPin" pointList="200,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_voltmeter.sim1".into()))
        .unwrap();
    c.power_on();

    for _ in 0..10 {
        c.tick();
    }

    let state = c.item_overload_state("Voltimeter-1");
    assert!(state.is_some(), "Voltmeter should have an overload state");
    let state = state.unwrap();
    assert!(
        state.crashed,
        "Voltmeter must be crashed due to display limit overflow"
    );
    assert!(
        state.reason.contains("meter overload"),
        "Reason must mention meter overload: {}",
        state.reason
    );
}

#[test]
fn test_lamp_overcurrent_overload() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // 50V across 12V 5W Lamp -> excessive overcurrent
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="50 V" Pos="0,0" />
<item itemtype="Lamp" CircId="Lamp-1" Voltage="12" Power="5" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="200,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Lamp-1-lPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="Lamp-1-rPin" pointList="200,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_lamp.sim1".into()))
        .unwrap();
    c.power_on();

    for _ in 0..10 {
        c.tick();
    }

    let state = c.item_overload_state("Lamp-1");
    assert!(state.is_some(), "Lamp should have an overload state");
    let state = state.unwrap();
    assert!(
        state.warning || state.crashed,
        "Lamp must be in warning or crashed state"
    );
    assert!(
        state.reason.contains("Overcurrent"),
        "Reason must mention Overcurrent: {}",
        state.reason
    );
}

#[test]
fn test_rgb_led_overcurrent_overload() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // 10V directly across RGB LED red pin (common cathode, cPin connected to Gnd, rPin to 10V)
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="10 V" Pos="0,0" />
<item itemtype="RgbLed" CircId="RgbLed-1" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="200,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="RgbLed-1-rPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="RgbLed-1-cPin" pointList="200,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_rgb_led.sim1".into()))
        .unwrap();
    c.power_on();

    for _ in 0..10 {
        c.tick();
    }

    let state = c.item_overload_state("RgbLed-1");
    assert!(state.is_some(), "RGB LED should have an overload state");
    let state = state.unwrap();
    assert!(
        state.warning || state.crashed,
        "RGB LED must be in warning or crashed state"
    );
    assert!(
        state.reason.contains("Overcurrent"),
        "Reason must mention Overcurrent: {}",
        state.reason
    );
}

#[test]
fn test_led_visual_glow_and_overload_rendering() {
    use cs_engine::canvas::{Palette, render_viewport};

    let mut c = Canvas::empty();
    c.set_view_size(200.0, 200.0);
    c.set_center(100.0, 0.0);

    // 1. Normal lit LED through a 220-ohm current limiting resistor
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="0,0" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="220" Pos="50,0" />
<item itemtype="Led" CircId="Led-1" Color="Yellow" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="150,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Resistor-1-lPin" pointList="0,0,50,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Resistor-1-rPin" endpinid="Led-1-lPin" pointList="50,0,100,0" />
<item itemtype="Connector" uid="conn-3" startpinid="Ground-1-Gnd" endpinid="Led-1-rPin" pointList="150,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_led_lit.sim1".into()))
        .unwrap();

    let pal = Palette::dark();
    // 1. Unlit render before power on
    let unpowered_pm = render_viewport(&c, &pal, 200, 200, 1.0, true).expect("unlit render failed");

    // Connect zero volts or disconnected LED: verify that unpowered LED with sim running does not glow
    let sim1_unpowered = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="Led" CircId="Led-1" Color="Yellow" Pos="100,0" />
</circuit>"#;
    let mut c_unpowered = Canvas::empty();
    c_unpowered.set_view_size(200.0, 200.0);
    c_unpowered.set_center(100.0, 0.0);
    c_unpowered
        .load_sim1(sim1_unpowered, Some("/tmp/test_led_unpowered.sim1".into()))
        .unwrap();
    c_unpowered.power_on();
    for _ in 0..10 {
        c_unpowered.tick();
    }
    let unpowered_sim_pm =
        render_viewport(&c_unpowered, &pal, 200, 200, 1.0, true).expect("render failed");

    // Power on -> lit state with glowing halo
    c.power_on();
    for _ in 0..10 {
        c.tick();
    }
    let lit_pm = render_viewport(&c, &pal, 200, 200, 1.0, true).expect("lit render failed");
    assert_ne!(
        unpowered_sim_pm.data(),
        lit_pm.data(),
        "Lit LED pixmap must differ from unpowered LED pixmap"
    );

    // 2. Heavy overcurrent directly across LED
    let sim1_overload = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="12 V" Pos="0,0" />
<item itemtype="Led" CircId="Led-1" Color="Yellow" MaxCurrent="0.02" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="150,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Led-1-lPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="Led-1-rPin" pointList="150,0,100,0" />
</circuit>"#;

    let mut c_overload = Canvas::empty();
    c_overload.set_view_size(200.0, 200.0);
    c_overload.set_center(100.0, 0.0);
    c_overload
        .load_sim1(sim1_overload, Some("/tmp/test_led_overload.sim1".into()))
        .unwrap();
    c_overload.power_on();
    for _ in 0..10 {
        c_overload.tick();
    }

    let ov_state = c_overload
        .item_overload_state("Led-1")
        .expect("must have overload state");
    assert!(
        ov_state.crashed || ov_state.warning,
        "must have warning or crashed state"
    );

    let ov_pm =
        render_viewport(&c_overload, &pal, 200, 200, 1.0, true).expect("overload render failed");
    assert_ne!(
        unpowered_pm.data(),
        ov_pm.data(),
        "Overloaded LED pixmap must differ from unpowered pixmap"
    );
}

#[test]
fn test_lamp_visual_glow_and_overload_rendering() {
    use cs_engine::canvas::{Palette, render_viewport};

    let mut c = Canvas::empty();
    c.set_view_size(200.0, 200.0);
    c.set_center(100.0, 0.0);

    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="12 V" Pos="0,0" />
<item itemtype="Lamp" CircId="Lamp-1" Voltage="12" Power="5" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="150,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Lamp-1-lPin" pointList="0,0,100,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Ground-1-Gnd" endpinid="Lamp-1-rPin" pointList="150,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_lamp_lit.sim1".into()))
        .unwrap();

    let pal = Palette::dark();
    let unlit_pm = render_viewport(&c, &pal, 200, 200, 1.0, true).expect("unlit render failed");

    c.power_on();
    for _ in 0..10 {
        c.tick();
    }
    let lit_pm = render_viewport(&c, &pal, 200, 200, 1.0, true).expect("lit render failed");
    assert_ne!(
        unlit_pm.data(),
        lit_pm.data(),
        "Lit Lamp pixmap must differ from unlit pixmap"
    );
}

#[test]
fn test_led_floating_cathode_no_overcurrent() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // Fixed Voltage (5V) connected ONLY to Led-1 anode (lPin).
    // Cathode (rPin) is disconnected / floating (open circuit).
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="0,0" />
<item itemtype="Led" CircId="Led-1" Color="Yellow" MaxCurrent="0.03" Pos="100,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Led-1-lPin" pointList="0,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_led_floating.sim1".into()))
        .unwrap();
    c.power_on();
    assert!(c.sim_running(), "Sim must be running");

    for _ in 0..10 {
        c.tick();
    }

    // Must NOT have any overload warning or crash state
    let state = c.item_overload_state("Led-1");
    assert!(
        state.is_none(),
        "Floating cathode LED must not trigger an overload warning: got {:?}",
        state
    );

    // Pin current must be 0
    let cur = c.pin_current("Led-1-lPin").unwrap_or(0.0);
    assert_eq!(cur, 0.0, "Current through open-circuit LED must be 0");
}

#[test]
fn test_diode_floating_cathode_no_current() {
    let mut c = Canvas::empty();
    c.set_view_size(800.0, 600.0);

    // Fixed Voltage (5V) connected ONLY to Diode-1 anode (lPin).
    // Cathode (rPin) is disconnected.
    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="0,0" />
<item itemtype="Diode" CircId="Diode-1" Pos="100,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Diode-1-lPin" pointList="0,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_diode_floating.sim1".into()))
        .unwrap();
    c.power_on();
    assert!(c.sim_running(), "Sim must be running");

    for _ in 0..10 {
        c.tick();
    }

    let state = c.item_overload_state("Diode-1");
    assert!(
        state.is_none(),
        "Floating cathode diode must not trigger overload: got {:?}",
        state
    );

    let cur = c.pin_current("Diode-1-lPin").unwrap_or(0.0);
    assert_eq!(cur, 0.0, "Current through open-circuit diode must be 0");
}

#[test]
fn test_rgb_led_visual_glow_and_overload_rendering() {
    use cs_engine::canvas::{Palette, render_viewport};

    let mut c = Canvas::empty();
    c.set_view_size(200.0, 200.0);
    c.set_center(100.0, 0.0);

    let sim1 = r#"<circuit version="1.0.0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="0,0" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="220" Pos="50,0" />
<item itemtype="RGBLed" CircId="RGBLed-1" Pos="100,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="150,0" />
<item itemtype="Connector" uid="conn-1" startpinid="Fixed Voltage-1-outnod" endpinid="Resistor-1-lPin" pointList="0,0,50,0" />
<item itemtype="Connector" uid="conn-2" startpinid="Resistor-1-rPin" endpinid="RGBLed-1-rPin" pointList="50,0,100,0" />
<item itemtype="Connector" uid="conn-3" startpinid="Ground-1-Gnd" endpinid="RGBLed-1-cPin" pointList="150,0,100,0" />
</circuit>"#;

    c.load_sim1(sim1, Some("/tmp/test_rgb_led_lit.sim1".into()))
        .unwrap();

    let pal = Palette::dark();
    let unlit_pm = render_viewport(&c, &pal, 200, 200, 1.0, true).expect("unlit render failed");

    c.power_on();
    for _ in 0..10 {
        c.tick();
    }
    let lit_pm = render_viewport(&c, &pal, 200, 200, 1.0, true).expect("lit render failed");
    assert_ne!(
        unlit_pm.data(),
        lit_pm.data(),
        "Lit RGB LED pixmap must differ from unlit RGB LED pixmap"
    );
}
