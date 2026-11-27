//! GPIO port queries, pin state updates, and direction synchronization.

use super::Device;
use crate::port::{self, Port};

impl Device {
    pub fn ports(&self) -> &[Port] {
        &self.host.ports
    }

    pub fn ports_mut(&mut self) -> &mut [Port] {
        &mut self.host.ports
    }

    pub fn gpio_pins(&self) -> impl Iterator<Item = &crate::port::GpioPin> {
        self.host.ports.iter().flat_map(|p| p.pins.iter())
    }

    pub fn gpio_pins_mut(&mut self) -> impl Iterator<Item = &mut crate::port::GpioPin> {
        self.host.ports.iter_mut().flat_map(|p| p.pins.iter_mut())
    }

    #[inline(always)]
    pub fn ports_dirty(&self) -> bool {
        self.host.ports_dirty
    }

    #[inline]
    pub fn set_ports_dirty(&mut self) {
        self.host.ports_dirty = true;
    }

    #[inline]
    pub fn clear_ports_dirty(&mut self) {
        self.host.ports_dirty = false;
        for p in &mut self.host.ports {
            p.dirty = false;
        }
    }

    /// Direct index-based GPIO pin input (zero string lookups).
    pub fn set_gpio_input(&mut self, pin_idx: usize, high: bool) {
        let volt = if high { self.vdd } else { 0.0 };
        self.set_gpio_input_volt(pin_idx, high, volt);
    }

    /// Direct index-based GPIO pin input with exact analog voltage for ADC sampling.
    pub fn set_gpio_input_volt(&mut self, mut pin_idx: usize, high: bool, volt: f64) {
        for pi in 0..self.host.ports.len() {
            let port_len = self.host.ports[pi].pins.len();
            if pin_idx < port_len {
                let old_volt = self.host.ports[pi].pins[pin_idx].inp_volt;
                self.host.ports[pi].pins[pin_idx].inp_volt = volt;
                if (old_volt - volt).abs() > 1e-4 {
                    self.is_spinning = false;
                    if !self.host.comps.is_empty() {
                        for c in &mut self.host.comps {
                            c.evaluate(
                                &mut self.host.data,
                                &self.host.ports,
                                &mut self.host.interrupts,
                            );
                        }
                    }
                }
                let old = self.host.ports[pi].pins[pin_idx].inp_state;
                if old == high {
                    return;
                }
                if self.host.rst_enabled && self.host.shared_reset_pin == Some((pi, pin_idx)) {
                    if !high {
                        if !self.host.shared_reset_held {
                            self.reset();
                            self.host.shared_reset_held = true;
                        }
                        self.host.ports[pi].pins[pin_idx].inp_state = false;
                        self.host.ports[pi].pins[pin_idx].inp_volt = volt;
                    } else {
                        self.host.ports[pi].pins[pin_idx].inp_state = true;
                        self.host.ports[pi].pins[pin_idx].inp_volt = volt;
                        self.host.shared_reset_held = false;
                    }
                    return;
                }
                if self.host.ext_osc_enabled && self.host.ext_clk_pin == Some((pi, pin_idx)) {
                    self.host.ports[pi].pins[pin_idx].inp_state = high;
                    if !old && high {
                        self.host.ext_clk_pending_edges =
                            self.host.ext_clk_pending_edges.saturating_add(1);
                    }
                    return;
                }
                self.is_spinning = false;
                self.host.ports[pi].pins[pin_idx].inp_state = high;
                let mask = self.host.ports[pi].pins[pin_idx].pin_mask;
                let ext_int = self.host.ports[pi].pins[pin_idx].ext_int;
                let trigger = self.host.ports[pi].pins[pin_idx].ext_int_trigger;
                if let Some(i) = ext_int {
                    let (fire, raise) = crate::port::extint_trigger(trigger, old, high);
                    if fire {
                        let _ = self
                            .host
                            .interrupts
                            .raise_val(i, raise, &mut self.host.data);
                    }
                }
                let val = if high { mask } else { 0 };
                if self.host.ports[pi].pin_changed(mask, val) {
                    let int_idx = self.host.ports[pi].interrupt;
                    let int_mask = self.host.ports[pi].int_mask;
                    if let Some(i) = int_idx {
                        if int_mask & mask != 0 {
                            let _ = self.host.interrupts.raise(i, &mut self.host.data);
                        }
                    }
                    if let Some(addr) = self.host.ports[pi].in_addr {
                        let state = self.host.ports[pi].pin_state as u8;
                        self.host.data.set(addr, state);
                    }
                }
                self.host.flush_peripherals();
                for t in &mut self.host.timers {
                    t.clock_pin_changed(
                        &mut self.host.ports,
                        &mut self.host.data,
                        &mut self.host.interrupts,
                    );
                }
                for u in &mut self.host.usarts {
                    u.rx_pin_changed(&self.host.ports);
                }
                for s in &mut self.host.spis {
                    s.on_pins_changed(
                        &mut self.host.ports,
                        &mut self.host.data,
                        &mut self.host.interrupts,
                    );
                }
                self.host.periph_dirty = true;
                if !self.host.ports_dirty && self.host.ports.iter().any(|p| p.dirty) {
                    self.host.ports_dirty = true;
                }
                return;
            }
            pin_idx -= port_len;
        }
    }

