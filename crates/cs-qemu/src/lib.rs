//! Thin host for the patched `qemu-system-*` cosim binaries.
//!
//! The wire format is `qemuArena_t` in `src/microsim/cores/qemu/qemudevice.h`
//! (mirrored byte-for-byte by `Esp32CsArena` in the qemu-cosim patch). This
//! crate owns the shared-memory arena (POSIX `shm_open` / Windows
//! `CreateFileMappingA`) and the child process (spawn / SIGSTOP / SIGCONT /
//! SIGKILL). ESP32/STM32 modules stay in a later MCU phase.

mod arena;
mod process;
#[cfg(any(unix, windows))]
mod shm;

pub use arena::{ARENA_SIZE, Arena, SimAction};
pub use process::QemuProcess;
#[cfg(any(unix, windows))]
pub use shm::SharedArena;

/// POSIX `shm_open` name: `"/" + pid + id`.
///
/// macOS caps the name at `PSHMNAMLEN` (31) including the leading `'/'` and
/// the NUL terminator, so the usable payload is 29 bytes. Linux is looser.
pub fn shm_key(pid: u32, id: &str) -> String {
    let mut key = format!("/{pid}{id}");
    // 30 bytes of text + NUL = 31.
    const MAX: usize = 30;
    if key.len() > MAX {
        key.truncate(MAX);
    }
    key
}

/// Look up `qemu-system-xtensa` the same way `Esp32` does: env, then
/// `data/bin` next to the executable / cwd.
pub fn qemu_xtensa_path() -> Option<std::path::PathBuf> {
    find_qemu("qemu-system-xtensa")
}

/// Look up `qemu-system-arm` the same way `Stm32` does.
pub fn qemu_arm_path() -> Option<std::path::PathBuf> {
    find_qemu("qemu-system-arm")
}

fn qemu_names(name: &str) -> Vec<String> {
    #[allow(unused_mut)]
    let mut names = vec![name.to_string()];
    #[cfg(windows)]
    if !name.ends_with(".exe") && !name.ends_with(".EXE") {
        names.push(format!("{name}.exe"));
    }
    names
}

/// Directories that may contain `qemu-system-*` or ESP32 ROM/efuse data.
///
/// Cargo runs package tests with CWD at `crates/<pkg>`, so CWD-relative
/// `resources/qemu-cosim/build/out` misses the in-tree binary. Walk from CWD,
/// the executable, and this crate's manifest dir up to the repo root.
pub fn qemu_search_dirs() -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("CS_QEMU_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(data) = dirs::data_dir() {
        let root = data.join("Circuit Simulator");
        dirs.push(root.join("data/bin"));
        dirs.push(root.join("data"));
        dirs.push(root);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("data/bin"));
            dirs.push(dir.join("../Resources/data/bin"));
            dirs.push(dir.join("../data/bin"));
        }
    }
    let mut seeds = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        seeds.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            seeds.push(dir.to_path_buf());
        }
    }
    seeds.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let mut seen = std::collections::HashSet::new();
    for seed in seeds {
        let mut cur = Some(seed);
        while let Some(dir) = cur {
            if !seen.insert(dir.clone()) {
                break;
            }
            dirs.push(dir.join("data/bin"));
            dirs.push(dir.join("resources/data/bin"));
            dirs.push(dir.join("resources/qemu-cosim/build/out"));
            dirs.push(dir.join("resources/qemu-cosim/build/out/qemu-bundle/qemu/share"));
            dirs.push(dir.join("resources/qemu-cosim/build/out/pc-bios"));
            cur = dir.parent().map(|p| p.to_path_buf());
        }
    }
    dirs
}

fn find_qemu(name: &str) -> Option<std::path::PathBuf> {
    let names = qemu_names(name);
    for dir in qemu_search_dirs() {
        for n in &names {
            let p = dir.join(n);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[derive(Debug)]
pub enum Error {
    Shm(String),
    Process(String),
    Io(std::io::Error),
    Unsupported(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Shm(s) | Error::Process(s) => write!(f, "{s}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Unsupported(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_matches_c_layout() {
        assert_eq!(ARENA_SIZE, 88);
        assert_eq!(std::mem::size_of::<Arena>(), 88);
        assert_eq!(std::mem::align_of::<Arena>(), 8);
        // simuAction is the doorbell and must stay the 7th u64 (offset 48).
        assert_eq!(std::mem::offset_of!(Arena, simu_action), 48);
        assert_eq!(std::mem::offset_of!(Arena, running), 64);
        assert_eq!(SimAction::Event as u64, 1 << 7);
        assert_eq!(SimAction::I2c as u64, 10);
    }

    #[test]
    fn shm_key_has_leading_slash_and_fits_macos() {
        let k = shm_key(12345, "Esp32-1");
        assert!(k.starts_with('/'));
        assert!(k.contains("12345"));
        assert!(k.len() <= 30, "{k}");
        let long = shm_key(1, &"x".repeat(80));
        assert_eq!(long.len(), 30);
        assert!(long.starts_with("/1"));
    }

    #[cfg(windows)]
    #[test]
    fn qemu_names_adds_exe() {
        assert_eq!(
            qemu_names("qemu-system-xtensa"),
            ["qemu-system-xtensa", "qemu-system-xtensa.exe"]
        );
        assert_eq!(
            qemu_names("qemu-system-xtensa.exe"),
            ["qemu-system-xtensa.exe"]
        );
    }

    #[test]
    fn search_dirs_include_in_tree_cosim_out() {
        let dirs = qemu_search_dirs();
        assert!(
            dirs.iter()
                .any(|d| d.ends_with("resources/qemu-cosim/build/out")
                    || d.ends_with("resources\\qemu-cosim\\build\\out")),
            "expected ancestor walk to include qemu-cosim/build/out, got {dirs:?}"
        );
    }
}
