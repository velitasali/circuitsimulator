//! MMIO register reads/writes, bit aliases, watch callbacks, and monitor queries.

use super::host::CpuHost;
use super::{Cpu, Device};
use crate::dataspace::{DataSpace, Watch};
use crate::desc::{ConfigSpec, CoreKind};

pub(crate) fn watch_config(data: &mut DataSpace, cfg: &ConfigSpec, a: Watch, b: Watch, c: Watch) {
    if !cfg.regs_a.is_empty() {
        for n in cfg.regs_a.split(',') {
            data.watch_reg_write(n.trim(), a);
        }
    }
    if !cfg.regs_b.is_empty() {
        for n in cfg.regs_b.split(',') {
            data.watch_reg_write(n.trim(), b);
        }
    }
    if !cfg.regs_c.is_empty() {
        for n in cfg.regs_c.split(',') {
            data.watch_reg_write(n.trim(), c);
        }
    }
    if !cfg.bits_a.is_empty() {
        data.watch_bits_write(&cfg.bits_a, a);
    }
    if !cfg.bits_b.is_empty() {
        data.watch_bits_write(&cfg.bits_b, b);
    }
    if !cfg.bits_c.is_empty() {
        data.watch_bits_write(&cfg.bits_c, c);
    }
}

impl CpuHost {
    pub(crate) fn read_reg(&mut self, addr: u16) -> u8 {
        let v = self.data.get(addr);
        let mut fired = [crate::dataspace::Watcher {
            watch: Watch::PortOut(0),
            mask: 0,
        }; 4];
        let n = {
            let ws = self.data.read_watchers(addr);
            if ws.is_empty() {
                return v;
            }
            let n = ws.len().min(4);
            fired[..n].copy_from_slice(&ws[..n]);
            n
        };
        self.data.reg_override = None;
        for w in &fired[..n] {
            let val = v & w.mask;
            self.apply_watch(w.watch, val, val, false);
        }
        self.data
            .reg_override
            .unwrap_or_else(|| self.data.get(addr))
    }

    pub(crate) fn write_reg(&mut self, addr: u16, v: u8, masked: bool) {
        let mut fired = [crate::dataspace::Watcher {
            watch: Watch::PortOut(0),
            mask: 0,
        }; 4];
        let n = {
            let ws = self.data.write_watchers(addr);
            if ws.is_empty() {
                self.data.write_reg_val(addr, v, masked);
                return;
            }
            let n = ws.len().min(4);
            fired[..n].copy_from_slice(&ws[..n]);
            n
        };
        let old = self.data.get(addr);
        self.data.reg_override = None;
        let (stored, _) = self.data.write_reg_val(addr, v, masked);
        for w in &fired[..n] {
            let val = stored & w.mask;
            match w.watch {
                Watch::PortOut(i) => {
                    self.ports_dirty = true;
                    if let Some(p) = self.ports.get_mut(i) {
                        p.out_changed(old, stored);
                    }
                }
                Watch::PortDir(i) => {
                    self.ports_dirty = true;
                    if let Some(p) = self.ports.get_mut(i) {
                        p.dir_changed(old, stored);
                    }
                }
                Watch::PortPinToggle(i) => {
                    if val == 0 {
                        continue;
                    }
                    self.ports_dirty = true;
                    let (out_addr, in_addr) = match self.ports.get(i) {
                        Some(p) => (p.out_addr, p.in_addr),
                        None => continue,
                    };
                    if let Some(out_addr) = out_addr {
                        let old_port = self.data.get(out_addr);
                        let new_port = old_port ^ val;
                        if let Some(p) = self.ports.get_mut(i) {
                            p.out_changed(old_port, new_port);
                        }
                        self.data.set(out_addr, new_port);
                    }
                    if let Some(in_addr) = in_addr {
                        self.data.set(in_addr, old);
                    }
                }
                Watch::CcpWriteH(i) if self.ccps.get(i).map(|c| c.is_pwm()).unwrap_or(false) => {
                    self.data.reg_override = Some(old);
                }
                _ => self.apply_watch(w.watch, old, val, true),
            }
        }
        if let Some(o) = self.data.reg_override.take() {
            self.data.set(addr, o);
        }
    }

