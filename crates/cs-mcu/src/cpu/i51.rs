//! Intel 8051 core (C++ `I51Core`). EA / ALE / PSEN external-program bus
//! plus MOVX XRAM (`mExtData` fallback; C++ instruction bodies were empty).

use std::collections::VecDeque;

use crate::dataspace::DataSpace;
use crate::device::CpuHost;
use crate::port::{Port, pin_idx, pin_inp, port_idx, port_named, set_pin_out};

pub const P: u8 = 0;
pub const OV: u8 = 2;
pub const RS0: u8 = 3;
pub const RS1: u8 = 4;
pub const AC: u8 = 6;
pub const CY: u8 = 7;

pub const REG_B: u16 = 0xF0;
pub const REG_DPL: u16 = 0x82;
pub const REG_DPH: u16 = 0x83;

const A_DIRE: u8 = 1;
const A_IMME: u8 = 1 << 2;
const A_RELA: u8 = 1 << 3;
const A_ORIG: u8 = 1 << 4;
const A_BIT: u8 = 1 << 5;
const A_16H: u8 = 1 << 6;
const A_16L: u8 = 1 << 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CpuState {
    Reset,
    Fetch,
    Operand,
    Exec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MemState {
    Idle,
    Laen,
    Addr,
    Ladi,
    Data,
    Read,
}

#[derive(Clone, Debug)]
pub struct I51 {
    pub acc_addr: u16,
    state: CpuState,
    read_pc: u16,
    last_pc: u16,
    pc: u16,
    read_cycle: i32,
    r_cycles: i32,
    opcode: u8,
    read_op: VecDeque<u8>,
    op_addr: u16,
    op0: u8,
    op2: u8,
    rx_addr: u8,
    bit_addr: u8,
    bit_mask: u8,
    invert: bool,
    ext_pgm: bool,
    mem_state: MemState,
    mem_read: bool,
    addr: u32,
    addr_h: u8,
    /// Last external-bus sample (C++ `m_data`).
    bus_data: u8,
    ps_step: u64,
    addr_set_time: u64,
    la_en_end_time: u64,
    read_set_time: u64,
    write_set_time: u64,
    read_bus_time: u64,
    data_time: u64,
    pub pending_bus_ps: Option<u64>,
    /// External data memory (C++ `mExtData`, 64K).
    pub xram: Vec<u8>,
}

impl I51 {
    pub fn new(data: &DataSpace) -> Self {
        Self {
            acc_addr: data.reg_addr("ACC").unwrap_or(0xE0),
            state: CpuState::Reset,
            read_pc: 0,
            last_pc: 0,
            pc: 0,
            read_cycle: 0,
            r_cycles: 2,
            opcode: 0,
            read_op: VecDeque::new(),
            op_addr: 0,
            op0: 0,
            op2: 0,
            rx_addr: 0,
            bit_addr: 0,
            bit_mask: 0,
            invert: false,
            ext_pgm: false,
            mem_state: MemState::Idle,
            mem_read: true,
            addr: 0,
            addr_h: 0,
            bus_data: 0,
            ps_step: 1,
            addr_set_time: 3,
            la_en_end_time: 4,
            read_set_time: 6,
            write_set_time: 6,
            read_bus_time: 12,
            data_time: 0,
            pending_bus_ps: None,
            xram: vec![0; 0x10000],
        }
    }

    pub fn reset(&mut self, ps_inst: u64, ports: &mut [Port]) {
        self.state = CpuState::Reset;
        self.read_pc = 0;
        self.last_pc = 0;
        self.pc = 0;
        self.read_cycle = 0;
        self.r_cycles = 2;
        self.opcode = 0;
        self.read_op.clear();
        self.op_addr = 0;
        self.op0 = 0;
        self.op2 = 0;
        self.invert = false;
        self.ext_pgm = false;
        self.mem_state = MemState::Idle;
        self.mem_read = true;
        self.bus_data = 0;
        self.pending_bus_ps = None;
        self.setup_times(ps_inst);
        self.stamp_ctrl(ports);
    }

    fn setup_times(&mut self, ps_inst: u64) {
        let read_cycle = (ps_inst / 2).max(24);
        self.ps_step = (read_cycle / 12).max(1);
        self.addr_set_time = 3 * self.ps_step;
        self.la_en_end_time = 4 * self.ps_step;
        self.read_set_time = 6 * self.ps_step;
        self.write_set_time = self.read_set_time;
        self.read_bus_time = read_cycle.saturating_sub(10).max(self.read_set_time + 1);
    }

    fn stamp_ctrl(&self, ports: &mut [Port]) {
        if let Some((pi, i)) = pin_idx(ports, "PSEN") {
            let p = &mut ports[pi].pins[i];
            p.is_out = true;
            p.out_state = true;
        }
        if let Some((pi, i)) = pin_idx(ports, "ALE") {
            let p = &mut ports[pi].pins[i];
            p.is_out = true;
            p.out_state = false;
        }
    }

    pub fn run_step(&mut self, h: &mut CpuHost) {
        h.cycles_done = 0;
        self.read_cycle += 1;

        let ext = pin_inp(&h.ports, "EA") == Some(false);
        if self.ext_pgm != ext {
            if let Some(p) = port_named(&mut h.ports, "PORT0") {
                p.control_port(ext, ext);
            }
            if let Some(p) = port_named(&mut h.ports, "PORT2") {
                p.control_port(ext, ext);
            }
            self.ext_pgm = ext;
        }

        let pgm = if ext {
            self.bus_data
        } else {
            h.flash(u32::from(self.read_pc)) as u8
        };

        match self.state {
            CpuState::Fetch => {
                self.opcode = pgm;
                self.read_cycle = 0;
                self.read_pc = self.read_pc.wrapping_add(1);
                self.last_pc = self.read_pc;
                self.decode(h);
                self.state = CpuState::Operand;
            }
            CpuState::Operand => self.read_operand(h, pgm),
            CpuState::Reset => self.state = CpuState::Fetch,
            CpuState::Exec => {}
        }

        if self.state == CpuState::Exec {
            h.pc = u32::from(self.last_pc);
            h.ret_addr = u32::from(self.last_pc);
            self.exec(h);
            self.read_pc = h.pc as u16;
            self.pc = h.pc as u16;
            self.state = CpuState::Fetch;
        } else {
            h.pc = u32::from(self.pc);
        }

        if ext {
            self.mem_read = true;
            self.addr = u32::from(self.read_pc);
            self.addr_h = (self.read_pc >> 8) as u8;
            self.mem_state = MemState::Laen;
            self.run_mem(h);
        }
        let prog_size = h.prog.len() as u16;
        if !ext && prog_size != 0 && self.read_pc >= prog_size {
            self.read_pc -= prog_size;
        }
        h.cycles_done = u32::from(self.read_cycle & 1 != 0);
    }

    /// C++ `I51Core::runEvent` (ALE / address / PSEN / data).
    pub fn run_mem(&mut self, h: &mut CpuHost) {
        match self.mem_state {
            MemState::Idle => {
                self.pending_bus_ps = None;
            }
            MemState::Laen => {
                set_pin_out(&mut h.ports, "ALE", true);
                self.pending_bus_ps = Some(self.addr_set_time);
                self.mem_state = MemState::Addr;
            }
            MemState::Addr => {
                if let Some(p) = port_named(&mut h.ports, "PORT0") {
                    p.set_out_state(self.addr as u32);
                }
                if let Some(p) = port_named(&mut h.ports, "PORT2") {
                    p.set_out_state(u32::from(self.addr_h));
                }
                self.pending_bus_ps = Some(self.la_en_end_time.saturating_sub(self.addr_set_time));
                self.mem_state = MemState::Ladi;
            }
            MemState::Ladi => {
                set_pin_out(&mut h.ports, "ALE", false);
                self.data_time = if self.mem_read {
                    self.read_set_time
                } else {
                    self.write_set_time
                };
                self.pending_bus_ps = Some(self.data_time.saturating_sub(self.la_en_end_time));
                self.mem_state = MemState::Data;
            }
            MemState::Data => {
                if self.mem_read {
                    if let Some(p) = port_named(&mut h.ports, "PORT0") {
                        p.set_out_state(0xFF);
                    }
                    set_pin_out(&mut h.ports, "PSEN", false);
                    self.pending_bus_ps =
                        Some(self.read_bus_time.saturating_sub(self.data_time).max(1));
                    self.mem_state = MemState::Read;
                } else {
                    if let Some(p) = port_named(&mut h.ports, "PORT0") {
                        p.set_out_state(u32::from(self.bus_data));
                    }
                    self.mem_state = MemState::Idle;
                    self.pending_bus_ps = None;
                    self.release_movx_bus(h);
                }
            }
            MemState::Read => {
                self.bus_data = port_idx(&h.ports, "PORT0")
                    .map(|i| h.ports[i].get_inp_state() as u8)
                    .unwrap_or(self.bus_data);
                set_pin_out(&mut h.ports, "PSEN", true);
                self.mem_state = MemState::Idle;
                self.pending_bus_ps = None;
                self.release_movx_bus(h);
            }
        }
    }

    fn release_movx_bus(&self, h: &mut CpuHost) {
        if self.ext_pgm {
            return;
        }
        if let Some(p) = port_named(&mut h.ports, "PORT0") {
            p.control_port(false, false);
        }
        if let Some(p) = port_named(&mut h.ports, "PORT2") {
            p.control_port(false, false);
        }
    }

    fn acc(&self, h: &CpuHost) -> u8 {
        h.data.get(self.acc_addr)
    }

    fn set_acc(&self, h: &mut CpuHost, v: u8) {
        h.data.set(self.acc_addr, v);
    }

    fn bank(h: &CpuHost) -> u8 {
        let s = h.data.get(h.data.sreg_addr);
        ((s & (1 << RS0)) >> RS0) | ((s & (1 << RS1)) >> RS1)
    }

    fn rx_val(&self, h: &CpuHost) -> u8 {
        let addr = u16::from((self.opcode & 1) + 8 * Self::bank(h));
        h.data.get(addr)
    }

    fn status(h: &CpuHost, bit: u8) -> bool {
        h.data.get(h.data.sreg_addr) & (1 << bit) != 0
    }

    fn write_s_bit(h: &mut CpuHost, bit: u8, val: bool) {
        let addr = h.data.sreg_addr;
        let mut s = h.data.get(addr);
        if val {
            s |= 1 << bit;
        } else {
            s &= !(1 << bit);
        }
        h.data.set(addr, s);
    }

    fn read_ind(&self, h: &mut CpuHost, addr: u16) -> u8 {
        h.get_ram(h.check_addr(addr))
    }

    fn write_ind(&self, h: &mut CpuHost, addr: u16, val: u8) {
        // C++ writes `m_opAddr` in the IRAM branch (not `addr`).
        if addr > h.low_data_end {
            if h.upper_data {
                h.data.set(addr.wrapping_add(h.reg_end), val);
            }
        } else {
            h.data.set(self.op_addr, val);
        }
    }

    fn push_stack8(&self, h: &mut CpuHost, v: u8) {
        let Some(sp) = h.spl_addr else {
            return;
        };
        let n = h.data.get(sp).wrapping_add(1);
        h.data.set(sp, n);
        let address = h.check_addr(u16::from(n));
        h.data.set(address, v);
    }

    fn pop_stack8(&self, h: &mut CpuHost) -> u8 {
        let Some(sp) = h.spl_addr else {
            return 0;
        };
        let n = h.data.get(sp);
        let address = h.check_addr(u16::from(n));
        let v = h.data.get(address);
        h.data.set(sp, n.wrapping_sub(1));
        v
    }

    fn add_flags(h: &mut CpuHost, v1: u8, v2: u8, acc: u8) {
        let sum = u16::from(v1) + u16::from(v2) + u16::from(acc);
        let c = ((sum >> 1) as u8) & (1 << 7) != 0;
        Self::write_s_bit(h, CY, c);
        let ac = ((u16::from(v1 & 0x0F) + u16::from(v2 & 0x0F) + u16::from(acc)) & (1 << 4)) != 0;
        Self::write_s_bit(h, AC, ac);
        let ov_sum = u16::from(v1 & 127) + u16::from(v2 & 127) + u16::from(acc);
        let ov = ((ov_sum as u8) ^ if c { 1 << 7 } else { 0 }) & (1 << 7) != 0;
        Self::write_s_bit(h, OV, ov);
    }

    fn sub_flags(h: &mut CpuHost, v1: u8, v2: u8, acc: u8) {
        let diff = i32::from(v1) - i32::from(v2) - i32::from(acc);
        let c = ((diff >> 1) as u8) & (1 << 7) != 0;
        Self::write_s_bit(h, CY, c);
        let ac_diff = i32::from(v1 & 0x0F) - i32::from(v2 & 0x0F) - i32::from(acc);
        Self::write_s_bit(h, AC, (ac_diff as u8) & (1 << 4) != 0);
        let ov_diff = i32::from(v1 & 127) - i32::from(v2 & 127) - i32::from(acc);
        let ov = ((ov_diff as u8) ^ if c { 1 << 7 } else { 0 }) & (1 << 7) != 0;
        Self::write_s_bit(h, OV, ov);
    }

    fn updt_parity(&self, h: &mut CpuHost) {
        let mut acu = self.acc(h);
        let mut parity = false;
        for _ in 0..8 {
            parity ^= acu & 1 != 0;
            acu >>= 1;
        }
        Self::write_s_bit(h, P, parity);
    }

    fn read_operand(&mut self, h: &mut CpuHost, pgm: u8) {
        self.read_pc = self.read_pc.wrapping_add(1);
        self.r_cycles -= 1;
        if self.r_cycles == 1 {
            self.state = CpuState::Exec;
        }
        let Some(addr_mode) = self.read_op.pop_front() else {
            // C++ returns without touching `m_lastPC` when operands are done
            // (the dummy read cycle of a 1-byte instruction).
            return;
        };
        if addr_mode & A_IMME != 0 {
            if addr_mode & A_ORIG != 0 {
                self.op0 = pgm;
            } else if addr_mode & A_RELA != 0 {
                self.op2 = pgm;
            } else if addr_mode & A_16H != 0 {
                self.op_addr = u16::from(pgm) << 8;
            } else if addr_mode & A_16L != 0 {
                self.op_addr |= u16::from(pgm);
            } else {
                self.op_addr = u16::from(pgm);
            }
        } else if addr_mode & A_DIRE != 0 {
            if addr_mode & A_ORIG != 0 {
                self.op0 = h.get_ram(u16::from(pgm));
            } else if addr_mode & A_RELA != 0 {
                self.op2 = h.get_ram(u16::from(pgm));
            } else {
                self.op_addr = u16::from(pgm);
            }
        } else if addr_mode & A_RELA != 0 {
            self.op2 = pgm;
        } else if addr_mode & A_BIT != 0 {
            self.bit_addr = pgm;
            self.bit_mask = 1 << (pgm & 7);
            if u16::from(self.bit_addr) > h.low_data_end {
                self.bit_addr &= 0xF8;
            } else {
                self.bit_addr >>= 3;
                self.bit_addr = self.bit_addr.wrapping_add(0x20);
            }
        }
        self.last_pc = self.read_pc;
    }

    fn oper_rgx(&mut self, h: &CpuHost) {
        self.op0 = h.data.get(u16::from(self.rx_addr));
    }
    fn oper_ind(&mut self, h: &mut CpuHost) {
        self.op0 = self.read_ind(h, u16::from(self.rx_val(h)));
    }
    fn oper_i08(&mut self) {
        self.read_op.push_back(A_IMME | A_ORIG);
    }
    fn oper_dir(&mut self) {
        self.read_op.push_back(A_DIRE | A_ORIG);
    }
    fn oper_acc(&mut self, h: &CpuHost) {
        self.op0 = self.acc(h);
    }
    fn opr2_i08(&mut self) {
        self.read_op.push_back(A_IMME | A_RELA);
    }
    fn opr2_dir(&mut self) {
        self.read_op.push_back(A_DIRE | A_RELA);
    }
    fn addr_rgx(&mut self) {
        self.op_addr = u16::from(self.rx_addr);
    }
    fn addr_ind(&mut self, h: &CpuHost) {
        self.op_addr = h.check_addr(u16::from(self.rx_val(h)));
    }
    fn addr_i08(&mut self) {
        self.read_op.push_back(A_IMME);
    }
    fn addr_i16(&mut self) {
        self.read_op.push_back(A_IMME | A_16H);
        self.read_op.push_back(A_IMME | A_16L);
    }
    fn addr_dir(&mut self) {
        self.read_op.push_back(A_DIRE);
    }
    fn addr_bit(&mut self, invert: bool) {
        self.read_op.push_back(A_BIT);
        self.invert = invert;
    }

    fn decode(&mut self, h: &mut CpuHost) {
        self.r_cycles = 2;
        self.read_op.clear();
        if self.opcode & 8 != 0 {
            self.rx_addr = (self.opcode & 0x07) + 8 * Self::bank(h);
            match self.opcode & 0xF0 {
                0x00 | 0x10 => self.addr_rgx(),
                0x20 | 0x30 | 0x40 | 0x50 | 0x60 => self.oper_rgx(h),
                0x70 => {
                    self.addr_rgx();
                    self.oper_i08();
                }
                0x80 => {
                    self.addr_dir();
                    self.oper_rgx(h);
                    self.r_cycles = 4;
                }
                0x90 => self.oper_rgx(h),
                0xA0 => {
                    self.addr_rgx();
                    self.oper_dir();
                    self.r_cycles = 4;
                }
                0xB0 => {
                    self.oper_rgx(h);
                    self.opr2_i08();
                    self.addr_dir();
                    self.r_cycles = 4;
                }
                0xC0 => self.addr_rgx(),
                0xD0 => {
                    self.op0 = self.rx_addr;
                    self.addr_dir();
                    self.r_cycles = 4;
                }
                0xE0 => self.oper_rgx(h),
                0xF0 => {
                    self.addr_rgx();
                    self.oper_acc(h);
                }
                _ => {}
            }
            return;
        }
        match self.opcode {
            0x00 | 0x03 | 0x04 => {}
            0x01 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x02 => {
                self.addr_i16();
                self.r_cycles = 4;
            }
            0x05 => self.addr_dir(),
            0x06 | 0x07 => self.addr_ind(h),
            0x10 => {
                self.addr_bit(false);
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x11 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x12 => {
                self.addr_i16();
                self.r_cycles = 4;
            }
            0x13 | 0x14 => {}
            0x15 => self.addr_dir(),
            0x16 | 0x17 => self.addr_ind(h),
            0x20 => {
                self.addr_bit(false);
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x21 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x22 => self.r_cycles = 4,
            0x23 => {}
            0x24 => self.oper_i08(),
            0x25 => self.oper_dir(),
            0x26 | 0x27 => self.oper_ind(h),
            0x30 => {
                self.addr_bit(false);
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x31 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x32 => self.r_cycles = 4,
            0x33 => {}
            0x34 => self.oper_i08(),
            0x35 => self.oper_dir(),
            0x36 | 0x37 => self.oper_ind(h),
            0x40 | 0x41 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x42 => {
                self.addr_dir();
                self.oper_acc(h);
            }
            0x43 => {
                self.addr_dir();
                self.oper_i08();
                self.r_cycles = 4;
            }
            0x44 => self.oper_i08(),
            0x45 => self.oper_dir(),
            0x46 | 0x47 => self.oper_ind(h),
            0x50 | 0x51 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x52 => {
                self.addr_dir();
                self.oper_acc(h);
            }
            0x53 => {
                self.addr_dir();
                self.oper_i08();
                self.r_cycles = 4;
            }
            0x54 => self.oper_i08(),
            0x55 => self.oper_dir(),
            0x56 | 0x57 => self.oper_ind(h),
            0x60 | 0x61 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x62 => {
                self.addr_dir();
                self.oper_acc(h);
            }
            0x63 => {
                self.addr_dir();
                self.oper_i08();
                self.r_cycles = 4;
            }
            0x64 => self.oper_i08(),
            0x65 => self.oper_dir(),
            0x66 | 0x67 => self.oper_ind(h),
            0x70 | 0x71 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x72 => {
                self.addr_bit(false);
                self.r_cycles = 4;
            }
            0x73 => self.r_cycles = 4,
            0x74 => self.oper_i08(),
            0x75 => {
                self.addr_dir();
                self.oper_i08();
                self.r_cycles = 4;
            }
            0x76 | 0x77 => {
                self.addr_ind(h);
                self.oper_i08();
            }
            0x80 | 0x81 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x82 => {
                self.addr_bit(false);
                self.r_cycles = 4;
            }
            0x83 => self.r_cycles = 4,
            0x84 => self.r_cycles = 8,
            0x85 => {
                self.oper_dir();
                self.addr_dir();
                self.r_cycles = 4;
            }
            0x86 | 0x87 => {
                self.addr_dir();
                self.oper_ind(h);
                self.r_cycles = 4;
            }
            0x90 => {
                self.addr_i16();
                self.r_cycles = 4;
            }
            0x91 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0x92 => {
                self.addr_bit(false);
                self.r_cycles = 4;
            }
            0x93 => self.r_cycles = 4,
            0x94 => self.oper_i08(),
            0x95 => self.oper_dir(),
            0x96 | 0x97 => self.oper_ind(h),
            0xA0 => {
                self.addr_bit(true);
                self.r_cycles = 4;
            }
            0xA1 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0xA2 => self.addr_bit(false),
            0xA3 => self.r_cycles = 4,
            0xA4 => self.r_cycles = 8,
            0xA5 => {}
            0xA6 | 0xA7 => {
                self.addr_ind(h);
                self.oper_dir();
                self.r_cycles = 4;
            }
            0xB0 => {
                self.addr_bit(true);
                self.r_cycles = 4;
            }
            0xB1 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0xB2 => self.addr_bit(false),
            0xB3 => {}
            0xB4 => {
                self.oper_acc(h);
                self.opr2_i08();
                self.addr_dir();
                self.r_cycles = 4;
            }
            0xB5 => {
                self.oper_acc(h);
                self.opr2_dir();
                self.addr_dir();
                self.r_cycles = 4;
            }
            0xB6 | 0xB7 => {
                self.oper_ind(h);
                self.opr2_i08();
                self.addr_dir();
            }
            0xC0 => {
                self.oper_dir();
                self.r_cycles = 4;
            }
            0xC1 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0xC2 => self.addr_bit(false),
            0xC3 | 0xC4 => {}
            0xC5 => self.addr_dir(),
            0xC6 | 0xC7 => self.addr_ind(h),
            0xD0 => {
                self.addr_dir();
                self.r_cycles = 4;
            }
            0xD1 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0xD2 => self.addr_bit(false),
            0xD3 | 0xD4 => {}
            0xD5 => {
                self.oper_i08();
                self.addr_dir();
                self.r_cycles = 4;
            }
            0xD6 | 0xD7 => self.addr_ind(h),
            0xE1 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0xE0 | 0xE2 | 0xE3 => self.r_cycles = 4,
            0xE4 => {}
            0xE5 => self.oper_dir(),
            0xE6 | 0xE7 => self.oper_ind(h),
            0xF1 => {
                self.addr_i08();
                self.r_cycles = 4;
            }
            0xF0 | 0xF2 | 0xF3 => self.r_cycles = 4,
            0xF4 => {}
            0xF5 => {
                self.addr_i08();
                self.oper_acc(h);
            }
            0xF6 | 0xF7 => {
                self.addr_ind(h);
                self.oper_acc(h);
            }
            _ => {}
        }
    }

    fn exec(&mut self, h: &mut CpuHost) {
        if self.opcode & 8 != 0 {
            self.rx_addr = (self.opcode & 0x07) + 8 * Self::bank(h);
            match self.opcode >> 4 {
                0x0 => self.inc(h),
                0x1 => self.dec(h),
                0x2 => self.add(h),
                0x3 => self.addc(h),
                0x4 => self.orl_a(h),
                0x5 => self.anl_a(h),
                0x6 => self.xrl_a(h),
                0x7 => self.mov_r(h),
                0x8 => self.mov_m(h),
                0x9 => self.subb(h),
                0xA => self.mov_r(h),
                0xB => self.cjne(h),
                0xC => self.xch_r(h),
                0xD => self.djnz(h),
                0xE => self.mov_a(h),
                0xF => self.mov_r(h),
                _ => {}
            }
        } else {
            match self.opcode {
                0x00 => {}
                0x01 => self.ajmp(h),
                0x02 => h.pc = u32::from(self.op_addr),
                0x03 => {
                    let a = self.acc(h);
                    self.set_acc(h, a.rotate_right(1));
                }
                0x04 => self.set_acc(h, self.acc(h).wrapping_add(1)),
                0x05 | 0x06 | 0x07 => self.inc(h),
                0x10 => self.jbc(h),
                0x11 => self.acall(h),
                0x12 => self.lcall(h),
                0x13 => self.rrc(h),
                0x14 => self.set_acc(h, self.acc(h).wrapping_sub(1)),
                0x15 | 0x16 | 0x17 => self.dec(h),
                0x20 => self.jb(h),
                0x21 => self.ajmp(h),
                0x22 => self.ret(h),
                0x23 => {
                    let a = self.acc(h);
                    self.set_acc(h, a.rotate_left(1));
                }
                0x24 | 0x25 | 0x26 | 0x27 => self.add(h),
                0x30 => self.jnb(h),
                0x31 => self.acall(h),
                0x32 => {
                    h.reti = true;
                    self.ret(h);
                }
                0x33 => self.rlc(h),
                0x34 | 0x35 | 0x36 | 0x37 => self.addc(h),
                0x40 => self.jc(h),
                0x41 => self.ajmp(h),
                0x42 | 0x43 => self.orl_m(h),
                0x44 | 0x45 | 0x46 | 0x47 => self.orl_a(h),
                0x50 => self.jnc(h),
                0x51 => self.acall(h),
                0x52 | 0x53 => self.anl_m(h),
                0x54 | 0x55 | 0x56 | 0x57 => self.anl_a(h),
                0x60 => self.jz(h),
                0x61 => self.ajmp(h),
                0x62 | 0x63 => self.xrl_m(h),
                0x64 | 0x65 | 0x66 | 0x67 => self.xrl_a(h),
                0x70 => self.jnz(h),
                0x71 => self.acall(h),
                0x72 => self.orl_c(h),
                0x73 => {
                    let dpl = h.get_reg16_lh(REG_DPL);
                    h.pc = u32::from(dpl) + u32::from(self.acc(h));
                }
                0x74 => self.mov_a(h),
                0x75 | 0x76 | 0x77 => self.mov_m(h),
                0x80 => h.pc = h.pc.wrapping_add_signed(i32::from(self.op_addr as i8)),
                0x81 => self.ajmp(h),
                0x82 => self.anl_c(h),
                0x83 => {
                    let addr = h.pc.wrapping_add(1).wrapping_add(u32::from(self.acc(h)));
                    self.set_acc(h, h.flash(addr) as u8);
                }
                0x84 => self.div_ab(h),
                0x85 | 0x86 | 0x87 => self.mov_m(h),
                0x90 => h.set_reg16_lh(REG_DPL, self.op_addr),
                0x91 => self.acall(h),
                0x92 => self.mov_bc(h),
                0x93 => {
                    let addr = u32::from(h.get_reg16_lh(REG_DPL)) + u32::from(self.acc(h));
                    self.set_acc(h, h.flash(addr) as u8);
                }
                0x94 | 0x95 | 0x96 | 0x97 => self.subb(h),
                0xA0 => self.orl_c(h),
                0xA1 => self.ajmp(h),
                0xA2 => self.mov_c(h),
                0xA3 => self.inc_dptr(h),
                0xA4 => self.mul_ab(h),
                0xA5 => h.pc = h.pc.wrapping_add(1),
                0xA6 | 0xA7 => {
                    let v = self.op0;
                    self.write_ind(h, self.op_addr, v);
                }
                0xB0 => self.anl_c(h),
                0xB1 => self.acall(h),
                0xB2 => self.cpl_b(h),
                0xB3 => {
                    let addr = h.data.sreg_addr;
                    let s = h.data.get(addr) ^ (1 << CY);
                    h.data.set(addr, s);
                }
                0xB4 | 0xB5 | 0xB6 | 0xB7 => self.cjne(h),
                0xC0 => self.push_stack8(h, self.op0),
                0xC1 => self.ajmp(h),
                0xC2 => self.clr_b(h),
                0xC3 => Self::write_s_bit(h, CY, false),
                0xC4 => {
                    let a = self.acc(h);
                    self.set_acc(h, a.rotate_left(4));
                }
                0xC5 | 0xC6 | 0xC7 => self.xch(h),
                0xD0 => {
                    let v = self.pop_stack8(h);
                    h.set_ram(self.op_addr, v);
                }
                0xD1 => self.acall(h),
                0xD2 => self.set_b(h),
                0xD3 => Self::write_s_bit(h, CY, true),
                0xD4 => self.da(h),
                0xD5 => self.djnz(h),
                0xD6 | 0xD7 => self.xchd(h),
                0xE0 => self.movx_a_dptr(h),
                0xE2 | 0xE3 => self.movx_a_ri(h),
                0xE1 => self.ajmp(h),
                0xE4 => self.set_acc(h, 0),
                0xE5 | 0xE6 | 0xE7 => self.mov_a(h),
                0xF0 => self.movx_dptr_a(h),
                0xF2 | 0xF3 => self.movx_ri_a(h),
                0xF1 => self.acall(h),
                0xF4 => self.set_acc(h, !self.acc(h)),
                0xF5 => self.mov_m(h),
                0xF6 | 0xF7 => {
                    let a = self.acc(h);
                    self.write_ind(h, self.op_addr, a);
                }
                _ => {}
            }
        }
        self.updt_parity(h);
    }

    fn ajmp(&self, h: &mut CpuHost) {
        h.pc = (h.pc & 0xF800) | u32::from(self.op_addr) | (u32::from(self.opcode & 0xE0) << 3);
    }
    fn acall(&mut self, h: &mut CpuHost) {
        h.ret_addr = h.pc;
        self.push_stack8(h, h.pc as u8);
        self.push_stack8(h, (h.pc >> 8) as u8);
        self.ajmp(h);
    }
    fn lcall(&mut self, h: &mut CpuHost) {
        h.ret_addr = h.pc;
        self.push_stack8(h, h.pc as u8);
        self.push_stack8(h, (h.pc >> 8) as u8);
        h.pc = u32::from(self.op_addr);
    }
    fn ret(&self, h: &mut CpuHost) {
        let hi = self.pop_stack8(h);
        let lo = self.pop_stack8(h);
        h.pc = (u32::from(hi) << 8) | u32::from(lo);
    }

    /// C++ `I51Core::INTERRUPT`.
    pub fn interrupt(&mut self, h: &mut CpuHost, vector: u32) {
        self.push_stack8(h, h.pc as u8);
        self.push_stack8(h, (h.pc >> 8) as u8);
        h.pc = vector & 0xFFFF;
        self.pc = h.pc as u16;
        self.read_pc = self.pc;
        self.last_pc = self.pc;
        self.state = CpuState::Fetch;
    }
    fn rel(h: &mut CpuHost, off: u16) {
        h.pc = h.pc.wrapping_add_signed(i32::from(off as i8));
    }
    fn jbc(&mut self, h: &mut CpuHost) {
        let v = h.get_ram(u16::from(self.bit_addr));
        if v & self.bit_mask != 0 {
            h.set_ram(u16::from(self.bit_addr), v & !self.bit_mask);
            Self::rel(h, self.op_addr);
        }
    }
    fn jb(&mut self, h: &mut CpuHost) {
        if h.get_ram(u16::from(self.bit_addr)) & self.bit_mask != 0 {
            Self::rel(h, self.op_addr);
        }
    }
    fn jnb(&mut self, h: &mut CpuHost) {
        if h.get_ram(u16::from(self.bit_addr)) & self.bit_mask == 0 {
            Self::rel(h, self.op_addr);
        }
    }
    fn jc(&self, h: &mut CpuHost) {
        if Self::status(h, CY) {
            Self::rel(h, self.op_addr);
        }
    }
    fn jnc(&self, h: &mut CpuHost) {
        if !Self::status(h, CY) {
            Self::rel(h, self.op_addr);
        }
    }
    fn jz(&self, h: &mut CpuHost) {
        if self.acc(h) == 0 {
            Self::rel(h, self.op_addr);
        }
    }
    fn jnz(&self, h: &mut CpuHost) {
        if self.acc(h) != 0 {
            Self::rel(h, self.op_addr);
        }
    }
    fn rrc(&self, h: &mut CpuHost) {
        let c = Self::status(h, CY);
        let a = self.acc(h);
        Self::write_s_bit(h, CY, a & 1 != 0);
        self.set_acc(h, (a >> 1) | if c { 0x80 } else { 0 });
    }
    fn rlc(&self, h: &mut CpuHost) {
        let a = self.acc(h);
        let newc = a & (1 << 7) != 0;
        let mut n = a << 1;
        if Self::status(h, CY) {
            n += 1;
        }
        self.set_acc(h, n);
        Self::write_s_bit(h, CY, newc);
    }
    fn inc(&self, h: &mut CpuHost) {
        let v = h.data.get(self.op_addr).wrapping_add(1);
        h.set_ram(self.op_addr, v);
    }
    fn dec(&self, h: &mut CpuHost) {
        let v = h.data.get(self.op_addr).wrapping_sub(1);
        h.set_ram(self.op_addr, v);
    }
    fn inc_dptr(&self, h: &mut CpuHost) {
        let lo = h.data.get(REG_DPL).wrapping_add(1);
        h.set_ram(REG_DPL, lo);
        if lo == 0 {
            let hi = h.data.get(REG_DPH).wrapping_add(1);
            h.set_ram(REG_DPH, hi);
        }
    }
    fn add(&self, h: &mut CpuHost) {
        let a = self.acc(h);
        Self::add_flags(h, self.op0, a, 0);
        self.set_acc(h, a.wrapping_add(self.op0));
    }
    fn addc(&self, h: &mut CpuHost) {
        let carry = u8::from(Self::status(h, CY));
        let a = self.acc(h);
        Self::add_flags(h, self.op0, a, carry);
        self.set_acc(h, a.wrapping_add(self.op0).wrapping_add(carry));
    }
    fn subb(&self, h: &mut CpuHost) {
        let carry = u8::from(Self::status(h, CY));
        let a = self.acc(h);
        Self::sub_flags(h, a, self.op0, carry);
        self.set_acc(h, a.wrapping_sub(self.op0).wrapping_sub(carry));
    }
    fn orl_m(&self, h: &mut CpuHost) {
        h.set_ram(self.op_addr, h.data.get(self.op_addr) | self.op0);
    }
    fn anl_m(&self, h: &mut CpuHost) {
        h.set_ram(self.op_addr, h.data.get(self.op_addr) & self.op0);
    }
    fn xrl_m(&self, h: &mut CpuHost) {
        h.set_ram(self.op_addr, h.data.get(self.op_addr) ^ self.op0);
    }
    fn orl_a(&self, h: &mut CpuHost) {
        self.set_acc(h, self.acc(h) | self.op0);
    }
    fn anl_a(&self, h: &mut CpuHost) {
        self.set_acc(h, self.acc(h) & self.op0);
    }
    fn xrl_a(&self, h: &mut CpuHost) {
        self.set_acc(h, self.acc(h) ^ self.op0);
    }
    fn mov_a(&self, h: &mut CpuHost) {
        self.set_acc(h, self.op0);
    }
    fn mov_r(&self, h: &mut CpuHost) {
        h.data.set(self.op_addr, self.op0);
    }
    fn mov_m(&self, h: &mut CpuHost) {
        h.set_ram(self.op_addr, self.op0);
    }
    fn xch(&self, h: &mut CpuHost) {
        let a = self.acc(h);
        let v = h.get_ram(self.op_addr);
        self.set_acc(h, v);
        h.set_ram(self.op_addr, a);
    }
    fn xch_r(&self, h: &mut CpuHost) {
        let a = self.acc(h);
        self.set_acc(h, h.data.get(self.op_addr));
        h.data.set(self.op_addr, a);
    }
    fn xchd(&self, h: &mut CpuHost) {
        let address = h.check_addr(self.op_addr);
        let value = h.data.get(address);
        let a = self.acc(h);
        h.data.set(address, (value & 0xF0) | (a & 0x0F));
        self.set_acc(h, (a & 0xF0) | (value & 0x0F));
    }
    fn cjne(&self, h: &mut CpuHost) {
        Self::write_s_bit(h, CY, self.op0 < self.op2);
        if self.op0 != self.op2 {
            Self::rel(h, self.op_addr);
        }
    }
    fn djnz(&self, h: &mut CpuHost) {
        let value = h.get_ram(u16::from(self.op0)).wrapping_sub(1);
        h.set_ram(u16::from(self.op0), value);
        if value != 0 {
            Self::rel(h, self.op_addr);
        }
    }
    fn mov_bc(&mut self, h: &mut CpuHost) {
        let carry = Self::status(h, CY);
        let cur = h.get_ram(u16::from(self.bit_addr));
        let value = if carry {
            cur | self.bit_mask
        } else {
            cur & !self.bit_mask
        };
        h.set_ram(u16::from(self.bit_addr), value);
    }
    fn mov_c(&mut self, h: &mut CpuHost) {
        let value = h.get_ram(u16::from(self.bit_addr)) & self.bit_mask != 0;
        Self::write_s_bit(h, CY, value);
    }
    fn orl_c(&mut self, h: &mut CpuHost) {
        let carry = Self::status(h, CY);
        let bit = h.get_ram(u16::from(self.bit_addr)) & self.bit_mask != 0;
        let value = if self.invert {
            if bit { true } else { carry }
        } else if bit {
            carry
        } else {
            true
        };
        Self::write_s_bit(h, CY, value);
    }
    fn anl_c(&mut self, h: &mut CpuHost) {
        let carry = Self::status(h, CY);
        let bit = h.get_ram(u16::from(self.bit_addr)) & self.bit_mask != 0;
        let value = if self.invert {
            if bit { false } else { carry }
        } else if bit {
            carry
        } else {
            false
        };
        Self::write_s_bit(h, CY, value);
    }
    fn clr_b(&mut self, h: &mut CpuHost) {
        let v = h.data.get(u16::from(self.bit_addr)) & !self.bit_mask;
        h.set_ram(u16::from(self.bit_addr), v);
    }
    fn set_b(&mut self, h: &mut CpuHost) {
        let v = h.data.get(u16::from(self.bit_addr)) | self.bit_mask;
        h.set_ram(u16::from(self.bit_addr), v);
    }
    fn cpl_b(&mut self, h: &mut CpuHost) {
        let v = h.data.get(u16::from(self.bit_addr)) ^ self.bit_mask;
        h.set_ram(u16::from(self.bit_addr), v);
    }
    fn mul_ab(&self, h: &mut CpuHost) {
        let res = u16::from(self.acc(h)) * u16::from(h.data.get(REG_B));
        self.set_acc(h, res as u8);
        h.set_ram(REG_B, (res >> 8) as u8);
        Self::write_s_bit(h, OV, false);
        Self::write_s_bit(h, CY, false);
    }
    fn div_ab(&self, h: &mut CpuHost) {
        let a = u16::from(self.acc(h));
        let b = u16::from(h.data.get(REG_B));
        if b != 0 {
            self.set_acc(h, (a / b) as u8);
            h.set_ram(REG_B, (a % b) as u8);
            Self::write_s_bit(h, OV, false);
        } else {
            Self::write_s_bit(h, OV, true);
        }
        Self::write_s_bit(h, CY, false);
    }
    fn da(&self, h: &mut CpuHost) {
        let mut acc = self.acc(h);
        let mut al = acc & 0x0F;
        if al > 0x09 || Self::status(h, AC) {
            al = al.wrapping_add(6);
            acc = acc.wrapping_add(6);
            if al & (1 << 4) != 0 {
                Self::write_s_bit(h, CY, true);
            }
        }
        let mut ah = (acc & 0xF0) >> 4;
        if ah > 0x09 || Self::status(h, AC) {
            ah = ah.wrapping_add(0x06);
            if ah & (1 << 4) != 0 {
                Self::write_s_bit(h, CY, true);
            }
            acc = (al & 0x0F) | (ah << 4);
        }
        self.set_acc(h, acc);
    }

    fn xram_addr_dptr(&self, h: &CpuHost) -> u16 {
        h.get_reg16_lh(REG_DPL)
    }

    fn xram_addr_ri(&self, h: &CpuHost) -> u16 {
        u16::from(self.rx_val(h))
    }

    fn xram_get(&self, addr: u16) -> u8 {
        self.xram.get(addr as usize).copied().unwrap_or(0xFF)
    }

    fn xram_set(&mut self, addr: u16, v: u8) {
        if let Some(slot) = self.xram.get_mut(addr as usize) {
            *slot = v;
        }
    }

    fn movx_a_dptr(&mut self, h: &mut CpuHost) {
        let addr = self.xram_addr_dptr(h);
        self.set_acc(h, self.xram_get(addr));
        self.start_movx_bus(h, addr, true, 0);
    }

    fn movx_a_ri(&mut self, h: &mut CpuHost) {
        let addr = self.xram_addr_ri(h);
        self.set_acc(h, self.xram_get(addr));
        self.start_movx_bus(h, addr, true, 0);
    }

    fn movx_dptr_a(&mut self, h: &mut CpuHost) {
        let addr = self.xram_addr_dptr(h);
        let a = self.acc(h);
        self.xram_set(addr, a);
        self.start_movx_bus(h, addr, false, a);
    }

    fn movx_ri_a(&mut self, h: &mut CpuHost) {
        let addr = self.xram_addr_ri(h);
        let a = self.acc(h);
        self.xram_set(addr, a);
        self.start_movx_bus(h, addr, false, a);
    }

    /// Drive PORT0/PORT2 for MOVX when not already fetching external program.
    fn start_movx_bus(&mut self, h: &mut CpuHost, addr: u16, read: bool, data: u8) {
        if self.ext_pgm {
            return;
        }
        if port_idx(&h.ports, "PORT0").is_none() {
            return;
        }
        if let Some(p) = port_named(&mut h.ports, "PORT0") {
            p.control_port(true, true);
        }
        if let Some(p) = port_named(&mut h.ports, "PORT2") {
            p.control_port(true, true);
        }
        self.mem_read = read;
        self.addr = u32::from(addr);
        self.addr_h = (addr >> 8) as u8;
        if !read {
            self.bus_data = data;
        }
        self.mem_state = MemState::Laen;
        self.run_mem(h);
    }
}
