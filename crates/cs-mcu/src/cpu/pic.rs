//! PIC mid-range (C++ `PicMrCore` / `Pic12Core` / `Pic14Core`).

use crate::dataspace::{DataSpace, RegBits};
use crate::device::CpuHost;

/// STATUS bits, C++ `enum { C, DC, Z, PD, TO, RP0, RP1, IRP }`.
pub const C: u8 = 0;
pub const DC: u8 = 1;
pub const Z: u8 = 2;
pub const PD: u8 = 3;
pub const TO: u8 = 4;
pub const IRP: u8 = 7;

#[derive(Clone, Debug)]
pub struct PicMr {
    pub w: u8,
    pub option_addr: Option<u16>,
    pub fsr_addr: Option<u16>,
    pub pcl_addr: Option<u16>,
    pub pclath_addr: Option<u16>,
    pub bank: u16,
    pub bank_bits: RegBits,
    pub stack: [u32; 8],
    pub sp: u8,
    pub stack_size: u8,
    pub tris_addr: Option<u16>,
    pub pending_option: bool,
}

impl PicMr {
    pub fn new(data: &DataSpace, stack_size: u8) -> Self {
        Self {
            w: 0,
            option_addr: data.reg_addr("OPTION"),
            fsr_addr: data.reg_addr("FSR"),
            pcl_addr: data.reg_addr("PCL"),
            pclath_addr: data.reg_addr("PCLATH"),
            bank: 0,
            bank_bits: data.get_reg_bits("R0,R1"),
            stack: [0; 8],
            sp: 0,
            stack_size,
            tris_addr: data.reg_addr("TRISGPIO").or_else(|| data.reg_addr("TRIS")),
            pending_option: false,
        }
    }

    pub fn reset(&mut self) {
        self.w = 0;
        self.sp = 0;
        self.bank = 0;
        self.stack = [0; 8];
        self.pending_option = false;
    }

    pub fn set_bank(&mut self, val: u8) {
        self.bank = u16::from(self.bank_bits.val(val)) << 7;
    }

    fn status(h: &CpuHost) -> u8 {
        h.data.get(h.data.sreg_addr)
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

    fn pclath(&self, h: &CpuHost) -> u8 {
        self.pclath_addr.map(|a| h.data.get(a)).unwrap_or(0)
    }

    fn set_pc(h: &mut CpuHost, pcl_addr: Option<u16>, pc: u32) {
        h.pc = pc & 0x0000_1FFF;
        if let Some(a) = pcl_addr {
            h.data.set(a, (h.pc & 0xFF) as u8);
        }
    }

    fn push_stack(&mut self, addr: u32) {
        self.stack[self.sp as usize] = addr;
        self.sp += 1;
        if self.sp == self.stack_size {
            self.sp = 0;
        }
    }

    fn pop_stack(&mut self) -> u32 {
        if self.sp == 0 {
            self.sp = self.stack_size.saturating_sub(1);
        } else {
            self.sp -= 1;
        }
        self.stack[self.sp as usize]
    }

    fn inc_default(h: &mut CpuHost, pcl_addr: Option<u16>) {
        Self::set_pc(h, pcl_addr, h.pc.wrapping_add(1));
        h.cycles_done = h.cycles_done.saturating_add(1);
    }

    fn add(h: &mut CpuHost, val1: u8, val2: u8) -> u8 {
        let newv = u16::from(val1) + u16::from(val2);
        Self::write_s_bit(h, Z, (newv & 0xFF) == 0);
        Self::write_s_bit(h, C, newv & 0x100 != 0);
        Self::write_s_bit(h, DC, (val1 & 0xF) + (val2 & 0xF) > 0x0F);
        newv as u8
    }

    fn sub(h: &mut CpuHost, val1: u8, val2: u8) -> u8 {
        let newv = i16::from(val1) - i16::from(val2);
        Self::write_s_bit(h, Z, (newv as u8) == 0);
        Self::write_s_bit(h, C, newv >= 0);
        Self::write_s_bit(h, DC, i16::from(val1 & 0xF) - i16::from(val2 & 0xF) >= 0);
        newv as u8
    }

    fn get_indf(&self, h: &CpuHost) -> u16 {
        let mut addr = u16::from(self.fsr_addr.map(|a| h.data.get(a)).unwrap_or(0));
        if Self::status(h) & (1 << IRP) != 0 {
            addr |= 1 << 8;
        }
        addr
    }
}

#[derive(Clone, Debug)]
pub struct Pic14 {
    pub mr: PicMr,
}

impl Pic14 {
    pub fn new(data: &DataSpace) -> Self {
        Self {
            mr: PicMr::new(data, 8),
        }
    }

