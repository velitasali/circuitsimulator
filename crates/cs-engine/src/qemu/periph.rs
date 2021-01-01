//! QEMU USART / SPI / TWI / timer register maps (C++ `Esp32*` / `Stm32*`).

use std::collections::VecDeque;

use crate::digital::{
    IoPin, PinMode, SpiMode, SpiModule, SpiPins, TwiMode, TwiModule, TwiPins, TwiState, UsartModule,
};

pub const ESP32_UART0_START: u64 = 0x0004_0000;
pub const ESP32_UART1_START: u64 = 0x0005_0000;
pub const ESP32_UART2_START: u64 = 0x0006_E000;
pub const ESP32_UART_SIZE: u64 = 0x1000;
pub const ESP32_I2C0_START: u64 = 0x0005_3000;
pub const ESP32_I2C1_START: u64 = 0x0006_7000;
pub const ESP32_I2C_SIZE: u64 = 0x1000;
pub const ESP32_HSPI_START: u64 = 0x0006_4000;
pub const ESP32_VSPI_START: u64 = 0x0006_5000;
pub const ESP32_SPI_SIZE: u64 = 0x1000;

pub const STM32_PERIPH_SIZE: u64 = 0x400;
pub const STM32_TIM2_START: u64 = 0x0000_0000;
pub const STM32_TIM3_START: u64 = 0x0000_0400;
pub const STM32_TIM4_START: u64 = 0x0000_0800;
pub const STM32_SPI2_START: u64 = 0x0000_3800;
pub const STM32_SPI3_START: u64 = 0x0000_3C00;
pub const STM32_USART2_START: u64 = 0x0000_4400;
pub const STM32_USART3_START: u64 = 0x0000_4800;
pub const STM32_UART4_START: u64 = 0x0000_4C00;
pub const STM32_UART5_START: u64 = 0x0000_5000;
pub const STM32_I2C1_START: u64 = 0x0000_5400;
pub const STM32_I2C2_START: u64 = 0x0000_5800;
pub const STM32_TIM1_START: u64 = 0x0001_2C00;
pub const STM32_SPI1_START: u64 = 0x0001_3000;
pub const STM32_USART1_START: u64 = 0x0001_3800;

const SR_TXE: u16 = 1 << 7;
const SR_TC: u16 = 1 << 6;
const SR_RXNE: u16 = 1 << 5;
const SR_ORE: u16 = 1 << 3;
const SPI_SR_TXE: u16 = 1 << 1;
const SPI_SR_RXNE: u16 = 1 << 0;
const SPI_SR_BUSY: u16 = 1 << 7;
const I2C_PE: u16 = 1 << 0;
const I2C_START: u16 = 1 << 8;
const I2C_STOP: u16 = 1 << 9;
const I2C_SB: u16 = 1 << 0;
const I2C_MSL: u16 = 1 << 0;
const I2C_BUSY: u16 = 1 << 1;

#[derive(Clone, Debug)]
pub struct QemuUsart {
    pub mem_start: u64,
    #[allow(dead_code)]
    pub number: u8,
    pub module: UsartModule,
    pub tx: Option<usize>,
    pub rx: Option<usize>,
    pub tx_fifo: VecDeque<u8>,
    pub rx_fifo: VecDeque<u8>,
    pub divider: f64,
    pub int_raw: u32,
    pub int_en: u32,
    pub rx_full_thrhd: usize,
    pub tx_empty_thrhd: usize,
    pub irq_level: u8,
    pub irq_src: u8,
    pub sr: u16,
    pub dr: u16,
    pub int_enable: u16,
    pub ore_read: u16,
    pub family: UsartFamily,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsartFamily {
    Esp32,
    Stm32,
}

impl QemuUsart {
    pub fn esp32(mem_start: u64, number: u8, tx: Option<usize>, rx: Option<usize>) -> Self {
        let mut module = UsartModule::new();
        module.set_baud_rate(115200);
        Self {
            mem_start,
            number,
            module,
            tx,
            rx,
            tx_fifo: VecDeque::new(),
            rx_fifo: VecDeque::new(),
            divider: 0.0,
            int_raw: 0,
            int_en: 0,
            rx_full_thrhd: 11,
            tx_empty_thrhd: 10,
            irq_level: 0,
            irq_src: 34 + number,
            sr: 0,
            dr: 0,
            int_enable: 0,
            ore_read: 0,
            family: UsartFamily::Esp32,
        }
    }

    pub fn stm32(mem_start: u64, number: u8, tx: Option<usize>, rx: Option<usize>) -> Self {
        let irq = [37u8, 38, 39, 52, 53][number.min(4) as usize];
        let mut u = Self::esp32(mem_start, number, tx, rx);
        u.family = UsartFamily::Stm32;
        u.irq_src = irq;
        u.module.set_baud_rate(9600);
        u.sr = SR_TXE | SR_TC;
        u
    }

