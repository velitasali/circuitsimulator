//! AVR 8-bit core (C++ `AvrCore`, based on the simavr decoder).

use crate::dataspace::{DataSpace, RegBits};
use crate::device::CpuHost;

pub const S_C: u8 = 0;
pub const S_Z: u8 = 1;
pub const S_N: u8 = 2;
pub const S_V: u8 = 3;
pub const S_S: u8 = 4;
pub const S_H: u8 = 5;
pub const S_T: u8 = 6;
pub const S_I: u8 = 7;

pub const R_XL: u8 = 0x1A;
#[allow(dead_code)]
pub const R_XH: u8 = 0x1B;
pub const R_YL: u8 = 0x1C;
#[allow(dead_code)]
pub const R_YH: u8 = 0x1D;
pub const R_ZL: u8 = 0x1E;
#[allow(dead_code)]
pub const R_ZH: u8 = 0x1F;

#[derive(Clone, Debug)]
pub struct Avr {
    eind_addr: Option<u16>,
    rampz_addr: Option<u16>,
    boot_start: u32,
    page_size: usize,
    tmp_used: Vec<u8>,
    tmp_page: Vec<u16>,
    selfprgen: RegBits,
    pgers: RegBits,
    pgwrt: RegBits,
}

impl Avr {
    pub fn new(data: &DataSpace, page_size: u8) -> Self {
        let page = page_size as usize;
        Self {
            eind_addr: data.reg_addr("EIND"),
            rampz_addr: data.reg_addr("RAMPZ"),
            boot_start: 0,
            page_size: page,
            tmp_used: vec![0; page],
            tmp_page: vec![0; page],
            selfprgen: data.get_reg_bits("SELFPRGEN"),
            pgers: data.get_reg_bits("PGERS"),
            pgwrt: data.get_reg_bits("PGWRT"),
        }
    }

    pub fn reset(&mut self) {
        self.tmp_used.fill(0);
        self.tmp_page.fill(0);
    }

    pub fn run_step(&mut self, h: &mut CpuHost) {
        h.cycles_done = 0;
        let instruction = h.flash(h.pc);
        let mut new_pc = h.pc.wrapping_add(1);
        let mut cycle: u32 = 1;

        match instruction & 0xF000 {
            0x0000 => self.op_0xxx(h, instruction, &mut cycle),
            0x1000 => self.op_1xxx(h, instruction, &mut new_pc, &mut cycle),
            0x2000 => op_logic(h, instruction),
            0x3000 => {
                // CPI -- 0011 kkkk hhhh kkkk
                let (_, vh, k) = vh4_k8(h, instruction);
                flags_sub_zns(h, vh.wrapping_sub(k), vh, k);
            }
            0x4000 => {
                // SBCI -- 0100 kkkk hhhh kkkk
                let (h4, vh, k) = vh4_k8(h, instruction);
                let res = vh.wrapping_sub(k).wrapping_sub(u8::from(status(h, S_C)));
                h.set_gpr(h4, res);
                flags_sub_rzns(h, res, vh, k);
            }
            0x5000 => {
                // SUBI -- 0101 kkkk hhhh kkkk
                let (h4, vh, k) = vh4_k8(h, instruction);
                let res = vh.wrapping_sub(k);
                h.set_gpr(h4, res);
                flags_sub_zns(h, res, vh, k);
            }
            0x6000 => {
                // ORI / SBR -- 0110 kkkk hhhh kkkk
                let (h4, vh, k) = vh4_k8(h, instruction);
                let res = vh | k;
                h.set_gpr(h4, res);
                flags_znv0s(h, res);
            }
            0x7000 => {
                // ANDI -- 0111 kkkk hhhh kkkk
                let (h4, vh, k) = vh4_k8(h, instruction);
                let res = vh & k;
                h.set_gpr(h4, res);
                flags_znv0s(h, res);
            }
            0x8000 | 0xA000 => {
                // LD/ST (LDD/STD) via Y/Z with displacement -- 10q0 qqsd dddd yqqq
                let mut v = 0u16;
                match instruction & 0xD008 {
                    0xA000 | 0x8000 => v = h.get_reg16_lh(u16::from(R_ZL)),
                    0xA008 | 0x8008 => v = h.get_reg16_lh(u16::from(R_YL)),
                    _ => {}
                }
                let d = d5(instruction);
                let q = q6(instruction);
                if instruction & 0x0200 != 0 {
                    h.set_ram(v.wrapping_add(u16::from(q)), h.gpr(d));
                } else {
                    let val = h.get_ram(v.wrapping_add(u16::from(q)));
                    h.set_gpr(d, val);
                }
                cycle += 1;
            }
            0x9000 => self.op_9xxx(h, instruction, &mut new_pc, &mut cycle),
            0xB000 => {
                let d = d5(instruction);
                let a = a6(instruction);
                if instruction & 0xF800 == 0xB800 {
                    // OUT A,Rr
                    h.set_ram(u16::from(a), h.gpr(d));
                } else if instruction & 0xF800 == 0xB000 {
                    // IN Rd,A
                    let val = h.get_ram(u16::from(a));
                    h.set_gpr(d, val);
                }
            }
            0xC000 => {
                // RJMP -- 1100 kkkk kkkk kkkk
                let k = (((instruction << 4) as i16) >> 4) as i32;
                new_pc = add_pc(new_pc, k, h.prog.len() as u32);
                cycle += 1;
            }
            0xD000 => {
                // RCALL -- 1101 kkkk kkkk kkkk
                let k = (((instruction << 4) as i16) >> 4) as i32;
                cycle += u32::from(h.prog_addr_size);
                h.push_stack(new_pc);
                h.ret_addr = new_pc;
                new_pc = add_pc(new_pc, k, h.prog.len() as u32);
            }
            0xE000 => {
                // LDI Rd,K -- 1110 kkkk dddd kkkk
                let (h4, k) = h4_k8(instruction);
                h.set_gpr(h4, k);
            }
            0xF000 => self.op_fxxx(h, instruction, &mut new_pc, &mut cycle),
            _ => {}
        }

        let prog_size = h.prog.len() as u32;
        if prog_size != 0 && new_pc >= prog_size {
            new_pc = 0;
        }
        h.pc = new_pc;
        h.cycles_done = cycle;
    }

