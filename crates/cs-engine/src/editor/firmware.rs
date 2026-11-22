//! Firmware project scaffolding for MCUs, Arduino boards, and QEMU devices.

use std::fs;
use std::path::{Path, PathBuf};

use cs_mcu::CoreKind;

use super::cfg::{self, FileConfig};

const ARDUINO_TEMPLATE: &str = "\
void setup() {
    // Put your setup code here, to run once:
}

void loop() {
    // Put your main code here, to run repeatedly:
}
";

const AVR_C_TEMPLATE: &str = "\
#include <avr/io.h>

int main(void) {
    while (1) {
    }
    return 0;
}
";

const GENERIC_C_TEMPLATE: &str = "\
int main(void) {
    while (1) {
    }
    return 0;
}
";

const ASCRIPT_TEMPLATE: &str = "\
void setup() {
}

void loop() {
}
";

/// Compiler and starter-file settings derived from a target MCU / board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePreset {
    pub mcu_label: String,
    pub mcu_device: String,
    pub compiler: String,
    pub board: String,
    pub custom_board: String,
    pub device: String,
    pub extension: String,
    /// Expected compiled binary extension next to the source file (`"bin"` for
    /// ESP32, `"hex"` for Arduino AVR), or empty when the source file itself
    /// should be linked on the MCU until first upload.
    pub output_extension: String,
    pub template: String,
}

/// Files created on disk by [`create_firmware_scaffold`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedFirmware {
    pub dir: PathBuf,
    pub source_path: PathBuf,
    pub cfg_path: PathBuf,
    pub linked_firmware_path: PathBuf,
}

fn normalize_avr_gcc_device(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.starts_with("mega") || lower.starts_with("tiny") {
        format!("at{lower}")
    } else {
        lower
    }
}

fn arduino_preset(
    mcu_label: String,
    mcu_device: String,
    board: &str,
    custom_board: &str,
    device: &str,
    output_ext: &str,
) -> FirmwarePreset {
    FirmwarePreset {
        mcu_label,
        mcu_device,
        compiler: "Arduino".into(),
        board: board.into(),
        custom_board: custom_board.into(),
        device: device.into(),
        extension: "ino".into(),
        output_extension: output_ext.into(),
        template: ARDUINO_TEMPLATE.into(),
    }
}