    pub fn reset(&mut self) {
        self.tx_fifo.clear();
        self.rx_fifo.clear();
        self.divider = 0.0;
        self.int_raw = 0;
        self.int_en = 0;
        self.irq_level = 0;
        self.sr = SR_TXE | SR_TC;
        self.dr = 0;
        self.int_enable = 0;
        self.ore_read = 0;
        if self.family == UsartFamily::Esp32 {
            self.module.set_baud_rate(115200);
        } else {
            self.module.set_baud_rate(9600);
        }
    }

    pub fn write(
        &mut self,
        offset: u64,
        value: u32,
        now: u64,
        pins: &mut [IoPin],
        apb: u32,
    ) -> Option<(u8, u8)> {
        match self.family {
            UsartFamily::Esp32 => self.esp32_write(offset, value, now, pins, apb),
            UsartFamily::Stm32 => self.stm32_write(offset, value, now, pins, apb),
        }
    }

    pub fn read(&mut self, offset: u64) -> u32 {
        match self.family {
            UsartFamily::Esp32 => self.esp32_read(offset),
            UsartFamily::Stm32 => self.stm32_read(offset),
        }
    }

    fn esp32_write(
        &mut self,
        offset: u64,
        value: u32,
        now: u64,
        pins: &mut [IoPin],
        apb: u32,
    ) -> Option<(u8, u8)> {
        match offset {
            0x00 => {
                if self.tx_fifo.len() >= 128 {
                    return self.update_irq();
                }
                self.tx_fifo.push_back(value as u8);
                if self.tx_fifo.len() == 1 {
                    self.module.send_byte(value as u8, now, pins, self.tx);
                }
                self.update_irq()
            }
            0x0C => {
                self.int_en = value;
                self.update_irq()
            }
            0x10 => {
                self.int_raw &= !value;
                self.update_irq()
            }
            0x14 => {
                let clk_fra = (value >> 20) & 0x0F;
                let clk_int = value & 0x000F_FFFF;
                self.divider = f64::from(clk_int) + f64::from(clk_fra) / 16.0;
                self.apply_baud(apb);
                None
            }
            0x20 => {
                let parity_en = value & (1 << 1) != 0;
                let parity_odd = value & 1 != 0;
                self.module.set_parity(if parity_en {
                    if parity_odd {
                        crate::digital::Parity::Odd
                    } else {
                        crate::digital::Parity::Even
                    }
                } else {
                    crate::digital::Parity::None
                });
                let data_bits = ((value >> 2) & 0b11) as u8;
                self.module.set_data_bits(5 + data_bits);
                let stop = (value >> 4) & 0b11;
                self.module.set_stop_bits(if stop == 3 { 2 } else { 1 });
                if value & (1 << 18) != 0 {
                    self.tx_fifo.clear();
                }
                if value & (1 << 17) != 0 {
                    self.rx_fifo.clear();
                }
                self.update_irq()
            }
            0x24 => {
                self.rx_full_thrhd = (value & 0x7F) as usize;
                self.tx_empty_thrhd = ((value >> 8) & 0x7F) as usize;
                self.update_irq()
            }
            _ => None,
        }
    }

    fn esp32_read(&mut self, offset: u64) -> u32 {
        match offset {
            0x00 => self.rx_fifo.pop_front().unwrap_or(0) as u32,
            0x04 => self.int_raw,
            0x08 => self.int_raw & self.int_en,
            0x0C => self.int_en,
            0x1C => (self.rx_fifo.len() as u32 & 0xFF) | ((self.tx_fifo.len() as u32 & 0xFF) << 16),
            0x24 => {
                ((self.rx_full_thrhd as u32) & 0x7F) | (((self.tx_empty_thrhd as u32) & 0x7F) << 8)
            }
            _ => 0,
        }
    }

    fn stm32_write(
        &mut self,
        offset: u64,
        value: u32,
        now: u64,
        pins: &mut [IoPin],
        apb: u32,
    ) -> Option<(u8, u8)> {
        match offset {
            0x00 => {
                if value & u32::from(SR_RXNE) == 0 {
                    self.sr &= !SR_RXNE;
                }
                if value & u32::from(SR_TC) == 0 {
                    self.sr &= !SR_TC;
                }
                self.update_irq()
            }
            0x04 => {
                self.dr = value as u16;
                if self.sr & SR_TC != 0 {
                    self.sr &= !SR_TC;
                    self.module.send_byte(self.dr as u8, now, pins, self.tx);
                    self.sr |= SR_TXE;
                } else {
                    self.sr &= !SR_TXE;
                }
                self.update_irq()
            }
            0x08 => {
                let fraction = f64::from(value & 0xF);
                self.divider = f64::from(value >> 4) + fraction / 16.0;
                self.apply_baud(apb);
                None
            }
            0x0C => {
                self.int_enable = value as u16 & ((1 << 7) | (1 << 6) | (1 << 5));
                let ue = value & (1 << 13) != 0;
                let te = value & (1 << 3) != 0;
                let re = value & (1 << 2) != 0;
                self.module.enable_tx(ue && te, pins, self.tx);
                self.module.enable_rx(ue && re);
                self.update_irq()
            }
            _ => None,
        }
    }

