//! Zilog Z80 core (C++ `Z80Core`). Unified 64K program image plus address /
//! data bus GPIO (`PORTA` / `PORTD` / `MREQ` / `IORQ` / `RD` / `WR`).

use crate::device::CpuHost;
use crate::port::{Port, pin_idx, pin_inp, port_named, set_pin_inp, set_pin_out};

/// C++ `m_delay` — 10 ns after each clock edge.
const BUS_DELAY: u64 = 10_000;

const R_B: u8 = 0;
const R_C: u8 = 1;
const R_D: u8 = 2;
const R_E: u8 = 3;
const R_H: u8 = 4;
const R_L: u8 = 5;
const R_XH: u8 = 6;
const R_YH: u8 = 8;

const RP_BC: u8 = R_B;
const RP_DE: u8 = R_D;
const RP_HL: u8 = R_H;
const RP_IX: u8 = R_XH;
const RP_IY: u8 = R_YH;
const RP_AF: u8 = 10;
const RP_SP: u8 = 11;

const PREFIX_NONE: u8 = 0;
const PREFIX_CB: u8 = 1;
const PREFIX_ED: u8 = 2;
const PREFIX_IX: u8 = 3;
const PREFIX_IY: u8 = 4;

const ST_NONE: u8 = 0;
const ST_HALT: u8 = 1;
const ST_XY_OFFSET: u8 = 2;
const ST_XY_FINISH: u8 = 3;

const M1_FETCH: u8 = 0;
const M1_INT: u8 = 1;
const M1_NMI: u8 = 2;
const M1_HALT: u8 = 3;
const M1_SPECIAL_RESET: u8 = 4;

const BUS_NONE: u8 = 0;
const BUS_M1: u8 = 1;
const BUS_INT_ACK: u8 = 2;
const BUS_MEM_READ: u8 = 3;
const BUS_MEM_WRITE: u8 = 4;
const BUS_IO_READ: u8 = 5;
const BUS_IO_WRITE: u8 = 6;

const F_C: u8 = 0x01;
const F_N: u8 = 0x02;
const F_P: u8 = 0x04;
const F_X: u8 = 0x08;
const F_H: u8 = 0x10;
const F_Y: u8 = 0x20;
const F_Z: u8 = 0x40;
const F_S: u8 = 0x80;

const PROD_ZILOG: u8 = 0;
const PROD_NEC: u8 = 1;
const PROD_ST: u8 = 2;

