//! C++ `eMcu`: flash, data space, GPIO ports, CPU step, frequency.

mod host;
mod ports;
mod registers;
mod step;
#[cfg(test)]
mod tests;

pub use host::CpuHost;

use crate::ccp::CcpUnit;
use crate::cpu::{Avr, I51, Mcs65, Pic12, Pic14, Z80};
use crate::dataspace::Watch;
use crate::desc::{CoreKind, McuDesc};
use crate::hex::load_hex;
use crate::interrupts::{IntCallback, Interrupts};
use crate::port::Port;
use crate::timer::Timer;
use crate::twi::Twi;
use crate::usart::Usart;
use crate::{Error, Result};
use registers::watch_config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McuState {
    Stopped,
    Error,
    Running,
    Sleeping,
}

#[derive(Clone, Debug)]
pub(crate) enum Cpu {
    None,
    Pic12(Pic12),
    Pic14(Pic14),
    Avr(Avr),
    I51(I51),
    Mcs65(Mcs65),
    Z80(Z80),
}

#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub core: CoreKind,
    pub(crate) cpu: Cpu,
    pub(crate) host: CpuHost,
    pub state: McuState,
    pub cycle: u64,
    pub freq: f64,
    pub c_per_inst: f64,
    pub c_per_tick: f64,
    pub ps_inst: u64,
    pub ps_tick: u64,
    pub vdd: f64,
    pub word_size: u8,
    pub(crate) bank_from_status: bool,
    pub(crate) eeprom: Vec<u8>,
    /// SPI module names from the `.mcu` description (C++ `m_transModules` SPI entries).
    pub(crate) spi_module_names: Vec<String>,
    /// Remaining ps until the next `runStep` (C++ `eMcu` event).
    pub(crate) cpu_remain: u64,
    /// Remaining ps until the next core bus event (I51 / 6502 / Z80 eElement).
    pub(crate) bus_remain: Option<u64>,
    /// Delay last advertised to the circuit event queue.
    pub(crate) sched: u64,
    pub(crate) cbs_buf: Vec<IntCallback>,
    pub is_spinning: bool,
}

/// Convenience alias for Device.
pub type Mcu = Device;

impl Device {
    pub fn usarts(&self) -> &[Usart] {
        &self.host.usarts
    }

    pub fn usart_names(&self) -> Vec<String> {
        self.host.usarts.iter().map(|u| u.name.clone()).collect()
    }

    pub fn twi_names(&self) -> Vec<String> {
        self.host.twis.iter().map(|t| t.name.clone()).collect()
    }

    pub fn spi_names(&self) -> Vec<String> {
        self.spi_module_names.clone()
    }

    /// C++ `eMcu::m_transModules` names, in USART / TWI / SPI order.
    pub fn trans_module_names(&self) -> Vec<String> {
        let mut names = self.usart_names();
        names.extend(self.twi_names());
        names.extend(self.spi_module_names.iter().cloned());
        names
    }

    #[inline(always)]
    pub fn has_periph_logs(&self) -> bool {
        self.host
            .usarts
            .iter()
            .any(|u| !u.tx_log.is_empty() || !u.rx_pin_log.is_empty())
            || self
                .host
                .twis
                .iter()
                .any(|t| !t.tx_log.is_empty() || !t.rx_log.is_empty())
    }

    pub fn take_usart_tx_bytes(&mut self, idx: usize) -> Vec<u8> {
        if let Some(u) = self.host.usarts.get_mut(idx) {
            std::mem::take(&mut u.tx_log)
        } else {
            Vec::new()
        }
    }

    pub fn take_usart_rx_bytes(&mut self, idx: usize) -> Vec<u8> {
        if let Some(u) = self.host.usarts.get_mut(idx) {
            std::mem::take(&mut u.rx_pin_log)
        } else {
            Vec::new()
        }
    }

    pub fn inject_usart_rx(&mut self, idx: usize, byte: u8) {
        if let Some(u) = self.host.usarts.get_mut(idx) {
            u.inject_rx_byte(byte, &mut self.host.data, &mut self.host.interrupts);
        }
    }

    pub fn take_twi_tx_bytes(&mut self, idx: usize) -> Vec<u8> {
        if let Some(t) = self.host.twis.get_mut(idx) {
            std::mem::take(&mut t.tx_log)
        } else {
            Vec::new()
        }
    }

    pub fn take_twi_rx_bytes(&mut self, idx: usize) -> Vec<u8> {
        if let Some(t) = self.host.twis.get_mut(idx) {
            std::mem::take(&mut t.rx_log)
        } else {
            Vec::new()
        }
    }

