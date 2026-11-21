use cs_engine::Circuit;
use cs_engine::canvas::scene::{Item, Scene};
use cs_engine::elements::{Comp, Kind};
use cs_engine::parse_sim1;
use cs_engine::wav::WavData;

#[test]
fn test_wavegen_sine_unipolar_simulation() {
    let mut circ = Circuit::default();
    circ.dt = 1e-6; // 1 us step
    // 1 kHz Sine, Amplitude = 2.5V, Offset = 2.5V -> 0V to 5V unipolar sine
    circ.add_comp(Comp::wave_gen(
        "wg1", "Sine", 1000.0, 2.5, 2.5, 0.5, 0.0, 100, false, false,
    ));
    circ.add_comp(Comp::resistor("r1", 1000.0));
    circ.add_comp(Comp::ground("gnd"));

    circ.connect("wg1-outnod", "r1-lPin");
    circ.connect("r1-rPin", "gnd-Gnd");

    circ.solve().unwrap();
    // At t=0, Sine(0) = 0.5 -> V = (2.5 - 2.5) + 5.0 * 0.5 = 2.5V
    let v_start = circ.pin_voltage("wg1-outnod").unwrap();
    assert!(
        (v_start - 2.5).abs() < 1e-3,
        "Expected 2.5V start, got {v_start}"
    );

    // Advance 250 us (1/4 period of 1 kHz) -> peak of sine
    for _ in 0..250 {
        circ.step().unwrap();
    }
    let v_peak = circ.pin_voltage("wg1-outnod").unwrap();
    assert!(
        (v_peak - 5.0).abs() < 0.05,
        "Expected ~5.0V peak, got {v_peak}"
    );

    // Advance another 500 us (3/4 period) -> trough of sine
    for _ in 0..500 {
        circ.step().unwrap();
    }
    let v_trough = circ.pin_voltage("wg1-outnod").unwrap();
    assert!(
        (v_trough - 0.0).abs() < 0.05,
        "Expected ~0.0V trough, got {v_trough}"
    );
}

#[test]
fn test_wavegen_saw_triangle_square_eval() {
    // Saw: ramp from 0 to 1 over period
    let saw = Comp::wave_gen("wg", "Saw", 1000.0, 1.0, 0.0, 0.5, 0.0, 100, false, false);
    assert!((saw.wavegen_calc_vout(0) - 0.0).abs() < 1e-6);
    assert!((saw.wavegen_calc_vout(500_000_000) - 0.5).abs() < 1e-3); // 500 us
    assert!((saw.wavegen_calc_vout(999_000_000) - 0.999).abs() < 1e-3);

    // Triangle: 50% duty rises 0..1 from 0..500us, falls 1..0 from 500us..1000us
    let tri = Comp::wave_gen(
        "wg", "Triangle", 1000.0, 1.0, 0.0, 0.5, 0.0, 100, false, false,
    );
    assert!((tri.wavegen_calc_vout(0) - 0.0).abs() < 1e-6);
    assert!((tri.wavegen_calc_vout(250_000_000) - 0.5).abs() < 1e-3);
    assert!((tri.wavegen_calc_vout(500_000_000) - 1.0).abs() < 1e-3);
    assert!((tri.wavegen_calc_vout(750_000_000) - 0.5).abs() < 1e-3);

    // Square: 25% duty -> 1.0 for first 250us, 0.0 for remaining 750us
    let sq = Comp::wave_gen(
        "wg", "Square", 1000.0, 1.0, 0.0, 0.25, 0.0, 100, false, false,
    );
    assert_eq!(sq.wavegen_calc_vout(0), 1.0);
    assert_eq!(sq.wavegen_calc_vout(200_000_000), 1.0);
    assert_eq!(sq.wavegen_calc_vout(260_000_000), 0.0);
    assert_eq!(sq.wavegen_calc_vout(900_000_000), 0.0);

    // Phase shift: 90 degrees on Square (250us delay on 1000us cycle)
    let sq_phase = Comp::wave_gen(
        "wg", "Square", 1000.0, 1.0, 0.0, 0.5, 90.0, 100, false, false,
    );
    // At t=0, effective t_cycle = (0 - 250us) % 1000us = 750us -> > 500us -> 0.0
    assert_eq!(sq_phase.wavegen_calc_vout(0), 0.0);
    // At t=300us, effective t_cycle = (300us - 250us) = 50us -> < 500us -> 1.0
    assert_eq!(sq_phase.wavegen_calc_vout(300_000_000), 1.0);

    // Legacy/palette duty stored as percent (50) must not collapse triangle→saw
    // or square→DC. C++ WaveGen keeps duty as 0..=100 and divides by 100.
    let tri_pct = Comp::wave_gen(
        "wg", "Triangle", 1000.0, 1.0, 0.0, 50.0, 0.0, 100, false, false,
    );
    assert!(
        (tri_pct.wavegen_calc_vout(250_000_000) - 0.5).abs() < 1e-3,
        "triangle 50% duty should be 0.5 at 1/4 period, not a saw (0.25)"
    );
    assert!(
        (tri_pct.wavegen_calc_vout(750_000_000) - 0.5).abs() < 1e-3,
        "triangle 50% duty should fall to 0.5 at 3/4 period, not a saw (0.75)"
    );
    let sq_pct = Comp::wave_gen(
        "wg", "Square", 1000.0, 1.0, 0.0, 50.0, 0.0, 100, false, false,
    );
    assert_eq!(sq_pct.wavegen_calc_vout(200_000_000), 1.0);
    assert_eq!(sq_pct.wavegen_calc_vout(600_000_000), 0.0);
}

