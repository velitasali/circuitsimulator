//! Component Library installer backend.
//!
//! Downloads component sets from `simulide.com`, manages installed packages,
//! extracts ZIP archives, and updates the active component catalog.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::DeflateDecoder;

pub const DEFAULT_COMPS_URL: &str = "https://simulide.com/p/direct_downloads/components/";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallItem {
    pub name: String,
    pub description: String,
    pub file: String,
    pub version: i64,
    pub version_next: i64,
    pub depends: String,
    pub author: String,
}

impl InstallItem {
    pub fn is_group_header(&self) -> bool {
        self.file.is_empty()
    }

    pub fn installed(&self) -> bool {
        self.version > 0
    }

    pub fn can_update(&self) -> bool {
        self.installed() && self.version_next > self.version
    }

    pub fn to_record_string(&self) -> String {
        format!("{};{}", self.name, self.version)
    }
}

/// Parses a `components.txt` index file into a list of `InstallItem`s.
pub fn parse_components_txt(
    content: &str,
    installed_versions: &BTreeMap<String, i64>,
) -> Vec<InstallItem> {
    let mut items = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split(';').map(str::trim).collect();
        let name = parts.first().copied().unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }

        if parts.len() < 4 {
            // Group header
            items.push(InstallItem {
                name,
                description: String::new(),
                file: String::new(),
                version: 0,
                version_next: 0,
                depends: String::new(),
                author: String::new(),
            });
            continue;
        }

        let description = parts.get(1).copied().unwrap_or("").to_string();
        let file = parts.get(2).copied().unwrap_or("").to_string();
        let v_str = parts.get(3).copied().unwrap_or("");
        let v_clean = v_str.trim_start_matches(['v', 'V']);
        let version_next = v_clean.parse::<i64>().unwrap_or(0);
        let depends = parts.get(4).copied().unwrap_or("").to_string();
        let author = parts.get(5).copied().unwrap_or("").to_string();

        let version = installed_versions.get(&name).copied().unwrap_or(0);

        items.push(InstallItem {
            name,
            description,
            file,
            version,
            version_next,
            depends,
            author,
        });
    }

    items
}

/// Downloads a URL to a string using curl or a simple HTTP GET.
pub fn download_url_string(url: &str) -> Result<String, String> {
    let bytes = download_url_bytes(url)?;
    String::from_utf8(bytes).map_err(|e| format!("UTF-8 decode error: {e}"))
}

/// Downloads a URL to a byte buffer using `curl`.
pub fn download_url_bytes(url: &str) -> Result<Vec<u8>, String> {
    let output = std::process::Command::new("curl")
        .arg("-sSL")
        .arg("--max-time")
        .arg("30")
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to execute curl: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Download failed with exit code: {:?}",
            output.status.code()
        ));
    }

    Ok(output.stdout)
}

