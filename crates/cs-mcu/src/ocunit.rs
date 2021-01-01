//! C++ `McuOcUnit` / `AvrOcUnit`: compare match, pin drive, PWM COM actions.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::OcUnitSpec;
use crate::interrupts::{IntCallback, Interrupts};
use crate::port::{self, Port};

/// C++ `ocAct_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OcAct {
    Non = 0,
    Tog = 1,
    Clr = 2,
    Set = 3,
}

impl OcAct {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Tog,
            2 => Self::Clr,
            3 => Self::Set,
            _ => Self::Non,
        }
    }
}

/// C++ `AvrOcUnit` vs `PicOcUnit` / `PicPwmUnit`.
#[derive(Clone, Debug)]
enum OcKind {
    Avr,
    PicCompare {
        spec_event: bool,
        reset_timer: bool,
        go_done: RegBits,
    },
    PicPwm {
        ccprxl: u8,
        c_low: u8,
        dc_bits: RegBits,
    },
}

#[derive(Clone, Debug)]
pub struct OcUnit {
    pub name: String,
    pub pin: String,
    pub pin_inv: Option<String>,
    ocr_l: Option<u16>,
    ocr_h: Option<u16>,
    config_bits: RegBits,
    interrupt: Option<usize>,
    pub enabled: bool,
    ctrl_pin: bool,
    pub mode: u8,
    com_act: OcAct,
    tov_act: OcAct,
    pub com_match: u16,
    ext_match: u16,
    pin_set: bool,
    ocr_mask: u16,
    pub remain_ps: Option<u64>,
    kind: OcKind,
}

impl OcUnit {
    pub fn from_spec(spec: &OcUnitSpec, data: &DataSpace, ints: &Interrupts) -> Self {
        Self {
            name: spec.name.clone(),
            pin: spec.pin.clone(),
            pin_inv: None,
            ocr_l: spec.ocreg.first().and_then(|n| data.reg_addr(n)),
            ocr_h: spec.ocreg.get(1).and_then(|n| data.reg_addr(n)),
            config_bits: if spec.bits.is_empty() {
                RegBits::default()
            } else {
                data.get_reg_bits(&spec.bits)
            },
            interrupt: ints.get(&spec.interrupt),
            enabled: false,
            ctrl_pin: false,
            mode: 0,
            com_act: OcAct::Non,
            tov_act: OcAct::Non,
            com_match: 0,
            ext_match: 0,
            pin_set: true,
            ocr_mask: 0xFFFF,
            remain_ps: None,
            kind: OcKind::Avr,
        }
    }

    /// C++ `PicOcUnit` (CCP compare, TIMER1).
    pub fn pic_compare(
        name: impl Into<String>,
        pin: impl Into<String>,
        ints: &Interrupts,
        interrupt: &str,
        data: &DataSpace,
    ) -> Self {
        Self {
            name: name.into(),
            pin: pin.into(),
            pin_inv: None,
            ocr_l: None,
            ocr_h: None,
            config_bits: RegBits::default(),
            interrupt: ints.get(interrupt),
            enabled: false,
            ctrl_pin: false,
            mode: 0,
            com_act: OcAct::Non,
            tov_act: OcAct::Non,
            com_match: 0,
            ext_match: 0,
            pin_set: true,
            ocr_mask: 0xFFFF,
            remain_ps: None,
            kind: OcKind::PicCompare {
                spec_event: false,
                reset_timer: false,
                go_done: data.get_reg_bits("GO/DONE"),
            },
        }
    }

    /// C++ `PicPwmUnit00` (type 0, `DCxB`) / `PicPwmUnit01` (type 1, `CCPxY,CCPxX`).
    pub fn pic_pwm(
        name: impl Into<String>,
        pin: impl Into<String>,
        ints: &Interrupts,
        interrupt: &str,
        pwm_type: i32,
        data: &DataSpace,
    ) -> Self {
        let name = name.into();
        let n = name.chars().last().unwrap_or('1');
        let dc = if pwm_type == 1 {
            data.get_reg_bits(&format!("CCP{n}Y,CCP{n}X"))
        } else {
            data.get_reg_bits(&format!("DC{n}B0,DC{n}B1"))
        };
        Self {
            name,
            pin: pin.into(),
            pin_inv: None,
            ocr_l: None,
            ocr_h: None,
            config_bits: RegBits::default(),
            interrupt: ints.get(interrupt),
            enabled: false,
            ctrl_pin: false,
            mode: 0,
            com_act: OcAct::Non,
            tov_act: OcAct::Non,
            com_match: 0,
            ext_match: 0,
            pin_set: true,
            ocr_mask: 0xFFFF,
            remain_ps: None,
            kind: OcKind::PicPwm {
                ccprxl: 0,
                c_low: 0,
                dc_bits: dc,
            },
        }
    }