    fn op_0xxx(&self, h: &mut CpuHost, instruction: u16, cycle: &mut u32) {
        if instruction == 0x0000 {
            return; // NOP
        }
        match instruction & 0xFC00 {
            0x0400 => {
                // CPC -- 0000 01rd dddd rrrr
                let (vd, vr) = vd5_vr5(h, instruction);
                let res = vd.wrapping_sub(vr).wrapping_sub(u8::from(status(h, S_C)));
                flags_sub_rzns(h, res, vd, vr);
            }
            0x0C00 => {
                // ADD -- 0000 11rd dddd rrrr
                let d = d5(instruction);
                let (vd, vr) = vd5_vr5(h, instruction);
                let res = vd.wrapping_add(vr);
                h.set_gpr(d, res);
                flags_add_zns(h, res, vd, vr);
            }
            0x0800 => {
                // SBC -- 0000 10rd dddd rrrr
                let d = d5(instruction);
                let (vd, vr) = vd5_vr5(h, instruction);
                let res = vd.wrapping_sub(vr).wrapping_sub(u8::from(status(h, S_C)));
                h.set_gpr(d, res);
                flags_sub_rzns(h, res, vd, vr);
            }
            _ => match instruction & 0xFF00 {
                0x0100 => {
                    // MOVW -- 0000 0001 dddd rrrr
                    let d = ((instruction >> 4) & 0xF) << 1;
                    let r = (instruction & 0xF) << 1;
                    let vr = h.get_reg16_lh(r);
                    h.set_reg16_lh(d, vr);
                }
                0x0200 => {
                    // MULS -- 0000 0010 dddd rrrr
                    let r = 16 + (instruction & 0xF) as u8;
                    let d = 16 + ((instruction >> 4) & 0xF) as u8;
                    let res = i16::from(h.gpr(r) as i8) * i16::from(h.gpr(d) as i8);
                    h.set_reg16_lh(0, res as u16);
                    write_s_bit(h, S_C, res as u16 & (1 << 15) != 0);
                    write_s_bit(h, S_Z, res == 0);
                    *cycle += 1;
                }
                0x0300 => {
                    // MULSU / FMUL / FMULS / FMULSU
                    let r = 16 + (instruction & 0x7) as u8;
                    let d = 16 + ((instruction >> 4) & 0x7) as u8;
                    let (mut res, mut c) = (0i16, false);
                    match instruction & 0x88 {
                        0x00 => {
                            // MULSU
                            res = i16::from(h.gpr(r)) * i16::from(h.gpr(d) as i8);
                            c = (res >> 15) & 1 != 0;
                        }
                        0x08 => {
                            // FMUL
                            res = i16::from(h.gpr(r)) * i16::from(h.gpr(d));
                            c = (res >> 15) & 1 != 0;
                            res <<= 1;
                        }
                        0x80 => {
                            // FMULS
                            res = i16::from(h.gpr(r) as i8) * i16::from(h.gpr(d) as i8);
                            c = (res >> 15) & 1 != 0;
                            res <<= 1;
                        }
                        0x88 => {
                            // FMULSU
                            res = i16::from(h.gpr(r)) * i16::from(h.gpr(d) as i8);
                            c = (res >> 15) & 1 != 0;
                            res <<= 1;
                        }
                        _ => {}
                    }
                    *cycle += 1;
                    h.set_reg16_lh(0, res as u16);
                    write_s_bit(h, S_C, c);
                    write_s_bit(h, S_Z, res == 0);
                }
                _ => {}
            },
        }
    }

