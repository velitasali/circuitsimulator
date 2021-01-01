//! Memory Table dialog façade matching C++ `MemTable` / `MemTableModel`.

use std::path::Path;

use crate::path_util::strip_file_url;
use cs_engine::memdata;
use qtbridge::qobject;
use serde_json::{Value, json};

pub struct MemoryTable {
    active_uid: String,
    title: String,
    is_rom: bool,
    cell_bytes: i32,
    word_bytes: i32,
    selected_address: i32,
    data: Vec<u8>,
    revision: i32,
    visible: bool,
}

impl Default for MemoryTable {
    fn default() -> Self {
        Self {
            active_uid: String::new(),
            title: cs_engine::i18n::tr("Memory Table"),
            is_rom: false,
            cell_bytes: 1,
            word_bytes: 1,
            selected_address: -1,
            data: Vec::new(),
            revision: 0,
            visible: false,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl MemoryTable {
    qproperty!("title", Read = title, Notify = title_changed);
    qproperty!(
        "visible",
        Read = is_visible,
        Write = set_visible,
        Notify = visible_changed
    );
    qproperty!("activeUid", Read = active_uid, Notify = active_changed);
    qproperty!("isRom", Read = is_rom, Notify = config_changed);
    qproperty!("cellBytes", Read = cell_bytes, Notify = config_changed);
    qproperty!("wordBytes", Read = word_bytes, Notify = config_changed);
    qproperty!(
        "selectedAddress",
        Read = selected_address,
        Write = set_selected_address,
        Notify = selection_changed
    );
    qproperty!("canSaveLoad", Read = can_save_load, Constant);
    qproperty!("rowCount", Read = row_count, Notify = data_changed);
    qproperty!("cellCount", Read = cell_count, Notify = data_changed);
    qproperty!("revision", Read = revision, Notify = data_changed);

    #[qsignal]
    fn title_changed(&mut self);
    #[qsignal]
    fn visible_changed(&mut self);
    #[qsignal]
    fn active_changed(&mut self);
    #[qsignal]
    fn config_changed(&mut self);
    #[qsignal]
    fn selection_changed(&mut self);
    #[qsignal]
    fn data_changed(&mut self);
    #[qsignal]
    fn scroll_to(&mut self, row: i32);

    fn title(&self) -> String {
        self.title.clone()
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn active_uid(&self) -> String {
        self.active_uid.clone()
    }

    fn is_rom(&self) -> bool {
        self.is_rom
    }

    fn cell_bytes(&self) -> i32 {
        self.cell_bytes
    }

    fn word_bytes(&self) -> i32 {
        self.word_bytes
    }

    fn selected_address(&self) -> i32 {
        self.selected_address
    }

    fn set_selected_address(&mut self, addr: i32) {
        if self.selected_address != addr {
            self.selected_address = addr;
            self.selection_changed();
        }
    }

    fn can_save_load(&self) -> bool {
        true
    }

    fn row_count(&self) -> i32 {
        if self.data.is_empty() {
            0
        } else {
            ((self.data.len() + 15) / 16) as i32
        }
    }

    fn cell_count(&self) -> i32 {
        self.data.len() as i32
    }

    fn revision(&self) -> i32 {
        self.revision
    }

    fn set_visible(&mut self, v: bool) {
        if self.visible != v {
            self.visible = v;
            self.visible_changed();
        }
    }

    #[qslot]
    fn open_for_item(
        &mut self,
        uid: String,
        title: String,
        is_rom: bool,
        cell_bytes: i32,
        data: Value,
    ) {
        self.active_uid = uid;
        self.title = if title.is_empty() {
            if is_rom {
                format!("{}: {}", cs_engine::i18n::tr("ROM"), self.active_uid)
            } else {
                format!("{}: {}", cs_engine::i18n::tr("RAM"), self.active_uid)
            }
        } else {
            title
        };
        self.is_rom = is_rom;
        self.cell_bytes = cell_bytes.max(1);
        self.word_bytes = cell_bytes.max(1);
        self.selected_address = -1;

        if let Value::Array(arr) = data {
            self.data = arr
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
        }
        if self.data.is_empty() {
            self.data = vec![0u8; 256];
        }

        self.visible = true;
        self.revision = self.revision.wrapping_add(1);

        self.title_changed();
        self.active_changed();
        self.config_changed();
        self.selection_changed();
        self.data_changed();
        self.visible_changed();
    }

    #[qslot]
    fn set_data_bytes(&mut self, data: Value) {
        if let Value::Array(arr) = data {
            let bytes: Vec<u8> = arr
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            if !bytes.is_empty() && (self.data.len() != bytes.len() || self.data != bytes) {
                self.data = bytes;
                self.revision = self.revision.wrapping_add(1);
                self.data_changed();
            }
        }
    }

    #[qslot]
    fn row_address(&self, row: i32) -> String {
        let addr = (row.max(0) as usize) * 16;
        let addr_digits = if self.data.len() > 0xFFFF { 6 } else { 4 };
        format!("0x{:0width$X}", addr, width = addr_digits)
    }

    #[qslot]
    fn row_data(&self, row_idx: i32, _rev: i32) -> Value {
        let row_addr = (row_idx.max(0) as usize) * 16;
        let addr_digits = if self.data.len() > 0xFFFF { 6 } else { 4 };

        let mut bytes_json = Vec::with_capacity(16);
        let mut ascii_json = Vec::with_capacity(16);
        let mut vals_json = Vec::with_capacity(16);
        let mut addrs_json = Vec::with_capacity(16);

        for col in 0..16 {
            let addr = row_addr + col;
            if addr < self.data.len() {
                let b = self.data[addr];
                bytes_json.push(json!(format!("{b:02X}")));
                vals_json.push(json!(b));
                addrs_json.push(json!(addr));

                let c = b as char;
                if c.is_ascii_graphic() || c == ' ' {
                    ascii_json.push(json!(c.to_string()));
                } else {
                    ascii_json.push(json!("."));
                }
            } else {
                bytes_json.push(json!("--"));
                ascii_json.push(json!(" "));
                vals_json.push(json!(-1));
                addrs_json.push(json!(-1));
            }
        }

        json!({
            "address": format!("0x{:0width$X}", row_addr, width = addr_digits),
            "bytes": bytes_json,
            "ascii": ascii_json,
            "vals": vals_json,
            "addrs": addrs_json,
        })
    }

    #[qslot]
    fn cell_clicked(&mut self, row: i32, col: i32) {
        if col == 16 {
            return;
        }
        let byte_col = if col > 16 { col - 17 } else { col };
        let addr = row * 16 + byte_col;
        if addr >= 0 && (addr as usize) < self.data.len() {
            self.selected_address = addr;
            self.selection_changed();
        }
    }

    #[qslot]
    fn cell_edited(&mut self, row: i32, col: i32, text: String) -> i32 {
        if col == 16 {
            return -1;
        }
        let byte_col = if col > 16 { col - 17 } else { col };
        let addr = (row * 16 + byte_col) as usize;
        if addr >= self.data.len() {
            return -1;
        }

        let new_val: Option<u8> = if col > 16 {
            if text.is_empty() {
                None
            } else {
                Some(text.chars().next().unwrap() as u8)
            }
        } else {
            u8::from_str_radix(text.trim(), 16).ok()
        };

        if let Some(val) = new_val {
            self.data[addr] = val;
            self.selected_address = addr as i32;
            self.revision = self.revision.wrapping_add(1);
            self.data_changed();
            self.selection_changed();
            val as i32
        } else {
            -1
        }
    }

    #[qslot]
    fn hover_text(&self, row: i32, col: i32) -> String {
        if col == 16 {
            return String::new();
        }
        let byte_col = if col > 16 { col - 17 } else { col };
        let addr = (row * 16 + byte_col) as usize;
        if addr >= self.data.len() {
            return String::new();
        }

        let val = self.data[addr];
        let addr_digits = if self.data.len() > 0xFFFF { 6 } else { 4 };
        let addr_hex = format!("{:0width$X}", addr, width = addr_digits);
        format!("Addr: 0x{addr_hex} ({addr})\nDec: {val:>3}\nOct: 0o{val:03o}\nBin: 0b{val:08b}")
    }

    #[qslot]
    fn save_to_file(&self, url: String) -> bool {
        let path_str = strip_file_url(&url);
        if path_str.is_empty() {
            return false;
        }
        let path = Path::new(&path_str);
        memdata::save_bytes(path, &self.data).is_ok()
    }

    #[qslot]
    fn load_from_file(&mut self, url: String) -> bool {
        let path_str = strip_file_url(&url);
        if path_str.is_empty() {
            return false;
        }
        let path = Path::new(&path_str);
        if memdata::load_bytes(path, &mut self.data).is_ok() {
            self.revision = self.revision.wrapping_add(1);
            self.data_changed();
            true
        } else {
            false
        }
    }
}
