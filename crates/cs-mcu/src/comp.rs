//! Analog Comparator peripheral for AVR (`ACSR`) and PIC (`CMCON0`).

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CompSpec, CoreKind};
use crate::interrupts::Interrupts;
use crate::port::{self, Port};

#[derive(Clone, Debug)]
pub struct Comparator {
    pub core: CoreKind,
    #[allow(dead_code)]
    pub name: String,
    ctrl_addr: Option<u16>,
    // AVR ACSR & ADC mux fallback
    acd: RegBits,
    acbg: RegBits,
    aco: RegBits,
    aci: RegBits,
    acis0: RegBits,
    acis1: RegBits,
    acme: RegBits,
    aden: RegBits,
    admux_addr: Option<u16>,
    adc_channels: Vec<Option<(usize, usize)>>,
    // PIC CMCON0 bits
    cout: RegBits,
    cinv: RegBits,
    interrupt: Option<usize>,
    pin_pos: Option<(usize, usize)>,
    pin_neg: Option<(usize, usize)>,
    last_out: bool,
}

impl Comparator {
    pub fn from_spec(
        spec: &CompSpec,
        data: &DataSpace,
        ports: &[Port],
        adc_pin_names: &[String],
        core: CoreKind,
        ints: &Interrupts,
    ) -> Self {
        let ctrl_name = spec
            .config
            .regs_a
            .split(',')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if core == CoreKind::Avr {
                "ACSR"
            } else {
                "CMCON0"
            });

        let pin_pos = spec.pins.first().and_then(|n| port::find_gpio(ports, n));
        let pin_neg = spec.pins.get(1).and_then(|n| port::find_gpio(ports, n));
        let adc_channels = adc_pin_names
            .iter()
            .map(|n| port::find_gpio(ports, n))
            .collect();

        let cout = if spec.type_id == 2 {
            first_bits(data, &["C2OUT", "COUT"])
        } else {
            first_bits(data, &["C1OUT", "COUT"])
        };
        let cinv = if spec.type_id == 2 {
            first_bits(data, &["C2INV", "CINV"])
        } else {
            first_bits(data, &["C1INV", "CINV"])
        };

        Self {
            core,
            name: spec.name.clone(),
            ctrl_addr: data.reg_addr(ctrl_name),
            acd: data.get_reg_bits("ACD"),
            acbg: data.get_reg_bits("ACBG"),
            aco: data.get_reg_bits("ACO"),
            aci: data.get_reg_bits("ACI"),
            acis0: data.get_reg_bits("ACIS0"),
            acis1: data.get_reg_bits("ACIS1"),
            acme: data.get_reg_bits("ACME"),
            aden: data.get_reg_bits("ADEN"),
            admux_addr: data.reg_addr("ADMUX"),
            adc_channels,
            cout,
            cinv,
            interrupt: ints.get(&spec.interrupt),
            pin_pos,
            pin_neg,
            last_out: false,
        }
    }

    pub fn reset(&mut self) {
        self.last_out = false;
    }

    /// Re-evaluate comparator output against current pin analog voltages.
    pub fn evaluate(&mut self, data: &mut DataSpace, ports: &[Port], ints: &mut Interrupts) {
        match self.core {
            CoreKind::Avr => self.eval_avr(data, ports, ints),
            CoreKind::Pic12 | CoreKind::Pic14 => self.eval_pic(data, ports, ints),
            _ => {}
        }
    }

    fn eval_avr(&mut self, data: &mut DataSpace, ports: &[Port], ints: &mut Interrupts) {
        let Some(addr) = self.ctrl_addr else {
            return;
        };
        let mut acsr = data.get(addr);
        if self.acd.mask != 0 && (acsr & self.acd.mask) != 0 {
            if self.aco.mask != 0 && (acsr & self.aco.mask) != 0 {
                acsr &= !self.aco.mask;
                data.set(addr, acsr);
            }
            self.last_out = false;
            return;
        }

        let v_pos = if self.acbg.mask != 0 && (acsr & self.acbg.mask) != 0 {
            1.1
        } else {
            pin_volt(ports, self.pin_pos).unwrap_or(0.0)
        };

        let use_adc_mux = self.acme.mask != 0
            && (data.get(self.acme.reg_addr) & self.acme.mask) != 0
            && (self.aden.mask == 0 || (data.get(self.aden.reg_addr) & self.aden.mask) == 0);
        let v_neg = if use_adc_mux {
            let ch = self
                .admux_addr
                .map(|a| usize::from(data.get(a) & 0x0F))
                .unwrap_or(0);
            self.adc_channels
                .get(ch)
                .copied()
                .flatten()
                .and_then(|loc| pin_volt(ports, Some(loc)))
                .unwrap_or(0.0)
        } else {
            pin_volt(ports, self.pin_neg).unwrap_or(0.0)
        };

        let out = v_pos > v_neg;
        if self.aco.mask != 0 {
            if out {
                acsr |= self.aco.mask;
            } else {
                acsr &= !self.aco.mask;
            }
            data.set(addr, acsr);
        }

        let prev = self.last_out;
        self.last_out = out;
        if out != prev {
            let is0 = self.acis0.mask != 0 && (acsr & self.acis0.mask) != 0;
            let is1 = self.acis1.mask != 0 && (acsr & self.acis1.mask) != 0;
            let fire = match (is1, is0) {
                (false, false) => true, // Toggle
                (true, false) => !out,  // Falling edge
                (true, true) => out,    // Rising edge
                (false, true) => false, // Reserved
            };
            if fire {
                if self.aci.mask != 0 {
                    let r = data.get(self.aci.reg_addr) | self.aci.mask;
                    data.set(self.aci.reg_addr, r);
                }
                if let Some(int_idx) = self.interrupt {
                    ints.raise(int_idx, data);
                }
            }
        }
    }

    fn eval_pic(&mut self, data: &mut DataSpace, ports: &[Port], ints: &mut Interrupts) {
        let Some(addr) = self.ctrl_addr else {
            return;
        };
        let mut cmcon = data.get(addr);
        // On PIC16F88/628/684, CM2:0 == 0b111 or 0b000 disables comparators.
        let mode = cmcon & 0x07;
        if mode == 0 || mode == 7 {
            self.last_out = false;
            return;
        }
        let v_pos = pin_volt(ports, self.pin_pos).unwrap_or(0.0);
        let v_neg = pin_volt(ports, self.pin_neg).unwrap_or(0.0);
        let mut out = v_pos > v_neg;
        if self.cinv.mask != 0 && (cmcon & self.cinv.mask) != 0 {
            out = !out;
        }
        if self.cout.mask != 0 {
            if out {
                cmcon |= self.cout.mask;
            } else {
                cmcon &= !self.cout.mask;
            }
            data.set(addr, cmcon);
        }
        if out != self.last_out {
            self.last_out = out;
            if let Some(int_idx) = self.interrupt {
                ints.raise(int_idx, data);
            }
        }
    }
}