    pub fn from_desc(id: impl Into<String>, desc: McuDesc) -> Result<Self> {
        if !desc.core.supported() {
            return Err(Error::Unsupported(format!(
                "MCU core `{}` is not in this slice (PIC12/PIC14/AVR/8051/6502/Z80 first)",
                desc.core_name
            )));
        }
        let id = id.into();
        let mut data = desc.data;
        let mut ports = Vec::new();
        let avr = desc.core == CoreKind::Avr;
        let i51 = desc.core == CoreKind::I51;
        for (i, spec) in desc.ports.iter().enumerate() {
            let mut port = Port::from_spec(spec, &id);
            if avr {
                port.pullup_from_out = true;
            }
            if i51 && !spec.is_io {
                port.always_out = spec.dir_reg.is_empty();
                port.and_bus_with_latch = true;
            }
            if !spec.out_reg.is_empty() {
                port.out_addr = data.reg_addr(&spec.out_reg);
                data.watch_reg_write(&spec.out_reg, Watch::PortOut(i));
                if spec.in_reg.is_empty() {
                    data.watch_reg_read(&spec.out_reg, Watch::PortInRead(i));
                }
            }
            if !spec.in_reg.is_empty() {
                port.in_addr = data.reg_addr(&spec.in_reg);
                if avr {
                    data.watch_reg_write(&spec.in_reg, Watch::PortPinToggle(i));
                }
            }
            if !spec.dir_reg.is_empty() {
                port.dir_addr = data.reg_addr(&spec.dir_reg);
                data.watch_reg_write(&spec.dir_reg, Watch::PortDir(i));
            }
            port.apply_const_pullups(spec);
            ports.push(port);
        }
        let bank_from_status = desc.core == CoreKind::Pic14;
        if bank_from_status {
            data.watch_bits_write("R0,R1", Watch::SetBank);
        }

        let mut interrupts = Interrupts::from_spec(&desc.interrupts, &data, desc.core);
        if !desc.interrupts.enable.is_empty() {
            data.watch_bits_write(&desc.interrupts.enable, Watch::EnableGlobalInt);
        }
        for (i, spec) in desc.interrupts.ints.iter().enumerate() {
            if !spec.enable.is_empty() {
                data.watch_bits_write(&spec.enable, Watch::IntEnable(i));
            }
            if !spec.priority.is_empty() && spec.priority.parse::<u8>().is_err() {
                data.watch_bits_write(&spec.priority, Watch::IntPriority(i));
            }
            if desc.core != CoreKind::Avr && !spec.flag.is_empty() {
                data.watch_bits_write(&spec.flag, Watch::IntFlagClear(i));
            }
            if spec.clear_on_one {
                if let Some(iv) = interrupts.ints.get(i) {
                    data.watch_write(iv.flag_reg, Watch::IntWriteFlag(i), 0xFF);
                }
            }
        }

        let mut timers = Vec::new();
        for spec in &desc.timers {
            let idx = timers.len();
            let t = Timer::from_spec(spec, &data, desc.core, &interrupts);
            watch_config(
                &mut data,
                &spec.config,
                Watch::TimerConfigA(idx),
                Watch::TimerConfigB(idx),
                Watch::TimerConfigC(idx),
            );
            if let Some(low) = spec.counter.first() {
                data.watch_reg_write(low, Watch::TimerCountL(idx));
                data.watch_reg_read(low, Watch::TimerCountRead(idx));
            }
            if let Some(high) = spec.counter.get(1) {
                data.watch_reg_write(high, Watch::TimerCountH(idx));
                data.watch_reg_read(high, Watch::TimerCountRead(idx));
            }
            if !spec.enable.is_empty() {
                data.watch_bits_write(&spec.enable, Watch::TimerEnable(idx));
            }
            if let Some(low) = spec.top_reg0.first() {
                data.watch_reg_write(low, Watch::TimerTop0(idx));
            }
            for (oi, oc) in spec.oc_units.iter().enumerate() {
                let oca_is_top = spec.top_reg0.is_empty() && oc.name.ends_with('A');
                if let Some(low) = oc.ocreg.first() {
                    if oca_is_top {
                        data.watch_reg_write(low, Watch::TimerTop0(idx));
                    } else {
                        data.watch_reg_write(low, Watch::OcWriteL { timer: idx, oc: oi });
                    }
                }
                if let Some(high) = oc.ocreg.get(1) {
                    data.watch_reg_write(high, Watch::OcWriteH { timer: idx, oc: oi });
                }
                if oc.bits.is_empty() {
                    watch_config(
                        &mut data,
                        &oc.config,
                        Watch::OcConfig { timer: idx, oc: oi },
                        Watch::OcConfig { timer: idx, oc: oi },
                        Watch::OcConfig { timer: idx, oc: oi },
                    );
                }
            }
            if let Some(ic) = &spec.ic_unit {
                if !ic.bits.is_empty() {
                    data.watch_bits_write(&ic.bits, Watch::IcConfig(idx));
                }
            }
            if desc.core == CoreKind::Avr && spec.type_id / 10 == 16 {
                let n = spec
                    .name
                    .chars()
                    .last()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(1);
                data.watch_reg_write(&format!("ICR{n}L"), Watch::TimerIcrL(idx));
            }
            timers.push(t);
        }

        let mut ccps = Vec::new();
        for spec in &desc.ccps {
            let idx = ccps.len();
            let c = CcpUnit::attach(spec, &mut timers, &data, &interrupts);
            watch_config(
                &mut data,
                &spec.config,
                Watch::CcpConfig(idx),
                Watch::CcpConfig(idx),
                Watch::CcpConfig(idx),
            );
            if let Some(low) = spec.ccpreg.first() {
                data.watch_reg_write(low, Watch::CcpWriteL(idx));
            }
            if let Some(high) = spec.ccpreg.get(1) {
                data.watch_reg_write(high, Watch::CcpWriteH(idx));
            }
            ccps.push(c);
        }

        let ext_kind = match desc.core {
            CoreKind::Pic12 | CoreKind::Pic14 => crate::port::ExtIntKind::Pic,
            CoreKind::I51 => crate::port::ExtIntKind::I51,
            _ => crate::port::ExtIntKind::Avr,
        };
        for (i, spec) in desc.ports.iter().enumerate() {
            if let Some(ispec) = &spec.interrupt {
                if let Some(p) = ports.get_mut(i) {
                    p.interrupt = interrupts.get(&ispec.name);
                    if !ispec.mask.is_empty() {
                        if let Ok(bits) = u32::from_str_radix(ispec.mask.trim(), 2) {
                            p.int_mask = bits;
                            p.rst_int_mask = false;
                        } else {
                            data.watch_reg_write(&ispec.mask, Watch::PortIntMask(i));
                        }
                    }
                    if !ispec.bitmask.is_empty() {
                        p.int_bits = data.get_reg_bits(&ispec.bitmask);
                        data.watch_bits_write(&ispec.bitmask, Watch::PortIntMask(i));
                    }
                }
            }
            for e in &spec.extints {
                let found = crate::port::find_gpio(&ports, &e.pin);
                if let Some((pi, pin_i)) = found {
                    if let Some(pin) = ports.get_mut(pi).and_then(|p| p.pins.get_mut(pin_i)) {
                        pin.ext_int = interrupts.get(&e.name);
                        pin.ext_int_kind = ext_kind;
                        if ext_kind == crate::port::ExtIntKind::Pic {
                            pin.ext_int_trigger = 2;
                        }
                        if !e.config_bits.is_empty() {
                            pin.ext_int_bits = data.get_reg_bits(&e.config_bits);
                            data.watch_bits_write(
                                &e.config_bits,
                                Watch::PinExtInt {
                                    port: pi,
                                    pin: pin_i,
                                },
                            );
                        }
                    }
                }
            }
        }

        let mut usarts = Vec::new();
        for spec in &desc.usarts {
            let idx = usarts.len();
            let u = Usart::from_spec(spec, &data, desc.core, &interrupts);
            watch_config(
                &mut data,
                &spec.config,
                Watch::UsartConfigA(idx),
                Watch::UsartConfigB(idx),
                Watch::UsartConfigC(idx),
            );
            if let Some(tx) = &spec.tx {
                if !tx.register.is_empty() {
                    data.watch_reg_write(&tx.register, Watch::UsartSend(idx));
                }
                if !tx.enable.is_empty() {
                    data.watch_bits_write(&tx.enable, Watch::UsartTxEnable(idx));
                }
            }
            if let Some(rx) = &spec.rx {
                let reg = if rx.register.is_empty() {
                    spec.tx.as_ref().map(|t| t.register.as_str()).unwrap_or("")
                } else {
                    rx.register.as_str()
                };
                if !reg.is_empty() {
                    data.watch_reg_read(reg, Watch::UsartRead(idx));
                }
                if !rx.enable.is_empty() {
                    data.watch_bits_write(&rx.enable, Watch::UsartRxEnable(idx));
                }
            }
            match desc.core {
                CoreKind::Avr => {
                    let n = spec
                        .name
                        .chars()
                        .last()
                        .filter(|c| c.is_ascii_digit())
                        .map(|c| c.to_string())
                        .unwrap_or_default();
                    let l = format!("UBRR{n}L");
                    let h = format!("UBRR{n}H");
                    if data.reg_exist(&l) {
                        data.watch_reg_write(&l, Watch::UsartBaudL(idx));
                    }
                    if data.reg_exist(&h) {
                        data.watch_reg_write(&h, Watch::UsartBaudH(idx));
                    }
                }
                CoreKind::Pic14 | CoreKind::Pic12 => {
                    if data.reg_exist("SPBRG") {
                        data.watch_reg_write("SPBRG", Watch::UsartBaudL(idx));
                    }
                    if data.reg_exist("SPBRGL") {
                        data.watch_reg_write("SPBRGL", Watch::UsartBaudL(idx));
                    }
                    if data.reg_exist("SPBRGH") {
                        data.watch_reg_write("SPBRGH", Watch::UsartBaudH(idx));
                    }
                }
                _ => {}
            }
            usarts.push(u);
        }
        if data.reg_exist("ADCSRA") {
            data.watch_reg_write("ADCSRA", Watch::AdcConfig);
        }
        for (ui, u) in usarts.iter().enumerate() {
            let _ = u;
            if desc.core == CoreKind::I51 {
                if let Some(ti) = interrupts.get("T1_OVF") {
                    interrupts.ints[ti]
                        .callbacks
                        .push(IntCallback::UsartTick(ui));
                }
            }
        }

        let mut twis = Vec::new();
        for spec in &desc.twis {
            let idx = twis.len();
            let t = Twi::from_spec(spec, &data, desc.core, &interrupts);
            watch_config(
                &mut data,
                &spec.config,
                Watch::TwiConfigA(idx),
                Watch::TwiConfigB(idx),
                Watch::TwiConfigB(idx),
            );
            if !spec.data_reg.is_empty() {
                data.watch_reg_write(&spec.data_reg, Watch::TwiDataWrite(idx));
                if desc.core == CoreKind::Pic14 {
                    data.watch_reg_read(&spec.data_reg, Watch::TwiDataRead(idx));
                }
            }
            if !spec.addr_reg.is_empty() {
                data.watch_reg_write(&spec.addr_reg, Watch::TwiAddr(idx));
            }
            if !spec.status_reg.is_empty() {
                data.watch_reg_write(&spec.status_reg, Watch::TwiStatus(idx));
            }
            twis.push(t);
        }

        let (spl_addr, sph_addr, sp_pre, sp_inc) = if let Some(st) = &desc.stack {
            (
                st.regs.first().and_then(|n| data.reg_addr(n)),
                st.regs.get(1).and_then(|n| data.reg_addr(n)),
                st.pre(),
                st.inc(),
            )
        } else {
            (None, None, false, -1)
        };
        let prog_addr_size = if desc.prog_size <= 0xFF {
            1
        } else if desc.prog_size <= 0xFFFF {
            2
        } else {
            3
        };
        let ret_cycles = if avr { 4 } else { 2 };

        let cpu = match desc.core {
            CoreKind::Pic12 => Cpu::Pic12(Pic12::new(&data)),
            CoreKind::Pic14 => Cpu::Pic14(Pic14::new(&data)),
            CoreKind::Avr => Cpu::Avr(Avr::new(&data, desc.prog_page)),
            CoreKind::I51 => Cpu::I51(I51::new(&data)),
            CoreKind::Mcs65 => Cpu::Mcs65(Mcs65::new()),
            CoreKind::Z80 => Cpu::Z80(Z80::new()),
            CoreKind::Scripted | CoreKind::Other => Cpu::None,
        };

        let fill = desc.prog_fill;
        let mut prog = vec![fill; desc.prog_size as usize];
        for (addr, val) in desc.prog_init {
            if (addr as usize) < prog.len() {
                prog[addr as usize] = val;
            }
        }

        let host_low_start = data.reg_start;
        let host_reg_end = data.reg_end;
        let host_ram_end = data.ram_size.saturating_sub(1);
        let mut dev = Self {
            id,
            core: desc.core,
            cpu,
            host: CpuHost {
                data,
                ports,
                prog,
                pc: 0,
                cycles_done: 0,
                ret_addr: 0,
                ret_cycles,
                sleep: false,
                wdr: false,
                reti: false,
                spl_addr,
                sph_addr,
                sp_pre,
                sp_inc,
                prog_addr_size,
                enable_int: None,
                low_data_end: if host_low_start > 0 {
                    host_low_start - 1
                } else {
                    0
                },
                reg_end: host_reg_end,
                upper_data: host_ram_end > u32::from(host_reg_end),
                interrupts,
                timers,
                ccps,
                usarts,
                twis,
                freq_hz: 0.0,
                ps_inst: 0,
                ports_dirty: true,
                periph_dirty: true,
            },
            state: McuState::Stopped,
            cycle: 0,
            freq: 0.0,
            c_per_inst: desc.inst_cycle,
            c_per_tick: desc.cpu_cycle,
            ps_inst: 0,
            ps_tick: 0,
            vdd: 5.0,
            word_size: desc.word_size.max(1),
            bank_from_status,
            eeprom: vec![0xFF; desc.eeprom_size as usize],
            spi_module_names: desc.spis.iter().map(|s| s.name.clone()).collect(),
            cpu_remain: 0,
            bus_remain: None,
            sched: 0,
            cbs_buf: Vec::new(),
            is_spinning: false,
        };
        if desc.freq > 0.0 {
            dev.force_freq(desc.freq);
        }
        dev.reset();
        Ok(dev)
    }