    /// C++ `AvrTimer810::updateOcUnit` inverted pin (`PORTB0` / `PORTB3`).
    pub fn set_pin_inv(&mut self, pin: Option<&str>, ports: &mut [Port]) {
        if let Some(old) = self.pin_inv.take() {
            port::control_gpio(ports, &old, false, false);
        }
        if let Some(p) = pin {
            port::control_gpio(ports, p, true, false);
            port::set_gpio_out(ports, p, false);
            self.pin_inv = Some(p.to_string());
        }
    }

    pub fn initialize(&mut self, ports: &mut [Port]) {
        self.com_match = 0;
        self.ext_match = 0;
        self.set_pin_inv(None, ports);
        port::control_gpio(ports, &self.pin, false, false);
        self.clear();
    }

    pub fn clear(&mut self) {
        self.enabled = false;
        self.ctrl_pin = false;
        self.mode = 0;
        self.com_act = OcAct::Non;
        self.tov_act = OcAct::Non;
        self.remain_ps = None;
        match &mut self.kind {
            OcKind::PicCompare {
                spec_event,
                reset_timer,
                ..
            } => {
                *spec_event = false;
                *reset_timer = false;
            }
            OcKind::PicPwm { ccprxl, c_low, .. } => {
                *ccprxl = 0;
                *c_low = 0;
            }
            OcKind::Avr => {}
        }
    }

    /// C++ `PicOcUnit::configure` — CCPxM 2 / 8 / 9 / 10 / 11.
    pub fn configure_pic_compare(&mut self, ccpxm: u8, ports: &mut [Port]) {
        let mut ctrl = true;
        let mut spec = false;
        self.set_oc_acts(OcAct::Non, OcAct::Non);
        match ccpxm {
            2 => self.set_oc_acts(OcAct::Tog, OcAct::Non),
            8 => self.set_oc_acts(OcAct::Set, OcAct::Clr),
            9 => self.set_oc_acts(OcAct::Clr, OcAct::Set),
            10 => ctrl = false,
            11 => {
                ctrl = false;
                spec = true;
            }
            _ => {}
        }
        if let OcKind::PicCompare { spec_event, .. } = &mut self.kind {
            *spec_event = spec;
        }
        port::control_gpio(ports, &self.pin, ctrl, false);
        self.ctrl_pin = ctrl;
        if !self.enabled {
            port::set_gpio_out(ports, &self.pin, false);
        }
        self.enabled = true;
    }

    /// C++ `PicPwmUnit::configure` — clear on match, set on TOV, 2 LSBs from CCPxCON.
    pub fn configure_pic_pwm(&mut self, ccpxcon: u8, ports: &mut [Port]) {
        if let OcKind::PicPwm { c_low, dc_bits, .. } = &mut self.kind {
            *c_low = dc_bits.val(ccpxcon);
        }
        self.set_oc_acts(OcAct::Clr, OcAct::Set);
        port::control_gpio(ports, &self.pin, true, false);
        port::set_gpio_out(ports, &self.pin, false);
        self.enabled = true;
        self.ctrl_pin = true;
    }

    /// C++ `AvrOcUnit::configure` — COMNX0,COMNX1.
    pub fn configure(&mut self, val: u8, ports: &mut [Port]) {
        if self.config_bits.mask == 0 {
            return;
        }
        self.mode = self.config_bits.val(val);
        let enabled = self.mode > 0;
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        port::control_gpio(ports, &self.pin, enabled, false);
        if enabled {
            port::set_gpio_out(ports, &self.pin, false);
        }
        self.ctrl_pin = enabled;
    }

    pub fn set_oc_acts(&mut self, com: OcAct, tov: OcAct) {
        self.com_act = com;
        self.tov_act = tov;
    }

    pub fn set_ocr_mask(&mut self, mask: u16, data: &DataSpace) {
        self.ocr_mask = mask;
        let mut m = u16::from(self.ocr_l.map(|a| data.get(a)).unwrap_or(0));
        if let Some(h) = self.ocr_h {
            m |= u16::from(data.get(h)) << 8;
        }
        self.com_match = m & self.ocr_mask;
    }

    pub fn ocr_write_l(&mut self, val: u8) {
        if let OcKind::PicPwm { ccprxl, .. } = &mut self.kind {
            *ccprxl = val;
        }
        self.com_match = (self.com_match & 0xFF00) | u16::from(val);
        self.com_match &= self.ocr_mask;
    }

    pub fn ocr_write_h(&mut self, val: u8) {
        self.com_match = (self.com_match & 0x00FF) | (u16::from(val) << 8);
        self.com_match &= self.ocr_mask;
    }

