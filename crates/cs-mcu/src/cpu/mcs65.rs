//! MOS 6502 core (C++ `Mcs65Cpu`). Unified 64K program image plus address /
//! data bus GPIO (`PORTA` / `PORTD` / `RW`).

use crate::device::CpuHost;
use crate::port::{Port, pin_inp, port_named, set_pin_inp, set_pin_out};

/// C++ `m_tHA` — 25 ns address delay.
const T_HA: u64 = 25_000;
/// C++ `m_tHW` — 20 ns write-data delay.
const T_HW: u64 = 20_000;

pub const C: u8 = 0;
pub const Z: u8 = 1;
pub const I: u8 = 2;
pub const D: u8 = 3;
pub const B: u8 = 4;
pub const O: u8 = 5;
pub const V: u8 = 6;
pub const N: u8 = 7;

const CONSTANT: u8 = 0x20;
const BREAK: u8 = 0x10;

const I_X: u8 = 1 << 0;
const I_Y: u8 = 1 << 1;
const I_C: u8 = 1 << 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CpuState {
    Reset,
    Fetch,
    Decode,
    Read,
    Exec,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum AddrMode {
    None = 0,
    Accu,
    Imme,
    Abso,
    Zero,
    ZedX,
    Indi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exec {
    Adc,
    And,
    Asl,
    Bit,
    Brk,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    Bxx,
}

#[derive(Clone, Debug)]
pub struct Mcs65 {
    pub p: u8,
    pub sp: u8,
    pub ac: u8,
    pub ir: u8,
    pub x: u8,
    pub y: u8,
    pc: u16,
    debug_pc: u16,
    state: CpuState,
    next_state: CpuState,
    next_clock: bool,
    halt: bool,
    cycle: i32,
    a_mode: AddrMode,
    a_flags: u8,
    exec: Option<Exec>,
    u8_tmp0: u8,
    u8_tmp1: u8,
    op0: u8,
    op_addr: u16,
    bus_addr: u16,
    data_bus: u8,
    data_mode_out: bool,
    rw_level: bool,
    sync_level: bool,
    /// C++ `Mcs65Cpu` eElement delay (`m_tHA` / `m_tHW`).
    pub pending_bus_ps: Option<u64>,
    isr: u16,
    isr_source: i8,
    nmi_state: bool,
    /// Active-low control pins; default inactive (high).
    pub irq_high: bool,
    pub nmi_high: bool,
    pub rdy_high: bool,
    pub so_high: bool,
}

impl Mcs65 {
    pub fn new() -> Self {
        Self {
            p: 0,
            sp: 0,
            ac: 0,
            ir: 0,
            x: 0,
            y: 0,
            pc: 0,
            debug_pc: 0,
            state: CpuState::Reset,
            next_state: CpuState::Fetch,
            next_clock: true,
            halt: false,
            cycle: 0,
            a_mode: AddrMode::None,
            a_flags: 0,
            exec: None,
            u8_tmp0: 0,
            u8_tmp1: 0,
            op0: 0,
            op_addr: 0,
            bus_addr: 0,
            data_bus: 0,
            data_mode_out: false,
            rw_level: true,
            sync_level: false,
            pending_bus_ps: None,
            isr: 0,
            isr_source: 0,
            nmi_state: false,
            irq_high: true,
            nmi_high: true,
            rdy_high: true,
            so_high: true,
        }
    }

    pub fn reset(&mut self, ports: &mut [Port]) {
        self.p = 0b0011_0100;
        self.state = CpuState::Reset;
        self.next_state = CpuState::Fetch;
        self.exec = None;
        self.debug_pc = 0;
        self.pc = 0;
        self.cycle = 0;
        self.sp = 0;
        self.ac = 0;
        self.ir = 0;
        self.x = 0;
        self.y = 0;
        self.isr = 0;
        self.isr_source = 0;
        self.next_clock = true;
        self.halt = false;
        self.nmi_state = false;
        self.a_mode = AddrMode::None;
        self.a_flags = 0;
        self.op0 = 0;
        self.op_addr = 0;
        self.bus_addr = 0;
        self.data_bus = 0;
        self.data_mode_out = false;
        self.rw_level = true;
        self.sync_level = false;
        self.pending_bus_ps = None;
        self.stamp(ports);
    }

    /// C++ `Mcs65Cpu::stamp`.
    pub fn stamp(&mut self, ports: &mut [Port]) {
        if let Some(p) = port_named(ports, "PORTA") {
            p.reset();
            p.set_pin_mode(true);
        }
        if let Some(p) = port_named(ports, "PORTD") {
            p.reset();
            p.set_pin_mode(false);
        }
        if let Some(p) = port_named(ports, "CPORT0") {
            p.reset();
        }
        set_pin_out(ports, "RW", true);
        set_pin_out(ports, "P2", false);
        set_pin_out(ports, "SYNC", false);
        // Active-low / ready inputs idle high when nothing is driving them.
        for name in ["IRQ", "NMI", "RDY", "SO"] {
            set_pin_inp(ports, name, true);
        }
        self.irq_high = pin_inp(ports, "IRQ").unwrap_or(true);
        self.nmi_high = pin_inp(ports, "NMI").unwrap_or(true);
        self.rdy_high = pin_inp(ports, "RDY").unwrap_or(true);
        self.so_high = pin_inp(ports, "SO").unwrap_or(true);
    }

    /// One phi edge (C++ `Mcs65Cpu::runStep`). Rising then falling alternate;
    /// `cyclesDone` is 1 only on the falling edge.
    pub fn run_step(&mut self, h: &mut CpuHost) {
        h.cycles_done = 0;
        self.sample_ctrl(&h.ports);
        if self.next_clock {
            self.clk_rising(h);
        } else {
            self.clk_falling(h);
        }
        set_pin_out(&mut h.ports, "P2", self.next_clock);
        self.next_clock = !self.next_clock;
        h.pc = u32::from(self.debug_pc);
    }

    fn sample_ctrl(&mut self, ports: &[Port]) {
        if let Some(v) = pin_inp(ports, "IRQ") {
            self.irq_high = v;
        }
        if let Some(v) = pin_inp(ports, "NMI") {
            self.nmi_high = v;
        }
        if let Some(v) = pin_inp(ports, "RDY") {
            self.rdy_high = v;
        }
        if let Some(v) = pin_inp(ports, "SO") {
            self.so_high = v;
        }
    }

    fn has_bus(ports: &[Port]) -> bool {
        ports.iter().any(|p| p.name == "PORTA")
    }

    fn queue_addr(&mut self, ports: &[Port]) {
        if Self::has_bus(ports) {
            self.pending_bus_ps = Some(T_HA);
        }
    }

    /// C++ `Mcs65Cpu::runEvent`: address after falling, write data after rising.
    pub fn run_bus(&mut self, h: &mut CpuHost) {
        if self.next_clock {
            if let Some(p) = port_named(&mut h.ports, "PORTA") {
                p.set_out_state(u32::from(self.bus_addr));
            }
            set_pin_out(&mut h.ports, "RW", self.rw_level);
            set_pin_out(&mut h.ports, "SYNC", self.sync_level);
            if self.data_mode_out {
                self.data_mode_out = false;
                if let Some(p) = port_named(&mut h.ports, "PORTD") {
                    p.set_pin_mode(false);
                }
            }
        } else {
            if !self.data_mode_out {
                self.data_mode_out = true;
                if let Some(p) = port_named(&mut h.ports, "PORTD") {
                    p.set_pin_mode(true);
                }
            }
            if let Some(p) = port_named(&mut h.ports, "PORTD") {
                p.set_out_state(u32::from(self.op0));
            }
        }
        self.pending_bus_ps = None;
    }

    fn clk_rising(&mut self, h: &mut CpuHost) {
        if self.halt {
            return;
        }
        if !self.so_high {
            self.write_s_bit(V, true);
        }
        if self.state != CpuState::Write {
            return;
        }
        Self::mem_write(h, self.bus_addr, self.op0);
        self.state = self.next_state;
        if Self::has_bus(&h.ports) {
            self.pending_bus_ps = Some(T_HW);
        }
    }

    fn clk_falling(&mut self, h: &mut CpuHost) {
        self.halt = !self.rdy_high;
        if self.halt {
            return;
        }
        // C++ `readDataBus` samples the pins; the address from `readMem` is
        // driven after `tHA`, so this cycle still sees the previous fetch.
        self.data_bus = h.flash(u32::from(self.bus_addr)) as u8;
        h.cycles_done = 1;
        self.cycle += 1;

        let nmi = self.nmi_high;
        if self.isr == 0 {
            if self.nmi_state && !nmi {
                self.isr_source = 1;
            } else if self.isr_source == 0 && !self.status(I) && !self.irq_high {
                self.isr_source = 2;
            }
        }
        self.nmi_state = nmi;

        if self.state == CpuState::Reset {
            self.reset_seq();
            if self.state == CpuState::Read {
                self.state = CpuState::Reset;
                self.queue_addr(&h.ports);
                return;
            }
        }
        if self.state == CpuState::Decode {
            self.sync_level = false;
            self.ir = self.data_bus;
            if self.isr_source != 0 {
                self.exec = Some(Exec::Brk);
                self.state = CpuState::Exec;
                self.pc = self.pc.wrapping_sub(1);
            } else {
                self.decode();
            }
        }
        if self.state == CpuState::Read {
            self.read_operand();
        }
        if self.state == CpuState::Exec {
            self.state = CpuState::Fetch;
            if let Some(e) = self.exec {
                self.execute(h, e);
            }
        }
        if self.state == CpuState::Write {
            // C++ drops RW; fetch waits for the write cycle.
            self.rw_level = false;
        } else {
            self.rw_level = true;
            if self.state == CpuState::Fetch {
                if self.a_mode <= AddrMode::Accu {
                    self.pc = self.pc.wrapping_sub(1);
                }
                self.debug_pc = self.pc;
                self.read_pgm();
                h.ret_addr = u32::from(self.pc);
                self.cycle = 0;
                self.state = CpuState::Decode;
                self.sync_level = true;
            }
        }
        self.queue_addr(&h.ports);
    }

    fn reset_seq(&mut self) {
        match self.cycle {
            1 => {}
            2 => self.read_mem(0x0001),
            3 => self.read_mem(0x0100),
            4 | 5 => {}
            6 => self.read_mem(0xFFFC),
            7 => {
                self.read_mem(0xFFFD);
                self.op0 = self.data_bus;
            }
            8 => {
                let data = self.data_bus;
                self.pc = u16::from(self.op0) | (u16::from(data) << 8);
                self.state = CpuState::Fetch;
                self.a_mode = AddrMode::Imme;
            }
            _ => {}
        }
    }

    fn decode(&mut self) {
        self.state = CpuState::Read;
        self.a_mode = AddrMode::None;
        self.a_flags = 0;
        self.exec = None;

        match self.ir {
            0x00 => {
                self.exec = Some(Exec::Brk);
                self.isr_source = 3;
                return;
            }
            0x08 => {
                self.exec = Some(Exec::Php);
                return;
            }
            0x18 => {
                self.exec = Some(Exec::Clc);
                return;
            }
            0x20 => {
                self.exec = Some(Exec::Jsr);
                self.a_mode = AddrMode::Abso;
                return;
            }
            0x28 => {
                self.exec = Some(Exec::Plp);
                return;
            }
            0x38 => {
                self.exec = Some(Exec::Sec);
                return;
            }
            0x40 => {
                self.exec = Some(Exec::Rti);
                return;
            }
            0x48 => {
                self.exec = Some(Exec::Pha);
                return;
            }
            0x58 => {
                self.exec = Some(Exec::Cli);
                return;
            }
            0x60 => {
                self.exec = Some(Exec::Rts);
                return;
            }
            0x68 => {
                self.exec = Some(Exec::Pla);
                return;
            }
            0x78 => {
                self.exec = Some(Exec::Sei);
                return;
            }
            0x88 => {
                self.exec = Some(Exec::Dey);
                return;
            }
            0x8A => {
                self.exec = Some(Exec::Txa);
                return;
            }
            0x98 => {
                self.exec = Some(Exec::Tya);
                return;
            }
            0x9A => {
                self.exec = Some(Exec::Txs);
                return;
            }
            0xA8 => {
                self.exec = Some(Exec::Tay);
                return;
            }
            0xAA => {
                self.exec = Some(Exec::Tax);
                return;
            }
            0xB8 => {
                self.exec = Some(Exec::Clv);
                return;
            }
            0xBA => {
                self.exec = Some(Exec::Tsx);
                return;
            }
            0xC8 => {
                self.exec = Some(Exec::Iny);
                return;
            }
            0xCA => {
                self.exec = Some(Exec::Dex);
                return;
            }
            0xD8 => {
                self.exec = Some(Exec::Cld);
                return;
            }
            0xE8 => {
                self.exec = Some(Exec::Inx);
                return;
            }
            0xEA => {
                self.exec = Some(Exec::Nop);
                return;
            }
            0xF8 => {
                self.exec = Some(Exec::Sed);
                return;
            }
            _ => {}
        }

        let group = self.ir & 0b0000_0011;
        let ocode = (self.ir & 0b1110_0000) >> 5;
        let atype = (self.ir & 0b0001_1100) >> 2;
        self.decode_addr(group, ocode, atype);
        self.decode_exec(group, ocode, atype);
    }

    fn decode_addr(&mut self, group: u8, ocode: u8, atype: u8) {
        match atype {
            0 => {
                if group == 1 {
                    self.a_mode = AddrMode::Indi;
                    self.a_flags = I_X;
                } else {
                    self.a_mode = AddrMode::Imme;
                }
            }
            1 => self.a_mode = AddrMode::Zero,
            2 => {
                if group == 1 {
                    self.a_mode = AddrMode::Imme;
                } else if group == 2 {
                    self.a_mode = AddrMode::Accu;
                } else {
                    // C++ fallthrough to case 3.
                    self.a_mode = AddrMode::Abso;
                }
            }
            3 => self.a_mode = AddrMode::Abso,
            4 => {
                if group == 0 {
                    self.a_mode = AddrMode::Imme;
                } else if group == 1 {
                    self.a_mode = AddrMode::Indi;
                    self.a_flags = I_Y;
                } else {
                    // C++ fallthrough to case 5.
                    self.decode_zedx(group, ocode);
                }
            }
            5 => self.decode_zedx(group, ocode),
            6 => {
                if group == 1 {
                    self.a_mode = AddrMode::Abso;
                    self.a_flags = I_Y;
                } else {
                    // C++ fallthrough to case 7.
                    self.a_mode = AddrMode::Abso;
                    self.a_flags = I_X;
                }
            }
            7 => {
                self.a_mode = AddrMode::Abso;
                self.a_flags = I_X;
            }
            _ => {}
        }
    }

    fn decode_zedx(&mut self, group: u8, ocode: u8) {
        self.a_mode = AddrMode::ZedX;
        if group == 2 && (ocode == 4 || ocode == 5) {
            self.a_flags = I_Y;
        } else {
            self.a_flags = I_X;
        }
    }

    fn decode_exec(&mut self, group: u8, ocode: u8, atype: u8) {
        match group {
            0 => {
                if atype == 4 {
                    self.u8_tmp0 = (self.ir & 0b1100_0000) >> 6;
                    self.u8_tmp1 = (self.ir & 0b0010_0000) >> 5;
                    self.exec = Some(Exec::Bxx);
                } else {
                    self.exec = match ocode {
                        1 => Some(Exec::Bit),
                        2 | 3 => Some(Exec::Jmp),
                        4 => Some(Exec::Sty),
                        5 => Some(Exec::Ldy),
                        6 => Some(Exec::Cpy),
                        7 => Some(Exec::Cpx),
                        _ => None,
                    };
                }
            }
            1 => {
                self.exec = match ocode {
                    0 => Some(Exec::Ora),
                    1 => Some(Exec::And),
                    2 => Some(Exec::Eor),
                    3 => Some(Exec::Adc),
                    4 => Some(Exec::Sta),
                    5 => Some(Exec::Lda),
                    6 => Some(Exec::Cmp),
                    7 => Some(Exec::Sbc),
                    _ => None,
                };
            }
            2 => {
                self.exec = match ocode {
                    0 => Some(Exec::Asl),
                    1 => Some(Exec::Rol),
                    2 => Some(Exec::Lsr),
                    3 => Some(Exec::Ror),
                    4 => Some(Exec::Stx),
                    5 => Some(Exec::Ldx),
                    6 => Some(Exec::Dec),
                    7 => Some(Exec::Inc),
                    _ => None,
                };
            }
            _ => {}
        }
    }

    fn read_operand(&mut self) {
        if self.cycle == 1 {
            self.read_pgm();
            return;
        }
        self.state = CpuState::Exec;
        match self.a_mode {
            AddrMode::None => {}
            AddrMode::Accu => self.op0 = self.ac,
            AddrMode::Imme => self.op0 = self.data_bus,
            AddrMode::Abso => self.read_abso(),
            AddrMode::Zero => self.read_zero(),
            AddrMode::ZedX => self.read_zedx(),
            AddrMode::Indi => self.read_indi(),
        }
    }

    fn read_abso(&mut self) {
        match self.cycle {
            2 => {
                self.op_addr = u16::from(self.data_bus);
                self.read_pgm();
            }
            3 => {
                if self.a_flags != 0 {
                    if self.a_flags & I_X != 0 {
                        self.op_addr = self.op_addr.wrapping_add(u16::from(self.x));
                    } else if self.a_flags & I_Y != 0 {
                        self.op_addr = self.op_addr.wrapping_add(u16::from(self.y));
                    }
                    if self.op_addr > 255 {
                        self.op_addr &= 0xFF;
                        self.a_flags |= I_C;
                    }
                }
                self.op_addr |= u16::from(self.data_bus) << 8;
                self.read_mem(self.op_addr);
            }
            4 => {
                if self.a_flags & I_C != 0 {
                    self.op_addr = self.op_addr.wrapping_add(256);
                    self.read_mem(self.op_addr);
                } else {
                    self.op0 = self.data_bus;
                }
            }
            5 => self.op0 = self.data_bus,
            _ => {}
        }
    }

    fn read_zero(&mut self) {
        match self.cycle {
            2 => {
                self.op_addr = u16::from(self.data_bus);
                self.read_mem(self.op_addr);
            }
            3 => self.op0 = self.data_bus,
            _ => {}
        }
    }

    fn read_zedx(&mut self) {
        match self.cycle {
            2 => {
                self.u8_tmp0 = self.data_bus;
                self.read_mem(u16::from(self.u8_tmp0));
            }
            3 => {
                if self.a_flags & I_X != 0 {
                    self.u8_tmp0 = self.u8_tmp0.wrapping_add(self.x);
                } else if self.a_flags & I_Y != 0 {
                    self.u8_tmp0 = self.u8_tmp0.wrapping_add(self.y);
                }
                self.op_addr = u16::from(self.u8_tmp0);
                self.read_mem(self.op_addr);
            }
            4 => self.op0 = self.data_bus,
            _ => {}
        }
    }

    fn read_indi(&mut self) {
        if self.a_flags & I_X != 0 {
            match self.cycle {
                2 => {
                    self.u8_tmp0 = self.data_bus;
                    self.read_mem(u16::from(self.u8_tmp0));
                }
                3 => {
                    self.u8_tmp0 = self.u8_tmp0.wrapping_add(self.x);
                    self.read_mem(u16::from(self.u8_tmp0));
                }
                4 => {
                    // C++: `m_u8Tmp0 = readDataBus() + 1` (low byte + 1 used as addrL).
                    self.u8_tmp0 = self.data_bus.wrapping_add(1);
                    self.read_mem(u16::from(self.u8_tmp0));
                }
                5 => {
                    self.op_addr = u16::from(self.u8_tmp0) | (u16::from(self.data_bus) << 8);
                    self.read_mem(self.op_addr);
                }
                6 => self.op0 = self.data_bus,
                _ => {}
            }
        } else {
            match self.cycle {
                2 => {
                    self.u8_tmp0 = self.data_bus;
                    self.read_mem(u16::from(self.u8_tmp0));
                }
                3 => {
                    self.u8_tmp1 = self.data_bus;
                    self.read_mem(u16::from(self.u8_tmp0.wrapping_add(1)));
                }
                4 => {
                    self.op_addr = u16::from(self.u8_tmp1) + u16::from(self.y);
                    if self.op_addr > 255 {
                        self.op_addr &= 0xFF;
                        self.a_flags |= I_C;
                    }
                    self.op_addr |= u16::from(self.data_bus) << 8;
                    self.read_mem(self.op_addr);
                }
                5 => {
                    if self.a_flags & I_C != 0 {
                        self.op_addr = self.op_addr.wrapping_add(256);
                        self.read_mem(self.op_addr);
                    } else {
                        self.op0 = self.data_bus;
                    }
                }
                6 => self.op0 = self.data_bus,
                _ => {}
            }
        }
    }

    fn read_pgm(&mut self) {
        let addr = self.pc;
        self.pc = self.pc.wrapping_add(1);
        self.read_mem(addr);
    }

    fn read_mem(&mut self, addr: u16) {
        self.bus_addr = addr;
        self.state = CpuState::Read;
    }

    fn write_mem(&mut self, addr: u16) {
        self.bus_addr = addr;
        self.state = CpuState::Write;
        self.next_state = CpuState::Fetch;
    }

    fn mem_write(h: &mut CpuHost, addr: u16, v: u8) {
        let i = addr as usize;
        if i < h.prog.len() {
            h.prog[i] = u16::from(v);
        }
    }

    fn push_stack8(&mut self, byte: u8) {
        self.op0 = byte;
        let addr = 0x0100 + u16::from(self.sp);
        self.sp = self.sp.wrapping_sub(1);
        self.write_mem(addr);
    }

    fn pop_stack8(&mut self) {
        self.sp = self.sp.wrapping_add(1);
        self.read_mem(0x0100 + u16::from(self.sp));
    }

    fn status(&self, bit: u8) -> bool {
        self.p & (1 << bit) != 0
    }

    fn write_s_bit(&mut self, bit: u8, val: bool) {
        if val {
            self.p |= 1 << bit;
        } else {
            self.p &= !(1 << bit);
        }
    }

    fn set_nz(&mut self, val: u8) {
        self.write_s_bit(N, val & 0x80 != 0);
        self.write_s_bit(Z, val == 0);
    }

    fn write_wflags(&mut self, c: bool) {
        self.write_s_bit(C, c);
        self.set_nz(self.op0);
        if self.a_mode == AddrMode::Accu {
            self.ac = self.op0;
        } else {
            self.write_mem(self.op_addr);
        }
    }

    fn execute(&mut self, h: &mut CpuHost, e: Exec) {
        match e {
            Exec::Adc => self.adc(),
            Exec::And => {
                self.ac &= self.op0;
                self.set_nz(self.ac);
            }
            Exec::Asl => {
                let carry = self.op0 & 0x80 != 0;
                self.op0 <<= 1;
                self.write_wflags(carry);
            }
            Exec::Bit => {
                let res = self.op0 & self.ac;
                self.write_s_bit(N, res & 0x80 != 0);
                self.p = (self.p & 0x3F) | (self.op0 & 0xC0) | CONSTANT | BREAK;
                self.write_s_bit(Z, res == 0);
            }
            Exec::Brk => self.brk(),
            Exec::Clc => self.write_s_bit(C, false),
            Exec::Cld => self.write_s_bit(D, false),
            Exec::Cli => self.write_s_bit(I, false),
            Exec::Clv => self.write_s_bit(V, false),
            Exec::Cmp => {
                self.write_s_bit(C, self.ac >= self.op0);
                self.set_nz(self.ac.wrapping_sub(self.op0));
            }
            Exec::Cpx => {
                self.write_s_bit(C, self.x >= self.op0);
                self.set_nz(self.x.wrapping_sub(self.op0));
            }
            Exec::Cpy => {
                self.write_s_bit(C, self.y >= self.op0);
                self.set_nz(self.y.wrapping_sub(self.op0));
            }
            Exec::Dec => {
                self.op0 = self.op0.wrapping_sub(1);
                self.set_nz(self.op0);
                self.write_mem(self.op_addr);
            }
            Exec::Dex => {
                self.x = self.x.wrapping_sub(1);
                self.set_nz(self.x);
            }
            Exec::Dey => {
                self.y = self.y.wrapping_sub(1);
                self.set_nz(self.y);
            }
            Exec::Eor => {
                self.ac ^= self.op0;
                self.set_nz(self.ac);
            }
            Exec::Inc => {
                self.op0 = self.op0.wrapping_add(1);
                self.set_nz(self.op0);
                self.write_mem(self.op_addr);
            }
            Exec::Inx => {
                self.x = self.x.wrapping_add(1);
                self.set_nz(self.x);
            }
            Exec::Iny => {
                self.y = self.y.wrapping_add(1);
                self.set_nz(self.y);
            }
            Exec::Jmp => self.pc = self.op_addr,
            Exec::Jsr => self.jsr(),
            Exec::Lda => {
                self.ac = self.op0;
                self.set_nz(self.ac);
            }
            Exec::Ldx => {
                self.x = self.op0;
                self.set_nz(self.x);
            }
            Exec::Ldy => {
                self.y = self.op0;
                self.set_nz(self.y);
            }
            Exec::Lsr => {
                let carry = self.op0 & 1 != 0;
                self.op0 >>= 1;
                self.write_wflags(carry);
            }
            Exec::Nop => {}
            Exec::Ora => {
                self.ac |= self.op0;
                self.set_nz(self.ac);
            }
            Exec::Pha => self.push_stack8(self.ac),
            Exec::Php => {
                // C++ `pushStack8( STATUS(C) | STATUS(B) )` — not the full P.
                let c = if self.status(C) { 1 << C } else { 0 };
                let b = if self.status(B) { 1 << B } else { 0 };
                self.push_stack8(c | b);
            }
            Exec::Pla => {
                if self.cycle == 2 {
                    self.pop_stack8();
                } else {
                    self.ac = self.data_bus;
                    self.set_nz(self.ac);
                }
            }
            Exec::Plp => {
                if self.cycle == 2 {
                    self.pop_stack8();
                } else {
                    self.p = self.data_bus | CONSTANT | BREAK;
                }
            }
            Exec::Rol => {
                let carry = self.op0 & 0x80 != 0;
                self.op0 <<= 1;
                if self.status(C) {
                    self.op0 |= 0x01;
                }
                self.write_wflags(carry);
            }
            Exec::Ror => {
                let carry = self.op0 & 1 != 0;
                self.op0 >>= 1;
                if self.status(C) {
                    self.op0 |= 0x80;
                }
                self.write_wflags(carry);
            }
            Exec::Rti => self.rti(h),
            Exec::Rts => self.rts(),
            Exec::Sbc => self.sbc(),
            Exec::Sec => self.write_s_bit(C, true),
            Exec::Sed => self.write_s_bit(D, true),
            Exec::Sei => self.write_s_bit(I, true),
            Exec::Sta => {
                self.op0 = self.ac;
                self.write_mem(self.op_addr);
            }
            Exec::Stx => {
                self.op0 = self.x;
                self.write_mem(self.op_addr);
            }
            Exec::Sty => {
                self.op0 = self.y;
                self.write_mem(self.op_addr);
            }
            Exec::Tax => {
                self.x = self.ac;
                self.set_nz(self.x);
            }
            Exec::Tay => {
                self.y = self.ac;
                self.set_nz(self.y);
            }
            Exec::Txa => {
                self.ac = self.x;
                self.set_nz(self.ac);
            }
            Exec::Tya => {
                self.ac = self.y;
                self.set_nz(self.ac);
            }
            Exec::Tsx => {
                self.x = self.sp;
                self.set_nz(self.x);
            }
            Exec::Txs => self.sp = self.x,
            Exec::Bxx => self.bxx(),
        }
    }

    fn adc(&mut self) {
        let m = self.op0;
        let c = u32::from(self.status(C));
        let mut tmp = u32::from(m) + u32::from(self.ac) + c;
        self.write_s_bit(Z, tmp & 0xFF == 0);
        if self.status(D) {
            if u32::from(self.ac & 0xF) + u32::from(m & 0xF) + c > 9 {
                tmp += 6;
            }
            self.write_s_bit(N, tmp & 0x80 != 0);
            self.write_s_bit(
                V,
                (self.ac ^ m) & 0x80 == 0 && (u32::from(self.ac) ^ tmp) & 0x80 != 0,
            );
            if tmp > 0x99 {
                tmp += 96;
            }
            self.write_s_bit(C, tmp > 0x99);
        } else {
            self.write_s_bit(N, tmp & 0x80 != 0);
            self.write_s_bit(
                V,
                (self.ac ^ m) & 0x80 == 0 && (u32::from(self.ac) ^ tmp) & 0x80 != 0,
            );
            self.write_s_bit(C, tmp > 0xFF);
        }
        self.ac = tmp as u8;
    }

    fn sbc(&mut self) {
        let m = self.op0;
        let borrow = u32::from(!self.status(C));
        let mut tmp = u32::from(self.ac)
            .wrapping_sub(u32::from(m))
            .wrapping_sub(borrow);
        self.write_s_bit(N, tmp & 0x80 != 0);
        self.write_s_bit(Z, tmp & 0xFF == 0);
        self.write_s_bit(
            V,
            (u32::from(self.ac) ^ tmp) & 0x80 != 0 && (self.ac ^ m) & 0x80 != 0,
        );
        if self.status(D) {
            if u32::from(self.ac & 0x0F).wrapping_sub(borrow) < u32::from(m & 0x0F) {
                tmp = tmp.wrapping_sub(6);
            }
            if tmp > 0x99 {
                tmp = tmp.wrapping_sub(0x60);
            }
        }
        self.write_s_bit(C, tmp < 0x100);
        self.ac = tmp as u8;
    }

    fn brk(&mut self) {
        match self.cycle {
            1 => self.state = CpuState::Exec,
            2 => {
                self.push_stack8((self.pc >> 8) as u8);
                self.next_state = CpuState::Exec;
            }
            3 => {
                self.push_stack8(self.pc as u8);
                self.next_state = CpuState::Exec;
            }
            4 => {
                let mut status = self.p;
                match self.isr_source {
                    1 => {
                        self.isr = 0xFFFA;
                        status &= !BREAK;
                    }
                    2 => {
                        self.isr = 0xFFFE;
                        status &= !BREAK;
                    }
                    _ => {
                        self.isr = 0xFFFE;
                        status |= BREAK;
                    }
                }
                self.push_stack8(status);
                self.next_state = CpuState::Exec;
            }
            5 => {
                self.read_mem(self.isr);
                self.a_mode = AddrMode::Imme;
            }
            6 => {
                self.read_mem(self.isr.wrapping_add(1));
                self.u8_tmp0 = self.op0;
                self.a_mode = AddrMode::Imme;
                self.write_s_bit(I, true);
            }
            7 => {
                self.pc = (u16::from(self.op0) << 8) | u16::from(self.u8_tmp0);
                self.isr_source = 0;
                self.isr = 0;
            }
            _ => {}
        }
    }

    fn jsr(&mut self) {
        match self.cycle {
            4 => {
                self.push_stack8((self.pc >> 8) as u8);
                self.next_state = CpuState::Exec;
            }
            5 => {
                self.push_stack8(self.pc as u8);
                self.next_state = CpuState::Exec;
            }
            6 => self.pc = self.op_addr,
            _ => {}
        }
    }

    fn rti(&mut self, h: &mut CpuHost) {
        match self.cycle {
            2 => self.pop_stack8(),
            3 => {
                self.pop_stack8();
                self.p = self.data_bus | CONSTANT | BREAK;
            }
            4 => {
                self.pop_stack8();
                self.op0 = self.data_bus;
            }
            5 => {
                self.pc = (u16::from(self.data_bus) << 8) | u16::from(self.op0);
                self.a_mode = AddrMode::Imme;
                self.write_s_bit(I, false);
                h.reti = true;
            }
            _ => {}
        }
    }

    fn rts(&mut self) {
        match self.cycle {
            2 => self.pop_stack8(),
            3 => {
                self.pop_stack8();
                self.op0 = self.data_bus;
            }
            4 => {
                self.pc = (u16::from(self.data_bus) << 8) | u16::from(self.op0);
                self.a_mode = AddrMode::Imme;
            }
            _ => {}
        }
    }

    fn bxx(&mut self) {
        // C++ maps flag 1 to O (bit 5), not V.
        let bit = match self.u8_tmp0 {
            0 => N,
            1 => O,
            2 => C,
            3 => Z,
            _ => Z,
        };
        let flag_val = u8::from(self.status(bit));
        if flag_val == self.u8_tmp1 {
            self.pc = self.pc.wrapping_add_signed(i16::from(self.op0 as i8));
        }
    }
}

impl Default for Mcs65 {
    fn default() -> Self {
        Self::new()
    }
}