#[test]
fn test_wavegen_bipolar_and_floating() {
    // 1. Ground-referenced Bipolar (2 pins: outnod and gndnod centered around Offset)
    let mut circ = Circuit::default();
    circ.dt = 1e-6;
    // Amplitude = 3.0V, Offset = 2.0V, Square wave 50% duty
    circ.add_comp(Comp::wave_gen(
        "wg", "Square", 1000.0, 3.0, 2.0, 0.5, 0.0, 100, true, false,
    ));
    circ.add_comp(Comp::resistor("r1", 1000.0));
    circ.add_comp(Comp::resistor("r2", 1000.0));
    circ.add_comp(Comp::ground("gnd"));

    circ.connect("wg-outnod", "r1-lPin");
    circ.connect("r1-rPin", "gnd-Gnd");
    circ.connect("wg-gndnod", "r2-lPin");
    circ.connect("r2-rPin", "gnd-Gnd");

    circ.solve().unwrap();
    // In high state (vout=1.0): volt = 2*3*(1-0.5) = +3.0V
    // outnod = offset + 1.5 = 3.5V, gndnod = offset - 1.5 = 0.5V
    let v_out = circ.pin_voltage("wg-outnod").unwrap();
    let v_gnd = circ.pin_voltage("wg-gndnod").unwrap();
    assert!((v_out - 3.5).abs() < 1e-3, "Expected 3.5V, got {v_out}");
    assert!((v_gnd - 0.5).abs() < 1e-3, "Expected 0.5V, got {v_gnd}");

    // 2. Floating Bipolar (acts as differential source between outnod and gndnod)
    let mut circ_fl = Circuit::default();
    circ_fl.dt = 1e-6;
    circ_fl.add_comp(Comp::wave_gen(
        "wg", "Square", 1000.0, 3.0, 0.0, 0.5, 0.0, 100, true, true,
    ));
    circ_fl.add_comp(Comp::resistor("r1", 1000.0));
    circ_fl.add_comp(Comp::ground("gnd"));
    // Ground one terminal and measure across resistor
    circ_fl.connect("wg-gndnod", "gnd-Gnd");
    circ_fl.connect("wg-outnod", "r1-lPin");
    circ_fl.connect("r1-rPin", "gnd-Gnd");

    circ_fl.solve().unwrap();
    // High state: differential voltage = 2 * 3.0 * (1.0 - 0.5) = 3.0V
    let v_diff =
        circ_fl.pin_voltage("wg-outnod").unwrap() - circ_fl.pin_voltage("wg-gndnod").unwrap();
    assert!(
        (v_diff - 3.0).abs() < 1e-3,
        "Expected 3.0V diff, got {v_diff}"
    );
}

#[test]
fn test_wavegen_wav_format_loading() {
    // Generate valid 16-bit PCM WAV in memory
    let sample_rate = 8000u32;
    let num_samples = 80;
    let mut data_bytes = Vec::new();
    for i in 0..num_samples {
        let val = if i < 40 { 16000i16 } else { -16000i16 };
        data_bytes.extend_from_slice(&val.to_le_bytes());
    }

    let mut wav_file = Vec::new();
    wav_file.extend_from_slice(b"RIFF");
    wav_file.extend_from_slice(&((36 + data_bytes.len()) as u32).to_le_bytes());
    wav_file.extend_from_slice(b"WAVEfmt ");
    wav_file.extend_from_slice(&16u32.to_le_bytes());
    wav_file.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav_file.extend_from_slice(&1u16.to_le_bytes()); // Mono
    wav_file.extend_from_slice(&sample_rate.to_le_bytes());
    wav_file.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav_file.extend_from_slice(&2u16.to_le_bytes());
    wav_file.extend_from_slice(&16u16.to_le_bytes());
    wav_file.extend_from_slice(b"data");
    wav_file.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    wav_file.extend_from_slice(&data_bytes);

    let parsed = WavData::from_bytes(&wav_file).expect("WAV parsing must succeed");
    assert_eq!(parsed.sample_rate, 8000);
    assert_eq!(parsed.samples.len(), 80);
    // Positive samples normalize to > 0.5, negative to < 0.5
    assert!(parsed.samples[10] > 0.7);
    assert!(parsed.samples[50] < 0.3);
}