fn first_bits(data: &DataSpace, names: &[&str]) -> RegBits {
    for name in names {
        let b = data.get_reg_bits(name);
        if b.mask != 0 {
            return b;
        }
    }
    RegBits::default()
}

fn pin_volt(ports: &[Port], loc: Option<(usize, usize)>) -> Option<f64> {
    let (pi, idx) = loc?;
    Some(ports.get(pi)?.pins.get(idx)?.inp_volt)
}

#[cfg(test)]
mod tests {
    use crate::Device;

    const AVR_COMP_XML: &str = r#"
<mcu name="mega328comp" core="AVR" data="512" prog="256" progword="2" inst_cycle="1" freq="16000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="PIND" addr="0x0029" reset="" mask="11111111"/>
    <register name="DDRD" addr="0x002A" reset="" mask="11111111"/>
    <register name="PORTD" addr="0x002B" reset="" mask="11111111"/>
    <register name="ACSR" addr="0x0050" reset="" mask="11011111" bits="ACIS0,ACIS1,ACIC,ACIE,ACI,ACO,ACBG,ACD"/>
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <port name="PORTD" pins="8" outreg="PORTD" inreg="PIND" dirreg="DDRD"/>
  <interrupts enable="I">
    <interrupt name="ACOMP" enable="ACIE" flag="ACI" priority="3" vector="0x002E" clear="1"/>
  </interrupts>
  <comp name="COMP" configregsA="ACSR" interrupt="ACOMP" pins="PORTD6,PORTD7"/>
</mcu>
"#;

    #[test]
    fn avr_comparator_tracks_analog_crossing_and_raises_irq() {
        let mut dev = Device::from_xml("m", AVR_COMP_XML).unwrap();
        // Rising-edge trigger: ACIS1=1, ACIS0=1 (0x03)
        dev.poke_reg(0x0050, 0x03);
        dev.set_pin_volt("PORTD7", 2.0); // AIN1 (V-) set first so V+ stays < V-
        dev.set_pin_volt("PORTD6", 1.0); // AIN0 (V+)
        assert_eq!(dev.ram(0x0050) & 0x20, 0, "ACO should be 0 when V+ < V-");
        assert_eq!(dev.ram(0x0050) & 0x10, 0, "ACI should not trigger yet");

        // Cross above AIN1 -> ACO flips 1 and ACI rises
        dev.set_pin_volt("PORTD6", 3.0);
        assert_ne!(dev.ram(0x0050) & 0x20, 0, "ACO should be 1 when V+ > V-");
        assert_ne!(dev.ram(0x0050) & 0x10, 0, "ACI should set on rising edge");

        // Internal 1.1V bandgap (ACBG = 0x40) with AIN1 = 2.0V -> ACO goes low
        dev.poke_reg(0x0050, 0x43);
        assert_eq!(dev.ram(0x0050) & 0x20, 0, "ACO should be 0 for 1.1V < 2.0V");
    }
}
