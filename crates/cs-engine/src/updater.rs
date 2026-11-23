//! GitHub Release update checker, build-kind detector, and asset downloader.
//!
//! Queries the GitHub REST API (`/repos/velitasali/CircuitSimulator/releases/latest`),
//! compares semantic versions, detects the active installation format
//! (Windows Inno Setup installer vs. portable archive, Linux AppImage vs. Debian
//! package, macOS disk image), and downloads the matching release asset.

use std::cmp::Ordering;
use std::env;
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::i18n;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// GitHub repository slug (`owner/repo`).
pub const GITHUB_REPO: &str = "velitasali/CircuitSimulator";

/// GitHub REST API endpoint for the latest published release.
pub const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/velitasali/CircuitSimulator/releases/latest";

/// Public HTML page listing all GitHub releases.
pub const GITHUB_RELEASES_PAGE: &str = "https://github.com/velitasali/CircuitSimulator/releases";

/// Compiled application version from workspace `Cargo.toml`.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the active application version for update checks, honoring the
/// `CS_FAKE_VERSION` environment variable when set (e.g. `CS_FAKE_VERSION=0.1.4`).
pub fn current_version() -> String {
    if let Ok(fake) = env::var("CS_FAKE_VERSION") {
        let trimmed = clean_version(&fake);
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    CURRENT_VERSION.to_string()
}

/// Recognized distribution / packaging formats produced by `.github/workflows/release.yml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildKind {
    WindowsInstaller,
    WindowsPortable,
    LinuxAppImage,
    LinuxDeb,
    MacOsDmg,
}

impl BuildKind {
    /// Stable identifier string exposed to QML.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WindowsInstaller => "windows-installer",
            Self::WindowsPortable => "windows-portable",
            Self::LinuxAppImage => "linux-appimage",
            Self::LinuxDeb => "linux-deb",
            Self::MacOsDmg => "macos-dmg",
        }
    }

    /// Localized human-readable package description.
    pub fn display_label(self) -> String {
        match self {
            Self::WindowsInstaller => i18n::tr("Windows Installer (.exe)"),
            Self::WindowsPortable => i18n::tr("Windows Portable (.zip)"),
            Self::LinuxAppImage => i18n::tr("Linux AppImage (.AppImage)"),
            Self::LinuxDeb => i18n::tr("Debian / Ubuntu Package (.deb)"),
            Self::MacOsDmg => i18n::tr("macOS Disk Image (.dmg)"),
        }
    }

    /// Target operating system identifier (`"windows"`, `"linux"`, or `"macos"`).
    pub const fn target_os(self) -> &'static str {
        match self {
            Self::WindowsInstaller | Self::WindowsPortable => "windows",
            Self::LinuxAppImage | Self::LinuxDeb => "linux",
            Self::MacOsDmg => "macos",
        }
    }

    /// Whether this package is an executable installer that can be launched directly to upgrade.
    pub const fn is_direct_installer(self) -> bool {
        matches!(self, Self::WindowsInstaller)
    }
}

/// Metadata for a single downloadable asset attached to a GitHub release.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub kind: Option<BuildKind>,
}

impl ReleaseAsset {
    /// Human-readable label combining the build type (if known), filename, and size.
    pub fn display_label(&self) -> String {
        let size_str = format_bytes(self.size);
        match self.kind {
            Some(kind) => {
                if size_str.is_empty() {
                    format!("{} — {}", kind.display_label(), self.name)
                } else {
                    format!("{} — {} ({size_str})", kind.display_label(), self.name)
                }
            }
            None => {
                if size_str.is_empty() {
                    self.name.clone()
                } else {
                    format!("{} ({size_str})", self.name)
                }
            }
        }
    }
}

