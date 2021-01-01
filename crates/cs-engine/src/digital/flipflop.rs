//! C++ `FlipFlopD` / `JK` / `RS` / `T` and `LatchD`.

use super::clock::{Clocked, Trigger};
use super::family::LogicFamily;
use super::gate::{GateUpdate, apply_family_out};
use super::pin::{IoPin, PinAction};
use super::queue::{OutQueue, Schedule};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlipFlopKind {
    #[default]
    D,
    Jk,
    Rs,
    T,
}

impl FlipFlopKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::D => "D",
            Self::Jk => "JK",
            Self::Rs => "RS",
            Self::T => "T",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "JK" => Self::Jk,
            "RS" => Self::Rs,
            "T" => Self::T,
            _ => Self::D,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlipFlopState {
    pub kind: FlipFlopKind,
    pub family: LogicFamily,
    pub clocked: Clocked,
    pub use_rs: bool,
    /// C++ `m_Q0` at stamp. Deterministic default (C++ uses `rand()`).
    pub q0: bool,
    pub d: IoPin,
    pub j: IoPin,
    pub k: IoPin,
    pub t: IoPin,
    pub set: IoPin,
    pub rst: IoPin,
    pub clk: IoPin,
    pub q: IoPin,
    pub qn: IoPin,
    pub queue: OutQueue,
}

impl FlipFlopState {
    pub fn d(id: &str) -> Self {
        Self::new(id, FlipFlopKind::D)
    }
    pub fn jk(id: &str) -> Self {
        Self::new(id, FlipFlopKind::Jk)
    }
    pub fn rs(id: &str) -> Self {
        Self::new(id, FlipFlopKind::Rs)
    }
    pub fn t(id: &str) -> Self {
        Self::new(id, FlipFlopKind::T)
    }

    fn new(id: &str, kind: FlipFlopKind) -> Self {
        let family = LogicFamily::new();
        let inp_high = family.inp_high_v;
        let inp_low = family.inp_low_v;
        let inp_imp = family.inp_imp;
        let mk_in = |suffix: &str| {
            let mut p = IoPin::input(format!("{id}-{suffix}"));
            p.set_thresholds(inp_high, inp_low);
            p.set_input_imp(inp_imp);
            p
        };
        let mut q = IoPin::output(format!("{id}-out0"));
        let mut qn = IoPin::output(format!("{id}-out1"));
        apply_family_out(&family, &mut q);
        apply_family_out(&family, &mut qn);
        // Pin ids match C++ `IoComponent::init` (`id-inN` / `id-outN`).
        let (data, set_s, rst_s, clk_s) = match kind {
            FlipFlopKind::D | FlipFlopKind::T => ("in0", "in1", "in2", "in3"),
            FlipFlopKind::Jk => ("in0", "in2", "in3", "in4"),
            FlipFlopKind::Rs => ("in0", "in0", "in1", "in2"),
        };
        let k_s = if kind == FlipFlopKind::Jk {
            "in1"
        } else {
            data
        };
        let mut s = Self {
            kind,
            family,
            clocked: Clocked::default(),
            use_rs: true,
            q0: false,
            d: mk_in(data),
            j: mk_in(data),
            k: mk_in(k_s),
            t: mk_in(data),
            set: mk_in(set_s),
            rst: mk_in(rst_s),
            clk: mk_in(clk_s),
            q,
            qn,
            queue: OutQueue::new(),
        };
        s.set.set_inverted(true);
        s.rst.set_inverted(true);
        s.clocked.trigger = Trigger::Clock;
        s
    }

