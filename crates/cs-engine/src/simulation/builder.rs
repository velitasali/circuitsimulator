//! Component builder methods, configuration, SIM1 loading, and test batch runner.

use rustc_hash::FxHashMap;
use std::path::Path;

use crate::Result;
use crate::digital::{EventQueue, EventTarget, FlipFlopState, GateState, LatchState};
use crate::elements::{Comp, Kind};
use crate::instruments::InstrumentState;
use crate::matrix::CircMatrix;
use crate::sim1::parse_sim1;
use crate::subcircuit::{SubcSearch, expand_parsed};

use super::Circuit;

impl Circuit {
    pub fn add_resistor(&mut self, id: impl Into<String>, ohms: f64) -> &mut Self {
        self.components.push(Comp::resistor(id, ohms));
        self
    }

    pub fn add_battery(&mut self, id: impl Into<String>, volts: f64, ohms: f64) -> &mut Self {
        self.components.push(Comp::battery(id, volts, ohms));
        self
    }

    pub fn add_ground(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::ground(id));
        self
    }

    pub fn add_fixed_volt(&mut self, id: impl Into<String>, volts: f64) -> &mut Self {
        self.components.push(Comp::fixed_volt(id, volts));
        self
    }

    pub fn add_node(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::junction(id));
        self
    }

    pub fn add_capacitor(&mut self, id: impl Into<String>, farads: f64) -> &mut Self {
        self.components.push(Comp::capacitor(id, farads));
        self
    }

    pub fn add_el_capacitor(&mut self, id: impl Into<String>, farads: f64) -> &mut Self {
        self.components.push(Comp::el_capacitor(id, farads));
        self
    }

    pub fn add_inductor(&mut self, id: impl Into<String>, henries: f64) -> &mut Self {
        self.components.push(Comp::inductor(id, henries));
        self
    }

    pub fn add_switch(&mut self, id: impl Into<String>, closed: bool) -> &mut Self {
        self.components.push(Comp::switch(id, closed));
        self
    }

    pub fn add_switch_with(
        &mut self,
        id: impl Into<String>,
        closed: bool,
        poles: usize,
        double_throw: bool,
    ) -> &mut Self {
        self.components
            .push(Comp::switch_full(id, closed, poles, double_throw));
        self
    }

    pub fn add_diode(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::diode(id));
        self
    }

    pub fn add_zener(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::zener(id));
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_diode_with(
        &mut self,
        id: impl Into<String>,
        zener: bool,
        threshold: f64,
        max_current: f64,
        resistance: f64,
        brkdown_v: f64,
        sat_current: f64,
        em_coef: f64,
    ) -> &mut Self {
        let mut comp = if zener {
            Comp::zener(id)
        } else {
            Comp::diode(id)
        };
        if let Kind::Diode { ref mut state, .. } = comp.kind {
            state.threshold = threshold;
            state.max_current = max_current;
            state.series_r = resistance;
            state.bk_down = brkdown_v;
            state.sat_cur = sat_current;
            state.em_coef = em_coef;
        }
        self.components.push(comp);
        self
    }

    pub fn add_led(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::led(id));
        self
    }

    pub fn add_bjt(&mut self, id: impl Into<String>, pnp: bool) -> &mut Self {
        self.components.push(Comp::bjt(id, pnp));
        self
    }

    pub fn add_bjt_with(
        &mut self,
        id: impl Into<String>,
        pnp: bool,
        gain: f64,
        threshold: f64,
    ) -> &mut Self {
        let mut comp = Comp::bjt(id, pnp);
        if let Kind::Bjt { ref mut state } = comp.kind {
            state.set_gain(gain);
            state.set_threshold(threshold);
        }
        self.components.push(comp);
        self
    }

    pub fn add_mosfet(
        &mut self,
        id: impl Into<String>,
        p_channel: bool,
        depletion: bool,
    ) -> &mut Self {
        self.components.push(Comp::mosfet(id, p_channel, depletion));
        self
    }

    pub fn add_mosfet_with(
        &mut self,
        id: impl Into<String>,
        p_channel: bool,
        depletion: bool,
        rdson: f64,
        threshold: f64,
    ) -> &mut Self {
        let mut comp = Comp::mosfet(id, p_channel, depletion);
        if let Kind::Mosfet { ref mut state } = comp.kind {
            state.set_rdson(rdson);
            state.set_threshold(threshold);
        }
        self.components.push(comp);
        self
    }

    pub fn add_opamp(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::opamp(id));
        self
    }

    pub fn add_jfet(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::jfet(id));
        self
    }

    pub fn add_comparator(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::comparator(id));
        self
    }

    pub fn add_volt_reg(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::volt_reg(id));
        self
    }

    pub fn add_scr(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::scr(id));
        self
    }

    pub fn add_triac(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::triac(id));
        self
    }

    pub fn add_diac(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::diac(id));
        self
    }

    pub fn add_analog_mux(&mut self, id: impl Into<String>, channels: usize) -> &mut Self {
        self.components.push(Comp::analog_mux(id, channels));
        self
    }

    pub fn add_potentiometer(
        &mut self,
        id: impl Into<String>,
        resistance: f64,
        wiper: f64,
    ) -> &mut Self {
        self.components
            .push(Comp::potentiometer(id, resistance, wiper));
        self
    }

    pub fn add_touchpad(
        &mut self,
        id: impl Into<String>,
        width: i32,
        height: i32,
        rx_min: f64,
        rx_max: f64,
        ry_min: f64,
        ry_max: f64,
        x_pos: i32,
        y_pos: i32,
    ) -> &mut Self {
        self.components.push(Comp::touchpad(
            id, width, height, rx_min, rx_max, ry_min, ry_max, x_pos, y_pos,
        ));
        self
    }

    pub fn add_ky023(
        &mut self,
        id: impl Into<String>,
        stick_x: f64,
        stick_y: f64,
        btn_down: bool,
    ) -> &mut Self {
        self.components
            .push(Comp::ky023(id, stick_x, stick_y, btn_down));
        self
    }

    pub fn add_ky040(
        &mut self,
        id: impl Into<String>,
        steps: u32,
        dial_val: i32,
        btn_closed: bool,
        state_a: bool,
        state_b: bool,
    ) -> &mut Self {
        self.components.push(Comp::ky040(
            id, steps, dial_val, btn_closed, state_a, state_b,
        ));
        self
    }

    pub fn add_sr04(
        &mut self,
        id: impl Into<String>,
        distance: f64,
        use_slider: bool,
    ) -> &mut Self {
        self.components.push(Comp::sr04(id, distance, use_slider));
        self
    }

    pub fn add_dht22(
        &mut self,
        id: impl Into<String>,
        model: impl Into<String>,
        temp: f64,
        humi: f64,
    ) -> &mut Self {
        self.components.push(Comp::dht22(id, model, temp, humi));
        self
    }

    pub fn add_ds18b20(
        &mut self,
        id: impl Into<String>,
        rom: impl Into<String>,
        temp: f64,
    ) -> &mut Self {
        self.components.push(Comp::ds18b20(id, rom, temp));
        self
    }

    pub fn add_ds1621(&mut self, id: impl Into<String>, temp: f64) -> &mut Self {
        self.components.push(Comp::ds1621(id, temp));
        self
    }

    pub fn add_ds1307(&mut self, id: impl Into<String>, time_updated: bool) -> &mut Self {
        self.components.push(Comp::ds1307(id, time_updated));
        self
    }

    pub fn add_dcmotor(
        &mut self,
        id: impl Into<String>,
        rpm_nominal: i32,
        volt_nominal: f64,
        resistance: f64,
    ) -> &mut Self {
        self.components
            .push(Comp::dcmotor(id, rpm_nominal, volt_nominal, resistance));
        self
    }

    pub fn add_stepper(
        &mut self,
        id: impl Into<String>,
        bipolar: bool,
        steps: i32,
        resistance: f64,
    ) -> &mut Self {
        self.components
            .push(Comp::stepper(id, bipolar, steps, resistance));
        self
    }

    pub fn add_servo(
        &mut self,
        id: impl Into<String>,
        speed: f64,
        min_pulse: f64,
        max_pulse: f64,
    ) -> &mut Self {
        self.components
            .push(Comp::servo(id, speed, min_pulse, max_pulse));
        self
    }

    pub fn add_sdcard(&mut self, id: impl Into<String>, file: impl Into<String>) -> &mut Self {
        self.components.push(Comp::sdcard(id, file));
        self
    }

    pub fn add_esp01(&mut self, id: impl Into<String>, baud_rate: u32, debug: bool) -> &mut Self {
        self.components.push(Comp::esp01(id, baud_rate, debug));
        self
    }

    pub fn add_probe(&mut self, id: impl Into<String>) -> &mut Self {
        self.add_probe_with(id, crate::instruments::PROBE_DEFAULT_THRESHOLD, false)
    }

    pub fn add_probe_with(
        &mut self,
        id: impl Into<String>,
        threshold: f64,
        small: bool,
    ) -> &mut Self {
        self.components.push(Comp::probe(id, threshold, small));
        self
    }

    pub fn add_voltmeter(&mut self, id: impl Into<String>, rms: bool) -> &mut Self {
        self.components.push(Comp::voltmeter(id, rms));
        self
    }

    pub fn add_ammeter(&mut self, id: impl Into<String>, rms: bool) -> &mut Self {
        self.components.push(Comp::ammeter(id, rms));
        self
    }

    pub fn add_audio_out(&mut self, id: impl Into<String>, impedance: f64) -> &mut Self {
        self.components.push(Comp::audio_out(id, impedance));
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_wave_gen(
        &mut self,
        id: impl Into<String>,
        wave_type: impl Into<String>,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
        phase: f64,
        steps: i32,
        bipolar: bool,
        floating: bool,
    ) -> &mut Self {
        self.components.push(Comp::wave_gen(
            id, wave_type, freq_hz, amplitude, offset, duty, phase, steps, bipolar, floating,
        ));
        self
    }

    pub fn add_freq_meter(&mut self, id: impl Into<String>) -> &mut Self {
        self.add_freq_meter_with(id, 0.1)
    }

    pub fn add_freq_meter_with(&mut self, id: impl Into<String>, filter: f64) -> &mut Self {
        self.components.push(Comp::freq_meter(id, filter));
        self
    }

    pub fn add_oscope(&mut self, id: impl Into<String>) -> &mut Self {
        self.add_oscope_with(id, true)
    }

    pub fn add_oscope_with(&mut self, id: impl Into<String>, connect_gnd: bool) -> &mut Self {
        self.components.push(Comp::oscope(id, connect_gnd));
        self
    }

    pub fn add_oscope_full(
        &mut self,
        id: impl Into<String>,
        connect_gnd: bool,
        input_imped: f64,
        tunnels: [String; 4],
    ) -> &mut Self {
        self.components
            .push(Comp::oscope_full(id, connect_gnd, input_imped, tunnels));
        self
    }

    pub fn add_lanalizer(&mut self, id: impl Into<String>) -> &mut Self {
        self.add_lanalizer_with(id, true)
    }

    pub fn add_lanalizer_with(&mut self, id: impl Into<String>, connect_gnd: bool) -> &mut Self {
        self.components.push(Comp::lanalizer(id, connect_gnd));
        self
    }

    pub fn add_lanalizer_full(
        &mut self,
        id: impl Into<String>,
        connect_gnd: bool,
        input_imped: f64,
        tunnels: [String; 8],
    ) -> &mut Self {
        self.components
            .push(Comp::lanalizer_full(id, connect_gnd, input_imped, tunnels));
        self
    }

    pub fn add_and(&mut self, id: impl Into<String>, n_inputs: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::and(&id, n_inputs)));
        self
    }

    pub fn add_nand(&mut self, id: impl Into<String>, n_inputs: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::nand(&id, n_inputs)));
        self
    }

    pub fn add_or(&mut self, id: impl Into<String>, n_inputs: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::or(&id, n_inputs)));
        self
    }

    pub fn add_nor(&mut self, id: impl Into<String>, n_inputs: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::nor(&id, n_inputs)));
        self
    }

    pub fn add_xor(&mut self, id: impl Into<String>, n_inputs: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::xor(&id, n_inputs)));
        self
    }

    pub fn add_xnor(&mut self, id: impl Into<String>, n_inputs: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::xnor(&id, n_inputs)));
        self
    }

    pub fn add_buffer(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::buffer(&id)));
        self
    }

    pub fn add_inverter(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::gate(id.clone(), GateState::inverter(&id)));
        self
    }

    pub fn add_flipflop_d(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::flipflop(id.clone(), FlipFlopState::d(&id)));
        self
    }

    pub fn add_flipflop_jk(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::flipflop(id.clone(), FlipFlopState::jk(&id)));
        self
    }

    pub fn add_flipflop_rs(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::flipflop(id.clone(), FlipFlopState::rs(&id)));
        self
    }

    pub fn add_flipflop_t(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::flipflop(id.clone(), FlipFlopState::t(&id)));
        self
    }

    pub fn add_latch(&mut self, id: impl Into<String>, channels: usize) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::latch(id.clone(), LatchState::new(&id, channels)));
        self
    }

    pub fn add_mcu_pin(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::mcu_pin(id));
        self
    }

    pub fn add_mcu(&mut self, id: impl Into<String>, device: cs_mcu::Device) -> &mut Self {
        self.components.push(Comp::mcu(id, device));
        self
    }

    pub fn add_qemu_esp32(&mut self, id: impl Into<String>) -> &mut Self {
        let id = id.into();
        self.components
            .push(Comp::qemu(id.clone(), crate::qemu::QemuComp::esp32(&id)));
        self
    }

    pub fn add_qemu_stm32(&mut self, id: impl Into<String>, port_n: u8) -> &mut Self {
        let id = id.into();
        self.components.push(Comp::qemu(
            id.clone(),
            crate::qemu::QemuComp::stm32(&id, port_n),
        ));
        self
    }

    pub fn add_script_cpu(
        &mut self,
        id: impl Into<String>,
        cpu: crate::script::ScriptCpu,
    ) -> &mut Self {
        self.components.push(Comp::script_cpu(id, cpu));
        self
    }

    pub fn components(&self) -> &[Comp] {
        &self.components
    }

    pub fn components_mut(&mut self) -> &mut [Comp] {
        &mut self.components
    }

    pub fn comp(&self, id: &str) -> Option<&Comp> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn comp_mut(&mut self, id: &str) -> Option<&mut Comp> {
        self.components.iter_mut().find(|c| c.id == id)
    }

    pub fn add_test_unit(&mut self, id: impl Into<String>) -> &mut Self {
        self.components.push(Comp::test_unit(id));
        self
    }

    pub fn add_tunnel(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pin_id: impl Into<String>,
    ) -> &mut Self {
        self.components.push(Comp::tunnel(id, name, pin_id));
        self
    }

    pub fn add_comp(&mut self, comp: Comp) -> &mut Self {
        self.components.push(comp);
        self
    }

    /// Set analog parameters on the LED with `id`.
    pub fn configure_led(
        &mut self,
        id: &str,
        threshold: f64,
        max_current: f64,
        resistance: f64,
        grounded: bool,
    ) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::Led { ref mut state } = c.kind {
                    state.threshold = threshold;
                    state.max_current = max_current;
                    state.impedance = resistance.max(1e-3);
                    state.grounded = grounded;
                    return true;
                }
            }
        }
        false
    }

    /// Set analog parameters on the op-amp with `id` (C++ property set).
    pub fn configure_opamp(
        &mut self,
        id: &str,
        gain: f64,
        out_imp: f64,
        volt_pos: f64,
        volt_neg: f64,
        power_pins: bool,
    ) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::OpAmp { ref mut state } = c.kind {
                    state.set_gain(gain);
                    state.set_out_imp(out_imp);
                    state.volt_pos = volt_pos;
                    state.volt_neg = volt_neg;
                    state.power_pins = power_pins;
                    return true;
                }
            }
        }
        false
    }

    /// Set analog parameters on the JFET with `id` (C++ property set).
    pub fn configure_jfet(&mut self, id: &str, idss: f64, vp: f64, lambda_inv: f64) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::Jfet { ref mut state } = c.kind {
                    state.set_idss(idss);
                    state.set_vp(vp);
                    state.set_lambda_inv(lambda_inv);
                    return true;
                }
            }
        }
        false
    }

    /// Set IoPin output properties on the comparator with `id`.
    pub fn configure_comparator(
        &mut self,
        id: &str,
        out_high: f64,
        out_low: f64,
        out_imp: f64,
        inverted: bool,
        open_col: bool,
    ) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::Comparator { ref mut state } = c.kind {
                    state.family.out_high_v = out_high;
                    state.family.out_low_v = out_low;
                    state.set_out_imp(out_imp);
                    state.inverted = inverted;
                    state.open_col = open_col;
                    state.apply_electric();
                    return true;
                }
            }
        }
        false
    }

    pub fn configure_test_unit(
        &mut self,
        id: &str,
        inputs: &str,
        outputs: &str,
        period: f64,
        truth: &[u32],
    ) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::TestUnit(t) = &mut c.kind {
                    t.set_inputs(id, inputs);
                    t.set_outputs(id, outputs);
                    t.set_period(period);
                    t.truth.clear();
                    t.truth.extend_from_slice(truth);
                    let n = t.steps() as usize;
                    if t.truth.len() < n {
                        t.truth.resize(n, 0);
                    }
                    t.samples.resize(n, 0);
                    return true;
                }
            }
        }
        false
    }

    /// Arm TestUnits and run until they finish or `timeout_ps` elapses.
    ///
    /// C++ `BatchTest` + `TestUnit::stamp` while `BatchTest::isRunning()`.
    pub fn run_batch(&mut self) -> Result<Vec<crate::digital::TestResult>> {
        self.solve()?;
        let slope = self.slope_steps;
        let mut armed: Vec<(usize, u64)> = Vec::new();
        let mut timeout = self.analog_ps().saturating_mul(100).max(1_000_000);
        for (i, c) in self.components.iter_mut().enumerate() {
            if let Kind::TestUnit(t) = &mut c.kind {
                t.stamp_init(slope);
                let half = t.half_period_ps();
                timeout = timeout.max(t.duration_ps().saturating_mul(4));
                armed.push((i, half));
            }
        }
        if armed.is_empty() {
            return Ok(Vec::new());
        }
        for (i, half) in armed {
            self.events
                .add(self.circ_time.saturating_add(half), EventTarget::Device(i));
        }
        if !self.sim_started {
            self.sim_started = true;
            self.events.add(
                self.circ_time.saturating_add(self.analog_ps()),
                EventTarget::AnalogClock,
            );
        }
        let end = self.circ_time.saturating_add(timeout);
        while self.circ_time < end && self.batch_pending() {
            let next = self.events.peek_time().unwrap_or(end).min(end);
            if next <= self.circ_time {
                break;
            }
            self.run_until(next)?;
        }
        Ok(self.collect_test_results())
    }

    pub(super) fn batch_pending(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(
                &c.kind,
                Kind::TestUnit(t) if !t.done
            )
        })
    }

    pub(super) fn collect_test_results(&self) -> Vec<crate::digital::TestResult> {
        self.components
            .iter()
            .filter_map(|c| match &c.kind {
                Kind::TestUnit(t) => Some(crate::digital::TestResult {
                    id: c.id.clone(),
                    ok: t.done && t.ok,
                }),
                _ => None,
            })
            .collect()
    }

    /// Set C++ `Voltage` on the regulator with `id`.
    pub fn configure_volt_reg(&mut self, id: &str, voltage: f64) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::VoltReg { ref mut state } = c.kind {
                    state.set_out_volt(voltage);
                    return true;
                }
            }
        }
        false
    }

    pub fn set_switch(&mut self, id: &str, closed: bool) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::Switch {
                    closed: ref mut s, ..
                } = c.kind
                {
                    *s = closed;
                    return true;
                }
            }
        }
        false
    }

    pub fn connect(&mut self, a: impl Into<String>, b: impl Into<String>) -> &mut Self {
        self.connectors.push((a.into(), b.into()));
        self
    }

    pub fn from_sim1(src: &str) -> Result<Self> {
        Self::from_sim1_with(src, &SubcSearch::default())
    }

    pub fn from_sim1_with(src: &str, search: &SubcSearch) -> Result<Self> {
        let parsed = parse_sim1(src)?;
        let dt = parsed.analog_dt;
        let max_nl_steps = parsed.max_nl_steps;
        let ps_per_sec = parsed.circ.ps_per_sec();
        let expanded = expand_parsed(parsed, search, 0)?;
        Ok(Self {
            components: expanded.components,
            connectors: expanded.connectors,
            nodes: Vec::new(),
            pin_net: FxHashMap::default(),
            matrix: CircMatrix::default(),
            volts_buf: Vec::new(),
            pin_caches: Vec::new(),
            skipped: expanded.skipped,
            dt,
            time: 0.0,
            max_nl_steps,
            instruments: InstrumentState::default(),
            circ_time: 1,
            slope_steps: 0,
            events: EventQueue::new(),
            sim_started: false,
            mcu_actions_buf: Vec::new(),
            audio_sink: None,
            ps_per_sec,
            last_motor_ps: 1,
            nodes_dirty: true,
            digital_updates_buf: Vec::new(),
            has_instruments_cached: true,
            has_digital_cached: true,
            has_nonlinear_cached: true,
            instrument_caches: Vec::new(),
            has_motors_cached: false,
            time_sources_cached: Vec::new(),
        })
    }

    pub fn load_sim1_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path)?;
        let search = SubcSearch::from_circuit_path(path.to_str()).with_standard_catalog();
        Self::from_sim1_with(&src, &search)
    }

    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