/// Parsed information about a GitHub release.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub name: String,
    pub body: String,
    pub html_url: String,
    pub published_at: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseInfo {
    /// Returns the release assets applicable to `os` (`"windows"`, `"linux"`, or `"macos"`),
    /// ordered so that the asset matching `preferred_kind` comes first.
    ///
    /// If no assets match `os` specifically, returns all downloadable binary assets.
    pub fn assets_for_os(&self, os: &str, preferred_kind: BuildKind) -> Vec<ReleaseAsset> {
        let mut matching: Vec<ReleaseAsset> = self
            .assets
            .iter()
            .filter(|a| a.kind.is_some_and(|k| k.target_os() == os))
            .cloned()
            .collect();

        if matching.is_empty() {
            matching = self.assets.clone();
        }

        matching.sort_by_key(|a| {
            if a.kind == Some(preferred_kind) {
                0usize
            } else {
                1usize
            }
        });

        matching
    }
}

/// Detects the current running build format on this machine.
pub fn detect_build_kind() -> BuildKind {
    let exe_path = env::current_exe().ok();
    let appimage_env = env::var("APPIMAGE").ok();
    detect_build_kind_with(
        env::consts::OS,
        exe_path.as_deref(),
        appimage_env.as_deref(),
    )
}

/// Pure build-kind detector for a given OS, executable path, and `APPIMAGE` environment value.
pub fn detect_build_kind_with(
    os: &str,
    exe_path: Option<&Path>,
    appimage_env: Option<&str>,
) -> BuildKind {
    match os {
        "windows" => {
            if let Some(exe) = exe_path
                && let Some(dir) = exe.parent()
                && has_inno_setup_uninstaller(dir)
            {
                return BuildKind::WindowsInstaller;
            }
            BuildKind::WindowsPortable
        }
        "linux" => {
            if appimage_env.is_some_and(|val| !val.trim().is_empty()) {
                return BuildKind::LinuxAppImage;
            }
            if let Some(exe) = exe_path {
                let s = exe.to_string_lossy();
                if s.starts_with("/opt/circuitsimulator") || s.starts_with("/usr/") {
                    return BuildKind::LinuxDeb;
                }
            }
            if Path::new("/var/lib/dpkg/info/circuitsimulator.list").is_file() {
                return BuildKind::LinuxDeb;
            }
            BuildKind::LinuxAppImage
        }
        "macos" => BuildKind::MacOsDmg,
        _ => BuildKind::LinuxAppImage,
    }
}

fn has_inno_setup_uninstaller(dir: &Path) -> bool {
    if dir.join("unins000.exe").is_file() || dir.join("unins000.dat").is_file() {
        return true;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.starts_with("unins") && (name.ends_with(".exe") || name.ends_with(".dat")) {
            return true;
        }
    }
    false
}

/// Classifies a release asset filename into its [`BuildKind`], returning `None` for
/// non-package files (such as `SHA256SUMS.txt` or `.cer` certificates).
pub fn classify_asset(name: &str) -> Option<BuildKind> {
    let lower = name.trim().to_ascii_lowercase();
    if is_ignored_auxiliary_asset(&lower) {
        return None;
    }
    if lower.ends_with(".dmg") {
        return Some(BuildKind::MacOsDmg);
    }
    if lower.ends_with(".appimage") {
        return Some(BuildKind::LinuxAppImage);
    }
    if lower.ends_with(".deb") {
        return Some(BuildKind::LinuxDeb);
    }
    if lower.ends_with(".exe") || lower.ends_with(".msi") {
        return Some(BuildKind::WindowsInstaller);
    }
    if lower.ends_with(".zip") && (lower.contains("windows") || lower.contains("win")) {
        return Some(BuildKind::WindowsPortable);
    }
    None
}

fn is_ignored_auxiliary_asset(lower_name: &str) -> bool {
    lower_name == "sha256sums.txt"
        || lower_name.ends_with(".sha256")
        || lower_name.ends_with(".txt")
        || lower_name.ends_with(".cer")
        || lower_name.ends_with(".crt")
        || lower_name.ends_with(".sig")
        || lower_name.ends_with(".asc")
}

/// Strips an optional leading `v`/`V` prefix from a tag or version string.
pub fn clean_version(raw: &str) -> &str {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed)
}

