//! C++ `Comparator`: analog differential compare, digital IoPin output
//! with `IoComponent` delay and optional rise/fall.

use crate::digital::{
    GateUpdate, IoPin, LogicFamily, OutQueue, PinAction, PinMode, Schedule, apply_family_out,
};

/// C++ `LogicFamily` constructor `m_outHighV = 5`.
pub const COMPARATOR_DEFAULT_OUT_HIGH: f64 = 5.0;
/// C++ `m_outLowV = 0`.
pub const COMPARATOR_DEFAULT_OUT_LOW: f64 = 0.0;
/// C++ `m_outImp = 40`.
pub const COMPARATOR_DEFAULT_OUT_IMP: f64 = 40.0;

#[derive(Clone, Debug)]
pub struct ComparatorState {
    pub family: LogicFamily,
    pub inverted: bool,
    pub open_col: bool,
    pub output: IoPin,
    pub queue: OutQueue,
}

impl Default for ComparatorState {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparatorState {
    pub fn new() -> Self {
        let family = LogicFamily::new();
        let mut output = IoPin::output(String::new());
        apply_family_out(&family, &mut output);
        let mut s = Self {
            family,
            inverted: false,
            open_col: false,
            output,
            queue: OutQueue::new(),
        };
        s.reset_stamp();
        s
    }

    pub fn attach_id(&mut self, id: &str) {
        self.output.id = format!("{id}-out");
    }

    pub fn out_high(&self) -> f64 {
        self.family.out_high_v
    }
    pub fn out_low(&self) -> f64 {
        self.family.out_low_v
    }
    pub fn out_imp(&self) -> f64 {
        self.family.out_imp
    }

    /// Electrical output voltage currently stamped (DC Thevenin).
    pub fn last_out(&self) -> f64 {
        self.output.norton().0
    }

    pub fn admit(&self) -> f64 {
        self.output.norton().1
    }

    pub fn set_out_imp(&mut self, imp: f64) {
        self.family.out_imp = imp.max(1e-12);
        self.output.set_output_imp(self.family.out_imp);
    }

    pub fn apply_electric(&mut self) {
        self.output
            .set_levels(self.family.out_high_v, self.family.out_low_v);
        self.output.set_output_imp(self.family.out_imp);
        self.output.set_inverted(self.inverted);
        self.output.set_pin_mode(if self.open_col {
            PinMode::OpenCo
        } else {
            PinMode::Output
        });
        self.output.time_ris = self.family.pin_rise_ps();
        self.output.time_fal = self.family.pin_fall_ps();
    }

    /// C++ `IoComponent::initState` then output low (or inverted high).
    pub fn reset_stamp(&mut self) {
        self.queue.clear();
        self.apply_electric();
        self.output.set_state_z(false);
        self.output.set_out_state(false);
        self.queue.out_value = 0;
        self.queue.next_out_val = 0;
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        self.output.initialize(slope_steps);
        self.reset_stamp();
    }

    /// Immediate DC compare used by unit tests (delay 0).
    pub fn volt_changed(&mut self, vp: f64, vn: f64) -> bool {
        let high = self.next_high(vp, vn);
        if high == (self.queue.out_value != 0) && self.output.get_out_state() == high {
            return true;
        }
        self.queue.next_out_val = u32::from(high);
        self.queue.out_value = self.queue.next_out_val;
        self.output.set_out_state(high);
        false
    }

    pub fn next_high(&self, vp: f64, vn: f64) -> bool {
        (vp - vn) > 0.0
    }

    /// C++ `Comparator::voltChanged` + `scheduleOutPuts`.
    pub fn schedule_from_compare(&mut self, vp: f64, vn: f64, circ_time: u64) -> GateUpdate {
        self.queue.next_out_val = u32::from(self.next_high(vp, vn));
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
        let pin_action = self.output.schedule_state(val & 1 != 0, 0);
        GateUpdate {
            stamp_changed: matches!(pin_action, PinAction::Immediate),
            device_event: None,
            pin_action,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_when_plus_above_minus() {
        let mut s = ComparatorState::new();
        s.reset_stamp();
        assert!(!s.volt_changed(5.0, 0.0));
        assert!((s.last_out() - 5.0).abs() < 1e-12);
        assert!(s.volt_changed(5.0, 0.0));
    }

    #[test]
    fn low_when_plus_below_minus() {
        let mut s = ComparatorState::new();
        s.reset_stamp();
        assert!(s.volt_changed(0.0, 5.0));
        assert!(s.last_out().abs() < 1e-12);
    }

    #[test]
    fn inverted_flips_the_output() {
        let mut s = ComparatorState::new();
        s.inverted = true;
        s.reset_stamp();
        assert!((s.last_out() - 5.0).abs() < 1e-12);
        assert!(!s.volt_changed(5.0, 0.0));
        assert!(s.last_out().abs() < 1e-12);
    }
}
