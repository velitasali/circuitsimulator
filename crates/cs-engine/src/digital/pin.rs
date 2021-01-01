//! C++ `IoPin`: digital I/O that stamps a Thevenin/Norton equivalent to ground.

use crate::CERO_DOUB;
use crate::HIGH_IMP;
use crate::LOW_IMP;

/// C++ `pinMode_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinMode {
    Undef,
    Input,
    OpenCo,
    Output,
    Source,
}

impl PinMode {
    /// C++ `pinMode_t`: `undef_mode=0`, `input=1`, `openCo=2`, `output=3`, `source=4`.
    pub fn from_u32(m: u32) -> Self {
        match m {
            1 => Self::Input,
            2 => Self::OpenCo,
            3 => Self::Output,
            4 => Self::Source,
            _ => Self::Undef,
        }
    }

    pub fn as_u32(self) -> u32 {
        match self {
            Self::Undef => 0,
            Self::Input => 1,
            Self::OpenCo => 2,
            Self::Output => 3,
            Self::Source => 4,
        }
    }
}

/// C++ open-collector high `m_gndAdmit = 1/1e8`.
pub const OC_HIGH_ADMIT: f64 = 1.0 / 1e8;
/// C++ `IoPin` constructor `m_outputImp = 40`.
pub const DEFAULT_OUT_IMP: f64 = 40.0;
/// C++ constructor `m_inputImp = high_imp`.
pub const DEFAULT_IN_IMP: f64 = HIGH_IMP;
/// C++ constructor `m_openImp = 1e9`.
pub const DEFAULT_OPEN_IMP: f64 = 1e9;
/// C++ constructor `m_timeRis` / `m_timeFal` (picoseconds).
pub const DEFAULT_EDGE_PS: u64 = 3750;
/// C++ `IoComponent::setRiseTime` multiplies family LH/HL by 1.25 for the pin.
pub const EDGE_PIN_MULT: f64 = 1.25;

#[derive(Clone, Debug)]
pub struct IoPin {
    pub id: String,
    pub node_idx: Option<usize>,
    pub mode: PinMode,
    pub inverted: bool,
    pub inp_high_v: f64,
    pub inp_low_v: f64,
    pub out_high_v: f64,
    pub out_low_v: f64,
    pub input_imp: f64,
    pub output_imp: f64,
    pub open_imp: f64,
    pub time_ris: u64,
    pub time_fal: u64,
    vdd_admit: f64,
    gnd_admit: f64,
    vdd_adm_ex: f64,
    gnd_adm_ex: f64,
    admit: f64,
    out_volt: f64,
    inp_state: bool,
    out_state: bool,
    next_state: bool,
    state_z: bool,
    /// Last sampled node voltage (C++ `ePin::getVoltage`).
    last_volt: f64,
    /// Current slope sample (C++ `m_step`).
    slope_step: i32,
    /// C++ `m_steps` from `Simulator::slopeSteps()`.
    slope_steps: i32,
    /// C++ `McuPin::m_outCtrl`: a peripheral is driving the pin.
    pub out_ctrl: bool,
    /// C++ `McuPin::m_dirCtrl`.
    pub dir_ctrl: bool,
    /// C++ `McuPin::m_portState` (port latch).
    pub port_state: bool,
    /// C++ `McuPin::m_isOut`.
    pub is_out: bool,
    /// C++ `extIntTrig_t` (Low=0 … Disabled=4).
    pub ext_int_trigger: u8,
}

impl IoPin {
    pub fn input(id: impl Into<String>) -> Self {
        let mut p = Self::blank(id);
        p.apply_mode(PinMode::Input);
        p
    }

    pub fn output(id: impl Into<String>) -> Self {
        let mut p = Self::blank(id);
        p.out_high_v = 5.0;
        p.out_low_v = 0.0;
        p.apply_mode(PinMode::Output);
        p
    }

    pub fn open_collector(id: impl Into<String>) -> Self {
        let mut p = Self::blank(id);
        p.out_high_v = 5.0;
        p.out_low_v = 0.0;
        p.apply_mode(PinMode::OpenCo);
        p
    }

