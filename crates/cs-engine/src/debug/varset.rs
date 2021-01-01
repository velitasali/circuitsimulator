//! Variable Watch and VarSet evaluator: multi-base expression resolution,
//! type casting, and SFR inspection.

use cs_mcu::Device;
use serde::{Deserialize, Serialize};

use super::symbols::DebugSymbols;
use crate::mcu::McuSnap;

/// Supported variable data types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarType {
    Uint8,
    Int8,
    Uint16,
    Int16,
    Uint32,
    Int32,
    Float32,
    Float64,
    Bool,
    Char,
    String { max_len: usize },
    BitField { bit: u8 },
    Custom(String),
}

impl VarType {
    pub fn from_str_name(name: &str) -> Self {
        let clean = name.trim().to_ascii_lowercase();
        match clean.as_str() {
            "uint8" | "uchar" | "byte" => VarType::Uint8,
            "int8" | "char" => VarType::Int8,
            "uint16" | "ushort" | "word" => VarType::Uint16,
            "int16" | "short" | "int" => VarType::Int16,
            "uint32" | "ulong" | "dword" => VarType::Uint32,
            "int32" | "long" => VarType::Int32,
            "float32" | "float" => VarType::Float32,
            "float64" | "double" => VarType::Float64,
            "bool" | "boolean" => VarType::Bool,
            "character" => VarType::Char,
            _ => {
                if let Some(rest) = clean.strip_prefix("string") {
                    let len = rest
                        .trim_matches(|c: char| !c.is_ascii_digit())
                        .parse::<usize>()
                        .unwrap_or(32);
                    VarType::String { max_len: len }
                } else {
                    VarType::Custom(name.to_string())
                }
            }
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            VarType::Uint8 => "uint8",
            VarType::Int8 => "int8",
            VarType::Uint16 => "uint16",
            VarType::Int16 => "int16",
            VarType::Uint32 => "uint32",
            VarType::Int32 => "int32",
            VarType::Float32 => "float32",
            VarType::Float64 => "float64",
            VarType::Bool => "bool",
            VarType::Char => "char",
            VarType::String { .. } => "string",
            VarType::BitField { .. } => "bitfield",
            VarType::Custom(s) => s.as_str(),
        }
    }

    pub fn size_in_bytes(&self) -> usize {
        match self {
            VarType::Uint8
            | VarType::Int8
            | VarType::Bool
            | VarType::Char
            | VarType::BitField { .. } => 1,
            VarType::Uint16 | VarType::Int16 => 2,
            VarType::Uint32 | VarType::Int32 | VarType::Float32 => 4,
            VarType::Float64 => 8,
            VarType::String { max_len } => *max_len,
            VarType::Custom(_) => 1,
        }
    }
}

/// Output display format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VarFormat {
    #[default]
    Auto,
    Dec,
    Hex,
    Bin,
    Char,
    Float,
    String,
}

impl VarFormat {
    pub fn format(&self, bytes: &[u8], var_type: &VarType) -> String {
        VarSet::format_bytes(bytes, var_type, *self, None).formatted
    }
}

/// A variable watch entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchEntry {
    pub name: String,
    pub expr: String,
    pub var_type: VarType,
    pub format: VarFormat,
}

impl WatchEntry {
    pub fn new(name: impl Into<String>, expr: impl Into<String>, var_type: VarType) -> Self {
        Self {
            name: name.into(),
            expr: expr.into(),
            var_type,
            format: VarFormat::Auto,
        }
    }
}

/// Evaluated variable value with raw bytes and formatted string representations.
#[derive(Clone, Debug, PartialEq)]
pub struct VarValue {
    pub name: String,
    pub expr: String,
    pub raw: u64,
    pub formatted: String,
    pub hex: String,
    pub dec: String,
    pub bin: String,
    pub type_name: String,
    pub address: Option<u32>,
}

/// Evaluator for variable watches and expressions.
pub struct VarSet;

