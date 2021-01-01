//! Circuit wrapper around `cs_mcu::Device`. GPIO is analog `IoPin`s; the core
//! steps on the picosecond event queue.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use cs_mcu::{CoreKind, Device, McuDesc, McuState};

use crate::digital::{IoPin, PinAction, PinMode};
use crate::package::{Package, PkgPin, convert_package, select_package};
use crate::script::ScriptCpu;
use crate::subcircuit::SubcSearch;

/// `.sim1` MCU properties before `{device}.mcu` is resolved.
#[derive(Clone, Debug, Default)]
pub struct McuItemSpec {
    pub device: String,
    pub frequency: Option<f64>,
    pub force_freq: bool,
    pub program: Option<String>,
    pub auto_load: bool,
    pub save_pgm: bool,
    pub pgm: Option<String>,
    pub logic_symbol: bool,
    pub package_name: Option<String>,
}

/// MCU instance on the netlist (C++ `Mcu` + `eMcu`).
#[derive(Clone, Debug)]
pub struct McuComp {
    pub device: Device,
    pub pins: Vec<IoPin>,
    pub node_indices: Vec<Option<usize>>,
    /// C++ `m_autoLoad`: reload `firmware` at simulation start.
    pub auto_load: bool,
    /// C++ `ForceFreq` (default true).
    pub force_freq: bool,
    /// C++ `m_savePGM`.
    pub save_pgm: bool,
    /// C++ `m_eMcu.m_firmware`, relative to [`Self::firmware_dir`].
    pub firmware: Option<String>,
    /// Circuit folder, or nested subcircuit folder while `Program` is applied.
    pub firmware_dir: Option<PathBuf>,
    pub cached_mcu_id: String,
    pub cached_usart_port_ids: Vec<String>,
    pub cached_twi_port_ids: Vec<String>,
}

/// Live MCU monitor dump (C++ `MCUMonitor::updateStep`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct McuSnap {
    pub id: String,
    pub status_bits: Vec<String>,
    pub status: u8,
    pub has_status: bool,
    pub pc: u32,
    pub word_size: u8,
    pub ram: Vec<u8>,
    pub registers: Vec<(String, u16)>,
    pub flash: Vec<u16>,
    pub eeprom: Vec<u8>,
}

impl McuSnap {
    pub fn from_mcu(id: &str, mcu: &McuComp) -> Self {
        Self::from_device(id, &mcu.device)
    }

    pub fn from_device(id: &str, d: &Device) -> Self {
        Self {
            id: id.to_string(),
            status_bits: d.status_bits().to_vec(),
            status: d.status(),
            has_status: d.has_status(),
            pc: d.pc(),
            word_size: d.word_size,
            ram: d.ram_dump(),
            registers: d.registers(),
            flash: d.flash_words().to_vec(),
            eeprom: d.eeprom().to_vec(),
        }
    }
}

/// Pending monitor writes applied on the canvas tick / `sync_mcu`.
#[derive(Clone, Copy, Debug)]
pub enum McuPoke {
    Ram { addr: u16, value: u8 },
    Flash { addr: u32, value: u16 },
    Eeprom { addr: u16, value: u8 },
}

fn snap_cell() -> &'static Mutex<McuSnap> {
    static CELL: OnceLock<Mutex<McuSnap>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(McuSnap::default()))
}

fn poke_cell() -> &'static Mutex<Vec<McuPoke>> {
    static CELL: OnceLock<Mutex<Vec<McuPoke>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn publish_monitor(snap: McuSnap) {
    if let Ok(mut g) = snap_cell().lock() {
        *g = snap;
    }
}

