//! Clock stepping, instruction execution, event scheduling, frequency timing, and interrupt handling.

use super::{Cpu, Device, McuState};
use crate::port;

/// Optional debug break/step filter evaluated inside [`Device::advance_batch`]
/// without locking any mutexes or resolving symbols per instruction.
#[derive(Clone, Copy, Debug)]
pub enum BatchDebugMode<'a> {
    /// Debugger inactive (maximum throughput).
    Off,
    /// Debugger running freely; stop if `pc` matches any address in `break_pcs` (sorted).
    Running { break_pcs: &'a [u32] },
    /// Debugger stepping into the next instruction; stop after 1 step.
    StepInto,
    /// Debugger stepping over; stop on `break_pcs`, `exit_pc`, `target_depth`, or stack depth return.
    StepOver {
        break_pcs: &'a [u32],
        target_sp: u32,
        target_depth: Option<u32>,
        exit_pc: Option<u32>,
    },
    /// Debugger stepping out; stop on `break_pcs`, `target_depth`, stack depth return, or `ret_addr`.
    StepOut {
        break_pcs: &'a [u32],
        target_sp: u32,
        target_depth: Option<u32>,
    },
}

/// Outcome of [`Device::advance_batch`].
#[derive(Clone, Copy, Debug)]
pub struct BatchStop {
    /// Total picoseconds elapsed across intermediate steps inside the batch.
    pub advanced_ps: u64,
    /// Set when a debug breakpoint or step condition matched: `(prev_pc, pc, sp, ret_addr)`.
    pub debug_hit: Option<(u32, u32, u16, u32)>,
    /// Call depth at the end of the batch (or when `debug_hit` triggered).
    pub debug_call_depth: u32,
}

#[inline(always)]
fn matches_break_pc(break_pcs: &[u32], pc: u32) -> bool {
    if break_pcs.is_empty() {
        return false;
    }
    if break_pcs.len() <= 16 {
        for &bp in break_pcs {
            if bp == pc {
                return true;
            }
        }
        false
    } else {
        break_pcs.binary_search(&pc).is_ok()
    }
}

impl Device {
    pub fn force_freq(&mut self, mut freq: f64) {
        self.host.flush_peripherals();
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
        for s in &mut self.host.spis {
            s.update_timing(self.ps_inst);
        }
        self.host.periph_dirty = true;
    }

    /// One CPU instruction (C++ `eMcu::stepCpu`). Tests call this directly.
    #[inline]
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

    /// Advance the MCU by any pending rising edges sampled on the primary external
    /// oscillator pin (XTAL1). Returns `true` if at least one CPU tick executed.
    pub fn step_ext_clock(&mut self) -> bool {
        let edges = std::mem::take(&mut self.host.ext_clk_pending_edges);
        if edges == 0 || !self.host.ext_osc_enabled || self.host.shared_reset_held {
            return false;
        }
        if !matches!(self.state, McuState::Running | McuState::Sleeping) {
            return false;
        }
        let ticks_per_cpu = self.c_per_tick.round().max(1.0) as u32;
        let tick_ps = self.ps_tick.max(1);
        let mut stepped = false;
        for _ in 0..edges {
            self.host.ext_osc_div += 1;
            if self.host.ext_osc_div < ticks_per_cpu {
                continue;
            }
            self.host.ext_osc_div = 0;
            stepped = true;

            self.host.periph_elapsed = self.host.periph_elapsed.saturating_add(tick_ps);
            if self.host.periph_dirty {
                self.host.recompute_periph_next();
            }
            if self.host.periph_elapsed >= self.host.periph_next {
                self.host.flush_peripherals();
                if self.host.wdt_reset_pending {
                    self.host.wdt_reset_pending = false;
                    self.reset();
                    self.start();
                    break;
                }
            }

            if self.state == McuState::Sleeping {
                self.dispatch_interrupts();
                continue;
            }

            if self.host.ext_cpu_cycles_wait > 1 {
                self.host.ext_cpu_cycles_wait -= 1;
            } else {
                self.step_cpu();
                self.host.ext_cpu_cycles_wait = self.cycles_done();
            }
        }
        stepped
    }

    #[inline(always)]
    fn dispatch_interrupts(&mut self) {
        let vector = self.host.interrupts.run(&mut self.host.data);
        if let Some(vec) = vector {
            self.take_interrupt(u32::from(vec));
        }
    }

