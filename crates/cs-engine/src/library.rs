//! Component list catalog. Names and categories match `ItemLibrary::loadItems`.
//! Search / expand live here so the QML list can stay a flat visible-row model.

use crate::canvas::PlaceKind;

/// Matches `treItemType_t` / componentlist.qml.
pub const TYPE_COMPONENT: i32 = 1;
pub const TYPE_CATEG_MAIN: i32 = 2;
pub const TYPE_CATEG_CHILD: i32 = 3;

pub const RECENT_CATEGORY_ID: u32 = 0x8000_0000;
pub const RECENT_ITEM_BASE_ID: u32 = 0x8000_0001;
pub const MAX_RECENT_ITEMS: usize = 10;

#[derive(Clone, Debug)]
pub struct LibraryItem {
    pub id: u32,
    pub caption: String,
    pub parent: Option<u32>,
    pub item_type: i32,
    pub comp_type: String,
    pub kind: Option<PlaceKind>,
    pub expanded: bool,
    pub icon: String,
    pub aliases: Vec<String>,
}

/// Text match ranking matching C++ `TreeItem::textMatchTier`:
/// Tier 0: Exact match (case-insensitive)
/// Tier 1: Whole-word match (e.g. "Transistor" in "Bipolar Junction Transistor")
/// Tier 2: Word prefix match (e.g. "Trans" in "Transistor")
/// Tier 3: Substring match
pub fn text_match_tier(text: &str, filter: &str) -> Option<i32> {
    let f = filter.trim();
    if f.is_empty() {
        return Some(0);
    }
    let text_lower = text.to_lowercase();
    let f_lower = f.to_lowercase();
    if text_lower == f_lower {
        return Some(0);
    }

    let words: Vec<&str> = text_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    for word in &words {
        if *word == f_lower {
            return Some(1);
        }
    }

    for word in &words {
        if word.starts_with(&f_lower) {
            return Some(2);
        }
    }

    if text_lower.contains(&f_lower) {
        return Some(3);
    }

    None
}

impl LibraryItem {
    /// Match quality rank for an item against a search filter:
    /// - 0..=3: Direct match on caption or comp_type
    /// - 4..=7: Alias match (4 + alias_tier)
    pub fn match_rank(&self, filter: &str) -> Option<i32> {
        let f = filter.trim();
        if f.is_empty() {
            return Some(0);
        }
        let translated = crate::i18n::tr(&self.caption);
        let mut best = text_match_tier(&self.caption, f);
        if let Some(tier) = text_match_tier(&translated, f) {
            if best.is_none() || tier < best.unwrap() {
                best = Some(tier);
            }
        }
        if let Some(tier) = text_match_tier(&self.comp_type, f) {
            if best.is_none() || tier < best.unwrap() {
                best = Some(tier);
            }
        }
        for alias in &self.aliases {
            let tr_alias = crate::i18n::tr(alias);
            if let Some(tier) = text_match_tier(alias, f) {
                let rank = 4 + tier;
                if best.is_none() || rank < best.unwrap() {
                    best = Some(rank);
                }
            }
            if let Some(tier) = text_match_tier(&tr_alias, f) {
                let rank = 4 + tier;
                if best.is_none() || rank < best.unwrap() {
                    best = Some(rank);
                }
            }
        }
        best
    }
}

#[derive(Clone, Debug)]
pub struct VisibleRow {
    pub id: u32,
    pub caption: String,
    pub item_type: i32,
    pub comp_type: String,
    pub kind: Option<PlaceKind>,
    pub depth: i32,
    pub expanded: bool,
    pub has_children: bool,
    pub icon: String,
}

#[derive(Clone, Debug)]
pub struct Library {
    items: Vec<LibraryItem>,
    filter: String,
    recent_components: Vec<String>,
    recent_expanded: bool,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub fn new() -> Self {
        let mut lib = Self {
            items: catalog(),
            filter: String::new(),
            recent_components: Vec::new(),
            recent_expanded: true,
        };
        // Main categories start collapsed except the first few, matching a
        // freshly opened list that the user can open. C++ restores expansion
        // from settings; we expand Sources and Passive so Resistor is one click
        // away on a new session.
        for it in &mut lib.items {
            if it.comp_type == "Sources"
                || it.comp_type == "Passive"
                || it.comp_type == "Resistors"
                || it.comp_type == "Reactive"
                || it.comp_type == "Switches"
                || it.comp_type == "Rectifiers"
                || it.comp_type == "Active"
                || it.comp_type == "Other Active"
                || it.comp_type == "Leds"
                || it.comp_type == "Meters"
            {
                it.expanded = true;
            }
        }
        lib
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
    }

    pub fn set_recent_components(&mut self, recents: Vec<String>) {
        self.recent_components = recents;
        if self.recent_components.len() > MAX_RECENT_ITEMS {
            self.recent_components.truncate(MAX_RECENT_ITEMS);
        }
    }

    pub fn recent_components(&self) -> &[String] {
        &self.recent_components
    }

    pub fn add_recent(&mut self, spec: &str) {
        let spec = spec.trim();
        if spec.is_empty() {
            return;
        }
        self.recent_components.retain(|s| s != spec);
        self.recent_components.insert(0, spec.to_string());
        if self.recent_components.len() > MAX_RECENT_ITEMS {
            self.recent_components.truncate(MAX_RECENT_ITEMS);
        }
    }

    pub fn clear_recent(&mut self) {
        self.recent_components.clear();
    }

    pub fn find_catalog_item(&self, spec: &str) -> Option<&LibraryItem> {
        let (cap, typ) = if let Some((c, t)) = spec.split_once(',') {
            (c.trim(), t.trim())
        } else {
            (spec.trim(), spec.trim())
        };
        // 1. Exact match on caption and comp_type
        if let Some(it) = self.items.iter().find(|i| {
            i.item_type == TYPE_COMPONENT
                && (i.caption.eq_ignore_ascii_case(cap) && i.comp_type.eq_ignore_ascii_case(typ))
        }) {
            return Some(it);
        }
        // 2. Match on caption
        if let Some(it) = self
            .items
            .iter()
            .find(|i| i.item_type == TYPE_COMPONENT && i.caption.eq_ignore_ascii_case(cap))
        {
            return Some(it);
        }
        // 3. Match on comp_type
        if let Some(it) = self
            .items
            .iter()
            .find(|i| i.item_type == TYPE_COMPONENT && i.comp_type.eq_ignore_ascii_case(typ))
        {
            return Some(it);
        }
        None
    }

