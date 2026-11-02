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

pub const OW_IDLE: u8 = 0;
pub const OW_RESET_ARMED: u8 = 2;
pub const OW_PRESENCE: u8 = 3;
pub const OW_ROM_CMD: u8 = 4;
pub const OW_READ_SCRATCH: u8 = 5;
pub const OW_SEARCH_ROM: u8 = 6;
pub const OW_MATCH_ROM: u8 = 7;
pub const OW_READ_ROM: u8 = 8;
pub const OW_FUNC_CMD: u8 = 9;
pub const OW_WRITE_SCRATCH: u8 = 10;
pub const OW_READ_POWER: u8 = 11;
pub const OW_CONVERT_WAIT: u8 = 12;

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
    pub rom: [u8; 8],
    pub active: bool,
    pub search_idx: u8,
    pub search_step: u8,
    pub match_idx: u8,
    pub eeprom: [u8; 3],
    pub converting: bool,
    pub parasitic: bool,
    pub rx_scratch_idx: u8,
    pub rx_scratch_buf: [u8; 3],
}

impl Default for OwLink {
    fn default() -> Self {
        let mut link = Self {
            phase: OW_IDLE,
            low_since: 0,
            saw_low: false,
            bit_n: 0,
            shifter: 0,
            cmd_n: 0,
            tx_i: 0,
            scratch: [0u8; 9],
            rom: [0x28, 0xFF, 0x2B, 0x45, 0x00, 0x00, 0x00, 0x00],
            active: true,
            search_idx: 0,
            search_step: 0,
            match_idx: 0,
            eeprom: [0x4B, 0x46, 0x7F],
            converting: false,
            parasitic: false,
            rx_scratch_idx: 0,
            rx_scratch_buf: [0; 3],
        };
        link.rom[7] = crc8_maxim(&link.rom[..7]);
        link.scratch = ow_scratch(22.0);
        link
    }
}

impl OwLink {
    pub fn on_pins(
        &mut self,
        rom_str: &str,
        temp: f64,
        dq_low: &mut bool,
        line: f64,
        vdd: f64,
        gnd: f64,
        t: u64,
    ) -> Drive {
        if self.rom == [0; 8] {
            self.rom = parse_rom_64(rom_str);
        }
        self.parasitic = (vdd - gnd) < 2.0;

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
            if !self.active {
                return Drive::QUIET;
            }
            match self.phase {
                OW_READ_SCRATCH => return self.on_read_scratch_slot(dq_low, t),
                OW_READ_ROM => return self.on_read_rom_slot(dq_low, t),
                OW_SEARCH_ROM => return self.on_search_read_slot(dq_low, t),
                OW_READ_POWER => return self.on_read_power_slot(dq_low, t),
                OW_CONVERT_WAIT => return self.on_convert_slot(dq_low, t),
                _ => return Drive::QUIET,
            }
        }

        if !rising || self.low_since == 0 {
            return Drive::QUIET;
        }

        let dur = t.saturating_sub(self.low_since);
        self.low_since = 0;

        // Reset pulse detection: low duration >= 480 us
        if dur >= 480 * US {
            self.phase = OW_RESET_ARMED;
            self.active = true;
            self.bit_n = 0;
            self.shifter = 0;
            self.cmd_n = 0;
            self.tx_i = 0;
            self.search_idx = 0;
            self.search_step = 0;
            self.match_idx = 0;
            self.rx_scratch_idx = 0;
            return Drive::arm(t.saturating_add(30 * US), false, true);
        }

        if !self.active {
            return Drive::QUIET;
        }