/// Choose the default compiler, board/device, extension, and template for an MCU.
pub fn preset_for_target(
    mcu_label: &str,
    device_name: &str,
    board_hint: Option<&str>,
    core: Option<CoreKind>,
) -> FirmwarePreset {
    let label = mcu_label.trim().to_string();
    let dev_display = if !device_name.trim().is_empty() {
        device_name.trim().to_string()
    } else if let Some(hint) = board_hint.filter(|h| !h.trim().is_empty()) {
        hint.trim().to_string()
    } else {
        label.clone()
    };
    let dev_lower = device_name.trim().to_ascii_lowercase();
    let hint_lower = board_hint.unwrap_or("").trim().to_ascii_lowercase();

    // 1. ESP32 (QEMU device or board containing ESP32)
    // QEMU requires DIO flash mode; DOIT ESP32 DEVKIT V1 builds DIO natively.
    if dev_lower.contains("esp32") || hint_lower.contains("esp32") {
        return arduino_preset(
            label,
            dev_display,
            "DOIT ESP32 DEVKIT V1",
            "esp32:esp32:esp32doit-devkit-v1",
            "esp32",
            "bin",
        );
    }

    // 2. STM32 QEMU device
    if dev_lower.contains("stm32") || hint_lower.contains("stm32") {
        return FirmwarePreset {
            mcu_label: label,
            mcu_device: dev_display,
            compiler: "None".into(),
            board: String::new(),
            custom_board: String::new(),
            device: device_name.trim().to_string(),
            extension: "c".into(),
            output_extension: String::new(),
            template: GENERIC_C_TEMPLATE.into(),
        };
    }

    // 3. Arduino board subcircuits (Uno, Mega, Nano, Duemilanove, Leonardo)
    if hint_lower.contains("mega") {
        return arduino_preset(label, dev_display, "Mega", "", "atmega2560", "hex");
    }
    if hint_lower.contains("nano") {
        return arduino_preset(label, dev_display, "Nano", "", "atmega328p", "hex");
    }
    if hint_lower.contains("duemilanove") || hint_lower.contains("diecimila") {
        return arduino_preset(label, dev_display, "Duemilanove", "", "atmega328p", "hex");
    }
    if hint_lower.contains("leonardo") {
        return arduino_preset(
            label,
            dev_display,
            "Arduino Leonardo",
            "arduino:avr:leonardo",
            "atmega32u4",
            "hex",
        );
    }
    if hint_lower == "uno" || hint_lower.contains("arduino uno") || hint_lower.contains("uno") {
        return arduino_preset(label, dev_display, "Uno", "", "atmega328p", "hex");
    }

    // 4. Standalone AVR MCUs
    let is_avr = core == Some(CoreKind::Avr)
        || dev_lower.starts_with("mega")
        || dev_lower.starts_with("atmega")
        || dev_lower.starts_with("tiny")
        || dev_lower.starts_with("attiny");
    if is_avr {
        if dev_lower.contains("328") {
            return arduino_preset(label, dev_display, "Uno", "", "atmega328p", "hex");
        }
        if dev_lower.contains("2560") {
            return arduino_preset(label, dev_display, "Mega", "", "atmega2560", "hex");
        }
        if dev_lower.contains("32u4") {
            return arduino_preset(
                label,
                dev_display,
                "Arduino Leonardo",
                "arduino:avr:leonardo",
                "atmega32u4",
                "hex",
            );
        }
        let avr_dev = normalize_avr_gcc_device(device_name);
        return FirmwarePreset {
            mcu_label: label,
            mcu_device: dev_display,
            compiler: "Avrgcc".into(),
            board: String::new(),
            custom_board: String::new(),
            device: avr_dev,
            extension: "c".into(),
            output_extension: String::new(),
            template: AVR_C_TEMPLATE.into(),
        };
    }

    // 5. Scripted MCU core
    if core == Some(CoreKind::Scripted) {
        return FirmwarePreset {
            mcu_label: label,
            mcu_device: dev_display,
            compiler: "AScript".into(),
            board: String::new(),
            custom_board: String::new(),
            device: device_name.trim().to_string(),
            extension: "as".into(),
            output_extension: String::new(),
            template: ASCRIPT_TEMPLATE.into(),
        };
    }

    // 6. PIC / 8051 / 6502 / Z80 / other MCUs
    let asm_header = if !dev_display.is_empty() {
        format!("; Firmware for {dev_display}\n")
    } else {
        "; Firmware\n".into()
    };
    FirmwarePreset {
        mcu_label: label,
        mcu_device: dev_display,
        compiler: "None".into(),
        board: String::new(),
        custom_board: String::new(),
        device: device_name.trim().to_string(),
        extension: "asm".into(),
        output_extension: String::new(),
        template: asm_header,
    }
}

/// Produce a clean default sketch/firmware name derived from an MCU label/device.
pub fn suggest_firmware_name(parent_dir: &Path, preset: &FirmwarePreset) -> String {
    let raw = if !preset.mcu_device.is_empty() {
        &preset.mcu_device
    } else if !preset.mcu_label.is_empty() {
        &preset.mcu_label
    } else {
        "firmware"
    };
    let mut stem = String::new();
    let mut prev_underscore = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            stem.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if !stem.is_empty() && !prev_underscore {
            stem.push('_');
            prev_underscore = true;
        }
    }
    let stem = stem.trim_end_matches('_');
    let base = if stem.is_empty() {
        "firmware".to_string()
    } else if stem.ends_with("_firmware") || stem == "firmware" {
        stem.to_string()
    } else {
        format!("{stem}_firmware")
    };

    if !parent_dir.as_os_str().is_empty() {
        let first = parent_dir
            .join(&base)
            .join(format!("{base}.{}", preset.extension));
        if !first.exists() {
            return base;
        }
        for i in 2..=99 {
            let cand = format!("{base}_{i}");
            let path = parent_dir
                .join(&cand)
                .join(format!("{cand}.{}", preset.extension));
            if !path.exists() {
                return cand;
            }
        }
    }
    base
}