    fn take_interrupt(&mut self, vector: u32) {
        self.is_spinning = false;
        if self.state == McuState::Sleeping {
            self.state = McuState::Running;
        }
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
    #[inline]
    pub fn advance(&mut self) {
        let elapsed = self.sched;
        self.cpu_remain = self.cpu_remain.saturating_sub(elapsed);
        if let Some(b) = self.bus_remain.as_mut() {
            *b = b.saturating_sub(elapsed);
        }
        self.host.periph_elapsed = self.host.periph_elapsed.saturating_add(elapsed);
        if self.host.periph_dirty {
            self.host.recompute_periph_next();
        }
        if self.host.periph_elapsed >= self.host.periph_next {
            self.host.flush_peripherals();
            if self.host.wdt_reset_pending {
                self.host.wdt_reset_pending = false;
                self.reset();
                self.start();
                self.sched = self.next_remain().max(1);
                return;
            }
        }
        if self.state == McuState::Sleeping {
            self.dispatch_interrupts();
            self.sched = self.next_remain().max(1);
            return;
        }
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

    /// Execute up to `max_steps` MCU steps inside `cs-mcu` without crossing the
    /// crate boundary per instruction, stopping early on GPIO changes, external
    /// deadlines (`max_ps`), idle/sleep skips, or a debug breakpoint/step hit.
    pub fn advance_batch(
        &mut self,
        max_ps: u64,
        max_steps: usize,
        debug: BatchDebugMode<'_>,
    ) -> BatchStop {
        let mut advanced_ps = 0u64;
        let mut steps = 0usize;
        let max_step_ps = self.ps_tick.saturating_mul(4);
        loop {
            let prev_pc = self.host.pc;
            if matches!(self.state, McuState::Running | McuState::Sleeping) {
                self.advance();
            }
            match debug {
                BatchDebugMode::Off => {}
                BatchDebugMode::Running { break_pcs } => {
                    let pc = self.host.pc;
                    if matches_break_pc(break_pcs, pc) {
                        return BatchStop {
                            advanced_ps,
                            debug_hit: Some((prev_pc, pc, self.host.get_sp(), self.host.ret_addr)),
                            debug_call_depth: self.host.call_depth,
                        };
                    }
                }
                BatchDebugMode::StepInto => {
                    return BatchStop {
                        advanced_ps,
                        debug_hit: Some((
                            prev_pc,
                            self.host.pc,
                            self.host.get_sp(),
                            self.host.ret_addr,
                        )),
                        debug_call_depth: self.host.call_depth,
                    };
                }
                BatchDebugMode::StepOver {
                    break_pcs,
                    target_sp,
                    target_depth,
                    exit_pc,
                } => {
                    let pc = self.host.pc;
                    let sp = self.host.get_sp();
                    let hit_step = if let Some(exit) = exit_pc {
                        pc == exit
                    } else if let Some(td) = target_depth {
                        self.host.call_depth <= td && pc != prev_pc
                    } else {
                        (target_sp == 0 || u32::from(sp) >= target_sp) && pc != prev_pc
                    };
                    if hit_step || matches_break_pc(break_pcs, pc) {
                        return BatchStop {
                            advanced_ps,
                            debug_hit: Some((prev_pc, pc, sp, self.host.ret_addr)),
                            debug_call_depth: self.host.call_depth,
                        };
                    }
                }
                BatchDebugMode::StepOut {
                    break_pcs,
                    target_sp,
                    target_depth,
                } => {
                    let pc = self.host.pc;
                    let sp = self.host.get_sp();
                    let ret_addr = self.host.ret_addr;
                    let hit_step = if let Some(td) = target_depth {
                        self.host.call_depth <= td && pc != prev_pc
                    } else {
                        (target_sp != 0 && u32::from(sp) >= target_sp)
                            || (ret_addr != 0 && pc == ret_addr)
                    };
                    if hit_step || matches_break_pc(break_pcs, pc) {
                        return BatchStop {
                            advanced_ps,
                            debug_hit: Some((prev_pc, pc, sp, ret_addr)),
                            debug_call_depth: self.host.call_depth,
                        };
                    }
                }
            }
            if !matches!(self.state, McuState::Running | McuState::Sleeping)
                || self.host.ports_dirty
            {
                break;
            }
            let delta = self.next_event_ps();
            if delta > max_step_ps
                || advanced_ps.saturating_add(delta) >= max_ps
                || steps >= max_steps
            {
                break;
            }
            advanced_ps = advanced_ps.saturating_add(delta);
            steps += 1;
        }
        BatchStop {
            advanced_ps,
            debug_hit: None,
            debug_call_depth: self.host.call_depth,
        }
    }

    #[inline(always)]
    fn next_remain(&mut self) -> u64 {
        let periph = self.host.periph_remain();
        let cpu = if self.is_spinning || self.state == McuState::Sleeping {
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

    #[inline(always)]
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

    #[inline]
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
                let sleep_allowed = host.sleep_se.mask == 0
                    || (host.data.get(host.sleep_se.reg_addr) & host.sleep_se.mask) != 0;
                if sleep_allowed {
                    *state = McuState::Sleeping;
                }
            }
            if host.wdr {
                for w in &mut host.wdts {
                    w.kick();
                }
                if !host.wdts.is_empty() {
                    host.periph_dirty = true;
                }
            }
            if host.reti {
                host.interrupts.ret_i();
            }
            if let Some(en) = host.enable_int {
                host.interrupts.enable_global(en);
            }
            host.cycles_done = host.cycles_done.max(1);
            for r in &mut host.roms {
                r.step_cycles(host.cycles_done, &mut host.data);
            }
            for w in &mut host.wdts {
                w.step_cycles(host.cycles_done, &mut host.data);
            }
            if host.clk_out_enabled {
                if let Some(pin) = host.intosc_clkout_pin.clone() {
                    let prev = port::gpio_out(&host.ports, &pin).unwrap_or(false);
                    port::set_gpio_out(&mut host.ports, &pin, !prev);
                    host.ports_dirty = true;
                }
            }
            let is_single_step = matches!(cpu, Cpu::Pic12(_) | Cpu::Pic14(_) | Cpu::Avr(_));
            is_single_step && host.pc == prev_pc && !host.sleep
        };
        if let Some(f) = self.host.pending_freq_hz.take() {
            self.force_freq(f);
        }
        self.is_spinning = spinning;
    }

    #[inline(always)]
    pub fn cycles_done(&self) -> u32 {
        self.host.cycles_done.max(1)
    }

    /// Next event delay in picoseconds (C++ `cyclesDone * m_psTick`, or an I51 bus slot).
    #[inline(always)]
    pub fn next_event_ps(&self) -> u64 {
        if self.sched > 0 {
            self.sched
        } else {
            u64::from(self.cycles_done()) * self.ps_tick.max(1)
        }
    }

    pub fn timer_remain_ps(&self) -> Option<u64> {
        self.host
            .timers
            .first()
            .and_then(|t| t.remain_ps)
            .map(|r| r.saturating_sub(self.host.periph_elapsed))
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

    pub fn call_depth(&self) -> u32 {
        self.host.call_depth
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
