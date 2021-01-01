use cs_engine::canvas::scene::Scene;
use cs_engine::canvas::{Canvas, Wire};

#[test]
fn test_oscope_properties_and_prop_groups() {
    let mut scene = Scene::new();
    let osc_id = scene.add_oscope(0.0, 0.0);
    let it = scene.item_by_id_mut(&osc_id).expect("osc item");

    // Verify Main tab, Tunnels tab, Test tab
    let groups = it.prop_groups();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].name, "Main");
    assert_eq!(groups[1].name, "Tunnels");
    assert_eq!(groups[2].name, "Test");

    // Verify Main tab properties
    assert_eq!(it.prop_text("Basic_X").as_deref(), Some("135"));
    assert_eq!(it.prop_text("Basic_Y").as_deref(), Some("135"));
    assert_eq!(it.prop_text("BufferSize").as_deref(), Some("600000"));
    assert_eq!(it.prop_bool("connectGnd"), Some(true));
    assert_eq!(it.prop_text("InputImped").as_deref(), Some("10 MΩ"));

    // Verify Tunnels tab properties
    assert_eq!(it.prop_text("Tunnel1").as_deref(), Some(""));
    assert_eq!(it.prop_text("Tunnel2").as_deref(), Some(""));

    // Verify Test tab properties
    assert_eq!(it.prop_text("TestTime").as_deref(), Some("0 s"));
    assert_eq!(it.prop_bool("DoTest"), Some(false));

    // Edit properties
    assert!(it.set_prop_text("Basic_X", "200"));
    assert!(it.set_prop_text("Basic_Y", "150"));
    assert!(it.set_prop_text("BufferSize", "300000"));
    assert!(it.set_prop_bool("connectGnd", false));
    assert!(it.set_prop_text("InputImped", "5 MΩ"));
    assert!(it.set_prop_text("Tunnel1", "netA"));
    assert!(it.set_prop_text("TestTime", "500 ns"));
    assert!(it.set_prop_bool("DoTest", true));

    assert_eq!(it.prop_text("Basic_X").as_deref(), Some("200"));
    assert_eq!(it.prop_text("Basic_Y").as_deref(), Some("150"));
    assert_eq!(it.prop_text("BufferSize").as_deref(), Some("300000"));
    assert_eq!(it.prop_bool("connectGnd"), Some(false));
    assert_eq!(it.prop_text("InputImped").as_deref(), Some("5 MΩ"));
    assert_eq!(it.prop_text("Tunnel1").as_deref(), Some("netA"));
    assert_eq!(it.prop_text("TestTime").as_deref(), Some("500 ns"));
    assert_eq!(it.prop_bool("DoTest"), Some(true));

    // Serialize to sim1 and reload
    let xml = scene.to_sim1();
    assert!(xml.contains(r#"Basic_X="200""#));
    assert!(xml.contains(r#"Basic_Y="150""#));
    assert!(xml.contains(r#"BufferSize="300000""#));
    assert!(xml.contains(r#"connectGnd="false""#));
    assert!(xml.contains(r#"InputImped="5 MΩ""#));
    assert!(xml.contains(r#"Tunnel1="netA""#));
    assert!(xml.contains(r#"TestTime="500 ns""#));
    assert!(xml.contains(r#"DoTest="true""#));

    let scene2 = Scene::from_sim1(&xml).expect("XML reload failed");
    let it2 = scene2.item_by_id(&osc_id).expect("osc item reloaded");
    assert_eq!(it2.prop_text("Basic_X").as_deref(), Some("200"));
    assert_eq!(it2.prop_text("Basic_Y").as_deref(), Some("150"));
    assert_eq!(it2.prop_text("BufferSize").as_deref(), Some("300000"));
    assert_eq!(it2.prop_bool("connectGnd"), Some(false));
    assert_eq!(it2.prop_text("InputImped").as_deref(), Some("5 MΩ"));
    assert_eq!(it2.prop_text("Tunnel1").as_deref(), Some("netA"));
    assert_eq!(it2.prop_text("TestTime").as_deref(), Some("500 ns"));
    assert_eq!(it2.prop_bool("DoTest"), Some(true));
}

#[test]
fn test_lanalizer_properties_and_prop_groups() {
    let mut scene = Scene::new();
    let la_id = scene.add_lanalizer(0.0, 0.0);
    let it = scene.item_by_id_mut(&la_id).expect("la item");

    let groups = it.prop_groups();
    assert_eq!(groups.len(), 4);
    assert_eq!(groups[0].name, "Main");
    assert_eq!(groups[1].name, "Tunnels");
    assert_eq!(groups[2].name, "Export");
    assert_eq!(groups[3].name, "Test");

    assert_eq!(it.prop_text("Basic_X").as_deref(), Some("135"));
    assert_eq!(it.prop_text("TimeStep").as_deref(), Some("1000"));
    assert_eq!(it.prop_bool("AutoExport"), Some(false));

    assert!(it.set_prop_text("TimeStep", "500"));
    assert!(it.set_prop_bool("AutoExport", true));
    assert!(it.set_prop_text("Tunnel8", "sig8"));

    assert_eq!(it.prop_text("TimeStep").as_deref(), Some("500"));
    assert_eq!(it.prop_bool("AutoExport"), Some(true));
    assert_eq!(it.prop_text("Tunnel8").as_deref(), Some("sig8"));

    let xml = scene.to_sim1();
    assert!(xml.contains(r#"TimeStep="500""#));
    assert!(xml.contains(r#"AutoExport="true""#));
    assert!(xml.contains(r#"Tunnel8="sig8""#));

    let scene2 = Scene::from_sim1(&xml).expect("XML reload failed");
    let it2 = scene2.item_by_id(&la_id).expect("la item reloaded");
    assert_eq!(it2.prop_text("TimeStep").as_deref(), Some("500"));
    assert_eq!(it2.prop_bool("AutoExport"), Some(true));
    assert_eq!(it2.prop_text("Tunnel8").as_deref(), Some("sig8"));
}

#[test]
fn test_oscope_tunnel_connectivity_and_selective_rendering() {
    let mut canvas = Canvas::new();
    let scene = canvas.scene_mut();

    // Add WaveGen 1 kHz Sine, 5V amplitude
    let wg_id = scene.add_wave_gen(0.0, 0.0);
    let wg = scene.item_by_id_mut(&wg_id).unwrap();
    wg.set_prop_text("Wave_Type", "Sine");
    wg.set_prop_text("Frequency", "1000 Hz");
    wg.set_prop_text("Amplitude", "5 V");

    // Add a Tunnel named "SIG_A" connected to WaveGen out
    let tun_id = scene.add_tunnel(40.0, 0.0);
    scene
        .item_by_id_mut(&tun_id)
        .unwrap()
        .set_prop_text("Name", "SIG_A");
    scene.add_saved_wire(Wire::from_saved(
        "w1",
        format!("{wg_id}-outnod"),
        format!("{tun_id}-pin"),
        vec![],
    ));

    // Add Ground connected to WaveGen gnd
    let gnd_id = scene.add_ground(0.0, 40.0);
    scene.add_saved_wire(Wire::from_saved(
        "w2",
        format!("{wg_id}-gndnod"),
        format!("{gnd_id}-gnd"),
        vec![],
    ));

    // Add Oscilloscope with Channel 1 connected via Tunnel "SIG_A"
    let osc_id = scene.add_oscope(100.0, 0.0);
    let osc = scene.item_by_id_mut(&osc_id).unwrap();
    osc.set_prop_text("Tunnel1", "SIG_A"); // Channel 1 (index 0)

    // Power on simulation
    canvas.power_on();

    // Step simulation (5 ticks = 166ms = 166 full cycles of 1 kHz)
    for _ in 0..5 {
        canvas.tick();
    }

    // Verify scope traces
    let traces = canvas.scope_traces();
    assert_eq!(traces.channels.len(), 4);

    // Channel 0 (connected via Tunnel "SIG_A") should be connected and have active samples
    let ch0 = &traces.channels[0];
    assert!(ch0.connected, "Channel 0 should be connected via tunnel");
    assert!(!ch0.samples.is_empty(), "Channel 0 should have samples");
    let max_v = ch0
        .samples
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let min_v = ch0.samples.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(max_v > 4.5, "Expected peak ~5.0V on Channel 0, got {max_v}");
    assert!(
        min_v < 0.5,
        "Expected trough ~0.0V on Channel 0, got {min_v}"
    );

    // Channels 1, 2, 3 (unconnected) should NOT be connected and have empty samples (no flat line)
    for i in 1..4 {
        let ch = &traces.channels[i];
        assert!(!ch.connected, "Channel {i} should be disconnected");
        assert!(
            ch.samples.is_empty(),
            "Channel {i} should have empty samples"
        );
    }

    // Verify frequency readout in canvas readings
    let readings = canvas.readings();
    let osc_rd = readings.get(&osc_id).expect("oscope reading view");
    let freqs: Vec<&str> = osc_rd.text.split(';').collect();
    assert_eq!(freqs.len(), 4);
    assert!(
        freqs[0].contains("kHz") || freqs[0].contains("Hz"),
        "Expected frequency readout for Ch0, got {}",
        freqs[0]
    );
    assert_eq!(freqs[1], "0 Hz");
    assert_eq!(freqs[2], "0 Hz");
    assert_eq!(freqs[3], "0 Hz");
}

#[test]
fn test_unseeded_tunnel_groups_connectivity() {
    let src = r#"<circuit version="1.0.0" stepSize="1000" stepsPS="1000000">
<item itemtype="Tunnel" CircId="Tunnel-1" Name="BUS_SIG" Pos="50,0" />
<item itemtype="Tunnel" CircId="Tunnel-2" Name="BUS_SIG" Pos="150,0" />
<item itemtype="Rail" CircId="Rail-1" Voltage="5 V" Pos="0,0" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" Pos="200,0" />
<item itemtype="Ground" CircId="Ground-1" Pos="250,0" />
<item itemtype="Connector" CircId="w1" startpinid="Rail-1-outnod" endpinid="Tunnel-1-pin" pointList="0,0 50,0" />
<item itemtype="Connector" CircId="w2" startpinid="Tunnel-2-pin" endpinid="Resistor-1-lPin" pointList="150,0 200,0" />
<item itemtype="Connector" CircId="w3" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" pointList="200,0 250,0" />
</circuit>"#;

    let mut canvas = Canvas::new();
    canvas.load_sim1(src, None).expect("load sim1");
    canvas.power_on();

    // Step simulation
    canvas.tick();

    // Check that Resistor-1-lPin receives voltage from Rail-1 via the unseeded tunnel net
    let v_r = canvas
        .pin_volts()
        .get("Resistor-1-lPin")
        .copied()
        .unwrap_or(0.0);
    assert!(
        (v_r - 5.0).abs() < 1e-2,
        "Expected 5.0V transferred across unseeded tunnel, got {v_r}"
    );
}

#[test]
fn test_oscope_auto_scale_and_trigger_controls() {
    let mut canvas = Canvas::new();
    let scene = canvas.scene_mut();

    // Add WaveGen 1 kHz Sine, 5V amplitude (0..5V swing)
    let wg_id = scene.add_wave_gen(0.0, 0.0);
    let wg = scene.item_by_id_mut(&wg_id).unwrap();
    wg.set_prop_text("Wave_Type", "Sine");
    wg.set_prop_text("Frequency", "1000 Hz");
    wg.set_prop_text("Amplitude", "5 V");

    let tun_id = scene.add_tunnel(40.0, 0.0);
    scene
        .item_by_id_mut(&tun_id)
        .unwrap()
        .set_prop_text("Name", "SIG_OSC");
    scene.add_saved_wire(Wire::from_saved(
        "w1",
        format!("{wg_id}-outnod"),
        format!("{tun_id}-pin"),
        vec![],
    ));

    let gnd_id = scene.add_ground(0.0, 40.0);
    scene.add_saved_wire(Wire::from_saved(
        "w2",
        format!("{wg_id}-gndnod"),
        format!("{gnd_id}-gnd"),
        vec![],
    ));

    let osc_id = scene.add_oscope(100.0, 0.0);
    let osc = scene.item_by_id_mut(&osc_id).unwrap();
    osc.set_prop_text("Tunnel1", "SIG_OSC");

    canvas.power_on();

    for _ in 0..5 {
        canvas.tick();
    }

    // Test Auto Scale on Channel 0 (bipolar -5V..+5V sine wave centered at 0V)
    let auto_res = canvas
        .auto_scale_scope(0)
        .expect("auto scale scope should return result");
    assert!(
        (auto_res.volt_pos - 0.0).abs() < 0.5,
        "volt_pos should center bipolar signal around 0.0V, got {}",
        auto_res.volt_pos
    );
    assert!(
        auto_res.volt_div >= 1.0 && auto_res.volt_div <= 2.0,
        "volt_div should fit 10Vpp swing, got {}",
        auto_res.volt_div
    );
    let td = auto_res
        .time_div
        .expect("periodic signal should have time_div");
    assert!(
        td >= 1e-4 && td <= 1e-3,
        "time_div should fit 1kHz signal, got {}",
        td
    );

    // Test Trigger controls on Canvas
    assert_eq!(
        canvas.scope_trigger(),
        -1,
        "Default scope trigger should be -1 (free-running)"
    );
    canvas.set_scope_trigger(0);
    assert_eq!(canvas.scope_trigger(), 0);
    canvas.set_scope_trigger(-1);
    assert_eq!(canvas.scope_trigger(), -1);
}
