//! Event-driven digital scheduler, logic gate propagation delay queue, and MCU pin I/O.

use crate::Result;
use crate::digital::{EventTarget, GateUpdate, IoPin, PinAction, PinMode};
use crate::elements::Kind;

use super::Circuit;

/// Norton pair `(V, G)` moved enough to need another matrix stamp.
fn norton_moved(before: (f64, f64), pin: &IoPin) -> bool {
    let (v, g) = pin.norton();
    (v - before.0).abs() > 1e-12 || (g - before.1).abs() > 1e-12
}

fn snap_nortons(pins: &[IoPin]) -> Vec<(f64, f64)> {
    pins.iter().map(IoPin::norton).collect()
}

fn nortons_moved(before: &[(f64, f64)], pins: &[IoPin]) -> bool {
    pins.len() != before.len()
        || pins
            .iter()
            .zip(before)
            .any(|(pin, &prev)| norton_moved(prev, pin))
}

/// A meter pin stayed when its new pin set continues the same previous net.
/// A probe joining that net does not count. An empty pin set stays empty.
fn meter_pin_stayed(
    old_groups: &[(Vec<String>, f64)],
    old_pins: &[String],
    new_pins: &[String],
) -> bool {
    match super::matrix_solver::inherited_index(old_groups, new_pins) {
        Some(i) => old_groups[i].0.as_slice() == old_pins,
        None => old_pins.is_empty(),
    }
}

impl Circuit {
    pub fn set_pin_inverted(&mut self, pin_id: &str, inverted: bool) {
        for c in &mut self.components {
            c.set_pin_inverted(pin_id, inverted);
        }
    }

    /// Update an existing component's Kind in place.
    ///
    /// Stamp parameters are picked up by the next `re_solve`. Named-net
    /// membership ([`crate::elements::Comp::named_net_pins`]) regroups nodes,
    /// copies voltages onto the pin sets that remain, and that `re_solve`
    /// keeps device stamps. Pin-count changes still replace the circuit.
    pub fn update_component_kind(&mut self, id: &str, kind: &Kind) -> bool {
        self.regroup_pending = false;
        let links_before = self.named_net_links();
        let mut updated = None;
        for (index, c) in self.components.iter_mut().enumerate() {
            if c.id == id {
                match (&mut c.kind, kind) {
                    (
                        Kind::Transformer {
                            inductance1,
                            inductance2,
                            coupling,
                            r_coil1,
                            r_coil2,
                            ..
                        },
                        Kind::Transformer {
                            inductance1: l1,
                            inductance2: l2,
                            coupling: k,
                            r_coil1: r1,
                            r_coil2: r2,
                            ..
                        },
                    ) => {
                        *inductance1 = *l1;
                        *inductance2 = *l2;
                        *coupling = *k;
                        *r_coil1 = *r1;
                        *r_coil2 = *r2;
                    }
                    (
                        Kind::Capacitor { capacitance, .. },
                        Kind::Capacitor {
                            capacitance: cap, ..
                        },
                    )
                    | (
                        Kind::ElCapacitor { capacitance, .. },
                        Kind::ElCapacitor {
                            capacitance: cap, ..
                        },
                    )
                    | (
                        Kind::VarCapacitor { capacitance, .. },
                        Kind::VarCapacitor {
                            capacitance: cap, ..
                        },
                    ) => {
                        *capacitance = *cap;
                    }
                    (
                        Kind::Inductor { inductance, .. },
                        Kind::Inductor {
                            inductance: ind, ..
                        },
                    )
                    | (
                        Kind::VarInductor { inductance, .. },
                        Kind::VarInductor {
                            inductance: ind, ..
                        },
                    ) => {
                        *inductance = *ind;
                    }
                    _ => {
                        // A live edit rebuilds the kind with an empty component id.
                        // Keep a tunnel endpoint that the replacement left blank:
                        // `Tunnel::to_element_kind` stores an empty pin id.
                        let keep_tunnel = match (&c.kind, kind) {
                            (
                                Kind::Tunnel { pin_id: old, .. },
                                Kind::Tunnel { pin_id: new, .. },
                            ) if new.is_empty() => Some(old.clone()),
                            _ => None,
                        };
                        let prev = std::mem::replace(&mut c.kind, kind.clone());
                        if let Some(pin_id) = keep_tunnel {
                            if let Kind::Tunnel { pin_id: slot, .. } = &mut c.kind {
                                *slot = pin_id;
                            }
                        }
                        c.kind.adopt_runtime(&prev);
                        c.kind.rebind_id(&c.id);
                    }
                }
                updated = Some(index);
                break;
            }
        }
        let Some(index) = updated else {
            return false;
        };
        if matches!(self.components[index].kind, Kind::Clock { .. }) {
            self.reschedule_clock(index);
        }
        if self.nets_are_bound() && links_before != self.named_net_links() {
            self.rebind_named_nets();
            self.regroup_pending = true;
        }
        true
    }

    fn named_net_links(&self) -> Vec<(String, String)> {
        let mut links = Vec::new();
        for c in &self.components {
            links.extend(c.named_net_pins());
        }
        links.sort();
        links
    }

    fn nets_are_bound(&self) -> bool {
        !self.pin_net.is_empty() || !self.nodes.is_empty()
    }

    /// Rebuild union-find, pin caches, and instrument node indices.
    /// Unchanged pin groups keep their voltage. Time, events, and device
    /// stamps stay. A channel drops history when its pins move. A meter drops
    /// running stats when its pins continue a different net.
    fn rebind_named_nets(&mut self) {
        let channels = self.instrument_net_groups();
        let meters = self.meter_pin_groups();
        let old_groups = self.group_volts();
        self.rebuild_nets_keeping_volts();
        self.blank_moved_channels(&channels);
        self.reset_moved_meters(&meters, &old_groups);
    }

