//! Response state for the Phase 5 sensors.
//!
//! Timed edges return an absolute picosecond stamp. The caller puts that on
//! the device event queue. A device has one live event, so `replace` cancels
//! the previous deadline first (WS2812 reset). Chained edges run from the
//! event handler, after the live event has already been popped.

use super::pin::IoPin;
use super::twi::{TwiModule, TwiTick};

const US: u64 = 1_000_000;

#[derive(Clone, Copy, Debug)]
pub struct Drive {
    pub changed: bool,
    pub arm_at: Option<u64>,
    pub replace: bool,
}

impl Drive {
    pub const QUIET: Self = Self {
        changed: false,
        arm_at: None,
        replace: false,
    };

    pub(crate) fn stamp(changed: bool) -> Self {
        Self {
            changed,
            arm_at: None,
            replace: false,
        }
    }

    pub(crate) fn arm(at: u64, changed: bool, replace: bool) -> Self {
        Self {
            changed,
            arm_at: Some(at),
            replace,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Sr04Echo {
    pub echo_until: u64,
}

impl Sr04Echo {
    pub fn on_pins(
        &mut self,
        trigger_high: &mut bool,
        echo_high: &mut bool,
        distance: f64,
        use_slider: bool,
        vcc: f64,
        gnd: f64,
        trig: f64,
        inp: f64,
        t: u64,
    ) -> Drive {
        let powered = (vcc - gnd) > 2.5;
        if !powered {
            let changed = *echo_high;
            *echo_high = false;
            *trigger_high = false;
            self.echo_until = 0;
            return Drive::stamp(changed);
        }
        let trig_high = (trig - gnd) > 2.0;
        let mut drive = Drive::QUIET;
        let rising = trig_high && !*trigger_high;
        if rising && self.echo_until == 0 {
            let meters = if use_slider {
                distance
            } else {
                inp.clamp(0.02, 4.0)
            };
            let ps = (2.0 * meters / 343.0 * 1e12).round().max(1.0) as u64;
            *echo_high = true;
            self.echo_until = t.saturating_add(ps);
            drive = Drive::arm(self.echo_until, true, false);
        }
        *trigger_high = trig_high;
        drive
    }

    pub fn on_event(&mut self, echo_high: &mut bool, t: u64) -> Drive {
        if *echo_high && self.echo_until > 0 && t >= self.echo_until {
            *echo_high = false;
            self.echo_until = 0;
            Drive::stamp(true)
        } else {
            Drive::QUIET
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DhtLink {
    pub phase: u8,
    pub bit_i: u8,
    pub low_since: u64,
    pub frame: u64,
}

impl DhtLink {
    pub fn on_pins(
        &mut self,
        model: &str,
        temp: f64,
        humi: f64,
        out_state: &mut bool,
        pin_driven: &mut bool,
        line: f64,
        t: u64,
    ) -> Drive {
        if *pin_driven {
            return Drive::QUIET;
        }
        let high = line > 2.5;
        if !high {
            if self.phase == 0 {
                self.phase = 1;
                self.low_since = t;
            }
            return Drive::QUIET;
        }
        if self.phase == 1 && t.saturating_sub(self.low_since) >= 1_000 * US {
            self.frame = dht_frame(model, temp, humi);
            self.bit_i = 0;
            self.phase = 2;
            *pin_driven = true;
            *out_state = false;
            return Drive::arm(t.saturating_add(80 * US), true, false);
        }
        self.phase = 0;
        Drive::QUIET
    }

    pub fn on_event(&mut self, out_state: &mut bool, pin_driven: &mut bool, t: u64) -> Drive {
        match self.phase {
            2 => {
                *out_state = true;
                self.phase = 3;
                Drive::arm(t.saturating_add(80 * US), true, false)
            }
            3 | 5 => {
                if self.bit_i >= 40 {
                    *pin_driven = false;
                    *out_state = true;
                    self.phase = 0;
                    return Drive::stamp(true);
                }
                *out_state = false;
                self.phase = 4;
                Drive::arm(t.saturating_add(50 * US), true, false)
            }
            4 => {
                let one = (self.frame >> (39 - u64::from(self.bit_i))) & 1 == 1;
                *out_state = true;
                self.phase = 5;
                self.bit_i = self.bit_i.saturating_add(1);
                let high = if one { 70 * US } else { 28 * US };
                Drive::arm(t.saturating_add(high), true, false)
            }
            _ => Drive::QUIET,
        }
    }
}

pub fn dht_frame(model: &str, temp: f64, humi: f64) -> u64 {
    let dht11 = model.contains("11");
    let (b0, b1, b2, b3) = if dht11 {
        let h = humi.round().clamp(0.0, 255.0) as u8;
        let tp = temp.round().clamp(0.0, 255.0) as u8;
        (h, 0, tp, 0)
    } else {
        let h = (humi * 10.0).round().clamp(0.0, 65535.0) as u16;
        let tp = (temp * 10.0).round().clamp(-32768.0, 32767.0) as i16 as u16;
        (
            (h >> 8) as u8,
            (h & 0xFF) as u8,
            (tp >> 8) as u8,
            (tp & 0xFF) as u8,
        )
    };
    let sum = b0.wrapping_add(b1).wrapping_add(b2).wrapping_add(b3);
    let mut frame = 0u64;
    for byte in [b0, b1, b2, b3, sum] {
        frame = (frame << 8) | u64::from(byte);
    }
    frame
}

#[derive(Clone, Debug)]
pub struct OwLink {
    pub phase: u8,
    pub low_since: u64,
    pub saw_low: bool,
    pub bit_n: u8,
    pub shifter: u8,
    pub cmd_n: u8,
    pub tx_i: u16,
    pub scratch: [u8; 9],
}

impl Default for OwLink {
    fn default() -> Self {
        Self {
            phase: 0,
            low_since: 0,
            saw_low: false,
            bit_n: 0,
            shifter: 0,
            cmd_n: 0,
            tx_i: 0,
            scratch: [0; 9],
        }
    }
}

impl OwLink {
    pub fn on_pins(&mut self, temp: f64, dq_low: &mut bool, line: f64, t: u64) -> Drive {
        if *dq_low {
            self.saw_low = true;
            return Drive::QUIET;
        }
        let low = line < 0.8;
        let falling = low && !self.saw_low;
        let rising = !low && self.saw_low;
        self.saw_low = low;
        if falling {
            self.low_since = t;
            if self.phase == 5 {
                return self.on_read_slot(dq_low, t);
            }
            return Drive::QUIET;
        }
        if !rising || self.low_since == 0 {
            return Drive::QUIET;
        }
        let dur = t.saturating_sub(self.low_since);
        self.low_since = 0;
        if dur >= 480 * US {
            self.phase = 2;
            self.cmd_n = 0;
            self.bit_n = 0;
            self.shifter = 0;
            self.tx_i = 0;
            return Drive::arm(t.saturating_add(30 * US), false, true);
        }
        if self.phase == 4 {
            let one = dur < 15 * US;
            self.push_bit(temp, one, dq_low, t)
        } else {
            Drive::QUIET
        }
    }

    pub fn on_event(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        match self.phase {
            2 => {
                *dq_low = true;
                self.phase = 3;
                self.saw_low = true;
                Drive::arm(t.saturating_add(100 * US), true, false)
            }
            3 => {
                *dq_low = false;
                self.phase = 4;
                self.saw_low = false;
                Drive::stamp(true)
            }
            5 => {
                *dq_low = false;
                self.saw_low = false;
                Drive::stamp(true)
            }
            _ => Drive::QUIET,
        }
    }

    fn push_bit(&mut self, temp: f64, one: bool, dq_low: &mut bool, t: u64) -> Drive {
        if one {
            self.shifter |= 1 << self.bit_n;
        }
        self.bit_n = self.bit_n.saturating_add(1);
        if self.bit_n < 8 {
            return Drive::QUIET;
        }
        let byte = self.shifter;
        self.shifter = 0;
        self.bit_n = 0;
        let cmd = self.cmd_n;
        self.cmd_n = 0;
        let read = if cmd == 0 {
            if byte == 0xCC {
                self.cmd_n = 1;
                false
            } else {
                byte == 0xBE
            }
        } else {
            byte == 0xBE
        };
        if !read {
            return Drive::QUIET;
        }
        self.scratch = ow_scratch(temp);
        self.phase = 5;
        self.tx_i = 0;
        let _ = (dq_low, t);
        Drive::QUIET
    }

    pub fn on_read_slot(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        if self.phase != 5 {
            return Drive::QUIET;
        }
        let bit_one = self.scratch_bit(self.tx_i);
        self.tx_i = self.tx_i.saturating_add(1);
        if self.tx_i >= 72 {
            self.phase = 4;
        }
        if bit_one {
            let changed = *dq_low;
            *dq_low = false;
            Drive::stamp(changed)
        } else {
            *dq_low = true;
            self.saw_low = true;
            Drive::arm(t.saturating_add(40 * US), true, true)
        }
    }

    fn scratch_bit(&self, index: u16) -> bool {
        let byte = self
            .scratch
            .get(index as usize / 8)
            .copied()
            .unwrap_or(0xFF);
        (byte >> (index % 8)) & 1 == 1
    }
}

pub fn ow_scratch(temp: f64) -> [u8; 9] {
    let raw = (temp / 0.0625).round().clamp(-32768.0, 32767.0) as i16;
    let mut scratch = [0u8; 9];
    scratch[0] = raw as u8;
    scratch[1] = (raw >> 8) as u8;
    scratch[2] = 0x4B;
    scratch[3] = 0x46;
    scratch[4] = 0x7F;
    scratch[5] = 0xFF;
    scratch[6] = 0x0C;
    scratch[7] = 0x10;
    scratch[8] = crc8_maxim(&scratch[..8]);
    scratch
}

pub fn crc8_maxim(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in data {
        let mut mix = byte;
        for _ in 0..8 {
            let lsb = (crc ^ mix) & 1;
            crc >>= 1;
            if lsb != 0 {
                crc ^= 0x8C;
            }
            mix >>= 1;
        }
    }
    crc
}

#[derive(Clone, Debug)]
pub struct I2cWire {
    pub scl: IoPin,
    pub sda: IoPin,
    pub twi: TwiModule,
}

impl I2cWire {
    pub fn new(addr: u8) -> Self {
        let mut scl = IoPin::open_collector("scl");
        scl.set_out_state(true);
        scl.set_pullup(1e5);
        scl.set_thresholds(2.5, 2.5);
        let mut sda = IoPin::open_collector("sda");
        sda.set_out_state(true);
        sda.set_pullup(1e5);
        sda.set_thresholds(2.5, 2.5);
        let mut twi = TwiModule::new();
        twi.set_slave(addr);
        Self { scl, sda, twi }
    }

    pub fn poll(&mut self, v_scl: f64, v_sda: f64) -> (TwiTick, bool) {
        self.scl.get_inp_state(v_scl);
        self.sda.get_inp_state(v_sda);
        let tick = self.twi.volt_changed_pins(&mut self.scl, &mut self.sda);
        let low = !self.sda.get_out_state();
        (tick, low)
    }
}

#[derive(Clone, Debug)]
pub struct Ds1621Bus {
    pub wire: I2cWire,
    pub pointer: u8,
    pub have_ptr: bool,
}

impl Default for Ds1621Bus {
    fn default() -> Self {
        Self {
            wire: I2cWire::new(0x48),
            pointer: 0xAA,
            have_ptr: false,
        }
    }
}

impl Ds1621Bus {
    pub fn on_pins(
        &mut self,
        temp: f64,
        th: &mut f64,
        tl: &mut f64,
        tout_high: &mut bool,
        sda_low: &mut bool,
        v_scl: f64,
        v_sda: f64,
        a2: bool,
        a1: bool,
        a0: bool,
    ) -> Drive {
        let mut changed = false;
        let next_tout = if temp >= *th {
            true
        } else if temp <= *tl {
            false
        } else {
            *tout_high
        };
        if *tout_high != next_tout {
            *tout_high = next_tout;
            changed = true;
        }
        let addr = 0x48 | (u8::from(a2) << 2) | (u8::from(a1) << 1) | u8::from(a0);
        if self.wire.twi.address != addr {
            self.wire.twi.set_slave(addr);
        }
        let (tick, low) = self.wire.poll(v_scl, v_sda);
        if *sda_low != low {
            *sda_low = low;
            changed = true;
        }
        if tick.need_start_write || tick.stopped {
            self.have_ptr = false;
        }
        if let Some(byte) = tick.byte_received {
            if !self.have_ptr {
                self.pointer = byte;
                self.have_ptr = true;
            } else {
                match self.pointer {
                    0xA1 => *th = f64::from(byte as i8),
                    0xA2 => *tl = f64::from(byte as i8),
                    _ => {}
                }
                if self.pointer == 0xA1 || self.pointer == 0xA2 {
                    let next = if temp >= *th {
                        true
                    } else if temp <= *tl {
                        false
                    } else {
                        *tout_high
                    };
                    if *tout_high != next {
                        *tout_high = next;
                        changed = true;
                    }
                }
            }
        }
        if tick.need_slave_write {
            self.wire.twi.tx_reg = ds1621_byte(temp, *th, *tl, self.pointer);
            if self.pointer == 0xAA {
                self.pointer = 0xAB;
            }
        }
        Drive::stamp(changed)
    }
}

fn ds1621_byte(temp: f64, th: f64, tl: f64, pointer: u8) -> u8 {
    match pointer {
        0xAA => temp.trunc().clamp(-128.0, 127.0) as i16 as u8,
        0xAB => u8::from((temp - temp.trunc()).abs() >= 0.5).saturating_mul(0x80),
        0xA1 => th.clamp(-128.0, 127.0) as i16 as u8,
        0xA2 => tl.clamp(-128.0, 127.0) as i16 as u8,
        _ => 0,
    }
}

#[derive(Clone, Debug)]
pub struct Ds1307Bus {
    pub wire: I2cWire,
    pub pointer: u8,
    pub have_ptr: bool,
    pub seeded: bool,
    pub armed: bool,
    pub regs: [u8; 8],
}

impl Default for Ds1307Bus {
    fn default() -> Self {
        Self {
            wire: I2cWire::new(0x68),
            pointer: 0,
            have_ptr: false,
            seeded: false,
            armed: false,
            regs: [0; 8],
        }
    }
}

impl Ds1307Bus {
    pub fn on_pins(
        &mut self,
        time_updated: bool,
        sqw_freq: &mut f64,
        sqw_enabled: &mut bool,
        sqw_state: &mut bool,
        sda_low: &mut bool,
        v_scl: f64,
        v_sda: f64,
        t: u64,
    ) -> Drive {
        if time_updated && !self.seeded {
            seed_clock(&mut self.regs);
            self.seeded = true;
        }
        let mut changed = false;
        let (tick, low) = self.wire.poll(v_scl, v_sda);
        if *sda_low != low {
            *sda_low = low;
            changed = true;
        }
        if tick.need_start_write || tick.stopped {
            self.have_ptr = false;
        }
        if let Some(byte) = tick.byte_received {
            if !self.have_ptr {
                self.pointer = byte & 0x07;
                self.have_ptr = true;
            } else {
                let idx = self.pointer & 0x07;
                self.regs[idx as usize] = byte;
                if idx == 7 {
                    apply_control(sqw_freq, sqw_enabled, sqw_state, byte);
                    changed = true;
                }
                self.pointer = (self.pointer + 1) & 0x07;
            }
        }
        if tick.need_slave_write {
            let idx = (self.pointer & 0x07) as usize;
            self.wire.twi.tx_reg = self.regs[idx];
            self.pointer = (self.pointer + 1) & 0x07;
        }
        if *sqw_enabled && !self.armed && sqw_half(*sqw_freq).is_some() {
            self.armed = true;
            let at = t.saturating_add(sqw_half(*sqw_freq).unwrap_or(1));
            return Drive::arm(at, changed, false);
        }
        Drive::stamp(changed)
    }

    pub fn on_sqw(
        &mut self,
        sqw_freq: f64,
        sqw_enabled: bool,
        sqw_state: &mut bool,
        t: u64,
    ) -> Drive {
        if !sqw_enabled {
            self.armed = false;
            return Drive::QUIET;
        }
        *sqw_state = !*sqw_state;
        self.armed = true;
        match sqw_half(sqw_freq) {
            Some(half) => Drive::arm(t.saturating_add(half), true, false),
            None => Drive::stamp(true),
        }
    }
}

pub fn sqw_half(freq: f64) -> Option<u64> {
    if !freq.is_finite() || freq <= 0.0 {
        return None;
    }
    Some((0.5e12 / freq).round().max(1.0) as u64)
}

fn apply_control(sqw_freq: &mut f64, sqw_enabled: &mut bool, sqw_state: &mut bool, reg: u8) {
    if reg & 0x10 != 0 {
        *sqw_enabled = true;
        *sqw_freq = match reg & 0x03 {
            0 => 1.0,
            1 => 4096.0,
            2 => 8192.0,
            _ => 32768.0,
        };
    } else {
        *sqw_enabled = false;
        *sqw_state = reg & 0x80 != 0;
    }
}

fn seed_clock(regs: &mut [u8; 8]) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let tod = (secs % 86_400) as u32;
    let (year, month, day) = civil_from_unix_days(days);
    let sec = tod % 60;
    let min = (tod / 60) % 60;
    let hour = tod / 3600;
    let weekday = ((days + 4).rem_euclid(7)) as u32;
    regs[0] = bcd(sec);
    regs[1] = bcd(min);
    regs[2] = bcd(hour);
    regs[3] = (weekday as u8) + 1;
    regs[4] = bcd(day);
    regs[5] = bcd(month);
    regs[6] = bcd((year.rem_euclid(100)) as u32);
}

fn bcd(n: u32) -> u8 {
    let n = n.min(99);
    ((n / 10) << 4 | (n % 10)) as u8
}

/// Howard Hinnant's `civil_from_days`, shifted so day 0 is 1970-01-01.
pub fn civil_from_unix_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[derive(Clone, Debug)]
pub struct EspUart {
    pub tx_high: bool,
    pub last_rx_high: bool,
    pub rx_on: bool,
    pub rx_bit: u8,
    pub rx_byte: u8,
    pub line: [u8; 16],
    pub line_len: usize,
    pub tx_shift: u64,
    pub tx_left: u8,
}

impl Default for EspUart {
    fn default() -> Self {
        Self {
            tx_high: true,
            last_rx_high: true,
            rx_on: false,
            rx_bit: 0,
            rx_byte: 0,
            line: [0; 16],
            line_len: 0,
            tx_shift: 0,
            tx_left: 0,
        }
    }
}

impl EspUart {
    pub fn on_pins(&mut self, baud: u32, rx_high: bool, t: u64) -> Drive {
        let mut drive = Drive::QUIET;
        if !self.rx_on && self.tx_left == 0 && self.last_rx_high && !rx_high {
            self.rx_on = true;
            self.rx_bit = 0;
            self.rx_byte = 0;
            let bit = uart_bit_ps(baud);
            drive = Drive::arm(t.saturating_add(bit + bit / 2), false, false);
        }
        self.last_rx_high = rx_high;
        drive
    }

    pub fn on_event(&mut self, baud: u32, rx_high: bool, t: u64) -> Drive {
        let bit = uart_bit_ps(baud);
        if self.rx_on {
            if self.rx_bit < 8 {
                if rx_high {
                    self.rx_byte |= 1 << self.rx_bit;
                }
                self.rx_bit += 1;
                return Drive::arm(t.saturating_add(bit), false, false);
            }
            self.rx_on = false;
            if rx_high && self.push_rx(self.rx_byte) {
                let changed = self.tx_high != (self.tx_shift & 1 == 1) || self.tx_left > 0;
                let level_changed = self.apply_tx_bit();
                let arm = if self.tx_left > 0 {
                    Some(t.saturating_add(bit))
                } else {
                    None
                };
                return Drive {
                    changed: level_changed || changed,
                    arm_at: arm,
                    replace: false,
                };
            }
            return Drive::QUIET;
        }
        if self.tx_left > 0 {
            let changed = self.apply_tx_bit();
            let arm = if self.tx_left > 0 {
                Some(t.saturating_add(bit))
            } else {
                None
            };
            return Drive {
                changed,
                arm_at: arm,
                replace: false,
            };
        }
        Drive::QUIET
    }

    fn push_rx(&mut self, byte: u8) -> bool {
        if self.line_len < self.line.len() {
            self.line[self.line_len] = byte;
            self.line_len += 1;
        } else {
            self.line.copy_within(1.., 0);
            let last = self.line.len() - 1;
            self.line[last] = byte;
        }
        if self.line_len >= 4 && &self.line[self.line_len - 4..self.line_len] == b"AT\r\n" {
            self.line_len = 0;
            self.load_tx(b"OK\r\n");
            return true;
        }
        false
    }

    fn load_tx(&mut self, bytes: &[u8]) {
        let mut shift = 0u64;
        let mut n = 0u8;
        for &byte in bytes {
            let frame = (u64::from(byte) << 1) | (1 << 9);
            shift |= frame << n;
            n = n.saturating_add(10);
        }
        self.tx_shift = shift;
        self.tx_left = n;
    }

    fn apply_tx_bit(&mut self) -> bool {
        if self.tx_left == 0 {
            let changed = !self.tx_high;
            self.tx_high = true;
            return changed;
        }
        let bit = self.tx_shift & 1 == 1;
        self.tx_shift >>= 1;
        self.tx_left -= 1;
        let changed = self.tx_high != bit;
        self.tx_high = bit;
        changed
    }
}

pub fn uart_bit_ps(baud: u32) -> u64 {
    (1_000_000_000_000 / u64::from(baud.max(1))).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dht22_frame_scales_tenths_and_checksum() {
        let frame = dht_frame("DHT22", 22.5, 68.5);
        let bytes = frame.to_be_bytes();
        assert_eq!(&bytes[3..], &[0x02, 0xAD, 0x00, 0xE1, 0x90]);
    }

    #[test]
    fn ds18_scratch_encodes_twelfths() {
        let scratch = ow_scratch(22.0);
        assert_eq!(scratch[0], 0x60);
        assert_eq!(scratch[1], 0x01);
        assert_eq!(crc8_maxim(&scratch[..8]), scratch[8]);
    }

    #[test]
    fn unix_day_zero_is_1970() {
        assert_eq!(civil_from_unix_days(0), (1970, 1, 1));
    }
}