    fn op_1xxx(&self, h: &mut CpuHost, instruction: u16, new_pc: &mut u32, cycle: &mut u32) {
        match instruction & 0xFC00 {
            0x1800 => {
                // SUB
                let d = d5(instruction);
                let (vd, vr) = vd5_vr5(h, instruction);
                let res = vd.wrapping_sub(vr);
                h.set_gpr(d, res);
                flags_sub_zns(h, res, vd, vr);
            }
            0x1000 => {
                // CPSE
                let (vd, vr) = vd5_vr5(h, instruction);
                if vd == vr {
                    skip(h, new_pc, cycle);
                }
            }
            0x1400 => {
                // CP
                let (vd, vr) = vd5_vr5(h, instruction);
                flags_sub_zns(h, vd.wrapping_sub(vr), vd, vr);
            }
            0x1C00 => {
                // ADC
                let d = d5(instruction);
                let (vd, vr) = vd5_vr5(h, instruction);
                let res = vd.wrapping_add(vr).wrapping_add(u8::from(status(h, S_C)));
                h.set_gpr(d, res);
                flags_add_zns(h, res, vd, vr);
            }
            _ => {}
        }
    }

    fn op_9xxx(&mut self, h: &mut CpuHost, instruction: u16, new_pc: &mut u32, cycle: &mut u32) {
        if instruction & 0xFF0F == 0x9408 {
            // BSET / BCLR (SEI/CLI/…)
            let bit = ((instruction >> 4) & 7) as u8;
            let set = instruction & 0x0080 == 0;
            write_s_bit(h, bit, set);
            if bit == S_I {
                h.enable_int = Some(u8::from(set));
            }
        }
        match instruction {
            0x9588 => h.sleep = true, // SLEEP
            0x9598 => {}              // BREAK (not implemented in C++)
            0x95A8 => h.wdr = true,   // WDR
            0x95E8 => {
                // SPM
                if h.pc >= self.boot_start {
                    self.write_flash(h);
                }
            }
            0x9409 | 0x9419 | 0x9509 | 0x9519 => {
                // IJMP / EIJMP / ICALL / EICALL
                let exte = instruction & 0x10 != 0;
                let call = instruction & 0x100 != 0;
                let mut z = u32::from(h.get_reg16_lh(u16::from(R_ZL)));
                if exte {
                    let Some(addr) = self.eind_addr else {
                        return;
                    };
                    z |= u32::from(h.data.get(addr)) << 16;
                }
                if call {
                    h.push_stack(*new_pc);
                    h.ret_addr = *new_pc;
                    *cycle += u32::from(h.prog_addr_size.saturating_sub(1));
                }
                *new_pc = z;
                *cycle += 1;
            }
            0x9518 | 0x9508 => {
                // RETI / RET
                if instruction == 0x9518 {
                    h.reti = true;
                }
                *new_pc = h.pop_stack();
                *cycle += 1 + u32::from(h.prog_addr_size);
            }
            0x95C8 => {
                // LPM R0 <- (Z)
                let z = h.get_reg16_lh(u16::from(R_ZL));
                *cycle += 2;
                h.set_gpr(0, lpm_byte(h, u32::from(z)));
            }
            0x95D8 => {
                // ELPM R0 <- (Z)
                let Some(rampz) = self.rampz_addr else {
                    return;
                };
                let z = u32::from(h.get_reg16_lh(u16::from(R_ZL)))
                    | (u32::from(h.data.get(rampz)) << 16);
                h.set_gpr(0, lpm_byte(h, z));
                *cycle += 2;
            }
            _ => self.op_9xxx_rest(h, instruction, new_pc, cycle),
        }
    }