#[test]
fn test_wavegen_scene_properties_and_pins() {
    let mut scene = Scene::default();
    let mut wg = Item::wave_gen("wg1", 100.0, 100.0, "Triangle", 500.0, 2.0, 1.0, 0.4);

    // Test unipolar pins
    assert_eq!(wg.pins().len(), 1);
    assert_eq!(wg.pins()[0].id, "wg1-outnod");
    assert_eq!(wg.body_rect().w, 16.0);
    assert_eq!(wg.body_rect().h, 16.0);

    // Switch to bipolar
    wg.set_prop_bool("Bipolar", true);
    assert_eq!(wg.pins().len(), 2);
    assert_eq!(wg.pins()[0].id, "wg1-outnod");
    assert_eq!(wg.pins()[1].id, "wg1-gndnod");

    // Set properties
    wg.set_prop_text("Wave_Type", "Random");
    wg.set_prop_text("Frequency", "2.5 kHz");
    wg.set_prop_text("Amplitude", "4.5 V");
    wg.set_prop_text("Offset", "-1.2 V");
    wg.set_prop_text("Phase", "45°");
    wg.set_prop_text("Steps", "200");
    wg.set_prop_bool("Floating", true);

    assert_eq!(wg.prop_text("Wave_Type").unwrap(), "Random");
    assert_eq!(wg.prop_text("Frequency").unwrap(), "2.5 kHz");
    assert_eq!(wg.prop_text("Amplitude").unwrap(), "4.5 V");
    assert_eq!(wg.prop_text("Offset").unwrap(), "-1.2 V");
    assert_eq!(wg.prop_text("Phase").unwrap(), "45 °");
    assert_eq!(wg.prop_text("Steps").unwrap(), "200");
    assert!(wg.prop_bool("Floating").unwrap());
    assert!(wg.prop_bool("Bipolar").unwrap());

    scene.add_saved_item(wg);

    // Test XML export & re-import
    let xml = scene.to_sim1();
    assert!(xml.contains("itemtype=\"WaveGen\""));
    assert!(xml.contains("WaveType=\"Random\"") || xml.contains("Wave_Type=\"Random\""));
    assert!(xml.contains("Bipolar=\"true\""));
    assert!(xml.contains("Floating=\"true\""));

    let parsed = parse_sim1(&xml).expect("XML parse should succeed");
    assert_eq!(parsed.items.len(), 1);
    match &parsed.items[0].comp.kind {
        Kind::WaveGen {
            wave_type,
            freq_hz,
            amplitude,
            offset,
            phase,
            steps,
            bipolar,
            floating,
            ..
        } => {
            assert_eq!(wave_type, "Random");
            assert_eq!(*freq_hz, 2500.0);
            assert_eq!(*amplitude, 4.5);
            assert_eq!(*offset, -1.2);
            assert_eq!(*phase, 45.0);
            assert_eq!(*steps, 200);
            assert!(*bipolar);
            assert!(*floating);
        }
        _ => panic!("Parsed item was not WaveGen"),
    }
}

