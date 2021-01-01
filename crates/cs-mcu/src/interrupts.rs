//! C++ `Interrupt` / `Interrupts`: flag, enable, pending list, vector dispatch.

use std::collections::HashMap;

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, InterruptsSpec};

/// Module notified when an interrupt is raised (C++ `Interrupt::callBack`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntCallback {
    /// 8051 USART: 16 timer overflows per bit (`I51Usart::callBack`).
    UsartTick(usize),
}

#[derive(Clone, Debug)]
pub struct Interrupt {
    pub name: String,
    pub vector: u16,
    pub enabled: u8,
    pub raised: bool,
    pub priority: u8,
    pub flag_mask: u8,
    pub flag_reg: u16,
    pub auto_clear: bool,
    pub remember: bool,
    pub continuous: bool,
    pub wakeup: u8,
    pub callbacks: Vec<IntCallback>,
}

impl Interrupt {
    pub fn reset(&mut self) {
        self.enabled = 0;
        self.raised = false;
        self.continuous = false;
    }
}

#[derive(Clone, Debug, Default)]
pub struct Interrupts {
    pub enabled: u8,
    pub reti: bool,
    pub en_global: RegBits,
    pub ints: Vec<Interrupt>,
    by_name: HashMap<String, usize>,
    pending: Vec<usize>,
    running: Vec<usize>,
    active: Option<usize>,
}

impl Interrupts {
    pub fn from_spec(spec: &InterruptsSpec, data: &DataSpace, core: CoreKind) -> Self {
        let mut ints = Vec::new();
        let mut by_name = HashMap::new();
        let pic = matches!(core, CoreKind::Pic12 | CoreKind::Pic14);
        for s in &spec.ints {
            let bits = if s.flag.is_empty() {
                RegBits::default()
            } else {
                data.get_reg_bits(&s.flag)
            };
            let priority = if s.priority.is_empty() {
                0
            } else if let Ok(p) = if s.priority.starts_with("0x") || s.priority.starts_with("0X") {
                u8::from_str_radix(&s.priority[2..], 16)
            } else {
                s.priority.parse::<u8>()
            } {
                p
            } else {
                0
            };
            let auto_clear = s.auto_clear.unwrap_or(!pic);
            let idx = ints.len();
            by_name.insert(s.name.clone(), idx);
            ints.push(Interrupt {
                name: s.name.clone(),
                vector: s.vector,
                enabled: 0,
                raised: false,
                priority,
                flag_mask: bits.mask,
                flag_reg: bits.reg_addr,
                auto_clear,
                remember: true,
                continuous: false,
                wakeup: s.wakeup,
                callbacks: Vec::new(),
            });
        }
        Self {
            enabled: 0,
            reti: false,
            en_global: if spec.enable.is_empty() {
                RegBits::default()
            } else {
                data.get_reg_bits(&spec.enable)
            },
            ints,
            by_name,
            pending: Vec::new(),
            running: Vec::new(),
            active: None,
        }
    }

