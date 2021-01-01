//! Circuit persistence (SIM1 load/save), firmware uploads, data files, and subcircuits.

use crate::canvas::drag::Drag;
use crate::canvas::events::Change;
use crate::canvas::export;
use crate::canvas::scene::{Part, Scene};
use crate::canvas::{Canvas, Point};
use crate::components::ShapeKind;

#[derive(Clone, Debug)]
pub struct MemoryItemInfo {
    pub id: String,
    pub title: String,
    pub is_rom: bool,
    pub cell_bytes: usize,
    pub word_bytes: usize,
    pub data: Vec<u8>,
}

impl Canvas {
    /// Load a HEX into the first or targeted MCU (scene + live circuit). C++ `BaseDebugger::upload`.
    pub fn upload_firmware(&mut self, path: &str) -> Change {
        self.upload_firmware_to(path, None)
    }

    pub fn upload_firmware_to(&mut self, path: &str, target_id: Option<&str>) -> Change {
        let p = std::path::Path::new(path);
        let mut ok = self.scene.upload_firmware_to(p, target_id);
        if let Some(c) = self.running.as_mut() {
            if let Some(target) = target_id {
                for (id, m) in c.mcus_mut() {
                    if id == target || target.strip_suffix(id).is_some_and(|p| p.ends_with('/')) {
                        if m.load_firmware_file(p).is_ok() {
                            m.firmware = Some(path.to_string());
                            ok = true;
                        }
                    }
                }
            } else if let Some((_, m)) = c.first_mcu_mut() {
                if m.load_firmware_file(p).is_ok() {
                    m.firmware = Some(path.to_string());
                    ok = true;
                }
            }
        }
        if ok {
            crate::logging::log_sim(format!("Firmware uploaded to MCU:\n{path}"));
            self.publish_mcu_snap();
            Change {
                items: true,
                sim: true,
                ..Change::default()
            }
        } else {
            crate::logging::log_sim(format!("ERROR: Failed uploading firmware: {path}"));
            Change::default()
        }
    }

