//! MCU Source-Level Debugger, Symbols, Watch Evaluator, and GDB RSP Server.

pub mod asdebugger;
pub mod gdb;
pub mod gdb_client;
pub mod symbols;
pub mod varset;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub use asdebugger::{AsBreakpoint, AsDebugState, AsDebugger};
pub use gdb::GdbServer;
pub use gdb_client::QemuGdbClient;
pub use symbols::{
    DebugSymbols, DebugVariable, ElfParser, FunctionSymbol, LstKind, LstParser, SourceLocation,
};
pub use varset::{VarFormat, VarSet, VarType, VarValue, WatchEntry};

use crate::mcu::{McuComp, McuSnap};

/// Control command sent from the UI / DebugSession to an attached external
/// debugger backend (e.g. `QemuGdbClient` for ESP32 / STM32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteDebugCmd {
    Resume,
    Pause,
    StepInto,
    StepOver,
    StepOut,
    Stop,
}

/// Reason why execution stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreakReason {
    Breakpoint {
        address: u32,
        location: Option<SourceLocation>,
    },
    StepCompleted,
    ManualPause,
    Trap,
    Watchpoint {
        address: u32,
        old_val: u32,
        new_val: u32,
    },
}

/// Execution state of the debug session.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DebugState {
    #[default]
    Idle,
    Running,
    Paused(BreakReason),
    SteppingInto,
    SteppingOver {
        target_sp: u32,
        exit_pc: Option<u32>,
    },
    SteppingOut {
        target_sp: u32,
    },
}

/// Thread-safe coordinator for MCU debugging sessions.
pub struct DebugSession {
    state: Mutex<DebugState>,
    symbols: Mutex<DebugSymbols>,
    line_breakpoints: Mutex<BTreeSet<(PathBuf, usize)>>,
    addr_breakpoints: Mutex<BTreeSet<u32>>,
    watch_entries: Mutex<Vec<WatchEntry>>,
    active_location: Mutex<Option<SourceLocation>>,
    active_mcu: Mutex<Option<McuComp>>,
    active: AtomicBool,
    paused: AtomicBool,
    is_global: bool,
    symbols_generation: AtomicU64,
    breakpoints_generation: AtomicU64,
    state_generation: AtomicU64,
    remote_attached: AtomicBool,
    remote_cmds: Mutex<VecDeque<RemoteDebugCmd>>,
    remote_pc: AtomicU32,
    remote_mem: Mutex<BTreeMap<u32, Vec<u8>>>,
    remote_mem_generation: AtomicU64,
    remote_mem_refresh_req: AtomicBool,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self {
            state: Mutex::new(DebugState::Idle),
            symbols: Mutex::new(DebugSymbols::default()),
            line_breakpoints: Mutex::new(BTreeSet::new()),
            addr_breakpoints: Mutex::new(BTreeSet::new()),
            watch_entries: Mutex::new(Vec::new()),
            active_location: Mutex::new(None),
            active_mcu: Mutex::new(None),
            active: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            is_global: false,
            symbols_generation: AtomicU64::new(0),
            breakpoints_generation: AtomicU64::new(0),
            state_generation: AtomicU64::new(0),
            remote_attached: AtomicBool::new(false),
            remote_cmds: Mutex::new(VecDeque::new()),
            remote_pc: AtomicU32::new(0),
            remote_mem: Mutex::new(BTreeMap::new()),
            remote_mem_generation: AtomicU64::new(0),
            remote_mem_refresh_req: AtomicBool::new(false),
        }
    }
}

static GLOBAL_DEBUG_ACTIVE: AtomicBool = AtomicBool::new(false);
static GLOBAL_DEBUG_PAUSED: AtomicBool = AtomicBool::new(false);

