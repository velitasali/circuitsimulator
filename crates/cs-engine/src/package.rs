//! Chip package layout: `.sim1` Package items and `.package` XML.
//!
//! Matches C++ `Chip::getPackages` / `Chip::convertPackage` / `Chip::initPackage`
//! and `SubPackage` package designer functionality.

use std::collections::BTreeMap;
use std::path::Path;

use crate::sim1::{parse_props, parse_xml_props};

/// One pin on a package (C++ `Chip::setPinStr` / `PackagePin`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PkgPin {
    pub id: String,
    pub label: String,
    pub pin_type: String,
    pub xpos: i32,
    pub ypos: i32,
    pub angle: i32,
    pub length: i32,
    pub space: i32,
}

impl PkgPin {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        pin_type: impl Into<String>,
        xpos: i32,
        ypos: i32,
        angle: i32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            pin_type: pin_type.into(),
            xpos,
            ypos,
            angle,
            length: 8,
            space: 0,
        }
    }

    pub fn unused(&self) -> bool {
        let t = self.pin_type.to_ascii_lowercase();
        t == "nc" || t == "unused"
    }

    pub fn is_bus(&self) -> bool {
        self.pin_type.eq_ignore_ascii_case("bus")
    }

    pub fn inverted(&self) -> bool {
        let t = self.pin_type.to_ascii_lowercase();
        t == "inv" || t == "inverted"
    }

    pub fn is_point(&self) -> bool {
        self.length < 7
    }

    pub fn set_point(&mut self, point: bool) {
        if point {
            self.length = 1;
        } else {
            self.length = 8;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SubcType {
    #[default]
    None,
    MCU,
    Logic,
    Board,
    Shield,
    Other,
}

impl SubcType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::MCU => "MCU",
            Self::Logic => "Logic",
            Self::Board => "Board",
            Self::Shield => "Shield",
            Self::Other => "Other",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        let clean = s.trim().replace("subc", "");
        match clean.to_ascii_lowercase().as_str() {
            "mcu" => Self::MCU,
            "logic" => Self::Logic,
            "board" => Self::Board,
            "shield" => Self::Shield,
            "other" => Self::Other,
            _ => Self::None,
        }
    }
}

/// Package body + pins. Size is in 8-pixel units (`m_area = 8*width × 8*height`).
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub subc_type: SubcType,
    pub logic_symbol: bool,
    pub border: bool,
    pub custom_color: bool,
    pub bckgndcolor: String,
    pub background: String,
    pub package_file: String,
    pub pins: Vec<PkgPin>,
}

impl Default for Package {
    fn default() -> Self {
        Self {
            name: String::new(),
            width: 4,
            height: 8,
            subc_type: SubcType::None,
            logic_symbol: false,
            border: false,
            custom_color: false,
            bckgndcolor: String::new(),
            background: String::new(),
            package_file: String::new(),
            pins: Vec::new(),
        }
    }
}

impl Package {
    pub fn body_w(&self) -> f64 {
        (self.width.max(1) * 8) as f64
    }

    pub fn body_h(&self) -> f64 {
        (self.height.max(1) * 8) as f64
    }

    pub fn electrical_pins(&self) -> impl Iterator<Item = &PkgPin> {
        self.pins.iter().filter(|p| !p.unused())
    }

    pub fn find_pin(&self, id: &str) -> Option<&PkgPin> {
        self.pins.iter().find(|p| p.id == id)
    }

    pub fn find_pin_mut(&mut self, id: &str) -> Option<&mut PkgPin> {
        self.pins.iter_mut().find(|p| p.id == id)
    }
}

/// C++ `Chip::cleanPinName`.
pub fn clean_pin_name(name: &str) -> String {
    name.replace("&#x3D;", "=")
        .replace("&#x3C;", "<")
        .replace("&#x3E;", ">")
        .replace("&#x3D", "=")
        .replace("&#x3C", "<")
        .replace("&#x3E", ">")
        .replace("&lt;", "<")
}