const TABLE_M_CYCLES: [[u8; 256]; 3] = [
    [
        1, 3, 2, 1, 1, 1, 2, 1, 1, 3, 2, 1, 1, 1, 2, 1, 3, 3, 2, 1, 1, 1, 2, 1, 3, 3, 2, 1, 1, 1,
        2, 1, 3, 3, 5, 1, 1, 1, 2, 1, 3, 3, 5, 1, 1, 1, 2, 1, 3, 3, 4, 1, 3, 3, 3, 1, 3, 3, 4, 1,
        1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1,
        1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2, 2, 2, 2, 2, 2, 1, 2,
        1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1,
        2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1,
        1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 3, 3, 3, 3, 5, 3, 2, 3, 3, 3, 3, 1, 5, 5, 2, 3, 3, 3,
        3, 3, 5, 3, 2, 3, 3, 1, 3, 3, 5, 1, 2, 3, 3, 3, 3, 5, 5, 3, 2, 3, 3, 1, 3, 1, 5, 1, 2, 3,
        3, 3, 3, 1, 5, 3, 2, 3, 3, 1, 3, 1, 5, 1, 2, 3,
    ],
    [
        1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1,
        3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1,
        1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1,
        1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
        1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1,
        3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1,
        1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1,
        1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1,
        1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1,
    ],
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 2, 2, 3, 5, 1, 3, 1, 1, 2, 2, 3, 5, 1, 3, 1, 1, 2, 2, 3, 5, 1, 3, 1, 1, 2, 2,
        3, 5, 1, 3, 1, 1, 2, 2, 3, 5, 1, 3, 1, 4, 2, 2, 3, 5, 1, 3, 1, 4, 2, 2, 3, 5, 1, 3, 1, 1,
        2, 2, 3, 5, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 1, 1, 1, 1, 3, 3, 3, 3, 1, 1, 1, 1, 4, 4, 4, 4,
        1, 1, 1, 1, 4, 4, 4, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ],
];
const TABLE_M1_T: [[u8; 256]; 3] = [
    [
        4, 4, 4, 6, 4, 4, 4, 4, 4, 4, 4, 6, 4, 4, 4, 4, 5, 4, 4, 6, 4, 4, 4, 4, 4, 4, 4, 6, 4, 4,
        4, 4, 4, 4, 4, 6, 4, 4, 4, 4, 4, 4, 4, 6, 4, 4, 4, 4, 4, 4, 4, 6, 4, 4, 4, 4, 4, 4, 4, 6,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 5, 4, 5, 5, 4, 4, 4, 4, 4, 4, 5, 5, 4,
        4, 4, 4, 5, 4, 5, 5, 4, 4, 4, 4, 4, 4, 5, 5, 4, 4, 4, 4, 5, 4, 5, 5, 4, 4, 4, 4, 4, 4, 5,
        5, 4, 4, 4, 4, 5, 4, 5, 5, 6, 4, 4, 4, 4, 4, 5,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    ],
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4,
        4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 4, 4, 4, 4, 4, 4, 5, 5, 4, 4, 4, 4, 4, 4, 5, 5,
        4, 4, 4, 4, 4, 4, 5, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    ],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MathOp {
    Inc,
    Dec,
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
    Cpl,
    Neg,
    Daa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShiftOp {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Sll,
    Srl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Block {
    Inc,
    IncRep,
    Dec,
    DecRep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dest {
    A,
    Reg(u8),
    Sdi,
    Discard,
    PairLow(u8),
    PairHigh(u8),
}

#[derive(Clone, Copy, Debug)]
struct Flags {
    bits: u8,
    changed: bool,
    last_changed: bool,
}

impl Flags {
    fn clear_changed(&mut self) {
        self.last_changed = self.changed;
        self.changed = false;
    }
    fn is_changed(self) -> bool {
        self.last_changed
    }
    fn get_s(self) -> bool {
        self.bits & F_S != 0
    }
    fn get_z(self) -> bool {
        self.bits & F_Z != 0
    }
    fn get_h(self) -> bool {
        self.bits & F_H != 0
    }
    fn get_p(self) -> bool {
        self.bits & F_P != 0
    }
    fn get_n(self) -> bool {
        self.bits & F_N != 0
    }
    fn get_c(self) -> bool {
        self.bits & F_C != 0
    }
    fn set_h(&mut self) {
        self.bits |= F_H;
        self.changed = true;
    }
    fn set_n(&mut self) {
        self.bits |= F_N;
        self.changed = true;
    }
    fn set_c(&mut self) {
        self.bits |= F_C;
        self.changed = true;
    }
    fn reset_y(&mut self) {
        self.bits &= !F_Y;
        self.changed = true;
    }
    fn reset_h(&mut self) {
        self.bits &= !F_H;
        self.changed = true;
    }
    fn reset_x(&mut self) {
        self.bits &= !F_X;
        self.changed = true;
    }
    fn reset_n(&mut self) {
        self.bits &= !F_N;
        self.changed = true;
    }
    fn reset_c(&mut self) {
        self.bits &= !F_C;
        self.changed = true;
    }
    fn reset_hn(&mut self) {
        self.bits &= !(F_H | F_N);
        self.changed = true;
    }
    fn neg_c(&mut self) {
        self.bits ^= F_C;
        self.changed = true;
    }
    fn copy_yx(&mut self, r: u8) {
        self.bits = (self.bits & !(F_Y | F_X)) | (r & (F_Y | F_X));
        self.changed = true;
    }
    fn copy_neg_c_to_h(&mut self) {
        self.bits &= !F_H;
        self.bits |= (!self.bits << 4) & F_H;
        self.changed = true;
    }
    fn and_z(&mut self, z: bool) {
        if !z {
            self.bits &= !F_Z;
        }
        self.changed = true;
    }
    fn or_yx(&mut self, r: u8) {
        self.bits |= r & (F_Y | F_X);
        self.changed = true;
    }
    fn store_p(&mut self, p: bool) {
        self.bits &= !F_P;
        self.bits |= u8::from(p) << 2;
        self.changed = true;
    }
    fn store_c(&mut self, c: bool) {
        self.bits &= !F_C;
        self.bits |= u8::from(c);
        self.changed = true;
    }
    fn store_hc(&mut self, hc: bool) {
        self.bits &= !(F_H | F_C);
        let v = u8::from(hc);
        self.bits |= v | (v << 4);
        self.changed = true;
    }
    fn copy_szero_z(&mut self, r: u8) {
        self.bits &= !(F_S | F_Z);
        self.bits |= r & F_S;
        if r == 0 {
            self.bits |= F_Z;
        }
        self.changed = true;
    }
    fn copy_szero_zp(&mut self, r: u8) {
        self.bits &= !(F_S | F_Z | F_P);
        self.bits |= r & F_S;
        if r == 0 {
            self.bits |= F_Z | F_P;
        }
        self.changed = true;
    }
    fn copy_syx_zero_z(&mut self, r: u8) {
        self.bits &= !(F_S | F_Z | F_Y | F_X);
        self.bits |= r & (F_S | F_Y | F_X);
        if r == 0 {
            self.bits |= F_Z;
        }
        self.changed = true;
    }
    fn copy_syx_zero_z_parity_p(&mut self, r: u8) {
        self.bits &= !(F_S | F_Z | F_Y | F_X | F_P);
        self.bits |= r & (F_S | F_Y | F_X);
        if r == 0 {
            self.bits |= F_Z;
        }
        if !parity_odd(r) {
            self.bits |= F_P;
        }
        self.changed = true;
    }
    fn overflow_hp(&mut self, r: u8) {
        self.bits &= !(F_H | F_P);
        if r & 0x08 != 0 {
            self.bits |= F_H;
        }
        if r & 0xC0 == 0x80 || r & 0xC0 == 0x40 {
            self.bits |= F_P;
        }
        self.changed = true;
    }
    fn overflow_hc(&mut self, r: u8) {
        self.bits &= !(F_H | F_C);
        if r & 0x08 != 0 {
            self.bits |= F_H;
        }
        if r & 0x80 != 0 {
            self.bits |= F_C;
        }
        self.changed = true;
    }
    fn overflow_hpc(&mut self, r: u8) {
        self.bits &= !(F_H | F_P | F_C);
        if r & 0x08 != 0 {
            self.bits |= F_H;
        }
        if r & 0xC0 == 0x80 || r & 0xC0 == 0x40 {
            self.bits |= F_P;
        }
        if r & 0x80 != 0 {
            self.bits |= F_C;
        }
        self.changed = true;
    }
    fn parity_p(&mut self, r: u8) {
        self.bits &= !F_P;
        if !parity_odd(r) {
            self.bits |= F_P;
        }
        self.changed = true;
    }
    fn copy_bit1_yx(&mut self, r: u8) {
        self.bits &= !(F_Y | F_X);
        self.bits |= r & F_X;
        if r & 0x02 != 0 {
            self.bits |= F_Y;
        }
        self.changed = true;
    }
    fn copy_bit7_n(&mut self, r: u8) {
        self.bits &= !F_N;
        if r & 0x80 != 0 {
            self.bits |= F_N;
        }
        self.changed = true;
    }
    fn inotxx_ph(&mut self, r1: u8, r2: u8) {
        if self.bits & F_C == 0 {
            self.bits ^= (u8::from(parity_odd(r2 & 0x07)) << 2) & F_P;
        } else {
            self.bits &= !F_H;
            if r1 & 0x80 != 0 {
                if r2 & 0x0F == 0x00 {
                    self.bits |= F_H;
                }
                self.bits ^= (u8::from(parity_odd(r2.wrapping_sub(1) & 0x07)) << 2) & F_P;
            } else {
                if r2 & 0x0F == 0x0F {
                    self.bits |= F_H;
                }
                self.bits ^= (u8::from(parity_odd(r2.wrapping_add(1) & 0x07)) << 2) & F_P;
            }
        }
        self.changed = true;
    }
}

fn parity_odd(mut reg: u8) -> bool {
    reg ^= reg >> 1;
    reg ^= reg >> 2;
    reg ^= reg >> 4;
    reg & 0x01 != 0
}

#[derive(Clone, Debug)]
pub struct Z80 {
    pub a: u8,
    f: Flags,
    regs: [u8; 10],
    regs_alt: [u8; 6],
    a_alt: u8,
    f_alt: u8,
    i: u8,
    r: u8,
    wz: u16,
    sp: u16,
    pc: u16,
    iff1: bool,
    iff2: bool,
    producer: u8,
    cmos: bool,
    io_wait: bool,
    int_vector: bool,
    auto_wait: u8,
    wait_t: bool,
    s_wait: bool,
    s_bus_req: bool,
    s_bus_ack: bool,
    bus_op: u8,
    last_bus_op: u8,
    next_clock: bool,
    nmi_ff: bool,
    s_last_nmi: bool,
    s_nmi: bool,
    s_int: bool,
    s_di: u8,
    s_do: u8,
    s_ao: u16,
    high_z: bool,
    /// C++ `Z80Core` eElement delay (`m_delay` = 10 ns).
    pub pending_bus_ps: Option<u64>,
    mc_t_states: u8,
    mc_m_cycles: u8,
    mc_state: u8,
    mc_prefix: u8,
    i_set: u8,
    xy: u8,
    i_reg: u8,
    int_mode: u8,
    rst_count: u8,
    normal_reset: bool,
    special_reset: bool,
    sm_t: u8,
    sm_last_t: bool,
    sm_t_after_int: u32,
    sm_m: u8,
    sm_pre_xy_m: u8,
    sm_m1: u8,
    rlrd_tmp: u8,
    /// Active-low control pins; default inactive (high).
    pub reset_high: bool,
    pub nmi_high: bool,
    pub int_high: bool,
    pub wait_high: bool,
    pub busreq_high: bool,
}

impl Z80 {
    pub fn new() -> Self {
        let mut z = Self {
            a: 0,
            f: Flags {
                bits: 0,
                changed: false,
                last_changed: false,
            },
            regs: [0; 10],
            regs_alt: [0; 6],
            a_alt: 0,
            f_alt: 0,
            i: 0,
            r: 0,
            wz: 0,
            sp: 0,
            pc: 0,
            iff1: false,
            iff2: false,
            producer: PROD_ZILOG,
            cmos: false,
            io_wait: true,
            int_vector: false,
            auto_wait: 0,
            wait_t: false,
            s_wait: false,
            s_bus_req: false,
            s_bus_ack: false,
            bus_op: BUS_NONE,
            last_bus_op: BUS_NONE,
            next_clock: true,
            nmi_ff: false,
            s_last_nmi: false,
            s_nmi: false,
            s_int: false,
            s_di: 0,
            s_do: 0,
            s_ao: 0,
            high_z: true,
            pending_bus_ps: None,
            mc_t_states: 4,
            mc_m_cycles: 1,
            mc_state: ST_NONE,
            mc_prefix: PREFIX_NONE,
            i_set: PREFIX_NONE,
            xy: RP_HL,
            i_reg: 0,
            int_mode: 0,
            rst_count: 0,
            normal_reset: false,
            special_reset: false,
            sm_t: 0,
            sm_last_t: true,
            sm_t_after_int: 1_000_000,
            sm_m: 1,
            sm_pre_xy_m: 1,
            sm_m1: M1_FETCH,
            rlrd_tmp: 0,
            reset_high: true,
            nmi_high: true,
            int_high: true,
            wait_high: true,
            busreq_high: true,
        };
        z.reset(&mut []);
        z
    }

    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    pub fn get_reg8(&self, r: u8) -> u8 {
        if (r as usize) < self.regs.len() {
            self.regs[r as usize]
        } else {
            0
        }
    }

    pub fn get_reg16_pair(&self, r: u8) -> u16 {
        if r == RP_SP {
            self.sp
        } else if (r as usize + 1) < self.regs.len() {
            (u16::from(self.regs[r as usize]) << 8) | u16::from(self.regs[r as usize + 1])
        } else {
            0
        }
    }

    pub fn reset(&mut self, ports: &mut [Port]) {
        self.release_bus(ports, false);
        self.a = 0xFF;
        self.f.bits = 0xFF;
        self.a_alt = 0xFF;
        self.f_alt = 0xFF;
        self.i = 0;
        self.r = 0;
        self.wz = 0;
        self.sp = 0xFFFF;
        self.pc = 0;
        self.iff1 = false;
        self.iff2 = false;
        self.f.clear_changed();
        self.mc_t_states = 4;
        self.mc_m_cycles = 1;
        self.mc_state = ST_NONE;
        self.mc_prefix = PREFIX_NONE;
        self.bus_op = BUS_NONE;
        self.i_set = PREFIX_NONE;
        self.xy = RP_HL;
        self.last_bus_op = BUS_NONE;
        self.i_reg = 0x00;
        self.int_mode = 0;
        self.high_z = true;
        self.pending_bus_ps = None;
        self.sm_t = 0;
        self.sm_last_t = true;
        self.sm_t_after_int = 1_000_000;
        self.sm_m = 1;
        self.sm_pre_xy_m = 1;
        self.sm_m1 = M1_FETCH;
        self.wait_t = false;
        self.auto_wait = 0;
        self.nmi_ff = false;
        self.s_last_nmi = false;
        self.s_nmi = false;
        self.s_int = false;
        self.s_wait = false;
        self.s_bus_req = false;
        self.s_bus_ack = false;
        self.s_di = 0;
        self.s_do = 0;
        self.s_ao = 0;
        self.stamp(ports);
    }

    /// C++ `Z80Core::stamp`.
    pub fn stamp(&mut self, ports: &mut [Port]) {
        set_pin_out(ports, "MREQ", true);
        set_pin_out(ports, "IORQ", true);
        set_pin_out(ports, "RD", true);
        set_pin_out(ports, "WR", true);
        set_pin_out(ports, "RFSH", true);
        set_pin_out(ports, "M1", true);
        set_pin_out(ports, "HALT", true);
        set_pin_out(ports, "BUSAK", true);
        for name in ["WAIT", "INT", "NMI", "RESET", "BUSRQ"] {
            set_pin_inp(ports, name, true);
        }
        self.reset_high = pin_inp(ports, "RESET").unwrap_or(true);
        self.nmi_high = pin_inp(ports, "NMI").unwrap_or(true);
        self.int_high = pin_inp(ports, "INT").unwrap_or(true);
        self.wait_high = pin_inp(ports, "WAIT").unwrap_or(true);
        self.busreq_high = pin_inp(ports, "BUSRQ").unwrap_or(true);
    }

    fn has_bus(ports: &[Port]) -> bool {
        ports.iter().any(|p| p.name == "PORTA")
    }

    fn release_bus(&mut self, ports: &mut [Port], rel: bool) {
        let output = !rel;
        for name in ["MREQ", "IORQ", "RD", "WR", "RFSH"] {
            if let Some((pi, i)) = pin_idx(ports, name) {
                ports[pi].pins[i].is_out = output;
            }
        }
        if let Some(p) = port_named(ports, "PORTA") {
            p.set_pin_mode(output);
        }
        if let Some(p) = port_named(ports, "PORTD") {
            p.set_pin_mode(false);
        }
        self.high_z = rel;
    }

    pub fn flags(&self) -> u8 {
        self.f.bits
    }

    /// One clock edge (C++ `Z80Core::runStep`). Rising then falling alternate.
    /// C++ never sets `cyclesDone`; `eMcu` treats 0 as 1, so every edge is 1 tick.
    pub fn run_step(&mut self, h: &mut CpuHost) {
        h.cycles_done = 1;
        self.sample_ctrl(&h.ports);
        if self.next_clock {
            self.clk_rising(h);
        } else {
            self.clk_falling(h);
        }
        self.next_clock = !self.next_clock;
        if Self::has_bus(&h.ports) {
            self.pending_bus_ps = Some(BUS_DELAY);
        }
        h.pc = u32::from(self.pc);
    }

    fn sample_ctrl(&mut self, ports: &[Port]) {
        if let Some(v) = pin_inp(ports, "RESET") {
            self.reset_high = v;
        }
        if let Some(v) = pin_inp(ports, "NMI") {
            self.nmi_high = v;
        }
        if let Some(v) = pin_inp(ports, "INT") {
            self.int_high = v;
        }
        if let Some(v) = pin_inp(ports, "WAIT") {
            self.wait_high = v;
        }
        if let Some(v) = pin_inp(ports, "BUSRQ") {
            self.busreq_high = v;
        }
    }

    /// C++ `Z80Core::runEvent` → rising/falling delayed bus pins.
    pub fn run_bus(&mut self, h: &mut CpuHost) {
        if self.next_clock {
            self.falling_delayed(h);
        } else {
            self.rising_delayed(h);
        }
        self.pending_bus_ps = None;
    }

    fn rising_delayed(&mut self, h: &mut CpuHost) {
        if (self.s_bus_ack || self.normal_reset) && !self.high_z {
            set_pin_out(&mut h.ports, "BUSAK", false);
            self.release_bus(&mut h.ports, true);
        }
        if self.s_bus_ack || self.normal_reset {
            return;
        }
        if self.high_z {
            self.release_bus(&mut h.ports, false);
        }
        match self.sm_t {
            1 if !self.wait_t => {
                if let Some(p) = port_named(&mut h.ports, "PORTA") {
                    p.set_out_state(u32::from(self.s_ao));
                }
                if self.last_bus_op == BUS_MEM_WRITE || self.last_bus_op == BUS_IO_WRITE {
                    if let Some(p) = port_named(&mut h.ports, "PORTD") {
                        p.set_pin_mode(false);
                    }
                }
                if self.bus_op == BUS_M1 || self.bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "M1", false);
                }
                if self.last_bus_op == BUS_M1 || self.last_bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "RFSH", true);
                }
            }
            2 if !self.wait_t => {
                if self.bus_op == BUS_IO_READ {
                    set_pin_out(&mut h.ports, "IORQ", false);
                    set_pin_out(&mut h.ports, "RD", false);
                }
                if self.bus_op == BUS_IO_WRITE {
                    set_pin_out(&mut h.ports, "IORQ", false);
                    set_pin_out(&mut h.ports, "WR", false);
                }
            }
            3 => {
                if self.bus_op == BUS_M1 || self.bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "M1", true);
                    set_pin_out(&mut h.ports, "RFSH", false);
                    if let Some(p) = port_named(&mut h.ports, "PORTA") {
                        p.set_out_state(u32::from(self.i) << 8 | u32::from(self.r));
                    }
                    self.r = ((self.r.wrapping_add(1)) & 0x7f) + (self.r & 0x80);
                }
                if self.bus_op == BUS_M1 {
                    set_pin_out(&mut h.ports, "MREQ", true);
                    set_pin_out(&mut h.ports, "RD", true);
                }
                if self.bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "IORQ", true);
                }
            }
            5 => {
                if self.bus_op == BUS_M1 || self.bus_op == BUS_INT_ACK {
                    if let Some(p) = port_named(&mut h.ports, "PORTA") {
                        p.set_out_state(u32::from(self.s_ao));
                    }
                    set_pin_out(&mut h.ports, "RFSH", true);
                }
            }
            _ => {}
        }
    }

    fn falling_delayed(&mut self, h: &mut CpuHost) {
        if self.s_bus_ack {
            if !self.s_bus_req {
                set_pin_out(&mut h.ports, "BUSAK", true);
            }
            return;
        }
        match self.sm_t {
            1 if !self.wait_t => {
                if self.bus_op == BUS_M1 {
                    set_pin_out(&mut h.ports, "MREQ", false);
                    set_pin_out(&mut h.ports, "RD", false);
                }
                if self.bus_op == BUS_MEM_READ {
                    set_pin_out(&mut h.ports, "MREQ", false);
                    set_pin_out(&mut h.ports, "RD", false);
                }
                if self.bus_op == BUS_MEM_WRITE {
                    set_pin_out(&mut h.ports, "MREQ", false);
                }
                if self.bus_op == BUS_MEM_WRITE || self.bus_op == BUS_IO_WRITE {
                    if let Some(p) = port_named(&mut h.ports, "PORTD") {
                        p.set_pin_mode(true);
                        p.set_out_state(u32::from(self.s_do));
                    }
                }
            }
            2 if !self.wait_t => {
                if self.bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "IORQ", false);
                }
                if self.bus_op == BUS_MEM_WRITE {
                    set_pin_out(&mut h.ports, "WR", false);
                }
            }
            3 => {
                if self.bus_op == BUS_M1 || self.bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "MREQ", false);
                }
                if self.bus_op == BUS_MEM_READ {
                    set_pin_out(&mut h.ports, "MREQ", true);
                    set_pin_out(&mut h.ports, "RD", true);
                }
                if self.bus_op == BUS_MEM_WRITE {
                    set_pin_out(&mut h.ports, "MREQ", true);
                    set_pin_out(&mut h.ports, "WR", true);
                }
                if self.bus_op == BUS_IO_READ {
                    set_pin_out(&mut h.ports, "IORQ", true);
                    set_pin_out(&mut h.ports, "RD", true);
                }
                if self.bus_op == BUS_IO_WRITE {
                    set_pin_out(&mut h.ports, "IORQ", true);
                    set_pin_out(&mut h.ports, "WR", true);
                }
            }
            4 => {
                if self.bus_op == BUS_M1 || self.bus_op == BUS_INT_ACK {
                    set_pin_out(&mut h.ports, "MREQ", true);
                }
                if self.i_reg == 0x76 && self.i_set == PREFIX_NONE {
                    set_pin_out(&mut h.ports, "HALT", false);
                }
                if self.s_nmi || (self.s_int && self.iff1) {
                    set_pin_out(&mut h.ports, "HALT", true);
                }
            }
            _ => {}
        }
    }

    fn clk_rising(&mut self, h: &mut CpuHost) {
        if !self.reset_high {
            if !self.normal_reset {
                if self.rst_count == 2 {
                    self.normal_reset = true;
                } else {
                    self.rst_count = self.rst_count.wrapping_add(1);
                }
            }
        } else if self.normal_reset {
            if self.rst_count > 0 {
                self.rst_count -= 1;
            } else {
                self.reset(&mut h.ports);
                self.normal_reset = false;
            }
        } else {
            self.rst_count = 0;
        }
        if self.sm_t_after_int < 1_000_000 {
            self.sm_t_after_int += 1;
        }
        if !self.normal_reset && (!self.s_bus_req || !self.s_bus_ack) {
            self.s_bus_ack = false;
            self.last_bus_op = self.bus_op;
            if self.sm_last_t && !self.s_bus_req {
                self.run_mcode();
            }
            self.next_t_state();
            if self.sm_t == 3 && self.sm_m == 1 {
                self.op_code_fetch(h);
            }
            if self.sm_t == 4 && self.sm_m == 7 && self.mc_prefix == PREFIX_CB {
                self.i_reg = self.s_di;
                self.pc = self.pc.wrapping_add(1);
                self.i_set = PREFIX_CB;
                self.mc_m_cycles = TABLE_M_CYCLES[PREFIX_CB as usize][self.i_reg as usize];
            }
        }
        self.s_last_nmi = self.s_nmi;
        self.s_nmi = !self.nmi_high;
        if !self.s_last_nmi && self.s_nmi {
            self.nmi_ff = true;
        }
        self.s_int = !self.int_high;
        self.s_bus_req = !self.busreq_high;
    }

    fn clk_falling(&mut self, h: &mut CpuHost) {
        self.s_wait = !self.wait_high;
        if self.s_bus_ack {
            return;
        }
        if self.sm_t == 3 {
            match self.bus_op {
                BUS_MEM_READ => self.s_di = mem(h, self.s_ao),
                BUS_IO_READ => self.s_di = 0xFF,
                BUS_MEM_WRITE => write_mem(h, self.s_ao, self.s_do),
                _ => {}
            }
        }
    }

    fn next_t_state(&mut self) {
        if self.sm_t != 2 || !self.s_wait {
            if self.sm_last_t {
                if self.mc_state == ST_HALT {
                    self.sm_m1 = M1_HALT;
                    self.mc_state = ST_NONE;
                }
                if self.s_bus_req {
                    self.s_bus_ack = true;
                } else {
                    self.sm_t = 1;
                    if self.mc_state == ST_XY_OFFSET {
                        self.sm_pre_xy_m = self.sm_m;
                        self.sm_m = 6;
                        self.mc_state = ST_NONE;
                    } else if self.sm_m == 7 || self.mc_state == ST_XY_FINISH {
                        self.sm_m = self.sm_pre_xy_m.wrapping_add(1);
                        self.mc_state = ST_NONE;
                    } else if self.sm_m == self.mc_m_cycles {
                        self.sm_m = 1;
                        self.bus_op = BUS_M1;
                        if self.sm_m1 != M1_HALT {
                            self.sm_m1 = M1_FETCH;
                        }
                        if self.special_reset {
                            self.sm_m1 = M1_SPECIAL_RESET;
                            self.special_reset = false;
                        }
                        if self.nmi_ff && self.mc_prefix == PREFIX_NONE {
                            self.nmi_ff = false;
                            self.sm_m1 = M1_NMI;
                            self.iff1 = false;
                        } else if self.s_int && self.iff1 && self.mc_prefix == PREFIX_NONE {
                            self.bus_op = BUS_INT_ACK;
                            self.sm_m1 = M1_INT;
                            self.auto_wait = 1;
                            self.iff1 = false;
                            self.iff2 = false;
                            self.sm_t_after_int = 0;
                        }
                    } else {
                        self.sm_m = self.sm_m.wrapping_add(1);
                    }
                }
            } else {
                self.wait_t = self.auto_wait > 0;
                if !self.wait_t {
                    self.sm_t = self.sm_t.wrapping_add(1);
                    if self.io_wait
                        && self.sm_t == 2
                        && matches!(self.bus_op, BUS_IO_READ | BUS_IO_WRITE | BUS_INT_ACK)
                    {
                        self.auto_wait = 1;
                    }
                    if self.sm_t == 2 && !self.reset_high {
                        self.special_reset = true;
                    }
                } else {
                    self.auto_wait -= 1;
                }
            }
            self.sm_last_t = self.sm_t == self.mc_t_states;
        } else {
            self.wait_t = true;
        }
    }

    fn op_code_fetch(&mut self, h: &mut CpuHost) {
        match self.mc_prefix {
            PREFIX_NONE => {
                self.xy = RP_HL;
                self.i_set = PREFIX_NONE;
            }
            PREFIX_CB => self.i_set = PREFIX_CB,
            PREFIX_ED => {
                self.xy = RP_HL;
                self.i_set = PREFIX_ED;
            }
            PREFIX_IX => {
                self.xy = RP_IX;
                self.i_set = PREFIX_NONE;
            }
            PREFIX_IY => {
                self.xy = RP_IY;
                self.i_set = PREFIX_NONE;
            }
            _ => {}
        }
        match self.sm_m1 {
            M1_FETCH => {
                self.i_reg = mem(h, self.s_ao);
                self.pc = self.pc.wrapping_add(1);
                let set = self.i_set as usize;
                self.mc_m_cycles = TABLE_M_CYCLES[set][self.i_reg as usize];
                self.mc_t_states = TABLE_M1_T[set][self.i_reg as usize];
            }
            M1_INT => {
                match self.int_mode {
                    0 => self.i_reg = 0xFF,
                    1 => self.i_reg = 0xFF,
                    2 => {
                        self.i_reg = 0x00;
                        self.s_di = 0xFF;
                        self.mc_m_cycles = 5;
                        self.mc_t_states = 5;
                    }
                    _ => self.i_reg = 0x00,
                }
                if self.int_mode != 2 {
                    let set = self.i_set as usize;
                    self.mc_m_cycles = TABLE_M_CYCLES[set][self.i_reg as usize];
                    self.mc_t_states = TABLE_M1_T[set][self.i_reg as usize];
                }
            }
            M1_NMI => {
                self.i_reg = 0x00;
                self.mc_m_cycles = 3;
                self.mc_t_states = 5;
            }
            M1_SPECIAL_RESET => {
                self.pc = 0;
                self.i_set = PREFIX_NONE;
                self.i_reg = 0x00;
                let set = self.i_set as usize;
                self.mc_m_cycles = TABLE_M_CYCLES[set][self.i_reg as usize];
                self.mc_t_states = TABLE_M1_T[set][self.i_reg as usize];
            }
            M1_HALT => {
                self.i_reg = 0x00;
                let set = self.i_set as usize;
                self.mc_m_cycles = TABLE_M_CYCLES[set][self.i_reg as usize];
                self.mc_t_states = TABLE_M1_T[set][self.i_reg as usize];
            }
            _ => {}
        }
    }

    fn run_mcode(&mut self) {
        self.mc_t_states = 3;
        self.s_ao = self.pc;
        self.mc_prefix = PREFIX_NONE;
        self.f.clear_changed();
        match self.i_set {
            PREFIX_NONE => self.run_unprefixed(),
            PREFIX_CB => self.run_cb(),
            PREFIX_ED => self.run_ed(),
            _ => {}
        }
        if self.sm_m == self.mc_m_cycles {
            self.mc_t_states = 4;
        }
    }

    fn run_unprefixed(&mut self) {
        let ir = self.i_reg;
        match ir {
            0x00 => {
                if self.sm_m1 == M1_INT && self.int_mode == 2 {
                    self.int_im2();
                }
                if self.sm_m1 == M1_NMI {
                    self.rst(0x66);
                }
            }
            0x01 => self.ld_rr_imm(RP_BC),
            0x02 => self.ld_indir_r(RP_BC, Dest::A),
            0x03 => self.set_pair(RP_BC, self.pair(RP_BC).wrapping_add(1)),
            0x04 => self.math_op(MathOp::Inc, Dest::Reg(R_B), 0, true),
            0x05 => self.math_op(MathOp::Dec, Dest::Reg(R_B), 0, true),
            0x06 => self.ld_r_imm(Dest::Reg(R_B)),
            0x07 => self.shift_op(ShiftOp::Rlc, Dest::A, false),
            0x08 => self.ex_af(),
            0x09 => self.math_op16(MathOp::Add, self.xy, RP_BC),
            0x0A => self.ld_r_indir(Dest::A, RP_BC),
            0x0B => self.set_pair(RP_BC, self.pair(RP_BC).wrapping_sub(1)),
            0x0C => self.math_op(MathOp::Inc, Dest::Reg(R_C), 0, true),
            0x0D => self.math_op(MathOp::Dec, Dest::Reg(R_C), 0, true),
            0x0E => self.ld_r_imm(Dest::Reg(R_C)),
            0x0F => self.shift_op(ShiftOp::Rrc, Dest::A, false),
            0x10 => {
                if self.sm_m == 1 {
                    self.regs[R_B as usize] = self.regs[R_B as usize].wrapping_sub(1);
                }
                let cond = self.regs[R_B as usize] != 0;
                self.jr(cond);
            }
            0x11 => self.ld_rr_imm(RP_DE),
            0x12 => self.ld_indir_r(RP_DE, Dest::A),
            0x13 => self.set_pair(RP_DE, self.pair(RP_DE).wrapping_add(1)),
            0x14 => self.math_op(MathOp::Inc, Dest::Reg(R_D), 0, true),
            0x15 => self.math_op(MathOp::Dec, Dest::Reg(R_D), 0, true),
            0x16 => self.ld_r_imm(Dest::Reg(R_D)),
            0x17 => self.shift_op(ShiftOp::Rl, Dest::A, false),
            0x18 => self.jr(true),
            0x19 => self.math_op16(MathOp::Add, self.xy, RP_DE),
            0x1A => self.ld_r_indir(Dest::A, RP_DE),
            0x1B => self.set_pair(RP_DE, self.pair(RP_DE).wrapping_sub(1)),
            0x1C => self.math_op(MathOp::Inc, Dest::Reg(R_E), 0, true),
            0x1D => self.math_op(MathOp::Dec, Dest::Reg(R_E), 0, true),
            0x1E => self.ld_r_imm(Dest::Reg(R_E)),
            0x1F => self.shift_op(ShiftOp::Rr, Dest::A, false),
            0x20 => self.jr(!self.f.get_z()),
            0x21 => self.ld_rr_imm(self.xy),
            0x22 => self.ld_mem_rr(self.xy),
            0x23 => self.set_pair(self.xy, self.pair(self.xy).wrapping_add(1)),
            0x24 => self.math_op(MathOp::Inc, Dest::Reg(self.xy), 0, true),
            0x25 => self.math_op(MathOp::Dec, Dest::Reg(self.xy), 0, true),
            0x26 => self.ld_r_imm(Dest::Reg(self.xy)),
            0x27 => self.math_op(MathOp::Daa, Dest::A, 0, true),
            0x28 => self.jr(self.f.get_z()),
            0x29 => self.math_op16(MathOp::Add, self.xy, self.xy),
            0x2A => self.ld_rr_mem(self.xy),
            0x2B => self.set_pair(self.xy, self.pair(self.xy).wrapping_sub(1)),
            0x2C => self.math_op(MathOp::Inc, Dest::Reg(self.xy + 1), 0, true),
            0x2D => self.math_op(MathOp::Dec, Dest::Reg(self.xy + 1), 0, true),
            0x2E => self.ld_r_imm(Dest::Reg(self.xy + 1)),
            0x2F => self.math_op(MathOp::Cpl, Dest::A, 0, true),
            0x30 => self.jr(!self.f.get_c()),
            0x31 => self.ld_rr_imm(RP_SP),
            0x32 => self.ld_mem_r(Dest::A),
            0x33 => self.sp = self.sp.wrapping_add(1),
            0x34 => self.inst_indir_xy(MathOp::Inc),
            0x35 => self.inst_indir_xy(MathOp::Dec),
            0x36 => self.ld_indir_xy_imm(),
            0x37 => {
                self.f.set_c();
                self.flags_scf_ccf();
            }
            0x38 => self.jr(self.f.get_c()),
            0x39 => self.math_op16(MathOp::Add, self.xy, RP_SP),
            0x3A => self.ld_r_mem(Dest::A),
            0x3B => self.sp = self.sp.wrapping_sub(1),
            0x3C => self.math_op(MathOp::Inc, Dest::A, 0, true),
            0x3D => self.math_op(MathOp::Dec, Dest::A, 0, true),
            0x3E => self.ld_r_imm(Dest::A),
            0x3F => {
                self.f.neg_c();
                self.flags_scf_ccf();
            }
            0x40..=0x75 | 0x77..=0x7F => {
                if ir == 0x76 {
                    self.mc_state = ST_HALT;
                } else {
                    let dst = (ir >> 3) & 7;
                    let src = ir & 7;
                    if src == 6 {
                        self.ld_r_indir_xy(ld_dst(dst));
                    } else if dst == 6 {
                        self.ld_indir_xy_r(ld_src_real(src));
                    } else {
                        let v = self.r8_xy(src);
                        self.set_r8_xy(dst, v);
                    }
                }
            }
            0x76 => self.mc_state = ST_HALT,
            0x80..=0xBF => {
                let op = alu_op((ir >> 3) & 7);
                let src = ir & 7;
                if src == 6 {
                    self.inst_indir_xy(op);
                } else {
                    let v = self.r8_xy(src);
                    self.math_op(op, Dest::A, v, true);
                }
            }
            0xC0 => self.ret(!self.f.get_z()),
            0xC1 => self.pop_rr(RP_BC),
            0xC2 => self.jp(!self.f.get_z()),
            0xC3 => self.jp(true),
            0xC4 => self.call(!self.f.get_z()),
            0xC5 => self.push_rr(RP_BC),
            0xC6 => self.inst_imm(MathOp::Add),
            0xC7 => self.rst(0x00),
            0xC8 => self.ret(self.f.get_z()),
            0xC9 => self.ret(true),
            0xCA => self.jp(self.f.get_z()),
            0xCB => self.prefix_cb(),
            0xCC => self.call(self.f.get_z()),
            0xCD => self.call(true),
            0xCE => self.inst_imm(MathOp::Adc),
            0xCF => self.rst(0x08),
            0xD0 => self.ret(!self.f.get_c()),
            0xD1 => self.pop_rr(RP_DE),
            0xD2 => self.jp(!self.f.get_c()),
            0xD3 => self.out_imm_r(Dest::A),
            0xD4 => self.call(!self.f.get_c()),
            0xD5 => self.push_rr(RP_DE),
            0xD6 => self.inst_imm(MathOp::Sub),
            0xD7 => self.rst(0x10),
            0xD8 => self.ret(self.f.get_c()),
            0xD9 => self.exx(),
            0xDA => self.jp(self.f.get_c()),
            0xDB => self.in_r_imm(Dest::A),
            0xDC => self.call(self.f.get_c()),
            0xDD => self.mc_prefix = PREFIX_IX,
            0xDE => self.inst_imm(MathOp::Sbc),
            0xDF => self.rst(0x18),
            0xE0 => self.ret(!self.f.get_p()),
            0xE1 => self.pop_rr(self.xy),
            0xE2 => self.jp(!self.f.get_p()),
            0xE3 => self.ex_sp_rr(self.xy),
            0xE4 => self.call(!self.f.get_p()),
            0xE5 => self.push_rr(self.xy),
            0xE6 => self.inst_imm(MathOp::And),
            0xE7 => self.rst(0x20),
            0xE8 => self.ret(self.f.get_p()),
            0xE9 => {
                self.pc = self.pair(self.xy);
                self.s_ao = self.pc;
            }
            0xEA => self.jp(self.f.get_p()),
            0xEB => self.ex_de_hl(),
            0xEC => self.call(self.f.get_p()),
            0xED => self.mc_prefix = PREFIX_ED,
            0xEE => self.inst_imm(MathOp::Xor),
            0xEF => self.rst(0x28),
            0xF0 => self.ret(!self.f.get_s()),
            0xF1 => self.pop_rr(RP_AF),
            0xF2 => self.jp(!self.f.get_s()),
            0xF3 => {
                self.iff1 = false;
                self.iff2 = false;
            }
            0xF4 => self.call(!self.f.get_s()),
            0xF5 => self.push_rr(RP_AF),
            0xF6 => self.inst_imm(MathOp::Or),
            0xF7 => self.rst(0x30),
            0xF8 => self.ret(self.f.get_s()),
            0xF9 => self.sp = self.pair(self.xy),
            0xFA => self.jp(self.f.get_s()),
            0xFB => {
                self.iff1 = true;
                self.iff2 = true;
                self.s_int = false;
            }
            0xFC => self.call(self.f.get_s()),
            0xFD => self.mc_prefix = PREFIX_IY,
            0xFE => self.inst_imm(MathOp::Cp),
            0xFF => self.rst(0x38),
        }
    }

    fn run_cb(&mut self) {
        let d = match self.i_reg & 0x07 {
            0 => Dest::Reg(R_B),
            1 => Dest::Reg(R_C),
            2 => Dest::Reg(R_D),
            3 => Dest::Reg(R_E),
            4 => Dest::Reg(R_H),
            5 => Dest::Reg(R_L),
            6 => Dest::Discard,
            _ => Dest::A,
        };
        self.inst_cb(self.i_reg, d);
    }

    fn run_ed(&mut self) {
        match self.i_reg {
            0x40 => self.in_r_rr(Dest::Reg(R_B), RP_BC),
            0x41 => self.out_rr_r(RP_BC, Dest::Reg(R_B)),
            0x42 => self.math_op16(MathOp::Sbc, RP_HL, RP_BC),
            0x43 => self.ld_mem_rr(RP_BC),
            0x44 | 0x4C | 0x54 | 0x5C | 0x64 | 0x6C | 0x74 | 0x7C => {
                self.math_op(MathOp::Neg, Dest::A, 0, true);
            }
            0x45 | 0x4D | 0x55 | 0x5D | 0x65 | 0x6D | 0x75 | 0x7D => self.retn(),
            0x46 | 0x4E | 0x66 | 0x6E => self.int_mode = 0,
            0x47 => self.i = self.a,
            0x48 => self.in_r_rr(Dest::Reg(R_C), RP_BC),
            0x49 => self.out_rr_r(RP_BC, Dest::Reg(R_C)),
            0x4A => self.math_op16(MathOp::Adc, RP_HL, RP_BC),
            0x4B => self.ld_rr_mem(RP_BC),
            0x4F => self.r = self.a,
            0x50 => self.in_r_rr(Dest::Reg(R_D), RP_BC),
            0x51 => self.out_rr_r(RP_BC, Dest::Reg(R_D)),
            0x52 => self.math_op16(MathOp::Sbc, RP_HL, RP_DE),
            0x53 => self.ld_mem_rr(RP_DE),
            0x56 | 0x76 => self.int_mode = 1,
            0x57 => {
                self.a = self.i;
                let a = self.a;
                let iff2 = self.iff2;
                self.f.copy_syx_zero_z(a);
                self.f.reset_hn();
                self.f.store_p(iff2);
            }
            0x58 => self.in_r_rr(Dest::Reg(R_E), RP_BC),
            0x59 => self.out_rr_r(RP_BC, Dest::Reg(R_E)),
            0x5A => self.math_op16(MathOp::Adc, RP_HL, RP_DE),
            0x5B => self.ld_rr_mem(RP_DE),
            0x5E | 0x7E => self.int_mode = 2,
            0x5F => {
                self.a = self.r;
                let a = self.a;
                let iff2 = self.iff2;
                self.f.copy_syx_zero_z(a);
                self.f.reset_hn();
                self.f.store_p(iff2);
            }
            0x60 => self.in_r_rr(Dest::Reg(R_H), RP_BC),
            0x61 => self.out_rr_r(RP_BC, Dest::Reg(R_H)),
            0x62 => self.math_op16(MathOp::Sbc, RP_HL, RP_HL),
            0x63 => self.ld_mem_rr(RP_HL),
            0x67 => self.rlrd(false),
            0x68 => self.in_r_rr(Dest::Reg(R_L), RP_BC),
            0x69 => self.out_rr_r(RP_BC, Dest::Reg(R_L)),
            0x6A => self.math_op16(MathOp::Adc, RP_HL, RP_HL),
            0x6B => self.ld_rr_mem(RP_HL),
            0x6F => self.rlrd(true),
            0x70 => self.in_r_rr(Dest::Discard, RP_BC),
            0x71 => {
                let v = if self.cmos { 0xFF } else { 0x00 };
                self.out_rr_imm(RP_BC, v);
            }
            0x72 => self.math_op16(MathOp::Sbc, RP_HL, RP_SP),
            0x73 => self.ld_mem_rr(RP_SP),
            0x78 => self.in_r_rr(Dest::A, RP_BC),
            0x79 => self.out_rr_r(RP_BC, Dest::A),
            0x7A => self.math_op16(MathOp::Adc, RP_HL, RP_SP),
            0x7B => self.ld_rr_mem(RP_SP),
            0xA0 => self.ldxx(Block::Inc),
            0xA1 => self.cpxx(Block::Inc),
            0xA2 => self.inxx(Block::Inc),
            0xA3 => self.otxx(Block::Inc),
            0xA8 => self.ldxx(Block::Dec),
            0xA9 => self.cpxx(Block::Dec),
            0xAA => self.inxx(Block::Dec),
            0xAB => self.otxx(Block::Dec),
            0xB0 => self.ldxx(Block::IncRep),
            0xB1 => self.cpxx(Block::IncRep),
            0xB2 => self.inxx(Block::IncRep),
            0xB3 => self.otxx(Block::IncRep),
            0xB8 => self.ldxx(Block::DecRep),
            0xB9 => self.cpxx(Block::DecRep),
            0xBA => self.inxx(Block::DecRep),
            0xBB => self.otxx(Block::DecRep),
            _ => {}
        }
    }

    fn pair(&self, p: u8) -> u16 {
        match p {
            RP_AF => (u16::from(self.a) << 8) | u16::from(self.f.bits),
            RP_SP => self.sp,
            p => (u16::from(self.regs[p as usize]) << 8) | u16::from(self.regs[p as usize + 1]),
        }
    }

    fn set_pair(&mut self, p: u8, v: u16) {
        match p {
            RP_AF => {
                self.a = (v >> 8) as u8;
                self.f.bits = v as u8;
            }
            RP_SP => self.sp = v,
            p => {
                self.regs[p as usize] = (v >> 8) as u8;
                self.regs[p as usize + 1] = v as u8;
            }
        }
    }

    fn pair_low(&self, p: u8) -> u8 {
        self.pair(p) as u8
    }
    fn pair_high(&self, p: u8) -> u8 {
        (self.pair(p) >> 8) as u8
    }
    fn set_pair_low(&mut self, p: u8, v: u8) {
        let n = (self.pair(p) & 0xFF00) | u16::from(v);
        self.set_pair(p, n);
    }
    fn set_pair_high(&mut self, p: u8, v: u8) {
        let n = (self.pair(p) & 0x00FF) | (u16::from(v) << 8);
        self.set_pair(p, n);
    }

    fn get_dest(&self, d: Dest) -> u8 {
        match d {
            Dest::A => self.a,
            Dest::Reg(i) => self.regs[i as usize],
            Dest::Sdi => self.s_di,
            Dest::Discard => 0,
            Dest::PairLow(p) => self.pair_low(p),
            Dest::PairHigh(p) => self.pair_high(p),
        }
    }

    fn set_dest(&mut self, d: Dest, v: u8) {
        match d {
            Dest::A => self.a = v,
            Dest::Reg(i) => self.regs[i as usize] = v,
            Dest::Sdi => self.s_di = v,
            Dest::Discard => {}
            Dest::PairLow(p) => self.set_pair_low(p, v),
            Dest::PairHigh(p) => self.set_pair_high(p, v),
        }
    }

    fn r8_xy(&self, r: u8) -> u8 {
        match r & 7 {
            0 => self.regs[R_B as usize],
            1 => self.regs[R_C as usize],
            2 => self.regs[R_D as usize],
            3 => self.regs[R_E as usize],
            4 => self.regs[self.xy as usize],
            5 => self.regs[self.xy as usize + 1],
            7 => self.a,
            _ => 0,
        }
    }

    fn set_r8_xy(&mut self, r: u8, v: u8) {
        match r & 7 {
            0 => self.regs[R_B as usize] = v,
            1 => self.regs[R_C as usize] = v,
            2 => self.regs[R_D as usize] = v,
            3 => self.regs[R_E as usize] = v,
            4 => self.regs[self.xy as usize] = v,
            5 => self.regs[self.xy as usize + 1] = v,
            7 => self.a = v,
            _ => {}
        }
    }

    fn read_mem(&mut self, addr: u16) {
        self.bus_op = BUS_MEM_READ;
        self.s_ao = addr;
    }
    fn write_mem_bus(&mut self, addr: u16, data: u8) {
        self.bus_op = BUS_MEM_WRITE;
        self.s_ao = addr;
        self.s_do = data;
    }
    fn read_io(&mut self, addr: u16) {
        self.bus_op = BUS_IO_READ;
        self.s_ao = addr;
    }
    fn write_io(&mut self, addr: u16, data: u8) {
        self.bus_op = BUS_IO_WRITE;
        self.s_ao = addr;
        self.s_do = data;
    }
    fn no_bus_op(&mut self, addr: u16, tstates: u8) {
        self.bus_op = BUS_NONE;
        self.s_ao = addr;
        self.mc_t_states = tstates;
    }
    fn push_stack8(&mut self, v: u8) {
        self.sp = self.sp.wrapping_sub(1);
        self.write_mem_bus(self.sp, v);
    }
    fn pop_stack8(&mut self) {
        self.read_mem(self.sp);
        self.sp = self.sp.wrapping_add(1);
    }
    fn jump_wz(&mut self) {
        self.pc = self.wz;
        self.s_ao = self.pc;
    }
    fn finish_inst(&mut self) {
        self.mc_m_cycles = self.sm_m;
    }
    fn mask(ireg: u8) -> u8 {
        1 << ((ireg >> 3) & 0x07)
    }

    fn flags_scf_ccf(&mut self) {
        match self.producer {
            PROD_ZILOG => {
                if self.f.is_changed() {
                    self.f.reset_x();
                    self.f.reset_y();
                }
            }
            PROD_NEC => {
                self.f.reset_x();
                self.f.reset_y();
            }
            PROD_ST => {
                self.f.reset_x();
                if self.f.is_changed() {
                    self.f.reset_y();
                }
            }
            _ => {}
        }
        let a = self.a;
        self.f.or_yx(a);
        self.f.copy_neg_c_to_h();
        self.f.reset_n();
    }

    fn ld_r_imm(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                let v = self.s_di;
                self.set_dest(d, v);
            }
            _ => {}
        }
    }

    fn ld_r_indir(&mut self, d: Dest, rp: u8) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(rp);
                self.read_mem(self.wz);
            }
            2 => {
                let v = self.s_di;
                self.set_dest(d, v);
                self.wz = self.wz.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn ld_r_indir_xy(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                if self.xy != RP_HL {
                    self.mc_state = ST_XY_OFFSET;
                    self.read_mem(self.pc);
                } else {
                    self.read_mem(self.pair(RP_HL));
                }
            }
            2 => {
                let v = self.s_di;
                self.set_dest(d, v);
            }
            6 => {
                self.wz = self
                    .pair(self.xy)
                    .wrapping_add_signed(self.s_di as i8 as i16);
                self.no_bus_op(self.pc, 5);
            }
            7 => {
                self.read_mem(self.wz);
                self.pc = self.pc.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn ld_r_mem(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                self.read_mem(self.wz);
            }
            4 => {
                let v = self.s_di;
                self.set_dest(d, v);
                self.wz = self.wz.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn ld_indir_r(&mut self, rp: u8, d: Dest) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(rp);
                let v = self.get_dest(d);
                self.write_mem_bus(self.wz, v);
            }
            2 => {
                self.wz = self.wz.wrapping_add(1);
                self.wz = (self.wz & 0x00FF) | (u16::from(self.a) << 8);
            }
            _ => {}
        }
    }

    fn ld_indir_xy_r(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                if self.xy != RP_HL {
                    self.mc_state = ST_XY_OFFSET;
                    self.read_mem(self.pc);
                } else {
                    let v = self.get_dest(d);
                    self.write_mem_bus(self.pair(RP_HL), v);
                }
            }
            2 => {}
            6 => {
                self.wz = self
                    .pair(self.xy)
                    .wrapping_add_signed(self.s_di as i8 as i16);
                self.no_bus_op(self.pc, 5);
            }
            7 => {
                let v = self.get_dest(d);
                self.write_mem_bus(self.wz, v);
                self.pc = self.pc.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn ld_mem_r(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                let v = self.get_dest(d);
                self.write_mem_bus(self.wz, v);
                self.wz = self.wz.wrapping_add(1);
            }
            4 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.a) << 8);
            }
            _ => {}
        }
    }

    fn ld_indir_xy_imm(&mut self) {
        match self.sm_m {
            1 => {
                if self.xy != RP_HL {
                    self.mc_state = ST_XY_OFFSET;
                }
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                if self.xy != RP_HL {
                    self.write_mem_bus(self.wz, self.s_di);
                } else {
                    self.write_mem_bus(self.pair(RP_HL), self.s_di);
                }
            }
            3 => {}
            6 => {
                self.wz = self
                    .pair(self.xy)
                    .wrapping_add_signed(self.s_di as i8 as i16);
                self.mc_state = ST_XY_FINISH;
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
                self.mc_t_states = 5;
            }
            _ => {}
        }
    }

    fn ld_rr_imm(&mut self, rp: u8) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.set_pair_low(rp, self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            3 => self.set_pair_high(rp, self.s_di),
            _ => {}
        }
    }

    fn ld_rr_mem(&mut self, rp: u8) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                self.read_mem(self.wz);
                self.wz = self.wz.wrapping_add(1);
            }
            4 => {
                self.set_pair_low(rp, self.s_di);
                self.read_mem(self.wz);
            }
            5 => self.set_pair_high(rp, self.s_di),
            _ => {}
        }
    }

    fn ld_mem_rr(&mut self, rp: u8) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                self.write_mem_bus(self.wz, self.pair_low(rp));
                self.wz = self.wz.wrapping_add(1);
            }
            4 => self.write_mem_bus(self.wz, self.pair_high(rp)),
            5 => {}
            _ => {}
        }
    }

    fn push_rr(&mut self, rp: u8) {
        match self.sm_m {
            1 => self.push_stack8(self.pair_high(rp)),
            2 => self.push_stack8(self.pair_low(rp)),
            3 => {}
            _ => {}
        }
    }

    fn pop_rr(&mut self, rp: u8) {
        match self.sm_m {
            1 => self.pop_stack8(),
            2 => {
                self.set_pair_low(rp, self.s_di);
                self.pop_stack8();
            }
            3 => self.set_pair_high(rp, self.s_di),
            _ => {}
        }
    }

    fn ex_af(&mut self) {
        let a = self.a;
        let f = self.f.bits;
        self.a = self.a_alt;
        self.f.bits = self.f_alt;
        self.a_alt = a;
        self.f_alt = f;
    }

    fn exx(&mut self) {
        for (i, alt) in [(R_B, 0), (R_C, 1), (R_D, 2), (R_E, 3), (R_H, 4), (R_L, 5)] {
            let t = self.regs[i as usize];
            self.regs[i as usize] = self.regs_alt[alt];
            self.regs_alt[alt] = t;
        }
    }

    fn ex_de_hl(&mut self) {
        let h = self.regs[R_H as usize];
        let l = self.regs[R_L as usize];
        self.regs[R_H as usize] = self.regs[R_D as usize];
        self.regs[R_L as usize] = self.regs[R_E as usize];
        self.regs[R_D as usize] = h;
        self.regs[R_E as usize] = l;
    }

    fn ex_sp_rr(&mut self, rp: u8) {
        match self.sm_m {
            1 => {
                self.read_mem(self.sp);
                self.sp = self.sp.wrapping_add(1);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                self.read_mem(self.sp);
                self.mc_t_states = 4;
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                self.write_mem_bus(self.sp, self.pair_high(rp));
                self.sp = self.sp.wrapping_sub(1);
                self.set_pair_high(rp, (self.wz >> 8) as u8);
            }
            4 => {
                self.write_mem_bus(self.sp, self.pair_low(rp));
                self.set_pair_low(rp, self.wz as u8);
                self.mc_t_states = 5;
            }
            5 => {}
            _ => {}
        }
    }

    fn inst_indir_xy(&mut self, op: MathOp) {
        match self.sm_m {
            1 => {
                if self.xy != RP_HL {
                    self.mc_state = ST_XY_OFFSET;
                    self.read_mem(self.pc);
                } else {
                    self.read_mem(self.pair(RP_HL));
                    if op == MathOp::Inc || op == MathOp::Dec {
                        self.mc_t_states = 4;
                    }
                }
            }
            2 => {
                if op == MathOp::Inc || op == MathOp::Dec {
                    self.math_op(op, Dest::Sdi, 0, true);
                    if self.xy != RP_HL {
                        self.write_mem_bus(self.wz, self.s_di);
                    } else {
                        self.write_mem_bus(self.pair(RP_HL), self.s_di);
                    }
                } else {
                    let v = self.s_di;
                    self.math_op(op, Dest::A, v, true);
                }
            }
            3 => {}
            6 => {
                self.wz = self
                    .pair(self.xy)
                    .wrapping_add_signed(self.s_di as i8 as i16);
                self.no_bus_op(self.pc, 5);
            }
            7 => {
                self.read_mem(self.wz);
                self.pc = self.pc.wrapping_add(1);
                if op == MathOp::Inc || op == MathOp::Dec {
                    self.mc_t_states = 4;
                }
            }
            _ => {}
        }
    }

    fn inst_imm(&mut self, op: MathOp) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                let v = self.s_di;
                self.math_op(op, Dest::A, v, true);
            }
            _ => {}
        }
    }

    fn math_op(&mut self, op: MathOp, d: Dest, reg: u8, change_all: bool) {
        let mut a = self.get_dest(d);
        let carry_in = a;
        match op {
            MathOp::Inc => {
                a = a.wrapping_add(1);
                let carry = carry_in & !a;
                self.f.copy_syx_zero_z(a);
                self.f.overflow_hp(carry);
                self.f.reset_n();
            }
            MathOp::Dec => {
                a = a.wrapping_sub(1);
                let carry = !carry_in & a;
                self.f.copy_syx_zero_z(a);
                self.f.overflow_hp(carry);
                self.f.set_n();
            }
            MathOp::Adc | MathOp::Add => {
                if op == MathOp::Adc {
                    a = a.wrapping_add(u8::from(self.f.get_c()));
                }
                a = a.wrapping_add(reg);
                let carry = (carry_in & reg) | (!a & (reg | carry_in));
                if change_all {
                    self.f.copy_syx_zero_z(a);
                    self.f.overflow_hpc(carry);
                    self.f.reset_n();
                } else {
                    self.f.copy_yx(a);
                    self.f.overflow_hc(carry);
                    self.f.reset_n();
                }
            }
            MathOp::Sbc | MathOp::Sub => {
                if op == MathOp::Sbc {
                    a = a.wrapping_sub(u8::from(self.f.get_c()));
                }
                a = a.wrapping_sub(reg);
                let carry = (!carry_in & reg) | (a & (reg | !carry_in));
                self.f.copy_syx_zero_z(a);
                self.f.overflow_hpc(carry);
                self.f.set_n();
            }
            MathOp::And => {
                a &= reg;
                self.f.copy_syx_zero_z_parity_p(a);
                self.f.set_h();
                self.f.reset_n();
                self.f.reset_c();
            }
            MathOp::Xor => {
                a ^= reg;
                self.f.copy_syx_zero_z_parity_p(a);
                self.f.reset_hn();
                self.f.reset_c();
            }
            MathOp::Or => {
                a |= reg;
                self.f.copy_syx_zero_z_parity_p(a);
                self.f.reset_hn();
                self.f.reset_c();
            }
            MathOp::Cp => {
                let tmp = a.wrapping_sub(reg);
                let carry = (!carry_in & reg) | (tmp & (reg | !carry_in));
                if change_all {
                    self.f.copy_szero_z(tmp);
                    self.f.copy_yx(reg);
                    self.f.overflow_hpc(carry);
                    self.f.set_n();
                } else {
                    self.f.copy_szero_z(tmp);
                    self.f.copy_yx(reg);
                    self.f.overflow_hp(carry);
                    self.f.set_n();
                }
            }
            MathOp::Cpl => {
                a = !a;
                self.f.copy_yx(a);
                self.f.set_h();
                self.f.set_n();
            }
            MathOp::Neg => {
                a = (!a).wrapping_add(1);
                let carry = carry_in | a;
                self.f.copy_syx_zero_z(a);
                self.f.overflow_hpc(carry);
                self.f.set_n();
            }
            MathOp::Daa => {
                let mut tmp = u16::from(a);
                if !self.f.get_n() {
                    if (tmp & 0x000F) > 9 || self.f.get_h() {
                        if (tmp & 0x000F) > 9 {
                            self.f.set_h();
                        } else {
                            self.f.reset_h();
                        }
                        tmp += 0x0006;
                    }
                    if (tmp & 0xFFF0) > 0x0090 || self.f.get_c() {
                        tmp += 0x0060;
                    }
                } else {
                    if (tmp & 0x000F) > 9 || self.f.get_h() {
                        if (tmp & 0x000F) > 5 {
                            self.f.reset_h();
                        }
                        tmp = tmp.wrapping_sub(0x0006);
                        tmp &= 0x00FF;
                    }
                    if a > 0x99 || self.f.get_c() {
                        tmp = tmp.wrapping_sub(0x0160);
                    }
                }
                a = tmp as u8;
                self.f.copy_syx_zero_z_parity_p(a);
                if tmp & 0xFF00 != 0 {
                    self.f.set_c();
                }
            }
        }
        self.set_dest(d, a);
    }

    fn math_op16(&mut self, op: MathOp, rp1: u8, rp2: u8) {
        match self.sm_m {
            1 => self.no_bus_op(self.pc, 4),
            2 => {
                self.wz = self.pair(rp1);
                let rhs = self.pair_low(rp2);
                self.math_op(op, Dest::PairLow(rp1), rhs, false);
                self.no_bus_op(self.pc, 3);
            }
            3 => {
                let rhs = self.pair_high(rp2);
                if op != MathOp::Sbc {
                    self.math_op(MathOp::Adc, Dest::PairHigh(rp1), rhs, op != MathOp::Add);
                } else {
                    self.math_op(MathOp::Sbc, Dest::PairHigh(rp1), rhs, true);
                }
                if op != MathOp::Add {
                    let z = self.pair_low(rp1) == 0;
                    self.f.and_z(z);
                }
                self.wz = self.wz.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn shift_op(&mut self, op: ShiftOp, d: Dest, change_szp: bool) {
        let mut reg = self.get_dest(d);
        let carry = match op {
            ShiftOp::Rlc => {
                let c = reg & 0x80 != 0;
                reg = reg.rotate_left(1);
                c
            }
            ShiftOp::Rrc => {
                let c = reg & 0x01 != 0;
                reg = reg.rotate_right(1);
                c
            }
            ShiftOp::Rl => {
                let c = reg & 0x80 != 0;
                reg = (reg << 1) | u8::from(self.f.get_c());
                c
            }
            ShiftOp::Rr => {
                let c = reg & 0x01 != 0;
                reg = (reg >> 1) | (u8::from(self.f.get_c()) << 7);
                c
            }
            ShiftOp::Sla => {
                let c = reg & 0x80 != 0;
                reg <<= 1;
                c
            }
            ShiftOp::Sra => {
                let c = reg & 0x01 != 0;
                reg = (reg >> 1) | (reg & 0x80);
                c
            }
            ShiftOp::Sll => {
                let c = reg & 0x80 != 0;
                reg = (reg << 1) | 0x01;
                c
            }
            ShiftOp::Srl => {
                let c = reg & 0x01 != 0;
                reg >>= 1;
                c
            }
        };
        if change_szp {
            self.f.copy_syx_zero_z_parity_p(reg);
            self.f.reset_hn();
            self.f.store_c(carry);
        } else {
            self.f.copy_yx(reg);
            self.f.reset_hn();
            self.f.store_c(carry);
        }
        self.set_dest(d, reg);
    }

    fn rlrd(&mut self, left: bool) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(RP_HL);
                self.read_mem(self.wz);
            }
            2 => {
                self.rlrd_tmp = self.a;
                if left {
                    self.a = (self.a & 0xF0) | (self.s_di >> 4);
                } else {
                    self.a = (self.a & 0xF0) | (self.s_di & 0x0F);
                }
                let a = self.a;
                self.f.copy_syx_zero_z_parity_p(a);
                self.f.reset_hn();
                self.no_bus_op(self.pc, 4);
            }
            3 => {
                let v = if left {
                    (self.s_di << 4) | (self.rlrd_tmp & 0x0F)
                } else {
                    (self.s_di >> 4) | (self.rlrd_tmp << 4)
                };
                self.write_mem_bus(self.wz, v);
            }
            4 => self.wz = self.wz.wrapping_add(1),
            _ => {}
        }
    }

    fn prefix_cb(&mut self) {
        match self.sm_m {
            1 => {
                self.mc_prefix = PREFIX_CB;
                if self.xy != RP_HL {
                    self.mc_state = ST_XY_OFFSET;
                    let pc = self.pc;
                    self.pc = pc.wrapping_add(1);
                    self.read_mem(pc);
                    self.mc_m_cycles = 2;
                }
            }
            6 => {
                self.mc_prefix = PREFIX_CB;
                self.read_mem(self.pc);
                self.wz = self
                    .pair(self.xy)
                    .wrapping_add_signed(self.s_di as i8 as i16);
                self.mc_t_states = 5;
            }
            _ => {}
        }
    }

    fn inst_cb(&mut self, ireg: u8, d: Dest) {
        match self.sm_m {
            1 => {
                if self.mc_m_cycles == 1 {
                    self.inst_cb_op(ireg, d);
                    if ireg & 0xC0 == 0x40 {
                        let v = self.get_dest(d);
                        self.f.copy_yx(v);
                    }
                } else {
                    self.read_mem(self.pair(RP_HL));
                    self.mc_t_states = 4;
                }
            }
            2 => {
                self.inst_cb_op(ireg, Dest::Sdi);
                if ireg & 0xC0 == 0x40 {
                    self.f.copy_yx((self.wz >> 8) as u8);
                    self.finish_inst();
                } else {
                    self.mc_m_cycles = 3;
                    if self.xy != RP_HL {
                        self.write_mem_bus(self.wz, self.s_di);
                    } else {
                        self.write_mem_bus(self.pair(RP_HL), self.s_di);
                    }
                }
            }
            3 => {
                let v = self.s_do;
                self.set_dest(d, v);
            }
            7 => {
                self.read_mem(self.wz);
                self.mc_t_states = 4;
            }
            _ => {}
        }
    }

    fn inst_cb_op(&mut self, ireg: u8, d: Dest) {
        match ireg & 0xC0 {
            0x00 => {
                let op = match ireg & 0x38 {
                    0x00 => ShiftOp::Rlc,
                    0x08 => ShiftOp::Rrc,
                    0x10 => ShiftOp::Rl,
                    0x18 => ShiftOp::Rr,
                    0x20 => ShiftOp::Sla,
                    0x28 => ShiftOp::Sra,
                    0x30 => ShiftOp::Sll,
                    _ => ShiftOp::Srl,
                };
                self.shift_op(op, d, true);
            }
            0x40 => {
                let v = self.get_dest(d) & Self::mask(ireg);
                self.f.copy_szero_zp(v);
                self.f.set_h();
                self.f.reset_n();
            }
            0x80 => {
                let v = self.get_dest(d) & !Self::mask(ireg);
                self.set_dest(d, v);
            }
            0xC0 => {
                let v = self.get_dest(d) | Self::mask(ireg);
                self.set_dest(d, v);
            }
            _ => {}
        }
    }

    fn in_r_imm(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (u16::from(self.get_dest(d)) << 8) | u16::from(self.s_di);
                self.read_io(self.wz);
            }
            3 => {
                let v = self.s_di;
                self.set_dest(d, v);
                self.wz = self.wz.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn in_r_rr(&mut self, d: Dest, rp: u8) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(rp);
                self.read_io(self.wz);
            }
            2 => {
                let v = self.s_di;
                self.set_dest(d, v);
                self.f.copy_syx_zero_z_parity_p(v);
                self.f.reset_hn();
                self.wz = self.wz.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn out_imm_r(&mut self, d: Dest) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                let v = self.get_dest(d);
                self.wz = (u16::from(v) << 8) | u16::from(self.s_di);
                self.write_io(self.wz, v);
            }
            3 => {
                let lo = (self.wz as u8).wrapping_add(1);
                self.wz = (self.wz & 0xFF00) | u16::from(lo);
            }
            _ => {}
        }
    }

    fn out_rr_r(&mut self, rp: u8, d: Dest) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(rp);
                let v = self.get_dest(d);
                self.write_io(self.wz, v);
            }
            2 => self.wz = self.wz.wrapping_add(1),
            _ => {}
        }
    }

    fn out_rr_imm(&mut self, rp: u8, v: u8) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(rp);
                self.write_io(self.wz, v);
            }
            2 => self.wz = self.wz.wrapping_add(1),
            _ => {}
        }
    }

    fn jr(&mut self, cond: bool) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                if cond {
                    self.no_bus_op(self.pc, 5);
                } else {
                    self.finish_inst();
                }
            }
            3 => {
                self.wz = self.pc.wrapping_add_signed(self.s_di as i8 as i16);
                self.jump_wz();
            }
            _ => {}
        }
    }

    fn jp(&mut self, cond: bool) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                if cond {
                    self.jump_wz();
                }
            }
            _ => {}
        }
    }

    fn call(&mut self, cond: bool) {
        match self.sm_m {
            1 => {
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                let pc = self.pc;
                self.pc = pc.wrapping_add(1);
                self.read_mem(pc);
                if cond {
                    self.mc_t_states = 4;
                }
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                if cond {
                    self.push_stack8((self.pc >> 8) as u8);
                } else {
                    self.finish_inst();
                }
            }
            4 => self.push_stack8(self.pc as u8),
            5 => self.jump_wz(),
            _ => {}
        }
    }

    fn rst(&mut self, addr: u8) {
        match self.sm_m {
            1 => self.push_stack8((self.pc >> 8) as u8),
            2 => self.push_stack8(self.pc as u8),
            3 => {
                self.wz = u16::from(addr);
                self.jump_wz();
            }
            _ => {}
        }
    }

    fn ret(&mut self, cond: bool) {
        match self.sm_m {
            1 => {
                if cond {
                    self.pop_stack8();
                } else {
                    self.finish_inst();
                }
            }
            2 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                self.pop_stack8();
            }
            3 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                self.jump_wz();
            }
            _ => {}
        }
    }

    fn retn(&mut self) {
        self.ret(true);
        if self.sm_m == 3 {
            self.iff1 = self.iff2;
        }
    }

    fn int_im2(&mut self) {
        match self.sm_m {
            1 => {
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
                self.push_stack8((self.pc >> 8) as u8);
            }
            2 => self.push_stack8(self.pc as u8),
            3 => {
                let hi = if self.int_vector { 0xFF } else { self.i };
                self.wz = (self.wz & 0x00FF) | (u16::from(hi) << 8);
                self.read_mem(self.wz);
                self.wz = self.wz.wrapping_add(1);
            }
            4 => {
                self.read_mem(self.wz);
                self.wz = (self.wz & 0xFF00) | u16::from(self.s_di);
            }
            5 => {
                self.wz = (self.wz & 0x00FF) | (u16::from(self.s_di) << 8);
                self.jump_wz();
            }
            _ => {}
        }
    }

    fn ldxx(&mut self, typ: Block) {
        match self.sm_m {
            1 => {
                self.set_pair(RP_BC, self.pair(RP_BC).wrapping_sub(1));
                self.read_mem(self.pair(RP_HL));
            }
            2 => {
                match typ {
                    Block::Inc | Block::IncRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_add(1));
                    }
                    Block::Dec | Block::DecRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_sub(1));
                    }
                }
                self.write_mem_bus(self.pair(RP_DE), self.s_di);
                self.mc_t_states = 5;
            }
            3 => {
                match typ {
                    Block::Inc | Block::IncRep => {
                        self.set_pair(RP_DE, self.pair(RP_DE).wrapping_add(1));
                    }
                    Block::Dec | Block::DecRep => {
                        self.set_pair(RP_DE, self.pair(RP_DE).wrapping_sub(1));
                    }
                }
                let yx = self.s_di.wrapping_add(self.a);
                let p = self.pair(RP_BC) != 0;
                self.f.copy_bit1_yx(yx);
                self.f.reset_hn();
                self.f.store_p(p);
                if self.f.get_p() {
                    if matches!(typ, Block::IncRep | Block::DecRep) {
                        self.no_bus_op(self.pc, 5);
                    }
                } else {
                    self.finish_inst();
                }
            }
            4 => {
                self.pc = self.pc.wrapping_sub(2);
                self.f.copy_yx((self.pc >> 8) as u8);
                self.wz = self.pc.wrapping_add(1);
                self.s_ao = self.pc;
            }
            _ => {}
        }
    }

    fn cpxx(&mut self, typ: Block) {
        match self.sm_m {
            1 => {
                self.set_pair(RP_BC, self.pair(RP_BC).wrapping_sub(1));
                self.read_mem(self.pair(RP_HL));
            }
            2 => {
                let v = self.s_di;
                self.math_op(MathOp::Cp, Dest::A, v, false);
                match typ {
                    Block::Inc | Block::IncRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_add(1));
                        self.wz = self.wz.wrapping_add(1);
                    }
                    Block::Dec | Block::DecRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_sub(1));
                        self.wz = self.wz.wrapping_sub(1);
                    }
                }
                self.no_bus_op(self.pc, 5);
            }
            3 => {
                let ioq = self
                    .a
                    .wrapping_sub(self.s_di)
                    .wrapping_sub(u8::from(self.f.get_h()));
                let p = self.pair(RP_BC) != 0;
                self.f.copy_bit1_yx(ioq);
                self.f.store_p(p);
                if !self.f.get_z() && self.f.get_p() && matches!(typ, Block::IncRep | Block::DecRep)
                {
                    self.no_bus_op(self.pc, 5);
                } else {
                    self.finish_inst();
                }
            }
            4 => {
                self.pc = self.pc.wrapping_sub(2);
                self.f.copy_yx((self.pc >> 8) as u8);
                self.wz = self.pc.wrapping_add(1);
                self.s_ao = self.pc;
            }
            _ => {}
        }
    }

    fn inxx(&mut self, typ: Block) {
        match self.sm_m {
            1 => {
                self.wz = self.pair(RP_BC);
                self.read_io(self.wz);
                self.regs[R_B as usize] = self.regs[R_B as usize].wrapping_sub(1);
            }
            2 => self.write_mem_bus(self.pair(RP_HL), self.s_di),
            3 => {
                let ioq = match typ {
                    Block::Inc | Block::IncRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_add(1));
                        self.wz = self.wz.wrapping_add(1);
                        u16::from(self.s_di) + u16::from(self.regs[R_C as usize].wrapping_add(1))
                    }
                    Block::Dec | Block::DecRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_sub(1));
                        self.wz = self.wz.wrapping_sub(1);
                        u16::from(self.s_di) + u16::from(self.regs[R_C as usize].wrapping_sub(1))
                    }
                };
                let b = self.regs[R_B as usize];
                self.f.copy_syx_zero_z(b);
                self.f.store_hc(ioq >> 8 != 0);
                self.f.parity_p((ioq as u8 & 0x07) ^ b);
                self.f.copy_bit7_n(self.s_di);
                if !self.f.get_z() {
                    if matches!(typ, Block::IncRep | Block::DecRep) {
                        self.no_bus_op(self.pc, 5);
                    }
                } else {
                    self.finish_inst();
                }
            }
            4 => {
                self.pc = self.pc.wrapping_sub(2);
                let b = self.regs[R_B as usize];
                let di = self.s_di;
                self.f.copy_yx((self.pc >> 8) as u8);
                self.f.inotxx_ph(di, b);
                self.wz = self.pc.wrapping_add(1);
                self.s_ao = self.pc;
            }
            _ => {}
        }
    }

    fn otxx(&mut self, typ: Block) {
        match self.sm_m {
            1 => {
                self.read_mem(self.pair(RP_HL));
                self.regs[R_B as usize] = self.regs[R_B as usize].wrapping_sub(1);
                self.wz = self.pair(RP_BC);
            }
            2 => self.write_io(self.pair(RP_BC), self.s_di),
            3 => {
                match typ {
                    Block::Inc | Block::IncRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_add(1));
                        self.wz = self.wz.wrapping_add(1);
                    }
                    Block::Dec | Block::DecRep => {
                        self.set_pair(RP_HL, self.pair(RP_HL).wrapping_sub(1));
                        self.wz = self.wz.wrapping_sub(1);
                    }
                }
                let ioq = u16::from(self.s_di) + u16::from(self.regs[R_L as usize]);
                let b = self.regs[R_B as usize];
                self.f.copy_syx_zero_z(b);
                self.f.store_hc(ioq >> 8 != 0);
                self.f.parity_p((ioq as u8 & 0x07) ^ b);
                self.f.copy_bit7_n(self.s_di);
                if !self.f.get_z() {
                    if matches!(typ, Block::IncRep | Block::DecRep) {
                        self.no_bus_op(self.pc, 5);
                    }
                } else {
                    self.finish_inst();
                }
            }
            4 => {
                self.pc = self.pc.wrapping_sub(2);
                let b = self.regs[R_B as usize];
                let di = self.s_di;
                self.f.copy_yx((self.pc >> 8) as u8);
                self.f.inotxx_ph(di, b);
                self.wz = self.pc.wrapping_add(1);
                self.s_ao = self.pc;
            }
            _ => {}
        }
    }
}

fn alu_op(n: u8) -> MathOp {
    match n {
        0 => MathOp::Add,
        1 => MathOp::Adc,
        2 => MathOp::Sub,
        3 => MathOp::Sbc,
        4 => MathOp::And,
        5 => MathOp::Xor,
        6 => MathOp::Or,
        _ => MathOp::Cp,
    }
}

fn ld_dst(r: u8) -> Dest {
    match r {
        0 => Dest::Reg(R_B),
        1 => Dest::Reg(R_C),
        2 => Dest::Reg(R_D),
        3 => Dest::Reg(R_E),
        4 => Dest::Reg(R_H),
        5 => Dest::Reg(R_L),
        7 => Dest::A,
        _ => Dest::Discard,
    }
}

fn ld_src_real(r: u8) -> Dest {
    ld_dst(r)
}

fn mem(h: &CpuHost, addr: u16) -> u8 {
    h.flash(u32::from(addr)) as u8
}

fn write_mem(h: &mut CpuHost, addr: u16, data: u8) {
    let i = addr as usize;
    if i < h.prog.len() {
        h.prog[i] = u16::from(data);
    }
}
