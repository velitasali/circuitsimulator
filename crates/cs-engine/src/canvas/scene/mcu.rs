//! MCU and firmware management, snapshotting, device discovery, and live synchronization.

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::Path;

use super::Scene;
use super::item::{Item, ProgrammableDevice};
use crate::Circuit;
use crate::circ1;
use crate::components::Part;
use crate::debug::DebugSession;
use crate::elements::Kind;
use crate::mcu::McuSnap;
use crate::subcircuit::{self, SubcSearch, SubcTreeNode};

impl Scene {
    pub fn hidden_mcu_snap(&self) -> McuSnap {
        self.mcu_snap()
    }

    pub fn mcu_snap(&self) -> McuSnap {
        self.items
            .iter()
            .find_map(|it| match &it.kind {
                Part::Mcu(mcu) => Some(McuSnap::from_mcu(&it.id, &mcu.mcu)),
                _ => None,
            })
            .or_else(|| {
                self.hidden.iter().find_map(|c| match &c.kind {
                    Kind::Mcu(m) => Some(McuSnap::from_mcu(&c.id, m)),
                    _ => None,
                })
            })
            .or_else(|| {
                self.items.iter().find_map(|it| match &it.kind {
                    Part::QemuDevice(_) => Some(McuSnap::from_qemu(
                        &it.id,
                        DebugSession::global().remote_pc(),
                    )),
                    _ => None,
                })
            })
            .or_else(|| {
                self.hidden.iter().find_map(|c| match &c.kind {
                    Kind::QemuDevice(_) => Some(McuSnap::from_qemu(
                        &c.id,
                        DebugSession::global().remote_pc(),
                    )),
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
    pub fn snapshot_subcircuits(&self, search: &SubcSearch) -> Vec<SubcTreeNode> {
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
                let resolved = if nested_path.is_none() || nested_src.is_empty() {
                    subcircuit::resolve_subc(device, search)
                } else {
                    None
                };
                let file_path = nested_path
                    .clone()
                    .or_else(|| {
                        resolved
                            .as_ref()
                            .and_then(|r| r.path.as_ref())
                            .map(|p| p.to_string_lossy().into_owned())
                    })
                    .unwrap_or_default();

                let src: Option<Cow<'_, str>> = if !nested_src.is_empty() {
                    Some(Cow::Borrowed(nested_src.as_str()))
                } else {
                    resolved.map(|r| Cow::Owned(r.src))
                };

                let children = if let Some(inner_src) = src
                    && subcircuit::may_contain_subcircuit(&inner_src)
                {
                    let child_search = if !file_path.is_empty() {
                        search.child_for(Path::new(&file_path))
                    } else {
                        search.clone()
                    };
                    let mut visited = HashSet::new();
                    visited.insert(subcircuit::subc_visit_key(&file_path, device));
                    subcircuit::snapshot_subcircuits_from_src_with_visited(
                        &inner_src,
                        &child_search,
                        1,
                        &mut visited,
                    )
                } else {
                    Vec::new()
                };

                result.push(SubcTreeNode {
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
        search: &SubcSearch,
        active_id: Option<&str>,
    ) -> Vec<ProgrammableDevice> {
        let mut devices = Vec::new();
        for it in &self.items {
            self.collect_item_programmable_devices(it, search, &mut devices);
        }

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

    pub(crate) fn collect_item_programmable_devices(
        &self,
        it: &Item,
        search: &SubcSearch,
        out: &mut Vec<ProgrammableDevice>,
    ) {
        match &it.kind {
            Part::Mcu(_) => {
                let label = it.label.clone();
                let kind = subcircuit::device_from_id(&it.id);
                let display_text = if !kind.is_empty() && kind != label {
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
                let label = it.label.clone();
                let kind = qemu.qemu.device.clone();
                let display_text = if !kind.is_empty() && kind != label {
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
                let board_label = &it.label;
                let src: Option<Cow<'_, str>> = if !subc.nested_src.is_empty() {
                    Some(Cow::Borrowed(subc.nested_src.as_str()))
                } else {
                    subcircuit::resolve_subc(&subc.device, search).map(|r| Cow::Owned(r.src))
                };
                if let Some(inner_src) = src
                    && subcircuit::may_contain_programmable_device(&inner_src)
                    && let Ok(parsed) = circ1::load_circ1(&inner_src).map(circ1::to_parsed_circuit)
                {
                    let has_exposed_mcu = parsed.items.iter().any(|inner_it| {
                        inner_it.main_comp
                            && matches!(
                                inner_it.comp.kind,
                                Kind::Mcu(_) | Kind::McuItem(_) | Kind::QemuDevice(_)
                            )
                    });
                    for inner_it in &parsed.items {
                        if has_exposed_mcu && !inner_it.main_comp {
                            continue;
                        }
                        let inner_dev = match &inner_it.comp.kind {
                            Kind::Mcu(_) => subcircuit::device_from_id(&inner_it.comp.id),
                            Kind::McuItem(inner_spec) => {
                                if !inner_spec.device.is_empty() {
                                    inner_spec.device.clone()
                                } else {
                                    subcircuit::device_from_id(&inner_it.comp.id)
                                }
                            }
                            Kind::QemuDevice(inner_qemu) => inner_qemu.device.clone(),
                            _ => continue,
                        };
                        let inner_id = &inner_it.comp.id;
                        let inner_label = inner_it.label.as_deref().unwrap_or(inner_id);
                        let full_label = format!("{board_label} / {inner_label}");
                        let display_text = if !inner_dev.is_empty() && inner_dev != inner_label {
                            format!("{full_label}  ({inner_dev})")
                        } else {
                            full_label.clone()
                        };
                        let qualified_id = format!("{}/{inner_id}", it.id);
                        out.push(ProgrammableDevice {
                            id: qualified_id.clone(),
                            uid: qualified_id,
                            label: full_label,
                            display_text,
                            kind: inner_dev,
                            is_active: false,
                        });
                    }
                }
            }
            _ => {}
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
        let path_str = path.to_string_lossy().into_owned();
        if let Some(target) = target_device_id {
            let (subc_target, inner_target) = target
                .split_once('/')
                .map(|(s, i)| (s, Some(i)))
                .unwrap_or((target, None));
            for it in &mut self.items {
                if it.id == target {
                    if let Part::Mcu(mcu) = &mut it.kind {
                        if mcu.mcu.load_firmware_file(path).is_ok() {
                            mcu.mcu.firmware = Some(path_str);
                            return true;
                        }
                    } else if let Part::QemuDevice(qemu) = &mut it.kind {
                        qemu.qemu.firmware = path_str;
                        return true;
                    }
                }
                if it.id == subc_target {
                    if let Part::Subcircuit(subc) = &mut it.kind {
                        if subcircuit::has_nested_mcu(&subc.nested_src) {
                            subc.program = path_str.clone();
                            subc.nested_src = subcircuit::set_nested_mcu_program(
                                &subc.nested_src,
                                inner_target,
                                &path_str,
                            );
                            return true;
                        }
                    }
                }
            }
            for c in &mut self.hidden {
                if c.id == target || target.strip_suffix(&c.id).is_some_and(|p| p.ends_with('/')) {
                    if let crate::elements::Kind::Mcu(mcu) = &mut c.kind {
                        if mcu.load_firmware_file(path).is_ok() {
                            mcu.firmware = Some(path_str);
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        for it in &mut self.items {
            if let Part::Mcu(mcu) = &mut it.kind {
                if mcu.mcu.load_firmware_file(path).is_ok() {
                    mcu.mcu.firmware = Some(path_str);
                    return true;
                }
            } else if let Part::Subcircuit(subc) = &mut it.kind {
                if subcircuit::has_nested_mcu(&subc.nested_src) {
                    subc.program = path_str.clone();
                    subc.nested_src =
                        subcircuit::set_nested_mcu_program(&subc.nested_src, None, &path_str);
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
