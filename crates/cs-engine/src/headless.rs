//! Headless CLI: `-nogui` / `-runcirc` / `-test` without a QApp.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::{Circuit, Error, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenKind {
    Circuit(PathBuf),
    Editor(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cli {
    Gui { open: Option<OpenKind> },
    RunCirc { path: PathBuf },
    Test { folder: PathBuf },
    Help,
    Error(String),
}

pub fn parse_args<I, S>(args: I) -> Cli
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let _argv0 = args.next();
    let rest: Vec<String> = args.map(|s| s.as_ref().to_string()).collect();
    parse_rest(&rest)
}

fn parse_rest(args: &[String]) -> Cli {
    let mut nogui = false;
    let mut runcirc: Option<PathBuf> = None;
    let mut test: Option<PathBuf> = None;
    let mut open: Option<OpenKind> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "-h" || arg == "--help" {
            return Cli::Help;
        }
        if arg == "-noproject" {
            i += 1;
            continue;
        }
        if arg == "-preview" || arg == "--preview" {
            i += 1;
            continue;
        }
        if arg == "-nogui" {
            nogui = true;
            i += 1;
            continue;
        }
        if arg == "-runcirc" {
            i += 1;
            let Some(file) = args.get(i) else {
                return Cli::Error("usage: -runcirc <circuit file>".into());
            };
            runcirc = Some(PathBuf::from(normalize_path(file)));
            break;
        }
        if arg == "-test" {
            i += 1;
            let Some(folder) = args.get(i) else {
                return Cli::Error("ERROR: missing argument for -test".into());
            };
            test = Some(PathBuf::from(folder));
            break;
        }
        if arg == "-leaktest" {
            return Cli::Error(
                "-leaktest is not supported (C++ leaktrack is not part of the Rust port)".into(),
            );
        }
        if arg.starts_with('-') {
            return Cli::Error(format!("ERROR: unrecognized argument {arg}"));
        }
        let path = normalize_path(arg);
        let p = Path::new(&path);
        if !p.exists() {
            return Cli::Error(format!("ERROR: unrecognized argument {arg}"));
        }
        if is_circuit(p) {
            open = Some(OpenKind::Circuit(p.to_path_buf()));
        } else {
            open = Some(OpenKind::Editor(p.to_path_buf()));
        }
        break;
    }

    if let Some(folder) = test {
        return Cli::Test { folder };
    }
    if let Some(path) = runcirc {
        return Cli::RunCirc { path };
    }
    if nogui {
        return match open {
            Some(OpenKind::Circuit(path)) => Cli::RunCirc { path },
            Some(OpenKind::Editor(_)) => Cli::Error(
                "-nogui needs a .circ1 / .sim1 / .sim2 circuit (or -runcirc / -test)".into(),
            ),
            None => {
                Cli::Error("-nogui needs -runcirc <file>, -test <folder>, or a circuit file".into())
            }
        };
    }
    Cli::Gui { open }
}

pub fn help_text() -> &'static str {
    "Circuit Simulator

Usage:
  circuitsimulator [file]
  circuitsimulator -nogui -runcirc <circuit.circ1>
  circuitsimulator -nogui -test <folder>
  circuitsimulator -h

Options:
  -nogui              Run without the QML shell
  -runcirc <file>     Load a circuit, power on, run until interrupted
  -test <folder>      Batch-test every .circ1 / .sim1 / .sim2 under folder
  -noproject          Ignored (no last-project restore yet)
  -h, --help          Show this help
"
}

fn is_circuit(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("circ1") | Some("sim1") | Some("sim2")
    )
}

/// C++ `main.cpp` file:// / %20 / Windows leading-slash strip.
pub fn normalize_path(arg: &str) -> String {
    let mut s = arg.to_string();
    if let Some(rest) = s.strip_prefix("file://") {
        s = rest.to_string();
    }
    s = s.replace("\r\n", "").replace("%20", " ");
    #[cfg(windows)]
    {
        if s.starts_with('/') {
            s.remove(0);
        }
    }
    s
}

