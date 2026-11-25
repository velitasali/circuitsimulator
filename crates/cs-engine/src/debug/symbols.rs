//! MCU debug symbols: ELF / DWARF and `.lst` list file mapping for flash addresses,
//! source code lines, functions, and variables.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str;

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
        let list = self.source_to_flash.entry(key).or_default();
        if !list.contains(&pc) {
            list.push(pc);
        }
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
    /// Falls back to the nearest preceding line entry within the same function
    /// (or within 32 address units) so stops mid-statement still highlight the line.
    pub fn lookup_address(&self, pc: u32) -> Option<&SourceLocation> {
        if let Some(exact) = self.flash_to_source.get(&pc) {
            return Some(exact);
        }
        if let Some((&prev_pc, loc)) = self.flash_to_source.range(..=pc).next_back() {
            let delta = pc.saturating_sub(prev_pc);
            let same_fn = match (self.function_at(prev_pc), self.function_at(pc)) {
                (Some(a), Some(b)) => a == b,
                (None, None) => delta <= 32,
                _ => false,
            };
            if same_fn && delta <= 64 {
                return Some(loc);
            }
        }
        None
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

    /// Resolve breakpoint address(es) for a source file and 1-based line.
    /// Returns the entry address of the line (or the nearest following executable
    /// line within 5 lines in the same file when the exact line has no code,
    /// e.g. blank line / comment / opening brace).
    pub fn resolve_breakpoint_addrs(&self, file: &Path, line: usize) -> Vec<u32> {
        let exact = self.lookup_lines(file, line);
        if !exact.is_empty() {
            return Self::select_line_entry_addrs(exact);
        }
        // Fallback: search up to 5 lines forward in the same file.
        let want_name = file.file_name();
        let mut best: Option<(usize, &[u32])> = None;
        for ((path, l), addrs) in &self.source_to_flash {
            if *l > line && *l <= line.saturating_add(5) && !addrs.is_empty() {
                let matches_file =
                    path == file || (want_name.is_some() && path.file_name() == want_name);
                if matches_file && best.is_none_or(|(bl, _)| *l < bl) {
                    best = Some((*l, addrs.as_slice()));
                }
            }
        }
        if let Some((_, addrs)) = best {
            return Self::select_line_entry_addrs(addrs);
        }
        Vec::new()
    }

    /// Collapse contiguous instructions belonging to the same source line down to
    /// the entry address of each disjoint code range (e.g. inlined copies get one
    /// breakpoint each, while multi-instruction lines only break once at entry).
    fn select_line_entry_addrs(addrs: &[u32]) -> Vec<u32> {
        let mut sorted: Vec<u32> = addrs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        let mut out = Vec::new();
        for addr in sorted {
            if let Some(&prev) = out.last() {
                // Consecutive instructions for the same statement are typically within 16 bytes.
                if addr.saturating_sub(prev) <= 16 {
                    continue;
                }
            }
            out.push(addr);
        }
        out
    }

    /// Return the function name that starts at or contains `pc`.
    pub fn function_at(&self, pc: u32) -> Option<&str> {
        if let Some(name) = self.pc_to_function.get(&pc) {
            return Some(name.as_str());
        }
        if let Some((_, name)) = self.pc_to_function.range(..=pc).next_back() {
            if let Some(fn_sym) = self.functions.get(name) {
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

        // ELF machine architecture: AVR uses 16-bit word-addressed PC in cs-mcu,
        // while ELF/DWARF stores byte addresses.
        const EM_AVR: u16 = 83;
        let e_machine = read_u16(18);
        let code_addr_div: u32 = if e_machine == EM_AVR { 2 } else { 1 };

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
            str::from_utf8(&slice[..len]).unwrap_or("")
        };

        // Find .symtab, .strtab, .debug_line, .debug_str, and .debug_line_str sections
        let mut symtab_offset = 0;
        let mut symtab_size = 0;
        let mut symtab_entsize = if is_64 { 24 } else { 16 };
        let mut symstr_offset = 0;
        let mut debug_line_offset = 0;
        let mut debug_line_size = 0;
        let mut debug_str_offset = 0;
        let mut debug_str_size = 0;
        let mut debug_line_str_offset = 0;
        let mut debug_line_str_size = 0;

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

            match name {
                ".symtab" => {
                    symtab_offset = sh_offset;
                    symtab_size = sh_size;
                    if sh_entsize > 0 {
                        symtab_entsize = sh_entsize;
                    }
                }
                ".strtab" => {
                    symstr_offset = sh_offset;
                }
                ".debug_line" => {
                    debug_line_offset = sh_offset;
                    debug_line_size = sh_size;
                }
                ".debug_str" => {
                    debug_str_offset = sh_offset;
                    debug_str_size = sh_size;
                }
                ".debug_line_str" => {
                    debug_line_str_offset = sh_offset;
                    debug_line_str_size = sh_size;
                }
                _ => {}
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
                str::from_utf8(&slice[..len]).unwrap_or("")
            };

            let count = symtab_size / symtab_entsize;
            for i in 0..count {
                let ent_offset = symtab_offset + i * symtab_entsize;
                if ent_offset + symtab_entsize > bytes.len() {
                    break;
                }
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
                    && !sym_name.contains('$')
                    && !sym_name.starts_with("CSWTCH")
                    && sym_name != "_edata"
                    && sym_name != "_etext"
                    && sym_name != "_end"
                {
                    // STT_FUNC = 2
                    if sym_type == 2 {
                        let start_pc = st_value / code_addr_div;
                        let end_pc = if st_size > 0 {
                            Some(
                                st_value.saturating_add(st_size as u32).saturating_sub(1)
                                    / code_addr_div,
                            )
                        } else {
                            None
                        };
                        syms.insert_function(sym_name, start_pc, end_pc);
                    }
                    // STT_OBJECT = 1 (Variables in .data/.bss) or STT_NOTYPE in AVR RAM range
                    else if (sym_type == 1 && st_value != 0)
                        || (sym_type == 0 && st_value >= 0x00800000 && st_value <= 0x0080FFFF)
                    {
                        let ram_addr = if st_value >= 0x00800000 && st_value <= 0x0080FFFF {
                            st_value - 0x00800000
                        } else {
                            st_value
                        };
                        let type_str = if st_size == 2 {
                            "uint16"
                        } else if st_size == 4 {
                            "uint32"
                        } else {
                            "uint8"
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
            let debug_str = if debug_str_offset > 0
                && debug_str_size > 0
                && debug_str_offset + debug_str_size <= bytes.len()
            {
                &bytes[debug_str_offset..debug_str_offset + debug_str_size]
            } else {
                &[]
            };
            let debug_line_str = if debug_line_str_offset > 0
                && debug_line_str_size > 0
                && debug_line_str_offset + debug_line_str_size <= bytes.len()
            {
                &bytes[debug_line_str_offset..debug_line_str_offset + debug_line_str_size]
            } else {
                &[]
            };
            Self::parse_dwarf_lines(
                &bytes[debug_line_offset..debug_line_offset + debug_line_size],
                debug_str,
                debug_line_str,
                is_64,
                is_le,
                code_addr_div,
                &mut syms,
            );
        }

        Some(syms)
    }

    /// DWARF `.debug_line` line number program parser supporting DWARF v2, v3, v4, and v5.
    fn parse_dwarf_lines(
        dwarf: &[u8],
        debug_str: &[u8],
        debug_line_str: &[u8],
        elf_is_64: bool,
        is_le: bool,
        code_addr_div: u32,
        syms: &mut DebugSymbols,
    ) {
        let r_u16 = |buf: &[u8], off: usize| -> u16 {
            if off + 2 > buf.len() {
                return 0;
            }
            if is_le {
                u16::from_le_bytes([buf[off], buf[off + 1]])
            } else {
                u16::from_be_bytes([buf[off], buf[off + 1]])
            }
        };
        let r_u32 = |buf: &[u8], off: usize| -> u32 {
            if off + 4 > buf.len() {
                return 0;
            }
            if is_le {
                u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
            } else {
                u32::from_be_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
            }
        };
        let r_u64 = |buf: &[u8], off: usize| -> u64 {
            if off + 8 > buf.len() {
                return 0;
            }
            if is_le {
                u64::from_le_bytes([
                    buf[off],
                    buf[off + 1],
                    buf[off + 2],
                    buf[off + 3],
                    buf[off + 4],
                    buf[off + 5],
                    buf[off + 6],
                    buf[off + 7],
                ])
            } else {
                u64::from_be_bytes([
                    buf[off],
                    buf[off + 1],
                    buf[off + 2],
                    buf[off + 3],
                    buf[off + 4],
                    buf[off + 5],
                    buf[off + 6],
                    buf[off + 7],
                ])
            }
        };

        let mut pos = 0usize;
        while pos + 10 <= dwarf.len() {
            let initial_len = r_u32(dwarf, pos);
            if initial_len == 0 {
                break;
            }
            let (is_dwarf64, unit_len, header_pos) = if initial_len == 0xFFFF_FFFF {
                if pos + 12 > dwarf.len() {
                    break;
                }
                (true, r_u64(dwarf, pos + 4) as usize, pos + 12)
            } else {
                (false, initial_len as usize, pos + 4)
            };
            let Some(program_end) = header_pos.checked_add(unit_len) else {
                break;
            };
            if program_end > dwarf.len() || header_pos + 6 > program_end {
                break;
            }

            let mut cur = header_pos;
            let version = r_u16(dwarf, cur);
            cur += 2;
            if !(2..=5).contains(&version) {
                pos = program_end;
                continue;
            }

            let address_size: u8 = if version >= 5 {
                if cur + 2 > program_end {
                    pos = program_end;
                    continue;
                }
                let addr_sz = dwarf[cur];
                let _seg_sel_sz = dwarf[cur + 1];
                cur += 2;
                addr_sz
            } else if elf_is_64 {
                8
            } else {
                4
            };

            let header_len = if is_dwarf64 {
                if cur + 8 > program_end {
                    pos = program_end;
                    continue;
                }
                let v = r_u64(dwarf, cur) as usize;
                cur += 8;
                v
            } else {
                if cur + 4 > program_end {
                    pos = program_end;
                    continue;
                }
                let v = r_u32(dwarf, cur) as usize;
                cur += 4;
                v
            };

            let Some(program_start) = cur.checked_add(header_len) else {
                pos = program_end;
                continue;
            };
            if program_start > program_end || cur + 5 > program_start {
                pos = program_end;
                continue;
            }

            let min_inst_len = u64::from(dwarf[cur].max(1));
            cur += 1;
            let max_ops_per_inst = if version >= 4 {
                if cur >= program_start {
                    pos = program_end;
                    continue;
                }
                let v = u64::from(dwarf[cur].max(1));
                cur += 1;
                v
            } else {
                1
            };
            if cur + 4 > program_start {
                pos = program_end;
                continue;
            }
            let default_is_stmt = dwarf[cur] != 0;
            let line_base = i64::from(dwarf[cur + 1] as i8);
            let line_range = u64::from(dwarf[cur + 2]);
            let opcode_base = dwarf[cur + 3];
            cur += 4;

            let std_op_count = opcode_base.saturating_sub(1) as usize;
            if cur + std_op_count > program_start {
                pos = program_end;
                continue;
            }
            let std_op_lens = &dwarf[cur..cur + std_op_count];
            cur += std_op_count;

            // Parse directory table and file name table
            let mut file_names: Vec<PathBuf> = Vec::new();
            if version <= 4 {
                // DWARF v2-v4: 1-based directory & file indices (0 = none/compilation dir)
                let mut dirs: Vec<String> = vec![String::new()];
                while cur < program_start && dwarf[cur] != 0 {
                    let s = read_cstr(dwarf, &mut cur, program_start);
                    dirs.push(strip_verbatim_prefix(&s).to_string());
                }
                if cur < program_start && dwarf[cur] == 0 {
                    cur += 1;
                }

                file_names.push(PathBuf::new()); // slot 0 unused in v2-v4
                while cur < program_start && dwarf[cur] != 0 {
                    let name = read_cstr(dwarf, &mut cur, program_start);
                    let dir_idx = read_uleb128(dwarf, &mut cur, program_start) as usize;
                    let _mtime = read_uleb128(dwarf, &mut cur, program_start);
                    let _fsize = read_uleb128(dwarf, &mut cur, program_start);
                    file_names.push(join_dwarf_path(
                        dirs.get(dir_idx).map(String::as_str),
                        &name,
                    ));
                }
            } else {
                // DWARF v5: format-described 0-based directory & file tables
                let mut dirs: Vec<String> = Vec::new();
                if cur < program_start {
                    let dir_fmt_count = dwarf[cur] as usize;
                    cur += 1;
                    let mut dir_fmts = Vec::with_capacity(dir_fmt_count);
                    for _ in 0..dir_fmt_count {
                        let ct = read_uleb128(dwarf, &mut cur, program_start);
                        let form = read_uleb128(dwarf, &mut cur, program_start);
                        dir_fmts.push((ct, form));
                    }
                    let dir_count = read_uleb128(dwarf, &mut cur, program_start) as usize;
                    for _ in 0..dir_count {
                        let mut dir_path = String::new();
                        for &(ct, form) in &dir_fmts {
                            let val = read_dwarf_form(
                                dwarf,
                                &mut cur,
                                program_start,
                                form,
                                is_dwarf64,
                                is_le,
                                debug_str,
                                debug_line_str,
                            );
                            if ct == 1 {
                                // DW_LNCT_path
                                if let DwarfVal::Str(s) = val {
                                    dir_path = strip_verbatim_prefix(&s).to_string();
                                }
                            }
                        }
                        dirs.push(dir_path);
                    }
                }
                if cur < program_start {
                    let file_fmt_count = dwarf[cur] as usize;
                    cur += 1;
                    let mut file_fmts = Vec::with_capacity(file_fmt_count);
                    for _ in 0..file_fmt_count {
                        let ct = read_uleb128(dwarf, &mut cur, program_start);
                        let form = read_uleb128(dwarf, &mut cur, program_start);
                        file_fmts.push((ct, form));
                    }
                    let file_count = read_uleb128(dwarf, &mut cur, program_start) as usize;
                    for _ in 0..file_count {
                        let mut name = String::new();
                        let mut dir_idx: usize = 0;
                        for &(ct, form) in &file_fmts {
                            let val = read_dwarf_form(
                                dwarf,
                                &mut cur,
                                program_start,
                                form,
                                is_dwarf64,
                                is_le,
                                debug_str,
                                debug_line_str,
                            );
                            match (ct, val) {
                                (1, DwarfVal::Str(s)) => name = s,
                                (2, DwarfVal::UInt(d)) => dir_idx = d as usize,
                                _ => {}
                            }
                        }
                        file_names.push(join_dwarf_path(
                            dirs.get(dir_idx).map(String::as_str),
                            &name,
                        ));
                    }
                }
            }

            if line_range == 0 {
                pos = program_end;
                continue;
            }

            // Run line number state machine
            let mut address: u64 = 0;
            let mut op_index: u64 = 0;
            let mut file_idx: usize = 1;
            let mut line: i64 = 1;
            let mut col: u32 = 0;
            let mut _is_stmt = default_is_stmt;

            let advance_op = |address: &mut u64, op_index: &mut u64, op_adv: u64| {
                if max_ops_per_inst <= 1 {
                    *address = address.wrapping_add(op_adv.wrapping_mul(min_inst_len));
                } else {
                    let total = *op_index + op_adv;
                    *address =
                        address.wrapping_add(min_inst_len.wrapping_mul(total / max_ops_per_inst));
                    *op_index = total % max_ops_per_inst;
                }
            };

            let mut ip = program_start;
            while ip < program_end {
                let op = dwarf[ip];
                ip += 1;

                if op == 0 {
                    // Extended opcode
                    let ext_len = read_uleb128(dwarf, &mut ip, program_end) as usize;
                    let ext_end = ip.saturating_add(ext_len).min(program_end);
                    if ip >= ext_end {
                        ip = ext_end;
                        continue;
                    }
                    let ext_op = dwarf[ip];
                    ip += 1;
                    match ext_op {
                        1 => {
                            // DW_LNE_end_sequence
                            address = 0;
                            op_index = 0;
                            file_idx = 1;
                            line = 1;
                            col = 0;
                            _is_stmt = default_is_stmt;
                        }
                        2 => {
                            // DW_LNE_set_address
                            let rem = ext_end.saturating_sub(ip);
                            if (address_size == 8 || rem == 8) && ip + 8 <= ext_end {
                                address = r_u64(dwarf, ip);
                            } else if ip + 4 <= ext_end {
                                address = u64::from(r_u32(dwarf, ip));
                            } else if ip + 2 <= ext_end {
                                address = u64::from(r_u16(dwarf, ip));
                            }
                            op_index = 0;
                        }
                        3 => {
                            // DW_LNE_define_file
                            let name = read_cstr(dwarf, &mut ip, ext_end);
                            let _dir_idx = read_uleb128(dwarf, &mut ip, ext_end);
                            let _mtime = read_uleb128(dwarf, &mut ip, ext_end);
                            let _fsize = read_uleb128(dwarf, &mut ip, ext_end);
                            file_names.push(join_dwarf_path(None, &name));
                        }
                        _ => {}
                    }
                    ip = ext_end;
                } else if op < opcode_base {
                    // Standard opcode
                    match op {
                        1 => {
                            // DW_LNS_copy
                            if line > 0 {
                                if let Some(file) = file_names.get(file_idx) {
                                    if !file.as_os_str().is_empty() {
                                        let pc = (address as u32) / code_addr_div;
                                        syms.insert_line(
                                            pc,
                                            SourceLocation::with_col(
                                                file.clone(),
                                                line as usize,
                                                col as usize,
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                        2 => {
                            // DW_LNS_advance_pc
                            let op_adv = read_uleb128(dwarf, &mut ip, program_end);
                            advance_op(&mut address, &mut op_index, op_adv);
                        }
                        3 => {
                            // DW_LNS_advance_line
                            let delta = read_sleb128(dwarf, &mut ip, program_end);
                            line = line.wrapping_add(delta);
                        }
                        4 => {
                            // DW_LNS_set_file
                            file_idx = read_uleb128(dwarf, &mut ip, program_end) as usize;
                        }
                        5 => {
                            // DW_LNS_set_column
                            col = read_uleb128(dwarf, &mut ip, program_end) as u32;
                        }
                        6 => {
                            // DW_LNS_negate_stmt
                            _is_stmt = !_is_stmt;
                        }
                        7 => {
                            // DW_LNS_set_basic_block (0 operands)
                        }
                        8 => {
                            // DW_LNS_const_add_pc
                            let adjusted = u64::from(255u8.saturating_sub(opcode_base));
                            let op_adv = adjusted / line_range;
                            advance_op(&mut address, &mut op_index, op_adv);
                        }
                        9 => {
                            // DW_LNS_fixed_advance_pc (u16 operand)
                            if ip + 2 <= program_end {
                                let adv = u64::from(r_u16(dwarf, ip));
                                ip += 2;
                                address = address.wrapping_add(adv);
                                op_index = 0;
                            }
                        }
                        10 | 11 => {
                            // DW_LNS_set_prologue_end / DW_LNS_set_epilogue_begin (0 operands)
                        }
                        12 => {
                            // DW_LNS_set_isa (1 ULEB128 operand)
                            let _ = read_uleb128(dwarf, &mut ip, program_end);
                        }
                        _ => {
                            // Unknown standard opcode: skip exact number of ULEB128 args from header.
                            let argc =
                                std_op_lens.get((op - 1) as usize).copied().unwrap_or(0) as usize;
                            for _ in 0..argc {
                                let _ = read_uleb128(dwarf, &mut ip, program_end);
                            }
                        }
                    }
                } else {
                    // Special opcode
                    let adjusted = u64::from(op - opcode_base);
                    let op_adv = adjusted / line_range;
                    let line_adv = line_base + (adjusted % line_range) as i64;

                    advance_op(&mut address, &mut op_index, op_adv);
                    line = line.wrapping_add(line_adv);

                    if line > 0 {
                        if let Some(file) = file_names.get(file_idx) {
                            if !file.as_os_str().is_empty() {
                                let pc = (address as u32) / code_addr_div;
                                syms.insert_line(
                                    pc,
                                    SourceLocation::with_col(
                                        file.clone(),
                                        line as usize,
                                        col as usize,
                                    ),
                                );
                            }
                        }
                    }
                }
            }

            pos = program_end;
        }
    }
}

enum DwarfVal {
    Str(String),
    UInt(u64),
    None,
}

fn read_cstr(buf: &[u8], pos: &mut usize, end: usize) -> String {
    let start = *pos;
    while *pos < end && buf[*pos] != 0 {
        *pos += 1;
    }
    let s = str::from_utf8(&buf[start..*pos]).unwrap_or("").to_string();
    if *pos < end && buf[*pos] == 0 {
        *pos += 1;
    }
    s
}

fn read_cstr_at(buf: &[u8], offset: usize) -> String {
    if offset >= buf.len() {
        return String::new();
    }
    let slice = &buf[offset..];
    let len = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    str::from_utf8(&slice[..len]).unwrap_or("").to_string()
}

fn read_uleb128(buf: &[u8], pos: &mut usize, end: usize) -> u64 {
    let mut result = 0u64;
    let mut shift = 0u32;
    while *pos < end {
        let b = buf[*pos];
        *pos += 1;
        if shift < 64 {
            result |= u64::from(b & 0x7F) << shift;
        }
        if (b & 0x80) == 0 {
            break;
        }
        shift = shift.saturating_add(7);
    }
    result
}

fn read_sleb128(buf: &[u8], pos: &mut usize, end: usize) -> i64 {
    let mut result = 0i64;
    let mut shift = 0u32;
    while *pos < end {
        let b = buf[*pos];
        *pos += 1;
        if shift < 64 {
            result |= i64::from(b & 0x7F) << shift;
        }
        shift = shift.saturating_add(7);
        if (b & 0x80) == 0 {
            if shift < 64 && (b & 0x40) != 0 {
                result |= !0i64 << shift;
            }
            break;
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn read_dwarf_form(
    buf: &[u8],
    pos: &mut usize,
    end: usize,
    form: u64,
    is_dwarf64: bool,
    is_le: bool,
    debug_str: &[u8],
    debug_line_str: &[u8],
) -> DwarfVal {
    let read_u16 = |b: &[u8], p: &mut usize| -> u16 {
        if *p + 2 > end {
            *p = end;
            return 0;
        }
        let v = if is_le {
            u16::from_le_bytes([b[*p], b[*p + 1]])
        } else {
            u16::from_be_bytes([b[*p], b[*p + 1]])
        };
        *p += 2;
        v
    };
    let read_u32 = |b: &[u8], p: &mut usize| -> u32 {
        if *p + 4 > end {
            *p = end;
            return 0;
        }
        let v = if is_le {
            u32::from_le_bytes([b[*p], b[*p + 1], b[*p + 2], b[*p + 3]])
        } else {
            u32::from_be_bytes([b[*p], b[*p + 1], b[*p + 2], b[*p + 3]])
        };
        *p += 4;
        v
    };
    let read_u64 = |b: &[u8], p: &mut usize| -> u64 {
        if *p + 8 > end {
            *p = end;
            return 0;
        }
        let v = if is_le {
            u64::from_le_bytes([
                b[*p],
                b[*p + 1],
                b[*p + 2],
                b[*p + 3],
                b[*p + 4],
                b[*p + 5],
                b[*p + 6],
                b[*p + 7],
            ])
        } else {
            u64::from_be_bytes([
                b[*p],
                b[*p + 1],
                b[*p + 2],
                b[*p + 3],
                b[*p + 4],
                b[*p + 5],
                b[*p + 6],
                b[*p + 7],
            ])
        };
        *p += 8;
        v
    };

    match form {
        0x08 => DwarfVal::Str(read_cstr(buf, pos, end)), // DW_FORM_string
        0x0e => {
            // DW_FORM_strp
            let off = if is_dwarf64 {
                read_u64(buf, pos) as usize
            } else {
                read_u32(buf, pos) as usize
            };
            DwarfVal::Str(read_cstr_at(debug_str, off))
        }
        0x1f => {
            // DW_FORM_line_strp
            let off = if is_dwarf64 {
                read_u64(buf, pos) as usize
            } else {
                read_u32(buf, pos) as usize
            };
            DwarfVal::Str(read_cstr_at(debug_line_str, off))
        }
        0x0b => {
            // DW_FORM_data1
            if *pos < end {
                let v = u64::from(buf[*pos]);
                *pos += 1;
                DwarfVal::UInt(v)
            } else {
                DwarfVal::None
            }
        }
        0x05 => DwarfVal::UInt(u64::from(read_u16(buf, pos))), // DW_FORM_data2
        0x06 => DwarfVal::UInt(u64::from(read_u32(buf, pos))), // DW_FORM_data4
        0x07 => DwarfVal::UInt(read_u64(buf, pos)),            // DW_FORM_data8
        0x0f => DwarfVal::UInt(read_uleb128(buf, pos, end)),   // DW_FORM_udata
        0x0d => DwarfVal::UInt(read_sleb128(buf, pos, end) as u64), // DW_FORM_sdata
        0x1e => {
            // DW_FORM_data16 (16-byte MD5 digest)
            *pos = (*pos + 16).min(end);
            DwarfVal::None
        }
        0x1d => {
            // DW_FORM_strp_sup
            if is_dwarf64 {
                let _ = read_u64(buf, pos);
            } else {
                let _ = read_u32(buf, pos);
            }
            DwarfVal::None
        }
        0x09 => {
            // DW_FORM_block
            let len = read_uleb128(buf, pos, end) as usize;
            *pos = (*pos + len).min(end);
            DwarfVal::None
        }
        0x0a => {
            // DW_FORM_block1
            if *pos < end {
                let len = buf[*pos] as usize;
                *pos = (*pos + 1 + len).min(end);
            }
            DwarfVal::None
        }
        0x03 => {
            // DW_FORM_block2
            let len = read_u16(buf, pos) as usize;
            *pos = (*pos + len).min(end);
            DwarfVal::None
        }
        0x04 => {
            // DW_FORM_block4
            let len = read_u32(buf, pos) as usize;
            *pos = (*pos + len).min(end);
            DwarfVal::None
        }
        _ => DwarfVal::None,
    }
}

fn strip_verbatim_prefix(s: &str) -> &str {
    s.strip_prefix(r"\\?\").unwrap_or(s)
}

fn join_dwarf_path(dir: Option<&str>, name: &str) -> PathBuf {
    let clean_name = strip_verbatim_prefix(name);
    let p = Path::new(clean_name);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if let Some(d) = dir {
        let clean_d = strip_verbatim_prefix(d);
        if !clean_d.is_empty() {
            return Path::new(clean_d).join(clean_name);
        }
    }
    p.to_path_buf()
}
