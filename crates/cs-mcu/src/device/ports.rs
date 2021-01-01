//! GPIO port queries, pin state updates, and direction synchronization.

use super::Device;
use crate::port::Port;

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

    #[inline]
    pub fn ports_dirty(&self) -> bool {
        self.host.ports_dirty || self.host.ports.iter().any(|p| p.dirty)
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
    pub fn set_gpio_input(&mut self, mut pin_idx: usize, high: bool) {
        self.is_spinning = false;
        for pi in 0..self.host.ports.len() {
            let port_len = self.host.ports[pi].pins.len();
            if pin_idx < port_len {
                let old = self.host.ports[pi].pins[pin_idx].inp_state;
                if old == high {
                    return;
                }
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
                return;
            }
            pin_idx -= port_len;
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
            }
            return;
        }
    }
}