    fn stm32_read(&mut self, offset: u64) -> u32 {
        match offset {
            0x00 => {
                self.ore_read = self.sr & SR_ORE;
                u32::from(self.sr)
            }
            0x04 => {
                if self.ore_read != 0 {
                    self.ore_read = 0;
                    self.sr &= !SR_ORE;
                }
                self.sr &= !SR_RXNE;
                u32::from(self.module.get_rx_data())
            }
            _ => 0,
        }
    }

    fn apply_baud(&mut self, apb: u32) {
        if self.divider <= 0.0 {
            return;
        }
        let freq = if apb == 0 { 80_000_000 } else { apb };
        let baud = ((f64::from(freq) / self.divider).round() as i32).max(1);
        self.module.set_baud_rate(baud);
    }

    pub fn on_frame_sent(&mut self, now: u64, pins: &mut [IoPin]) -> Option<(u8, u8)> {
        match self.family {
            UsartFamily::Esp32 => {
                let _ = self.tx_fifo.pop_front();
                if let Some(&next) = self.tx_fifo.front() {
                    self.module.send_byte(next, now, pins, self.tx);
                }
                self.update_irq()
            }
            UsartFamily::Stm32 => {
                if self.sr & SR_TXE != 0 {
                    self.sr |= SR_TC;
                } else {
                    self.sr &= !SR_TC;
                    self.module.send_byte(self.dr as u8, now, pins, self.tx);
                    self.sr |= SR_TXE;
                }
                self.update_irq()
            }
        }
    }

    pub fn on_byte_received(&mut self, data: u8) -> Option<(u8, u8)> {
        match self.family {
            UsartFamily::Esp32 => {
                self.rx_fifo.push_back(data);
                self.update_irq()
            }
            UsartFamily::Stm32 => {
                if self.sr & SR_RXNE != 0 {
                    self.sr |= SR_ORE;
                    self.ore_read = 0;
                } else {
                    self.sr |= SR_RXNE;
                }
                self.module.inject_rx(data);
                self.update_irq()
            }
        }
    }

