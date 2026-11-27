//! Hardware SPI peripheral for AVR (`SPCR`/`SPSR`/`SPDR`) and PIC MSSP (`SSPCON`/`SSPSTAT`/`SSPBUF`).

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, SpiSpec};
use crate::interrupts::Interrupts;
use crate::port::{self, Port};

#[derive(Clone, Debug)]
pub struct Spi {
    pub core: CoreKind,
    #[allow(dead_code)]
    pub name: String,
    mosi: Option<String>,
    miso: Option<String>,
    sck: Option<String>,
    ss: Option<String>,
    spcr_addr: Option<u16>,
    spsr_addr: Option<u16>,
    spdr_addr: Option<u16>,
    // AVR control/status bits
    spe: RegBits,
    mstr: RegBits,
    dord: RegBits,
    cpol_bit: RegBits,
    cpha_bit: RegBits,
    spr0: RegBits,
    spr1: RegBits,
    spi2x: RegBits,
    wcol: RegBits,
    spif: RegBits,
    // PIC MSSP bits
    sspen: RegBits,
    ckp: RegBits,
    sspm_mask: u8,
    sspm_shift: u8,
    cke: RegBits,
    bf: RegBits,
    interrupt: Option<usize>,
    prescalers: Vec<u32>,
    // Runtime state
    pub enabled: bool,
    pub master: bool,
    lsb_first: bool,
    cpol: bool,
    cpha: bool,
    prescaler: u32,
    half_period_ps: u64,
    pub remain_ps: Option<u64>,
    active: bool,
    tx_byte: u8,
    rx_shift: u8,
    rx_buf: u8,
    bit_idx: u8,
    phase_second_half: bool,
    spsr_read_with_spif: bool,
    sck_last: bool,
    pins_claimed: bool,
}

impl Spi {
    pub fn from_spec(spec: &SpiSpec, data: &DataSpace, core: CoreKind, ints: &Interrupts) -> Self {
        let spcr_name = spec
            .config
            .regs_a
            .split(',')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if core == CoreKind::Avr {
                "SPCR"
            } else {
                "SSPCON"
            });
        let spsr_name = if !spec.status_reg.is_empty() {
            spec.status_reg.as_str()
        } else if core == CoreKind::Avr {
            "SPSR"
        } else {
            "SSPSTAT"
        };
        let spdr_name = if !spec.data_reg.is_empty() {
            spec.data_reg.as_str()
        } else if core == CoreKind::Avr {
            "SPDR"
        } else {
            "SSPBUF"
        };

        let mut prescalers: Vec<u32> = spec
            .prescalers
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .filter(|&p| p > 0)
            .collect();
        if prescalers.is_empty() {
            prescalers = vec![4, 16, 64, 128, 2, 8, 32, 64];
        }

        let sspm = data.get_reg_bits("SSPM0,SSPM1,SSPM2,SSPM3");
        let sspm_mask = if sspm.mask != 0 { sspm.mask } else { 0x0F };
        let sspm_shift = sspm.bit0;

