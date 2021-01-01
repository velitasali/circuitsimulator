//! Compiler XML (`<compiler>` / `<step>`) and `std::process` builds.
//! C++ `Compiler::loadCompiler` / `compile` / `replaceData`.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::settings::AppSettings;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileStep {
    pub command: String,
    pub arguments: String,
    pub args_debug: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompilerSpec {
    pub name: String,
    pub type_name: String,
    pub file: PathBuf,
    pub build_path: String,
    pub incl_path: String,
    pub syntax: Option<String>,
    pub upload_hex: bool,
    pub steps: Vec<CompileStep>,
}

impl CompilerSpec {
    pub fn is_none(&self) -> bool {
        self.name.is_empty() || self.name == "None"
    }

    pub fn is_arduino(&self) -> bool {
        self.type_name.eq_ignore_ascii_case("arduino") || self.name.eq_ignore_ascii_case("arduino")
    }

    pub fn uses_device(&self) -> bool {
        self.is_arduino()
            || self
                .steps
                .iter()
                .any(|s| s.arguments.contains("$device") || s.args_debug.contains("$device"))
    }

    pub fn uses_family(&self) -> bool {
        self.steps
            .iter()
            .any(|s| s.arguments.contains("$family") || s.args_debug.contains("$family"))
    }

    pub fn uses_extra_args(&self) -> bool {
        self.steps
            .iter()
            .any(|s| s.arguments.contains("$extraArgs") || s.args_debug.contains("$extraArgs"))
    }