    fn map(&self, h: &CpuHost, addr: u16) -> u16 {
        h.data.mapper_addr(addr.wrapping_add(self.mr.bank))
    }

    fn get_ram(&self, h: &mut CpuHost, f: u8) -> u8 {
        let mut addr = self.map(h, u16::from(f));
        if addr == 0 {
            addr = self.mr.get_indf(h);
        }
        h.get_ram(addr)
    }
}

impl Pic14 {
    fn set_ram(&mut self, h: &mut CpuHost, f: u8, v: u8) {
        let mut addr = self.map(h, u16::from(f));
        if Some(addr) == self.mr.pcl_addr {
            let pch = u32::from(self.mr.pclath(h)) << 8;
            PicMr::set_pc(h, self.mr.pcl_addr, u32::from(v) + pch);
            return;
        }
        if addr == 0 {
            addr = self.mr.get_indf(h);
        }
        h.set_ram(addr, v);
    }

    pub fn run_step(&mut self, h: &mut CpuHost) {
        let instr = h.flash(h.pc) & 0x3FFF;
        h.cycles_done = 0;
        PicMr::inc_default(h, self.mr.pcl_addr);
        h.ret_addr = h.pc;
        self.decode(h, instr);
    }

    fn decode(&mut self, h: &mut CpuHost, instr: u16) {
        if instr & 0x3F80 == 0 {
            match instr & 0x000C {
                0x0008 => {
                    if instr == 0x0008 {
                        self.ret(h);
                    } else if instr == 0x0009 {
                        self.reti(h);
                    }
                }
                0x0000 => {
                    if instr == 0x0062 {
                        self.option();
                    } else if instr == 0x0063 {
                        self.sleep(h);
                    }
                }
                0x0004 => {
                    if instr == 0x0064 {
                        self.clrwdt(h);
                    }
                }
                _ => {}
            }
            return;
        }
        if instr & 0x3000 == 0 {
            let f = (instr & 0x7F) as u8;
            let d = ((instr >> 7) & 1) as u8;
            if instr & 0x3800 == 0 {
                match instr & 0x0700 {
                    0x0000 => self.movwf(h, f),
                    0x0100 => self.clr(h, f, d),
                    0x0200 => self.subwf(h, f, d),
                    0x0300 => self.decf(h, f, d),
                    0x0400 => self.iorwf(h, f, d),
                    0x0500 => self.andwf(h, f, d),
                    0x0600 => self.xorwf(h, f, d),
                    0x0700 => self.addwf(h, f, d),
                    _ => {}
                }
            } else {
                match instr & 0x0700 {
                    0x0000 => self.movf(h, f, d),
                    0x0100 => self.comf(h, f, d),
                    0x0200 => self.incf(h, f, d),
                    0x0300 => self.decfsz(h, f, d),
                    0x0400 => self.rrf(h, f, d),
                    0x0500 => self.rlf(h, f, d),
                    0x0600 => self.swapf(h, f, d),
                    0x0700 => self.incfsz(h, f, d),
                    _ => {}
                }
            }
            return;
        }
        if instr & 0x3000 == 0x1000 {
            let f = (instr & 0x7F) as u8;
            let b = ((instr >> 7) & 7) as u8;
            match instr & 0x3C00 {
                0x1000 => self.bcf(h, f, b),
                0x1400 => self.bsf(h, f, b),
                0x1800 => self.btfsc(h, f, b),
                0x1C00 => self.btfss(h, f, b),
                _ => {}
            }
        } else if instr & 0x3000 == 0x2000 {
            let k = instr & 0x07FF;
            if instr & 0x0800 == 0 {
                self.call(h, k);
            } else {
                self.goto(h, k);
            }
        } else if instr & 0x3000 == 0x3000 {
            let k = (instr & 0xFF) as u8;
            match instr & 0x3C00 {
                0x3000 => self.mr.w = k,
                0x3400 => {
                    self.mr.w = k;
                    self.ret(h);
                }
                0x3800 => match instr & 0x3F00 {
                    0x3800 => {
                        self.mr.w |= k;
                        PicMr::write_s_bit(h, Z, self.mr.w == 0);
                    }
                    0x3900 => {
                        self.mr.w &= k;
                        PicMr::write_s_bit(h, Z, self.mr.w == 0);
                    }
                    0x3A00 => {
                        self.mr.w ^= k;
                        PicMr::write_s_bit(h, Z, self.mr.w == 0);
                    }
                    _ => {}
                },
                0x3C00 => {
                    if instr & 0x0200 == 0 {
                        self.mr.w = PicMr::sub(h, k, self.mr.w);
                    } else {
                        self.mr.w = PicMr::add(h, k, self.mr.w);
                    }
                }
                _ => {}
            }
        }
    }