    pub fn pin_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        match self.kind {
            FlipFlopKind::D => ids.push(self.d.id.clone()),
            FlipFlopKind::Jk => {
                ids.push(self.j.id.clone());
                ids.push(self.k.id.clone());
            }
            FlipFlopKind::T => ids.push(self.t.id.clone()),
            FlipFlopKind::Rs => {}
        }
        if self.kind != FlipFlopKind::Rs {
            ids.push(self.set.id.clone());
            ids.push(self.rst.id.clone());
        } else {
            ids.push(self.set.id.clone());
            ids.push(self.rst.id.clone());
        }
        ids.push(self.clk.id.clone());
        ids.push(self.q.id.clone());
        ids.push(self.qn.id.clone());
        ids
    }

    pub fn pin_at_mut(&mut self, idx: usize) -> Option<&mut IoPin> {
        self.pins_mut_vec().into_iter().nth(idx)
    }

    fn pins_mut_vec(&mut self) -> Vec<&mut IoPin> {
        // Stable index: data pins, S, R, CLK, Q, !Q — used for slope events.
        match self.kind {
            FlipFlopKind::D => vec![
                &mut self.d,
                &mut self.set,
                &mut self.rst,
                &mut self.clk,
                &mut self.q,
                &mut self.qn,
            ],
            FlipFlopKind::Jk => vec![
                &mut self.j,
                &mut self.k,
                &mut self.set,
                &mut self.rst,
                &mut self.clk,
                &mut self.q,
                &mut self.qn,
            ],
            FlipFlopKind::T => vec![
                &mut self.t,
                &mut self.set,
                &mut self.rst,
                &mut self.clk,
                &mut self.q,
                &mut self.qn,
            ],
            FlipFlopKind::Rs => vec![
                &mut self.set,
                &mut self.rst,
                &mut self.clk,
                &mut self.q,
                &mut self.qn,
            ],
        }
    }

    pub fn q_pin_index(&self) -> usize {
        match self.kind {
            FlipFlopKind::D | FlipFlopKind::T => 4,
            FlipFlopKind::Jk => 5,
            FlipFlopKind::Rs => 3,
        }
    }

    pub fn apply_family(&mut self) {
        let f = self.family.clone();
        for p in [
            &mut self.d,
            &mut self.j,
            &mut self.k,
            &mut self.t,
            &mut self.set,
            &mut self.rst,
            &mut self.clk,
        ] {
            p.set_thresholds(f.inp_high_v, f.inp_low_v);
            p.set_input_imp(f.inp_imp);
        }
        apply_family_out(&f, &mut self.q);
        apply_family_out(&f, &mut self.qn);
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        for p in self.pins_mut_vec() {
            p.initialize(slope_steps);
        }
        self.clocked.stamp(Some(&self.clk));
        self.q.set_state_z(false);
        self.qn.set_state_z(false);
        self.q.set_out_state(false);
        self.qn.set_out_state(false);
        self.queue.clear();
        self.q.set_out_state(self.q0);
        self.qn.set_out_state(!self.q0);
        self.queue.out_value = if self.q0 { 1 } else { 2 };
        self.queue.next_out_val = self.queue.out_value;
    }

    pub fn volt_changed(&mut self, volt_of: &impl Fn(&str) -> f64, circ_time: u64) -> GateUpdate {
        let clk_v = volt_of(&self.clk.id);
        let clk = self.clk.get_inp_state(clk_v);
        self.clocked.update(Some(clk));

        if self.kind == FlipFlopKind::Rs {
            if !self.clocked.allow() {
                return GateUpdate::none();
            }
            let set = self.s_state(volt_of);
            let reset = self.r_state(volt_of);
            if set || reset {
                self.queue.next_out_val = u32::from(set) + if reset { 2 } else { 0 };
                return self.apply_schedule(circ_time);
            }
            return GateUpdate::none();
        }

        let set = self.s_state(volt_of);
        let reset = self.r_state(volt_of);
        if set || reset {
            self.queue.next_out_val = u32::from(set) + if reset { 2 } else { 0 };
        } else if self.clocked.allow() {
            self.calc_output(volt_of);
        }
        self.apply_schedule(circ_time)
    }

    pub fn run_event(&mut self, circ_time: u64) -> GateUpdate {
        let (val, follow) = self.queue.run_outputs(circ_time);
        let mut u = self.apply_bits(val);
        if let Some(d) = follow {
            u.device_event = Some(d);
        }
        u
    }

    fn s_state(&mut self, volt_of: &impl Fn(&str) -> f64) -> bool {
        if self.kind == FlipFlopKind::Rs || self.use_rs {
            let v = volt_of(&self.set.id);
            self.set.get_inp_state(v)
        } else {
            false
        }
    }

    fn r_state(&mut self, volt_of: &impl Fn(&str) -> f64) -> bool {
        if self.kind == FlipFlopKind::Rs || self.use_rs {
            let v = volt_of(&self.rst.id);
            self.rst.get_inp_state(v)
        } else {
            false
        }
    }

    fn calc_output(&mut self, volt_of: &impl Fn(&str) -> f64) {
        match self.kind {
            FlipFlopKind::D => {
                let d = self.d.get_inp_state(volt_of(&self.d.id));
                self.queue.next_out_val = if d { 1 } else { 2 };
            }
            FlipFlopKind::Jk => {
                let j = self.j.get_inp_state(volt_of(&self.j.id));
                let k = self.k.get_inp_state(volt_of(&self.k.id));
                let q = self.q.get_out_state();
                let q0 = (j && !q) || (!k && q);
                self.q0 = q0;
                self.queue.next_out_val = if q0 { 1 } else { 2 };
            }
            FlipFlopKind::T => {
                let t = self.t.get_inp_state(volt_of(&self.t.id));
                if t {
                    self.queue.next_out_val = if self.qn.get_out_state() { 1 } else { 2 };
                }
            }
            FlipFlopKind::Rs => {}
        }
    }

    fn apply_schedule(&mut self, circ_time: u64) -> GateUpdate {
        match self.queue.schedule(self.family.delay_ps(), circ_time) {
            Schedule::Immediate => self.apply_bits(self.queue.out_value),
            Schedule::Arm { delay_ps } => GateUpdate {
                stamp_changed: false,
                device_event: Some(delay_ps),
                pin_action: PinAction::None,
            },
            Schedule::Queued | Schedule::Unchanged => GateUpdate::none(),
        }
    }

    fn apply_bits(&mut self, val: u32) -> GateUpdate {
        let a = self.q.schedule_state(val & 1 != 0, 0);
        let b = self.qn.schedule_state(val & 2 != 0, 0);
        GateUpdate {
            stamp_changed: matches!(a, PinAction::Immediate) || matches!(b, PinAction::Immediate),
            device_event: None,
            pin_action: a,
        }
    }
}

