//! Headless circuit simulation coordinator: timekeeping, stepping loop, and public facade.

use rustc_hash::FxHashMap;

use crate::Result;
use crate::digital::{EventQueue, EventTarget, GateUpdate, PinAction, ps_to_secs};
use crate::elements::{ANALOG_DT_DEFAULT, Comp};
use crate::instruments::InstrumentState;
use crate::matrix::CircMatrix;
use crate::net::ENode;

pub(crate) use periph_sync::CachedInstrument;

mod builder;
mod digital_queue;
mod matrix_solver;
mod nonlinear;
mod periph_sync;
mod reactive;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct Circuit {
    components: Vec<Comp>,
    connectors: Vec<(String, String)>,
    nodes: Vec<ENode>,
    pin_net: FxHashMap<String, usize>,
    matrix: CircMatrix,
    volts_buf: Vec<f64>,
    pin_caches: Vec<crate::elements::PinCache>,
    skipped: Vec<String>,
    /// Analog clock period in seconds (C++ `reaStep` picoseconds / 1e12).
    pub dt: f64,
    /// Accumulated analog time in seconds.
    pub time: f64,
    /// C++ `Simulator::m_maxNlstp` (default 100000).
    pub max_nl_steps: u32,
    pub instruments: InstrumentState,
    /// C++ `Simulator::m_circTime` (picoseconds). Starts at 1.
    pub circ_time: u64,
    /// C++ `Simulator::m_slopeSteps` (0 = instant edges).
    pub slope_steps: i32,
    events: EventQueue,
    sim_started: bool,
    mcu_actions_buf: Vec<(usize, PinAction)>,
    audio_sink: Option<std::sync::Arc<std::sync::Mutex<dyn crate::audio::AudioSink>>>,
    /// Speed in picoseconds per second (default 1e12 = 100% speed).
    pub ps_per_sec: u64,
    pub last_motor_ps: u64,
    nodes_dirty: bool,
    digital_updates_buf: Vec<(usize, GateUpdate, usize)>,
    has_instruments_cached: bool,
    has_digital_cached: bool,
    has_nonlinear_cached: bool,
    instrument_caches: Vec<CachedInstrument>,
    has_motors_cached: bool,
    time_sources_cached: Vec<usize>,
}

impl Default for Circuit {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            connectors: Vec::new(),
            nodes: Vec::new(),
            pin_net: FxHashMap::default(),
            matrix: CircMatrix::default(),
            volts_buf: Vec::new(),
            pin_caches: Vec::new(),
            skipped: Vec::new(),
            dt: ANALOG_DT_DEFAULT,
            time: 0.0,
            max_nl_steps: 100_000,
            instruments: InstrumentState::default(),
            circ_time: 1,
            slope_steps: 0,
            events: EventQueue::new(),
            sim_started: false,
            mcu_actions_buf: Vec::new(),
            audio_sink: None,
            ps_per_sec: 1_000_000_000_000,
            last_motor_ps: 1,
            nodes_dirty: true,
            digital_updates_buf: Vec::new(),
            has_instruments_cached: true,
            has_digital_cached: true,
            has_nonlinear_cached: true,
            instrument_caches: Vec::new(),
            has_motors_cached: false,
            time_sources_cached: Vec::new(),
        }
    }
}

