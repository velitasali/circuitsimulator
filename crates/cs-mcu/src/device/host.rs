//! CPU host view: data space, flash, program counter, and stack operations.

use crate::ccp::CcpUnit;
use crate::dataspace::DataSpace;
use crate::interrupts::Interrupts;
use crate::port::Port;
use crate::timer::Timer;
use crate::twi::Twi;
use crate::usart::Usart;

/// Mutable CPU view: data space, flash, PC, GPIO. Watchers run here.
#[derive(Clone, Debug)]
pub struct CpuHost {
    pub data: DataSpace,
    pub ports: Vec<Port>,
    pub prog: Vec<u16>,
    pub pc: u32,
    pub cycles_done: u32,
    pub ret_addr: u32,
    pub ret_cycles: u8,
    pub sleep: bool,
    pub wdr: bool,
    pub reti: bool,
    pub spl_addr: Option<u16>,
    pub sph_addr: Option<u16>,
    pub sp_pre: bool,
    pub sp_inc: i16,
    pub prog_addr_size: u8,
    pub enable_int: Option<u8>,
    pub low_data_end: u16,
    pub reg_end: u16,
    pub upper_data: bool,
    pub interrupts: Interrupts,
    pub timers: Vec<Timer>,
    pub ccps: Vec<CcpUnit>,
    pub usarts: Vec<Usart>,
    pub twis: Vec<Twi>,
    pub freq_hz: f64,
    pub ps_inst: u64,
    pub ports_dirty: bool,
    pub periph_dirty: bool,
}

impl CpuHost {
    pub fn flash(&self, pc: u32) -> u16 {
        self.prog.get(pc as usize).copied().unwrap_or(0xFFFF)
    }

    pub fn check_addr(&self, addr: u16) -> u16 {
        if self.upper_data && addr > self.low_data_end {
            addr.wrapping_add(self.reg_end)
        } else {
            addr
        }
    }

    pub fn get_ram(&mut self, addr: u16) -> u8 {
        if addr >= self.data.reg_start && addr <= self.data.reg_end {
            self.read_reg(addr)
        } else if (addr as u32) < self.data.ram_size {
            self.data.get(addr)
        } else {
            0
        }
    }

    pub fn set_ram(&mut self, addr: u16, v: u8) {
        if addr >= self.data.reg_start && addr <= self.data.reg_end {
            self.write_reg(addr, v, true);
        } else if (addr as u32) < self.data.ram_size {
            self.data.set(addr, v);
        }
    }

    pub fn gpr(&self, n: u8) -> u8 {
        self.data.get(u16::from(n))
    }

    pub fn set_gpr(&mut self, n: u8, v: u8) {
        self.data.set(u16::from(n), v);
    }

    pub fn get_reg16_lh(&self, addr: u16) -> u16 {
        u16::from(self.data.get(addr)) | (u16::from(self.data.get(addr.wrapping_add(1))) << 8)
    }

    pub fn set_reg16_lh(&mut self, addr: u16, val: u16) {
        self.write_reg(addr, val as u8, true);
        self.write_reg(addr.wrapping_add(1), (val >> 8) as u8, true);
    }

    pub fn set_reg16_hl(&mut self, addr: u16, val: u16) {
        self.write_reg(addr.wrapping_add(1), (val >> 8) as u8, true);
        self.write_reg(addr, val as u8, true);
    }

    pub fn get_sp(&self) -> u16 {
        let mut sp = self
            .spl_addr
            .map(|a| u16::from(self.data.get(a)))
            .unwrap_or(0);
        if let Some(a) = self.sph_addr {
            sp |= u16::from(self.data.get(a)) << 8;
        }
        sp
    }

    pub fn set_sp(&mut self, sp: u16) {
        if let Some(a) = self.spl_addr {
            self.data.set(a, sp as u8);
        }
        if let Some(a) = self.sph_addr {
            self.data.set(a, (sp >> 8) as u8);
        }
    }

    pub fn push_stack(&mut self, mut addr: u32) {
        if self.spl_addr.is_none() {
            return;
        }
        let mut sp = self.get_sp();
        if self.sp_pre {
            sp = sp.wrapping_add_signed(self.sp_inc);
        }
        for _ in 0..self.prog_addr_size {
            self.set_ram(sp, addr as u8);
            addr >>= 8;
            sp = sp.wrapping_add_signed(self.sp_inc);
        }
        if self.sp_pre {
            sp = sp.wrapping_add_signed(-self.sp_inc);
        }
        self.set_sp(sp);
    }

    pub fn pop_stack(&mut self) -> u32 {
        if self.spl_addr.is_none() {
            return 0;
        }
        let mut sp = self.get_sp();
        let mut res = 0u32;
        if !self.sp_pre {
            sp = sp.wrapping_add_signed(-self.sp_inc);
        }
        for _ in 0..self.prog_addr_size {
            res = (res << 8) | u32::from(self.get_ram(sp));
            sp = sp.wrapping_add_signed(-self.sp_inc);
        }
        if !self.sp_pre {
            sp = sp.wrapping_add_signed(self.sp_inc);
        }
        self.set_sp(sp);
        res
    }

    pub fn push_stack8(&mut self, v: u8) {
        if self.spl_addr.is_none() {
            return;
        }
        let mut sp = self.get_sp();
        if self.sp_pre {
            sp = sp.wrapping_add_signed(self.sp_inc);
        }
        self.set_ram(sp, v);
        if !self.sp_pre {
            sp = sp.wrapping_add_signed(self.sp_inc);
        }
        self.set_sp(sp);
    }

    pub fn pop_stack8(&mut self) -> u8 {
        if self.spl_addr.is_none() {
            return 0;
        }
        let mut sp = self.get_sp();
        if !self.sp_pre {
            sp = sp.wrapping_add_signed(-self.sp_inc);
        }
        let res = self.get_ram(sp);
        if self.sp_pre {
            sp = sp.wrapping_add_signed(-self.sp_inc);
        }
        self.set_sp(sp);
        res
    }
}