    /// Sample an analog voltage onto a named GPIO pin (updates both voltage and logic level).
    /// Accepts either the full pin ID (e.g. `"mega328-1-PORTC0"`) or short XML name (`"PORTC0"`).
    pub fn set_pin_volt(&mut self, pin_name: &str, volt: f64) {
        let high = volt > self.vdd * 0.5;
        let mut flat_idx = 0usize;
        for port in &self.host.ports {
            if let Some(pos) = port.pins.iter().position(|pin| pin.name == pin_name) {
                self.set_gpio_input_volt(flat_idx + pos, high, volt);
                return;
            }
            flat_idx += port.pins.len();
        }
        if let Some((pi, idx)) = port::find_gpio(&self.host.ports, pin_name) {
            let base: usize = self.host.ports[..pi].iter().map(|p| p.pins.len()).sum();
            self.set_gpio_input_volt(base + idx, high, volt);
        }
    }

    /// Sample an analog voltage onto a GPIO pin (C++ `McuPin::voltChanged`).
    pub fn set_pin_input(&mut self, pin_name: &str, high: bool) {
        for pi in 0..self.host.ports.len() {
            let Some(idx) = self.host.ports[pi]
                .pins
                .iter()
                .position(|pin| pin.name == pin_name)
            else {
                continue;
            };
            let old = self.host.ports[pi].pins[idx].inp_state;
            if old == high {
                return;
            }
            if self.host.rst_enabled && self.host.shared_reset_pin == Some((pi, idx)) {
                if !high {
                    if !self.host.shared_reset_held {
                        self.reset();
                        self.host.shared_reset_held = true;
                    }
                    self.host.ports[pi].pins[idx].inp_state = false;
                } else {
                    self.host.ports[pi].pins[idx].inp_state = true;
                    self.host.shared_reset_held = false;
                }
                return;
            }
            if self.host.ext_osc_enabled && self.host.ext_clk_pin == Some((pi, idx)) {
                self.host.ports[pi].pins[idx].inp_state = high;
                if !old && high {
                    self.host.ext_clk_pending_edges =
                        self.host.ext_clk_pending_edges.saturating_add(1);
                }
                return;
            }
            self.host.ports[pi].pins[idx].inp_state = high;
            let mask = self.host.ports[pi].pins[idx].pin_mask;
            let ext_int = self.host.ports[pi].pins[idx].ext_int;
            let trigger = self.host.ports[pi].pins[idx].ext_int_trigger;
            if old != high {
                self.is_spinning = false;
                if let Some(i) = ext_int {
                    let (fire, raise) = crate::port::extint_trigger(trigger, old, high);
                    if fire {
                        let _ = self
                            .host
                            .interrupts
                            .raise_val(i, raise, &mut self.host.data);
                    }
                }
                let val = if high { mask } else { 0 };
                if self.host.ports[pi].pin_changed(mask, val) {
                    let int_idx = self.host.ports[pi].interrupt;
                    let int_mask = self.host.ports[pi].int_mask;
                    if let Some(i) = int_idx {
                        if int_mask & mask != 0 {
                            let _ = self.host.interrupts.raise(i, &mut self.host.data);
                        }
                    }
                    if let Some(addr) = self.host.ports[pi].in_addr {
                        let state = self.host.ports[pi].pin_state as u8;
                        self.host.data.set(addr, state);
                    }
                }
                self.host.flush_peripherals();
                for t in &mut self.host.timers {
                    t.clock_pin_changed(
                        &mut self.host.ports,
                        &mut self.host.data,
                        &mut self.host.interrupts,
                    );
                }
                for u in &mut self.host.usarts {
                    u.rx_pin_changed(&self.host.ports);
                }
                for s in &mut self.host.spis {
                    s.on_pins_changed(
                        &mut self.host.ports,
                        &mut self.host.data,
                        &mut self.host.interrupts,
                    );
                }
                self.host.periph_dirty = true;
                if !self.host.ports_dirty && self.host.ports.iter().any(|p| p.dirty) {
                    self.host.ports_dirty = true;
                }
            }
            return;
        }
    }
}
