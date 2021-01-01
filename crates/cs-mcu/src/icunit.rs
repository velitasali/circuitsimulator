//! C++ `McuIcUnit` / `AvrIcUnit`: input capture on a pin edge.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::IcUnitSpec;
use crate::interrupts::{IntCallback, Interrupts};
use crate::port::{self, Port};

#[derive(Clone, Debug)]
pub struct IcUnit {
    #[allow(dead_code)]
    pub name: String,
    pub pin: String,
    ic_reg_l: Option<u16>,
    ic_reg_h: Option<u16>,
    pub interrupt: Option<usize>,
    config_bits: RegBits,
    pub enabled: bool,
    in_state: bool,
    falling_edge: bool,
    prescaler: u64,
    counter: u64,
}

impl IcUnit {
    pub fn from_spec(spec: &IcUnitSpec, data: &DataSpace, ints: &Interrupts) -> Self {
        Self {
            name: spec.name.clone(),
            pin: spec.pin.clone(),
            ic_reg_l: spec.icreg.first().and_then(|n| data.reg_addr(n)),
            ic_reg_h: spec.icreg.get(1).and_then(|n| data.reg_addr(n)),
            interrupt: ints.get(&spec.interrupt),
            config_bits: if spec.bits.is_empty() {
                RegBits::default()
            } else {
                data.get_reg_bits(&spec.bits)
            },
            enabled: false,
            in_state: false,
            falling_edge: false,
            prescaler: 1,
            counter: 0,
        }
    }

    pub fn initialize(&mut self) {
        self.prescaler = 1;
        self.clear();
    }

    pub fn clear(&mut self) {
        self.counter = 0;
        self.enabled = false;
        self.in_state = false;
        self.falling_edge = false;
    }

    pub fn enable(&mut self, en: bool) {
        self.enabled = en;
    }

    /// C++ `AvrIcUnit::configure` — ICES, ICNC. ICES=0 falling, ICES=1 rising.
    pub fn configure(&mut self, val: u8) {
        if self.config_bits.mask == 0 {
            return;
        }
        let bits = self.config_bits.val(val);
        self.falling_edge = bits & 1 == 0;
    }

    /// C++ `PicIcUnit::configure` — CCPxM 4 falling, 5 rising, 6/7 rising with presc 4/16.
    pub fn configure_pic(&mut self, ccpxm: u8) {
        self.enabled = true;
        self.falling_edge = false;
        self.prescaler = 1;
        self.counter = 0;
        match ccpxm {
            4 => self.falling_edge = true,
            6 => self.prescaler = 4,
            7 => self.prescaler = 16,
            _ => {}
        }
    }

    /// C++ `PicCcpUnit` capture: CCP pin + CCPRxL/H.
    pub fn pic_ccp(
        name: impl Into<String>,
        pin: impl Into<String>,
        icreg: &[String],
        interrupt: &str,
        data: &DataSpace,
        ints: &Interrupts,
    ) -> Self {
        Self {
            name: name.into(),
            pin: pin.into(),
            ic_reg_l: icreg.first().and_then(|n| data.reg_addr(n)),
            ic_reg_h: icreg.get(1).and_then(|n| data.reg_addr(n)),
            interrupt: ints.get(interrupt),
            config_bits: RegBits::default(),
            enabled: false,
            in_state: false,
            falling_edge: false,
            prescaler: 1,
            counter: 0,
        }
    }

    /// C++ `McuIcUnit::voltChanged`.
    pub fn pin_changed(
        &mut self,
        ports: &[Port],
        count: u32,
        data: &mut DataSpace,
        ints: &mut Interrupts,
    ) -> Vec<IntCallback> {
        if !self.enabled {
            return Vec::new();
        }
        let Some(state) = port::gpio_inp(ports, &self.pin) else {
            return Vec::new();
        };
        if self.in_state == state {
            return Vec::new();
        }
        self.in_state = state;
        if state == self.falling_edge {
            return Vec::new();
        }
        self.counter += 1;
        if self.counter < self.prescaler {
            return Vec::new();
        }
        self.counter = 0;
        if let Some(a) = self.ic_reg_l {
            data.set(a, count as u8);
        }
        if let Some(a) = self.ic_reg_h {
            data.set(a, (count >> 8) as u8);
        }
        if let Some(i) = self.interrupt {
            ints.raise(i, data)
        } else {
            Vec::new()
        }
    }
}
