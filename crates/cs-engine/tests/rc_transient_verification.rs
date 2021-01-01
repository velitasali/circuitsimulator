use cs_engine::Circuit;
use cs_engine::elements::Comp;
use cs_engine::instruments::{SCOPE_COLORS, ScopeSampler};

#[test]
fn test_capacitor_exponential_charging_curve() {
    let mut circ = Circuit::default();
    circ.dt = 1e-6; // 1 us time step

    // RC Circuit: 5V Source + 1kΩ Resistor + 1µF Capacitor -> tau = 1 ms = 1,000,000,000 ps
    circ.add_comp(Comp::fixed_volt("rail", 5.0));
    circ.add_comp(Comp::resistor("r1", 1000.0));
    circ.add_comp(Comp::capacitor("c1", 1e-6));
    circ.add_comp(Comp::ground("gnd"));

    circ.connect("rail-outnod", "r1-lPin");
    circ.connect("r1-rPin", "c1-lPin");
    circ.connect("c1-rPin", "gnd-Gnd");

    circ.solve().expect("solve circuit");

    let tau_s = 1e-3; // 1 ms
    let tau_ps = 1_000_000_000u64; // 1 ms in ps
    let v_source = 5.0;

    let mut sim_voltages = Vec::new();
    let mut analytical_voltages = Vec::new();
    let fractions = [0.0, 0.2, 0.5, 0.7, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0];

    for &frac in &fractions {
        let target_time_ps = (frac * tau_ps as f64).round() as u64;
        if target_time_ps > circ.circ_time {
            circ.run_ps(target_time_ps - circ.circ_time).expect("run");
        }
        let v_sim = circ.pin_voltage("c1-lPin").unwrap();
        let t_s = frac * tau_s;
        let v_exact = v_source * (1.0 - (-t_s / tau_s).exp());

        sim_voltages.push(v_sim);
        analytical_voltages.push(v_exact);

        // Discretization error with Backward Euler at dt/tau = 1e-3 is < 0.01V
        let err = (v_sim - v_exact).abs();
        assert!(
            err < 0.02,
            "At t = {frac:.1} tau ({t_s:.4}s): sim = {v_sim:.4}V, exact = {v_exact:.4}V, error = {err:.4}V"
        );
    }

    // Verify non-linearity: The voltage gain per 0.5 tau must strictly decrease (concave down / exponential)
    // dV in [0, 0.5 tau] vs dV in [0.5 tau, 1.0 tau] vs dV in [1.0 tau, 1.5 tau]
    let dv1 = sim_voltages[2] - sim_voltages[0]; // [0.0 to 0.5 tau]
    let dv2 = sim_voltages[4] - sim_voltages[2]; // [0.5 to 1.0 tau]
    let dv3 = sim_voltages[5] - sim_voltages[4]; // [1.0 to 1.5 tau]

    assert!(
        dv1 > dv2 && dv2 > dv3,
        "Charging curve must be strictly non-linear (concave down): dv1={dv1:.4}, dv2={dv2:.4}, dv3={dv3:.4}"
    );

    // Verify that the second discrete derivative is negative: (v[k+1] - v[k]) < (v[k] - v[k-1])
    let d2v = (sim_voltages[4] - sim_voltages[2]) - (sim_voltages[2] - sim_voltages[0]);
    assert!(
        d2v < -0.2,
        "Second difference must be significantly negative, confirming exponential curvature: d2v = {d2v}"
    );
}

