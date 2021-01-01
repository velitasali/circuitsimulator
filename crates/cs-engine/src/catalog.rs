//! Component-set XML (`itemlib` / `itemset` / `item`). C++ `ComponentList::loadXml`,
//! `getDataFile`, `getFileDir`. MCU family entries and file lookup live here;
//! other XML types are stored for lookup but not added to the canvas catalog.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use quick_xml::Reader;
use quick_xml::events::Event;

/// One `<item>` from an itemlib XML file.
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
            });
        }
    }

    /// App data folders matching C++ `ComponentList::createList` + installer sets.
    pub fn load_standard() -> Self {
        let mut c = Self::new();
        for dir in standard_dirs() {
            c.load_dir(&dir);
        }
        let settings = crate::settings::get();
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

/// Directory where downloadable component sets are installed.
pub fn components_dir() -> PathBuf {
    let settings = crate::settings::get();
    if !settings.user_path.is_empty() {
        let u = PathBuf::from(&settings.user_path).join("components");
        if u.exists() {
            return u;
        }
    }
    if let Some(data) = dirs::data_dir() {
        let comps = data.join("Circuit Simulator").join("components");
        let _ = std::fs::create_dir_all(&comps);
        comps
    } else {
        let p = PathBuf::from("components");
        let _ = std::fs::create_dir_all(&p);
        p
    }
}

pub fn standard_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let settings = crate::settings::get();
    if !settings.user_path.is_empty() {
        let u = PathBuf::from(&settings.user_path);
        dirs.push(u.clone());
        dirs.push(u.join("components"));
    }
    if let Some(data) = dirs::data_dir() {
        let root = data.join("Circuit Simulator");
        dirs.push(root.join("data"));
        let comps = root.join("components");
        dirs.push(comps.clone());
        if let Ok(rd) = std::fs::read_dir(&comps) {
            let mut kids: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            kids.sort();
            dirs.extend(kids);
        }
    }
    dirs.push(PathBuf::from("data"));
    dirs.push(PathBuf::from("./data"));
    dirs
}

fn parse_itemlib(src: &str, xml_file: &Path) -> Vec<CatalogItem> {
    let mut reader = Reader::from_str(src);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut in_lib = false;
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
                        set_category = cat;
                        set_type = attrs.get("type").cloned().unwrap_or_default();
                        set_folder = attrs.get("folder").cloned().unwrap_or_default();
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
}