    fn pin_group(&self, pin: &str) -> Vec<String> {
        let Some(&node) = self.pin_net.get(pin) else {
            return Vec::new();
        };
        let mut pins: Vec<String> = self
            .pin_net
            .iter()
            .filter_map(|(id, idx)| (*idx == node).then(|| id.clone()))
            .collect();
        pins.sort();
        pins
    }

    fn instrument_net_groups(&self) -> Vec<(String, bool, Vec<Vec<String>>)> {
        let mut out = Vec::new();
        for c in &self.components {
            match &c.kind {
                Kind::Oscope { .. } => {
                    let mut groups = Vec::with_capacity(5);
                    for i in 0..4 {
                        groups.push(self.pin_group(&c.plot_pin(i)));
                    }
                    groups.push(self.pin_group(&c.plot_gnd_pin()));
                    out.push((c.id.clone(), true, groups));
                }
                Kind::LAnalizer { .. } => {
                    let groups = (0..8).map(|i| self.pin_group(&c.plot_pin(i))).collect();
                    out.push((c.id.clone(), false, groups));
                }
                _ => {}
            }
        }
        out
    }

    fn blank_moved_channels(&mut self, previous: &[(String, bool, Vec<Vec<String>>)]) {
        let current = self.instrument_net_groups();
        for (id, is_scope, groups) in &current {
            let Some((_, _, old)) = previous
                .iter()
                .find(|(pid, pscope, _)| pid == id && *pscope == *is_scope)
            else {
                continue;
            };
            let nch = if *is_scope { 4 } else { 8 };
            // Scope samples are channel minus the ground pin. A new ground
            // reference makes every channel's stored waveform stale.
            let gnd_moved = *is_scope && old.get(4) != groups.get(4);
            let sampler = if *is_scope {
                self.instruments.scopes.get_mut(id)
            } else {
                self.instruments.las.get_mut(id)
            };
            let Some(sampler) = sampler else {
                continue;
            };
            for ch in 0..nch {
                if gnd_moved || old.get(ch) != groups.get(ch) {
                    sampler.blank_channel(ch);
                }
            }
        }
    }

    fn meter_pin_groups(&self) -> Vec<(String, bool, Vec<Vec<String>>)> {
        let mut out = Vec::new();
        for c in &self.components {
            match &c.kind {
                Kind::Voltmeter { .. } | Kind::Ammeter { .. } => {
                    out.push((
                        c.id.clone(),
                        false,
                        vec![
                            self.pin_group(&c.left_pin()),
                            self.pin_group(&c.right_pin()),
                        ],
                    ));
                }
                Kind::FreqMeter { .. } => {
                    out.push((c.id.clone(), true, vec![self.pin_group(&c.left_pin())]));
                }
                _ => {}
            }
        }
        out
    }

    fn reset_moved_meters(
        &mut self,
        previous: &[(String, bool, Vec<Vec<String>>)],
        old_groups: &[(Vec<String>, f64)],
    ) {
        let current = self.meter_pin_groups();
        for (id, is_freq, groups) in &current {
            let stayed = previous
                .iter()
                .find(|(pid, freq, _)| pid == id && freq == is_freq)
                .is_some_and(|(_, _, old)| {
                    old.len() == groups.len()
                        && old
                            .iter()
                            .zip(groups.iter())
                            .all(|(old_g, new_g)| meter_pin_stayed(old_groups, old_g, new_g))
                });
            if stayed {
                continue;
            }
            if *is_freq {
                if let Some(state) = self.instruments.freq.get_mut(id) {
                    *state = crate::instruments::FreqMeterState::default();
                }
            } else if let Some(state) = self.instruments.meters.get_mut(id) {
                *state = crate::instruments::MeterStats::default();
            }
        }
    }

    /// Drop an armed clock edge and arm the next one for the kind's frequency.
    /// A zero frequency stays cancelled. The level matches `clock_is_high` at the current time.
    fn reschedule_clock(&mut self, index: usize) {
        self.events.cancel(EventTarget::Device(index));
        let now = self.circ_time;
        let Some(comp) = self.components.get(index) else {
            return;
        };
        let high = comp.clock_is_high(now);
        let delta = comp.clock_next_event_ps(now);
        if let Kind::Clock { state, .. } = &mut self.components[index].kind {
            *state = high;
        }
        if let Some(dt) = delta {
            self.events
                .add(now.saturating_add(dt), EventTarget::Device(index));
        }
    }

