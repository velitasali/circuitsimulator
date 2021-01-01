//! SubPackage: subcircuit IC package footprint designer component.

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::package::{Package, package_pins_to_sim1_prop, parse_sim1_package_pins};

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 128;

/// Package footprint container item for the Package Designer.
#[derive(Clone, Debug, PartialEq)]
pub struct SubPackage {
    pub package: Package,
}

impl crate::canvas::Item {
    pub fn subpackage(
        id: impl Into<String>,
        x: f64,
        y: f64,
        package: crate::package::Package,
    ) -> Self {
        Self::new(id, x, y, SubPackage { package })
    }
}

impl Default for SubPackage {
    fn default() -> Self {
        Self {
            package: Package::default(),
        }
    }
}

impl SubPackage {
    pub const TYPE_ID: &'static str = "SubPackage";
    pub fn new(package: Package) -> Self {
        Self { package }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::SubPackage
    }

    fn get_name(&self) -> PropValue {
        PropValue::String(self.package.name.clone())
    }
    fn set_name(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.name = expect_string("Name", v)?;
        Ok(())
    }

    fn get_width(&self) -> PropValue {
        PropValue::Int(self.package.width as i64)
    }
    fn set_width(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.width = expect_int("Width", v)?.clamp(MIN_DIM, MAX_DIM) as i32;
        Ok(())
    }

    fn get_height(&self) -> PropValue {
        PropValue::Int(self.package.height as i64)
    }
    fn set_height(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.height = expect_int("Height", v)?.clamp(MIN_DIM, MAX_DIM) as i32;
        Ok(())
    }

    fn get_subc_type(&self) -> PropValue {
        PropValue::Enum(self.package.subc_type.as_str().to_string())
    }
    fn set_subc_type(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("SubcType", v)?;
        self.package.subc_type = crate::package::SubcType::from_str_name(&s);
        Ok(())
    }

    fn get_logic_symbol(&self) -> PropValue {
        PropValue::Bool(self.package.logic_symbol)
    }
    fn set_logic_symbol(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.logic_symbol = expect_bool("LogicSymbol", v)?;
        Ok(())
    }

    fn get_custom_color(&self) -> PropValue {
        PropValue::Bool(self.package.custom_color)
    }
    fn set_custom_color(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.custom_color = expect_bool("CustomColor", v)?;
        Ok(())
    }

    fn get_bckgndcolor(&self) -> PropValue {
        PropValue::String(self.package.bckgndcolor.clone())
    }
    fn set_bckgndcolor(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.bckgndcolor = expect_string("BckGndColor", v)?;
        Ok(())
    }

    fn get_border(&self) -> PropValue {
        PropValue::Bool(self.package.border)
    }
    fn set_border(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.border = expect_bool("Border", v)?;
        Ok(())
    }

    fn get_background(&self) -> PropValue {
        PropValue::String(self.package.background.clone())
    }
    fn set_background(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.background = expect_string("Background", v)?;
        Ok(())
    }

    fn get_package_file(&self) -> PropValue {
        PropValue::String(self.package.package_file.clone())
    }
    fn set_package_file(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.package_file = expect_string("PackageFile", v)?;
        Ok(())
    }

    fn get_pins(&self) -> PropValue {
        PropValue::String(package_pins_to_sim1_prop(&self.package.pins))
    }
    fn set_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Pins", v)?;
        self.package.pins = parse_sim1_package_pins(&s);
        Ok(())
    }
}

