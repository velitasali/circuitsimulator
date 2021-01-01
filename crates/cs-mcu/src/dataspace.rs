//! C++ `DataSpace`: RAM including SFRs, address map, write masks, watchers.

use std::collections::HashMap;

/// C++ `regInfo_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegInfo {
    pub address: u16,
    pub reset_val: u8,
}

/// Watcher kind stored on a register. Dispatched by [`crate::Device`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Watch {
    PortOut(usize),
    PortDir(usize),
    PortInRead(usize),
    /// AVR: write to `PINx` toggles `PORTx` (C++ `AvrPort::pinRegChanged`).
    PortPinToggle(usize),
    SetBank,
    EnableGlobalInt,
    IntEnable(usize),
    IntFlagClear(usize),
    IntWriteFlag(usize),
    IntPriority(usize),
    TimerEnable(usize),
    TimerCountL(usize),
    TimerCountH(usize),
    TimerCountRead(usize),
    TimerConfigA(usize),
    TimerConfigB(usize),
    TimerConfigC(usize),
    TimerTop0(usize),
    /// OCR low-byte write (C++ `McuOcUnit::ocrWriteL`).
    OcWriteL {
        timer: usize,
        oc: usize,
    },
    OcWriteH {
        timer: usize,
        oc: usize,
    },
    OcConfig {
        timer: usize,
        oc: usize,
    },
    IcConfig(usize),
    /// AVR ICR low-byte write (C++ `AvrTimer16bit::ICRXLchanged`).
    TimerIcrL(usize),
    /// C++ `PicCcpUnit::configureA` / `ccprWriteL` / `ccprWriteH`.
    CcpConfig(usize),
    CcpWriteL(usize),
    CcpWriteH(usize),
    /// C++ `McuPort::intChanged` pin-change mask.
    PortIntMask(usize),
    /// C++ `McuPin::ConfExtInt`.
    PinExtInt {
        port: usize,
        pin: usize,
    },
    UsartSend(usize),
    UsartRead(usize),
    UsartConfigA(usize),
    UsartConfigB(usize),
    UsartConfigC(usize),
    UsartTxEnable(usize),
    UsartRxEnable(usize),
    UsartBaudL(usize),
    UsartBaudH(usize),
    /// AVR ADCSRA write: complete a conversion when ADSC is set.
    AdcConfig,
    TwiConfigA(usize),
    TwiConfigB(usize),
    TwiStatus(usize),
    TwiDataWrite(usize),
    TwiDataRead(usize),
    TwiAddr(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Watcher {
    pub watch: Watch,
    pub mask: u8,
}

/// C++ `regBits_t` without a live pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegBits {
    pub bit0: u8,
    pub mask: u8,
    pub reg_addr: u16,
}

