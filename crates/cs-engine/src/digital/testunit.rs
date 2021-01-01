//! C++ `TestUnit`: drive DUT inputs through every combination and check a
//! stored truth table. Headless only — canvas drawing waits on catalog-last.

use super::family::LogicFamily;
use super::gate::{GateUpdate, apply_family_out};
use super::pin::{IoPin, PinAction};
use super::{ps_to_secs, secs_to_ps};

/// C++ constructor `m_period = 1e-7` (100 ns).
pub const DEFAULT_PERIOD: f64 = 1e-7;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    pub id: String,
    pub ok: bool,
}

#[derive(Clone, Debug)]
pub struct TestUnitState {
    pub family: LogicFamily,
    /// Property `Inputs`: pins that drive the DUT. C++ `m_outPin` `{id}-outN`.
    pub drive: Vec<IoPin>,
    /// Property `Outputs`: pins that read the DUT. C++ `m_inpPin` `{id}-inN`.
    pub sense: Vec<IoPin>,
    pub input_str: String,
    pub output_str: String,
    /// Seconds between steps (C++ `m_period`).
    pub period: f64,
    pub truth: Vec<u32>,
    pub samples: Vec<u32>,
    /// Combination currently being driven (`m_outValue`).
    pub out_value: u32,
    pub read: bool,
    pub done: bool,
    pub ok: bool,
}

impl TestUnitState {
    pub fn new(id: &str) -> Self {
        let mut s = Self {
            family: LogicFamily::new(),
            drive: Vec::new(),
            sense: Vec::new(),
            input_str: String::new(),
            output_str: String::new(),
            period: DEFAULT_PERIOD,
            truth: Vec::new(),
            samples: Vec::new(),
            out_value: 0,
            read: false,
            done: false,
            ok: false,
        };
        s.set_inputs(id, "O");
        s.set_outputs(id, "I0,I1");
        s
    }

    pub fn pin_ids(&self) -> Vec<String> {
        self.drive
            .iter()
            .chain(self.sense.iter())
            .map(|p| p.id.clone())
            .collect()
    }

    pub fn pins_mut(&mut self) -> impl Iterator<Item = &mut IoPin> {
        self.drive.iter_mut().chain(self.sense.iter_mut())
    }

    pub fn apply_family(&mut self) {
        let f = self.family.clone();
        for p in &mut self.drive {
            apply_family_out(&f, p);
        }
        for p in &mut self.sense {
            p.set_thresholds(f.inp_high_v, f.inp_low_v);
            p.set_input_imp(f.inp_imp);
        }
    }

    pub fn set_period(&mut self, seconds: f64) {
        self.period = seconds.max(1e-15);
    }

    pub fn half_period_ps(&self) -> u64 {
        secs_to_ps(self.period / 2.0).max(1)
    }

    pub fn steps(&self) -> u32 {
        let n = self.drive.len();
        if n >= 32 { u32::MAX } else { 1u32 << n }
    }

    /// Duration of one full sweep: drive+read half-periods per combination.
    pub fn duration_ps(&self) -> u64 {
        self.half_period_ps()
            .saturating_mul(self.steps() as u64 * 2)
    }

