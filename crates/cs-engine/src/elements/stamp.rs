//! Matrix stamping routines and companion model numerical stamping helpers.

use super::*;
use crate::CERO_DOUB;
use crate::elements::diode::LedState;
use crate::net::ENode;

#[inline]
fn stamp_cached_two_terminal(
    n0: Option<usize>,
    n1: Option<usize>,
    nodes: &mut [ENode],
    g: f64,
    i_src: f64,
) {
    let (Some(n0), Some(n1)) = (n0, n1) else {
        return;
    };
    if n0 == n1 {
        return;
    }
    nodes[n0].add_admit(Some(n1), g);
    nodes[n1].add_admit(Some(n0), g);
    if i_src != 0.0 {
        nodes[n0].add_current(i_src);
        nodes[n1].add_current(-i_src);
    }
}

#[inline]
fn stamp_cached_admit_directed(
    from: Option<usize>,
    to: Option<usize>,
    nodes: &mut [ENode],
    g: f64,
) {
    let (Some(f), Some(t)) = (from, to) else {
        return;
    };
    if f == t {
        return;
    }
    nodes[f].add_admit(Some(t), g);
}

#[inline]
fn stamp_cached_current_at(node: Option<usize>, nodes: &mut [ENode], i: f64) {
    if let Some(n) = node {
        if i != 0.0 {
            nodes[n].add_current(i);
        }
    }
}

#[inline]
fn stamp_cached_pin_to_gnd(node: Option<usize>, nodes: &mut [ENode], volt: f64, admit: f64) {
    if let Some(n) = node {
        nodes[n].add_admit(None, admit);
        if volt != 0.0 {
            nodes[n].add_current(volt * admit);
        }
    }
}

/// C++ `eLed` + `LedBase::m_gndEnode`: grounded cathode is always 0 V and is
/// not in the matrix, so the Norton pair stamps anode to implicit ground.
#[inline]
fn stamp_led(cache: &PinCache, nodes: &mut [ENode], state: &LedState) {
    if state.grounded {
        let Some(n0) = cache.left else {
            return;
        };
        nodes[n0].add_admit(None, state.admit);
        if state.th_current != 0.0 {
            nodes[n0].add_current(state.th_current);
        }
    } else {
        stamp_cached_two_terminal(
            cache.left,
            cache.right,
            nodes,
            state.admit,
            state.th_current,
        );
    }
}