    pub fn set_expanded(&mut self, id: u32, expanded: bool) {
        if id == RECENT_CATEGORY_ID {
            self.recent_expanded = expanded;
            return;
        }
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            if it.item_type != TYPE_COMPONENT {
                it.expanded = expanded;
            }
        }
    }

    pub fn is_expanded(&self, id: u32) -> bool {
        if id == RECENT_CATEGORY_ID {
            return self.recent_expanded;
        }
        self.items
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.expanded)
            .unwrap_or(false)
    }

    pub fn item(&self, id: u32) -> Option<&LibraryItem> {
        if id >= RECENT_ITEM_BASE_ID {
            let idx = (id - RECENT_ITEM_BASE_ID) as usize;
            if let Some(spec) = self.recent_components.get(idx) {
                return self.find_catalog_item(spec);
            }
        }
        self.items.iter().find(|i| i.id == id)
    }

    pub fn items(&self) -> &[LibraryItem] {
        &self.items
    }

    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut out = Vec::new();
        let searching = !self.filter.trim().is_empty();
        let needle = self.filter.to_lowercase();

        // Prepend Recently Used category if there are recent items
        if !self.recent_components.is_empty() {
            let mut recent_rows: Vec<VisibleRow> = Vec::new();
            for (idx, spec) in self.recent_components.iter().enumerate() {
                if let Some(orig) = self.find_catalog_item(spec) {
                    let matches_search = !searching
                        || orig.match_rank(&needle).is_some()
                        || orig.caption.to_lowercase().contains(&needle)
                        || orig.comp_type.to_lowercase().contains(&needle);
                    if matches_search {
                        recent_rows.push(VisibleRow {
                            id: RECENT_ITEM_BASE_ID + idx as u32,
                            caption: orig.caption.clone(),
                            item_type: TYPE_COMPONENT,
                            comp_type: orig.comp_type.clone(),
                            kind: orig.kind,
                            depth: 1,
                            expanded: false,
                            has_children: false,
                            icon: orig.icon.clone(),
                        });
                    }
                }
            }

            let cat_name = "Recently Used";
            let cat_translated = crate::i18n::tr(cat_name);
            let cat_matches = !searching
                || cat_name.to_lowercase().contains(&needle)
                || cat_translated.to_lowercase().contains(&needle);

            if !recent_rows.is_empty() || cat_matches {
                let expanded = if searching {
                    true
                } else {
                    self.recent_expanded
                };
                out.push(VisibleRow {
                    id: RECENT_CATEGORY_ID,
                    caption: cat_name.to_string(),
                    item_type: TYPE_CATEG_MAIN,
                    comp_type: cat_name.to_string(),
                    kind: None,
                    depth: 0,
                    expanded,
                    has_children: !recent_rows.is_empty(),
                    icon: String::new(),
                });
                if expanded {
                    out.extend(recent_rows);
                }
            }
        }

        let roots: Vec<u32> = self
            .items
            .iter()
            .filter(|i| i.parent.is_none())
            .map(|i| i.id)
            .collect();
        for id in roots {
            self.walk(id, 0, searching, &needle, &mut out);
        }
        out
    }

    fn item_matches(&self, id: u32, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        let Some(it) = self.item(id) else {
            return false;
        };
        it.match_rank(needle).is_some()
            || it.caption.to_lowercase().contains(needle)
            || it.comp_type.to_lowercase().contains(needle)
    }

    fn matches(&self, id: u32, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        if self.item_matches(id, needle) {
            return true;
        }
        self.children(id).iter().any(|c| self.matches(*c, needle))
    }

    fn children(&self, id: u32) -> Vec<u32> {
        self.items
            .iter()
            .filter(|i| i.parent == Some(id))
            .map(|i| i.id)
            .collect()
    }

    fn has_children(&self, id: u32) -> bool {
        self.items.iter().any(|i| i.parent == Some(id))
    }

    fn walk(&self, id: u32, depth: i32, searching: bool, needle: &str, out: &mut Vec<VisibleRow>) {
        if searching && !self.matches(id, needle) {
            return;
        }
        let Some(it) = self.item(id) else {
            return;
        };
        let kids = self.has_children(id);
        let expanded = if searching { kids } else { it.expanded };
        out.push(VisibleRow {
            id: it.id,
            caption: it.caption.clone(),
            item_type: it.item_type,
            comp_type: it.comp_type.clone(),
            kind: it.kind,
            depth,
            expanded,
            has_children: kids,
            icon: it.icon.clone(),
        });
        if expanded {
            let mut children = self.children(id);
            if searching {
                let cat_matched = self.item_matches(id, needle);
                children.retain(|c| cat_matched || self.matches(*c, needle));
                children.sort_by_key(|&c| {
                    self.item(c)
                        .and_then(|item| item.match_rank(needle))
                        .unwrap_or(8)
                });
            }
            for c in children {
                self.walk(c, depth + 1, searching, needle, out);
            }
        }
    }

    fn next_id(&self) -> u32 {
        self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1
    }

    fn find_category(&self, name: &str) -> Option<u32> {
        self.items
            .iter()
            .find(|i| i.item_type != TYPE_COMPONENT && (i.caption == name || i.comp_type == name))
            .map(|i| i.id)
    }

    /// C++ `getCategory` / `addCategory`. Reuses an existing node of this name.
    pub fn ensure_category(&mut self, name: &str, parent: Option<u32>) -> u32 {
        if let Some(id) = self.find_category(name) {
            return id;
        }
        let id = self.next_id();
        let item_type = if parent.is_none() {
            TYPE_CATEG_MAIN
        } else {
            TYPE_CATEG_CHILD
        };
        self.items.push(LibraryItem {
            id,
            caption: name.to_string(),
            parent,
            item_type,
            comp_type: name.to_string(),
            kind: None,
            expanded: false,
            icon: String::new(),
            aliases: Vec::new(),
        });
        id
    }

    fn ensure_category_path(&mut self, path: &str) -> Option<u32> {
        if path.is_empty() {
            return None;
        }
        let mut parent = None;
        let mut last = None;
        for segment in path.split('/') {
            if segment.is_empty() {
                continue;
            }
            let id = self.ensure_category(segment, parent);
            parent = Some(id);
            last = Some(id);
        }
        last
    }

    /// Add MCU family entries from itemlib XML (C++ `loadXml` type="MCU").
    pub fn add_catalog_mcus(&mut self, catalog: &crate::catalog::Catalog) {
        for item in catalog.mcu_items() {
            if self.items.iter().any(|i| {
                i.item_type == TYPE_COMPONENT && i.caption == item.name && i.comp_type == "MCU"
            }) {
                continue;
            }
            let parent = self.ensure_category_path(&item.category);
            self.items.push(LibraryItem {
                id: self.next_id(),
                caption: item.name.clone(),
                parent,
                item_type: TYPE_COMPONENT,
                comp_type: "MCU".into(),
                kind: Some(PlaceKind::Mcu),
                expanded: false,
                icon: "ic2.svg".into(),
                aliases: vec![
                    "MCU".into(),
                    "Microcontroller".into(),
                    item.category.clone(),
                ],
            });
        }
    }
}

struct Spec {
    caption: &'static str,
    parent: &'static str,
    kind: Option<PlaceKind>,
    icon: &'static str,
    aliases: &'static [&'static str],
}

