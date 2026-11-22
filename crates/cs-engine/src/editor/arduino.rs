//! Arduino compilation via arduino-cli. C++ InoDebugger::compile.

use std::cmp;
use std::collections::HashMap;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use super::compiler::{
    CompileResult, DEFAULT_COMPILE_TIMEOUT, command_exists, parse_error_lines,
    run_command_logged_cancel,
};
use super::lsp::strip_windows_verbatim;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn cli_name() -> &'static str {
    if cfg!(windows) {
        "arduino-cli.exe"
    } else {
        "arduino-cli"
    }
}

fn builder_name() -> &'static str {
    if cfg!(windows) {
        "arduino-builder.exe"
    } else {
        "arduino-builder"
    }
}

/// Electron `resources/app` folders for Arduino IDE 2.x.
/// Windows/Linux: `<install>/resources/app`
/// macOS: `<install>.app/Contents/Resources/app`
fn ide2_resource_roots(install: &Path) -> Vec<PathBuf> {
    let mut roots = vec![
        install.join("resources/app"),
        install.join("Contents/Resources/app"),
    ];
    // The user (or a candidate) may already be pointing at Resources or app.
    if let Some(name) = install.file_name() {
        if name == "app" {
            roots.push(install.to_path_buf());
        } else if name == "Resources" {
            roots.push(install.join("app"));
        }
    }
    roots
}

fn find_exe_shallow(dir: &Path, file_name: &str, max_depth: i32) -> Option<PathBuf> {
    if max_depth < 0 || !dir.is_dir() {
        return None;
    }
    let rd = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_file() && name_str == file_name {
            return Some(path);
        }
        if path.is_dir() && name_str != "node_modules" {
            subdirs.push(path);
        }
    }
    for d in subdirs {
        if let Some(found) = find_exe_shallow(&d, file_name, max_depth - 1) {
            return Some(found);
        }
    }
    None
}

fn find_cli_in_install(install: &Path) -> Option<PathBuf> {
    let cli = cli_name();
    let direct = install.join(cli);
    if direct.is_file() {
        return Some(direct);
    }
    for root in ide2_resource_roots(install) {
        if let Some(found) = find_exe_shallow(&root, cli, 6) {
            return Some(found);
        }
    }
    None
}

/// Locate the `arduino-cli` binary.
///
/// If `tool_path` is non-empty, treat it as an Arduino IDE install (or a
/// folder that directly contains the CLI). Otherwise check PATH, then well
/// known IDE locations.
pub fn find_arduino_cli(tool_path: &str) -> Option<PathBuf> {
    if !tool_path.is_empty() {
        return find_cli_in_install(Path::new(tool_path));
    }

    if command_exists("arduino-cli") {
        return Some(PathBuf::from("arduino-cli"));
    }

    for cand in tool_path_candidates() {
        if let Some(found) = find_cli_in_install(Path::new(&cand)) {
            return Some(found);
        }
    }
    None
}

/// C++ `InoDebugger::isArduinoInstallDir`, plus macOS `.app` bundles.
pub fn is_arduino_install_dir(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let dir = Path::new(path);
    let builder = builder_name();
    if dir.join(builder).is_file() || dir.join("Contents/Java").join(builder).is_file() {
        return true;
    }
    find_cli_in_install(dir).is_some()
}

/// C++ `InoDebugger::checkToolPath`.
pub fn check_tool_path(path: &str) -> String {
    if is_arduino_install_dir(path) {
        String::new()
    } else {
        "No Arduino IDE found here (expected arduino-builder, or an IDE 2.x install with resources/app)"
            .into()
    }
}

fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        if let Some(local) = dirs::data_local_dir() {
            roots.push(local.join("Programs"));
        }
        roots.push(PathBuf::from("C:/Program Files"));
        roots.push(PathBuf::from("C:/Program Files (x86)"));
    }
    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Applications"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Applications"));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        roots.push(PathBuf::from("/opt"));
        roots.push(PathBuf::from("/usr/share"));
        roots.push(PathBuf::from("/usr/local/share"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home);
        }
    }
    roots
}

fn push_candidate(candidates: &mut Vec<String>, path: &Path) {
    let mut candidate = path.to_string_lossy().into_owned();
    if !candidate.ends_with('/') && !candidate.ends_with('\\') {
        candidate.push('/');
    }
    if is_arduino_install_dir(&candidate) && !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

/// C++ `InoDebugger::toolPathCandidates`.
pub fn tool_path_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    for root in search_roots() {
        let Ok(rd) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if !entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("arduino")
            {
                continue;
            }
            push_candidate(&mut candidates, &path);
        }
    }
    candidates
}

fn add_inc(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_dir() && !paths.contains(&path) {
        paths.push(path);
    }
}

fn add_inc_subdirs(paths: &mut Vec<PathBuf>, base: &Path) {
    if !base.is_dir() {
        return;
    }
    add_inc(paths, base.to_path_buf());
    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.starts_with('.')
                    && name_str != "node_modules"
                    && name_str != "build"
                    && name_str != "target"
                {
                    add_inc(paths, p.clone());
                    let src = p.join("src");
                    if src.is_dir() {
                        add_inc(paths, src);
                    }
                    let utility = p.join("utility");
                    if utility.is_dir() {
                        add_inc(paths, utility);
                    }
                }
            }
        }
    }
}

pub fn arduino15_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "macos")]
        dirs.push(home.join("Library/Arduino15"));
        dirs.push(home.join(".arduino15"));
    }
    #[cfg(windows)]
    {
        if let Some(local) = dirs::data_local_dir() {
            dirs.push(local.join("Arduino15"));
        }
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".arduino15"));
            dirs.push(home.join("AppData/Local/Arduino15"));
        }
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

