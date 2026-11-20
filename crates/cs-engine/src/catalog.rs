//! Component-set XML (`itemlib` / `itemset` / `item`). C++ `ComponentList::loadXml`,
//! `getDataFile`, `getFileDir`. MCU family entries and file lookup live here;
//! other XML types are stored for lookup but not added to the canvas catalog.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{installer, package, settings};

use quick_xml::Reader;
use quick_xml::events::Event;

/// Category name used for project/circuit-local custom subcircuits.
pub const LOCAL_SUBCIRCUIT_CATEGORY: &str = "Custom";

/// One `<item>` from an itemlib XML file or a locally discovered subcircuit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogItem {
    pub name: String,
    pub typ: String,
    pub category: String,
    /// itemset `folder` (no trailing slash).
    pub folder: String,
    /// Path of the itemlib XML that defined this item.
    pub xml_file: PathBuf,
    pub package: Option<String>,
    pub data: Option<String>,
    pub info: Option<String>,
    /// True for project-local or circuit-local custom components.
    pub is_custom: bool,
}

impl CatalogItem {
    pub fn xml_dir(&self) -> PathBuf {
        self.xml_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// C++ `m_dirFileList[name] = xmlDir + "/" + folder`.
    pub fn dir(&self) -> PathBuf {
        let d = self.xml_dir();
        if self.folder.is_empty() {
            d
        } else {
            d.join(&self.folder)
        }
    }

    /// C++ `Mcu::Mcu` catalog branch: `(mcuFile, baseFile)` without extension on
    /// the package base. `package` / `data` default to `{name}/{name}`.
    pub fn mcu_paths(&self) -> (PathBuf, PathBuf) {
        let default = format!("{}/{}", self.name, self.name);
        let pkg = self.package.as_deref().unwrap_or(&default);
        let dat = self.data.as_deref().unwrap_or(pkg);
        (self.join_rel(&format!("{dat}.mcu")), self.join_rel(pkg))
    }

    fn join_rel(&self, rel: &str) -> PathBuf {
        let mut p = self.xml_dir();
        if !self.folder.is_empty() {
            p.push(&self.folder);
        }
        for part in rel.split(['/', '\\']).filter(|s| !s.is_empty()) {
            p.push(part);
        }
        p
    }
}

/// Parsed itemlib maps. First name wins (C++ `m_components.contains`).
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    items: Vec<CatalogItem>,
    by_name: HashMap<String, usize>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, name: &str) -> Option<&CatalogItem> {
        self.by_name.get(name).and_then(|&i| self.items.get(i))
    }

    pub fn items(&self) -> &[CatalogItem] {
        &self.items
    }

    pub fn mcu_items(&self) -> impl Iterator<Item = &CatalogItem> {
        self.items.iter().filter(|i| i.typ == "MCU")
    }

    /// C++ `getDataFile`.
    pub fn data_file(&self, name: &str) -> Option<&Path> {
        self.get(name).map(|i| i.xml_file.as_path())
    }

    /// C++ `getFileDir`.
    pub fn file_dir(&self, name: &str) -> Option<PathBuf> {
        self.get(name).map(|i| i.dir())
    }

    pub fn insert(&mut self, item: CatalogItem) {
        if self.by_name.contains_key(&item.name) {
            return;
        }
        let i = self.items.len();
        self.by_name.insert(item.name.clone(), i);
        self.items.push(item);
    }

    /// C++ `ComponentList::loadXml`.
    pub fn load_xml_str(&mut self, src: &str, xml_file: &Path) {
        for item in parse_itemlib(src, xml_file) {
            self.insert(item);
        }
    }

    pub fn load_xml_file(&mut self, path: &Path) {
        let Ok(src) = std::fs::read_to_string(path) else {
            return;
        };
        self.load_xml_str(&src, path);
    }

    /// C++ `LoadCompSetAt`: `*.xml` in this directory, not recursive.
    pub fn load_dir(&mut self, dir: &Path) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut xmls: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("xml") && p.is_file())
            .collect();
        xmls.sort();
        for p in xmls {
            self.load_xml_file(&p);
        }
    }

    /// Folders under `{user}/test` that contain `{name}/{name}.mcu` (C++ `loadTest`).
    pub fn load_test_dir(&mut self, user_dir: &Path) {
        let test = user_dir.join("test");
        let Ok(rd) = std::fs::read_dir(&test) else {
            return;
        };
        let mut names: Vec<String> = rd
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            let mcu = test.join(&name).join(format!("{name}.mcu"));
            if !mcu.is_file() {
                continue;
            }
            self.insert(CatalogItem {
                name: name.clone(),
                typ: "MCU".into(),
                category: "test".into(),
                folder: name.clone(),
                xml_file: test.join("test.xml"),
                package: None,
                data: None,
                info: None,
                is_custom: false,
            });
        }
    }

    /// Scan a local circuit/project directory (non-recursively, plus immediate
    /// `{name}/{name}.{circ1,sim2,sim1}` subcircuit folders and `*.xml` itemlibs)
    /// for custom subcircuits.
    pub fn load_local_subcircuits(&mut self, dir: &Path, exclude_file: Option<&Path>) {
        if !dir.is_dir() {
            return;
        }
        // 1. Local itemlib XML files in this directory.
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        let mut xmls = Vec::new();
        let mut subdirs: Vec<(String, PathBuf)> = Vec::new();
        let mut direct_files: Vec<(String, u8, PathBuf)> = Vec::new();
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                if !name.eq_ignore_ascii_case("data") {
                    subdirs.push((name, path));
                }
            } else if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                match ext.as_str() {
                    "xml" => xmls.push(path),
                    "circ1" | "sim2" | "sim1" => {
                        let prio = ext_priority(&ext);
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            if !stem.is_empty() {
                                direct_files.push((stem.to_string(), prio, path));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        xmls.sort();
        for p in xmls {
            if let Ok(src) = fs::read_to_string(&p) {
                for mut item in parse_itemlib(&src, &p) {
                    item.is_custom = true;
                    self.insert(item);
                }
            }
        }

        // 2. Subcircuit folders: `{dir}/{name}/{name}.{circ1,sim2,sim1}`.
        subdirs.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, sub) in subdirs {
            for ext in ["circ1", "sim2", "sim1"] {
                let cand = sub.join(format!("{name}.{ext}"));
                if !cand.is_file() {
                    continue;
                }
                if exclude_file.is_some_and(|ex| same_path(&cand, ex)) {
                    break;
                }
                if has_subcircuit_package(&cand, &name) {
                    self.insert(CatalogItem {
                        name: name.clone(),
                        typ: "Subcircuit".into(),
                        category: LOCAL_SUBCIRCUIT_CATEGORY.into(),
                        folder: String::new(),
                        xml_file: dir.join("local.xml"),
                        package: None,
                        data: None,
                        info: None,
                        is_custom: true,
                    });
                }
                break;
            }
        }

        // 3. Direct subcircuit files in `dir`: `{dir}/{name}.{circ1,sim2,sim1}`.
        direct_files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut last_stem = String::new();
        for (stem, _prio, cand) in direct_files {
            if stem == last_stem {
                continue;
            }
            last_stem = stem.clone();
            if exclude_file.is_some_and(|ex| same_path(&cand, ex)) {
                continue;
            }
            if has_subcircuit_package(&cand, &stem) {
                self.insert(CatalogItem {
                    name: stem,
                    typ: "Subcircuit".into(),
                    category: LOCAL_SUBCIRCUIT_CATEGORY.into(),
                    folder: String::new(),
                    xml_file: dir.join("local.xml"),
                    package: None,
                    data: None,
                    info: None,
                    is_custom: true,
                });
            }
        }
    }

    /// App data folders matching C++ `ComponentList::createList` + installer sets.
    pub fn load_standard() -> Self {
        let mut c = Self::new();
        for dir in standard_dirs() {
            c.load_dir(&dir);
        }
        let settings = settings::get();
        if !settings.user_path.is_empty() {
            c.load_test_dir(Path::new(&settings.user_path));
        }
        c
    }
}

