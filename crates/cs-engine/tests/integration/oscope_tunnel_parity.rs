use cs_engine::Circuit;
use cs_engine::canvas::scene::Scene;
use cs_engine::canvas::{Canvas, Wire};
use cs_engine::elements::Comp;

#[test]
fn test_oscope_properties_and_prop_groups() {
    let mut scene = Scene::new();
    let osc_id = scene.add_oscope(0.0, 0.0);
    let it = scene.item_by_id_mut(&osc_id).expect("osc item");

    // Verify Main tab and Tunnels tab (hidden TestTime/DoTest drop the empty Test tab)
    let groups = it.prop_groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "Main");
    assert_eq!(groups[1].name, "Tunnels");

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
    let xml = scene.to_circ1();
    assert!(xml.contains(r#"Basic_X="200""#));
    assert!(xml.contains(r#"Basic_Y="150""#));
    assert!(xml.contains(r#"BufferSize="300000""#));
    assert!(xml.contains(r#"connectGnd="false""#));
    assert!(xml.contains(r#"InputImped="5 MΩ""#));
    assert!(xml.contains(r#"Tunnel1="netA""#));
    assert!(xml.contains(r#"TestTime="500 ns""#));
    assert!(xml.contains(r#"DoTest="true""#));

    let scene2 = Scene::from_circ1(&xml).expect("XML reload failed");
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
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].name, "Main");
    assert_eq!(groups[1].name, "Tunnels");
    assert_eq!(groups[2].name, "Export");

    assert_eq!(it.prop_text("Basic_X").as_deref(), Some("135"));
    assert_eq!(it.prop_text("TimeStep").as_deref(), Some("1000"));
    assert_eq!(it.prop_bool("AutoExport"), Some(false));

    assert!(it.set_prop_text("TimeStep", "500"));
    assert!(it.set_prop_bool("AutoExport", true));
    assert!(it.set_prop_text("Tunnel8", "sig8"));

    assert_eq!(it.prop_text("TimeStep").as_deref(), Some("500"));
    assert_eq!(it.prop_bool("AutoExport"), Some(true));
    assert_eq!(it.prop_text("Tunnel8").as_deref(), Some("sig8"));

    let xml = scene.to_circ1();
    assert!(xml.contains(r#"TimeStep="500""#));
    assert!(xml.contains(r#"AutoExport="true""#));
    assert!(xml.contains(r#"Tunnel8="sig8""#));

    let scene2 = Scene::from_circ1(&xml).expect("XML reload failed");
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

    // Verify on-canvas live scope traces resolution is 256
    let live = canvas
        .live_scope_traces(&osc_id)
        .expect("live oscope traces");
    assert_eq!(live.channels.len(), 4);
    assert_eq!(
        live.channels[0].samples.len(),
        cs_engine::instruments::SCOPE_CANVAS_DISPLAY_N
    );
    assert_eq!(live.channels[0].samples.len(), 256);

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
    canvas.load_circ1(src, None).expect("load sim1");
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

fn add_rail_tunnel(canvas: &mut Canvas, name: &str) -> (String, String) {
    let rail_id = canvas.scene_mut().add_rail(0.0, 0.0);
    let tun_id = canvas.scene_mut().add_tunnel(40.0, 0.0);
    assert!(
        canvas
            .scene_mut()
            .item_by_id_mut(&tun_id)
            .unwrap()
            .set_prop_text("Name", name)
    );
    let rail_pin = format!("{rail_id}-outnod");
    let tun_pin = format!("{tun_id}-pin");
    canvas
        .scene_mut()
        .add_saved_wire(Wire::from_saved("w-rail", rail_pin, tun_pin, vec![]));
    (rail_id, tun_id)
}

fn channel_peak(samples: &[f64]) -> f64 {
    samples.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

#[test]
fn live_oscope_tunnel_retargets_while_running() {
    let mut canvas = Canvas::new();
    let (_rail_id, tun_id) = add_rail_tunnel(&mut canvas, "SIG_A");
    let osc_id = canvas.scene_mut().add_oscope(100.0, 0.0);

    canvas.power_on();
    for _ in 0..3 {
        canvas.tick();
    }
    let started = canvas.circ_time();
    assert!(
        started > 1,
        "simulation should have advanced before the edit, t={started}"
    );

    let idle = canvas.live_scope_traces(&osc_id).expect("oscope sampler");
    assert!(
        !idle.channels[0].connected,
        "channel 1 starts on its probe pin"
    );

    canvas.set_prop_text_for(&osc_id, "Tunnel1".into(), "SIG_A".into());
    assert!(
        canvas.circ_time() >= started,
        "a tunnel edit must keep the running clock"
    );
    let joined = canvas.live_scope_traces(&osc_id).expect("oscope sampler");
    assert!(
        joined.channels[0].connected,
        "channel 1 should follow SIG_A"
    );
    let peak = channel_peak(&joined.channels[0].samples);
    assert!(peak > 4.5, "expected the 5 V rail on channel 1, got {peak}");

    canvas.set_prop_text_for(&osc_id, "InputImped".into(), "1 MΩ".into());
    assert!(canvas.circ_time() >= started);
    let held = canvas.live_scope_traces(&osc_id).expect("oscope sampler");
    assert!(
        held.channels[0].connected,
        "impedance is a stamp parameter and leaves the net in place"
    );

    canvas.set_prop_text_for(&tun_id, "Name".into(), "OTHER".into());
    let dropped = canvas.live_scope_traces(&osc_id).expect("oscope sampler");
    let peak = channel_peak(&dropped.channels[0].samples);
    assert!(
        peak < 0.5,
        "renaming the tunnel should take the rail off channel 1, got {peak}"
    );

    canvas.set_prop_text_for(&osc_id, "Tunnel1".into(), "".into());
    let cleared = canvas.live_scope_traces(&osc_id).expect("oscope sampler");
    assert!(
        !cleared.channels[0].connected,
        "an empty tunnel name returns the channel to its probe pin"
    );

    canvas.set_prop_text_for(&osc_id, "Tunnel1".into(), "OTHER".into());
    let again = canvas.live_scope_traces(&osc_id).expect("oscope sampler");
    assert!(again.channels[0].connected);
    let peak = channel_peak(&again.channels[0].samples);
    assert!(
        peak > 4.5,
        "expected the 5 V rail after the rename, got {peak}"
    );
}

#[test]
fn live_logic_analyzer_tunnel_retargets_while_running() {
    let mut canvas = Canvas::new();
    let (_rail_id, _tun_id) = add_rail_tunnel(&mut canvas, "SIG_A");
    let la_id = canvas.scene_mut().add_lanalizer(100.0, 0.0);

    canvas.power_on();
    canvas.tick();

    let idle = canvas.live_la_traces(&la_id).expect("analyzer sampler");
    assert!(!idle.channels[0].connected);

    canvas.set_prop_text_for(&la_id, "Tunnel1".into(), "SIG_A".into());
    let joined = canvas.live_la_traces(&la_id).expect("analyzer sampler");
    assert!(joined.channels[0].connected, "D0 should follow SIG_A");
    let peak = channel_peak(&joined.channels[0].samples);
    assert!(
        peak > 0.5,
        "expected a logic high from the rail, got {peak}"
    );
}

#[test]
fn resized_oscope_and_la_paint_and_bounds_match_basic_xy() {
    use cs_engine::canvas::Rect;
    use cs_engine::canvas::draw::{Align, Color, Draw, PaintCtx, Palette};
    use cs_engine::components::{Component, Drawable, LogicAnalyzer, Oscope, PropValue};

    struct RoundRectRecorder {
        rects: Vec<(f64, f64, f64, f64)>,
    }
    impl Draw for RoundRectRecorder {
        fn fill_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
        fn stroke_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, _: f64, _: Color) {
            self.rects.push((x, y, w, h));
        }
        fn stroke_round_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_circle(&mut self, _: f64, _: f64, _: f64, _: Color) {}
        fn stroke_circle(&mut self, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
        fn stroke_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn fill_poly(&mut self, _: &[[f64; 2]], _: Color) {}
        fn stroke_poly(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
        fn polyline(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
        fn line(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn arc(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
        fn text(&mut self, _: f64, _: f64, _: &str, _: f64, _: Color, _: Align) {}
        fn text_bold(&mut self, _: f64, _: f64, _: &str, _: f64, _: Color, _: Align) {}
        fn grid_dots(&mut self, _: Rect, _: i32, _: Color) {}
        fn push(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64) {}
        fn pop(&mut self) {}
    }

    let canvas = Canvas::new();
    let pal = Palette::light();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: "inst",
        on_board: false,
        active_item_id: None,
    };

    // For Basic_X = 250, Basic_Y = 210:
    // screen = (-6, -101.5, 245, 203), chassis = (-80, -109.5, 328, 219)
    let expected_chassis = Rect::new(-80.0, -109.5, 328.0, 219.0);
    let expected_screen = (-6.0, -101.5, 245.0, 203.0);

    let mut osc = Oscope::default();
    osc.set_prop("Basic_X", PropValue::Int(250)).unwrap();
    osc.set_prop("Basic_Y", PropValue::Int(210)).unwrap();
    assert_eq!(osc.body(), expected_chassis);

    let mut rec_osc = RoundRectRecorder { rects: Vec::new() };
    osc.paint(&mut rec_osc, &ctx);
    assert!(rec_osc.rects.contains(&(
        expected_chassis.x,
        expected_chassis.y,
        expected_chassis.w,
        expected_chassis.h,
    )));
    assert!(rec_osc.rects.contains(&expected_screen));

    let mut la = LogicAnalyzer::default();
    la.set_prop("Basic_X", PropValue::Int(250)).unwrap();
    la.set_prop("Basic_Y", PropValue::Int(210)).unwrap();
    assert_eq!(la.body(), expected_chassis);

    let mut rec_la = RoundRectRecorder { rects: Vec::new() };
    la.paint(&mut rec_la, &ctx);
    assert!(rec_la.rects.contains(&(
        expected_chassis.x,
        expected_chassis.y,
        expected_chassis.w,
        expected_chassis.h,
    )));
    assert!(rec_la.rects.contains(&expected_screen));
}

#[test]
fn bounded_buffer_size_limits_captured_samples_in_circuit() {
    let mut c = Circuit::new();
    c.add_comp(Comp::wave_gen(
        "WG", "Sine", 10_000.0, 2.5, 2.5, 0.5, 0.0, 100, false, false,
    ));
    c.add_comp(Comp::resistor("R1", 1000.0));
    c.add_comp(Comp::ground("GND"));
    c.add_comp(Comp::oscope_with_buffer(
        "OSC",
        true,
        10.0,
        120,
        Default::default(),
    ));
    c.add_comp(Comp::lanalizer_with_buffer(
        "LA",
        true,
        10.0,
        150,
        Default::default(),
    ));

    c.connect("WG-outnod", "R1-lPin");
    c.connect("R1-rPin", "GND-Gnd");
    c.connect("WG-outnod", "OSC-Pin0");
    c.connect("GND-Gnd", "OSC-PinG");
    c.connect("WG-outnod", "LA-Pin0");

    c.solve().unwrap();
    c.run_ps(1_000_000_000).unwrap(); // 1 ms = 10 cycles @ 10 kHz (~1000 steps)

    let osc = c.instruments.scopes.get("OSC").expect("oscope sampler");
    assert_eq!(osc.capacity(), 120);
    assert_eq!(osc.len(), 120);
    assert_eq!(osc.allocated_len(), 120);

    let la = c.instruments.las.get("LA").expect("LA sampler");
    assert_eq!(la.capacity(), 150);
    assert!(la.len() <= 150 && la.len() > 0);
}

#[test]
fn logic_analyzer_vcd_export_and_auto_export_on_pause() {
    use std::fs;

    let mut canvas = Canvas::new();
    let clk_id = canvas.scene_mut().add_clock(0.0, 0.0);
    let tun_id = canvas.scene_mut().add_tunnel(40.0, 0.0);
    let la_id = canvas.scene_mut().add_lanalizer(160.0, 0.0);

    canvas.set_prop_text_for(&tun_id, "Name".into(), "CLK".into());
    canvas.set_prop_text_for(&clk_id, "Freq".into(), "10 kHz".into());
    canvas.set_prop_text_for(&la_id, "Tunnel1".into(), "CLK".into());
    canvas.set_prop_text_for(&la_id, "AutoExport".into(), "true".into());
    canvas.set_prop_text_for(&la_id, "TimeStep".into(), "1000".into());

    let clk_pin = format!("{clk_id}-outnod");
    let tun_pin = format!("{tun_id}-pin");
    canvas
        .scene_mut()
        .add_saved_wire(Wire::from_saved("w-clk", clk_pin, tun_pin, vec![]));

    canvas.power_on();
    for _ in 0..3 {
        canvas.tick();
    }

    // Manual VCD export works even before the circuit has a file path.
    let vcd = canvas.export_la_vcd().expect("export_la_vcd");
    assert!(vcd.contains("$timescale\n   1ns\n$end"), "vcd:\n{vcd}");
    assert!(vcd.contains("$var wire 1 ! CLK $end"), "vcd:\n{vcd}");
    assert!(vcd.contains("$dumpvars\n"), "vcd:\n{vcd}");
    assert!(
        vcd.contains("\n1!\n") && vcd.contains("\n0!\n"),
        "expected rising and falling edges on CLK:\n{vcd}"
    );

    // Unsaved circuit skips AutoExport cleanly on pause.
    canvas.pause_sim();
    canvas.resume_sim();

    // Saved circuit writes `<stem>_<la_id>.vcd` automatically on pause.
    let tmp_dir = std::env::temp_dir().join(format!("cs_vcd_auto_export_{}", std::process::id()));
    let _ = fs::create_dir_all(&tmp_dir);
    let circuit_path = tmp_dir.join("demo_circuit.sim1");
    canvas.set_file_path(Some(circuit_path.to_string_lossy().into_owned()));

    canvas.pause_sim();

    let expected_vcd = tmp_dir.join(format!("demo_circuit_{la_id}.vcd"));
    let written = fs::read_to_string(&expected_vcd).expect("auto-exported VCD file should exist");
    assert!(written.contains("$var wire 1 ! CLK $end"));
    assert!(written.contains("$timescale\n   1ns\n$end"));

    let _ = fs::remove_dir_all(&tmp_dir);
}