/// Validate and normalize a user-entered firmware name for the given extension.
pub fn validate_firmware_name(raw_name: &str, extension: &str) -> Result<String, String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err("Enter a firmware name.".into());
    }
    let suffix = format!(".{extension}");
    let stem = if trimmed.len() > suffix.len()
        && trimmed[trimmed.len() - suffix.len()..].eq_ignore_ascii_case(&suffix)
    {
        &trimmed[..trimmed.len() - suffix.len()]
    } else {
        trimmed
    };
    if stem.is_empty() || stem == "." || stem == ".." {
        return Err("Invalid firmware name.".into());
    }
    if stem.chars().any(|c| {
        matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
    }) {
        return Err("Firmware name contains invalid path characters.".into());
    }
    if extension.eq_ignore_ascii_case("ino") {
        let first = stem.chars().next().unwrap_or('_');
        if !first.is_ascii_alphanumeric() && first != '_' {
            return Err(
                "Arduino sketch names must start with a letter, digit, or underscore.".into(),
            );
        }
        if !stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            return Err(
                "Arduino sketch names may only contain letters, digits, '_', '-', and '.'.".into(),
            );
        }
    }
    Ok(stem.to_string())
}

/// Compute `(sketch_dir, source_path, linked_firmware_path)` for a validated name.
pub fn scaffold_paths(
    parent_dir: &Path,
    stem: &str,
    preset: &FirmwarePreset,
) -> (PathBuf, PathBuf, PathBuf) {
    let dir = parent_dir.join(stem);
    let source = dir.join(format!("{stem}.{}", preset.extension));
    let linked = if !preset.output_extension.is_empty() {
        dir.join(format!("{stem}.{}", preset.output_extension))
    } else {
        source.clone()
    };
    (dir, source, linked)
}

