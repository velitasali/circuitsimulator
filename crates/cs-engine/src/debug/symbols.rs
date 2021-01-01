//! MCU debug symbols: ELF / DWARF and `.lst` list file mapping for flash addresses,
//! source code lines, functions, and variables.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Source code location corresponding to a flash program address.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(file: impl Into<PathBuf>, line: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column: 0,
        }
    }

    pub fn with_col(file: impl Into<PathBuf>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

/// Function symbol with entry address and optional end address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSymbol {
    pub name: String,
    pub start_pc: u32,
    pub end_pc: Option<u32>,
}

/// Variable symbol with name, type, and memory address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugVariable {
    pub name: String,
    pub var_type: String,
    pub address: u32,
    pub size: usize,
    pub is_sfr: bool,
}

/// Debug symbols table holding flash-to-source, source-to-flash, and symbol mappings.
#[derive(Clone, Debug, Default)]
pub struct DebugSymbols {
    /// Flash PC (byte/word address) -> Source file and line number.
    pub flash_to_source: BTreeMap<u32, SourceLocation>,
    /// Normalized source file + 1-based line -> matching flash PC addresses.
    pub source_to_flash: BTreeMap<(PathBuf, usize), Vec<u32>>,
    /// Function name -> function symbol.
    pub functions: BTreeMap<String, FunctionSymbol>,
    /// Function start PC -> function name.
    pub pc_to_function: BTreeMap<u32, String>,
    /// Variables found in symbol tables or list files.
    pub variables: Vec<DebugVariable>,
    /// Variable name -> index in `variables`.
    pub var_lookup: BTreeMap<String, usize>,
}

impl DebugSymbols {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_line(&mut self, pc: u32, loc: SourceLocation) {
        let key = (loc.file.clone(), loc.line);
        self.source_to_flash.entry(key).or_default().push(pc);
        self.flash_to_source.insert(pc, loc);
    }

    pub fn insert_function(&mut self, name: impl Into<String>, start_pc: u32, end_pc: Option<u32>) {
        let name = name.into();
        self.pc_to_function.insert(start_pc, name.clone());
        self.functions.insert(
            name.clone(),
            FunctionSymbol {
                name,
                start_pc,
                end_pc,
            },
        );
    }

    pub fn insert_variable(
        &mut self,
        name: impl Into<String>,
        var_type: impl Into<String>,
        address: u32,
        size: usize,
        is_sfr: bool,
    ) {
        let name = name.into();
        let idx = self.variables.len();
        self.variables.push(DebugVariable {
            name: name.clone(),
            var_type: var_type.into(),
            address,
            size,
            is_sfr,
        });
        self.var_lookup.insert(name, idx);
    }

    /// Lookup source location for a given program counter.
    pub fn lookup_address(&self, pc: u32) -> Option<&SourceLocation> {
        self.flash_to_source.get(&pc)
    }

    /// Lookup all flash addresses mapped to a given source line.
    pub fn lookup_lines(&self, file: &Path, line: usize) -> &[u32] {
        let key = (file.to_path_buf(), line);
        if let Some(list) = self.source_to_flash.get(&key) {
            return list;
        }
        // Try matching by file name (basename) if full path lookup fails.
        if let Some(name) = file.file_name() {
            for ((path, l), addrs) in &self.source_to_flash {
                if *l == line && path.file_name() == Some(name) {
                    return addrs;
                }
            }
        }
        &[]
    }

    /// Return the function name that starts at or contains `pc`.
    pub fn function_at(&self, pc: u32) -> Option<&str> {
        if let Some(name) = self.pc_to_function.get(&pc) {
            return Some(name.as_str());
        }
        for fn_sym in self.functions.values() {
            if fn_sym.start_pc <= pc {
                if let Some(end) = fn_sym.end_pc {
                    if pc <= end {
                        return Some(fn_sym.name.as_str());
                    }
                }
            }
        }
        None
    }

    /// Find a variable by name.
    pub fn variable_named(&self, name: &str) -> Option<&DebugVariable> {
        self.var_lookup
            .get(name)
            .and_then(|&idx| self.variables.get(idx))
    }

    pub fn is_empty(&self) -> bool {
        self.flash_to_source.is_empty() && self.functions.is_empty() && self.variables.is_empty()
    }