    /// MCU core drives a digital pin (output high/low).
    pub fn mcu_pin_drive(&mut self, id: &str, high: bool) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::McuPin(p) = &mut c.kind {
                    let a = p.drive(high);
                    // Immediate stamp change; caller should solve().
                    let _ = a;
                    return true;
                }
            }
        }
        false
    }

    pub fn mcu_pin_set_mode(&mut self, id: &str, mode: PinMode) -> bool {
        for c in &mut self.components {
            if c.id == id {
                if let Kind::McuPin(p) = &mut c.kind {
                    p.set_mode(mode);
                    return true;
                }
            }
        }
        false
    }

    pub fn mcu_set_pin_mode(&mut self, pin_id: &str, mode: PinMode) -> bool {
        let is_out = matches!(mode, PinMode::Output | PinMode::Source | PinMode::OpenCo);
        let open_coll = matches!(mode, PinMode::OpenCo);
        for c in &mut self.components {
            match &mut c.kind {
                Kind::Mcu(m) => {
                    for g in m.device.gpio_pins_mut() {
                        if g.name == pin_id || pin_id.ends_with(&g.label) {
                            g.is_out = is_out;
                            g.open_coll = open_coll;
                        }
                    }
                    for p in &mut m.pins {
                        if p.id == pin_id {
                            p.set_pin_mode(mode);
                            return true;
                        }
                    }
                    if pin_id.starts_with(&c.id) {
                        let local = pin_id
                            .strip_prefix(&c.id)
                            .and_then(|s| s.strip_prefix('-'))
                            .unwrap_or(pin_id);
                        if let Some(g) = crate::mcu::match_gpio(m, &c.id, local) {
                            let g_name = g.name.clone();
                            for g_mut in m.device.gpio_pins_mut() {
                                if g_mut.name == g_name {
                                    g_mut.is_out = is_out;
                                    g_mut.open_coll = open_coll;
                                }
                            }
                            for p in &mut m.pins {
                                if p.id == g_name {
                                    p.set_pin_mode(mode);
                                    return true;
                                }
                            }
                        }
                    }
                }
                Kind::QemuDevice(q) => {
                    for p in &mut q.pins {
                        if p.id == pin_id {
                            p.set_pin_mode(mode);
                            return true;
                        }
                    }
                }
                Kind::McuPin(p) => {
                    if p.pin.id == pin_id || c.id == pin_id {
                        p.set_mode(mode);
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn mcu_set_pin_pullup(&mut self, pin_id: &str, pullup: bool) -> bool {
        for c in &mut self.components {
            match &mut c.kind {
                Kind::Mcu(m) => {
                    for g in m.device.gpio_pins_mut() {
                        if g.name == pin_id || pin_id.ends_with(&g.label) {
                            g.pullup = pullup;
                        }
                    }
                    for p in &mut m.pins {
                        if p.id == pin_id {
                            p.set_pullup(if pullup { 1e5 } else { 0.0 });
                            return true;
                        }
                    }
                    if pin_id.starts_with(&c.id) {
                        let local = pin_id
                            .strip_prefix(&c.id)
                            .and_then(|s| s.strip_prefix('-'))
                            .unwrap_or(pin_id);
                        if let Some(g) = crate::mcu::match_gpio(m, &c.id, local) {
                            let g_name = g.name.clone();
                            for g_mut in m.device.gpio_pins_mut() {
                                if g_mut.name == g_name {
                                    g_mut.pullup = pullup;
                                }
                            }
                            for p in &mut m.pins {
                                if p.id == g_name {
                                    p.set_pullup(if pullup { 1e5 } else { 0.0 });
                                    return true;
                                }
                            }
                        }
                    }
                }
                Kind::QemuDevice(q) => {
                    for p in &mut q.pins {
                        if p.id == pin_id {
                            p.set_pullup(if pullup { 1e5 } else { 0.0 });
                            return true;
                        }
                    }
                }
                Kind::McuPin(p) => {
                    if p.pin.id == pin_id || c.id == pin_id {
                        p.pin.set_pullup(if pullup { 1e5 } else { 0.0 });
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn mcu_pin_read(&mut self, id: &str) -> Option<bool> {
        let pin = format!("{id}-pin");
        let v = self.pin_voltage(&pin)?;
        for c in &mut self.components {
            if c.id == id {
                if let Kind::McuPin(p) = &mut c.kind {
                    return Some(p.read(v));
                }
            }
        }
        None
    }

    /// C++ `Tpd_ps` / `pd_n` on a logic device.
    pub fn set_prop_delay(&mut self, id: &str, seconds: f64) -> bool {
        for c in &mut self.components {
            if c.id == id {
                match &mut c.kind {
                    Kind::Gate(g) => {
                        g.family.set_prop_delay_s(seconds);
                        return true;
                    }
                    Kind::FlipFlop(f) => {
                        f.family.set_prop_delay_s(seconds);
                        return true;
                    }
                    Kind::Latch(l) => {
                        l.family.set_prop_delay_s(seconds);
                        return true;
                    }
                    Kind::Comparator { state } => {
                        state.family.set_prop_delay_s(seconds);
                        return true;
                    }
                    Kind::TestUnit(t) => {
                        t.family.set_prop_delay_s(seconds);
                        t.apply_family();
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// C++ TestUnit `Inputs` / `Outputs` / `Period` / `Truth`.

    pub fn scan_has_digital(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(
                &c.kind,
                Kind::Gate(_)
                    | Kind::FlipFlop(_)
                    | Kind::Latch(_)
                    | Kind::McuPin(_)
                    | Kind::Mcu(_)
                    | Kind::QemuDevice(_)
                    | Kind::ScriptCpu(_)
                    | Kind::Comparator { .. }
                    | Kind::TestUnit(_)
                    | Kind::Hd44780(_)
                    | Kind::Ssd1306(_)
                    | Kind::Aip31068(_)
                    | Kind::Sh1107(_)
                    | Kind::Pcd8544(_)
                    | Kind::Ks0108(_)
                    | Kind::Mux(_)
                    | Kind::Demux(_)
                    | Kind::BcdToDec(_)
                    | Kind::DecToBcd(_)
                    | Kind::BcdTo7S(_)
                    | Kind::I2CToParallel(_)
                    | Kind::Adc(_)
                    | Kind::Dac(_)
                    | Kind::Counter(_)
                    | Kind::BinCounter(_)
                    | Kind::FullAdder(_)
                    | Kind::HalfAdder(_)
                    | Kind::MagnitudeComp(_)
                    | Kind::ShiftReg(_)
                    | Kind::Function(_)
                    | Kind::Memory(_)
                    | Kind::DynamicMemory(_)
                    | Kind::I2CRam(_)
                    | Kind::Lm555(_)
                    | Kind::Servo { .. }
                    | Kind::Max72xx { .. }
                    | Kind::Ws2812 { .. }
                    | Kind::Sr04 { .. }
                    | Kind::Dht22 { .. }
                    | Kind::Ds18b20 { .. }
                    | Kind::Ds1621 { .. }
                    | Kind::Ds1307 { .. }
                    | Kind::Esp01 { .. }
            )
        })
    }

    pub fn has_digital(&self) -> bool {
        self.has_digital_cached
    }

    #[inline]

    pub(super) fn drain_logic_events(&mut self) -> Result<()> {
        let limit = self.circ_time.saturating_add(self.analog_ps());
        let mut n = 0u32;
        while n < self.max_nl_steps.max(1) {
            match self.events.peek_target() {
                Some(EventTarget::AnalogClock) | None => break,
                Some(_) => {}
            }
            let Some(t) = self.events.peek_time() else {
                break;
            };
            if t > limit {
                break;
            }
            let Some((t, target)) = self.events.pop_due(limit) else {
                break;
            };
            self.circ_time = t;
            let changed = self.run_event(target)?;
            if changed {
                self.solve_circuit()?;
            }
            n += 1;
        }
        Ok(())
    }

    pub(super) fn digital_volt_changed(&mut self) -> bool {
        if !self.has_digital() {
            return false;
        }
        let pin_net = &self.pin_net;
        let nodes = &self.nodes;
        let v_of = |pin: &str| -> f64 {
            pin_net
                .get(pin)
                .and_then(|&i| nodes.get(i))
                .map(|n| n.volt)
                .unwrap_or(0.0)
        };
        let t = self.circ_time;
        let mut stamp_changed = false;
        let mut periph_arms: Vec<(usize, crate::digital::Drive)> = Vec::new();
        let mut updates = std::mem::take(&mut self.digital_updates_buf);
        updates.clear();
        for (i, (c, cache)) in self.components.iter_mut().zip(&self.pin_caches).enumerate() {
            let comp_id = c.id.clone();
            let (u, pin_idx) = match &mut c.kind {
                Kind::Gate(g) => {
                    let idx = g.output_pin_index();
                    (g.volt_changed(&v_of, t), idx)
                }
                Kind::FlipFlop(f) => {
                    let idx = f.q_pin_index();
                    (f.volt_changed(&v_of, t), idx)
                }
                Kind::Latch(l) => (l.volt_changed(&v_of, t), l.channels),
                Kind::Comparator { state } => {
                    let vp = cache
                        .in0
                        .and_then(|idx| nodes.get(idx))
                        .map(|n| n.volt)
                        .unwrap_or(0.0);
                    let vn = cache
                        .in1
                        .and_then(|idx| nodes.get(idx))
                        .map(|n| n.volt)
                        .unwrap_or(0.0);
                    (state.schedule_from_compare(vp, vn, t), 0)
                }
                Kind::Mcu(m) => {
                    m.sample_inputs_nodes(nodes);
                    continue;
                }
                Kind::QemuDevice(q) => {
                    q.sample_inputs_nodes(nodes);
                    q.check_reset();
                    continue;
                }
                Kind::ScriptCpu(s) => {
                    s.volt_changed(&v_of);
                    continue;
                }
                Kind::Hd44780(h) => {
                    let v_en = v_of(&h.pin_en.id);
                    h.pin_en.get_inp_state(v_en);
                    let v_rs = v_of(&h.pin_rs.id);
                    h.pin_rs.get_inp_state(v_rs);
                    let v_rw = v_of(&h.pin_rw.id);
                    h.pin_rw.get_inp_state(v_rw);
                    for p in &mut h.data_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    h.step(t);
                    continue;
                }
                Kind::Ssd1306(s) => {
                    let v_scl = v_of(&s.pin_scl.id);
                    let v_sda = v_of(&s.pin_sda.id);
                    let old_scl = s.pin_scl.inp_state();
                    let old_sda = s.pin_sda.inp_state();
                    let new_scl = s.pin_scl.get_inp_state(v_scl);
                    let new_sda = s.pin_sda.get_inp_state(v_sda);
                    if old_scl != new_scl || old_sda != new_sda {
                        s.volt_changed_direct();
                    }
                    continue;
                }
                Kind::Aip31068(a) => {
                    let v_scl = v_of(&a.pin_scl.id);
                    let v_sda = v_of(&a.pin_sda.id);
                    a.pin_scl.get_inp_state(v_scl);
                    a.pin_sda.get_inp_state(v_sda);
                    let before = a.pin_sda.norton();
                    a.step(t);
                    if norton_moved(before, &a.pin_sda) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Sh1107(s) => {
                    let v_scl = v_of(&s.pin_scl.id);
                    let v_sda = v_of(&s.pin_sda.id);
                    s.pin_scl.get_inp_state(v_scl);
                    s.pin_sda.get_inp_state(v_sda);
                    let before = s.pin_sda.norton();
                    s.step(t);
                    if norton_moved(before, &s.pin_sda) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Pcd8544(p) => {
                    let v_rst = v_of(&p.pin_rst.id);
                    let v_cs = v_of(&p.pin_cs.id);
                    let v_dc = v_of(&p.pin_dc.id);
                    let v_si = v_of(&p.pin_si.id);
                    let v_scl = v_of(&p.pin_scl.id);
                    p.pin_rst.get_inp_state(v_rst);
                    p.pin_cs.get_inp_state(v_cs);
                    p.pin_dc.get_inp_state(v_dc);
                    p.pin_si.get_inp_state(v_si);
                    p.pin_scl.get_inp_state(v_scl);
                    p.step(t);
                    continue;
                }
                Kind::Ks0108(k) => {
                    let v_rst = v_of(&k.pin_rst.id);
                    let v_cs1 = v_of(&k.pin_cs1.id);
                    let v_cs2 = v_of(&k.pin_cs2.id);
                    let v_en = v_of(&k.pin_en.id);
                    let v_rw = v_of(&k.pin_rw.id);
                    let v_dc = v_of(&k.pin_dc.id);
                    k.pin_rst.get_inp_state(v_rst);
                    k.pin_cs1.get_inp_state(v_cs1);
                    k.pin_cs2.get_inp_state(v_cs2);
                    k.pin_en.get_inp_state(v_en);
                    k.pin_rw.get_inp_state(v_rw);
                    k.pin_dc.get_inp_state(v_dc);
                    for p in &mut k.data_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    k.step(t);
                    continue;
                }
                Kind::Mux(m) => {
                    for p in &mut m.inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    for p in &mut m.addr_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    if let Some(en) = &mut m.enable {
                        let v = v_of(&en.id);
                        en.get_inp_state(v);
                    }
                    let before_y = m.output.norton();
                    let before_yn = m.out_inverted.norton();
                    m.eval();
                    if norton_moved(before_y, &m.output) || norton_moved(before_yn, &m.out_inverted)
                    {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Demux(d) => {
                    let v = v_of(&d.input.id);
                    d.input.get_inp_state(v);
                    for p in &mut d.addr_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    if let Some(en) = &mut d.enable {
                        let v = v_of(&en.id);
                        en.get_inp_state(v);
                    }
                    let before = snap_nortons(&d.outputs);
                    d.eval();
                    if nortons_moved(&before, &d.outputs) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::BcdToDec(b) => {
                    for p in &mut b.inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let v_oe = v_of(&b.oe.id);
                    b.oe.get_inp_state(v_oe);
                    let before = snap_nortons(&b.outputs);
                    b.eval();
                    if nortons_moved(&before, &b.outputs) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::DecToBcd(d) => {
                    for p in &mut d.inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let v = v_of(&d.ei.id);
                    d.ei.get_inp_state(v);
                    let before = snap_nortons(&d.outputs);
                    let before_gs = d.gs.norton();
                    let before_eo = d.eo.norton();
                    d.eval();
                    if nortons_moved(&before, &d.outputs)
                        || norton_moved(before_gs, &d.gs)
                        || norton_moved(before_eo, &d.eo)
                    {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::BcdTo7S(b) => {
                    for p in &mut b.inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    // Lamp test and ripple blanking are not external pins.
                    // Hold them inactive (active-low).
                    b.lt.get_inp_state(5.0);
                    b.rbi.get_inp_state(5.0);
                    let v_bi = v_of(&b.bi_rbo.id);
                    b.bi_rbo.get_inp_state(v_bi);
                    let before = snap_nortons(&b.segments);
                    b.eval();
                    if nortons_moved(&before, &b.segments) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::I2CToParallel(p) => {
                    let v_scl = v_of(&p.scl.id);
                    let v_sda = v_of(&p.sda.id);
                    p.scl.get_inp_state(v_scl);
                    p.sda.get_inp_state(v_sda);
                    for pin in &mut p.ports {
                        let v = v_of(&pin.id);
                        pin.get_inp_state(v);
                    }
                    let before_ports = snap_nortons(&p.ports);
                    let before_sda = p.sda.norton();
                    let before_int = p.int_pin.norton();
                    p.poll();
                    if nortons_moved(&before_ports, &p.ports)
                        || norton_moved(before_sda, &p.sda)
                        || norton_moved(before_int, &p.int_pin)
                    {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Adc(a) => {
                    let v_in = cache
                        .in0
                        .and_then(|idx| nodes.get(idx))
                        .map(|n| n.volt)
                        .unwrap_or(0.0);
                    // Start-of-conversion and output-enable are not schematic pins.
                    // Keep the outputs driven.
                    a.oe.get_inp_state(5.0);
                    let before = snap_nortons(&a.outputs);
                    let before_eoc = a.eoc.norton();
                    a.convert(v_in);
                    if nortons_moved(&before, &a.outputs) || norton_moved(before_eoc, &a.eoc) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Dac(d) => {
                    for p in &mut d.inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let before = d.vout;
                    d.eval();
                    if (d.vout - before).abs() > 1e-12 {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Counter(c) => {
                    let v_clk = v_of(&c.clock.id);
                    let v_rst = v_of(&c.reset.id);
                    c.clock.get_inp_state(v_clk);
                    c.reset.get_inp_state(v_rst);
                    // Enable is not an external pin. An unconnected sample would stay low.
                    c.enable.get_inp_state(5.0);
                    let before_qs = snap_nortons(&c.outputs);
                    let before_out = c.output.norton();
                    c.eval();
                    if nortons_moved(&before_qs, &c.outputs) || norton_moved(before_out, &c.output)
                    {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::BinCounter(b) => {
                    let v_clka = v_of(&b.clk_a.id);
                    let v_rst = v_of(&b.r0_1.id);
                    b.clk_a.get_inp_state(v_clka);
                    b.r0_1.get_inp_state(v_rst);
                    // One schematic reset must satisfy the AND of the two internal resets.
                    // Section B is clocked from QA inside `eval`, not from this pin.
                    b.r0_2.get_inp_state(v_rst);
                    b.r9_1.get_inp_state(0.0);
                    b.r9_2.get_inp_state(0.0);
                    let before = snap_nortons(&b.outputs);
                    b.eval();
                    if nortons_moved(&before, &b.outputs) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::FullAdder(fa) => {
                    for p in &mut fa.a_inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    for p in &mut fa.b_inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let v_ci = v_of(&fa.ci.id);
                    fa.ci.get_inp_state(v_ci);
                    let before = snap_nortons(&fa.sum_outputs);
                    let before_co = fa.co.norton();
                    fa.eval();
                    if nortons_moved(&before, &fa.sum_outputs) || norton_moved(before_co, &fa.co) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::HalfAdder(fa) => {
                    for p in &mut fa.a_inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    for p in &mut fa.b_inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    fa.ci.get_inp_state(0.0);
                    let before = snap_nortons(&fa.sum_outputs);
                    let before_co = fa.co.norton();
                    fa.eval();
                    if nortons_moved(&before, &fa.sum_outputs) || norton_moved(before_co, &fa.co) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::MagnitudeComp(mc) => {
                    for p in &mut mc.a_inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    for p in &mut mc.b_inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let v_gt = v_of(&mc.cascade_gt.id);
                    let v_eq = v_of(&mc.cascade_eq.id);
                    let v_lt = v_of(&mc.cascade_lt.id);
                    mc.cascade_gt.get_inp_state(v_gt);
                    mc.cascade_eq.get_inp_state(v_eq);
                    mc.cascade_lt.get_inp_state(v_lt);
                    let before_gt = mc.out_gt.norton();
                    let before_eq = mc.out_eq.norton();
                    let before_lt = mc.out_lt.norton();
                    mc.eval();
                    if norton_moved(before_gt, &mc.out_gt)
                        || norton_moved(before_eq, &mc.out_eq)
                        || norton_moved(before_lt, &mc.out_lt)
                    {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::ShiftReg(sr) => {
                    let v_ds = v_of(&sr.ser_in.id);
                    let v_sh = v_of(&sr.clk_shift.id);
                    let v_mr = v_of(&sr.master_reset.id);
                    let v_oe = v_of(&sr.oe.id);
                    sr.ser_in.get_inp_state(v_ds);
                    sr.clk_shift.get_inp_state(v_sh);
                    // One schematic clock shifts and updates the stored outputs.
                    sr.clk_latch.get_inp_state(v_sh);
                    sr.master_reset.get_inp_state(v_mr);
                    sr.oe.get_inp_state(v_oe);
                    let before = snap_nortons(&sr.outputs);
                    let before_ser = sr.ser_out.norton();
                    sr.eval();
                    if nortons_moved(&before, &sr.outputs) || norton_moved(before_ser, &sr.ser_out)
                    {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Function(f) => {
                    for p in &mut f.inputs {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let before = f.output.norton();
                    f.eval();
                    if norton_moved(before, &f.output) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Memory(m) => {
                    for p in &mut m.addr_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    for p in &mut m.data_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let v_cs = v_of(&m.cs.id);
                    let v_oe = v_of(&m.oe.id);
                    let v_we = v_of(&m.we.id);
                    m.cs.get_inp_state(v_cs);
                    m.oe.get_inp_state(v_oe);
                    m.we.get_inp_state(v_we);
                    let before = snap_nortons(&m.data_pins);
                    m.eval();
                    if nortons_moved(&before, &m.data_pins) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::DynamicMemory(dm) => {
                    for p in &mut dm.addr_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    for p in &mut dm.data_pins {
                        let v = v_of(&p.id);
                        p.get_inp_state(v);
                    }
                    let v_ras = v_of(&dm.ras.id);
                    let v_cas = v_of(&dm.cas.id);
                    let v_we = v_of(&dm.we.id);
                    let v_oe = v_of(&dm.oe.id);
                    dm.ras.get_inp_state(v_ras);
                    dm.cas.get_inp_state(v_cas);
                    dm.we.get_inp_state(v_we);
                    dm.oe.get_inp_state(v_oe);
                    let before = snap_nortons(&dm.data_pins);
                    dm.eval();
                    if nortons_moved(&before, &dm.data_pins) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::I2CRam(r) => {
                    let v_scl = v_of(&r.scl.id);
                    let v_sda = v_of(&r.sda.id);
                    r.scl.get_inp_state(v_scl);
                    r.sda.get_inp_state(v_sda);
                    let before = r.sda.norton();
                    r.tick();
                    if norton_moved(before, &r.sda) {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Lm555(lm) => {
                    let v_node = |opt: Option<usize>| {
                        opt.and_then(|idx| nodes.get(idx))
                            .map(|n| n.volt)
                            .unwrap_or(0.0)
                    };
                    let v_gnd = v_node(cache.la_pins[0]);
                    let v_trg = v_node(cache.la_pins[1]);
                    let v_rst = v_node(cache.la_pins[3]);
                    let v_cv = v_node(cache.la_pins[4]);
                    let v_thr = v_node(cache.la_pins[5]);
                    let v_vcc = v_node(cache.la_pins[7]);
                    let before = (lm.out_state, lm.discharge_on, lm.out_high_v, lm.out_low_v);
                    lm.step(v_gnd, v_vcc, v_trg, v_thr, Some(v_cv), v_rst);
                    let level_moved = (lm.out_high_v - before.2).abs() > 1e-12
                        || (lm.out_low_v - before.3).abs() > 1e-12;
                    if lm.out_state != before.0 || lm.discharge_on != before.1 || level_moved {
                        stamp_changed = true;
                    }
                    continue;
                }
                Kind::Servo {
                    min_pulse,
                    max_pulse,
                    target_pos,
                    pulse_start_ps,
                    sig_high,
                    ..
                } => {
                    let v_node = |opt: Option<usize>| {
                        opt.and_then(|idx| nodes.get(idx))
                            .map(|n| n.volt)
                            .unwrap_or(0.0)
                    };
                    let v_vplus = v_node(cache.left);
                    let v_gnd = v_node(cache.mid);
                    let v_sig = v_node(cache.right);
                    let is_powered = (v_vplus - v_gnd) > 2.5;
                    let is_high = (v_sig - v_gnd) > 2.0;

                    if !is_powered {
                        *target_pos = 90.0;
                        *pulse_start_ps = 0;
                        *sig_high = false;
                    } else if !*sig_high && is_high {
                        // Rising edge
                        *pulse_start_ps = t;
                        *sig_high = true;
                    } else if *sig_high && !is_high {
                        // Falling edge
                        if *pulse_start_ps > 0 && t >= *pulse_start_ps {
                            let duration_us = (t - *pulse_start_ps) as f64 / 1_000_000.0;
                            let target =
                                (duration_us - *min_pulse) * 180.0 / (*max_pulse - *min_pulse);
                            *target_pos = target.clamp(0.0, 180.0);
                        }
                        *pulse_start_ps = 0;
                        *sig_high = false;
                    }
                    continue;
                }
                other => {
                    let drive = super::periph_proto::on_pins(other, &comp_id, &v_of, t);
                    if drive.changed {
                        stamp_changed = true;
                    }
                    if drive.arm_at.is_some() {
                        periph_arms.push((i, drive));
                    }
                    continue;
                }
            };
            if u.stamp_changed {
                stamp_changed = true;
            }
            updates.push((i, u, pin_idx));
        }
        for (i, u, pin_idx) in &updates {
            self.apply_update(*i, *u, *pin_idx);
        }
        for (i, drive) in periph_arms {
            let Some(at) = drive.arm_at else {
                continue;
            };
            let target = EventTarget::Device(i);
            if drive.replace {
                self.events.cancel(target);
            }
            self.events.add(at, target);
        }
        self.digital_updates_buf = updates;
        stamp_changed
    }

    pub(super) fn apply_update(&mut self, comp: usize, u: GateUpdate, pin_idx: usize) {
        if let Some(d) = u.device_event {
            self.events
                .add(self.circ_time.saturating_add(d), EventTarget::Device(comp));
        }
        self.apply_pin_action(comp, pin_idx, u.pin_action);
    }

    pub(super) fn apply_pin_action(&mut self, comp: usize, pin_idx: usize, action: PinAction) {
        match action {
            PinAction::None | PinAction::Immediate => {}
            PinAction::Event { delay_ps, cancel } => {
                let target = EventTarget::Pin { comp, pin: pin_idx };
                if cancel {
                    self.events.cancel(target);
                }
                self.events
                    .add(self.circ_time.saturating_add(delay_ps), target);
            }
        }
    }

    pub(super) fn run_event(&mut self, target: EventTarget) -> Result<bool> {
        match target {
            EventTarget::AnalogClock => self.analog_clock_event(),
            EventTarget::Device(i) => {
                let mcu_step = if let Some(Kind::Mcu(m)) =
                    self.components.get_mut(i).map(|c| &mut c.kind)
                {
                    let is_debug = crate::debug::DebugSession::is_active_global();
                    let next_deadline = self.events.peek_time().unwrap_or(u64::MAX);
                    let mut steps = 0usize;
                    m.drain_host_serial();
                    loop {
                        if self.nodes_dirty {
                            m.sample_inputs_nodes(&self.nodes);
                        }
                        let prev_pc = m.device.pc();
                        if m.device.state == cs_mcu::McuState::Running {
                            m.device.advance();
                        }
                        if is_debug {
                            let pc = m.device.pc();
                            let sp = m.device.sp();
                            let ret_addr = m.device.ret_addr();
                            let _ = crate::debug::DebugSession::global()
                                .on_mcu_step(pc, prev_pc, sp, ret_addr);
                            break;
                        }
                        if m.device.state != cs_mcu::McuState::Running || m.device.ports_dirty() {
                            break;
                        }
                        let delta = m.device.next_event_ps();
                        let max_step_ps = m.device.ps_tick.saturating_mul(4);
                        if delta > max_step_ps
                            || self.circ_time.saturating_add(delta) >= next_deadline
                            || steps >= 2048
                        {
                            break;
                        }
                        self.circ_time = self.circ_time.saturating_add(delta);
                        steps += 1;
                    }
                    m.flush_periph_logs();
                    self.mcu_actions_buf.clear();
                    m.sync_gpio_to_pins_buf(&mut self.mcu_actions_buf);
                    let next = if m.device.state == cs_mcu::McuState::Running {
                        Some(m.device.next_event_ps())
                    } else {
                        None
                    };
                    Some(next)
                } else {
                    None
                };
                if let Some(next_ps) = mcu_step {
                    let mut changed = false;
                    for idx in 0..self.mcu_actions_buf.len() {
                        let (pin, action) = self.mcu_actions_buf[idx];
                        if !matches!(action, PinAction::None) {
                            changed = true;
                        }
                        self.apply_pin_action(i, pin, action);
                    }
                    if let Some(ps) = next_ps {
                        self.events
                            .add(self.circ_time.saturating_add(ps), EventTarget::Device(i));
                    }
                    return Ok(changed);
                }
                if let Some(c) = self.components.get_mut(i) {
                    if let Kind::AudioOut { source, last_v } = &mut c.kind {
                        let nodes = &self.nodes;
                        let pc = self.pin_caches.get(i);
                        let v_l = pc
                            .and_then(|p| p.left)
                            .and_then(|j| nodes.get(j))
                            .map(|n| n.volt)
                            .unwrap_or(0.0);
                        let v_r = pc
                            .and_then(|p| p.right)
                            .and_then(|j| nodes.get(j))
                            .map(|n| n.volt)
                            .unwrap_or(0.0);
                        let volt_pn = v_l - v_r;
                        *last_v = volt_pn;
                        let chunk = source.push(volt_pn, self.circ_time);
                        if let Some(chunk) = chunk {
                            if let Some(sink) = &self.audio_sink {
                                if let Ok(mut s) = sink.lock() {
                                    s.write(&chunk);
                                }
                            }
                        }
                        let sample_rate = self
                            .audio_sink
                            .as_ref()
                            .and_then(|s| s.lock().ok().map(|s| s.sample_rate()))
                            .unwrap_or(crate::audio::SAMPLE_RATE);
                        let audio_period_ps =
                            crate::audio::sample_period_ps(sample_rate, self.ps_per_sec).max(1);
                        self.events.add(
                            self.circ_time.saturating_add(audio_period_ps),
                            EventTarget::Device(i),
                        );
                        return Ok(false);
                    }
                    if let Kind::WaveGen { .. } = &c.kind {
                        let v = c.wavegen_calc_vout(self.circ_time);
                        let delta = c.wavegen_next_event_ps(self.circ_time);
                        let mut changed = false;
                        if let Kind::WaveGen { v_out, .. } = &mut c.kind {
                            if (v - *v_out).abs() > 1e-12 {
                                *v_out = v;
                                changed = true;
                            }
                        }
                        if let Some(dt) = delta {
                            self.events
                                .add(self.circ_time.saturating_add(dt), EventTarget::Device(i));
                        }
                        return Ok(changed);
                    }
                    if let Kind::Clock { .. } = &c.kind {
                        let high = c.clock_is_high(self.circ_time);
                        let delta = c.clock_next_event_ps(self.circ_time);
                        let mut changed = false;
                        if let Kind::Clock { state, .. } = &mut c.kind {
                            if *state != high {
                                *state = high;
                                changed = true;
                            }
                        }
                        if let Some(dt) = delta {
                            self.events
                                .add(self.circ_time.saturating_add(dt), EventTarget::Device(i));
                        }
                        return Ok(changed);
                    }
                }
                let qemu_step = if let Some(c) = self.components.get_mut(i) {
                    if let Kind::QemuDevice(q) = &mut c.kind {
                        self.mcu_actions_buf.clear();
                        q.run_event_nodes_buf(
                            &self.nodes,
                            self.circ_time,
                            &mut self.mcu_actions_buf,
                        );
                        let changed = q.take_pins_dirty();
                        let next = q.next_event_delay(self.circ_time);
                        Some((changed, next))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((mut changed, next_ps)) = qemu_step {
                    for idx in 0..self.mcu_actions_buf.len() {
                        let (pin, action) = self.mcu_actions_buf[idx];
                        if !matches!(action, PinAction::None) {
                            changed = true;
                        }
                        self.apply_pin_action(i, pin, action);
                    }
                    if let Some(ps) = next_ps {
                        self.events
                            .add(self.circ_time.saturating_add(ps), EventTarget::Device(i));
                    }
                    return Ok(changed);
                }
                let t = self.circ_time;
                let (u, mcu_result) = {
                    let pin_net = &self.pin_net;
                    let nodes = &self.nodes;
                    let v_of = |pin: &str| -> f64 {
                        pin_net
                            .get(pin)
                            .and_then(|&j| nodes.get(j))
                            .map(|n| n.volt)
                            .unwrap_or(0.0)
                    };
                    match self.components.get_mut(i).map(|c| &mut c.kind) {
                        Some(Kind::ScriptCpu(s)) => {
                            s.circ_time = t;
                            let actions = s.run_event(&v_of);
                            let next = s.take_event().unwrap_or(s.ps_tick).max(1);
                            (None, Some((actions, Some(next), false)))
                        }
                        Some(Kind::Gate(g)) => (Some((g.run_event(t), g.output_pin_index())), None),
                        Some(Kind::FlipFlop(f)) => (Some((f.run_event(t), f.q_pin_index())), None),
                        Some(Kind::Latch(l)) => (Some((l.run_event(t), l.channels)), None),
                        Some(Kind::Comparator { state }) => (Some((state.run_event(t), 0)), None),
                        Some(Kind::TestUnit(tu)) => (Some((tu.run_event(&v_of, t), 0)), None),
                        _ => (None, None),
                    }
                };
                if let Some((actions, next_ps, always_changed)) = mcu_result {
                    let mut changed = always_changed;
                    for (pin, action) in actions {
                        if !matches!(action, PinAction::None) {
                            changed = true;
                        }
                        self.apply_pin_action(i, pin, action);
                    }
                    if let Some(ps) = next_ps {
                        self.events
                            .add(self.circ_time.saturating_add(ps), EventTarget::Device(i));
                    }
                    return Ok(changed);
                }
                if let Some((u, pin_idx)) = u {
                    let changed = u.stamp_changed || !matches!(u.pin_action, PinAction::None);
                    self.apply_update(i, u, pin_idx);
                    return Ok(changed);
                }
                let id = self
                    .components
                    .get(i)
                    .map(|c| c.id.clone())
                    .unwrap_or_default();
                let pin_net = &self.pin_net;
                let nodes = &self.nodes;
                let v_of = |pin: &str| -> f64 {
                    pin_net
                        .get(pin)
                        .and_then(|&j| nodes.get(j))
                        .map(|n| n.volt)
                        .unwrap_or(0.0)
                };
                let drive = self
                    .components
                    .get_mut(i)
                    .and_then(|c| super::periph_proto::on_event(&mut c.kind, &id, v_of, t));
                if let Some(drive) = drive {
                    if let Some(at) = drive.arm_at {
                        self.events.add(at, EventTarget::Device(i));
                    }
                    return Ok(drive.changed);
                }
                Ok(false)
            }
            EventTarget::Pin { comp, pin } => {
                let action = self
                    .components
                    .get_mut(comp)
                    .and_then(|c| match &mut c.kind {
                        Kind::Gate(g) => g.pin_at_mut(pin).map(|p| p.run_slope()),
                        Kind::FlipFlop(f) => f.pin_at_mut(pin).map(|p| p.run_slope()),
                        Kind::Latch(l) => l.pin_at_mut(pin).map(|p| p.run_slope()),
                        Kind::Comparator { state } => Some(state.output.run_slope()),
                        Kind::McuPin(p) => Some(p.pin.run_slope()),
                        Kind::Mcu(m) => m.pins.get_mut(pin).map(|p| p.run_slope()),
                        Kind::QemuDevice(q) => q.pins.get_mut(pin).map(|p| p.run_slope()),
                        Kind::ScriptCpu(s) => {
                            let mut n = 0;
                            if pin < s.pins.len() {
                                s.pins.get_mut(pin).map(|p| p.run_slope())
                            } else {
                                n += s.pins.len();
                                let mut found = None;
                                for port in &mut s.ports {
                                    if pin < n + port.pins.len() {
                                        found = port.pins.get_mut(pin - n).map(|p| p.run_slope());
                                        break;
                                    }
                                    n += port.pins.len();
                                }
                                found
                            }
                        }
                        Kind::TestUnit(tu) => {
                            let n = tu.drive.len();
                            if pin < n {
                                tu.drive.get_mut(pin).map(|p| p.run_slope())
                            } else {
                                tu.sense.get_mut(pin - n).map(|p| p.run_slope())
                            }
                        }
                        _ => None,
                    });
                if let Some(a) = action {
                    self.apply_pin_action(comp, pin, a);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}
