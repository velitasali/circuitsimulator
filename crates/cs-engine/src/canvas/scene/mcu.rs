//! MCU and firmware management, snapshotting, device discovery, and live synchronization.

use super::Scene;
use super::item::{Item, ProgrammableDevice};
use crate::Circuit;
use crate::components::Part;

impl Scene {
    pub fn hidden_mcu_snap(&self) -> crate::mcu::McuSnap {
        self.mcu_snap()
    }

    pub fn mcu_snap(&self) -> crate::mcu::McuSnap {
        self.items
            .iter()
            .find_map(|it| match &it.kind {
                Part::Mcu(mcu) => Some(crate::mcu::McuSnap::from_mcu(&it.id, &mcu.mcu)),
                _ => None,
            })
            .or_else(|| {
                self.hidden.iter().find_map(|c| match &c.kind {
                    crate::elements::Kind::Mcu(m) => Some(crate::mcu::McuSnap::from_mcu(&c.id, m)),
                    _ => None,
                })
            })
            .unwrap_or_default()
    }

    pub(crate) fn apply_mcu_poke_to(m: &mut crate::mcu::McuComp, poke: crate::mcu::McuPoke) {
        match poke {
            crate::mcu::McuPoke::Ram { addr, value } => {
                m.device.set_monitor_ram(addr, value);
            }
            crate::mcu::McuPoke::Flash { addr, value } => {
                m.device.set_flash(addr as usize, value);
            }
            crate::mcu::McuPoke::Eeprom { addr, value } => {
                m.device.set_eeprom(addr as usize, value);
            }
        }
    }

    pub fn poke_hidden_mcu(&mut self, poke: crate::mcu::McuPoke) -> bool {
        self.poke_mcu(poke)
    }