/// Create `<parent_dir>/<name>/<name>.<ext>` and its `.cfg` compiler sidecar.
pub fn create_firmware_scaffold(
    parent_dir: &Path,
    raw_name: &str,
    preset: &FirmwarePreset,
) -> Result<CreatedFirmware, String> {
    if parent_dir.as_os_str().is_empty() {
        return Err("Choose a directory to save the firmware.".into());
    }
    let stem = validate_firmware_name(raw_name, &preset.extension)?;
    let (dir, source_path, linked_firmware_path) = scaffold_paths(parent_dir, &stem, preset);

    if source_path.exists() {
        return Err(format!(
            "Firmware file already exists: {}",
            source_path.display()
        ));
    }

    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {e}"))?;
    fs::write(&source_path, &preset.template)
        .map_err(|e| format!("Failed to write firmware source: {e}"))?;

    let src_str = source_path.to_string_lossy().into_owned();
    let cfg_path = PathBuf::from(format!("{src_str}.cfg"));
    let cfg = FileConfig {
        compiler: preset.compiler.clone(),
        load_compiler: !preset.compiler.is_empty() && preset.compiler != "None",
        board: preset.board.clone(),
        custom_board: preset.custom_board.clone(),
        device: preset.device.clone(),
        ..FileConfig::default()
    };
    cfg::save_cfg(&src_str, &cfg).map_err(|e| format!("Failed to write firmware settings: {e}"))?;

    Ok(CreatedFirmware {
        dir,
        source_path,
        cfg_path,
        linked_firmware_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::arduino::{BoardEntry, default_boards, match_board, resolve_board_fqbn};
    use crate::editor::cfg::load_cfg;
    use std::env;

    #[test]
    fn esp32_preset_uses_doit_devkit_v1_and_matches_before_and_after_discovery() {
        let preset = preset_for_target("Esp32-1", "ESP32", None, None);
        assert_eq!(preset.compiler, "Arduino");
        assert_eq!(preset.board, "DOIT ESP32 DEVKIT V1");
        assert_eq!(preset.custom_board, "esp32:esp32:esp32doit-devkit-v1");
        assert_eq!(preset.device, "esp32");
        assert_eq!(preset.extension, "ino");
        assert_eq!(preset.output_extension, "bin");

        // Before arduino-cli board listall completes, falls back to Custom with exact FQBN.
        let defaults = default_boards();
        let (b0, cb0, dev0) = match_board(
            &preset.board,
            &preset.custom_board,
            &preset.device,
            &defaults,
        );
        assert_eq!(b0, "Custom");
        assert_eq!(cb0, "esp32:esp32:esp32doit-devkit-v1");
        assert_eq!(dev0, "esp32");
        assert_eq!(
            resolve_board_fqbn(&b0, &cb0, &defaults),
            "esp32:esp32:esp32doit-devkit-v1"
        );

        // Once arduino-cli discovers the ESP32 core, snaps to the named board entry.
        let mut with_esp = defaults;
        with_esp.push(BoardEntry {
            name: "DOIT ESP32 DEVKIT V1".into(),
            fqbn: "esp32:esp32:esp32doit-devkit-v1".into(),
        });
        let (b1, cb1, dev1) = match_board(
            &preset.board,
            &preset.custom_board,
            &preset.device,
            &with_esp,
        );
        assert_eq!(b1, "DOIT ESP32 DEVKIT V1");
        assert_eq!(cb1, "");
        assert_eq!(dev1, "esp32");
        assert_eq!(
            resolve_board_fqbn(&b1, &cb1, &with_esp),
            "esp32:esp32:esp32doit-devkit-v1"
        );
    }

    #[test]
    fn arduino_board_and_avr_presets() {
        let uno = preset_for_target("Uno-1", "mega328", Some("Uno"), Some(CoreKind::Avr));
        assert_eq!(uno.compiler, "Arduino");
        assert_eq!(uno.board, "Uno");
        assert_eq!(uno.device, "atmega328p");
        assert_eq!(uno.extension, "ino");
        assert_eq!(uno.output_extension, "hex");

        let mega = preset_for_target("Mega-1", "mega2560", Some("Mega"), Some(CoreKind::Avr));
        assert_eq!(mega.board, "Mega");
        assert_eq!(mega.device, "atmega2560");

        let nano = preset_for_target("Nano-1", "mega328", Some("Nano"), Some(CoreKind::Avr));
        assert_eq!(nano.board, "Nano");

        let tiny = preset_for_target("tiny85-1", "tiny85", None, Some(CoreKind::Avr));
        assert_eq!(tiny.compiler, "Avrgcc");
        assert_eq!(tiny.device, "attiny85");
        assert_eq!(tiny.extension, "c");
        assert_eq!(tiny.output_extension, "");

        let pic = preset_for_target("pic16f877-1", "pic16f877", None, Some(CoreKind::Pic14));
        assert_eq!(pic.compiler, "None");
        assert_eq!(pic.extension, "asm");
    }

    #[test]
    fn validate_name_rules() {
        assert_eq!(validate_firmware_name("blink", "ino").unwrap(), "blink");
        assert_eq!(validate_firmware_name("blink.ino", "ino").unwrap(), "blink");
        assert_eq!(
            validate_firmware_name("my_sketch-1", "ino").unwrap(),
            "my_sketch-1"
        );
        assert!(validate_firmware_name("", "ino").is_err());
        assert!(validate_firmware_name("my sketch", "ino").is_err());
        assert!(validate_firmware_name("../evil", "ino").is_err());
        assert!(validate_firmware_name("-bad", "ino").is_err());
    }

    #[test]
    fn create_scaffold_writes_dir_source_cfg_and_blocks_overwrite() {
        let root = env::temp_dir().join("cs_test_create_firmware_scaffold");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let preset = preset_for_target("Esp32-1", "ESP32", None, None);
        let created = create_firmware_scaffold(&root, "blink", &preset).unwrap();

        assert_eq!(created.dir, root.join("blink"));
        assert_eq!(created.source_path, root.join("blink").join("blink.ino"));
        assert_eq!(
            created.linked_firmware_path,
            root.join("blink").join("blink.bin")
        );
        assert!(created.source_path.is_file());
        assert!(created.cfg_path.is_file());

        let src = fs::read_to_string(&created.source_path).unwrap();
        assert!(src.contains("void setup()"));
        assert!(src.contains("void loop()"));

        let cfg = load_cfg(&created.source_path.to_string_lossy());
        assert_eq!(cfg.compiler, "Arduino");
        assert!(cfg.load_compiler);
        assert_eq!(cfg.board, "DOIT ESP32 DEVKIT V1");
        assert_eq!(cfg.custom_board, "esp32:esp32:esp32doit-devkit-v1");
        assert_eq!(cfg.device, "esp32");

        // Second call must refuse to overwrite.
        let dup = create_firmware_scaffold(&root, "blink", &preset);
        assert!(dup.is_err());

        let _ = fs::remove_dir_all(&root);
    }
}
