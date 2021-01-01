//! C++ `PicCcpUnit`: Capture / Compare / PWM attached to TIMER1 + TIMER2.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::CcpSpec;
use crate::icunit::IcUnit;
use crate::interrupts::Interrupts;
use crate::ocunit::OcUnit;
use crate::port::Port;
use crate::timer::Timer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CcpMode {
    Off,
    Cap,
    Com,
    Pwm,
}

#[derive(Clone, Debug)]
pub struct CcpUnit {
    #[allow(dead_code)]
    pub name: String,
    mode: u8,
    ccp_mode: CcpMode,
    ccpxm: RegBits,
    timer1: Option<usize>,
    timer2: Option<usize>,
    com_oc: Option<usize>,
    pwm_oc: Option<usize>,
}

impl CcpUnit {
    pub fn attach(
        spec: &CcpSpec,
        timers: &mut [Timer],
        data: &DataSpace,
        ints: &Interrupts,
    ) -> Self {
        let n = spec.name.chars().last().unwrap_or('1');
        let enhanced = spec.name.contains('+');
        let e = if enhanced { "+" } else { "" };
        let timer1 = timers.iter().position(|t| t.name == "TIMER1");
        let timer2 = timers.iter().position(|t| t.name == "TIMER2");
        let mut com_oc = None;
        let mut pwm_oc = None;
        if let Some(i) = timer1 {
            let oc = OcUnit::pic_compare(
                format!("OC{e}{n}"),
                spec.pin.clone(),
                ints,
                &spec.interrupt,
                data,
            );
            com_oc = Some(timers[i].add_oc_unit(oc));
            let ic = IcUnit::pic_ccp(
                format!("IC{n}"),
                spec.pin.clone(),
                &spec.ccpreg,
                &spec.interrupt,
                data,
                ints,
            );
            timers[i].set_ic_unit(ic);
        }
        if let Some(i) = timer2 {
            let oc = OcUnit::pic_pwm(
                format!("PWM{e}{n}"),
                spec.pin.clone(),
                ints,
                &spec.interrupt,
                spec.type_id,
                data,
            );
            pwm_oc = Some(timers[i].add_oc_unit(oc));
        }
        Self {
            name: spec.name.clone(),
            mode: 0,
            ccp_mode: CcpMode::Off,
            ccpxm: data.get_reg_bits(&format!("CCP{n}M0,CCP{n}M1,CCP{n}M2,CCP{n}M3")),
            timer1,
            timer2,
            com_oc,
            pwm_oc,
        }
    }

    pub fn initialize(&mut self, timers: &mut [Timer], ports: &mut [Port]) {
        self.mode = 0;
        self.ccp_mode = CcpMode::Off;
        self.init_units(timers, ports);
    }

    fn init_units(&mut self, timers: &mut [Timer], ports: &mut [Port]) {
        if let (Some(t), Some(oc)) = (self.timer1, self.com_oc) {
            if let Some(timer) = timers.get_mut(t) {
                if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.initialize(ports);
                }
                if let Some(ic) = &mut timer.ic_unit {
                    ic.initialize();
                }
            }
        }
        if let (Some(t), Some(oc)) = (self.timer2, self.pwm_oc) {
            if let Some(timer) = timers.get_mut(t) {
                if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.initialize(ports);
                }
            }
        }
    }

    pub fn is_pwm(&self) -> bool {
        self.ccp_mode == CcpMode::Pwm
    }

    /// C++ `PicCcpUnit::configureA`.
    pub fn configure_a(&mut self, ccpxcon: u8, timers: &mut [Timer], ports: &mut [Port]) {
        if ccpxcon == self.mode {
            return;
        }
        self.mode = ccpxcon;
        let ccpxm = if self.ccpxm.mask == 0 {
            ccpxcon & 0x0F
        } else {
            self.ccpxm.val(ccpxcon)
        };
        if ccpxm == 0 {
            self.ccp_mode = CcpMode::Off;
            self.init_units(timers, ports);
        } else if ccpxm < 4 {
            self.ccp_mode = CcpMode::Com;
            self.configure_compare(ccpxm, timers, ports);
        } else if ccpxm < 8 {
            self.ccp_mode = CcpMode::Cap;
            self.configure_capture(ccpxm, timers);
        } else if ccpxm < 12 {
            self.ccp_mode = CcpMode::Com;
            self.configure_compare(ccpxm, timers, ports);
        } else {
            self.ccp_mode = CcpMode::Pwm;
            self.configure_pwm(ccpxcon, timers, ports);
        }
    }

    fn configure_compare(&mut self, ccpxm: u8, timers: &mut [Timer], ports: &mut [Port]) {
        if let (Some(t), Some(oc)) = (self.timer1, self.com_oc) {
            if let Some(timer) = timers.get_mut(t) {
                if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.configure_pic_compare(ccpxm, ports);
                }
            }
        }
    }

    fn configure_capture(&mut self, ccpxm: u8, timers: &mut [Timer]) {
        if let Some(t) = self.timer1 {
            if let Some(timer) = timers.get_mut(t) {
                if let Some(ic) = &mut timer.ic_unit {
                    ic.configure_pic(ccpxm);
                }
            }
        }
    }

    fn configure_pwm(&mut self, ccpxcon: u8, timers: &mut [Timer], ports: &mut [Port]) {
        if let (Some(t), Some(oc)) = (self.timer2, self.pwm_oc) {
            if let Some(timer) = timers.get_mut(t) {
                if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.configure_pic_pwm(ccpxcon, ports);
                }
            }
        }
    }

    /// C++ `PicCcpUnit::ccprWriteL`.
    pub fn ccpr_write_l(
        &mut self,
        val: u8,
        timers: &mut [Timer],
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        if let (Some(t), Some(oc)) = (self.timer2, self.pwm_oc) {
            if let Some(timer) = timers.get_mut(t) {
                if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.ocr_write_l(val);
                }
                timer.oc_shedule(oc, data, ports, ps_inst);
            }
        }
        if let (Some(t), Some(oc)) = (self.timer1, self.com_oc) {
            if let Some(timer) = timers.get_mut(t) {
                let enabled = if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.ocr_write_l(val);
                    u.enabled
                } else {
                    false
                };
                if enabled {
                    timer.oc_shedule(oc, data, ports, ps_inst);
                }
            }
        }
    }

    /// C++ `PicCcpUnit::ccprWriteH` (PWM: read-only handled by the caller).
    pub fn ccpr_write_h(
        &mut self,
        val: u8,
        timers: &mut [Timer],
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        if self.ccp_mode == CcpMode::Pwm {
            return;
        }
        if let (Some(t), Some(oc)) = (self.timer1, self.com_oc) {
            if let Some(timer) = timers.get_mut(t) {
                let enabled = if let Some(u) = timer.oc_units.get_mut(oc) {
                    u.ocr_write_h(val);
                    u.enabled
                } else {
                    false
                };
                if enabled {
                    timer.oc_shedule(oc, data, ports, ps_inst);
                }
            }
        }
    }
}
