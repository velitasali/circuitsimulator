//! MNA matrix stamping, linear solving, and node voltage / pin current probing.

use rustc_hash::FxHashMap;

use crate::elements::Kind;
use crate::elements::pins::split_comp_pin_suffix;
use crate::instruments::ScopeSampler;
use crate::matrix::CircMatrix;
use crate::net::{ENode, UnionFind};
use crate::{Error, Result};

use super::{CachedInstrument, Circuit};

impl Circuit {
    pub(super) fn solve_circuit(&mut self) -> Result<()> {
        let n = self.nodes.len();
        if n == 0 {
            return Ok(());
        }
        self.update_sources();
        // Nonlinear parts Newton-iterate. A linear circuit still re-stamps when a
        // digital output changes: one pass settles a single IC, and a combinational
        // chain needs one pass per stage. The cap stops a ring oscillator from
        // looping forever.
        const DIGITAL_SETTLE_ITERS: usize = 32;
        let max = if self.has_nonlinear_cached {
            self.max_nl_steps.max(1) as usize
        } else {
            DIGITAL_SETTLE_ITERS
        };
        for _ in 0..max {
            for node in &mut self.nodes {
                node.clear_stamps();
            }
            for (c, cache) in self.components.iter().zip(&self.pin_caches) {
                c.stamp_with_cache(cache, &self.pin_net, &mut self.nodes, self.dt);
            }

            self.matrix.clear_stamps();
            for node in &self.nodes {
                node.stamp_into(&mut self.matrix);
            }

            let ok = self.matrix.solve(&mut self.volts_buf);
            for (node, &v) in self.nodes.iter_mut().zip(&self.volts_buf) {
                node.volt = v;
            }
            self.nodes_dirty = true;
            if !ok {
                return Err(Error::Singular);
            }
            let analog_ok = self.update_nonlinear();
            if !analog_ok {
                continue;
            }
            if self.digital_volt_changed() {
                continue;
            }
            return Ok(());
        }
        Err(Error::NotConverged)
    }