fn catalog() -> Vec<LibraryItem> {
    // Fourth field is `LibraryItem` type / category key. Empty parent = main.
    const SPECS: &[Spec] = &[
        Spec {
            caption: "Meters",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Probe",
            parent: "Meters",
            kind: Some(PlaceKind::Probe),
            icon: "probe.svg",
            aliases: &["Logic Probe", "Voltage Probe", "Meter", "Test Point"],
        },
        Spec {
            caption: "Voltmeter",
            parent: "Meters",
            kind: Some(PlaceKind::Voltmeter),
            icon: "voltimeter.svg",
            aliases: &["Volt Meter", "Voltage Meter", "Volts", "Meter"],
        },
        Spec {
            caption: "Ampmeter",
            parent: "Meters",
            kind: Some(PlaceKind::Ammeter),
            icon: "amperimeter.svg",
            aliases: &[
                "Amp Meter",
                "Amperimeter",
                "Ammeter",
                "Current Meter",
                "Amperes",
                "Meter",
            ],
        },
        Spec {
            caption: "Frequency Meter",
            parent: "Meters",
            kind: Some(PlaceKind::FreqMeter),
            icon: "frequencimeter.svg",
            aliases: &["FreqMeter", "Frequency Counter", "Frequencimeter", "Hz"],
        },
        Spec {
            caption: "Oscilloscope",
            parent: "Meters",
            kind: Some(PlaceKind::Oscope),
            icon: "oscope.svg",
            aliases: &["Oscilloscope", "Oscope", "Scope", "Waveform Display"],
        },
        Spec {
            caption: "Logic Analyzer",
            parent: "Meters",
            kind: Some(PlaceKind::LogicAnalyzer),
            icon: "lanalizer.svg",
            aliases: &["LAnalizer", "Digital Analyzer", "Logic Trace"],
        },
        Spec {
            caption: "Sources",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Fixed Voltage",
            parent: "Sources",
            kind: Some(PlaceKind::FixedVolt),
            icon: "voltage.svg",
            aliases: &[
                "FixedVolt",
                "VCC",
                "VDD",
                "Power Supply",
                "Voltage Source",
                "5V",
                "3.3V",
            ],
        },
        Spec {
            caption: "Clock",
            parent: "Sources",
            kind: Some(PlaceKind::Clock),
            icon: "clock.svg",
            aliases: &[
                "Pulse Generator",
                "Square Wave",
                "Oscillator",
                "Clock Signal",
            ],
        },
        Spec {
            caption: "Wave Generator",
            parent: "Sources",
            kind: Some(PlaceKind::WaveGen),
            icon: "wavegen.svg",
            aliases: &[
                "WaveGen",
                "Function Generator",
                "Signal Generator",
                "Sine Wave",
            ],
        },
        Spec {
            caption: "Voltage Source",
            parent: "Sources",
            kind: Some(PlaceKind::VoltSource),
            icon: "voltsource.svg",
            aliases: &["VoltSource", "AC Voltage", "Signal Source", "Power Source"],
        },
        Spec {
            caption: "Current Source",
            parent: "Sources",
            kind: Some(PlaceKind::CurrSource),
            icon: "cursource.svg",
            aliases: &["CurrSource", "Constant Current", "Current Generator"],
        },
        Spec {
            caption: "Current Controlled Source",
            parent: "Sources",
            kind: Some(PlaceKind::Csource),
            icon: "csource.svg",
            aliases: &["Csource", "CCCS", "CCVS", "Dependent Source"],
        },
        Spec {
            caption: "Battery",
            parent: "Sources",
            kind: Some(PlaceKind::Battery),
            icon: "battery.svg",
            aliases: &["DC Source", "Cell", "Power Cell", "Accumulator"],
        },
        Spec {
            caption: "Rail",
            parent: "Sources",
            kind: Some(PlaceKind::Rail),
            icon: "rail.svg",
            aliases: &["Power Rail", "Voltage Rail", "Bus Rail"],
        },
        Spec {
            caption: "Ground (0 V)",
            parent: "Sources",
            kind: Some(PlaceKind::Ground),
            icon: "ground.svg",
            aliases: &["Ground", "GND", "Earth", "0V", "Zero Volt"],
        },
        Spec {
            caption: "Switches",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Push Button",
            parent: "Switches",
            kind: Some(PlaceKind::Push),
            icon: "push.svg",
            aliases: &[
                "Push",
                "PushSwitch",
                "Momentary Switch",
                "Tactile Switch",
                "Button",
            ],
        },
        Spec {
            caption: "Switch",
            parent: "Switches",
            kind: Some(PlaceKind::Switch),
            icon: "switch.svg",
            aliases: &["SPST", "SPDT", "Toggle Switch", "Knife Switch"],
        },
        Spec {
            caption: "Switch DIP",
            parent: "Switches",
            kind: Some(PlaceKind::SwitchDip),
            icon: "switchdip.svg",
            aliases: &["SwitchDip", "Switch Dip", "DIP Switch", "Slide Switch"],
        },
        Spec {
            caption: "Relay",
            parent: "Switches",
            kind: Some(PlaceKind::Relay),
            icon: "relay-spst.svg",
            aliases: &["Electromechanical Relay", "Contactor", "SPDT Relay"],
        },
        Spec {
            caption: "Keypad",
            parent: "Switches",
            kind: Some(PlaceKind::KeyPad),
            icon: "keypad.svg",
            aliases: &["KeyPad", "Numeric Keypad", "Matrix Keypad", "Keyboard"],
        },
        Spec {
            caption: "Passive",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Resistors",
            parent: "Passive",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Resistor",
            parent: "Resistors",
            kind: Some(PlaceKind::Resistor),
            icon: "resistor.svg",
            aliases: &["Resistance", "Passive"],
        },
        Spec {
            caption: "Resistor DIP",
            parent: "Resistors",
            kind: Some(PlaceKind::ResistorDip),
            icon: "resistordip.svg",
            aliases: &[
                "ResistorDip",
                "DIP Resistor",
                "Resistor Network",
                "Resistor Array",
            ],
        },
        Spec {
            caption: "Potentiometer",
            parent: "Resistors",
            kind: Some(PlaceKind::Potentiometer),
            icon: "potentiometer.svg",
            aliases: &["Pot", "Variable Resistor", "Trimpot", "Rheostat"],
        },
        Spec {
            caption: "Variable Resistor",
            parent: "Resistors",
            kind: Some(PlaceKind::VarResistor),
            icon: "varresistor.svg",
            aliases: &["VarResistor", "Rheostat", "Trimpot", "Potentiometer"],
        },
        Spec {
            caption: "Resistive Sensors",
            parent: "Passive",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "LDR",
            parent: "Resistive Sensors",
            kind: Some(PlaceKind::Ldr),
            icon: "ldr.svg",
            aliases: &[
                "Photoresistor",
                "Light Dependent Resistor",
                "Photocell",
                "Optical Sensor",
            ],
        },
        Spec {
            caption: "Thermistor",
            parent: "Resistive Sensors",
            kind: Some(PlaceKind::Thermistor),
            icon: "thermistor.svg",
            aliases: &["Temperature Sensor", "NTC", "PTC", "Thermal Sensor"],
        },
        Spec {
            caption: "RTD",
            parent: "Resistive Sensors",
            kind: Some(PlaceKind::Rtd),
            icon: "rtd.svg",
            aliases: &[
                "Resistance Temperature Detector",
                "PT100",
                "Temperature Sensor",
            ],
        },
        Spec {
            caption: "Strain Gauge",
            parent: "Resistive Sensors",
            kind: Some(PlaceKind::Strain),
            icon: "strain.svg",
            aliases: &["Strain", "Load Cell", "Force Sensor", "Pressure Sensor"],
        },
        Spec {
            caption: "Reactive",
            parent: "Passive",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Capacitor",
            parent: "Reactive",
            kind: Some(PlaceKind::Capacitor),
            icon: "capacitor.svg",
            aliases: &["Cap", "Condenser", "Passive"],
        },
        Spec {
            caption: "Electrolytic Capacitor",
            parent: "Reactive",
            kind: Some(PlaceKind::ElCapacitor),
            icon: "elcapacitor.svg",
            aliases: &["ElCapacitor", "Polarized Capacitor", "Elko", "Capacitor"],
        },
        Spec {
            caption: "Variable Capacitor",
            parent: "Reactive",
            kind: Some(PlaceKind::VarCapacitor),
            icon: "varcapacitor.svg",
            aliases: &["VarCapacitor", "Trimmer", "Tuning Capacitor", "Capacitor"],
        },
        Spec {
            caption: "Inductor",
            parent: "Reactive",
            kind: Some(PlaceKind::Inductor),
            icon: "inductor.svg",
            aliases: &["Coil", "Choke", "Solenoid", "Reactor"],
        },
        Spec {
            caption: "Variable Inductor",
            parent: "Reactive",
            kind: Some(PlaceKind::VarInductor),
            icon: "varinductor.svg",
            aliases: &["VarInductor", "Tunable Inductor", "Coil"],
        },
        Spec {
            caption: "Transformer",
            parent: "Reactive",
            kind: Some(PlaceKind::Transformer),
            icon: "transformer.svg",
            aliases: &["Xformer", "Coupled Inductor", "Isolation Transformer"],
        },
        Spec {
            caption: "Active",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Rectifiers",
            parent: "Active",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Diode",
            parent: "Rectifiers",
            kind: Some(PlaceKind::Diode),
            icon: "diode.svg",
            aliases: &["PN Diode", "Rectifier", "Semiconductor"],
        },
        Spec {
            caption: "Zener Diode",
            parent: "Rectifiers",
            kind: Some(PlaceKind::Zener),
            icon: "zener.svg",
            aliases: &["Zener", "Voltage Reference", "Regulator Diode", "Diode"],
        },
        Spec {
            caption: "SCR",
            parent: "Rectifiers",
            kind: Some(PlaceKind::Scr),
            icon: "scr.svg",
            aliases: &["Thyristor", "Silicon Controlled Rectifier", "Semiconductor"],
        },
        Spec {
            caption: "Diac",
            parent: "Rectifiers",
            kind: Some(PlaceKind::Diac),
            icon: "diac.svg",
            aliases: &[
                "Diode for Alternating Current",
                "Bidirectional Trigger Diode",
                "Trigger Diode",
            ],
        },
        Spec {
            caption: "Triac",
            parent: "Rectifiers",
            kind: Some(PlaceKind::Triac),
            icon: "triac.svg",
            aliases: &[
                "Triode for Alternating Current",
                "Bidirectional Triode Thyristor",
                "Thyristor",
                "AC Switch",
            ],
        },
        Spec {
            caption: "Transistors",
            parent: "Active",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "BJT",
            parent: "Transistors",
            kind: Some(PlaceKind::Bjt),
            icon: "bjt.svg",
            aliases: &[
                "Bipolar Junction Transistor",
                "Transistor",
                "NPN",
                "PNP",
                "Bipolar Transistor",
            ],
        },
        Spec {
            caption: "MOSFET",
            parent: "Transistors",
            kind: Some(PlaceKind::Mosfet),
            icon: "mosfet.svg",
            aliases: &[
                "Mosfet",
                "Metal-Oxide-Semiconductor Field-Effect Transistor",
                "IGFET",
                "Transistor",
                "FET",
                "NMOS",
                "PMOS",
            ],
        },
        Spec {
            caption: "JFET",
            parent: "Transistors",
            kind: Some(PlaceKind::Jfet),
            icon: "jfet.svg",
            aliases: &[
                "Jfet",
                "Junction Field-Effect Transistor",
                "Transistor",
                "FET",
                "Junction FET",
            ],
        },
        Spec {
            caption: "Other Active",
            parent: "Active",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "OpAmp",
            parent: "Other Active",
            kind: Some(PlaceKind::OpAmp),
            icon: "opamp.svg",
            aliases: &["Op-Amp", "Operational Amplifier", "Amplifier", "Diff Amp"],
        },
        Spec {
            caption: "Comparator",
            parent: "Other Active",
            kind: Some(PlaceKind::Comparator),
            icon: "opamp.svg",
            aliases: &[
                "Voltage Comparator",
                "OpAmp Comparator",
                "Threshold Detector",
            ],
        },
        Spec {
            caption: "Voltage Regulator",
            parent: "Other Active",
            kind: Some(PlaceKind::VoltReg),
            icon: "voltreg.svg",
            aliases: &[
                "Volt. Regulator",
                "VoltReg",
                "LDO",
                "Linear Regulator",
                "Power Supply",
            ],
        },
        Spec {
            caption: "Analog Mux",
            parent: "Other Active",
            kind: Some(PlaceKind::AnalogMux),
            icon: "1to3-c.svg",
            aliases: &[
                "Analog Multiplexer",
                "MuxAnalog",
                "Multiplexer",
                "Switch Matrix",
            ],
        },
        Spec {
            caption: "Outputs",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Leds",
            parent: "Outputs",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Led",
            parent: "Leds",
            kind: Some(PlaceKind::Led),
            icon: "led.svg",
            aliases: &["LED", "Light Emitting Diode", "Diode", "Indicator"],
        },
        Spec {
            caption: "RGB LED",
            parent: "Leds",
            kind: Some(PlaceKind::RgbLed),
            icon: "ledrgb.svg",
            aliases: &[
                "RGB Led",
                "RGB LED",
                "RGBLed",
                "Multicolor LED",
                "Color LED",
            ],
        },
        Spec {
            caption: "Led Bar",
            parent: "Leds",
            kind: Some(PlaceKind::LedBar),
            icon: "ledbar.svg",
            aliases: &["LED Bar", "LedBar", "Bargraph", "LED Array"],
        },
        Spec {
            caption: "7-Segment Display",
            parent: "Leds",
            kind: Some(PlaceKind::SevenSegment),
            icon: "seven_segment.svg",
            aliases: &[
                "7 Segment",
                "Seven Segment",
                "SevenSegment",
                "7-Seg",
                "Numeric Display",
                "Display",
                "Digit",
            ],
        },
        Spec {
            caption: "Led Matrix",
            parent: "Leds",
            kind: Some(PlaceKind::LedMatrix),
            icon: "ledmatrix.svg",
            aliases: &["LED Matrix", "LedMatrix", "Dot Matrix", "Matrix Display"],
        },
        Spec {
            caption: "MAX72xx Matrix",
            parent: "Leds",
            kind: Some(PlaceKind::Max72xx),
            icon: "max72xx.svg",
            aliases: &[
                "Max72xx Matrix",
                "Max72xx",
                "MAX7219",
                "MAX7221",
                "LED Matrix Driver",
                "Matrix Driver",
            ],
        },
        Spec {
            caption: "WS2812 RGB LED",
            parent: "Leds",
            kind: Some(PlaceKind::Ws2812),
            icon: "ws2812.svg",
            aliases: &[
                "WS2812",
                "WS2812B",
                "Neopixel",
                "Addressable LED",
                "RGB LED",
                "WS2812 LED",
            ],
        },
        Spec {
            caption: "Displays",
            parent: "Outputs",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "AIP31068 I2C",
            parent: "Displays",
            kind: Some(PlaceKind::AIP31068),
            icon: "aip31068.svg",
            aliases: &["Aip31068", "I2C LCD", "Character LCD", "Display"],
        },
        Spec {
            caption: "GC9A01A",
            parent: "Displays",
            kind: Some(PlaceKind::GC9A01A),
            icon: "gc9a01a.svg",
            aliases: &["Round LCD", "TFT Display", "SPI Display", "Display"],
        },
        Spec {
            caption: "HD44780",
            parent: "Displays",
            kind: Some(PlaceKind::Hd44780),
            icon: "hd44780.svg",
            aliases: &[
                "Character LCD",
                "LCD Display",
                "Alphanumeric LCD",
                "1602 LCD",
                "Display",
            ],
        },
        Spec {
            caption: "ILI9341",
            parent: "Displays",
            kind: Some(PlaceKind::ILI9341),
            icon: "ili9341.svg",
            aliases: &["TFT LCD", "Color LCD", "SPI LCD", "Display"],
        },
        Spec {
            caption: "KS0108",
            parent: "Displays",
            kind: Some(PlaceKind::KS0108),
            icon: "ks0108.svg",
            aliases: &["Graphic LCD", "GLCD", "128x64 LCD", "Display"],
        },
        Spec {
            caption: "PCD8544",
            parent: "Displays",
            kind: Some(PlaceKind::PCD8544),
            icon: "pcd8544.svg",
            aliases: &["Nokia 5110", "Nokia LCD", "Graphic LCD", "Display"],
        },
        Spec {
            caption: "PCF8833",
            parent: "Displays",
            kind: Some(PlaceKind::PCF8833),
            icon: "pcf8833.svg",
            aliases: &["Color LCD", "Nokia 6100", "Display"],
        },
        Spec {
            caption: "SH1107",
            parent: "Displays",
            kind: Some(PlaceKind::SH1107),
            icon: "sh1107.svg",
            aliases: &["OLED Display", "128x128 OLED", "Display"],
        },
        Spec {
            caption: "SSD1306",
            parent: "Displays",
            kind: Some(PlaceKind::SSD1306),
            icon: "ssd1306.svg",
            aliases: &[
                "OLED Display",
                "I2C OLED",
                "0.96 OLED",
                "128x64 OLED",
                "Display",
            ],
        },
        Spec {
            caption: "ST7735",
            parent: "Displays",
            kind: Some(PlaceKind::ST7735),
            icon: "st7735.svg",
            aliases: &["TFT Display", "Color TFT", "1.8 TFT", "Display"],
        },
        Spec {
            caption: "ST7789",
            parent: "Displays",
            kind: Some(PlaceKind::ST7789),
            icon: "st7789.svg",
            aliases: &["IPS Display", "Color TFT", "240x240 Display", "Display"],
        },
        Spec {
            caption: "Motors",
            parent: "Outputs",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "DC Motor",
            parent: "Motors",
            kind: Some(PlaceKind::DcMotor),
            icon: "dcmotor.svg",
            aliases: &["DcMotor", "Motor", "Direct Current Motor", "Actuator"],
        },
        Spec {
            caption: "Stepper Motor",
            parent: "Motors",
            kind: Some(PlaceKind::Stepper),
            icon: "steeper.svg",
            aliases: &["Stepper", "Stepper Motor", "Step Motor", "Motor"],
        },
        Spec {
            caption: "Servo Motor",
            parent: "Motors",
            kind: Some(PlaceKind::Servo),
            icon: "servo.svg",
            aliases: &["Servo", "Servo Motor", "RC Servo", "Actuator"],
        },
        Spec {
            caption: "Other Outputs",
            parent: "Outputs",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Audio Out",
            parent: "Other Outputs",
            kind: Some(PlaceKind::AudioOut),
            icon: "audio_out.svg",
            aliases: &[
                "AudioOut",
                "Speaker",
                "Buzzer",
                "Sound",
                "Beeper",
                "Audio Output",
            ],
        },
        Spec {
            caption: "Incandescent Lamp",
            parent: "Other Outputs",
            kind: Some(PlaceKind::Lamp),
            icon: "lamp.svg",
            aliases: &["Lamp", "Light Bulb", "Bulb", "Incandescent", "Light"],
        },
        Spec {
            caption: "Micro",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "MCU",
            parent: "Micro",
            kind: Some(PlaceKind::Mcu),
            icon: "ic2.svg",
            aliases: &["Microcontroller", "Microprocessor", "CPU", "Processor"],
        },
        Spec {
            caption: "Arduino",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "AVR",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "PIC",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "I51",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "MCS65",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Z80",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "STM32",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Espressif",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Esp32",
            parent: "Espressif",
            kind: Some(PlaceKind::QemuDevice),
            icon: "ic2.svg",
            aliases: &[
                "ESP32",
                "ESP-32",
                "ESP8266",
                "QEMU",
                "Microcontroller",
                "MCU",
                "Wi-Fi",
                "Bluetooth",
            ],
        },
        Spec {
            caption: "Shields",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Sensors",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "HC-SR04 Ultrasonic Sensor",
            parent: "Sensors",
            kind: Some(PlaceKind::SR04),
            icon: "sr04.svg",
            aliases: &[
                "SR04",
                "HC-SR04",
                "Ultrasonic Sensor",
                "Distance Sensor",
                "Sonar",
                "Rangefinder",
            ],
        },
        Spec {
            caption: "DHT22 Temperature & Humidity Sensor",
            parent: "Sensors",
            kind: Some(PlaceKind::DHT22),
            icon: "dht22.svg",
            aliases: &[
                "DHT22",
                "AM2302",
                "Humidity Sensor",
                "Temperature Sensor",
                "DHT11",
                "Climate Sensor",
            ],
        },
        Spec {
            caption: "DS1621 I2C Temperature Sensor",
            parent: "Sensors",
            kind: Some(PlaceKind::DS1621),
            icon: "dsxxx_ico.svg",
            aliases: &["DS1621", "I2C Temperature Sensor", "Digital Thermometer"],
        },
        Spec {
            caption: "DS18B20 1-Wire Temperature Sensor",
            parent: "Sensors",
            kind: Some(PlaceKind::DS18B20),
            icon: "dsxxx_ico.svg",
            aliases: &[
                "DS18B20",
                "1-Wire Temperature Sensor",
                "Dallas Temperature",
                "Thermal Sensor",
            ],
        },
        Spec {
            caption: "Peripherals",
            parent: "Micro",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "SD Card Reader",
            parent: "Peripherals",
            kind: Some(PlaceKind::SdCard),
            icon: "sdcard.svg",
            aliases: &[
                "SD Card",
                "SD Card Reader",
                "SdCard",
                "MicroSD",
                "Flash Memory",
                "SPI Memory",
            ],
        },
        Spec {
            caption: "Serial Port",
            parent: "Peripherals",
            kind: Some(PlaceKind::SerialPort),
            icon: "serialport.svg",
            aliases: &["SerialPort", "UART", "COM Port", "TTY", "RS232"],
        },
        Spec {
            caption: "Serial Terminal",
            parent: "Peripherals",
            kind: Some(PlaceKind::SerialTerm),
            icon: "serialterm.svg",
            aliases: &["SerialTerm", "Console", "TTY Terminal", "Monitor"],
        },
        Spec {
            caption: "TouchPad (Resistive)",
            parent: "Peripherals",
            kind: Some(PlaceKind::TouchPad),
            icon: "touch.svg",
            aliases: &[
                "TouchPad",
                "TouchPad (Resistive)",
                "Capacitive Touch",
                "Touch Sensor",
                "Keypad",
            ],
        },
        Spec {
            caption: "Joystick Dual Axis (KY-023)",
            parent: "Peripherals",
            kind: Some(PlaceKind::KY023),
            icon: "ky-023.svg",
            aliases: &[
                "KY023",
                "KY-023",
                "Dual-Axis Joystick",
                "Analog Joystick",
                "Thumbstick",
                "Joystick Dual Axis",
            ],
        },
        Spec {
            caption: "Rotary Encoder (KY-040)",
            parent: "Peripherals",
            kind: Some(PlaceKind::KY040),
            icon: "ky-040.svg",
            aliases: &[
                "KY040",
                "KY-040",
                "Rotary Encoder",
                "Encoder",
                "Knob",
                "Rotary Encoder (relative)",
            ],
        },
        Spec {
            caption: "DS1307 Real Time Clock",
            parent: "Peripherals",
            kind: Some(PlaceKind::DS1307),
            icon: "ic2.svg",
            aliases: &[
                "DS1307",
                "RTC",
                "Real Time Clock",
                "I2C Clock",
                "Clock Calendar",
            ],
        },
        Spec {
            caption: "ESP-01 Wi-Fi Module",
            parent: "Peripherals",
            kind: Some(PlaceKind::Esp01),
            icon: "esp01_ico.svg",
            aliases: &[
                "ESP-01",
                "Esp01",
                "ESP8266",
                "WiFi Module",
                "Wireless Serial",
                "Esp01 (TCP)",
            ],
        },
        Spec {
            caption: "Logic",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Gates",
            parent: "Logic",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Buffer",
            parent: "Gates",
            kind: Some(PlaceKind::BufferGate),
            icon: "buffer.svg",
            aliases: &["Non-inverting Buffer", "Logic Buffer", "Driver", "Gate"],
        },
        Spec {
            caption: "And Gate",
            parent: "Gates",
            kind: Some(PlaceKind::AndGate),
            icon: "andgate.svg",
            aliases: &["AndGate", "AND", "AND Gate", "Logic Gate", "Gate"],
        },
        Spec {
            caption: "Or Gate",
            parent: "Gates",
            kind: Some(PlaceKind::OrGate),
            icon: "orgate.svg",
            aliases: &["OrGate", "OR", "OR Gate", "Logic Gate", "Gate"],
        },
        Spec {
            caption: "Xor Gate",
            parent: "Gates",
            kind: Some(PlaceKind::XorGate),
            icon: "xorgate.svg",
            aliases: &["XorGate", "XOR", "Exclusive OR", "Logic Gate", "Gate"],
        },
        Spec {
            caption: "Arithmetic",
            parent: "Logic",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Simple Counter",
            parent: "Arithmetic",
            kind: Some(PlaceKind::Counter),
            icon: "2to1.svg",
            aliases: &["Counter", "Simple Counter", "Digital Counter", "Divider"],
        },
        Spec {
            caption: "Binary Counter",
            parent: "Arithmetic",
            kind: Some(PlaceKind::BinCounter),
            icon: "2to3g.svg",
            aliases: &["BinCounter", "Ripple Counter", "4-bit Counter"],
        },
        Spec {
            caption: "Full Adder",
            parent: "Arithmetic",
            kind: Some(PlaceKind::FullAdder),
            icon: "2to2.svg",
            aliases: &["FullAdder", "Adder", "Binary Adder", "Arithmetic"],
        },
        Spec {
            caption: "Magnitude Comparator",
            parent: "Arithmetic",
            kind: Some(PlaceKind::MagnitudeComp),
            icon: "3to2g.svg",
            aliases: &["MagnitudeComp", "Digital Comparator", "Bit Comparator"],
        },
        Spec {
            caption: "Shift Register",
            parent: "Arithmetic",
            kind: Some(PlaceKind::ShiftReg),
            icon: "1to3.svg",
            aliases: &["ShiftReg", "SIPO", "PISO", "Register"],
        },
        Spec {
            caption: "Function",
            parent: "Arithmetic",
            kind: Some(PlaceKind::Function),
            icon: "subc.svg",
            aliases: &["Logic Function", "Boolean Function", "Custom Logic"],
        },
        Spec {
            caption: "Memory",
            parent: "Logic",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Flip-Flop D",
            parent: "Memory",
            kind: Some(PlaceKind::FlipFlopD),
            icon: "2to2.svg",
            aliases: &[
                "FlipFlopD",
                "D Flip-Flop",
                "D Flip Flop",
                "D-Type",
                "Register",
                "Memory",
            ],
        },
        Spec {
            caption: "Flip-Flop T",
            parent: "Memory",
            kind: Some(PlaceKind::FlipFlopT),
            icon: "2to2.svg",
            aliases: &["FlipFlopT", "T Flip-Flop", "Toggle Flip-Flop"],
        },
        Spec {
            caption: "Flip-Flop RS",
            parent: "Memory",
            kind: Some(PlaceKind::FlipFlopRS),
            icon: "2to2.svg",
            aliases: &["FlipFlopRS", "RS Flip-Flop", "SR Latch", "Set-Reset Latch"],
        },
        Spec {
            caption: "Flip-Flop JK",
            parent: "Memory",
            kind: Some(PlaceKind::FlipFlopJK),
            icon: "3to2.svg",
            aliases: &["FlipFlopJK", "JK Flip-Flop", "Universal Flip-Flop"],
        },
        Spec {
            caption: "Latch D",
            parent: "Memory",
            kind: Some(PlaceKind::LatchD),
            icon: "subc.svg",
            aliases: &["LatchD", "D Latch", "Transparent Latch", "Data Latch"],
        },
        Spec {
            caption: "RAM/ROM Memory",
            parent: "Memory",
            kind: Some(PlaceKind::Memory),
            icon: "2to3g.svg",
            aliases: &["Memory", "RAM", "ROM", "Ram/Rom", "Memory Array"],
        },
        Spec {
            caption: "Dynamic RAM",
            parent: "Memory",
            kind: Some(PlaceKind::DynamicMemory),
            icon: "2to3g.svg",
            aliases: &["DynamicMemory", "Dynamic Ram", "DRAM", "Memory"],
        },
        Spec {
            caption: "I2C RAM",
            parent: "Memory",
            kind: Some(PlaceKind::I2CRam),
            icon: "2to3.svg",
            aliases: &["I2CRam", "I2C Memory", "EEPROM", "24C02"],
        },
        Spec {
            caption: "Converters",
            parent: "Logic",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Mux",
            parent: "Converters",
            kind: Some(PlaceKind::Mux),
            icon: "mux.svg",
            aliases: &["Multiplexer", "Digital Mux", "Data Selector"],
        },
        Spec {
            caption: "Demux",
            parent: "Converters",
            kind: Some(PlaceKind::Demux),
            icon: "demux.svg",
            aliases: &["Demultiplexer", "Decoder", "Data Distributor"],
        },
        Spec {
            caption: "BCD to Decimal Decoder",
            parent: "Converters",
            kind: Some(PlaceKind::BcdToDec),
            icon: "2to3g.svg",
            aliases: &[
                "BCD to Dec",
                "BcdToDec",
                "Decoder(4 to 10/16)",
                "BCD Decoder",
                "1-of-10 Decoder",
            ],
        },
        Spec {
            caption: "Decimal to BCD Priority Encoder",
            parent: "Converters",
            kind: Some(PlaceKind::DecToBcd),
            icon: "3to2g.svg",
            aliases: &[
                "Dec to BCD",
                "DecToBcd",
                "Encoder(10/16 to 4)",
                "Priority Encoder",
            ],
        },
        Spec {
            caption: "BCD to 7-Segment Decoder",
            parent: "Converters",
            kind: Some(PlaceKind::BcdTo7Segment),
            icon: "2to3g.svg",
            aliases: &[
                "BCD to 7S",
                "BcdTo7S",
                "7-Segment Decoder",
                "BCD to 7-Segment",
            ],
        },
        Spec {
            caption: "I2C to Parallel",
            parent: "Converters",
            kind: Some(PlaceKind::I2CToParallel),
            icon: "2to3g.svg",
            aliases: &["I2CToParallel", "PCF8574", "I/O Expander", "Port Expander"],
        },
        Spec {
            caption: "Other Logic",
            parent: "Logic",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "ADC",
            parent: "Other Logic",
            kind: Some(PlaceKind::Adc),
            icon: "1to3.svg",
            aliases: &[
                "Analog to Digital Converter",
                "A/D Converter",
                "Analog Input",
            ],
        },
        Spec {
            caption: "DAC",
            parent: "Other Logic",
            kind: Some(PlaceKind::Dac),
            icon: "3to1.svg",
            aliases: &[
                "Digital to Analog Converter",
                "D/A Converter",
                "Analog Output",
            ],
        },
        Spec {
            caption: "7-Segment Display with BCD Decoder",
            parent: "Other Logic",
            kind: Some(PlaceKind::SevenSegmentBCD),
            icon: "7segbcd.svg",
            aliases: &[
                "7 Segment BCD",
                "SevenSegmentBCD",
                "7 Seg BCD",
                "7-Segment Display with Decoder",
            ],
        },
        Spec {
            caption: "LM555 Timer",
            parent: "Other Logic",
            kind: Some(PlaceKind::Lm555),
            icon: "ic2.svg",
            aliases: &[
                "555",
                "LM555",
                "NE555",
                "555 Timer",
                "Timer IC",
                "Timer",
                "Oscillator",
            ],
        },
        Spec {
            caption: "Connectors",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Bus",
            parent: "Connectors",
            kind: Some(PlaceKind::Bus),
            icon: "bus.svg",
            aliases: &["Data Bus", "Wire Bus", "Bus Line", "Connector"],
        },
        Spec {
            caption: "Tunnel",
            parent: "Connectors",
            kind: Some(PlaceKind::Tunnel),
            icon: "tunnel.svg",
            aliases: &["Net Label", "Named Net", "Wireless Connection", "Connector"],
        },
        Spec {
            caption: "Socket",
            parent: "Connectors",
            kind: Some(PlaceKind::Socket),
            icon: "socket.svg",
            aliases: &["Connector Socket", "Female Header", "Connector"],
        },
        Spec {
            caption: "Header",
            parent: "Connectors",
            kind: Some(PlaceKind::Header),
            icon: "header.svg",
            aliases: &["Pin Header", "Male Header", "Breakout", "Connector"],
        },
        Spec {
            caption: "Graphical",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Image",
            parent: "Graphical",
            kind: Some(PlaceKind::Image),
            icon: "img.svg",
            aliases: &["Graphic", "Picture", "Bitmap", "Photo"],
        },
        Spec {
            caption: "Text",
            parent: "Graphical",
            kind: Some(PlaceKind::TextComponent),
            icon: "text.svg",
            aliases: &["TextComponent", "Annotation", "Label", "Comment"],
        },
        Spec {
            caption: "Rectangle",
            parent: "Graphical",
            kind: Some(PlaceKind::Rectangle),
            icon: "rectangle.svg",
            aliases: &["Box", "Graphic", "Shape"],
        },
        Spec {
            caption: "Ellipse",
            parent: "Graphical",
            kind: Some(PlaceKind::Ellipse),
            icon: "ellipse.svg",
            aliases: &["Circle", "Graphic", "Shape"],
        },
        Spec {
            caption: "Line",
            parent: "Graphical",
            kind: Some(PlaceKind::Line),
            icon: "line.svg",
            aliases: &["Separator", "Graphic", "Shape"],
        },
        Spec {
            caption: "Other",
            parent: "",
            kind: None,
            icon: "",
            aliases: &[],
        },
        Spec {
            caption: "Subcircuit Package",
            parent: "Other",
            kind: Some(PlaceKind::SubPackage),
            icon: "ic2.svg",
            aliases: &[
                "SubPackage",
                "IC Package",
                "DIP Chip",
                "Custom Chip",
                "Package",
            ],
        },
        Spec {
            caption: "Test Unit",
            parent: "Other",
            kind: Some(PlaceKind::TestUnit),
            icon: "bug.svg",
            aliases: &["TestUnit", "Automated Testing", "Testbench", "Unit Test"],
        },
        Spec {
            caption: "Dial",
            parent: "Other",
            kind: Some(PlaceKind::Dial),
            icon: "dial.svg",
            aliases: &["Control Knob", "Potentiometer Dial", "Input Dial"],
        },
    ];

    let mut items: Vec<LibraryItem> = Vec::with_capacity(SPECS.len());
    for (i, spec) in SPECS.iter().enumerate() {
        let id = i as u32 + 1;
        let parent = if spec.parent.is_empty() {
            None
        } else {
            items
                .iter()
                .rev()
                .find(|it| it.comp_type == spec.parent && it.item_type != TYPE_COMPONENT)
                .map(|it| it.id)
        };
        let item_type = if spec.kind.is_some() {
            TYPE_COMPONENT
        } else if spec.parent.is_empty() {
            TYPE_CATEG_MAIN
        } else {
            TYPE_CATEG_CHILD
        };
        let comp_type = spec
            .kind
            .map(|k| k.as_str().to_string())
            .unwrap_or_else(|| spec.caption.to_string());
        items.push(LibraryItem {
            id,
            caption: spec.caption.to_string(),
            parent,
            item_type,
            comp_type,
            kind: spec.kind,
            expanded: false,
            icon: spec.icon.to_string(),
            aliases: spec.aliases.iter().map(|&s| s.to_string()).collect(),
        });
    }
    items
}

