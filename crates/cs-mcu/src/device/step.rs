//! Clock stepping, instruction execution, event scheduling, frequency timing, and interrupt handling.

use super::{Cpu, Device, McuState};
use crate::interrupts::IntCallback;

impl Device {
    pub fn force_freq(&mut self, mut freq: f64) {
        if freq < 0.0 {
            freq = 0.0;
        } else if freq > 100.0 * 1e6 {
            freq = 100.0 * 1e6;
        }
        if freq > 0.0 {
            self.ps_inst = (1e12 * (self.c_per_inst / freq)).round() as u64;
            self.ps_tick = (1e12 * (self.c_per_tick / freq)).round() as u64;
        }
        self.freq = freq;
        self.host.freq_hz = freq;
        self.host.ps_inst = self.ps_inst;
        for t in &mut self.host.timers {
            t.schedule(&mut self.host.ports, self.ps_inst);
        }
        for twi in &mut self.host.twis {
            twi.update_freq(self.freq);
        }
    }

    /// One CPU instruction (C++ `eMcu::stepCpu`). Tests call this directly.
    pub fn step_cpu(&mut self) {
        if self.host.prog.is_empty() || (self.host.pc as usize) < self.host.prog.len() {
            if self.state == McuState::Running {
                self.run_step();
            }
            self.dispatch_interrupts();
        } else {
            self.state = McuState::Error;
        }
        self.cycle += u64::from(self.host.cycles_done);
        self.sync_bus_remain();
    }

    fn dispatch_interrupts(&mut self) {
        let vector = self.host.interrupts.run(&mut self.host.data);
        if let Some(vec) = vector {
            self.take_interrupt(u32::from(vec));
        }
    }

    fn take_interrupt(&mut self, vector: u32) {
        self.is_spinning = false;
        let Device { cpu, host, .. } = self;
        match cpu {
            Cpu::None => {}
            Cpu::Pic12(c) => c.interrupt(host, vector),
            Cpu::Pic14(c) => c.interrupt(host, vector),
            Cpu::Avr(_) => {
                host.push_stack(host.pc);
                host.pc = vector;
                host.cycles_done = u32::from(host.ret_cycles);
            }
            Cpu::I51(c) => c.interrupt(host, vector),
            Cpu::Mcs65(_) | Cpu::Z80(_) => {
                host.push_stack(host.pc);
                host.pc = vector;
                host.cycles_done = u32::from(host.ret_cycles);
            }
        }
    }

    /// C++ `eMcu::runEvent` interleaved with timer / USART / `I51Core::runEvent`.
    pub fn advance(&mut self) {
        let elapsed = self.sched;
        self.cpu_remain = self.cpu_remain.saturating_sub(elapsed);
        if let Some(b) = self.bus_remain.as_mut() {
            *b = b.saturating_sub(elapsed);
        }
        self.advance_peripherals(elapsed);
        let bus_due = self.bus_remain == Some(0);
        if bus_due {
            self.run_bus();
        } else if self.cpu_remain == 0 {
            if self.is_spinning {
                let skipped_cycles = elapsed / self.ps_tick.max(1);
                if skipped_cycles > 0 {
                    self.cycle = self.cycle.saturating_add(skipped_cycles);
                }
            }
            self.step_cpu();
            self.cpu_remain = u64::from(self.cycles_done()) * self.ps_tick.max(1);
        }
        self.sync_bus_remain();
        self.sched = self.next_remain().max(1);
    }

    fn advance_peripherals(&mut self, elapsed: u64) {
        self.cbs_buf.clear();
        for t in &mut self.host.timers {
            if t.running {
                t.advance(
                    elapsed,
                    &mut self.host.data,
                    &mut self.host.interrupts,
                    &mut self.host.ports,
                    &mut self.cbs_buf,
                );
            }
        }
        for u in &mut self.host.usarts {
            if u.tx_remain.is_some() {
                u.advance_tx(
                    elapsed,
                    &mut self.host.data,
                    &mut self.host.interrupts,
                    &mut self.host.ports,
                );
            }
            if u.rx_remain.is_some() {
                u.advance_rx(
                    elapsed,
                    &mut self.host.data,
                    &mut self.host.interrupts,
                    &self.host.ports,
                );
            }
        }
        for twi in &mut self.host.twis {
            if twi.remain_ps.is_some()
                && twi.step_time(
                    elapsed,
                    &mut self.host.ports,
                    &mut self.host.data,
                    &mut self.host.interrupts,
                )
            {
                self.host.ports_dirty = true;
            }
        }
        if !self.cbs_buf.is_empty() {
            self.apply_int_callbacks();
        }
    }

    fn apply_int_callbacks(&mut self) {
        for idx in 0..self.cbs_buf.len() {
            match self.cbs_buf[idx] {
                IntCallback::UsartTick(i) => {
                    if let Some(u) = self.host.usarts.get_mut(i) {
                        u.timer_tick(
                            &mut self.host.data,
                            &mut self.host.interrupts,
                            &mut self.host.ports,
                        );
                    }
                }
            }
        }
    }

    fn compute_peripheral_remain(&self) -> u64 {
        let mut n = u64::MAX;
        for t in &self.host.timers {
            if t.running {
                if let Some(r) = t.next_remain() {
                    n = n.min(r.max(1));
                }
            }
        }
        for u in &self.host.usarts {
            if let Some(r) = u.tx_remain {
                n = n.min(r.max(1));
            }
            if let Some(r) = u.rx_remain {
                n = n.min(r.max(1));
            }
        }
        for twi in &self.host.twis {
            if let Some(r) = twi.remain_ps {
                n = n.min(r.max(1));
            }
        }
        n
    }

