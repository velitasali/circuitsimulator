//! On-chip EEPROM SFR controller for AVR (`EECR`/`EEDR`/`EEAR`) and PIC (`EECON1`/`EECON2`/`EEDAT`/`EEADR`).

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, RomSpec};
use crate::interrupts::Interrupts;

#[derive(Clone, Debug)]
pub struct EepromCtrl {
    pub core: CoreKind,
    #[allow(dead_code)]
    pub name: String,
    ctrl_addr: Option<u16>,
    data_addr: Option<u16>,
    addr_l: Option<u16>,
    addr_h: Option<u16>,
    // AVR bits
    eere: RegBits,
    eepe: RegBits,
    eempe: RegBits,
    eepm0: RegBits,
    eepm1: RegBits,
    // PIC bits
    rd: RegBits,
    wr: RegBits,
    wren: RegBits,
    interrupt: Option<usize>,
    /// Remaining CPU cycles during which AVR `EEMPE` allows an `EEPE` write.
    avr_write_window: u8,
    /// PIC `EECON2` unlock sequence state: 0 = locked, 1 = saw 0x55, 2 = saw 0x55 then 0xAA.
    pic_unlock_stage: u8,
}

impl EepromCtrl {
    pub fn from_spec(spec: &RomSpec, data: &DataSpace, core: CoreKind, ints: &Interrupts) -> Self {
        let ctrl_name = spec
            .config
            .regs_a
            .split(',')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if core == CoreKind::Avr {
                "EECR"
            } else {
                "EECON1"
            });
        let data_name = if !spec.data_reg.is_empty() {
            spec.data_reg.as_str()
        } else if core == CoreKind::Avr {
            "EEDR"
        } else {
            "EEDAT"
        };
        let addr_l = spec
            .addr_regs
            .first()
            .and_then(|n| data.reg_addr(n))
            .or_else(|| {
                if core == CoreKind::Avr {
                    data.reg_addr("EEARL").or_else(|| data.reg_addr("EEAR"))
                } else {
                    data.reg_addr("EEADR")
                }
            });
        let addr_h = spec
            .addr_regs
            .get(1)
            .and_then(|n| data.reg_addr(n))
            .or_else(|| {
                if core == CoreKind::Avr {
                    data.reg_addr("EEARH")
                } else {
                    data.reg_addr("EEADRH")
                }
            });

        let eepe = first_bits(data, &["EEPE", "EEWE"]);
        let eempe = first_bits(data, &["EEMPE", "EEMWE"]);