    pub(crate) fn apply_watch(&mut self, w: Watch, old: u8, val: u8, write: bool) {
        match w {
            Watch::TimerEnable(_)
            | Watch::TimerCountL(_)
            | Watch::TimerCountH(_)
            | Watch::TimerCountRead(_)
            | Watch::TimerConfigA(_)
            | Watch::TimerConfigB(_)
            | Watch::TimerConfigC(_)
            | Watch::TimerTop0(_)
            | Watch::TimerIcrL(_)
            | Watch::OcWriteL { .. }
            | Watch::OcWriteH { .. }
            | Watch::OcConfig { .. }
            | Watch::IcConfig(_)
            | Watch::UsartSend(_)
            | Watch::UsartRead(_)
            | Watch::UsartConfigA(_)
            | Watch::UsartConfigB(_)
            | Watch::UsartConfigC(_)
            | Watch::UsartTxEnable(_)
            | Watch::UsartRxEnable(_)
            | Watch::UsartBaudL(_)
            | Watch::UsartBaudH(_)
            | Watch::AdcConfig
            | Watch::CcpConfig(_)
            | Watch::CcpWriteL(_)
            | Watch::CcpWriteH(_)
            | Watch::TwiConfigA(_)
            | Watch::TwiConfigB(_)
            | Watch::TwiStatus(_)
            | Watch::TwiDataWrite(_)
            | Watch::TwiAddr(_) => {
                self.periph_dirty = true;
            }
            _ => {}
        }
        match w {
            Watch::PortInRead(i) => {
                if self.data.is_cpu_read {
                    if let Some(p) = self.ports.get(i) {
                        self.data.reg_override = Some(p.get_inp_state() as u8);
                    }
                } else if let Some(p) = self.ports.get(i) {
                    if let Some(addr) = p.out_addr {
                        self.data.reg_override = Some(self.data.get(addr));
                    }
                }
            }
            Watch::SetBank => {
                let _ = val;
            }
            Watch::PortOut(_) | Watch::PortDir(_) | Watch::PortPinToggle(_) => {}
            Watch::EnableGlobalInt => self.interrupts.enable_global(val),
            Watch::IntEnable(i) => self.interrupts.enable_flag(i, val),
            Watch::IntFlagClear(i) => self.interrupts.flag_cleared(i),
            Watch::IntWriteFlag(i) => self.interrupts.write_flag(i, val, &mut self.data),
            Watch::IntPriority(i) => self.interrupts.set_priority(i, val),
            Watch::TimerEnable(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.enable(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::TimerCountL(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.count_write_l(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::TimerCountH(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.count_write_h(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::TimerCountRead(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.updt_count(&mut self.data);
                }
            }
            Watch::TimerConfigA(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.configure_a(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::TimerConfigB(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.configure_b(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::TimerConfigC(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.configure_c(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::TimerTop0(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.top_reg0_changed(val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::OcWriteL { timer, oc } => {
                if let Some(t) = self.timers.get_mut(timer) {
                    t.ocr_write_l(oc, val, &mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::OcWriteH { timer, oc } => {
                if let Some(t) = self.timers.get_mut(timer) {
                    t.ocr_write_h(oc, val);
                }
            }
            Watch::OcConfig { timer, oc } => {
                if let Some(t) = self.timers.get_mut(timer) {
                    t.oc_configure(oc, val, &mut self.ports);
                }
            }
            Watch::IcConfig(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.ic_configure(val);
                }
            }
            Watch::TimerIcrL(i) => {
                if let Some(t) = self.timers.get_mut(i) {
                    t.icr_changed(&mut self.data, &mut self.ports, self.ps_inst);
                }
            }
            Watch::PortIntMask(i) => {
                if let Some(p) = self.ports.get_mut(i) {
                    p.int_changed(val);
                }
            }
            Watch::PinExtInt { port, pin } => {
                let ext = self
                    .ports
                    .get_mut(port)
                    .and_then(|p| p.pins.get_mut(pin))
                    .map(|p| (p.conf_ext_int(val), p.ext_int));
                if let Some((cfg, idx)) = ext {
                    if let (Some((cont, auto)), Some(i)) = (cfg, idx) {
                        self.interrupts.set_continuous(i, cont, &mut self.data);
                        self.interrupts.set_auto_clear(i, auto);
                    }
                }
            }
            Watch::UsartSend(i) => {
                if write {
                    if let Some(u) = self.usarts.get_mut(i) {
                        u.send_byte(val, &mut self.data, &mut self.interrupts, &mut self.ports);
                    }
                }
            }
            Watch::UsartRead(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.read_byte(&mut self.data, &mut self.interrupts);
                }
            }
            Watch::UsartConfigA(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.configure_a(val, &mut self.ports, &mut self.data, self.ps_inst);
                }
            }
            Watch::UsartConfigB(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.configure_b(val, &mut self.ports, &mut self.data, self.ps_inst);
                }
            }
            Watch::UsartConfigC(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.configure_c(val, &mut self.data, self.ps_inst);
                }
            }
            Watch::UsartTxEnable(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.set_tx_enabled(val != 0, &mut self.ports);
                }
            }
            Watch::UsartRxEnable(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.set_rx_enabled(val != 0, &mut self.ports);
                }
            }
            Watch::UsartBaudL(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.set_baud_l(val, &mut self.data, self.ps_inst);
                }
            }
            Watch::UsartBaudH(i) => {
                if let Some(u) = self.usarts.get_mut(i) {
                    u.set_baud_h(val, &mut self.data, self.ps_inst);
                }
            }
            Watch::CcpConfig(i) => {
                if let Some(c) = self.ccps.get_mut(i) {
                    c.configure_a(val, &mut self.timers, &mut self.ports);
                }
            }
            Watch::CcpWriteL(i) => {
                if let Some(c) = self.ccps.get_mut(i) {
                    c.ccpr_write_l(
                        val,
                        &mut self.timers,
                        &mut self.data,
                        &mut self.ports,
                        self.ps_inst,
                    );
                }
            }
            Watch::CcpWriteH(i) => {
                if let Some(c) = self.ccps.get_mut(i) {
                    c.ccpr_write_h(
                        val,
                        &mut self.timers,
                        &mut self.data,
                        &mut self.ports,
                        self.ps_inst,
                    );
                }
            }
            Watch::TwiConfigA(i) => {
                if let Some(t) = self.twis.get_mut(i) {
                    t.configure_a(
                        old,
                        val,
                        &mut self.data,
                        &mut self.ports,
                        &mut self.interrupts,
                        self.freq_hz,
                    );
                }
            }
            Watch::TwiConfigB(i) => {
                if let Some(t) = self.twis.get_mut(i) {
                    t.configure_b(val, self.freq_hz);
                }
            }
            Watch::TwiStatus(i) => {
                if let Some(t) = self.twis.get_mut(i) {
                    t.write_status(val, self.freq_hz);
                }
            }
            Watch::TwiDataWrite(i) => {
                if write {
                    if let Some(t) = self.twis.get_mut(i) {
                        t.write_twi_reg(val, &mut self.data, &mut self.ports);
                    }
                }
            }
            Watch::TwiDataRead(i) => {
                let _ = i;
            }
            Watch::TwiAddr(i) => {
                if let Some(t) = self.twis.get_mut(i) {
                    t.write_addr_reg(val);
                }
            }
            Watch::AdcConfig => {
                if write {
                    complete_adc_conversion(&mut self.data, val);
                }
            }
        }
    }
}

/// Finish an AVR ADC conversion started by writing ADSC (C++ `McuAdc` / `AvrAdc`).
/// analogRead polls ADSC; completing in the write watcher unblocks it.
fn complete_adc_conversion(data: &mut crate::dataspace::DataSpace, val: u8) {
    let adsc = data.get_reg_bits("ADSC");
    if adsc.mask == 0 || val & adsc.mask == 0 {
        return;
    }
    let adif = data.get_reg_bits("ADIF");
    let mut v = val & !adsc.mask;
    if adif.mask != 0 {
        v |= adif.mask;
    }
    data.set(adsc.reg_addr, v);
}

impl Device {
    /// Write an SFR through watchers (tests / monitor poke of timer, INTCON, USART).
    pub fn poke_reg(&mut self, addr: u16, v: u8) {
        self.host.write_reg(addr, v, true);
    }

    /// Read an SFR through watchers (tests / CPU read of USART UDR, timer count, etc.).
    pub fn cpu_read_reg(&mut self, addr: u16) -> u8 {
        self.host.data.is_cpu_read = true;
        let v = self.host.read_reg(addr);
        self.host.data.is_cpu_read = false;
        v
    }

    pub fn cpu_read_reg_by_name(&mut self, name: &str) -> Option<u8> {
        let addr = self.host.data.reg_addr(name)?;
        Some(self.cpu_read_reg(addr))
    }

    pub fn ram(&self, addr: u16) -> u8 {
        self.host.data.get(addr)
    }

    pub fn ram_size(&self) -> u32 {
        self.host.data.ram_size
    }

    pub fn status_bits(&self) -> &[String] {
        &self.host.data.status_bits
    }

    /// C++ `cpu()->getStatus() != nullptr`.
    pub fn has_status(&self) -> bool {
        !self.host.data.status_bits.is_empty()
    }

    pub fn registers(&self) -> Vec<(String, u16)> {
        self.host.data.registers()
    }

    /// C++ `DataSpace::getRamValue` (mapped, not a CPU read).
    pub fn monitor_ram(&self, addr: u16) -> u8 {
        self.host.data.monitor_get(addr)
    }

    /// Whole data space as the MCU monitor hex dump sees it.
    pub fn ram_dump(&self) -> Vec<u8> {
        (0..self.ram_size() as u16)
            .map(|a| self.monitor_ram(a))
            .collect()
    }

    /// C++ `DataSpace::setRamValue` (mapped `writeReg`).
    pub fn set_monitor_ram(&mut self, addr: u16, v: u8) {
        let Some(mapped) = self.host.data.mapped_addr(addr) else {
            return;
        };
        let old = self.host.data.get(mapped);
        let fired = self.host.data.write_reg(mapped, v, true);
        let stored = self.host.data.get(mapped);
        for (w, val) in fired {
            match w {
                Watch::PortOut(i) => {
                    if let Some(p) = self.host.ports.get_mut(i) {
                        p.out_changed(old, stored);
                    }
                }
                Watch::PortDir(i) => {
                    if let Some(p) = self.host.ports.get_mut(i) {
                        p.dir_changed(old, stored);
                    }
                }
                Watch::PortPinToggle(i) => {
                    if val == 0 {
                        continue;
                    }
                    let (out_addr, in_addr) = match self.host.ports.get(i) {
                        Some(p) => (p.out_addr, p.in_addr),
                        None => continue,
                    };
                    if let Some(out_addr) = out_addr {
                        let old_port = self.host.data.get(out_addr);
                        let new_port = old_port ^ val;
                        if let Some(p) = self.host.ports.get_mut(i) {
                            p.out_changed(old_port, new_port);
                        }
                        self.host.data.set(out_addr, new_port);
                    }
                    if let Some(in_addr) = in_addr {
                        self.host.data.set(in_addr, old);
                    }
                }
                Watch::SetBank => {
                    if let Cpu::Pic14(c) = &mut self.cpu {
                        c.mr.set_bank(self.host.data.get(self.host.data.sreg_addr));
                    }
                }
                Watch::PortInRead(_) => {}
                _ => {}
            }
        }
    }

    pub fn status(&self) -> u8 {
        match &self.cpu {
            Cpu::Mcs65(c) => c.p,
            Cpu::Z80(c) => c.flags(),
            _ => self.host.data.get(self.host.data.sreg_addr),
        }
    }

    pub fn read_reg_by_name(&self, name: &str) -> Option<u32> {
        let upper = name.to_ascii_uppercase();
        if upper == "PC" {
            return Some(self.host.pc);
        }
        if upper == "SP" {
            if self.core == CoreKind::Z80 {
                return Some(u32::from(match &self.cpu {
                    Cpu::Z80(c) => c.get_sp(),
                    _ => 0,
                }));
            }
            if self.core == CoreKind::Mcs65 {
                return Some(u32::from(match &self.cpu {
                    Cpu::Mcs65(c) => c.sp,
                    _ => 0,
                }));
            }
            return Some(u32::from(self.host.get_sp()));
        }
        if upper == "W" {
            return Some(u32::from(self.w()));
        }
        if upper == "STATUS" || upper == "SREG" || upper == "PSW" || upper == "P" {
            return Some(u32::from(self.status()));
        }
        if self.core == CoreKind::Avr {
            if let Some(r_num) = upper.strip_prefix('R') {
                if let Ok(idx) = r_num.parse::<u16>() {
                    if idx < 32 {
                        return Some(u32::from(self.host.data.get(idx)));
                    }
                }
            }
        }
        if self.core == CoreKind::Mcs65 {
            match upper.as_str() {
                "A" | "ACC" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Mcs65(c) => c.ac,
                        _ => 0,
                    }));
                }
                "X" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Mcs65(c) => c.x,
                        _ => 0,
                    }));
                }
                "Y" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Mcs65(c) => c.y,
                        _ => 0,
                    }));
                }
                "S" | "STACK" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Mcs65(c) => c.sp,
                        _ => 0,
                    }));
                }
                _ => {}
            }
        }
        if self.core == CoreKind::Z80 {
            match upper.as_str() {
                "A" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.a,
                        _ => 0,
                    }));
                }
                "F" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.flags(),
                        _ => 0,
                    }));
                }
                "B" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg8(0),
                        _ => 0,
                    }));
                }
                "C" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg8(1),
                        _ => 0,
                    }));
                }
                "D" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg8(2),
                        _ => 0,
                    }));
                }
                "E" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg8(3),
                        _ => 0,
                    }));
                }
                "H" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg8(4),
                        _ => 0,
                    }));
                }
                "L" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg8(5),
                        _ => 0,
                    }));
                }
                "IX" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg16_pair(6),
                        _ => 0,
                    }));
                }
                "IY" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg16_pair(8),
                        _ => 0,
                    }));
                }
                "BC" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg16_pair(0),
                        _ => 0,
                    }));
                }
                "DE" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg16_pair(2),
                        _ => 0,
                    }));
                }
                "HL" => {
                    return Some(u32::from(match &self.cpu {
                        Cpu::Z80(c) => c.get_reg16_pair(4),
                        _ => 0,
                    }));
                }
                _ => {}
            }
        }
        if let Some(addr) = self.host.data.reg_addr(name) {
            return Some(u32::from(self.host.data.get(addr)));
        }
        None
    }

    pub fn write_reg_by_name(&mut self, name: &str, val: u32) -> bool {
        let upper = name.to_ascii_uppercase();
        if upper == "PC" {
            self.host.pc = val;
            return true;
        }
        if upper == "SP" {
            self.host.set_sp(val as u16);
            return true;
        }
        if self.core == CoreKind::Avr {
            if let Some(r_num) = upper.strip_prefix('R') {
                if let Ok(idx) = r_num.parse::<u16>() {
                    if idx < 32 {
                        self.host.data.set(idx, val as u8);
                        return true;
                    }
                }
            }
        }
        if let Some(addr) = self.host.data.reg_addr(name) {
            self.host.write_reg(addr, val as u8, true);
            return true;
        }
        false
    }
}