impl DebugSession {
    pub fn global() -> &'static DebugSession {
        static INSTANCE: OnceLock<DebugSession> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            is_global: true,
            ..Self::default()
        })
    }

    #[inline(always)]
    pub fn is_active_global() -> bool {
        GLOBAL_DEBUG_ACTIVE.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn is_paused_global() -> bool {
        GLOBAL_DEBUG_PAUSED.load(Ordering::Relaxed)
    }

    fn set_state_internal(&self, new_state: DebugState) {
        let paused = matches!(new_state, DebugState::Paused(_));
        self.paused.store(paused, Ordering::Release);
        if self.is_global {
            GLOBAL_DEBUG_PAUSED.store(paused, Ordering::Release);
        }
        *self.state.lock().unwrap() = new_state;
        self.state_generation.fetch_add(1, Ordering::Release);
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Release);
        if self.is_global {
            GLOBAL_DEBUG_ACTIVE.store(active, Ordering::Release);
        }
        self.state_generation.fetch_add(1, Ordering::Release);
        if !active {
            self.set_state_internal(DebugState::Idle);
            *self.active_location.lock().unwrap() = None;
        }
    }

    pub fn state(&self) -> DebugState {
        self.state.lock().unwrap().clone()
    }

    pub fn state_generation(&self) -> u64 {
        self.state_generation.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn load_symbols(&self, syms: DebugSymbols) {
        *self.symbols.lock().unwrap() = syms;
        self.symbols_generation.fetch_add(1, Ordering::Release);
        self.resync_addr_breakpoints();
    }

    pub fn symbols_generation(&self) -> u64 {
        self.symbols_generation.load(Ordering::Acquire)
    }

    pub fn breakpoints_generation(&self) -> u64 {
        self.breakpoints_generation.load(Ordering::Acquire)
    }

    pub fn symbols(&self) -> DebugSymbols {
        self.symbols.lock().unwrap().clone()
    }

    /// Clone only the variable table, not the full line-number maps.
    pub fn debug_variables(&self) -> Vec<DebugVariable> {
        self.symbols.lock().unwrap().variables.clone()
    }

    fn resync_addr_breakpoints(&self) {
        let line_bps: Vec<(PathBuf, usize)> = self
            .line_breakpoints
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect();
        let resolved: BTreeSet<u32> = {
            let syms = self.symbols.lock().unwrap();
            let mut set = BTreeSet::new();
            for (file, line) in &line_bps {
                for addr in syms.resolve_breakpoint_addrs(file, *line) {
                    set.insert(addr);
                }
            }
            set
        };
        *self.addr_breakpoints.lock().unwrap() = resolved;
        self.breakpoints_generation.fetch_add(1, Ordering::Release);
    }

    pub fn add_breakpoint(&self, file: impl Into<PathBuf>, line: usize) {
        let file_path = file.into();
        self.line_breakpoints
            .lock()
            .unwrap()
            .insert((file_path, line));
        self.resync_addr_breakpoints();
    }

    pub fn remove_breakpoint(&self, file: &Path, line: usize) {
        let want_name = file.file_name().map(|n| n.to_os_string());
        self.line_breakpoints.lock().unwrap().retain(|(f, l)| {
            if *l != line {
                return true;
            }
            if f == file {
                return false;
            }
            if let Some(ref wn) = want_name {
                if f.file_name() == Some(wn.as_os_str()) {
                    return false;
                }
            }
            true
        });
        self.resync_addr_breakpoints();
    }

    /// Replace all line breakpoints associated with `file` with `lines`.
    pub fn set_breakpoints_for_file(&self, file: &Path, lines: &[usize]) {
        let want_name = file.file_name().map(|n| n.to_os_string());
        {
            let mut bps = self.line_breakpoints.lock().unwrap();
            bps.retain(|(f, _)| {
                if f == file {
                    return false;
                }
                if let Some(ref wn) = want_name {
                    if f.file_name() == Some(wn.as_os_str()) {
                        return false;
                    }
                }
                true
            });
            for &line in lines {
                if line > 0 {
                    bps.insert((file.to_path_buf(), line));
                }
            }
        }
        self.resync_addr_breakpoints();
    }

    pub fn toggle_breakpoint(&self, file: impl Into<PathBuf>, line: usize) -> bool {
        let file_path = file.into();
        let exists = self.has_breakpoint(&file_path, line);
        if exists {
            self.remove_breakpoint(&file_path, line);
            false
        } else {
            self.add_breakpoint(file_path, line);
            true
        }
    }

    pub fn has_breakpoint(&self, file: &Path, line: usize) -> bool {
        let bps = self.line_breakpoints.lock().unwrap();
        if bps.contains(&(file.to_path_buf(), line)) {
            return true;
        }
        if let Some(name) = file.file_name() {
            for (f, l) in bps.iter() {
                if *l == line && f.file_name() == Some(name) {
                    return true;
                }
            }
        }
        false
    }

    pub fn add_addr_breakpoint(&self, pc: u32) {
        self.addr_breakpoints.lock().unwrap().insert(pc);
        self.breakpoints_generation.fetch_add(1, Ordering::Release);
    }

    pub fn remove_addr_breakpoint(&self, pc: u32) {
        self.addr_breakpoints.lock().unwrap().remove(&pc);
        self.breakpoints_generation.fetch_add(1, Ordering::Release);
    }

    pub fn has_addr_breakpoint(&self, pc: u32) -> bool {
        self.addr_breakpoints.lock().unwrap().contains(&pc)
    }

    pub fn addr_breakpoints(&self) -> Vec<u32> {
        self.addr_breakpoints
            .lock()
            .unwrap()
            .iter()
            .copied()
            .collect()
    }

    pub fn active_location(&self) -> Option<SourceLocation> {
        self.active_location.lock().unwrap().clone()
    }

    fn push_remote_cmd(&self, cmd: RemoteDebugCmd) {
        if self.remote_attached.load(Ordering::Acquire) {
            self.remote_cmds.lock().unwrap().push_back(cmd);
        }
    }

    pub fn set_remote_attached(&self, attached: bool) {
        self.remote_attached.store(attached, Ordering::Release);
        if !attached {
            self.remote_cmds.lock().unwrap().clear();
            self.remote_pc.store(0, Ordering::Release);
            self.remote_mem.lock().unwrap().clear();
            self.remote_mem_generation.fetch_add(1, Ordering::Release);
        }
    }

    pub fn is_remote_attached(&self) -> bool {
        self.remote_attached.load(Ordering::Acquire)
    }

    pub fn pop_remote_cmd(&self) -> Option<RemoteDebugCmd> {
        self.remote_cmds.lock().unwrap().pop_front()
    }

    pub fn set_remote_pc(&self, pc: u32) {
        self.remote_pc.store(pc, Ordering::Release);
    }

    pub fn remote_pc(&self) -> u32 {
        self.remote_pc.load(Ordering::Acquire)
    }

    pub fn update_remote_mem(&self, entries: Vec<(u32, Vec<u8>)>) {
        let mut mem = self.remote_mem.lock().unwrap();
        for (addr, data) in entries {
            mem.insert(addr, data);
        }
        drop(mem);
        self.remote_mem_generation.fetch_add(1, Ordering::Release);
    }

    pub fn remote_mem_generation(&self) -> u64 {
        self.remote_mem_generation.load(Ordering::Acquire)
    }

    pub fn request_remote_mem_refresh(&self) {
        self.remote_mem_refresh_req.store(true, Ordering::Release);
    }

    pub fn take_remote_mem_refresh(&self) -> bool {
        self.remote_mem_refresh_req.swap(false, Ordering::AcqRel)
    }

    /// Resolve all current watch entries to `(address, byte_len)` pairs so an
    /// attached remote debugger (QEMU GDB) knows which memory regions to read.
    pub fn watch_mem_ranges(&self) -> Vec<(u32, usize)> {
        let entries = self.watch_entries.lock().unwrap().clone();
        let syms = self.symbols.lock().unwrap();
        let mut ranges = Vec::new();
        for e in &entries {
            if let Some(addr) = VarSet::resolve_watch_address(&e.expr, Some(&syms), None) {
                let len = e.var_type.size_in_bytes().clamp(1, 256);
                ranges.push((addr, len));
            }
        }
        ranges
    }

    /// Pause execution at a specific target PC (used by external QEMU GDB backend).
    pub fn set_paused_at(&self, pc: u32, reason: BreakReason) {
        self.remote_pc.store(pc, Ordering::Release);
        let loc = {
            let syms = self.symbols.lock().unwrap();
            syms.lookup_address(pc).cloned()
        };
        if let Some(ref l) = loc {
            *self.active_location.lock().unwrap() = Some(l.clone());
        }
        let reason = match reason {
            BreakReason::Breakpoint { address, location } => BreakReason::Breakpoint {
                address,
                location: location.or(loc),
            },
            other => other,
        };
        self.set_state_internal(DebugState::Paused(reason));
    }

    pub fn step_into(&self) {
        self.set_active(true);
        self.set_state_internal(DebugState::SteppingInto);
        self.push_remote_cmd(RemoteDebugCmd::StepInto);
    }

    pub fn step_over(&self) {
        self.set_active(true);
        let sp = self
            .with_mcu(|mcu| mcu.as_ref().map(|m| m.device.sp()))
            .flatten()
            .unwrap_or(0);
        let ret_addr = self
            .with_mcu(|mcu| mcu.as_ref().map(|m| m.device.ret_addr()))
            .flatten()
            .unwrap_or(0);
        let exit_pc = if ret_addr != 0 { Some(ret_addr) } else { None };
        self.set_state_internal(DebugState::SteppingOver {
            target_sp: u32::from(sp),
            exit_pc,
        });
        self.push_remote_cmd(RemoteDebugCmd::StepOver);
    }

    pub fn step_over_target(&self, target_sp: u32, exit_pc: Option<u32>) {
        self.set_active(true);
        self.set_state_internal(DebugState::SteppingOver { target_sp, exit_pc });
        self.push_remote_cmd(RemoteDebugCmd::StepOver);
    }

    pub fn step_out(&self) {
        self.set_active(true);
        let sp = self
            .with_mcu(|mcu| mcu.as_ref().map(|m| m.device.sp()))
            .flatten()
            .unwrap_or(0);
        self.set_state_internal(DebugState::SteppingOut {
            target_sp: u32::from(sp),
        });
        self.push_remote_cmd(RemoteDebugCmd::StepOut);
    }

    pub fn step_out_target(&self, target_sp: u32) {
        self.set_active(true);
        self.set_state_internal(DebugState::SteppingOut { target_sp });
        self.push_remote_cmd(RemoteDebugCmd::StepOut);
    }

    pub fn pause(&self) {
        self.set_active(true);
        self.set_state_internal(DebugState::Paused(BreakReason::ManualPause));
        self.push_remote_cmd(RemoteDebugCmd::Pause);
    }

    pub fn resume(&self) {
        self.set_active(true);
        self.set_state_internal(DebugState::Running);
        self.push_remote_cmd(RemoteDebugCmd::Resume);
    }

    pub fn stop(&self) {
        self.push_remote_cmd(RemoteDebugCmd::Stop);
        self.set_active(false);
        self.set_state_internal(DebugState::Idle);
        *self.active_location.lock().unwrap() = None;
    }

    /// Step check hook called on each MCU step instruction.
    /// Returns Some(reason) if execution should break/pause.
    pub fn on_mcu_step(
        &self,
        pc: u32,
        prev_pc: u32,
        sp: u16,
        ret_addr: u32,
    ) -> Option<BreakReason> {
        if !self.is_active() {
            return None;
        }

        let loc = {
            let syms = self.symbols.lock().unwrap();
            syms.lookup_address(pc).cloned()
        };
        if let Some(ref l) = loc {
            *self.active_location.lock().unwrap() = Some(l.clone());
        }

        // 1. Check address breakpoints
        if self.has_addr_breakpoint(pc) {
            let reason = BreakReason::Breakpoint {
                address: pc,
                location: loc.clone(),
            };
            self.set_state_internal(DebugState::Paused(reason.clone()));
            return Some(reason);
        }

        // 2. Check line breakpoints (only when entering a new source line or exact hit)
        if let Some(ref l) = loc {
            if self.has_breakpoint(&l.file, l.line) {
                let prev_same_line = {
                    let syms = self.symbols.lock().unwrap();
                    syms.lookup_address(prev_pc)
                        .is_some_and(|pl| pl.line == l.line && pl.file == l.file)
                };
                if !prev_same_line {
                    let reason = BreakReason::Breakpoint {
                        address: pc,
                        location: loc.clone(),
                    };
                    self.set_state_internal(DebugState::Paused(reason.clone()));
                    return Some(reason);
                }
            }
        }

        // 3. Check stepping modes
        let step_pause = {
            let mut st = self.state.lock().unwrap();
            match *st {
                DebugState::SteppingInto => {
                    if pc != prev_pc {
                        let reason = BreakReason::StepCompleted;
                        *st = DebugState::Paused(reason.clone());
                        Some(reason)
                    } else {
                        None
                    }
                }
                DebugState::SteppingOver { target_sp, exit_pc } => {
                    if let Some(exit) = exit_pc {
                        if pc == exit {
                            let reason = BreakReason::StepCompleted;
                            *st = DebugState::Paused(reason.clone());
                            Some(reason)
                        } else {
                            None
                        }
                    } else if (target_sp == 0 || u32::from(sp) >= target_sp) && pc != prev_pc {
                        let reason = BreakReason::StepCompleted;
                        *st = DebugState::Paused(reason.clone());
                        Some(reason)
                    } else {
                        None
                    }
                }
                DebugState::SteppingOut { target_sp } => {
                    if (target_sp != 0 && u32::from(sp) >= target_sp)
                        || (ret_addr != 0 && pc == ret_addr)
                    {
                        let reason = BreakReason::StepCompleted;
                        *st = DebugState::Paused(reason.clone());
                        Some(reason)
                    } else {
                        None
                    }
                }
                DebugState::Paused(ref r) => Some(r.clone()),
                DebugState::Running | DebugState::Idle => None,
            }
        };
        if let Some(reason) = step_pause {
            self.paused.store(true, Ordering::Release);
            if self.is_global {
                GLOBAL_DEBUG_PAUSED.store(true, Ordering::Release);
            }
            self.state_generation.fetch_add(1, Ordering::Release);
            return Some(reason);
        }

        None
    }

    pub fn set_active_mcu(&self, mcu: Option<McuComp>) {
        *self.active_mcu.lock().unwrap() = mcu;
    }

    pub fn with_mcu<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Option<McuComp>) -> R,
    {
        let guard = self.active_mcu.lock().unwrap();
        Some(f(&guard))
    }

    pub fn with_mcu_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Option<McuComp>) -> R,
    {
        let mut guard = self.active_mcu.lock().unwrap();
        Some(f(&mut guard))
    }

    pub fn add_watch(&self, entry: WatchEntry) {
        let mut guard = self.watch_entries.lock().unwrap();
        // Replace existing watch with same name if already present
        if let Some(pos) = guard
            .iter()
            .position(|w| w.name.eq_ignore_ascii_case(&entry.name))
        {
            guard[pos] = entry;
        } else {
            guard.push(entry);
        }
        drop(guard);
        self.request_remote_mem_refresh();
    }

    pub fn remove_watch(&self, name: &str) {
        self.watch_entries
            .lock()
            .unwrap()
            .retain(|w| !w.name.eq_ignore_ascii_case(name));
    }

    pub fn clear_watches(&self) {
        self.watch_entries.lock().unwrap().clear();
    }

    pub fn watches(&self) -> Vec<WatchEntry> {
        self.watch_entries.lock().unwrap().clone()
    }

    pub fn evaluate_watches(&self, snap: &McuSnap) -> Vec<VarValue> {
        let entries = self.watch_entries.lock().unwrap().clone();
        let syms = self.symbols.lock().unwrap();
        let remote_mem = self.remote_mem.lock().unwrap();
        entries
            .iter()
            .map(|e| {
                VarSet::eval_snap_with_remote(e, snap, Some(&syms), Some(&remote_mem))
                    .unwrap_or_else(|| {
                        let resolved_addr =
                            VarSet::resolve_watch_address(&e.expr, Some(&syms), Some(snap));
                        VarValue {
                            name: e.name.clone(),
                            expr: e.expr.clone(),
                            raw: 0,
                            formatted: "---".to_string(),
                            hex: "---".to_string(),
                            dec: "---".to_string(),
                            bin: "---".to_string(),
                            type_name: e.var_type.type_name().to_string(),
                            address: resolved_addr,
                        }
                    })
            })
            .collect()
    }
}