impl Circuit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn solve(&mut self) -> Result<()> {
        self.build_nets();
        self.circ_time = 1;
        self.last_motor_ps = 1;
        self.events.clear();
        self.sim_started = false;
        self.reset_device_stamps();
        if self.nodes.is_empty() {
            self.sample_instruments();
            self.update_motors_dt(0);
            return Ok(());
        }
        if let Err(e) = self.solve_circuit() {
            crate::logging::log_sim(format!("Simulation solve error: {e}"));
            return Err(e);
        }
        self.drain_logic_events()?;
        self.sample_instruments();
        self.last_motor_ps = self.circ_time;
        self.update_motors_dt(0);
        Ok(())
    }

    /// Fast re-solve when component parameters change in-place.
    /// Skips rebuilding nets and topology, directly re-stamps and solves the matrix.
    pub fn re_solve(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            if !self.components.is_empty() {
                return self.solve();
            }
            self.sample_instruments();
            self.update_motors_dt(0);
            return Ok(());
        }
        self.reset_device_stamps();
        if let Err(e) = self.solve_circuit() {
            crate::logging::log_sim(format!("Simulation solve error: {e}"));
            return Err(e);
        }
        self.drain_logic_events()?;
        self.sample_instruments();
        self.update_motors_dt(0);
        self.ensure_analog_clock();
        Ok(())
    }

    /// Advance one analog clock period, including digital events in that window.
    pub fn step(&mut self) -> Result<()> {
        if self.nodes.is_empty() && self.pin_net.is_empty() {
            self.solve()?;
        }
        if !self.sim_started {
            if self.nodes.is_empty() && !self.components.is_empty() {
                self.solve()?;
            }
            self.circ_time = 1;
            self.last_motor_ps = 1;
            self.events.clear();
            self.sim_started = true;
            if self.has_reactive() {
                self.events.add(
                    self.circ_time.saturating_add(self.analog_ps()),
                    EventTarget::AnalogClock,
                );
            }
            self.start_mcus();
        }
        let end = self.circ_time.saturating_add(self.analog_ps());
        self.run_until(end)
    }

    pub fn step_n(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            self.step()?;
        }
        Ok(())
    }

    /// Run the C++ `runCircuit` loop up to `end` picoseconds (absolute).
    pub fn run_until(&mut self, end: u64) -> Result<()> {
        self.run_until_budgeted(end, None)
    }

    /// Run the C++ `runCircuit` loop up to `end` picoseconds with an optional wall-clock time limit.
    pub fn run_until_budgeted(
        &mut self,
        end: u64,
        max_dur: Option<std::time::Duration>,
    ) -> Result<()> {
        if !self.sim_started && self.nodes.is_empty() && !self.components.is_empty() {
            self.build_nets();
            self.reset_device_stamps();
            if !self.nodes.is_empty() {
                self.solve_circuit()?;
            }
        }
        let mut needs_solve = false;
        let start = max_dur.map(|_| std::time::Instant::now());
        let mut steps = 0usize;
        loop {
            if crate::debug::DebugSession::is_paused_global() {
                break;
            }
            let Some((t, target)) = self.events.pop_due(end) else {
                if self.sim_started {
                    self.circ_time = end;
                }
                break;
            };
            self.circ_time = t;
            let changed = self.run_event(target)?;
            if changed {
                needs_solve = true;
                self.ensure_analog_clock();
            }
            if needs_solve {
                self.solve_circuit()?;
                needs_solve = false;
            } else {
                self.nodes_dirty = false;
            }
            if self.has_instruments() {
                self.time = ps_to_secs(self.circ_time.saturating_sub(1));
                self.sample_instruments();
            }
            if crate::debug::DebugSession::is_paused_global() {
                break;
            }
            steps += 1;
            if steps & 0x3F == 0 {
                if let (Some(start), Some(limit)) = (start, max_dur) {
                    if start.elapsed() >= limit {
                        break;
                    }
                }
            }
        }
        if needs_solve {
            self.solve_circuit()?;
        }
        self.time = ps_to_secs(self.circ_time.saturating_sub(1));
        self.sample_instruments();
        let dt_ps = self.circ_time.saturating_sub(self.last_motor_ps);
        self.update_motors_dt(dt_ps);
        self.last_motor_ps = self.circ_time;
        self.flush_audio();
        Ok(())
    }

    /// Restamp and resolve without re-initializing device state. Used after
    /// live changes (MCU pin drive, switch) during a running sim.
    pub fn update(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            return self.solve();
        }
        self.solve_circuit()?;
        self.drain_logic_events()?;
        self.sample_instruments();
        let dt_ps = self.circ_time.saturating_sub(self.last_motor_ps);
        self.update_motors_dt(dt_ps);
        self.last_motor_ps = self.circ_time;
        self.ensure_analog_clock();
        Ok(())
    }

    /// Advance `ps` picoseconds from the current circuit time.
    pub fn run_ps(&mut self, ps: u64) -> Result<()> {
        self.run_ps_budgeted(ps, None)
    }

    /// Advance up to `ps` picoseconds with an optional wall-clock time limit.
    pub fn run_ps_budgeted(&mut self, ps: u64, max_dur: Option<std::time::Duration>) -> Result<()> {
        if !self.sim_started {
            self.sim_started = true;
            if self.nodes.is_empty() {
                self.build_nets();
                self.reset_device_stamps();
            }
            self.circ_time = 1;
            self.events.clear();
            if !self.nodes.is_empty() {
                self.solve_circuit()?;
            }
            if self.has_reactive() {
                self.events.add(
                    self.circ_time.saturating_add(self.analog_ps()),
                    EventTarget::AnalogClock,
                );
            }
            self.start_mcus();
        }
        let end = self.circ_time.saturating_add(ps);
        self.run_until_budgeted(end, max_dur)
    }

    pub(super) fn ensure_analog_clock(&mut self) {
        if self.has_reactive() && !self.events.has_event(&EventTarget::AnalogClock) {
            self.events.add(
                self.circ_time.saturating_add(self.analog_ps()),
                EventTarget::AnalogClock,
            );
        }
    }

    pub(super) fn analog_clock_event(&mut self) -> Result<bool> {
        let sources_changed = self.update_sources();
        self.solve_circuit()?;
        if self.has_reactive() {
            self.update_reactive();
        }
        let should_reschedule = self.has_reactive() || sources_changed;
        if should_reschedule {
            self.events.add(
                self.circ_time.saturating_add(self.analog_ps()),
                EventTarget::AnalogClock,
            );
        }
        Ok(false)
    }

    pub fn component_memory_bytes(&self, id: &str) -> Option<Vec<u8>> {
        let comp = self.components.iter().find(|c| c.id == id)?;
        match &comp.kind {
            crate::elements::Kind::Memory(m) => Some(m.data.clone()),
            crate::elements::Kind::DynamicMemory(m) => Some(m.data.clone()),
            crate::elements::Kind::I2CRam(m) => Some(m.data.clone()),
            _ => None,
        }
    }

    pub fn set_component_memory_bytes(&mut self, id: &str, data: &[u8]) -> bool {
        let Some(comp) = self.components.iter_mut().find(|c| c.id == id) else {
            return false;
        };
        match &mut comp.kind {
            crate::elements::Kind::Memory(m) => {
                let n = m.data.len().min(data.len());
                m.data[..n].copy_from_slice(&data[..n]);
                true
            }
            crate::elements::Kind::DynamicMemory(m) => {
                let n = m.data.len().min(data.len());
                m.data[..n].copy_from_slice(&data[..n]);
                true
            }
            crate::elements::Kind::I2CRam(m) => {
                let n = m.data.len().min(data.len());
                m.data[..n].copy_from_slice(&data[..n]);
                true
            }
            _ => false,
        }
    }

    pub fn set_component_memory_byte(&mut self, id: &str, addr: usize, val: u8) -> bool {
        let Some(comp) = self.components.iter_mut().find(|c| c.id == id) else {
            return false;
        };
        match &mut comp.kind {
            crate::elements::Kind::Memory(m) => {
                if addr < m.data.len() {
                    m.data[addr] = val;
                    true
                } else {
                    false
                }
            }
            crate::elements::Kind::DynamicMemory(m) => {
                if addr < m.data.len() {
                    m.data[addr] = val;
                    true
                } else {
                    false
                }
            }
            crate::elements::Kind::I2CRam(m) => {
                if addr < m.data.len() {
                    m.data[addr] = val;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
