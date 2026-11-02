//! A small RV32I interpreter.
//!
//! ADDI, ADD, SUB, LW, SW, BEQ, and JAL. `x0` stays zero. Like the Thumb
//! core, this is not attached to `Device::step`.

/// RV32I machine. `pc` is the address of the next instruction.
#[derive(Clone, Debug)]
pub struct Rv32 {
    pub x: [u32; 32],
    pub pc: u32,
    pub mem: Vec<u8>,
}

impl Rv32 {
    pub fn new(mem: Vec<u8>) -> Self {
        Self {
            x: [0; 32],
            pc: 0,
            mem,
        }
    }

    /// Execute one instruction. Returns false when `pc` is out of range or
    /// the opcode is not in this subset.
    pub fn step(&mut self) -> bool {
        let pc = self.pc;
        let Some(op) = self.read_u32(pc) else {
            return false;
        };
        let opcode = op & 0x7F;
        let rd = ((op >> 7) & 0x1F) as usize;
        let funct3 = (op >> 12) & 7;
        let rs1 = ((op >> 15) & 0x1F) as usize;
        let rs2 = ((op >> 20) & 0x1F) as usize;
        let funct7 = op >> 25;
        match opcode {
            0x13 if funct3 == 0 => {
                // ADDI
                let imm = sext(op >> 20, 12);
                self.set(rd, self.x[rs1].wrapping_add(imm as u32));
                self.pc = pc.wrapping_add(4);
                true
            }
            0x33 if funct3 == 0 && funct7 == 0 => {
                // ADD
                self.set(rd, self.x[rs1].wrapping_add(self.x[rs2]));
                self.pc = pc.wrapping_add(4);
                true
            }
            0x33 if funct3 == 0 && funct7 == 0x20 => {
                // SUB
                self.set(rd, self.x[rs1].wrapping_sub(self.x[rs2]));
                self.pc = pc.wrapping_add(4);
                true
            }
            0x03 if funct3 == 0x2 => {
                // LW
                let imm = sext(op >> 20, 12);
                let addr = self.x[rs1].wrapping_add(imm as u32);
                let v = self.read_u32(addr).unwrap_or(0);
                self.set(rd, v);
                self.pc = pc.wrapping_add(4);
                true
            }
            0x23 if funct3 == 0x2 => {
                // SW
                let imm = sext(((op >> 25) << 5) | ((op >> 7) & 0x1F), 12);
                let addr = self.x[rs1].wrapping_add(imm as u32);
                self.write_u32(addr, self.x[rs2]);
                self.pc = pc.wrapping_add(4);
                true
            }
            0x63 if funct3 == 0 => {
                // BEQ
                let imm = beq_imm(op);
                if self.x[rs1] == self.x[rs2] {
                    self.pc = pc.wrapping_add(imm as u32);
                } else {
                    self.pc = pc.wrapping_add(4);
                }
                true
            }
            0x6F => {
                // JAL
                let imm = jal_imm(op);
                self.set(rd, pc.wrapping_add(4));
                self.pc = pc.wrapping_add(imm as u32);
                true
            }
            _ => false,
        }
    }

    fn set(&mut self, rd: usize, v: u32) {
        if rd != 0 {
            self.x[rd] = v;
        }
    }

    fn read_u32(&self, addr: u32) -> Option<u32> {
        let i = addr as usize;
        if i + 3 >= self.mem.len() {
            return None;
        }
        Some(
            u32::from(self.mem[i])
                | (u32::from(self.mem[i + 1]) << 8)
                | (u32::from(self.mem[i + 2]) << 16)
                | (u32::from(self.mem[i + 3]) << 24),
        )
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

fn sext(val: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((val << shift) as i32) >> shift
}

fn beq_imm(op: u32) -> i32 {
    let imm12 = (op >> 31) & 1;
    let imm11 = (op >> 7) & 1;
    let imm10_5 = (op >> 25) & 0x3F;
    let imm4_1 = (op >> 8) & 0xF;
    let raw = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1);
    sext(raw, 13)
}

fn jal_imm(op: u32) -> i32 {
    let imm20 = (op >> 31) & 1;
    let imm19_12 = (op >> 12) & 0xFF;
    let imm11 = (op >> 20) & 1;
    let imm10_1 = (op >> 21) & 0x3FF;
    let raw = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1);
    sext(raw, 21)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::CoreKind;
    use crate::desc::McuDesc;
    use crate::device::Device;

    fn words(ops: &[u32]) -> Vec<u8> {
        let mut mem = Vec::new();
        for op in ops {
            mem.extend_from_slice(&op.to_le_bytes());
        }
        mem
    }

    #[test]
    fn adds_and_subtracts() {
        // ADDI x1, x0, 5; ADDI x2, x0, 7; ADD x3, x1, x2; SUB x4, x2, x1
        let mem = words(&[0x0050_0093, 0x0070_0113, 0x0020_81B3, 0x4011_0233]);
        let mut cpu = Rv32::new(mem);
        for _ in 0..4 {
            assert!(cpu.step());
        }
        assert_eq!(cpu.x[1], 5);
        assert_eq!(cpu.x[2], 7);
        assert_eq!(cpu.x[3], 12);
        assert_eq!(cpu.x[4], 2);
        assert_eq!(cpu.x[0], 0);
    }

    #[test]
    fn stores_loads_and_branches() {
        // ADDI x1, x0, 0x2A; SW x1, 16(x0); LW x2, 16(x0);
        // BEQ x0, x0, +8; ADDI x3, x0, 1; ADDI x3, x0, 2
        // SW x1, 16(x0): imm[4:0] = 16 sits in bits 11:7.
        let sw = (1 << 20) | (2 << 12) | (16 << 7) | 0x23;
        let lw = (16 << 20) | (2 << 12) | (2 << 7) | 0x03;
        let beq = 0x0000_0463_u32; // offset 8
        let mem = words(&[
            0x02A0_0093, // ADDI x1, x0, 42
            sw,
            lw,
            beq,
            0x0010_0193, // ADDI x3, x0, 1
            0x0020_0193, // ADDI x3, x0, 2
        ]);
        let mut cpu = Rv32::new(mem);
        for _ in 0..5 {
            assert!(cpu.step(), "pc {:#x}", cpu.pc);
        }
        assert_eq!(cpu.x[2], 42);
        assert_eq!(cpu.x[3], 2, "the branch did not skip the first addi");
    }

    #[test]
    fn jal_writes_the_link() {
        // JAL x1, +8; ADDI x2, x0, 1; ADDI x2, x0, 2
        let jal = 0x0080_00EF_u32;
        let mem = words(&[jal, 0x0010_0113, 0x0020_0113]);
        let mut cpu = Rv32::new(mem);
        assert!(cpu.step());
        assert_eq!(cpu.x[1], 4);
        assert_eq!(cpu.pc, 8);
        assert!(cpu.step());
        assert_eq!(cpu.x[2], 2);
    }

    #[test]
    fn core_kind_is_recognized_and_not_a_device() {
        assert_eq!(CoreKind::from_attr("RV32"), CoreKind::Rv32);
        assert_eq!(CoreKind::from_attr("RiscV"), CoreKind::Rv32);
        assert!(!CoreKind::Rv32.supported());
        let err = Device::from_desc("m", McuDesc::new(CoreKind::Rv32, "RV32")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("standalone"), "{msg}");
    }
}