pub fn monitor_snap() -> McuSnap {
    snap_cell().lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn queue_poke(p: McuPoke) {
    if let Ok(mut g) = poke_cell().lock() {
        g.push(p);
    }
}

pub fn take_pokes() -> Vec<McuPoke> {
    poke_cell()
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default()
}

/// C++ `decToBase` (fixed width, uppercase hex digits).
pub fn dec_to_base(mut value: u32, base: u32, digits: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEF";
    let mut out = String::new();
    let base = base.max(2);
    for _ in 0..digits {
        let d = if value >= base { value % base } else { value };
        out.insert(0, DIGITS[d as usize] as char);
        value /= base;
    }
    out
}

/// C++ `val2hex`: hex without `0x`, padded to an even width.
pub fn val2hex(d: u32) -> String {
    if d == 0 {
        return "00".into();
    }
    let s = format!("{d:X}");
    if s.len() % 2 == 1 { format!("0{s}") } else { s }
}

/// C++ `RamTable::updateValues` uint8 cell.
pub fn format_u8_watch(v: u8) -> String {
    let v = u32::from(v);
    format!(
        "{} 0x{} {}",
        dec_to_base(v, 10, 3),
        dec_to_base(v, 16, 2),
        dec_to_base(v, 2, 8)
    )
}

impl McuComp {
    pub fn from_device(device: Device) -> Self {
        let vdd = device.vdd;
        let mut pins = Vec::new();
        for g in device.gpio_pins() {
            let mut p = IoPin::input(g.name.clone());
            p.set_levels(vdd, 0.0);
            p.set_thresholds(vdd / 2.0, vdd / 2.0);
            if g.pullup {
                p.set_pullup(1e5);
            }
            pins.push(p);
        }
        let n_pins = pins.len();
        let mcu_id = &device.id;
        let cached_usart_port_ids = device
            .usart_names()
            .into_iter()
            .map(|u| format!("{mcu_id}:{u}"))
            .collect();
        let cached_twi_port_ids = device
            .twi_names()
            .into_iter()
            .map(|t| format!("{mcu_id}:{t}"))
            .collect();
        Self {
            cached_mcu_id: device.id.clone(),
            device,
            pins,
            node_indices: vec![None; n_pins],
            auto_load: false,
            force_freq: true,
            save_pgm: false,
            firmware: None,
            firmware_dir: None,
            cached_usart_port_ids,
            cached_twi_port_ids,
        }
    }
}

impl Default for McuComp {
    fn default() -> Self {
        const DEFAULT_XML: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="PORTA" addr="0x05" reset="0"/>
  </regblock>
</mcu>
"#;
        let device = cs_mcu::Device::from_xml("", DEFAULT_XML).expect("default mcu xml");
        Self::from_device(device)
    }
}

impl McuComp {
    pub fn refresh_cached_port_ids(&mut self) {
        self.cached_mcu_id = self.device.id.clone();
        let mcu_id = &self.cached_mcu_id;
        self.cached_usart_port_ids = self
            .device
            .usart_names()
            .into_iter()
            .map(|u| format!("{mcu_id}:{u}"))
            .collect();
        self.cached_twi_port_ids = self
            .device
            .twi_names()
            .into_iter()
            .map(|t| format!("{mcu_id}:{t}"))
            .collect();
    }

    pub fn update_node_indices(&mut self, pin_net: &rustc_hash::FxHashMap<String, usize>) {
        self.node_indices = self
            .pins
            .iter_mut()
            .map(|p| {
                let n = pin_net.get(&p.id).copied();
                p.node_idx = n;
                n
            })
            .collect();
    }

    pub fn pin_ids(&self) -> Vec<String> {
        self.pins.iter().map(|p| p.id.clone()).collect()
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        crate::logging::log_sim(format!("Initializing MCU '{}'", self.device.id));
        self.device.reset();
        if let Some(path) = self.firmware_path() {
            let _ = self.load_firmware_file(&path);
        }
        for p in &mut self.pins {
            p.initialize(slope_steps);
        }
        self.sync_gpio_to_pins();
    }

    pub fn firmware_path(&self) -> Option<PathBuf> {
        resolve_firmware(self.firmware_dir.as_deref(), self.firmware.as_deref()?)
    }

    pub fn load_firmware_file(&mut self, path: &Path) -> crate::Result<()> {
        crate::logging::log_sim(format!(
            "Loading firmware into MCU '{}' from '{}'...",
            self.device.id,
            path.display()
        ));
        // Automatically check for and load debug symbols
        let mut loaded_syms: Option<crate::debug::DebugSymbols> = None;
        if let Ok(bytes) = std::fs::read(path) {
            if bytes.starts_with(b"\x7fELF") {
                loaded_syms = crate::debug::ElfParser::parse(&bytes);
            }
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let elf_candidates = [
            path.with_extension("elf"),
            parent.join(format!("{stem}.ino.elf")),
            path.with_extension(""),
        ];
        for elf_path in &elf_candidates {
            if elf_path.is_file() {
                if let Ok(bytes) = std::fs::read(elf_path) {
                    if bytes.starts_with(b"\x7fELF") {
                        if let Some(parsed) = crate::debug::ElfParser::parse(&bytes) {
                            if let Some(s) = &mut loaded_syms {
                                s.merge(parsed);
                            } else {
                                loaded_syms = Some(parsed);
                            }
                        }
                    }
                }
            }
        }
        for ext in &["lst", "sym", "map", "lss"] {
            let sym_file = path.with_extension(ext);
            if sym_file.is_file() {
                if let Ok(content) = std::fs::read_to_string(&sym_file) {
                    let parsed =
                        crate::debug::LstParser::parse(&content, path, crate::debug::LstKind::Auto);
                    if let Some(s) = &mut loaded_syms {
                        s.merge(parsed);
                    } else {
                        loaded_syms = Some(parsed);
                    }
                }
            }
        }
        if let Some(syms) = loaded_syms {
            crate::debug::DebugSession::global().load_symbols(syms);
        }

        // Load hex file into device
        let hex_path = if let Ok(bytes) = std::fs::read(path) {
            if bytes.starts_with(b"\x7fELF") {
                let cand = path.with_extension("hex");
                if cand.is_file() {
                    cand
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        };

        if let Ok(src) = std::fs::read_to_string(&hex_path) {
            let _ = self.device.load_hex(&src);
        }
        Ok(())
    }

    /// C++ `Mcu::setPGM`: comma-separated flash words.
    pub fn load_pgm_str(&mut self, pgm: &str) {
        for (i, token) in pgm.split(',').enumerate() {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Ok(v) = token.parse::<u16>() {
                self.device.set_flash(i, v);
            } else if let Ok(v) = token.parse::<i32>() {
                self.device.set_flash(i, v as u16);
            }
        }
    }

    pub fn sync_gpio_to_pins_buf(&mut self, actions: &mut Vec<(usize, PinAction)>) {
        if !self.device.ports_dirty() {
            return;
        }
        self.device.clear_ports_dirty();
        let mut i = 0;
        for g in self.device.gpio_pins() {
            if i >= self.pins.len() {
                break;
            }
            let is_out = g.is_out;
            let out_state = g.out_state;
            let pullup = g.pullup;
            let open_coll = g.open_coll;
            let pin = &mut self.pins[i];
            if is_out {
                let mode = if open_coll {
                    PinMode::OpenCo
                } else {
                    PinMode::Output
                };
                let mut changed = false;
                if pin.mode != mode {
                    pin.set_pin_mode(mode);
                    changed = true;
                }
                if pin.get_out_state() != out_state {
                    pin.set_out_state(out_state);
                    changed = true;
                }
                if changed {
                    actions.push((i, PinAction::Immediate));
                }
            } else {
                let mut changed = false;
                if pin.mode != PinMode::Input {
                    pin.set_pin_mode(PinMode::Input);
                    changed = true;
                }
                if (pin.has_pullup()) != pullup {
                    pin.set_pullup(if pullup { 1e5 } else { 0.0 });
                    changed = true;
                }
                if changed {
                    actions.push((i, PinAction::Immediate));
                }
            }
            i += 1;
        }
    }

    pub fn sync_gpio_to_pins(&mut self) -> Vec<(usize, PinAction)> {
        let mut actions = Vec::new();
        self.sync_gpio_to_pins_buf(&mut actions);
        actions
    }

    /// Fast path: direct indexing via pre-resolved `node_indices`.
    pub fn sample_inputs_nodes(&mut self, nodes: &[crate::net::ENode]) {
        for (i, p) in self.pins.iter_mut().enumerate() {
            if let Some(&Some(node_idx)) = self.node_indices.get(i) {
                if let Some(n) = nodes.get(node_idx) {
                    let high = p.get_inp_state(n.volt);
                    self.device.set_gpio_input(i, high);
                }
            }
        }
    }

    /// `None` means the pin is not on a net; keep the stamp default (6502 RDY
    /// / Z80 RESET idle high when unwired).
    pub fn sample_inputs(&mut self, v_of: &impl Fn(&str) -> Option<f64>) {
        for (i, p) in self.pins.iter_mut().enumerate() {
            if let Some(v) = v_of(&p.id) {
                let high = p.get_inp_state(v);
                self.device.set_gpio_input(i, high);
            }
        }
    }

    pub fn drain_host_serial(&mut self) {
        if !crate::serial::has_pending_send() {
            return;
        }
        if self.cached_usart_port_ids.is_empty() && !self.device.usarts().is_empty() {
            self.refresh_cached_port_ids();
        }
        let mcu_id = &self.cached_mcu_id;
        for (u_idx, port_id) in self.cached_usart_port_ids.iter().enumerate() {
            let mut pending = crate::serial::take_send(port_id);
            if u_idx == 0 {
                pending.extend(crate::serial::take_send(mcu_id));
                pending.extend(crate::serial::take_send("default"));
            }
            for b in pending {
                crate::serial::publish_in(port_id, b);
                if u_idx == 0 {
                    crate::serial::publish_in(mcu_id, b);
                    crate::serial::publish_in("default", b);
                }
                self.device.inject_usart_rx(u_idx, b);
            }
        }
    }

    pub fn flush_periph_logs(&mut self) {
        if !self.device.has_periph_logs() {
            return;
        }
        if (self.cached_usart_port_ids.is_empty() && !self.device.usarts().is_empty())
            || (self.cached_twi_port_ids.is_empty() && !self.device.twi_names().is_empty())
        {
            self.refresh_cached_port_ids();
        }
        let mcu_id = &self.cached_mcu_id;
        for (u_idx, port_id) in self.cached_usart_port_ids.iter().enumerate() {
            let tx_bytes = self.device.take_usart_tx_bytes(u_idx);
            for b in tx_bytes {
                crate::serial::publish_out(port_id, b);
                if u_idx == 0 {
                    crate::serial::publish_out(mcu_id, b);
                    crate::serial::publish_out("default", b);
                }
            }
            let rx_bytes = self.device.take_usart_rx_bytes(u_idx);
            for b in rx_bytes {
                crate::serial::publish_in(port_id, b);
                if u_idx == 0 {
                    crate::serial::publish_in(mcu_id, b);
                    crate::serial::publish_in("default", b);
                }
            }
        }
        for (t_idx, port_id) in self.cached_twi_port_ids.iter().enumerate() {
            let tx_bytes = self.device.take_twi_tx_bytes(t_idx);
            for b in tx_bytes {
                crate::serial::publish_out(port_id, b);
            }
            let rx_bytes = self.device.take_twi_rx_bytes(t_idx);
            for b in rx_bytes {
                crate::serial::publish_in(port_id, b);
            }
        }
    }

    /// One CPU instruction. Returns per-pin `PinAction`s from GPIO changes.
    pub fn run_event(&mut self, v_of: &impl Fn(&str) -> Option<f64>) -> Vec<(usize, PinAction)> {
        self.drain_host_serial();
        self.sample_inputs(v_of);
        let prev_pc = self.device.pc();
        if self.device.state == McuState::Running {
            self.device.advance();
        }
        self.flush_periph_logs();
        if crate::debug::DebugSession::is_active_global() {
            let pc = self.device.pc();
            let sp = self.device.sp();
            let ret_addr = self.device.ret_addr();
            let _ = crate::debug::DebugSession::global().on_mcu_step(pc, prev_pc, sp, ret_addr);
        }
        self.sync_gpio_to_pins()
    }

    /// Paste / CircId remap: GPIO names and IoPin ids are `{id}-PORTA0`.
    pub fn rebind_id(&mut self, new_id: &str) {
        let old = self.device.id.clone();
        if old == new_id {
            return;
        }
        self.device.id = new_id.to_string();
        self.refresh_cached_port_ids();
        for p in &mut self.pins {
            if let Some(rest) = p.id.strip_prefix(&old) {
                p.id = format!("{new_id}{rest}");
            }
        }
        for g in self.device.gpio_pins_mut() {
            if let Some(rest) = g.name.strip_prefix(&old) {
                g.name = format!("{new_id}{rest}");
            }
        }
    }
}

/// C++ `Mcu::Mcu` PIC name rewrite (`p16f84a` → `p16F84`).
pub fn canonicalize_device(name: &str) -> String {
    let mut d = name.to_string();
    if d.contains('@') {
        d = d.rsplit('@').next().unwrap_or(&d).to_string();
    }
    if d.starts_with('p') {
        if d.ends_with('a') {
            d.pop();
        }
        d = d.replace('f', "F");
    }
    d
}

enum McuSource {
    Memory(String),
    File { mcu: PathBuf, package_base: PathBuf },
}

/// Locate `{device}.mcu` the same way C++ `Mcu::Mcu` does: next to the
/// circuit, then `data/{device}/`, then catalog XML (`getDataFile`), then
/// `{device}/`.
pub fn find_mcu_file(device: &str, circuit_path: Option<&Path>) -> Option<PathBuf> {
    match find_mcu(
        device,
        &SubcSearch::from_circuit_path(circuit_path.and_then(|p| p.to_str()))
            .with_standard_catalog(),
    ) {
        Some(McuSource::File { mcu, .. }) => Some(mcu),
        _ => None,
    }
}

fn find_mcu(device: &str, search: &SubcSearch) -> Option<McuSource> {
    if let Some(xml) = search.memory_mcu.get(device) {
        return Some(McuSource::Memory(xml.clone()));
    }
    let mut dirs = Vec::new();
    if let Some(d) = &search.circuit_dir {
        dirs.push(d.clone());
        dirs.push(d.join("data").join(device));
        dirs.push(d.join(device));
    }
    dirs.extend(search.data_dirs.iter().cloned());
    for dir in dirs {
        if let Some(src) = file_in_dir(&dir, device) {
            return Some(src);
        }
    }
    if let Some(item) = search.catalog.get(device) {
        let (mcu, package_base) = item.mcu_paths();
        if mcu.is_file() {
            return Some(McuSource::File { mcu, package_base });
        }
        let dir = item.dir();
        if let Some(src) = file_in_dir(&dir, device) {
            return Some(src);
        }
        if let Some(src) = file_in_dir(&dir.join(device), device) {
            return Some(src);
        }
    }
    dirs = vec![PathBuf::from("data").join(device), PathBuf::from(device)];
    for dir in dirs {
        if let Some(src) = file_in_dir(&dir, device) {
            return Some(src);
        }
    }
    None
}

fn file_in_dir(dir: &Path, device: &str) -> Option<McuSource> {
    for cand in [
        dir.join(format!("{device}.mcu")),
        dir.join(device).join(format!("{device}.mcu")),
    ] {
        if cand.is_file() {
            let package_base = cand.with_extension("");
            return Some(McuSource::File {
                mcu: cand,
                package_base,
            });
        }
    }
    None
}

pub fn load_device(id: &str, device: &str, circuit_path: Option<&Path>) -> crate::Result<Device> {
    load_device_with(
        id,
        device,
        &SubcSearch::from_circuit_path(circuit_path.and_then(|p| p.to_str())),
    )
}

pub fn load_desc_with(
    device: &str,
    search: &SubcSearch,
) -> crate::Result<(McuDesc, Option<PathBuf>)> {
    let device = canonicalize_device(device);
    match find_mcu(&device, search) {
        Some(McuSource::Memory(xml)) => {
            let desc =
                cs_mcu::parse_mcu_xml(&xml).map_err(|e| crate::Error::Parse(e.to_string()))?;
            Ok((desc, None))
        }
        Some(McuSource::File { mcu, .. }) => {
            let desc =
                cs_mcu::parse_mcu_file(&mcu).map_err(|e| crate::Error::Parse(e.to_string()))?;
            Ok((desc, Some(mcu)))
        }
        None => Err(crate::Error::Parse(format!(
            "MCU files not found for {device}"
        ))),
    }
}

pub fn load_device_with(id: &str, device: &str, search: &SubcSearch) -> crate::Result<Device> {
    let (desc, _) = load_desc_with(device, search)?;
    Device::from_desc(id, desc).map_err(|e| crate::Error::Parse(e.to_string()))
}

fn load_script_source(desc: &McuDesc, mcu_path: Option<&Path>, search: &SubcSearch) -> String {
    let Some(name) = desc.script.as_deref().filter(|s| !s.is_empty()) else {
        return String::new();
    };
    if let Some(src) = search.memory_script.get(name) {
        return src.clone();
    }
    if let Some(path) = mcu_path.and_then(|p| p.parent().map(|d| d.join(name)))
        && let Ok(s) = std::fs::read_to_string(path)
    {
        return s;
    }
    if let Some(dir) = &search.circuit_dir
        && let Ok(s) = std::fs::read_to_string(dir.join(name))
    {
        return s;
    }
    String::new()
}

/// Native `cs-mcu` core or scripted AngelScript CPU (C++ `core="scripted"`).
pub enum InstantiatedMcu {
    Native(McuComp),
    Script(ScriptCpu),
}

fn apply_freq_to_script(cpu: &mut ScriptCpu, spec: &McuItemSpec, desc: &McuDesc) {
    let Some(f) = spec.frequency else {
        return;
    };
    if spec.force_freq || desc.freq <= 0.0 {
        let c_per = if desc.cpu_cycle > 0.0 {
            desc.cpu_cycle
        } else {
            desc.inst_cycle.max(1.0)
        };
        cpu.ps_tick = ((1e12 * c_per / f).round() as u64).max(1);
    }
}

pub fn resolve_firmware(dir: Option<&Path>, program: &str) -> Option<PathBuf> {
    if program.is_empty() {
        return None;
    }
    let p = Path::new(program);
    let path = if p.is_absolute() {
        p.to_path_buf()
    } else if let Some(d) = dir {
        d.join(p)
    } else {
        p.to_path_buf()
    };
    if path.exists() {
        return Some(path);
    }

    // Fallback for sketches previously built into in-tree build directories:
    // e.g. <sketch_dir>/build/<board>/<sketch>.ino.hex
    // Fall back to <sketch_dir>/<sketch>.hex or <sketch_dir>/<sketch>.bin
    let mut curr = path.parent();
    while let Some(parent) = curr {
        if parent.file_name().map(|n| n == "build").unwrap_or(false) {
            if let Some(sketch_dir) = parent.parent() {
                let sketch_name = sketch_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let candidates = [
                    sketch_dir.join(format!("{sketch_name}.hex")),
                    sketch_dir.join(format!("{sketch_name}.bin")),
                ];
                for cand in candidates {
                    if cand.is_file() {
                        return Some(cand);
                    }
                }
            }
            break;
        }
        curr = parent.parent();
    }

    // Also check alongside the circuit directory if specified
    if let Some(d) = dir {
        if let Some(file_name) = path.file_name() {
            let cand = d.join(file_name);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }

    Some(path)
}

/// Source extensions Circuit Simulator can compile, in the order a firmware file is most likely
/// to have come from one (matching C++ `Chip::sourceExts`).
pub const SOURCE_EXTENSIONS: &[&str] = &["ino", "pde", "cpp", "c", "S", "asm", "gcb", "bas"];

/// Replace or change the file extension of a path (C++ `changeExt`).
pub fn change_extension(path: &Path, ext: &str) -> PathBuf {
    let ext = ext.strip_prefix('.').unwrap_or(ext);
    path.with_extension(ext)
}

/// Find the source file corresponding to a firmware path (C++ `Chip::firmwareSource`).
/// Checks `.ino`, `.pde`, `.cpp`, `.c`, `.S`, `.asm`, `.gcb`, `.bas` next to the firmware.
pub fn firmware_source(firmware: &Path) -> Option<PathBuf> {
    if firmware.as_os_str().is_empty() {
        return None;
    }

    // If the path itself is an existing file with a source extension, return it.
    if firmware.is_file() {
        if let Some(ext) = firmware.extension().and_then(|e| e.to_str()) {
            if SOURCE_EXTENSIONS
                .iter()
                .any(|&se| se.eq_ignore_ascii_case(ext))
            {
                return Some(firmware.to_path_buf());
            }
        }
    }

    // Try candidate extensions in priority order.
    for &ext in SOURCE_EXTENSIONS {
        // 1. firmware.with_extension(ext), e.g. "Blink.hex" -> "Blink.ino"
        let candidate = change_extension(firmware, ext);
        if candidate.is_file() {
            return Some(candidate);
        }

        // 2. If firmware had compound extensions (e.g. "Blink.ino.hex" or "Blink.cpp.bin"),
        // check if stripping the trailing extension yields an existing source file.
        if let Some(stem) = firmware.file_stem() {
            let stem_path = Path::new(stem);
            if let Some(stem_ext) = stem_path.extension().and_then(|e| e.to_str()) {
                if stem_ext.eq_ignore_ascii_case(ext) {
                    let candidate = if let Some(parent) = firmware.parent() {
                        parent.join(stem)
                    } else {
                        PathBuf::from(stem)
                    };
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    None
}

/// Build a netlist MCU from a `.sim1` `McuItem` (C++ `Mcu::construct` + props).
/// Scripted cores become [`InstantiatedMcu::Script`].
pub fn instantiate_any(
    id: &str,
    spec: &McuItemSpec,
    search: &SubcSearch,
) -> crate::Result<InstantiatedMcu> {
    let (desc, mcu_path) = load_desc_with(&spec.device, search)?;
    if desc.core == CoreKind::Scripted {
        let script = load_script_source(&desc, mcu_path.as_deref(), search);
        let mut cpu = ScriptCpu::from_mcu_desc(id, &desc, script);
        apply_freq_to_script(&mut cpu, spec, &desc);
        return Ok(InstantiatedMcu::Script(cpu));
    }
    let mut mcu = McuComp::from_device(
        Device::from_desc(id, desc).map_err(|e| crate::Error::Parse(e.to_string()))?,
    );
    apply_item_props(&mut mcu, spec, search);
    Ok(InstantiatedMcu::Native(mcu))
}

/// Native cores only (canvas chip / tests). Scripted `.mcu` files error.
pub fn instantiate_item(
    id: &str,
    spec: &McuItemSpec,
    search: &SubcSearch,
) -> crate::Result<McuComp> {
    match instantiate_any(id, spec, search)? {
        InstantiatedMcu::Native(mcu) => Ok(mcu),
        InstantiatedMcu::Script(_) => Err(crate::Error::Parse(format!(
            "MCU core `scripted` is not a native chip ({id})"
        ))),
    }
}

fn apply_item_props(mcu: &mut McuComp, spec: &McuItemSpec, search: &SubcSearch) {
    if let Some(f) = spec.frequency
        && (spec.force_freq || mcu.device.freq <= 0.0)
    {
        mcu.device.force_freq(f);
    }
    mcu.auto_load = spec.auto_load;
    mcu.force_freq = spec.force_freq;
    mcu.save_pgm = spec.save_pgm;
    mcu.firmware_dir = search.circuit_dir.clone();
    if spec.save_pgm {
        if let Some(pgm) = &spec.pgm {
            mcu.load_pgm_str(pgm);
        }
    } else if let Some(pro) = &spec.program {
        mcu.firmware = Some(pro.clone());
        if let Some(path) = mcu.firmware_path()
            && path.is_file()
        {
            let _ = mcu.load_firmware_file(&path);
        }
    }
}

/// C++ `{device}.package` / `{device}_LS.package` next to the `.mcu` file.
pub fn load_packages(device: &str, search: &SubcSearch) -> BTreeMap<String, Package> {
    let device = canonicalize_device(device);
    let mut list = BTreeMap::new();
    if let Some(xml) = search.memory_pkg.get(&device) {
        let (_, mut pkg) = convert_package(xml);
        if pkg.name.is_empty() {
            pkg.name = device.clone();
        }
        let key = format!("1- {device}_DIP");
        list.insert(key, pkg);
        return list;
    }
    let mut bases = Vec::new();
    if let Some(McuSource::File { mcu, package_base }) = find_mcu(&device, search) {
        bases.push(package_base);
        bases.push(mcu.with_extension(""));
        if let Some(parent) = mcu.parent() {
            bases.push(parent.join(&device));
        }
    }
    if let Some(d) = &search.circuit_dir {
        bases.push(d.join(&device).join(&device));
        bases.push(d.join("data").join(&device).join(&device));
        bases.push(d.join(&device));
    }
    for dir in &search.data_dirs {
        bases.push(dir.join(&device).join(&device));
        bases.push(dir.join(&device));
    }
    for base in bases {
        load_pkg_pair(&base, &device, &mut list);
        if !list.is_empty() {
            break;
        }
    }
    list
}

fn load_pkg_pair(base: &Path, device: &str, list: &mut BTreeMap<String, Package>) {
    let mut dip = base.as_os_str().to_os_string();
    dip.push(".package");
    let mut ls = base.as_os_str().to_os_string();
    ls.push("_LS.package");
    let dip = PathBuf::from(dip);
    let ls = PathBuf::from(ls);
    if dip.is_file()
        && let Ok(text) = std::fs::read_to_string(&dip)
    {
        let (_, mut pkg) = convert_package(&text);
        let key = format!("1- {device}_DIP");
        if pkg.name.is_empty() {
            pkg.name = device.to_string();
        }
        list.insert(key, pkg);
    }
    if ls.is_file()
        && let Ok(text) = std::fs::read_to_string(&ls)
    {
        let (_, mut pkg) = convert_package(&text);
        let key = format!("2- {device}_LS");
        if pkg.name.is_empty() {
            pkg.name = device.to_string();
        }
        list.insert(key, pkg);
    }
}

/// DIP outline from GPIO when the device has no `.package` (tests, incomplete data).
pub fn gpio_dip_package(mcu: &McuComp, instance_id: &str) -> Package {
    let gpio: Vec<&cs_mcu::GpioPin> = mcu.device.gpio_pins().collect();
    let n = gpio.len().max(1);
    let per_side = n.div_ceil(2).max(2);
    let width = 4;
    let height = (per_side as i32 + 1).max(4);
    let mut pins = Vec::with_capacity(n);
    for (i, g) in gpio.iter().enumerate() {
        let suffix = gpio_suffix(&g.name, instance_id).to_string();
        let side_i = i % per_side;
        let ypos = 8 + side_i as i32 * 8;
        let (xpos, angle) = if i < per_side {
            (-8, 180)
        } else {
            (width * 8 + 8, 0)
        };
        let label = if g.label.chars().all(|c| c.is_ascii_digit()) {
            suffix.clone()
        } else {
            g.label.clone()
        };
        pins.push(PkgPin {
            id: suffix,
            label,
            pin_type: String::new(),
            xpos,
            ypos,
            angle,
            length: 8,
            space: 0,
        });
    }
    Package {
        name: crate::subcircuit::device_from_id(instance_id),
        width,
        height,
        logic_symbol: false,
        border: false,
        pins,
        ..Package::default()
    }
}

fn gpio_suffix<'a>(full: &'a str, instance_id: &str) -> &'a str {
    full.strip_prefix(instance_id)
        .and_then(|s| s.strip_prefix('-'))
        .unwrap_or(full)
}

/// C++ `McuPort::getPin` plus PIC `RA0` → `PORTA0`.
pub fn match_gpio<'a>(
    mcu: &'a McuComp,
    instance_id: &str,
    pin_name: &str,
) -> Option<&'a cs_mcu::GpioPin> {
    if pin_name.is_empty() || pin_name == "0" {
        return None;
    }
    for port in mcu.device.ports() {
        if let Some(rest) = pin_name.strip_prefix(&port.name) {
            if let Ok(n) = rest.parse::<u8>()
                && let Some(p) = port.pins.iter().find(|p| p.number == n)
            {
                return Some(p);
            }
        } else if let Some(last_char) = port.name.chars().last() {
            let mut prefix = [0u8; 5];
            prefix[0] = b'P';
            let char_len = last_char.encode_utf8(&mut prefix[1..]).len();
            if let Ok(short_str) = std::str::from_utf8(&prefix[..1 + char_len])
                && let Some(rest) = pin_name.strip_prefix(short_str)
                && let Ok(n) = rest.parse::<u8>()
                && let Some(p) = port.pins.iter().find(|p| p.number == n)
            {
                return Some(p);
            }
        }
        if let Some(n) = pic_ra_number(&port.name, pin_name)
            && let Some(p) = port.pins.iter().find(|p| p.number == n)
        {
            return Some(p);
        }
        for p in &port.pins {
            let suffix = gpio_suffix(&p.name, instance_id);
            let stripped = suffix.strip_prefix(&port.name).unwrap_or(suffix);
            if suffix == pin_name || stripped == pin_name || p.label == pin_name {
                return Some(p);
            }
        }
    }
    None
}

/// PIC package ids `RA0` / `RB3` map onto `PORTA` / `PORTB`.
fn pic_ra_number(port: &str, pin_name: &str) -> Option<u8> {
    let b = pin_name.as_bytes();
    if b.len() < 3 || b[0] != b'R' {
        return None;
    }
    let letter = b[1] as char;
    if !port.ends_with(letter) {
        return None;
    }
    pin_name[2..].parse().ok()
}

fn remap_pkg_pins(pkg: &mut Package, mcu: &McuComp, instance_id: &str) {
    for pin in &mut pkg.pins {
        if pin.unused() {
            continue;
        }
        if let Some(g) = match_gpio(mcu, instance_id, &pin.id) {
            pin.id = gpio_suffix(&g.name, instance_id).to_string();
        }
    }
}

/// Package used to draw the MCU chip. Falls back to a GPIO DIP.
pub fn canvas_package(
    mcu: &McuComp,
    instance_id: &str,
    packages: &BTreeMap<String, Package>,
    logic_symbol: bool,
    named: Option<&str>,
) -> Package {
    let mut pkg = select_package(packages, logic_symbol, named)
        .cloned()
        .unwrap_or_else(|| gpio_dip_package(mcu, instance_id));
    remap_pkg_pins(&mut pkg, mcu, instance_id);
    pkg
}

/// Layout + live device for a canvas MCU chip.
#[derive(Clone, Debug)]
pub struct McuView {
    pub mcu: McuComp,
    pub package: Package,
    pub packages: BTreeMap<String, Package>,
    pub logic_symbol: bool,
}

pub fn instantiate_view(
    id: &str,
    spec: &McuItemSpec,
    search: &SubcSearch,
) -> crate::Result<McuView> {
    let mcu = match instantiate_item(id, spec, search) {
        Ok(m) => m,
        Err(_) => {
            let (desc, _) = load_desc_with(&spec.device, search)?;
            let mut mcu = McuComp::from_device(
                Device::from_desc(id, desc).map_err(|e| crate::Error::Parse(e.to_string()))?,
            );
            apply_item_props(&mut mcu, spec, search);
            mcu
        }
    };
    let packages = load_packages(&spec.device, search);
    let logic_symbol = spec.logic_symbol;
    let package = canvas_package(
        &mcu,
        id,
        &packages,
        logic_symbol,
        spec.package_name.as_deref(),
    );
    Ok(McuView {
        mcu,
        package,
        packages,
        logic_symbol,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_format_matches_cpp() {
        assert_eq!(dec_to_base(5, 10, 3), "005");
        assert_eq!(dec_to_base(5, 16, 2), "05");
        assert_eq!(dec_to_base(5, 2, 8), "00000101");
        assert_eq!(format_u8_watch(5), "005 0x05 00000101");
        assert_eq!(val2hex(0), "00");
        assert_eq!(val2hex(0xA), "0A");
        assert_eq!(val2hex(0x10), "10");
    }

    const PIC14: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="PORTA" addr="0x05" reset="0"/>
  </regblock>
  <regblock start="0x80" end="0x8F">
    <register name="TRISA" addr="0x85" reset="11111111"/>
  </regblock>
  <datablock start="0x0C" end="0x4F"/>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
</mcu>
"#;

    #[test]
    fn gpio_ids_match_package_aliases() {
        let search = SubcSearch::default().with_memory_mcu("pic14test", PIC14);
        let spec = McuItemSpec {
            device: "pic14test".into(),
            ..Default::default()
        };
        let mcu = instantiate_item("pic14test-1", &spec, &search).unwrap();
        assert!(match_gpio(&mcu, "pic14test-1", "PORTA0").is_some());
        assert!(match_gpio(&mcu, "pic14test-1", "PA0").is_some());
        assert!(match_gpio(&mcu, "pic14test-1", "RA0").is_some());
        let dip = gpio_dip_package(&mcu, "pic14test-1");
        assert_eq!(dip.pins.len(), 5);
        assert_eq!(dip.pins[0].id, "PORTA0");
        assert_eq!(dip.pins[0].angle, 180);
        assert_eq!(dip.pins.last().unwrap().angle, 0);
    }

    #[test]
    fn package_pin_ids_remap_to_gpio() {
        let pkg_xml = r#"<packageB name="DIP" width="4" height="6">
    <pin type="" xpos="-8" ypos="8" angle="180" length="8" id="RA0" label="RA0" />
    <pin type="" xpos="-8" ypos="16" angle="180" length="8" id="RA1" label="RA1" />
    <pin type="nc" xpos="40" ypos="8" angle="0" length="8" id="Vdd" label="Vdd" />
</packageB>
"#;
        let search = SubcSearch::default()
            .with_memory_mcu("pic14test", PIC14)
            .with_memory_pkg("pic14test", pkg_xml);
        let spec = McuItemSpec {
            device: "pic14test".into(),
            ..Default::default()
        };
        let view = instantiate_view("pic14test-1", &spec, &search).unwrap();
        assert_eq!(view.package.pins[0].id, "PORTA0");
        assert_eq!(view.package.pins[0].label, "RA0");
        assert_eq!(view.package.pins[1].id, "PORTA1");
        assert!(view.package.pins[2].unused());
        assert_eq!(view.package.pins[2].id, "Vdd");
    }

    #[test]
    fn rebind_rewrites_gpio_and_iopin_ids() {
        let search = SubcSearch::default().with_memory_mcu("pic14test", PIC14);
        let spec = McuItemSpec {
            device: "pic14test".into(),
            ..Default::default()
        };
        let mut mcu = instantiate_item("pic14test-1", &spec, &search).unwrap();
        mcu.rebind_id("pic14test-2");
        assert!(mcu.pin_ids().iter().any(|id| id == "pic14test-2-PORTA0"));
        assert!(
            mcu.device
                .gpio_pins()
                .any(|g| g.name == "pic14test-2-PORTA0")
        );
        assert_eq!(mcu.device.id, "pic14test-2");
    }

    #[test]
    fn catalog_xml_resolves_mcu_and_shared_package() {
        let dir = std::env::temp_dir().join(format!("cs-mcu-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("AVR").join("tiny13")).unwrap();
        std::fs::write(
            dir.join("avr.xml"),
            r#"<itemlib>
            <itemset category="AVR/attiny" type="MCU">
                <item name="tiny13" package="AVR/tiny13/tiny13" data="AVR/tiny13" />
            </itemset>
            </itemlib>"#,
        )
        .unwrap();
        std::fs::write(dir.join("AVR").join("tiny13.mcu"), PIC14).unwrap();
        std::fs::write(
            dir.join("AVR").join("tiny13").join("tiny13.package"),
            r#"<packageB name="DIP" width="4" height="6">
    <pin type="" xpos="-8" ypos="24" angle="180" length="8" id="RA0" label="RA0" />
    <pin type="" xpos="40" ypos="8" angle="0" length="8" id="RA1" label="RA1" />
</packageB>"#,
        )
        .unwrap();
        let mut catalog = crate::catalog::Catalog::new();
        catalog.load_dir(&dir);
        let search = SubcSearch::default().with_catalog(catalog);
        let spec = McuItemSpec {
            device: "tiny13".into(),
            ..Default::default()
        };
        let view = instantiate_view("tiny13-1", &spec, &search).unwrap();
        assert_eq!(view.mcu.device.gpio_pins().count(), 5);
        assert_eq!(view.package.pins[0].id, "PORTA0");
        assert_eq!(view.package.pins[0].xpos, -8);
        assert_eq!(view.package.pins[0].ypos, 24);
        assert_eq!(view.package.pins[1].id, "PORTA1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn catalog_folder_default_package_path() {
        let dir = std::env::temp_dir().join(format!("cs-mcu-cat-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("MCS65").join("6502")).unwrap();
        std::fs::write(
            dir.join("mcs65.xml"),
            r#"<itemlib>
            <itemset category="MCS65" type="MCU" folder="MCS65">
                <item name="6502" />
            </itemset>
            </itemlib>"#,
        )
        .unwrap();
        std::fs::write(dir.join("MCS65").join("6502").join("6502.mcu"), PIC14).unwrap();
        let mut catalog = crate::catalog::Catalog::new();
        catalog.load_dir(&dir);
        let search = SubcSearch::default().with_catalog(catalog);
        let spec = McuItemSpec {
            device: "6502".into(),
            ..Default::default()
        };
        let mcu = instantiate_item("6502-1", &spec, &search).unwrap();
        assert_eq!(mcu.device.gpio_pins().count(), 5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn instantiate_scripted_core_from_xml() {
        let xml = r#"
<mcu core="scripted" script="cpu.as" data="256" prog="64">
  <ioport name="P" pins="TX,RX,MOSI,MISO,SCK,SS,SDA,SCL"/>
  <usart name="UART0"><trunit type="tx" pin="TX"/><trunit type="rx" pin="RX"/></usart>
  <spi name="SPI0" pins="MOSI,MISO,SCK,SS"/>
  <twi name="TWI0" pins="SDA,SCL"/>
</mcu>
"#;
        let search = SubcSearch::default()
            .with_memory_mcu("scriptuart", xml)
            .with_memory_script("cpu.as", "void reset() { UART0.setBaudRate(9600); }");
        let spec = McuItemSpec {
            device: "scriptuart".into(),
            ..Default::default()
        };
        match instantiate_any("scriptuart-1", &spec, &search).unwrap() {
            InstantiatedMcu::Script(cpu) => {
                assert_eq!(cpu.usarts.len(), 1);
                assert_eq!(cpu.usarts[0].name, "UART0");
                assert_eq!(cpu.spis.len(), 1);
                assert_eq!(cpu.twis.len(), 1);
            }
            InstantiatedMcu::Native(_) => panic!("expected scripted core"),
        }
        assert!(instantiate_item("scriptuart-1", &spec, &search).is_err());
    }

    #[test]
    fn test_change_extension() {
        let p = Path::new("/path/to/Blink.hex");
        assert_eq!(
            change_extension(p, "ino"),
            PathBuf::from("/path/to/Blink.ino")
        );
        assert_eq!(
            change_extension(p, ".ino"),
            PathBuf::from("/path/to/Blink.ino")
        );
        assert_eq!(
            change_extension(p, "cpp"),
            PathBuf::from("/path/to/Blink.cpp")
        );
    }

    #[test]
    fn test_firmware_source_resolution() {
        let temp = std::env::temp_dir().join("cs_test_firmware_source");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        let hex_path = temp.join("Blink.hex");
        let ino_path = temp.join("Blink.ino");
        let cpp_path = temp.join("Blink.cpp");

        std::fs::write(&hex_path, ":10000000...").unwrap();

        // 1. No source files exist yet
        assert_eq!(firmware_source(&hex_path), None);

        // 2. .cpp exists
        std::fs::write(&cpp_path, "int main() {}").unwrap();
        assert_eq!(firmware_source(&hex_path), Some(cpp_path.clone()));

        // 3. .ino exists (higher priority than .cpp)
        std::fs::write(&ino_path, "void setup() {} void loop() {}").unwrap();
        assert_eq!(firmware_source(&hex_path), Some(ino_path.clone()));

        // 4. Directly passing .ino returns itself
        assert_eq!(firmware_source(&ino_path), Some(ino_path.clone()));

        // 5. Compound extensions: Blink.ino.hex -> Blink.ino
        let compound_hex = temp.join("Blink.ino.hex");
        std::fs::write(&compound_hex, ":10000000...").unwrap();
        assert_eq!(firmware_source(&compound_hex), Some(ino_path.clone()));

        let _ = std::fs::remove_dir_all(&temp);
    }
}
