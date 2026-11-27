//! Watchdog Timer peripheral for AVR (`WDTCSR`/`WDTCR`) and PIC (`OPTION`/`WDTCON`).

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, WdtSpec};
use crate::interrupts::Interrupts;

/// Action requested by the watchdog timer when its timeout expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WdtAction {
    None,
    Reset,
}

#[derive(Clone, Debug)]
pub struct Wdt {
    pub core: CoreKind,
    #[allow(dead_code)]
    pub name: String,
    ctrl_addr: Option<u16>,
    // AVR bits
    wde: RegBits,
    wdce: RegBits,
    wdie: RegBits,
    wdif: RegBits,
    wdp0: RegBits,
    wdp1: RegBits,
    wdp2: RegBits,
    wdp3: RegBits,
    // PIC bits
    swdten: RegBits,
    psa: RegBits,
    ps_bits: RegBits,
    interrupt: Option<usize>,
    // Runtime state
    pub forced_enabled: bool,
    pub wde_enabled: bool,
    pub wdie_enabled: bool,
    period_ps: u64,
    pub remain_ps: Option<u64>,
    change_window: u8,
}

impl Wdt {
    pub fn from_spec(spec: &WdtSpec, data: &DataSpace, core: CoreKind, ints: &Interrupts) -> Self {
        let ctrl_name = spec
            .config
            .regs_a
            .split(',')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if core == CoreKind::Avr {
                "WDTCSR"
            } else {
                "WDTCON"
            });

        Self {
            core,
            name: spec.name.clone(),
            ctrl_addr: data.reg_addr(ctrl_name),
            wde: data.get_reg_bits("WDE"),
            wdce: data.get_reg_bits("WDCE"),
            wdie: data.get_reg_bits("WDIE"),
            wdif: data.get_reg_bits("WDIF"),
            wdp0: data.get_reg_bits("WDP0"),
            wdp1: data.get_reg_bits("WDP1"),
            wdp2: data.get_reg_bits("WDP2"),
            wdp3: data.get_reg_bits("WDP3"),
            swdten: data.get_reg_bits("SWDTEN"),
            psa: data.get_reg_bits("PSA"),
            ps_bits: data.get_reg_bits("PS0,PS1,PS2"),
            interrupt: ints.get(&spec.interrupt),
            forced_enabled: false,
            wde_enabled: false,
            wdie_enabled: false,
            period_ps: 16_000_000_000, // 16 ms default
            remain_ps: None,
            change_window: 0,
        }
    }

    pub fn reset(&mut self) {
        self.change_window = 0;
        self.wde_enabled = self.forced_enabled;
        self.wdie_enabled = false;
        self.period_ps = if self.core == CoreKind::Avr {
            16_000_000_000
        } else {
            18_000_000_000
        };
        self.remain_ps = if self.is_active() {
            Some(self.period_ps)
        } else {
            None
        };
    }

    pub fn set_forced_enabled(&mut self, enabled: bool) {
        self.forced_enabled = enabled;
        if enabled {
            self.wde_enabled = true;
            if self.remain_ps.is_none() {
                self.remain_ps = Some(self.period_ps);
            }
        } else if !self.wdie_enabled && !self.wde_enabled {
            self.remain_ps = None;
        }
    }

    pub fn is_active(&self) -> bool {
        self.forced_enabled || self.wde_enabled || self.wdie_enabled
    }

    /// Called when the CPU executes `WDR` (AVR) or `CLRWDT` (PIC).
    pub fn kick(&mut self) {
        if self.is_active() {
            self.remain_ps = Some(self.period_ps);
        }
    }

    /// Tick down the AVR 4-cycle `WDCE` change-enable window.
    pub fn step_cycles(&mut self, cycles: u32, data: &mut DataSpace) {
        if self.change_window == 0 {
            return;
        }
        self.change_window = self.change_window.saturating_sub(cycles.min(255) as u8);
        if self.change_window == 0 && self.wdce.mask != 0 {
            let r = data.get(self.wdce.reg_addr) & !self.wdce.mask;
            data.set(self.wdce.reg_addr, r);
        }
    }

    pub fn configure(&mut self, val: u8, data: &mut DataSpace) {
        match self.core {
            CoreKind::Avr => self.configure_avr(val, data),
            CoreKind::Pic12 | CoreKind::Pic14 => self.configure_pic(val),
            _ => {}
        }
    }

    fn configure_avr(&mut self, val: u8, data: &mut DataSpace) {
        let wdce_set = self.wdce.mask != 0 && (val & self.wdce.mask) != 0;
        let wde_set = self.wde.mask != 0 && (val & self.wde.mask) != 0;

        // AVR datasheet: writing WDCE=1 & WDE=1 opens a 4-cycle window to change WDE/WDP.
        if wdce_set && wde_set && self.change_window == 0 {
            self.change_window = 5;
            return;
        }

        let can_change_prescaler = self.wdce.mask == 0 || self.change_window > 0 || wdce_set;
        if can_change_prescaler {
            self.wde_enabled = self.forced_enabled || wde_set;
            let mut wdp = 0u32;
            if self.wdp0.mask != 0 && (val & self.wdp0.mask) != 0 {
                wdp |= 1;
            }
            if self.wdp1.mask != 0 && (val & self.wdp1.mask) != 0 {
                wdp |= 2;
            }
            if self.wdp2.mask != 0 && (val & self.wdp2.mask) != 0 {
                wdp |= 4;
            }
            if self.wdp3.mask != 0 && (val & self.wdp3.mask) != 0 {
                wdp |= 8;
            }
            // 128 kHz WDT oscillator: WDP=0 -> 2048 cycles (~16 ms), doubling up to WDP=9 (~8.192 s).
            let shift = wdp.min(9);
            self.period_ps = 16_000_000_000u64 << shift;
            self.change_window = 0;
            if self.wdce.mask != 0 {
                let r = data.get(self.wdce.reg_addr) & !self.wdce.mask;
                data.set(self.wdce.reg_addr, r);
            }
        }

        self.wdie_enabled = self.wdie.mask != 0 && (val & self.wdie.mask) != 0;

        if self.is_active() {
            self.remain_ps = Some(self.period_ps);
        } else {
            self.remain_ps = None;
        }
    }

    fn configure_pic(&mut self, val: u8) {
        if self.swdten.mask != 0 {
            self.wde_enabled = self.forced_enabled || (val & self.swdten.mask) != 0;
        }
        let psa_wdt = self.psa.mask != 0 && (val & self.psa.mask) != 0;
        let ps = if psa_wdt && self.ps_bits.mask != 0 {
            1u64 << u32::from(self.ps_bits.val(val))
        } else {
            1
        };
        self.period_ps = 18_000_000_000u64.saturating_mul(ps);
        if self.is_active() {
            self.remain_ps = Some(self.period_ps);
        } else {
            self.remain_ps = None;
        }
    }

    pub fn advance(
        &mut self,
        elapsed: u64,
        data: &mut DataSpace,
        ints: &mut Interrupts,
    ) -> WdtAction {
        let Some(rem) = self.remain_ps else {
            return WdtAction::None;
        };
        if elapsed < rem {
            self.remain_ps = Some(rem - elapsed);
            return WdtAction::None;
        }
        if !self.is_active() {
            self.remain_ps = None;
            return WdtAction::None;
        }

        if self.core == CoreKind::Avr && self.wdie_enabled {
            // Interrupt mode (or Interrupt + System Reset mode: first timeout fires IRQ and clears WDIE).
            if self.wde_enabled {
                self.wdie_enabled = false;
                if let Some(ca) = self.ctrl_addr {
                    let r = data.get(ca) & !self.wdie.mask;
                    data.set(ca, r);
                }
            }
            if self.wdif.mask != 0 {
                let r = data.get(self.wdif.reg_addr) | self.wdif.mask;
                data.set(self.wdif.reg_addr, r);
            }
            if let Some(int_idx) = self.interrupt {
                ints.raise(int_idx, data);
            }
            self.remain_ps = Some(self.period_ps);
            return WdtAction::None;
        }

        if self.wde_enabled || self.forced_enabled {
            self.remain_ps = Some(self.period_ps);
            return WdtAction::Reset;
        }

        self.remain_ps = None;
        WdtAction::None
    }
}

