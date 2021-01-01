//! Generic I²C / TWI engine (C++ `TwiModule`).

use super::PinMode;
use super::clock::{ClkState, Clocked};
use super::pin::IoPin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiMode {
    Off = 0,
    Master = 1,
    Slave = 2,
}

/// AVR TWI status codes (C++ `twiState_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TwiState {
    BusError = 0x00,
    Start = 0x08,
    RepStart = 0x10,
    MtxAdrAck = 0x18,
    MtxAdrNack = 0x20,
    MtxDataAck = 0x28,
    MtxDataNack = 0x30,
    ArbLost = 0x38,
    MrxAdrAck = 0x40,
    MrxAdrNack = 0x48,
    MrxDataAck = 0x50,
    MrxDataNack = 0x58,
    SrxAdrAck = 0x60,
    SrxGenAck = 0x70,
    SrxAdrDataAck = 0x80,
    SrxAdrDataNack = 0x88,
    SrxGenDataAck = 0x90,
    SrxGenDataNack = 0x98,
    StxAdrAck = 0xA8,
    StxDataAck = 0xB8,
    StxDataNack = 0xC0,
    NoState = 0xF8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum I2cState {
    Idle,
    Stop,
    Start,
    Read,
    Write,
    Ack,
    EndAck,
    ReadAck,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TwiPins {
    pub scl: Option<usize>,
    pub sda: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TwiTick {
    pub state: Option<TwiState>,
    pub byte_received: Option<u8>,
    pub byte_sent: Option<u8>,
    pub buffer_empty: bool,
    pub need_slave_write: bool,
    pub need_start_write: bool,
    pub stopped: bool,
}

#[derive(Clone, Debug)]
pub struct TwiModule {
    pub mode: TwiMode,
    pub twi_state: TwiState,
    pub address: u8,
    pub tx_reg: u8,
    pub rx_reg: u8,
    pub send_ack: bool,
    pub gen_call: bool,
    pub clock_period: u64,
    enabled: bool,
    i2c_state: I2cState,
    last_state: I2cState,
    next_state: TwiState,
    last_sda: bool,
    sda_state: bool,
    toggle_scl: bool,
    is_addr: bool,
    write: bool,
    master_ack: bool,
    addr_match: bool,
    bit_ptr: i32,
    addr_bits: i32,
    pub due: Option<u64>,
    clocked: Clocked,
}

impl Default for TwiModule {
    fn default() -> Self {
        Self::new()
    }
}

impl TwiModule {
    pub fn new() -> Self {
        let mut t = Self {
            mode: TwiMode::Off,
            twi_state: TwiState::NoState,
            address: 0,
            tx_reg: 0,
            rx_reg: 0,
            send_ack: false,
            gen_call: false,
            clock_period: 5_000_000, // 100 kHz half-period
            enabled: true,
            i2c_state: I2cState::Idle,
            last_state: I2cState::Idle,
            next_state: TwiState::NoState,
            last_sda: true,
            sda_state: true,
            toggle_scl: false,
            is_addr: false,
            write: false,
            master_ack: true,
            addr_match: false,
            bit_ptr: 0,
            addr_bits: 7,
            due: None,
            clocked: Clocked::default(),
        };
        t.set_freq_khz(100.0);
        t
    }

    pub fn set_freq_khz(&mut self, f: f64) {
        let freq = (f * 1e3).max(1.0);
        self.clock_period = (1e12 / freq / 2.0).round() as u64;
        if self.clock_period == 0 {
            self.clock_period = 1;
        }
    }

    pub fn set_address(&mut self, a: u8) {
        self.address = a;
    }

    pub fn set_slave(&mut self, addr: u8) {
        self.mode = TwiMode::Slave;
        self.set_address(addr);
    }

    pub fn status(&self) -> u8 {
        self.twi_state as u8
    }

    pub fn set_mode(&mut self, mode: TwiMode, now: u64, pins: &mut [IoPin], map: TwiPins) {
        if map.scl.is_none() || map.sda.is_none() {
            return;
        }
        if mode != TwiMode::Off {
            if let Some(i) = map.scl {
                if let Some(p) = pins.get_mut(i) {
                    p.set_pin_mode(PinMode::OpenCo);
                    p.set_pullup(1e5);
                    let _ = p.schedule_state(true, 10_000);
                }
            }
            self.set_sda(true, 0, pins, map);
        }
        self.mode = mode;
        self.i2c_state = I2cState::Idle;
        self.toggle_scl = false;
        self.due = None;
        let _ = now;
    }

    fn set_scl(&mut self, st: bool, pins: &mut [IoPin], map: TwiPins) {
        if let Some(i) = map.scl {
            if let Some(p) = pins.get_mut(i) {
                p.set_out_state(st);
            }
        }
    }

    fn set_sda(&mut self, st: bool, delay: u64, pins: &mut [IoPin], map: TwiPins) {
        if let Some(i) = map.sda {
            if let Some(p) = pins.get_mut(i) {
                if delay == 0 {
                    p.set_out_state(st);
                } else {
                    let _ = p.schedule_state(st, delay);
                    p.set_out_state(st);
                }
            }
        }
    }

    fn get_sda(&mut self, pins: &[IoPin], map: TwiPins) {
        self.sda_state = map
            .sda
            .and_then(|i| pins.get(i))
            .map(|p| p.last_inp_state())
            .unwrap_or(true);
    }

    pub fn master_start(&mut self, now: u64, pins: &mut [IoPin], map: TwiPins) {
        self.i2c_state = I2cState::Start;
        self.due = None;
        self.run_event(now, pins, map);
    }

    pub fn master_write(
        &mut self,
        data: u8,
        is_addr: bool,
        write: bool,
        _pins: &mut [IoPin],
        _map: TwiPins,
    ) {
        self.is_addr = is_addr;
        self.write = write;
        self.i2c_state = I2cState::Write;
        self.tx_reg = data;
        self.bit_ptr = 7;
    }

    pub fn master_read(&mut self, ack: bool, now: u64, pins: &mut [IoPin], map: TwiPins) {
        self.send_ack = ack;
        self.set_sda(true, 0, pins, map);
        self.bit_ptr = 0;
        self.rx_reg = 0;
        self.i2c_state = I2cState::Read;
        self.due = Some(now.saturating_add(self.clock_period.max(1)));
    }

    pub fn master_stop(&mut self, pins: &mut [IoPin], map: TwiPins) {
        self.i2c_state = I2cState::Stop;
        self.set_sda(false, 0, pins, map);
    }

    fn write_bit(&mut self, pins: &mut [IoPin], map: TwiPins) -> bool {
        if self.bit_ptr < 0 {
            self.set_sda(true, 0, pins, map);
            self.last_state = self.i2c_state;
            self.i2c_state = I2cState::ReadAck;
            return true;
        }
        let bit = (self.tx_reg >> self.bit_ptr) & 1 != 0;
        self.bit_ptr -= 1;
        if self.mode == TwiMode::Master {
            self.set_sda(bit, 0, pins, map);
        } else {
            self.set_sda(bit, 10_000, pins, map);
        }
        false
    }

    fn read_bit(&mut self) {
        if self.bit_ptr > 0 {
            self.rx_reg <<= 1;
        }
        self.rx_reg += u8::from(self.sda_state);
        self.bit_ptr += 1;
    }

    pub fn run_event(&mut self, now: u64, pins: &mut [IoPin], map: TwiPins) -> TwiTick {
        let mut tick = TwiTick::default();
        if self.mode != TwiMode::Master {
            return tick;
        }
        let clk_level = map.scl.and_then(|i| pins.get(i)).map(|p| p.get_out_state());
        self.clocked.update(clk_level);
        let clk_low = matches!(self.clocked.clk_state, ClkState::Low | ClkState::Falling);

        if self.toggle_scl {
            self.set_scl(clk_low, pins, map);
            self.toggle_scl = false;
            self.due = Some(now.saturating_add((self.clock_period / 2).max(1)));
            return tick;
        }
        self.get_sda(pins, map);

        match self.i2c_state {
            I2cState::Idle => {}
            I2cState::Stop => {
                if self.sda_state && clk_low {
                    self.set_sda(false, 0, pins, map);
                } else if !self.sda_state && clk_low {
                    self.set_scl(true, pins, map);
                } else if !self.sda_state && !clk_low {
                    self.set_sda(true, 0, pins, map);
                } else if self.sda_state && !clk_low {
                    self.twi_state = TwiState::NoState;
                    tick.state = Some(TwiState::NoState);
                    self.i2c_state = I2cState::Idle;
                    self.due = None;
                    return tick;
                }
            }
            I2cState::Start => {
                if clk_low {
                    self.set_scl(true, pins, map);
                } else if self.sda_state {
                    self.set_sda(false, 0, pins, map);
                } else if !clk_low {
                    self.set_scl(false, pins, map);
                    let is_rep = self.last_state == I2cState::Write;
                    self.i2c_state = I2cState::Idle;
                    let st = if is_rep {
                        TwiState::RepStart
                    } else {
                        TwiState::Start
                    };
                    self.twi_state = st;
                    tick.state = Some(st);
                }
            }
            I2cState::Read => {
                if !clk_low {
                    self.read_bit();
                    if self.bit_ptr == 8 {
                        tick.byte_received = Some(self.rx_reg);
                        self.bit_ptr = 0;
                        self.last_state = self.i2c_state;
                        self.i2c_state = I2cState::Ack;
                    }
                }
                self.toggle_scl = true;
            }
            I2cState::Write => {
                if clk_low {
                    if self.write_bit(pins, map) {
                        tick.buffer_empty = true;
                        tick.byte_sent = Some(self.tx_reg);
                    }
                }
                self.toggle_scl = true;
            }
            I2cState::Ack => {
                if self.master_ack {
                    if clk_low {
                        if self.send_ack {
                            self.set_sda(false, 0, pins, map);
                        }
                        self.i2c_state = I2cState::EndAck;
                    }
                    self.toggle_scl = true;
                }
            }
            I2cState::EndAck => {
                if clk_low {
                    self.set_sda(true, 0, pins, map);
                    let st = if self.send_ack {
                        TwiState::MrxDataAck
                    } else {
                        TwiState::MrxDataNack
                    };
                    self.i2c_state = I2cState::Idle;
                    self.twi_state = st;
                    tick.state = Some(st);
                } else {
                    self.toggle_scl = true;
                }
            }
            I2cState::ReadAck => {
                if clk_low {
                    let next = self.next_state;
                    self.i2c_state = I2cState::Idle;
                    self.twi_state = next;
                    tick.state = Some(next);
                } else {
                    if self.is_addr {
                        self.next_state = if self.write {
                            if self.sda_state {
                                TwiState::MtxAdrNack
                            } else {
                                TwiState::MtxAdrAck
                            }
                        } else if self.sda_state {
                            TwiState::MrxAdrNack
                        } else {
                            TwiState::MrxAdrAck
                        };
                    } else {
                        self.next_state = if self.sda_state {
                            TwiState::MtxDataNack
                        } else {
                            TwiState::MtxDataAck
                        };
                    }
                    self.toggle_scl = true;
                }
            }
        }
        if self.i2c_state != I2cState::Idle || self.toggle_scl {
            let time = if self.toggle_scl {
                (self.clock_period / 2).max(1)
            } else {
                self.clock_period.max(1)
            };
            self.due = Some(now.saturating_add(time));
        }
        tick
    }

    pub fn volt_changed_pins(&mut self, scl: &mut IoPin, sda: &mut IoPin) -> TwiTick {
        let mut tick = TwiTick::default();
        if self.mode != TwiMode::Slave {
            return tick;
        }
        let clk_level = Some(scl.last_inp_state());
        self.clocked.update(clk_level);
        self.sda_state = sda.last_inp_state();

        if self.clocked.clk_state == ClkState::High && self.i2c_state != I2cState::Ack {
            if self.last_sda && !self.sda_state {
                self.bit_ptr = 0;
                self.rx_reg = 0;
                self.i2c_state = I2cState::Start;
            } else if !self.last_sda && self.sda_state {
                self.i2c_state = I2cState::Stop;
                tick.stopped = true;
            }
        } else if self.clocked.clk_state == ClkState::Rising {
            if self.i2c_state == I2cState::Start {
                self.read_bit();
                if self.bit_ptr > self.addr_bits {
                    let rw = self.rx_reg % 2 != 0;
                    self.rx_reg >>= 1;
                    self.addr_match = self.rx_reg == self.address;
                    let general = self.gen_call && self.rx_reg == 0;
                    if (self.addr_match || general) && self.enabled {
                        self.send_ack = true;
                        if rw {
                            self.next_state = TwiState::StxAdrAck;
                            self.i2c_state = I2cState::Read;
                            tick.need_slave_write = true;
                            self.bit_ptr = 7;
                        } else {
                            self.next_state = if self.addr_match {
                                TwiState::SrxAdrAck
                            } else {
                                TwiState::SrxGenAck
                            };
                            self.i2c_state = I2cState::Write;
                            self.bit_ptr = 0;
                            tick.need_start_write = true;
                        }
                        self.last_state = self.i2c_state;
                        self.i2c_state = I2cState::Ack;
                    } else {
                        self.i2c_state = I2cState::Stop;
                        self.rx_reg = 0;
                    }
                }
            } else if self.i2c_state == I2cState::Write {
                self.read_bit();
                if self.bit_ptr == 8 {
                    tick.byte_received = Some(self.rx_reg);
                    self.bit_ptr = 0;
                    self.last_state = self.i2c_state;
                    self.i2c_state = I2cState::Ack;
                }
            } else if self.i2c_state == I2cState::ReadAck {
                self.twi_state = if self.sda_state {
                    TwiState::StxDataNack
                } else {
                    TwiState::StxDataAck
                };
                tick.state = Some(self.twi_state);
                if !self.sda_state {
                    self.i2c_state = self.last_state;
                    tick.need_slave_write = true;
                    self.bit_ptr = 7;
                } else {
                    self.i2c_state = I2cState::Idle;
                }
            } else if self.i2c_state == I2cState::EndAck {
                self.twi_state = self.next_state;
                tick.state = Some(self.twi_state);
            }
        } else if self.enabled && self.clocked.clk_state == ClkState::Falling {
            if self.i2c_state == I2cState::Ack {
                let st = !self.send_ack;
                let _ = sda.schedule_state(st, 10_000);
                sda.set_out_state(st);
                self.i2c_state = I2cState::EndAck;
            } else if self.i2c_state == I2cState::EndAck {
                self.i2c_state = self.last_state;
                if self.i2c_state != I2cState::Read {
                    let _ = sda.schedule_state(true, 10_000);
                    sda.set_out_state(true);
                }
                self.rx_reg = 0;
            }
            if self.i2c_state == I2cState::Read {
                if self.bit_ptr < 0 {
                    let _ = sda.schedule_state(true, 10_000);
                    sda.set_out_state(true);
                    self.last_state = self.i2c_state;
                    self.i2c_state = I2cState::ReadAck;
                } else {
                    let bit = (self.tx_reg >> self.bit_ptr) & 1 != 0;
                    self.bit_ptr -= 1;
                    let _ = sda.schedule_state(bit, 10_000);
                    sda.set_out_state(bit);
                }
            }
        }
        self.last_sda = self.sda_state;
        tick
    }

    pub fn volt_changed(&mut self, pins: &mut [IoPin], map: TwiPins) -> TwiTick {
        let mut tick = TwiTick::default();
        if self.mode != TwiMode::Slave {
            return tick;
        }
        let clk_level = map
            .scl
            .and_then(|i| pins.get(i))
            .map(|p| p.last_inp_state());
        self.clocked.update(clk_level);
        self.get_sda(pins, map);

        if self.clocked.clk_state == ClkState::High && self.i2c_state != I2cState::Ack {
            if self.last_sda && !self.sda_state {
                self.bit_ptr = 0;
                self.rx_reg = 0;
                self.i2c_state = I2cState::Start;
            } else if !self.last_sda && self.sda_state {
                self.i2c_state = I2cState::Stop;
                tick.stopped = true;
            }
        } else if self.clocked.clk_state == ClkState::Rising {
            if self.i2c_state == I2cState::Start {
                self.read_bit();
                if self.bit_ptr > self.addr_bits {
                    let rw = self.rx_reg % 2 != 0;
                    self.rx_reg >>= 1;
                    self.addr_match = self.rx_reg == self.address;
                    let general = self.gen_call && self.rx_reg == 0;
                    if (self.addr_match || general) && self.enabled {
                        self.send_ack = true;
                        if rw {
                            self.next_state = TwiState::StxAdrAck;
                            self.i2c_state = I2cState::Read;
                            tick.need_slave_write = true;
                            self.bit_ptr = 7;
                        } else {
                            self.next_state = if self.addr_match {
                                TwiState::SrxAdrAck
                            } else {
                                TwiState::SrxGenAck
                            };
                            self.i2c_state = I2cState::Write;
                            self.bit_ptr = 0;
                            tick.need_start_write = true;
                        }
                        self.last_state = self.i2c_state;
                        self.i2c_state = I2cState::Ack;
                    } else {
                        self.i2c_state = I2cState::Stop;
                        self.rx_reg = 0;
                    }
                }
            } else if self.i2c_state == I2cState::Write {
                self.read_bit();
                if self.bit_ptr == 8 {
                    tick.byte_received = Some(self.rx_reg);
                    self.bit_ptr = 0;
                    self.last_state = self.i2c_state;
                    self.i2c_state = I2cState::Ack;
                }
            } else if self.i2c_state == I2cState::ReadAck {
                self.twi_state = if self.sda_state {
                    TwiState::StxDataNack
                } else {
                    TwiState::StxDataAck
                };
                tick.state = Some(self.twi_state);
                if !self.sda_state {
                    self.i2c_state = self.last_state;
                    tick.need_slave_write = true;
                    self.bit_ptr = 7;
                } else {
                    self.i2c_state = I2cState::Idle;
                }
            } else if self.i2c_state == I2cState::EndAck {
                self.twi_state = self.next_state;
                tick.state = Some(self.twi_state);
            }
        } else if self.enabled && self.clocked.clk_state == ClkState::Falling {
            if self.i2c_state == I2cState::Ack {
                self.set_sda(!self.send_ack, 10_000, pins, map);
                self.i2c_state = I2cState::EndAck;
            } else if self.i2c_state == I2cState::EndAck {
                self.i2c_state = self.last_state;
                if self.i2c_state != I2cState::Read {
                    self.set_sda(true, 10_000, pins, map);
                }
                self.rx_reg = 0;
            }
            if self.i2c_state == I2cState::Read {
                let _ = self.write_bit(pins, map);
            }
        }
        self.last_sda = self.sda_state;
        tick
    }

    pub fn remain(&self, now: u64) -> Option<u64> {
        self.due.map(|t| t.saturating_sub(now).max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_start_reaches_start_state() {
        let mut pins = vec![IoPin::open_collector("SCL"), IoPin::open_collector("SDA")];
        pins[0].set_out_state(true);
        pins[1].set_out_state(true);
        pins[0].get_inp_state(5.0);
        pins[1].get_inp_state(5.0);
        let map = TwiPins {
            scl: Some(0),
            sda: Some(1),
        };
        let mut t = TwiModule::new();
        t.clock_period = 1_000;
        t.set_mode(TwiMode::Master, 0, &mut pins, map);
        t.master_start(0, &mut pins, map);
        let mut now = 0u64;
        let mut saw_start = false;
        for _ in 0..32 {
            for p in &mut pins {
                let v = if p.get_out_state() { 5.0 } else { 0.0 };
                p.get_inp_state(v);
            }
            if let Some(d) = t.remain(now) {
                now += d;
            } else {
                break;
            }
            let tick = t.run_event(now, &mut pins, map);
            if tick.state == Some(TwiState::Start) {
                saw_start = true;
                break;
            }
        }
        assert!(saw_start, "state={}", t.status());
    }
}