/// C++ `Chip::getPackages`: Package items at the top of a `.sim1` / `.sim2`.
/// Stops at the first non-Package item. Key is the package `label`.
pub fn packages_from_sim1(src: &str) -> (BTreeMap<String, Package>, String) {
    let mut list = BTreeMap::new();
    let mut subc_type = SubcType::None;
    for raw in src.lines() {
        let line = raw.trim();
        if !line.starts_with("<item") {
            continue;
        }
        let mut properties = parse_xml_props(line);
        if properties.is_empty() {
            continue;
        }
        let (n0, type_) = properties.remove(0);
        if n0 != "itemtype" {
            continue;
        }
        if type_ != "Package" && type_ != "SubPackage" {
            break;
        }
        let mut pkg_name = String::new();
        let mut pkg_str = String::from("Package; ");
        let mut add = false;
        for (prop_name, mut prop_value) in properties {
            if prop_name == "SubcType" {
                let st = SubcType::from_str_name(&prop_value);
                if st != SubcType::None {
                    subc_type = st;
                }
            } else if prop_name == "label" || prop_name == "Name" {
                if !prop_value.is_empty() {
                    pkg_name = prop_value.clone();
                }
            }
            if prop_name == "Pins" {
                prop_value = prop_value.replace("&#xa;", "\n");
                if prop_value.contains('&') {
                    prop_value = clean_pin_name(&prop_value);
                }
                pkg_str.push('\n');
                pkg_str.push_str(&prop_value);
                add = true;
            } else {
                pkg_str.push_str(&prop_name);
                pkg_str.push('=');
                pkg_str.push_str(&prop_value);
                pkg_str.push_str("; ");
            }
        }
        if add && !pkg_name.is_empty() {
            let mut pkg = parse_package_str(&pkg_str);
            if pkg.subc_type == SubcType::None && subc_type != SubcType::None {
                pkg.subc_type = subc_type;
            }
            pkg.name = pkg_name.clone();
            list.insert(pkg_name, pkg);
        }
    }
    (list, subc_type.as_str().to_string())
}

/// C++ `Chip::convertPackage`: `.package` XML → internal `Package; …` / `Pin; …`.
pub fn convert_package(pkg_text: &str) -> (String, Package) {
    let mut pkg_str = String::new();
    let mut subc_type = SubcType::None;
    for line in pkg_text.lines() {
        let line = line.trim();
        if line.starts_with("<!") || line.starts_with("</") || line.is_empty() {
            continue;
        }
        let properties = parse_xml_props(line);
        if line.starts_with("<package") {
            pkg_str.push_str("Package; ");
            for (name, mut value) in properties {
                if name == "type" {
                    value = value.replace("subc", "");
                    subc_type = SubcType::from_str_name(&value);
                }
                pkg_str.push_str(&name);
                pkg_str.push('=');
                pkg_str.push_str(&value);
                pkg_str.push_str("; ");
            }
        } else if !properties.is_empty() {
            pkg_str.push_str("Pin; ");
            for (name, mut value) in properties {
                if value.contains('&') {
                    value = clean_pin_name(&value);
                }
                pkg_str.push_str(&name);
                pkg_str.push('=');
                pkg_str.push_str(&value);
                pkg_str.push_str("; ");
            }
        }
        pkg_str.push('\n');
    }
    let mut pkg = parse_package_str(&pkg_str);
    if pkg.subc_type == SubcType::None {
        pkg.subc_type = subc_type;
    }
    (pkg_str, pkg)
}

/// C++ `Chip::initPackage` over the converted `Package; …\nPin; …` text.
pub fn parse_package_str(pkg_str: &str) -> Package {
    let mut pkg = Package::default();
    for line in pkg_str.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut properties = parse_props(line);
        if properties.is_empty() {
            break;
        }
        let item = properties.remove(0).0;
        if item == "Package" {
            for (name, val) in properties {
                let n = name.to_ascii_lowercase();
                let first = val.split(' ').next().unwrap_or("");
                match n.as_str() {
                    "width" => pkg.width = first.parse().unwrap_or(pkg.width),
                    "height" => pkg.height = first.parse().unwrap_or(pkg.height),
                    "name" => {
                        if val != "Package" {
                            pkg.name = val;
                        }
                    }
                    "border" => pkg.border = val == "true",
                    "logic_symbol" => pkg.logic_symbol = val == "true",
                    "custom_color" => pkg.custom_color = val == "true",
                    "bckgndcolor" => pkg.bckgndcolor = val,
                    "background" => pkg.background = val,
                    "package_file" => pkg.package_file = val,
                    "type" => {
                        let t = val.replace("subc", "");
                        if !t.is_empty() {
                            pkg.subc_type = SubcType::from_str_name(&t);
                        }
                    }
                    _ => {}
                }
            }
        } else if item == "Pin" {
            pkg.pins.push(parse_pin_props(&properties));
        }
    }
    pkg
}