pub fn arduino_user_library_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(doc) = dirs::document_dir() {
        dirs.push(doc.join("Arduino/libraries"));
        dirs.push(doc.join("Arduino/Libraries"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Arduino/libraries"));
        dirs.push(home.join("Documents/Arduino/libraries"));
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

pub fn target_arch_for_board(board: &str, fqbn: &str) -> &'static str {
    let b = board.to_ascii_lowercase();
    let f = fqbn.to_ascii_lowercase();
    if b.contains("esp32") || f.contains("esp32") {
        "esp32"
    } else if b.contains("samd") || f.contains("samd") {
        "samd"
    } else if b.contains("rp2040") || f.contains("rp2040") {
        "rp2040"
    } else {
        "avr"
    }
}

/// Discovers include paths for Arduino sketches so that clangd / LSP can resolve
/// `#include <Arduino.h>`, standard libraries (Wire, SPI), and user libraries (e.g. Adafruit_SSD1306).
pub fn arduino_include_paths(
    file_path: &str,
    tool_path: &str,
    custom_incl: &str,
    board: &str,
) -> Vec<PathBuf> {
    let target_arch = target_arch_for_board(board, "");
    let mut paths = Vec::new();

    // 1. User libraries (e.g. ~/Documents/Arduino/libraries)
    for lib_root in arduino_user_library_dirs() {
        add_inc_subdirs(&mut paths, &lib_root);
    }

    // 2. Arduino15 data packages (cores, variants, avr-gcc include, libraries)
    let is_mega = board.to_ascii_lowercase().contains("mega");
    for a15 in arduino15_dirs() {
        add_inc_subdirs(&mut paths, &a15.join("libraries"));

        let packages = a15.join("packages");
        if packages.is_dir() {
            if let Ok(pkg_entries) = std::fs::read_dir(&packages) {
                for pkg_entry in pkg_entries.flatten() {
                    let hw_dir = pkg_entry.path().join("hardware");
                    if let Ok(arch_entries) = std::fs::read_dir(&hw_dir) {
                        for arch_entry in arch_entries.flatten() {
                            let arch_name = arch_entry.file_name();
                            let arch_str = arch_name.to_string_lossy().to_ascii_lowercase();
                            if arch_str != target_arch {
                                continue;
                            }
                            if let Ok(ver_entries) = std::fs::read_dir(arch_entry.path()) {
                                for ver_entry in ver_entries.flatten() {
                                    let vpath = ver_entry.path();
                                    if !vpath.is_dir() {
                                        continue;
                                    }
                                    add_inc(&mut paths, vpath.join("cores/arduino"));
                                    if target_arch == "esp32" {
                                        add_inc(&mut paths, vpath.join("cores/esp32"));
                                        add_inc(&mut paths, vpath.join("variants/esp32"));
                                    }
                                    if is_mega {
                                        add_inc(&mut paths, vpath.join("variants/mega"));
                                    }
                                    add_inc(&mut paths, vpath.join("variants/standard"));
                                    add_inc_subdirs(&mut paths, &vpath.join("libraries"));
                                }
                            }
                        }
                    }

                    if target_arch == "avr" {
                        let tools_dir = pkg_entry.path().join("tools");
                        if let Ok(tool_entries) = fs::read_dir(&tools_dir) {
                            for tool_entry in tool_entries.flatten() {
                                let tool_name = tool_entry.file_name();
                                let tool_str = tool_name.to_string_lossy().to_ascii_lowercase();
                                if !tool_str.contains("avr") {
                                    continue;
                                }
                                if let Ok(ver_entries) = fs::read_dir(tool_entry.path()) {
                                    for ver_entry in ver_entries.flatten() {
                                        let vpath = ver_entry.path();
                                        if vpath.is_dir() {
                                            add_inc(&mut paths, vpath.join("avr/include"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Tool path and parents (classical Arduino IDE structure)
    if !tool_path.is_empty() {
        let mut cur = Some(PathBuf::from(tool_path));
        for _ in 0..5 {
            if let Some(c) = cur {
                if c.is_dir() {
                    if target_arch == "avr" {
                        add_inc(&mut paths, c.join("hardware/arduino/avr/cores/arduino"));
                        if is_mega {
                            add_inc(&mut paths, c.join("hardware/arduino/avr/variants/mega"));
                        }
                        add_inc(&mut paths, c.join("hardware/arduino/avr/variants/standard"));
                        add_inc(&mut paths, c.join("hardware/tools/avr/avr/include"));
                        add_inc_subdirs(&mut paths, &c.join("hardware/arduino/avr/libraries"));
                    }
                    add_inc(&mut paths, c.join("cores/arduino"));
                    add_inc(&mut paths, c.join("variants/standard"));
                    add_inc_subdirs(&mut paths, &c.join("libraries"));
                }
                cur = c.parent().map(|p| p.to_path_buf());
            } else {
                break;
            }
        }
    }

    // 4. Sketch directory
    if !file_path.is_empty() {
        if let Some(parent) = Path::new(file_path).parent() {
            add_inc(&mut paths, parent.join("src"));
            add_inc_subdirs(&mut paths, &parent.join("libraries"));
        }
    }

    // 5. Custom include paths
    if !custom_incl.is_empty() {
        for p in custom_incl.split([';', ',', '\n']) {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                add_inc_subdirs(&mut paths, Path::new(trimmed));
            }
        }
    }

    paths
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Esp32BoardMeta {
    pub mcu: String,
    pub tarch: String,
    pub variant: String,
    pub board_define: String,
    pub f_cpu: String,
    pub defines: Vec<String>,
}

impl Default for Esp32BoardMeta {
    fn default() -> Self {
        Self {
            mcu: "esp32".into(),
            tarch: "xtensa".into(),
            variant: "esp32".into(),
            board_define: "ESP32_DEV".into(),
            f_cpu: "240000000L".into(),
            defines: Vec::new(),
        }
    }
}

fn infer_esp32_mcu(board: &str, fqbn: &str) -> &'static str {
    let combined = format!("{} {}", board, fqbn).to_ascii_lowercase();
    for mcu in [
        "esp32s3", "esp32-s3", "esp32s2", "esp32-s2", "esp32c6", "esp32-c6", "esp32c5", "esp32-c5",
        "esp32c3", "esp32-c3", "esp32h2", "esp32-h2", "esp32p4", "esp32-p4",
    ] {
        if combined.contains(mcu) {
            return match mcu {
                "esp32s3" | "esp32-s3" => "esp32s3",
                "esp32s2" | "esp32-s2" => "esp32s2",
                "esp32c6" | "esp32-c6" => "esp32c6",
                "esp32c5" | "esp32-c5" => "esp32c5",
                "esp32c3" | "esp32-c3" => "esp32c3",
                "esp32h2" | "esp32-h2" => "esp32h2",
                "esp32p4" | "esp32-p4" => "esp32p4",
                _ => "esp32",
            };
        }
    }
    "esp32"
}

fn default_tarch_for_mcu(mcu: &str) -> &'static str {
    let lower = mcu.to_ascii_lowercase();
    if lower.starts_with("esp32c") || lower.starts_with("esp32h") || lower.starts_with("esp32p") {
        "riscv32"
    } else {
        "xtensa"
    }
}

/// Extract ESP32 board build metadata (`build.mcu`, `build.tarch`, `build.variant`, etc.)
/// from Arduino ESP32 `boards.txt` using either the FQBN board id or the UI board name.
pub fn parse_esp32_board_meta(boards_txt: &str, board_name: &str, fqbn: &str) -> Esp32BoardMeta {
    let inferred_mcu = infer_esp32_mcu(board_name, fqbn);
    let mut meta = Esp32BoardMeta {
        mcu: inferred_mcu.into(),
        tarch: default_tarch_for_mcu(inferred_mcu).into(),
        variant: inferred_mcu.into(),
        ..Default::default()
    };

    // 1. Try FQBN board id (3rd colon-separated field: vendor:arch:board[:options])
    let mut board_id: Option<String> = fqbn
        .split(':')
        .nth(2)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // 2. Otherwise match `<id>.name=<board_name>` in boards.txt
    if board_id.is_none() && !board_name.is_empty() {
        let want = board_name.trim();
        for line in boards_txt.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                if let Some(id) = k.strip_suffix(".name") {
                    if v.trim().eq_ignore_ascii_case(want) {
                        board_id = Some(id.to_string());
                        break;
                    }
                }
            }
        }
    }

    let Some(id) = board_id else {
        return meta;
    };

    let prefix = format!("{id}.build.");
    for line in boards_txt.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let Some((key, val)) = rest.split_once('=') else {
            continue;
        };
        let val = val.trim();
        if val.is_empty() {
            continue;
        }
        match key {
            "mcu" => meta.mcu = val.to_ascii_lowercase(),
            "tarch" => meta.tarch = val.to_ascii_lowercase(),
            "variant" => meta.variant = val.to_string(),
            "board" => meta.board_define = val.to_string(),
            "f_cpu" => meta.f_cpu = val.to_string(),
            "defines" => {
                for tok in val.split_whitespace() {
                    if tok.starts_with("-D") {
                        meta.defines.push(tok.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    meta
}

fn latest_subdir(parent: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(parent)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort_by(|a, b| {
        let na = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let nb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        version_cmp(na, nb)
    });
    dirs.pop()
}

fn version_cmp(a: &str, b: &str) -> cmp::Ordering {
    let parts_a: Vec<&str> = a.split(['.', '-', '_']).collect();
    let parts_b: Vec<&str> = b.split(['.', '-', '_']).collect();
    for (pa, pb) in parts_a.iter().zip(parts_b.iter()) {
        let ord = match (pa.parse::<u64>(), pb.parse::<u64>()) {
            (Ok(na), Ok(nb)) => na.cmp(&nb),
            _ => pa.cmp(pb),
        };
        if ord != cmp::Ordering::Equal {
            return ord;
        }
    }
    parts_a.len().cmp(&parts_b.len())
}

fn find_esp32_package_root() -> Option<(PathBuf, PathBuf)> {
    for a15 in arduino15_dirs() {
        let pkg = a15.join("packages/esp32");
        let hw = pkg.join("hardware/esp32");
        if let Some(core_ver) = latest_subdir(&hw) {
            return Some((pkg, core_ver));
        }
    }
    None
}

fn push_flag_pair(out: &mut Vec<String>, flag: &str, path: &Path) {
    if path.is_dir() {
        let s = strip_windows_verbatim(&path.to_string_lossy());
        if !out
            .windows(2)
            .any(|w| w[0] == flag && w[1].eq_ignore_ascii_case(&s))
        {
            out.push(flag.to_string());
            out.push(s);
        }
    }
}

fn add_esp32_toolchain_sys_includes(out: &mut Vec<String>, pkg_root: &Path, meta: &Esp32BoardMeta) {
    let tools = pkg_root.join("tools");
    let is_riscv = meta.tarch == "riscv32";
    let triple = if is_riscv {
        "riscv32-esp-elf"
    } else {
        "xtensa-esp-elf"
    };

    let candidate_tool_dirs: Vec<PathBuf> = if is_riscv {
        vec![tools.join("esp-rv32"), tools.join("riscv32-esp-elf-gcc")]
    } else {
        vec![
            tools.join("esp-x32"),
            tools.join(format!("xtensa-{}-elf-gcc", meta.mcu)),
            tools.join("xtensa-esp32-elf-gcc"),
        ]
    };

    let Some(tc_root) = candidate_tool_dirs.iter().find_map(|d| latest_subdir(d)) else {
        return;
    };

    // Match either `<triple>` (esp-x32 14.x) or `xtensa-esp32-elf` (legacy 2.x).
    let triple_candidates = [
        triple.to_string(),
        format!("xtensa-{}-elf", meta.mcu),
        "xtensa-esp32-elf".to_string(),
    ];
    let Some((actual_triple, target_dir)) = triple_candidates.iter().find_map(|t| {
        let direct = tc_root.join(t);
        if direct.join("include").is_dir() {
            return Some((t.clone(), direct));
        }
        let pico = tc_root.join("picolibc").join(t);
        if pico.join("include").is_dir() {
            return Some((t.clone(), pico));
        }
        None
    }) else {
        return;
    };

    let inc_dir = target_dir.join("include");
    if let Some(cxx_ver_dir) = latest_subdir(&inc_dir.join("c++")) {
        push_flag_pair(out, "-isystem", &cxx_ver_dir);
        let arch_mcu = cxx_ver_dir.join(&actual_triple).join(&meta.mcu);
        if arch_mcu.is_dir() {
            push_flag_pair(out, "-isystem", &arch_mcu);
        } else {
            push_flag_pair(out, "-isystem", &cxx_ver_dir.join(&actual_triple));
        }
        push_flag_pair(out, "-isystem", &cxx_ver_dir.join("backward"));
    }

    // GCC include-fixed (if present under lib/gcc/<triple>/<ver>/include-fixed)
    if let Some(gcc_ver_dir) = latest_subdir(&tc_root.join("lib/gcc").join(&actual_triple)) {
        push_flag_pair(out, "-isystem", &gcc_ver_dir.join("include-fixed"));
    }

    push_flag_pair(out, "-isystem", &inc_dir);
    push_flag_pair(out, "-isystem", &target_dir.join("sys-include"));
}

/// Build clangd-compatible compile flags for an ESP32 Arduino sketch.
pub fn esp32_compile_flags(
    file_path: &str,
    custom_incl: &str,
    board: &str,
    fqbn: &str,
) -> Vec<String> {
    let mut flags = vec!["-x".into(), "c++".into(), "-std=gnu++20".into()];

    let pkg_and_core = find_esp32_package_root();
    let meta = if let Some((_, ref core_dir)) = pkg_and_core {
        let boards_txt = fs::read_to_string(core_dir.join("boards.txt")).unwrap_or_default();
        parse_esp32_board_meta(&boards_txt, board, fqbn)
    } else {
        parse_esp32_board_meta("", board, fqbn)
    };

    let target_triple = if meta.tarch == "riscv32" {
        "riscv32-esp-elf"
    } else {
        "xtensa-esp-elf"
    };
    flags.push(format!("--target={target_triple}"));
    flags.push(format!("-DF_CPU={}", meta.f_cpu));
    flags.push("-DARDUINO=10819".into());
    flags.push("-DARDUINO_ARCH_ESP32".into());
    if !meta.board_define.is_empty() {
        flags.push(format!("-DARDUINO_{}", meta.board_define));
    }
    let mcu_upper = meta.mcu.to_ascii_uppercase();
    flags.push(format!("-D{mcu_upper}={mcu_upper}"));
    flags.push("-DCORE_DEBUG_LEVEL=0".into());
    flags.push("-DARDUINO_USB_CDC_ON_BOOT=0".into());
    for def in &meta.defines {
        if !flags.contains(def) {
            flags.push(def.clone());
        }
    }

    if let Some((pkg_root, core_dir)) = pkg_and_core {
        // Core 3.x: packages/esp32/tools/<mcu>-libs/<ver>
        // Core 2.x: packages/esp32/hardware/esp32/<ver>/tools/sdk/<mcu>
        let core_ver_name = core_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let libs_parent = pkg_root.join("tools").join(format!("{}-libs", meta.mcu));
        let libs_dir = {
            let exact = libs_parent.join(core_ver_name);
            if exact.is_dir() {
                Some(exact)
            } else {
                latest_subdir(&libs_parent).or_else(|| {
                    let legacy = core_dir.join("tools/sdk").join(&meta.mcu);
                    legacy.is_dir().then_some(legacy)
                })
            }
        };

        if let Some(ref libs) = libs_dir {
            // Read SDK preprocessor defines
            if let Ok(defines_txt) = fs::read_to_string(libs.join("flags/defines")) {
                for tok in defines_txt.split_whitespace() {
                    if tok.starts_with("-D") && !flags.contains(&tok.to_string()) {
                        flags.push(tok.to_string());
                    }
                }
            }
        }

        // Core & variant headers
        push_flag_pair(&mut flags, "-I", &core_dir.join("cores/esp32"));
        let variant_dir = core_dir.join("variants").join(&meta.variant);
        if variant_dir.is_dir() {
            push_flag_pair(&mut flags, "-I", &variant_dir);
        } else {
            let fallback_variant = core_dir.join("variants").join(&meta.mcu);
            if fallback_variant.is_dir() {
                push_flag_pair(&mut flags, "-I", &fallback_variant);
            } else {
                push_flag_pair(&mut flags, "-I", &core_dir.join("variants/esp32"));
            }
        }

        if let Some(ref libs) = libs_dir {
            // sdkconfig.h directory (dio_qspi / qio_qspi / dio_opi / qio_opi)
            for cfg_subdir in [
                "dio_qspi/include",
                "qio_qspi/include",
                "dio_opi/include",
                "qio_opi/include",
                "include/config",
            ] {
                let cand = libs.join(cfg_subdir);
                if cand.join("sdkconfig.h").is_file() {
                    push_flag_pair(&mut flags, "-I", &cand);
                    break;
                }
            }

            // Expand SDK includes list (`-iwithprefixbefore <rel>`)
            if let Ok(inc_txt) = fs::read_to_string(libs.join("flags/includes")) {
                let inc_base = libs.join("include");
                let toks: Vec<&str> = inc_txt.split_whitespace().collect();
                let mut i = 0;
                while i < toks.len() {
                    if toks[i] == "-iwithprefixbefore" && i + 1 < toks.len() {
                        let p = inc_base.join(toks[i + 1]);
                        push_flag_pair(&mut flags, "-I", &p);
                        i += 2;
                    } else if let Some(rest) = toks[i].strip_prefix("-I") {
                        let p = if rest.is_empty() && i + 1 < toks.len() {
                            i += 1;
                            PathBuf::from(toks[i])
                        } else {
                            PathBuf::from(rest)
                        };
                        push_flag_pair(&mut flags, "-I", &p);
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
            }
        }

        // Core bundled libraries (WiFi, Wire, SPI, Preferences, etc.)
        let mut extra_inc = Vec::new();
        add_inc_subdirs(&mut extra_inc, &core_dir.join("libraries"));
        for lib_root in arduino_user_library_dirs() {
            add_inc_subdirs(&mut extra_inc, &lib_root);
        }
        for a15 in arduino15_dirs() {
            add_inc_subdirs(&mut extra_inc, &a15.join("libraries"));
        }
        if !file_path.is_empty() {
            if let Some(parent) = Path::new(file_path).parent() {
                add_inc(&mut extra_inc, parent.join("src"));
                add_inc_subdirs(&mut extra_inc, &parent.join("libraries"));
            }
        }
        if !custom_incl.is_empty() {
            for p in custom_incl.split([';', ',', '\n']) {
                let trimmed = p.trim();
                if !trimmed.is_empty() {
                    add_inc_subdirs(&mut extra_inc, Path::new(trimmed));
                }
            }
        }
        for inc in extra_inc {
            push_flag_pair(&mut flags, "-I", &inc);
        }

        // Toolchain C++ and libc sysroot headers
        add_esp32_toolchain_sys_includes(&mut flags, &pkg_root, &meta);
    }

    flags.push("-include".into());
    flags.push("Arduino.h".into());
    flags
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardEntry {
    pub name: String,
    pub fqbn: String,
}

/// Default built-in boards matching C++ InoDebugger.
pub fn default_boards() -> Vec<BoardEntry> {
    vec![
        BoardEntry {
            name: "Uno".into(),
            fqbn: "arduino:avr:uno".into(),
        },
        BoardEntry {
            name: "Mega".into(),
            fqbn: "arduino:avr:megaADK".into(),
        },
        BoardEntry {
            name: "Nano".into(),
            fqbn: "arduino:avr:nano".into(),
        },
        BoardEntry {
            name: "Duemilanove".into(),
            fqbn: "arduino:avr:diecimila".into(),
        },
    ]
}

/// Parse stdout from `arduino-cli board listall` into `BoardEntry` items.
/// C++ InoDebugger::setToolPath line parsing.
pub fn parse_board_list(stdout: &str) -> Vec<BoardEntry> {
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Board Name") {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 {
            continue;
        }
        let fqbn = tokens.last().unwrap().to_string();
        if !fqbn.contains(':') {
            continue;
        }
        let name_tokens = &tokens[..tokens.len() - 1];
        let name = name_tokens.join(" ").replace([',', ';'], "");
        if !name.is_empty() {
            entries.push(BoardEntry { name, fqbn });
        }
    }
    entries
}

/// Discovered Arduino boards: default boards plus boards from `arduino-cli board listall`.
pub fn list_arduino_boards(tool_path: &str) -> Vec<BoardEntry> {
    let mut list = default_boards();
    if let Some(cli) = find_arduino_cli(tool_path) {
        let mut cmd = Command::new(&cli);
        cmd.args(["board", "listall"]);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(out) = cmd.output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for entry in parse_board_list(&stdout) {
                if !list
                    .iter()
                    .any(|b| b.name.eq_ignore_ascii_case(&entry.name))
                {
                    list.push(entry);
                }
            }
        }
    }
    list
}

/// Formatted board list for UI ComboBox (Uno, Mega, Nano, Duemilanove, Custom, ...discovered).
pub fn board_names_for_ui(boards: &[BoardEntry]) -> Vec<String> {
    let mut names = Vec::new();
    for def in ["Uno", "Mega", "Nano", "Duemilanove"] {
        if boards.iter().any(|b| b.name == def) && !names.contains(&def.to_string()) {
            names.push(def.to_string());
        }
    }
    names.push("Custom".to_string());
    for b in boards {
        if !names.contains(&b.name) && b.name != "Custom" {
            names.push(b.name.clone());
        }
    }
    names
}

/// Determine MCU device string for compile_flags.txt.
/// C++ InoDebugger::setBoard.
pub fn device_for_board_or_fqbn(board: &str, fqbn: &str) -> String {
    let b = board.to_ascii_lowercase();
    let f = fqbn.to_ascii_lowercase();
    if b == "mega" || f.contains("2560") || f.contains("mega") {
        "atmega2560".into()
    } else if b == "leonardo" || f.contains("32u4") || f.contains("leonardo") {
        "atmega32u4".into()
    } else if b.contains("esp32") || f.contains("esp32") {
        "esp32".into()
    } else {
        // Uno, Nano, Duemilanove, or generic AVR
        "atmega328p".into()
    }
}

/// Resolve FQBN for a given board choice.
pub fn resolve_board_fqbn(board: &str, custom_board: &str, boards: &[BoardEntry]) -> String {
    if board.eq_ignore_ascii_case("custom") {
        if !custom_board.is_empty() {
            return custom_board.to_string();
        }
        return "arduino:avr:uno".to_string();
    }
    if let Some(entry) = boards.iter().find(|b| b.name.eq_ignore_ascii_case(board)) {
        return entry.fqbn.clone();
    }
    if board.contains(':') {
        return board.to_string();
    }
    "arduino:avr:uno".to_string()
}

static DIO_BOARD_CACHE: LazyLock<Mutex<HashMap<String, (String, Option<String>)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ensure ESP32 boards use DIO flash mode for QEMU simulation.
/// C++ InoDebugger::forceDioFlashMode.
pub fn force_dio_flash_mode(fqbn: &str) -> String {
    let (resolved, _) = resolve_dio_flash_mode_with_cli(fqbn, find_arduino_cli("").as_deref());
    resolved
}

/// Query board details from `arduino-cli` (or cache) to ensure DIO flash mode is selected
/// if the board builds for QIO and supports the `FlashMode` menu option.
/// Returns `(resolved_fqbn, optional_log_message)`.
pub fn resolve_dio_flash_mode_with_cli(fqbn: &str, cli: Option<&Path>) -> (String, Option<String>) {
    if !is_esp32_fqbn(fqbn) {
        return (fqbn.to_string(), None);
    }
    // An explicit choice wins, even one QEMU can't honour
    if fqbn.to_ascii_lowercase().contains("flashmode=") {
        return (fqbn.to_string(), None);
    }

    if let Ok(map) = DIO_BOARD_CACHE.lock() {
        if let Some(cached) = map.get(fqbn) {
            return cached.clone();
        }
    }

    let Some(cli_path) = cli else {
        return fallback_dio_flash_mode(fqbn);
    };

    let mut cmd = Command::new(cli_path);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.args(["board", "details", "--fqbn", fqbn, "--format", "json"]);
    cmd.env("TERM", "dumb");
    cmd.env_remove("DYLD_LIBRARY_PATH");
    cmd.env_remove("DYLD_FALLBACK_LIBRARY_PATH");
    cmd.env_remove("DYLD_FRAMEWORK_PATH");

    let Ok(out) = cmd.output() else {
        return fallback_dio_flash_mode(fqbn);
    };

    if !out.status.success() {
        return fallback_dio_flash_mode(fqbn);
    }

    let Ok(board) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return fallback_dio_flash_mode(fqbn);
    };

    let result = parse_board_details_dio(&board, fqbn);
    if let Ok(mut map) = DIO_BOARD_CACHE.lock() {
        map.insert(fqbn.to_string(), result.clone());
    }
    result
}

/// Parse `arduino-cli board details --format json` output to determine whether DIO flash mode
/// should be forced or warned about.
pub fn parse_board_details_dio(board: &serde_json::Value, fqbn: &str) -> (String, Option<String>) {
    let mut boot = String::new();
    if let Some(build_props) = board.get("build_properties").and_then(|v| v.as_array()) {
        for prop in build_props {
            if let Some(p) = prop.as_str() {
                if let Some(val) = p.strip_prefix("build.boot=") {
                    boot = val.to_string();
                    break;
                }
            }
        }
    }

    let mut can_select_dio = false;
    if let Some(options) = board.get("config_options").and_then(|v| v.as_array()) {
        for opt in options {
            if opt.get("option").and_then(|o| o.as_str()) == Some("FlashMode") {
                if let Some(values) = opt.get("values").and_then(|v| v.as_array()) {
                    for val in values {
                        if val.get("value").and_then(|v| v.as_str()) == Some("dio") {
                            can_select_dio = true;
                            break;
                        }
                    }
                }
                break;
            }
        }
    }

    let mut result = fqbn.to_string();
    let mut log_msg = None;
    if !boot.is_empty() && boot != "dio" {
        if can_select_dio {
            let sep = if fqbn.matches(':').count() > 2 {
                ","
            } else {
                ":"
            };
            result = format!("{fqbn}{sep}FlashMode=dio");
            log_msg = Some("Using FlashMode=dio: QEMU cannot enable QIO flash mode".to_string());
        } else {
            log_msg = Some(
                "Warning: this board builds for QIO flash and has no FlashMode option to override. QEMU cannot enable QIO, so the firmware will likely panic at startup. Pick a board that builds DIO, e.g. \"DOIT ESP32 DEVKIT V1\".".to_string(),
            );
        }
    }

    (result, log_msg)
}

fn fallback_dio_flash_mode(fqbn: &str) -> (String, Option<String>) {
    let fqbn_lower = fqbn.to_ascii_lowercase();
    // Boards like "esp32doit-devkit-v1" build for DIO by default and have no FlashMode menu option.
    if fqbn_lower.contains("doit") {
        return (fqbn.to_string(), None);
    }
    // Generic "esp32:esp32:esp32" builds for QIO and supports FlashMode=dio.
    if fqbn_lower.starts_with("esp32:esp32:esp32") {
        let sep = if fqbn.matches(':').count() > 2 {
            ","
        } else {
            ":"
        };
        return (
            format!("{fqbn}{sep}FlashMode=dio"),
            Some("Using FlashMode=dio: QEMU cannot enable QIO flash mode".to_string()),
        );
    }
    (fqbn.to_string(), None)
}

/// Match saved settings (board name, custom board, legacy device/FQBN/MCU) against the board list.
/// Returns `(board_name, custom_board, mcu_device)`.
pub fn match_board(
    saved_board: &str,
    saved_custom: &str,
    saved_device: &str,
    boards: &[BoardEntry],
) -> (String, String, String) {
    let sb = saved_board.trim();
    let sc = saved_custom.trim();
    let sd = saved_device.trim();

    // 1. Explicitly Custom
    if sb.eq_ignore_ascii_case("custom") {
        let custom_val = if !sc.is_empty() {
            sc.to_string()
        } else if sd.contains(':') {
            sd.to_string()
        } else {
            String::new()
        };
        let mcu = device_for_board_or_fqbn("Custom", &custom_val);
        return ("Custom".into(), custom_val, mcu);
    }

    // Candidate query string to match: saved_board, or fallback to saved_device
    let query = if !sb.is_empty() && !sb.eq_ignore_ascii_case("none") {
        sb
    } else if !sd.is_empty() && !sd.eq_ignore_ascii_case("none") {
        sd
    } else {
        ""
    };

    if query.is_empty() {
        let def = "Uno";
        let fqbn = resolve_board_fqbn(def, "", boards);
        return (
            def.into(),
            String::new(),
            device_for_board_or_fqbn(def, &fqbn),
        );
    }

    // 2. Exact match on board name (case-sensitive)
    if let Some(entry) = boards.iter().find(|b| b.name == query) {
        return (
            entry.name.clone(),
            String::new(),
            device_for_board_or_fqbn(&entry.name, &entry.fqbn),
        );
    }

    // 3. Case-insensitive match on board name
    if let Some(entry) = boards.iter().find(|b| b.name.eq_ignore_ascii_case(query)) {
        return (
            entry.name.clone(),
            String::new(),
            device_for_board_or_fqbn(&entry.name, &entry.fqbn),
        );
    }

    // 4. Exact / case-insensitive match on FQBN
    // Prefer short default boards if multiple share the same FQBN
    let fqbn_matches: Vec<&BoardEntry> = boards
        .iter()
        .filter(|b| b.fqbn.eq_ignore_ascii_case(query))
        .collect();
    if !fqbn_matches.is_empty() {
        if let Some(entry) = fqbn_matches
            .iter()
            .find(|b| ["Uno", "Mega", "Nano", "Duemilanove"].contains(&b.name.as_str()))
        {
            return (
                entry.name.clone(),
                String::new(),
                device_for_board_or_fqbn(&entry.name, &entry.fqbn),
            );
        }
        let entry = fqbn_matches[0];
        return (
            entry.name.clone(),
            String::new(),
            device_for_board_or_fqbn(&entry.name, &entry.fqbn),
        );
    }

    // 5. Match by MCU device (e.g. "atmega328p" -> "Uno", "atmega2560" -> "Mega")
    let q_lower = query.to_ascii_lowercase();
    if q_lower == "atmega328p" {
        return ("Uno".into(), String::new(), "atmega328p".into());
    } else if q_lower == "atmega2560" {
        return ("Mega".into(), String::new(), "atmega2560".into());
    } else if q_lower == "atmega32u4" {
        if let Some(entry) = boards
            .iter()
            .find(|b| b.fqbn.contains("32u4") || b.name.eq_ignore_ascii_case("leonardo"))
        {
            return (entry.name.clone(), String::new(), "atmega32u4".into());
        }
    }

    // 6. Fuzzy / nickname matches
    if q_lower.contains("diecimila") {
        return ("Duemilanove".into(), String::new(), "atmega328p".into());
    } else if q_lower.contains("megaadk") || q_lower == "mega" {
        return ("Mega".into(), String::new(), "atmega2560".into());
    } else if q_lower == "nano" {
        return ("Nano".into(), String::new(), "atmega328p".into());
    } else if q_lower == "uno" {
        return ("Uno".into(), String::new(), "atmega328p".into());
    }

    // 7. Check if query is contained in any board name (e.g. "Uno" in "Arduino Uno")
    if let Some(entry) = boards.iter().find(|b| {
        let b_lower = b.name.to_ascii_lowercase();
        b_lower.split_whitespace().any(|word| word == q_lower)
    }) {
        return (
            entry.name.clone(),
            String::new(),
            device_for_board_or_fqbn(&entry.name, &entry.fqbn),
        );
    }

    // 8. If non-empty query did not match standard boards, treat as Custom
    let custom_val = if !sc.is_empty() {
        sc.to_string()
    } else {
        query.to_string()
    };
    let mcu = device_for_board_or_fqbn("Custom", &custom_val);
    ("Custom".into(), custom_val, mcu)
}

/// Compile an Arduino sketch using `arduino-cli`.
///
/// `file_path` is the `.ino` sketch path, `device` is the board FQBN
/// (e.g. `arduino:avr:uno`), and `tool_path` is an optional directory
/// to search for `arduino-cli`.
pub fn compile_arduino(file_path: &str, device: &str, tool_path: &str) -> CompileResult {
    compile_arduino_with_log(file_path, device, tool_path, &mut |_| {})
}

fn emit_log(r: &mut CompileResult, on_log: &mut dyn FnMut(&str), line: impl Into<String>) {
    let line = line.into();
    crate::logging::log_compiler(&line);
    on_log(&line);
    r.log.push(line);
}

pub fn compile_arduino_with_log(
    file_path: &str,
    device: &str,
    tool_path: &str,
    on_log: &mut dyn FnMut(&str),
) -> CompileResult {
    let dummy_cancel = AtomicBool::new(false);
    compile_arduino_with_log_cancel(file_path, device, tool_path, &dummy_cancel, on_log)
}

pub fn compile_arduino_with_log_cancel(
    file_path: &str,
    device: &str,
    tool_path: &str,
    cancel: &AtomicBool,
    on_log: &mut dyn FnMut(&str),
) -> CompileResult {
    if cancel.load(Ordering::Relaxed) {
        let mut r = CompileResult::default();
        emit_log(&mut r, on_log, "     Compilation cancelled by user.");
        r.error = -1;
        return r;
    }
    match start_arduino_compile(file_path, device, tool_path) {
        Err(r) => {
            for line in &r.log {
                crate::logging::log_compiler(line);
                on_log(line);
            }
            r
        }
        Ok((started, logs)) => {
            for line in &logs {
                crate::logging::log_compiler(line);
                on_log(line);
            }
            let file_path = started.file_path.clone();
            let fqbn = started.fqbn.clone();
            let mut cmd = started.cmd;
            match run_command_logged_cancel(&mut cmd, cancel, DEFAULT_COMPILE_TIMEOUT, on_log) {
                Ok((success, combined)) => {
                    if !success {
                        let build_dir = resolve_arduino_build_dir(&file_path, &fqbn);
                        let _ = std::fs::remove_file(build_dir.join("includes.cache"));
                        let _ = std::fs::remove_file(build_dir.join("libraries.cache"));
                    }
                    let mut res = finish_arduino_compile(&file_path, &fqbn, success, &combined);
                    for line in &res.log {
                        if *line != combined {
                            crate::logging::log_compiler(line);
                            on_log(line);
                        }
                    }
                    let mut full_log = logs;
                    full_log.extend(res.log);
                    res.log = full_log;
                    res
                }
                Err(e) => {
                    let build_dir = resolve_arduino_build_dir(&file_path, &fqbn);
                    let _ = std::fs::remove_file(build_dir.join("includes.cache"));
                    let _ = std::fs::remove_file(build_dir.join("libraries.cache"));
                    let mut r = CompileResult::default();
                    r.log = logs;
                    emit_log(&mut r, on_log, format!("ERROR: {e}"));
                    r.error = -1;
                    r
                }
            }
        }
    }
}

/// Prepared `arduino-cli compile` configuration.
pub struct StartedArduino {
    pub cmd: Command,
    pub file_path: String,
    pub fqbn: String,
    pub cli: PathBuf,
}

pub fn resolve_arduino_build_dir(file_path: &str, fqbn: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut s = DefaultHasher::new();
    file_path.hash(&mut s);
    let hash = format!("{:016x}", s.finish());
    let fqbn_dots = fqbn.replace(':', ".");
    let base = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("CircuitSimulator")
        .join("arduino_build");
    base.join(format!("{hash}_{fqbn_dots}"))
}

pub fn start_arduino_compile(
    file_path: &str,
    device: &str,
    tool_path: &str,
) -> Result<(StartedArduino, Vec<String>), CompileResult> {
    let mut logs = Vec::new();
    logs.push("Locating Arduino tools…".into());
    let cli = match find_arduino_cli(tool_path) {
        Some(p) => p,
        None => {
            let mut r = CompileResult::default();
            let err =
                "Error: arduino-cli not found. Ensure Arduino IDE or arduino-cli is installed."
                    .to_string();
            println!("{err}");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            logs.push(err);
            r.log = logs;
            r.error = -1;
            return Err(r);
        }
    };
    if device.is_empty() {
        let mut r = CompileResult::default();
        let err = "Error: No board selected (set Device to a board FQBN, e.g. arduino:avr:uno)"
            .to_string();
        println!("{err}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        logs.push(err);
        r.log = logs;
        r.error = -1;
        return Err(r);
    }
    let (fqbn, dio_msg) = if device.contains(':') {
        resolve_dio_flash_mode_with_cli(device, Some(&cli))
    } else {
        let boards = default_boards();
        let (board, custom, _) = match_board(device, "", device, &boards);
        let resolved = resolve_board_fqbn(&board, &custom, &boards);
        if resolved.is_empty() {
            ("arduino:avr:uno".to_string(), None)
        } else {
            resolve_dio_flash_mode_with_cli(&resolved, Some(&cli))
        }
    };
    if let Some(msg) = dio_msg {
        println!("{msg}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        logs.push(msg);
    }

    let cleaned_file = strip_windows_verbatim(file_path);
    let file_path = fs::canonicalize(&cleaned_file)
        .map(|p| strip_windows_verbatim(&p.to_string_lossy()))
        .unwrap_or(cleaned_file);
    let cli = fs::canonicalize(&cli)
        .map(|p| PathBuf::from(strip_windows_verbatim(&p.to_string_lossy())))
        .unwrap_or(cli);

    let p = Path::new(&file_path);
    let sketch_dir = p.parent().unwrap_or_else(|| Path::new("."));

    // Clean up any legacy in-tree build directory in the sketch folder so arduino-cli
    // does not recursively scan cached build files as sketch sources.
    let legacy_build = sketch_dir.join("build");
    if legacy_build.is_dir() {
        let _ = std::fs::remove_dir_all(&legacy_build);
    }

    let build_dir = resolve_arduino_build_dir(&file_path, &fqbn);
    let _ = std::fs::create_dir_all(&build_dir);
    let build_dir_str = build_dir.to_string_lossy().into_owned();

    // Invalidate stale detector cache files:
    // arduino-cli's internal SketchLibrariesDetector saves include metadata into includes.cache.
    // If files are modified, headers change, or a previous run was interrupted/timed out,
    // detectorCache.expect enters an O(N^2) path-formatting loop (detectorCacheEntry.Equals ->
    // fmt.Sprintf) that hangs arduino-cli at 100% CPU for >60 seconds.
    // Object files (.o) in core/ and libraries/ are tracked separately by timestamp and
    // remain cached, so purging includes.cache and libraries.cache guarantees include discovery
    // never hangs while preserving fast (~1s) incremental compilation.
    let _ = std::fs::remove_file(build_dir.join("includes.cache"));
    let _ = std::fs::remove_file(build_dir.join("libraries.cache"));

    let mut cmd = Command::new(&cli);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdin(std::process::Stdio::null());
    cmd.env("TERM", "dumb");
    cmd.env_remove("DYLD_LIBRARY_PATH");
    cmd.env_remove("DYLD_FALLBACK_LIBRARY_PATH");
    cmd.env_remove("DYLD_FRAMEWORK_PATH");
    cmd.args([
        "compile",
        "--fqbn",
        &fqbn,
        "--build-path",
        &build_dir_str,
        "--discovery-timeout",
        "0s",
        "--no-color",
        &file_path,
    ]);
    if let Some(dir) = Path::new(&file_path).parent() {
        if !dir.as_os_str().is_empty() {
            cmd.current_dir(dir);
        }
    }

    let exec_cmd = format!(
        "Executing:\n{} compile --fqbn {} --build-path {} --discovery-timeout 0s --no-color {}\n",
        cli.display(),
        fqbn,
        build_dir_str,
        file_path,
    );
    println!("{exec_cmd}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    logs.push(exec_cmd);

    Ok((
        StartedArduino {
            cmd,
            file_path,
            fqbn,
            cli,
        },
        logs,
    ))
}

pub fn finish_arduino_compile(
    file_path: &str,
    fqbn: &str,
    success: bool,
    combined: &str,
) -> CompileResult {
    let mut r = CompileResult::default();
    if !combined.is_empty() {
        r.log.push(combined.to_string());
    }

    let p = Path::new(file_path);
    let file_name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("sketch");
    let file_ext = p
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    let (e, es, ws) = parse_error_lines(combined, file_name, &file_ext);
    if e > 0 {
        r.error = e;
    }
    r.errors = es;
    r.warnings = ws;

    if r.error > 0 {
        return r;
    }
    if !success {
        r.error = -1;
        return r;
    }

    let is_esp32 = is_esp32_fqbn(fqbn);
    let sketch_dir = p.parent().unwrap_or_else(|| Path::new("."));
    let build_dir = resolve_arduino_build_dir(file_path, fqbn);
    let expected = expected_build_output(&build_dir, file_name, is_esp32);

    let Some(built) = firmware_in_dir(&build_dir, file_name, is_esp32)
        .or_else(|| find_exported_firmware(sketch_dir, fqbn, file_name, is_esp32))
    else {
        r.log.push(format!(
            "\nError: Build finished without errors but did not produce:\n{}\n",
            expected.display()
        ));
        r.error = -1;
        return r;
    };

    let dest = sketch_firmware_path(p, is_esp32);
    if !install_sketch_firmware(&built, &dest, &build_dir, file_name, is_esp32, &mut r.log) {
        r.error = -1;
        return r;
    }
    r.firmware = dest.to_string_lossy().into_owned();
    r
}

fn is_esp32_fqbn(fqbn: &str) -> bool {
    let lower = fqbn.to_ascii_lowercase();
    lower.starts_with("esp32:") || lower.contains(":esp32:")
}

/// `{sketchDir}/{fileName}.hex` or `.bin`. C++ `InoDebugger::upload`.
fn sketch_firmware_path(sketch: &Path, is_esp32: bool) -> PathBuf {
    let stem = sketch
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sketch");
    let ext = if is_esp32 { "bin" } else { "hex" };
    sketch
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.{ext}"))
}

fn expected_build_output(build_dir: &Path, file_name: &str, is_esp32: bool) -> PathBuf {
    let name = if is_esp32 {
        format!("{file_name}.ino.merged.bin")
    } else {
        format!("{file_name}.ino.hex")
    };
    build_dir.join(name)
}

fn firmware_in_dir(dir: &Path, file_name: &str, is_esp32: bool) -> Option<PathBuf> {
    let preferred: &[&str] = if is_esp32 {
        &[".ino.merged.bin", ".merged.bin", ".ino.bin", ".bin"]
    } else {
        &[".ino.hex", ".hex"]
    };
    for suffix in preferred {
        let p = dir.join(format!("{file_name}{suffix}"));
        if p.is_file() {
            return Some(p);
        }
    }
    find_firmware(dir, is_esp32)
}

fn find_exported_firmware(
    sketch_dir: &Path,
    fqbn: &str,
    file_name: &str,
    is_esp32: bool,
) -> Option<PathBuf> {
    let fqbn_dots = fqbn.replace(':', ".");
    let build_dir = sketch_dir.join("build").join(&fqbn_dots);
    if let Some(p) = firmware_in_dir(&build_dir, file_name, is_esp32) {
        return Some(p);
    }
    let build_root = sketch_dir.join("build");
    let rd = std::fs::read_dir(&build_root).ok()?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(p) = firmware_in_dir(&path, file_name, is_esp32) {
                return Some(p);
            }
        }
    }
    None
}

/// Copy arduino-cli's `{name}.ino.hex` (or `.ino.merged.bin`) next to the
/// sketch as `{name}.hex` (or `{name}.bin`). C++ `InoDebugger::upload`.
fn install_sketch_firmware(
    built: &Path,
    dest: &Path,
    build_dir: &Path,
    file_name: &str,
    _is_esp32: bool,
    log: &mut Vec<String>,
) -> bool {
    if built != dest {
        if dest.exists() {
            if let Err(e) = std::fs::remove_file(dest) {
                log.push(format!(
                    "\nError: Could not remove the old firmware, it is still in use:\n{}\n{e}\n",
                    dest.display()
                ));
                return false;
            }
        }
        if let Err(e) = std::fs::copy(built, dest) {
            log.push(format!(
                "\nError: Could not copy the firmware to:\n{}\n{e}\n",
                dest.display()
            ));
            return false;
        }
    }

    copy_debug_elf(build_dir, dest, file_name, log);
    true
}

/// Copy `{name}.ino.elf` (or `{name}.elf`) next to the sketch firmware so the
/// MCU monitor can list program variables. Arduino-CLI emits this for AVR and
/// ESP32; previously only the ESP32 path was copied.
fn copy_debug_elf(build_dir: &Path, dest: &Path, file_name: &str, log: &mut Vec<String>) {
    let candidates = [
        build_dir.join(format!("{file_name}.ino.elf")),
        build_dir.join(format!("{file_name}.elf")),
    ];
    for elf_src in candidates {
        if !elf_src.is_file() {
            continue;
        }
        let elf_dst = dest.with_extension("elf");
        let _ = std::fs::remove_file(&elf_dst);
        if std::fs::copy(&elf_src, &elf_dst).is_err() {
            log.push(format!(
                "\nWarning: Could not update the debug symbols file:\n{}\n",
                elf_dst.display()
            ));
        }
        return;
    }
}

/// Look for a firmware image in `dir`. Skip `with_bootloader` variants so
/// the flash image used for upload matches C++ `{name}.ino.hex`.
fn find_firmware(dir: &Path, is_esp32: bool) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hex: Option<PathBuf> = None;
    let mut bin: Option<PathBuf> = None;
    let mut elf: Option<PathBuf> = None;
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.contains("with_bootloader") || name.contains("bootloader") {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if ext == "hex" && hex.is_none() {
            hex = Some(path);
        } else if ext == "bin" && bin.is_none() {
            bin = Some(path);
        } else if ext == "elf" && elf.is_none() {
            elf = Some(path);
        }
    }
    if is_esp32 {
        bin.or(hex).or(elf)
    } else {
        hex.or(elf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_arduino_cli_empty_path() {
        let result = find_arduino_cli("");
        if command_exists("arduino-cli") {
            assert!(result.is_some());
        }
    }

    #[test]
    fn detects_macos_ide2_bundle_layout() {
        let dir = std::env::temp_dir().join("cs_arduino_bundle_test");
        let cli_dir = dir.join("Arduino IDE.app/Contents/Resources/app/lib/backend/resources");
        let _ = std::fs::create_dir_all(&cli_dir);
        let _ = std::fs::write(cli_dir.join(cli_name()), "");
        let app = format!("{}/", dir.join("Arduino IDE.app").to_string_lossy());
        assert!(is_arduino_install_dir(&app), "{app}");
        assert!(check_tool_path(&app).is_empty());
        assert!(
            find_arduino_cli(&app).is_some(),
            "cli not found under {app}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn finds_system_arduino_ide_if_installed() {
        let app = PathBuf::from("/Applications/Arduino IDE.app");
        if !app.is_dir() {
            return;
        }
        assert!(
            is_arduino_install_dir("/Applications/Arduino IDE.app/"),
            "installed Arduino IDE.app was not recognized"
        );
        let cands = tool_path_candidates();
        assert!(
            cands.iter().any(|c| c.contains("Arduino IDE.app")),
            "candidates: {cands:?}"
        );
    }

    #[test]
    fn find_arduino_cli_nonexistent_dir() {
        let result = find_arduino_cli("/nonexistent/path/that/does/not/exist");
        assert!(result.is_none());
    }

    #[test]
    fn compile_no_cli() {
        // With a bogus tool_path, arduino-cli won't be found.
        let r = compile_arduino("sketch.ino", "arduino:avr:uno", "/no/such/dir");
        assert_eq!(r.error, -1);
        assert!(r.log.iter().any(|l| l.contains("arduino-cli not found")));
    }

    #[test]
    fn compile_no_board() {
        // Even if we can't find arduino-cli, empty device triggers its own
        // error when tool_path is empty and cli is on PATH. Test with a
        // temp dir containing a fake arduino-cli.
        let dir = std::env::temp_dir().join("cs_arduino_test");
        let _ = std::fs::create_dir_all(&dir);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let fake = dir.join("arduino-cli");
            let _ = std::fs::write(&fake, "#!/bin/sh\nexit 0\n");
            let _ = std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755));
        }
        let r = compile_arduino("sketch.ino", "", dir.to_str().unwrap_or(""));
        // On unix we created a fake cli, so we get the "No board" error.
        // On other platforms cli isn't found, so we get "not found".
        assert_eq!(r.error, -1);
        let combined: String = r.log.join("\n");
        assert!(
            combined.contains("No board selected") || combined.contains("not found"),
            "unexpected log: {combined}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_arduino_install_dir_detects_builder() {
        let dir = std::env::temp_dir().join("cs_arduino_ide_test");
        let _ = std::fs::create_dir_all(&dir);
        let builder = dir.join(builder_name());
        let _ = std::fs::write(&builder, "");
        let path = format!("{}/", dir.to_string_lossy());
        assert!(is_arduino_install_dir(&path));
        assert!(check_tool_path(&path).is_empty());
        assert!(!check_tool_path("").is_empty());
        assert!(!is_arduino_install_dir("/no/such/arduino"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_arduino_error_lines() {
        // Simulate arduino-cli output that includes gcc-style error lines.
        let output = concat!(
            "/tmp/Blink/Blink.ino:10:1: error: expected ';' before '}' token\n",
            "/tmp/Blink/Blink.ino:5:7: warning: unused variable 'x'\n",
            "Compilation error: exit status 1\n",
        );
        let (first_err, errors, warnings) = parse_error_lines(output, "Blink", ".ino");
        assert_eq!(first_err, 10);
        assert_eq!(errors, vec![10]);
        assert_eq!(warnings, vec![5]);
    }

    #[test]
    fn parse_arduino_no_errors() {
        let output = "Sketch uses 924 bytes (2%) of program storage space.\nGlobal variables use 9 bytes (0%) of dynamic memory.\n";
        let (first_err, errors, warnings) = parse_error_lines(output, "Blink", ".ino");
        assert_eq!(first_err, 0);
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_default_boards() {
        let defs = default_boards();
        assert_eq!(defs.len(), 4);
        assert_eq!(defs[0].name, "Uno");
        assert_eq!(defs[0].fqbn, "arduino:avr:uno");
        assert_eq!(defs[1].name, "Mega");
        assert_eq!(defs[1].fqbn, "arduino:avr:megaADK");
        assert_eq!(defs[2].name, "Nano");
        assert_eq!(defs[2].fqbn, "arduino:avr:nano");
        assert_eq!(defs[3].name, "Duemilanove");
        assert_eq!(defs[3].fqbn, "arduino:avr:diecimila");
    }

    #[test]
    fn test_parse_board_list() {
        let sample = concat!(
            "Board Name                                  FQBN\n",
            "Arduino Uno                                 arduino:avr:uno\n",
            "Arduino Nano                                arduino:avr:nano\n",
            "Arduino Mega or Mega 2560                   arduino:avr:mega\n",
            "ESP32 Dev Module                            esp32:esp32:esp32\n",
        );
        let parsed = parse_board_list(sample);
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].name, "Arduino Uno");
        assert_eq!(parsed[0].fqbn, "arduino:avr:uno");
        assert_eq!(parsed[1].name, "Arduino Nano");
        assert_eq!(parsed[1].fqbn, "arduino:avr:nano");
        assert_eq!(parsed[2].name, "Arduino Mega or Mega 2560");
        assert_eq!(parsed[2].fqbn, "arduino:avr:mega");
        assert_eq!(parsed[3].name, "ESP32 Dev Module");
        assert_eq!(parsed[3].fqbn, "esp32:esp32:esp32");
    }

    #[test]
    fn test_board_names_for_ui() {
        let mut boards = default_boards();
        boards.push(BoardEntry {
            name: "ESP32 Dev Module".into(),
            fqbn: "esp32:esp32:esp32".into(),
        });
        let ui_names = board_names_for_ui(&boards);
        assert_eq!(
            ui_names,
            vec![
                "Uno",
                "Mega",
                "Nano",
                "Duemilanove",
                "Custom",
                "ESP32 Dev Module"
            ]
        );
    }

    #[test]
    fn test_match_board_exact_and_case() {
        let boards = default_boards();
        // Exact
        let (b, cb, dev) = match_board("Nano", "", "", &boards);
        assert_eq!(b, "Nano");
        assert_eq!(cb, "");
        assert_eq!(dev, "atmega328p");

        // Case-insensitive
        let (b, cb, dev) = match_board("mega", "", "", &boards);
        assert_eq!(b, "Mega");
        assert_eq!(cb, "");
        assert_eq!(dev, "atmega2560");

        let (b, cb, dev) = match_board("UNO", "", "", &boards);
        assert_eq!(b, "Uno");
        assert_eq!(cb, "");
        assert_eq!(dev, "atmega328p");
    }

    #[test]
    fn test_match_board_fqbn() {
        let mut boards = default_boards();
        boards.push(BoardEntry {
            name: "Arduino Uno".into(),
            fqbn: "arduino:avr:uno".into(),
        });
        boards.push(BoardEntry {
            name: "ESP32 Dev Module".into(),
            fqbn: "esp32:esp32:esp32".into(),
        });

        // FQBN for Uno should match standard "Uno"
        let (b, _, dev) = match_board("arduino:avr:uno", "", "", &boards);
        assert_eq!(b, "Uno");
        assert_eq!(dev, "atmega328p");

        // FQBN for Mega
        let (b, _, dev) = match_board("arduino:avr:megaADK", "", "", &boards);
        assert_eq!(b, "Mega");
        assert_eq!(dev, "atmega2560");

        // FQBN for Nano
        let (b, _, dev) = match_board("arduino:avr:nano", "", "", &boards);
        assert_eq!(b, "Nano");
        assert_eq!(dev, "atmega328p");

        // FQBN for Duemilanove
        let (b, _, dev) = match_board("arduino:avr:diecimila", "", "", &boards);
        assert_eq!(b, "Duemilanove");
        assert_eq!(dev, "atmega328p");

        // FQBN for ESP32 Dev Module
        let (b, _, dev) = match_board("esp32:esp32:esp32", "", "", &boards);
        assert_eq!(b, "ESP32 Dev Module");
        assert_eq!(dev, "esp32");
    }

    #[test]
    fn test_match_board_legacy_device_and_mcu() {
        let boards = default_boards();

        // Empty board but saved_device = "arduino:avr:uno"
        let (b, cb, dev) = match_board("", "", "arduino:avr:uno", &boards);
        assert_eq!(b, "Uno");
        assert_eq!(cb, "");
        assert_eq!(dev, "atmega328p");

        // Saved device MCU "atmega2560" -> Mega
        let (b, _, dev) = match_board("", "", "atmega2560", &boards);
        assert_eq!(b, "Mega");
        assert_eq!(dev, "atmega2560");

        // Saved device MCU "atmega328p" -> Uno
        let (b, _, dev) = match_board("", "", "atmega328p", &boards);
        assert_eq!(b, "Uno");
        assert_eq!(dev, "atmega328p");
    }

    #[test]
    fn test_match_board_custom_and_fallback() {
        let boards = default_boards();

        // Explicitly Custom
        let (b, cb, _) = match_board("Custom", "esp32:esp32:myboard", "", &boards);
        assert_eq!(b, "Custom");
        assert_eq!(cb, "esp32:esp32:myboard");

        // Unrecognized FQBN treated as Custom
        let (b, cb, _) = match_board("custom_vendor:avr:coolboard", "", "", &boards);
        assert_eq!(b, "Custom");
        assert_eq!(cb, "custom_vendor:avr:coolboard");

        // Completely empty -> defaults to Uno
        let (b, cb, dev) = match_board("", "", "", &boards);
        assert_eq!(b, "Uno");
        assert_eq!(cb, "");
        assert_eq!(dev, "atmega328p");
    }

    #[test]
    fn test_force_dio_flash_mode() {
        assert_eq!(
            force_dio_flash_mode("esp32:esp32:esp32"),
            "esp32:esp32:esp32:FlashMode=dio"
        );
        assert_eq!(
            force_dio_flash_mode("esp32:esp32:esp32:UploadSpeed=115200"),
            "esp32:esp32:esp32:UploadSpeed=115200,FlashMode=dio"
        );
        assert_eq!(
            force_dio_flash_mode("esp32:esp32:esp32:FlashMode=qio"),
            "esp32:esp32:esp32:FlashMode=qio"
        );
        assert_eq!(force_dio_flash_mode("arduino:avr:uno"), "arduino:avr:uno");
        assert_eq!(
            force_dio_flash_mode("esp32:esp32:esp32doit-devkit-v1"),
            "esp32:esp32:esp32doit-devkit-v1"
        );
    }

    #[test]
    fn test_parse_board_details_dio() {
        // 1. Board already configured for DIO (e.g. DOIT ESP32 DEVKIT V1)
        let doit_json = serde_json::json!({
            "build_properties": [
                "build.boot=dio",
                "build.boot_freq=80m"
            ],
            "config_options": [
                {
                    "option": "UploadSpeed",
                    "values": [{"value": "115200"}]
                }
            ]
        });
        let (fqbn, log) = parse_board_details_dio(&doit_json, "esp32:esp32:esp32doit-devkit-v1");
        assert_eq!(fqbn, "esp32:esp32:esp32doit-devkit-v1");
        assert!(log.is_none());

        // 2. Board configured for QIO but supports FlashMode=dio menu (e.g. generic ESP32)
        let qio_json = serde_json::json!({
            "build_properties": [
                "build.boot=qio",
                "build.boot_freq=80m"
            ],
            "config_options": [
                {
                    "option": "FlashMode",
                    "values": [
                        {"value": "qio"},
                        {"value": "dio"}
                    ]
                }
            ]
        });
        let (fqbn, log) = parse_board_details_dio(&qio_json, "esp32:esp32:esp32");
        assert_eq!(fqbn, "esp32:esp32:esp32:FlashMode=dio");
        assert!(log.unwrap().contains("Using FlashMode=dio"));

        // 3. QIO board with existing option
        let (fqbn, log) =
            parse_board_details_dio(&qio_json, "esp32:esp32:esp32:UploadSpeed=115200");
        assert_eq!(fqbn, "esp32:esp32:esp32:UploadSpeed=115200,FlashMode=dio");
        assert!(log.unwrap().contains("Using FlashMode=dio"));

        // 4. QIO board that lacks FlashMode option
        let qio_no_menu = serde_json::json!({
            "build_properties": [
                "build.boot=qio"
            ],
            "config_options": []
        });
        let (fqbn, log) = parse_board_details_dio(&qio_no_menu, "esp32:esp32:customqio");
        assert_eq!(fqbn, "esp32:esp32:customqio");
        assert!(log.unwrap().contains("DOIT ESP32 DEVKIT V1"));
    }

    #[test]
    fn test_resolve_dio_flash_mode_for_esp32doit() {
        let cli = find_arduino_cli("");
        let (fqbn, log) =
            resolve_dio_flash_mode_with_cli("esp32:esp32:esp32doit-devkit-v1", cli.as_deref());
        assert_eq!(fqbn, "esp32:esp32:esp32doit-devkit-v1");
        assert!(!fqbn.contains("FlashMode"));
        assert!(log.is_none());
    }

    #[test]
    fn sketch_firmware_path_strips_ino() {
        let ino = Path::new("/tmp/Blink/Blink.ino");
        assert_eq!(
            sketch_firmware_path(ino, false),
            PathBuf::from("/tmp/Blink/Blink.hex")
        );
        assert_eq!(
            sketch_firmware_path(ino, true),
            PathBuf::from("/tmp/Blink/Blink.bin")
        );
    }

    #[test]
    fn install_copies_ino_hex_as_hex() {
        let dir = std::env::temp_dir().join("cs_arduino_fw_copy");
        let _ = std::fs::remove_dir_all(&dir);
        let build = dir.join("build").join("arduino.avr.uno");
        std::fs::create_dir_all(&build).unwrap();
        let built = build.join("Blink.ino.hex");
        std::fs::write(&built, b":00000001FF").unwrap();
        // A with_bootloader sibling must not be preferred over .ino.hex.
        std::fs::write(build.join("Blink.ino.with_bootloader.hex"), b"boot").unwrap();

        let found = find_exported_firmware(&dir, "arduino:avr:uno", "Blink", false).unwrap();
        assert_eq!(found, built);

        let dest = sketch_firmware_path(&dir.join("Blink.ino"), false);
        let mut log = Vec::new();
        assert!(install_sketch_firmware(
            &found, &dest, &build, "Blink", false, &mut log
        ));
        assert_eq!(dest.file_name().unwrap(), "Blink.hex");
        assert_eq!(std::fs::read(&dest).unwrap(), b":00000001FF");
        assert!(log.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_copies_avr_elf_next_to_hex() {
        let dir = std::env::temp_dir().join("cs_arduino_avr_elf_copy");
        let _ = std::fs::remove_dir_all(&dir);
        let build = dir.join("build").join("arduino.avr.uno");
        std::fs::create_dir_all(&build).unwrap();
        let built = build.join("Blink.ino.hex");
        std::fs::write(&built, b":00000001FF").unwrap();
        std::fs::write(build.join("Blink.ino.elf"), b"elf").unwrap();

        let dest = sketch_firmware_path(&dir.join("Blink.ino"), false);
        let mut log = Vec::new();
        assert!(install_sketch_firmware(
            &built, &dest, &build, "Blink", false, &mut log
        ));
        assert_eq!(std::fs::read(dir.join("Blink.elf")).unwrap(), b"elf");
        assert!(log.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_copies_esp32_merged_bin() {
        let dir = std::env::temp_dir().join("cs_arduino_esp32_fw_copy");
        let _ = std::fs::remove_dir_all(&dir);
        let build = dir.join("build").join("esp32.esp32.esp32.FlashMode=dio");
        std::fs::create_dir_all(&build).unwrap();
        let built = build.join("App.ino.merged.bin");
        std::fs::write(&built, b"firmware").unwrap();
        std::fs::write(build.join("App.ino.elf"), b"elf").unwrap();

        let fqbn = "esp32:esp32:esp32:FlashMode=dio";
        let found = find_exported_firmware(&dir, fqbn, "App", true).unwrap();
        assert_eq!(found, built);

        let dest = sketch_firmware_path(&dir.join("App.ino"), true);
        let mut log = Vec::new();
        assert!(install_sketch_firmware(
            &found, &dest, &build, "App", true, &mut log
        ));
        assert_eq!(dest.file_name().unwrap(), "App.bin");
        assert_eq!(std::fs::read(&dest).unwrap(), b"firmware");
        assert_eq!(std::fs::read(dir.join("App.elf")).unwrap(), b"elf");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_esp32_board_meta_by_fqbn_and_name() {
        let sample = concat!(
            "esp32doit-devkit-v1.name=DOIT ESP32 DEVKIT V1\n",
            "esp32doit-devkit-v1.build.tarch=xtensa\n",
            "esp32doit-devkit-v1.build.mcu=esp32\n",
            "esp32doit-devkit-v1.build.variant=doitESP32devkitV1\n",
            "esp32doit-devkit-v1.build.board=ESP32_DEV\n",
            "esp32doit-devkit-v1.build.f_cpu=240000000L\n",
            "esp32c3.name=ESP32C3 Dev Module\n",
            "esp32c3.build.tarch=riscv32\n",
            "esp32c3.build.mcu=esp32c3\n",
            "esp32c3.build.variant=esp32c3\n",
            "esp32c3.build.board=ESP32C3_DEV\n",
            "esp32c3.build.f_cpu=160000000L\n",
        );

        let by_fqbn = parse_esp32_board_meta(sample, "", "esp32:esp32:esp32doit-devkit-v1");
        assert_eq!(by_fqbn.mcu, "esp32");
        assert_eq!(by_fqbn.tarch, "xtensa");
        assert_eq!(by_fqbn.variant, "doitESP32devkitV1");
        assert_eq!(by_fqbn.board_define, "ESP32_DEV");
        assert_eq!(by_fqbn.f_cpu, "240000000L");

        let by_name = parse_esp32_board_meta(sample, "DOIT ESP32 DEVKIT V1", "");
        assert_eq!(by_name, by_fqbn);

        let c3 = parse_esp32_board_meta(sample, "ESP32C3 Dev Module", "");
        assert_eq!(c3.mcu, "esp32c3");
        assert_eq!(c3.tarch, "riscv32");
        assert_eq!(c3.variant, "esp32c3");

        assert_eq!(
            device_for_board_or_fqbn("DOIT ESP32 DEVKIT V1", ""),
            "esp32"
        );
    }

    #[test]
    fn esp32_lsp_zero_false_errors_and_completions() {
        use super::super::compiler::{CompileCtx, CompilerSpec, write_compile_flags};
        use super::super::lsp::{LspClient, LspEvent, find_clangd};
        use std::env;
        use std::thread;
        use std::time::{Duration, Instant};

        let Some(clangd) = find_clangd() else {
            return;
        };
        if find_esp32_package_root().is_none() {
            return;
        }

        let dir = env::temp_dir().join("cs_esp32_lsp_integration_test with spaces");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let ino = dir.join("esp32_uart.ino");
        let src = concat!(
            "#include <Arduino.h>\n",
            "\n",
            "void setup() {\n",
            "  Serial.begin(115200);\n",
            "  delay(1000);\n",
            "  Serial.println(\"ESP32 is awake!\");\n",
            "}\n",
            "\n",
            "void loop() {\n",
            "  Serial.print(\"Hello: \");\n",
            "  Serial.println(millis());\n",
            "  delay(1000);\n",
            "}\n",
        );
        fs::write(&ino, src).unwrap();

        let ino_str = ino.to_string_lossy().into_owned();
        let ctx = CompileCtx {
            file: ino_str.clone(),
            board: "DOIT ESP32 DEVKIT V1".into(),
            device: "esp32".into(),
            fqbn: "esp32:esp32:esp32doit-devkit-v1".into(),
            ..Default::default()
        };
        let spec = CompilerSpec {
            name: "Arduino".into(),
            type_name: "arduino".into(),
            ..Default::default()
        };
        assert!(write_compile_flags(&ctx, &spec));

        let mut lsp = LspClient::default();
        lsp.start(&clangd, &dir.to_string_lossy());
        lsp.open_file(&ino_str, src);

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut got_diags = None;
        while Instant::now() < deadline {
            for ev in lsp.take_events() {
                if let LspEvent::Diagnostics { uri, items } = ev {
                    if uri.ends_with("esp32_uart.ino") {
                        got_diags = Some(items);
                    }
                }
            }
            if got_diags.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        let diags = got_diags.expect("clangd should publish diagnostics for ESP32 sketch");
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == 1).collect();
        assert!(
            errors.is_empty(),
            "expected zero errors for valid ESP32 sketch, got: {errors:?}"
        );

        // Now delete one char (`Serial.println` -> `Serial.printl`) and request completion
        let edited = src.replace("Serial.println(\"ESP32", "Serial.printl(\"ESP32");
        let t0 = Instant::now();
        lsp.change_file(&ino_str, &edited);
        lsp.request_completion(&ino_str, 5, 15);
        assert!(
            t0.elapsed() < Duration::from_millis(200),
            "LSP edit + completion request must not block caller thread"
        );

        let comp_deadline = Instant::now() + Duration::from_secs(10);
        let mut completions = Vec::new();
        while Instant::now() < comp_deadline {
            for ev in lsp.take_events() {
                if let LspEvent::Completions(items) = ev {
                    completions = items;
                }
            }
            if !completions.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(
            completions.iter().any(|c| c.label.starts_with("println")),
            "expected HardwareSerial::println completion, got: {completions:?}"
        );

        lsp.stop();
        let _ = fs::remove_dir_all(&dir);
    }
}