    pub fn get(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    pub fn reset(&mut self) {
        if self.en_global.reg_addr != 0 {
            self.enabled = 0;
        } else {
            self.enabled = 1;
        }
        self.reti = false;
        self.active = None;
        self.pending.clear();
        self.running.clear();
        for i in &mut self.ints {
            i.reset();
        }
    }

    pub fn enable_global(&mut self, en: u8) {
        self.enabled = en;
    }

    /// C++ `Interrupt::setContinuous` (AVR INT0 low-level).
    pub fn set_continuous(&mut self, idx: usize, c: bool, data: &mut DataSpace) {
        let raised = {
            let Some(iv) = self.ints.get_mut(idx) else {
                return;
            };
            if iv.continuous == c {
                return;
            }
            iv.continuous = c;
            iv.auto_clear = !c;
            iv.raised
        };
        if raised {
            self.clear_flag(idx, data);
        }
    }

    /// C++ `Interrupt::setAutoClear`.
    pub fn set_auto_clear(&mut self, idx: usize, a: bool) {
        if let Some(iv) = self.ints.get_mut(idx) {
            iv.auto_clear = a;
        }
    }

    pub fn ret_i(&mut self) {
        self.reti = true;
    }

    pub fn write_global_flag(&mut self, data: &mut DataSpace, flag: u8) {
        if self.en_global.reg_addr != 0 || self.en_global.mask != 0 {
            let mut r = data.get(self.en_global.reg_addr);
            if flag != 0 {
                r |= self.en_global.mask;
            } else {
                r &= !self.en_global.mask;
            }
            data.set(self.en_global.reg_addr, r);
        }
        self.enabled = flag;
    }

    pub fn enable_flag(&mut self, idx: usize, en: u8) {
        let Some(iv) = self.ints.get_mut(idx) else {
            return;
        };
        if iv.enabled == en {
            return;
        }
        iv.enabled = en;
        let remember = iv.raised && iv.remember && en != 0;
        if en == 0 {
            self.rem_pending(idx);
        } else if remember {
            self.add_pending(idx);
        }
    }

    pub fn set_priority(&mut self, idx: usize, p: u8) {
        if let Some(iv) = self.ints.get_mut(idx) {
            iv.priority = p;
        }
    }

    pub fn raise(&mut self, idx: usize, data: &mut DataSpace) -> Vec<IntCallback> {
        self.raise_val(idx, 1, data)
    }

    pub fn raise_val(&mut self, idx: usize, v: u8, data: &mut DataSpace) -> Vec<IntCallback> {
        let Some(iv) = self.ints.get_mut(idx) else {
            return Vec::new();
        };
        if v == 0 {
            if iv.auto_clear || iv.continuous {
                self.clear_flag(idx, data);
            }
            return Vec::new();
        }
        let cbs = iv.callbacks.clone();
        if iv.raised {
            return cbs;
        }
        iv.raised = true;
        let continuous = iv.continuous;
        let flag_mask = iv.flag_mask;
        let flag_reg = iv.flag_reg;
        let enabled = iv.enabled;
        if !continuous && flag_mask != 0 {
            let r = data.get(flag_reg) | flag_mask;
            data.set(flag_reg, r);
        }
        if enabled != 0 {
            self.add_pending(idx);
        }
        cbs
    }

    pub fn clear_flag(&mut self, idx: usize, data: &mut DataSpace) {
        let Some(iv) = self.ints.get_mut(idx) else {
            return;
        };
        if !iv.raised {
            return;
        }
        iv.raised = false;
        let flag_mask = iv.flag_mask;
        let flag_reg = iv.flag_reg;
        if flag_mask != 0 {
            let r = data.get(flag_reg) & !flag_mask;
            data.set(flag_reg, r);
        }
        self.rem_pending(idx);
    }

    pub fn flag_cleared(&mut self, idx: usize) {
        let Some(iv) = self.ints.get_mut(idx) else {
            return;
        };
        if !iv.raised {
            return;
        }
        iv.raised = false;
        self.rem_pending(idx);
    }

    /// C++ `Interrupt::writeFlag`: write 1 to clear (AVR `clear="1"`).
    pub fn write_flag(&mut self, idx: usize, v: u8, data: &mut DataSpace) {
        let Some(iv) = self.ints.get(idx) else {
            return;
        };
        let mask = iv.flag_mask;
        let reg = iv.flag_reg;
        let mut over = data.reg_override.unwrap_or(v);
        if over & mask != 0 {
            over &= !mask;
            data.reg_override = Some(over);
            self.flag_cleared(idx);
        } else if data.get(reg) & mask != 0 {
            data.reg_override = Some(over | mask);
        }
    }

    fn add_pending(&mut self, idx: usize) {
        if self.pending.contains(&idx) {
            return;
        }
        let prio = self.ints[idx].priority;
        let pos = self
            .pending
            .iter()
            .position(|&i| prio > self.ints[i].priority)
            .unwrap_or(self.pending.len());
        self.pending.insert(pos, idx);
    }

    fn rem_pending(&mut self, idx: usize) {
        self.pending.retain(|&i| i != idx);
    }

    /// C++ `Interrupts::runInterrupts`. Returns the vector to `INTERRUPT`.
    #[inline]
    pub fn run(&mut self, data: &mut DataSpace) -> Option<u16> {
        if !self.reti && (self.enabled == 0 || self.pending.is_empty()) {
            return None;
        }
        if self.reti {
            self.reti = false;
            let Some(active) = self.active else {
                return None;
            };
            let auto = self.ints[active].auto_clear;
            if auto {
                self.clear_flag(active, data);
            }
            if let Some(prev) = self.running.pop() {
                self.active = Some(prev);
            } else {
                let cont = self.ints[active].continuous && self.ints[active].raised;
                self.active = None;
                if cont {
                    self.add_pending(active);
                }
            }
            self.write_global_flag(data, 1);
            return None;
        }
        if self.enabled == 0 {
            return None;
        }
        let Some(&pending) = self.pending.first() else {
            return None;
        };
        if let Some(active) = self.active {
            if self.ints[pending].priority > self.ints[active].priority {
                self.running.push(active);
            } else {
                return None;
            }
        }
        self.pending.remove(0);
        self.active = Some(pending);
        self.write_global_flag(data, 0);
        let vector = self.ints[pending].vector;
        if vector == 0 { None } else { Some(vector) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::InterruptSpec;

    fn data_with_intcon() -> DataSpace {
        let mut d = DataSpace::new(32);
        d.add_register(
            "INTCON",
            0x0B,
            0,
            None,
            "RBIF,INTF,T0IF,RBIE,INTE,T0IE,PEIE,GIE",
        );
        d
    }

    #[test]
    fn raise_then_execute_then_reti() {
        let data = data_with_intcon();
        let spec = InterruptsSpec {
            enable: "GIE".into(),
            ints: vec![InterruptSpec {
                name: "T0_OVF".into(),
                vector: 4,
                enable: "T0IE".into(),
                flag: "T0IF".into(),
                priority: "1".into(),
                ..Default::default()
            }],
        };
        let mut ints = Interrupts::from_spec(&spec, &data, CoreKind::Pic14);
        let mut data = data;
        ints.reset();
        ints.enable_flag(0, 1);
        ints.enable_global(1);
        ints.raise(0, &mut data);
        assert_ne!(data.get(0x0B) & (1 << 2), 0); // T0IF
        assert_eq!(ints.run(&mut data), Some(4));
        assert_eq!(ints.enabled, 0);
        ints.ret_i();
        assert_eq!(ints.run(&mut data), None);
        assert_ne!(ints.enabled, 0);
        assert_ne!(data.get(0x0B) & (1 << 7), 0); // GIE
    }
}
