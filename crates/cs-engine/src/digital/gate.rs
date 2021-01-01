//! Combinational gates matching C++ `Gate` / `AndGate` / `OrGate` / `XorGate` / `Buffer`.

use super::family::LogicFamily;
use super::pin::{IoPin, PinAction, PinMode};
use super::queue::{OutQueue, Schedule};

/// C++ `Gate::calcOutput` variants. NAND/NOR/XNOR/Inverter are inverted outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateOp {
    /// High iff every input is high (Buffer is 1-input And).
    And,
    /// High iff any input is high.
    Or,
    /// High iff exactly one input is high (C++ `XorGate`, not parity).
    Xor,
}

#[derive(Clone, Debug)]
pub struct GateState {
    pub op: GateOp,
    pub init_high: bool,
    pub family: LogicFamily,
    pub inputs: Vec<IoPin>,
    pub output: IoPin,
    pub oe: Option<IoPin>,
    pub tristate: bool,
    pub out_enable: bool,
    pub queue: OutQueue,
}

impl GateState {
    pub fn and(id: &str, n_inputs: usize) -> Self {
        Self::new(id, GateOp::And, n_inputs.max(1))
    }

    pub fn or(id: &str, n_inputs: usize) -> Self {
        Self::new(id, GateOp::Or, n_inputs.max(2))
    }

    pub fn xor(id: &str, n_inputs: usize) -> Self {
        Self::new(id, GateOp::Xor, n_inputs.max(2))
    }

    pub fn buffer(id: &str) -> Self {
        let mut g = Self::new(id, GateOp::And, 1);
        g.oe = Some(IoPin::input(format!("{id}-Pin_outEnable")));
        if let Some(oe) = g.oe.as_mut() {
            oe.set_inverted(true);
        }
        g.tristate = false;
        g
    }

    pub fn inverter(id: &str) -> Self {
        let mut g = Self::buffer(id);
        g.output.set_inverted(true);
        g
    }

    pub fn nand(id: &str, n_inputs: usize) -> Self {
        let mut g = Self::and(id, n_inputs);
        g.output.set_inverted(true);
        g
    }

    pub fn nor(id: &str, n_inputs: usize) -> Self {
        let mut g = Self::or(id, n_inputs);
        g.output.set_inverted(true);
        g
    }

    pub fn xnor(id: &str, n_inputs: usize) -> Self {
        let mut g = Self::xor(id, n_inputs);
        g.output.set_inverted(true);
        g
    }

    fn new(id: &str, op: GateOp, n_inputs: usize) -> Self {
        let family = LogicFamily::new();
        let inputs: Vec<IoPin> = (0..n_inputs)
            .map(|i| {
                let mut p = IoPin::input(format!("{id}-in{i}"));
                p.set_thresholds(family.inp_high_v, family.inp_low_v);
                p.set_input_imp(family.inp_imp);
                p
            })
            .collect();
        // C++ `setNumOuts(1, "", -1)` → `id-out` (no index).
        let mut output = IoPin::output(format!("{id}-out"));
        apply_family_out(&family, &mut output);
        Self {
            op,
            init_high: false,
            family,
            inputs,
            output,
            oe: None,
            tristate: false,
            out_enable: true,
            queue: OutQueue::new(),
        }
    }

    pub fn pin_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.inputs.iter().map(|p| p.id.clone()).collect();
        ids.push(self.output.id.clone());
        if let Some(oe) = &self.oe {
            ids.push(oe.id.clone());
        }
        ids
    }

    pub fn pins_mut(&mut self) -> impl Iterator<Item = &mut IoPin> {
        self.inputs
            .iter_mut()
            .chain(std::iter::once(&mut self.output))
            .chain(self.oe.as_mut())
    }

    pub fn pin_at_mut(&mut self, idx: usize) -> Option<&mut IoPin> {
        if idx < self.inputs.len() {
            return Some(&mut self.inputs[idx]);
        }
        if idx == self.inputs.len() {
            return Some(&mut self.output);
        }
        if idx == self.inputs.len() + 1 {
            return self.oe.as_mut();
        }
        None
    }

    pub fn output_pin_index(&self) -> usize {
        self.inputs.len()
    }

    pub fn apply_family(&mut self) {
        let f = self.family.clone();
        for p in self.inputs.iter_mut().chain(self.oe.as_mut()) {
            p.set_thresholds(f.inp_high_v, f.inp_low_v);
            p.set_input_imp(f.inp_imp);
        }
        apply_family_out(&f, &mut self.output);
    }

    pub fn set_open_col(&mut self, oc: bool) {
        self.output
            .set_pin_mode(if oc { PinMode::OpenCo } else { PinMode::Output });
    }

    pub fn set_tristate(&mut self, t: bool) {
        self.tristate = t;
        if !t {
            self.out_enable = true;
            self.output.set_state_z(false);
        }
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        for p in self.pins_mut() {
            p.initialize(slope_steps);
        }
        self.output.set_state_z(false);
        self.output.set_out_state(false);
        self.queue.clear();
        self.out_enable = true;
        self.output.set_out_state(self.init_high);
        self.queue.out_value = u32::from(self.init_high);
        self.queue.next_out_val = self.queue.out_value;
    }

    /// C++ `Gate::voltChanged`. `volt_of` maps pin id → node voltage (0 if floating).
    pub fn volt_changed(&mut self, volt_of: &impl Fn(&str) -> f64, circ_time: u64) -> GateUpdate {
        if self.tristate {
            self.update_out_enabled(volt_of);
        }
        let mut highs = 0usize;
        for p in &mut self.inputs {
            let v = volt_of(&p.id);
            if p.get_inp_state(v) {
                highs += 1;
            }
        }
        let out = match self.op {
            GateOp::And => highs == self.inputs.len(),
            GateOp::Or => highs > 0,
            GateOp::Xor => highs == 1,
        };
        self.queue.next_out_val = u32::from(out);
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

    fn update_out_enabled(&mut self, volt_of: &impl Fn(&str) -> f64) {
        let Some(oe) = self.oe.as_mut() else {
            return;
        };
        let en = oe.get_inp_state(volt_of(&oe.id));
        if en != self.out_enable {
            self.out_enable = en;
            self.output.set_state_z(!en);
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
        let pin_action = self.output.schedule_state(val & 1 != 0, 0);
        GateUpdate {
            stamp_changed: matches!(pin_action, PinAction::Immediate),
            device_event: None,
            pin_action,
        }
    }
}

pub fn apply_family_out(family: &LogicFamily, pin: &mut IoPin) {
    pin.set_levels(family.out_high_v, family.out_low_v);
    pin.set_output_imp(family.out_imp);
    pin.set_thresholds(family.inp_high_v, family.inp_low_v);
    pin.time_ris = family.pin_rise_ps();
    pin.time_fal = family.pin_fall_ps();
}

#[derive(Clone, Copy, Debug)]
pub struct GateUpdate {
    pub stamp_changed: bool,
    pub device_event: Option<u64>,
    pub pin_action: PinAction,
}

impl GateUpdate {
    pub fn none() -> Self {
        Self {
            stamp_changed: false,
            device_event: None,
            pin_action: PinAction::None,
        }
    }
}