    fn movwf(&mut self, h: &mut CpuHost, f: u8) {
        let w = self.mr.w;
        self.set_ram(h, f, w);
    }
    fn clr(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        self.set_value(h, 0, f, d);
        PicMr::write_s_bit(h, Z, true);
    }
    fn subwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let a = self.get_ram(h, f);
        let w = self.mr.w;
        let new_v = PicMr::sub(h, a, w);
        self.set_value(h, new_v, f, d);
    }
    fn decf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_sub(1);
        self.set_value_z(h, new_v, f, d);
    }
    fn iorwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) | self.mr.w;
        self.set_value_z(h, new_v, f, d);
    }
    fn andwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) & self.mr.w;
        self.set_value_z(h, new_v, f, d);
    }
    fn xorwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) ^ self.mr.w;
        self.set_value_z(h, new_v, f, d);
    }
    fn addwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let a = self.get_ram(h, f);
        let w = self.mr.w;
        let new_v = PicMr::add(h, a, w);
        self.set_value(h, new_v, f, d);
    }
    fn movf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f);
        self.set_value_z(h, new_v, f, d);
    }
    fn comf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) ^ 0xFF;
        self.set_value_z(h, new_v, f, d);
    }
    fn incf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_add(1);
        self.set_value_z(h, new_v, f, d);
    }
    fn decfsz(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_sub(1);
        self.set_value(h, new_v, f, d);
        if new_v == 0 {
            PicMr::inc_default(h, self.mr.pcl_addr);
        }
    }
    fn rrf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let old = self.get_ram(h, f);
        let mut new_v = old >> 1;
        if PicMr::status(h) & (1 << C) != 0 {
            new_v |= 1 << 7;
        }
        PicMr::write_s_bit(h, C, old & 1 != 0);
        self.set_value(h, new_v, f, d);
    }
    fn rlf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let old = self.get_ram(h, f);
        let mut new_v = old << 1;
        if PicMr::status(h) & (1 << C) != 0 {
            new_v |= 1;
        }
        PicMr::write_s_bit(h, C, old & (1 << 7) != 0);
        self.set_value(h, new_v, f, d);
    }
    fn swapf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let old = self.get_ram(h, f);
        let new_v = ((old >> 4) & 0x0F) | ((old << 4) & 0xF0);
        self.set_value(h, new_v, f, d);
    }
    fn incfsz(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_add(1);
        self.set_value(h, new_v, f, d);
        if new_v == 0 {
            PicMr::inc_default(h, self.mr.pcl_addr);
        }
    }
    fn bcf(&mut self, h: &mut CpuHost, f: u8, b: u8) {
        let v = self.get_ram(h, f) & !(1 << b);
        self.set_ram(h, f, v);
    }
    fn bsf(&mut self, h: &mut CpuHost, f: u8, b: u8) {
        let v = self.get_ram(h, f) | (1 << b);
        self.set_ram(h, f, v);
    }
    fn btfsc(&mut self, h: &mut CpuHost, f: u8, b: u8) {
        if self.get_ram(h, f) & (1 << b) == 0 {
            PicMr::inc_default(h, self.mr.pcl_addr);
        }
    }
    fn btfss(&mut self, h: &mut CpuHost, f: u8, b: u8) {
        if self.get_ram(h, f) & (1 << b) != 0 {
            PicMr::inc_default(h, self.mr.pcl_addr);
        }
    }
    fn call(&mut self, h: &mut CpuHost, k: u16) {
        let dest = u32::from(k) | (u32::from(self.mr.pclath(h) & 0b0001_1000) << 8);
        self.call_addr(h, dest);
    }
    fn goto(&mut self, h: &mut CpuHost, k: u16) {
        let dest = u32::from(k) | (u32::from(self.mr.pclath(h) & 0b0001_1000) << 8);
        PicMr::set_pc(h, self.mr.pcl_addr, dest);
        h.cycles_done = u32::from(h.ret_cycles);
    }
    fn call_addr(&mut self, h: &mut CpuHost, addr: u32) {
        self.mr.push_stack(h.pc);
        PicMr::set_pc(h, self.mr.pcl_addr, addr);
        h.cycles_done = u32::from(h.ret_cycles);
    }
    fn ret(&mut self, h: &mut CpuHost) {
        let a = self.mr.pop_stack();
        PicMr::set_pc(h, self.mr.pcl_addr, a);
        h.cycles_done = u32::from(h.ret_cycles);
    }
    fn reti(&mut self, h: &mut CpuHost) {
        h.reti = true;
        self.ret(h);
    }
    pub fn interrupt(&mut self, h: &mut CpuHost, vector: u32) {
        self.call_addr(h, vector);
    }
    fn option(&mut self) {
        self.mr.pending_option = true;
    }
    fn sleep(&mut self, h: &mut CpuHost) {
        PicMr::write_s_bit(h, PD, false);
        PicMr::write_s_bit(h, TO, true);
        h.sleep = true;
    }
    fn clrwdt(&mut self, h: &mut CpuHost) {
        PicMr::write_s_bit(h, PD, true);
        PicMr::write_s_bit(h, TO, true);
        h.wdr = true;
    }

    fn set_value(&mut self, h: &mut CpuHost, new_v: u8, f: u8, d: u8) {
        if d != 0 {
            self.set_ram(h, f, new_v);
        } else {
            self.mr.w = new_v;
        }
    }
    fn set_value_z(&mut self, h: &mut CpuHost, new_v: u8, f: u8, d: u8) {
        self.set_value(h, new_v, f, d);
        PicMr::write_s_bit(h, Z, new_v == 0);
    }
}

