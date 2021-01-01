//! Unified Logging System for Circuit Simulator.
//!
//! Provides three categories of logging:
//! 1. `Default` (General application events, lifecycle, CLI output, export, settings)
//! 2. `Simulation` (Circuit simulation runtime, solver, MCU, QEMU, instruments, script prints)
//! 3. `Compiler` (Build toolchain, step execution, process logs, diagnostics, firmware creation)
//!
//! All logs (Default, Simulation, and Compiler) funnel through the default logging path (stdout)
//! so that CLI users can see all system outputs in real time.
//! In addition, Simulation and Compiler logs are collected in dedicated thread-safe buffers
//! to be consumed by their respective UI panes (`SimulatorLog` and `CompilerLog`).

use std::ffi::CStr;
use std::io::Write;
use std::os::raw::c_char;
use std::sync::Mutex;

/// The category of a log message.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LogCategory {
    /// General application messages, CLI runner output, settings, and file operations.
    Default,
    /// Simulation lifecycle, circuit solver, MCU events, QEMU cosim, and script prints.
    Simulation,
    /// Compiler toolchains, step executions, diagnostic output, and firmware builds.
    Compiler,
}

/// The severity or stream level of a log message.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum LogLevel {
    #[default]
    Default,
    Stdout,
    Stderr,
    Info,
    Warning,
    Error,
    Success,
}

/// A structured log entry carrying text and its level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub text: String,
    pub level: LogLevel,
}

static SIM_LOG_BUFFER: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());
static COMP_LOG_BUFFER: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());

type LogSinkFn = Box<dyn Fn(LogCategory, &str) + Send + Sync>;
static CUSTOM_SINK: Mutex<Option<LogSinkFn>> = Mutex::new(None);

/// Set an optional custom log sink (useful for testing or headless log redirection).
pub fn set_custom_sink(sink: Option<LogSinkFn>) {
    let mut guard = CUSTOM_SINK.lock().unwrap();
    *guard = sink;
}

/// Core log dispatcher: funnels to the default logging path (stdout) and routes
/// domain-specific logs to their corresponding UI buffers.
pub fn push(category: LogCategory, msg: impl std::fmt::Display) {
    push_with_level(category, LogLevel::Default, msg);
}

/// Core log dispatcher with an explicit log level.
pub fn push_with_level(category: LogCategory, level: LogLevel, msg: impl std::fmt::Display) {
    let text = msg.to_string();

    // Check if a custom sink is registered
    let custom_handled = {
        let guard = CUSTOM_SINK.lock().unwrap();
        if let Some(ref sink) = *guard {
            sink(category, &text);
            true
        } else {
            false
        }
    };

    // Default logging path: stdout with flush (so CLI users see all logs)
    if !custom_handled {
        println!("{text}");
        let _ = std::io::stdout().flush();
    }

    let entry = LogEntry { text, level };

    // Route to domain buffers for UI panes
    match category {
        LogCategory::Default => {}
        LogCategory::Simulation => {
            let mut buf = SIM_LOG_BUFFER.lock().unwrap();
            buf.push(entry);
        }
        LogCategory::Compiler => {
            let mut buf = COMP_LOG_BUFFER.lock().unwrap();
            buf.push(entry);
        }
    }
}

/// Push a default application message (funneled to stdout only).
pub fn log_default(msg: impl std::fmt::Display) {
    push(LogCategory::Default, msg);
}

/// Push a simulation message (funneled to stdout and enqueued for Simulator Messages pane).
pub fn log_sim(msg: impl std::fmt::Display) {
    push(LogCategory::Simulation, msg);
}

/// Push a simulation error message.
pub fn log_sim_err(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Simulation, LogLevel::Error, msg);
}

/// Push a simulation warning message.
pub fn log_sim_warn(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Simulation, LogLevel::Warning, msg);
}

/// Push a simulation info message.
pub fn log_sim_info(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Simulation, LogLevel::Info, msg);
}

/// Push a compiler message (funneled to stdout and enqueued for Compiler Messages pane).
pub fn log_compiler(msg: impl std::fmt::Display) {
    push(LogCategory::Compiler, msg);
}

/// Push a compiler standard output message.
pub fn log_compiler_stdout(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Compiler, LogLevel::Stdout, msg);
}