    /// Merge symbols from another `DebugSymbols` instance into `self`.
    pub fn merge(&mut self, other: DebugSymbols) {
        for (pc, loc) in other.flash_to_source {
            self.flash_to_source.insert(pc, loc);
        }
        for (k, v) in other.source_to_flash {
            self.source_to_flash.entry(k).or_default().extend(v);
        }
        for (name, sym) in other.functions {
            self.pc_to_function.insert(sym.start_pc, name.clone());
            self.functions.insert(name, sym);
        }
        for v in other.variables {
            if !self.var_lookup.contains_key(&v.name) {
                let idx = self.variables.len();
                self.var_lookup.insert(v.name.clone(), idx);
                self.variables.push(v);
            }
        }
    }
}

/// Syntax style of `.lst` list file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LstKind {
    /// GPASM list format (PIC). No colon, hex address at column 0.
    Gpasm,
    /// AVRA / GAVRASM list format (AVR).
    Avra,
    /// SDCC list format (8051 / Z80).
    Sdcc,
    /// CA65 list format (6502).
    Ca65,
    /// Z80ASM list format (Z80).
    Z80asm,
    /// Microchip XC8 list format.
    Xc8,
    /// Auto-detect list format based on content.
    Auto,
}

/// Parser for `.lst`, `.sym`, `.map`, and assembly list files.
pub struct LstParser;

impl LstParser {
    pub fn parse(content: &str, src_file: &Path, kind: LstKind) -> DebugSymbols {
        let mut syms = DebugSymbols::new();
        let detected_kind = if kind == LstKind::Auto {
            Self::detect_kind(content)
        } else {
            kind
        };

        match detected_kind {
            LstKind::Gpasm | LstKind::Xc8 => Self::parse_gpasm(content, src_file, &mut syms),
            LstKind::Avra => Self::parse_avra(content, src_file, &mut syms),
            LstKind::Sdcc => Self::parse_sdcc(content, src_file, &mut syms),
            LstKind::Ca65 => Self::parse_ca65(content, src_file, &mut syms),
            LstKind::Z80asm => Self::parse_z80asm(content, src_file, &mut syms),
            LstKind::Auto => Self::parse_generic(content, src_file, &mut syms),
        }

        // Also extract any EQU / .equ / symbol definitions throughout the file
        Self::parse_symbols_and_equates(content, &mut syms);

        syms
    }

    fn detect_kind(content: &str) -> LstKind {
        if content.contains("GPASM") || content.contains("gpasm") || content.contains("MPASM") {
            LstKind::Gpasm
        } else if content.contains("AVRA") || content.contains("gavrasm") {
            LstKind::Avra
        } else if content.contains("SDCC") || content.contains(".area") {
            LstKind::Sdcc
        } else if content.contains("ca65") || content.contains("CA65") {
            LstKind::Ca65
        } else if content.contains("z80asm") || content.contains("Z80ASM") {
            LstKind::Z80asm
        } else if content.contains("Microchip") || content.contains("XC8") {
            LstKind::Xc8
        } else {
            LstKind::Auto
        }
    }