fn parse_pin_props(properties: &[(String, String)]) -> PkgPin {
    let mut pin = PkgPin {
        id: String::new(),
        label: String::new(),
        pin_type: String::new(),
        xpos: 0,
        ypos: 8,
        angle: 0,
        length: 8,
        space: 0,
    };
    for (name, val) in properties {
        match name.as_str() {
            "xpos" => pin.xpos = val.parse().unwrap_or(pin.xpos),
            "ypos" => pin.ypos = val.parse().unwrap_or(pin.ypos),
            "angle" => pin.angle = val.parse().unwrap_or(pin.angle),
            "length" => pin.length = val.parse().unwrap_or(pin.length),
            "space" => pin.space = val.parse().unwrap_or(pin.space),
            "id" => pin.id = val.trim().to_string(),
            "label" => pin.label = val.clone(),
            "type" => pin.pin_type = val.clone(),
            _ => {}
        }
    }
    if pin.id.is_empty() {
        pin.id = pin.label.clone();
    }
    pin
}

/// Export a `Package` as `.package` XML format.
pub fn package_to_xml(pkg: &Package) -> String {
    let mut out = String::new();
    out.push_str("<!-- This file was generated by Circuit Simulator -->\n\n");
    let custom_color_str = if pkg.custom_color { "true" } else { "false" };
    let border_str = if pkg.border { "true" } else { "false" };
    let ls_str = if pkg.logic_symbol { "true" } else { "false" };
    let bg_color = if pkg.bckgndcolor.is_empty() {
        "#323246"
    } else {
        &pkg.bckgndcolor
    };

    out.push_str(&format!(
        "<packageB name=\"{}\" width=\"{}\" height=\"{}\" background=\"{}\" custom_color=\"{}\" bckgndcolor=\"{}\" border=\"{}\" type=\"{}\" logic_symbol=\"{}\" >\n\n",
        pkg.name, pkg.width, pkg.height, pkg.background, custom_color_str, bg_color, border_str, pkg.subc_type.as_str(), ls_str
    ));

    for pin in &pkg.pins {
        let ptype = if pin.unused() {
            "nc"
        } else if pin.is_bus() {
            "bus"
        } else if pin.inverted() {
            "inv"
        } else if pin.pin_type == "nul" {
            "nul"
        } else if pin.pin_type == "rst" {
            "rst"
        } else {
            ""
        };
        let escaped_label = pin
            .label
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let escaped_id = pin
            .id
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        out.push_str(&format!(
            "    <pin type=\"{}\" xpos=\"{}\" ypos=\"{}\" angle=\"{}\" length=\"{}\" space=\"{}\" id=\"{}\" label=\"{}\" />\n",
            ptype, pin.xpos, pin.ypos, pin.angle, pin.length, pin.space, escaped_id, escaped_label
        ));
    }

    out.push_str("\n</packageB>\n");
    out
}

/// Save a `Package` to a `.package` file on disk.
pub fn save_package_file(pkg: &Package, path: &Path) -> std::io::Result<()> {
    let xml = package_to_xml(pkg);
    std::fs::write(path, xml)
}

/// Load a `Package` from a `.package` file on disk.
pub fn load_package_file(path: &Path) -> std::io::Result<Package> {
    let content = std::fs::read_to_string(path)?;
    let (_, pkg) = convert_package(&content);
    Ok(pkg)
}

