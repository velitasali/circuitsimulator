//! Instantiate a SubCircuit as one chip: load nested `.circ1`, remap CircIds,
//! and join package pins to inner Tunnels by name.
//!
//! Matches C++ `SubCircuit::construct` / `loadSubCircuit` / `addPin`.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::canvas::{Point, Rect, map_rect};
use crate::catalog::Catalog;
use crate::circ1::{self, BoardPlacement, ParsedCircuit};
use crate::compat::simulide;
use crate::components::Part;
use crate::digital::LogicOverride;
use crate::elements::{Comp, Kind};
use crate::package::{
    Package, SubcType, convert_package, logic_symbol_for_package_name, packages_from_circ1,
    select_package, select_package_key,
};
use crate::xml_util::{parse_bool, parse_xml_props, prop};
use crate::{Error, Result};

const MAX_NEST: u32 = 16;

fn resolve_qemu_firmware(q: &mut crate::qemu::QemuComp, search: &SubcSearch) {
    q.firmware_dir = search.circuit_dir.clone();
    if q.firmware.is_empty() {
        return;
    }
    let p = Path::new(&q.firmware);
    if p.is_absolute() {
        return;
    }
    if let Some(dir) = &search.circuit_dir {
        let abs = dir.join(p);
        if abs.exists() {
            q.firmware = abs.to_string_lossy().into_owned();
        }
    }
}

/// Where to look for `{device}.circ1` / `{device}.package` (or legacy SimulIDE files).
#[derive(Clone, Debug, Default)]
pub struct SubcSearch {
    /// Directory of the parent circuit file.
    pub circuit_dir: Option<PathBuf>,
    /// Extra roots (user data folders).
    pub data_dirs: Vec<PathBuf>,
    /// Tests: device name → nested circuit source.
    pub memory: HashMap<String, String>,
    /// Tests: device name → `.package` XML (used when the circuit has no Package items).
    pub memory_pkg: HashMap<String, String>,
    /// Tests: device name → `.mcu` XML.
    pub memory_mcu: HashMap<String, String>,
    /// Tests: script file name (`cpu.as`) → AngelScript source.
    pub memory_script: HashMap<String, String>,
    /// C++ `ComponentList` itemlib maps (`getDataFile` / `getFileDir`).
    pub catalog: Catalog,
}

impl SubcSearch {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_circuit_path(path: Option<&str>) -> Self {
        let circuit_dir = path.and_then(|p| Path::new(p).parent().map(|d| d.to_path_buf()));
        Self {
            circuit_dir,
            ..Self::default()
        }
    }

    pub fn with_memory(mut self, device: impl Into<String>, src: impl Into<String>) -> Self {
        self.memory.insert(device.into(), src.into());
        self
    }

    pub fn with_memory_mcu(mut self, device: impl Into<String>, xml: impl Into<String>) -> Self {
        self.memory_mcu.insert(device.into(), xml.into());
        self
    }

    pub fn with_memory_script(mut self, file: impl Into<String>, src: impl Into<String>) -> Self {
        self.memory_script.insert(file.into(), src.into());
        self
    }

    pub fn with_memory_pkg(mut self, device: impl Into<String>, xml: impl Into<String>) -> Self {
        self.memory_pkg.insert(device.into(), xml.into());
        self
    }

    pub fn with_catalog(mut self, catalog: Catalog) -> Self {
        self.catalog = catalog;
        self
    }

    pub fn with_standard_catalog(self) -> Self {
        self.with_catalog(crate::catalog::standard())
    }