    pub fn pin_voltage(&self, pin: &str) -> Option<f64> {
        if let Some(&idx) = self.pin_net.get(pin) {
            return Some(self.nodes[idx].volt);
        }
        for c in &self.components {
            match &c.kind {
                Kind::Mcu(m) => {
                    for p in &m.pins {
                        if p.id == pin {
                            return Some(p.voltage());
                        }
                    }
                    if pin.starts_with(&c.id) {
                        let local = pin
                            .strip_prefix(&c.id)
                            .and_then(|s| s.strip_prefix('-'))
                            .unwrap_or(pin);
                        if let Some(g) = crate::mcu::match_gpio(m, &c.id, local) {
                            for p in &m.pins {
                                if p.id == g.name {
                                    return Some(p.voltage());
                                }
                            }
                        }
                    }
                }
                Kind::QemuDevice(q) => {
                    for p in &q.pins {
                        if p.id == pin {
                            return Some(p.voltage());
                        }
                    }
                }
                Kind::ScriptCpu(s) => {
                    for p in s
                        .pins
                        .iter()
                        .chain(s.ports.iter().flat_map(|po| po.pins.iter()))
                    {
                        if p.id == pin {
                            return Some(p.voltage());
                        }
                    }
                }
                Kind::McuPin(p) => {
                    if p.pin.id == pin {
                        return Some(p.pin.voltage());
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn pin_direction(&self, pin: &str) -> Option<crate::canvas::PinDirection> {
        use crate::canvas::PinDirection;
        use crate::digital::PinMode;
        for c in &self.components {
            match &c.kind {
                Kind::Mcu(m) => {
                    for p in &m.pins {
                        if p.id == pin {
                            return match p.mode {
                                PinMode::Input => Some(PinDirection::In),
                                PinMode::Output | PinMode::Source => Some(PinDirection::Out),
                                PinMode::OpenCo => Some(PinDirection::OpenCo),
                                PinMode::Undef => None,
                            };
                        }
                    }
                    if pin.starts_with(&c.id) {
                        let local = pin
                            .strip_prefix(&c.id)
                            .and_then(|s| s.strip_prefix('-'))
                            .unwrap_or(pin);
                        if let Some(g) = crate::mcu::match_gpio(m, &c.id, local) {
                            for p in &m.pins {
                                if p.id == g.name {
                                    return match p.mode {
                                        PinMode::Input => Some(PinDirection::In),
                                        PinMode::Output | PinMode::Source => {
                                            Some(PinDirection::Out)
                                        }
                                        PinMode::OpenCo => Some(PinDirection::OpenCo),
                                        PinMode::Undef => None,
                                    };
                                }
                            }
                        }
                    }
                }
                Kind::QemuDevice(q) => {
                    for p in &q.pins {
                        if p.id == pin {
                            return match p.mode {
                                PinMode::Input => Some(PinDirection::In),
                                PinMode::Output | PinMode::Source => Some(PinDirection::Out),
                                PinMode::OpenCo => Some(PinDirection::OpenCo),
                                PinMode::Undef => None,
                            };
                        }
                    }
                }
                Kind::ScriptCpu(s) => {
                    for p in s
                        .pins
                        .iter()
                        .chain(s.ports.iter().flat_map(|po| po.pins.iter()))
                    {
                        if p.id == pin {
                            return match p.mode {
                                PinMode::Input => Some(PinDirection::In),
                                PinMode::Output | PinMode::Source => Some(PinDirection::Out),
                                PinMode::OpenCo => Some(PinDirection::OpenCo),
                                PinMode::Undef => None,
                            };
                        }
                    }
                }
                Kind::McuPin(p) => {
                    if p.pin.id == pin {
                        return match p.pin.mode {
                            PinMode::Input => Some(PinDirection::In),
                            PinMode::Output | PinMode::Source => Some(PinDirection::Out),
                            PinMode::OpenCo => Some(PinDirection::OpenCo),
                            PinMode::Undef => None,
                        };
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn pin_has_pullup(&self, pin: &str) -> bool {
        for c in &self.components {
            match &c.kind {
                Kind::Mcu(m) => {
                    for p in &m.pins {
                        if p.id == pin {
                            return p.has_pullup();
                        }
                    }
                    if pin.starts_with(&c.id) {
                        let local = pin
                            .strip_prefix(&c.id)
                            .and_then(|s| s.strip_prefix('-'))
                            .unwrap_or(pin);
                        if let Some(g) = crate::mcu::match_gpio(m, &c.id, local) {
                            for p in &m.pins {
                                if p.id == g.name {
                                    return p.has_pullup();
                                }
                            }
                        }
                    }
                }
                Kind::QemuDevice(q) => {
                    for p in &q.pins {
                        if p.id == pin {
                            return p.has_pullup();
                        }
                    }
                }
                Kind::ScriptCpu(s) => {
                    for p in s
                        .pins
                        .iter()
                        .chain(s.ports.iter().flat_map(|po| po.pins.iter()))
                    {
                        if p.id == pin {
                            return p.has_pullup();
                        }
                    }
                }
                Kind::McuPin(p) => {
                    if p.pin.id == pin {
                        return p.pin.has_pullup();
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn resistor_current(&self, id: &str) -> Option<f64> {
        let (comp, cache) = self
            .components
            .iter()
            .zip(&self.pin_caches)
            .find(|(c, _)| c.id == id)?;
        let Kind::Resistor { resistance } = &comp.kind else {
            return None;
        };
        let v0 = cache.left.and_then(|i| self.nodes.get(i)).map(|n| n.volt)?;
        let v1 = cache
            .right
            .and_then(|i| self.nodes.get(i))
            .map(|n| n.volt)?;
        Some((v0 - v1) / *resistance)
    }

    /// Current leaving `pin` into a connected wire (amperes).
    pub fn current_out_of_pin(&self, pin: &str) -> Option<f64> {
        for (c, cache) in self.components.iter().zip(&self.pin_caches) {
            let Some(suffix) = split_comp_pin_suffix(&c.id, pin) else {
                continue;
            };
            if let Some(i) =
                c.kind
                    .current_out_of_pin(&c.id, pin, suffix, cache, &self.nodes, self.dt, |p| {
                        self.pin_voltage(p)
                    })
            {
                return Some(i);
            }
        }
        None
    }

    pub(super) fn build_nets(&mut self) {
        self.nodes.clear();
        self.pin_net.clear();

        let mut uf = UnionFind::default();
        for (a, b) in &self.connectors {
            uf.union(a, b);
        }
        let mut tunnels: FxHashMap<String, Vec<String>> = FxHashMap::default();
        for c in &self.components {
            match &c.kind {
                Kind::Tunnel { name, pin_id } => {
                    if !name.is_empty() {
                        tunnels
                            .entry(name.clone())
                            .or_default()
                            .push(pin_id.clone());
                    }
                }
                Kind::Oscope {
                    tunnels: ch_tunnels,
                    ..
                } => {
                    for (i, tun) in ch_tunnels.iter().enumerate() {
                        if !tun.is_empty() {
                            tunnels.entry(tun.clone()).or_default().push(c.plot_pin(i));
                        }
                    }
                }
                Kind::LAnalizer {
                    tunnels: ch_tunnels,
                    ..
                } => {
                    for (i, tun) in ch_tunnels.iter().enumerate() {
                        if !tun.is_empty() {
                            tunnels.entry(tun.clone()).or_default().push(c.plot_pin(i));
                        }
                    }
                }
                _ => {}
            }
        }
        for pins in tunnels.values() {
            if let Some(first) = pins.first() {
                for p in pins {
                    uf.union(first, p);
                }
            }
        }
        for c in &self.components {
            if matches!(&c.kind, Kind::Junction) {
                let pins = c.pin_ids();
                let present: Vec<String> = pins.into_iter().filter(|p| uf.contains(p)).collect();
                for w in present.windows(2) {
                    uf.union(&w[0], &w[1]);
                }
            }
            if matches!(&c.kind, Kind::Diode { .. }) {
                let left = c.left_pin();
                let right = c.right_pin();
                if uf.contains(&left) || uf.contains(&right) {
                    uf.add(&c.mid_pin());
                }
            }
            if matches!(&c.kind, Kind::Potentiometer { .. }) {
                // Saved wires still use PinA/PinB/PinM. `pin_ids` lists only
                // lPin/rPin/wPin, and the connection graph is built from
                // `pin_ids`, so an alias-only net would be its own island.
                let pa = c.pot_pin_a();
                let pb = c.pot_pin_b();
                let pm = c.pot_pin_m();
                let l = format!("{}-lPin", c.id);
                let r = format!("{}-rPin", c.id);
                let w = format!("{}-wPin", c.id);
                if uf.contains(&pa) {
                    uf.union(&pa, &l);
                }
                if uf.contains(&pb) {
                    uf.union(&pb, &r);
                }
                if uf.contains(&pm) {
                    uf.union(&pm, &w);
                }
                if !uf.contains(&w) && (uf.contains(&l) || uf.contains(&r)) {
                    uf.add(&w);
                }
            }
            if matches!(&c.kind, Kind::TouchPad { .. }) {
                let xp = format!("{}-vrx_p", c.id);
                let xm = format!("{}-vrx_m", c.id);
                let yp = format!("{}-vry_p", c.id);
                let ym = format!("{}-vry_m", c.id);
                if uf.contains(&xp) || uf.contains(&xm) || uf.contains(&yp) || uf.contains(&ym) {
                    uf.add(&format!("{}-nodeX", c.id));
                    uf.add(&format!("{}-nodeY", c.id));
                }
            }
            // Older wires used these suffixes. Union them onto the schematic pin
            // so both names land on one node. `pin_ids` lists only the schematic name.
            let bind_alias = |uf: &mut crate::net::UnionFind, alias: &str, canon: &str| {
                if uf.contains(alias) {
                    uf.union(alias, canon);
                }
            };
            match &c.kind {
                Kind::Ws2812 { .. } => {
                    bind_alias(&mut uf, &format!("{}-vdd", c.id), &format!("{}-vcc", c.id));
                }
                Kind::Scr { .. } => {
                    bind_alias(
                        &mut uf,
                        &format!("{}-lPin", c.id),
                        &format!("{}-aPin", c.id),
                    );
                    bind_alias(
                        &mut uf,
                        &format!("{}-rPin", c.id),
                        &format!("{}-kPin", c.id),
                    );
                }
                Kind::Triac { .. } => {
                    bind_alias(
                        &mut uf,
                        &format!("{}-mt1Pin", c.id),
                        &format!("{}-lPin", c.id),
                    );
                    bind_alias(
                        &mut uf,
                        &format!("{}-mt2Pin", c.id),
                        &format!("{}-rPin", c.id),
                    );
                }
                Kind::SevenSegment { .. } => {
                    for (alias, canon) in [
                        ("a", "pin_a"),
                        ("b", "pin_b"),
                        ("c", "pin_c"),
                        ("d", "pin_d"),
                        ("e", "pin_e"),
                        ("f", "pin_f"),
                        ("g", "pin_g"),
                        ("dp", "pin_dot"),
                        ("com", "pin_commona"),
                    ] {
                        bind_alias(
                            &mut uf,
                            &format!("{}-{alias}", c.id),
                            &format!("{}-{canon}", c.id),
                        );
                    }
                }
                Kind::AnalogMux { channels, .. } => {
                    let ch = (*channels).max(2);
                    bind_alias(
                        &mut uf,
                        &format!("{}-com", c.id),
                        &format!("{}-PinInput", c.id),
                    );
                    bind_alias(
                        &mut uf,
                        &format!("{}-en", c.id),
                        &format!("{}-PinEnable", c.id),
                    );
                    for i in 0..ch {
                        bind_alias(
                            &mut uf,
                            &format!("{}-ch{i}", c.id),
                            &format!("{}-pinY{i}", c.id),
                        );
                    }
                }
                _ => {}
            }
            if let Kind::Switch {
                poles,
                double_throw,
                ..
            } = &c.kind
            {
                let n = (*poles).max(1);
                for i in 0..n {
                    let pin_p = format!("{}-pinP{i}", c.id);
                    let l_pin = format!("{}-lPin{i}", c.id);
                    if uf.contains(&pin_p) && uf.contains(&l_pin) {
                        uf.union(&pin_p, &l_pin);
                    } else if uf.contains(&l_pin) {
                        uf.union(&l_pin, &pin_p);
                    }
                    if !*double_throw {
                        let sw_n = format!("{}-switch{i}pinN", c.id);
                        let r_pin = format!("{}-rPin{i}", c.id);
                        if uf.contains(&sw_n) && uf.contains(&r_pin) {
                            uf.union(&sw_n, &r_pin);
                        } else if uf.contains(&r_pin) {
                            uf.union(&r_pin, &sw_n);
                        }
                    }
                }
                if n == 1 {
                    let pin_p = format!("{}-pinP0", c.id);
                    let l_pin = format!("{}-lPin", c.id);
                    if uf.contains(&pin_p) && uf.contains(&l_pin) {
                        uf.union(&pin_p, &l_pin);
                    } else if uf.contains(&l_pin) {
                        uf.union(&l_pin, &pin_p);
                    }
                    if !*double_throw {
                        let sw_n = format!("{}-switch0pinN", c.id);
                        let r_pin = format!("{}-rPin", c.id);
                        if uf.contains(&sw_n) && uf.contains(&r_pin) {
                            uf.union(&sw_n, &r_pin);
                        } else if uf.contains(&r_pin) {
                            uf.union(&r_pin, &sw_n);
                        }
                    }
                }
            }
            if let Kind::Header { pins_count } | Kind::Socket { pins_count } = &c.kind {
                // Each row is one conductor: the lead `pin{i}` and the body end `pin{i+n}`.
                join_connector_rows(&mut uf, &c.id, *pins_count);
            }
            if let Kind::Push { poles, .. } = &c.kind {
                let n = (*poles).max(1);
                for i in 0..n {
                    let pin_p = format!("{}-pinP{i}", c.id);
                    let l_pin = format!("{}-lPin{i}", c.id);
                    if uf.contains(&pin_p) && uf.contains(&l_pin) {
                        uf.union(&pin_p, &l_pin);
                    } else if uf.contains(&pin_p) {
                        uf.union(&pin_p, &l_pin);
                    }
                    let sw_n = format!("{}-switch{i}pinN", c.id);
                    let r_pin = format!("{}-rPin{i}", c.id);
                    if uf.contains(&sw_n) && uf.contains(&r_pin) {
                        uf.union(&sw_n, &r_pin);
                    } else if uf.contains(&sw_n) {
                        uf.union(&sw_n, &r_pin);
                    }
                }
                if n == 1 {
                    let pin_p = format!("{}-pinP0", c.id);
                    let l_pin = format!("{}-lPin", c.id);
                    if uf.contains(&pin_p) && uf.contains(&l_pin) {
                        uf.union(&pin_p, &l_pin);
                    } else if uf.contains(&pin_p) {
                        uf.union(&pin_p, &l_pin);
                    }
                    let sw_n = format!("{}-switch0pinN", c.id);
                    let r_pin = format!("{}-rPin", c.id);
                    if uf.contains(&sw_n) && uf.contains(&r_pin) {
                        uf.union(&sw_n, &r_pin);
                    } else if uf.contains(&sw_n) {
                        uf.union(&sw_n, &r_pin);
                    }
                }
            }
        }

        // A bus is a bundle, not one node. The two trunk ends are the same spine.
        // Segments whose trunks share a net share each line (`ePin1` is line 0).
        let buses: Vec<(String, usize)> = self
            .components
            .iter()
            .filter_map(|c| match &c.kind {
                Kind::Bus { width } => Some((c.id.clone(), *width)),
                _ => None,
            })
            .collect();
        join_bus_lines(&mut uf, &buses);

        let groups = uf.groups();
        for pins in groups {
            // C++ `createNodes` starts a net only from a non-Node pin that has
            // a connector. A net of only Node-* pins is dropped.
            let has_device = pins.iter().any(|p| !p.starts_with("Node"));
            if !has_device {
                continue;
            }
            let num = self.nodes.len();
            let mut node = ENode::new(format!("eNodeSim-{num}"), num);
            for p in &pins {
                self.pin_net.insert(p.clone(), num);
                node.pin_ids.push(p.clone());
            }
            self.nodes.push(node);
        }

        let n = self.nodes.len();
        self.matrix = CircMatrix::new(n);
        self.volts_buf = vec![0.0; n];
        let mut connections = vec![Vec::new(); n];
        for c in &self.components {
            let mut comp_nodes = Vec::with_capacity(4);
            for p in c.pin_ids() {
                if let Some(&node_idx) = self.pin_net.get(&p) {
                    if !comp_nodes.contains(&node_idx) {
                        comp_nodes.push(node_idx);
                    }
                }
            }
            for i in 0..comp_nodes.len() {
                for j in (i + 1)..comp_nodes.len() {
                    let u = comp_nodes[i];
                    let v = comp_nodes[j];
                    if !connections[u].contains(&v) {
                        connections[u].push(v);
                    }
                    if !connections[v].contains(&u) {
                        connections[v].push(u);
                    }
                }
            }
        }
        self.matrix.analyze(&connections);
        self.pin_caches = self
            .components
            .iter()
            .map(|c| c.build_pin_cache(&self.pin_net))
            .collect();
        self.has_instruments_cached = self.scan_has_instruments();
        self.has_digital_cached = self.scan_has_digital();
        self.has_nonlinear_cached = self.scan_has_nonlinear();

        let mut instrument_caches = Vec::new();
        let mut time_sources_cached = Vec::new();
        let mut has_motors = false;

        for (idx, c) in self.components.iter().enumerate() {
            match &c.kind {
                Kind::WaveGen { .. } | Kind::Clock { .. } => {
                    time_sources_cached.push(idx);
                }
                Kind::DcMotor { .. } | Kind::Servo { .. } | Kind::Stepper { .. } => {
                    has_motors = true;
                }
                Kind::Oscope { .. } => {
                    let gnd_pin = c.plot_gnd_pin();
                    let gnd_node = self.pin_net.get(&gnd_pin).copied();
                    let mut pins = [None; 4];
                    for i in 0..4 {
                        let p = c.plot_pin(i);
                        pins[i] = self.pin_net.get(&p).copied();
                    }
                    instrument_caches.push(CachedInstrument::Oscope {
                        comp_id: c.id.clone(),
                        gnd_node,
                        pins,
                    });
                    self.instruments
                        .scopes
                        .entry(c.id.clone())
                        .or_insert_with(ScopeSampler::scope);
                }
                Kind::LAnalizer { .. } => {
                    let mut pins = [None; 8];
                    for i in 0..8 {
                        let p = c.plot_pin(i);
                        pins[i] = self.pin_net.get(&p).copied();
                    }
                    instrument_caches.push(CachedInstrument::LAnalizer {
                        comp_id: c.id.clone(),
                        pins,
                    });
                    self.instruments
                        .las
                        .entry(c.id.clone())
                        .or_insert_with(ScopeSampler::analyzer);
                }
                Kind::Voltmeter { rms, .. } => {
                    let left_node = self.pin_net.get(&c.left_pin()).copied();
                    let right_node = self.pin_net.get(&c.right_pin()).copied();
                    instrument_caches.push(CachedInstrument::Voltmeter {
                        comp_idx: idx,
                        comp_id: c.id.clone(),
                        rms: *rms,
                        left_node,
                        right_node,
                    });
                    self.instruments.meters.entry(c.id.clone()).or_default();
                }
                Kind::Ammeter { rms, .. } => {
                    let left_node = self.pin_net.get(&c.left_pin()).copied();
                    let right_node = self.pin_net.get(&c.right_pin()).copied();
                    instrument_caches.push(CachedInstrument::Ammeter {
                        comp_idx: idx,
                        comp_id: c.id.clone(),
                        rms: *rms,
                        left_node,
                        right_node,
                    });
                    self.instruments.meters.entry(c.id.clone()).or_default();
                }
                Kind::FreqMeter { filter } => {
                    let left_node = self.pin_net.get(&c.left_pin()).copied();
                    instrument_caches.push(CachedInstrument::FreqMeter {
                        comp_id: c.id.clone(),
                        filter: *filter,
                        left_node,
                    });
                    self.instruments.freq.entry(c.id.clone()).or_default();
                }
                _ => {}
            }
        }
        self.instrument_caches = instrument_caches;
        self.time_sources_cached = time_sources_cached;
        self.has_motors_cached = has_motors;
        for c in &mut self.components {
            match &mut c.kind {
                Kind::Mcu(m) => m.update_node_indices(&self.pin_net),
                Kind::QemuDevice(q) => q.update_node_indices(&self.pin_net),
                _ => {}
            }
        }
    }
}

/// Put both names on one net when either end is already wired.
fn join_present(uf: &mut UnionFind, a: &str, b: &str) {
    if uf.contains(a) || uf.contains(b) {
        uf.union(a, b);
    }
}

/// Header and socket: row `i` is `pin{i}` on the lead and `pin{i+n}` on the body.
fn join_connector_rows(uf: &mut UnionFind, id: &str, pins_count: usize) {
    let n = pins_count.max(1);
    for i in 0..n {
        join_present(uf, &format!("{id}-pin{i}"), &format!("{id}-pin{}", i + n));
    }
}

/// Trunk ends of one bus are the same spine. Line `ePin{k}` is shared by every
/// bus in a trunk-connected group that has that line. Lines are not shorted together.
fn join_bus_lines(uf: &mut UnionFind, buses: &[(String, usize)]) {
    for (id, _) in buses {
        join_present(uf, &format!("{id}-ePin0"), &format!("{id}-busPinI"));
    }
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, (id, _)) in buses.iter().enumerate() {
        let trunk = format!("{id}-ePin0");
        if !uf.contains(&trunk) {
            continue;
        }
        let root = uf.find(&trunk);
        if let Some(group) = groups.iter_mut().find(|(found, _)| found == &root) {
            group.1.push(index);
        } else {
            groups.push((root, vec![index]));
        }
    }
    for (_, members) in groups {
        if members.len() < 2 {
            continue;
        }
        let max_lines = members
            .iter()
            .map(|&i| buses[i].1.max(1))
            .max()
            .unwrap_or(0);
        for line in 1..=max_lines {
            let mut pins = Vec::new();
            for &i in &members {
                if line > buses[i].1.max(1) {
                    continue;
                }
                pins.push(format!("{}-ePin{line}", buses[i].0));
            }
            // A line nobody tapped stays local. One wired tap carries the line
            // through the other segments, including a middle tap with no wire.
            if !pins.iter().any(|pin| uf.contains(pin)) {
                continue;
            }
            let Some((anchor, rest)) = pins.split_first() else {
                continue;
            };
            for pin in rest {
                uf.union(anchor, pin);
            }
        }
    }
}