/// Convert a list of pins to `.sim1` `Pins` property format.
pub fn package_pins_to_sim1_prop(pins: &[PkgPin]) -> String {
    let mut out = String::new();
    for pin in pins {
        let ptype = if pin.unused() {
            "nc"
        } else if pin.is_bus() {
            "bus"
        } else if pin.inverted() {
            "inv"
        } else if pin.pin_type == "nul" {
            "nul"
        } else if pin.pin_type == "rst" {
            "rst"
        } else {
            ""
        };
        out.push_str(&format!(
            "Pin; type={}; xpos={}; ypos={}; angle={}; length={}; space={}; id={}; label={}&#xa;",
            ptype, pin.xpos, pin.ypos, pin.angle, pin.length, pin.space, pin.id, pin.label
        ));
    }
    out
}

/// Parse `.sim1` `Pins` property format to a `Vec<PkgPin>`.
pub fn parse_sim1_package_pins(pins_str: &str) -> Vec<PkgPin> {
    let mut pins = Vec::new();
    for part in pins_str.split("&#xa;") {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut props = parse_props(trimmed);
        if props.is_empty() {
            continue;
        }
        let item = props.remove(0).0;
        if item == "Pin" {
            pins.push(parse_pin_props(&props));
        }
    }
    pins
}

/// Bulk pin generator matching C++ `SubPackage::generatePins`.
/// Places pins on left (angle 180), right (0), top (90), bottom (270).
/// Resizes package width and height as needed to fit the pin counts without overlapping corners.
pub fn generate_pins(
    pkg: &mut Package,
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
    prefix: &str,
    start_index: usize,
    clear_first: bool,
) {
    if clear_first {
        pkg.pins.clear();
    }
    let needed_height = (left.max(right) as i32) + 1;
    let needed_width = (top.max(bottom) as i32) + 1;
    if needed_height > pkg.height {
        pkg.height = needed_height;
    }
    if needed_width > pkg.width {
        pkg.width = needed_width;
    }

    let mut index = start_index;
    for i in 0..left {
        let id = format!("{prefix}{index}");
        pkg.pins.push(PkgPin {
            id: id.clone(),
            label: id,
            pin_type: String::new(),
            xpos: -8,
            ypos: 8 * (i as i32 + 1),
            angle: 180,
            length: 8,
            space: 0,
        });
        index += 1;
    }
    for i in 0..right {
        let id = format!("{prefix}{index}");
        pkg.pins.push(PkgPin {
            id: id.clone(),
            label: id,
            pin_type: String::new(),
            xpos: pkg.width * 8 + 8,
            ypos: 8 * (i as i32 + 1),
            angle: 0,
            length: 8,
            space: 0,
        });
        index += 1;
    }
    for i in 0..top {
        let id = format!("{prefix}{index}");
        pkg.pins.push(PkgPin {
            id: id.clone(),
            label: id,
            pin_type: String::new(),
            xpos: 8 * (i as i32 + 1),
            ypos: -8,
            angle: 90,
            length: 8,
            space: 0,
        });
        index += 1;
    }
    for i in 0..bottom {
        let id = format!("{prefix}{index}");
        pkg.pins.push(PkgPin {
            id: id.clone(),
            label: id,
            pin_type: String::new(),
            xpos: 8 * (i as i32 + 1),
            ypos: pkg.height * 8 + 8,
            angle: 270,
            length: 8,
            space: 0,
        });
        index += 1;
    }
}

/// Generate a standard Dual In-line Package (DIP) IC footprint.
/// Pin count should be even (e.g. 8, 14, 16, 20, 24, 28, 40).
/// Pins 1..N/2 run down the left side (angle 180), and pins (N/2+1)..N run up the right side (angle 0).
pub fn generate_dip_footprint(name: &str, pin_count: usize, custom_width: Option<i32>) -> Package {
    let half = pin_count / 2;
    let height = (half as i32) + 1;
    let width = custom_width.unwrap_or(if pin_count >= 24 { 6 } else { 4 });
    let mut pkg = Package {
        name: name.to_string(),
        width,
        height,
        subc_type: SubcType::Logic,
        logic_symbol: false,
        border: false,
        custom_color: false,
        bckgndcolor: "#141e3c".to_string(),
        background: String::new(),
        package_file: format!("{name}.package"),
        pins: Vec::with_capacity(pin_count),
    };

    // Left side: 1 to half (top to bottom)
    for i in 0..half {
        let pin_num = i + 1;
        let id = pin_num.to_string();
        pkg.pins.push(PkgPin {
            id: id.clone(),
            label: id,
            pin_type: String::new(),
            xpos: -8,
            ypos: 8 * (i as i32 + 1),
            angle: 180,
            length: 8,
            space: 0,
        });
    }

    // Right side: pin_count down to half + 1 (bottom to top, standard DIP counter-clockwise)
    for i in 0..half {
        let pin_num = pin_count - i;
        let id = pin_num.to_string();
        pkg.pins.push(PkgPin {
            id: id.clone(),
            label: id,
            pin_type: String::new(),
            xpos: width * 8 + 8,
            ypos: 8 * (i as i32 + 1),
            angle: 0,
            length: 8,
            space: 0,
        });
    }

    pkg
}