use std::sync::RwLock;

/// Cached standard catalog (user + Application Support component sets).
static STANDARD_CACHE: RwLock<Option<Catalog>> = RwLock::new(None);

pub fn standard() -> Catalog {
    if let Ok(guard) = STANDARD_CACHE.read() {
        if let Some(c) = &*guard {
            return c.clone();
        }
    }
    let c = Catalog::load_standard();
    if let Ok(mut guard) = STANDARD_CACHE.write() {
        *guard = Some(c.clone());
    }
    c
}

/// Force reloads the standard catalog from disk, dropping the cached copy.
pub fn reload_standard() -> Catalog {
    let c = Catalog::load_standard();
    if let Ok(mut guard) = STANDARD_CACHE.write() {
        *guard = Some(c.clone());
    }
    c
}

/// Discover custom/local subcircuits in the active circuit directory, its `data/`
/// subfolder, and the open project directory (and its `data/` subfolder).
pub fn local_subcircuits(circuit_path: Option<&str>, project_dir: Option<&str>) -> Catalog {
    let mut c = Catalog::new();
    let exclude = circuit_path
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Path::new);
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push_unique = |dir: PathBuf| {
        if dir.is_dir() && !dirs.iter().any(|d| same_path(d, &dir)) {
            dirs.push(dir);
        }
    };
    if let Some(circ) = exclude {
        if let Some(parent) = circ.parent() {
            push_unique(parent.to_path_buf());
            push_unique(parent.join("data"));
        }
    }
    if let Some(proj) = project_dir.map(str::trim).filter(|s| !s.is_empty()) {
        let p = PathBuf::from(proj);
        push_unique(p.clone());
        push_unique(p.join("data"));
    }
    for dir in dirs {
        c.load_local_subcircuits(&dir, exclude);
    }
    c
}