#[test]
fn test_wavegen_oscilloscope_sine_capture() {
    let mut circ = Circuit::default();
    circ.dt = 1e-6; // 1 us step
    // 1 kHz Sine, Amplitude = 2.5V, Offset = 2.5V (0V..5V unipolar sine wave)
    circ.add_comp(Comp::wave_gen(
        "wg1", "Sine", 1000.0, 2.5, 2.5, 0.5, 0.0, 100, false, false,
    ));
    circ.add_comp(Comp::oscope("osc1", false));
    circ.add_comp(Comp::ground("gnd"));

    // Connect WaveGen out to Oscope channel 0 and gnd to scope gnd
    circ.connect("wg1-outnod", "osc1-Pin0");
    circ.connect("gnd-Gnd", "osc1-PinG");

    // Run simulation for 20 ms (20,000 steps of 1 us, which is 20 full cycles of 1 kHz)
    circ.solve().unwrap();
    circ.run_ps(20_000_000_000).unwrap(); // 20 ms in ps

    let sampler = circ
        .instruments
        .scopes
        .get("osc1")
        .expect("ScopeSampler should exist");
    // Request 1 ms/div (10 ms display window -> 10 complete sine cycles across 512 display points)
    let buffer = sampler.traces(
        1e-3,
        0.0,
        0,
        2.5,
        &cs_engine::instruments::SCOPE_COLORS,
        None,
    );
    assert_eq!(buffer.channels.len(), 4);
    let ch0 = &buffer.channels[0].samples;
    assert_eq!(ch0.len(), 512);

    let max_v = ch0.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_v = ch0.iter().copied().fold(f64::INFINITY, f64::min);

    // Sine peak should reach ~5.0V, trough should reach ~0.0V
    assert!(
        max_v >= 4.90,
        "Expected peak ~5.0V on oscilloscope, got {max_v}"
    );
    assert!(
        min_v <= 0.10,
        "Expected trough ~0.0V on oscilloscope, got {min_v}"
    );

    // Verify waveform is continuous and has multiple transitions between high and low
    let mut high_count = 0;
    let mut low_count = 0;
    for &v in ch0 {
        if v > 4.5 {
            high_count += 1;
        } else if v < 0.5 {
            low_count += 1;
        }
    }
    assert!(
        high_count > 20,
        "Should have multiple high peaks across 10 cycles, got {high_count}"
    );
    assert!(
        low_count > 20,
        "Should have multiple low troughs across 10 cycles, got {low_count}"
    );
}

#[test]
fn test_wavegen_oscilloscope_square_not_ramp() {
    let mut circ = Circuit::default();
    circ.dt = 1e-6;
    circ.add_comp(Comp::wave_gen(
        "wg1", "Square", 1000.0, 2.5, 2.5, 0.5, 0.0, 100, false, false,
    ));
    circ.add_comp(Comp::oscope("osc1", false));
    circ.add_comp(Comp::ground("gnd"));
    circ.connect("wg1-outnod", "osc1-Pin0");
    circ.connect("gnd-Gnd", "osc1-PinG");

    circ.solve().unwrap();
    circ.run_ps(20_000_000_000).unwrap();

    let sampler = circ
        .instruments
        .scopes
        .get("osc1")
        .expect("ScopeSampler should exist");
    let buffer = sampler.traces(
        1e-3,
        0.0,
        0,
        2.5,
        &cs_engine::instruments::SCOPE_COLORS,
        None,
    );
    let ch0 = &buffer.channels[0].samples;
    assert_eq!(ch0.len(), 512);

    let mut high = 0;
    let mut low = 0;
    let mut mid = 0;
    for &v in ch0 {
        if v > 4.5 {
            high += 1;
        } else if v < 0.5 {
            low += 1;
        } else if v > 1.0 && v < 4.0 {
            mid += 1;
        }
    }
    assert!(
        high > 80 && low > 80,
        "square should visit both rails, high={high} low={low}"
    );
    assert!(
        mid < 40,
        "square must not draw as a linear ramp; {mid} samples sat between 1V and 4V"
    );
}

#[test]
fn test_wavegen_palette_triangle_not_saw() {
    let mut scene = Scene::default();
    let id = scene.add_wave_gen(0.0, 0.0);
    {
        let wg = scene.item_by_id_mut(&id).expect("WaveGen");
        assert!(wg.set_prop_text("WaveType", "Triangle"));
        assert!((wg.duty() - 0.5).abs() < 1e-12);
    }
    let item = scene.item_by_id(&id).unwrap();
    let kind = item.to_element_kind().expect("Kind::WaveGen");
    let comp = Comp {
        id: id.clone(),
        kind,
    };
    // 1 kHz default: 250 us = 0.5 on a triangle, 0.25 on a saw.
    assert!((comp.wavegen_calc_vout(250_000_000) - 0.5).abs() < 1e-3);
    assert!((comp.wavegen_calc_vout(750_000_000) - 0.5).abs() < 1e-3);
}

