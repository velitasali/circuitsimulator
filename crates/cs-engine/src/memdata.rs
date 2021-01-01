//! C++ `MemData` load/save of byte memories (EEPROM, RAM, ROM).

use std::path::Path;

/// Load `.data` (comma-separated decimals), Intel HEX, or raw `.bin` into `dest`.
/// Does not resize `dest` (C++ `resize=false`).
pub fn load_bytes(path: &Path, dest: &mut [u8]) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "data" {
        load_dat(path, dest)
    } else if ext == "hex" || ext == "ihx" {
        load_hex_bytes(path, dest)
    } else {
        load_bin(path, dest)
    }
}

/// Save as `.data` (comma-separated decimals, 16 per line) or raw binary.
pub fn save_bytes(path: &Path, data: &[u8]) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "data" {
        save_dat(path, data)
    } else {
        std::fs::write(path, data).map_err(|e| format!("Cannot write file {}: {e}", path.display()))
    }
}

fn load_dat(path: &Path, dest: &mut [u8]) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut addr = 0usize;
    for line in src.lines() {
        let line = line.replace('\t', "").replace(' ', "");
        if line.is_empty() {
            continue;
        }
        for sdata in line.split(',') {
            if sdata.is_empty() {
                continue;
            }
            let Ok(val) = sdata.parse::<i32>() else {
                continue;
            };
            if addr >= dest.len() {
                crate::logging::log_sim(format!(
                    "\nMemData::loadDat: Data doesn't fit in Memory {}\n",
                    dest.len().saturating_sub(1)
                ));
                return Ok(());
            }
            dest[addr] = val as u8;
            addr += 1;
        }
    }
    Ok(())
}

fn load_hex_bytes(path: &Path, dest: &mut [u8]) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut words = vec![0u16; dest.len()];
    cs_mcu::load_hex(&src, &mut words, false, 8).map_err(|e| e.to_string())?;
    for (i, w) in words.iter().enumerate() {
        if i < dest.len() {
            dest[i] = *w as u8;
        }
    }
    Ok(())
}

fn load_bin(path: &Path, dest: &mut [u8]) -> Result<(), String> {
    let ba = std::fs::read(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let n = ba.len().min(dest.len());
    dest[..n].copy_from_slice(&ba[..n]);
    if ba.len() > dest.len() {
        crate::logging::log_sim(format!(
            "\nMemData::loadBin: Data doesn't fit in Memory {}\n",
            dest.len().saturating_sub(1)
        ));
    }
    Ok(())
}

fn save_dat(path: &Path, data: &[u8]) -> Result<(), String> {
    let mut output = String::new();
    for (i, val) in data.iter().enumerate() {
        output.push_str(&format!("{val:4}"));
        if (i + 1) % 16 == 0 {
            output.push('\n');
        } else if i + 1 < data.len() {
            output.push(',');
        }
    }
    std::fs::write(path, output).map_err(|e| format!("Cannot write file {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("cs-memdata-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.data");
        let data = vec![1u8, 2, 3, 10, 255];
        save_bytes(&path, &data).unwrap();
        let mut dest = vec![0u8; 8];
        load_bytes(&path, &mut dest).unwrap();
        assert_eq!(&dest[..5], &data[..]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bin_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("cs-memdata-bin-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.bin");
        let data = vec![0xAAu8, 0xBB, 0xCC];
        save_bytes(&path, &data).unwrap();
        let mut dest = vec![0u8; 4];
        load_bytes(&path, &mut dest).unwrap();
        assert_eq!(&dest[..3], &data[..]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