/// C++ `LatchD` (default 8 channels, Enable trigger).
#[derive(Clone, Debug)]
pub struct LatchState {
    pub family: LogicFamily,
    pub clocked: Clocked,
    pub channels: usize,
    pub use_reset: bool,
    pub tristate: bool,
    pub out_enable: bool,
    pub inputs: Vec<IoPin>,
    pub outputs: Vec<IoPin>,
    pub clk: IoPin,
    pub reset: IoPin,
    pub oe: IoPin,
    pub queue: OutQueue,
}

impl LatchState {
    pub fn new(id: &str, channels: usize) -> Self {
        let n = channels.clamp(1, 32);
        let family = LogicFamily::new();
        let inp_high = family.inp_high_v;
        let inp_low = family.inp_low_v;
        let inp_imp = family.inp_imp;
        let mk_in = |name: String| {
            let mut p = IoPin::input(name);
            p.set_thresholds(inp_high, inp_low);
            p.set_input_imp(inp_imp);
            p
        };
        let inputs: Vec<IoPin> = (0..n).map(|i| mk_in(format!("{id}-in{i}"))).collect();
        let mut outputs: Vec<IoPin> = (0..n)
            .map(|i| IoPin::output(format!("{id}-out{i}")))
            .collect();
        for o in &mut outputs {
            apply_family_out(&family, o);
        }
        let mut oe = mk_in(format!("{id}-Pin_outEnable"));
        oe.set_inverted(true);
        let mut s = Self {
            family,
            clocked: Clocked::default(),
            channels: n,
            use_reset: false,
            tristate: true,
            out_enable: true,
            inputs,
            outputs,
            clk: mk_in(format!("{id}-Pin_clock")),
            reset: mk_in(format!("{id}-Pin_reset")),
            oe,
            queue: OutQueue::new(),
        };
        s.clocked.trigger = Trigger::Enable;
        s
    }