    fn op_9xxx_rest(
        &mut self,
        h: &mut CpuHost,
        instruction: u16,
        new_pc: &mut u32,
        cycle: &mut u32,
    ) {
        match instruction & 0xFE0F {
            0x9000 => {
                // LDS -- 32-bit
                let d = d5(instruction);
                let x = h.flash(*new_pc);
                *new_pc += 1;
                let val = h.get_ram(x);
                h.set_gpr(d, val);
                *cycle += 1;
            }
            0x9005 | 0x9004 => {
                // LPM Rd / LPM Rd, Z+
                let d = d5(instruction);
                let mut z = h.get_reg16_lh(u16::from(R_ZL));
                h.set_gpr(d, lpm_byte(h, u32::from(z)));
                if instruction & 1 != 0 {
                    z = z.wrapping_add(1);
                    h.set_reg16_hl(u16::from(R_ZL), z);
                }
                *cycle += 2;
            }
            0x9006 | 0x9007 => {
                // ELPM Rd / ELPM Rd, Z+
                let Some(rampz) = self.rampz_addr else {
                    return;
                };
                let mut z = u32::from(h.get_reg16_lh(u16::from(R_ZL)))
                    | (u32::from(h.data.get(rampz)) << 16);
                let d = d5(instruction);
                h.set_gpr(d, lpm_byte(h, z));
                if instruction & 1 != 0 {
                    z = z.wrapping_add(1);
                    h.data.set(rampz, (z >> 16) as u8);
                    h.set_reg16_hl(u16::from(R_ZL), z as u16);
                }
                *cycle += 2;
            }
            0x900C | 0x900D | 0x900E => {
                // LD X / X+ / -X
                ld_ptr(h, instruction, R_XL, cycle);
            }
            0x920C | 0x920D | 0x920E => {
                // ST X / X+ / -X
                st_ptr(h, instruction, R_XL, cycle);
            }
            0x9009 | 0x900A => {
                // LD Y+ / -Y
                ld_ptr(h, instruction, R_YL, cycle);
            }
            0x9209 | 0x920A => {
                // ST Y+ / -Y
                st_ptr(h, instruction, R_YL, cycle);
            }
            0x9200 => {
                // STS -- 32-bit
                let vd = h.gpr(d5(instruction));
                let x = h.flash(*new_pc);
                *new_pc += 1;
                *cycle += 1;
                h.set_ram(x, vd);
            }
            0x9001 | 0x9002 => {
                // LD Z+ / -Z
                ld_ptr(h, instruction, R_ZL, cycle);
            }
            0x9201 | 0x9202 => {
                // ST Z+ / -Z
                st_ptr(h, instruction, R_ZL, cycle);
            }
            0x900F => {
                // POP
                let d = d5(instruction);
                let val = h.pop_stack8();
                h.set_gpr(d, val);
                *cycle += 1;
            }
            0x920F => {
                // PUSH
                h.push_stack8(h.gpr(d5(instruction)));
                *cycle += 1;
            }
            0x9400 => {
                // COM
                let d = d5(instruction);
                let res = 0xFF - h.gpr(d);
                h.set_gpr(d, res);
                flags_znv0s(h, res);
                set_s_bit(h, S_C);
            }
            0x9401 => {
                // NEG
                let d = d5(instruction);
                let vd = h.gpr(d);
                let res = 0u8.wrapping_sub(vd);
                h.set_gpr(d, res);
                write_s_bit(h, S_H, ((res >> 3) | (vd >> 3)) & 1 != 0);
                write_s_bit(h, S_V, res == 0x80);
                write_s_bit(h, S_C, res != 0);
                flags_zns(h, res);
            }
            0x9402 => {
                // SWAP
                let d = d5(instruction);
                let vd = h.gpr(d);
                h.set_gpr(d, (vd >> 4) | (vd << 4));
            }
            0x9403 => {
                // INC
                let d = d5(instruction);
                let res = h.gpr(d).wrapping_add(1);
                h.set_gpr(d, res);
                write_s_bit(h, S_V, res == 0x80);
                flags_zns(h, res);
            }
            0x9405 => {
                // ASR
                let d = d5(instruction);
                let vd = h.gpr(d);
                let res = (vd >> 1) | (vd & 0x80);
                h.set_gpr(d, res);
                flags_zcnvs(h, res, vd);
            }
            0x9406 => {
                // LSR
                let d = d5(instruction);
                let vd = h.gpr(d);
                let res = vd >> 1;
                h.set_gpr(d, res);
                clear_s_bit(h, S_N);
                flags_zcvs(h, res, vd);
            }
            0x9407 => {
                // ROR
                let d = d5(instruction);
                let vd = h.gpr(d);
                let res = (if status(h, S_C) { 0x80 } else { 0 }) | (vd >> 1);
                h.set_gpr(d, res);
                flags_zcnvs(h, res, vd);
            }
            0x940A => {
                // DEC
                let d = d5(instruction);
                let res = h.gpr(d).wrapping_sub(1);
                h.set_gpr(d, res);
                write_s_bit(h, S_V, res == 0x7F);
                flags_zns(h, res);
            }
            0x940C | 0x940D => {
                // JMP -- 32-bit
                let mut a = u32::from((instruction & 0x01F0) >> 3) | u32::from(instruction & 1);
                let x = h.flash(*new_pc);
                a = (a << 16) | u32::from(x);
                *new_pc = a;
                *cycle += 2;
            }
            0x940E | 0x940F => {
                // CALL -- 32-bit
                let mut a = u32::from((instruction & 0x01F0) >> 3) | u32::from(instruction & 1);
                let x = h.flash(*new_pc);
                a = (a << 16) | u32::from(x);
                *new_pc += 1;
                h.push_stack(*new_pc);
                h.ret_addr = *new_pc;
                *cycle += 1 + u32::from(h.prog_addr_size);
                *new_pc = a;
            }
            _ => self.op_9xxx_io(h, instruction, new_pc, cycle),
        }
    }