/// Generate a Logic Symbol (LS) block footprint.
/// Inputs on the left (angle 180), outputs on the right (angle 0), optional top/bottom pins.
pub fn generate_ls_footprint(
    name: &str,
    inputs: &[&str],
    outputs: &[&str],
    top_pins: &[&str],
    bottom_pins: &[&str],
) -> Package {
    let side_max = inputs.len().max(outputs.len());
    let height = (side_max as i32 + 1).max(4);
    let tb_max = top_pins.len().max(bottom_pins.len());
    let width = (tb_max as i32 + 1).max(6);

    let mut pkg = Package {
        name: name.to_string(),
        width,
        height,
        subc_type: SubcType::Logic,
        logic_symbol: true,
        border: true,
        custom_color: false,
        bckgndcolor: "#ebf0ff".to_string(),
        background: String::new(),
        package_file: format!("{name}_LS.package"),
        pins: Vec::new(),
    };

    for (i, &inp) in inputs.iter().enumerate() {
        let mut ptype = String::new();
        let mut clean_label = inp;
        if clean_label.starts_with('!') || clean_label.starts_with('~') {
            ptype = "inv".to_string();
            clean_label = &clean_label[1..];
        }
        pkg.pins.push(PkgPin {
            id: clean_label.to_string(),
            label: clean_label.to_string(),
            pin_type: ptype,
            xpos: -8,
            ypos: 8 * (i as i32 + 1),
            angle: 180,
            length: 8,
            space: 0,
        });
    }

    for (i, &outp) in outputs.iter().enumerate() {
        let mut ptype = String::new();
        let mut clean_label = outp;
        if clean_label.starts_with('!') || clean_label.starts_with('~') {
            ptype = "inv".to_string();
            clean_label = &clean_label[1..];
        }
        pkg.pins.push(PkgPin {
            id: clean_label.to_string(),
            label: clean_label.to_string(),
            pin_type: ptype,
            xpos: width * 8 + 8,
            ypos: 8 * (i as i32 + 1),
            angle: 0,
            length: 8,
            space: 0,
        });
    }

    for (i, &tp) in top_pins.iter().enumerate() {
        pkg.pins.push(PkgPin {
            id: tp.to_string(),
            label: tp.to_string(),
            pin_type: String::new(),
            xpos: 8 * (i as i32 + 1),
            ypos: -8,
            angle: 90,
            length: 8,
            space: 0,
        });
    }

    for (i, &bp) in bottom_pins.iter().enumerate() {
        pkg.pins.push(PkgPin {
            id: bp.to_string(),
            label: bp.to_string(),
            pin_type: String::new(),
            xpos: 8 * (i as i32 + 1),
            ypos: height * 8 + 8,
            angle: 270,
            length: 8,
            space: 0,
        });
    }

    pkg
}

/// Pick DIP vs logic-symbol package the way `Chip::setPackage` / `propNotFound` do.
pub fn select_package<'a>(
    packages: &'a BTreeMap<String, Package>,
    logic_symbol: bool,
    named: Option<&str>,
) -> Option<&'a Package> {
    if packages.is_empty() {
        return None;
    }
    if let Some(name) = named {
        if let Some(p) = packages.get(name) {
            return Some(p);
        }
    }
    let mut dip = None;
    let mut ls = None;
    for (key, pkg) in packages {
        if key.ends_with("DIP") {
            dip = Some(pkg);
        } else {
            ls = Some(pkg);
        }
    }
    let first = packages.values().next();
    if logic_symbol {
        ls.or(first)
    } else if packages.len() > 1 {
        dip.or(first)
    } else {
        first
    }
}

