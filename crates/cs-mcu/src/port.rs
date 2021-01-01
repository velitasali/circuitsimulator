//! C++ `McuPort` / `McuPin` GPIO (logical). Analog IoPin stamps stay in the host.

/// C++ `extIntTrig_t` interpretation (AVR ISC vs PIC INTEDG vs 8051 IT).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ExtIntKind {
    #[default]
    Avr,
    Pic,
    I51,
}

#[derive(Clone, Debug)]
pub struct PortSpec {
    pub name: String,
    pub n_pins: u8,
    pub pin_mask: u32,
    pub out_reg: String,
    pub in_reg: String,
    pub dir_reg: String,
    pub dir_inv: bool,
    pub out_mask: u32,
    /// C++ `m_inpMask`: true means *inactive* (permanent input is 0 in the mask).
    pub inp_mask: u32,
    pub pullups: String,
    pub reset_pin: String,
    /// C++ `opencol` bitmask.
    pub open_col: u32,
    /// C++ `<ioport>` (no SFR). Named labels when `pins="ALE,PSEN,EA"`.
    pub is_io: bool,
    pub pin_labels: Vec<String>,
    /// C++ `<port><interrupt>` pin-change INT.
    pub interrupt: Option<crate::desc::PortIntSpec>,
    /// C++ `<port><extint>`.
    pub extints: Vec<crate::desc::ExtIntSpec>,
}

#[derive(Clone, Debug)]
pub struct GpioPin {
    pub name: String,
    /// C++ `getPin` suffix: `"0"` or `"EA"`.
    pub label: String,
    pub number: u8,
    /// C++ `IoPort` uses `uint32_t` so 16-bit address buses fit.
    pub pin_mask: u32,
    pub out_state: bool,
    pub port_state: bool,
    pub is_out: bool,
    pub pullup: bool,
    pub open_coll: bool,
    pub inp_state: bool,
    pub out_mask: bool,
    pub inp_mask_inactive: bool,
    /// C++ `McuPin::m_outCtrl`: a peripheral is driving the pin.
    pub out_ctrl: bool,
    /// C++ `McuPin::m_dirCtrl`.
    pub dir_ctrl: bool,
    /// C++ `McuPin::m_extInt` index into [`crate::Interrupts`].
    pub ext_int: Option<usize>,
    /// C++ `extIntTrig_t`: Low=0, Change=1, Falling=2, Rising=3, Disabled=4.
    pub ext_int_trigger: u8,
    /// C++ `McuPin::m_extIntBits`.
    pub ext_int_bits: crate::dataspace::RegBits,
    /// AVR 4-level ISC, PIC INTEDG, 8051 IT0/IT1.
    pub ext_int_kind: ExtIntKind,
}

impl GpioPin {
    pub fn new(port: &str, i: u8) -> Self {
        Self {
            name: format!("{port}{i}"),
            label: i.to_string(),
            number: i,
            pin_mask: 1u32 << i,
            out_state: false,
            port_state: false,
            is_out: false,
            pullup: false,
            open_coll: false,
            inp_state: false,
            out_mask: false,
            inp_mask_inactive: true,
            out_ctrl: false,
            dir_ctrl: false,
            ext_int: None,
            ext_int_trigger: 0,
            ext_int_bits: crate::dataspace::RegBits::default(),
            ext_int_kind: ExtIntKind::Avr,
        }
    }

    /// C++ `McuPin::setDirection`. `out` true → output.
    pub fn set_direction(&mut self, out: bool) {
        if self.dir_ctrl {
            return;
        }
        self.is_out = (out || self.out_mask) && self.inp_mask_inactive;
    }

    pub fn set_port_state(&mut self, state: bool) {
        self.port_state = state;
        if self.out_ctrl {
            return;
        }
        self.out_state = state;
    }

    /// C++ `McuPin::controlPin`.
    pub fn control_pin(&mut self, out_ctrl: bool, dir_ctrl: bool) {
        let release_dir = !dir_ctrl && self.dir_ctrl;
        self.dir_ctrl = dir_ctrl;
        if release_dir {
            self.is_out = self.out_mask && self.inp_mask_inactive;
        }
        let release_out = !out_ctrl && self.out_ctrl;
        self.out_ctrl = out_ctrl;
        if release_out && self.is_out {
            self.out_state = self.port_state;
        }
    }

    /// C++ `McuPin::setOutState` while a peripheral owns the pin.
    pub fn set_ctrl_out(&mut self, high: bool) {
        if self.out_ctrl {
            self.out_state = high;
            self.is_out = self.inp_mask_inactive;
        }
    }