    fn update_irq(&mut self) -> Option<(u8, u8)> {
        match self.family {
            UsartFamily::Esp32 => {
                if self.tx_fifo.len() <= self.tx_empty_thrhd {
                    self.int_raw |= 1 << 1;
                } else {
                    self.int_raw &= !(1 << 1);
                }
                if self.tx_fifo.is_empty() {
                    self.int_raw |= 1 << 14;
                } else {
                    self.int_raw &= !(1 << 14);
                }
                if self.rx_fifo.len() >= self.rx_full_thrhd {
                    self.int_raw |= 1;
                } else {
                    self.int_raw &= !1;
                }
                let level = u8::from(self.int_raw & self.int_en != 0);
                if self.irq_level != level {
                    self.irq_level = level;
                    return Some((self.irq_src, level));
                }
            }
            UsartFamily::Stm32 => {
                let flags = self.sr & (SR_TC | SR_TXE | SR_RXNE | SR_ORE);
                let level = u8::from(self.int_enable & flags != 0);
                if self.irq_level != level {
                    self.irq_level = level;
                    return Some((self.irq_src, level));
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct QemuSpi {
    pub mem_start: u64,
    pub module: SpiModule,
    pub pins: SpiPins,
    pub cr1: u16,
    pub cr2: u16,
    pub status: u16,
    pub family: SpiFamily,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiFamily {
    Esp32,
    Stm32,
}

impl QemuSpi {
    pub fn esp32(mem_start: u64, pins: SpiPins) -> Self {
        Self {
            mem_start,
            module: SpiModule::new(),
            pins,
            cr1: 0,
            cr2: 0,
            status: 0,
            family: SpiFamily::Esp32,
        }
    }

    pub fn stm32(mem_start: u64, pins: SpiPins) -> Self {
        Self {
            mem_start,
            module: SpiModule::new(),
            pins,
            cr1: 0,
            cr2: 0,
            status: SPI_SR_TXE,
            family: SpiFamily::Stm32,
        }
    }

    pub fn reset(&mut self) {
        self.cr1 = 0;
        self.cr2 = 0;
        self.status = if self.family == SpiFamily::Stm32 {
            SPI_SR_TXE
        } else {
            0
        };
        self.module.mode = SpiMode::Off;
    }

    pub fn write(&mut self, offset: u64, value: u32, now: u64, pins: &mut [IoPin], ps_inst: f64) {
        match self.family {
            SpiFamily::Esp32 => match offset {
                0x18 => self.module.clock_period = (value as u64 / 2).max(1),
                0x38 => {
                    let slave = value & (1 << 30) != 0;
                    self.module.set_mode(
                        if slave {
                            SpiMode::Slave
                        } else {
                            SpiMode::Master
                        },
                        pins,
                        self.pins,
                    );
                }
                0x80 => {
                    self.module.sr_reg = value as u8;
                    self.module.start_transaction(now, pins, self.pins);
                }
                _ => {}
            },
            SpiFamily::Stm32 => match offset {
                0x00 => {
                    let new = value as u16;
                    if self.cr1 == new {
                        return;
                    }
                    let cpha = new & 1 != 0;
                    let cpol = new & (1 << 1) != 0;
                    self.module.set_cpol_cpha(cpol, cpha);
                    let master = new & (1 << 2) != 0;
                    let mode = if master {
                        SpiMode::Master
                    } else {
                        SpiMode::Slave
                    };
                    let spr = ((new >> 3) & 0b111) as usize;
                    let presc = [2u16, 4, 8, 16, 32, 64, 128, 256][spr.min(7)];
                    let period = (ps_inst.max(1.0) * f64::from(presc)) as u64;
                    self.module.clock_period = period.max(1);
                    let enabled = new & (1 << 6) != 0;
                    if !enabled {
                        self.module.set_mode(SpiMode::Off, pins, self.pins);
                    } else {
                        self.module.set_mode(mode, pins, self.pins);
                        if mode == SpiMode::Master {
                            if let Some(i) = self.pins.clk {
                                if let Some(p) = pins.get_mut(i) {
                                    p.set_out_state(cpol);
                                }
                            }
                        }
                    }
                    self.module.lsb_first = new & (1 << 7) != 0;
                    self.cr1 = new;
                }
                0x04 => self.cr2 = value as u16,
                0x0C => {
                    self.status |= SPI_SR_BUSY;
                    self.status &= !SPI_SR_TXE;
                    self.module.sr_reg = value as u8;
                    self.module.start_transaction(now, pins, self.pins);
                }
                _ => {}
            },
        }
    }

    pub fn read(&self, offset: u64) -> u32 {
        match self.family {
            SpiFamily::Esp32 => {
                if offset == 0x80 {
                    u32::from(self.module.data_reg)
                } else {
                    0
                }
            }
            SpiFamily::Stm32 => match offset {
                0x00 => u32::from(self.cr1),
                0x04 => u32::from(self.cr2),
                0x08 => u32::from(self.status),
                0x0C => u32::from(self.module.data_reg),
                _ => 0,
            },
        }
    }

    pub fn on_end(&mut self) {
        self.module.data_reg = self.module.sr_reg;
        if self.family == SpiFamily::Stm32 {
            self.status |= SPI_SR_RXNE | SPI_SR_TXE;
            self.status &= !SPI_SR_BUSY;
        }
    }
}

#[derive(Clone, Debug)]
pub struct QemuTwi {
    pub mem_start: u64,
    #[allow(dead_code)]
    pub number: u8,
    pub module: TwiModule,
    pub pins: TwiPins,
    pub family: TwiFamily,
    pub ctr: u32,
    pub int_raw: u32,
    pub int_ena: u32,
    pub cmd: [u32; 16],
    pub tx_fifo: VecDeque<u8>,
    pub rx_fifo: VecDeque<u8>,
    pub cmd_idx: usize,
    pub byte_count: u8,
    pub first_byte: bool,
    pub err_stop: bool,
    pub busy: bool,
    pub fifo_conf: u32,
    pub low_period: u32,
    pub high_period: u32,
    pub cr1: u16,
    pub cr2: u16,
    pub sr1: u16,
    pub sr2: u16,
    pub dr: u16,
    pub enabled: bool,
    pub irq_src: u8,
    pub irq_level: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiFamily {
    Esp32,
    Stm32,
}

const CMD_DONE: u32 = 1 << 31;
const INT_TRANS_COMPLETE: u32 = 1 << 7;
const INT_END_DETECT: u32 = 1 << 3;
const INT_ACK_ERR: u32 = 1 << 10;

impl QemuTwi {
    pub fn esp32(mem_start: u64, number: u8, pins: TwiPins) -> Self {
        Self {
            mem_start,
            number,
            module: TwiModule::new(),
            pins,
            family: TwiFamily::Esp32,
            ctr: 0,
            int_raw: 0,
            int_ena: 0,
            cmd: [0; 16],
            tx_fifo: VecDeque::new(),
            rx_fifo: VecDeque::new(),
            cmd_idx: 0,
            byte_count: 0,
            first_byte: false,
            err_stop: false,
            busy: false,
            fifo_conf: 0,
            low_period: 0,
            high_period: 0,
            cr1: 0,
            cr2: 0,
            sr1: 0,
            sr2: 0,
            dr: 0,
            enabled: false,
            irq_src: if number == 0 { 49 } else { 50 },
            irq_level: 0,
        }
    }

    pub fn stm32(mem_start: u64, number: u8, pins: TwiPins) -> Self {
        let mut t = Self::esp32(mem_start, number, pins);
        t.family = TwiFamily::Stm32;
        t.irq_src = if number == 0 { 31 } else { 33 };
        t
    }

    pub fn reset(&mut self) {
        self.ctr = 0;
        self.int_raw = 0;
        self.int_ena = 0;
        self.cmd = [0; 16];
        self.tx_fifo.clear();
        self.rx_fifo.clear();
        self.cmd_idx = 0;
        self.byte_count = 0;
        self.first_byte = false;
        self.err_stop = false;
        self.busy = false;
        self.cr1 = 0;
        self.cr2 = 0;
        self.sr1 = 0;
        self.sr2 = 0;
        self.dr = 0;
        self.enabled = false;
        self.irq_level = 0;
    }

    pub fn write(
        &mut self,
        offset: u64,
        value: u32,
        now: u64,
        pins: &mut [IoPin],
    ) -> Option<(u8, u8)> {
        match self.family {
            TwiFamily::Esp32 => self.esp32_write(offset, value, now, pins),
            TwiFamily::Stm32 => self.stm32_write(offset, value, now, pins),
        }
    }

    pub fn read(&mut self, offset: u64) -> u32 {
        match self.family {
            TwiFamily::Esp32 => self.esp32_read(offset),
            TwiFamily::Stm32 => self.stm32_read(offset),
        }
    }

    fn esp32_write(
        &mut self,
        offset: u64,
        value: u32,
        now: u64,
        pins: &mut [IoPin],
    ) -> Option<(u8, u8)> {
        if (0x58..0x58 + 16 * 4).contains(&offset) {
            let idx = ((offset - 0x58) / 4) as usize;
            self.cmd[idx] = value;
            return None;
        }
        if (0x1C..=0x1F).contains(&offset) || (0x100..0x200).contains(&offset) {
            self.tx_fifo.push_back(value as u8);
            return None;
        }
        match offset {
            0x00 => {
                self.low_period = value;
                let cycles = (value & 0xFFFF).max(1);
                self.module.clock_period = u64::from(cycles) * 12_500;
            }
            0x04 => {
                let mode = if value & (1 << 4) != 0 {
                    TwiMode::Master
                } else {
                    TwiMode::Slave
                };
                if let Some(i) = self.pins.sda {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_pullup(1e5);
                        p.set_pin_mode(PinMode::OpenCo);
                    }
                }
                if let Some(i) = self.pins.scl {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_pullup(1e5);
                        p.set_pin_mode(PinMode::Output);
                    }
                }
                self.module.set_mode(mode, now, pins, self.pins);
                let mut val = value;
                if val & (1 << 5) != 0 {
                    val &= !(1 << 5);
                    self.run_transaction(now, pins);
                }
                self.ctr = val;
            }
            0x18 => {
                self.fifo_conf = value;
                if value & (1 << 12) != 0 {
                    self.rx_fifo.clear();
                }
                if value & (1 << 13) != 0 {
                    self.tx_fifo.clear();
                }
            }
            0x24 => self.int_raw &= !value,
            0x28 => self.int_ena = value,
            0x38 => self.high_period = value,
            _ => {}
        }
        self.update_irq()
    }

    fn esp32_read(&mut self, offset: u64) -> u32 {
        if (0x58..0x58 + 16 * 4).contains(&offset) {
            let idx = ((offset - 0x58) / 4) as usize;
            return self.cmd[idx];
        }
        if (0x1C..=0x1F).contains(&offset) || (0x100..0x200).contains(&offset) {
            return u32::from(self.rx_fifo.pop_front().unwrap_or(0));
        }
        match offset {
            0x00 => self.low_period,
            0x04 => self.ctr,
            0x08 => {
                let mut val = 0u32;
                if self.busy && !self.err_stop {
                    val |= 1 << 4;
                }
                val |= (self.rx_fifo.len().min(32) as u32) << 8;
                val |= (self.tx_fifo.len().min(32) as u32) << 18;
                val
            }
            0x18 => self.fifo_conf,
            0x20 => self.int_raw,
            0x28 => self.int_ena,
            0x2C => self.int_raw & self.int_ena,
            0x38 => self.high_period,
            _ => 0,
        }
    }

    fn stm32_write(
        &mut self,
        offset: u64,
        value: u32,
        now: u64,
        pins: &mut [IoPin],
    ) -> Option<(u8, u8)> {
        match offset {
            0x00 => {
                let new = value as u16;
                let enabled = new & I2C_PE != 0;
                if self.enabled != enabled {
                    if enabled {
                        self.module.set_mode(TwiMode::Master, now, pins, self.pins);
                    } else {
                        self.module.set_mode(TwiMode::Off, now, pins, self.pins);
                        self.sr1 = 0;
                        self.sr2 = 0;
                    }
                    self.enabled = enabled;
                }
                if new & I2C_START != 0 && self.module.mode == TwiMode::Master {
                    self.sr2 |= I2C_MSL | I2C_BUSY;
                    self.module.master_start(now, pins, self.pins);
                }
                if new & I2C_STOP != 0 && self.module.mode == TwiMode::Master {
                    self.module.master_stop(pins, self.pins);
                }
                self.cr1 = new & !I2C_START;
            }
            0x04 => self.cr2 = value as u16,
            0x10 => {
                self.dr = value as u16;
                let status = self.module.status();
                let is_addr = status == TwiState::Start as u8 || status == TwiState::RepStart as u8;
                let write = if is_addr { value & 1 == 0 } else { true };
                self.module
                    .master_write(value as u8, is_addr, write, pins, self.pins);
            }
            _ => {}
        }
        self.update_irq()
    }

    fn stm32_read(&mut self, offset: u64) -> u32 {
        match offset {
            0x00 => u32::from(self.cr1),
            0x04 => u32::from(self.cr2),
            0x10 => u32::from(self.dr),
            0x14 => {
                if self.module.twi_state == TwiState::Start {
                    self.sr1 |= I2C_SB;
                }
                u32::from(self.sr1)
            }
            0x18 => u32::from(self.sr2),
            _ => 0,
        }
    }

    fn run_transaction(&mut self, now: u64, pins: &mut [IoPin]) {
        self.cmd_idx = 0;
        self.byte_count = 0;
        self.err_stop = false;
        self.first_byte = false;
        self.busy = true;
        for c in &mut self.cmd {
            *c &= !CMD_DONE;
        }
        self.int_raw &= !(INT_TRANS_COMPLETE | INT_END_DETECT | INT_ACK_ERR);
        self.advance_cmd(now, pins);
    }

    fn advance_cmd(&mut self, now: u64, pins: &mut [IoPin]) {
        while self.cmd_idx < 16 {
            let cmd = self.cmd[self.cmd_idx];
            let opcode = (cmd >> 11) & 7;
            match opcode {
                0 => {
                    self.first_byte = true;
                    self.module.master_start(now, pins, self.pins);
                    return;
                }
                1 => {
                    let n = (cmd & 0xFF) as u8;
                    if n == 0 {
                        self.cmd[self.cmd_idx] |= CMD_DONE;
                        self.cmd_idx += 1;
                        continue;
                    }
                    if self.byte_count == 0 {
                        self.byte_count = n;
                    }
                    if let Some(b) = self.tx_fifo.pop_front() {
                        self.byte_count = self.byte_count.saturating_sub(1);
                        let is_addr = self.first_byte;
                        self.first_byte = false;
                        let write = if is_addr { b & 1 == 0 } else { true };
                        self.module.master_write(b, is_addr, write, pins, self.pins);
                        return;
                    }
                    return;
                }
                2 => {
                    let n = (cmd & 0xFF) as u8;
                    if n == 0 {
                        self.cmd[self.cmd_idx] |= CMD_DONE;
                        self.cmd_idx += 1;
                        continue;
                    }
                    if self.byte_count == 0 {
                        self.byte_count = n;
                    }
                    let ack = self.byte_count > 1;
                    self.module.master_read(ack, now, pins, self.pins);
                    return;
                }
                3 | 4 => {
                    self.cmd[self.cmd_idx] |= CMD_DONE;
                    self.cmd_idx += 1;
                    self.module.master_stop(pins, self.pins);
                    self.busy = false;
                    self.int_raw |= INT_TRANS_COMPLETE | INT_END_DETECT;
                    return;
                }
                _ => {
                    self.cmd[self.cmd_idx] |= CMD_DONE;
                    self.cmd_idx += 1;
                }
            }
        }
        self.module.master_stop(pins, self.pins);
        self.busy = false;
        self.int_raw |= INT_TRANS_COMPLETE;
    }

    pub fn on_state(&mut self, state: TwiState, now: u64, pins: &mut [IoPin]) {
        if self.family != TwiFamily::Esp32 || self.err_stop {
            if self.family == TwiFamily::Stm32 && state == TwiState::Start {
                self.sr1 |= I2C_SB;
            }
            return;
        }
        match state {
            TwiState::Start | TwiState::RepStart => {
                self.first_byte = true;
                if self.cmd_idx < 16 {
                    self.cmd[self.cmd_idx] |= CMD_DONE;
                    self.cmd_idx += 1;
                }
                self.advance_cmd(now, pins);
            }
            TwiState::MtxAdrAck | TwiState::MtxDataAck => {
                if self.byte_count > 0 {
                    if let Some(b) = self.tx_fifo.pop_front() {
                        self.byte_count = self.byte_count.saturating_sub(1);
                        self.module.master_write(b, false, true, pins, self.pins);
                    }
                } else {
                    if self.cmd_idx < 16 {
                        self.cmd[self.cmd_idx] |= CMD_DONE;
                        self.cmd_idx += 1;
                    }
                    self.advance_cmd(now, pins);
                }
            }
            TwiState::MrxAdrAck => {
                if self.cmd_idx < 16 {
                    self.cmd[self.cmd_idx] |= CMD_DONE;
                    self.cmd_idx += 1;
                }
                self.advance_cmd(now, pins);
            }
            TwiState::MrxDataAck | TwiState::MrxDataNack => {
                self.rx_fifo.push_back(self.module.rx_reg);
                self.byte_count = self.byte_count.saturating_sub(1);
                if self.byte_count > 0 {
                    let ack = self.byte_count > 1;
                    self.module.master_read(ack, now, pins, self.pins);
                } else {
                    if self.cmd_idx < 16 {
                        self.cmd[self.cmd_idx] |= CMD_DONE;
                        self.cmd_idx += 1;
                    }
                    self.advance_cmd(now, pins);
                }
            }
            TwiState::MtxAdrNack | TwiState::MtxDataNack | TwiState::MrxAdrNack => {
                self.int_raw |= INT_ACK_ERR | INT_TRANS_COMPLETE;
                self.err_stop = true;
                self.busy = false;
                self.module.master_stop(pins, self.pins);
            }
            _ => {}
        }
    }

    fn update_irq(&mut self) -> Option<(u8, u8)> {
        let level = u8::from(self.int_raw & self.int_ena != 0);
        if self.irq_level != level {
            self.irq_level = level;
            Some((self.irq_src, level))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct QemuTimer {
    pub mem_start: u64,
    pub cr1: u16,
    pub cr2: u16,
    pub smcr: u16,
    pub dier: u16,
    pub sr: u16,
    pub egr: u16,
    pub ccmr: [u16; 2],
    pub ccer: u16,
    pub psc: u16,
    pub arr: u16,
    pub rcr: u16,
    pub ccr: [u16; 4],
    pub oc_pins: [Option<usize>; 4],
}

impl QemuTimer {
    pub fn new(mem_start: u64, oc_pins: [Option<usize>; 4]) -> Self {
        Self {
            mem_start,
            cr1: 0,
            cr2: 0,
            smcr: 0,
            dier: 0,
            sr: 0,
            egr: 0,
            ccmr: [0; 2],
            ccer: 0,
            psc: 0,
            arr: 0,
            rcr: 0,
            ccr: [0; 4],
            oc_pins,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.mem_start, self.oc_pins);
    }

    pub fn write(&mut self, offset: u64, value: u32, pins: &mut [IoPin]) {
        // C++ Stm32Timer::writeRegister currently drives OC from offset 0:
        // channel = data[7:0], state = data[8].
        if offset == 0 {
            let ch = (value & 0xFF) as usize;
            let state = value & (1 << 8) != 0;
            if let Some(Some(idx)) = self.oc_pins.get(ch) {
                if let Some(p) = pins.get_mut(*idx) {
                    p.set_pin_mode(PinMode::Output);
                    p.set_out_state(state);
                }
            }
            return;
        }
        match offset {
            0x00 => self.cr1 = value as u16 & 0x3FF,
            0x04 => self.cr2 = value as u16 & 0x00F8,
            0x08 => self.smcr = value as u16,
            0x0C => self.dier = value as u16 & 0x5F5F,
            0x10 => self.sr = value as u16,
            0x14 => self.egr = value as u16,
            0x18 => self.ccmr[0] = value as u16,
            0x1C => self.ccmr[1] = value as u16,
            0x20 => self.ccer = value as u16,
            0x28 => self.psc = value as u16,
            0x2C => self.arr = value as u16,
            0x30 => self.rcr = value as u16,
            0x34 => self.ccr[0] = value as u16,
            0x38 => self.ccr[1] = value as u16,
            0x3C => self.ccr[2] = value as u16,
            0x40 => self.ccr[3] = value as u16,
            _ => {}
        }
    }

    pub fn read(&self, offset: u64) -> u32 {
        match offset {
            0x00 => u32::from(self.cr1),
            0x04 => u32::from(self.cr2),
            0x08 => u32::from(self.smcr),
            0x0C => u32::from(self.dier),
            0x10 => u32::from(self.sr),
            0x14 => u32::from(self.egr),
            0x18 => u32::from(self.ccmr[0]),
            0x1C => u32::from(self.ccmr[1]),
            0x20 => u32::from(self.ccer),
            0x24 => 0, // CNT not tracked independently
            0x28 => u32::from(self.psc),
            0x2C => u32::from(self.arr),
            0x30 => u32::from(self.rcr),
            0x34 => u32::from(self.ccr[0]),
            0x38 => u32::from(self.ccr[1]),
            0x3C => u32::from(self.ccr[2]),
            0x40 => u32::from(self.ccr[3]),
            _ => 0,
        }
    }
}

pub const STM32_AFIO_START: u64 = 0x0001_0000;
pub const ESP32_IOMUX_START: u64 = 0x0004_9000;
pub const ESP32_IOMUX_SIZE: u64 = 0x1000;

/// STM32 Alternate Function I/O (AFIO) module.
#[derive(Clone, Debug)]
pub struct Stm32Afio {
    pub mem_start: u64,
    pub evcr: u32,
    pub mapr: u32,
    pub exticr: [u32; 4],
    pub mapr2: u32,
}

impl Stm32Afio {
    pub fn new(mem_start: u64) -> Self {
        Self {
            mem_start,
            evcr: 0,
            mapr: 0,
            exticr: [0; 4],
            mapr2: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.mem_start);
    }

    pub fn read(&self, offset: u64) -> u32 {
        match offset {
            0x00 => self.evcr,
            0x04 => self.mapr,
            0x08 => self.exticr[0],
            0x0C => self.exticr[1],
            0x10 => self.exticr[2],
            0x14 => self.exticr[3],
            0x1C => self.mapr2,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u64, val: u32) {
        match offset {
            0x00 => self.evcr = val,
            0x04 => self.mapr = val,
            0x08 => self.exticr[0] = val,
            0x0C => self.exticr[1] = val,
            0x10 => self.exticr[2] = val,
            0x14 => self.exticr[3] = val,
            0x1C => self.mapr2 = val,
            _ => {}
        }
    }

    pub fn spi1_remap(&self) -> bool {
        (self.mapr & 0x01) != 0
    }

    pub fn i2c1_remap(&self) -> bool {
        (self.mapr & (1 << 1)) != 0
    }

    pub fn usart1_remap(&self) -> bool {
        (self.mapr & (1 << 2)) != 0
    }

    pub fn usart2_remap(&self) -> bool {
        (self.mapr & (1 << 3)) != 0
    }

    pub fn usart3_remap(&self) -> u32 {
        (self.mapr >> 4) & 0x03
    }

    pub fn tim1_remap(&self) -> u32 {
        (self.mapr >> 6) & 0x03
    }

    pub fn tim2_remap(&self) -> u32 {
        (self.mapr >> 8) & 0x03
    }

    pub fn tim3_remap(&self) -> u32 {
        (self.mapr >> 10) & 0x03
    }

    pub fn tim4_remap(&self) -> bool {
        (self.mapr & (1 << 12)) != 0
    }
}

/// ESP32 IOMUX and GPIO Matrix routing module.
#[derive(Clone, Debug)]
pub struct Esp32IoMux {
    pub mem_start: u64,
    pub iomux_regs: [u32; 40],
    pub func_in_sel: [u32; 256],
    pub func_out_sel: [u32; 40],
}

impl Esp32IoMux {
    pub fn new(mem_start: u64) -> Self {
        Self {
            mem_start,
            iomux_regs: [0; 40],
            func_in_sel: [0; 256],
            func_out_sel: [0; 40],
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.mem_start);
    }

    pub fn read(&self, offset: u64) -> u32 {
        let index = (offset / 4) as usize;
        if index < self.iomux_regs.len() {
            self.iomux_regs[index]
        } else {
            0
        }
    }

    pub fn write(&mut self, offset: u64, val: u32) {
        let index = (offset / 4) as usize;
        if index < self.iomux_regs.len() {
            self.iomux_regs[index] = val;
        }
    }

    pub fn set_func_in(&mut self, func: usize, val: u32) {
        if func < self.func_in_sel.len() {
            self.func_in_sel[func] = val;
        }
    }

    pub fn set_func_out(&mut self, pin: usize, val: u32) {
        if pin < self.func_out_sel.len() {
            self.func_out_sel[pin] = val;
        }
    }
}