impl Component for SubPackage {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Subcircuit IC package footprint."
    }

    fn props() -> &'static [PropDef<Self>] {
        const W: PropDef<SubPackage> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_DIM,
                MAX_DIM,
                SubPackage::get_width,
                SubPackage::set_width,
            )
            .with_info("WIdth in grid cells.");
            p.structural = true;
            p
        };
        const H: PropDef<SubPackage> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_DIM,
                MAX_DIM,
                SubPackage::get_height,
                SubPackage::set_height,
            )
            .with_info("Height in grid cells.");
            p.structural = true;
            p
        };
        const LS: PropDef<SubPackage> = {
            let mut p = PropDef::bool(
                "LogicSymbol",
                "Logic Symbol",
                SubPackage::get_logic_symbol,
                SubPackage::set_logic_symbol,
            )
            .with_info(
                "If yes, use A logic symbol representation.\nIf no, use a \"Chip\" representation.",
            )
            .with_info(
                "If yes, use A logic symbol representation.\nIf no, use a \"Chip\" representation.",
            )
            .with_info(
                "If yes, use A logic symbol representation.\nIf no, use a \"Chip\" representation.",
            );
            p.structural = true;
            p
        };
        const PINS: PropDef<SubPackage> = {
            let mut p = PropDef::string("Pins", "Pins", SubPackage::get_pins, SubPackage::set_pins)
                .with_info("Generate Pins");
            p.structural = true;
            p
        };
        const SUBC_TYPE_OPTIONS: &[&str] = &["None", "MCU", "Logic", "Board", "Shield", "Other"];
        static PROPS: &[PropDef<SubPackage>] = &[
            PropDef::string(
                "Name",
                "Name",
                SubPackage::get_name,
                SubPackage::set_name,
            )
            .with_info("Name shown at the center of the component.\nLeave empty to show nothing.\nuse \"Package\" to use package file name."),
            W,
            H,
            PropDef::enumeration(
                "SubcType",
                "SubcType",
                SUBC_TYPE_OPTIONS,
                SubPackage::get_subc_type,
                SubPackage::set_subc_type,
            ).with_info("Package type."),
            LS,
            PropDef::bool(
                "CustomColor",
                "Custom Color",
                SubPackage::get_custom_color,
                SubPackage::set_custom_color,
            ).with_info("Use a custom background color instead of the default one."),
            PropDef::string(
                "BckGndColor",
                "Background Color",
                SubPackage::get_bckgndcolor,
                SubPackage::set_bckgndcolor,
            ).with_info("Custom background color."),
            PropDef::bool(
                "Border",
                "Border",
                SubPackage::get_border,
                SubPackage::set_border,
            ).with_info("Draw a border around the background image."),
            PropDef::string(
                "Background",
                "Background",
                SubPackage::get_background,
                SubPackage::set_background,
            ).with_info("File for background image."),
            PropDef::string(
                "PackageFile",
                "Package File",
                SubPackage::get_package_file,
                SubPackage::set_package_file,
            ).with_info("Path to package file."),
            PINS,
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        self.package
            .pins
            .iter()
            .map(|p| {
                CompPin::new(
                    p.id.clone(),
                    p.xpos as f64,
                    p.ypos as f64,
                    p.angle,
                    p.length as f64,
                )
            })
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(0.0, 0.0, self.package.body_w(), self.package.body_h())
    }
}

impl Stampable for SubPackage {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // SubPackage is a visual footprint designer element on the canvas.
    }
}

impl super::drawable::Drawable for SubPackage {
    fn paint(
        &self,
        d: &mut dyn crate::canvas::draw::Draw,
        ctx: &crate::canvas::draw::PaintCtx,
    ) -> bool {
        let w = self.package.body_w().max(8.0);
        let h = self.package.body_h().max(8.0);
        let label = if !self.package.name.is_empty() && self.package.name != "Package" {
            &self.package.name
        } else {
            ""
        };
        super::drawable::paint_dip_package(
            d,
            ctx.pal,
            w,
            h,
            label,
            self.package.logic_symbol,
            self.package.custom_color,
            &self.package.bckgndcolor,
            false,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_subpackage(&mut self, x: f64, y: f64) -> String {
        self.next_subc += 1;
        let id = format!("SubPackage-{}", self.next_subc);
        let mut pkg = crate::package::Package::default();
        pkg.name = id.clone();
        self.items
            .push(crate::canvas::Item::subpackage(&id, x, y, pkg));
        id
    }

    pub fn package_mut(&mut self, uid: &str) -> Option<&mut crate::package::Package> {
        let it = self.item_by_id_mut(uid)?;
        match &mut it.kind {
            crate::components::Part::SubPackage(p) => Some(&mut p.package),
            crate::components::Part::Subcircuit(p) => Some(&mut p.package),
            crate::components::Part::Mcu(p) => Some(&mut p.package),
            crate::components::Part::QemuDevice(p) => Some(&mut p.package),
            _ => None,
        }
    }

    pub fn package(&self, uid: &str) -> Option<&crate::package::Package> {
        self.item_by_id(uid).and_then(|it| it.kind.package())
    }

    pub fn package_ref(&self, uid: &str) -> Option<&crate::package::Package> {
        self.package(uid)
    }

    pub fn add_package_pin(
        &mut self,
        uid: &str,
        angle: i32,
        x: i32,
        y: i32,
        id: &str,
        label: &str,
    ) -> bool {
        if let Some(pkg) = self.package_mut(uid) {
            pkg.pins.push(crate::package::PkgPin {
                id: id.to_string(),
                label: label.to_string(),
                pin_type: String::new(),
                xpos: x,
                ypos: y,
                angle,
                length: 8,
                space: 0,
            });
            true
        } else {
            false
        }
    }

    pub fn remove_package_pin(&mut self, uid: &str, pin_id: &str) -> bool {
        if let Some(pkg) = self.package_mut(uid) {
            let before = pkg.pins.len();
            pkg.pins.retain(|p| p.id != pin_id);
            pkg.pins.len() < before
        } else {
            false
        }
    }

    pub fn update_package_pin(
        &mut self,
        uid: &str,
        old_pin_id: &str,
        new_pin: crate::package::PkgPin,
    ) -> bool {
        if let Some(pkg) = self.package_mut(uid) {
            if let Some(p) = pkg.pins.iter_mut().find(|p| p.id == old_pin_id) {
                *p = new_pin;
                return true;
            }
        }
        false
    }

    pub fn generate_package_pins(
        &mut self,
        uid: &str,
        left: usize,
        right: usize,
        top: usize,
        bottom: usize,
        prefix: &str,
        start_index: usize,
        clear_first: bool,
    ) -> bool {
        if let Some(pkg) = self.package_mut(uid) {
            crate::package::generate_pins(
                pkg,
                left,
                right,
                top,
                bottom,
                prefix,
                start_index,
                clear_first,
            );
            true
        } else {
            false
        }
    }

    pub fn set_package_dimensions(&mut self, uid: &str, width: i32, height: i32) -> bool {
        if let Some(pkg) = self.package_mut(uid) {
            pkg.width = width.clamp(1, 128);
            pkg.height = height.clamp(1, 128);
            true
        } else {
            false
        }
    }

    pub fn set_package_footprint_dip(
        &mut self,
        uid: &str,
        name: &str,
        pin_count: usize,
        custom_width: Option<i32>,
    ) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::SubPackage(p) = &mut item.kind {
                let pkg = crate::package::generate_dip_footprint(name, pin_count, custom_width);
                p.package = pkg;
                return true;
            }
        }
        false
    }