    fn op_9xxx_io(&self, h: &mut CpuHost, instruction: u16, new_pc: &mut u32, cycle: &mut u32) {
        match instruction & 0xFF00 {
            0x9600 => {
                // ADIW
                let (p, k, vp) = vp2_k6(h, instruction);
                let res = vp.wrapping_add(u16::from(k));
                h.set_reg16_hl(u16::from(p), res);
                write_s_bit(h, S_V, (!vp & res) & (1 << 15) != 0);
                write_s_bit(h, S_C, (!res & vp) & (1 << 15) != 0);
                flags_zns16(h, res);
                *cycle += 1;
            }
            0x9700 => {
                // SBIW
                let (p, k, vp) = vp2_k6(h, instruction);
                let res = vp.wrapping_sub(u16::from(k));
                h.set_reg16_hl(u16::from(p), res);
                write_s_bit(h, S_V, (vp & !res) & (1 << 15) != 0);
                write_s_bit(h, S_C, (res & !vp) & (1 << 15) != 0);
                flags_zns16(h, res);
                *cycle += 1;
            }
            0x9800 => {
                // CBI
                let (io, mask) = io5_b3mask(instruction);
                let res = h.get_ram(u16::from(io)) & !mask;
                h.set_ram(u16::from(io), res);
                *cycle += 1;
            }
            0x9900 => {
                // SBIC
                let (io, mask) = io5_b3mask(instruction);
                if h.get_ram(u16::from(io)) & mask == 0 {
                    skip(h, new_pc, cycle);
                }
            }
            0x9A00 => {
                // SBI
                let (io, mask) = io5_b3mask(instruction);
                let res = h.get_ram(u16::from(io)) | mask;
                h.set_ram(u16::from(io), res);
                *cycle += 1;
            }
            0x9B00 => {
                // SBIS
                let (io, mask) = io5_b3mask(instruction);
                if h.get_ram(u16::from(io)) & mask != 0 {
                    skip(h, new_pc, cycle);
                }
            }
            _ if instruction & 0xFC00 == 0x9C00 => {
                // MUL -- 1001 11rd dddd rrrr
                let (vd, vr) = vd5_vr5(h, instruction);
                let res = u16::from(vd) * u16::from(vr);
                *cycle += 1;
                h.set_reg16_lh(0, res);
                write_s_bit(h, S_Z, res == 0);
                write_s_bit(h, S_C, res & (1 << 15) != 0);
            }
            _ => {}
        }
    }