        Self {
            core,
            name: spec.name.clone(),
            ctrl_addr: data.reg_addr(ctrl_name),
            data_addr: data.reg_addr(data_name),
            addr_l,
            addr_h,
            eere: data.get_reg_bits("EERE"),
            eepe,
            eempe,
            eepm0: data.get_reg_bits("EEPM0"),
            eepm1: data.get_reg_bits("EEPM1"),
            rd: data.get_reg_bits("RD"),
            wr: data.get_reg_bits("WR"),
            wren: data.get_reg_bits("WREN"),
            interrupt: ints.get(&spec.interrupt),
            avr_write_window: 0,
            pic_unlock_stage: 0,
        }
    }

    pub fn reset(&mut self) {
        self.avr_write_window = 0;
        self.pic_unlock_stage = 0;
    }

    /// Tick down AVR `EEMPE` 4-cycle write-enable window after each CPU instruction.
    pub fn step_cycles(&mut self, cycles: u32, data: &mut DataSpace) {
        if self.avr_write_window == 0 {
            return;
        }
        self.avr_write_window = self.avr_write_window.saturating_sub(cycles.min(255) as u8);
        if self.avr_write_window == 0 && self.eempe.mask != 0 {
            let r = data.get(self.eempe.reg_addr) & !self.eempe.mask;
            data.set(self.eempe.reg_addr, r);
        }
    }

    /// Handle write to `EECR` (AVR) or `EECON1` (PIC).
    pub fn write_ctrl(
        &mut self,
        val: u8,
        data: &mut DataSpace,
        eeprom: &mut [u8],
        ints: &mut Interrupts,
    ) {
        match self.core {
            CoreKind::Avr => self.write_avr_eecr(val, data, eeprom, ints),
            CoreKind::Pic12 | CoreKind::Pic14 => self.write_pic_eecon1(val, data, eeprom, ints),
            _ => {}
        }
    }

    /// Handle write to PIC `EECON2` (`0x55` -> `0xAA` write-unlock sequence).
    pub fn write_unlock(&mut self, val: u8) {
        if !matches!(self.core, CoreKind::Pic12 | CoreKind::Pic14) {
            return;
        }
        match val {
            0x55 => self.pic_unlock_stage = 1,
            0xAA if self.pic_unlock_stage == 1 => self.pic_unlock_stage = 2,
            _ => self.pic_unlock_stage = 0,
        }
    }

    fn current_addr(&self, data: &DataSpace) -> usize {
        let low = self.addr_l.map(|a| usize::from(data.get(a))).unwrap_or(0);
        let high = self.addr_h.map(|a| usize::from(data.get(a))).unwrap_or(0);
        low | (high << 8)
    }

    fn write_avr_eecr(
        &mut self,
        val: u8,
        data: &mut DataSpace,
        eeprom: &mut [u8],
        ints: &mut Interrupts,
    ) {
        let mut eecr = val;
        let addr = self.current_addr(data);

        // Read trigger (`EERE`)
        if self.eere.mask != 0 && (val & self.eere.mask) != 0 {
            let byte = eeprom.get(addr).copied().unwrap_or(0xFF);
            if let Some(da) = self.data_addr {
                data.set(da, byte);
            }
            eecr &= !self.eere.mask;
        }

        // Master write enable (`EEMPE` / `EEMWE`) opens a 4-cycle window.
        // Note: setting `EEMPE` and `EEPE` in the same write is ignored by hardware.
        let setting_eempe = self.eempe.mask != 0 && (val & self.eempe.mask) != 0;
        let setting_eepe = self.eepe.mask != 0 && (val & self.eepe.mask) != 0;
        if setting_eepe {
            if self.avr_write_window > 0 {
                if let Some(slot) = eeprom.get_mut(addr) {
                    let eedr = self.data_addr.map(|a| data.get(a)).unwrap_or(0xFF);
                    let m0 = self.eepm0.mask != 0 && (val & self.eepm0.mask) != 0;
                    let m1 = self.eepm1.mask != 0 && (val & self.eepm1.mask) != 0;
                    match (m1, m0) {
                        (false, false) => *slot = eedr, // Erase + Write
                        (false, true) => *slot = 0xFF,  // Erase only
                        (true, false) => *slot &= eedr, // Write only
                        (true, true) => *slot = eedr,   // Reserved -> standard write
                    }
                }
                self.avr_write_window = 0;
                eecr &= !self.eempe.mask;
                if let Some(int_idx) = self.interrupt {
                    ints.raise(int_idx, data);
                }
            }
            // Write completes immediately in simulation; clear EEPE.
            eecr &= !self.eepe.mask;
        } else if setting_eempe {
            // 4 cycles after this instruction (+1 so the current instruction's decrement leaves 4).
            self.avr_write_window = 5;
        }

        if let Some(ca) = self.ctrl_addr {
            data.set(ca, eecr);
        }
    }

    fn write_pic_eecon1(
        &mut self,
        val: u8,
        data: &mut DataSpace,
        eeprom: &mut [u8],
        ints: &mut Interrupts,
    ) {
        let mut eecon1 = val;
        let addr = self.current_addr(data);

        if self.rd.mask != 0 && (val & self.rd.mask) != 0 {
            let byte = eeprom.get(addr).copied().unwrap_or(0xFF);
            if let Some(da) = self.data_addr {
                data.set(da, byte);
            }
            eecon1 &= !self.rd.mask;
        }

        if self.wr.mask != 0 && (val & self.wr.mask) != 0 {
            let wren_on = self.wren.mask == 0 || (val & self.wren.mask) != 0;
            if wren_on && self.pic_unlock_stage == 2 {
                if let Some(slot) = eeprom.get_mut(addr) {
                    *slot = self.data_addr.map(|a| data.get(a)).unwrap_or(0xFF);
                }
                if let Some(int_idx) = self.interrupt {
                    ints.raise(int_idx, data);
                }
            }
            self.pic_unlock_stage = 0;
            eecon1 &= !self.wr.mask;
        }

        if let Some(ca) = self.ctrl_addr {
            data.set(ca, eecon1);
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

#[cfg(test)]
mod tests {
    use crate::Device;

    const AVR_EEPROM_XML: &str = r#"
<mcu name="mega328ee" core="AVR" data="512" prog="256" progword="2" eeprom="64" inst_cycle="1" freq="16000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="EECR" addr="0x003F" reset="" bits="EERE,EEPE,EEMPE,EERIE,EEPM0,EEPM1,0,0"/>
    <register name="EEDR" addr="0x0040" reset=""/>
    <register name="EEARL" addr="0x0041" reset=""/>
    <register name="EEARH" addr="0x0042" reset=""/>
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <interrupts enable="I">
    <interrupt name="EEPR" enable="EERIE" flag="" priority="4" vector="0x002C"/>
  </interrupts>
  <rom name="ROM" configregsA="EECR" dataregs="EEDR" addressreg="EEARL,EEARH" interrupt="EEPR"/>
</mcu>
"#;

    const PIC_EEPROM_XML: &str = r#"
<mcu name="pic16ee" core="Pic14" data="512" prog="256" progword="2" eeprom="64" inst_cycle="4" freq="4000000">
  <regblock start="0x0000" end="0x01FF" streg="STATUS">
    <register name="STATUS" addr="0x0003" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="EEDAT" addr="0x010C" reset=""/>
    <register name="EEADR" addr="0x010D" reset=""/>
    <register name="EECON1" addr="0x018C" reset="" bits="RD,WR,WREN,WRERR,0,0,0,EEPGD"/>
    <register name="EECON2" addr="0x018D" reset=""/>
  </regblock>
  <rom name="ROM" configregsA="EECON1" configregsB="EECON2" dataregs="EEDAT" addressreg="EEADR"/>
</mcu>
"#;

    #[test]
    fn avr_eeprom_write_requires_eempe_and_reads_back() {
        let mut dev = Device::from_xml("m", AVR_EEPROM_XML).unwrap();
        // Set address 0x05, data 0x42
        dev.poke_reg(0x0041, 0x05);
        dev.poke_reg(0x0042, 0x00);
        dev.poke_reg(0x0040, 0x42);

        // Attempt EEPE without EEMPE -> ignored
        dev.poke_reg(0x003F, 0x02);
        assert_eq!(dev.eeprom()[5], 0xFF);
        assert_eq!(dev.ram(0x003F) & 0x02, 0);

        // Arm EEMPE (0x04), then trigger EEPE (0x02)
        dev.poke_reg(0x003F, 0x04);
        dev.poke_reg(0x003F, 0x02);
        assert_eq!(dev.eeprom()[5], 0x42);

        // Clear EEDR, then trigger EERE (0x01)
        dev.poke_reg(0x0040, 0x00);
        dev.poke_reg(0x003F, 0x01);
        assert_eq!(dev.ram(0x0040), 0x42);
        assert_eq!(dev.ram(0x003F) & 0x01, 0);
    }

    #[test]
    fn pic_eeprom_unlock_sequence_write_and_read() {
        let mut dev = Device::from_xml("p", PIC_EEPROM_XML).unwrap();
        dev.poke_reg(0x010D, 0x0A); // EEADR = 10
        dev.poke_reg(0x010C, 0x7E); // EEDAT = 0x7E

        // WR without 0x55/0xAA unlock -> ignored
        dev.poke_reg(0x018C, 0x06); // WREN | WR
        assert_eq!(dev.eeprom()[10], 0xFF);

        // Proper 0x55 -> 0xAA unlock sequence
        dev.poke_reg(0x018D, 0x55);
        dev.poke_reg(0x018D, 0xAA);
        dev.poke_reg(0x018C, 0x06); // WREN | WR
        assert_eq!(dev.eeprom()[10], 0x7E);
        assert_eq!(dev.ram(0x018C) & 0x02, 0);

        // Read back via RD (0x01)
        dev.poke_reg(0x010C, 0x00);
        dev.poke_reg(0x018C, 0x01);
        assert_eq!(dev.ram(0x010C), 0x7E);
        assert_eq!(dev.ram(0x018C) & 0x01, 0);
    }
}