        Self {
            core,
            name: spec.name.clone(),
            mosi: spec.pins.first().cloned(),
            miso: spec.pins.get(1).cloned(),
            sck: spec.pins.get(2).cloned(),
            ss: spec.pins.get(3).cloned(),
            spcr_addr: data.reg_addr(spcr_name),
            spsr_addr: data.reg_addr(spsr_name),
            spdr_addr: data.reg_addr(spdr_name),
            spe: data.get_reg_bits("SPE"),
            mstr: data.get_reg_bits("MSTR"),
            dord: data.get_reg_bits("DORD"),
            cpol_bit: data.get_reg_bits("CPOL"),
            cpha_bit: data.get_reg_bits("CPHA"),
            spr0: data.get_reg_bits("SPR0"),
            spr1: data.get_reg_bits("SPR1"),
            spi2x: data.get_reg_bits("SPI2X"),
            wcol: data.get_reg_bits("WCOL"),
            spif: data.get_reg_bits("SPIF"),
            sspen: data.get_reg_bits("SSPEN"),
            ckp: data.get_reg_bits("CKP"),
            sspm_mask,
            sspm_shift,
            cke: data.get_reg_bits("CKE"),
            bf: data.get_reg_bits("BF"),
            interrupt: ints.get(&spec.interrupt),
            prescalers,
            enabled: false,
            master: false,
            lsb_first: false,
            cpol: false,
            cpha: false,
            prescaler: 4,
            half_period_ps: 125_000,
            remain_ps: None,
            active: false,
            tx_byte: 0,
            rx_shift: 0,
            rx_buf: 0,
            bit_idx: 0,
            phase_second_half: false,
            spsr_read_with_spif: false,
            sck_last: false,
            pins_claimed: false,
        }
    }

    pub fn reset(&mut self, ports: &mut [Port]) {
        self.release_pins(ports);
        self.enabled = false;
        self.master = false;
        self.lsb_first = false;
        self.cpol = false;
        self.cpha = false;
        self.prescaler = self.prescalers.first().copied().unwrap_or(4);
        self.remain_ps = None;
        self.active = false;
        self.tx_byte = 0;
        self.rx_shift = 0;
        self.rx_buf = 0;
        self.bit_idx = 0;
        self.phase_second_half = false;
        self.spsr_read_with_spif = false;
        self.sck_last = false;
    }

    pub fn update_timing(&mut self, ps_inst: u64) {
        let base = ps_inst.max(1);
        self.half_period_ps = (base.saturating_mul(u64::from(self.prescaler.max(2))) / 2).max(1);
    }

    pub fn configure(&mut self, data: &DataSpace, ports: &mut [Port], ps_inst: u64) {
        let spcr = self.spcr_addr.map(|a| data.get(a)).unwrap_or(0);
        let spsr = self.spsr_addr.map(|a| data.get(a)).unwrap_or(0);

        match self.core {
            CoreKind::Avr => {
                self.enabled = self.spe.mask != 0 && (spcr & self.spe.mask) != 0;
                self.master = self.mstr.mask != 0 && (spcr & self.mstr.mask) != 0;
                self.lsb_first = self.dord.mask != 0 && (spcr & self.dord.mask) != 0;
                self.cpol = self.cpol_bit.mask != 0 && (spcr & self.cpol_bit.mask) != 0;
                self.cpha = self.cpha_bit.mask != 0 && (spcr & self.cpha_bit.mask) != 0;
                let mut idx = 0usize;
                if self.spr0.mask != 0 && (spcr & self.spr0.mask) != 0 {
                    idx |= 1;
                }
                if self.spr1.mask != 0 && (spcr & self.spr1.mask) != 0 {
                    idx |= 2;
                }
                if self.spi2x.mask != 0 && (spsr & self.spi2x.mask) != 0 {
                    idx |= 4;
                }
                self.prescaler = self
                    .prescalers
                    .get(idx)
                    .copied()
                    .unwrap_or_else(|| self.prescalers.first().copied().unwrap_or(4));
            }
            CoreKind::Pic12 | CoreKind::Pic14 => {
                let sspm = (spcr & self.sspm_mask) >> self.sspm_shift;
                let is_spi_mode = matches!(sspm, 0..=5);
                self.enabled = is_spi_mode && self.sspen.mask != 0 && (spcr & self.sspen.mask) != 0;
                self.master = matches!(sspm, 0..=3);
                self.lsb_first = false;
                self.cpol = self.ckp.mask != 0 && (spcr & self.ckp.mask) != 0;
                // On PIC MSSP, CKE=1 transmits on active-to-idle (equivalent to CPHA=0);
                // CKE=0 transmits on idle-to-active (equivalent to CPHA=1).
                let cke_set = self.cke.mask != 0 && (spsr & self.cke.mask) != 0;
                self.cpha = !cke_set;
                self.prescaler = match sspm {
                    0 => 4,
                    1 => 16,
                    2 => 64,
                    _ => 4,
                };
            }
            _ => {
                self.enabled = false;
            }
        }

        self.update_timing(ps_inst);
        if self.enabled {
            self.claim_pins(ports);
            if self.master {
                if let Some(sck) = &self.sck {
                    port::set_gpio_out(ports, sck, self.cpol);
                }
            } else if let Some(sck) = &self.sck {
                self.sck_last = port::gpio_inp(ports, sck).unwrap_or(self.cpol);
            }
        } else {
            self.active = false;
            self.remain_ps = None;
            self.release_pins(ports);
        }
    }

    pub fn write_data(&mut self, val: u8, data: &mut DataSpace, ports: &mut [Port]) {
        if !self.enabled {
            return;
        }
        if self.core == CoreKind::Avr && self.spsr_read_with_spif {
            self.clear_spif(data);
        }
        if self.active {
            // Write collision while shift register is busy.
            if self.wcol.mask != 0 {
                let r = data.get(self.wcol.reg_addr) | self.wcol.mask;
                data.set(self.wcol.reg_addr, r);
            }
            return;
        }

        self.tx_byte = val;
        self.rx_shift = 0;
        self.bit_idx = 0;
        self.phase_second_half = false;

        if self.master {
            self.active = true;
            if let Some(sck) = &self.sck {
                port::set_gpio_out(ports, sck, self.cpol);
            }
            if !self.cpha {
                self.drive_mosi_bit(ports, 0);
            }
            self.remain_ps = Some(self.half_period_ps.max(1));
        } else if !self.cpha {
            // Slave preloads bit 0 onto MISO before first SCK edge when CPHA == 0.
            self.drive_miso_bit(ports, 0);
        }
    }

    pub fn read_data(&mut self, data: &mut DataSpace) -> u8 {
        if self.core == CoreKind::Avr && self.spsr_read_with_spif {
            self.clear_spif(data);
        } else if matches!(self.core, CoreKind::Pic12 | CoreKind::Pic14) && self.bf.mask != 0 {
            let r = data.get(self.bf.reg_addr) & !self.bf.mask;
            data.set(self.bf.reg_addr, r);
        }
        self.rx_buf
    }

    pub fn read_status(&mut self, data: &DataSpace) {
        if self.core == CoreKind::Avr && self.spif.mask != 0 {
            let spsr = self.spsr_addr.map(|a| data.get(a)).unwrap_or(0);
            self.spsr_read_with_spif = (spsr & self.spif.mask) != 0;
        }
    }

    /// Advance master-mode SPI shift clock by `elapsed` picoseconds.
    pub fn advance(
        &mut self,
        elapsed: u64,
        data: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &mut [Port],
    ) -> bool {
        let Some(rem) = self.remain_ps else {
            return false;
        };
        if elapsed < rem {
            self.remain_ps = Some(rem - elapsed);
            return false;
        }
        if !self.enabled || !self.master || !self.active {
            self.remain_ps = None;
            return false;
        }

        if !self.phase_second_half {
            // Leading edge: idle (cpol) -> active (!cpol)
            if let Some(sck) = &self.sck {
                port::set_gpio_out(ports, sck, !self.cpol);
            }
            if !self.cpha {
                self.sample_miso_bit(ports, self.bit_idx);
            } else {
                self.drive_mosi_bit(ports, self.bit_idx);
            }
            self.phase_second_half = true;
            self.remain_ps = Some(self.half_period_ps.max(1));
        } else {
            // Trailing edge: active (!cpol) -> idle (cpol)
            if let Some(sck) = &self.sck {
                port::set_gpio_out(ports, sck, self.cpol);
            }
            if self.cpha {
                self.sample_miso_bit(ports, self.bit_idx);
            }
            self.bit_idx += 1;
            if self.bit_idx >= 8 {
                self.finish_transfer(data, ints);
            } else {
                if !self.cpha {
                    self.drive_mosi_bit(ports, self.bit_idx);
                }
                self.phase_second_half = false;
                self.remain_ps = Some(self.half_period_ps.max(1));
            }
        }
        true
    }

    /// Sample incoming SCK/SS/MOSI in slave mode when a GPIO input changes.
    pub fn on_pins_changed(
        &mut self,
        ports: &mut [Port],
        data: &mut DataSpace,
        ints: &mut Interrupts,
    ) {
        if !self.enabled || self.master {
            return;
        }
        if let Some(ss) = &self.ss {
            if port::gpio_inp(ports, ss).unwrap_or(false) {
                self.bit_idx = 0;
                self.rx_shift = 0;
                if let Some(sck) = &self.sck {
                    self.sck_last = port::gpio_inp(ports, sck).unwrap_or(self.cpol);
                }
                return;
            }
        }
        let Some(sck_name) = &self.sck else {
            return;
        };
        let sck_now = port::gpio_inp(ports, sck_name).unwrap_or(self.cpol);
        if sck_now == self.sck_last {
            return;
        }
        self.sck_last = sck_now;

        let is_leading = sck_now != self.cpol;
        let is_sample_edge = is_leading != self.cpha;
        if is_sample_edge {
            if let Some(mosi) = &self.mosi {
                let bit = port::gpio_inp(ports, mosi).unwrap_or(false);
                self.store_rx_bit(self.bit_idx, bit);
            }
            self.bit_idx += 1;
            if self.bit_idx >= 8 {
                self.bit_idx = 0;
                self.finish_transfer(data, ints);
                self.rx_shift = 0;
            }
        } else {
            self.drive_miso_bit(ports, self.bit_idx);
        }
    }

    fn finish_transfer(&mut self, data: &mut DataSpace, ints: &mut Interrupts) {
        self.active = false;
        self.remain_ps = None;
        self.rx_buf = self.rx_shift;
        if let Some(addr) = self.spdr_addr {
            data.set(addr, self.rx_buf);
        }
        match self.core {
            CoreKind::Avr => {
                if self.spif.mask != 0 {
                    let r = data.get(self.spif.reg_addr) | self.spif.mask;
                    data.set(self.spif.reg_addr, r);
                }
                self.spsr_read_with_spif = false;
            }
            CoreKind::Pic12 | CoreKind::Pic14 => {
                if self.bf.mask != 0 {
                    let r = data.get(self.bf.reg_addr) | self.bf.mask;
                    data.set(self.bf.reg_addr, r);
                }
            }
            _ => {}
        }
        if let Some(int_idx) = self.interrupt {
            ints.raise(int_idx, data);
        }
    }

    fn clear_spif(&mut self, data: &mut DataSpace) {
        self.spsr_read_with_spif = false;
        if self.spif.mask != 0 {
            let r = data.get(self.spif.reg_addr) & !self.spif.mask;
            data.set(self.spif.reg_addr, r);
        }
        if self.wcol.mask != 0 {
            let r = data.get(self.wcol.reg_addr) & !self.wcol.mask;
            data.set(self.wcol.reg_addr, r);
        }
    }

    fn tx_bit(&self, bit_idx: u8) -> bool {
        let shift = if self.lsb_first {
            bit_idx & 7
        } else {
            7 - (bit_idx & 7)
        };
        ((self.tx_byte >> shift) & 1) != 0
    }

    fn store_rx_bit(&mut self, bit_idx: u8, high: bool) {
        if !high {
            return;
        }
        let shift = if self.lsb_first {
            bit_idx & 7
        } else {
            7 - (bit_idx & 7)
        };
        self.rx_shift |= 1u8 << shift;
    }

    fn drive_mosi_bit(&self, ports: &mut [Port], bit_idx: u8) {
        if let Some(mosi) = &self.mosi {
            port::set_gpio_out(ports, mosi, self.tx_bit(bit_idx));
        }
    }

    fn drive_miso_bit(&self, ports: &mut [Port], bit_idx: u8) {
        if let Some(miso) = &self.miso {
            port::set_gpio_out(ports, miso, self.tx_bit(bit_idx));
        }
    }

    fn sample_miso_bit(&mut self, ports: &[Port], bit_idx: u8) {
        if let Some(miso) = &self.miso {
            let bit = port::gpio_inp(ports, miso).unwrap_or(false);
            self.store_rx_bit(bit_idx, bit);
        }
    }

    fn claim_pins(&mut self, ports: &mut [Port]) {
        self.release_pins(ports);
        self.pins_claimed = true;
        if self.master {
            if let Some(mosi) = &self.mosi {
                port::control_gpio(ports, mosi, true, false);
            }
            if let Some(sck) = &self.sck {
                port::control_gpio(ports, sck, true, false);
            }
            if let Some(miso) = &self.miso {
                port::control_gpio(ports, miso, false, true);
                port::set_gpio_dir(ports, miso, false);
            }
        } else {
            if let Some(miso) = &self.miso {
                port::control_gpio(ports, miso, true, false);
            }
            if let Some(mosi) = &self.mosi {
                port::control_gpio(ports, mosi, false, true);
                port::set_gpio_dir(ports, mosi, false);
            }
            if let Some(sck) = &self.sck {
                port::control_gpio(ports, sck, false, true);
                port::set_gpio_dir(ports, sck, false);
            }
        }
    }

    fn release_pins(&mut self, ports: &mut [Port]) {
        if !self.pins_claimed {
            return;
        }
        self.pins_claimed = false;
        for pin in [&self.mosi, &self.miso, &self.sck].into_iter().flatten() {
            port::control_gpio(ports, pin, false, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Device;

    const AVR_SPI_XML: &str = r#"
<mcu name="mega328spi" core="AVR" data="512" prog="256" progword="2" inst_cycle="1" freq="16000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="PINB" addr="0x0023" reset="" mask="11111111"/>
    <register name="DDRB" addr="0x0024" reset="" mask="11111111"/>
    <register name="PORTB" addr="0x0025" reset="" mask="11111111"/>
    <register name="SPCR" addr="0x004C" reset="" bits="SPR0,SPR1,CPHA,CPOL,MSTR,DORD,SPE,SPIE"/>
    <register name="SPSR" addr="0x004D" reset="" mask="00000001" bits="SPI2X,0,0,0,0,0,WCOL,SPIF"/>
    <register name="SPDR" addr="0x004E" reset=""/>
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <port name="PORTB" pins="8" outreg="PORTB" inreg="PINB" dirreg="DDRB"/>
  <interrupts enable="I">
    <interrupt name="SPI" enable="SPIE" flag="SPIF" priority="9" vector="0x0022"/>
  </interrupts>
  <spi name="SPI" configregsA="SPCR" dataregs="SPDR" statusreg="SPSR"
       interrupt="SPI" prescalers="4,16,64,128,2,8,32,64"
       pins="PORTB3,PORTB4,PORTB5,PORTB2"/>
</mcu>
"#;

    #[test]
    fn avr_spi_master_loopback_modes_and_spif_clear() {
        for mode in 0u8..4 {
            let mut dev = Device::from_xml("m", AVR_SPI_XML).unwrap();
            dev.start();
            // DDRB: MOSI (PB3), SCK (PB5), SS (PB2) outputs
            dev.poke_reg(0x0024, (1 << 3) | (1 << 5) | (1 << 2));
            let cpol = if (mode & 2) != 0 { 1 << 3 } else { 0 };
            let cpha = if (mode & 1) != 0 { 1 << 2 } else { 0 };
            // SPE (0x40) | MSTR (0x10) | CPOL | CPHA | prescaler /4
            dev.poke_reg(0x004C, 0x50 | cpol | cpha);
            // Send 0xA5 with MOSI looped back to MISO on each step
            dev.poke_reg(0x004E, 0xA5);

            for _ in 0..200 {
                // Loop MOSI (PORTB3) -> MISO (PORTB4)
                let mosi = dev
                    .gpio_pins()
                    .find(|p| p.name.ends_with("PORTB3"))
                    .map(|p| p.out_state)
                    .unwrap_or(false);
                dev.set_pin_input("m-PORTB4", mosi);
                dev.advance();
                if (dev.ram(0x004D) & 0x80) != 0 {
                    break;
                }
            }
            assert_ne!(
                dev.ram(0x004D) & 0x80,
                0,
                "SPIF must set after transfer in mode {mode}"
            );
            // Two-step AVR SPIF clear: read SPSR then read SPDR
            let _ = dev.cpu_read_reg(0x004D);
            let rx = dev.cpu_read_reg(0x004E);
            assert_eq!(rx, 0xA5, "loopback byte mismatch in mode {mode}");
            assert_eq!(
                dev.ram(0x004D) & 0x80,
                0,
                "SPIF must clear after SPSR+SPDR read"
            );
        }
    }

    #[test]
    fn avr_spi_lsb_first_transfer() {
        let mut dev = Device::from_xml("m", AVR_SPI_XML).unwrap();
        dev.start();
        dev.poke_reg(0x0024, (1 << 3) | (1 << 5) | (1 << 2));
        // SPE | MSTR | DORD (LSB first = 0x20)
        dev.poke_reg(0x004C, 0x70);
        dev.poke_reg(0x004E, 0xB4);
        let mut shifted_out = Vec::new();
        let mut last_sck = false;
        for _ in 0..200 {
            let mosi = dev
                .gpio_pins()
                .find(|p| p.name.ends_with("PORTB3"))
                .map(|p| p.out_state)
                .unwrap_or(false);
            let sck = dev
                .gpio_pins()
                .find(|p| p.name.ends_with("PORTB5"))
                .map(|p| p.out_state)
                .unwrap_or(false);
            if sck && !last_sck {
                shifted_out.push(mosi);
            }
            last_sck = sck;
            dev.set_pin_input("m-PORTB4", mosi);
            dev.advance();
            if (dev.ram(0x004D) & 0x80) != 0 {
                break;
            }
        }
        // 0xB4 = 0b10110100 -> LSB first: 0,0,1,0,1,1,0,1
        assert_eq!(
            shifted_out,
            vec![false, false, true, false, true, true, false, true]
        );
        assert_eq!(dev.cpu_read_reg(0x004E), 0xB4);
    }
}