impl VarSet {
    /// Evaluate a single watch entry against a running `Device`.
    pub fn eval_device(
        entry: &WatchEntry,
        device: &Device,
        symbols: Option<&DebugSymbols>,
    ) -> Option<VarValue> {
        let (addr, is_reg) = Self::resolve_address(&entry.expr, symbols, Some(device))?;
        if is_reg {
            if let Some(val) = device.read_reg_by_name(&entry.expr) {
                let mut val_res =
                    Self::format_value(u64::from(val), &entry.var_type, entry.format, Some(addr));
                val_res.name = entry.name.clone();
                val_res.expr = entry.expr.clone();
                return Some(val_res);
            }
        }

        let bytes = Self::read_device_bytes(device, addr as u16, entry.var_type.size_in_bytes());
        let mut val_res = Self::format_bytes(&bytes, &entry.var_type, entry.format, Some(addr));
        val_res.name = entry.name.clone();
        val_res.expr = entry.expr.clone();
        Some(val_res)
    }

    /// Evaluate a watch entry against a snapshot `McuSnap`.
    pub fn eval_snap(
        entry: &WatchEntry,
        snap: &McuSnap,
        symbols: Option<&DebugSymbols>,
    ) -> Option<VarValue> {
        let (addr, is_reg) = Self::resolve_address_snap(&entry.expr, symbols, snap)?;
        if is_reg {
            for (name, reg_addr) in &snap.registers {
                if name.eq_ignore_ascii_case(&entry.expr) {
                    let val = snap.ram.get(*reg_addr as usize).copied().unwrap_or(0);
                    let mut val_res = Self::format_value(
                        u64::from(val),
                        &entry.var_type,
                        entry.format,
                        Some(*reg_addr as u32),
                    );
                    val_res.name = entry.name.clone();
                    val_res.expr = entry.expr.clone();
                    return Some(val_res);
                }
            }
        }

        let bytes = Self::read_snap_bytes(snap, addr as usize, entry.var_type.size_in_bytes());
        let mut val_res = Self::format_bytes(&bytes, &entry.var_type, entry.format, Some(addr));
        val_res.name = entry.name.clone();
        val_res.expr = entry.expr.clone();
        Some(val_res)
    }

    fn resolve_address(
        expr: &str,
        symbols: Option<&DebugSymbols>,
        device: Option<&Device>,
    ) -> Option<(u32, bool)> {
        let clean = expr.trim();
        // 1. Direct hex or decimal number address
        if let Some(hex) = clean
            .strip_prefix("0x")
            .or_else(|| clean.strip_prefix("0X"))
        {
            if let Ok(addr) = u32::from_str_radix(hex, 16) {
                return Some((addr, false));
            }
        }
        if let Ok(addr) = clean.parse::<u32>() {
            return Some((addr, false));
        }

        // 2. Check Device registers / SFRs
        if let Some(dev) = device {
            if dev.read_reg_by_name(clean).is_some() {
                return Some((0, true));
            }
        }

        // 3. Check symbol table
        if let Some(syms) = symbols {
            if let Some(v) = syms.variable_named(clean) {
                return Some((v.address, v.is_sfr));
            }
        }

        None
    }

    fn resolve_address_snap(
        expr: &str,
        symbols: Option<&DebugSymbols>,
        snap: &McuSnap,
    ) -> Option<(u32, bool)> {
        let clean = expr.trim();
        if let Some(hex) = clean
            .strip_prefix("0x")
            .or_else(|| clean.strip_prefix("0X"))
        {
            if let Ok(addr) = u32::from_str_radix(hex, 16) {
                return Some((addr, false));
            }
        }
        if let Ok(addr) = clean.parse::<u32>() {
            return Some((addr, false));
        }

        for (name, addr) in &snap.registers {
            if name.eq_ignore_ascii_case(clean) {
                return Some((*addr as u32, true));
            }
        }

        if let Some(syms) = symbols {
            if let Some(v) = syms.variable_named(clean) {
                return Some((v.address, v.is_sfr));
            }
        }

        None
    }

    fn read_device_bytes(device: &Device, addr: u16, len: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(len);
        for i in 0..len {
            buf.push(device.monitor_ram(addr.wrapping_add(i as u16)));
        }
        buf
    }

    fn read_snap_bytes(snap: &McuSnap, addr: usize, len: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(len);
        for i in 0..len {
            buf.push(snap.ram.get(addr + i).copied().unwrap_or(0));
        }
        buf
    }