    /// Parse GPASM list format.
    /// Lines typically look like: `000000 3000   movlw 0` or `0000   3000   00003  MOVLW  0x00`
    fn parse_gpasm(content: &str, src_file: &Path, syms: &mut DebugSymbols) {
        let mut line_no = 1;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                line_no += 1;
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                // First part is hex address (e.g. 000000 or 0004)
                if let Ok(addr) = u32::from_str_radix(parts[0], 16) {
                    // Check if second part is hex opcode
                    if u32::from_str_radix(parts[1], 16).is_ok() {
                        let mut src_line = line_no;
                        if parts.len() >= 3 {
                            if let Ok(l) = parts[2].parse::<usize>() {
                                src_line = l;
                            }
                        }
                        syms.insert_line(addr, SourceLocation::new(src_file, src_line));
                        // Check for label / function definition
                        if parts.len() >= 4 && parts[3].ends_with(':') {
                            let label = parts[3].trim_end_matches(':');
                            syms.insert_function(label, addr, None);
                        } else if parts.len() >= 3 && parts[2].ends_with(':') {
                            let label = parts[2].trim_end_matches(':');
                            syms.insert_function(label, addr, None);
                        }
                    }
                }
            }
            line_no += 1;
        }
    }

    /// Parse AVRA / GAVRASM list format.
    /// Lines often look like: `000000 c001 rjmp reset` or `000004 9508: ret` or `12: 000002 e001 ldi r16, 1`
    fn parse_avra(content: &str, src_file: &Path, syms: &mut DebugSymbols) {
        let mut default_line = 1;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                default_line += 1;
                continue;
            }

            let mut working_line = trimmed;
            let mut src_line = default_line;

            // Check for line number prefix "12: 000002 ..."
            if let Some((l_str, rest)) = working_line.split_once(':') {
                if let Ok(num) = l_str.trim().parse::<usize>() {
                    src_line = num;
                    working_line = rest.trim();
                }
            }

            let parts: Vec<&str> = working_line.split_whitespace().collect();
            if parts.len() >= 2 {
                let addr_str = parts[0].trim_end_matches(':');
                if let Ok(addr) = u32::from_str_radix(addr_str, 16) {
                    if u32::from_str_radix(parts[1].trim_end_matches(':'), 16).is_ok() {
                        syms.insert_line(addr, SourceLocation::new(src_file, src_line));
                        if parts.len() >= 3 && parts[2].ends_with(':') {
                            let label = parts[2].trim_end_matches(':');
                            syms.insert_function(label, addr, None);
                        }
                    }
                }
            }
            default_line += 1;
        }
    }

    /// Parse SDCC list format.
    fn parse_sdcc(content: &str, src_file: &Path, syms: &mut DebugSymbols) {
        let mut line_no = 1;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                line_no += 1;
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(addr) = u32::from_str_radix(parts[0], 16) {
                    syms.insert_line(addr, SourceLocation::new(src_file, line_no));
                }
            }
            line_no += 1;
        }
    }

    /// Parse CA65 list format.
    fn parse_ca65(content: &str, src_file: &Path, syms: &mut DebugSymbols) {
        let mut line_no = 1;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                line_no += 1;
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let addr_str = parts[0].trim_end_matches('r').trim_end_matches(':');
                if let Ok(addr) = u32::from_str_radix(addr_str, 16) {
                    let mut opcode_idx = 1;
                    if parts.len() > 2
                        && u32::from_str_radix(parts[1], 16).is_err()
                        && parts[1].parse::<usize>().is_ok()
                    {
                        opcode_idx = 2;
                    }
                    if parts.len() > opcode_idx
                        && u32::from_str_radix(parts[opcode_idx], 16).is_ok()
                    {
                        syms.insert_line(addr, SourceLocation::new(src_file, line_no));
                    }
                }
            }
            line_no += 1;
        }
    }

    /// Parse Z80ASM list format.
    fn parse_z80asm(content: &str, src_file: &Path, syms: &mut DebugSymbols) {
        let mut line_no = 1;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                line_no += 1;
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(addr) = u32::from_str_radix(parts[0], 16) {
                    if u32::from_str_radix(parts[1], 16).is_ok() {
                        syms.insert_line(addr, SourceLocation::new(src_file, line_no));
                    }
                }
            }
            line_no += 1;
        }
    }

    /// Generic fallback parser for addresses and lines.
    fn parse_generic(content: &str, src_file: &Path, syms: &mut DebugSymbols) {
        let mut line_no = 1;
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let clean_addr = parts[0].trim_matches(|c: char| !c.is_ascii_hexdigit());
                if !clean_addr.is_empty() {
                    if let Ok(addr) = u32::from_str_radix(clean_addr, 16) {
                        syms.insert_line(addr, SourceLocation::new(src_file, line_no));
                    }
                }
            }
            line_no += 1;
        }
    }

    /// Parse EQU directives, `.equ`, `.def`, `cblock`, symbol dumps, and `.map` files into variables.
    fn parse_symbols_and_equates(content: &str, syms: &mut DebugSymbols) {
        let mut cblock_addr: Option<u32> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // cblock handling (PIC)
            if let Some(rest) = trimmed
                .strip_prefix("cblock")
                .or_else(|| trimmed.strip_prefix("CBLOCK"))
            {
                let arg = rest.trim().split_whitespace().next().unwrap_or("0");
                let clean = arg
                    .trim_start_matches("0x")
                    .trim_start_matches("0X")
                    .trim_end_matches('h')
                    .trim_end_matches('H');
                cblock_addr = u32::from_str_radix(clean, 16)
                    .ok()
                    .or_else(|| clean.parse::<u32>().ok());
                continue;
            }
            if trimmed.eq_ignore_ascii_case("endc") {
                cblock_addr = None;
                continue;
            }
            if let Some(curr) = cblock_addr {
                let name = trimmed
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(':');
                if !name.is_empty() && !name.starts_with(';') {
                    syms.insert_variable(name, "uint8", curr, 1, false);
                    cblock_addr = Some(curr + 1);
                    continue;
                }
            }

            // Ignore comments unless symbol section
            let line_no_comment = if let Some((code, _)) = trimmed.split_once(';') {
                code.trim()
            } else {
                trimmed
            };
            if line_no_comment.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line_no_comment.split_whitespace().collect();

            // Format 1: NAME EQU <value> / NAME equ <value> / NAME = <value>
            if parts.len() >= 3 {
                let op = parts[1];
                if op.eq_ignore_ascii_case("EQU")
                    || op.eq_ignore_ascii_case(".EQU")
                    || op.eq_ignore_ascii_case(".SET")
                    || op == "="
                    || op == "=="
                {
                    let name = parts[0].trim_end_matches(':');
                    let val_str = parts[2]
                        .trim_start_matches("0x")
                        .trim_start_matches("0X")
                        .trim_end_matches('h')
                        .trim_end_matches('H')
                        .trim_matches('$');
                    if let Ok(addr) = u32::from_str_radix(val_str, 16) {
                        syms.insert_variable(name, "uint8", addr, 1, false);
                        continue;
                    } else if let Ok(addr) = val_str.parse::<u32>() {
                        syms.insert_variable(name, "uint8", addr, 1, false);
                        continue;
                    }
                }
            }

            // Format 2: .equ NAME, <value> / .set NAME, <value> / .def NAME = <reg>
            if parts.len() >= 2
                && (parts[0].eq_ignore_ascii_case(".equ")
                    || parts[0].eq_ignore_ascii_case(".set")
                    || parts[0].eq_ignore_ascii_case(".def"))
            {
                let rest = line_no_comment[parts[0].len()..].trim();
                if let Some((name_part, val_part)) =
                    rest.split_once(',').or_else(|| rest.split_once('='))
                {
                    let name = name_part.trim();
                    let val_str = val_part
                        .trim()
                        .trim_start_matches("0x")
                        .trim_start_matches("0X")
                        .trim_end_matches('h')
                        .trim_end_matches('H')
                        .trim_matches('$');
                    if let Ok(addr) = u32::from_str_radix(val_str, 16) {
                        syms.insert_variable(name, "uint8", addr, 1, false);
                        continue;
                    } else if let Ok(addr) = val_str.parse::<u32>() {
                        syms.insert_variable(name, "uint8", addr, 1, false);
                        continue;
                    }
                }
            }

            // Format 3: GNU nm / objdump -t output:
            // 00800100 g     O .data	00000002 counter
            // 00800102 g     O .bss	00000001 state
            // 00000020 g       0x20 my_var
            if parts.len() >= 4 {
                if let Ok(addr) = u32::from_str_radix(parts[0], 16) {
                    let name = parts[parts.len() - 1];
                    if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        // In AVR-GCC, RAM addresses in symbol tables start at 0x00800000
                        let ram_addr = if addr >= 0x00800000 && addr <= 0x0080FFFF {
                            addr - 0x00800000
                        } else {
                            addr
                        };
                        syms.insert_variable(name, "uint8", ram_addr, 1, false);
                    }
                }
            }
        }
    }
}