    pub fn set_inputs(&mut self, id: &str, names: &str) {
        let names = split_names(names);
        self.input_str = names.join(",");
        self.drive = names
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mut p = IoPin::output(format!("{id}-out{i}"));
                apply_family_out(&self.family, &mut p);
                p
            })
            .collect();
        self.resize_vectors();
    }

    pub fn set_outputs(&mut self, id: &str, names: &str) {
        let names = split_names(names);
        self.output_str = names.join(",");
        self.sense = names
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mut p = IoPin::input(format!("{id}-in{i}"));
                p.set_thresholds(self.family.inp_high_v, self.family.inp_low_v);
                p.set_input_imp(self.family.inp_imp);
                p
            })
            .collect();
        self.resize_vectors();
    }

    pub fn set_truth_str(&mut self, t: &str) {
        let size = self.steps() as usize;
        let mut truth = Vec::new();
        for val in t.split(',') {
            let val = val.trim();
            if val.is_empty() {
                continue;
            }
            let n = u32::from_str_radix(val, 16).unwrap_or(0);
            truth.push(n);
            if truth.len() == size {
                break;
            }
        }
        if truth.len() < size {
            truth.resize(size, 0);
        }
        self.truth = truth;
    }

    pub fn truth_str(&self) -> String {
        self.truth
            .iter()
            .map(|v| format!("{v:x}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn resize_vectors(&mut self) {
        let n = self.steps() as usize;
        self.samples.clear();
        self.samples.resize(n, 0);
        if self.truth.len() != n {
            self.truth.clear();
            self.truth.resize(n, 0);
        }
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        for p in self.pins_mut() {
            p.initialize(slope_steps);
        }
        for p in &mut self.drive {
            p.set_out_state(false);
        }
        self.read = false;
        self.out_value = 0;
        self.done = false;
        self.ok = false;
        self.samples.fill(0);
    }

    /// C++ `TestUnit::runEvent`. `volt_of` maps pin id → node voltage.
    pub fn run_event(&mut self, volt_of: &impl Fn(&str) -> f64, _circ_time: u64) -> GateUpdate {
        if self.done {
            return GateUpdate::none();
        }
        let half = self.half_period_ps();
        if self.read {
            self.read = false;
            let mut input_val = 0u32;
            for (i, pin) in self.sense.iter_mut().enumerate() {
                if pin.get_inp_state(volt_of(&pin.id)) {
                    input_val |= 1 << i;
                }
            }
            let idx = self.out_value as usize;
            if idx < self.samples.len() {
                self.samples[idx] = input_val;
            }
            self.out_value = self.out_value.saturating_add(1);
            if self.out_value < self.steps() {
                GateUpdate {
                    stamp_changed: false,
                    device_event: Some(half),
                    pin_action: PinAction::None,
                }
            } else {
                self.done = true;
                self.ok = check_truth(&self.truth, &self.samples, self.sense.len());
                GateUpdate::none()
            }
        } else {
            self.read = true;
            for (i, pin) in self.drive.iter_mut().enumerate() {
                let state = self.out_value & (1 << i) != 0;
                pin.set_out_state(state);
            }
            GateUpdate {
                stamp_changed: true,
                device_event: Some(half),
                pin_action: PinAction::Immediate,
            }
        }
    }

    pub fn expected_time_s(&self) -> f64 {
        ps_to_secs(self.duration_ps())
    }
}

fn split_names(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty() && p != " ")
        .collect()
}

/// C++ `TruthTable::checkThruth`: each output bit of each row must match.
fn check_truth(truth: &[u32], samples: &[u32], n_outputs: usize) -> bool {
    let n = truth.len().min(samples.len());
    for row in 0..n {
        for bit in 0..n_outputs {
            let mask = 1u32 << bit;
            if (truth[row] & mask) != (samples[row] & mask) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pins() {
        let t = TestUnitState::new("TestUnit-1");
        assert_eq!(t.drive.len(), 1);
        assert_eq!(t.sense.len(), 2);
        assert_eq!(t.drive[0].id, "TestUnit-1-out0");
        assert_eq!(t.sense[0].id, "TestUnit-1-in0");
        assert_eq!(t.sense[1].id, "TestUnit-1-in1");
        assert_eq!(t.steps(), 2);
    }

    #[test]
    fn two_drive_pins_four_steps() {
        let mut t = TestUnitState::new("TU");
        t.set_inputs("TU", "A,B");
        t.set_outputs("TU", "Y");
        assert_eq!(t.drive.len(), 2);
        assert_eq!(t.sense.len(), 1);
        assert_eq!(t.steps(), 4);
        t.set_truth_str("0,0,0,1,");
        assert_eq!(t.truth, vec![0, 0, 0, 1]);
        assert_eq!(t.truth_str(), "0,0,0,1");
    }

    #[test]
    fn check_and_table() {
        assert!(check_truth(&[0, 0, 0, 1], &[0, 0, 0, 1], 1));
        assert!(!check_truth(&[0, 0, 0, 1], &[0, 0, 0, 0], 1));
    }
}