    /// C++ `McuOcUnit::sheduleEvents`. PIC PWM uses `rot=2` (8+2 bits).
    pub fn shedule_events(
        &mut self,
        ovf: u32,
        count_val: u32,
        reverse: bool,
        ext_clock: bool,
        ps_per_tick: u64,
        rot: i32,
    ) {
        let mut rot = rot;
        let pwm10 = match &self.kind {
            OcKind::PicPwm { ccprxl, c_low, .. } => Some((*ccprxl, *c_low)),
            _ => None,
        };
        if let Some((ccprxl, c_low)) = pwm10 {
            self.com_match = (u16::from(ccprxl) << 2) | u16::from(c_low);
            rot = 2;
        }
        let mut ovf = ovf;
        let mut count_val = count_val;
        if rot != 0 {
            ovf <<= rot;
            count_val <<= rot;
        }
        let (match_v, pin_set) = if reverse {
            (ovf.saturating_sub(u32::from(self.com_match)), false)
        } else {
            (u32::from(self.com_match), true)
        };
        self.pin_set = pin_set;
        if ext_clock {
            self.ext_match = match_v as u16;
            self.remain_ps = None;
            return;
        }
        if match_v <= ovf && match_v >= count_val {
            let mut next = (u64::from(match_v - count_val) * ps_per_tick.max(1)).max(1);
            if rot != 0 {
                next >>= rot;
            }
            self.remain_ps = Some(next.max(1));
        } else {
            self.remain_ps = None;
        }
    }

    pub fn clock_step(
        &mut self,
        count: u16,
        ports: &mut [Port],
        ints: &mut Interrupts,
        data: &mut DataSpace,
        ps_inst: u64,
    ) -> (Vec<IntCallback>, bool) {
        if count == self.ext_match {
            self.run_event(ports, ints, data, ps_inst)
        } else {
            (Vec::new(), false)
        }
    }

    /// C++ `McuOcUnit::runEvent` / `PicOcUnit::runEvent` / `PicPwmUnit::runEvent`.
    /// Second return is `PicOcUnit` special-event timer reset.
    pub fn run_event(
        &mut self,
        ports: &mut [Port],
        ints: &mut Interrupts,
        data: &mut DataSpace,
        ps_inst: u64,
    ) -> (Vec<IntCallback>, bool) {
        if matches!(self.kind, OcKind::PicCompare { .. }) {
            if !self.enabled {
                self.remain_ps = None;
                return (Vec::new(), false);
            }
            let (do_reset, spec_event, go_done) = match &self.kind {
                OcKind::PicCompare {
                    reset_timer,
                    spec_event,
                    go_done,
                } => (*reset_timer, *spec_event, *go_done),
                _ => unreachable!(),
            };
            if do_reset {
                if let OcKind::PicCompare { reset_timer, .. } = &mut self.kind {
                    *reset_timer = false;
                }
                self.remain_ps = None;
                return (Vec::new(), true);
            }
            if spec_event {
                if let OcKind::PicCompare { reset_timer, .. } = &mut self.kind {
                    *reset_timer = true;
                }
                self.remain_ps = Some(ps_inst.max(1));
                if go_done.mask != 0 {
                    let v = data.get(go_done.reg_addr) | go_done.mask;
                    data.set(go_done.reg_addr, v);
                }
            } else {
                self.remain_ps = None;
            }
            let cbs = if let Some(i) = self.interrupt {
                ints.raise(i, data)
            } else {
                Vec::new()
            };
            self.drive_pin(self.com_act, ports, ps_inst);
            return (cbs, false);
        }
        if matches!(self.kind, OcKind::PicPwm { .. }) && !self.enabled {
            self.remain_ps = None;
            return (Vec::new(), false);
        }
        self.remain_ps = None;
        let cbs = if let Some(i) = self.interrupt {
            ints.raise(i, data)
        } else {
            Vec::new()
        };
        if self.enabled {
            self.drive_pin(self.com_act, ports, ps_inst);
        }
        (cbs, false)
    }

    /// C++ `McuOcUnit::comMatch` (FOC).
    pub fn com_match(&mut self, ports: &mut [Port], ps_inst: u64) {
        self.drive_pin(self.com_act, ports, ps_inst);
    }

    /// C++ `McuOcUnit::tov`.
    pub fn tov(&mut self, ports: &mut [Port], ps_inst: u64) {
        self.drive_pin(self.tov_act, ports, ps_inst);
    }

    fn drive_pin(&mut self, act: OcAct, ports: &mut [Port], _ps_inst: u64) {
        if act == OcAct::Non {
            return;
        }
        let pin_state = match act {
            OcAct::Tog => !port::gpio_out(ports, &self.pin).unwrap_or(false),
            OcAct::Clr => !self.pin_set,
            OcAct::Set => self.pin_set,
            OcAct::Non => return,
        };
        self.set_pin_state(pin_state, ports);
    }

    fn set_pin_state(&mut self, state: bool, ports: &mut [Port]) {
        if self.ctrl_pin {
            port::set_gpio_out(ports, &self.pin, state);
            if let Some(inv) = &self.pin_inv {
                port::set_gpio_out(ports, inv, !state);
            }
        }
    }
}