    pub fn set_package_footprint_ls(
        &mut self,
        uid: &str,
        name: &str,
        inputs: &[&str],
        outputs: &[&str],
        top: &[&str],
        bottom: &[&str],
    ) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::SubPackage(p) = &mut item.kind {
                let pkg = crate::package::generate_ls_footprint(name, inputs, outputs, top, bottom);
                p.package = pkg;
                return true;
            }
        }
        false
    }

    pub fn load_package_xml(&mut self, uid: &str, xml: &str) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::SubPackage(p) = &mut item.kind {
                let (_, pkg) = crate::package::convert_package(xml);
                p.package = pkg;
                return true;
            }
        }
        false
    }

    pub fn export_package_xml(&self, uid: &str) -> Option<String> {
        let pkg = self.package(uid)?;
        Some(crate::package::package_to_xml(pkg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::PkgPin;

    #[test]
    fn subpackage_defaults_and_props() {
        let mut sp = SubPackage::default();
        assert_eq!(sp.type_id(), "SubPackage");
        assert_eq!(sp.get_prop_text("Name").unwrap(), "");
        assert_eq!(sp.get_prop_text("Width").unwrap(), "4");
        assert_eq!(sp.get_prop_text("Height").unwrap(), "8");
        assert_eq!(sp.get_prop_text("SubcType").unwrap(), "None");
        assert_eq!(sp.get_prop_text("LogicSymbol").unwrap(), "false");
        assert_eq!(sp.get_prop_text("CustomColor").unwrap(), "false");
        assert_eq!(sp.get_prop_text("BckGndColor").unwrap(), "");
        assert_eq!(sp.get_prop_text("Border").unwrap(), "false");
        assert_eq!(sp.get_prop_text("Background").unwrap(), "");
        assert_eq!(sp.get_prop_text("PackageFile").unwrap(), "");
        assert_eq!(sp.get_prop_text("Pins").unwrap(), "");

        sp.set_prop_text("Name", "DIP8").unwrap();
        assert_eq!(sp.package.name, "DIP8");
        sp.set_prop_text("Width", "6").unwrap();
        assert_eq!(sp.package.width, 6);
        sp.set_prop_text("Height", "10").unwrap();
        assert_eq!(sp.package.height, 10);
        sp.set_prop_text("CustomColor", "true").unwrap();
        assert_eq!(sp.package.custom_color, true);
        sp.set_prop_text("BckGndColor", "#3a3a5a").unwrap();
        assert_eq!(sp.package.bckgndcolor, "#3a3a5a");
    }

    #[test]
    fn subpackage_pins_prop_roundtrip() {
        let mut sp = SubPackage::default();
        sp.package
            .pins
            .push(PkgPin::new("1", "VCC", "", -8, 8, 180));
        sp.package.pins.push(PkgPin::new("2", "GND", "", 40, 8, 0));

        let formatted = sp.get_prop_text("Pins").unwrap();
        assert!(!formatted.is_empty());

        let mut sp2 = SubPackage::default();
        sp2.set_prop_text("Pins", &formatted).unwrap();
        assert_eq!(sp2.package.pins.len(), 2);
        assert_eq!(sp2.package.pins[0].id, "1");
        assert_eq!(sp2.package.pins[0].label, "VCC");
        assert_eq!(sp2.package.pins[1].id, "2");
        assert_eq!(sp2.package.pins[1].label, "GND");
    }

    #[test]
    fn subpackage_body_and_element_kind() {
        let sp = SubPackage::default();
        assert_eq!(sp.body().w, 32.0);
        assert_eq!(sp.body().h, 64.0);
        assert!(matches!(sp.to_element_kind(), Kind::SubPackage));
    }
}