    pub fn uses_incl_path(&self) -> bool {
        self.is_arduino()
            || self
                .steps
                .iter()
                .any(|s| s.arguments.contains("$inclPath") || s.args_debug.contains("$inclPath"))
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompileCtx {
    pub file: String,
    pub board: String,
    pub custom_board: String,
    pub device: String,
    pub family: String,
    pub extra_args: String,
    pub incl_path: String,
    pub tool_path: String,
    pub debug: bool,
    /// Resolved Arduino FQBN. When set, compile skips `arduino-cli board listall`.
    pub fqbn: String,
}

/// Live compiler output and the finished result. C++ `Compiler::waitForProcess`
/// pumped the Qt event loop; we stream from a worker thread instead.
#[derive(Debug)]
pub enum CompileEvent {
    Log(String),
    Finished(CompileResult),
}

/// Background `compile()`. The GUI drains [`CompileJob::drain`] on the UI thread.
pub struct CompileJob {
    rx: mpsc::Receiver<CompileEvent>,
    cancelled: Arc<AtomicBool>,
}

impl CompileJob {
    pub fn spawn(spec: CompilerSpec, ctx: CompileCtx) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_flag = cancelled.clone();
        let _ = thread::Builder::new()
            .name("cs-compile".into())
            .spawn(move || {
                let tx_log = tx.clone();
                let result = compile_with_log_cancel(&spec, &ctx, &cancel_flag, &mut |line| {
                    let _ = tx_log.send(CompileEvent::Log(line.to_string()));
                });
                let _ = tx.send(CompileEvent::Finished(result));
            });
        Self { rx, cancelled }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn drain(&self) -> Vec<CompileEvent> {
        let mut events = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(ev) => events.push(ev),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !events
                        .iter()
                        .any(|e| matches!(e, CompileEvent::Finished(_)))
                    {
                        let mut r = CompileResult::default();
                        r.error = -1;
                        r.log.push("     WARNING: Compilation Not Done".into());
                        events.push(CompileEvent::Log(
                            "     WARNING: Compilation Not Done".into(),
                        ));
                        events.push(CompileEvent::Finished(r));
                    }
                    break;
                }
            }
        }
        events
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompileResult {
    /// 0 = ok, >0 = first error line, −1 = not done / toolchain missing.
    pub error: i32,
    pub firmware: String,
    pub errors: Vec<i32>,
    pub warnings: Vec<i32>,
    pub log: Vec<String>,
}

pub fn parse_compiler_xml(src: &str, file: &Path) -> Option<CompilerSpec> {
    let mut reader = Reader::from_str(src);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut spec: Option<CompilerSpec> = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut attrs = BTreeMap::new();
                for a in e.attributes().flatten() {
                    let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                    let v = a
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    attrs.insert(k, v);
                }
                if tag == "compiler" {
                    let name = attrs.get("name")?.clone();
                    let type_name = attrs.get("type")?.clone();
                    spec = Some(CompilerSpec {
                        name,
                        type_name,
                        file: file.to_path_buf(),
                        build_path: attrs.get("buildPath").cloned().unwrap_or_default(),
                        incl_path: attrs.get("inclPath").cloned().unwrap_or_default(),
                        syntax: attrs.get("syntax").cloned(),
                        upload_hex: attrs.get("uploadhex").map(|s| s != "false").unwrap_or(true),
                        steps: Vec::new(),
                    });
                } else if tag == "step" {
                    if let Some(s) = spec.as_mut() {
                        let arguments = attrs.get("arguments").cloned().unwrap_or_default();
                        let args_debug = attrs.get("argsDebug").cloned().unwrap_or_default();
                        s.steps.push(CompileStep {
                            command: attrs.get("command").cloned().unwrap_or_default(),
                            arguments,
                            args_debug,
                        });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
    spec
}

fn add_quotes(s: &str) -> String {
    format!("\"{s}\"")
}

/// C++ `Compiler::replaceData` then `$device` / `$family`.
pub fn replace_data(
    str: &str,
    ctx: &CompileCtx,
    file_dir: &str,
    file_name: &str,
    file_ext: &str,
    build_path: &str,
) -> String {
    str.replace("$filePath", &add_quotes(&ctx.file))
        .replace("$fileDir", file_dir)
        .replace("$fileName", file_name)
        .replace("$fileExt", file_ext)
        .replace("$inclPath", &add_quotes(&ctx.incl_path))
        .replace("$buildPath", &add_quotes(build_path))
        .replace("$extraArgs", &ctx.extra_args)
        .replace("$device", &ctx.device)
        .replace("$family", &ctx.family)
}

/// Split a command string the way `QProcess::splitCommand` does for
/// `"dir/"file` (quote close then more text stays in the same argv).
pub fn split_command(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quote = !in_quote;
            }
            c if c.is_whitespace() && !in_quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            '\\' if in_quote && chars.peek() == Some(&'"') => {
                chars.next();
                cur.push('"');
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn file_parts(path: &str) -> (String, String, String) {
    let p = Path::new(path);
    let dir = p
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| {
            let mut s = d.to_string_lossy().into_owned();
            if !s.ends_with('/') && !s.ends_with('\\') {
                s.push('/');
            }
            s
        })
        .unwrap_or_else(|| "./".into());
    let name = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    (dir, name, ext)
}

fn resolve_build_dir(
    spec: &CompilerSpec,
    ctx: &CompileCtx,
    file_dir: &str,
    file_name: &str,
    file_ext: &str,
) -> String {
    if spec.build_path.is_empty() {
        return file_dir.to_string();
    }
    let rel = replace_data(
        &spec.build_path,
        ctx,
        file_dir,
        file_name,
        file_ext,
        file_dir,
    );
    let rel = rel.trim_matches('"').to_string();
    let dir = Path::new(file_dir).join(&rel);
    let _ = std::fs::create_dir_all(&dir);
    let mut s = dir.to_string_lossy().into_owned();
    if !s.ends_with('/') && !s.ends_with('\\') {
        s.push('/');
    }
    s
}

/// Locate `cmd` in `search_dirs`, or on PATH when `search_dirs` is empty.
/// C++ `QStandardPaths::findExecutable`.
pub fn find_executable<P: AsRef<Path>>(cmd: &str, search_dirs: &[P]) -> Option<PathBuf> {
    if cmd.is_empty() {
        return None;
    }
    #[cfg(windows)]
    let names = {
        let mut names = vec![cmd.to_string()];
        if !cmd.to_ascii_lowercase().ends_with(".exe") {
            names.push(format!("{cmd}.exe"));
        }
        names
    };
    #[cfg(not(windows))]
    let names = vec![cmd.to_string()];

    if search_dirs.is_empty() {
        let p = Path::new(cmd);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
        #[cfg(windows)]
        {
            let exe = p.with_extension("exe");
            if exe.is_file() {
                return Some(exe);
            }
        }
        if p.is_absolute() {
            return None;
        }
        let paths = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&paths) {
            for name in &names {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
        return None;
    }

    for dir in search_dirs {
        for name in &names {
            let cand = dir.as_ref().join(name);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

pub fn command_exists(cmd: &str) -> bool {
    find_executable(cmd, &[] as &[&Path]).is_some()
}

fn ensure_trailing_slash(path: &str) -> String {
    if path.is_empty() || path.ends_with('/') || path.ends_with('\\') {
        return path.to_string();
    }
    format!("{path}/")
}

/// Directories the Tool Path combo should offer. C++ `Compiler::toolPathCandidates`.
pub fn tool_path_candidates(spec: &CompilerSpec) -> Vec<String> {
    if spec.is_arduino() {
        return super::arduino::tool_path_candidates();
    }
    let commands: Vec<&str> = spec
        .steps
        .iter()
        .map(|s| s.command.as_str())
        .filter(|c| !c.is_empty())
        .collect();
    if commands.is_empty() {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    if commands.iter().all(|c| command_exists(c)) {
        candidates.push(String::new());
    }
    if let Some(first) = commands.first() {
        if let Some(exe) = find_executable(first, &[] as &[&Path]) {
            if let Some(dir) = exe.parent() {
                let dir = ensure_trailing_slash(&dir.to_string_lossy());
                if !candidates.contains(&dir) {
                    candidates.push(dir);
                }
            }
        }
    }
    candidates
}

/// Empty string means the path is fine. C++ `Compiler::checkToolPath`.
pub fn check_tool_path(spec: &CompilerSpec, path: &str) -> String {
    if spec.is_arduino() {
        return super::arduino::check_tool_path(path);
    }
    if spec.steps.iter().all(|s| s.command.is_empty()) {
        return String::new();
    }
    let dirs: Vec<PathBuf> = if path.is_empty() {
        Vec::new()
    } else {
        vec![PathBuf::from(path)]
    };
    let mut missing = Vec::new();
    for step in &spec.steps {
        if step.command.is_empty() {
            continue;
        }
        if find_executable(&step.command, &dirs).is_none() {
            missing.push(step.command.clone());
        }
    }
    if missing.is_empty() {
        String::new()
    } else {
        format!("Not found here: {}", missing.join(", "))
    }
}

fn join_tool(tool_path: &str, command: &str) -> String {
    if tool_path.is_empty() {
        return command.to_string();
    }
    Path::new(tool_path)
        .join(command)
        .to_string_lossy()
        .into_owned()
}

/// Parse gcc-style `file.c:12:3: error:` lines. C++ `Compiler::getErrorLine`.
pub fn parse_error_lines(txt: &str, file_name: &str, file_ext: &str) -> (i32, Vec<i32>, Vec<i32>) {
    let needle = format!("{file_name}{file_ext}");
    let mut first_error = 0i32;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for line in txt.lines() {
        if !line.contains(&needle) {
            continue;
        }
        let after = line.rsplit_once(&needle).map(|(_, r)| r).unwrap_or(line);
        let Some(num) = first_number(after) else {
            continue;
        };
        let lower = after.to_ascii_lowercase();
        if lower.contains("error") {
            if first_error == 0 {
                first_error = num;
            }
            if !errors.contains(&num) {
                errors.push(num);
            }
        } else if lower.contains("warning") && !warnings.contains(&num) {
            warnings.push(num);
        }
    }
    (first_error, errors, warnings)
}

fn first_number(s: &str) -> Option<i32> {
    let mut n = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            n.push(c);
        } else if !n.is_empty() {
            break;
        }
    }
    if n.is_empty() { None } else { n.parse().ok() }
}

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

pub fn compile(spec: &CompilerSpec, ctx: &CompileCtx) -> CompileResult {
    compile_with_log(spec, ctx, &mut |_| {})
}

fn emit_log(r: &mut CompileResult, on_log: &mut dyn FnMut(&str), line: impl Into<String>) {
    let line = line.into();
    crate::logging::log_compiler(&line);
    on_log(&line);
    r.log.push(line);
}

pub const DEFAULT_COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

/// Run `cmd` on the caller thread (the compile worker) and forward its output.
#[allow(dead_code)]
pub(crate) fn run_command_logged(
    cmd: &mut Command,
    on_log: &mut dyn FnMut(&str),
) -> Result<(bool, String), String> {
    let dummy_cancel = AtomicBool::new(false);
    run_command_logged_cancel(cmd, &dummy_cancel, DEFAULT_COMPILE_TIMEOUT, on_log)
}

pub(crate) fn run_command_logged_cancel(
    cmd: &mut Command,
    cancel: &AtomicBool,
    timeout: Duration,
    on_log: &mut dyn FnMut(&str),
) -> Result<(bool, String), String> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(unix)]
    cmd.process_group(0);

    cmd.env_remove("DYLD_LIBRARY_PATH");
    cmd.env_remove("DYLD_FALLBACK_LIBRARY_PATH");
    cmd.env_remove("DYLD_FRAMEWORK_PATH");

    let start = Instant::now();
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let err_msg = format!("ERROR: Failed to spawn process: {e}");
            crate::logging::log_compiler(&err_msg);
            on_log(&err_msg);
            return Err(e.to_string());
        }
    };
    let child_pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (line_tx, line_rx) = mpsc::channel::<(bool, String)>();

    let _t_out = stdout.map(|out| {
        let tx = line_tx.clone();
        thread::Builder::new()
            .name("cs-proc-stdout".into())
            .spawn(move || {
                let reader = std::io::BufReader::new(out);
                for line in reader.lines() {
                    match line {
                        Ok(l) => {
                            if tx.send((false, l)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
    });

    let _t_err = stderr.map(|err| {
        let tx = line_tx.clone();
        thread::Builder::new()
            .name("cs-proc-stderr".into())
            .spawn(move || {
                let reader = std::io::BufReader::new(err);
                for line in reader.lines() {
                    match line {
                        Ok(l) => {
                            if tx.send((true, l)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
    });

    drop(line_tx);

    let mut combined = String::new();
    let mut timed_out = false;
    let mut was_cancelled = false;
    let poll_interval = Duration::from_millis(20);

    let exit_status = loop {
        while let Ok((is_stderr, line)) = line_rx.try_recv() {
            if is_stderr {
                crate::logging::log_compiler_stderr(&line);
            } else {
                crate::logging::log_compiler_stdout(&line);
            }
            on_log(&line);
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&line);
        }

        if cancel.load(Ordering::Relaxed) {
            was_cancelled = true;
            let msg = "     Compilation cancelled by user.";
            crate::logging::log_compiler(msg);
            on_log(msg);
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(msg);
            #[cfg(unix)]
            unsafe {
                kill(-(child_pid as i32), 9);
            }
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }

        if start.elapsed() >= timeout {
            timed_out = true;
            let msg = format!(
                "     ERROR: Compilation timed out after {}s",
                timeout.as_secs()
            );
            crate::logging::log_compiler(&msg);
            on_log(&msg);
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&msg);
            #[cfg(unix)]
            unsafe {
                kill(-(child_pid as i32), 9);
            }
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }

        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(poll_interval),
            Err(e) => {
                #[cfg(unix)]
                unsafe {
                    kill(-(child_pid as i32), 9);
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    };

    #[cfg(unix)]
    unsafe {
        // Once the primary compiler process exits, kill any orphaned background daemons
        // (such as serial-discovery) that were spawned in its process group and inherited
        // open stdout/stderr pipes.
        kill(-(child_pid as i32), 9);
    }

    if let Some(Ok(t)) = _t_out {
        let _ = t.join();
    }
    if let Some(Ok(t)) = _t_err {
        let _ = t.join();
    }

    // Now all output from both stdout and stderr has been fully written into line_rx,
    // and both reader threads have terminated cleanly.
    while let Ok((is_stderr, line)) = line_rx.try_recv() {
        if is_stderr {
            crate::logging::log_compiler_stderr(&line);
        } else {
            crate::logging::log_compiler_stdout(&line);
        }
        on_log(&line);
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&line);
    }

    let elapsed = start.elapsed().as_secs_f32();
    let success = if was_cancelled || timed_out {
        false
    } else if let Some(status) = exit_status {
        let msg = format!(
            "[compiler] Process exited with status {:?} in {:.2}s",
            status.code(),
            elapsed
        );
        crate::logging::log_compiler(&msg);
        status.success()
    } else {
        false
    };

    Ok((success, combined))
}

pub fn compile_with_log(
    spec: &CompilerSpec,
    ctx: &CompileCtx,
    on_log: &mut dyn FnMut(&str),
) -> CompileResult {
    let dummy_cancel = AtomicBool::new(false);
    compile_with_log_cancel(spec, ctx, &dummy_cancel, on_log)
}

pub fn compile_with_log_cancel(
    spec: &CompilerSpec,
    ctx: &CompileCtx,
    cancel: &AtomicBool,
    on_log: &mut dyn FnMut(&str),
) -> CompileResult {
    let mut r = CompileResult::default();
    if cancel.load(Ordering::Relaxed) {
        emit_log(&mut r, on_log, "     Compilation cancelled by user.");
        r.error = -1;
        return r;
    }
    if spec.is_none() {
        emit_log(&mut r, on_log, "     No Compiler Defined");
        r.error = -1;
        return r;
    }
    if ctx.file.is_empty() {
        emit_log(&mut r, on_log, "     Error: File not saved");
        r.error = -1;
        return r;
    }
    if spec.is_arduino() {
        let fqbn = if !ctx.fqbn.is_empty() {
            ctx.fqbn.clone()
        } else {
            let boards = super::arduino::default_boards();
            let (board, custom, _) =
                super::arduino::match_board(&ctx.board, &ctx.custom_board, &ctx.device, &boards);
            super::arduino::resolve_board_fqbn(&board, &custom, &boards)
        };
        return super::arduino::compile_arduino_with_log_cancel(
            &ctx.file,
            &fqbn,
            &ctx.tool_path,
            cancel,
            on_log,
        );
    }
    if spec.type_name.eq_ignore_ascii_case("ascript") || spec.name.eq_ignore_ascii_case("ascript") {
        let result = super::ascript::compile_ascript(&ctx.file);
        for line in &result.log {
            on_log(line);
        }
        return result;
    }
    if spec.steps.is_empty() {
        emit_log(&mut r, on_log, "     No command Defined");
        r.error = -1;
        return r;
    }
    let (file_dir, file_name, file_ext) = file_parts(&ctx.file);
    let build_path = resolve_build_dir(spec, ctx, &file_dir, &file_name, &file_ext);
    emit_log(
        &mut r,
        on_log,
        format!("Compiling '{}' using compiler '{}'...", ctx.file, spec.name),
    );
    if !ctx.device.is_empty() || !ctx.family.is_empty() {
        emit_log(
            &mut r,
            on_log,
            format!("Target device: {}, family: {}", ctx.device, ctx.family),
        );
    }

    let total_steps = spec.steps.len();
    for (idx, step) in spec.steps.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            emit_log(&mut r, on_log, "     Compilation cancelled by user.");
            r.error = -1;
            return r;
        }
        let exe = join_tool(&ctx.tool_path, &step.command);
        if !command_exists(&exe) {
            emit_log(&mut r, on_log, format!("ERROR: {exe}"));
            emit_log(&mut r, on_log, "     : Executable not found");
            emit_log(&mut r, on_log, "     : Check that Tool Path is correct");
            r.error = -1;
            return r;
        }
        let raw = if ctx.debug && !step.args_debug.is_empty() {
            step.args_debug.as_str()
        } else {
            step.arguments.as_str()
        };
        if raw.contains("$family") && ctx.family.is_empty() {
            emit_log(&mut r, on_log, "     Error: Family not defined");
            r.error = -1;
            return r;
        }
        if raw.contains("$device") && ctx.device.is_empty() {
            emit_log(&mut r, on_log, "     Error: Device not defined");
            r.error = -1;
            return r;
        }
        let args_s = replace_data(raw, ctx, &file_dir, &file_name, &file_ext, &build_path);
        let argv = split_command(&args_s);
        let mut shown = exe.clone();
        if !args_s.is_empty() {
            shown.push(' ');
            shown.push_str(args_s.trim());
        }
        emit_log(
            &mut r,
            on_log,
            format!("[Step {}/{}] Executing:\n{shown}\n", idx + 1, total_steps),
        );

        let mut cmd = Command::new(&exe);
        cmd.args(&argv);
        let cur = file_dir.trim_end_matches(['/', '\\']);
        if !cur.is_empty() && Path::new(cur).is_dir() {
            cmd.current_dir(cur);
        }
        match run_command_logged_cancel(&mut cmd, cancel, DEFAULT_COMPILE_TIMEOUT, on_log) {
            Ok((success, combined)) => {
                if !combined.is_empty() {
                    r.log.push(combined.clone());
                    let (e, es, ws) = parse_error_lines(&combined, &file_name, &file_ext);
                    if e > 0 && r.error == 0 {
                        r.error = e;
                    }
                    r.errors.extend(es);
                    r.warnings.extend(ws);
                }
                if r.error > 0 {
                    let err_count = r.errors.len().max(1);
                    let warn_count = r.warnings.len();
                    emit_log(
                        &mut r,
                        on_log,
                        format!(
                            "Compilation failed at step {} with {err_count} error(s), {warn_count} warning(s)",
                            idx + 1
                        ),
                    );
                    return r;
                }
                if !success && r.error == 0 {
                    r.error = -1;
                    emit_log(
                        &mut r,
                        on_log,
                        format!("Step {} failed with non-zero exit status", idx + 1),
                    );
                    return r;
                }
            }
            Err(e) => {
                emit_log(&mut r, on_log, format!("ERROR: {e}"));
                r.error = -1;
                return r;
            }
        }
    }

    let hex = Path::new(&build_path).join(format!("{file_name}.hex"));
    if spec.upload_hex || file_ext == ".hex" {
        let fw = hex.to_string_lossy().into_owned();
        r.firmware = fw.clone();
        emit_log(
            &mut r,
            on_log,
            format!("Compilation succeeded: firmware target is '{fw}'"),
        );
    } else {
        emit_log(&mut r, on_log, "Compilation succeeded.");
    }
    r
}

pub fn extract_ino_prototypes(source: &str) -> Vec<String> {
    let mut protos = Vec::new();
    let re = regex::Regex::new(
        r"(?m)^\s*((?:(?:unsigned|signed|const|inline|static)\s+)*(?:void|bool|char|int|short|long|float|double|uint8_t|uint16_t|uint32_t|int8_t|int16_t|int32_t|size_t|String|[A-Za-z_][A-Za-z0-9_]*)(?:\s*[*&]+\s*|\s+)[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\))\s*\{",
    );
    if let Ok(re) = re {
        for cap in re.captures_iter(source) {
            if let Some(m) = cap.get(1) {
                let sig = m.as_str().trim();
                let first_word = sig.split_whitespace().next().unwrap_or("");
                if matches!(
                    first_word,
                    "if" | "while" | "for" | "switch" | "catch" | "class" | "struct"
                ) {
                    continue;
                }
                let normalized = sig.split_whitespace().collect::<Vec<_>>().join(" ");
                protos.push(format!("{normalized};"));
            }
        }
    }
    protos
}

pub fn write_compile_flags(ctx: &CompileCtx, spec: &CompilerSpec) {
    if ctx.file.is_empty() {
        return;
    }
    let (file_dir, _, file_ext) = file_parts(&ctx.file);
    let path = Path::new(file_dir.trim_end_matches(['/', '\\'])).join("compile_flags.txt");
    let mut out = String::from("-x\nc++\n-std=gnu++17\n");
    let is_arduino = spec.is_arduino()
        || file_ext.eq_ignore_ascii_case(".ino")
        || file_ext.eq_ignore_ascii_case(".pde");
    let is_avr = is_arduino
        || spec.type_name.to_ascii_lowercase().contains("avr")
        || spec.name.to_ascii_lowercase().contains("avr");

    if is_avr {
        out.push_str("--target=avr\n");
        let dev = if !ctx.device.is_empty() && !ctx.device.contains(':') {
            ctx.device.to_ascii_lowercase()
        } else if !ctx.board.is_empty() {
            super::arduino::device_for_board_or_fqbn(&ctx.board, &ctx.custom_board)
        } else {
            "atmega328p".to_string()
        };
        out.push_str(&format!("-mmcu={dev}\n"));
        out.push_str(&format!("-D__AVR_{}__\n", dev.to_ascii_uppercase()));
        out.push_str("-DARDUINO=10819\n-DARDUINO_ARCH_AVR\n-DF_CPU=16000000L\n");
    } else if !ctx.device.is_empty() {
        out.push_str(&format!("-mmcu={}\n", ctx.device));
    }

    let inc_paths = if is_arduino || is_avr {
        super::arduino::arduino_include_paths(&ctx.file, &ctx.tool_path, &ctx.incl_path, &ctx.board)
    } else {
        Vec::new()
    };

    for inc in inc_paths {
        out.push_str("-I\n");
        out.push_str(&inc.to_string_lossy());
        out.push('\n');
    }

    if !is_arduino && !is_avr && !ctx.incl_path.is_empty() {
        for p in ctx.incl_path.split([';', ',', '\n']) {
            let p = p.trim();
            if !p.is_empty() {
                out.push_str("-I\n");
                out.push_str(p);
                out.push('\n');
            }
        }
    }

    if is_arduino || is_avr {
        out.push_str("-include\nArduino.h\n");
        let proto_header_path =
            Path::new(file_dir.trim_end_matches(['/', '\\'])).join(".arduino_prototypes.h");
        if proto_header_path.is_file() {
            let _ = std::fs::remove_file(&proto_header_path);
        }
    }

    if !ctx.extra_args.is_empty() {
        for a in ctx.extra_args.split_whitespace() {
            out.push_str(a);
            out.push('\n');
        }
    }
    let _ = std::fs::write(path, out);
}

fn embedded() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "arduino.xml",
            include_str!("../../../../resources/data/codeeditor/compilers/compilers/arduino.xml"),
        ),
        (
            "angelscript.xml",
            include_str!(
                "../../../../resources/data/codeeditor/compilers/compilers/angelscript.xml"
            ),
        ),
        (
            "avrgcc.xml",
            include_str!("../../../../resources/data/codeeditor/compilers/compilers/avrgcc.xml"),
        ),
        (
            "dummy_c.xml",
            include_str!("../../../../resources/data/codeeditor/compilers/compilers/dummy_c.xml"),
        ),
        (
            "default.xml",
            include_str!("../../../../resources/data/codeeditor/compilers/default.xml"),
        ),
    ]
}

fn load_dir(map: &mut BTreeMap<String, CompilerSpec>, dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("xml"))
        })
        .collect();
    files.sort();
    for f in files {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        if let Some(spec) = parse_compiler_xml(&src, &f) {
            map.entry(spec.name.clone()).or_insert(spec);
        }
    }
}

/// C++ `EditorWindow::loadCompilers`: Arduino / AScript first, then user dirs,
/// then the embedded XML set. First name wins.
pub fn load_compilers(settings: &AppSettings) -> BTreeMap<String, CompilerSpec> {
    let mut map = BTreeMap::new();
    for (name, src) in embedded() {
        let path = PathBuf::from(name);
        if let Some(spec) = parse_compiler_xml(src, &path) {
            map.entry(spec.name.clone()).or_insert(spec);
        }
    }
    // User dirs override only names not already present? C++ loads user
    // *before* embedded Avrgcc, *after* Arduino/AScript. Rebuild to match:
    map.clear();
    // Seed Arduino / AScript from embedded (always present).
    for key in ["arduino.xml", "angelscript.xml"] {
        if let Some((_, src)) = embedded().into_iter().find(|(n, _)| *n == key) {
            if let Some(spec) = parse_compiler_xml(src, Path::new(key)) {
                map.entry(spec.name.clone()).or_insert(spec);
            }
        }
    }
    let mut dirs = Vec::new();
    if !settings.user_path.is_empty() {
        let u = PathBuf::from(&settings.user_path);
        dirs.push(u.join("codeeditor/compilers"));
        dirs.push(u.join("codeeditor/compilers/compilers"));
        dirs.push(u.join("codeeditor/compilers/assemblers"));
    }
    if let Some(data) = dirs::data_dir() {
        let root = data
            .join("Circuit Simulator")
            .join("data")
            .join("codeeditor");
        dirs.push(root.join("compilers/compilers"));
        dirs.push(root.join("compilers/assemblers"));
    }
    dirs.push(PathBuf::from(
        "resources/data/codeeditor/compilers/compilers",
    ));
    dirs.push(PathBuf::from(
        "resources/data/codeeditor/compilers/assemblers",
    ));
    dirs.push(PathBuf::from("data/codeeditor/compilers/compilers"));
    for d in &dirs {
        load_dir(&mut map, d);
    }
    // Embedded leftovers (Avrgcc, Dummy C, None).
    for (name, src) in embedded() {
        if let Some(spec) = parse_compiler_xml(src, Path::new(name)) {
            map.entry(spec.name.clone()).or_insert(spec);
        }
    }
    map
}

pub fn compiler_names(map: &BTreeMap<String, CompilerSpec>) -> Vec<String> {
    let mut names: Vec<String> = map.keys().cloned().filter(|n| n != "None").collect();
    names.sort();
    names.insert(0, "None".into());
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const AVR: &str = r#"<!DOCTYPE CircuitSimulator>
<compiler name="Avrgcc" type="avrgcc" buildPath="build_$fileName" >
    <step
        command="avr-gcc"
        arguments=" -mmcu=$device -Wall -g -Os -o $buildPath$fileName.elf $filePath"
        argsDebug=" -mmcu=$device -Wall -g -Og -o $buildPath$fileName.elf $filePath"
    />
    <step
        command="avr-objcopy"
        arguments=" -j .text -j .data -O ihex $buildPath$fileName.elf $buildPath$fileName.hex"
    />
</compiler>
"#;

    #[test]
    fn parse_avrgcc() {
        let spec = parse_compiler_xml(AVR, Path::new("avrgcc.xml")).unwrap();
        assert_eq!(spec.name, "Avrgcc");
        assert_eq!(spec.type_name, "avrgcc");
        assert_eq!(spec.steps.len(), 2);
        assert!(spec.uses_device());
        assert!(!spec.uses_family());
        assert_eq!(spec.steps[0].command, "avr-gcc");
    }

    #[test]
    fn split_quoted_concat() {
        // C++ addQuotes(buildPath) + fileName → one argv: dir/file
        let args = split_command(" -o \"/tmp/build_main/\"main.elf \"/tmp/main.c\"");
        assert_eq!(args, vec!["-o", "/tmp/build_main/main.elf", "/tmp/main.c"]);
    }

    #[test]
    fn error_line_from_gcc() {
        let txt = "main.c:12:3: error: expected ';'\nmain.c:20:1: warning: unused\n";
        let (e, es, ws) = parse_error_lines(txt, "main", ".c");
        assert_eq!(e, 12);
        assert_eq!(es, vec![12]);
        assert_eq!(ws, vec![20]);
    }

    #[test]
    fn replace_device() {
        let ctx = CompileCtx {
            file: "/tmp/blink.c".into(),
            device: "atmega328p".into(),
            ..Default::default()
        };
        let s = replace_data(
            " -mmcu=$device -o $buildPath$fileName.elf $filePath",
            &ctx,
            "/tmp/",
            "blink",
            ".c",
            "/tmp/build_blink/",
        );
        assert!(s.contains("atmega328p"), "{s}");
        assert!(s.contains("\"/tmp/blink.c\""), "{s}");
        assert!(s.contains("\"/tmp/build_blink/\"blink.elf"), "{s}");
    }

    #[test]
    #[cfg(unix)]
    fn echo_step_runs() {
        if !command_exists("echo") {
            return;
        }
        let spec = CompilerSpec {
            name: "Echo".into(),
            type_name: "test".into(),
            upload_hex: false,
            steps: vec![CompileStep {
                command: "echo".into(),
                arguments: "hello $fileName".into(),
                args_debug: String::new(),
            }],
            ..Default::default()
        };
        let dir = std::env::temp_dir();
        let file = dir.join("cs_echo_test.c");
        let _ = std::fs::write(&file, "int main(){}\n");
        let ctx = CompileCtx {
            file: file.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let r = compile(&spec, &ctx);
        assert_eq!(r.error, 0, "{:?}", r.log);
        assert!(r.log.iter().any(|l| l.contains("hello")), "{:?}", r.log);
    }

    #[test]
    #[cfg(unix)]
    fn compile_job_streams_then_finishes() {
        if !command_exists("echo") {
            return;
        }
        let spec = CompilerSpec {
            name: "Echo".into(),
            type_name: "test".into(),
            upload_hex: false,
            steps: vec![CompileStep {
                command: "echo".into(),
                arguments: "hello $fileName".into(),
                args_debug: String::new(),
            }],
            ..Default::default()
        };
        let dir = std::env::temp_dir();
        let file = dir.join("cs_echo_job_test.c");
        let _ = std::fs::write(&file, "int main(){}\n");
        let ctx = CompileCtx {
            file: file.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let job = CompileJob::spawn(spec, ctx);
        let start = std::time::Instant::now();
        let mut logs = Vec::new();
        let mut finished = None;
        while start.elapsed() < Duration::from_secs(2) {
            for ev in job.drain() {
                match ev {
                    CompileEvent::Log(line) => logs.push(line),
                    CompileEvent::Finished(r) => finished = Some(r),
                }
            }
            if finished.is_some() {
                break;
            }
        }
        let r = finished.expect("compile job should finish");
        assert_eq!(r.error, 0, "{logs:?}");
        assert!(
            logs.iter()
                .any(|l| l.contains("Executing") || l.contains("hello")),
            "{logs:?}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn compile_job_arduino_mega_display() {
        let ino_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/Mega_Display/Mega_Display.ino");
        if !ino_path.is_file() {
            return;
        }
        let ino = ino_path.to_str().unwrap();
        let spec = CompilerSpec {
            name: "Arduino".into(),
            type_name: "arduino".into(),
            ..Default::default()
        };
        let ctx = CompileCtx {
            file: ino.into(),
            device: "arduino:avr:uno".into(),
            fqbn: "arduino:avr:uno".into(),
            ..Default::default()
        };
        let job = CompileJob::spawn(spec, ctx);
        let start = std::time::Instant::now();
        let mut logs = Vec::new();
        let mut finished = None;
        while start.elapsed() < Duration::from_secs(10) {
            for ev in job.drain() {
                match ev {
                    CompileEvent::Log(line) => logs.push(line),
                    CompileEvent::Finished(r) => finished = Some(r),
                }
            }
            if finished.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let r = finished.expect("Arduino compile job should finish within 10s");
        println!(
            "Arduino compile finished in {:.2}s with error {}",
            start.elapsed().as_secs_f32(),
            r.error
        );
        println!("Firmware: {}", r.firmware);
        if logs
            .iter()
            .any(|l| l.contains("bad CPU type in executable"))
        {
            eprintln!(
                "Skipping compile_job_arduino_mega_display: local Arduino AVR toolchain is 32-bit (bad CPU type)"
            );
            return;
        }
        assert_eq!(r.error, 0);
        assert!(!r.firmware.is_empty());
        assert!(Path::new(&r.firmware).is_file());
    }

    #[test]
    #[cfg(unix)]
    fn run_command_logged_does_not_hang_on_stdin() {
        // `cat` with inherited stdin waits forever. The compiler used to hang
        // the same way after switching from `Command::output()` to `spawn()`.
        let mut cmd = Command::new("cat");
        let start = std::time::Instant::now();
        let result = run_command_logged(&mut cmd, &mut |_| {});
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "cat should get EOF on stdin immediately"
        );
        let (success, out) = result.expect("cat should run");
        assert!(success, "{out}");
    }

    #[test]
    #[cfg(unix)]
    fn compile_job_can_be_cancelled() {
        if !command_exists("sleep") {
            return;
        }
        let spec = CompilerSpec {
            name: "Sleep".into(),
            type_name: "test".into(),
            upload_hex: false,
            steps: vec![CompileStep {
                command: "sleep".into(),
                arguments: "10".into(),
                args_debug: String::new(),
            }],
            ..Default::default()
        };
        let dir = std::env::temp_dir();
        let file = dir.join("cs_sleep_job_test.c");
        let _ = std::fs::write(&file, "int main(){}\n");
        let ctx = CompileCtx {
            file: file.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let job = CompileJob::spawn(spec, ctx);
        thread::sleep(Duration::from_millis(50));
        let start = std::time::Instant::now();
        job.cancel();

        let mut finished = None;
        let mut logs = Vec::new();
        while start.elapsed() < Duration::from_secs(2) {
            for ev in job.drain() {
                match ev {
                    CompileEvent::Log(line) => logs.push(line),
                    CompileEvent::Finished(r) => finished = Some(r),
                }
            }
            if finished.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let r = finished.expect("cancelled job should finish quickly");
        assert_eq!(r.error, -1);
        assert!(logs.iter().any(|l| l.contains("cancelled")), "{logs:?}");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "took too long: {:?}",
            start.elapsed()
        );
    }

    #[test]
    #[cfg(unix)]
    fn run_command_logged_timeout() {
        if !command_exists("sleep") {
            return;
        }
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let cancel = AtomicBool::new(false);
        let start = std::time::Instant::now();
        let res =
            run_command_logged_cancel(&mut cmd, &cancel, Duration::from_millis(150), &mut |_| {});
        assert!(start.elapsed() < Duration::from_secs(2));
        let (success, combined) = res.expect("command should return result after timeout");
        assert!(!success);
        assert!(combined.contains("timed out"), "{combined}");
    }

    #[test]
    fn names_include_avrgcc() {
        let map = load_compilers(&AppSettings::default());
        let names = compiler_names(&map);
        assert!(names.contains(&"None".into()), "{names:?}");
        assert!(names.contains(&"Avrgcc".into()), "{names:?}");
        assert!(names.contains(&"Arduino".into()), "{names:?}");
        assert_eq!(names[0], "None");
    }

    #[test]
    #[cfg(unix)]
    fn check_tool_path_on_path() {
        if !command_exists("echo") {
            return;
        }
        let spec = CompilerSpec {
            name: "Echo".into(),
            type_name: "test".into(),
            steps: vec![CompileStep {
                command: "echo".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(check_tool_path(&spec, "").is_empty());
        let cands = tool_path_candidates(&spec);
        assert!(cands.iter().any(|c| c.is_empty()), "{cands:?}");
        let miss = check_tool_path(&spec, "/no/such/compiler/dir");
        assert!(miss.contains("echo"), "{miss}");
    }

    #[test]
    fn check_tool_path_no_steps() {
        let spec = CompilerSpec {
            name: "None".into(),
            ..Default::default()
        };
        assert!(check_tool_path(&spec, "").is_empty());
        assert!(tool_path_candidates(&spec).is_empty());
    }

    #[test]
    fn write_compile_flags_arduino() {
        let dir = std::env::temp_dir().join("cs_test_flags");
        let _ = std::fs::create_dir_all(&dir);
        let ino = dir.join("test.ino");
        let _ = std::fs::write(&ino, "void setup(){}\nvoid loop(){}\n");
        let ctx = CompileCtx {
            file: ino.to_string_lossy().into_owned(),
            board: "Uno".into(),
            device: "atmega328p".into(),
            ..Default::default()
        };
        let spec = CompilerSpec {
            name: "Arduino".into(),
            type_name: "arduino".into(),
            ..Default::default()
        };
        write_compile_flags(&ctx, &spec);
        let flags_file = dir.join("compile_flags.txt");
        assert!(flags_file.is_file());
        let content = std::fs::read_to_string(&flags_file).unwrap();
        assert!(content.contains("-x\nc++"));
        assert!(content.contains("--target=avr"));
        assert!(content.contains("-mmcu=atmega328p"));
        assert!(content.contains("-include\nArduino.h"));
        assert!(!content.contains(".arduino_prototypes.h"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_ino_prototypes() {
        let code = r#"
#include <Arduino.h>

void setup() {
    init();
}

void loop() {
    testdrawline();
}

void testdrawline() {
    // draw line
}

int readSensor(int pin, bool calibrate) {
    return 42;
}

static const uint8_t* getPtr(void) {
    return nullptr;
}
"#;
        let protos = extract_ino_prototypes(code);
        assert_eq!(protos.len(), 5);
        assert_eq!(protos[0], "void setup();");
        assert_eq!(protos[1], "void loop();");
        assert_eq!(protos[2], "void testdrawline();");
        assert_eq!(protos[3], "int readSensor(int pin, bool calibrate);");
        assert_eq!(protos[4], "static const uint8_t* getPtr(void);");
    }
}