    fn op_fxxx(&self, h: &mut CpuHost, instruction: u16, new_pc: &mut u32, cycle: &mut u32) {
        match instruction & 0xFE00 {
            0xF000 | 0xF200 | 0xF400 | 0xF600 => {
                // BRBC / BRBS
                let o = ((instruction << 6) as i16) >> 9;
                let s = (instruction & 7) as u8;
                let set = instruction & 0x0400 == 0;
                let flag = status(h, s);
                if (flag && set) || (!flag && !set) {
                    *cycle += 1;
                    *new_pc = new_pc.wrapping_add_signed(i32::from(o));
                }
            }
            0xF800 | 0xF900 => {
                // BLD
                let (d, vd, mask) = vd5_s3_mask(h, instruction);
                let v = (vd & !mask) | if status(h, S_T) { mask } else { 0 };
                h.set_gpr(d, v);
            }
            0xFA00 | 0xFB00 => {
                // BST
                let (_d, vd, s) = vd5_s3(h, instruction);
                write_s_bit(h, S_T, (vd >> s) & 1 != 0);
            }
            0xFC00 | 0xFE00 => {
                // SBRC / SBRS
                let (_d, vd, mask) = vd5_s3_mask(h, instruction);
                let set = instruction & 0x0200 != 0;
                let bit = vd & mask != 0;
                if (bit && set) || (!bit && !set) {
                    skip(h, new_pc, cycle);
                }
            }
            _ => {}
        }
    }

    fn write_flash(&mut self, h: &mut CpuHost) {
        if self.page_size == 0 {
            return;
        }
        if !bits_bool(h, self.selfprgen) {
            return;
        }
        let mut z = u32::from(h.get_reg16_lh(u16::from(R_ZL)));
        if let Some(a) = self.rampz_addr {
            z |= u32::from(h.data.get(a)) << 16;
        }
        if bits_bool(h, self.pgers) {
            z >>= 1;
            for _ in 0..self.page_size {
                if (z as usize) < h.prog.len() {
                    h.prog[z as usize] = 0xFF;
                }
                z = z.wrapping_add(1);
            }
        } else if bits_bool(h, self.pgwrt) {
            z &= !(self.page_size as u32 - 1);
            z >>= 1;
            for i in 0..self.page_size {
                if (z as usize) < h.prog.len() {
                    h.prog[z as usize] = self.tmp_page[i];
                }
                self.tmp_page[i] = 0;
                self.tmp_used[i] = 0;
                z = z.wrapping_add(1);
            }
        } else {
            let r01 = h.get_reg16_lh(0);
            z >>= 1;
            let addr = (z as usize) % self.page_size;
            if self.tmp_used[addr] == 0 {
                self.tmp_used[addr] = 1;
                self.tmp_page[addr] = r01;
            }
        }
        clear_reg_bits(h, self.selfprgen);
    }
}

fn op_logic(h: &mut CpuHost, instruction: u16) {
    let d = d5(instruction);
    let (vd, vr) = vd5_vr5(h, instruction);
    let mut res = vr;
    let mut znv = true;
    match instruction & 0xFC00 {
        0x2000 => res &= vd,   // AND
        0x2400 => res ^= vd,   // EOR
        0x2800 => res |= vd,   // OR
        0x2C00 => znv = false, // MOV, res = vr
        _ => {}
    }
    if znv {
        flags_znv0s(h, res);
    }
    h.set_gpr(d, res);
}

fn ld_ptr(h: &mut CpuHost, instruction: u16, ptr: u8, cycle: &mut u32) {
    let op = instruction & 3;
    let d = d5(instruction);
    let mut x = h.get_reg16_lh(u16::from(ptr));
    *cycle += 1;
    if op == 2 {
        x = x.wrapping_sub(1);
    }
    let vd = h.get_ram(x);
    if op == 1 {
        x = x.wrapping_add(1);
    }
    h.set_reg16_hl(u16::from(ptr), x);
    h.set_gpr(d, vd);
}

fn st_ptr(h: &mut CpuHost, instruction: u16, ptr: u8, cycle: &mut u32) {
    let op = instruction & 3;
    let vd = h.gpr(d5(instruction));
    let mut x = h.get_reg16_lh(u16::from(ptr));
    *cycle += 1;
    if op == 2 {
        x = x.wrapping_sub(1);
    }
    h.set_ram(x, vd);
    if op == 1 {
        x = x.wrapping_add(1);
    }
    h.set_reg16_hl(u16::from(ptr), x);
}

