//! Peripheral synchronization hooks (MCU, QEMU cosim, AngelScript, Audio, Motors, Instruments).

use crate::digital::EventTarget;
use crate::elements::Kind;
use crate::instruments::ScopeSampler;

use super::Circuit;

#[derive(Clone, Debug)]
pub(crate) enum CachedInstrument {
    Oscope {
        comp_id: String,
        gnd_node: Option<usize>,
        pins: [Option<usize>; 4],
    },
    LAnalizer {
        comp_id: String,
        pins: [Option<usize>; 8],
    },
    Voltmeter {
        comp_idx: usize,
        comp_id: String,
        rms: bool,
        left_node: Option<usize>,
        right_node: Option<usize>,
    },
    Ammeter {
        comp_idx: usize,
        comp_id: String,
        rms: bool,
        left_node: Option<usize>,
        right_node: Option<usize>,
    },
    FreqMeter {
        comp_id: String,
        filter: f64,
        left_node: Option<usize>,
    },
}

impl Circuit {
    pub fn first_mcu(&self) -> Option<(&str, &crate::mcu::McuComp)> {
        self.components.iter().find_map(|c| match &c.kind {
            Kind::Mcu(m) => Some((c.id.as_str(), m)),
            _ => None,
        })
    }

    pub fn first_mcu_mut(&mut self) -> Option<(&str, &mut crate::mcu::McuComp)> {
        self.components.iter_mut().find_map(|c| match &mut c.kind {
            Kind::Mcu(m) => Some((c.id.as_str(), m)),
            _ => None,
        })
    }

    pub fn mcus(&self) -> impl Iterator<Item = (&str, &crate::mcu::McuComp)> {
        self.components.iter().filter_map(|c| match &c.kind {
            Kind::Mcu(m) => Some((c.id.as_str(), m)),
            _ => None,
        })
    }

    pub fn mcus_mut(&mut self) -> impl Iterator<Item = (&str, &mut crate::mcu::McuComp)> {
        self.components
            .iter_mut()
            .filter_map(|c| match &mut c.kind {
                Kind::Mcu(m) => Some((c.id.as_str(), m)),
                _ => None,
            })
    }

    /// C++ `MCUMonitor::updateStep` snapshot of the first MCU.
    pub fn mcu_snap(&self) -> crate::mcu::McuSnap {
        self.first_mcu()
            .map(|(id, m)| crate::mcu::McuSnap::from_mcu(id, m))
            .unwrap_or_default()
    }

    pub fn poke_mcu_ram(&mut self, addr: u16, value: u8) -> bool {
        if let Some((_, m)) = self.first_mcu_mut() {
            m.device.set_monitor_ram(addr, value);
            true
        } else {
            false
        }
    }

    pub fn poke_mcu_flash(&mut self, addr: u32, value: u16) -> bool {
        if let Some((_, m)) = self.first_mcu_mut() {
            m.device.set_flash(addr as usize, value);
            true
        } else {
            false
        }
    }

    pub fn poke_mcu_eeprom(&mut self, addr: u16, value: u8) -> bool {
        if let Some((_, m)) = self.first_mcu_mut() {
            m.device.set_eeprom(addr as usize, value);
            true
        } else {
            false
        }
    }

    pub fn apply_mcu_poke(&mut self, poke: crate::mcu::McuPoke) -> bool {
        match poke {
            crate::mcu::McuPoke::Ram { addr, value } => self.poke_mcu_ram(addr, value),
            crate::mcu::McuPoke::Flash { addr, value } => self.poke_mcu_flash(addr, value),
            crate::mcu::McuPoke::Eeprom { addr, value } => self.poke_mcu_eeprom(addr, value),
        }
    }

    pub fn qemu_mut(&mut self, id: &str) -> Option<&mut crate::qemu::QemuComp> {
        self.components.iter_mut().find_map(|c| {
            if c.id == id {
                if let Kind::QemuDevice(q) = &mut c.kind {
                    return Some(q);
                }
            }
            None
        })
    }

    pub fn has_audio_out(&self) -> bool {
        self.components
            .iter()
            .any(|c| matches!(&c.kind, Kind::AudioOut { .. }))
    }

    pub fn set_audio_sink(
        &mut self,
        sink: std::sync::Arc<std::sync::Mutex<dyn crate::audio::AudioSink>>,
    ) {
        if let Ok(mut s) = sink.lock() {
            s.resume();
        }
        self.audio_sink = Some(sink);
    }