    /// Resolve the firmware HEX path for an MCU or QemuDevice by component UID.
    pub fn firmware_path_for(&self, uid: &str) -> Option<std::path::PathBuf> {
        let circuit_dir = self
            .file_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).parent());

        let mut target_id = uid;
        let first_id;
        if target_id.is_empty() {
            if let Some(it) = self
                .scene
                .items()
                .iter()
                .find(|it| matches!(it.kind, Part::Mcu(_) | Part::QemuDevice(_)))
            {
                first_id = it.id.clone();
                target_id = &first_id;
            }
        }
        let resolved = self.resolve_item_uid(target_id);
        let target_id = resolved.as_str();

        if let Some(it) = self.scene.item_by_id(target_id) {
            match &it.kind {
                Part::Mcu(mcu) => {
                    return mcu.mcu.firmware.as_ref().and_then(|prog| {
                        let dir = mcu.mcu.firmware_dir.as_deref().or(circuit_dir);
                        crate::mcu::resolve_firmware(dir, prog)
                    });
                }
                Part::QemuDevice(qemu) => {
                    if qemu.qemu.firmware.is_empty() {
                        return None;
                    }
                    let dir = qemu.qemu.firmware_dir.as_deref().or(circuit_dir);
                    return crate::mcu::resolve_firmware(dir, &qemu.qemu.firmware).or_else(|| {
                        let p = std::path::PathBuf::from(&qemu.qemu.firmware);
                        if p.exists() { Some(p) } else { None }
                    });
                }
                _ => {}
            }
        }

        for c in self.scene.hidden() {
            if c.id == target_id
                || target_id
                    .strip_suffix(&c.id)
                    .is_some_and(|p| p.ends_with('/'))
            {
                match &c.kind {
                    crate::elements::Kind::Mcu(mcu) => {
                        let prog = mcu.firmware.as_deref()?;
                        let dir = mcu.firmware_dir.as_deref().or(circuit_dir);
                        return crate::mcu::resolve_firmware(dir, prog);
                    }
                    crate::elements::Kind::QemuDevice(qemu) => {
                        if qemu.firmware.is_empty() {
                            return None;
                        }
                        let dir = qemu.firmware_dir.as_deref().or(circuit_dir);
                        return crate::mcu::resolve_firmware(dir, &qemu.firmware);
                    }
                    _ => {}
                }
            }
        }

        if let Some(c) = self.running.as_ref() {
            for (id, m) in c.mcus() {
                if id == target_id || target_id.strip_suffix(id).is_some_and(|p| p.ends_with('/')) {
                    let prog = m.firmware.as_deref()?;
                    let dir = m.firmware_dir.as_deref().or(circuit_dir);
                    return crate::mcu::resolve_firmware(dir, prog);
                }
            }
        }

        None
    }

    /// Look up the firmware source file (.ino, .cpp, etc.) for an MCU or QemuDevice by component UID.
    pub fn firmware_source_for(&self, uid: &str) -> Option<std::path::PathBuf> {
        let firm_path = self.firmware_path_for(uid)?;
        crate::mcu::firmware_source(&firm_path)
    }

    pub(crate) fn relative_to_circuit(&self, abs: &str) -> String {
        let abs_path = std::path::Path::new(abs);
        if let Some(cp) = self.file_path.as_deref() {
            if let Some(parent) = std::path::Path::new(cp).parent() {
                if let Ok(rel) = abs_path.strip_prefix(parent) {
                    return rel.to_string_lossy().replace('\\', "/");
                }
            }
        }
        abs.replace('\\', "/")
    }

    fn resolve_item_uid<'a>(&'a self, uid: &'a str) -> String {
        if !uid.is_empty() {
            return uid.to_string();
        }
        self.scene
            .items()
            .iter()
            .find(|it| it.selected)
            .map(|it| it.id.clone())
            .unwrap_or_default()
    }

    /// C++ `Mcu::slotLoad` / `QemuDevice::slotLoad`.
    pub fn load_firmware_path(&mut self, uid: &str, path: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        if uid.is_empty() || path.is_empty() {
            return Change::default();
        }
        if self.sim_running {
            let _ = self.power_off();
        }
        self.push_undo();
        let rel = self.relative_to_circuit(path);
        let firm_dir = self
            .file_path
            .as_deref()
            .and_then(|cp| std::path::Path::new(cp).parent().map(|p| p.to_path_buf()));
        let mut c = self.upload_firmware_to(path, Some(&uid));
        if !c.items {
            self.history.undo.pop();
            return c;
        }
        if let Some(it) = self.scene.item_by_id_mut(&uid) {
            match &mut it.kind {
                Part::Mcu(mcu) => {
                    mcu.mcu.firmware = Some(rel);
                    mcu.mcu.firmware_dir = firm_dir;
                }
                Part::QemuDevice(qemu) => {
                    qemu.qemu.firmware = rel;
                    qemu.qemu.firmware_dir = firm_dir;
                }
                _ => {}
            }
        }
        c.history = true;
        c.props = true;
        c
    }

    /// C++ `Mcu::slotReload` / `QemuDevice::slotReload`.
    pub fn reload_firmware(&mut self, uid: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        match self.firmware_path_for(&uid) {
            Some(p) if p.is_file() => {
                let path = p.to_string_lossy().into_owned();
                self.load_firmware_path(&uid, &path)
            }
            _ => {
                crate::logging::log_sim("No File to reload ");
                Change::default()
            }
        }
    }

    pub(crate) fn eeprom_bytes(&self, uid: &str) -> Option<Vec<u8>> {
        if let Some(it) = self.scene.item_by_id(uid) {
            if let Part::Mcu(mcu) = &it.kind {
                return Some(mcu.mcu.device.eeprom().to_vec());
            }
        }
        if let Some(c) = self.running.as_ref() {
            for (id, m) in c.mcus() {
                if id == uid || uid.strip_suffix(id).is_some_and(|p| p.ends_with('/')) {
                    return Some(m.device.eeprom().to_vec());
                }
            }
        }
        None
    }

    /// C++ `Mcu::loadEEPROM`.
    pub fn load_eeprom_file(&mut self, uid: &str, path: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        let Some(mut data) = self.eeprom_bytes(&uid) else {
            return Change::default();
        };
        if let Err(e) = crate::memdata::load_bytes(std::path::Path::new(path), &mut data) {
            crate::logging::log_sim(e);
            return Change::default();
        }
        self.push_undo();
        if let Some(it) = self.scene.item_by_id_mut(&uid) {
            if let Part::Mcu(mcu) = &mut it.kind {
                mcu.mcu.device.replace_eeprom(&data);
            }
        }
        if let Some(c) = self.running.as_mut() {
            for (id, m) in c.mcus_mut() {
                if id == uid || uid.strip_suffix(id).is_some_and(|p| p.ends_with('/')) {
                    m.device.replace_eeprom(&data);
                }
            }
        }
        self.publish_mcu_snap();
        Change {
            items: true,
            history: true,
            sim: true,
            ..Change::default()
        }
    }

    /// C++ `Mcu::saveEEPROM`.
    pub fn save_eeprom_file(&self, uid: &str, path: &str) -> bool {
        let uid = if uid.is_empty() {
            self.scene
                .items()
                .iter()
                .find(|it| it.selected)
                .map(|it| it.id.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            uid.to_string()
        };
        let Some(data) = self.eeprom_bytes(&uid) else {
            return false;
        };
        if let Err(e) = crate::memdata::save_bytes(std::path::Path::new(path), &data) {
            crate::logging::log_sim(e);
            return false;
        }
        true
    }

    pub fn memory_item_info(&self, uid: &str) -> Option<MemoryItemInfo> {
        let it = self.scene.item_by_id(uid)?;
        let running_data = self
            .running
            .as_ref()
            .and_then(|c| c.component_memory_bytes(uid));
        match &it.kind {
            Part::Memory(m) => {
                let cell_bytes = (m.data_bits + 7) / 8;
                let total = 1 << m.addr_bits;
                let data = running_data.unwrap_or_else(|| {
                    let mut d = m.data.clone();
                    if d.len() < total {
                        d.resize(total, 0);
                    }
                    d
                });
                let title = if m.is_rom {
                    format!("ROM: {}", it.id)
                } else {
                    format!("RAM: {}", it.id)
                };
                Some(MemoryItemInfo {
                    id: it.id.clone(),
                    title,
                    is_rom: m.is_rom,
                    cell_bytes: cell_bytes.max(1),
                    word_bytes: cell_bytes.max(1),
                    data,
                })
            }
            Part::DynamicMemory(m) => {
                let total = 1 << m.addr_bits;
                let data = running_data.unwrap_or_else(|| {
                    let mut d = m.data.clone();
                    if d.len() < total {
                        d.resize(total, 0);
                    }
                    d
                });
                Some(MemoryItemInfo {
                    id: it.id.clone(),
                    title: format!("Dynamic RAM: {}", it.id),
                    is_rom: false,
                    cell_bytes: 1,
                    word_bytes: 1,
                    data,
                })
            }
            Part::I2CRam(m) => {
                let total = m.size_bytes.max(1);
                let data = running_data.unwrap_or_else(|| {
                    let mut d = m.data.clone();
                    if d.len() < total {
                        d.resize(total, 0);
                    }
                    d
                });
                Some(MemoryItemInfo {
                    id: it.id.clone(),
                    title: format!("I2C RAM: {}", it.id),
                    is_rom: false,
                    cell_bytes: 1,
                    word_bytes: 1,
                    data,
                })
            }
            _ => None,
        }
    }

    pub fn get_memory_bytes(&self, uid: &str) -> Option<Vec<u8>> {
        if let Some(c) = &self.running
            && let Some(bytes) = c.component_memory_bytes(uid)
        {
            return Some(bytes);
        }
        self.item_data_bytes(uid)
    }

    pub fn set_memory_byte(&mut self, uid: &str, addr: usize, val: u8) -> bool {
        if let Some(c) = self.running.as_mut() {
            c.set_component_memory_byte(uid, addr, val);
        }
        if let Some(it) = self.scene.item_by_id_mut(uid) {
            match &mut it.kind {
                Part::Memory(m) => {
                    let total = 1 << m.addr_bits;
                    if m.data.len() < total {
                        m.data.resize(total, 0);
                    }
                    if addr < m.data.len() {
                        m.data[addr] = val;
                        return true;
                    }
                }
                Part::DynamicMemory(m) => {
                    let total = 1 << m.addr_bits;
                    if m.data.len() < total {
                        m.data.resize(total, 0);
                    }
                    if addr < m.data.len() {
                        m.data[addr] = val;
                        return true;
                    }
                }
                Part::I2CRam(m) => {
                    if m.data.len() < m.size_bytes {
                        m.data.resize(m.size_bytes, 0);
                    }
                    if addr < m.data.len() {
                        m.data[addr] = val;
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn set_memory_bytes(&mut self, uid: &str, data: &[u8]) -> bool {
        if let Some(c) = self.running.as_mut() {
            c.set_component_memory_bytes(uid, data);
        }
        if let Some(it) = self.scene.item_by_id_mut(uid) {
            match &mut it.kind {
                Part::Memory(m) => {
                    let total = 1 << m.addr_bits;
                    if m.data.len() < total {
                        m.data.resize(total, 0);
                    }
                    let n = m.data.len().min(data.len());
                    m.data[..n].copy_from_slice(&data[..n]);
                    return true;
                }
                Part::DynamicMemory(m) => {
                    let total = 1 << m.addr_bits;
                    if m.data.len() < total {
                        m.data.resize(total, 0);
                    }
                    let n = m.data.len().min(data.len());
                    m.data[..n].copy_from_slice(&data[..n]);
                    return true;
                }
                Part::I2CRam(m) => {
                    if m.data.len() < m.size_bytes {
                        m.data.resize(m.size_bytes, 0);
                    }
                    let n = m.data.len().min(data.len());
                    m.data[..n].copy_from_slice(&data[..n]);
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn item_data_bytes(&self, uid: &str) -> Option<Vec<u8>> {
        let it = self.scene.item_by_id(uid)?;
        match &it.kind {
            Part::Memory(m) => Some(m.data.clone()),
            Part::DynamicMemory(m) => Some(m.data.clone()),
            Part::I2CRam(m) => Some(m.data.clone()),
            _ => None,
        }
    }

    pub(crate) fn set_item_data_bytes(&mut self, uid: &str, data: Vec<u8>) -> bool {
        let Some(it) = self.scene.item_by_id_mut(uid) else {
            return false;
        };
        match &mut it.kind {
            Part::Memory(m) => {
                let n = m.data.len().min(data.len());
                m.data[..n].copy_from_slice(&data[..n]);
                true
            }
            Part::DynamicMemory(m) => {
                let n = m.data.len().min(data.len());
                m.data[..n].copy_from_slice(&data[..n]);
                true
            }
            Part::I2CRam(m) => {
                let n = m.data.len().min(data.len());
                m.data[..n].copy_from_slice(&data[..n]);
                true
            }
            _ => false,
        }
    }

    pub fn load_item_data_file(&mut self, uid: &str, path: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        let Some(mut data) = self.item_data_bytes(&uid) else {
            return Change::default();
        };
        if data.is_empty() {
            data.resize(256, 0);
        }
        if let Err(e) = crate::memdata::load_bytes(std::path::Path::new(path), &mut data) {
            crate::logging::log_sim(e);
            return Change::default();
        }
        self.push_undo();
        if !self.set_item_data_bytes(&uid, data) {
            self.history.undo.pop();
            return Change::default();
        }
        Change::edit()
    }

    pub fn save_item_data_file(&self, uid: &str, path: &str) -> bool {
        let uid = if uid.is_empty() {
            self.scene
                .items()
                .iter()
                .find(|it| it.selected)
                .map(|it| it.id.clone())
                .unwrap_or_default()
                .to_string()
        } else {
            uid.to_string()
        };
        let Some(data) = self.item_data_bytes(&uid) else {
            return false;
        };
        if let Err(e) = crate::memdata::save_bytes(std::path::Path::new(path), &data) {
            crate::logging::log_sim(e);
            return false;
        }
        true
    }

    pub fn memory_table_rows(&self, uid: &str) -> Vec<serde_json::Value> {
        let uid = if uid.is_empty() {
            self.scene
                .items()
                .iter()
                .find(|it| it.selected)
                .map(|it| it.id.clone())
                .unwrap_or_default()
        } else {
            uid.to_string()
        };
        let Some(data) = self
            .item_data_bytes(&uid)
            .or_else(|| self.eeprom_bytes(&uid))
        else {
            return Vec::new();
        };
        data.iter()
            .enumerate()
            .map(|(i, v)| {
                serde_json::json!({
                    "addr": format!("0x{i:04X}"),
                    "hex": format!("{v:02X}"),
                    "dec": *v,
                })
            })
            .collect()
    }

    pub fn load_functions_file(&mut self, uid: &str, path: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        let Ok(src) = std::fs::read_to_string(path) else {
            return Change::default();
        };
        let mut lines = src.lines();
        let n_in = lines
            .next()
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(1);
        let _n_out = lines.next();
        let expr: String = lines.collect::<Vec<_>>().join("\n");
        self.push_undo();
        let Some(it) = self.scene.item_by_id_mut(&uid) else {
            self.history.undo.pop();
            return Change::default();
        };
        if let Part::Function(func) = &mut it.kind {
            func.n_inputs = n_in.clamp(1, 16);
            func.expression = expr;
            Change::edit()
        } else {
            self.history.undo.pop();
            Change::default()
        }
    }

    pub fn save_functions_file(&self, uid: &str, path: &str) -> bool {
        let uid = if uid.is_empty() {
            self.scene
                .items()
                .iter()
                .find(|it| it.selected)
                .map(|it| it.id.clone())
                .unwrap_or_default()
        } else {
            uid.to_string()
        };
        let Some(it) = self.scene.item_by_id(&uid) else {
            return false;
        };
        let Part::Function(func) = &it.kind else {
            return false;
        };
        let body = format!("{}\n1\n{}", func.n_inputs, func.expression);
        std::fs::write(path, body).is_ok()
    }

    pub fn load_image_file(&mut self, uid: &str, path: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => return Change::default(),
        };
        let pixmap = match crate::components::shape::decode_image_bytes(&bytes) {
            Some(pm) => pm,
            None => return Change::default(),
        };

        self.push_undo();
        let Some(it) = self.scene.item_by_id_mut(&uid) else {
            self.history.undo.pop();
            return Change::default();
        };
        if let Part::Shape(shape) = &mut it.kind {
            if shape.shape_kind == ShapeKind::Image {
                shape.width = pixmap.width() as f64;
                shape.height = pixmap.height() as f64;
                shape.image_file = path.to_string();
                if shape.embed_bck {
                    shape.bck_data = crate::components::shape::bytes_to_hex(&bytes);
                }
                shape.image_pixmap = Some(std::sync::Arc::new(pixmap));
                return Change::edit();
            }
        }
        self.history.undo.pop();
        Change::default()
    }

    pub fn save_image_item(&self, uid: &str, path: &str) -> bool {
        let Some(it) = self.scene.item_by_id(uid) else {
            return false;
        };
        if let Part::Shape(shape) = &it.kind {
            if shape.shape_kind == ShapeKind::Image {
                if !shape.bck_data.is_empty() {
                    let bytes = crate::components::shape::hex_to_bytes(&shape.bck_data);
                    return std::fs::write(path, bytes).is_ok();
                } else if !shape.image_file.is_empty()
                    && std::path::Path::new(&shape.image_file).exists()
                {
                    return std::fs::copy(&shape.image_file, path).is_ok();
                } else if let Some(pm) = &shape.image_pixmap {
                    if let Ok(png_bytes) = pm.encode_png() {
                        return std::fs::write(path, png_bytes).is_ok();
                    }
                }
            }
        }
        false
    }

    pub fn load_sd_image(&mut self, uid: &str, path: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        self.push_undo();
        let Some(it) = self.scene.item_by_id_mut(&uid) else {
            self.history.undo.pop();
            return Change::default();
        };
        if let Part::SdCard(sd) = &mut it.kind {
            sd.file = path.to_string();
            Change::edit()
        } else {
            self.history.undo.pop();
            Change::default()
        }
    }

    pub fn eject_sd_card(&mut self, uid: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        self.push_undo();
        let Some(it) = self.scene.item_by_id_mut(&uid) else {
            self.history.undo.pop();
            return Change::default();
        };
        if let Part::SdCard(sd) = &mut it.kind {
            sd.file.clear();
            Change::edit()
        } else {
            self.history.undo.pop();
            Change::default()
        }
    }

    pub fn set_tunnel_group_visible(&mut self, uid: &str, visible: bool) -> Change {
        let uid = self.resolve_item_uid(uid);
        let Some(name) = self.scene.item_by_id(&uid).and_then(|it| {
            if let Part::Tunnel(t) = &it.kind {
                Some(t.name.clone())
            } else {
                None
            }
        }) else {
            return Change::default();
        };
        self.push_undo();
        if visible {
            for it in self.scene.items_mut() {
                if let Part::Tunnel(t) = &mut it.kind {
                    t.show = t.name == name;
                }
            }
        } else {
            for it in self.scene.items_mut() {
                if let Part::Tunnel(t) = &mut it.kind {
                    if t.name == name {
                        t.show = false;
                    }
                }
            }
        }
        Change::edit()
    }

    pub fn rename_tunnel_group(&mut self, uid: &str, new_name: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        if new_name.is_empty() {
            return Change::default();
        }
        let Some(old) = self.scene.item_by_id(&uid).and_then(|it| {
            if let Part::Tunnel(t) = &it.kind {
                Some(t.name.clone())
            } else {
                None
            }
        }) else {
            return Change::default();
        };
        self.push_undo();
        for it in self.scene.items_mut() {
            if let Part::Tunnel(t) = &mut it.kind {
                if t.name == old {
                    t.name = new_name.to_string();
                }
            }
        }
        Change::edit()
    }

    pub fn toggle_probe_pause(&mut self, uid: &str) -> Change {
        let uid = self.resolve_item_uid(uid);
        self.push_undo();
        let Some(it) = self.scene.item_by_id_mut(&uid) else {
            self.history.undo.pop();
            return Change::default();
        };
        if let Part::Probe(p) = &mut it.kind {
            p.pause_at_change = !p.pause_at_change;
            Change::edit()
        } else {
            self.history.undo.pop();
            Change::default()
        }
    }

    pub fn test_unit_truth(&self, uid: &str) -> Vec<u32> {
        let uid = if uid.is_empty() {
            self.scene
                .items()
                .iter()
                .find(|it| it.selected)
                .map(|it| it.id.clone())
                .unwrap_or_default()
        } else {
            uid.to_string()
        };
        self.scene
            .item_by_id(&uid)
            .and_then(|it| {
                if let Part::TestUnit(t) = &it.kind {
                    Some(t.truth.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    pub(crate) fn selected_or_nested_mcu(&self) -> Option<&crate::canvas::scene::Item> {
        let it = self.scene.items().iter().find(|it| it.selected)?;
        match &it.kind {
            Part::Mcu(_) | Part::QemuDevice(_) => Some(it),
            Part::Subcircuit(_) => {
                let uid = self.selected_nested_mcu_uid()?;
                self.scene.item_by_id(&uid)
            }
            _ => None,
        }
    }

    pub fn selected_has_firmware(&self) -> bool {
        let Some(it) = self.selected_or_nested_mcu() else {
            return false;
        };
        match &it.kind {
            Part::Mcu(mcu) => mcu.mcu.firmware.is_some(),
            Part::QemuDevice(qemu) => !qemu.qemu.firmware.is_empty(),
            _ => false,
        }
    }

    pub fn selected_has_flash(&self) -> bool {
        if let Some(it) = self.selected_or_nested_mcu() {
            return match &it.kind {
                Part::Mcu(mcu) => mcu.mcu.device.flash_size() > 0,
                Part::QemuDevice(_) => true,
                _ => false,
            };
        }
        self.selected_nested_mcu_uid().is_some()
    }

    pub fn selected_has_eeprom(&self) -> bool {
        let Some(it) = self.selected_or_nested_mcu() else {
            return false;
        };
        matches!(&it.kind, Part::Mcu(mcu) if !mcu.mcu.device.eeprom().is_empty())
    }

    pub fn selected_nested_mcu_uid(&self) -> Option<String> {
        let it = self.scene.items().iter().find(|it| it.selected)?;
        if !matches!(&it.kind, Part::Subcircuit(_)) {
            return None;
        }
        let devices = self
            .scene
            .collect_programmable_devices(&self.search(), None);
        devices
            .into_iter()
            .find(|d| d.label.starts_with(&it.label) || d.id.contains(&it.id))
            .map(|d| d.uid)
    }

    /// Load a WAV file into a WaveGen item. C++ `WaveGen::slotLoad` / `WaveGen::setFile`.
    pub fn load_wav_file(&mut self, item_id: &str, path: &str) -> Change {
        let p = std::path::Path::new(path);
        let mut ok = self.scene.load_wav_file(item_id, p);
        if !ok {
            if let Some(circuit_path) = self.file_path.as_ref() {
                if let Some(parent) = std::path::Path::new(circuit_path).parent() {
                    let resolved = parent.join(p);
                    ok = self.scene.load_wav_file(item_id, &resolved);
                }
            }
        }
        if ok {
            let mut c = Change {
                items: true,
                sim: true,
                ..Change::default()
            };
            c.merge(self.refresh_sim());
            c
        } else {
            Change::default()
        }
    }

    pub fn new_circuit(&mut self) -> Change {
        self.new_circuit_with(crate::CircSettings::default())
    }

    pub fn new_circuit_with(&mut self, settings: crate::CircSettings) -> Change {
        self.scene = Scene::new();
        *self.scene.settings_mut() = settings.clone();
        self.viewport
            .set_scene_size(settings.width as f64, settings.height as f64);
        self.drag = Drag::None;
        self.banding = false;
        self.file_path = None;
        self.hovered_pin = None;
        self.history.clear();
        self.pending = None;
        self.prop_uid = None;
        self.prop_open = false;
        self.open_prop_uids.clear();
        self.last_opened_prop_uid = None;
        self.saved_sim1 = self.scene.to_sim1();
        self.overload_states.clear();
        self.overload_log.clear();
        self.overload_escalated = false;
        let mut c = self.power_off();
        c.items = true;
        c.wires = true;
        c.history = true;
        c.props = true;
        c.open_props = true;
        c.settings = true;
        c.viewport = true;
        c
    }

    pub fn set_catalog(&mut self, catalog: crate::catalog::Catalog) {
        self.catalog = catalog;
    }

    pub(crate) fn search(&self) -> crate::subcircuit::SubcSearch {
        let mut s = crate::subcircuit::SubcSearch::from_circuit_path(self.file_path.as_deref());
        s.catalog = if self.catalog.is_empty() {
            crate::catalog::standard()
        } else {
            self.catalog.clone()
        };
        s
    }

    pub fn collect_programmable_devices(
        &self,
        active_id: Option<&str>,
    ) -> Vec<crate::canvas::scene::ProgrammableDevice> {
        self.scene
            .collect_programmable_devices(&self.search(), active_id)
    }

    pub fn snapshot_subcircuits(&self) -> Vec<crate::subcircuit::SubcTreeNode> {
        self.scene.snapshot_subcircuits(&self.search())
    }

    pub fn load_sim1(&mut self, src: &str, path: Option<String>) -> crate::Result<Change> {
        self.load_sim1_as(src, path, true)
    }

    /// Load recovered content while keeping `saved_sim1` as the on-disk baseline
    /// so `is_modified()` stays true until the user saves.
    pub fn load_recovered(
        &mut self,
        src: &str,
        path: Option<String>,
        disk: &str,
    ) -> crate::Result<Change> {
        let search = {
            let mut s = crate::subcircuit::SubcSearch::from_circuit_path(path.as_deref());
            s.catalog = if self.catalog.is_empty() {
                crate::catalog::standard()
            } else {
                self.catalog.clone()
            };
            s
        };
        let saved = if disk.is_empty() {
            String::new()
        } else {
            Scene::from_sim1_with(disk, &search)
                .map(|sc| sc.to_sim1())
                .unwrap_or_else(|_| disk.to_string())
        };
        let c = self.load_sim1_as(src, path, false)?;
        self.saved_sim1 = saved;
        Ok(c)
    }

    pub(crate) fn load_sim1_as(
        &mut self,
        src: &str,
        path: Option<String>,
        mark_saved: bool,
    ) -> crate::Result<Change> {
        let mut search = crate::subcircuit::SubcSearch::from_circuit_path(path.as_deref());
        search.catalog = if self.catalog.is_empty() {
            crate::catalog::standard()
        } else {
            self.catalog.clone()
        };
        self.scene = Scene::from_sim1_with(src, &search)?;
        let s = self.scene.settings();
        self.viewport
            .set_scene_size(s.width as f64, s.height as f64);
        self.drag = Drag::None;
        self.banding = false;
        self.file_path = path;
        self.hovered_pin = None;
        self.history.clear();
        self.pending = None;
        self.prop_uid = None;
        self.prop_open = false;
        self.open_prop_uids.clear();
        self.last_opened_prop_uid = None;
        if mark_saved {
            self.saved_sim1 = self.scene.to_sim1();
        }
        let mut c = Change {
            items: true,
            wires: true,
            history: true,
            props: true,
            open_props: true,
            settings: true,
            viewport: true,
            ..Change::default()
        };
        c.merge(self.zoom_to_fit());
        c.merge(self.refresh_sim());
        self.publish_mcu_snap();
        Ok(c)
    }

    pub fn to_sim1(&self) -> String {
        self.scene.to_sim1()
    }

    /// Circuit path with `.png`, or `circuit.png` if unsaved. C++ `changeExt`.
    pub fn suggest_image_path(&self) -> String {
        match self.file_path.as_ref() {
            Some(p) => {
                let mut pb = std::path::PathBuf::from(p);
                pb.set_extension("png");
                pb.to_string_lossy().into_owned()
            }
            None => {
                let s = crate::settings::get();
                if let Some(dir) = s.last_project_dir.as_ref() {
                    let mut pb = std::path::PathBuf::from(dir);
                    pb.push("circuit.png");
                    pb.to_string_lossy().into_owned()
                } else {
                    "circuit.png".into()
                }
            }
        }
    }

    /// Raster / SVG from scene primitives. No Qt.
    pub fn save_image(
        &self,
        path: impl AsRef<std::path::Path>,
        palette: &export::Palette,
    ) -> crate::Result<()> {
        export::save_image(self, path.as_ref(), palette)
    }

    /// Overlay another `.sim1` at its original coordinates (canvas Import).
    pub fn import_sim1(&mut self, src: &str) -> Change {
        self.push_undo();
        match self
            .scene
            .paste_sim1_with(src, Point::zero(), &self.search())
        {
            Ok(true) => {
                let mut c = Change::edit();
                c.merge(self.refresh_sim());
                c
            }
            _ => {
                self.history.undo.pop();
                Change::default()
            }
        }
    }

    pub fn set_file_path(&mut self, path: Option<String>) {
        self.file_path = path;
    }
}