#[cfg(test)]
mod tests {
    use crate::Device;

    const AVR_WDT_XML: &str = r#"
<mcu name="mega328wdt" core="AVR" data="512" prog="256" progword="2" inst_cycle="1" freq="16000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
    <register name="WDTCSR" addr="0x0060" reset="" bits="WDP0,WDP1,WDP2,WDE,WDCE,WDP3,WDIE,WDIF"/>
  </regblock>
  <interrupts enable="I">
    <interrupt name="WDT" enable="WDIE" flag="WDIF" priority="20" vector="0x000C"/>
  </interrupts>
  <wdt name="WDT00" configregsA="WDTCSR" interrupt="WDT"
       prescalers="512,256,128,64,32,16,8,4,2,1,1,1,1,1,1,1"/>
</mcu>
"#;

    #[test]
    fn avr_wdt_timeout_fires_interrupt_and_reset() {
        let mut dev = Device::from_xml("m", AVR_WDT_XML).unwrap();
        // Put `rjmp .-2` at PC=0x0004 and `nop` at 0x000C (WDT vector)
        dev.set_flash(0x0000, 0xC003); // rjmp +3 -> PC=0x0004
        dev.set_flash(0x0004, 0xCFFF); // rjmp .-2 (spin)
        dev.set_flash(0x000C, 0x0000); // nop at WDT vector
        dev.start();
        // Enable global interrupts (`I` = 0x80)
        dev.poke_reg(0x005F, 0x80);
        // Timed sequence: WDCE | WDE, then WDIE | WDP=0 (16 ms interrupt mode)
        dev.poke_reg(0x0060, 0x18);
        dev.poke_reg(0x0060, 0x40);

        // Step past 16 ms using advance() (fast-forwards through spin loop)
        let mut hit_wdt_vec = false;
        for _ in 0..50 {
            dev.advance();
            if dev.pc() == 0x000C {
                hit_wdt_vec = true;
                break;
            }
        }
        assert!(
            hit_wdt_vec,
            "expected WDT interrupt at 0x000C, pc=0x{:x}",
            dev.pc()
        );
    }

    #[test]
    fn avr_wdt_system_reset_returns_pc_to_zero() {
        let mut dev = Device::from_xml("m", AVR_WDT_XML).unwrap();
        dev.set_flash(0x0000, 0xC003); // rjmp -> 0x0004
        dev.set_flash(0x0004, 0xCFFF); // spin at 0x0004
        dev.start();
        dev.advance(); // execute rjmp -> pc=0x0004
        assert_eq!(dev.pc(), 0x0004);

        // Enable WDE=1, WDP=0 (16 ms reset mode)
        dev.poke_reg(0x0060, 0x18);
        dev.poke_reg(0x0060, 0x08);

        let mut reset_seen = false;
        for _ in 0..50 {
            dev.advance();
            if dev.pc() == 0x0000 {
                reset_seen = true;
                break;
            }
        }
        assert!(reset_seen, "WDT system reset should reset PC to 0");
    }
}
