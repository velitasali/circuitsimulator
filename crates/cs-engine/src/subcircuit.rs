//! Instantiate a SubCircuit as one chip: load nested `.sim1`, remap CircIds,
//! and join package pins to inner Tunnels by name.
//!
//! Matches C++ `SubCircuit::construct` / `loadSubCircuit` / `addPin`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::catalog::Catalog;
use crate::elements::{Comp, Kind};
use crate::package::{Package, convert_package, packages_from_sim1, select_package};
use crate::sim1::{ParsedCircuit, parse_sim1, parse_xml_props};
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

/// Where to look for `{device}.sim1` / `{device}.package`.
#[derive(Clone, Debug, Default)]
pub struct SubcSearch {
    /// Directory of the parent circuit file.
    pub circuit_dir: Option<PathBuf>,
    /// Extra roots (user data folders).
    pub data_dirs: Vec<PathBuf>,
    /// Tests: device name → nested `.sim1` source.
    pub memory: HashMap<String, String>,
    /// Tests: device name → `.package` XML (used when the sim1 has no Package items).
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
        Self {
            circuit_dir: file.parent().map(|d| d.to_path_buf()),
            data_dirs: self.data_dirs.clone(),
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
                .map(|d| d.join(format!("{device}.sim1"))),
        });
    }
    let mut dirs = Vec::new();
    if let Some(d) = &search.circuit_dir {
        dirs.push(d.clone());
    }
    dirs.extend(search.data_dirs.iter().cloned());
    if let Some(dir) = search.catalog.file_dir(device) {
        dirs.push(dir);
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
    vec![
        dir.join(format!("{device}.circ1")),
        nested.join(format!("{device}.circ1")),
        dir.join(format!("{device}.sim2")),
        nested.join(format!("{device}.sim2")),
        dir.join(format!("{device}.sim1")),
        nested.join(format!("{device}.sim1")),
        dir.join("data").join(format!("{device}.circ1")),
        dir.join("data")
            .join(device)
            .join(format!("{device}.circ1")),
        dir.join("data").join(format!("{device}.sim2")),
        dir.join("data").join(device).join(format!("{device}.sim2")),
        dir.join("data").join(format!("{device}.sim1")),
        dir.join("data").join(device).join(format!("{device}.sim1")),
    ]
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
    let mut parts: Vec<&str> = id.split('-').collect();
    if parts.len() <= 1 {
        return id.to_string();
    }
    parts.pop();
    let device = parts.join("-");
    if let Some((head, tail)) = device.split_once('@') {
        if head.parse::<i64>().is_ok() {
            return tail.rsplit('@').next().unwrap_or(tail).to_string();
        }
    }
    device
}

/// C++ `numId = m_id; numId.remove(m_device + "-")`.
pub fn instance_num_id(id: &str, device: &str) -> String {
    id.replace(&format!("{device}-"), "")
}

fn load_packages(
    device: &str,
    src: &str,
    search: &SubcSearch,
    file: Option<&Path>,
) -> BTreeMap<String, Package> {
    let (mut list, _) = packages_from_sim1(src);
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
    let parsed = parse_sim1(&prefixed)?;
    let child_search = nested_path
        .map(|p| search.child_for(p))
        .unwrap_or_else(|| search.clone());
    let expanded = expand_parsed(parsed, &child_search, depth + 1)?;

    let mut components = expanded.components;
    let connectors = expanded.connectors;
    let skipped = expanded.skipped;

    for pin in package.electrical_pins() {
        let pin_id = format!("{instance_id}-{}", pin.id);
        let name = pin_id.clone();
        components.push(Comp::tunnel(pin_id.clone(), name, pin_id));
    }

    Ok(SubcInstance {
        components,
        connectors,
        package,
        skipped,
    })
}

pub struct Expanded {
    pub components: Vec<Comp>,
    pub connectors: Vec<(String, String)>,
    pub skipped: Vec<String>,
}