impl RegBits {
    pub fn val(&self, reg: u8) -> u8 {
        if self.mask == 0 || self.bit0 >= 8 {
            0
        } else {
            (reg & self.mask) >> self.bit0
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataSpace {
    pub ram_size: u32,
    data: Vec<u8>,
    addr_map: Vec<u16>,
    reg_info: HashMap<String, RegInfo>,
    bit_masks: HashMap<String, u8>,
    bit_regs: HashMap<String, u16>,
    pub write_watch: Vec<Vec<Watcher>>,
    pub read_watch: Vec<Vec<Watcher>>,
    pub reg_mask: Vec<u8>,
    pub reg_start: u16,
    pub reg_end: u16,
    pub sreg_addr: u16,
    pub status_bits: Vec<String>,
    pub is_cpu_read: bool,
    pub reg_override: Option<u8>,
}

impl DataSpace {
    pub fn new(size: u32) -> Self {
        let n = size as usize;
        Self {
            ram_size: size,
            data: vec![0; n],
            addr_map: vec![0xFFFF; n],
            reg_mask: Vec::new(),
            reg_info: HashMap::new(),
            bit_masks: HashMap::new(),
            bit_regs: HashMap::new(),
            write_watch: vec![Vec::new(); n],
            read_watch: vec![Vec::new(); n],
            reg_start: 0xFFFF,
            reg_end: 0,
            sreg_addr: 0,
            status_bits: Vec::new(),
            is_cpu_read: true,
            reg_override: None,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn get(&self, addr: u16) -> u8 {
        self.data.get(addr as usize).copied().unwrap_or(0)
    }

    pub fn set(&mut self, addr: u16, v: u8) {
        if let Some(slot) = self.data.get_mut(addr as usize) {
            *slot = v;
        }
    }

    pub fn mapper_addr(&self, addr: u16) -> u16 {
        self.addr_map.get(addr as usize).copied().unwrap_or(0xFFFF)
    }

    pub fn set_map(&mut self, addr: u16, map_to: u16) {
        if let Some(slot) = self.addr_map.get_mut(addr as usize) {
            *slot = map_to;
        }
    }

    pub fn map_range(&mut self, start: u16, end: u16, mut map_to: u16) {
        for i in start..=end {
            if self.mapper_addr(i) == 0xFFFF {
                self.set_map(i, map_to);
            }
            map_to = map_to.wrapping_add(1);
        }
    }

    /// C++ `m_regMask.resize(regEnd + 1, 0xFF)`. New slots default to fully writable.
    pub fn ensure_reg_mask(&mut self, addr: u16) {
        let need = addr as usize + 1;
        if self.reg_mask.len() < need {
            self.reg_mask.resize(need, 0xFF);
        }
    }

    pub fn add_register(&mut self, name: &str, addr: u16, reset: u8, mask: Option<u8>, bits: &str) {
        if addr as u32 >= self.ram_size {
            return;
        }
        if addr < self.reg_start {
            self.reg_start = addr;
        }
        if addr > self.reg_end {
            self.reg_end = addr;
        }
        self.ensure_reg_mask(addr);
        if let Some(m) = mask {
            if let Some(slot) = self.reg_mask.get_mut(addr as usize) {
                *slot = m;
            }
        }
        self.set_map(addr, addr);
        self.reg_info.insert(
            name.to_string(),
            RegInfo {
                address: addr,
                reset_val: reset,
            },
        );
        if bits.is_empty() {
            return;
        }
        for (i, bit_name) in bits.split(',').enumerate() {
            if bit_name == "0" {
                continue;
            }
            let bit_mask = 1u8 << (i % 8);
            let bit_addr = addr + (i / 8) as u16;
            for alias in bit_name.split('|') {
                let alias = alias.trim();
                if alias.is_empty() {
                    continue;
                }
                self.bit_masks.insert(alias.to_string(), bit_mask);
                self.bit_regs.insert(alias.to_string(), bit_addr);
            }
        }
    }

    pub fn set_status(&mut self, name: &str, bits: Vec<String>) {
        if let Some(info) = self.reg_info.get(name) {
            self.sreg_addr = info.address;
        }
        self.status_bits = bits;
    }

    pub fn reg_addr(&self, name: &str) -> Option<u16> {
        self.reg_info
            .get(name)
            .or_else(|| self.reg_info.get(&name.to_ascii_uppercase()))
            .map(|r| r.address)
    }

    pub fn reg_exist(&self, name: &str) -> bool {
        self.reg_info.contains_key(name)
    }

    /// Named SFRs, sorted by name (C++ `QHash` keys then `sort`).
    pub fn registers(&self) -> Vec<(String, u16)> {
        let mut v: Vec<(String, u16)> = self
            .reg_info
            .iter()
            .map(|(n, i)| (n.clone(), i.address))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    /// C++ `getMapperAddr`; `None` if the cell is unmapped (`0xFFFF`).
    pub fn mapped_addr(&self, addr: u16) -> Option<u16> {
        let mapped = self.mapper_addr(addr);
        if mapped == 0xFFFF || u32::from(mapped) >= self.ram_size {
            None
        } else {
            Some(mapped)
        }
    }

    /// C++ `getRamValue` without CPU read watchers (`m_isCpuRead = false`).
    pub fn monitor_get(&self, addr: u16) -> u8 {
        self.mapped_addr(addr).map(|a| self.get(a)).unwrap_or(0)
    }

    pub fn watch_write(&mut self, addr: u16, watch: Watch, mask: u8) {
        let idx = addr as usize;
        if idx >= self.write_watch.len() {
            self.write_watch.resize(idx + 1, Vec::new());
        }
        self.write_watch[idx].insert(0, Watcher { watch, mask });
    }

    pub fn watch_read(&mut self, addr: u16, watch: Watch, mask: u8) {
        let idx = addr as usize;
        if idx >= self.read_watch.len() {
            self.read_watch.resize(idx + 1, Vec::new());
        }
        self.read_watch[idx].insert(0, Watcher { watch, mask });
    }

    #[inline(always)]
    pub fn read_watchers(&self, addr: u16) -> &[Watcher] {
        let idx = addr as usize;
        if idx < self.read_watch.len() {
            &self.read_watch[idx]
        } else {
            &[]
        }
    }

    #[inline(always)]
    pub fn write_watchers(&self, addr: u16) -> &[Watcher] {
        let idx = addr as usize;
        if idx < self.write_watch.len() {
            &self.write_watch[idx]
        } else {
            &[]
        }
    }

    pub fn watch_reg_write(&mut self, name: &str, watch: Watch) {
        if let Some(addr) = self.reg_addr(name) {
            self.watch_write(addr, watch, 0xFF);
        }
    }

    pub fn watch_reg_read(&mut self, name: &str, watch: Watch) {
        if let Some(addr) = self.reg_addr(name) {
            self.watch_read(addr, watch, 0xFF);
        }
    }

    pub fn watch_bits_write(&mut self, bit_names: &str, watch: Watch) {
        let list: Vec<&str> = bit_names.split(',').map(str::trim).collect();
        let Some(first) = list.first() else {
            return;
        };
        let mut mask = 0u8;
        for b in &list {
            mask |= self.bit_masks.get(*b).copied().unwrap_or(0);
        }
        if let Some(&addr) = self.bit_regs.get(*first) {
            if addr != 0 || self.reg_start == 0 {
                self.watch_write(addr, watch, mask);
            }
        }
    }

    pub fn get_reg_bits(&self, bit_names: &str) -> RegBits {
        let list: Vec<&str> = bit_names.split(',').map(str::trim).collect();
        let mut mask = 0u8;
        for b in &list {
            mask |= self.bit_masks.get(*b).copied().unwrap_or(0);
        }
        let mut bit0 = 0u8;
        if mask != 0 {
            let mut m = mask;
            while bit0 < 8 && (m & 1) == 0 {
                m >>= 1;
                bit0 += 1;
            }
        }
        let reg_addr = list
            .first()
            .and_then(|b| self.bit_regs.get(*b).copied())
            .unwrap_or(0);
        RegBits {
            bit0,
            mask,
            reg_addr,
        }
    }

    pub fn initialize(&mut self) -> Vec<(Watch, u8)> {
        self.is_cpu_read = true;
        let mut fired = Vec::new();
        for i in 0..self.data.len() as u16 {
            fired.extend(self.write_reg(i, 0, false));
        }
        let resets: Vec<(u16, u8)> = self
            .reg_info
            .values()
            .filter(|r| r.reset_val != 0)
            .map(|r| (r.address, r.reset_val))
            .collect();
        for (addr, v) in resets {
            fired.extend(self.write_reg(addr, v, false));
            self.set(addr, v);
        }
        fired
    }

    /// C++ `readReg`. Callers apply read watchers (which may set [`Self::reg_override`]).
    pub fn read_reg_raw(&self, addr: u16) -> (u8, Vec<(Watch, u8)>) {
        let v = self.get(addr);
        let watchers = self.read_watchers(addr);
        if watchers.is_empty() {
            return (v, Vec::new());
        }
        let fired = watchers
            .iter()
            .map(|w| (w.watch, v & w.mask))
            .collect::<Vec<_>>();
        (v, fired)
    }

    pub fn write_reg_val(&mut self, addr: u16, mut v: u8, masked: bool) -> (u8, u8) {
        let mut mask = 255u8;
        if masked {
            if (addr as usize) < self.reg_mask.len() {
                mask = self.reg_mask[addr as usize];
            }
            if mask != 0xFF && mask != 0x00 {
                v = (self.get(addr) & !mask) | (v & mask);
            }
        }
        if mask != 0x00 {
            self.set(addr, v);
        }
        (v, mask)
    }

    pub fn write_reg(&mut self, addr: u16, v: u8, masked: bool) -> Vec<(Watch, u8)> {
        let (stored, _mask) = self.write_reg_val(addr, v, masked);
        let watchers = self.write_watchers(addr);
        if watchers.is_empty() {
            return Vec::new();
        }
        watchers
            .iter()
            .map(|w| (w.watch, stored & w.mask))
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_mask_protects_readonly_bits() {
        let mut ds = DataSpace::new(16);
        ds.add_register(
            "STATUS",
            3,
            0x18,
            Some(0b0011_1111),
            "C,DC,Z,PD,TO,RP0,RP1,IRP",
        );
        ds.initialize();
        ds.write_reg(3, 0xFF, true);
        // mask 0x3F: top two bits stay at reset (IRP/RP1 were 0 in 0x18... wait reset is 0x18)
        // 0x18 & !0x3F | 0xFF & 0x3F = 0x00 | 0x3F = 0x3F. Reset 00011000, bits 6-7 already 0.
        assert_eq!(ds.get(3), 0x3F);
    }

    #[test]
    fn regblock_end_keeps_write_masks() {
        // C++ resizes m_regMask when the regblock end grows. Setting reg_end
        // first used to skip that resize, so UCSRA = U2X wiped UDRE.
        let mut ds = DataSpace::new(256);
        ds.reg_end = 0xFF;
        ds.add_register(
            "UCSR0A",
            0xC0,
            0b0010_0000,
            Some(0b0100_0011),
            "MPCM0,U2X0,UPE0,DOR0,FE0,UDRE0,TXC0,RXC0",
        );
        ds.initialize();
        ds.write_reg(0xC0, 0x02, true);
        assert_eq!(
            ds.get(0xC0),
            0x22,
            "UDRE (bit 5) must survive Serial.begin UCSRA = U2X"
        );
    }

    #[test]
    fn bit_aliases_and_reg_bits() {
        let mut ds = DataSpace::new(16);
        ds.add_register("STATUS", 3, 0, None, "C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP");
        let rb = ds.get_reg_bits("R0,R1");
        assert_eq!(rb.mask, 0b0110_0000);
        assert_eq!(rb.bit0, 5);
        assert_eq!(rb.val(0b0010_0000), 1);
    }

    #[test]
    fn monitor_get_follows_mapper() {
        let mut ds = DataSpace::new(0x90);
        ds.add_register("STATUS", 3, 0x18, None, "C,DC,Z,PD,TO,RP0,RP1,IRP");
        ds.set(3, 0x5A);
        ds.set_map(0x83, 3);
        assert_eq!(ds.monitor_get(3), 0x5A);
        assert_eq!(ds.monitor_get(0x83), 0x5A);
        assert_eq!(ds.monitor_get(0x7F), 0);
        let regs = ds.registers();
        assert_eq!(regs, vec![("STATUS".into(), 3)]);
    }

    #[test]
    fn missing_reg_bits_does_not_overflow() {
        let ds = DataSpace::new(16);
        let rb = ds.get_reg_bits("NONEXISTENT");
        assert_eq!(rb.mask, 0);
        assert_eq!(rb.bit0, 0);
        assert_eq!(rb.val(0xFF), 0);

        let custom_rb = RegBits {
            bit0: 8,
            mask: 0,
            reg_addr: 0,
        };
        assert_eq!(custom_rb.val(0xFF), 0);
    }
}
