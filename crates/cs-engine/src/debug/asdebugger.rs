//! AngelScript source-level debugger bridge (`asdebugger`).
//! Hooks script line execution, breakpoints, and variable inspection inside `.as` scripts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use super::symbols::SourceLocation;

/// Breakpoint in an AngelScript script file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsBreakpoint {
    pub file: PathBuf,
    pub line: usize,
}

/// Execution state of AngelScript debugger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AsDebugState {
    #[default]
    Idle,
    Running,
    Paused,
    SteppingInto,
    SteppingOver {
        target_depth: usize,
    },
    SteppingOut {
        target_depth: usize,
    },
}

/// AngelScript debugger session coordinator.
pub struct AsDebugger {
    state: Mutex<AsDebugState>,
    breakpoints: Mutex<BTreeSet<AsBreakpoint>>,
    current_location: Mutex<Option<SourceLocation>>,
    call_depth: Mutex<usize>,
    variables: Mutex<BTreeMap<String, String>>,
    active: AtomicBool,
}

impl Default for AsDebugger {
    fn default() -> Self {
        Self {
            state: Mutex::new(AsDebugState::Idle),
            breakpoints: Mutex::new(BTreeSet::new()),
            current_location: Mutex::new(None),
            call_depth: Mutex::new(0),
            variables: Mutex::new(BTreeMap::new()),
            active: AtomicBool::new(false),
        }
    }
}

impl AsDebugger {
    pub fn global() -> &'static AsDebugger {
        static INSTANCE: OnceLock<AsDebugger> = OnceLock::new();
        INSTANCE.get_or_init(AsDebugger::default)
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
        if !active {
            *self.state.lock().unwrap() = AsDebugState::Idle;
            *self.current_location.lock().unwrap() = None;
        }
    }

    pub fn state(&self) -> AsDebugState {
        *self.state.lock().unwrap()
    }

    pub fn add_breakpoint(&self, file: impl Into<PathBuf>, line: usize) {
        self.breakpoints.lock().unwrap().insert(AsBreakpoint {
            file: file.into(),
            line,
        });
    }

    pub fn remove_breakpoint(&self, file: &Path, line: usize) {
        self.breakpoints.lock().unwrap().remove(&AsBreakpoint {
            file: file.to_path_buf(),
            line,
        });
    }

    pub fn has_breakpoint(&self, file: &Path, line: usize) -> bool {
        let bps = self.breakpoints.lock().unwrap();
        if bps.contains(&AsBreakpoint {
            file: file.to_path_buf(),
            line,
        }) {
            return true;
        }
        if let Some(name) = file.file_name() {
            for bp in bps.iter() {
                if bp.line == line && bp.file.file_name() == Some(name) {
                    return true;
                }
            }
        }
        false
    }

    pub fn current_location(&self) -> Option<SourceLocation> {
        self.current_location.lock().unwrap().clone()
    }

    pub fn step_into(&self) {
        *self.state.lock().unwrap() = AsDebugState::SteppingInto;
    }

    pub fn step_over(&self) {
        let depth = *self.call_depth.lock().unwrap();
        *self.state.lock().unwrap() = AsDebugState::SteppingOver {
            target_depth: depth,
        };
    }

    pub fn step_out(&self) {
        let depth = *self.call_depth.lock().unwrap();
        let target = depth.saturating_sub(1);
        *self.state.lock().unwrap() = AsDebugState::SteppingOut {
            target_depth: target,
        };
    }

    pub fn pause(&self) {
        *self.state.lock().unwrap() = AsDebugState::Paused;
    }

    pub fn resume(&self) {
        *self.state.lock().unwrap() = AsDebugState::Running;
    }

    pub fn reset(&self) {
        *self.state.lock().unwrap() = AsDebugState::Idle;
        *self.current_location.lock().unwrap() = None;
        *self.call_depth.lock().unwrap() = 0;
        self.variables.lock().unwrap().clear();
    }

    /// Called by the AngelScript line callback hook on each executed line.
    /// Returns true if execution should pause at this line.
    pub fn on_line(&self, file: &Path, line: usize, depth: usize) -> bool {
        if !self.is_active() {
            return false;
        }
        *self.call_depth.lock().unwrap() = depth;
        let loc = SourceLocation::new(file, line);
        *self.current_location.lock().unwrap() = Some(loc);

        let mut st = self.state.lock().unwrap();
        let hit_bp = self.has_breakpoint(file, line);

        if hit_bp {
            *st = AsDebugState::Paused;
            return true;
        }

        match *st {
            AsDebugState::SteppingInto => {
                *st = AsDebugState::Paused;
                true
            }
            AsDebugState::SteppingOver { target_depth } => {
                if depth <= target_depth {
                    *st = AsDebugState::Paused;
                    true
                } else {
                    false
                }
            }
            AsDebugState::SteppingOut { target_depth } => {
                if depth <= target_depth {
                    *st = AsDebugState::Paused;
                    true
                } else {
                    false
                }
            }
            AsDebugState::Paused => true,
            AsDebugState::Running | AsDebugState::Idle => false,
        }
    }

    pub fn set_variable(&self, name: impl Into<String>, value: impl Into<String>) {
        self.variables
            .lock()
            .unwrap()
            .insert(name.into(), value.into());
    }

    pub fn get_variables(&self) -> BTreeMap<String, String> {
        self.variables.lock().unwrap().clone()
    }
}