fn lpm_byte(h: &CpuHost, z: u32) -> u8 {
    let mut prg = h.flash(z / 2);
    if z & 1 != 0 {
        prg >>= 8;
    }
    prg as u8
}

fn skip(h: &CpuHost, new_pc: &mut u32, cycle: &mut u32) {
    if is_instr_32b(h, *new_pc) {
        *new_pc += 2;
        *cycle += 2;
    } else {
        *new_pc += 1;
        *cycle += 1;
    }
}

fn is_instr_32b(h: &CpuHost, pc: u32) -> bool {
    let o = h.flash(pc) & 0xFC0F;
    o == 0x9200 || o == 0x9000 || o == 0x940C || o == 0x940D || o == 0x940E || o == 0x940F
}

fn add_pc(pc: u32, k: i32, size: u32) -> u32 {
    if size == 0 {
        return 0;
    }
    pc.wrapping_add_signed(k) % size
}

fn d5(o: u16) -> u8 {
    ((o >> 4) & 0x1F) as u8
}

fn r5(o: u16) -> u8 {
    (((o >> 5) & 0x10) | (o & 0xF)) as u8
}

fn vd5_vr5(h: &CpuHost, o: u16) -> (u8, u8) {
    (h.gpr(d5(o)), h.gpr(r5(o)))
}

fn h4_k8(o: u16) -> (u8, u8) {
    let h = 16 + ((o >> 4) & 0xF) as u8;
    let k = (((o & 0x0F00) >> 4) | (o & 0xF)) as u8;
    (h, k)
}

fn vh4_k8(h: &CpuHost, o: u16) -> (u8, u8, u8) {
    let (reg, k) = h4_k8(o);
    (reg, h.gpr(reg), k)
}

fn q6(o: u16) -> u8 {
    (((o & 0x2000) >> 8) | ((o & 0x0C00) >> 7) | (o & 0x7)) as u8
}

fn a6(o: u16) -> u8 {
    (((((o >> 9) & 3) << 4) | (o & 0xF)) + 32) as u8
}

fn io5_b3mask(o: u16) -> (u8, u8) {
    let io = (((o >> 3) & 0x1F) + 32) as u8;
    let mask = 1u8 << (o & 0x7);
    (io, mask)
}

fn vp2_k6(h: &CpuHost, o: u16) -> (u8, u8, u16) {
    let p = 24 + ((o >> 3) & 0x6) as u8;
    let k = (((o & 0x00C0) >> 2) | (o & 0xF)) as u8;
    let vp = h.get_reg16_lh(u16::from(p));
    (p, k, vp)
}

fn vd5_s3(h: &CpuHost, o: u16) -> (u8, u8, u8) {
    let d = d5(o);
    (d, h.gpr(d), (o & 7) as u8)
}

fn vd5_s3_mask(h: &CpuHost, o: u16) -> (u8, u8, u8) {
    let (d, vd, s) = vd5_s3(h, o);
    (d, vd, 1u8 << s)
}

#[inline(always)]
fn status(h: &CpuHost, bit: u8) -> bool {
    h.data.get(h.data.sreg_addr) & (1 << bit) != 0
}

#[inline(always)]
fn write_sreg_bits(h: &mut CpuHost, mask: u8, val: u8) {
    let addr = h.data.sreg_addr;
    let s = h.data.get(addr);
    h.data.set(addr, (s & !mask) | (val & mask));
}

#[inline(always)]
fn write_s_bit(h: &mut CpuHost, bit: u8, val: bool) {
    let mask = 1 << bit;
    let val_byte = if val { mask } else { 0 };
    write_sreg_bits(h, mask, val_byte);
}

#[inline(always)]
fn set_s_bit(h: &mut CpuHost, bit: u8) {
    write_sreg_bits(h, 1 << bit, 1 << bit);
}

#[inline(always)]
fn clear_s_bit(h: &mut CpuHost, bit: u8) {
    write_sreg_bits(h, 1 << bit, 0);
}