    fn next_remain(&mut self) -> u64 {
        let periph = self.compute_peripheral_remain();
        let cpu = if self.is_spinning {
            let horizon = periph.min(self.bus_remain.unwrap_or(u64::MAX));
            if horizon < u64::MAX {
                horizon.max(self.ps_tick.max(1))
            } else {
                1_000_000_000 // 1 ms in picoseconds when idle
            }
        } else {
            self.cpu_remain.max(1)
        };
        if self.bus_remain.is_none() && cpu <= periph {
            return cpu;
        }
        let mut n = cpu;
        if let Some(b) = self.bus_remain {
            n = if b == 0 { 1 } else { n.min(b) };
        }
        n.min(periph)
    }

    fn sync_bus_remain(&mut self) {
        self.bus_remain = match &self.cpu {
            Cpu::I51(c) => c.pending_bus_ps,
            Cpu::Mcs65(c) => c.pending_bus_ps,
            Cpu::Z80(c) => c.pending_bus_ps,
            _ => None,
        };
    }

    fn run_bus(&mut self) {
        let Device { cpu, host, .. } = self;
        match cpu {
            Cpu::I51(c) => c.run_mem(host),
            Cpu::Mcs65(c) => c.run_bus(host),
            Cpu::Z80(c) => c.run_bus(host),
            _ => {}
        }
    }

    fn run_step(&mut self) {
        let prev_pc = self.host.pc;
        self.host.cycles_done = 0;
        self.host.sleep = false;
        self.host.wdr = false;
        self.host.reti = false;
        self.host.enable_int = None;
        let spinning = {
            let Device {
                cpu,
                host,
                bank_from_status,
                state,
                ..
            } = self;
            match cpu {
                Cpu::None => host.cycles_done = 1,
                Cpu::Pic12(c) => c.run_step(host),
                Cpu::Pic14(c) => c.run_step(host),
                Cpu::Avr(c) => c.run_step(host),
                Cpu::I51(c) => c.run_step(host),
                Cpu::Mcs65(c) => c.run_step(host),
                Cpu::Z80(c) => c.run_step(host),
            }
            match cpu {
                Cpu::Pic12(c) => {
                    if let Some(w) = c.take_option() {
                        if let Some(addr) = c.mr.option_addr {
                            host.write_reg(addr, w, true);
                        }
                    }
                    if let Some(w) = c.take_tris() {
                        if let Some(addr) = c.mr.tris_addr {
                            host.write_reg(addr, w, true);
                        }
                    }
                }
                Cpu::Pic14(c) => {
                    if let Some(w) = c.take_option() {
                        if let Some(addr) = c.mr.option_addr {
                            host.write_reg(addr, w, true);
                        }
                    }
                    if *bank_from_status {
                        let sreg = host.data.sreg_addr;
                        c.mr.set_bank(host.data.get(sreg));
                    }
                }
                Cpu::Avr(_) | Cpu::I51(_) | Cpu::Mcs65(_) | Cpu::Z80(_) | Cpu::None => {}
            }
            if host.sleep {
                *state = McuState::Sleeping;
            }
            if host.reti {
                host.interrupts.ret_i();
            }
            if let Some(en) = host.enable_int {
                host.interrupts.enable_global(en);
            }
            host.cycles_done = host.cycles_done.max(1);
            let is_single_step = matches!(cpu, Cpu::Pic12(_) | Cpu::Pic14(_) | Cpu::Avr(_));
            is_single_step && host.pc == prev_pc && !host.sleep
        };
        self.is_spinning = spinning;
    }

    pub fn cycles_done(&self) -> u32 {
        self.host.cycles_done.max(1)
    }

    /// Next event delay in picoseconds (C++ `cyclesDone * m_psTick`, or an I51 bus slot).
    pub fn next_event_ps(&self) -> u64 {
        if self.sched > 0 {
            self.sched
        } else {
            u64::from(self.cycles_done()) * self.ps_tick.max(1)
        }
    }

    pub fn timer_remain_ps(&self) -> Option<u64> {
        self.host.timers.first().and_then(|t| t.remain_ps)
    }

    pub fn irq_global(&self) -> u8 {
        self.host.interrupts.enabled
    }

    pub fn pc(&self) -> u32 {
        self.host.pc
    }

    pub fn w(&self) -> u8 {
        match &self.cpu {
            Cpu::None => 0,
            Cpu::Pic12(c) => c.mr.w,
            Cpu::Pic14(c) => c.mr.w,
            Cpu::Avr(_) => self.host.data.get(0),
            Cpu::I51(c) => self.host.data.get(c.acc_addr),
            Cpu::Mcs65(c) => c.ac,
            Cpu::Z80(c) => c.a,
        }
    }

    pub fn ret_addr(&self) -> u32 {
        self.host.ret_addr
    }

    pub fn sp(&self) -> u16 {
        self.host.get_sp()
    }

    pub fn set_sp(&mut self, sp: u16) {
        self.host.set_sp(sp);
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.host.pc = pc;
    }

    pub fn xram(&self) -> &[u8] {
        match &self.cpu {
            Cpu::I51(c) => &c.xram,
            _ => &[],
        }
    }
}