/// Extracts a ZIP archive buffer into the destination directory with Zip Slip protection.
pub fn extract_zip(zip_bytes: &[u8], dest_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if zip_bytes.len() < 22 {
        return Err("Buffer too small for ZIP file".into());
    }

    // Locate End of Central Directory Record (EOCD)
    let eocd_sig = [0x50, 0x4B, 0x05, 0x06];
    let mut eocd_pos = None;
    let min_pos = zip_bytes.len().saturating_sub(65557);
    for i in (min_pos..=zip_bytes.len() - 22).rev() {
        if zip_bytes[i..i + 4] == eocd_sig {
            eocd_pos = Some(i);
            break;
        }
    }

    let eocd_offset = eocd_pos.ok_or_else(|| "EOCD signature not found in ZIP".to_string())?;

    let num_entries =
        u16::from_le_bytes([zip_bytes[eocd_offset + 10], zip_bytes[eocd_offset + 11]]) as usize;

    let cd_offset = u32::from_le_bytes([
        zip_bytes[eocd_offset + 16],
        zip_bytes[eocd_offset + 17],
        zip_bytes[eocd_offset + 18],
        zip_bytes[eocd_offset + 19],
    ]) as usize;

    let mut extracted_paths = Vec::new();
    let mut curr = cd_offset;

    for _ in 0..num_entries {
        if curr + 46 > zip_bytes.len() {
            return Err("Unexpected end of Central Directory".into());
        }

        if zip_bytes[curr..curr + 4] != [0x50, 0x4B, 0x01, 0x02] {
            return Err("Invalid Central Directory Header signature".into());
        }

        let compression = u16::from_le_bytes([zip_bytes[curr + 10], zip_bytes[curr + 11]]);
        let comp_size = u32::from_le_bytes([
            zip_bytes[curr + 20],
            zip_bytes[curr + 21],
            zip_bytes[curr + 22],
            zip_bytes[curr + 23],
        ]) as usize;
        let uncomp_size = u32::from_le_bytes([
            zip_bytes[curr + 24],
            zip_bytes[curr + 25],
            zip_bytes[curr + 26],
            zip_bytes[curr + 27],
        ]) as usize;
        let fn_len = u16::from_le_bytes([zip_bytes[curr + 28], zip_bytes[curr + 29]]) as usize;
        let extra_len = u16::from_le_bytes([zip_bytes[curr + 30], zip_bytes[curr + 31]]) as usize;
        let comment_len = u16::from_le_bytes([zip_bytes[curr + 32], zip_bytes[curr + 33]]) as usize;
        let local_header_offset = u32::from_le_bytes([
            zip_bytes[curr + 42],
            zip_bytes[curr + 43],
            zip_bytes[curr + 44],
            zip_bytes[curr + 45],
        ]) as usize;

        let name_start = curr + 46;
        let name_end = name_start + fn_len;
        if name_end > zip_bytes.len() {
            return Err("Invalid filename in Central Directory".into());
        }

        let rel_path_str =
            String::from_utf8_lossy(&zip_bytes[name_start..name_end]).replace('\\', "/");
        curr = name_end + extra_len + comment_len;

        // Sanitize path against directory traversal
        let sanitized = Path::new(&rel_path_str);
        if sanitized.is_absolute()
            || sanitized
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            continue; // Skip dangerous entries
        }

        let target_path = dest_dir.join(sanitized);

        if rel_path_str.ends_with('/') {
            // Directory entry
            fs::create_dir_all(&target_path)
                .map_err(|e| format!("Failed to create dir {}: {e}", target_path.display()))?;
            continue;
        }

        // Read Local Header to find data offset
        if local_header_offset + 30 > zip_bytes.len() {
            return Err("Local header offset out of bounds".into());
        }
        if zip_bytes[local_header_offset..local_header_offset + 4] != [0x50, 0x4B, 0x03, 0x04] {
            return Err("Invalid Local Header signature".into());
        }

        let local_fn_len = u16::from_le_bytes([
            zip_bytes[local_header_offset + 26],
            zip_bytes[local_header_offset + 27],
        ]) as usize;
        let local_extra_len = u16::from_le_bytes([
            zip_bytes[local_header_offset + 28],
            zip_bytes[local_header_offset + 29],
        ]) as usize;

        let data_start = local_header_offset + 30 + local_fn_len + local_extra_len;
        let data_end = data_start + comp_size;
        if data_end > zip_bytes.len() {
            return Err("File data out of bounds".into());
        }

        let raw_data = &zip_bytes[data_start..data_end];

        let decompressed = match compression {
            0 => raw_data.to_vec(),
            8 => {
                let mut decoder = DeflateDecoder::new(raw_data);
                let mut out = Vec::with_capacity(uncomp_size);
                decoder
                    .read_to_end(&mut out)
                    .map_err(|e| format!("Deflate decompression error: {e}"))?;
                out
            }
            other => return Err(format!("Unsupported ZIP compression method: {other}")),
        };

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir {}: {e}", parent.display()))?;
        }

        let mut f = File::create(&target_path)
            .map_err(|e| format!("Failed to create file {}: {e}", target_path.display()))?;
        f.write_all(&decompressed)
            .map_err(|e| format!("Failed to write file {}: {e}", target_path.display()))?;

        extracted_paths.push(target_path);
    }

    Ok(extracted_paths)
}

/// Core Installer Manager for handling component downloads and updates.
#[derive(Clone, Debug)]
pub struct InstallerManager {
    comps_dir: PathBuf,
    comps_url: String,
    items: Vec<InstallItem>,
}

impl Default for InstallerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallerManager {
    pub fn new() -> Self {
        let comps_dir = crate::catalog::components_dir();
        let comps_url = DEFAULT_COMPS_URL.to_string();
        let mut mgr = Self {
            comps_dir,
            comps_url,
            items: Vec::new(),
        };
        mgr.load_local_list();
        mgr
    }

    pub fn comps_dir(&self) -> &Path {
        &self.comps_dir
    }

    pub fn items(&self) -> &[InstallItem] {
        &self.items
    }

    pub fn item(&self, name: &str) -> Option<&InstallItem> {
        self.items.iter().find(|i| i.name == name)
    }