    pub fn child_for(&self, file: &Path) -> Self {
        let child_dir = file.parent().map(|d| d.to_path_buf());
        let mut data_dirs = self.data_dirs.clone();
        if let Some(parent_dir) = &self.circuit_dir {
            if child_dir.as_ref() != Some(parent_dir) && !data_dirs.contains(parent_dir) {
                data_dirs.push(parent_dir.clone());
            }
        }
        Self {
            circuit_dir: child_dir,
            data_dirs,
            memory: self.memory.clone(),
            memory_pkg: self.memory_pkg.clone(),
            memory_mcu: self.memory_mcu.clone(),
            memory_script: self.memory_script.clone(),
            catalog: self.catalog.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedSubc {
    pub src: String,
    pub path: Option<PathBuf>,
}

pub fn resolve_subc(device: &str, search: &SubcSearch) -> Option<ResolvedSubc> {
    if let Some(src) = search.memory.get(device) {
        return Some(ResolvedSubc {
            src: src.clone(),
            path: search
                .circuit_dir
                .as_ref()
                .map(|d| d.join(format!("{device}.circ1"))),
        });
    }
    let mut dirs = Vec::new();
    if let Some(d) = &search.circuit_dir {
        dirs.push(d.clone());
    }
    for d in &search.data_dirs {
        if !dirs.contains(d) {
            dirs.push(d.clone());
        }
    }
    if let Some(dir) = search.catalog.file_dir(device) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    for dir in dirs {
        for cand in candidate_files(&dir, device) {
            if cand.is_file() {
                if let Ok(src) = std::fs::read_to_string(&cand) {
                    return Some(ResolvedSubc {
                        src,
                        path: Some(cand),
                    });
                }
            }
        }
    }
    None
}

fn candidate_files(dir: &Path, device: &str) -> Vec<PathBuf> {
    let nested = dir.join(device);
    let data = dir.join("data");
    let data_nested = data.join(device);
    let mut out = Vec::with_capacity(simulide::CIRCUIT_EXTENSIONS.len() * 4);
    for ext in simulide::CIRCUIT_EXTENSIONS {
        let file = format!("{device}.{ext}");
        out.push(dir.join(&file));
        out.push(nested.join(&file));
    }
    for ext in simulide::CIRCUIT_EXTENSIONS {
        let file = format!("{device}.{ext}");
        out.push(data.join(&file));
        out.push(data_nested.join(&file));
    }
    out
}

fn sibling_package_files(dir: &Path, device: &str) -> Vec<(String, PathBuf)> {
    vec![
        ("2- DIP".into(), dir.join(format!("{device}.package"))),
        (
            "1- Logic Symbol".into(),
            dir.join(format!("{device}_LS.package")),
        ),
    ]
}

/// C++ `Chip::getDevice`.
pub fn device_from_id(id: &str) -> String {
    let leaf = id.rsplit('/').next().unwrap_or(id);
    let leaf = leaf.rsplit('@').next().unwrap_or(leaf);
    let mut parts: Vec<&str> = leaf.split('-').collect();
    if parts.len() <= 1 {
        return leaf.to_string();
    }
    parts.pop();
    parts.join("-")
}

/// C++ `numId = m_id; numId.remove(m_device + "-")`.
pub fn instance_num_id(id: &str, device: &str) -> String {
    if !device.is_empty() {
        let pat = format!("{device}-");
        if id.contains(&pat) {
            return id.replace(&pat, "");
        }
    }
    if let Some((head, suffix)) = id.rsplit_once('-') {
        if let Some((prefix, _)) = head.rsplit_once('@') {
            return format!("{prefix}@{suffix}");
        }
        return suffix.to_string();
    }
    id.to_string()
}

fn load_packages(
    device: &str,
    src: &str,
    search: &SubcSearch,
    file: Option<&Path>,
) -> BTreeMap<String, Package> {
    let (mut list, _) = packages_from_circ1(src);
    if !list.is_empty() {
        return list;
    }
    if let Some(xml) = search.memory_pkg.get(device) {
        let (_, pkg) = convert_package(xml);
        let mut pkg = pkg;
        if pkg.name.is_empty() {
            pkg.name = device.to_string();
        }
        list.insert("2- DIP".into(), pkg);
        return list;
    }
    let dirs = file
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .into_iter()
        .chain(search.circuit_dir.clone());
    for dir in dirs {
        for (key, path) in sibling_package_files(&dir, device) {
            if path.is_file() {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    let (_, mut pkg) = convert_package(&text);
                    if pkg.name.is_empty() {
                        pkg.name = device.to_string();
                    }
                    list.insert(key, pkg);
                }
            }
        }
        if !list.is_empty() {
            break;
        }
    }
    list
}

/// Flattened inner circuit plus the package used for the chip outline.
#[derive(Clone, Debug)]
pub struct SubcInstance {
    pub components: Vec<Comp>,
    pub connectors: Vec<(String, String)>,
    pub links: Vec<(String, Vec<String>)>,
    pub package: Package,
    pub skipped: Vec<String>,
}

pub fn instantiate(
    instance_id: &str,
    device: &str,
    nested_src: &str,
    search: &SubcSearch,
    logic_symbol: bool,
    package_name: Option<&str>,
    depth: u32,
    logic: Option<LogicOverride>,
) -> Result<SubcInstance> {
    instantiate_resolved(
        instance_id,
        device,
        nested_src,
        None,
        search,
        logic_symbol,
        package_name,
        depth,
        logic,
    )
}

fn instantiate_resolved(
    instance_id: &str,
    device: &str,
    nested_src: &str,
    nested_path: Option<&Path>,
    search: &SubcSearch,
    logic_symbol: bool,
    package_name: Option<&str>,
    depth: u32,
    logic: Option<LogicOverride>,
) -> Result<SubcInstance> {
    if depth >= MAX_NEST {
        return Err(Error::Parse(format!(
            "subcircuit nest limit ({MAX_NEST}) at {instance_id}"
        )));
    }
    let packages = load_packages(device, nested_src, search, nested_path);
    let Some(package) = select_package(&packages, logic_symbol, package_name).cloned() else {
        return Err(Error::Parse(format!("no packages for subcircuit {device}")));
    };

    let num_id = instance_num_id(instance_id, device);
    let prefixed = prefix_nested_doc(nested_src, &num_id, instance_id);
    let parsed = circ1::to_parsed_circuit(circ1::load_circ1(&prefixed)?);
    let child_search = nested_path
        .map(|p| search.child_for(p))
        .unwrap_or_else(|| search.clone());
    // LogicSubc pushes only when this device is a logic subcircuit.
    let logic = if package.subc_type == SubcType::Logic {
        logic
    } else {
        None
    };
    let expanded = expand_parsed(parsed, &child_search, depth + 1, logic)?;

    let mut components = expanded.components;
    if let Some(parent_dir) = search.circuit_dir.as_deref() {
        for comp in &mut components {
            if let Kind::Mcu(mcu) = &mut comp.kind {
                let missing = mcu.firmware_path().is_none_or(|p| !p.is_file());
                if missing {
                    if let Some(fw) = mcu.firmware.as_deref() {
                        if let Some(resolved_fw) =
                            crate::mcu::resolve_firmware(Some(parent_dir), fw)
                        {
                            if resolved_fw.is_file() {
                                mcu.firmware_dir = Some(parent_dir.to_path_buf());
                                let _ = mcu.load_firmware_file(&resolved_fw);
                            }
                        }
                    }
                }
            }
        }
    }
    let connectors = expanded.connectors;
    let links = expanded.links;
    let skipped = expanded.skipped;

    for pin in package.electrical_pins() {
        let pin_id = format!("{instance_id}-{}", pin.id);
        let name = pin_id.clone();
        components.push(Comp::tunnel(pin_id.clone(), name, pin_id));
    }

    Ok(SubcInstance {
        components,
        connectors,
        links,
        package,
        skipped,
    })
}

pub struct Expanded {
    pub components: Vec<Comp>,
    pub connectors: Vec<(String, String)>,
    pub links: Vec<(String, Vec<String>)>,
    pub skipped: Vec<String>,
}

/// Walk a parsed document, flattening every Subcircuit item.
pub fn expand_parsed(
    parsed: ParsedCircuit,
    search: &SubcSearch,
    depth: u32,
    logic: Option<LogicOverride>,
) -> Result<Expanded> {
    let mut components = Vec::new();
    let mut connectors: Vec<(String, String)> = parsed
        .connectors
        .into_iter()
        .map(|c| (c.start, c.end))
        .collect();
    let mut links: Vec<(String, Vec<String>)> = Vec::new();
    let mut skipped = parsed.skipped;
    for item in parsed.items {
        let item_links = item.links;
        match item.comp.kind {
            Kind::McuItem(ref spec) => {
                let comp_id = item.comp.id.clone();
                match crate::mcu::instantiate_any(&item.comp.id, spec, search) {
                    Ok(crate::mcu::InstantiatedMcu::Native(mcu)) => {
                        components.push(Comp {
                            id: item.comp.id,
                            kind: Kind::Mcu(mcu),
                        });
                        if !item_links.is_empty() {
                            links.push((comp_id, item_links));
                        }
                    }
                    Ok(crate::mcu::InstantiatedMcu::Script(cpu)) => {
                        components.push(Comp::script_cpu(item.comp.id, cpu));
                        if !item_links.is_empty() {
                            links.push((comp_id, item_links));
                        }
                    }
                    Err(e) => skipped.push(format!("MCU:{} ({e})", item.comp.id)),
                }
            }
            Kind::Subcircuit {
                ref device,
                logic_symbol,
                ref package_name,
                ref exposed_overrides,
            } => {
                let Some(resolved) = resolve_subc(device, search) else {
                    skipped.push(format!("Subcircuit:{device}"));
                    continue;
                };
                let nested_src: Cow<'_, str> = if exposed_overrides.is_empty() {
                    Cow::Borrowed(&resolved.src)
                } else {
                    Cow::Owned(apply_exposed_overrides(&resolved.src, exposed_overrides))
                };
                match instantiate_resolved(
                    &item.comp.id,
                    device,
                    &nested_src,
                    resolved.path.as_deref(),
                    search,
                    logic_symbol,
                    package_name.as_deref(),
                    depth,
                    logic,
                ) {
                    Ok(inst) => {
                        components.extend(inst.components);
                        connectors.extend(inst.connectors);
                        links.extend(inst.links);
                        skipped.extend(inst.skipped);
                    }
                    Err(e) => skipped.push(format!("Subcircuit:{} ({e})", item.comp.id)),
                }
            }
            _ => {
                let mut comp = item.comp;
                if let Kind::QemuDevice(q) = &mut comp.kind {
                    resolve_qemu_firmware(q, search);
                }
                if let Some(levels) = logic {
                    apply_logic_override(&mut comp, levels);
                }
                if !item_links.is_empty() {
                    links.push((comp.id.clone(), item_links));
                }
                components.push(comp);
            }
        }
    }
    Ok(Expanded {
        components,
        connectors,
        links,
        skipped,
    })
}

/// `LogicSubc` setter: `setPropStr` on each direct child. Parts without a logic
/// family ignore it. A nested logic subcircuit receives the same override and
/// pushes it onward. Any other nested subcircuit does not.
fn apply_logic_override(comp: &mut Comp, levels: LogicOverride) {
    match &mut comp.kind {
        Kind::Gate(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::FlipFlop(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Latch(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Comparator { state } => {
            levels.apply(&mut state.family);
            state.apply_electric();
        }
        Kind::TestUnit(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Mux(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Demux(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::BcdToDec(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::DecToBcd(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::BcdTo7S(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::I2CToParallel(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Adc(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Dac(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Counter(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::BinCounter(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::FullAdder(s) | Kind::HalfAdder(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::MagnitudeComp(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::ShiftReg(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Function(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::Memory(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::DynamicMemory(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        Kind::I2CRam(s) => {
            levels.apply(&mut s.family);
            s.apply_family();
        }
        _ => {}
    }
}

/// Prefix inner CircIds / connector pin ids / tunnel names (C++ `numId+"@"+uid`).
pub fn prefix_nested_doc(src: &str, num_id: &str, instance_id: &str) -> String {
    src.lines()
        .map(|line| prefix_item_line(line, num_id, instance_id))
        .collect::<Vec<_>>()
        .join("\n")
}

fn prefix_item_line(line: &str, num_id: &str, instance_id: &str) -> String {
    let trimmed = line.trim();
    if !trimmed.starts_with("<item") {
        return line.to_string();
    }
    let mut properties = parse_xml_props(trimmed);
    if properties.is_empty() {
        return line.to_string();
    }
    let typ = if properties[0].0 == "itemtype" {
        properties[0].1.clone()
    } else {
        return line.to_string();
    };
    if typ == "Package" || typ == "SubPackage" {
        return line.to_string();
    }
    let prefix_val = |v: &str| {
        if v.starts_with(&format!("{num_id}@")) {
            v.to_string()
        } else {
            format!("{num_id}@{v}")
        }
    };
    if typ == "Connector" {
        for (k, v) in properties.iter_mut() {
            if k == "startpinid" || k == "endpinid" || k == "uid" || k == "CircId" {
                *v = prefix_val(v);
            }
        }
    } else {
        let has_circ_id = properties.iter().any(|(k, _)| k == "CircId");
        for (k, v) in properties.iter_mut() {
            if k == "CircId" || (!has_circ_id && k == "uid") {
                *v = prefix_val(v);
            }
            if k == "Links" && !v.is_empty() {
                let prefixed: Vec<String> = v
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(&prefix_val)
                    .collect();
                if !prefixed.is_empty() {
                    *v = format!("{},", prefixed.join(","));
                }
            }
            if typ == "Tunnel" && k == "Name" && !v.is_empty() {
                let pfx = format!("{instance_id}-");
                if !v.starts_with(&pfx) {
                    *v = format!("{pfx}{v}");
                }
            }
        }
    }
    rebuild_item(&properties)
}

fn rebuild_item(properties: &[(String, String)]) -> String {
    let mut s = String::from("<item");
    for (k, v) in properties {
        s.push(' ');
        s.push_str(k);
        s.push_str("=\"");
        s.push_str(v);
        s.push('"');
    }
    s.push_str(" />");
    s
}

fn is_nested_mcu_type(typ: &str) -> bool {
    matches!(typ, "MCU" | "Mcu" | "QemuDevice")
}

/// Returns `true` if `src` contains at least one nested MCU or QemuDevice item.
pub fn has_nested_mcu(src: &str) -> bool {
    for line in src.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<item") {
            continue;
        }
        let properties = parse_xml_props(trimmed);
        if let Some(typ) = properties
            .first()
            .filter(|p| p.0 == "itemtype")
            .map(|p| p.1.as_str())
        {
            if is_nested_mcu_type(typ) {
                return true;
            }
        }
    }
    false
}

fn is_exposed_attr(properties: &[(String, String)]) -> bool {
    prop(properties, "mainComp")
        .or_else(|| prop(properties, "MainComp"))
        .map(parse_bool)
        .unwrap_or(false)
}

/// Returns the `Program` / `program` attribute of the exposed (`mainComp="true"`) nested
/// MCU/QemuDevice if present, falling back to the first nested MCU/QemuDevice with a non-empty program.
pub fn nested_mcu_program(src: &str) -> Option<String> {
    let mut fallback = None;
    for line in src.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<item") {
            continue;
        }
        let properties = parse_xml_props(trimmed);
        let Some(typ) = properties
            .first()
            .filter(|p| p.0 == "itemtype")
            .map(|p| p.1.as_str())
        else {
            continue;
        };
        if !is_nested_mcu_type(typ) {
            continue;
        }
        if let Some(prog) = prop(&properties, "Program")
            .or_else(|| prop(&properties, "program"))
            .filter(|s| !s.is_empty())
        {
            if is_exposed_attr(&properties) {
                return Some(prog.to_string());
            }
            if fallback.is_none() {
                fallback = Some(prog.to_string());
            }
        }
    }
    fallback
}

/// Updates the `Program` attribute on a matching (or exposed `mainComp="true"`, else first,
/// if `target_inner_id` is `None`) nested MCU or QemuDevice `<item>` line in `src`.
pub fn set_nested_mcu_program(src: &str, target_inner_id: Option<&str>, program: &str) -> String {
    let preferred_id = if target_inner_id.is_some() {
        target_inner_id.map(str::to_string)
    } else {
        let mut first_mcu = None;
        let mut exposed_mcu = None;
        for line in src.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("<item") {
                continue;
            }
            let properties = parse_xml_props(trimmed);
            let Some(typ) = properties
                .first()
                .filter(|p| p.0 == "itemtype")
                .map(|p| p.1.as_str())
            else {
                continue;
            };
            if !is_nested_mcu_type(typ) {
                continue;
            }
            if let Some(cid) = prop(&properties, "CircId") {
                if first_mcu.is_none() {
                    first_mcu = Some(cid.to_string());
                }
                if is_exposed_attr(&properties) && exposed_mcu.is_none() {
                    exposed_mcu = Some(cid.to_string());
                }
            }
        }
        exposed_mcu.or(first_mcu)
    };

    let mut updated = false;
    src.lines()
        .map(|line| {
            if updated {
                return line.to_string();
            }
            let trimmed = line.trim();
            if !trimmed.starts_with("<item") {
                return line.to_string();
            }
            let mut properties = parse_xml_props(trimmed);
            let Some(typ) = properties
                .first()
                .filter(|p| p.0 == "itemtype")
                .map(|p| p.1.as_str())
            else {
                return line.to_string();
            };
            if !is_nested_mcu_type(typ) {
                return line.to_string();
            }
            if let Some(target) = preferred_id.as_deref() {
                if prop(&properties, "CircId") != Some(target) {
                    return line.to_string();
                }
            }
            if let Some((_, val)) = properties.iter_mut().find(|(k, _)| k == "Program") {
                *val = program.to_string();
            } else if let Some((_, val)) = properties.iter_mut().find(|(k, _)| k == "program") {
                *val = program.to_string();
            } else {
                properties.push(("Program".to_string(), program.to_string()));
            }
            updated = true;
            rebuild_item(&properties)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// An inner component marked `mainComp="true"` inside a subcircuit document.
#[derive(Clone, Debug)]
pub struct ExposedItem {
    pub circ_id: String,
    pub label: String,
    pub part: Part,
}

/// A visual/interactive inner component mounted on a Board/Shield subcircuit's package face.
#[derive(Clone, Debug)]
pub struct BoardChild {
    /// Inner `CircId` (e.g. `"Led-1"`). Combined with `num_id@` in the flattened simulation.
    pub inner_id: String,
    /// Placement relative to the `Subcircuit`'s package top-left `(0, 0)`.
    pub placement: BoardPlacement,
    /// Component visual/property state (cloned from `nested_src` and kept in sync during sim).
    pub part: Box<Part>,
}

impl PartialEq for BoardChild {
    fn eq(&self, other: &Self) -> bool {
        self.inner_id == other.inner_id
            && self.placement == other.placement
            && self.part.type_name() == other.part.type_name()
    }
}

/// Extract inner components placed on the face of a Board/Shield `package`.
///
/// Only active when `package.subc_type` is `Board` or `Shield` and `logic_symbol` is `false`.
/// Any non-structural item whose board-space body intersects `(0, 0, body_w, body_h)` is returned
/// in document order.
pub fn board_children(src: &str, package: &Package, logic_symbol: bool) -> Vec<BoardChild> {
    if src.is_empty()
        || logic_symbol
        || !matches!(package.subc_type, SubcType::Board | SubcType::Shield)
    {
        return Vec::new();
    }
    let Ok(parsed) = circ1::load_circ1(src) else {
        return Vec::new();
    };
    let pkg_rect = Rect::new(0.0, 0.0, package.body_w(), package.body_h());
    parsed
        .items
        .into_iter()
        .filter_map(|it| {
            if matches!(
                it.part,
                Part::SubPackage(_) | Part::Node(_) | Part::Tunnel(_) | Part::Bus(_)
            ) {
                return None;
            }
            let placement = it.graphic.board?;
            let child_rect = map_rect(
                Point::new(placement.x, placement.y),
                it.part.body(),
                placement.rotation,
                placement.hflip,
                placement.vflip,
            );
            if !pkg_rect.intersects(&child_rect) {
                return None;
            }
            Some(BoardChild {
                inner_id: it.circ_id,
                placement,
                part: Box::new(it.part),
            })
        })
        .collect()
}

/// Compute all simulation pin IDs belonging to `part` under `sim_id`.
pub fn part_pin_ids(sim_id: &str, part: &Part) -> Vec<String> {
    if let Some(pkg) = part.package() {
        let hide_nul = !matches!(part, Part::SubPackage(_));
        return pkg
            .pins
            .iter()
            .filter(|p| !(hide_nul && p.is_nul()))
            .map(|p| format!("{sim_id}-{}", p.id))
            .collect();
    }
    part.pin_geoms()
        .into_iter()
        .map(|cp| {
            if cp.suffix.starts_with('-') {
                format!("{sim_id}{}", cp.suffix)
            } else {
                format!("{sim_id}-{}", cp.suffix)
            }
        })
        .collect()
}

/// Parse all components flagged `mainComp="true"` inside a subcircuit `.circ1` source.
pub fn exposed_items(src: &str) -> Vec<ExposedItem> {
    if src.is_empty() {
        return Vec::new();
    }
    let Ok(parsed) = circ1::load_circ1(src) else {
        return Vec::new();
    };
    parsed
        .items
        .into_iter()
        .filter(|it| {
            it.graphic.main_comp
                || matches!(&it.part, crate::components::Part::Mcu(m) if m.is_main_comp)
        })
        .filter(|it| {
            !matches!(
                it.part,
                crate::components::Part::SubPackage(_) | crate::components::Part::Node(_)
            )
        })
        .map(|it| ExposedItem {
            label: it.graphic.label.unwrap_or_else(|| it.circ_id.clone()),
            circ_id: it.circ_id,
            part: it.part,
        })
        .collect()
}

/// Result of applying a single exposed-component property edit to a subcircuit document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExposedPropUpdate {
    /// Updated nested `.circ1` document text.
    pub updated_src: String,
    /// Persisted `(prop_id, formatted_value)` entries modified by this edit (including
    /// any coupled `PropUpdate` targets such as `Diode::Threshold -> SatCurrent`).
    pub persisted_changes: Vec<(String, String)>,
}

fn escape_override_token(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '"' => out.push_str("%22"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            ';' => out.push_str("%3B"),
            '=' => out.push_str("%3D"),
            '&' => out.push_str("%26"),
            other => out.push(other),
        }
    }
    out
}

fn unescape_override_token(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// Encode per-instance exposed-component property overrides into a single XML-safe attribute value.
pub fn encode_exposed_overrides(overrides: &BTreeMap<(String, String), String>) -> String {
    overrides
        .iter()
        .map(|((inner_id, prop_name), val)| {
            format!(
                "{};{}={}",
                escape_override_token(inner_id),
                escape_override_token(prop_name),
                escape_override_token(val),
            )
        })
        .collect::<Vec<_>>()
        .join("&#xa;")
}

/// Decode per-instance exposed-component property overrides from the `ExposedProps` attribute.
pub fn decode_exposed_overrides(raw: &str) -> BTreeMap<(String, String), String> {
    let mut out = BTreeMap::new();
    for entry in raw.split("&#xa;") {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Some((lhs, val)) = entry.split_once('=') else {
            continue;
        };
        let Some((inner_id, prop_name)) = lhs.split_once(';') else {
            continue;
        };
        let inner_id = unescape_override_token(inner_id);
        let prop_name = unescape_override_token(prop_name);
        let val = unescape_override_token(val);
        if !inner_id.is_empty() && !prop_name.is_empty() {
            out.insert((inner_id, prop_name), val);
        }
    }
    out
}

/// Re-apply all saved per-instance exposed-component property overrides onto a subcircuit source.
/// Entries whose target item or property no longer exists are ignored gracefully.
pub fn apply_exposed_overrides(
    src: &str,
    overrides: &BTreeMap<(String, String), String>,
) -> String {
    if src.is_empty() || overrides.is_empty() {
        return src.to_string();
    }
    let mut out = src.to_string();
    for ((inner_id, prop_name), val) in overrides {
        if let Some(updated) = apply_exposed_item_prop(&out, inner_id, prop_name, val) {
            out = updated.updated_src;
        }
    }
    out
}

fn escape_xml_attr(s: &str) -> String {
    s.replace('\r', "")
        .replace('\n', "&#xa;")
        .replace('"', "&quot;")
}

fn attr_matches_prop(attr_key: &str, prop_name: &str) -> bool {
    if attr_key == prop_name || attr_key.eq_ignore_ascii_case(prop_name) {
        return true;
    }
    let clean_prop = prop_name.replace('_', "");
    if attr_key.replace('_', "").eq_ignore_ascii_case(&clean_prop) {
        return true;
    }
    matches!(
        (attr_key, prop_name),
        ("Freq", "Frequency")
            | ("Inverted", "InvertOutputs")
            | ("DT", "DoubleThrow")
            | ("Size", "Segments")
    )
}

/// Update a property on a specific inner item (by `CircId` or legacy `uid`) inside a subcircuit
/// `.circ1` source. Returns both the updated document and the persisted property changes.
pub fn apply_exposed_item_prop(
    src: &str,
    target_inner_id: &str,
    prop_name: &str,
    value_text: &str,
) -> Option<ExposedPropUpdate> {
    // Parse with circ1 (or SimulIDE compat converter) to validate & normalize via the component's own PropDef.
    let parsed = circ1::load_circ1(src).ok()?;
    let mut item = parsed
        .items
        .into_iter()
        .find(|it| it.circ_id == target_inner_id)?;

    let before: HashMap<&'static str, String> = item.part.persisted_props().into_iter().collect();
    item.part.set_prop_text(prop_name, value_text).ok()?;
    let after = item.part.persisted_props();

    let clean_prop = prop_name.replace('_', "");
    let mut persisted_changes: Vec<(String, String)> = Vec::new();
    for (k, v) in &after {
        if before.get(k) != Some(v) {
            persisted_changes.push(((*k).to_string(), v.clone()));
        }
    }
    // Ensure explicitly-set persisted properties are always recorded even if equal to current value
    // (e.g. when re-applying overrides or explicitly pinning a value).
    if let Some((k, v)) = after.iter().find(|(k, _)| {
        *k == prop_name
            || k.eq_ignore_ascii_case(prop_name)
            || k.replace('_', "").eq_ignore_ascii_case(&clean_prop)
    }) {
        if !persisted_changes.iter().any(|(ck, _)| ck == k) {
            persisted_changes.push(((*k).to_string(), v.clone()));
        }
    }

    let patches: Vec<(String, String)> = if !persisted_changes.is_empty() {
        persisted_changes.clone()
    } else {
        let formatted = item
            .part
            .get_prop_text(prop_name)
            .unwrap_or_else(|| value_text.to_string());
        vec![(prop_name.to_string(), formatted)]
    };

    let mut updated = false;
    let out = src
        .lines()
        .map(|line| {
            if updated {
                return line.to_string();
            }
            let trimmed = line.trim();
            if !trimmed.starts_with("<item") {
                return line.to_string();
            }
            let mut properties = parse_xml_props(trimmed);
            let line_id = prop(&properties, "CircId").or_else(|| prop(&properties, "uid"));
            if line_id != Some(target_inner_id) {
                return line.to_string();
            }
            for (k, v) in &patches {
                let escaped = escape_xml_attr(v);
                if let Some((_, val)) = properties
                    .iter_mut()
                    .find(|(ak, _)| attr_matches_prop(ak, k))
                {
                    *val = escaped;
                } else {
                    properties.push((k.clone(), escaped));
                }
            }
            updated = true;
            rebuild_item(&properties)
        })
        .collect::<Vec<_>>()
        .join("\n");
    updated.then_some(ExposedPropUpdate {
        updated_src: out,
        persisted_changes,
    })
}

/// Update a property on a specific inner item (by `CircId` or legacy `uid`) inside a subcircuit `.circ1` source.
/// Returns the updated document string if the item and property were successfully updated.
pub fn set_exposed_item_prop(
    src: &str,
    target_inner_id: &str,
    prop_name: &str,
    value_text: &str,
) -> Option<String> {
    apply_exposed_item_prop(src, target_inner_id, prop_name, value_text).map(|u| u.updated_src)
}

/// Layout used by the canvas chip, plus nested source for later flatten.
#[derive(Clone, Debug)]
pub struct SubcView {
    pub device: String,
    pub package: Package,
    pub packages: BTreeMap<String, Package>,
    pub nested_src: String,
    pub nested_path: Option<String>,
    pub logic_symbol: bool,
}

pub fn load_view(
    instance_id: &str,
    device: &str,
    search: &SubcSearch,
    logic_symbol: bool,
    package_name: Option<&str>,
) -> Result<SubcView> {
    let _ = instance_id;
    let Some(resolved) = resolve_subc(device, search) else {
        return Err(Error::Parse(format!("subcircuit {device} not found")));
    };
    let packages = load_packages(device, &resolved.src, search, resolved.path.as_deref());
    let Some((key, pkg)) = select_package_key(&packages, logic_symbol, package_name) else {
        return Err(Error::Parse(format!("no packages for subcircuit {device}")));
    };
    let package = pkg.clone();
    let effective_logic_symbol = if packages.len() == 1 {
        if matches!(package.subc_type, SubcType::Board | SubcType::Shield) {
            package.logic_symbol
        } else if !logic_symbol {
            logic_symbol_for_package_name(key, package.logic_symbol)
        } else {
            logic_symbol
        }
    } else {
        logic_symbol
    };
    Ok(SubcView {
        device: device.to_string(),
        package,
        packages,
        nested_src: resolved.src,
        nested_path: resolved.path.map(|p| p.to_string_lossy().into_owned()),
        logic_symbol: effective_logic_symbol,
    })
}

/// A node in the subcircuit tree hierarchy snapshot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubcTreeNode {
    pub label: String,
    pub file_path: String,
    pub children: Vec<SubcTreeNode>,
}

pub(crate) fn subc_visit_key(file_path: &str, device: &str) -> String {
    if !file_path.is_empty() {
        let norm = file_path.replace('\\', "/");
        #[cfg(windows)]
        {
            norm.to_ascii_lowercase()
        }
        #[cfg(not(windows))]
        {
            norm
        }
    } else {
        format!("device:{device}")
    }
}

/// Snapshot subcircuit tree from circuit source text.
pub fn snapshot_subcircuits_from_src(src: &str, search: &SubcSearch) -> Vec<SubcTreeNode> {
    let mut visited = HashSet::new();
    snapshot_subcircuits_from_src_inner(src, search, 0, &mut visited)
}

pub(crate) fn snapshot_subcircuits_from_src_with_visited(
    src: &str,
    search: &SubcSearch,
    depth: u32,
    visited: &mut HashSet<String>,
) -> Vec<SubcTreeNode> {
    snapshot_subcircuits_from_src_inner(src, search, depth, visited)
}

#[inline]
pub(crate) fn contains_ascii_ignore_case(haystack: &[u8], needle_lower: &[u8]) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    haystack
        .windows(needle_lower.len())
        .any(|w| w.eq_ignore_ascii_case(needle_lower))
}

#[inline]
pub(crate) fn may_contain_subcircuit(src: &str) -> bool {
    let b = src.as_bytes();
    contains_ascii_ignore_case(b, b"subcircuit") || contains_ascii_ignore_case(b, b"sub_circuit")
}

#[inline]
pub(crate) fn may_contain_programmable_device(src: &str) -> bool {
    let b = src.as_bytes();
    contains_ascii_ignore_case(b, b"mcu")
        || contains_ascii_ignore_case(b, b"micro")
        || contains_ascii_ignore_case(b, b"qemu")
}

fn snapshot_subcircuits_from_src_inner(
    src: &str,
    search: &SubcSearch,
    depth: u32,
    visited: &mut HashSet<String>,
) -> Vec<SubcTreeNode> {
    if depth >= MAX_NEST || !may_contain_subcircuit(src) {
        return Vec::new();
    }
    let Ok(parsed) = circ1::load_circ1(src) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for it in &parsed.items {
        if let Part::Subcircuit(subc) = &it.part {
            let dev = if !subc.device.is_empty() {
                subc.device.clone()
            } else {
                device_from_id(&it.circ_id)
            };
            let label = it
                .graphic
                .label
                .as_deref()
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    if !it.circ_id.is_empty() {
                        it.circ_id.clone()
                    } else {
                        dev.clone()
                    }
                });
            let resolved = resolve_subc(&dev, search);
            let file_path = resolved
                .as_ref()
                .and_then(|r| r.path.as_ref())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            let key = subc_visit_key(&file_path, &dev);
            let children = if let Some(r) = resolved {
                if visited.insert(key.clone()) {
                    let child_search = if !file_path.is_empty() {
                        search.child_for(Path::new(&file_path))
                    } else {
                        search.clone()
                    };
                    let kids = snapshot_subcircuits_from_src_inner(
                        &r.src,
                        &child_search,
                        depth + 1,
                        visited,
                    );
                    visited.remove(&key);
                    kids
                } else {
                    Vec::new()
                }
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

/// Snapshot subcircuit tree from parsed circuit items.
pub fn snapshot_subcircuits_from_parsed(
    parsed: &ParsedCircuit,
    search: &SubcSearch,
) -> Vec<SubcTreeNode> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    for it in &parsed.items {
        if let Kind::Subcircuit { device, .. } = &it.comp.kind {
            let dev = if !device.is_empty() {
                device.clone()
            } else {
                device_from_id(&it.comp.id)
            };
            let label = it
                .label
                .as_ref()
                .filter(|l| !l.is_empty())
                .cloned()
                .unwrap_or_else(|| {
                    if !it.comp.id.is_empty() {
                        it.comp.id.clone()
                    } else {
                        dev.clone()
                    }
                });
            let resolved = resolve_subc(&dev, search);
            let file_path = resolved
                .as_ref()
                .and_then(|r| r.path.as_ref())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            let key = subc_visit_key(&file_path, &dev);
            let children = if let Some(r) = resolved {
                if visited.insert(key.clone()) {
                    let child_search = if !file_path.is_empty() {
                        search.child_for(Path::new(&file_path))
                    } else {
                        search.clone()
                    };
                    let kids =
                        snapshot_subcircuits_from_src_inner(&r.src, &child_search, 1, &mut visited);
                    visited.remove(&key);
                    kids
                } else {
                    Vec::new()
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_from_plain_and_nested_ids() {
        assert_eq!(device_from_id("vdiv-1"), "vdiv");
        assert_eq!(device_from_id("74HC00-12"), "74HC00");
        assert_eq!(device_from_id("1@74HC00-2"), "74HC00");
        assert_eq!(device_from_id("Arduino-1@mega328-2"), "mega328");
        assert_eq!(device_from_id("Uno-1/mega328-109"), "mega328");
        assert_eq!(instance_num_id("vdiv-1", "vdiv"), "1");
        assert_eq!(instance_num_id("1@74HC00-2", "74HC00"), "1@2");
        assert_eq!(instance_num_id("Arduino-1", "Arduino_Uno"), "1");
        assert_eq!(instance_num_id("1@Arduino-2", "Arduino_Uno"), "1@2");
    }

    #[test]
    fn nested_mcu_program_helpers() {
        let plain = "<circuit>\n<item itemtype=\"Resistor\" CircId=\"R-1\" />\n</circuit>";
        assert!(!has_nested_mcu(plain));
        assert_eq!(nested_mcu_program(plain), None);

        let board = r#"<circuit>
<item itemtype="Package" CircId="Package-1" />
<item itemtype="MCU" CircId="mega328-109" Program="" />
<item itemtype="MCU" CircId="tiny85-2" Program="aux.hex" />
</circuit>"#;
        assert!(has_nested_mcu(board));
        assert_eq!(nested_mcu_program(board).as_deref(), Some("aux.hex"));

        let updated = set_nested_mcu_program(board, Some("mega328-109"), "blink.hex");
        assert_eq!(nested_mcu_program(&updated).as_deref(), Some("blink.hex"));
        assert!(updated.contains(r#"CircId="mega328-109" Program="blink.hex""#));
        assert!(updated.contains(r#"CircId="tiny85-2" Program="aux.hex""#));

        let no_prog_attr = "<circuit>\n<item itemtype=\"Mcu\" CircId=\"mega328-1\" />\n</circuit>";
        let inserted = set_nested_mcu_program(no_prog_attr, None, "fw.hex");
        assert_eq!(nested_mcu_program(&inserted).as_deref(), Some("fw.hex"));
    }

    #[test]
    fn prefix_rewrites_circid_and_tunnel_name() {
        let src = r#"<item itemtype="Tunnel" CircId="Tunnel-1" Name="in" />
<item itemtype="Connector" uid="connector-1" startpinid="Tunnel-1-pin" endpinid="Resistor-1-lPin" />
<item itemtype="Package" CircId="Package-1" label="DIP" />"#;
        let out = prefix_nested_doc(src, "1", "vdiv-1");
        assert!(out.contains("CircId=\"1@Tunnel-1\""));
        assert!(out.contains("Name=\"vdiv-1-in\""));
        assert!(out.contains("startpinid=\"1@Tunnel-1-pin\""));
        assert!(out.contains("itemtype=\"Package\" CircId=\"Package-1\""));
    }

    #[test]
    fn test_snapshot_subcircuits_tree() {
        let root_sim1 = r#"<circuit version="2.0.0">
<item itemtype="Subcircuit" CircId="Sub1" label="Subcircuit Alpha" device="sub_a" />
<item itemtype="Subcircuit" CircId="Sub2" label="Subcircuit Beta" device="sub_b" />
</circuit>"#;

        let sub_a_src = r#"<circuit version="2.0.0">
<item itemtype="Subcircuit" CircId="Child1" label="Child Gamma" device="sub_c" />
</circuit>"#;

        let sub_b_src = r#"<circuit version="2.0.0">
<item itemtype="Resistor" CircId="R1" />
</circuit>"#;

        let sub_c_src = r#"<circuit version="2.0.0">
<item itemtype="Capacitor" CircId="C1" />
</circuit>"#;

        let mut search = SubcSearch::empty();
        search
            .memory
            .insert("sub_a".to_string(), sub_a_src.to_string());
        search
            .memory
            .insert("sub_b".to_string(), sub_b_src.to_string());
        search
            .memory
            .insert("sub_c".to_string(), sub_c_src.to_string());

        let parsed = circ1::to_parsed_circuit(circ1::load_circ1(root_sim1).unwrap());
        let tree = snapshot_subcircuits_from_parsed(&parsed, &search);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].label, "Subcircuit Alpha");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].label, "Child Gamma");
        assert_eq!(tree[1].label, "Subcircuit Beta");
        assert_eq!(tree[1].children.len(), 0);
    }
}