        let one = dur < 15 * US;
        match self.phase {
            OW_ROM_CMD => self.push_rom_cmd_bit(temp, one),
            OW_SEARCH_ROM => self.push_search_direction_bit(one),
            OW_MATCH_ROM => self.push_match_rom_bit(one),
            OW_FUNC_CMD => self.push_func_cmd_bit(temp, one),
            OW_WRITE_SCRATCH => self.push_write_scratch_bit(one),
            _ => Drive::QUIET,
        }
    }

    pub fn on_event(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        match self.phase {
            OW_RESET_ARMED => {
                *dq_low = true;
                self.phase = OW_PRESENCE;
                self.saw_low = true;
                Drive::arm(t.saturating_add(100 * US), true, false)
            }
            OW_PRESENCE => {
                *dq_low = false;
                self.phase = OW_ROM_CMD;
                self.saw_low = false;
                Drive::stamp(true)
            }
            _ => {
                let changed = *dq_low;
                *dq_low = false;
                self.saw_low = false;
                Drive::stamp(changed)
            }
        }
    }

    fn push_rom_cmd_bit(&mut self, temp: f64, one: bool) -> Drive {
        if one {
            self.shifter |= 1 << self.bit_n;
        }
        self.bit_n = self.bit_n.saturating_add(1);
        if self.bit_n < 8 {
            return Drive::QUIET;
        }
        let cmd = self.shifter;
        self.shifter = 0;
        self.bit_n = 0;

        match cmd {
            0xCC => {
                // Skip ROM
                self.phase = OW_FUNC_CMD;
            }
            0x33 => {
                // Read ROM
                self.phase = OW_READ_ROM;
                self.tx_i = 0;
            }
            0x55 => {
                // Match ROM
                self.phase = OW_MATCH_ROM;
                self.match_idx = 0;
            }
            0xF0 => {
                // Search ROM
                self.phase = OW_SEARCH_ROM;
                self.search_idx = 0;
                self.search_step = 0;
            }
            0xEC => {
                // Alarm Search
                let in_alarm = temp >= f64::from(self.scratch[2] as i8)
                    || temp <= f64::from(self.scratch[3] as i8);
                if in_alarm {
                    self.phase = OW_SEARCH_ROM;
                    self.search_idx = 0;
                    self.search_step = 0;
                } else {
                    self.active = false;
                    self.phase = OW_IDLE;
                }
            }
            _ => {
                self.active = false;
                self.phase = OW_IDLE;
            }
        }
        Drive::QUIET
    }

    fn push_search_direction_bit(&mut self, one: bool) -> Drive {
        if self.search_step != 2 {
            return Drive::QUIET;
        }
        let bit = self.rom_bit(self.search_idx);
        if one == bit {
            self.search_idx += 1;
            self.search_step = 0;
            if self.search_idx >= 64 {
                self.phase = OW_FUNC_CMD;
            }
        } else {
            self.active = false;
            self.phase = OW_IDLE;
        }
        Drive::QUIET
    }

    fn push_match_rom_bit(&mut self, one: bool) -> Drive {
        let bit = self.rom_bit(self.match_idx);
        if one == bit {
            self.match_idx += 1;
            if self.match_idx >= 64 {
                self.phase = OW_FUNC_CMD;
            }
        } else {
            self.active = false;
            self.phase = OW_IDLE;
        }
        Drive::QUIET
    }

    fn push_func_cmd_bit(&mut self, temp: f64, one: bool) -> Drive {
        if one {
            self.shifter |= 1 << self.bit_n;
        }
        self.bit_n = self.bit_n.saturating_add(1);
        if self.bit_n < 8 {
            return Drive::QUIET;
        }
        let cmd = self.shifter;
        self.shifter = 0;
        self.bit_n = 0;

        match cmd {
            0xBE => {
                // Read Scratchpad
                self.update_scratchpad_temp(temp);
                self.phase = OW_READ_SCRATCH;
                self.tx_i = 0;
            }
            0x44 => {
                // Convert T
                self.update_scratchpad_temp(temp);
                self.converting = true;
                self.phase = OW_CONVERT_WAIT;
            }
            0x4E => {
                // Write Scratchpad
                self.phase = OW_WRITE_SCRATCH;
                self.rx_scratch_idx = 0;
            }
            0x48 => {
                // Copy Scratchpad to EEPROM
                self.eeprom[0] = self.scratch[2];
                self.eeprom[1] = self.scratch[3];
                self.eeprom[2] = self.scratch[4];
                self.phase = OW_FUNC_CMD;
            }
            0xB8 => {
                // Recall EEPROM to Scratchpad
                self.scratch[2] = self.eeprom[0];
                self.scratch[3] = self.eeprom[1];
                self.scratch[4] = self.eeprom[2];
                self.scratch[8] = crc8_maxim(&self.scratch[..8]);
                self.phase = OW_FUNC_CMD;
            }
            0xB4 => {
                // Read Power Supply
                self.phase = OW_READ_POWER;
            }
            _ => {
                self.phase = OW_FUNC_CMD;
            }
        }
        Drive::QUIET
    }

    fn push_write_scratch_bit(&mut self, one: bool) -> Drive {
        if one {
            self.shifter |= 1 << self.bit_n;
        }
        self.bit_n = self.bit_n.saturating_add(1);
        if self.bit_n < 8 {
            return Drive::QUIET;
        }
        let val = self.shifter;
        self.shifter = 0;
        self.bit_n = 0;
        if (self.rx_scratch_idx as usize) < self.rx_scratch_buf.len() {
            self.rx_scratch_buf[self.rx_scratch_idx as usize] = val;
            self.rx_scratch_idx += 1;
        }
        if self.rx_scratch_idx >= 3 {
            self.scratch[2] = self.rx_scratch_buf[0]; // Th
            self.scratch[3] = self.rx_scratch_buf[1]; // Tl
            self.scratch[4] = self.rx_scratch_buf[2]; // Config
            self.scratch[8] = crc8_maxim(&self.scratch[..8]);
            self.phase = OW_FUNC_CMD;
        }
        Drive::QUIET
    }

    fn on_search_read_slot(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        let bit = self.rom_bit(self.search_idx);
        if self.search_step == 0 {
            // Slot 1: true bit
            self.search_step = 1;
            if !bit {
                *dq_low = true;
                self.saw_low = true;
                Drive::arm(t.saturating_add(40 * US), true, true)
            } else {
                Drive::QUIET
            }
        } else if self.search_step == 1 {
            // Slot 2: inverted bit
            self.search_step = 2;
            if bit {
                *dq_low = true;
                self.saw_low = true;
                Drive::arm(t.saturating_add(40 * US), true, true)
            } else {
                Drive::QUIET
            }
        } else {
            Drive::QUIET
        }
    }

    fn on_read_rom_slot(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        if self.tx_i >= 64 {
            self.phase = OW_IDLE;
            return Drive::QUIET;
        }
        let bit = self.rom_bit(self.tx_i as u8);
        self.tx_i += 1;
        if self.tx_i >= 64 {
            self.phase = OW_IDLE;
        }
        if !bit {
            *dq_low = true;
            self.saw_low = true;
            Drive::arm(t.saturating_add(40 * US), true, true)
        } else {
            Drive::QUIET
        }
    }

    fn on_read_power_slot(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        self.phase = OW_FUNC_CMD;
        if self.parasitic {
            *dq_low = true;
            self.saw_low = true;
            Drive::arm(t.saturating_add(40 * US), true, true)
        } else {
            Drive::QUIET
        }
    }

    fn on_convert_slot(&mut self, _dq_low: &mut bool, _t: u64) -> Drive {
        self.converting = false;
        self.phase = OW_FUNC_CMD;
        Drive::QUIET
    }

    pub fn on_read_scratch_slot(&mut self, dq_low: &mut bool, t: u64) -> Drive {
        let bit_one = self.scratch_bit(self.tx_i);
        self.tx_i = self.tx_i.saturating_add(1);
        if self.tx_i >= 72 {
            self.phase = OW_FUNC_CMD;
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

    fn rom_bit(&self, index: u8) -> bool {
        let byte = self.rom.get((index / 8) as usize).copied().unwrap_or(0xFF);
        (byte >> (index % 8)) & 1 == 1
    }

    fn update_scratchpad_temp(&mut self, temp: f64) {
        let res = (self.scratch[4] >> 5) & 0x03;
        let mut raw = (temp / 0.0625).round().clamp(-32768.0, 32767.0) as i16;
        match res {
            0 => raw &= !0x07, // 9-bit resolution (0.5°C)
            1 => raw &= !0x03, // 10-bit resolution (0.25°C)
            2 => raw &= !0x01, // 11-bit resolution (0.125°C)
            _ => {}            // 12-bit resolution (0.0625°C)
        }
        self.scratch[0] = raw as u8;
        self.scratch[1] = (raw >> 8) as u8;
        self.scratch[8] = crc8_maxim(&self.scratch[..8]);
    }
}

pub fn parse_rom_64(rom_str: &str) -> [u8; 8] {
    let clean: String = rom_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let mut bytes = [0u8; 8];
    bytes[0] = 0x28; // DS18B20 1-Wire family code
    let mut i = 0;
    while i < clean.len() && i / 2 < 8 {
        let end = (i + 2).min(clean.len());
        if let Ok(b) = u8::from_str_radix(&clean[i..end], 16) {
            bytes[i / 2] = b;
        }
        i += 2;
    }
    if bytes[0] != 0x28 {
        bytes[0] = 0x28;
    }
    if bytes[7] == 0 {
        bytes[7] = crc8_maxim(&bytes[..7]);
    }
    bytes
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
    pub last_tick_t: u64,
    pub regs: [u8; 64],
}

impl Default for Ds1307Bus {
    fn default() -> Self {
        Self {
            wire: I2cWire::new(0x68),
            pointer: 0,
            have_ptr: false,
            seeded: false,
            armed: false,
            last_tick_t: 0,
            regs: [0; 64],
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
            seed_clock(&mut self.regs[0..8].try_into().unwrap());
            self.seeded = true;
        }
        if self.last_tick_t == 0 {
            self.last_tick_t = t;
        } else {
            let elapsed = t.saturating_sub(self.last_tick_t);
            if elapsed >= 1_000_000_000_000 {
                let secs = elapsed / 1_000_000_000_000;
                self.last_tick_t += secs * 1_000_000_000_000;
                self.advance_seconds(secs);
            }
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
                self.pointer = byte & 0x3F;
                self.have_ptr = true;
            } else {
                let idx = (self.pointer & 0x3F) as usize;
                self.regs[idx] = byte;
                if idx == 7 {
                    apply_control(sqw_freq, sqw_enabled, sqw_state, byte);
                    changed = true;
                }
                self.pointer = (self.pointer + 1) & 0x3F;
            }
        }
        if tick.need_slave_write {
            let idx = (self.pointer & 0x3F) as usize;
            self.wire.twi.tx_reg = self.regs[idx];
            self.pointer = (self.pointer + 1) & 0x3F;
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

    pub fn advance_seconds(&mut self, secs: u64) {
        if secs == 0 {
            return;
        }
        // Bit 7 of register 0 is CH (Clock Halt: 1 = stopped, 0 = running)
        if (self.regs[0] & 0x80) != 0 {
            return;
        }
        let ch = self.regs[0] & 0x80;
        let mut sec = bcd_to_u32(self.regs[0] & 0x7F);
        let mut min = bcd_to_u32(self.regs[1] & 0x7F);
        let is_12h = (self.regs[2] & 0x40) != 0;
        let mut hour_24 = if is_12h {
            let pm = (self.regs[2] & 0x20) != 0;
            let h12 = bcd_to_u32(self.regs[2] & 0x1F);
            if pm {
                if h12 == 12 { 12 } else { (h12 + 12) % 24 }
            } else if h12 == 12 {
                0
            } else {
                h12
            }
        } else {
            bcd_to_u32(self.regs[2] & 0x3F)
        };
        let mut dow = u32::from(self.regs[3] & 0x07).max(1);
        let mut day = bcd_to_u32(self.regs[4] & 0x3F).max(1);
        let mut month = bcd_to_u32(self.regs[5] & 0x1F).max(1);
        let mut year = bcd_to_u32(self.regs[6]);

        let total_secs = u64::from(sec) + secs;
        sec = (total_secs % 60) as u32;
        let total_mins = u64::from(min) + (total_secs / 60);
        min = (total_mins % 60) as u32;
        let total_hours = u64::from(hour_24) + (total_mins / 60);
        hour_24 = (total_hours % 24) as u32;
        let days_to_add = total_hours / 24;

        if days_to_add > 0 {
            dow = ((dow - 1 + (days_to_add as u32)) % 7) + 1;
            for _ in 0..days_to_add {
                day += 1;
                let leap = (year % 4) == 0;
                let max_days = match month {
                    1 => 31,
                    2 => {
                        if leap {
                            29
                        } else {
                            28
                        }
                    }
                    3 => 31,
                    4 => 30,
                    5 => 31,
                    6 => 30,
                    7 => 31,
                    8 => 31,
                    9 => 30,
                    10 => 31,
                    11 => 30,
                    12 => 31,
                    _ => 31,
                };
                if day > max_days {
                    day = 1;
                    month += 1;
                    if month > 12 {
                        month = 1;
                        year = (year + 1) % 100;
                    }
                }
            }
        }

        self.regs[0] = ch | bcd(sec);
        self.regs[1] = bcd(min);
        if is_12h {
            let (h12, pm) = if hour_24 == 0 {
                (12, false)
            } else if hour_24 < 12 {
                (hour_24, false)
            } else if hour_24 == 12 {
                (12, true)
            } else {
                (hour_24 - 12, true)
            };
            self.regs[2] = 0x40 | (if pm { 0x20 } else { 0x00 }) | bcd(h12);
        } else {
            self.regs[2] = bcd(hour_24);
        }
        self.regs[3] = dow as u8;
        self.regs[4] = bcd(day);
        self.regs[5] = bcd(month);
        self.regs[6] = bcd(year);
    }
}

fn bcd_to_u32(val: u8) -> u32 {
    u32::from((val >> 4) * 10 + (val & 0x0F))
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

#[derive(Clone, Debug, PartialEq)]
pub struct EspConn {
    pub connected: bool,
    pub is_udp: bool,
    pub remote_ip: String,
    pub remote_port: u16,
}

impl Default for EspConn {
    fn default() -> Self {
        Self {
            connected: false,
            is_udp: false,
            remote_ip: String::new(),
            remote_port: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EspUart {
    pub tx_high: bool,
    pub last_rx_high: bool,
    pub rx_on: bool,
    pub rx_bit: u8,
    pub rx_byte: u8,
    pub rx_line: Vec<u8>,
    pub rx_raw_mode: bool,
    pub rx_raw_expected: usize,
    pub rx_raw_buf: Vec<u8>,
    pub rx_raw_conn_id: u8,
    pub tx_queue: Vec<u8>,
    pub tx_frame: u16,
    pub tx_bits_left: u8,
    pub cwmode: u8,
    pub connected_ssid: Option<String>,
    pub cipmux: bool,
    pub conns: [EspConn; 4],
}

impl Default for EspUart {
    fn default() -> Self {
        Self {
            tx_high: true,
            last_rx_high: true,
            rx_on: false,
            rx_bit: 0,
            rx_byte: 0,
            rx_line: Vec::with_capacity(64),
            rx_raw_mode: false,
            rx_raw_expected: 0,
            rx_raw_buf: Vec::with_capacity(256),
            rx_raw_conn_id: 0,
            tx_queue: Vec::new(),
            tx_frame: 0,
            tx_bits_left: 0,
            cwmode: 1,
            connected_ssid: None,
            cipmux: false,
            conns: [
                EspConn::default(),
                EspConn::default(),
                EspConn::default(),
                EspConn::default(),
            ],
        }
    }
}

impl EspUart {
    pub fn on_pins(&mut self, baud: u32, rx_high: bool, t: u64) -> Drive {
        let mut drive = Drive::QUIET;
        if !self.rx_on && self.last_rx_high && !rx_high {
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
                let changed = self.apply_tx_bit();
                let arm = if self.tx_bits_left > 0 || !self.tx_queue.is_empty() {
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
            return Drive::QUIET;
        }

        if self.tx_bits_left > 0 || !self.tx_queue.is_empty() {
            let changed = self.apply_tx_bit();
            let arm = if self.tx_bits_left > 0 || !self.tx_queue.is_empty() {
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

    pub fn load_tx(&mut self, bytes: &[u8]) {
        self.tx_queue.extend_from_slice(bytes);
        if self.tx_bits_left == 0 && !self.tx_queue.is_empty() {
            let byte = self.tx_queue.remove(0);
            self.tx_frame = (u16::from(byte) << 1) | (1 << 9);
            self.tx_bits_left = 10;
        }
    }

    pub fn apply_tx_bit(&mut self) -> bool {
        if self.tx_bits_left == 0 {
            if !self.tx_queue.is_empty() {
                let byte = self.tx_queue.remove(0);
                self.tx_frame = (u16::from(byte) << 1) | (1 << 9);
                self.tx_bits_left = 10;
            } else {
                let changed = !self.tx_high;
                self.tx_high = true;
                return changed;
            }
        }
        let bit = (self.tx_frame & 1) != 0;
        self.tx_frame >>= 1;
        self.tx_bits_left -= 1;
        let changed = self.tx_high != bit;
        self.tx_high = bit;
        changed
    }

    pub fn push_rx(&mut self, byte: u8) -> bool {
        if self.rx_raw_mode {
            self.rx_raw_buf.push(byte);
            if self.rx_raw_buf.len() >= self.rx_raw_expected {
                self.rx_raw_mode = false;
                let payload_len = self.rx_raw_buf.len();
                let mut resp = format!("Recv {} bytes\r\n\r\nSEND OK\r\n", payload_len);
                let conn_id = self.rx_raw_conn_id;
                let is_http = self.rx_raw_buf.starts_with(b"GET ")
                    || self.rx_raw_buf.starts_with(b"POST ")
                    || self.rx_raw_buf.starts_with(b"HEAD ");
                if is_http {
                    let body = "{\"status\":\"ok\",\"sim\":true}";
                    let http = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    if self.cipmux {
                        resp.push_str(&format!("+IPD,{},{}:{}", conn_id, http.len(), http));
                    } else {
                        resp.push_str(&format!("+IPD,{}:{}", http.len(), http));
                    }
                } else {
                    let echo = String::from_utf8_lossy(&self.rx_raw_buf);
                    if self.cipmux {
                        resp.push_str(&format!("+IPD,{},{}:{}", conn_id, echo.len(), echo));
                    } else {
                        resp.push_str(&format!("+IPD,{}:{}", echo.len(), echo));
                    }
                }
                self.load_tx(resp.as_bytes());
                return true;
            }
            return false;
        }

        self.rx_line.push(byte);
        if byte == b'\n' {
            let line = String::from_utf8_lossy(&self.rx_line).trim().to_string();
            self.rx_line.clear();
            if !line.is_empty() {
                self.handle_command(&line);
                return true;
            }
        }
        false
    }

    fn handle_command(&mut self, cmd: &str) {
        let upper = cmd.trim().to_uppercase();
        if upper == "AT" {
            self.load_tx(b"OK\r\n");
        } else if upper == "AT+RST" {
            self.connected_ssid = None;
            for c in &mut self.conns {
                c.connected = false;
            }
            self.load_tx(b"\r\nOK\r\nready\r\n");
        } else if upper == "AT+GMR" {
            self.load_tx(b"AT version:1.3.0.0(Jul 14 2016 18:54:01)\r\nSDK version:2.0.0(5a87e44)\r\nv1.0.0.3\r\nOK\r\n");
        } else if upper == "AT+CWMODE?" {
            let resp = format!("+CWMODE:{}\r\nOK\r\n", self.cwmode);
            self.load_tx(resp.as_bytes());
        } else if upper.starts_with("AT+CWMODE=") {
            let arg = upper.trim_start_matches("AT+CWMODE=").trim();
            if let Ok(m) = arg.parse::<u8>() {
                self.cwmode = m;
                self.load_tx(b"OK\r\n");
            } else {
                self.load_tx(b"ERROR\r\n");
            }
        } else if upper == "AT+CWLAP" {
            self.load_tx(b"+CWLAP:(3,\"SimNet\",-45,\"1a:2b:3c:4d:5e:6f\",1)\r\n+CWLAP:(4,\"GuestWiFi\",-72,\"1a:2b:3c:4d:5e:70\",6)\r\nOK\r\n");
        } else if upper == "AT+CWJAP?" {
            if let Some(ref ssid) = self.connected_ssid {
                let resp = format!("+CWJAP:\"{}\",\"1a:2b:3c:4d:5e:6f\",1,-45\r\nOK\r\n", ssid);
                self.load_tx(resp.as_bytes());
            } else {
                self.load_tx(b"No AP\r\nOK\r\n");
            }
        } else if upper.starts_with("AT+CWJAP=")
            || upper.starts_with("AT+CWJAP_CUR=")
            || upper.starts_with("AT+CWJAP_DEF=")
        {
            let rest = if upper.starts_with("AT+CWJAP=") {
                &cmd[9..]
            } else {
                &cmd[13..]
            };
            let parts: Vec<&str> = rest.split(',').collect();
            let raw_ssid = parts.first().copied().unwrap_or("\"SimNet\"");
            let ssid = raw_ssid.trim().trim_matches('"');
            self.connected_ssid = Some(ssid.to_string());
            self.load_tx(b"WIFI CONNECTED\r\nWIFI GOT IP\r\nOK\r\n");
        } else if upper == "AT+CWQAP" {
            self.connected_ssid = None;
            self.load_tx(b"OK\r\nWIFI DISCONNECT\r\n");
        } else if upper == "AT+CIFSR" {
            self.load_tx(
                b"+CIFSR:STAIP,\"192.168.4.2\"\r\n+CIFSR:STAMAC,\"18:fe:34:a1:b2:c3\"\r\nOK\r\n",
            );
        } else if upper == "AT+CIPSTATUS" {
            let status = if self.conns.iter().any(|c| c.connected) {
                3
            } else if self.connected_ssid.is_some() {
                2
            } else {
                5
            };
            let resp = format!("STATUS:{}\r\nOK\r\n", status);
            self.load_tx(resp.as_bytes());
        } else if upper == "AT+CIPMUX?" {
            let resp = format!("+CIPMUX:{}\r\nOK\r\n", if self.cipmux { 1 } else { 0 });
            self.load_tx(resp.as_bytes());
        } else if upper.starts_with("AT+CIPMUX=") {
            let arg = upper.trim_start_matches("AT+CIPMUX=").trim();
            if let Ok(m) = arg.parse::<u8>() {
                self.cipmux = m != 0;
                self.load_tx(b"OK\r\n");
            } else {
                self.load_tx(b"ERROR\r\n");
            }
        } else if upper.starts_with("AT+CIPSTART=") {
            let rest = &cmd[12..];
            let parts: Vec<&str> = rest.split(',').collect();
            let mut conn_id = 0usize;
            let mut offset = 0usize;
            if self.cipmux {
                if let Ok(id) = parts[0].trim().parse::<usize>() {
                    conn_id = id.min(3);
                    offset = 1;
                }
            }
            let conn_type = parts
                .get(offset)
                .copied()
                .unwrap_or("\"TCP\"")
                .to_uppercase();
            let host = parts
                .get(offset + 1)
                .copied()
                .unwrap_or("\"127.0.0.1\"")
                .trim()
                .trim_matches('"');
            let port = parts
                .get(offset + 2)
                .and_then(|p| p.trim().parse::<u16>().ok())
                .unwrap_or(80);
            self.conns[conn_id] = EspConn {
                connected: true,
                is_udp: conn_type.contains("UDP"),
                remote_ip: host.to_string(),
                remote_port: port,
            };
            self.load_tx(b"CONNECT\r\n\r\nOK\r\n");
        } else if upper.starts_with("AT+CIPSEND=") {
            let rest = &upper[11..];
            let parts: Vec<&str> = rest.split(',').collect();
            let (conn_id, len) = if self.cipmux && parts.len() >= 2 {
                (
                    parts[0].trim().parse::<u8>().unwrap_or(0),
                    parts[1].trim().parse::<usize>().unwrap_or(0),
                )
            } else {
                (0, parts[0].trim().parse::<usize>().unwrap_or(0))
            };
            self.rx_raw_mode = true;
            self.rx_raw_conn_id = conn_id;
            self.rx_raw_expected = len;
            self.rx_raw_buf.clear();
            self.load_tx(b">\r\n");
        } else if upper == "AT+CIPCLOSE" || upper.starts_with("AT+CIPCLOSE=") {
            if upper.starts_with("AT+CIPCLOSE=") {
                let id = upper[12..].trim().parse::<usize>().unwrap_or(0).min(3);
                self.conns[id].connected = false;
            } else {
                self.conns[0].connected = false;
            }
            self.load_tx(b"CLOSED\r\n\r\nOK\r\n");
        } else {
            self.load_tx(b"ERROR\r\n");
        }
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