    fn format_bytes(
        bytes: &[u8],
        var_type: &VarType,
        fmt: VarFormat,
        address: Option<u32>,
    ) -> VarValue {
        let mut raw: u64 = 0;
        for (i, &b) in bytes.iter().take(8).enumerate() {
            raw |= (u64::from(b)) << (i * 8);
        }

        if let VarType::String { max_len } = var_type {
            let limit = bytes.len().min(*max_len);
            let slice = &bytes[..limit];
            let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let str_val = String::from_utf8_lossy(&slice[..end]).to_string();
            return VarValue {
                name: String::new(),
                expr: String::new(),
                raw,
                formatted: format!("\"{}\"", str_val),
                hex: format!("0x{:X}", raw),
                dec: raw.to_string(),
                bin: format!("0b{:b}", raw),
                type_name: var_type.type_name().to_string(),
                address,
            };
        }

        Self::format_value(raw, var_type, fmt, address)
    }

    fn format_value(
        raw: u64,
        var_type: &VarType,
        fmt: VarFormat,
        address: Option<u32>,
    ) -> VarValue {
        let type_name = var_type.type_name().to_string();
        let hex_str = match var_type {
            VarType::Uint8 | VarType::Int8 | VarType::Bool | VarType::Char => {
                format!("0x{:02X}", raw as u8)
            }
            VarType::Uint16 | VarType::Int16 => format!("0x{:04X}", raw as u16),
            VarType::Uint32 | VarType::Int32 | VarType::Float32 => format!("0x{:08X}", raw as u32),
            _ => format!("0x{:X}", raw),
        };

        let dec_str = match var_type {
            VarType::Int8 => (raw as u8 as i8).to_string(),
            VarType::Uint8 => (raw as u8).to_string(),
            VarType::Int16 => (raw as u16 as i16).to_string(),
            VarType::Uint16 => (raw as u16).to_string(),
            VarType::Int32 => (raw as u32 as i32).to_string(),
            VarType::Uint32 => (raw as u32).to_string(),
            VarType::Float32 => {
                let f = f32::from_bits(raw as u32);
                format!("{:.6}", f)
            }
            VarType::Float64 => {
                let f = f64::from_bits(raw);
                format!("{:.6}", f)
            }
            VarType::Bool => if (raw & 1) != 0 { "true" } else { "false" }.to_string(),
            _ => raw.to_string(),
        };

        let bin_str = match var_type {
            VarType::Uint8 | VarType::Int8 | VarType::Bool | VarType::Char => {
                format!("0b{:08b}", raw as u8)
            }
            VarType::Uint16 | VarType::Int16 => format!("0b{:016b}", raw as u16),
            VarType::Uint32 | VarType::Int32 | VarType::Float32 => format!("0b{:032b}", raw as u32),
            _ => format!("0b{:b}", raw),
        };

        let formatted = match fmt {
            VarFormat::Hex => hex_str.clone(),
            VarFormat::Dec => dec_str.clone(),
            VarFormat::Bin => bin_str.clone(),
            VarFormat::Char => {
                let c = raw as u8 as char;
                if c.is_ascii_graphic() || c == ' ' {
                    format!("'{}'", c)
                } else {
                    format!("'\\x{:02X}'", raw as u8)
                }
            }
            VarFormat::Float => match var_type {
                VarType::Float32 => format!("{:.6}", f32::from_bits(raw as u32)),
                VarType::Float64 => format!("{:.6}", f64::from_bits(raw)),
                _ => dec_str.clone(),
            },
            VarFormat::Auto | VarFormat::String => match var_type {
                VarType::Char => {
                    let c = raw as u8 as char;
                    if c.is_ascii_graphic() || c == ' ' {
                        format!("'{}' ({})", c, raw as u8)
                    } else {
                        format!("0x{:02X}", raw as u8)
                    }
                }
                VarType::Bool => {
                    if (raw & 1) != 0 {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    }
                }
                VarType::Float32 => format!("{:.6}", f32::from_bits(raw as u32)),
                VarType::Float64 => format!("{:.6}", f64::from_bits(raw)),
                _ => format!("{} ({})", dec_str, hex_str),
            },
        };

        VarValue {
            name: String::new(),
            expr: String::new(),
            raw,
            formatted,
            hex: hex_str,
            dec: dec_str,
            bin: bin_str,
            type_name,
            address,
        }
    }
}