impl Comp {
    pub fn stamp_with_cache(
        &self,
        cache: &PinCache,
        pin_net: &rustc_hash::FxHashMap<String, usize>,
        nodes: &mut [ENode],
        dt: f64,
    ) {
        match &self.kind {
            Kind::Resistor { resistance } => {
                let g = 1.0 / resistance;
                stamp_cached_two_terminal(cache.left, cache.right, nodes, g, 0.0);
            }
            Kind::Battery {
                voltage,
                resistance,
            } => {
                let g = 1.0 / resistance;
                stamp_cached_two_terminal(cache.left, cache.right, nodes, g, voltage * g);
            }
            Kind::Ground => {
                if let Some(n) = cache.gnd {
                    nodes[n].add_admit(None, SOURCE_ADMIT);
                    nodes[n].add_current(CERO_DOUB * SOURCE_ADMIT);
                }
            }
            Kind::FixedVolt { voltage } => {
                if let Some(n) = cache.out {
                    nodes[n].add_admit(None, SOURCE_ADMIT);
                    nodes[n].add_current(*voltage * SOURCE_ADMIT);
                }
            }
            Kind::Rail { voltage } => {
                stamp_cached_pin_to_gnd(cache.out, nodes, *voltage, SOURCE_ADMIT);
            }
            Kind::Clock { voltage, state, .. } => {
                let v = if *state { *voltage } else { 0.0 };
                stamp_cached_pin_to_gnd(cache.out, nodes, v, SOURCE_ADMIT);
            }
            Kind::Junction => {}
            Kind::Capacitor { capacitance, volt } | Kind::ElCapacitor { capacitance, volt } => {
                if dt > 0.0 {
                    let g = capacitance / dt;
                    stamp_cached_two_terminal(cache.left, cache.right, nodes, g, volt * g);
                }
            }
            Kind::Inductor { inductance, ieq } => {
                if dt > 0.0 {
                    let g = dt / inductance;
                    stamp_cached_two_terminal(cache.left, cache.right, nodes, g, *ieq);
                }
            }
            Kind::Switch {
                closed,
                double_throw,
                ..
            } => {
                if *double_throw {
                    for &(p, n0, n1) in &cache.switch_poles {
                        let target = if *closed { n0 } else { n1 };
                        stamp_cached_two_terminal(p, target, nodes, SWITCH_CLOSED_ADMIT, 0.0);
                    }
                } else if *closed {
                    if cache.switch_poles.len() <= 1 {
                        stamp_cached_two_terminal(
                            cache.left,
                            cache.right,
                            nodes,
                            SWITCH_CLOSED_ADMIT,
                            0.0,
                        );
                    } else {
                        for &(p, n0, _) in &cache.switch_poles {
                            stamp_cached_two_terminal(p, n0, nodes, SWITCH_CLOSED_ADMIT, 0.0);
                        }
                    }
                }
            }
            Kind::Potentiometer { resistance, wiper } => {
                let r1 = (*wiper * resistance).max(RESISTOR_MIN_OHMS);
                let r2 = ((1.0 - wiper) * resistance).max(RESISTOR_MIN_OHMS);
                stamp_cached_two_terminal(cache.left, cache.mid, nodes, 1.0 / r1, 0.0);
                stamp_cached_two_terminal(cache.mid, cache.right, nodes, 1.0 / r2, 0.0);
            }
            Kind::Diode { state, .. } => {
                if let (Some(l), Some(m), Some(r)) = (cache.left, cache.mid, cache.right) {
                    stamp_cached_two_terminal(Some(l), Some(m), nodes, state.admit, state.i_src());
                    stamp_cached_two_terminal(Some(m), Some(r), nodes, 1.0 / state.series_r, 0.0);
                } else {
                    self.stamp(pin_net, nodes, dt);
                }
            }
            Kind::Led { state } => stamp_led(cache, nodes, state),
            Kind::Bjt { state } => {
                let c = cache.collector;
                let e = cache.emitter;
                let b = cache.base;
                stamp_cached_admit_directed(b, c, nodes, -state.gec - state.gcc);
                stamp_cached_admit_directed(c, b, nodes, -state.gce - state.gcc);
                stamp_cached_admit_directed(b, e, nodes, -state.gee - state.gce);
                stamp_cached_admit_directed(e, b, nodes, -state.gee - state.gec);
                stamp_cached_admit_directed(c, e, nodes, state.gce);
                stamp_cached_admit_directed(e, c, nodes, state.gec);
                stamp_cached_current_at(b, nodes, state.i_base);
                stamp_cached_current_at(c, nodes, state.i_coll);
                stamp_cached_current_at(e, nodes, state.i_emit);
            }
            Kind::Mosfet { state } => {
                stamp_cached_two_terminal(
                    cache.drain,
                    cache.source,
                    nodes,
                    state.admit,
                    state.current,
                );
            }
            Kind::OpAmp { state } => {
                stamp_cached_pin_to_gnd(cache.output_amp, nodes, state.last_out, state.admit());
            }
            Kind::Jfet { state } => {
                stamp_cached_two_terminal(cache.drain, cache.source, nodes, state.admit, 0.0);
            }
            Kind::Comparator { state } => {
                stamp_iopin(pin_net, nodes, &state.output);
            }
            Kind::VoltReg { state } => {
                stamp_cached_two_terminal(
                    cache.voltreg_in,
                    cache.voltreg_out,
                    nodes,
                    state.admit,
                    state.last_current,
                );
            }
            Kind::Probe { .. } => {
                stamp_cached_pin_to_gnd(cache.probe, nodes, 0.0, crate::instruments::PROBE_ADMIT);
            }
            Kind::Voltmeter { last_out, .. } => {
                stamp_cached_two_terminal(
                    cache.left,
                    cache.right,
                    nodes,
                    1.0 / crate::instruments::VOLTMETER_OHMS,
                    0.0,
                );
                stamp_cached_pin_to_gnd(cache.out, nodes, *last_out, SOURCE_ADMIT);
            }
            Kind::Ammeter { last_out, .. } => {
                stamp_cached_two_terminal(
                    cache.left,
                    cache.right,
                    nodes,
                    1.0 / crate::instruments::AMMETER_OHMS,
                    0.0,
                );
                stamp_cached_pin_to_gnd(cache.out, nodes, *last_out, SOURCE_ADMIT);
            }
            Kind::FreqMeter { .. } => {}
            Kind::Oscope {
                connect_gnd,
                input_imped,
                ..
            } => {
                if *connect_gnd {
                    let admit = if *input_imped > 0.0 {
                        1.0 / (*input_imped * 1e6)
                    } else {
                        crate::instruments::PLOT_INPUT_ADMIT
                    };
                    for &p in &cache.oscope_pins {
                        if let Some(n) = p {
                            nodes[n].add_admit(None, admit);
                        }
                    }
                    if let Some(n) = cache.oscope_gnd {
                        nodes[n].add_admit(None, admit);
                    }
                }
            }
            Kind::LAnalizer {
                connect_gnd,
                input_imped,
                ..
            } => {
                if *connect_gnd {
                    let admit = if *input_imped > 0.0 {
                        1.0 / (*input_imped * 1e6)
                    } else {
                        crate::instruments::PLOT_INPUT_ADMIT
                    };
                    for &p in &cache.la_pins {
                        if let Some(n) = p {
                            nodes[n].add_admit(None, admit);
                        }
                    }
                }
            }
            Kind::WaveGen {
                amplitude,
                offset,
                bipolar,
                floating,
                v_out,
                ..
            } => {
                if *bipolar {
                    let volt = 2.0 * *amplitude * (*v_out - 0.5);
                    if *floating {
                        stamp_cached_two_terminal(
                            cache.out,
                            cache.gnd,
                            nodes,
                            SOURCE_ADMIT,
                            volt * SOURCE_ADMIT,
                        );
                    } else {
                        let half_v = volt / 2.0;
                        if let Some(n) = cache.out {
                            nodes[n].add_admit(None, SOURCE_ADMIT);
                            nodes[n].add_current((*offset + half_v) * SOURCE_ADMIT);
                        }
                        if let Some(n) = cache.gnd {
                            nodes[n].add_admit(None, SOURCE_ADMIT);
                            nodes[n].add_current((*offset - half_v) * SOURCE_ADMIT);
                        }
                    }
                } else {
                    let volt_base = *offset - *amplitude;
                    let v = volt_base + 2.0 * *amplitude * *v_out;
                    if let Some(n) = cache.out {
                        nodes[n].add_admit(None, SOURCE_ADMIT);
                        nodes[n].add_current(v * SOURCE_ADMIT);
                    }
                }
            }
            Kind::Gate(g) => {
                for p in g.inputs.iter().chain(g.oe.as_ref()) {
                    stamp_iopin(pin_net, nodes, p);
                }
                stamp_iopin(pin_net, nodes, &g.output);
            }
            Kind::FlipFlop(f) => stamp_flipflop(pin_net, nodes, f),
            Kind::Latch(l) => {
                for p in l
                    .inputs
                    .iter()
                    .chain(l.outputs.iter())
                    .chain([&l.clk, &l.reset, &l.oe])
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::McuPin(p) => stamp_iopin(pin_net, nodes, &p.pin),
            Kind::Mcu(m) => {
                for p in &m.pins {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::QemuDevice(q) => {
                for p in &q.pins {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::ScriptCpu(s) => {
                for p in s
                    .pins
                    .iter()
                    .chain(s.ports.iter().flat_map(|po| po.pins.iter()))
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::TestUnit(tu) => {
                for p in tu.drive.iter().chain(tu.sense.iter()) {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::AudioOut { source, .. } => {
                stamp_cached_two_terminal(
                    cache.left,
                    cache.right,
                    nodes,
                    source.stamp_admit(),
                    0.0,
                );
            }
            _ => {
                self.stamp(pin_net, nodes, dt);
            }
        }
    }

    pub fn stamp(
        &self,
        pin_net: &rustc_hash::FxHashMap<String, usize>,
        nodes: &mut [ENode],
        dt: f64,
    ) {
        match &self.kind {
            Kind::Resistor { resistance } => {
                let g = 1.0 / resistance;
                stamp_two_terminal(self, pin_net, nodes, g, 0.0);
            }
            Kind::Battery {
                voltage,
                resistance,
            } => {
                let g = 1.0 / resistance;
                stamp_two_terminal(self, pin_net, nodes, g, voltage * g);
            }
            Kind::Ground => stamp_to_gnd(self, pin_net, nodes, CERO_DOUB, SOURCE_ADMIT),
            Kind::FixedVolt { voltage } => {
                stamp_to_gnd(self, pin_net, nodes, *voltage, SOURCE_ADMIT)
            }
            Kind::Junction => {}
            Kind::Capacitor { capacitance, volt } | Kind::ElCapacitor { capacitance, volt } => {
                if dt > 0.0 {
                    let g = capacitance / dt;
                    stamp_two_terminal(self, pin_net, nodes, g, volt * g);
                }
            }
            Kind::Inductor { inductance, ieq } => {
                if dt > 0.0 {
                    let g = dt / inductance;
                    stamp_two_terminal(self, pin_net, nodes, g, *ieq);
                }
            }
            Kind::Switch {
                closed,
                poles,
                double_throw,
            } => {
                if *double_throw {
                    for i in 0..*poles {
                        let target_pin = if *closed {
                            format!("{}-switch{}pinN", self.id, 2 * i)
                        } else {
                            format!("{}-switch{}pinN", self.id, 2 * i + 1)
                        };
                        stamp_between(
                            pin_net,
                            nodes,
                            &format!("{}-pinP{i}", self.id),
                            &target_pin,
                            SWITCH_CLOSED_ADMIT,
                            0.0,
                        );
                    }
                } else if *closed {
                    for i in 0..*poles {
                        stamp_between(
                            pin_net,
                            nodes,
                            &format!("{}-pinP{i}", self.id),
                            &format!("{}-switch{i}pinN", self.id),
                            SWITCH_CLOSED_ADMIT,
                            0.0,
                        );
                    }
                }
            }
            Kind::Diode { state, .. } => {
                stamp_between(
                    pin_net,
                    nodes,
                    &self.left_pin(),
                    &self.mid_pin(),
                    state.admit,
                    state.i_src(),
                );
                stamp_between(
                    pin_net,
                    nodes,
                    &self.mid_pin(),
                    &self.right_pin(),
                    1.0 / state.series_r,
                    0.0,
                );
            }
            Kind::Led { state } => {
                if state.grounded {
                    if let Some(&n0) = pin_net.get(&self.left_pin()) {
                        nodes[n0].add_admit(None, state.admit);
                        if state.th_current != 0.0 {
                            nodes[n0].add_current(state.th_current);
                        }
                    }
                } else {
                    stamp_two_terminal(self, pin_net, nodes, state.admit, state.th_current);
                }
            }
            Kind::Bjt { state } => {
                let c = self.collector_pin();
                let e = self.emitter_pin();
                let b = self.base_pin();
                // Directed stamps matching C++ ePin pairs BC/CB/BE/EB/CE/EC.
                stamp_admit_directed(pin_net, nodes, &b, &c, -state.gec - state.gcc);
                stamp_admit_directed(pin_net, nodes, &c, &b, -state.gce - state.gcc);
                stamp_admit_directed(pin_net, nodes, &b, &e, -state.gee - state.gce);
                stamp_admit_directed(pin_net, nodes, &e, &b, -state.gee - state.gec);
                stamp_admit_directed(pin_net, nodes, &c, &e, state.gce);
                stamp_admit_directed(pin_net, nodes, &e, &c, state.gec);
                stamp_current_at(pin_net, nodes, &b, state.i_base);
                stamp_current_at(pin_net, nodes, &c, state.i_coll);
                stamp_current_at(pin_net, nodes, &e, state.i_emit);
            }
            Kind::Mosfet { state } => {
                stamp_between(
                    pin_net,
                    nodes,
                    &self.drain_pin(),
                    &self.source_pin(),
                    state.admit,
                    state.current,
                );
            }
            Kind::OpAmp { state } => {
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &self.output_amp_pin(),
                    state.last_out,
                    state.admit(),
                );
            }
            Kind::Jfet { state } => {
                // C++ eJfet current stamp is a no-op; only G = Id/Vds is real.
                stamp_between(
                    pin_net,
                    nodes,
                    &self.drain_pin(),
                    &self.source_pin(),
                    state.admit,
                    0.0,
                );
            }
            Kind::Comparator { state } => {
                stamp_iopin(pin_net, nodes, &state.output);
            }
            Kind::VoltReg { state } => {
                stamp_between(
                    pin_net,
                    nodes,
                    &self.voltreg_in_pin(),
                    &self.voltreg_out_pin(),
                    state.admit,
                    state.last_current,
                );
            }
            Kind::Probe { .. } => {
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &self.probe_pin(),
                    0.0,
                    crate::instruments::PROBE_ADMIT,
                );
            }
            Kind::Voltmeter { last_out, .. } => {
                stamp_two_terminal(
                    self,
                    pin_net,
                    nodes,
                    1.0 / crate::instruments::VOLTMETER_OHMS,
                    0.0,
                );
                stamp_pin_to_gnd(pin_net, nodes, &self.out_pin(), *last_out, SOURCE_ADMIT);
            }
            Kind::Ammeter { last_out, .. } => {
                stamp_two_terminal(
                    self,
                    pin_net,
                    nodes,
                    1.0 / crate::instruments::AMMETER_OHMS,
                    0.0,
                );
                stamp_pin_to_gnd(pin_net, nodes, &self.out_pin(), *last_out, SOURCE_ADMIT);
            }
            Kind::FreqMeter { .. } => {}
            Kind::Oscope {
                connect_gnd,
                input_imped,
                ..
            } => {
                if *connect_gnd {
                    let admit = if *input_imped > 0.0 {
                        1.0 / (*input_imped * 1e6)
                    } else {
                        crate::instruments::PLOT_INPUT_ADMIT
                    };
                    for i in 0..4 {
                        stamp_pin_to_gnd(pin_net, nodes, &self.plot_pin(i), 0.0, admit);
                    }
                    stamp_pin_to_gnd(pin_net, nodes, &self.plot_gnd_pin(), 0.0, admit);
                }
            }
            Kind::LAnalizer {
                connect_gnd,
                input_imped,
                ..
            } => {
                if *connect_gnd {
                    let admit = if *input_imped > 0.0 {
                        1.0 / (*input_imped * 1e6)
                    } else {
                        crate::instruments::PLOT_INPUT_ADMIT
                    };
                    for i in 0..8 {
                        stamp_pin_to_gnd(pin_net, nodes, &self.plot_pin(i), 0.0, admit);
                    }
                }
            }
            Kind::Gate(g) => {
                for p in g.inputs.iter().chain(g.oe.as_ref()) {
                    stamp_iopin(pin_net, nodes, p);
                }
                stamp_iopin(pin_net, nodes, &g.output);
            }
            Kind::FlipFlop(f) => stamp_flipflop(pin_net, nodes, f),
            Kind::Latch(l) => {
                for p in l
                    .inputs
                    .iter()
                    .chain(l.outputs.iter())
                    .chain([&l.clk, &l.reset, &l.oe])
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::McuPin(p) => stamp_iopin(pin_net, nodes, &p.pin),
            Kind::Mcu(m) => {
                for p in &m.pins {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::QemuDevice(q) => {
                for p in &q.pins {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::ScriptCpu(s) => {
                for p in s
                    .pins
                    .iter()
                    .chain(s.ports.iter().flat_map(|po| po.pins.iter()))
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::TestUnit(tu) => {
                for p in tu.drive.iter().chain(tu.sense.iter()) {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::Hd44780(h) => {
                for p in h.pins() {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::Ssd1306(s) => {
                stamp_iopin(pin_net, nodes, &s.pin_scl);
                stamp_iopin(pin_net, nodes, &s.pin_sda);
            }
            Kind::Aip31068(a) => {
                for p in a.pins() {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::Sh1107(s) => {
                for p in s.pins() {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::Pcd8544(p) => {
                for pin in p.pins() {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::Ks0108(k) => {
                for pin in k.pins() {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::Mux(m) => {
                for p in m
                    .inputs
                    .iter()
                    .chain(m.addr_pins.iter())
                    .chain(m.enable.iter())
                    .chain(std::iter::once(&m.output))
                    .chain(std::iter::once(&m.out_inverted))
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::Demux(d) => {
                for p in std::iter::once(&d.input)
                    .chain(d.addr_pins.iter())
                    .chain(d.enable.iter())
                    .chain(d.outputs.iter())
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::BcdToDec(b) => {
                for p in b.inputs.iter().chain(b.outputs.iter()) {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::DecToBcd(d) => {
                for p in d
                    .inputs
                    .iter()
                    .chain(d.outputs.iter())
                    .chain(std::iter::once(&d.gs))
                    .chain(std::iter::once(&d.eo))
                    .chain(std::iter::once(&d.ei))
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::BcdTo7S(b) => {
                for p in b
                    .inputs
                    .iter()
                    .chain(std::iter::once(&b.lt))
                    .chain(std::iter::once(&b.rbi))
                    .chain(std::iter::once(&b.bi_rbo))
                    .chain(b.segments.iter())
                {
                    stamp_iopin(pin_net, nodes, p);
                }
            }
            Kind::I2CToParallel(p) => {
                for pin in p
                    .ports
                    .iter()
                    .chain(std::iter::once(&p.int_pin))
                    .chain(std::iter::once(&p.scl))
                    .chain(std::iter::once(&p.sda))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::Adc(a) => {
                for pin in a
                    .outputs
                    .iter()
                    .chain(std::iter::once(&a.soc))
                    .chain(std::iter::once(&a.eoc))
                    .chain(std::iter::once(&a.oe))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::Dac(d) => {
                for pin in &d.inputs {
                    stamp_iopin(pin_net, nodes, pin);
                }
                stamp_to_gnd(self, pin_net, nodes, d.vout, SOURCE_ADMIT);
            }
            Kind::Counter(c) => {
                for pin in c
                    .outputs
                    .iter()
                    .chain(std::iter::once(&c.clock))
                    .chain(std::iter::once(&c.reset))
                    .chain(std::iter::once(&c.enable))
                    .chain(std::iter::once(&c.output))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::BinCounter(b) => {
                for pin in b
                    .outputs
                    .iter()
                    .chain(std::iter::once(&b.clk_a))
                    .chain(std::iter::once(&b.clk_b))
                    .chain(std::iter::once(&b.r0_1))
                    .chain(std::iter::once(&b.r0_2))
                    .chain(std::iter::once(&b.r9_1))
                    .chain(std::iter::once(&b.r9_2))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::FullAdder(fa) => {
                for pin in fa
                    .a_inputs
                    .iter()
                    .chain(fa.b_inputs.iter())
                    .chain(std::iter::once(&fa.ci))
                    .chain(fa.sum_outputs.iter())
                    .chain(std::iter::once(&fa.co))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::HalfAdder(fa) => {
                for pin in fa
                    .a_inputs
                    .iter()
                    .chain(fa.b_inputs.iter())
                    .chain(fa.sum_outputs.iter())
                    .chain(std::iter::once(&fa.co))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::MagnitudeComp(mc) => {
                for pin in mc
                    .a_inputs
                    .iter()
                    .chain(mc.b_inputs.iter())
                    .chain(std::iter::once(&mc.cascade_gt))
                    .chain(std::iter::once(&mc.cascade_eq))
                    .chain(std::iter::once(&mc.cascade_lt))
                    .chain(std::iter::once(&mc.out_gt))
                    .chain(std::iter::once(&mc.out_eq))
                    .chain(std::iter::once(&mc.out_lt))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::ShiftReg(sr) => {
                for pin in sr
                    .outputs
                    .iter()
                    .chain(std::iter::once(&sr.ser_in))
                    .chain(std::iter::once(&sr.clk_shift))
                    .chain(std::iter::once(&sr.clk_latch))
                    .chain(std::iter::once(&sr.master_reset))
                    .chain(std::iter::once(&sr.oe))
                    .chain(std::iter::once(&sr.ser_out))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::Function(f) => {
                for pin in f.inputs.iter().chain(std::iter::once(&f.output)) {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::Memory(m) => {
                for pin in m
                    .addr_pins
                    .iter()
                    .chain(m.data_pins.iter())
                    .chain(std::iter::once(&m.cs))
                    .chain(std::iter::once(&m.oe))
                    .chain(std::iter::once(&m.we))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::DynamicMemory(dm) => {
                for pin in dm
                    .addr_pins
                    .iter()
                    .chain(dm.data_pins.iter())
                    .chain(std::iter::once(&dm.ras))
                    .chain(std::iter::once(&dm.cas))
                    .chain(std::iter::once(&dm.we))
                    .chain(std::iter::once(&dm.oe))
                {
                    stamp_iopin(pin_net, nodes, pin);
                }
            }
            Kind::I2CRam(r) => {
                stamp_iopin(pin_net, nodes, &r.scl);
                stamp_iopin(pin_net, nodes, &r.sda);
            }
            Kind::Lm555(lm) => {
                let gnd_pin = format!("{}-ePin0", self.id);
                let out_pin = format!("{}-ePin2", self.id);
                let dis_pin = format!("{}-ePin6", self.id);
                let vcc_pin = format!("{}-ePin7", self.id);
                stamp_between(
                    pin_net,
                    nodes,
                    &vcc_pin,
                    &format!("{}-ePin4", self.id),
                    1.0 / 5000.0,
                    0.0,
                );
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-ePin4", self.id),
                    &gnd_pin,
                    1.0 / 10000.0,
                    0.0,
                );
                let out_v = if lm.out_state {
                    lm.out_high_v
                } else {
                    lm.out_low_v
                };
                stamp_pin_to_gnd(pin_net, nodes, &out_pin, out_v, 1.0 / 3.5);
                if lm.discharge_on {
                    stamp_between(pin_net, nodes, &dis_pin, &gnd_pin, 1.0 / 1.0, 0.0);
                }
            }
            Kind::Clock { voltage, state, .. } => {
                stamp_to_gnd(
                    self,
                    pin_net,
                    nodes,
                    if *state { *voltage } else { 0.0 },
                    SOURCE_ADMIT,
                );
            }
            Kind::Rail { voltage } => {
                stamp_to_gnd(self, pin_net, nodes, *voltage, SOURCE_ADMIT);
            }
            Kind::VoltSource { value, running } => {
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-outPin", self.id),
                    if *running { *value } else { 0.0 },
                    SOURCE_ADMIT,
                );
            }
            Kind::CurrSource { value, running } => {
                if *running {
                    stamp_current_at(pin_net, nodes, &format!("{}-outPin", self.id), *value);
                }
            }
            Kind::Csource {
                curr_source,
                volt,
                current,
                ..
            } => {
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-s1Pin", self.id),
                    &format!("{}-s2Pin", self.id),
                    if *curr_source { 1e-6 } else { SOURCE_ADMIT },
                    if *curr_source {
                        *current
                    } else {
                        *volt * SOURCE_ADMIT
                    },
                );
            }
            Kind::WaveGen {
                amplitude,
                offset,
                bipolar,
                floating,
                v_out,
                ..
            } => {
                let gnd_pin = format!("{}-gndnod", self.id);
                let out_pin = format!("{}-outnod", self.id);
                if *bipolar {
                    let volt = 2.0 * *amplitude * (*v_out - 0.5);
                    if *floating {
                        stamp_between(
                            pin_net,
                            nodes,
                            &out_pin,
                            &gnd_pin,
                            SOURCE_ADMIT,
                            volt * SOURCE_ADMIT,
                        );
                    } else {
                        let half_v = volt / 2.0;
                        stamp_pin_to_gnd(pin_net, nodes, &out_pin, *offset + half_v, SOURCE_ADMIT);
                        stamp_pin_to_gnd(pin_net, nodes, &gnd_pin, *offset - half_v, SOURCE_ADMIT);
                    }
                } else {
                    let volt_base = *offset - *amplitude;
                    let v = volt_base + 2.0 * *amplitude * *v_out;
                    stamp_pin_to_gnd(pin_net, nodes, &out_pin, v, SOURCE_ADMIT);
                }
            }
            Kind::Push { closed, poles } => {
                if *closed {
                    for i in 0..*poles {
                        stamp_between(
                            pin_net,
                            nodes,
                            &format!("{}-lPin{i}", self.id),
                            &format!("{}-rPin{i}", self.id),
                            SWITCH_CLOSED_ADMIT,
                            0.0,
                        );
                    }
                }
            }
            Kind::SwitchDip {
                size,
                state,
                common_pin,
            } => {
                for i in 0..*size {
                    if (state & (1 << i)) != 0 {
                        if *common_pin {
                            stamp_between(
                                pin_net,
                                nodes,
                                &format!("{}-com", self.id),
                                &format!("{}-pin{i}", self.id),
                                SWITCH_CLOSED_ADMIT,
                                0.0,
                            );
                        } else {
                            stamp_between(
                                pin_net,
                                nodes,
                                &format!("{}-pin{}", self.id, 2 * i),
                                &format!("{}-pin{}", self.id, 2 * i + 1),
                                SWITCH_CLOSED_ADMIT,
                                0.0,
                            );
                        }
                    }
                }
            }
            Kind::KeyPad {
                rows,
                cols,
                pressed,
                ..
            } => {
                if let Some((r, c)) = pressed {
                    if *r < *rows && *c < *cols {
                        let row_pin = format!("{}-Pin{r}", self.id);
                        let col_pin = format!("{}-Pin{}", self.id, rows + c);
                        stamp_between(pin_net, nodes, &row_pin, &col_pin, SWITCH_CLOSED_ADMIT, 0.0);
                    }
                }
            }
            Kind::LedMatrix { .. }
            | Kind::Max72xx { .. }
            | Kind::Ws2812 { .. }
            | Kind::Dial { .. }
            | Kind::Shape { .. }
            | Kind::SubPackage
            | Kind::SevenSegmentBCD { .. } => {}
            Kind::Relay { active, .. } => {
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-lPin", self.id),
                    &format!("{}-rPin", self.id),
                    1.0 / 100.0,
                    0.0,
                );
                if *active {
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-c1", self.id),
                        &format!("{}-c2", self.id),
                        SWITCH_CLOSED_ADMIT,
                        0.0,
                    );
                }
            }
            Kind::Potentiometer { resistance, wiper } => {
                let r1 = (*wiper * resistance).max(RESISTOR_MIN_OHMS);
                let r2 = ((1.0 - wiper) * resistance).max(RESISTOR_MIN_OHMS);
                let a = self.pot_pin_a_node(pin_net);
                let m = self.pot_pin_m_node(pin_net);
                let b = self.pot_pin_b_node(pin_net);
                stamp_cached_two_terminal(a, m, nodes, 1.0 / r1, 0.0);
                stamp_cached_two_terminal(m, b, nodes, 1.0 / r2, 0.0);
            }
            Kind::TouchPad {
                width,
                height,
                rx_min,
                rx_max,
                ry_min,
                ry_max,
                x_pos,
                y_pos,
            } => {
                let w = (*width as f64).max(1.0);
                let h = (*height as f64).max(1.0);
                let (x_res_a, x_res_b, y_res_a, y_res_b, t_admit) = if *x_pos < 0 || *y_pos < 0 {
                    (
                        (rx_max - rx_min).abs() / 2.0,
                        (rx_max - rx_min).abs() / 2.0,
                        (ry_max - ry_min).abs() / 2.0,
                        (ry_max - ry_min).abs() / 2.0,
                        0.0,
                    )
                } else {
                    let xp = (*x_pos as f64).clamp(0.0, w);
                    let yp = (*y_pos as f64).clamp(0.0, h);
                    let xa = rx_min + (rx_max - rx_min) * (xp / w);
                    let xb = rx_min + rx_max - xa;
                    let ya = ry_min + (ry_max - ry_min) * (yp / h);
                    let yb = ry_min + ry_max - ya;
                    (xa, xb, ya, yb, 1.0)
                };
                let ra = x_res_a.max(RESISTOR_MIN_OHMS);
                let rb = x_res_b.max(RESISTOR_MIN_OHMS);
                let rya = y_res_a.max(RESISTOR_MIN_OHMS);
                let ryb = y_res_b.max(RESISTOR_MIN_OHMS);
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-vrx_p", self.id),
                    &format!("{}-nodeX", self.id),
                    1.0 / ra,
                    0.0,
                );
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-vrx_m", self.id),
                    &format!("{}-nodeX", self.id),
                    1.0 / rb,
                    0.0,
                );
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-vry_p", self.id),
                    &format!("{}-nodeY", self.id),
                    1.0 / rya,
                    0.0,
                );
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-vry_m", self.id),
                    &format!("{}-nodeY", self.id),
                    1.0 / ryb,
                    0.0,
                );
                if t_admit > 0.0 {
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-nodeX", self.id),
                        &format!("{}-nodeY", self.id),
                        t_admit,
                        0.0,
                    );
                }
            }
            Kind::Ky023 {
                stick_x,
                stick_y,
                btn_down,
            } => {
                let legacy_scale = stick_x.abs() > 1.0 || stick_y.abs() > 1.0;
                let x_norm = if legacy_scale {
                    ((*stick_x + 25.0) / 50.0).clamp(0.0, 1.0)
                } else {
                    ((*stick_x + 1.0) / 2.0).clamp(0.0, 1.0)
                };
                let y_norm = if legacy_scale {
                    ((*stick_y + 25.0) / 50.0).clamp(0.0, 1.0)
                } else {
                    ((*stick_y + 1.0) / 2.0).clamp(0.0, 1.0)
                };
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-vrx", self.id),
                    5.0 * x_norm,
                    SOURCE_ADMIT,
                );
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-vry", self.id),
                    5.0 * y_norm,
                    SOURCE_ADMIT,
                );
                if *btn_down {
                    stamp_pin_to_gnd(pin_net, nodes, &format!("{}-sw", self.id), 0.0, 1000.0);
                } else {
                    stamp_pin_to_gnd(
                        pin_net,
                        nodes,
                        &format!("{}-sw", self.id),
                        5.0,
                        1.0 / 2000.0,
                    );
                }
            }
            Kind::Ky040 {
                state_a,
                state_b,
                btn_closed,
                ..
            } => {
                let va = if *state_a { 5.0 } else { 0.0 };
                let vb = if *state_b { 5.0 } else { 0.0 };
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-clk", self.id),
                    va,
                    SOURCE_ADMIT,
                );
                stamp_pin_to_gnd(pin_net, nodes, &format!("{}-dt", self.id), vb, SOURCE_ADMIT);
                if *btn_closed {
                    stamp_pin_to_gnd(pin_net, nodes, &format!("{}-sw", self.id), 0.0, 1000.0);
                } else {
                    stamp_pin_to_gnd(
                        pin_net,
                        nodes,
                        &format!("{}-sw", self.id),
                        5.0,
                        1.0 / 2000.0,
                    );
                }
            }
            Kind::Sr04 { echo_high, .. } => {
                let ve = if *echo_high { 5.0 } else { 0.0 };
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-outpin", self.id),
                    ve,
                    SOURCE_ADMIT,
                );
            }
            Kind::Dht22 {
                out_state,
                pin_driven,
                ..
            } => {
                if *pin_driven {
                    let v = if *out_state { 5.0 } else { 0.0 };
                    stamp_pin_to_gnd(
                        pin_net,
                        nodes,
                        &format!("{}-inPin", self.id),
                        v,
                        SOURCE_ADMIT,
                    );
                }
            }
            Kind::Ds18b20 { dq_low, .. } => {
                if *dq_low {
                    stamp_pin_to_gnd(
                        pin_net,
                        nodes,
                        &format!("{}-inPin", self.id),
                        0.0,
                        SWITCH_CLOSED_ADMIT,
                    );
                }
            }
            Kind::Ds1621 {
                tout_high, sda_low, ..
            } => {
                let vt = if *tout_high { 5.0 } else { 0.0 };
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-outPin0", self.id),
                    vt,
                    SOURCE_ADMIT,
                );
                if *sda_low {
                    stamp_pin_to_gnd(
                        pin_net,
                        nodes,
                        &format!("{}-inPin0", self.id),
                        0.0,
                        SWITCH_CLOSED_ADMIT,
                    );
                }
            }
            Kind::Ds1307 {
                sqw_state, sda_low, ..
            } => {
                let v_sqw = if *sqw_state { 5.0 } else { 0.0 };
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-PinSQW", self.id),
                    v_sqw,
                    SOURCE_ADMIT,
                );
                if *sda_low {
                    stamp_pin_to_gnd(
                        pin_net,
                        nodes,
                        &format!("{}-PinSDA", self.id),
                        0.0,
                        SWITCH_CLOSED_ADMIT,
                    );
                }
            }
            Kind::DcMotor { resistance, .. } => {
                let g = 1.0 / resistance.max(RESISTOR_MIN_OHMS);
                stamp_between(
                    pin_net,
                    nodes,
                    &format!("{}-lPin", self.id),
                    &format!("{}-rPin", self.id),
                    g,
                    0.0,
                );
            }
            Kind::Stepper {
                bipolar,
                resistance,
                ..
            } => {
                let g = 1.0 / resistance.max(RESISTOR_MIN_OHMS);
                if *bipolar {
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-PinA1", self.id),
                        &format!("{}-PinA2", self.id),
                        g,
                        0.0,
                    );
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-PinB1", self.id),
                        &format!("{}-PinB2", self.id),
                        g,
                        0.0,
                    );
                } else {
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-PinA1", self.id),
                        &format!("{}-PinCo", self.id),
                        g,
                        0.0,
                    );
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-PinA2", self.id),
                        &format!("{}-PinCo", self.id),
                        g,
                        0.0,
                    );
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-PinB1", self.id),
                        &format!("{}-PinCo", self.id),
                        g,
                        0.0,
                    );
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-PinB2", self.id),
                        &format!("{}-PinCo", self.id),
                        g,
                        0.0,
                    );
                }
            }
            Kind::Servo { .. } => {
                // High input impedance on Sig pin
                stamp_pin_to_gnd(pin_net, nodes, &format!("{}-PinSig", self.id), 0.0, 1e-7);
            }
            Kind::SdCard { .. } => {}
            Kind::Esp01 { .. } => {
                stamp_pin_to_gnd(
                    pin_net,
                    nodes,
                    &format!("{}-pin0", self.id),
                    3.3,
                    SOURCE_ADMIT,
                );
            }
            Kind::TftDisplay { .. } | Kind::Pcf8833Display { .. } => {}
            Kind::VarResistor { resistance }
            | Kind::Ldr { resistance, .. }
            | Kind::Thermistor { resistance, .. }
            | Kind::Rtd { resistance, .. }
            | Kind::Strain { resistance, .. }
            | Kind::Lamp { resistance, .. } => {
                stamp_two_terminal(
                    self,
                    pin_net,
                    nodes,
                    1.0 / resistance.max(RESISTOR_MIN_OHMS),
                    0.0,
                );
            }
            Kind::ResistorDip {
                size,
                resistance,
                bussed,
            } => {
                let g = 1.0 / resistance.max(RESISTOR_MIN_OHMS);
                if *bussed {
                    for i in 0..*size {
                        stamp_between(
                            pin_net,
                            nodes,
                            &format!("{}-com", self.id),
                            &format!("{}-pin{i}", self.id),
                            g,
                            0.0,
                        );
                    }
                } else {
                    for i in 0..*size {
                        stamp_between(
                            pin_net,
                            nodes,
                            &format!("{}-lPin{i}", self.id),
                            &format!("{}-rPin{i}", self.id),
                            g,
                            0.0,
                        );
                    }
                }
            }
            Kind::VarCapacitor { capacitance, volt } => {
                if dt > 0.0 {
                    let g = capacitance / dt;
                    stamp_two_terminal(self, pin_net, nodes, g, volt * g);
                }
            }
            Kind::VarInductor { inductance, ieq } => {
                if dt > 0.0 {
                    let g = dt / inductance;
                    stamp_two_terminal(self, pin_net, nodes, g, *ieq);
                }
            }
            Kind::Transformer {
                inductance1,
                inductance2,
                ..
            } => {
                if dt > 0.0 {
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-p1", self.id),
                        &format!("{}-p2", self.id),
                        dt / inductance1.max(1e-12),
                        0.0,
                    );
                    stamp_between(
                        pin_net,
                        nodes,
                        &format!("{}-s1", self.id),
                        &format!("{}-s2", self.id),
                        dt / inductance2.max(1e-12),
                        0.0,
                    );
                }
            }
            Kind::Scr { conducting, .. } => {
                let a = self.scr_anode_pin();
                let k = self.scr_cathode_pin();
                let g = self.scr_gate_pin();
                stamp_between(pin_net, nodes, &g, &k, 1.0 / 100.0, 0.0);
                if *conducting {
                    stamp_between(
                        pin_net,
                        nodes,
                        &a,
                        &k,
                        SWITCH_CLOSED_ADMIT,
                        0.7 * SWITCH_CLOSED_ADMIT,
                    );
                } else {
                    stamp_between(pin_net, nodes, &a, &k, 1e-9, 0.0);
                }
            }
            Kind::Triac { conducting, .. } => {
                let mt1 = self.triac_mt1_pin();
                let mt2 = self.triac_mt2_pin();
                let g = self.triac_gate_pin();
                stamp_between(pin_net, nodes, &g, &mt1, 1.0 / 100.0, 0.0);
                if *conducting {
                    stamp_between(pin_net, nodes, &mt1, &mt2, SWITCH_CLOSED_ADMIT, 0.0);
                } else {
                    stamp_between(pin_net, nodes, &mt1, &mt2, 1e-9, 0.0);
                }
            }
            Kind::Diac { conducting, .. } => {
                let l = self.left_pin();
                let r = self.right_pin();
                if *conducting {
                    stamp_between(pin_net, nodes, &l, &r, 1.0 / 500.0, 0.0);
                } else {
                    stamp_between(pin_net, nodes, &l, &r, 1e-8, 0.0);
                }
            }
            Kind::AnalogMux {
                selected,
                on_res,
                channels,
                ..
            } => {
                let com = self.mux_common_pin();
                for i in 0..*channels {
                    let ch = self.mux_channel_pin(i);
                    if i == *selected {
                        stamp_between(pin_net, nodes, &ch, &com, 1.0 / on_res.max(1.0), 0.0);
                    } else {
                        stamp_between(pin_net, nodes, &ch, &com, 1e-7, 0.0);
                    }
                }
            }
            Kind::LedBar {
                segments, grounded, ..
            } => {
                for i in 0..*segments {
                    let la = format!("{}-lPin{i}", self.id);
                    if *grounded {
                        stamp_pin_to_gnd(pin_net, nodes, &la, 1.8, 1.0 / 100.0);
                    } else {
                        let ra = format!("{}-rPin{i}", self.id);
                        stamp_between(pin_net, nodes, &la, &ra, 1.0 / 100.0, 1.8 / 100.0);
                    }
                }
            }
            Kind::RgbLed { common_anode, .. } => {
                let r_pin = format!("{}-rPin", self.id);
                let g_pin = format!("{}-gPin", self.id);
                let b_pin = format!("{}-bPin", self.id);
                let c_pin = format!("{}-cPin", self.id);
                if *common_anode {
                    stamp_between(pin_net, nodes, &c_pin, &r_pin, 1.0 / 100.0, 1.8 / 100.0);
                    stamp_between(pin_net, nodes, &c_pin, &g_pin, 1.0 / 100.0, 1.8 / 100.0);
                    stamp_between(pin_net, nodes, &c_pin, &b_pin, 1.0 / 100.0, 1.8 / 100.0);
                } else {
                    stamp_between(pin_net, nodes, &r_pin, &c_pin, 1.0 / 100.0, 1.8 / 100.0);
                    stamp_between(pin_net, nodes, &g_pin, &c_pin, 1.0 / 100.0, 1.8 / 100.0);
                    stamp_between(pin_net, nodes, &b_pin, &c_pin, 1.0 / 100.0, 1.8 / 100.0);
                }
            }
            Kind::SevenSegment { common_anode, .. } => {
                let segs = [
                    "pin_a", "pin_b", "pin_c", "pin_d", "pin_e", "pin_f", "pin_g", "pin_dot", "a",
                    "b", "c", "d", "e", "f", "g", "dp",
                ];
                let com_a = format!("{}-pin_commona", self.id);
                let com_b = format!("{}-com", self.id);
                for seg in segs {
                    let seg_pin = format!("{}-{seg}", self.id);
                    if *common_anode {
                        stamp_between(pin_net, nodes, &com_a, &seg_pin, 1.0 / 100.0, 1.8 / 100.0);
                        stamp_between(pin_net, nodes, &com_b, &seg_pin, 1.0 / 100.0, 1.8 / 100.0);
                    } else {
                        stamp_between(pin_net, nodes, &seg_pin, &com_a, 1.0 / 100.0, 1.8 / 100.0);
                        stamp_between(pin_net, nodes, &seg_pin, &com_b, 1.0 / 100.0, 1.8 / 100.0);
                    }
                }
            }
            Kind::AudioOut { source, .. } => {
                stamp_two_terminal(self, pin_net, nodes, source.stamp_admit(), 0.0);
            }
            Kind::SerialPort { .. }
            | Kind::SerialTerm { .. }
            | Kind::Bus { .. }
            | Kind::Header { .. }
            | Kind::Socket { .. } => {}
            Kind::Tunnel { .. } | Kind::Subcircuit { .. } | Kind::McuItem(_) => {}
        }
    }
}

#[inline]
fn stamp_iopin(
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    pin: &crate::digital::IoPin,
) {
    let (v, g) = pin.norton();
    if let Some(n) = pin.node_idx {
        nodes[n].add_admit(None, g);
        nodes[n].add_current(v * g);
    } else {
        stamp_pin_to_gnd(pin_net, nodes, &pin.id, v, g);
    }
}

fn stamp_flipflop(
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    f: &FlipFlopState,
) {
    let mut seen = std::collections::HashSet::new();
    for p in [&f.d, &f.j, &f.k, &f.t, &f.set, &f.rst, &f.clk, &f.q, &f.qn] {
        if seen.insert(p.id.as_str()) {
            stamp_iopin(pin_net, nodes, p);
        }
    }
}

/// Two-terminal stamp: G between pins and a Norton current `i_src` out of the
/// left pin (into the right). Battery uses `i_src = V·G`; capacitor uses
/// `V_prev·G`; inductor uses the companion `Ieq`.
fn stamp_two_terminal(
    comp: &Comp,
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    g: f64,
    i_src: f64,
) {
    let Some(&n0) = pin_net.get(&comp.left_pin()) else {
        return;
    };
    let Some(&n1) = pin_net.get(&comp.right_pin()) else {
        return;
    };
    nodes[n0].add_admit(Some(n1), g);
    nodes[n1].add_admit(Some(n0), g);
    if i_src != 0.0 {
        nodes[n0].add_current(i_src);
        nodes[n1].add_current(-i_src);
    }
}

/// IoPin `source` mode: Norton equivalent of a voltage to implicit ground.
fn stamp_to_gnd(
    comp: &Comp,
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    volts: f64,
    g: f64,
) {
    let pin = match comp.kind {
        Kind::Ground => comp.gnd_pin(),
        _ => comp.out_pin(),
    };
    stamp_pin_to_gnd(pin_net, nodes, &pin, volts, g);
}

fn stamp_pin_to_gnd(
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    pin: &str,
    volts: f64,
    g: f64,
) {
    let Some(&n) = pin_net.get(pin) else {
        return;
    };
    nodes[n].add_admit(None, g);
    nodes[n].add_current(volts * g);
}

fn stamp_between(
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    a: &str,
    b: &str,
    g: f64,
    i_src: f64,
) {
    let Some(&n0) = pin_net.get(a) else {
        return;
    };
    let Some(&n1) = pin_net.get(b) else {
        return;
    };
    nodes[n0].add_admit(Some(n1), g);
    nodes[n1].add_admit(Some(n0), g);
    if i_src != 0.0 {
        nodes[n0].add_current(i_src);
        nodes[n1].add_current(-i_src);
    }
}

/// One-sided admitance: G on `from`'s diagonal and off-diagonal to `to`.
fn stamp_admit_directed(
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    from: &str,
    to: &str,
    g: f64,
) {
    let Some(&n0) = pin_net.get(from) else {
        return;
    };
    let Some(&n1) = pin_net.get(to) else {
        return;
    };
    if n0 == n1 {
        return;
    }
    nodes[n0].add_admit(Some(n1), g);
}

fn stamp_current_at(
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    nodes: &mut [ENode],
    pin: &str,
    i: f64,
) {
    let Some(&n) = pin_net.get(pin) else {
        return;
    };
    if i != 0.0 {
        nodes[n].add_current(i);
    }
}