    pub fn pin_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.inputs.iter().map(|p| p.id.clone()).collect();
        ids.extend(self.outputs.iter().map(|p| p.id.clone()));
        ids.push(self.clk.id.clone());
        ids.push(self.reset.id.clone());
        ids.push(self.oe.id.clone());
        ids
    }

    pub fn pin_at_mut(&mut self, idx: usize) -> Option<&mut IoPin> {
        let n = self.channels;
        if idx < n {
            return Some(&mut self.inputs[idx]);
        }
        if idx < 2 * n {
            return Some(&mut self.outputs[idx - n]);
        }
        match idx - 2 * n {
            0 => Some(&mut self.clk),
            1 => Some(&mut self.reset),
            2 => Some(&mut self.oe),
            _ => None,
        }
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        for p in self
            .inputs
            .iter_mut()
            .chain(self.outputs.iter_mut())
            .chain([&mut self.clk, &mut self.reset, &mut self.oe])
        {
            p.initialize(slope_steps);
        }
        self.clocked.stamp(Some(&self.clk));
        for o in &mut self.outputs {
            o.set_state_z(false);
            o.set_out_state(false);
        }
        self.queue.clear();
        self.out_enable = true;
    }

    pub fn volt_changed(&mut self, volt_of: &impl Fn(&str) -> f64, circ_time: u64) -> GateUpdate {
        if self.tristate {
            let en = self.oe.get_inp_state(volt_of(&self.oe.id));
            if en != self.out_enable {
                self.out_enable = en;
                for o in &mut self.outputs {
                    o.set_state_z(!en);
                }
            }
        }
        let clk = self.clk.get_inp_state(volt_of(&self.clk.id));
        self.clocked.update(Some(clk));
        if self.use_reset && self.reset.get_inp_state(volt_of(&self.reset.id)) {
            self.queue.next_out_val = 0;
        } else if self.clocked.allow() {
            let mut v = 0u32;
            for (i, p) in self.inputs.iter_mut().enumerate() {
                if p.get_inp_state(volt_of(&p.id)) {
                    v |= 1 << i;
                }
            }
            self.queue.next_out_val = v;
        }
        match self.queue.schedule(self.family.delay_ps(), circ_time) {
            Schedule::Immediate => self.apply_bits(self.queue.out_value),
            Schedule::Arm { delay_ps } => GateUpdate {
                stamp_changed: false,
                device_event: Some(delay_ps),
                pin_action: PinAction::None,
            },
            Schedule::Queued | Schedule::Unchanged => GateUpdate::none(),
        }
    }

    pub fn run_event(&mut self, circ_time: u64) -> GateUpdate {
        let (val, follow) = self.queue.run_outputs(circ_time);
        let mut u = self.apply_bits(val);
        if let Some(d) = follow {
            u.device_event = Some(d);
        }
        u
    }

    fn apply_bits(&mut self, val: u32) -> GateUpdate {
        let mut changed = false;
        for (i, o) in self.outputs.iter_mut().enumerate() {
            let a = o.schedule_state((val & (1 << i)) != 0, 0);
            if matches!(a, PinAction::Immediate) {
                changed = true;
            }
        }
        GateUpdate {
            stamp_changed: changed,
            device_event: None,
            pin_action: PinAction::None,
        }
    }
}