    /// C++ `McuPin::ConfExtInt` / `AvrPin` / `PicPin` / `I51Pin`.
    /// Returns `(continuous, auto_clear)` for the interrupt controller.
    pub fn conf_ext_int(&mut self, bits: u8) -> Option<(bool, bool)> {
        if self.ext_int.is_none() {
            return None;
        }
        match self.ext_int_kind {
            ExtIntKind::Avr => {
                self.ext_int_trigger = self.ext_int_bits.val(bits);
                let low = self.ext_int_trigger == 0;
                Some((low, !low))
            }
            ExtIntKind::Pic => {
                self.ext_int_trigger = if self.ext_int_bits.val(bits) != 0 {
                    3
                } else {
                    2
                };
                None
            }
            ExtIntKind::I51 => {
                let falling = self.ext_int_bits.val(bits) != 0;
                self.ext_int_trigger = if falling { 2 } else { 0 };
                Some((false, falling))
            }
        }
    }
}

/// C++ `McuPin::voltChanged` extint trigger. Returns `(fire, raise_val)`.
pub fn extint_trigger(trigger: u8, old: bool, new: bool) -> (bool, u8) {
    match trigger {
        0 => (old != new, u8::from(!new)), // pinLow: fallthrough to pinChange, raise = !new
        1 => (old != new, 1),              // pinChange
        2 => (old && !new, 1),             // pinFalling
        3 => (!old && new, 1),             // pinRising
        _ => (false, 0),
    }
}

#[derive(Clone, Debug)]
pub struct Port {
    pub name: String,
    pub pins: Vec<GpioPin>,
    pub dir_inv: bool,
    pub out_addr: Option<u16>,
    pub in_addr: Option<u16>,
    pub dir_addr: Option<u16>,
    pub pin_state: u32,
    pu_from_reg: bool,
    /// AVR: `PORTx` bit 1 enables the input pullup (C++ `AvrPin::setPortState`).
    pub pullup_from_out: bool,
    /// 8051: no DDR; the latch is the output (C++ ports without `dirreg`).
    pub always_out: bool,
    /// 8051 `I51Pin::setOutState`: bus drive AND the port latch.
    pub and_bus_with_latch: bool,
    /// C++ `<ioport>` (`IoPort`): 16-bit address/data buses, no SFR `out_ctrl`.
    pub is_io: bool,
    /// C++ `McuPort::m_interrupt` pin-change INT.
    pub interrupt: Option<usize>,
    /// C++ `McuPort::m_intMask`.
    pub int_mask: u32,
    /// C++ `McuPort::m_rstIntMask` (register-controlled mask resets to 0).
    pub rst_int_mask: bool,
    /// C++ `McuPort::m_intBits`.
    pub int_bits: crate::dataspace::RegBits,
    /// Fast change detection for external pin syncing.
    pub dirty: bool,
}

impl Port {
    pub fn from_spec(spec: &PortSpec, id: &str) -> Self {
        let n = spec.n_pins as usize;
        let mut pins = Vec::with_capacity(n);
        for i in 0..spec.n_pins {
            if spec.pin_mask & (1u32 << i) == 0 {
                continue;
            }
            let mut p = GpioPin::new(&spec.name, i);
            if let Some(label) = spec.pin_labels.get(i as usize) {
                p.label = label.clone();
                p.name = format!("{id}-{}{label}", spec.name);
            } else {
                p.name = format!("{id}-{}{i}", spec.name);
            }
            p.out_mask = spec.out_mask & (1u32 << i) != 0;
            p.inp_mask_inactive = spec.inp_mask & (1u32 << i) != 0;
            p.open_coll = spec.open_col & (1 << i) != 0;
            pins.push(p);
        }
        Self {
            name: spec.name.clone(),
            pins,
            dir_inv: spec.dir_inv,
            out_addr: None,
            in_addr: None,
            dir_addr: None,
            pin_state: 0,
            pu_from_reg: !spec.pullups.is_empty() && parse_bin_ok(&spec.pullups).is_none(),
            pullup_from_out: false,
            always_out: false,
            and_bus_with_latch: false,
            is_io: spec.is_io,
            interrupt: None,
            int_mask: 0,
            rst_int_mask: true,
            int_bits: crate::dataspace::RegBits::default(),
            dirty: true,
        }
    }