#[test]
fn test_capacitor_exponential_discharging_curve() {
    let mut circ = Circuit::default();
    circ.dt = 1e-6; // 1 us time step

    // RC Circuit: 1kΩ Resistor + 1µF Capacitor, charged initially to 5V
    circ.add_comp(Comp::resistor("r1", 1000.0));
    let mut cap = Comp::capacitor("c1", 1e-6);
    if let cs_engine::elements::Kind::Capacitor { ref mut volt, .. } = cap.kind {
        *volt = 5.0;
    }
    circ.add_comp(cap);
    circ.add_comp(Comp::ground("gnd"));

    circ.connect("r1-rPin", "c1-lPin");
    circ.connect("r1-lPin", "gnd-Gnd");
    circ.connect("c1-rPin", "gnd-Gnd");

    circ.solve().expect("solve circuit");

    let tau_s = 1e-3; // 1 ms
    let tau_ps = 1_000_000_000u64; // 1 ms in ps
    let v_init = 5.0;

    let mut sim_voltages = Vec::new();
    let fractions = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0];

    for &frac in &fractions {
        let target_time_ps = (frac * tau_ps as f64).round() as u64;
        if target_time_ps > circ.circ_time {
            circ.run_ps(target_time_ps - circ.circ_time).expect("run");
        }
        let v_sim = circ.pin_voltage("c1-lPin").unwrap();
        let t_s = frac * tau_s;
        let v_exact = v_init * (-t_s / tau_s).exp();

        sim_voltages.push(v_sim);

        let err = (v_sim - v_exact).abs();
        assert!(
            err < 0.02,
            "At t = {frac:.1} tau ({t_s:.4}s): sim = {v_sim:.4}V, exact = {v_exact:.4}V, error = {err:.4}V"
        );
    }

    // Verify non-linearity: The voltage drop per 0.5 tau must strictly decrease (convex / exponential decay)
    let drop1 = (sim_voltages[0] - sim_voltages[1]).abs(); // [0.0 to 0.5 tau]
    let drop2 = (sim_voltages[1] - sim_voltages[2]).abs(); // [0.5 to 1.0 tau]
    let drop3 = (sim_voltages[2] - sim_voltages[3]).abs(); // [1.0 to 1.5 tau]

    assert!(
        drop1 > drop2 && drop2 > drop3,
        "Discharge curve must be strictly non-linear (convex): drop1={drop1:.4}, drop2={drop2:.4}, drop3={drop3:.4}"
    );
}

#[test]
fn test_oscope_sampling_fidelity_rc_charging() {
    let mut sampler = ScopeSampler::scope();
    let tau = 1e-3; // 1 ms
    let time_div = 2e-4; // 200 µs/div -> 10 divs = 2 ms total display window (2 tau)

    // Feed an analytical RC exponential waveform into the oscilloscope sampler
    let dt = 1e-6; // 1 µs simulation steps
    let total_steps = 3000; // 3 ms

    for step in 0..total_steps {
        let t = step as f64 * dt;
        let v_c = 5.0 * (1.0 - (-t / tau).exp());
        sampler.push(
            t,
            &[v_c, 0.0, 0.0, 0.0],
            &[true, false, false, false],
            -1,
            0.0,
            true,
            0.1,
        );
    }

    // Extract 512 screen display samples over the 2 ms window
    let plot = sampler.traces_with_count(time_div, 0.0, -1, 0.0, &SCOPE_COLORS, None, 512);

    let ch0 = &plot.channels[0];
    assert!(ch0.connected);
    assert_eq!(ch0.samples.len(), 512);

    // Verify that sampled points on the screen follow the non-linear exponential shape
    let s_start = ch0.samples[0];
    let s_mid = ch0.samples[256];
    let s_end = ch0.samples[511];

    // Total window is 2 ms, so start is at t=1ms (1 tau) and end is at t=3ms (3 tau)
    // s_start should be ~ 5*(1 - e^-1) = 3.16V
    // s_end should be ~ 5*(1 - e^-3) = 4.75V
    assert!(
        s_end > s_mid && s_mid > s_start,
        "Trace must be strictly increasing: start={s_start}, mid={s_mid}, end={s_end}"
    );

    // Check that the slope is decreasing across the screen:
    let delta1 = s_mid - s_start;
    let delta2 = s_end - s_mid;
    assert!(
        delta1 > delta2,
        "Oscilloscope trace must preserve exponential curvature without linear flattening: delta1={delta1:.4}, delta2={delta2:.4}"
    );
}