    /// Snapshot subcircuit tree hierarchy from the scene items.
    pub fn snapshot_subcircuits(
        &self,
        search: &crate::subcircuit::SubcSearch,
    ) -> Vec<crate::subcircuit::SubcTreeNode> {
        let mut result = Vec::new();
        for it in &self.items {
            if let Part::Subcircuit(subc) = &it.kind {
                let device = &subc.device;
                let nested_src = &subc.nested_src;
                let nested_path = &subc.nested_path;
                let label = if !it.label.is_empty() {
                    it.label.clone()
                } else if !device.is_empty() {
                    device.clone()
                } else {
                    it.id.clone()
                };
                let file_path = nested_path
                    .clone()
                    .or_else(|| {
                        crate::subcircuit::resolve_subc(device, search)
                            .and_then(|r| r.path.map(|p| p.to_string_lossy().into_owned()))
                    })
                    .unwrap_or_default();

                let src = if !nested_src.is_empty() {
                    Some(nested_src.clone())
                } else {
                    crate::subcircuit::resolve_subc(device, search).map(|r| r.src)
                };

                let children = if let Some(inner_src) = src {
                    if let Ok(parsed) = crate::sim1::parse_sim1(&inner_src) {
                        let child_search = if !file_path.is_empty() {
                            search.child_for(std::path::Path::new(&file_path))
                        } else {
                            search.clone()
                        };
                        crate::subcircuit::snapshot_subcircuits_from_parsed(&parsed, &child_search)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                result.push(crate::subcircuit::SubcTreeNode {
                    label,
                    file_path,
                    children,
                });
            }
        }
        result.sort_by(|a, b| a.label.cmp(&b.label));
        result
    }

    /// A programmable device (MCU, QEMU device, or board-mounted MCU).
    pub fn collect_programmable_devices(
        &self,
        search: &crate::subcircuit::SubcSearch,
        active_id: Option<&str>,
    ) -> Vec<ProgrammableDevice> {
        let mut devices = Vec::new();
        self.collect_devices_recursive(&self.items, search, "", &mut devices);

        // Sort keys built once: board-mounted MCUs group under their board, matching C++
        devices.sort_by(|a, b| a.label.cmp(&b.label));

        for (i, dev) in devices.iter_mut().enumerate() {
            if let Some(aid) = active_id {
                dev.is_active = dev.id == aid || dev.uid == aid;
            } else {
                dev.is_active = i == 0;
            }
        }
        devices
    }

    fn collect_devices_recursive(
        &self,
        items: &[Item],
        search: &crate::subcircuit::SubcSearch,
        parent_label: &str,
        out: &mut Vec<ProgrammableDevice>,
    ) {
        for it in items {
            match &it.kind {
                Part::Mcu(_) => {
                    let mut label = it.label.clone();
                    if !parent_label.is_empty() {
                        label = format!("{parent_label} / {label}");
                    }
                    let kind = crate::subcircuit::device_from_id(&it.id);
                    let display_text = if !kind.is_empty() && kind != label && kind != it.label {
                        format!("{label}  ({kind})")
                    } else {
                        label.clone()
                    };
                    out.push(ProgrammableDevice {
                        id: it.id.clone(),
                        uid: it.id.clone(),
                        label,
                        display_text,
                        kind,
                        is_active: false,
                    });
                }
                Part::QemuDevice(qemu) => {
                    let mut label = it.label.clone();
                    if !parent_label.is_empty() {
                        label = format!("{parent_label} / {label}");
                    }
                    let kind = qemu.qemu.device.clone();
                    let display_text = if !kind.is_empty() && kind != label && kind != it.label {
                        format!("{label}  ({kind})")
                    } else {
                        label.clone()
                    };
                    out.push(ProgrammableDevice {
                        id: it.id.clone(),
                        uid: it.id.clone(),
                        label,
                        display_text,
                        kind,
                        is_active: qemu.active,
                    });
                }
                Part::Subcircuit(subc) => {
                    let board_label = if parent_label.is_empty() {
                        it.label.clone()
                    } else {
                        format!("{parent_label} / {}", it.label)
                    };
                    let src = if !subc.nested_src.is_empty() {
                        Some(subc.nested_src.clone())
                    } else {
                        crate::subcircuit::resolve_subc(&subc.device, search).map(|r| r.src)
                    };
                    if let Some(inner_src) = src {
                        if let Ok(parsed) = crate::sim1::parse_sim1(&inner_src) {
                            let _child_search = if let Some(p) = &subc.nested_path {
                                search.child_for(std::path::Path::new(p))
                            } else {
                                search.clone()
                            };
                            for inner_it in &parsed.items {
                                match &inner_it.comp.kind {
                                    crate::elements::Kind::Mcu(_) => {
                                        let inner_id = inner_it.comp.id.clone();
                                        let inner_label = inner_it
                                            .label
                                            .as_ref()
                                            .unwrap_or(&inner_it.comp.id)
                                            .clone();
                                        let inner_dev =
                                            crate::subcircuit::device_from_id(&inner_id);
                                        let full_label = format!("{board_label} / {inner_label}");
                                        let display_text =
                                            if !inner_dev.is_empty() && inner_dev != inner_label {
                                                format!("{full_label}  ({inner_dev})")
                                            } else {
                                                full_label.clone()
                                            };
                                        out.push(ProgrammableDevice {
                                            id: format!("{}/{}", it.id, inner_id),
                                            uid: format!("{}/{}", it.id, inner_id),
                                            label: full_label,
                                            display_text,
                                            kind: inner_dev,
                                            is_active: false,
                                        });
                                    }
                                    crate::elements::Kind::McuItem(inner_spec) => {
                                        let inner_id = inner_it.comp.id.clone();
                                        let inner_label = inner_it
                                            .label
                                            .as_ref()
                                            .unwrap_or(&inner_it.comp.id)
                                            .clone();
                                        let inner_dev = if !inner_spec.device.is_empty() {
                                            inner_spec.device.clone()
                                        } else {
                                            crate::subcircuit::device_from_id(&inner_id)
                                        };
                                        let full_label = format!("{board_label} / {inner_label}");
                                        let display_text =
                                            if !inner_dev.is_empty() && inner_dev != inner_label {
                                                format!("{full_label}  ({inner_dev})")
                                            } else {
                                                full_label.clone()
                                            };
                                        out.push(ProgrammableDevice {
                                            id: format!("{}/{}", it.id, inner_id),
                                            uid: format!("{}/{}", it.id, inner_id),
                                            label: full_label,
                                            display_text,
                                            kind: inner_dev,
                                            is_active: false,
                                        });
                                    }
                                    crate::elements::Kind::QemuDevice(inner_qemu) => {
                                        let inner_id = inner_it.comp.id.clone();
                                        let inner_label = inner_it
                                            .label
                                            .as_ref()
                                            .unwrap_or(&inner_it.comp.id)
                                            .clone();
                                        let inner_dev = inner_qemu.device.clone();
                                        let full_label = format!("{board_label} / {inner_label}");
                                        let display_text =
                                            if !inner_dev.is_empty() && inner_dev != inner_label {
                                                format!("{full_label}  ({inner_dev})")
                                            } else {
                                                full_label.clone()
                                            };
                                        out.push(ProgrammableDevice {
                                            id: format!("{}/{}", it.id, inner_id),
                                            uid: format!("{}/{}", it.id, inner_id),
                                            label: full_label,
                                            display_text,
                                            kind: inner_dev,
                                            is_active: false,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// C++ `Mcu::self()->load(firmware)` — first MCU in netlist order.
    pub fn upload_firmware(&mut self, path: &std::path::Path) -> bool {
        self.upload_firmware_to(path, None)
    }

    pub fn upload_firmware_to(
        &mut self,
        path: &std::path::Path,
        target_device_id: Option<&str>,
    ) -> bool {
        if !path.is_file() {
            return false;
        }
        if let Some(target) = target_device_id {
            for it in &mut self.items {
                if it.id == target {
                    if let Part::Mcu(mcu) = &mut it.kind {
                        if mcu.mcu.load_firmware_file(path).is_ok() {
                            mcu.mcu.firmware = Some(path.to_string_lossy().into_owned());
                            return true;
                        }
                    } else if let Part::QemuDevice(qemu) = &mut it.kind {
                        qemu.qemu.firmware = path.to_string_lossy().into_owned();
                        return true;
                    }
                }
            }
            for c in &mut self.hidden {
                if c.id == target || target.strip_suffix(&c.id).is_some_and(|p| p.ends_with('/')) {
                    if let crate::elements::Kind::Mcu(mcu) = &mut c.kind {
                        if mcu.load_firmware_file(path).is_ok() {
                            mcu.firmware = Some(path.to_string_lossy().into_owned());
                            return true;
                        }
                    }
                }
            }
        }
        for it in &mut self.items {
            if let Part::Mcu(mcu) = &mut it.kind {
                if mcu.mcu.load_firmware_file(path).is_ok() {
                    mcu.mcu.firmware = Some(path.to_string_lossy().into_owned());
                    return true;
                }
            }
        }
        false
    }

    /// Load a WAV file into a WaveGen component, setting wave_type="Wav" and freq_hz=sample_rate.
    pub fn load_wav_file(&mut self, item_id: &str, path: &std::path::Path) -> bool {
        let Ok(wav) = crate::wav::WavData::from_file(path) else {
            return false;
        };
        for it in &mut self.items {
            if it.id == item_id {
                if let Part::WaveGen(wg) = &mut it.kind {
                    wg.wave_type = crate::components::WaveType::Wav;
                    wg.freq_hz = wav.sample_rate as f64;
                    wg.file = path.to_string_lossy().into_owned();
                    wg.wav_data = Some(wav.samples);
                    return true;
                }
            }
        }
        false
    }

    pub fn poke_mcu(&mut self, poke: crate::mcu::McuPoke) -> bool {
        for it in &mut self.items {
            if let Part::Mcu(mcu) = &mut it.kind {
                Self::apply_mcu_poke_to(&mut mcu.mcu, poke);
                return true;
            }
        }
        for c in &mut self.hidden {
            if let crate::elements::Kind::Mcu(m) = &mut c.kind {
                Self::apply_mcu_poke_to(m, poke);
                return true;
            }
        }
        false
    }

    /// Copy live MCU state back so the monitor still shows it after power-off.
    pub fn capture_mcus_from(&mut self, circuit: &Circuit) {
        for c in circuit.components() {
            if let crate::elements::Kind::Mcu(mcu) = &c.kind {
                if let Some(it) = self.items.iter_mut().find(|h| h.id == c.id)
                    && let Part::Mcu(slot) = &mut it.kind
                {
                    slot.mcu = mcu.clone();
                }
                if let Some(slot) = self.hidden.iter_mut().find(|h| h.id == c.id) {
                    slot.kind = crate::elements::Kind::Mcu(mcu.clone());
                }
            }
        }
    }

    /// Copy live memory state back so memory components keep their data after power-off.
    pub fn capture_memories_from(&mut self, circuit: &Circuit) {
        for c in circuit.components() {
            match &c.kind {
                crate::elements::Kind::Memory(m) => {
                    if let Some(it) = self.items.iter_mut().find(|h| h.id == c.id)
                        && let Part::Memory(p) = &mut it.kind
                    {
                        p.data = m.data.clone();
                    }
                }
                crate::elements::Kind::DynamicMemory(dm) => {
                    if let Some(it) = self.items.iter_mut().find(|h| h.id == c.id)
                        && let Part::DynamicMemory(p) = &mut it.kind
                    {
                        p.data = dm.data.clone();
                    }
                }
                crate::elements::Kind::I2CRam(r) => {
                    if let Some(it) = self.items.iter_mut().find(|h| h.id == c.id)
                        && let Part::I2CRam(p) = &mut it.kind
                    {
                        p.data = r.data.clone();
                    }
                }
                _ => {}
            }
        }
    }
}