    pub fn reset(&mut self) {
        self.dirty = true;
        self.pin_state = 0;
        if self.rst_int_mask {
            self.int_mask = 0;
        }
        for p in &mut self.pins {
            p.out_state = false;
            p.port_state = false;
            p.is_out = p.out_mask && p.inp_mask_inactive;
            p.inp_state = p.pullup;
            p.out_ctrl = false;
            p.dir_ctrl = false;
            p.ext_int_trigger = match p.ext_int_kind {
                ExtIntKind::Pic => 2, // pinFalling
                _ => 0,
            };
        }
    }

    /// C++ `McuPort::intChanged`.
    pub fn int_changed(&mut self, val: u8) {
        if self.int_bits.mask != 0 {
            self.int_mask = u32::from(self.int_bits.val(val));
        } else {
            self.int_mask = u32::from(val);
        }
    }

    /// C++ `McuPort::controlPort`.
    pub fn control_port(&mut self, out_ctrl: bool, dir_ctrl: bool) {
        self.dirty = true;
        for p in &mut self.pins {
            let release_dir = !dir_ctrl && p.dir_ctrl;
            p.dir_ctrl = dir_ctrl;
            if release_dir {
                p.is_out = (p.out_mask || self.always_out) && p.inp_mask_inactive;
            }
            let release_out = !out_ctrl && p.out_ctrl;
            p.out_ctrl = out_ctrl;
            if release_out && p.is_out {
                p.out_state = p.port_state;
            }
        }
    }

    /// C++ `IoPort::setPinMode`.
    pub fn set_pin_mode(&mut self, output: bool) {
        self.dirty = true;
        for p in &mut self.pins {
            p.is_out = output && p.inp_mask_inactive;
        }
    }

    /// C++ `McuPort::setOutState` / `IoPort::setOutState`.
    pub fn set_out_state(&mut self, val: u32) {
        self.dirty = true;
        for p in &mut self.pins {
            if !self.is_io && !p.out_ctrl {
                continue;
            }
            let bit = val & p.pin_mask != 0;
            let state = if self.and_bus_with_latch {
                bit && p.port_state
            } else {
                bit
            };
            p.out_state = state;
            if !self.is_io {
                p.is_out = p.inp_mask_inactive;
            }
        }
    }

    /// C++ `McuPort::outChanged`.
    pub fn out_changed(&mut self, old: u8, val: u8) {
        let changed = u32::from(old ^ val);
        if changed == 0 {
            return;
        }
        self.dirty = true;
        let val = u32::from(val);
        for p in &mut self.pins {
            if changed & p.pin_mask != 0 {
                let state = val & p.pin_mask != 0;
                p.set_port_state(state);
                if self.always_out {
                    p.is_out = p.inp_mask_inactive;
                }
                if self.pullup_from_out {
                    p.pullup = state;
                }
            }
        }
    }

    /// C++ `McuPort::dirChanged`. `val` is the register value before invert.
    pub fn dir_changed(&mut self, old: u8, val: u8) {
        let changed = u32::from(old ^ val);
        if changed == 0 {
            return;
        }
        self.dirty = true;
        let mut val = u32::from(val);
        if self.dir_inv {
            val = !val;
        }
        for p in &mut self.pins {
            if changed & p.pin_mask != 0 {
                p.set_direction(val & p.pin_mask != 0);
            }
        }
    }

    pub fn pin_changed(&mut self, pin_mask: u32, val: u32) -> bool {
        let pin_state = (self.pin_state & !pin_mask) | (val & pin_mask);
        if pin_state == self.pin_state {
            return false;
        }
        self.pin_state = pin_state;
        true
    }

    pub fn get_inp_state(&self) -> u32 {
        let mut data = 0u32;
        for p in &self.pins {
            if p.inp_state {
                data |= p.pin_mask;
            }
        }
        data
    }

    pub fn set_pullups(&mut self, pu_mask: u32) {
        self.dirty = true;
        for p in &mut self.pins {
            p.pullup = pu_mask & p.pin_mask != 0;
        }
    }
}

fn parse_bin_ok(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    u8::from_str_radix(s, 2).ok()
}

impl Port {
    pub fn apply_const_pullups(&mut self, spec: &PortSpec) {
        if let Some(mask) = parse_bin_ok(&spec.pullups) {
            self.set_pullups(u32::from(mask));
        }
        let _ = self.pu_from_reg;
    }
}

pub fn pin_idx(ports: &[Port], label: &str) -> Option<(usize, usize)> {
    find_gpio(ports, label)
}