fn ext_priority(ext: &str) -> u8 {
    match ext {
        "circ1" => 0,
        "sim2" => 1,
        "sim1" => 2,
        _ => 3,
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => {
            let sa = a.to_string_lossy().replace('\\', "/");
            let sb = b.to_string_lossy().replace('\\', "/");
            if cfg!(windows) {
                sa.eq_ignore_ascii_case(&sb)
            } else {
                sa == sb
            }
        }
    }
}

fn has_subcircuit_package(circuit_file: &Path, device: &str) -> bool {
    if let Some(dir) = circuit_file.parent() {
        for suffix in [".package", "_LS.package"] {
            let p = dir.join(format!("{device}{suffix}"));
            if p.is_file() {
                if let Ok(text) = fs::read_to_string(&p) {
                    if text.contains("<package") {
                        let (_, pkg) = package::convert_package(&text);
                        if !pkg.pins.is_empty() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    let Ok(src) = fs::read_to_string(circuit_file) else {
        return false;
    };
    if !src.contains("Package") {
        return false;
    }
    let (pkgs, _) = package::packages_from_sim1(&src);
    !pkgs.is_empty()
}

fn push_dir_with_children(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dir.is_dir() {
        return;
    }
    dirs.push(dir.clone());
    if let Ok(rd) = fs::read_dir(&dir) {
        let mut kids: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        kids.sort();
        dirs.extend(kids);
    }
}

/// Directory where downloadable component sets are installed.
pub fn components_dir() -> PathBuf {
    let settings = settings::get();
    let comps = if !settings.user_path.is_empty() {
        PathBuf::from(&settings.user_path).join("components")
    } else if let Some(data) = dirs::data_dir() {
        data.join("Circuit Simulator").join("components")
    } else {
        PathBuf::from("components")
    };
    let _ = fs::create_dir_all(&comps);
    installer::ensure_bundled_components_seeded(&comps);
    comps
}

pub fn standard_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let settings = settings::get();
    if !settings.user_path.is_empty() {
        let u = PathBuf::from(&settings.user_path);
        dirs.push(u);
    }
    if let Some(data) = dirs::data_dir() {
        let root = data.join("Circuit Simulator");
        dirs.push(root.join("data"));
    }
    let comps = components_dir();
    push_dir_with_children(&mut dirs, comps.clone());
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for comp_rel in ["components", "../Resources/components", "../components"] {
                let app_comps = exe_dir.join(comp_rel);
                push_dir_with_children(&mut dirs, app_comps);
            }
            for data_rel in ["data", "../Resources/data", "../data"] {
                let app_data = exe_dir.join(data_rel);
                if app_data.is_dir() {
                    dirs.push(app_data);
                }
            }
        }
    }
    dirs.push(PathBuf::from("data"));
    dirs.push(PathBuf::from("./data"));
    if !comps.join("components.txt").is_file() {
        let fixture_comps =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/components");
        push_dir_with_children(&mut dirs, fixture_comps);
    }
    dirs
}

fn parse_itemlib(src: &str, xml_file: &Path) -> Vec<CatalogItem> {
    let mut reader = Reader::from_str(src);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut in_lib = false;
    let mut group_prefix: Option<(String, String)> = None;
    let mut set_category = String::new();
    let mut set_type = String::new();
    let mut set_folder = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut attrs = HashMap::new();
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                    let val = a
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    attrs.insert(key, val);
                }
                match name.as_str() {
                    "itemlib" => in_lib = true,
                    "itemset" if in_lib => {
                        let mut cat = attrs.get("category").cloned().unwrap_or_default();
                        cat = cat.replace("IC 74", "Logic/IC 74");
                        let typ_attr = attrs.get("type").cloned().unwrap_or_default();
                        let folder_attr = attrs.get("folder").cloned().unwrap_or_default();
                        if typ_attr.is_empty() && folder_attr.is_empty() {
                            if let Some((parent, child)) = cat.rsplit_once('/') {
                                if !parent.is_empty() && !child.is_empty() {
                                    group_prefix = Some((parent.to_string(), child.to_string()));
                                }
                            }
                        } else if let Some((parent, child)) = &group_prefix {
                            if cat == *child || cat.starts_with(&format!("{child}/")) {
                                cat = format!("{parent}/{cat}");
                            }
                        }
                        set_category = cat;
                        set_type = typ_attr;
                        set_folder = folder_attr;
                    }
                    "item" if in_lib => {
                        let Some(item_name) = attrs.get("name").cloned() else {
                            buf.clear();
                            continue;
                        };
                        if item_name.is_empty() {
                            buf.clear();
                            continue;
                        }
                        out.push(CatalogItem {
                            name: item_name,
                            typ: attrs
                                .get("type")
                                .cloned()
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| set_type.clone()),
                            category: set_category.clone(),
                            folder: set_folder.clone(),
                            xml_file: xml_file.to_path_buf(),
                            package: attrs.get("package").cloned().filter(|s| !s.is_empty()),
                            data: attrs.get("data").cloned().filter(|s| !s.is_empty()),
                            info: attrs.get("info").cloned().filter(|s| !s.is_empty()),
                            is_custom: false,
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const AVR_XML: &str = r#"
<itemlib>
    <itemset category="AVR/attiny" type="MCU">
        <item name="tiny13" package="AVR/tiny13/tiny13" data="AVR/tiny13" />
        <item name="tiny25" package="AVR/tinyx5/tinyx5" data="AVR/tiny25" />
    </itemset>
    <itemset category="AVR/atmega" type="MCU">
        <item name="mega328" package="AVR/megax8/megax8" data="AVR/mega328" />
        <item name="m8 TQFP" package="AVR/megax8/mx8_tqfp" data="AVR/mega8" />
    </itemset>
    <itemset category="MCS65" type="MCU" folder="MCS65">
        <item name="6502" />
    </itemset>
    <itemset category="Arduino" type="Subcircuit" folder="arduino">
        <item name="Uno" />
    </itemset>
</itemlib>
"#;

    #[test]
    fn parse_avr_style_and_folder_defaults() {
        let mut c = Catalog::new();
        let xml = PathBuf::from("/lib/AVR/avr.xml");
        c.load_xml_str(AVR_XML, &xml);
        let t = c.get("tiny13").unwrap();
        assert_eq!(t.typ, "MCU");
        assert_eq!(t.category, "AVR/attiny");
        assert_eq!(t.package.as_deref(), Some("AVR/tiny13/tiny13"));
        assert_eq!(t.data.as_deref(), Some("AVR/tiny13"));
        let (mcu, base) = t.mcu_paths();
        assert_eq!(mcu, PathBuf::from("/lib/AVR/AVR/tiny13.mcu"));
        assert_eq!(base, PathBuf::from("/lib/AVR/AVR/tiny13/tiny13"));
        let m8 = c.get("m8 TQFP").unwrap();
        let (mcu, base) = m8.mcu_paths();
        assert_eq!(mcu, PathBuf::from("/lib/AVR/AVR/mega8.mcu"));
        assert_eq!(base, PathBuf::from("/lib/AVR/AVR/megax8/mx8_tqfp"));
        let cpu = c.get("6502").unwrap();
        assert_eq!(cpu.folder, "MCS65");
        let (mcu, base) = cpu.mcu_paths();
        assert_eq!(mcu, PathBuf::from("/lib/AVR/MCS65/6502/6502.mcu"));
        assert_eq!(base, PathBuf::from("/lib/AVR/MCS65/6502/6502"));
        assert_eq!(c.get("Uno").unwrap().typ, "Subcircuit");
        assert_eq!(c.file_dir("Uno"), Some(PathBuf::from("/lib/AVR/arduino")));
        assert_eq!(c.mcu_items().count(), 5);
    }

    #[test]
    fn first_name_wins() {
        let mut c = Catalog::new();
        let a = PathBuf::from("/a/a.xml");
        c.load_xml_str(
            r#"<itemlib><itemset category="AVR" type="MCU">
            <item name="tiny13" data="AVR/one" />
            </itemset></itemlib>"#,
            &a,
        );
        c.load_xml_str(
            r#"<itemlib><itemset category="AVR" type="MCU">
            <item name="tiny13" data="AVR/two" />
            </itemset></itemlib>"#,
            &PathBuf::from("/b/b.xml"),
        );
        assert_eq!(c.get("tiny13").unwrap().data.as_deref(), Some("AVR/one"));
        assert_eq!(c.data_file("tiny13"), Some(a.as_path()));
    }

    #[test]
    fn load_dir_and_test_folder() {
        let dir = std::env::temp_dir().join(format!("cs-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("AVR")).unwrap();
        std::fs::write(dir.join("AVR").join("avr.xml"), AVR_XML).unwrap();
        std::fs::create_dir_all(dir.join("test").join("pic14test")).unwrap();
        std::fs::write(
            dir.join("test").join("pic14test").join("pic14test.mcu"),
            "<mcu core=\"Pic14\" data=\"16\" prog=\"16\"/>",
        )
        .unwrap();
        let mut c = Catalog::new();
        c.load_dir(&dir.join("AVR"));
        c.load_test_dir(&dir);
        assert!(c.get("mega328").is_some());
        let t = c.get("pic14test").unwrap();
        assert_eq!(t.typ, "MCU");
        assert_eq!(t.category, "test");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn local_subcircuits_discovers_project_and_data_folders() {
        let dir = env::temp_dir().join(format!("cs-local-subc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("LED_PKG")).unwrap();
        fs::create_dir_all(dir.join("TESTPKG")).unwrap();
        fs::create_dir_all(dir.join("esp32_led")).unwrap();
        fs::create_dir_all(dir.join("data").join("CUSTOM_GATE")).unwrap();

        let led_pkg = r#"<circuit version="1.0.0" >
<item itemtype="Led" CircId="Led-1" Color="Yellow" mainComp="true" />
<item itemtype="Tunnel" CircId="Tunnel-2" Name="anode" />
<item itemtype="Tunnel" CircId="Tunnel-3" Name="cathode" />
<item itemtype="SubPackage" CircId="SubPackage-2" Name="LS" Width="6" Height="5" SubcType="Logic" LogicSymbol="true" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=anode; label=an&#xa;Pin; type=; xpos=56; ypos=8; angle=0; length=8; space=0; id=cathode; label=ca&#xa;" />
</circuit>"#;
        let test_pkg = r#"<circuit version="2.0.0" >
<item itemtype="Package" CircId="Package-1" label="Package-1" Width="4" Height="8" SubcType="Board" Logic_Symbol="true" Pins="Pin; type=; xpos=-8; ypos=12; angle=180; length=8; space=0; id=trm1; label=TRM1&#xa;" />
<item itemtype="Tunnel" CircId="Tunnel-1" Name="trm1" />
</circuit>"#;
        let regular_circ = r#"<circuit version="1.0.0" >
<item itemtype="Capacitor" CircId="Capacitor-1" />
</circuit>"#;

        fs::write(dir.join("LED_PKG").join("LED_PKG.circ1"), led_pkg).unwrap();
        fs::write(dir.join("TESTPKG").join("TESTPKG.sim2"), test_pkg).unwrap();
        fs::write(
            dir.join("esp32_led").join("esp32_led.ino"),
            "void setup(){}",
        )
        .unwrap();
        fs::write(
            dir.join("data")
                .join("CUSTOM_GATE")
                .join("CUSTOM_GATE.circ1"),
            led_pkg,
        )
        .unwrap();
        let root_circ = dir.join("capacitor.circ1");
        fs::write(&root_circ, regular_circ).unwrap();

        // Opening root circuit finds LED_PKG, TESTPKG, and data/CUSTOM_GATE, ignoring regular circuits & sketch dirs.
        let cat = local_subcircuits(root_circ.to_str(), dir.to_str());
        assert!(
            cat.get("LED_PKG")
                .is_some_and(|i| i.is_custom && i.typ == "Subcircuit")
        );
        assert!(
            cat.get("TESTPKG")
                .is_some_and(|i| i.is_custom && i.typ == "Subcircuit")
        );
        assert!(
            cat.get("CUSTOM_GATE")
                .is_some_and(|i| i.is_custom && i.typ == "Subcircuit")
        );
        assert!(cat.get("capacitor").is_none());
        assert!(cat.get("esp32_led").is_none());

        // Opening LED_PKG.circ1 directly excludes LED_PKG itself, while keeping sibling project subcircuits.
        let led_path = dir.join("LED_PKG").join("LED_PKG.circ1");
        let cat_inside = local_subcircuits(led_path.to_str(), dir.to_str());
        assert!(cat_inside.get("LED_PKG").is_none());
        assert!(cat_inside.get("TESTPKG").is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