/// Compares two semantic version strings (e.g. `"0.1.5"` vs `"0.1.6"` or `"0.2.0-beta.1"`).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (a_nums, a_pre) = parse_semver_parts(a);
    let (b_nums, b_pre) = parse_semver_parts(b);

    let max_len = a_nums.len().max(b_nums.len());
    for i in 0..max_len {
        let av = a_nums.get(i).copied().unwrap_or(0);
        let bv = b_nums.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            Ordering::Equal => continue,
            non_eq => return non_eq,
        }
    }

    match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        // A normal release is newer than a pre-release of the same version.
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(ap), Some(bp)) => compare_prerelease(ap, bp),
    }
}

/// Returns `true` when `candidate` is strictly newer than `current`.
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    compare_versions(candidate, current) == Ordering::Greater
}

fn parse_semver_parts(raw: &str) -> (Vec<u64>, Option<&str>) {
    let cleaned = clean_version(raw);
    // Strip build metadata (+...)
    let without_build = cleaned.split('+').next().unwrap_or(cleaned);
    let (core, pre) = match without_build.split_once('-') {
        Some((c, p)) => (c, (!p.is_empty()).then_some(p)),
        None => (without_build, None),
    };
    let nums: Vec<u64> = core
        .split('.')
        .map(|part| part.trim().parse::<u64>().unwrap_or(0))
        .collect();
    (nums, pre)
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    let len = a_parts.len().min(b_parts.len());
    for i in 0..len {
        let ap = a_parts[i];
        let bp = b_parts[i];
        let ord = match (ap.parse::<u64>(), bp.parse::<u64>()) {
            (Ok(an), Ok(bn)) => an.cmp(&bn),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => ap.cmp(bp),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

/// Parses a GitHub Release JSON payload (`/releases/latest`) into [`ReleaseInfo`].
pub fn parse_github_release(json_str: &str) -> Result<ReleaseInfo, String> {
    let v: Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid GitHub JSON: {e}"))?;

    if let Some(msg) = v.get("message").and_then(Value::as_str)
        && v.get("tag_name").is_none()
    {
        return Err(format!("GitHub API error: {msg}"));
    }

    let tag_name = v
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if tag_name.is_empty() {
        return Err("Release payload is missing tag_name".into());
    }

    let version = clean_version(&tag_name).to_string();
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&tag_name)
        .trim()
        .to_string();
    let body = v
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .replace("\r\n", "\n")
        .trim()
        .to_string();
    let html_url = v
        .get("html_url")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(GITHUB_RELEASES_PAGE)
        .to_string();
    let published_at = v
        .get("published_at")
        .and_then(Value::as_str)
        .unwrap_or("")
        .split('T')
        .next()
        .unwrap_or("")
        .to_string();
    let prerelease = v
        .get("prerelease")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut assets = Vec::new();
    if let Some(arr) = v.get("assets").and_then(Value::as_array) {
        for item in arr {
            let Some(asset_name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(download_url) = item.get("browser_download_url").and_then(Value::as_str)
            else {
                continue;
            };
            if is_ignored_auxiliary_asset(&asset_name.to_ascii_lowercase()) {
                continue;
            }
            let size = item.get("size").and_then(Value::as_u64).unwrap_or(0);
            let kind = classify_asset(asset_name);
            assets.push(ReleaseAsset {
                name: asset_name.to_string(),
                download_url: download_url.to_string(),
                size,
                kind,
            });
        }
    }

    Ok(ReleaseInfo {
        tag_name,
        version,
        name,
        body,
        html_url,
        published_at,
        prerelease,
        assets,
    })
}

/// Fetches the latest GitHub release information from the GitHub REST API.
pub fn fetch_latest_release() -> Result<ReleaseInfo, String> {
    let user_agent = format!("User-Agent: CircuitSimulator/{CURRENT_VERSION}");
    let mut cmd = Command::new("curl");
    cmd.arg("-sSL")
        .arg("--max-time")
        .arg("20")
        .arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg("-H")
        .arg(&user_agent)
        .arg(GITHUB_LATEST_RELEASE_API);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute curl: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!(
                "Network request failed (exit code {:?})",
                output.status.code()
            ));
        }
        return Err(stderr);
    }

    let body = String::from_utf8(output.stdout).map_err(|e| format!("UTF-8 decode error: {e}"))?;
    parse_github_release(&body)
}

