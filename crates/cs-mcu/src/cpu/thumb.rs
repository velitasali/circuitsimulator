//! A small Thumb interpreter.
//!
//! It runs a handful of 16-bit instructions on its own memory. It is not a
//! schematic MCU: `Device` still reports this core as unsupported, because
//! the pin and peripheral model is not wired up.

/// Thumb machine. `r[15]` is the address of the next instruction.
#[derive(Clone, Debug)]
pub struct Thumb {
    pub r: [u32; 16],
    pub mem: Vec<u8>,
    pub z: bool,
    pub n: bool,
    pub c: bool,
}

impl Thumb {
    pub fn new(mem: Vec<u8>) -> Self {
        let sp = (mem.len() & !3) as u32;
        let mut cpu = Self {
            r: [0; 16],
            mem,
            z: false,
            n: false,
            c: false,
        };
        cpu.r[13] = sp;
        cpu
    }

    /// Execute one instruction. Returns false when the opcode is not in this
    /// subset or the program counter is outside memory.
    pub fn step(&mut self) -> bool {
        let pc = self.r[15];
        if pc % 2 != 0 {
            return false;
        }
        let Some(op) = self.read_u16(pc) else {
            return false;
        };
        self.r[15] = pc.wrapping_add(2);
        if (op & 0xF800) == 0x2000 {
            // MOVS Rd, #imm8
            let rd = ((op >> 8) & 7) as usize;
            let imm = u32::from(op & 0xFF);
            self.r[rd] = imm;
            self.set_nz(imm);
            true
        } else if (op & 0xF800) == 0x3000 {
            // ADDS Rd, #imm8
            let rd = ((op >> 8) & 7) as usize;
            let imm = u32::from(op & 0xFF);
            let (sum, carry) = self.r[rd].overflowing_add(imm);
            self.r[rd] = sum;
            self.c = carry;
            self.set_nz(sum);
            true
        } else if (op & 0xF800) == 0x3800 {
            // SUBS Rd, #imm8
            let rd = ((op >> 8) & 7) as usize;
            let imm = u32::from(op & 0xFF);
            let (diff, borrow) = self.r[rd].overflowing_sub(imm);
            self.r[rd] = diff;
            self.c = !borrow;
            self.set_nz(diff);
            true
        } else if (op & 0xF800) == 0x6800 {
            // LDR Rt, [Rn, #imm5]
            let imm = u32::from((op >> 6) & 0x1F) << 2;
            let rn = ((op >> 3) & 7) as usize;
            let rt = (op & 7) as usize;
            self.r[rt] = self.read_u32(self.r[rn].wrapping_add(imm));
            true
        } else if (op & 0xF800) == 0x6000 {
            // STR Rt, [Rn, #imm5]
            let imm = u32::from((op >> 6) & 0x1F) << 2;
            let rn = ((op >> 3) & 7) as usize;
            let rt = (op & 7) as usize;
            let addr = self.r[rn].wrapping_add(imm);
            let val = self.r[rt];
            self.write_u32(addr, val);
            true
        } else if (op & 0xF800) == 0xE000 {
            // B imm11. Target is PC+4 plus the shifted offset. PC was already
            // advanced by 2, so the architectural PC (instruction + 4) is r15+2.
            let offset = sign_extend_u11(u32::from(op & 0x7FF)) << 1;
            let arch_pc = pc.wrapping_add(4);
            self.r[15] = arch_pc.wrapping_add(offset as u32);
            true
        } else if (op & 0xFF87) == 0x4700 {
            // BX Rm
            let rm = ((op >> 3) & 0xF) as usize;
            self.r[15] = self.r[rm] & !1;
            true
        } else if (op & 0xFE00) == 0xB400 {
            // PUSH. Bit 8 is LR. Bits 7..0 are r0..r7.
            let mut regs = Vec::new();
            for i in 0..8u16 {
                if op & (1u16 << i) != 0 {
                    regs.push(i as usize);
                }
            }
            if op & (1u16 << 8) != 0 {
                regs.push(14);
            }
            self.push(&regs);
            true
        } else if (op & 0xFE00) == 0xBC00 {
            // POP. Bit 8 is PC.
            let mut regs = Vec::new();
            for i in 0..8u16 {
                if op & (1u16 << i) != 0 {
                    regs.push(i as usize);
                }
            }
            if op & (1u16 << 8) != 0 {
                regs.push(15);
            }
            self.pop(&regs);
            true
        } else {
            self.r[15] = pc;
            false
        }
    }