    pub fn from_xml(id: impl Into<String>, xml: &str) -> Result<Self> {
        Self::from_desc(id, crate::parse_mcu_xml(xml)?)
    }

    pub fn flash_words(&self) -> &[u16] {
        &self.host.prog
    }

    pub fn eeprom(&self) -> &[u8] {
        &self.eeprom
    }

    pub fn set_eeprom(&mut self, addr: usize, v: u8) {
        if let Some(slot) = self.eeprom.get_mut(addr) {
            *slot = v;
        }
    }

    pub fn replace_eeprom(&mut self, data: &[u8]) {
        for (i, v) in data.iter().enumerate() {
            if let Some(slot) = self.eeprom.get_mut(i) {
                *slot = *v;
            }
        }
    }

    pub fn flash_size(&self) -> usize {
        self.host.prog.len()
    }

    pub fn load_words(&mut self, words: &[u16]) {
        for (i, w) in words.iter().enumerate() {
            self.set_flash(i, *w);
        }
    }

    pub fn set_flash(&mut self, addr: usize, val: u16) {
        if addr < self.host.prog.len() {
            self.host.prog[addr] = val;
        }
    }

    pub fn flash_word(&self, addr: usize) -> Option<u16> {
        self.host.prog.get(addr).copied()
    }

    pub fn load_hex(&mut self, src: &str) -> Result<()> {
        load_hex(
            src,
            &mut self.host.prog,
            false,
            u32::from(self.word_size) * 8,
        )?;
        Ok(())
    }

