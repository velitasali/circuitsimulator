//! MCU monitor façade. Live RAM / STATUS / PC come from `cs_engine::mcu`
//! snapshots published by the canvas.

use std::collections::HashMap;

use cs_engine::debug::{DebugSession, VarType, WatchEntry};
use cs_engine::mcu::{self, McuPoke, McuSnap, val2hex};
use qtbridge::qobject;
use serde_json::{Value, json};

pub struct McuMonitor {
    snap: McuSnap,
    byte_mode: bool,
    jump_to_address: bool,
    current_tab: i32,
    ram_page: i32,
    flash_page: i32,
    eeprom_page: i32,
    ram_revision: i32,
    flash_revision: i32,
    eeprom_revision: i32,
    watch_revision: i32,
    symbols_gen: u64,
    jump_address: String,
    watch_eval: Vec<Value>,
    addr_names: HashMap<u16, String>,
}

impl Default for McuMonitor {
    fn default() -> Self {
        Self {
            snap: McuSnap::default(),
            byte_mode: false,
            jump_to_address: false,
            current_tab: 0,
            ram_page: 0,
            flash_page: 0,
            eeprom_page: 0,
            ram_revision: 0,
            flash_revision: 0,
            eeprom_revision: 0,
            watch_revision: 0,
            symbols_gen: 0,
            jump_address: String::new(),
            watch_eval: Vec::new(),
            addr_names: HashMap::new(),
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl McuMonitor {
    qproperty!("statusBits", Read = status_bits, Notify = status_changed);
    qproperty!("statusValue", Read = status_value, Notify = status_changed);
    qproperty!("hasStatus", Read = has_status, Notify = status_changed);
    qproperty!("pcValue", Read = pc_value, Notify = pc_changed);
    qproperty!("pcHex", Read = pc_hex, Notify = pc_changed);
    qproperty!("pcFlashRow", Read = pc_flash_row, Notify = pc_changed);
    qproperty!("mcuId", Read = mcu_id, Notify = status_changed);
    qproperty!(
        "byteMode",
        Read = byte_mode,
        Write = set_byte_mode,
        Notify = byte_mode_changed
    );
    qproperty!(
        "jumpToAddress",
        Read = jump_to_address,
        Write = set_jump_to_address,
        Notify = jump_changed
    );
    qproperty!(
        "jumpAddress",
        Read = jump_address,
        Write = set_jump_address,
        Notify = jump_address_changed
    );
    qproperty!("ramRowCount", Read = ram_row_count, Notify = ram_changed);
    qproperty!("ramTotalRows", Read = ram_total_rows, Notify = ram_changed);
    qproperty!("ramRevision", Read = ram_revision, Notify = ram_changed);

    qproperty!(
        "flashRowCount",
        Read = flash_row_count,
        Notify = flash_changed
    );
    qproperty!(
        "flashTotalWords",
        Read = flash_total_words,
        Notify = flash_changed
    );
    qproperty!(
        "flashRevision",
        Read = flash_revision,
        Notify = flash_changed
    );

    qproperty!(
        "eepromRowCount",
        Read = eeprom_row_count,
        Notify = eeprom_changed
    );
    qproperty!(
        "eepromTotalBytes",
        Read = eeprom_total_bytes,
        Notify = eeprom_changed
    );
    qproperty!(
        "eepromRevision",
        Read = eeprom_revision,
        Notify = eeprom_changed
    );

    qproperty!(
        "ramPage",
        Read = ram_page,
        Write = set_ram_page,
        Notify = ram_page_changed
    );
    qproperty!(
        "ramPageCount",
        Read = ram_page_count,
        Notify = ram_page_changed
    );
    qproperty!(
        "ramPageRowCount",
        Read = ram_row_count,
        Notify = ram_page_changed
    );

    qproperty!(
        "flashPage",
        Read = flash_page,
        Write = set_flash_page,
        Notify = flash_page_changed
    );
    qproperty!(
        "flashPageCount",
        Read = flash_page_count,
        Notify = flash_page_changed
    );
    qproperty!(
        "flashPageRowCount",
        Read = flash_row_count,
        Notify = flash_page_changed
    );

    qproperty!(
        "eepromPage",
        Read = eeprom_page,
        Write = set_eeprom_page,
        Notify = eeprom_page_changed
    );
    qproperty!(
        "eepromPageCount",
        Read = eeprom_page_count,
        Notify = eeprom_page_changed
    );
    qproperty!(
        "eepromPageRowCount",
        Read = eeprom_row_count,
        Notify = eeprom_page_changed
    );

    qproperty!("splitWatch", Read = split_watch, Notify = tabs_changed);
    qproperty!("tabs", Read = tabs, Notify = tabs_changed);
    qproperty!("ramRows", Read = ram_rows, Notify = ram_changed);
    qproperty!("ramTableRows", Read = ram_table_rows, Notify = ram_changed);
    qproperty!("watchRows", Read = watch_rows, Notify = watch_changed);
    qproperty!("watchCount", Read = watch_count, Notify = watch_changed);
    qproperty!(
        "watchRevision",
        Read = watch_revision,
        Notify = watch_changed
    );
    qproperty!("flashRows", Read = flash_rows, Notify = flash_changed);
    qproperty!(
        "flashTableRows",
        Read = flash_table_rows,
        Notify = flash_changed
    );
    qproperty!("eepromRows", Read = eeprom_rows, Notify = eeprom_changed);
    qproperty!(
        "eepromTableRows",
        Read = eeprom_table_rows,
        Notify = eeprom_changed
    );
    qproperty!("registers", Read = registers, Notify = registers_changed);
    qproperty!("variables", Read = variables, Notify = variables_changed);

    #[qsignal]
    fn status_changed(&mut self);
    #[qsignal]
    fn pc_changed(&mut self);
    #[qsignal]
    fn byte_mode_changed(&mut self);
    #[qsignal]
    fn jump_changed(&mut self);
    #[qsignal]
    fn jump_address_changed(&mut self);
    #[qsignal]
    fn ram_page_changed(&mut self);
    #[qsignal]
    fn flash_page_changed(&mut self);
    #[qsignal]
    fn eeprom_page_changed(&mut self);
    #[qsignal]
    fn tabs_changed(&mut self);
    #[qsignal]
    fn ram_changed(&mut self);
    #[qsignal]
    fn watch_changed(&mut self);
    #[qsignal]
    fn flash_changed(&mut self);
    #[qsignal]
    fn eeprom_changed(&mut self);
    #[qsignal]
    fn registers_changed(&mut self);
    #[qsignal]
    fn variables_changed(&mut self);

    fn has_status(&self) -> bool {
        self.snap.has_status
    }

    fn split_watch(&self) -> bool {
        !self.snap.ram.is_empty() && !self.snap.registers.is_empty()
    }

    fn mcu_id(&self) -> String {
        self.snap.id.clone()
    }

    fn ram_revision(&self) -> i32 {
        self.ram_revision
    }

    fn flash_revision(&self) -> i32 {
        self.flash_revision
    }

    fn eeprom_revision(&self) -> i32 {
        self.eeprom_revision
    }

    fn watch_revision(&self) -> i32 {
        self.watch_revision
    }

    fn byte_mode(&self) -> bool {
        self.byte_mode
    }

    fn jump_to_address(&self) -> bool {
        self.jump_to_address
    }

    fn jump_address(&self) -> String {
        self.jump_address.clone()
    }

    fn ram_page(&self) -> i32 {
        self.ram_page
    }

    fn flash_page(&self) -> i32 {
        self.flash_page
    }

    fn eeprom_page(&self) -> i32 {
        self.eeprom_page
    }

    fn status_bits(&self) -> Vec<String> {
        let mut bits = self.snap.status_bits.clone();
        bits.resize(8, String::new());
        bits
    }

    fn status_value(&self) -> i32 {
        i32::from(self.snap.status)
    }

    fn display_pc(&self) -> u32 {
        if self.byte_mode {
            self.snap
                .pc
                .saturating_mul(u32::from(self.snap.word_size.max(1)))
        } else {
            self.snap.pc
        }
    }

    fn pc_flash_row(&self) -> i32 {
        (self.snap.pc / 8) as i32
    }

    fn pc_value(&self) -> i32 {
        self.display_pc() as i32
    }

    fn pc_hex(&self) -> String {
        format!("0x{}", val2hex(self.display_pc()))
    }

    fn set_byte_mode(&mut self, v: bool) {
        if self.byte_mode == v {
            return;
        }
        self.byte_mode = v;
        self.flash_revision = self.flash_revision.wrapping_add(1);
        self.byte_mode_changed();
        self.flash_changed();
        self.status_changed();
        self.pc_changed();
    }

    fn set_jump_to_address(&mut self, v: bool) {
        if self.jump_to_address == v {
            return;
        }
        self.jump_to_address = v;
        self.jump_changed();
    }

    fn set_jump_address(&mut self, s: String) {
        self.jump_address = s;
        self.jump_address_changed();
    }

    fn set_ram_page(&mut self, p: i32) {
        let max = (self.ram_page_count() - 1).max(0);
        let clamped = p.clamp(0, max);
        if self.ram_page != clamped {
            self.ram_page = clamped;
            self.ram_page_changed();
            self.ram_changed();
        }
    }

    fn ram_row_count(&self) -> i32 {
        let total = self.snap.ram.len();
        if total == 0 {
            0
        } else {
            ((total + 15) / 16) as i32
        }
    }

    fn ram_total_rows(&self) -> i32 {
        self.snap.ram.len() as i32
    }

    fn flash_row_count(&self) -> i32 {
        let total = self.snap.flash.len();
        if total == 0 {
            0
        } else {
            ((total + 7) / 8) as i32
        }
    }

    fn flash_total_words(&self) -> i32 {
        self.snap.flash.len() as i32
    }

    fn eeprom_row_count(&self) -> i32 {
        let total = self.snap.eeprom.len();
        if total == 0 {
            0
        } else {
            ((total + 15) / 16) as i32
        }
    }

    fn eeprom_total_bytes(&self) -> i32 {
        self.snap.eeprom.len() as i32
    }

    fn ram_page_count(&self) -> i32 {
        1
    }

    fn flash_page_count(&self) -> i32 {
        1
    }

    fn eeprom_page_count(&self) -> i32 {
        1
    }

    fn watch_count(&self) -> i32 {
        self.watch_eval.len() as i32
    }

    fn set_flash_page(&mut self, p: i32) {
        let max = (self.flash_page_count() - 1).max(0);
        let clamped = p.clamp(0, max);
        if self.flash_page != clamped {
            self.flash_page = clamped;
            self.flash_page_changed();
            self.flash_changed();
        }
    }

    fn set_eeprom_page(&mut self, p: i32) {
        let max = (self.eeprom_page_count() - 1).max(0);
        let clamped = p.clamp(0, max);
        if self.eeprom_page != clamped {
            self.eeprom_page = clamped;
            self.eeprom_page_changed();
            self.eeprom_changed();
        }
    }

    fn tabs(&self) -> Value {
        if self.snap.id.is_empty() {
            return json!([{ "title": cs_engine::i18n::tr("Watch"), "source": "watch" }]);
        }
        let mut tabs = vec![
            json!({ "title": cs_engine::i18n::tr("Watch"), "source": "watch" }),
            json!({ "title": cs_engine::i18n::tr("RAM (Hex Table)"), "source": "ram_matrix" }),
        ];
        if !self.snap.ram.is_empty() {
            tabs.push(json!({ "title": cs_engine::i18n::tr("RAM List"), "source": "ram" }));
        }
        if !self.snap.flash.is_empty() {
            tabs.push(json!({ "title": cs_engine::i18n::tr("Flash"), "source": "flash" }));
        }
        if !self.snap.eeprom.is_empty() {
            tabs.push(json!({ "title": cs_engine::i18n::tr("EEPROM"), "source": "eeprom" }));
        }
        Value::Array(tabs)
    }

    fn registers(&self) -> Value {
        let mut sorted_regs = self.snap.registers.clone();
        sorted_regs.sort_by(|a, b| a.0.cmp(&b.0));
        let arr: Vec<Value> = sorted_regs
            .iter()
            .map(|(name, addr)| {
                json!({
                    "name": name,
                    "address": format!("0x{addr:04X}"),
                })
            })
            .collect();
        Value::Array(arr)
    }

    fn variables(&self) -> Value {
        let mut vars = DebugSession::global().debug_variables();
        vars.sort_by(|a, b| a.name.cmp(&b.name));
        let arr: Vec<Value> = vars
            .iter()
            .map(|v| {
                json!({
                    "name": v.name,
                    "address": format!("0x{:04X}", v.address),
                    "type": v.var_type,
                    "size": v.size,
                    "isSfr": v.is_sfr,
                })
            })
            .collect();
        Value::Array(arr)
    }

    #[qslot]
    fn ram_row_data(&self, row_idx: i32, _rev: i32) -> Value {
        let row_addr = (row_idx.max(0) as usize) * 16;

        let mut bytes_json = Vec::with_capacity(16);
        let mut vals_json = Vec::with_capacity(16);
        let mut addrs_json = Vec::with_capacity(16);
        let mut ascii = String::with_capacity(16);

        for col in 0..16 {
            let addr = row_addr + col;
            if addr < self.snap.ram.len() {
                let b = self.snap.ram[addr];
                bytes_json.push(json!(format!("{b:02X}")));
                vals_json.push(json!(b));
                addrs_json.push(json!(addr));
                let c = b as char;
                if c.is_ascii_graphic() || c == ' ' {
                    ascii.push(c);
                } else {
                    ascii.push('.');
                }
            } else {
                bytes_json.push(json!("--"));
                vals_json.push(json!(-1));
                addrs_json.push(json!(-1));
                ascii.push(' ');
            }
        }

        json!({
            "address": format!("{row_addr:04X}"),
            "bytes": bytes_json,
            "vals": vals_json,
            "addrs": addrs_json,
            "ascii": ascii,
        })
    }

    #[qslot]
    fn flash_row_data(&self, row_idx: i32, _rev: i32) -> Value {
        let word_size = self.snap.word_size.max(1) as usize;
        let addr_mult = if self.byte_mode { word_size } else { 1 };
        let base_word = (row_idx.max(0) as usize) * 8;
        let row_addr = base_word * addr_mult;

        let mut words_json = Vec::with_capacity(8);
        let mut vals_json = Vec::with_capacity(8);
        let mut addrs_json = Vec::with_capacity(8);
        let mut ascii = String::with_capacity(16);

        for col in 0..8 {
            let word_idx = base_word + col;
            let display_addr = word_idx * addr_mult;
            if word_idx < self.snap.flash.len() {
                let w = self.snap.flash[word_idx];
                words_json.push(json!(format!("{w:04X}")));
                vals_json.push(json!(w));
                addrs_json.push(json!(display_addr));
                let low_byte = (w & 0xFF) as u8;
                let high_byte = ((w >> 8) & 0xFF) as u8;
                for b in [low_byte, high_byte] {
                    let c = b as char;
                    if c.is_ascii_graphic() || c == ' ' {
                        ascii.push(c);
                    } else {
                        ascii.push('.');
                    }
                }
            } else {
                words_json.push(json!("----"));
                vals_json.push(json!(-1));
                addrs_json.push(json!(-1));
                ascii.push_str("  ");
            }
        }

        json!({
            "address": format!("{row_addr:04X}"),
            "words": words_json,
            "vals": vals_json,
            "addrs": addrs_json,
            "ascii": ascii,
        })
    }

    #[qslot]
    fn eeprom_row_data(&self, row_idx: i32, _rev: i32) -> Value {
        let row_addr = (row_idx.max(0) as usize) * 16;

        let mut bytes_json = Vec::with_capacity(16);
        let mut vals_json = Vec::with_capacity(16);
        let mut addrs_json = Vec::with_capacity(16);
        let mut ascii = String::with_capacity(16);

        for col in 0..16 {
            let addr = row_addr + col;
            if addr < self.snap.eeprom.len() {
                let b = self.snap.eeprom[addr];
                bytes_json.push(json!(format!("{b:02X}")));
                vals_json.push(json!(b));
                addrs_json.push(json!(addr));
                let c = b as char;
                if c.is_ascii_graphic() || c == ' ' {
                    ascii.push(c);
                } else {
                    ascii.push('.');
                }
            } else {
                bytes_json.push(json!("--"));
                vals_json.push(json!(-1));
                addrs_json.push(json!(-1));
                ascii.push(' ');
            }
        }

        json!({
            "address": format!("{row_addr:04X}"),
            "bytes": bytes_json,
            "vals": vals_json,
            "addrs": addrs_json,
            "ascii": ascii,
        })
    }

    #[qslot]
    fn ram_list_row_data(&self, addr: i32, _rev: i32) -> Value {
        if addr < 0 || addr as usize >= self.snap.ram.len() {
            return json!({
                "address": "----",
                "name": "",
                "value": "--",
                "hex": "--",
                "dec": 0,
                "bin": "",
                "type": "uint8",
                "val": -1,
                "addr": -1,
            });
        }
        let uaddr = addr as usize;
        let val = self.snap.ram[uaddr];
        let name = self
            .addr_names
            .get(&(addr as u16))
            .cloned()
            .unwrap_or_default();
        let hex_str = format!("{val:02X}");
        let dec_str = format!("{val}");
        let bin_str = format!("0b{val:08b}");
        let formatted_val = format!("{val:>3}  0x{hex_str}  {bin_str}");
        json!({
            "address": format!("{addr:04X}"),
            "name": name,
            "value": formatted_val,
            "hex": hex_str,
            "dec": dec_str,
            "bin": bin_str,
            "type": "uint8",
            "val": val,
            "addr": addr,
        })
    }

    fn ram_rows(&self) -> Value {
        let arr: Vec<Value> = self
            .snap
            .ram
            .iter()
            .enumerate()
            .map(|(addr, val)| {
                json!({
                    "address": format!("{addr:04X}"),
                    "name": self.addr_names.get(&(addr as u16)).cloned().unwrap_or_default(),
                    "value": format!("{val:02X}"),
                    "type": "uint8",
                })
            })
            .collect();
        Value::Array(arr)
    }

    fn ram_table_rows(&self) -> Value {
        let page_start = (self.ram_page as usize) * 960;
        let mut rows = Vec::with_capacity(60);

        for row_idx in 0..60 {
            let row_addr = page_start + row_idx * 16;
            if row_addr >= self.snap.ram.len() && row_idx > 0 && !self.snap.ram.is_empty() {
                break;
            }

            let mut bytes_json = Vec::with_capacity(16);
            let mut ascii = String::with_capacity(16);

            for col in 0..16 {
                let addr = row_addr + col;
                if addr < self.snap.ram.len() {
                    let b = self.snap.ram[addr];
                    bytes_json.push(json!({
                        "hex": format!("{b:02X}"),
                        "val": b,
                        "addr": addr,
                    }));
                    let c = b as char;
                    if c.is_ascii_graphic() || c == ' ' {
                        ascii.push(c);
                    } else {
                        ascii.push('.');
                    }
                } else {
                    bytes_json.push(json!({
                        "hex": "--",
                        "val": -1,
                        "addr": -1,
                    }));
                    ascii.push(' ');
                }
            }

            rows.push(json!({
                "address": format!("{row_addr:04X}"),
                "row_index": row_idx,
                "bytes": bytes_json,
                "ascii": ascii,
            }));
        }

        Value::Array(rows)
    }

    #[qslot]
    fn watch_row_data(&self, index: i32, _rev: i32) -> Value {
        if index < 0 || index as usize >= self.watch_eval.len() {
            return json!({
                "name": "",
                "expr": "",
                "address": "---",
                "value": "---",
                "type": "uint8",
                "hex": "---",
                "dec": "---",
                "bin": "---",
                "addr": -1,
            });
        }
        self.watch_eval[index as usize].clone()
    }

    fn watch_rows(&self) -> Value {
        Value::Array(self.watch_eval.clone())
    }

    fn flash_rows(&self) -> Value {
        let width = if self.snap.word_size > 1 { 4 } else { 2 };
        let arr: Vec<Value> = self
            .snap
            .flash
            .iter()
            .enumerate()
            .map(|(addr, val)| {
                json!({
                    "address": format!("{addr:04X}"),
                    "name": "",
                    "value": format!("{val:0width$X}", width = width),
                    "type": if self.snap.word_size > 1 { "uint16" } else { "uint8" },
                })
            })
            .collect();
        Value::Array(arr)
    }

    fn flash_table_rows(&self) -> Value {
        let page_start = (self.flash_page as usize) * 480;
        let mut rows = Vec::with_capacity(60);

        for row_idx in 0..60 {
            let row_addr = page_start + row_idx * 8;
            if row_addr >= self.snap.flash.len() && row_idx > 0 && !self.snap.flash.is_empty() {
                break;
            }

            let mut words_json = Vec::with_capacity(8);
            let mut ascii = String::with_capacity(16);

            for col in 0..8 {
                let addr = row_addr + col;
                if addr < self.snap.flash.len() {
                    let w = self.snap.flash[addr];
                    let low_byte = (w & 0xFF) as u8;
                    let high_byte = ((w >> 8) & 0xFF) as u8;
                    words_json.push(json!({
                        "hex": format!("{w:04X}"),
                        "val": w,
                        "addr": addr,
                    }));
                    for b in [low_byte, high_byte] {
                        let c = b as char;
                        if c.is_ascii_graphic() || c == ' ' {
                            ascii.push(c);
                        } else {
                            ascii.push('.');
                        }
                    }
                } else {
                    words_json.push(json!({
                        "hex": "----",
                        "val": -1,
                        "addr": -1,
                    }));
                    ascii.push_str("  ");
                }
            }

            rows.push(json!({
                "address": format!("{row_addr:04X}"),
                "row_index": row_idx,
                "words": words_json,
                "ascii": ascii,
            }));
        }

        Value::Array(rows)
    }

    fn eeprom_rows(&self) -> Value {
        let arr: Vec<Value> = self
            .snap
            .eeprom
            .iter()
            .enumerate()
            .map(|(addr, val)| {
                json!({
                    "address": format!("{addr:04X}"),
                    "name": "",
                    "value": format!("{val:02X}"),
                    "type": "uint8",
                })
            })
            .collect();
        Value::Array(arr)
    }

    fn eeprom_table_rows(&self) -> Value {
        let page_start = (self.eeprom_page as usize) * 960;
        let mut rows = Vec::with_capacity(60);

        for row_idx in 0..60 {
            let row_addr = page_start + row_idx * 16;
            if row_addr >= self.snap.eeprom.len() && row_idx > 0 && !self.snap.eeprom.is_empty() {
                break;
            }

            let mut bytes_json = Vec::with_capacity(16);
            let mut ascii = String::with_capacity(16);

            for col in 0..16 {
                let addr = row_addr + col;
                if addr < self.snap.eeprom.len() {
                    let b = self.snap.eeprom[addr];
                    bytes_json.push(json!({
                        "hex": format!("{b:02X}"),
                        "val": b,
                        "addr": addr,
                    }));
                    let c = b as char;
                    if c.is_ascii_graphic() || c == ' ' {
                        ascii.push(c);
                    } else {
                        ascii.push('.');
                    }
                } else {
                    bytes_json.push(json!({
                        "hex": "--",
                        "val": -1,
                        "addr": -1,
                    }));
                    ascii.push(' ');
                }
            }

            rows.push(json!({
                "address": format!("{row_addr:04X}"),
                "row_index": row_idx,
                "bytes": bytes_json,
                "ascii": ascii,
            }));
        }

        Value::Array(rows)
    }

    fn rebuild_addr_names(&mut self) {
        self.addr_names = self
            .snap
            .registers
            .iter()
            .map(|(n, a)| (*a, n.clone()))
            .collect();
    }

    fn rebuild_watches(&mut self) {
        let evaluated = DebugSession::global().evaluate_watches(&self.snap);
        let json: Vec<Value> = evaluated
            .into_iter()
            .map(|v| {
                json!({
                    "name": v.name,
                    "expr": v.expr,
                    "address": v.address.map(|a| format!("0x{a:04X}")).unwrap_or_else(|| "---".to_string()),
                    "value": v.formatted,
                    "type": v.type_name,
                    "hex": v.hex,
                    "dec": v.dec,
                    "bin": v.bin,
                    "addr": v.address.map(|a| a as i32).unwrap_or(-1),
                })
            })
            .collect();
        if json != self.watch_eval {
            self.watch_eval = json;
            self.watch_revision = self.watch_revision.wrapping_add(1);
            self.watch_changed();
        }
    }

    fn poll_symbols(&mut self) {
        let generation = DebugSession::global().symbols_generation();
        if generation != self.symbols_gen {
            self.symbols_gen = generation;
            self.variables_changed();
        }
    }

    fn apply_snap(&mut self, snap: McuSnap) {
        let tabs_changed = self.snap.id != snap.id
            || self.snap.ram.len() != snap.ram.len()
            || self.snap.flash.len() != snap.flash.len()
            || self.snap.eeprom.len() != snap.eeprom.len()
            || self.snap.registers.len() != snap.registers.len();
        let pc_changed = self.snap.pc != snap.pc;
        let status_changed = self.snap.status != snap.status
            || pc_changed
            || self.snap.status_bits != snap.status_bits
            || self.snap.has_status != snap.has_status
            || self.snap.id != snap.id;
        let ram_changed = self.snap.ram != snap.ram;
        let flash_changed = self.snap.flash != snap.flash;
        let eeprom_changed = self.snap.eeprom != snap.eeprom;
        let registers_changed = self.snap.registers != snap.registers;

        if ram_changed {
            self.ram_revision = self.ram_revision.wrapping_add(1);
        }
        if flash_changed {
            self.flash_revision = self.flash_revision.wrapping_add(1);
        }
        if eeprom_changed {
            self.eeprom_revision = self.eeprom_revision.wrapping_add(1);
        }

        self.snap = snap;
        if registers_changed || tabs_changed {
            self.rebuild_addr_names();
        }
        if ram_changed || registers_changed || tabs_changed {
            self.rebuild_watches();
        }
        if tabs_changed {
            self.tabs_changed();
        }
        if status_changed {
            self.status_changed();
        }
        if pc_changed {
            self.pc_changed();
        }
        if ram_changed || tabs_changed {
            self.ram_changed();
        }
        if flash_changed || tabs_changed {
            self.flash_changed();
        }
        if eeprom_changed || tabs_changed {
            self.eeprom_changed();
        }
        if registers_changed || tabs_changed {
            self.registers_changed();
        }
    }

    #[qslot]
    fn refresh(&mut self) {
        self.apply_snap(mcu::monitor_snap());
        self.poll_symbols();
    }

    #[qslot]
    fn set_current_tab(&mut self, index: i32) {
        self.current_tab = index;
    }

    #[qslot]
    fn next_ram_page(&mut self) {
        self.set_ram_page(self.ram_page + 1);
    }

    #[qslot]
    fn prev_ram_page(&mut self) {
        self.set_ram_page(self.ram_page - 1);
    }

    #[qslot]
    fn next_flash_page(&mut self) {
        self.set_flash_page(self.flash_page + 1);
    }

    #[qslot]
    fn prev_flash_page(&mut self) {
        self.set_flash_page(self.flash_page - 1);
    }

    #[qslot]
    fn next_eeprom_page(&mut self) {
        self.set_eeprom_page(self.eeprom_page + 1);
    }

    #[qslot]
    fn prev_eeprom_page(&mut self) {
        self.set_eeprom_page(self.eeprom_page - 1);
    }

    #[qslot]
    fn jump_to_ram_address(&mut self, addr: i32) {
        if addr >= 0 {
            let page = addr / 960;
            self.set_ram_page(page);
        }
    }

    #[qslot]
    fn jump_to_hex_address(&mut self, hex_str: String) {
        let clean = hex_str
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        if let Ok(addr) = i32::from_str_radix(clean, 16) {
            self.jump_to_ram_address(addr);
        } else if let Ok(addr) = clean.parse::<i32>() {
            self.jump_to_ram_address(addr);
        }
    }

    #[qslot]
    fn toggle_status_bit(&mut self, bit_index: i32) {
        if bit_index < 0 || bit_index >= 8 {
            return;
        }
        let bit = 1u8 << (bit_index as u8);
        let new_status = self.snap.status ^ bit;
        self.snap.status = new_status;
        self.status_changed();

        // Queue poke to status register if address is known
        for (name, addr) in &self.snap.registers {
            if name == "STATUS" || name == "SREG" || name == "PSW" {
                mcu::queue_poke(McuPoke::Ram {
                    addr: *addr,
                    value: new_status,
                });
                break;
            }
        }
    }

    #[qslot]
    fn poke_sfr_bit(&mut self, reg_name: String, bit_index: i32, val: bool) {
        if bit_index < 0 || bit_index >= 8 {
            return;
        }
        for (name, addr) in &self.snap.registers {
            if name.eq_ignore_ascii_case(&reg_name) {
                let current = self.snap.ram.get(*addr as usize).copied().unwrap_or(0);
                let mask = 1u8 << (bit_index as u8);
                let new_val = if val { current | mask } else { current & !mask };
                mcu::queue_poke(McuPoke::Ram {
                    addr: *addr,
                    value: new_val,
                });
                break;
            }
        }
    }

    #[qslot]
    fn add_watch(&mut self, name: String, expr: String, type_name: String) {
        let var_type = VarType::from_str_name(&type_name);
        let n = name.trim();
        let e = expr.trim();
        let watch_name = if !n.is_empty() {
            n.to_string()
        } else {
            e.to_string()
        };
        let watch_expr = if !e.is_empty() {
            e.to_string()
        } else {
            watch_name.clone()
        };
        if !watch_name.is_empty() {
            DebugSession::global().add_watch(WatchEntry::new(watch_name, watch_expr, var_type));
            self.rebuild_watches();
        }
    }

    #[qslot]
    fn remove_watch(&mut self, name: String) {
        DebugSession::global().remove_watch(name.trim());
        self.rebuild_watches();
    }

    #[qslot]
    fn clear_watches(&mut self) {
        DebugSession::global().clear_watches();
        self.rebuild_watches();
    }

    #[qslot]
    fn add_all_registers_watch(&mut self) {
        let session = DebugSession::global();
        for (name, _) in &self.snap.registers {
            session.add_watch(WatchEntry::new(name.clone(), name.clone(), VarType::Uint8));
        }
        self.rebuild_watches();
    }

    #[qslot]
    fn poke_ram(&mut self, address: i32, val: i32) {
        if address < 0 {
            return;
        }
        let addr = address as u16;
        let value = val as u8;
        mcu::queue_poke(McuPoke::Ram { addr, value });
        if (addr as usize) < self.snap.ram.len() {
            self.snap.ram[addr as usize] = value;
            self.ram_revision = self.ram_revision.wrapping_add(1);
            self.ram_changed();
            self.rebuild_watches();
        }
    }

    #[qslot]
    fn poke_flash(&mut self, address: i32, val: i32) {
        if address < 0 {
            return;
        }
        mcu::queue_poke(McuPoke::Flash {
            addr: address as u32,
            value: val as u16,
        });
        self.flash_revision = self.flash_revision.wrapping_add(1);
        self.flash_changed();
    }

    #[qslot]
    fn poke_eeprom(&mut self, address: i32, val: i32) {
        if address < 0 {
            return;
        }
        mcu::queue_poke(McuPoke::Eeprom {
            addr: address as u16,
            value: val as u8,
        });
        self.eeprom_revision = self.eeprom_revision.wrapping_add(1);
        self.eeprom_changed();
    }
}