    fn push(&mut self, regs: &[usize]) {
        let bytes = (regs.len() as u32).wrapping_mul(4);
        self.r[13] = self.r[13].wrapping_sub(bytes);
        let mut addr = self.r[13];
        for &reg in regs {
            let v = self.r[reg];
            self.write_u32(addr, v);
            addr = addr.wrapping_add(4);
        }
    }

    fn pop(&mut self, regs: &[usize]) {
        let mut addr = self.r[13];
        for &reg in regs {
            let v = self.read_u32(addr);
            if reg == 15 {
                self.r[15] = v & !1;
            } else {
                self.r[reg] = v;
            }
            addr = addr.wrapping_add(4);
        }
        self.r[13] = self.r[13].wrapping_add((regs.len() as u32).wrapping_mul(4));
    }

    fn set_nz(&mut self, v: u32) {
        self.z = v == 0;
        self.n = (v as i32) < 0;
    }

    fn read_u16(&self, addr: u32) -> Option<u16> {
        let i = addr as usize;
        if i + 1 >= self.mem.len() {
            return None;
        }
        Some(u16::from(self.mem[i]) | (u16::from(self.mem[i + 1]) << 8))
    }

    fn read_u32(&self, addr: u32) -> u32 {
        let i = addr as usize;
        if i + 3 >= self.mem.len() {
            return 0;
        }
        u32::from(self.mem[i])
            | (u32::from(self.mem[i + 1]) << 8)
            | (u32::from(self.mem[i + 2]) << 16)
            | (u32::from(self.mem[i + 3]) << 24)
    }

    fn write_u32(&mut self, addr: u32, v: u32) {
        let i = addr as usize;
        if i + 3 >= self.mem.len() {
            return;
        }
        self.mem[i] = v as u8;
        self.mem[i + 1] = (v >> 8) as u8;
        self.mem[i + 2] = (v >> 16) as u8;
        self.mem[i + 3] = (v >> 24) as u8;
    }
}

fn sign_extend_u11(val: u32) -> i32 {
    ((val << 21) as i32) >> 21
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::CoreKind;
    use crate::desc::McuDesc;
    use crate::device::Device;

    fn le(words: &[u16]) -> Vec<u8> {
        let mut mem = vec![0u8; 256];
        for (i, w) in words.iter().enumerate() {
            mem[i * 2] = *w as u8;
            mem[i * 2 + 1] = (*w >> 8) as u8;
        }
        mem
    }

    #[test]
    fn adds_an_immediate_and_branches_over_a_store() {
        // MOVS r0, #1; ADDS r0, #1; B +0 (skip next); MOVS r0, #9; MOVS r1, #3
        let mem = le(&[0x2001, 0x3001, 0xE000, 0x2009, 0x2103]);
        let mut cpu = Thumb::new(mem);
        assert!(cpu.step());
        assert!(cpu.step());
        assert_eq!(cpu.r[0], 2);
        assert!(cpu.step());
        assert!(cpu.step());
        assert_eq!(cpu.r[0], 2, "the skipped move wrote r0");
        assert_eq!(cpu.r[1], 3);
    }

    #[test]
    fn stores_and_loads_a_word() {
        let mem = le(&[0x2120, 0x2007, 0x6008, 0x2000, 0x6808]);
        // MOVS r1, #4; MOVS r0, #7; STR r0, [r1, #0]; MOVS r0, #0; LDR r0, [r1, #0]
        let mut cpu = Thumb::new(mem);
        for _ in 0..5 {
            assert!(cpu.step());
        }
        assert_eq!(cpu.r[0], 7);
    }

    #[test]
    fn push_and_pop_round_trip() {
        // MOVS r0, #42; PUSH {r0}; MOVS r0, #0; POP {r0}
        let mem = le(&[0x202A, 0xB401, 0x2000, 0xBC01]);
        let mut cpu = Thumb::new(mem);
        for _ in 0..4 {
            assert!(cpu.step());
        }
        assert_eq!(cpu.r[0], 42);
    }

    #[test]
    fn core_kind_is_recognized_and_not_a_device() {
        assert_eq!(CoreKind::from_attr("Thumb"), CoreKind::Thumb);
        assert_eq!(CoreKind::from_attr("Cortex-M"), CoreKind::Thumb);
        assert!(!CoreKind::Thumb.supported());
        let err = Device::from_desc("m", McuDesc::new(CoreKind::Thumb, "Thumb")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("standalone"), "{msg}");
    }
}