/// Match C++ `eMcu::getIoPin` / `getMcuPin`: label, `{PORT}{n}`, or name suffix.
pub fn find_gpio(ports: &[Port], name: &str) -> Option<(usize, usize)> {
    for (pi, p) in ports.iter().enumerate() {
        for (i, pin) in p.pins.iter().enumerate() {
            if pin.label == name
                || pin.name == name
                || pin.name.ends_with(name)
                || port_pin_matches(name, &p.name, pin.number)
            {
                return Some((pi, i));
            }
        }
    }
    None
}

/// Zero-allocation check: does `name == "{port_name}{pin_number}"`?
#[inline]
fn port_pin_matches(name: &str, port_name: &str, pin_number: u8) -> bool {
    if let Some(suffix) = name.strip_prefix(port_name) {
        // Compare the suffix against the decimal representation of pin_number
        // without allocating. pin_number is u8 so at most 3 digits.
        let mut buf = [0u8; 3];
        let n = itoa_u8(pin_number, &mut buf);
        suffix.as_bytes() == &buf[..n]
    } else {
        false
    }
}

/// Write the decimal representation of a u8 into `buf` and return the length.
#[inline]
fn itoa_u8(mut v: u8, buf: &mut [u8; 3]) -> usize {
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut i = 0usize;
    let mut tmp = [0u8; 3];
    while v > 0 {
        tmp[i] = b'0' + (v % 10);
        v /= 10;
        i += 1;
    }
    // reverse into buf
    for j in 0..i {
        buf[j] = tmp[i - 1 - j];
    }
    i
}

pub fn gpio_inp(ports: &[Port], name: &str) -> Option<bool> {
    let (pi, i) = find_gpio(ports, name)?;
    Some(ports[pi].pins[i].inp_state)
}

pub fn gpio_out(ports: &[Port], name: &str) -> Option<bool> {
    let (pi, i) = find_gpio(ports, name)?;
    Some(ports[pi].pins[i].out_state)
}

pub fn control_gpio(ports: &mut [Port], name: &str, out_ctrl: bool, dir_ctrl: bool) {
    if let Some((pi, i)) = find_gpio(ports, name) {
        ports[pi].dirty = true;
        ports[pi].pins[i].control_pin(out_ctrl, dir_ctrl);
    }
}

pub fn set_gpio_out(ports: &mut [Port], name: &str, high: bool) {
    if let Some((pi, i)) = find_gpio(ports, name) {
        ports[pi].dirty = true;
        let p = &mut ports[pi].pins[i];
        if p.out_ctrl {
            p.set_ctrl_out(high);
        } else {
            p.is_out = true;
            p.out_state = high;
        }
    }
}

pub fn set_gpio_dir(ports: &mut [Port], name: &str, output: bool) {
    if let Some((pi, i)) = find_gpio(ports, name) {
        ports[pi].dirty = true;
        ports[pi].pins[i].is_out = output && ports[pi].pins[i].inp_mask_inactive;
    }
}

pub fn pin_inp(ports: &[Port], label: &str) -> Option<bool> {
    let (pi, i) = pin_idx(ports, label)?;
    Some(ports[pi].pins[i].inp_state)
}

pub fn set_pin_out(ports: &mut [Port], label: &str, high: bool) {
    if let Some((pi, i)) = pin_idx(ports, label) {
        ports[pi].dirty = true;
        let p = &mut ports[pi].pins[i];
        p.is_out = true;
        p.out_state = high;
    }
}

pub fn set_pin_inp(ports: &mut [Port], label: &str, high: bool) {
    if let Some((pi, i)) = pin_idx(ports, label) {
        ports[pi].dirty = true;
        let p = &mut ports[pi].pins[i];
        p.is_out = false;
        p.inp_state = high;
        // Unconnected active-low control pins idle high (C++ packages
        // tie them off; analog nodes otherwise float to 0 V and halt the CPU).
        p.pullup = high;
    }
}

pub fn port_idx(ports: &[Port], name: &str) -> Option<usize> {
    ports.iter().position(|p| p.name == name)
}

pub fn port_named<'a>(ports: &'a mut [Port], name: &str) -> Option<&'a mut Port> {
    let i = port_idx(ports, name)?;
    ports.get_mut(i)
}

#[allow(dead_code)]
pub fn port_out_val(ports: &[Port], name: &str) -> u32 {
    let Some(i) = port_idx(ports, name) else {
        return 0;
    };
    let mut v = 0u32;
    for p in &ports[i].pins {
        if p.out_state {
            v |= p.pin_mask;
        }
    }
    v
}