/// Parser for ELF 32/64-bit binaries and DWARF symbol tables.
pub struct ElfParser;

impl ElfParser {
    /// Parse ELF binary bytes and extract symbol table and line mappings.
    pub fn parse(bytes: &[u8]) -> Option<DebugSymbols> {
        if bytes.len() < 52 {
            return None;
        }
        // Check ELF magic \x7fELF
        if &bytes[0..4] != b"\x7fELF" {
            return None;
        }

        let is_64 = bytes[4] == 2;
        let is_le = bytes[5] == 1;

        let mut syms = DebugSymbols::new();

        let read_u16 = |offset: usize| -> u16 {
            if offset + 2 > bytes.len() {
                return 0;
            }
            if is_le {
                u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
            } else {
                u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
            }
        };

        let read_u32 = |offset: usize| -> u32 {
            if offset + 4 > bytes.len() {
                return 0;
            }
            if is_le {
                u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ])
            } else {
                u32::from_be_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ])
            }
        };

        let read_u64 = |offset: usize| -> u64 {
            if is_64 {
                if offset + 8 > bytes.len() {
                    return 0;
                }
                if is_le {
                    u64::from_le_bytes([
                        bytes[offset],
                        bytes[offset + 1],
                        bytes[offset + 2],
                        bytes[offset + 3],
                        bytes[offset + 4],
                        bytes[offset + 5],
                        bytes[offset + 6],
                        bytes[offset + 7],
                    ])
                } else {
                    u64::from_be_bytes([
                        bytes[offset],
                        bytes[offset + 1],
                        bytes[offset + 2],
                        bytes[offset + 3],
                        bytes[offset + 4],
                        bytes[offset + 5],
                        bytes[offset + 6],
                        bytes[offset + 7],
                    ])
                }
            } else {
                u64::from(read_u32(offset))
            }
        };

        // Section header table offset
        let shoff = if is_64 {
            read_u64(40) as usize
        } else {
            read_u32(32) as usize
        };
        let shentsize = if is_64 {
            read_u16(58) as usize
        } else {
            read_u16(46) as usize
        };
        let shnum = if is_64 {
            read_u16(60) as usize
        } else {
            read_u16(48) as usize
        };
        let shstrndx = if is_64 {
            read_u16(62) as usize
        } else {
            read_u16(50) as usize
        };

        if shoff == 0 || shentsize == 0 || shnum == 0 || shstrndx >= shnum {
            return Some(syms);
        }

        // Section names string table
        let str_sh_offset = shoff + shstrndx * shentsize;
        let str_offset = if is_64 {
            read_u64(str_sh_offset + 24) as usize
        } else {
            read_u32(str_sh_offset + 16) as usize
        };

        let get_section_name = |name_offset: usize| -> &str {
            if str_offset + name_offset >= bytes.len() {
                return "";
            }
            let slice = &bytes[str_offset + name_offset..];
            let len = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            std::str::from_utf8(&slice[..len]).unwrap_or("")
        };

        // Find .symtab, .strtab, and .debug_line sections
        let mut symtab_offset = 0;
        let mut symtab_size = 0;
        let mut symtab_entsize = if is_64 { 24 } else { 16 };
        let mut symstr_offset = 0;
        let mut debug_line_offset = 0;
        let mut debug_line_size = 0;

        for i in 0..shnum {
            let offset = shoff + i * shentsize;
            let sh_name = read_u32(offset) as usize;
            let name = get_section_name(sh_name);
            let sh_offset = if is_64 {
                read_u64(offset + 24) as usize
            } else {
                read_u32(offset + 16) as usize
            };
            let sh_size = if is_64 {
                read_u64(offset + 32) as usize
            } else {
                read_u32(offset + 20) as usize
            };
            let sh_entsize = if is_64 {
                read_u64(offset + 56) as usize
            } else {
                read_u32(offset + 36) as usize
            };

            if name == ".symtab" {
                symtab_offset = sh_offset;
                symtab_size = sh_size;
                if sh_entsize > 0 {
                    symtab_entsize = sh_entsize;
                }
            } else if name == ".strtab" {
                symstr_offset = sh_offset;
            } else if name == ".debug_line" {
                debug_line_offset = sh_offset;
                debug_line_size = sh_size;
            }
        }

        // Parse Symbol Table (.symtab)
        if symtab_offset > 0 && symstr_offset > 0 && symtab_size > 0 {
            let get_sym_name = |name_offset: usize| -> &str {
                if symstr_offset + name_offset >= bytes.len() {
                    return "";
                }
                let slice = &bytes[symstr_offset + name_offset..];
                let len = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
                std::str::from_utf8(&slice[..len]).unwrap_or("")
            };

            let count = symtab_size / symtab_entsize;
            for i in 0..count {
                let ent_offset = symtab_offset + i * symtab_entsize;
                let (st_name, st_value, st_size, st_info) = if is_64 {
                    let name = read_u32(ent_offset) as usize;
                    let info = bytes[ent_offset + 4];
                    let val = read_u64(ent_offset + 8) as u32;
                    let sz = read_u64(ent_offset + 16) as usize;
                    (name, val, sz, info)
                } else {
                    let name = read_u32(ent_offset) as usize;
                    let val = read_u32(ent_offset + 4);
                    let sz = read_u32(ent_offset + 8) as usize;
                    let info = bytes[ent_offset + 12];
                    (name, val, sz, info)
                };

                let sym_type = st_info & 0x0F;
                let sym_name = get_sym_name(st_name);

                if !sym_name.is_empty()
                    && !sym_name.starts_with('.')
                    && !sym_name.starts_with("__")
                    && !sym_name.starts_with("_vector")
                    && sym_name != "_edata"
                    && sym_name != "_etext"
                    && sym_name != "_end"
                {
                    // STT_FUNC = 2
                    if sym_type == 2 {
                        let end_pc = if st_size > 0 {
                            Some(st_value.saturating_add(st_size as u32).saturating_sub(1))
                        } else {
                            None
                        };
                        syms.insert_function(sym_name, st_value, end_pc);
                    }
                    // STT_OBJECT = 1 (Variables in .data/.bss) or STT_NOTYPE in RAM range
                    else if sym_type == 1
                        || (sym_type == 0 && st_value >= 0x00800000 && st_value <= 0x0080FFFF)
                    {
                        let (ram_addr, type_str) =
                            if st_value >= 0x00800000 && st_value <= 0x0080FFFF {
                                (
                                    st_value - 0x00800000,
                                    if st_size == 2 {
                                        "uint16"
                                    } else if st_size == 4 {
                                        "uint32"
                                    } else {
                                        "uint8"
                                    },
                                )
                            } else {
                                (
                                    st_value,
                                    if st_size == 2 {
                                        "uint16"
                                    } else if st_size == 4 {
                                        "uint32"
                                    } else {
                                        "uint8"
                                    },
                                )
                            };
                        let sz = if st_size == 0 { 1 } else { st_size };
                        syms.insert_variable(sym_name, type_str, ram_addr, sz, false);
                    }
                }
            }
        }

        // Parse DWARF .debug_line if present
        if debug_line_offset > 0
            && debug_line_size > 0
            && debug_line_offset + debug_line_size <= bytes.len()
        {
            Self::parse_dwarf_lines(
                &bytes[debug_line_offset..debug_line_offset + debug_line_size],
                &mut syms,
            );
        }

        Some(syms)
    }

    /// Basic DWARF `.debug_line` line program parser.
    fn parse_dwarf_lines(dwarf: &[u8], syms: &mut DebugSymbols) {
        if dwarf.len() < 30 {
            return;
        }
        let mut pos = 0;
        while pos + 10 < dwarf.len() {
            let unit_len =
                u32::from_le_bytes([dwarf[pos], dwarf[pos + 1], dwarf[pos + 2], dwarf[pos + 3]])
                    as usize;
            if unit_len == 0 || pos + 4 + unit_len > dwarf.len() {
                break;
            }
            let _version = u16::from_le_bytes([dwarf[pos + 4], dwarf[pos + 5]]);
            let header_len = u32::from_le_bytes([
                dwarf[pos + 6],
                dwarf[pos + 7],
                dwarf[pos + 8],
                dwarf[pos + 9],
            ]) as usize;

            let min_inst_len = dwarf[pos + 10] as u32;
            let _default_is_stmt = dwarf[pos + 11] != 0;
            let line_base = dwarf[pos + 12] as i8 as i32;
            let line_range = dwarf[pos + 13] as u32;
            let opcode_base = dwarf[pos + 14];

            let program_start = pos + 10 + header_len;
            let program_end = pos + 4 + unit_len;

            // Extract file names from header
            let mut file_names: Vec<PathBuf> = vec![PathBuf::from("main.c")];
            let mut search_pos = pos + 15 + (opcode_base.saturating_sub(1) as usize);
            // Skip include directories
            while search_pos < program_start && dwarf[search_pos] != 0 {
                while search_pos < program_start && dwarf[search_pos] != 0 {
                    search_pos += 1;
                }
                search_pos += 1;
            }
            search_pos += 1;

            // Read file entries
            while search_pos < program_start && dwarf[search_pos] != 0 {
                let start_f = search_pos;
                while search_pos < program_start && dwarf[search_pos] != 0 {
                    search_pos += 1;
                }
                if let Ok(name) = std::str::from_utf8(&dwarf[start_f..search_pos]) {
                    file_names.push(PathBuf::from(name));
                }
                search_pos += 1;
                // Skip dir index, time, length (ULEB128)
                for _ in 0..3 {
                    while search_pos < program_start && (dwarf[search_pos] & 0x80) != 0 {
                        search_pos += 1;
                    }
                    search_pos += 1;
                }
            }

            // Run line number state machine
            let mut address: u32 = 0;
            let mut file_idx: usize = 1;
            let mut line: u32 = 1;
            let col: u32 = 0;

            let mut ip = program_start;
            while ip < program_end {
                let op = dwarf[ip];
                ip += 1;

                if op == 0 {
                    // Extended opcode
                    if ip >= program_end {
                        break;
                    }
                    let ext_len = dwarf[ip] as usize;
                    ip += 1;
                    if ip >= program_end {
                        break;
                    }
                    let ext_op = dwarf[ip];
                    ip += 1;
                    if ext_op == 1 {
                        // DW_LNE_end_sequence
                        address = 0;
                        line = 1;
                    } else if ext_op == 2 && ext_len >= 5 {
                        // DW_LNE_set_address
                        if ip + 4 <= program_end {
                            address = u32::from_le_bytes([
                                dwarf[ip],
                                dwarf[ip + 1],
                                dwarf[ip + 2],
                                dwarf[ip + 3],
                            ]);
                            ip += 4;
                        }
                    } else {
                        ip += ext_len.saturating_sub(1);
                    }
                } else if op < opcode_base {
                    // Standard opcode
                    match op {
                        1 => {
                            // DW_LNS_copy
                            let file = file_names
                                .get(file_idx)
                                .cloned()
                                .unwrap_or_else(|| PathBuf::from("main.c"));
                            syms.insert_line(
                                address,
                                SourceLocation::with_col(file, line as usize, col as usize),
                            );
                        }
                        2 => {
                            // DW_LNS_advance_pc
                            if ip < program_end {
                                let mut uleb = 0u32;
                                let mut shift = 0;
                                loop {
                                    if ip >= program_end {
                                        break;
                                    }
                                    let b = dwarf[ip];
                                    ip += 1;
                                    uleb |= (u32::from(b & 0x7F)) << shift;
                                    if (b & 0x80) == 0 {
                                        break;
                                    }
                                    shift += 7;
                                }
                                address = address.wrapping_add(uleb.wrapping_mul(min_inst_len));
                            }
                        }
                        3 => {
                            // DW_LNS_advance_line
                            if ip < program_end {
                                let mut sleb = 0i32;
                                let mut shift = 0;
                                loop {
                                    if ip >= program_end {
                                        break;
                                    }
                                    let b = dwarf[ip];
                                    ip += 1;
                                    sleb |= (i32::from(b & 0x7F)) << shift;
                                    shift += 7;
                                    if (b & 0x80) == 0 {
                                        if shift < 32 && (b & 0x40) != 0 {
                                            sleb |= !0 << shift;
                                        }
                                        break;
                                    }
                                }
                                line = line.wrapping_add_signed(sleb);
                            }
                        }
                        4 => {
                            // DW_LNS_set_file
                            if ip < program_end {
                                file_idx = dwarf[ip] as usize;
                                ip += 1;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Special opcode
                    let adjusted = (op - opcode_base) as u32;
                    let addr_adv = (adjusted / line_range) * min_inst_len;
                    let line_adv = line_base + (adjusted % line_range) as i32;

                    address = address.wrapping_add(addr_adv);
                    line = line.wrapping_add_signed(line_adv);

                    let file = file_names
                        .get(file_idx)
                        .cloned()
                        .unwrap_or_else(|| PathBuf::from("main.c"));
                    syms.insert_line(
                        address,
                        SourceLocation::with_col(file, line as usize, col as usize),
                    );
                }
            }

            pos += 4 + unit_len;
        }
    }
}
