//! MCU Source-Level Debugger, Symbols, Watch Evaluator, and GDB RSP Server.

pub mod asdebugger;
pub mod gdb;
pub mod symbols;
pub mod varset;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub use asdebugger::{AsBreakpoint, AsDebugState, AsDebugger};
pub use gdb::GdbServer;
pub use symbols::{
    DebugSymbols, DebugVariable, ElfParser, FunctionSymbol, LstKind, LstParser, SourceLocation,
};
pub use varset::{VarFormat, VarSet, VarType, VarValue, WatchEntry};

use crate::mcu::{McuComp, McuSnap};

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
    symbols_generation: AtomicU64,
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
            symbols_generation: AtomicU64::new(0),
        }
    }
}

static GLOBAL_DEBUG_ACTIVE: AtomicBool = AtomicBool::new(false);
static GLOBAL_DEBUG_PAUSED: AtomicBool = AtomicBool::new(false);

impl DebugSession {
    pub fn global() -> &'static DebugSession {
        static INSTANCE: OnceLock<DebugSession> = OnceLock::new();
        INSTANCE.get_or_init(DebugSession::default)
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
        GLOBAL_DEBUG_PAUSED.store(paused, Ordering::Release);
        *self.state.lock().unwrap() = new_state;
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Release);
        GLOBAL_DEBUG_ACTIVE.store(active, Ordering::Release);
        if !active {
            self.set_state_internal(DebugState::Idle);
            *self.active_location.lock().unwrap() = None;
        }
    }

    pub fn state(&self) -> DebugState {
        self.state.lock().unwrap().clone()
    }

    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn load_symbols(&self, syms: DebugSymbols) {
        *self.symbols.lock().unwrap() = syms;
        self.symbols_generation.fetch_add(1, Ordering::Release);
    }

    pub fn symbols_generation(&self) -> u64 {
        self.symbols_generation.load(Ordering::Acquire)
    }

    pub fn symbols(&self) -> DebugSymbols {
        self.symbols.lock().unwrap().clone()
    }

    /// Clone only the variable table, not the full line-number maps.
    pub fn debug_variables(&self) -> Vec<DebugVariable> {
        self.symbols.lock().unwrap().variables.clone()
    }

    pub fn add_breakpoint(&self, file: impl Into<PathBuf>, line: usize) {
        let file_path = file.into();
        self.line_breakpoints
            .lock()
            .unwrap()
            .insert((file_path.clone(), line));

        // Synchronize with flash addresses from symbols
        let syms = self.symbols.lock().unwrap();
        for &addr in syms.lookup_lines(&file_path, line) {
            self.addr_breakpoints.lock().unwrap().insert(addr);
        }
    }

    pub fn remove_breakpoint(&self, file: &Path, line: usize) {
        self.line_breakpoints
            .lock()
            .unwrap()
            .remove(&(file.to_path_buf(), line));

        let syms = self.symbols.lock().unwrap();
        for &addr in syms.lookup_lines(file, line) {
            self.addr_breakpoints.lock().unwrap().remove(&addr);
        }
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
    }

    pub fn remove_addr_breakpoint(&self, pc: u32) {
        self.addr_breakpoints.lock().unwrap().remove(&pc);
    }

    pub fn has_addr_breakpoint(&self, pc: u32) -> bool {
        self.addr_breakpoints.lock().unwrap().contains(&pc)
    }

    pub fn active_location(&self) -> Option<SourceLocation> {
        self.active_location.lock().unwrap().clone()
    }

    pub fn step_into(&self) {
        self.set_active(true);
        self.set_state_internal(DebugState::SteppingInto);
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
    }

    pub fn step_over_target(&self, target_sp: u32, exit_pc: Option<u32>) {
        self.set_active(true);
        self.set_state_internal(DebugState::SteppingOver { target_sp, exit_pc });
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
    }

    pub fn step_out_target(&self, target_sp: u32) {
        self.set_active(true);
        self.set_state_internal(DebugState::SteppingOut { target_sp });
    }

    pub fn pause(&self) {
        self.set_active(true);
        self.set_state_internal(DebugState::Paused(BreakReason::ManualPause));
    }

    pub fn resume(&self) {
        self.set_active(true);
        self.set_state_internal(DebugState::Running);
    }

    pub fn stop(&self) {
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

        let syms = self.symbols.lock().unwrap();
        let loc = syms.lookup_address(pc).cloned();
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

        // 2. Check line breakpoints
        if let Some(ref l) = loc {
            if self.has_breakpoint(&l.file, l.line) {
                let reason = BreakReason::Breakpoint {
                    address: pc,
                    location: loc.clone(),
                };
                self.set_state_internal(DebugState::Paused(reason.clone()));
                return Some(reason);
            }
        }

        // 3. Check stepping modes
        let mut st = self.state.lock().unwrap();
        match *st {
            DebugState::SteppingInto => {
                if pc != prev_pc {
                    let reason = BreakReason::StepCompleted;
                    *st = DebugState::Paused(reason.clone());
                    self.paused.store(true, Ordering::Release);
                    return Some(reason);
                }
            }
            DebugState::SteppingOver { target_sp, exit_pc } => {
                if let Some(exit) = exit_pc {
                    if pc == exit {
                        let reason = BreakReason::StepCompleted;
                        *st = DebugState::Paused(reason.clone());
                        self.paused.store(true, Ordering::Release);
                        return Some(reason);
                    }
                } else if (target_sp == 0 || u32::from(sp) >= target_sp) && pc != prev_pc {
                    let reason = BreakReason::StepCompleted;
                    *st = DebugState::Paused(reason.clone());
                    self.paused.store(true, Ordering::Release);
                    return Some(reason);
                }
            }
            DebugState::SteppingOut { target_sp } => {
                if (target_sp != 0 && u32::from(sp) >= target_sp)
                    || (ret_addr != 0 && pc == ret_addr)
                {
                    let reason = BreakReason::StepCompleted;
                    *st = DebugState::Paused(reason.clone());
                    self.paused.store(true, Ordering::Release);
                    return Some(reason);
                }
            }
            DebugState::Paused(ref r) => {
                return Some(r.clone());
            }
            DebugState::Running | DebugState::Idle => {}
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
        let entries = self.watch_entries.lock().unwrap();
        let syms = self.symbols.lock().unwrap();
        entries
            .iter()
            .map(|e| {
                VarSet::eval_snap(e, snap, Some(&syms)).unwrap_or_else(|| VarValue {
                    name: e.name.clone(),
                    expr: e.expr.clone(),
                    raw: 0,
                    formatted: "---".to_string(),
                    hex: "---".to_string(),
                    dec: "---".to_string(),
                    bin: "---".to_string(),
                    type_name: e.var_type.type_name().to_string(),
                    address: None,
                })
            })
            .collect()
    }
}