/// Helper to sanitize any raw or legacy selector strings (e.g. "1- mega328_DIP" -> "mega328", "2- DIP" -> "").
pub fn clean_chip_label(raw: &str) -> &str {
    let s = raw.trim();
    if s.is_empty() {
        return "";
    }
    // Strip leading selector prefix like "1- ", "2- ", "1 - "
    let s = if let Some(idx) = s.find('-') {
        let prefix = s[..idx].trim();
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
            s[idx + 1..].trim_start()
        } else {
            s
        }
    } else {
        s
    };

    // Strip trailing footprint suffixes like "_DIP", "_LS", "-DIP", "-LS"
    let s = if let Some(stripped) = s
        .strip_suffix("_DIP")
        .or_else(|| s.strip_suffix("_dip"))
        .or_else(|| s.strip_suffix("_LS"))
        .or_else(|| s.strip_suffix("_ls"))
        .or_else(|| s.strip_suffix("-DIP"))
        .or_else(|| s.strip_suffix("-dip"))
        .or_else(|| s.strip_suffix("-LS"))
        .or_else(|| s.strip_suffix("-ls"))
        .or_else(|| s.strip_suffix(" DIP"))
        .or_else(|| s.strip_suffix(" dip"))
        .or_else(|| s.strip_suffix(" LS"))
        .or_else(|| s.strip_suffix(" ls"))
    {
        stripped.trim()
    } else {
        s
    };

    if matches!(
        s,
        "" | "Package"
            | "package"
            | "SubPackage"
            | "subpackage"
            | "DIP"
            | "dip"
            | "LS"
            | "ls"
            | "Logic Symbol"
            | "logic symbol"
            | "None"
            | "none"
    ) {
        ""
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PKG_ITEM: &str = r#"<circuit version="2.0.0" >
<item itemtype="Package" CircId="Package-1" label="DIP" width="4" height="4" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in; label=in&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=out; label=out&#xa;Pin; type=nc; xpos=16; ypos=40; angle=270; length=8; space=0; id=nc1; label=NC" SubcType="None" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" />
</circuit>
"#;

    #[test]
    fn packages_from_sim1_stops_at_resistor() {
        let (list, typ) = packages_from_sim1(PKG_ITEM);
        assert_eq!(typ, "None");
        let pkg = list.get("DIP").expect("DIP");
        assert_eq!(pkg.width, 4);
        assert_eq!(pkg.height, 4);
        assert_eq!(pkg.pins.len(), 3);
        assert_eq!(pkg.pins[0].id, "in");
        assert_eq!(pkg.pins[0].xpos, -8);
        assert_eq!(pkg.pins[0].angle, 180);
        assert!(pkg.pins[2].unused());
        assert_eq!(pkg.electrical_pins().count(), 2);
    }

    #[test]
    fn convert_package_xml() {
        let xml = r#"<!-- generated -->
<packageB name="ESP32" width="15" height="15" background="" type="None" >
    <pin type=""    xpos="128"  ypos="32"   angle="0"   length="8"  space="0"   id="G05"  label="G05" />
    <pin type="nc"  xpos="48"   ypos="-8"   angle="90"  length="8"  space="0"   id="XTALi" label="XTALi"/>
</packageB>
"#;
        let (_, pkg) = convert_package(xml);
        assert_eq!(pkg.width, 15);
        assert_eq!(pkg.height, 15);
        assert_eq!(pkg.pins.len(), 2);
        assert_eq!(pkg.pins[0].id, "G05");
        assert_eq!(pkg.pins[0].xpos, 128);
        assert!(pkg.pins[1].unused());
    }

    #[test]
    fn test_package_to_xml_roundtrip() {
        let mut pkg = Package {
            name: "74HC00".to_string(),
            width: 4,
            height: 8,
            subc_type: SubcType::Logic,
            logic_symbol: false,
            border: true,
            custom_color: true,
            bckgndcolor: "#1a2b3c".to_string(),
            background: "img.png".to_string(),
            package_file: "74HC00.package".to_string(),
            pins: Vec::new(),
        };
        pkg.pins.push(PkgPin::new("1A", "1A", "", -8, 8, 180));
        pkg.pins.push(PkgPin::new("1B", "1B", "inv", -8, 16, 180));
        pkg.pins.push(PkgPin::new("1Y", "1Y", "", 40, 8, 0));
        pkg.pins.push(PkgPin::new("NC", "NC", "nc", 16, 72, 270));

        let xml = package_to_xml(&pkg);
        let (_, parsed) = convert_package(&xml);

        assert_eq!(parsed.name, "74HC00");
        assert_eq!(parsed.width, 4);
        assert_eq!(parsed.height, 8);
        assert_eq!(parsed.subc_type, SubcType::Logic);
        assert_eq!(parsed.logic_symbol, false);
        assert_eq!(parsed.border, true);
        assert_eq!(parsed.custom_color, true);
        assert_eq!(parsed.bckgndcolor, "#1a2b3c");
        assert_eq!(parsed.pins.len(), 4);
        assert_eq!(parsed.pins[0].id, "1A");
        assert!(parsed.pins[1].inverted());
        assert!(parsed.pins[3].unused());
    }

    #[test]
    fn test_generate_dip() {
        let dip8 = generate_dip_footprint("NE555", 8, None);
        assert_eq!(dip8.name, "NE555");
        assert_eq!(dip8.width, 4);
        assert_eq!(dip8.height, 5);
        assert_eq!(dip8.pins.len(), 8);
        // Pin 1: left top
        assert_eq!(dip8.pins[0].id, "1");
        assert_eq!(dip8.pins[0].angle, 180);
        assert_eq!(dip8.pins[0].ypos, 8);
        // Pin 4: left bottom
        assert_eq!(dip8.pins[3].id, "4");
        assert_eq!(dip8.pins[3].angle, 180);
        assert_eq!(dip8.pins[3].ypos, 32);
        // Pin 8: right top
        assert_eq!(dip8.pins[4].id, "8");
        assert_eq!(dip8.pins[4].angle, 0);
        assert_eq!(dip8.pins[4].ypos, 8);
        // Pin 5: right bottom
        assert_eq!(dip8.pins[7].id, "5");
        assert_eq!(dip8.pins[7].angle, 0);
        assert_eq!(dip8.pins[7].ypos, 32);
    }

    #[test]
    fn test_generate_ls() {
        let ls = generate_ls_footprint("AND2", &["A", "B"], &["Y"], &["VCC"], &["GND"]);
        assert_eq!(ls.name, "AND2");
        assert!(ls.logic_symbol);
        assert_eq!(ls.pins.len(), 5);
        assert_eq!(ls.pins[0].id, "A");
        assert_eq!(ls.pins[0].angle, 180);
        assert_eq!(ls.pins[2].id, "Y");
        assert_eq!(ls.pins[2].angle, 0);
        assert_eq!(ls.pins[3].id, "VCC");
        assert_eq!(ls.pins[3].angle, 90);
        assert_eq!(ls.pins[4].id, "GND");
    }

    #[test]
    fn test_clean_chip_label() {
        assert_eq!(clean_chip_label("1- mega328_DIP"), "mega328");
        assert_eq!(clean_chip_label("2- mega328_LS"), "mega328");
        assert_eq!(clean_chip_label("1- ESP32_DIP"), "ESP32");
        assert_eq!(clean_chip_label("1- pic16f84_DIP"), "pic16f84");
        assert_eq!(clean_chip_label("2- DIP"), "");
        assert_eq!(clean_chip_label("1- Logic Symbol"), "");
        assert_eq!(clean_chip_label("Package"), "");
        assert_eq!(clean_chip_label("DIP"), "");
        assert_eq!(clean_chip_label("ATmega328"), "ATmega328");
        assert_eq!(clean_chip_label("74HC00"), "74HC00");
        assert_eq!(clean_chip_label("NE555"), "NE555");
        assert_eq!(clean_chip_label("DIP8"), "DIP8");
    }
}