    pub fn core_kind(&self) -> CoreKind {
        self.core
    }

    pub fn reset(&mut self) {
        self.state = McuState::Stopped;
        self.cycle = 0;
        self.is_spinning = false;
        self.host.cycles_done = 0;
        self.host.pc = 0;
        self.host.ret_addr = 0;
        self.host.interrupts.reset();
        self.cpu_remain = 0;
        self.bus_remain = None;
        self.sched = 0;
        self.host.ports_dirty = true;
        self.host.periph_dirty = true;
        for p in &mut self.host.ports {
            p.reset();
        }
        for t in &mut self.host.timers {
            t.initialize(&mut self.host.ports);
        }
        for c in &mut self.host.ccps {
            c.initialize(&mut self.host.timers, &mut self.host.ports);
        }
        for u in &mut self.host.usarts {
            u.reset(&mut self.host.ports, &mut self.host.interrupts);
        }
        let fired = self.host.data.initialize();
        self.apply_init_watches(fired);
        for u in &mut self.host.usarts {
            u.reset_status(&mut self.host.data, &mut self.host.interrupts);
        }
        for t in &mut self.host.timers {
            t.schedule(&mut self.host.ports, self.ps_inst);
        }
        for twi in &mut self.host.twis {
            twi.update_freq(self.freq);
        }
        match &mut self.cpu {
            Cpu::None => {}
            Cpu::Pic12(c) => c.mr.reset(),
            Cpu::Pic14(c) => {
                c.mr.reset();
                c.mr.set_bank(self.host.data.get(self.host.data.sreg_addr));
            }
            Cpu::Avr(c) => {
                c.reset();
                // C++ `AvrCore::reset`: ramEnd = m_dataMemEnd - 1 = ramSize - 2.
                let ram_end = self.host.data.ram_size.saturating_sub(2) as u16;
                if let Some(a) = self.host.spl_addr {
                    self.host.data.set(a, ram_end as u8);
                }
                if let Some(a) = self.host.sph_addr {
                    self.host.data.set(a, (ram_end >> 8) as u8);
                }
            }
            Cpu::I51(c) => c.reset(self.ps_inst, &mut self.host.ports),
            Cpu::Mcs65(c) => c.reset(&mut self.host.ports),
            Cpu::Z80(c) => c.reset(&mut self.host.ports),
        }
        for p in &mut self.host.ports {
            if let Some(addr) = p.in_addr {
                self.host.data.set(addr, p.get_inp_state() as u8);
            }
        }
    }