    pub fn clear_audio_sink(&mut self) {
        self.audio_sink = None;
    }

    pub fn flush_audio(&mut self) {
        if let Some(sink) = &self.audio_sink {
            for c in &mut self.components {
                if let Kind::AudioOut { source, .. } = &mut c.kind {
                    let samples = source.take_samples();
                    if !samples.is_empty() {
                        if let Ok(mut s) = sink.lock() {
                            s.write(&samples);
                        }
                    }
                }
            }
        }
    }

    pub fn take_audio_samples(&mut self) -> Vec<i16> {
        let mut all = Vec::new();
        for c in &mut self.components {
            if let Kind::AudioOut { source, .. } = &mut c.kind {
                all.extend(source.take_samples());
            }
        }
        all
    }

    pub fn scan_has_instruments(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(
                &c.kind,
                Kind::Probe { .. }
                    | Kind::Voltmeter { .. }
                    | Kind::Ammeter { .. }
                    | Kind::FreqMeter { .. }
                    | Kind::Oscope { .. }
                    | Kind::LAnalizer { .. }
            )
        })
    }

    pub fn has_instruments(&self) -> bool {
        self.has_instruments_cached
    }

    #[inline]

    /// C++ `QemuDevice::stopQemuProcess` while the circuit is paused.
    pub fn pause_qemu(&mut self) {
        for c in &mut self.components {
            if let Kind::QemuDevice(q) = &mut c.kind {
                q.pause();
            }
        }
    }

    /// C++ `QemuDevice::resumeQemuProcess`.
    pub fn resume_qemu(&mut self) {
        for c in &mut self.components {
            if let Kind::QemuDevice(q) = &mut c.kind {
                q.resume();
            }
        }
    }

    pub(super) fn start_mcus(&mut self) {
        let sample_rate = self
            .audio_sink
            .as_ref()
            .and_then(|s| s.lock().ok().map(|s| s.sample_rate()))
            .unwrap_or(crate::audio::SAMPLE_RATE);
        let audio_period_ps = crate::audio::sample_period_ps(sample_rate, self.ps_per_sec).max(1);

        for i in 0..self.components.len() {
            let ps = match &mut self.components[i].kind {
                Kind::Mcu(m) => {
                    if m.device.freq <= 0.0 {
                        None
                    } else {
                        m.device.start();
                        Some(m.device.ps_tick.max(1))
                    }
                }
                Kind::ScriptCpu(s) => Some(s.ps_tick.max(1)),
                Kind::QemuDevice(q) => {
                    if q.is_live() {
                        Some(1)
                    } else {
                        None
                    }
                }
                Kind::AudioOut { .. } => Some(audio_period_ps),
                Kind::WaveGen { .. } => self.components[i].wavegen_next_event_ps(self.circ_time),
                _ => None,
            };
            if let Some(ps) = ps {
                self.events
                    .add(self.circ_time.saturating_add(ps), EventTarget::Device(i));
            }
        }
    }

    pub fn update_motors_dt(&mut self, dt_ps: u64) {
        if !self.has_motors_cached {
            return;
        }
        let nodes = &self.nodes;
        let pin_caches = &self.pin_caches;
        let dt_s = dt_ps as f64 / 1e12;
        for (i, c) in self.components.iter_mut().enumerate() {
            if let Kind::DcMotor {
                rpm_nominal,
                volt_nominal,
                resistance: _,
                ref mut speed,
                ref mut angle,
            } = c.kind
            {
                let pc = pin_caches.get(i);
                let v0 = pc
                    .and_then(|p| p.left)
                    .and_then(|j| nodes.get(j))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let v1 = pc
                    .and_then(|p| p.right)
                    .and_then(|j| nodes.get(j))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let v = v0 - v1;
                let s = if volt_nominal > 1e-9 {
                    v / volt_nominal
                } else {
                    0.0
                };
                *speed = s;
                if dt_s > 0.0 {
                    let rpm = rpm_nominal.max(1) as f64;
                    let d_ang = 360.0 * (rpm / 60.0) * s * dt_s;
                    *angle = (*angle + d_ang).rem_euclid(360.0);
                }
            } else if let Kind::Servo {
                speed,
                ref mut pos,
                target_pos,
                ..
            } = c.kind
            {
                if dt_s > 0.0 && (target_pos - *pos).abs() > 1e-4 {
                    let max_move = dt_s / speed.max(1e-4) * 60.0;
                    let delta = target_pos - *pos;
                    let abs_delta = delta.abs();
                    let step = if abs_delta > max_move {
                        delta.signum() * max_move
                    } else {
                        delta
                    };
                    *pos += step;
                }
            }
        }
    }

    pub(super) fn sample_instruments(&mut self) {
        if !self.has_instruments() {
            return;
        }
        let t = self.time;
        let nodes = &self.nodes;

        for inst in &self.instrument_caches {
            match inst {
                CachedInstrument::Voltmeter {
                    comp_idx,
                    comp_id,
                    rms,
                    left_node,
                    right_node,
                } => {
                    let v = match (
                        left_node.and_then(|i| nodes.get(i)),
                        right_node.and_then(|i| nodes.get(i)),
                    ) {
                        (Some(n0), Some(n1)) => n0.volt - n1.volt,
                        _ => 0.0,
                    };
                    let stats = match self.instruments.meters.get_mut(comp_id) {
                        Some(s) => s,
                        None => self.instruments.meters.entry(comp_id.clone()).or_default(),
                    };
                    stats.sample(v, *rms, t);
                    let out = stats.display_value(*rms);
                    if let Some(c) = self.components.get_mut(*comp_idx) {
                        if let Kind::Voltmeter { last_out, .. } = &mut c.kind {
                            *last_out = out;
                        }
                    }
                }
                CachedInstrument::Ammeter {
                    comp_idx,
                    comp_id,
                    rms,
                    left_node,
                    right_node,
                } => {
                    let i = match (
                        left_node.and_then(|i| nodes.get(i)),
                        right_node.and_then(|i| nodes.get(i)),
                    ) {
                        (Some(n0), Some(n1)) => {
                            (n0.volt - n1.volt) / crate::instruments::AMMETER_OHMS
                        }
                        _ => 0.0,
                    };
                    let stats = match self.instruments.meters.get_mut(comp_id) {
                        Some(s) => s,
                        None => self.instruments.meters.entry(comp_id.clone()).or_default(),
                    };
                    stats.sample(i, *rms, t);
                    let out = stats.display_value(*rms);
                    if let Some(c) = self.components.get_mut(*comp_idx) {
                        if let Kind::Ammeter { last_out, .. } = &mut c.kind {
                            *last_out = out;
                        }
                    }
                }
                CachedInstrument::FreqMeter {
                    comp_id,
                    filter,
                    left_node,
                } => {
                    let v = left_node
                        .and_then(|i| nodes.get(i))
                        .map(|n| n.volt)
                        .unwrap_or(0.0);
                    let st = match self.instruments.freq.get_mut(comp_id) {
                        Some(s) => s,
                        None => self.instruments.freq.entry(comp_id.clone()).or_default(),
                    };
                    st.sample(v, *filter, t);
                }
                CachedInstrument::Oscope {
                    comp_id,
                    gnd_node,
                    pins,
                } => {
                    let g = gnd_node
                        .and_then(|n| nodes.get(n))
                        .map(|nd| nd.volt)
                        .unwrap_or(0.0);
                    let mut vals = [0.0; 4];
                    let mut connected = [false; 4];
                    for (i, p_opt) in pins.iter().enumerate() {
                        if let Some(n) = p_opt {
                            connected[i] = true;
                            let v = nodes.get(*n).map(|nd| nd.volt).unwrap_or(0.0);
                            vals[i] = v - g;
                        }
                    }
                    let s = match self.instruments.scopes.get_mut(comp_id) {
                        Some(s) => s,
                        None => self
                            .instruments
                            .scopes
                            .entry(comp_id.clone())
                            .or_insert_with(ScopeSampler::scope),
                    };
                    s.push(t, &vals, &connected, -1, 0.0, true, 0.1);
                }
                CachedInstrument::LAnalizer { comp_id, pins } => {
                    let mut vals = [0.0; 8];
                    let mut connected = [false; 8];
                    for (i, p_opt) in pins.iter().enumerate() {
                        if let Some(n) = p_opt {
                            connected[i] = true;
                            let v = nodes.get(*n).map(|nd| nd.volt).unwrap_or(0.0);
                            vals[i] = v;
                        }
                    }
                    let s = match self.instruments.las.get_mut(comp_id) {
                        Some(s) => s,
                        None => self
                            .instruments
                            .las
                            .entry(comp_id.clone())
                            .or_insert_with(ScopeSampler::analyzer),
                    };
                    s.push(t, &vals, &connected, -1, 0.0, true, 0.1);
                }
            }
        }
    }
}
