//! SerialPort and SerialTerm UART bit-bang engine.
//!
//! Bridges schematic RX/TX pins to host serial ports and virtual terminal sessions.
//! Follows configurable UART framing: 1 start bit (low), 5–9 data bits (LSB first),
//! and 1–2 stop bits (high).

use std::collections::VecDeque;

use super::sensors::{Drive, uart_bit_ps};
use crate::serial;

#[derive(Clone, Debug, PartialEq)]
pub struct SerialUart {
    pub tx_high: bool,
    pub last_rx_high: bool,
    pub rx_on: bool,
    pub rx_bit: u8,
    pub rx_byte: u16,
    pub rx_stop_ok: bool,
    pub rx_due: Option<u64>,
    pub tx_shift: u16,
    pub tx_left: u8,
    pub tx_due: Option<u64>,
    pub tx_queue: VecDeque<u8>,
}

impl Default for SerialUart {
    fn default() -> Self {
        Self {
            tx_high: true,
            last_rx_high: true,
            rx_on: false,
            rx_bit: 0,
            rx_byte: 0,
            rx_stop_ok: true,
            rx_due: None,
            tx_shift: 0,
            tx_left: 0,
            tx_due: None,
            tx_queue: VecDeque::new(),
        }
    }
}

impl SerialUart {
    fn load_tx_frame(&mut self, byte: u8, data_bits: usize, stop_bits: usize) {
        let d = data_bits.clamp(5, 9) as u8;
        let s = stop_bits.clamp(1, 2) as u8;
        let data_mask = (1u16 << d) - 1;
        let payload = u16::from(byte) & data_mask;
        // Stop bits (s high bits) plus 1 idle-high sentinel bit above start + data bits.
        let stop_and_idle = ((1u16 << (s + 1)) - 1) << (1 + d);
        self.tx_shift = (payload << 1) | stop_and_idle;
        self.tx_left = 1 + d + s + 1;
    }

    pub fn on_pins(
        &mut self,
        comp_id: &str,
        baud: u32,
        data_bits: usize,
        stop_bits: usize,
        rx_high: bool,
        t: u64,
    ) -> Drive {
        let bit = uart_bit_ps(baud);
        let mut changed = false;

        // Drain any pending bytes sent by the user/host
        self.drain_host_send(comp_id);

        // Start TX if idle and bytes are queued
        if self.tx_left == 0 && self.tx_due.is_none() {
            if let Some(byte) = self.tx_queue.pop_front() {
                self.load_tx_frame(byte, data_bits, stop_bits);
                changed |= self.apply_tx_bit();
                self.tx_due = Some(t.saturating_add(bit));
            }
        }

        // Detect falling edge on RX (start bit)
        if !self.rx_on && self.last_rx_high && !rx_high {
            self.rx_on = true;
            self.rx_bit = 0;
            self.rx_byte = 0;
            self.rx_stop_ok = true;
            self.rx_due = Some(t.saturating_add(bit + bit / 2));
        }
        self.last_rx_high = rx_high;

        self.make_drive(changed)
    }

    pub fn on_event(
        &mut self,
        comp_id: &str,
        baud: u32,
        data_bits: usize,
        stop_bits: usize,
        rx_high: bool,
        t: u64,
    ) -> Drive {
        let bit = uart_bit_ps(baud);
        let d = data_bits.clamp(5, 9) as u8;
        let s = stop_bits.clamp(1, 2) as u8;
        let mut changed = false;

        // Advance RX sampling
        if let Some(due) = self.rx_due {
            if t >= due {
                if self.rx_on {
                    if self.rx_bit < d {
                        if rx_high {
                            self.rx_byte |= 1u16 << self.rx_bit;
                        }
                        self.rx_bit += 1;
                        self.rx_due = Some(t.saturating_add(bit));
                    } else {
                        // Stop bit interval
                        if !rx_high {
                            self.rx_stop_ok = false;
                        }
                        self.rx_bit += 1;
                        if self.rx_bit < d + s {
                            self.rx_due = Some(t.saturating_add(bit));
                        } else {
                            self.rx_on = false;
                            self.rx_due = None;
                            if self.rx_stop_ok {
                                let out_byte = (self.rx_byte & 0xFF) as u8;
                                let mon_id = format!("{comp_id}:Serial");
                                let term_id = format!("{comp_id}:Terminal");
                                serial::publish_out(&mon_id, out_byte);
                                serial::publish_out(&term_id, out_byte);
                                serial::publish_out(comp_id, out_byte);
                                serial::publish_out("default", out_byte);
                            }
                        }
                    }
                } else {
                    self.rx_due = None;
                }
            }
        }

        // Advance TX bit transmission
        if let Some(due) = self.tx_due {
            if t >= due {
                if self.tx_left > 1 {
                    changed |= self.apply_tx_bit();
                    self.tx_due = Some(t.saturating_add(bit));
                } else if self.tx_left == 1 {
                    // Final stop bit interval completed
                    changed |= self.apply_tx_bit();
                    self.drain_host_send(comp_id);
                    if let Some(byte) = self.tx_queue.pop_front() {
                        self.load_tx_frame(byte, data_bits, stop_bits);
                        changed |= self.apply_tx_bit();
                        self.tx_due = Some(t.saturating_add(bit));
                    } else {
                        self.tx_due = None;
                    }
                } else {
                    self.tx_due = None;
                }
            }
        }

        self.make_drive(changed)
    }

    pub fn poll_send(
        &mut self,
        comp_id: &str,
        baud: u32,
        data_bits: usize,
        stop_bits: usize,
        t: u64,
    ) -> Drive {
        self.drain_host_send(comp_id);
        if self.tx_left == 0 && self.tx_due.is_none() {
            if let Some(byte) = self.tx_queue.pop_front() {
                let bit = uart_bit_ps(baud);
                self.load_tx_frame(byte, data_bits, stop_bits);
                let changed = self.apply_tx_bit();
                self.tx_due = Some(t.saturating_add(bit));
                return self.make_drive(changed);
            }
        }
        Drive::QUIET
    }

    fn drain_host_send(&mut self, comp_id: &str) {
        if !serial::has_pending_send() {
            return;
        }
        let mon_id = format!("{comp_id}:Serial");
        let term_id = format!("{comp_id}:Terminal");
        let mut pending = serial::take_send(&mon_id);
        pending.extend(serial::take_send(&term_id));
        pending.extend(serial::take_send(comp_id));
        if pending.is_empty() {
            pending.extend(serial::take_send("default"));
        }
        self.tx_queue.extend(pending);
    }

    fn apply_tx_bit(&mut self) -> bool {
        if self.tx_left == 0 {
            let ch = !self.tx_high;
            self.tx_high = true;
            return ch;
        }
        let bit = (self.tx_shift & 1) == 1;
        self.tx_shift >>= 1;
        self.tx_left -= 1;
        let ch = self.tx_high != bit;
        self.tx_high = bit;
        ch
    }

    fn make_drive(&self, changed: bool) -> Drive {
        let arm_at = match (self.rx_due, self.tx_due) {
            (Some(r), Some(x)) => Some(r.min(x)),
            (Some(r), None) => Some(r),
            (None, Some(x)) => Some(x),
            (None, None) => None,
        };
        Drive {
            changed,
            arm_at,
            replace: true,
        }
    }
}