    fn apply_init_watches(&mut self, fired: Vec<(Watch, u8)>) {
        for (w, val) in fired {
            match w {
                Watch::SetBank => {
                    if let Cpu::Pic14(c) = &mut self.cpu {
                        c.mr.set_bank(val);
                    }
                }
                Watch::PortDir(i) => {
                    if let Some(p) = self.host.ports.get_mut(i) {
                        let stored = p.dir_addr.map(|a| self.host.data.get(a)).unwrap_or(0xFF);
                        p.dir_changed(!stored, stored);
                    }
                }
                Watch::PortOut(i) => {
                    if let Some(p) = self.host.ports.get_mut(i) {
                        let stored = p.out_addr.map(|a| self.host.data.get(a)).unwrap_or(0);
                        p.out_changed(!stored, stored);
                    }
                }
                Watch::PortInRead(_) | Watch::PortPinToggle(_) => {
                    let _ = val;
                }
                Watch::EnableGlobalInt => self.host.interrupts.enable_global(val),
                Watch::IntEnable(i) => self.host.interrupts.enable_flag(i, val),
                Watch::IntFlagClear(_) | Watch::IntWriteFlag(_) => {}
                Watch::IntPriority(i) => self.host.interrupts.set_priority(i, val),
                Watch::TimerEnable(i) => {
                    if let Some(t) = self.host.timers.get_mut(i) {
                        t.enable(val, &mut self.host.data, &mut self.host.ports, self.ps_inst);
                    }
                }
                Watch::CcpConfig(i) => {
                    if let Some(c) = self.host.ccps.get_mut(i) {
                        c.configure_a(val, &mut self.host.timers, &mut self.host.ports);
                    }
                }
                Watch::TimerConfigA(i) => {
                    if let Some(t) = self.host.timers.get_mut(i) {
                        t.configure_a(val, &mut self.host.data, &mut self.host.ports, self.ps_inst);
                    }
                }
                Watch::TimerConfigB(i) => {
                    if let Some(t) = self.host.timers.get_mut(i) {
                        t.configure_b(val, &mut self.host.data, &mut self.host.ports, self.ps_inst);
                    }
                }
                Watch::TimerCountL(i) => {
                    if let Some(t) = self.host.timers.get_mut(i) {
                        t.count_write_l(
                            val,
                            &mut self.host.data,
                            &mut self.host.ports,
                            self.ps_inst,
                        );
                    }
                }
                Watch::TimerCountH(i) => {
                    if let Some(t) = self.host.timers.get_mut(i) {
                        t.count_write_h(
                            val,
                            &mut self.host.data,
                            &mut self.host.ports,
                            self.ps_inst,
                        );
                    }
                }
                Watch::UsartConfigA(i) => {
                    if let Some(u) = self.host.usarts.get_mut(i) {
                        u.configure_a(val, &mut self.host.ports, &mut self.host.data, self.ps_inst);
                    }
                }
                Watch::UsartConfigB(i) => {
                    if let Some(u) = self.host.usarts.get_mut(i) {
                        u.configure_b(val, &mut self.host.ports, &mut self.host.data, self.ps_inst);
                    }
                }
                _ => {
                    let _ = val;
                }
            }
        }
    }

    pub fn start(&mut self) {
        if self.state == McuState::Running {
            return;
        }
        self.state = McuState::Running;
    }

    pub fn hard_reset(&mut self, reset: bool) {
        let is_reset = self.state == McuState::Stopped;
        if reset == is_reset {
            return;
        }
        if reset {
            self.reset();
        } else {
            self.start();
        }
    }
}
