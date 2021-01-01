//! C++ `McuTwi` / `AvrTwi` hardware TWI / I2C peripheral.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, TwiSpec};
use crate::interrupts::Interrupts;
use crate::port::Port;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiMode {
    Off = 0,
    Master = 1,
    Slave = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I2cState {
    Idle,
    Start,
    Write,
    Read,
    Ack,
    EndAck,
    ReadAck,
    Stop,
}

// AVR TWI Status codes
pub const TWI_BUS_ERROR: u8 = 0x00;
pub const TWI_START: u8 = 0x08;
pub const TWI_REP_START: u8 = 0x10;
pub const TWI_MTX_ADR_ACK: u8 = 0x18;
pub const TWI_MTX_ADR_NACK: u8 = 0x20;
pub const TWI_MTX_DATA_ACK: u8 = 0x28;
pub const TWI_MTX_DATA_NACK: u8 = 0x30;
pub const TWI_ARB_LOST: u8 = 0x38;
pub const TWI_MRX_ADR_ACK: u8 = 0x40;
pub const TWI_MRX_ADR_NACK: u8 = 0x48;
pub const TWI_MRX_DATA_ACK: u8 = 0x50;
pub const TWI_MRX_DATA_NACK: u8 = 0x58;
pub const TWI_SRX_ADR_ACK: u8 = 0x60;
pub const TWI_SRX_GEN_ACK: u8 = 0x70;
pub const TWI_SRX_ADR_DATA_ACK: u8 = 0x80;
pub const TWI_SRX_ADR_DATA_NACK: u8 = 0x88;
pub const TWI_SRX_GEN_DATA_ACK: u8 = 0x90;
pub const TWI_SRX_GEN_DATA_NACK: u8 = 0x98;
pub const TWI_SRX_STOP_RESTART: u8 = 0xA0;
pub const TWI_STX_ADR_ACK: u8 = 0xA8;
pub const TWI_STX_DATA_ACK: u8 = 0xB8;
pub const TWI_STX_DATA_NACK: u8 = 0xC0;
pub const TWI_NO_STATE: u8 = 0xF8;

#[derive(Clone, Debug)]
pub struct Twi {
    pub core: CoreKind,
    pub name: String,
    pub sda_pin: Option<String>,
    pub scl_pin: Option<String>,
    pub addr_reg_addr: Option<u16>,
    pub data_reg_addr: Option<u16>,
    pub stat_reg_addr: Option<u16>,
    pub twcr_addr: Option<u16>,
    pub twbr_addr: Option<u16>,
    // AVR bit positions in TWCR
    pub twen: RegBits,
    pub twwc: RegBits,
    pub twsto: RegBits,
    pub twsta: RegBits,
    pub twea: RegBits,
    pub twint: RegBits,
    pub twie: RegBits,
    pub interrupt: Option<usize>,
    pub prescalers: Vec<u32>,
    pub pr_index: usize,
    pub prescaler: u32,
    pub bit_rate: u8,
    pub freq_khz: f64,
    pub clock_period_ps: u64,
    pub twi_state: u8,
    pub mode: TwiMode,
    pub i2c_state: I2cState,
    pub last_state: I2cState,
    pub next_state: u8,
    pub tx_reg: u8,
    pub rx_reg: u8,
    pub address: u8,
    pub gen_call: bool,
    pub send_ack: bool,
    pub is_addr: bool,
    pub write: bool,
    pub bit_ptr: i32,
    pub sda_out: bool,
    pub scl_out: bool,
    pub sda_in: bool,
    pub scl_in: bool,
    pub last_sda_in: bool,
    pub toggle_scl: bool,
    pub enabled: bool,
    pub master_ack: bool,
    pub remain_ps: Option<u64>,
    /// Cached GPIO index for SDA pin: `(port_idx, pin_idx)`.
    sda_gpio: Option<(usize, usize)>,
    /// Cached GPIO index for SCL pin: `(port_idx, pin_idx)`.
    scl_gpio: Option<(usize, usize)>,
    pub tx_log: Vec<u8>,
    pub rx_log: Vec<u8>,
}

impl Twi {
    pub fn from_spec(
        spec: &TwiSpec,
        data: &DataSpace,
        core: CoreKind,
        interrupts: &Interrupts,
    ) -> Self {
        let mut prescalers = Vec::new();
        for s in spec.prescalers.split(',') {
            let s = s.trim();
            if let Ok(v) = s.parse::<u32>() {
                prescalers.push(v);
            }
        }
        if prescalers.is_empty() {
            prescalers = vec![1, 4, 16, 64];
        }

        let mut sda_pin = None;
        let mut scl_pin = None;
        if let Some(s) = spec.pins.first() {
            if !s.is_empty() {
                sda_pin = Some(s.clone());
            }
        }
        if let Some(s) = spec.pins.get(1) {
            if !s.is_empty() {
                scl_pin = Some(s.clone());
            }
        }

        let n = spec
            .name
            .chars()
            .last()
            .filter(|c| c.is_ascii_digit())
            .map(|c| c.to_string())
            .unwrap_or_default();

        let twcr_name = format!("TWCR{n}");
        let twbr_name = format!("TWBR{n}");
        let twsr_name = if !spec.status_reg.is_empty() {
            spec.status_reg.clone()
        } else {
            format!("TWSR{n}")
        };
        let twdr_name = if !spec.data_reg.is_empty() {
            spec.data_reg.clone()
        } else {
            format!("TWDR{n}")
        };
        let twar_name = if !spec.addr_reg.is_empty() {
            spec.addr_reg.clone()
        } else {
            format!("TWAR{n}")
        };

        let twcr_addr = data.reg_addr(&twcr_name);
        let twbr_addr = data.reg_addr(&twbr_name);
        let stat_reg_addr = data.reg_addr(&twsr_name);
        let data_reg_addr = data.reg_addr(&twdr_name);
        let addr_reg_addr = data.reg_addr(&twar_name);

        let twen = data.get_reg_bits(&format!("TWEN{n}"));
        let twwc = data.get_reg_bits(&format!("TWWC{n}"));
        let twsto = data.get_reg_bits(&format!("TWSTO{n}"));
        let twsta = data.get_reg_bits(&format!("TWSTA{n}"));
        let twea = data.get_reg_bits(&format!("TWEA{n}"));
        let twint = data.get_reg_bits(&format!("TWINT{n}"));
        let twie = data.get_reg_bits(&format!("TWIE{n}"));

        let interrupt = interrupts.get(&spec.interrupt);

        let mut twi = Self {
            core,
            name: spec.name.clone(),
            sda_pin,
            scl_pin,
            addr_reg_addr,
            data_reg_addr,
            stat_reg_addr,
            twcr_addr,
            twbr_addr,
            twen,
            twwc,
            twsto,
            twsta,
            twea,
            twint,
            twie,
            interrupt,
            prescalers,
            pr_index: 0,
            prescaler: 1,
            bit_rate: 0,
            freq_khz: 100.0,
            clock_period_ps: 5_000_000, // default 100 kHz half-period = 5 us = 5,000,000 ps
            twi_state: TWI_NO_STATE,
            mode: TwiMode::Off,
            i2c_state: I2cState::Idle,
            last_state: I2cState::Idle,
            next_state: TWI_NO_STATE,
            tx_reg: 0,
            rx_reg: 0,
            address: 0,
            gen_call: false,
            send_ack: false,
            is_addr: false,
            write: false,
            bit_ptr: 0,
            sda_out: true,
            scl_out: true,
            sda_in: true,
            scl_in: true,
            last_sda_in: true,
            toggle_scl: false,
            enabled: true,
            master_ack: true,
            remain_ps: None,
            sda_gpio: None,
            scl_gpio: None,
            tx_log: Vec::new(),
            rx_log: Vec::new(),
        };
        twi.set_freq_khz(100.0);
        twi
    }

    pub fn set_freq_khz(&mut self, f: f64) {
        let freq = (f * 1e3).max(1.0);
        self.freq_khz = f;
        self.clock_period_ps = (1e12 / freq / 2.0).round() as u64;
        if self.clock_period_ps == 0 {
            self.clock_period_ps = 1;
        }
    }

    pub fn update_freq(&mut self, mcu_freq_hz: f64) {
        if mcu_freq_hz > 0.0 {
            let denom = 16.0 + 2.0 * (self.bit_rate as f64) * (self.prescaler as f64);
            let freq = mcu_freq_hz / denom.max(1.0);
            self.set_freq_khz(freq / 1e3);
        }
    }

    pub fn configure_a(
        &mut self,
        old_twcr: u8,
        new_twcr: u8,
        data: &mut DataSpace,
        ports: &mut [Port],
        interrupts: &mut Interrupts,
        mcu_freq_hz: f64,
    ) {
        let twen_mask = if self.twen.mask != 0 {
            self.twen.mask
        } else {
            0x04
        };
        let twsta_mask = if self.twsta.mask != 0 {
            self.twsta.mask
        } else {
            0x20
        };
        let twsto_mask = if self.twsto.mask != 0 {
            self.twsto.mask
        } else {
            0x10
        };
        let twint_mask = if self.twint.mask != 0 {
            self.twint.mask
        } else {
            0x80
        };
        let twea_mask = if self.twea.mask != 0 {
            self.twea.mask
        } else {
            0x40
        };

        let old_en = old_twcr & twen_mask != 0;
        let new_en = new_twcr & twen_mask != 0;

        if old_en && !new_en {
            // Disable TWI
            self.mode = TwiMode::Off;
            self.release_pins(ports);
        }
        if !new_en {
            return;
        }

        if !old_en {
            // Enable TWI
            self.acquire_pins(ports);
            self.update_freq(mcu_freq_hz);
        }

        let old_start = old_twcr & twsta_mask != 0;
        let new_start = new_twcr & twsta_mask != 0;
        if new_start && !old_start {
            if self.mode != TwiMode::Master {
                self.mode = TwiMode::Master;
            }
            self.master_start(ports);
        }

        let old_stop = old_twcr & twsto_mask != 0;
        let new_stop = new_twcr & twsto_mask != 0;
        if new_stop && !old_stop {
            if self.mode == TwiMode::Master {
                if !new_start && self.twi_state < TWI_NO_STATE {
                    self.master_stop(ports, data, interrupts);
                }
            } else {
                self.mode = TwiMode::Slave;
            }
        }

        let clear_twint = new_twcr & twint_mask != 0;
        let twea = new_twcr & twea_mask != 0;
        if !new_stop && !new_start && !clear_twint {
            if self.mode != TwiMode::Slave {
                self.mode = TwiMode::Slave;
            }
            self.enabled = twea;
        }

        let is_data = clear_twint && !new_stop && !new_start;
        if !is_data {
            return;
        }

        if self.mode == TwiMode::Master {
            if self.twi_state == TWI_MRX_ADR_ACK || self.twi_state == TWI_MRX_DATA_ACK {
                self.master_read(twea, ports);
            }
        }
    }

    pub fn configure_b(&mut self, val: u8, mcu_freq_hz: f64) {
        if self.bit_rate != val {
            self.bit_rate = val;
            self.update_freq(mcu_freq_hz);
        }
    }

    pub fn write_status(&mut self, val: u8, mcu_freq_hz: f64) {
        let pr = (val & 0x03) as usize;
        if self.pr_index != pr {
            self.pr_index = pr;
            self.prescaler = self.prescalers.get(pr).copied().unwrap_or(1);
            self.update_freq(mcu_freq_hz);
        }
    }

    pub fn write_addr_reg(&mut self, val: u8) {
        self.gen_call = (val & 1) != 0;
        self.address = val >> 1;
    }

    pub fn write_twi_reg(&mut self, new_twdr: u8, data: &mut DataSpace, ports: &mut [Port]) {
        if self.mode == TwiMode::Slave {
            self.tx_reg = new_twdr;
        }
        if self.mode != TwiMode::Master {
            return;
        }

        let twcr = self.twcr_addr.map(|a| data.get(a)).unwrap_or(0);
        let twint_mask = if self.twint.mask != 0 {
            self.twint.mask
        } else {
            0x80
        };
        let twwc_mask = if self.twwc.mask != 0 {
            self.twwc.mask
        } else {
            0x08
        };

        let twint = twcr & twint_mask != 0;
        if twint {
            // Clear Write Collision bit TWWC
            if let Some(a) = self.twcr_addr {
                let v = data.get(a) & !twwc_mask;
                data.set(a, v);
            }
        } else {
            // Set Write Collision bit TWWC
            if let Some(a) = self.twcr_addr {
                let v = data.get(a) | twwc_mask;
                data.set(a, v);
            }
            return;
        }

        let is_addr = self.twi_state == TWI_START || self.twi_state == TWI_REP_START;
        let write = if is_addr { (new_twdr & 1) == 0 } else { true };

        self.master_write(new_twdr, is_addr, write, ports);
    }

    pub fn acquire_pins(&mut self, ports: &mut [Port]) {
        // Cache GPIO indices once so hot-path methods use direct O(1) indexing.
        self.sda_gpio = self
            .sda_pin
            .as_ref()
            .and_then(|n| crate::port::find_gpio(ports, n));
        self.scl_gpio = self
            .scl_pin
            .as_ref()
            .and_then(|n| crate::port::find_gpio(ports, n));
        self.set_pin_open_coll(&self.sda_pin, true, ports);
        self.set_pin_open_coll(&self.scl_pin, true, ports);
        self.set_sda(true, ports);
        self.set_scl(true, ports);
    }

    pub fn release_pins(&mut self, ports: &mut [Port]) {
        self.remain_ps = None;
        if let Some((pi, _)) = self.sda_gpio {
            if let Some(port) = ports.get_mut(pi) {
                port.dirty = true;
            }
        }
        if let Some((pi, _)) = self.scl_gpio {
            if let Some(port) = ports.get_mut(pi) {
                port.dirty = true;
            }
        }
        self.sda_gpio = None;
        self.scl_gpio = None;
    }

    fn set_pin_open_coll(&self, pin_name: &Option<String>, open_coll: bool, ports: &mut [Port]) {
        if let Some(name) = pin_name {
            if let Some((pi, pin_i)) = crate::port::find_gpio(ports, name) {
                if let Some(p) = ports.get_mut(pi).and_then(|p| p.pins.get_mut(pin_i)) {
                    p.open_coll = open_coll;
                    p.pullup = true;
                    ports[pi].dirty = true;
                }
            }
        }
    }

    fn set_sda(&mut self, st: bool, ports: &mut [Port]) {
        self.sda_out = st;
        if let Some((pi, pin_i)) = self.sda_gpio {
            ports[pi].dirty = true;
            if let Some(p) = ports.get_mut(pi).and_then(|p| p.pins.get_mut(pin_i)) {
                p.out_state = st;
                p.is_out = true;
            }
        }
    }

    fn set_scl(&mut self, st: bool, ports: &mut [Port]) {
        self.scl_out = st;
        if let Some((pi, pin_i)) = self.scl_gpio {
            ports[pi].dirty = true;
            if let Some(p) = ports.get_mut(pi).and_then(|p| p.pins.get_mut(pin_i)) {
                p.out_state = st;
                p.is_out = true;
            }
        }
    }

    fn sample_inputs(&mut self, ports: &[Port]) {
        if let Some((pi, pin_i)) = self.sda_gpio {
            if let Some(p) = ports.get(pi).and_then(|p| p.pins.get(pin_i)) {
                self.sda_in = p.inp_state;
            }
        }
        if let Some((pi, pin_i)) = self.scl_gpio {
            if let Some(p) = ports.get(pi).and_then(|p| p.pins.get(pin_i)) {
                self.scl_in = p.inp_state;
            }
        }
    }

    pub fn master_start(&mut self, ports: &mut [Port]) {
        self.i2c_state = I2cState::Start;
        self.remain_ps = Some(1);
        self.run_master_step(ports);
    }

    pub fn master_write(&mut self, data: u8, is_addr: bool, write: bool, _ports: &mut [Port]) {
        self.is_addr = is_addr;
        self.write = write;
        self.i2c_state = I2cState::Write;
        self.tx_reg = data;
        self.bit_ptr = 7;
        self.remain_ps = Some(self.clock_period_ps.max(1));
    }

    pub fn master_read(&mut self, ack: bool, ports: &mut [Port]) {
        self.send_ack = ack;
        self.set_sda(true, ports);
        self.bit_ptr = 0;
        self.rx_reg = 0;
        self.i2c_state = I2cState::Read;
        self.remain_ps = Some(self.clock_period_ps.max(1));
    }

    pub fn master_stop(
        &mut self,
        ports: &mut [Port],
        _data: &mut DataSpace,
        _interrupts: &mut Interrupts,
    ) {
        self.i2c_state = I2cState::Stop;
        self.set_sda(false, ports);
        self.remain_ps = Some(self.clock_period_ps.max(1));
    }

    fn write_bit(&mut self, ports: &mut [Port]) -> bool {
        if self.bit_ptr < 0 {
            self.set_sda(true, ports);
            self.last_state = self.i2c_state;
            self.i2c_state = I2cState::ReadAck;
            return true;
        }
        let bit = (self.tx_reg >> self.bit_ptr) & 1 != 0;
        self.bit_ptr -= 1;
        self.set_sda(bit, ports);
        false
    }

    fn read_bit(&mut self) {
        if self.bit_ptr > 0 {
            self.rx_reg <<= 1;
        }
        self.rx_reg |= u8::from(self.sda_in);
        self.bit_ptr += 1;
    }

    pub fn step_time(
        &mut self,
        ps: u64,
        ports: &mut [Port],
        data: &mut DataSpace,
        interrupts: &mut Interrupts,
    ) -> bool {
        if let Some(rem) = self.remain_ps {
            if ps >= rem {
                self.remain_ps = None;
                self.run_event(ports, data, interrupts);
                true
            } else {
                self.remain_ps = Some(rem - ps);
                false
            }
        } else {
            false
        }
    }

    fn set_twi_state(&mut self, state: u8, data: &mut DataSpace, interrupts: &mut Interrupts) {
        self.twi_state = state;

        // Status register
        if let Some(a) = self.stat_reg_addr {
            let old = data.get(a);
            data.set(a, (old & 0x07) | (state & 0xF8));
        }

        if state == TWI_NO_STATE && self.i2c_state == I2cState::Idle {
            // Clear TWSTO bit
            if let Some(a) = self.twcr_addr {
                let twsto_mask = if self.twsto.mask != 0 {
                    self.twsto.mask
                } else {
                    0x10
                };
                let v = data.get(a) & !twsto_mask;
                data.set(a, v);
            }
        } else {
            // Set TWINT bit
            if let Some(a) = self.twcr_addr {
                let twint_mask = if self.twint.mask != 0 {
                    self.twint.mask
                } else {
                    0x80
                };
                let v = data.get(a) | twint_mask;
                data.set(a, v);

                let twie_mask = if self.twie.mask != 0 {
                    self.twie.mask
                } else {
                    0x01
                };
                if (v & twie_mask) != 0 {
                    if let Some(i) = self.interrupt {
                        interrupts.raise(i, data);
                    }
                }
            }

            if self.mode == TwiMode::Master {
                if state == TWI_MRX_DATA_ACK || state == TWI_MRX_DATA_NACK {
                    if let Some(a) = self.data_reg_addr {
                        data.set(a, self.rx_reg);
                    }
                }
            } else if state == TWI_SRX_ADR_DATA_ACK
                || state == TWI_SRX_ADR_DATA_NACK
                || state == TWI_SRX_GEN_DATA_ACK
                || state == TWI_SRX_GEN_DATA_NACK
            {
                if let Some(a) = self.data_reg_addr {
                    data.set(a, self.rx_reg);
                }
            }
        }
    }

    pub fn run_event(
        &mut self,
        ports: &mut [Port],
        data: &mut DataSpace,
        interrupts: &mut Interrupts,
    ) {
        if self.mode != TwiMode::Master {
            return;
        }
        self.sample_inputs(ports);

        let clk_low = !self.scl_out;

        if self.toggle_scl {
            self.set_scl(clk_low, ports);
            self.toggle_scl = false;
            self.remain_ps = Some((self.clock_period_ps / 2).max(1));
            return;
        }

        match self.i2c_state {
            I2cState::Idle => {}
            I2cState::Stop => {
                if self.sda_out && clk_low {
                    self.set_sda(false, ports);
                } else if !self.sda_out && clk_low {
                    self.set_scl(true, ports);
                } else if !self.sda_out && !clk_low {
                    self.set_sda(true, ports);
                } else if self.sda_out && !clk_low {
                    self.i2c_state = I2cState::Idle;
                    self.set_twi_state(TWI_NO_STATE, data, interrupts);
                    self.remain_ps = None;
                    return;
                }
            }
            I2cState::Start => {
                if clk_low {
                    self.set_scl(true, ports);
                } else if self.sda_out {
                    self.set_sda(false, ports);
                } else if !clk_low {
                    self.set_scl(false, ports);
                    let is_rep = self.last_state == I2cState::Write;
                    self.i2c_state = I2cState::Idle;
                    let st = if is_rep { TWI_REP_START } else { TWI_START };
                    self.set_twi_state(st, data, interrupts);
                }
            }
            I2cState::Read => {
                if !clk_low {
                    self.read_bit();
                    if self.bit_ptr == 8 {
                        self.rx_log.push(self.rx_reg);
                        self.bit_ptr = 0;
                        self.last_state = self.i2c_state;
                        self.i2c_state = I2cState::Ack;
                    }
                }
                self.toggle_scl = true;
            }
            I2cState::Write => {
                if clk_low {
                    if self.write_bit(ports) {
                        self.tx_log.push(self.tx_reg);
                    }
                }
                self.toggle_scl = true;
            }
            I2cState::Ack => {
                if self.master_ack {
                    if clk_low {
                        if self.send_ack {
                            self.set_sda(false, ports);
                        }
                        self.i2c_state = I2cState::EndAck;
                    }
                    self.toggle_scl = true;
                }
            }
            I2cState::EndAck => {
                if clk_low {
                    self.set_sda(true, ports);
                    let st = if self.send_ack {
                        TWI_MRX_DATA_ACK
                    } else {
                        TWI_MRX_DATA_NACK
                    };
                    self.i2c_state = I2cState::Idle;
                    self.set_twi_state(st, data, interrupts);
                } else {
                    self.toggle_scl = true;
                }
            }
            I2cState::ReadAck => {
                if clk_low {
                    let next = self.next_state;
                    self.i2c_state = I2cState::Idle;
                    self.set_twi_state(next, data, interrupts);
                } else {
                    if self.is_addr {
                        self.next_state = if self.write {
                            if self.sda_in {
                                TWI_MTX_ADR_NACK
                            } else {
                                TWI_MTX_ADR_ACK
                            }
                        } else if self.sda_in {
                            TWI_MRX_ADR_NACK
                        } else {
                            TWI_MRX_ADR_ACK
                        };
                    } else {
                        self.next_state = if self.sda_in {
                            TWI_MTX_DATA_NACK
                        } else {
                            TWI_MTX_DATA_ACK
                        };
                    }
                    self.toggle_scl = true;
                }
            }
        }

        if self.i2c_state != I2cState::Idle || self.toggle_scl {
            let time = if self.toggle_scl {
                (self.clock_period_ps / 2).max(1)
            } else {
                self.clock_period_ps.max(1)
            };
            self.remain_ps = Some(time);
        }
    }

    fn run_master_step(&mut self, ports: &mut [Port]) {
        self.sample_inputs(ports);
        let clk_low = !self.scl_out;
        if self.i2c_state == I2cState::Start {
            if clk_low {
                self.set_scl(true, ports);
                self.remain_ps = Some((self.clock_period_ps / 2).max(1));
            } else if self.sda_in {
                self.set_sda(false, ports);
                self.remain_ps = Some((self.clock_period_ps / 2).max(1));
            }
        }
    }
}