    fn blank(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            node_idx: None,
            mode: PinMode::Undef,
            inverted: false,
            inp_high_v: 2.5,
            inp_low_v: 2.5,
            out_high_v: 5.0,
            out_low_v: 0.0,
            input_imp: DEFAULT_IN_IMP,
            output_imp: DEFAULT_OUT_IMP,
            open_imp: DEFAULT_OPEN_IMP,
            time_ris: DEFAULT_EDGE_PS,
            time_fal: DEFAULT_EDGE_PS,
            vdd_admit: 0.0,
            gnd_admit: CERO_DOUB,
            vdd_adm_ex: 0.0,
            gnd_adm_ex: 0.0,
            admit: 1.0 / CERO_DOUB,
            out_volt: CERO_DOUB,
            inp_state: false,
            out_state: false,
            next_state: false,
            state_z: false,
            last_volt: 0.0,
            slope_step: 0,
            slope_steps: 0,
            out_ctrl: false,
            dir_ctrl: false,
            port_state: false,
            is_out: false,
            ext_int_trigger: 0,
        }
    }

    pub fn initialize(&mut self, slope_steps: i32) {
        self.slope_step = 0;
        self.slope_steps = slope_steps.max(0);
        self.inp_state = false;
        self.out_state = false;
        self.next_state = false;
        self.out_ctrl = false;
        self.dir_ctrl = false;
        self.port_state = false;
        self.is_out = matches!(
            self.mode,
            PinMode::Output | PinMode::OpenCo | PinMode::Source
        );
        self.ext_int_trigger = 0;
    }

    /// C++ `McuPin::setDirection`. `out` true → output.
    pub fn set_direction(&mut self, out: bool) {
        self.is_out = out;
        if self.dir_ctrl {
            return;
        }
        self.set_pin_mode(if out { PinMode::Output } else { PinMode::Input });
    }

    /// C++ `McuPin::setPortState`.
    pub fn set_port_state(&mut self, state: bool) {
        self.port_state = state;
        if self.out_ctrl {
            return;
        }
        self.out_state = state;
        if self.is_out {
            self.set_out_state(state);
        }
    }

    /// C++ `McuPin::controlPin`.
    pub fn control_pin(&mut self, out_ctrl: bool, dir_ctrl: bool) {
        if !dir_ctrl && self.dir_ctrl {
            self.set_pin_mode(if self.is_out {
                PinMode::Output
            } else {
                PinMode::Input
            });
        }
        self.dir_ctrl = dir_ctrl;
        if !out_ctrl && self.out_ctrl {
            if self.is_out {
                self.set_out_state(self.port_state);
            } else {
                self.out_state = self.port_state;
            }
        }
        self.out_ctrl = out_ctrl;
    }

    /// C++ `McuPin::setOutState` (peripheral drive; no-op unless `out_ctrl`).
    pub fn mcu_set_out_state(&mut self, state: bool) {
        if self.out_ctrl {
            self.set_out_state(state);
        }
    }

    /// C++ `McuPin::setExtInt`.
    pub fn set_ext_int(&mut self, mode: u32) {
        self.ext_int_trigger = mode.min(4) as u8;
    }

    pub fn set_pin_mode(&mut self, mode: PinMode) {
        if self.mode == mode {
            return;
        }
        self.apply_mode(mode);
    }

    fn apply_mode(&mut self, mode: PinMode) {
        self.mode = mode;
        match mode {
            PinMode::Undef => return,
            PinMode::Input => {
                self.vdd_admit = 0.0;
                self.gnd_admit = 1.0 / self.input_imp.max(1e-14);
            }
            PinMode::Output => {
                self.admit = 1.0 / self.output_imp.max(1e-14);
                self.vdd_admit = self.admit;
                self.gnd_admit = 0.0;
            }
            PinMode::OpenCo => {
                self.vdd_admit = 0.0;
            }
            PinMode::Source => {
                self.vdd_admit = HIGH_IMP;
                self.gnd_admit = LOW_IMP;
                self.out_state = true;
            }
        }
        if matches!(mode, PinMode::OpenCo | PinMode::Output | PinMode::Source) {
            self.set_out_state(self.out_state);
        } else {
            self.updt_state();
        }
    }

    pub fn set_levels(&mut self, high: f64, low: f64) {
        self.out_high_v = high;
        self.out_low_v = low;
        if matches!(
            self.mode,
            PinMode::OpenCo | PinMode::Output | PinMode::Source
        ) {
            self.set_out_state(self.out_state);
        }
    }

    pub fn set_thresholds(&mut self, high: f64, low: f64) {
        self.inp_high_v = high;
        self.inp_low_v = low;
    }

    pub fn set_output_imp(&mut self, imp: f64) {
        self.output_imp = imp.max(1e-14);
        if self.mode == PinMode::Output {
            self.vdd_admit = 1.0 / self.output_imp;
            self.updt_state();
        }
    }

    pub fn set_input_imp(&mut self, imp: f64) {
        self.input_imp = imp.max(1e-14);
        if self.mode == PinMode::Input {
            self.gnd_admit = 1.0 / self.input_imp;
            self.updt_state();
        }
    }

    pub fn set_open_imp(&mut self, imp: f64) {
        self.open_imp = imp.max(1e-14);
        if self.mode == PinMode::Output && self.state_z {
            self.set_impedance(self.open_imp);
        }
    }

    pub fn set_pullup(&mut self, p: f64) {
        self.vdd_adm_ex = if p > 0.0 { 1.0 / p } else { 0.0 };
        if matches!(self.mode, PinMode::Undef | PinMode::Input | PinMode::OpenCo) || self.state_z {
            self.updt_state();
        }
    }

    pub fn has_pullup(&self) -> bool {
        self.vdd_adm_ex > 0.0
    }

    pub fn set_inverted(&mut self, invert: bool) {
        if self.inverted == invert {
            return;
        }
        self.inverted = invert;
        if matches!(
            self.mode,
            PinMode::OpenCo | PinMode::Output | PinMode::Source
        ) {
            self.set_out_state(self.out_state);
        }
    }

    pub fn set_state_z(&mut self, z: bool) {
        self.state_z = z;
        if z {
            self.out_volt = self.out_low_v;
            self.set_impedance(self.open_imp);
        } else {
            let pm = self.mode;
            self.mode = PinMode::Undef;
            self.apply_mode(pm);
        }
    }

    fn set_impedance(&mut self, imp: f64) {
        self.admit = 1.0 / imp.max(1e-14);
        let vdd = self.vdd_admit + self.vdd_adm_ex;
        let current = self.out_high_v * vdd;
        self.out_volt = if self.admit.abs() > 0.0 {
            current / self.admit
        } else {
            0.0
        };
    }

    fn updt_state(&mut self) {
        let vdd = self.vdd_admit + self.vdd_adm_ex;
        let gnd = self.gnd_admit + self.gnd_adm_ex;
        let current = self.out_high_v * vdd;
        self.admit = vdd + gnd;
        self.out_volt = if self.admit.abs() > 0.0 {
            current / self.admit
        } else {
            0.0
        };
    }

    /// Last `getInpState` return (already inverted).
    pub fn last_inp_state(&self) -> bool {
        if self.inverted {
            !self.inp_state
        } else {
            self.inp_state
        }
    }

    /// C++ `getVoltage`.
    pub fn get_voltage(&self) -> f64 {
        self.last_volt
    }

    pub fn out_volt(&self) -> f64 {
        self.out_volt
    }

    pub fn voltage(&self) -> f64 {
        if self.out_volt.abs() > 1e-9 {
            self.out_volt
        } else {
            self.last_volt
        }
    }

    /// C++ `setVoltage`.
    pub fn set_voltage(&mut self, volt: f64) {
        self.out_volt = volt;
    }

    /// C++ `setOutStatFast`.
    pub fn set_out_stat_fast(&mut self, high: bool) {
        self.out_state = high;
        self.next_state = high;
        self.out_volt = if high {
            self.out_high_v
        } else {
            self.out_low_v
        };
    }

    /// C++ `getInpState`.
    pub fn get_inp_state(&mut self, volt: f64) -> bool {
        self.last_volt = volt;
        if volt > self.inp_high_v {
            self.inp_state = true;
        } else if volt < self.inp_low_v {
            self.inp_state = false;
        }
        if self.inverted {
            !self.inp_state
        } else {
            self.inp_state
        }
    }

    /// Read sampled input state.
    pub fn inp_state(&self) -> bool {
        if self.inverted {
            !self.inp_state
        } else {
            self.inp_state
        }
    }

    /// C++ `getOutState`.
    pub fn get_out_state(&self) -> bool {
        if self.slope_step != 0 {
            self.next_state
        } else {
            self.out_state
        }
    }

    /// C++ `setOutState`.
    pub fn set_out_state(&mut self, high: bool) {
        self.out_state = high;
        self.next_state = high;
        if matches!(self.mode, PinMode::Undef | PinMode::Input) || self.state_z {
            return;
        }
        let mut high = high;
        if self.inverted {
            high = !high;
        }
        if self.mode == PinMode::OpenCo {
            self.gnd_admit = if high {
                OC_HIGH_ADMIT
            } else {
                1.0 / self.output_imp.max(1e-14)
            };
            self.updt_state();
        } else {
            self.out_volt = if high {
                self.out_high_v
            } else {
                self.out_low_v
            };
        }
    }

    /// Norton pair stamped to implicit ground: `(V, G)`.
    pub fn norton(&self) -> (f64, f64) {
        (self.out_volt, self.admit.max(0.0))
    }

    pub fn current_out(&self, node_volt: f64) -> f64 {
        match self.mode {
            PinMode::Input => node_volt * self.admit,
            PinMode::Output | PinMode::OpenCo => (self.out_volt - node_volt) * self.admit,
            _ => 0.0,
        }
    }

    /// C++ `scheduleState`. `time` is extra delay in picoseconds.
    pub fn schedule_state(&mut self, state: bool, time: u64) -> PinAction {
        if self.next_state == state {
            return PinAction::None;
        }
        self.next_state = state;
        if self.slope_step != 0 {
            self.slope_step = self.slope_steps - self.slope_step;
        }
        if time > 0 {
            PinAction::Event {
                delay_ps: time,
                cancel: true,
            }
        } else if self.slope_steps > 0 {
            self.run_slope()
        } else {
            self.set_out_state(self.next_state);
            PinAction::Immediate
        }
    }

    /// C++ `IoPin::runEvent` (rise/fall staircase).
    pub fn run_slope(&mut self) -> PinAction {
        if self.slope_step == self.slope_steps {
            self.slope_step = 0;
            self.set_out_state(self.next_state);
            return PinAction::Immediate;
        }
        let mut next = self.next_state;
        if self.inverted {
            next = !next;
        }
        if self.mode == PinMode::OpenCo {
            let step = if next {
                f64::from(self.slope_step)
            } else {
                f64::from(self.slope_steps - self.slope_step)
            };
            let delta = (1e4 * step / f64::from(self.slope_steps.max(1))).powi(2);
            self.gnd_admit = 1.0 / (self.output_imp + delta);
            self.updt_state();
        } else {
            let mut delta = f64::from(self.slope_step);
            if self.slope_step == 0 {
                delta = 1e-5;
            }
            let span = self.out_high_v - self.out_low_v;
            let steps = f64::from(self.slope_steps.max(1));
            self.out_volt = if next {
                self.out_low_v + delta * span / steps
            } else {
                self.out_high_v - delta * span / steps
            };
        }
        let edge = if next { self.time_ris } else { self.time_fal };
        let delay = (edge / self.slope_steps.max(1) as u64).max(1);
        self.slope_step += 1;
        PinAction::Event {
            delay_ps: delay,
            cancel: false,
        }
    }
}

/// What the simulator should do after a pin mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinAction {
    None,
    Immediate,
    Event { delay_ps: u64, cancel: bool },
}