#[inline(always)]
fn flags_zns(h: &mut CpuHost, res: u8) {
    let sz = u8::from(res == 0);
    let sn = (res >> 7) & 1;
    let sv = (h.data.get(h.data.sreg_addr) >> S_V) & 1;
    let ss = sn ^ sv;
    let mask = (1 << S_Z) | (1 << S_N) | (1 << S_S);
    let val = (sz << S_Z) | (sn << S_N) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_sub_rzns(h: &mut CpuHost, res: u8, rd: u8, rr: u8) {
    let sub_carry = (!rd & rr) | (rr & res) | (res & !rd);
    let sh = (sub_carry >> 3) & 1;
    let sc = (sub_carry >> 7) & 1;
    let sv = (((rd & !rr & !res) | (!rd & rr & res)) >> 7) & 1;
    let sn = (res >> 7) & 1;
    let ss = sn ^ sv;
    let mut mask = (1 << S_H) | (1 << S_C) | (1 << S_V) | (1 << S_N) | (1 << S_S);
    let val = (sh << S_H) | (sc << S_C) | (sv << S_V) | (sn << S_N) | (ss << S_S);
    if res != 0 {
        mask |= 1 << S_Z;
    }
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_add_zns(h: &mut CpuHost, res: u8, rd: u8, rr: u8) {
    let add_carry = (rd & rr) | (rr & !res) | (!res & rd);
    let sh = (add_carry >> 3) & 1;
    let sc = (add_carry >> 7) & 1;
    let sv = (((rd & rr & !res) | (!rd & !rr & res)) >> 7) & 1;
    let sz = u8::from(res == 0);
    let sn = (res >> 7) & 1;
    let ss = sn ^ sv;
    let mask = (1 << S_H) | (1 << S_C) | (1 << S_V) | (1 << S_Z) | (1 << S_N) | (1 << S_S);
    let val = (sh << S_H) | (sc << S_C) | (sv << S_V) | (sz << S_Z) | (sn << S_N) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_sub_zns(h: &mut CpuHost, res: u8, rd: u8, rr: u8) {
    let sub_carry = (!rd & rr) | (rr & res) | (res & !rd);
    let sh = (sub_carry >> 3) & 1;
    let sc = (sub_carry >> 7) & 1;
    let sv = (((rd & !rr & !res) | (!rd & rr & res)) >> 7) & 1;
    let sz = u8::from(res == 0);
    let sn = (res >> 7) & 1;
    let ss = sn ^ sv;
    let mask = (1 << S_H) | (1 << S_C) | (1 << S_V) | (1 << S_Z) | (1 << S_N) | (1 << S_S);
    let val = (sh << S_H) | (sc << S_C) | (sv << S_V) | (sz << S_Z) | (sn << S_N) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_znv0s(h: &mut CpuHost, res: u8) {
    let sz = u8::from(res == 0);
    let sn = (res >> 7) & 1;
    let ss = sn;
    let mask = (1 << S_V) | (1 << S_Z) | (1 << S_N) | (1 << S_S);
    let val = (sz << S_Z) | (sn << S_N) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_zcnvs(h: &mut CpuHost, res: u8, vr: u8) {
    let sz = u8::from(res == 0);
    let sc = vr & 1;
    let sn = (res >> 7) & 1;
    let sv = sn ^ sc;
    let ss = sn ^ sv;
    let mask = (1 << S_Z) | (1 << S_C) | (1 << S_N) | (1 << S_V) | (1 << S_S);
    let val = (sz << S_Z) | (sc << S_C) | (sn << S_N) | (sv << S_V) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_zcvs(h: &mut CpuHost, res: u8, vr: u8) {
    let sz = u8::from(res == 0);
    let sc = vr & 1;
    let sn = (h.data.get(h.data.sreg_addr) >> S_N) & 1;
    let sv = sn ^ sc;
    let ss = sn ^ sv;
    let mask = (1 << S_Z) | (1 << S_C) | (1 << S_V) | (1 << S_S);
    let val = (sz << S_Z) | (sc << S_C) | (sv << S_V) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

#[inline(always)]
fn flags_zns16(h: &mut CpuHost, res: u16) {
    let sz = u8::from(res == 0);
    let sn = ((res >> 15) & 1) as u8;
    let sv = (h.data.get(h.data.sreg_addr) >> S_V) & 1;
    let ss = sn ^ sv;
    let mask = (1 << S_Z) | (1 << S_N) | (1 << S_S);
    let val = (sz << S_Z) | (sn << S_N) | (ss << S_S);
    write_sreg_bits(h, mask, val);
}

fn bits_bool(h: &CpuHost, b: RegBits) -> bool {
    b.mask != 0 && h.data.get(b.reg_addr) & b.mask != 0
}

fn clear_reg_bits(h: &mut CpuHost, b: RegBits) {
    if b.mask == 0 {
        return;
    }
    let v = h.data.get(b.reg_addr) & !b.mask;
    h.data.set(b.reg_addr, v);
}