#[test]
fn test_wavegen_wav_playback_simulation() {
    // 1. Synthesize an in-memory 8 kHz WAV with 16-bit PCM samples
    let sample_rate = 8000u32;
    let total_samples = 80;
    let mut wav_bytes = Vec::new();
    let mut pcm_bytes = Vec::new();
    for i in 0..total_samples {
        // First 40 samples at +20000 (normalized ~0.805), next 40 samples at -20000 (normalized ~0.195)
        let sample_val = if i < 40 { 20000i16 } else { -20000i16 };
        pcm_bytes.extend_from_slice(&sample_val.to_le_bytes());
    }

    wav_bytes.extend_from_slice(b"RIFF");
    wav_bytes.extend_from_slice(&((36 + pcm_bytes.len()) as u32).to_le_bytes());
    wav_bytes.extend_from_slice(b"WAVEfmt ");
    wav_bytes.extend_from_slice(&16u32.to_le_bytes());
    wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 channel
    wav_bytes.extend_from_slice(&sample_rate.to_le_bytes());
    wav_bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav_bytes.extend_from_slice(&2u16.to_le_bytes());
    wav_bytes.extend_from_slice(&16u16.to_le_bytes());
    wav_bytes.extend_from_slice(b"data");
    wav_bytes.extend_from_slice(&(pcm_bytes.len() as u32).to_le_bytes());
    wav_bytes.extend_from_slice(&pcm_bytes);

    let wav = WavData::from_bytes(&wav_bytes).expect("Valid WAV data");
    assert_eq!(wav.samples.len(), 80);

    // 2. Setup circuit with WaveGen configured with WAV data
    let mut circ = Circuit::default();
    circ.dt = 1e-6; // 1 us step
    // Amplitude = 2.5V, Offset = 2.5V -> 0V to 5V output span
    circ.add_comp(Comp::wave_gen_with_wav_data(
        "wg_wav",
        "Wav",
        sample_rate as f64,
        2.5,
        2.5,
        0.5,
        0.0,
        100,
        false,
        false,
        "",
        Some(wav.samples.clone()),
    ));
    circ.add_comp(Comp::resistor("r1", 1000.0));
    circ.add_comp(Comp::ground("gnd"));

    circ.connect("wg_wav-outnod", "r1-lPin");
    circ.connect("r1-rPin", "gnd-Gnd");

    circ.solve().unwrap();

    // At t=0 (first block, positive sample ~0.805 -> V ~ 0 + 5.0 * 0.805 = ~4.02V)
    let v_first = circ.pin_voltage("wg_wav-outnod").unwrap();
    assert!(
        (v_first - 4.02).abs() < 0.1,
        "Expected ~4.02V first block, got {v_first}"
    );

    // Advance 5 ms (40 samples at 8 kHz = 5 ms = 5,000,000,000 ps) -> transitions to negative sample ~0.195
    circ.run_ps(5_000_000_000).unwrap();
    let v_second = circ.pin_voltage("wg_wav-outnod").unwrap();
    assert!(
        (v_second - 0.98).abs() < 0.1,
        "Expected ~0.98V second block, got {v_second}"
    );

    // Advance another 5 ms (wraps around to first block)
    circ.run_ps(5_000_000_000).unwrap();
    let v_wrap = circ.pin_voltage("wg_wav-outnod").unwrap();
    assert!(
        (v_wrap - 4.02).abs() < 0.1,
        "Expected ~4.02V wrapped block, got {v_wrap}"
    );
}

#[test]
fn test_event_driven_wavegen_speed_and_accuracy() {
    let mut circ = Circuit::default();
    // 1 kHz Square wave: 50% duty, 0 to 5V
    circ.add_comp(Comp::wave_gen(
        "wg1", "Square", 1000.0, 2.5, 2.5, 0.5, 0.0, 100, false, false,
    ));
    circ.add_comp(Comp::resistor("r1", 1000.0));
    circ.add_comp(Comp::ground("gnd"));
    circ.connect("wg1-outnod", "r1-lPin");
    circ.connect("r1-rPin", "gnd-Gnd");

    circ.solve().unwrap();
    // Initial state: t=0 -> high (5V)
    assert!((circ.pin_voltage("wg1-outnod").unwrap() - 5.0).abs() < 1e-3);

    // Advance 0.5s of simulation in 25 frames of 20ms each (standard GUI frame budget)
    let t0 = std::time::Instant::now();
    for _ in 0..25 {
        circ.run_ps_budgeted(20_000_000_000, None).unwrap(); // 20 ms
    }
    let elapsed = t0.elapsed();
    let sim_time_s = circ.circ_time as f64 / 1e12;
    let real_speed = (sim_time_s / elapsed.as_secs_f64()) * 100.0;
    println!(
        "\n[EVENT-DRIVEN WAVEGEN 1kHz Square] 0.5s simulated in {:?} -> Speed: {:.1}%",
        elapsed, real_speed
    );
    // Effective speed should be vastly faster than 100% real-time (hundreds or thousands of %)!
    assert!(
        real_speed > 200.0,
        "Expected speed > 200%, got {real_speed}%"
    );
}