impl Pic14 {
    pub fn take_option(&mut self) -> Option<u8> {
        if self.mr.pending_option {
            self.mr.pending_option = false;
            Some(self.mr.w)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pic12 {
    pub mr: PicMr,
    pending_tris: bool,
}

impl Pic12 {
    pub fn new(data: &DataSpace) -> Self {
        let mut mr = PicMr::new(data, 8);
        mr.w = 0;
        Self {
            mr,
            pending_tris: false,
        }
    }

    pub fn take_option(&mut self) -> Option<u8> {
        if self.mr.pending_option {
            self.mr.pending_option = false;
            Some(self.mr.w)
        } else {
            None
        }
    }

    pub fn take_tris(&mut self) -> Option<u8> {
        if self.pending_tris {
            self.pending_tris = false;
            Some(self.mr.w)
        } else {
            None
        }
    }

    pub fn run_step(&mut self, h: &mut CpuHost) {
        let instr = h.flash(h.pc) & 0x0FFF;
        h.cycles_done = 0;
        PicMr::inc_default(h, self.mr.pcl_addr);
        if instr != 0 {
            self.decode(h, instr);
        }
    }

    fn decode(&mut self, h: &mut CpuHost, instr: u16) {
        if instr & 0xFF8 == 0 {
            if instr == 0x02 {
                self.mr.pending_option = true;
            } else if instr == 0x03 {
                PicMr::write_s_bit(h, PD, false);
                PicMr::write_s_bit(h, TO, true);
                h.sleep = true;
            } else if instr == 0x04 {
                PicMr::write_s_bit(h, PD, true);
                PicMr::write_s_bit(h, TO, true);
                h.wdr = true;
            } else if instr == 0x06 {
                self.pending_tris = true;
            }
            return;
        }
        if instr & 0xC00 == 0 {
            let f = (instr & 0x1F) as u8;
            let d = ((instr >> 5) & 1) as u8;
            if instr & 0xE00 == 0 {
                match instr & 0x1C0 {
                    0x000 => self.movwf(h, f),
                    0x040 => self.clr(h, f),
                    0x080 => self.subwf(h, f, d),
                    0x0C0 => self.decf(h, f, d),
                    0x100 => self.iorwf(h, f, d),
                    0x140 => self.andwf(h, f, d),
                    0x180 => self.xorwf(h, f, d),
                    0x1C0 => self.addwf(h, f, d),
                    _ => {}
                }
            } else {
                match instr & 0x1C0 {
                    0x000 => self.movf(h, f, d),
                    0x040 => self.comf(h, f, d),
                    0x080 => self.incf(h, f, d),
                    0x0C0 => self.decfsz(h, f, d),
                    0x100 => self.rrf(h, f, d),
                    0x140 => self.rlf(h, f, d),
                    0x180 => self.swapf(h, f, d),
                    0x1C0 => self.incfsz(h, f, d),
                    _ => {}
                }
            }
            return;
        }
        if instr & 0xC00 == 0x400 {
            let f = (instr & 0x1F) as u8;
            let b = ((instr >> 5) & 7) as u8;
            match instr & 0xF00 {
                0x400 => {
                    let v = self.get_ram(h, f) & !(1 << b);
                    self.set_ram(h, f, v);
                }
                0x500 => {
                    let v = self.get_ram(h, f) | (1 << b);
                    self.set_ram(h, f, v);
                }
                0x600 => {
                    if self.get_ram(h, f) & (1 << b) == 0 {
                        PicMr::inc_default(h, self.mr.pcl_addr);
                    }
                }
                0x700 => {
                    if self.get_ram(h, f) & (1 << b) != 0 {
                        PicMr::inc_default(h, self.mr.pcl_addr);
                    }
                }
                _ => {}
            }
        } else if instr & 0xE00 == 0x800 {
            let k = instr & 0x0FF;
            if instr & 0x100 == 0x100 {
                self.call(h, k);
            } else {
                self.mr.w = k as u8;
                self.ret(h);
            }
        } else if instr & 0xE00 == 0xA00 {
            let k = instr & 0x1FF;
            self.goto(h, k);
        } else if instr & 0xC00 == 0xC00 {
            let k = (instr & 0x0FF) as u8;
            match instr & 0xF00 {
                0xC00 => self.mr.w = k,
                0xD00 => {
                    self.mr.w |= k;
                    PicMr::write_s_bit(h, Z, self.mr.w == 0);
                }
                0xE00 => {
                    self.mr.w &= k;
                    PicMr::write_s_bit(h, Z, self.mr.w == 0);
                }
                0xF00 => {
                    self.mr.w ^= k;
                    PicMr::write_s_bit(h, Z, self.mr.w == 0);
                }
                _ => {}
            }
        }
    }

    fn get_ram(&mut self, h: &mut CpuHost, f: u8) -> u8 {
        h.get_ram(u16::from(f))
    }
    fn set_ram(&mut self, h: &mut CpuHost, f: u8, v: u8) {
        h.set_ram(u16::from(f), v);
    }
    fn set_value(&mut self, h: &mut CpuHost, new_v: u8, f: u8, d: u8) {
        if d != 0 {
            self.set_ram(h, f, new_v);
        } else {
            self.mr.w = new_v;
        }
    }
    fn set_value_z(&mut self, h: &mut CpuHost, new_v: u8, f: u8, d: u8) {
        self.set_value(h, new_v, f, d);
        PicMr::write_s_bit(h, Z, new_v == 0);
    }
    fn movwf(&mut self, h: &mut CpuHost, f: u8) {
        let w = self.mr.w;
        self.set_ram(h, f, w);
    }
    fn clr(&mut self, h: &mut CpuHost, f: u8) {
        self.set_value(h, 0, f, 1);
        PicMr::write_s_bit(h, Z, true);
    }
    fn subwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let a = self.get_ram(h, f);
        let w = self.mr.w;
        let new_v = PicMr::sub(h, a, w);
        self.set_value(h, new_v, f, d);
    }
    fn decf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_sub(1);
        self.set_value_z(h, new_v, f, d);
    }
    fn iorwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) | self.mr.w;
        self.set_value_z(h, new_v, f, d);
    }
    fn andwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) & self.mr.w;
        self.set_value_z(h, new_v, f, d);
    }
    fn xorwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) ^ self.mr.w;
        self.set_value_z(h, new_v, f, d);
    }
    fn addwf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let a = self.get_ram(h, f);
        let w = self.mr.w;
        let new_v = PicMr::add(h, a, w);
        self.set_value(h, new_v, f, d);
    }
    fn movf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f);
        self.set_value_z(h, new_v, f, d);
    }
    fn comf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f) ^ 0xFF;
        self.set_value_z(h, new_v, f, d);
    }
    fn incf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_add(1);
        self.set_value_z(h, new_v, f, d);
    }
    fn decfsz(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_sub(1);
        self.set_value(h, new_v, f, d);
        if new_v == 0 {
            PicMr::inc_default(h, self.mr.pcl_addr);
        }
    }
    fn rrf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let old = self.get_ram(h, f);
        let mut new_v = old >> 1;
        if PicMr::status(h) & (1 << C) != 0 {
            new_v |= 1 << 7;
        }
        PicMr::write_s_bit(h, C, old & 1 != 0);
        self.set_value(h, new_v, f, d);
    }
    fn rlf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let old = self.get_ram(h, f);
        let mut new_v = old << 1;
        if PicMr::status(h) & (1 << C) != 0 {
            new_v |= 1;
        }
        PicMr::write_s_bit(h, C, old & (1 << 7) != 0);
        self.set_value(h, new_v, f, d);
    }
    fn swapf(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let old = self.get_ram(h, f);
        let new_v = ((old >> 4) & 0x0F) | ((old << 4) & 0xF0);
        self.set_value(h, new_v, f, d);
    }
    fn incfsz(&mut self, h: &mut CpuHost, f: u8, d: u8) {
        let new_v = self.get_ram(h, f).wrapping_add(1);
        self.set_value(h, new_v, f, d);
        if new_v == 0 {
            PicMr::inc_default(h, self.mr.pcl_addr);
        }
    }
    fn call(&mut self, h: &mut CpuHost, k: u16) {
        self.mr.push_stack(h.pc);
        PicMr::set_pc(h, self.mr.pcl_addr, u32::from(k));
        h.cycles_done = u32::from(h.ret_cycles);
    }
    fn goto(&mut self, h: &mut CpuHost, k: u16) {
        PicMr::set_pc(h, self.mr.pcl_addr, u32::from(k));
        h.cycles_done = u32::from(h.ret_cycles);
    }
    fn ret(&mut self, h: &mut CpuHost) {
        let a = self.mr.pop_stack();
        PicMr::set_pc(h, self.mr.pcl_addr, a);
        h.cycles_done = u32::from(h.ret_cycles);
    }
    pub fn interrupt(&mut self, h: &mut CpuHost, vector: u32) {
        self.mr.push_stack(h.pc);
        PicMr::set_pc(h, self.mr.pcl_addr, vector);
        h.cycles_done = u32::from(h.ret_cycles);
    }
}
