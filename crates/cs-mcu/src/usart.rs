//! C++ `McuUsart` / `UsartModule` / `UartTx` / `UartRx`.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, UsartSpec};
use crate::interrupts::Interrupts;
use crate::port::{self, Port};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Generic,
    Avr,
    Pic,
    I51,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TxState {
    Stopped,
    Idle,
    Transmit,
    TxEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RxState {
    Stopped,
    Idle,
    Receive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Parity {
    None = 0,
    Even = 2,
    Odd = 3,
}

const FRAME_ERROR: u16 = 1 << 12;
const DATA_OVERRUN: u16 = 1 << 13;
const PARITY_ERROR: u16 = 1 << 14;

#[derive(Clone, Debug)]
pub struct Usart {
    pub name: String,
    kind: Kind,
    tx_enabled: bool,
    rx_enabled: bool,
    tx_state: TxState,
    rx_state: RxState,
    tx_pin: Option<String>,
    rx_pin: Option<String>,
    rx_gpio: Option<(usize, usize)>,
    period: u64,
    data_bits: u8,
    data_mask: u8,
    stop_bits: u8,
    parity: Parity,
    speed_x2: bool,
    pub tx_buffered: bool,
    tx_buffer: u8,
    tx_data: u8,
    tx_frame: u16,
    tx_frame_size: u8,
    tx_bit: u8,
    pub tx_remain: Option<u64>,
    rx_frame: u16,
    rx_frame_size: u8,
    rx_bit: u8,
    rx_start_high: bool,
    rx_fifo: [u16; 2],
    rx_fifo_p: i8,
    fifo_size: i8,
    pub rx_remain: Option<u64>,
    empty_int: Option<usize>,
    tx_int: Option<usize>,
    rx_int: Option<usize>,
    // AVR
    u2xn: RegBits,
    txen: RegBits,
    rxen: RegBits,
    udre: RegBits,
    ucsra_addr: Option<u16>,
    ubrrl_addr: Option<u16>,
    ubrrh: u8,
    ucsz01: RegBits,
    ucsz2: RegBits,
    pari: RegBits,
    stop_rb: RegBits,
    ucsz2_val: u8,
    ucsz01_val: u8,
    // PIC
    txen_pic: RegBits,
    cren: RegBits,
    spen: RegBits,
    brgh: RegBits,
    sync: RegBits,
    tx9: RegBits,
    txif: RegBits,
    trmt: RegBits,
    pir1_addr: Option<u16>,
    spbrgl_addr: Option<u16>,
    spbrgh_addr: Option<u16>,
    pic_enabled: bool,
    // I51
    sm: RegBits,
    smod: RegBits,
    smod_val: u8,
    smod_div: bool,
    counter: u8,
    timer1_int: Option<usize>,
    pub tx_log: Vec<u8>,
    pub rx_pin_log: Vec<u8>,
}

impl Usart {
    pub fn from_spec(
        spec: &UsartSpec,
        data: &DataSpace,
        core: CoreKind,
        ints: &Interrupts,
    ) -> Self {
        let core_name = spec.core.as_deref().unwrap_or(match core {
            CoreKind::I51 => "8051",
            CoreKind::Avr => "AVR",
            CoreKind::Pic12 | CoreKind::Pic14 => "Pic14",
            _ => "",
        });
        let kind = match core_name {
            "8051" => Kind::I51,
            "AVR" => Kind::Avr,
            "Pic14" | "Pic14e" | "Pic12" => Kind::Pic,
            _ => Kind::Generic,
        };
        let n = if spec
            .name
            .chars()
            .last()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            spec.name.chars().last().unwrap().to_string()
        } else {
            String::new()
        };
        let mut u = Self {
            name: spec.name.clone(),
            kind,
            tx_enabled: false,
            rx_enabled: false,
            tx_state: TxState::Stopped,
            rx_state: RxState::Stopped,
            tx_pin: spec.tx.as_ref().and_then(|t| t.pins.first().cloned()),
            rx_pin: spec.rx.as_ref().and_then(|t| t.pins.first().cloned()),
            rx_gpio: None,
            period: 0,
            data_bits: 8,
            data_mask: 0xFF,
            stop_bits: 1,
            parity: Parity::None,
            speed_x2: false,
            tx_buffered: false,
            tx_buffer: 0,
            tx_data: 0,
            tx_frame: 0,
            tx_frame_size: 0,
            tx_bit: 0,
            tx_remain: None,
            rx_frame: 0,
            rx_frame_size: 0,
            rx_bit: 0,
            rx_start_high: false,
            rx_fifo: [0; 2],
            rx_fifo_p: -1,
            fifo_size: if kind == Kind::I51 { 1 } else { 2 },
            rx_remain: None,
            empty_int: ints.get(&spec.interrupt),
            tx_int: spec.tx.as_ref().and_then(|t| ints.get(&t.interrupt)),
            rx_int: spec.rx.as_ref().and_then(|t| ints.get(&t.interrupt)),
            u2xn: data.get_reg_bits(&format!("U2X{n}")),
            txen: data.get_reg_bits(&format!("TXEN{n}")),
            rxen: data.get_reg_bits(&format!("RXEN{n}")),
            udre: data.get_reg_bits(&format!("UDRE{n}")),
            ucsra_addr: data
                .reg_addr(&format!("UCSR{n}A"))
                .or_else(|| data.reg_addr("UCSRA")),
            ubrrl_addr: data
                .reg_addr(&format!("UBRR{n}L"))
                .or_else(|| data.reg_addr(&format!("UBRR{n}"))),
            ubrrh: 0,
            ucsz01: data.get_reg_bits(&format!("UCSZ{n}0,UCSZ{n}1")),
            ucsz2: data.get_reg_bits(&format!("UCSZ{n}2")),
            pari: data.get_reg_bits(&format!("UPM{n}0,UPM{n}1")),
            stop_rb: data.get_reg_bits(&format!("USBS{n}")),
            ucsz2_val: 0,
            ucsz01_val: if kind == Kind::Avr { 3 } else { 0 },
            txen_pic: data.get_reg_bits("TXEN"),
            cren: data.get_reg_bits("CREN"),
            spen: data.get_reg_bits("SPEN"),
            brgh: data.get_reg_bits("BRGH"),
            sync: data.get_reg_bits("SYNC"),
            tx9: data.get_reg_bits("TX9"),
            txif: data.get_reg_bits("TXIF"),
            trmt: {
                let b = data.get_reg_bits("TRMT");
                if b.mask != 0 {
                    b
                } else {
                    data.get_reg_bits("TMRT")
                }
            },
            pir1_addr: data.reg_addr("PIR1"),
            spbrgl_addr: data.reg_addr("SPBRG").or_else(|| data.reg_addr("SPBRGL")),
            spbrgh_addr: data.reg_addr("SPBRGH"),
            pic_enabled: false,
            sm: data.get_reg_bits("SM1,SM0"),
            smod: data.get_reg_bits("SMOD"),
            smod_val: 0,
            smod_div: false,
            counter: 0,
            timer1_int: ints.get("T1_OVF"),
            tx_log: Vec::new(),
            rx_pin_log: Vec::new(),
        };
        if n.is_empty() {
            u.u2xn = data.get_reg_bits("U2X");
            u.txen = data.get_reg_bits("TXEN");
            u.rxen = data.get_reg_bits("RXEN");
            u.udre = data.get_reg_bits("UDRE");
            u.ucsz01 = data.get_reg_bits("UCSZ0,UCSZ1");
            u.ucsz2 = data.get_reg_bits("UCSZ2");
            u.pari = data.get_reg_bits("UPM0,UPM1");
            u.stop_rb = data.get_reg_bits("USBS");
        }
        u
    }

    pub fn reset(&mut self, ports: &mut [Port], ints: &mut Interrupts) {
        self.tx_enabled = self.kind == Kind::I51;
        self.rx_enabled = false;
        self.tx_state = if self.tx_enabled {
            TxState::Idle
        } else {
            TxState::Stopped
        };
        self.rx_state = RxState::Stopped;
        self.tx_remain = None;
        self.rx_remain = None;
        self.tx_buffered = false;
        self.rx_fifo_p = -1;
        self.counter = 0;
        self.smod_div = false;
        self.tx_log.clear();
        self.rx_pin_log.clear();
        if self.kind == Kind::Avr {
            self.ucsz01_val = 3;
            self.ucsz2_val = 0;
            self.set_data_bits(8);
        }
        if self.kind == Kind::I51 {
            if let Some(i) = self.timer1_int {
                if !ints.ints[i]
                    .callbacks
                    .contains(&crate::interrupts::IntCallback::UsartTick(0))
                {
                    // index filled later by Device
                    let _ = i;
                }
            }
        }
        let _ = ports;
    }

    pub fn reset_status(&mut self, ram: &mut DataSpace, ints: &mut Interrupts) {
        match self.kind {
            Kind::Avr => {
                if let Some(addr) = self.ucsra_addr {
                    if self.udre.mask != 0 {
                        ram.set(addr, ram.get(addr) | self.udre.mask);
                    }
                }
                if let Some(i) = self.empty_int {
                    ints.raise(i, ram);
                }
            }
            Kind::Pic => {
                if let Some(addr) = self.pir1_addr {
                    if self.txif.mask != 0 {
                        ram.set(addr, ram.get(addr) | self.txif.mask);
                    }
                }
                if self.trmt.mask != 0 && self.trmt.reg_addr != 0 {
                    ram.set(
                        self.trmt.reg_addr,
                        ram.get(self.trmt.reg_addr) | self.trmt.mask,
                    );
                }
                if let Some(i) = self.empty_int {
                    ints.raise(i, ram);
                }
            }
            _ => {}
        }
    }

    pub fn set_period(&mut self, period: u64) {
        self.period = period;
    }

    fn set_data_bits(&mut self, b: u8) {
        self.data_bits = b.max(1);
        self.data_mask = if self.data_bits >= 8 {
            0xFF
        } else {
            (1u8 << self.data_bits).wrapping_sub(1)
        };
    }

    pub fn configure_a(&mut self, val: u8, ports: &mut [Port], data: &mut DataSpace, ps_inst: u64) {
        match self.kind {
            Kind::Avr => {
                let x2 = self.u2xn.mask != 0 && val & self.u2xn.mask != 0;
                if x2 != self.speed_x2 {
                    self.speed_x2 = x2;
                    self.set_avr_baud(data, ps_inst);
                }
            }
            Kind::Pic => self.pic_txsta(val, ports, data, ps_inst),
            Kind::I51 => self.i51_scon(val, ps_inst),
            Kind::Generic => {}
        }
    }

    pub fn configure_b(&mut self, val: u8, ports: &mut [Port], data: &mut DataSpace, ps_inst: u64) {
        match self.kind {
            Kind::Avr => self.avr_ucsrb(val, ports, data, ps_inst),
            Kind::Pic => self.pic_rcsta(val, ports),
            Kind::I51 => {
                self.smod_val = self.smod.val(val);
            }
            Kind::Generic => {}
        }
    }

    pub fn configure_c(&mut self, val: u8, data: &mut DataSpace, ps_inst: u64) {
        if self.kind != Kind::Avr {
            return;
        }
        self.stop_bits = self.stop_rb.val(val) + 1;
        self.ucsz01_val = self.ucsz01.val(val);
        self.set_data_bits(self.ucsz01_val + self.ucsz2_val + 5);
        let par = self.pari.val(val);
        self.parity = match par {
            2 => Parity::Even,
            3 => Parity::Odd,
            _ => Parity::None,
        };
        let _ = (data, ps_inst);
    }

    fn avr_ucsrb(&mut self, val: u8, ports: &mut [Port], data: &mut DataSpace, ps_inst: u64) {
        self.ucsz2_val = self.ucsz2.val(val) << 2;
        self.set_data_bits(self.ucsz01_val + self.ucsz2_val + 5);
        let tx = self.txen.mask == 0 || val & self.txen.mask != 0;
        self.set_tx_enabled(tx, ports);
        let rx = self.rxen.mask != 0 && val & self.rxen.mask != 0;
        self.set_rx_enabled(rx, ports);
        let _ = (data, ps_inst);
    }

    fn pic_txsta(&mut self, val: u8, ports: &mut [Port], data: &mut DataSpace, ps_inst: u64) {
        let tx = self.txen_pic.mask != 0 && val & self.txen_pic.mask != 0;
        if tx != self.tx_enabled {
            if let Some(addr) = self.pir1_addr {
                if self.txif.mask != 0 {
                    data.set(addr, data.get(addr) | self.txif.mask);
                }
            }
            self.set_tx_enabled(tx, ports);
        }
        self.set_data_bits(self.tx9.val(val) + 8);
        self.speed_x2 = self.brgh.mask != 0 && val & self.brgh.mask != 0;
        self.set_pic_baud(data, ps_inst);
        let _ = self.sync;
    }

    fn pic_rcsta(&mut self, val: u8, ports: &mut [Port]) {
        self.pic_enabled = self.spen.mask == 0 || val & self.spen.mask != 0;
        let rx = self.cren.mask != 0 && val & self.cren.mask != 0;
        self.set_rx_enabled(rx, ports);
    }

    fn i51_scon(&mut self, val: u8, ps_inst: u64) {
        let mode = self.sm.val(val);
        match mode {
            2 => {
                self.set_period(ps_inst.max(1));
                self.set_data_bits(9);
            }
            1 | 3 => {
                self.set_data_bits(if mode == 1 { 8 } else { 9 });
            }
            _ => {}
        }
    }

    pub fn set_baud_l(&mut self, val: u8, data: &mut DataSpace, ps_inst: u64) {
        if let Some(a) = self.ubrrl_addr.or(self.spbrgl_addr) {
            data.set(a, val);
        }
        match self.kind {
            Kind::Avr => self.set_avr_baud(data, ps_inst),
            Kind::Pic => self.set_pic_baud(data, ps_inst),
            _ => {}
        }
    }

    pub fn set_baud_h(&mut self, val: u8, data: &mut DataSpace, ps_inst: u64) {
        self.ubrrh = val;
        if let Some(a) = self.spbrgh_addr {
            data.set(a, val);
        }
        match self.kind {
            Kind::Avr => self.set_avr_baud(data, ps_inst),
            Kind::Pic => self.set_pic_baud(data, ps_inst),
            _ => {}
        }
    }

    fn set_avr_baud(&mut self, data: &DataSpace, ps_inst: u64) {
        let l = self.ubrrl_addr.map(|a| data.get(a)).unwrap_or(0);
        let ubrr = u16::from(l) | (u16::from(self.ubrrh & 0x0F) << 8);
        let mut period = 16 * u64::from(ubrr + 1) * ps_inst.max(1);
        if self.speed_x2 {
            period /= 2;
        }
        self.set_period(period);
    }

    fn set_pic_baud(&mut self, data: &DataSpace, ps_inst: u64) {
        let mut spbrg = u16::from(self.spbrgl_addr.map(|a| data.get(a)).unwrap_or(0));
        if let Some(a) = self.spbrgh_addr {
            spbrg |= u16::from(data.get(a)) << 8;
        }
        let mult: u64 = if self.speed_x2 { 4 } else { 16 };
        self.set_period(mult * u64::from(spbrg + 1) * ps_inst.max(1));
    }

    pub fn set_tx_enabled(&mut self, en: bool, ports: &mut [Port]) {
        if en == self.tx_enabled {
            return;
        }
        self.tx_enabled = en;
        if let Some(name) = &self.tx_pin {
            port::control_gpio(ports, name, en, en);
            if en {
                port::set_gpio_out(ports, name, true);
            }
        }
        self.tx_state = if en { TxState::Idle } else { TxState::Stopped };
        if !en {
            self.tx_remain = None;
        }
    }

    pub fn set_rx_enabled(&mut self, en: bool, ports: &mut [Port]) {
        if en == self.rx_enabled {
            return;
        }
        self.rx_enabled = en;
        if let Some(name) = &self.rx_pin {
            port::control_gpio(ports, name, en, en);
            if en {
                port::set_gpio_dir(ports, name, false);
            }
        }
        if en {
            self.rx_state = RxState::Idle;
            self.rx_frame_size =
                1 + self.data_bits + u8::from(self.parity != Parity::None) + self.stop_bits;
            self.rx_bit = 0;
            self.rx_fifo_p = -1;
            self.rx_start_high = self
                .rx_pin
                .as_ref()
                .and_then(|n| port::gpio_inp(ports, n))
                .unwrap_or(true);
        } else {
            self.rx_state = RxState::Stopped;
            self.rx_remain = None;
        }
    }

    pub fn send_byte(
        &mut self,
        data: u8,
        ram: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &mut [Port],
    ) {
        match self.kind {
            Kind::Avr => {
                if !self.tx_enabled {
                    return;
                }
                let empty = self.ucsra_addr.map(|a| ram.get(a)).unwrap_or(0xFF);
                if self.udre.mask == 0 || empty & self.udre.mask != 0 {
                    if self.tx_state == TxState::Transmit {
                        if let Some(addr) = self.ucsra_addr {
                            if self.udre.mask != 0 {
                                ram.set(addr, ram.get(addr) & !self.udre.mask);
                            }
                        }
                        if let Some(i) = self.empty_int {
                            ints.clear_flag(i, ram);
                        }
                    }
                    self.process_data(data, ram, ints, ports);
                }
            }
            Kind::Pic => {
                let pir = self.pir1_addr.map(|a| ram.get(a)).unwrap_or(0xFF);
                if self.txif.mask == 0 || pir & self.txif.mask != 0 {
                    if self.tx_state == TxState::Transmit {
                        if let Some(addr) = self.pir1_addr {
                            if self.txif.mask != 0 {
                                ram.set(addr, ram.get(addr) & !self.txif.mask);
                            }
                        }
                        if let Some(i) = self.empty_int {
                            ints.clear_flag(i, ram);
                        }
                    }
                    self.process_data(data, ram, ints, ports);
                }
            }
            _ => self.process_data(data, ram, ints, ports),
        }
    }

    fn process_data(
        &mut self,
        data: u8,
        ram: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &mut [Port],
    ) {
        self.tx_buffer = data;
        self.tx_buffered = true;
        if self.tx_enabled && self.tx_state == TxState::Idle {
            self.start_tx(ram, ints, ports);
        }
    }

    fn start_tx(&mut self, ram: &mut DataSpace, ints: &mut Interrupts, ports: &mut [Port]) {
        self.tx_buffered = false;
        if let Some(i) = self.empty_int {
            ints.raise(i, ram);
        }
        if self.kind == Kind::Avr {
            if let Some(addr) = self.ucsra_addr {
                if self.udre.mask != 0 {
                    ram.set(addr, ram.get(addr) | self.udre.mask);
                }
            }
        }
        if self.kind == Kind::Pic {
            if let Some(addr) = self.pir1_addr {
                if self.txif.mask != 0 {
                    ram.set(addr, ram.get(addr) | self.txif.mask);
                }
            }
            if self.trmt.mask != 0 && self.trmt.reg_addr != 0 {
                ram.set(
                    self.trmt.reg_addr,
                    ram.get(self.trmt.reg_addr) & !self.trmt.mask,
                );
            }
        }
        self.tx_data = self.tx_buffer;
        self.tx_state = TxState::Transmit;
        let data = self.tx_data & self.data_mask;
        self.tx_frame = u16::from(data) << 1;
        self.tx_frame_size = self.data_bits + 1;
        if self.parity != Parity::None {
            if parity_of(data, self.data_bits, self.parity) {
                self.tx_frame |= 1 << self.tx_frame_size;
            }
            self.tx_frame_size += 1;
        }
        for _ in 0..self.stop_bits {
            self.tx_frame |= 1 << self.tx_frame_size;
            self.tx_frame_size += 1;
        }
        self.tx_bit = 0;
        if self.period != 0 {
            self.send_bit(ports);
        }
    }

    fn send_bit(&mut self, ports: &mut [Port]) {
        if let Some(name) = &self.tx_pin {
            port::set_gpio_out(ports, name, self.tx_frame & 1 != 0);
        }
        self.tx_frame >>= 1;
        self.tx_bit += 1;
        if self.tx_bit == self.tx_frame_size {
            self.tx_state = TxState::TxEnd;
        }
        if self.period != 0 {
            self.tx_remain = Some(self.period);
        }
    }

    pub fn tx_event(&mut self, ram: &mut DataSpace, ints: &mut Interrupts, ports: &mut [Port]) {
        match self.tx_state {
            TxState::Stopped => self.tx_remain = None,
            TxState::Transmit => self.send_bit(ports),
            TxState::TxEnd => {
                self.tx_log.push((self.tx_data & self.data_mask) as u8);
                if self.tx_buffered {
                    self.start_tx(ram, ints, ports);
                } else {
                    self.tx_state = TxState::Idle;
                    if let Some(name) = &self.tx_pin {
                        port::set_gpio_out(ports, name, true);
                    }
                    if let Some(i) = self.tx_int {
                        ints.raise(i, ram);
                    }
                    if self.kind == Kind::Pic {
                        if self.trmt.mask != 0 && self.trmt.reg_addr != 0 {
                            ram.set(
                                self.trmt.reg_addr,
                                ram.get(self.trmt.reg_addr) | self.trmt.mask,
                            );
                        }
                    }
                    self.tx_remain = None;
                }
            }
            TxState::Idle => self.tx_remain = None,
        }
    }

    pub fn read_byte(&mut self, ram: &mut DataSpace, ints: &mut Interrupts) {
        if !ram.is_cpu_read {
            return;
        }
        ram.reg_override = Some(self.get_rx_data(ints, ram));
    }

    fn get_rx_data(&mut self, ints: &mut Interrupts, ram: &mut DataSpace) -> u8 {
        if self.rx_fifo_p < 0 {
            return 0;
        }
        let frame = self.rx_fifo[0];
        let data = (frame as u8) & self.data_mask;
        self.rx_fifo_p -= 1;
        if self.fifo_size > 1 && self.rx_fifo_p == 0 {
            self.rx_fifo[0] = self.rx_fifo[1];
            if let Some(i) = self.rx_int {
                ints.raise(i, ram);
            }
        } else if let Some(i) = self.rx_int {
            ints.clear_flag(i, ram);
        }
        data
    }

    pub fn rx_pin_changed(&mut self, ports: &[Port]) {
        if !self.rx_enabled || self.rx_state != RxState::Idle {
            return;
        }
        if self.rx_gpio.is_none() {
            if let Some(name) = &self.rx_pin {
                self.rx_gpio = port::find_gpio(ports, name);
            }
        }
        let Some((pi, pin_i)) = self.rx_gpio else {
            return;
        };
        let Some(bit) = ports
            .get(pi)
            .and_then(|p| p.pins.get(pin_i))
            .map(|p| p.inp_state)
        else {
            return;
        };
        if !self.rx_start_high && bit {
            self.rx_start_high = true;
        } else if self.rx_start_high && !bit {
            self.rx_state = RxState::Receive;
            if self.period != 0 {
                self.rx_remain = Some((self.period / 2).max(1));
            }
        }
    }

    pub fn rx_event(&mut self, ram: &mut DataSpace, ints: &mut Interrupts, ports: &[Port]) {
        if self.rx_state != RxState::Receive {
            self.rx_remain = None;
            return;
        }
        if self.rx_gpio.is_none() {
            if let Some(name) = &self.rx_pin {
                self.rx_gpio = port::find_gpio(ports, name);
            }
        }
        let bit = self
            .rx_gpio
            .and_then(|(pi, pin_i)| ports.get(pi)?.pins.get(pin_i).map(|p| p.inp_state))
            .unwrap_or(true);
        if bit {
            if self.rx_bit == 0 {
                self.rx_end();
                return;
            }
            self.rx_frame += 1 << self.rx_bit;
        }
        self.rx_bit += 1;
        if self.rx_bit == self.rx_frame_size {
            self.rx_frame >>= 1;
            let data = (self.rx_frame as u8) & self.data_mask;
            self.rx_pin_log.push(data);
            self.byte_received(self.rx_frame, ram, ints);
            self.rx_end();
        } else if self.period != 0 {
            self.rx_remain = Some(self.period);
        }
    }

    fn rx_end(&mut self) {
        self.rx_bit = 0;
        self.rx_frame = 0;
        self.rx_state = RxState::Idle;
        self.rx_remain = None;
        self.rx_start_high = false;
    }

    pub fn inject_rx_byte(&mut self, byte: u8, ram: &mut DataSpace, ints: &mut Interrupts) {
        if !self.rx_enabled {
            return;
        }
        let data = (byte & self.data_mask) as u16;
        let mut frame = data;
        let stop_bit = 1u16 << (self.data_bits + u8::from(self.parity != Parity::None));
        frame |= stop_bit;
        if self.parity != Parity::None {
            let p = parity_of(byte, self.data_bits, self.parity);
            if p {
                frame |= 1 << self.data_bits;
            }
        }
        self.byte_received(frame, ram, ints);
    }

    fn byte_received(&mut self, mut frame: u16, ram: &mut DataSpace, ints: &mut Interrupts) {
        if self.rx_fifo_p == self.fifo_size {
            self.rx_fifo_p -= 1;
            frame |= DATA_OVERRUN;
        }
        let stop_bit = 1u16 << (self.data_bits + u8::from(self.parity != Parity::None));
        if frame & stop_bit == 0 {
            frame |= FRAME_ERROR;
        }
        if self.parity != Parity::None {
            let p = parity_of(frame as u8, self.data_bits, self.parity);
            let pbit = frame & (1 << self.data_bits) != 0;
            if p != pbit {
                frame |= PARITY_ERROR;
            }
        }
        self.rx_fifo_p += 1;
        self.rx_fifo[self.rx_fifo_p as usize] = frame;
        if self.rx_fifo_p == 0 {
            if let Some(i) = self.rx_int {
                ints.raise(i, ram);
            }
        }
        let _ = FRAME_ERROR;
        let _ = DATA_OVERRUN;
        let _ = PARITY_ERROR;
    }

    /// 8051: 16 timer overflows per bit.
    pub fn timer_tick(&mut self, ram: &mut DataSpace, ints: &mut Interrupts, ports: &mut [Port]) {
        self.counter = self.counter.wrapping_add(1);
        if self.counter != 16 {
            return;
        }
        self.counter = 0;
        if self.smod_val & 1 == 0 {
            self.smod_div = !self.smod_div;
            if self.smod_div {
                return;
            }
        }
        self.tx_event(ram, ints, ports);
        self.rx_event(ram, ints, ports);
    }

    pub fn advance_tx(
        &mut self,
        elapsed: u64,
        ram: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &mut [Port],
    ) {
        let Some(r) = self.tx_remain else {
            return;
        };
        if elapsed < r {
            self.tx_remain = Some(r - elapsed);
            return;
        }
        self.tx_event(ram, ints, ports);
    }

    pub fn advance_rx(
        &mut self,
        elapsed: u64,
        ram: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &[Port],
    ) {
        let Some(r) = self.rx_remain else {
            return;
        };
        if elapsed < r {
            self.rx_remain = Some(r - elapsed);
            return;
        }
        self.rx_event(ram, ints, ports);
    }
}

fn parity_of(mut data: u8, bits: u8, par: Parity) -> bool {
    let mut p = false;
    for _ in 0..bits {
        p ^= data & 1 != 0;
        data >>= 1;
    }
    if par == Parity::Odd {
        p = !p;
    }
    p
}