/// Returns the directory where downloaded updates should be saved
/// (prefers the user's `Downloads` directory, falling back to the system temp directory).
pub fn default_download_dir() -> PathBuf {
    if let Some(dir) = dirs::download_dir()
        && (dir.is_dir() || fs::create_dir_all(&dir).is_ok())
    {
        return dir;
    }
    let fallback = env::temp_dir().join("CircuitSimulator-Updates");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// Downloads a release asset URL to `dest_dir/<file_name>`.
/// Writes to a `.part` file first and atomically renames on completion.
pub fn download_release_asset(
    download_url: &str,
    file_name: &str,
    dest_dir: &Path,
) -> Result<PathBuf, String> {
    let safe_name = Path::new(file_name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| "Invalid asset filename".to_string())?;

    fs::create_dir_all(dest_dir).map_err(|e| {
        format!(
            "Failed to create download directory {}: {e}",
            dest_dir.display()
        )
    })?;

    let final_path = dest_dir.join(safe_name);
    let part_path = dest_dir.join(format!("{safe_name}.part"));
    let _ = fs::remove_file(&part_path);

    let user_agent = format!("User-Agent: CircuitSimulator/{CURRENT_VERSION}");
    let mut cmd = Command::new("curl");
    cmd.arg("-fSL")
        .arg("--max-time")
        .arg("600")
        .arg("-H")
        .arg(&user_agent)
        .arg("-o")
        .arg(&part_path)
        .arg(download_url);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute curl: {e}"))?;

    if !output.status.success() {
        let _ = fs::remove_file(&part_path);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!(
                "Download failed with exit code {:?}",
                output.status.code()
            ));
        }
        return Err(stderr);
    }

    if !part_path.is_file() {
        return Err("Downloaded file was not created".into());
    }

    if final_path.exists() {
        let _ = fs::remove_file(&final_path);
    }
    fs::rename(&part_path, &final_path)
        .map_err(|e| format!("Failed to finalize {}: {e}", final_path.display()))?;

    #[cfg(unix)]
    if safe_name.to_ascii_lowercase().ends_with(".appimage") {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&final_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&final_path, perms);
        }
    }

    Ok(final_path)
}