/// Push a compiler standard error message.
pub fn log_compiler_stderr(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Compiler, LogLevel::Stderr, msg);
}

/// Push a compiler error message.
pub fn log_compiler_err(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Compiler, LogLevel::Error, msg);
}

/// Push a compiler warning message.
pub fn log_compiler_warn(msg: impl std::fmt::Display) {
    push_with_level(LogCategory::Compiler, LogLevel::Warning, msg);
}

/// Drain and return all buffered simulation log entries.
pub fn drain_simulation_entries() -> Vec<LogEntry> {
    let mut buf = SIM_LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}

/// Drain and return all buffered compiler log entries.
pub fn drain_compiler_entries() -> Vec<LogEntry> {
    let mut buf = COMP_LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}

/// Drain and return all buffered simulation log message strings.
pub fn drain_simulation() -> Vec<String> {
    drain_simulation_entries()
        .into_iter()
        .map(|e| e.text)
        .collect()
}

/// Drain and return all buffered compiler log message strings.
pub fn drain_compiler() -> Vec<String> {
    drain_compiler_entries()
        .into_iter()
        .map(|e| e.text)
        .collect()
}

/// Clear any pending simulation logs.
pub fn clear_simulation() {
    let mut buf = SIM_LOG_BUFFER.lock().unwrap();
    buf.clear();
}

/// Clear any pending compiler logs.
pub fn clear_compiler() {
    let mut buf = COMP_LOG_BUFFER.lock().unwrap();
    buf.clear();
}

/// Clear all pending logs in all UI buffers.
pub fn clear_all() {
    clear_simulation();
    clear_compiler();
}

unsafe extern "C" fn script_print_cb(msg: *const c_char) {
    if !msg.is_null() {
        if let Ok(s) = unsafe { CStr::from_ptr(msg) }.to_str() {
            log_sim(s);
        }
    }
}

/// Install script print callback into `cs-script`.
pub fn init_script_logging() {
    cs_script::set_print_callback(Some(script_print_cb));
}

/// Macro for logging default application messages.
#[macro_export]
macro_rules! log_default {
    ($($arg:tt)*) => {
        $crate::logging::log_default(format!($($arg)*))
    };
}

/// Macro for logging simulation messages.
#[macro_export]
macro_rules! log_sim {
    ($($arg:tt)*) => {
        $crate::logging::log_sim(format!($($arg)*))
    };
}

/// Macro for logging compiler messages.
#[macro_export]
macro_rules! log_comp {
    ($($arg:tt)*) => {
        $crate::logging::log_compiler(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_log_categories_routing_and_funneling() {
        clear_all();

        let funnel_logs = Arc::new(Mutex::new(Vec::<(LogCategory, String)>::new()));
        let funnel_clone = funnel_logs.clone();
        set_custom_sink(Some(Box::new(move |cat, msg| {
            funnel_logs.lock().unwrap().push((cat, msg.to_string()));
        })));

        log_default("App initialized");
        log_sim("Sim running");
        log_compiler("Compilation started");

        // Restore default sink
        set_custom_sink(None);

        // Check that all three were funneled to the default logging path
        let captured = funnel_clone.lock().unwrap().clone();
        assert_eq!(captured.len(), 3);
        assert_eq!(
            captured[0],
            (LogCategory::Default, "App initialized".into())
        );
        assert_eq!(captured[1], (LogCategory::Simulation, "Sim running".into()));
        assert_eq!(
            captured[2],
            (LogCategory::Compiler, "Compilation started".into())
        );

        // Check that simulation buffer only contains simulation log
        let sim_drained = drain_simulation();
        assert_eq!(sim_drained, vec!["Sim running".to_string()]);
        assert!(drain_simulation().is_empty());

        // Check that compiler buffer only contains compiler log
        let comp_drained = drain_compiler();
        assert_eq!(comp_drained, vec!["Compilation started".to_string()]);
        assert!(drain_compiler().is_empty());
    }

    #[test]
    fn test_clear_buffers() {
        clear_all();
        log_sim("sim msg");
        log_compiler("comp msg");

        clear_simulation();
        assert!(drain_simulation().is_empty());
        assert_eq!(drain_compiler(), vec!["comp msg".to_string()]);

        log_sim("sim msg 2");
        clear_all();
        assert!(drain_simulation().is_empty());
        assert!(drain_compiler().is_empty());
    }
}
