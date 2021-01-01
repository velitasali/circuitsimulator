//! C++ `IoPort`: a named group of [`IoPin`]s for AngelScript and scripted MCUs.

use super::{IoPin, PinAction, PinMode};

#[derive(Clone, Debug)]
pub struct OutState {
    pub time: u64,
    pub state: u64,
}

#[derive(Clone, Debug)]
pub struct IoPort {
    pub name: String,
    pin_state: u32,
    next_state: u32,
    pin_direction: u32,
    pin_mode: PinMode,
    pub pins: Vec<IoPin>,
    index: usize,
    sequences: Vec<Vec<OutState>>,
    pending_event: Option<u64>,
}

impl IoPort {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pin_state: 0,
            next_state: 0,
            pin_direction: 0,
            pin_mode: PinMode::Input,
            pins: Vec::new(),
            index: 0,
            sequences: Vec::new(),
            pending_event: None,
        }
    }

    /// C++ `createPins` with a numeric width.
    pub fn with_pins(name: impl Into<String>, owner: &str, n: usize) -> Self {
        let name = name.into();
        let mut p = Self::new(name.clone());
        for i in 0..n {
            let mut pin = IoPin::input(format!("{owner}-{name}{i}"));
            pin.set_levels(5.0, 0.0);
            p.pins.push(pin);
        }
        p
    }

    /// C++ `createPins` with a comma-separated name list (`pins="TX,RX"`).
    /// Pin id is `{owner}-{portName}{label}` (`Script-1-PTX`).
    pub fn with_named_pins(name: impl Into<String>, owner: &str, labels: &[String]) -> Self {
        let name = name.into();
        let mut p = Self::new(name.clone());
        for label in labels {
            let mut pin = IoPin::input(format!("{owner}-{name}{label}"));
            pin.set_levels(5.0, 0.0);
            p.pins.push(pin);
        }
        p
    }

    pub fn size(&self) -> usize {
        self.pins.len()
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.pin_state = 0;
        self.next_state = 0;
        self.pin_direction = 0;
        self.pin_mode = PinMode::Input;
        self.sequences.clear();
        self.pending_event = None;
        for p in &mut self.pins {
            p.set_out_state(false);
            p.set_pin_mode(PinMode::Input);
        }
    }

    pub fn set_out_state(&mut self, val: u32) {
        let changed = self.pin_state ^ val;
        if changed == 0 {
            return;
        }
        self.pin_state = val;
        for (bit, p) in self.pins.iter_mut().enumerate() {
            let flag = 1u32 << bit;
            if changed & flag != 0 {
                p.set_out_state(val & flag != 0);
            }
        }
    }

    pub fn set_out_stat_fast(&mut self, val: u32) {
        let changed = self.pin_state ^ val;
        if changed == 0 {
            return;
        }
        self.pin_state = val;
        for (bit, p) in self.pins.iter_mut().enumerate() {
            let flag = 1u32 << bit;
            if changed & flag != 0 {
                p.set_out_stat_fast(val & flag != 0);
            }
        }
    }

    pub fn get_inp_state(&self) -> u32 {
        let mut data = 0u32;
        for (bit, p) in self.pins.iter().enumerate() {
            if p.last_inp_state() {
                data |= 1 << bit;
            }
        }
        data
    }

    pub fn set_direction(&mut self, val: u32) {
        let changed = self.pin_direction ^ val;
        if changed == 0 {
            return;
        }
        self.pin_direction = val;
        for (bit, p) in self.pins.iter_mut().enumerate() {
            let flag = 1u32 << bit;
            if changed & flag != 0 {
                p.set_pin_mode(if val & flag != 0 {
                    PinMode::Output
                } else {
                    PinMode::Input
                });
            }
        }
    }

    pub fn set_pin_mode(&mut self, mode: PinMode) {
        if self.pin_mode == mode {
            return;
        }
        self.pin_mode = mode;
        for p in &mut self.pins {
            p.set_pin_mode(mode);
        }
    }

    /// C++ `McuPort::controlPort`.
    pub fn control_port(&mut self, out_ctrl: bool, dir_ctrl: bool) {
        for p in &mut self.pins {
            p.control_pin(out_ctrl, dir_ctrl);
        }
    }

    /// C++ `McuPort::setDirection` (per-pin `McuPin::setDirection`).
    pub fn mcu_set_direction(&mut self, val: u32) {
        for (bit, p) in self.pins.iter_mut().enumerate() {
            p.set_direction(val & (1u32 << bit) != 0);
        }
    }

    /// C++ `McuPort::setOutState` (peripheral `McuPin::setOutState`).
    pub fn mcu_set_out_state(&mut self, val: u32) {
        for (bit, p) in self.pins.iter_mut().enumerate() {
            let state = val & (1u32 << bit) != 0;
            if state != p.get_out_state() {
                p.mcu_set_out_state(state);
            }
        }
    }

    /// C++ `scheduleState`. Returns the event delay if one was queued.
    pub fn schedule_state(&mut self, val: u32, time: u64) -> Option<u64> {
        if self.pin_state == val {
            return None;
        }
        self.next_state = val;
        if time == 0 {
            self.set_out_state(val);
            None
        } else {
            self.pending_event = Some(time);
            Some(time)
        }
    }

    pub fn take_pending_event(&mut self) -> Option<u64> {
        self.pending_event.take()
    }

    pub fn run_event(&mut self) -> PinAction {
        self.set_out_state(self.next_state);
        PinAction::Immediate
    }

    pub fn add_sequence(&mut self, seq: Vec<OutState>) {
        if !seq.is_empty() {
            self.sequences.push(seq);
        }
    }

    pub fn trigger(&mut self, n: u32) -> Option<u64> {
        if self.index != 0 {
            return None;
        }
        let seq = self.sequences.get(n as usize)?;
        let step = seq.first()?;
        self.next_state = step.state as u32;
        self.index = 1;
        if self.index >= seq.len() {
            self.index = 0;
        }
        if step.time == 0 {
            self.set_out_state(self.next_state);
            None
        } else {
            Some(step.time)
        }
    }

    pub fn get_pin_n(&mut self, i: u8) -> Option<&mut IoPin> {
        self.pins.get_mut(i as usize)
    }

    pub fn get_pin(&mut self, pin_name: &str) -> Option<&mut IoPin> {
        let short = format!("P{}", self.name.chars().last().unwrap_or('?'));
        if let Some(rest) = pin_name
            .strip_prefix(&self.name)
            .or_else(|| pin_name.strip_prefix(&short))
        {
            if let Ok(n) = rest.parse::<usize>() {
                return self.pins.get_mut(n);
            }
        }
        self.pins.iter_mut().find(|p| {
            p.id.rsplit('-')
                .next()
                .is_some_and(|s| s == pin_name || s.strip_prefix(&self.name) == Some(pin_name))
        })
    }
}