/// Formats a byte count into a human-readable string (e.g. `"24.5 MB"`).
pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return String::new();
    }
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SAMPLE_RELEASE_JSON: &str = r###"{
        "tag_name": "0.1.6",
        "name": "Circuit Simulator 0.1.6",
        "body": "## What's Changed\r\n- Added GitHub update checker\r\n- Improved schematic rendering",
        "html_url": "https://github.com/velitasali/CircuitSimulator/releases/tag/0.1.6",
        "published_at": "2026-11-23T10:00:00Z",
        "prerelease": false,
        "assets": [
            {
                "name": "CircuitSimulator-x86_64.AppImage",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/CircuitSimulator-x86_64.AppImage",
                "size": 45088768
            },
            {
                "name": "circuitsimulator_0.1.6_amd64.deb",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/circuitsimulator_0.1.6_amd64.deb",
                "size": 31457280
            },
            {
                "name": "CircuitSimulator-setup.exe",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/CircuitSimulator-setup.exe",
                "size": 26214400
            },
            {
                "name": "CircuitSimulator-windows-x86_64.zip",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/CircuitSimulator-windows-x86_64.zip",
                "size": 36700160
            },
            {
                "name": "CircuitSimulator-Public.cer",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/CircuitSimulator-Public.cer",
                "size": 1024
            },
            {
                "name": "CircuitSimulator-macos.dmg",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/CircuitSimulator-macos.dmg",
                "size": 39845888
            },
            {
                "name": "SHA256SUMS.txt",
                "browser_download_url": "https://github.com/velitasali/CircuitSimulator/releases/download/0.1.6/SHA256SUMS.txt",
                "size": 512
            }
        ]
    }"###;

    #[test]
    fn semver_comparison_handles_prereleases() {
        assert!(is_newer_version("0.1.5", "0.1.6"));
        assert!(is_newer_version("0.1.5", "0.2.0"));
        assert!(is_newer_version("0.1.5", "1.0.0"));
        assert!(!is_newer_version("0.1.5", "0.1.5"));
        assert!(!is_newer_version("0.1.5", "0.1.4"));
        assert!(!is_newer_version("0.1.5", "0.1.5-rc.1"));
        assert!(is_newer_version("0.1.5-rc.1", "0.1.5"));
        assert!(is_newer_version("0.1.5-beta.1", "0.1.5-beta.2"));
        assert!(is_newer_version("0.1.5", "0.1.6-alpha.1"));
    }

    #[test]
    fn parse_release_filters_auxiliary_files_and_ranks_preferred_build() {
        let info = parse_github_release(SAMPLE_RELEASE_JSON).expect("parse release");
        assert_eq!(info.version, "0.1.6");
        assert_eq!(info.published_at, "2026-11-23");
        assert!(!info.prerelease);
        // SHA256SUMS.txt and CircuitSimulator-Public.cer must be excluded.
        assert_eq!(info.assets.len(), 5);

        let win_installer_first = info.assets_for_os("windows", BuildKind::WindowsInstaller);
        assert_eq!(win_installer_first.len(), 2);
        assert_eq!(win_installer_first[0].name, "CircuitSimulator-setup.exe");
        assert_eq!(
            win_installer_first[1].name,
            "CircuitSimulator-windows-x86_64.zip"
        );

        let win_portable_first = info.assets_for_os("windows", BuildKind::WindowsPortable);
        assert_eq!(win_portable_first.len(), 2);
        assert_eq!(
            win_portable_first[0].name,
            "CircuitSimulator-windows-x86_64.zip"
        );
        assert_eq!(win_portable_first[1].name, "CircuitSimulator-setup.exe");

        let linux_deb_first = info.assets_for_os("linux", BuildKind::LinuxDeb);
        assert_eq!(linux_deb_first.len(), 2);
        assert_eq!(linux_deb_first[0].name, "circuitsimulator_0.1.6_amd64.deb");
        assert_eq!(linux_deb_first[1].name, "CircuitSimulator-x86_64.AppImage");

        let macos_assets = info.assets_for_os("macos", BuildKind::MacOsDmg);
        assert_eq!(macos_assets.len(), 1);
        assert_eq!(macos_assets[0].name, "CircuitSimulator-macos.dmg");
    }

    #[test]
    fn build_kind_detection_across_platforms() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp = env::temp_dir().join(format!("cs_updater_test_{nanos}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let fake_exe = tmp.join("circuitsimulator.exe");
        fs::write(&fake_exe, b"").unwrap();

        // Without unins000.exe -> WindowsPortable
        assert_eq!(
            detect_build_kind_with("windows", Some(&fake_exe), None),
            BuildKind::WindowsPortable
        );

        // With unins000.exe -> WindowsInstaller
        fs::write(tmp.join("unins000.exe"), b"").unwrap();
        assert_eq!(
            detect_build_kind_with("windows", Some(&fake_exe), None),
            BuildKind::WindowsInstaller
        );

        let _ = fs::remove_dir_all(&tmp);

        // Linux with APPIMAGE env -> LinuxAppImage
        assert_eq!(
            detect_build_kind_with(
                "linux",
                Some(Path::new("/tmp/.mount_CS/usr/bin/circuitsimulator")),
                Some("/home/user/CircuitSimulator-x86_64.AppImage")
            ),
            BuildKind::LinuxAppImage
        );

        // Linux under /opt/circuitsimulator without APPIMAGE -> LinuxDeb
        assert_eq!(
            detect_build_kind_with(
                "linux",
                Some(Path::new("/opt/circuitsimulator/usr/bin/circuitsimulator")),
                None
            ),
            BuildKind::LinuxDeb
        );

        // macOS -> MacOsDmg
        assert_eq!(
            detect_build_kind_with(
                "macos",
                Some(Path::new(
                    "/Applications/Circuit Simulator.app/Contents/MacOS/circuitsimulator"
                )),
                None
            ),
            BuildKind::MacOsDmg
        );
    }

    #[test]
    fn format_bytes_human_readable() {
        assert_eq!(format_bytes(0), "");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2 KB");
        assert_eq!(format_bytes(26_214_400), "25.0 MB");
    }
}