/// C++ `LeakTest::runAndHold`: load, solve, step until `keep_going` is false.
pub fn run_circ(path: &Path, mut keep_going: impl FnMut(&Circuit) -> bool) -> Result<()> {
    if !path.exists() {
        return Err(Error::Parse(format!(
            "circuit not found: {}",
            path.display()
        )));
    }
    crate::logging::log_default(format!("[runcirc] loading {}", path.display()));
    let mut c = Circuit::load_sim1_path(path)?;
    c.solve()?;
    crate::logging::log_default("[runcirc] running");
    while keep_going(&c) {
        c.step_n(2_000)?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct BatchReport {
    pub tested: usize,
    pub failed: Vec<String>,
    pub skipped: Vec<String>,
}

impl BatchReport {
    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// C++ `BatchTest::doBatchTest`.
pub fn run_batch_folder(folder: &Path) -> Result<BatchReport> {
    if !folder.is_dir() {
        return Err(Error::Parse(format!(
            "Folder doesn't exist:\n{}",
            folder.display()
        )));
    }
    let files = collect_circuits(folder);
    let mut report = BatchReport {
        tested: 0,
        failed: Vec::new(),
        skipped: Vec::new(),
    };
    for file in files {
        crate::logging::log_default(format!("Testing {}", file.display()));
        match run_one_test(&file) {
            Ok(TestFile::Passed) => report.tested += 1,
            Ok(TestFile::Skipped(why)) => {
                report.tested += 1;
                report.skipped.push(format!("{} ({why})", file.display()));
            }
            Ok(TestFile::Failed(why)) => {
                report.tested += 1;
                report.failed.push(format!("{} ({why})", file.display()));
            }
            Err(e) => {
                report.tested += 1;
                report.failed.push(format!("{} ({e})", file.display()));
            }
        }
    }
    Ok(report)
}

pub fn print_batch_report(report: &BatchReport, mut out: impl Write) -> io::Result<()> {
    if report.ok() {
        if report.tested == 0 {
            writeln!(out, "No .circ1 / .sim1 / .sim2 files found")?;
        } else {
            writeln!(out, "All tests passed")?;
        }
        for s in &report.skipped {
            writeln!(out, "skipped {s}")?;
        }
    } else {
        writeln!(out, "{} Tests failed:", report.failed.len())?;
        for f in &report.failed {
            writeln!(out, "{f}")?;
        }
    }
    Ok(())
}

enum TestFile {
    Passed,
    Skipped(String),
    Failed(String),
}

fn run_one_test(path: &Path) -> Result<TestFile> {
    let mut c = Circuit::load_sim1_path(path)?;
    let results = c.run_batch()?;
    if results.is_empty() {
        return Ok(TestFile::Skipped("no TestUnit".into()));
    }
    if results.iter().all(|r| r.ok) {
        Ok(TestFile::Passed)
    } else {
        let names: Vec<&str> = results
            .iter()
            .filter(|r| !r.ok)
            .map(|r| r.id.as_str())
            .collect();
        Ok(TestFile::Failed(format!(
            "{} failed: {}",
            names.len(),
            names.join(", ")
        )))
    }
}

fn collect_circuits(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_circuits_into(dir, &mut out);
    out.sort();
    out
}

fn collect_circuits_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if is_circuit(&path) {
            out.push(path);
        }
    }
    dirs.sort();
    for d in dirs {
        collect_circuits_into(&d, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_help() {
        assert_eq!(parse_args(["cs", "-h"]), Cli::Help);
        assert_eq!(parse_args(["cs", "--help"]), Cli::Help);
    }

    #[test]
    fn parse_runcirc() {
        match parse_args(["cs", "-runcirc", "a.sim1"]) {
            Cli::RunCirc { path } => assert_eq!(path, PathBuf::from("a.sim1")),
            other => panic!("{other:?}"),
        }
        match parse_args(["cs", "-nogui", "-runcirc", "a.sim1"]) {
            Cli::RunCirc { path } => assert_eq!(path, PathBuf::from("a.sim1")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_test() {
        match parse_args(["cs", "-test", "/tmp/t"]) {
            Cli::Test { folder } => assert_eq!(folder, PathBuf::from("/tmp/t")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_nogui_needs_work() {
        match parse_args(["cs", "-nogui"]) {
            Cli::Error(_) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_gui_passthrough() {
        match parse_args(["cs"]) {
            Cli::Gui { open: None } => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_runcirc_missing() {
        match parse_args(["cs", "-runcirc"]) {
            Cli::Error(msg) => assert!(msg.contains("-runcirc")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn file_url_strip() {
        #[cfg(not(windows))]
        {
            assert_eq!(normalize_path("file:///tmp/a.sim1"), "/tmp/a.sim1");
            assert_eq!(
                normalize_path("file:///tmp/my%20circ.sim1"),
                "/tmp/my circ.sim1"
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(normalize_path("file:///C:/tmp/a.sim1"), "C:/tmp/a.sim1");
            assert_eq!(
                normalize_path("file:///C:/tmp/my%20circ.sim1"),
                "C:/tmp/my circ.sim1"
            );
        }
    }
}
