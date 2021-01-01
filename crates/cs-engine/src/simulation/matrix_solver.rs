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
        let max = if self.has_nonlinear_cached {
            self.max_nl_steps.max(1) as usize
        } else {
            2
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
                let pa = c.pot_pin_a();
                let pb = c.pot_pin_b();
                let pm = c.pot_pin_m();
                let l = format!("{}-lPin", c.id);
                let r = format!("{}-rPin", c.id);
                let w = format!("{}-wPin", c.id);
                if uf.contains(&pa) && uf.contains(&l) {
                    uf.union(&pa, &l);
                }
                if uf.contains(&pb) && uf.contains(&r) {
                    uf.union(&pb, &r);
                }
                if uf.contains(&w) {
                    uf.union(&w, &pm);
                } else if !uf.contains(&pm)
                    && (uf.contains(&pa) || uf.contains(&pb) || uf.contains(&l) || uf.contains(&r))
                {
                    uf.add(&pm);
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
                Kind::WaveGen { .. } => {
                    time_sources_cached.push(idx);
                }
                Kind::DcMotor { .. } | Kind::Servo { .. } => {
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