    /// Loads the list from the local `components.txt` if present.
    pub fn load_local_list(&mut self) {
        let comp_file = self.comps_dir.join("components.txt");
        let content = fs::read_to_string(&comp_file).unwrap_or_default();
        let settings = crate::settings::get();
        self.items = parse_components_txt(&content, &settings.installed_components);
    }

    /// Fetches the latest `components.txt` from the remote server, saves it, and parses it.
    pub fn check_for_updates(&mut self) -> Result<bool, String> {
        let url = format!("{}dloadset.php?file=components.txt", self.comps_url);
        let content = download_url_string(&url)?;

        let comp_file = self.comps_dir.join("components.txt");
        if let Some(parent) = comp_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&comp_file, &content);

        let settings = crate::settings::get();
        let new_items = parse_components_txt(&content, &settings.installed_components);

        let has_updates = new_items.iter().any(|i| i.can_update());
        self.items = new_items;

        Ok(has_updates)
    }

    /// Installs a component set by name. Automatically handles prerequisite dependencies.
    pub fn install_set(&mut self, name: &str) -> Result<(), String> {
        let item = self
            .item(name)
            .cloned()
            .ok_or_else(|| format!("Component set '{name}' not found in catalog"))?;

        if item.is_group_header() {
            return Ok(());
        }

        // Check if item has a dependency that needs to be installed first
        if !item.depends.is_empty() {
            let settings = crate::settings::get();
            if !settings.installed_components.contains_key(&item.depends) {
                if let Some(dep_item) = self.item(&item.depends).cloned() {
                    if !dep_item.is_group_header() {
                        self.download_and_extract_item(&dep_item)?;
                    }
                }
            }
        }

        self.download_and_extract_item(&item)?;
        Ok(())
    }

    fn download_and_extract_item(&mut self, item: &InstallItem) -> Result<(), String> {
        let url = format!("{}dloadset.php?file={}", self.comps_url, item.file);
        let zip_bytes = download_url_bytes(&url)?;

        extract_zip(&zip_bytes, &self.comps_dir)?;

        // Update installed versions in settings
        let mut settings = crate::settings::get();
        let target_ver = if item.version_next > 0 {
            item.version_next
        } else {
            item.version
        };
        settings
            .installed_components
            .insert(item.name.clone(), target_ver);
        crate::settings::replace(settings);

        // Update local item state
        if let Some(it) = self.items.iter_mut().find(|i| i.name == item.name) {
            it.version = target_ver;
        }

        // Invalidate and reload the component catalog
        crate::catalog::reload_standard();

        Ok(())
    }

    /// Uninstalls a component set by removing its directory and settings record.
    pub fn uninstall_set(&mut self, name: &str) -> Result<(), String> {
        let target_folder = self.comps_dir.join(name);
        if target_folder.exists() {
            let _ = fs::remove_dir_all(&target_folder);
        }

        let mut settings = crate::settings::get();
        settings.installed_components.remove(name);
        crate::settings::replace(settings);

        if let Some(it) = self.items.iter_mut().find(|i| i.name == name) {
            it.version = 0;
        }

        crate::catalog::reload_standard();
        Ok(())
    }

    /// Returns the list of component / subcircuit item names found in the installed folder.
    pub fn get_group_items(&self, group_name: &str) -> Vec<String> {
        let group_dir = self.comps_dir.join(group_name);
        if !group_dir.exists() {
            return Vec::new();
        }

        let mut catalog = crate::catalog::Catalog::new();
        catalog.load_dir(&group_dir);
        let mut names: Vec<String> = catalog.items().iter().map(|it| it.name.clone()).collect();
        names.sort();
        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_components_txt() {
        let src = r#"
MCUs and CPUs:
Arduino; Arduino boards.; Arduino.zip; 2507102250; AVR; Santiago
AVR; avr microcontrollers.; AVR.zip; v2507102250;; Atmel
PIC; 12/14 bit microcontrollers.; PIC.zip; 2512301110
MCS51; 8051 MCU; MCS51.zip; 2507102250
Analog:
Analog; Analog components.; Analog.zip; 2507102250
"#;
        let mut installed = BTreeMap::new();
        installed.insert("AVR".to_string(), 2507102250);
        installed.insert("PIC".to_string(), 2500000000); // Older version installed -> can update

        let items = parse_components_txt(src, &installed);
        assert_eq!(items.len(), 7);

        // Header 1
        assert!(items[0].is_group_header());
        assert_eq!(items[0].name, "MCUs and CPUs:");

        // Arduino
        assert_eq!(items[1].name, "Arduino");
        assert_eq!(items[1].description, "Arduino boards.");
        assert_eq!(items[1].file, "Arduino.zip");
        assert_eq!(items[1].version_next, 2507102250);
        assert_eq!(items[1].depends, "AVR");
        assert_eq!(items[1].author, "Santiago");
        assert!(!items[1].installed());

        // AVR
        assert_eq!(items[2].name, "AVR");
        assert_eq!(items[2].version, 2507102250);
        assert_eq!(items[2].version_next, 2507102250);
        assert!(items[2].installed());
        assert!(!items[2].can_update());

        // PIC
        assert_eq!(items[3].name, "PIC");
        assert_eq!(items[3].version, 2500000000);
        assert_eq!(items[3].version_next, 2512301110);
        assert!(items[3].installed());
        assert!(items[3].can_update());

        // Header 2
        assert!(items[5].is_group_header());
        assert_eq!(items[5].name, "Analog:");
    }

    #[test]
    fn test_zip_extraction_stored() {
        // Construct a simple in-memory ZIP with 1 stored file
        let mut zip_data = Vec::new();

        let filename = b"test/hello.txt";
        let content = b"Hello, Circuit Simulator!";

        // Local header
        let local_header_offset = zip_data.len() as u32;
        zip_data.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // Signature
        zip_data.extend_from_slice(&[20, 0]); // Version
        zip_data.extend_from_slice(&[0, 0]); // Flags
        zip_data.extend_from_slice(&[0, 0]); // Compression: stored
        zip_data.extend_from_slice(&[0, 0, 0, 0]); // Time/Date
        zip_data.extend_from_slice(&[0, 0, 0, 0]); // CRC32 (ignored in test)
        zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes()); // Comp size
        zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes()); // Uncomp size
        zip_data.extend_from_slice(&(filename.len() as u16).to_le_bytes()); // Filename len
        zip_data.extend_from_slice(&[0, 0]); // Extra len
        zip_data.extend_from_slice(filename);
        zip_data.extend_from_slice(content);

        // Central Directory Header
        let cd_offset = zip_data.len() as u32;
        zip_data.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // Signature
        zip_data.extend_from_slice(&[20, 0]); // Version made by
        zip_data.extend_from_slice(&[20, 0]); // Version needed
        zip_data.extend_from_slice(&[0, 0]); // Flags
        zip_data.extend_from_slice(&[0, 0]); // Compression: stored
        zip_data.extend_from_slice(&[0, 0, 0, 0]); // Time/Date
        zip_data.extend_from_slice(&[0, 0, 0, 0]); // CRC32
        zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes()); // Comp size
        zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes()); // Uncomp size
        zip_data.extend_from_slice(&(filename.len() as u16).to_le_bytes()); // Filename len
        zip_data.extend_from_slice(&[0, 0]); // Extra len
        zip_data.extend_from_slice(&[0, 0]); // Comment len
        zip_data.extend_from_slice(&[0, 0]); // Disk start
        zip_data.extend_from_slice(&[0, 0]); // Internal attr
        zip_data.extend_from_slice(&[0, 0, 0, 0]); // External attr
        zip_data.extend_from_slice(&local_header_offset.to_le_bytes()); // Local header offset
        zip_data.extend_from_slice(filename);

        let cd_size = (zip_data.len() as u32) - cd_offset;

        // EOCD
        zip_data.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // Signature
        zip_data.extend_from_slice(&[0, 0]); // Disk number
        zip_data.extend_from_slice(&[0, 0]); // CD disk number
        zip_data.extend_from_slice(&[1, 0]); // Num entries on disk
        zip_data.extend_from_slice(&[1, 0]); // Total entries
        zip_data.extend_from_slice(&cd_size.to_le_bytes()); // CD size
        zip_data.extend_from_slice(&cd_offset.to_le_bytes()); // CD offset
        zip_data.extend_from_slice(&[0, 0]); // Comment len

        let tmp_dir = std::env::temp_dir().join("cs_test_zip_extract");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        let extracted = extract_zip(&zip_data, &tmp_dir).unwrap();
        assert_eq!(extracted.len(), 1);

        let target_file = tmp_dir.join("test/hello.txt");
        assert!(target_file.exists());
        let read_content = fs::read_to_string(&target_file).unwrap();
        assert_eq!(read_content, "Hello, Circuit Simulator!");

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