/// Walk a parsed document, flattening every Subcircuit item.
pub fn expand_parsed(parsed: ParsedCircuit, search: &SubcSearch, depth: u32) -> Result<Expanded> {
    let mut components = Vec::new();
    let mut connectors: Vec<(String, String)> = parsed
        .connectors
        .into_iter()
        .map(|c| (c.start, c.end))
        .collect();
    let mut skipped = parsed.skipped;
    for item in parsed.items {
        match item.comp.kind {
            Kind::McuItem(ref spec) => {
                match crate::mcu::instantiate_any(&item.comp.id, spec, search) {
                    Ok(crate::mcu::InstantiatedMcu::Native(mcu)) => components.push(Comp {
                        id: item.comp.id,
                        kind: Kind::Mcu(mcu),
                    }),
                    Ok(crate::mcu::InstantiatedMcu::Script(cpu)) => {
                        components.push(Comp::script_cpu(item.comp.id, cpu));
                    }
                    Err(e) => skipped.push(format!("MCU:{} ({e})", item.comp.id)),
                }
            }
            Kind::Subcircuit {
                ref device,
                logic_symbol,
                ref package_name,
            } => {
                let Some(resolved) = resolve_subc(device, search) else {
                    skipped.push(format!("Subcircuit:{device}"));
                    continue;
                };
                match instantiate_resolved(
                    &item.comp.id,
                    device,
                    &resolved.src,
                    resolved.path.as_deref(),
                    search,
                    logic_symbol,
                    package_name.as_deref(),
                    depth,
                ) {
                    Ok(inst) => {
                        components.extend(inst.components);
                        connectors.extend(inst.connectors);
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
                components.push(comp);
            }
        }
    }
    Ok(Expanded {
        components,
        connectors,
        skipped,
    })
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
    if typ == "Package" {
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
        for (k, v) in properties.iter_mut() {
            if k == "CircId" {
                *v = prefix_val(v);
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

/// Layout used by the canvas chip, plus nested source for later flatten.
#[derive(Clone, Debug)]
pub struct SubcView {
    pub device: String,
    pub package: Package,
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
    let Some(package) = select_package(&packages, logic_symbol, package_name).cloned() else {
        return Err(Error::Parse(format!("no packages for subcircuit {device}")));
    };
    Ok(SubcView {
        device: device.to_string(),
        package,
        nested_src: resolved.src,
        nested_path: resolved.path.map(|p| p.to_string_lossy().into_owned()),
        logic_symbol,
    })
}

/// A node in the subcircuit tree hierarchy snapshot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubcTreeNode {
    pub label: String,
    pub file_path: String,
    pub children: Vec<SubcTreeNode>,
}

/// Snapshot subcircuit tree from parsed circuit items.
pub fn snapshot_subcircuits_from_parsed(
    parsed: &ParsedCircuit,
    search: &SubcSearch,
) -> Vec<SubcTreeNode> {
    let mut result = Vec::new();
    for it in &parsed.items {
        if let crate::elements::Kind::Subcircuit { device, .. } = &it.comp.kind {
            let label = it
                .label
                .as_ref()
                .filter(|l| !l.is_empty())
                .unwrap_or(&it.comp.id)
                .clone();
            let dev = device_from_id(device);
            let resolved = resolve_subc(&dev, search);
            let file_path = resolved
                .as_ref()
                .and_then(|r| r.path.as_ref())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            let children = if let Some(r) = resolved {
                if let Ok(p) = parse_sim1(&r.src) {
                    let child_search = if !file_path.is_empty() {
                        search.child_for(Path::new(&file_path))
                    } else {
                        search.clone()
                    };
                    snapshot_subcircuits_from_parsed(&p, &child_search)
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
        assert_eq!(instance_num_id("vdiv-1", "vdiv"), "1");
        assert_eq!(instance_num_id("1@74HC00-2", "74HC00"), "1@2");
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

        let parsed = crate::sim1::parse_sim1(root_sim1).unwrap();
        let tree = snapshot_subcircuits_from_parsed(&parsed, &search);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].label, "Subcircuit Alpha");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].label, "Child Gamma");
        assert_eq!(tree[1].label, "Subcircuit Beta");
        assert_eq!(tree[1].children.len(), 0);
    }
}