/// Return `(caption, comp_type)` for every placeable component in the catalog.
/// This is the **single source of truth** used by both the sidebar list and
/// `Canvas::add_all_components()`. Category headers are excluded.
pub fn all_placeable_specs() -> Vec<(String, String)> {
    let mut lib = Library::new();
    lib.add_catalog_mcus(&crate::catalog::standard());
    lib.items()
        .iter()
        .filter(|it| it.item_type == TYPE_COMPONENT)
        .map(|it| (it.caption.clone(), it.comp_type.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistor_is_under_passive() {
        let lib = Library::new();
        let rows = lib.visible_rows();
        let names: Vec<_> = rows.iter().map(|r| r.caption.as_str()).collect();
        assert!(names.contains(&"Sources"));
        assert!(names.contains(&"Passive"));
        assert!(names.contains(&"Resistors"));
        assert!(names.contains(&"Resistor"));
        let r = rows.iter().find(|r| r.comp_type == "Resistor").unwrap();
        assert_eq!(r.item_type, TYPE_COMPONENT);
        assert_eq!(r.depth, 2);
    }

    #[test]
    fn search_finds_oscope_and_ancestors() {
        let mut lib = Library::new();
        lib.set_filter("oscope");
        let rows = lib.visible_rows();
        let names: Vec<_> = rows.iter().map(|r| r.caption.as_str()).collect();
        assert_eq!(names, vec!["Meters", "Oscilloscope"]);
    }

    #[test]
    fn collapse_hides_children() {
        let mut lib = Library::new();
        let passive = lib
            .visible_rows()
            .iter()
            .find(|r| r.comp_type == "Passive")
            .unwrap()
            .id;
        lib.set_expanded(passive, false);
        let rows = lib.visible_rows();
        let names: Vec<_> = rows.iter().map(|r| r.caption.as_str()).collect();
        assert!(!names.contains(&"Resistor"));
        assert!(names.contains(&"Passive"));
    }

    #[test]
    fn zener_catalog_id_matches_sim1() {
        let lib = Library::new();
        let z = lib
            .items
            .iter()
            .find(|i| i.caption == "Zener Diode")
            .unwrap();
        assert_eq!(z.comp_type, "Zener");
    }

    #[test]
    fn opamp_is_listed() {
        let lib = Library::new();
        let rows = lib.visible_rows();
        let oa = rows.iter().find(|r| r.comp_type == "OpAmp").unwrap();
        assert_eq!(oa.caption, "OpAmp");
        assert_eq!(oa.item_type, TYPE_COMPONENT);
    }

    #[test]
    fn jfet_catalog_id_matches_sim1() {
        let lib = Library::new();
        let j = lib.items.iter().find(|i| i.caption == "JFET").unwrap();
        assert_eq!(j.comp_type, "Jfet");
    }

    #[test]
    fn comparator_catalog_id_matches_sim1() {
        let lib = Library::new();
        let c = lib
            .items
            .iter()
            .find(|i| i.caption == "Comparator")
            .unwrap();
        assert_eq!(c.comp_type, "Comparator");
    }

    #[test]
    fn voltreg_catalog_id_matches_sim1() {
        let lib = Library::new();
        let v = lib
            .items
            .iter()
            .find(|i| i.caption == "Voltage Regulator")
            .unwrap();
        assert_eq!(v.comp_type, "VoltReg");
    }

    #[test]
    fn catalog_xml_mcus_land_under_family_categories() {
        let mut cat = crate::catalog::Catalog::new();
        cat.load_xml_str(
            r#"<itemlib>
            <itemset category="AVR/attiny" type="MCU">
                <item name="tiny13" data="AVR/tiny13" />
            </itemset>
            <itemset category="PIC/p16F" type="MCU">
                <item name="p16F84" data="PIC/p16F84" />
            </itemset>
            </itemlib>"#,
            std::path::Path::new("/lib/avr.xml"),
        );
        let mut lib = Library::new();
        lib.add_catalog_mcus(&cat);
        lib.set_filter("tiny13");
        let names: Vec<(String, String, i32)> = lib
            .visible_rows()
            .into_iter()
            .map(|r| (r.caption, r.comp_type, r.item_type))
            .collect();
        assert!(
            names
                .iter()
                .any(|(c, t, k)| c == "tiny13" && t == "MCU" && *k == TYPE_COMPONENT),
            "{names:?}"
        );
        assert!(names.iter().any(|(c, _, _)| c == "AVR"));
        assert!(names.iter().any(|(c, _, _)| c == "attiny"));
        lib.set_filter("p16F84");
        let names: Vec<String> = lib.visible_rows().into_iter().map(|r| r.caption).collect();
        assert!(names.iter().any(|c| c == "PIC"));
        assert!(names.iter().any(|c| c == "p16F"));
        assert!(names.iter().any(|c| c == "p16F84"));
    }

    #[test]
    fn search_transistor_finds_bjt_mosfet_jfet() {
        let mut lib = Library::new();
        lib.set_filter("transistor");
        let rows = lib.visible_rows();
        let names: Vec<_> = rows.iter().map(|r| r.caption.as_str()).collect();
        assert!(names.contains(&"Active"), "{names:?}");
        assert!(names.contains(&"Transistors"), "{names:?}");
        assert!(names.contains(&"BJT"), "{names:?}");
        assert!(names.contains(&"MOSFET"), "{names:?}");
        assert!(names.contains(&"JFET"), "{names:?}");
    }

    #[test]
    fn search_npn_finds_bjt() {
        let mut lib = Library::new();
        lib.set_filter("npn");
        let rows = lib.visible_rows();
        let names: Vec<_> = rows.iter().map(|r| r.caption.as_str()).collect();
        assert!(names.contains(&"BJT"), "{names:?}");
    }

    #[test]
    fn text_match_tier_works_correctly() {
        assert_eq!(text_match_tier("BJT", "bjt"), Some(0)); // Exact
        assert_eq!(
            text_match_tier("Bipolar Junction Transistor", "transistor"),
            Some(1)
        ); // Whole word
        assert_eq!(text_match_tier("Electrolytic Capacitor", "elec"), Some(2)); // Word prefix
        assert_eq!(text_match_tier("Capacitor", "paci"), Some(3)); // Substring
        assert_eq!(text_match_tier("Resistor", "xyz"), None); // No match
    }
}
