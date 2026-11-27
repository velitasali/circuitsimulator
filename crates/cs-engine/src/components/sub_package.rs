//! SubPackage: subcircuit IC package footprint designer component.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
use super::shape;
use super::{CompPin, Component, Part, Stampable};
use crate::canvas::{ImageBuf, Rect};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::package::{Package, package_pins_to_prop, parse_package_pins};

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 128;

/// Decode a package `Background` image from a direct path, any relative search directory,
/// or legacy embedded hex-encoded image bytes.
pub fn decode_package_background(bg: &str, dirs: &[&Path]) -> Option<Arc<ImageBuf>> {
    let trimmed = bg.trim();
    if trimmed.is_empty() {
        return None;
    }
    let direct = Path::new(trimmed);
    if direct.is_file()
        && let Ok(bytes) = fs::read(direct)
        && let Some(pm) = shape::decode_image_bytes(&bytes)
    {
        return Some(Arc::new(pm));
    }
    for dir in dirs {
        let cand = dir.join(trimmed);
        if cand.is_file()
            && let Ok(bytes) = fs::read(&cand)
            && let Some(pm) = shape::decode_image_bytes(&bytes)
        {
            return Some(Arc::new(pm));
        }
    }
    if trimmed.len() >= 16
        && trimmed.len() % 2 == 0
        && trimmed.bytes().all(|b| b.is_ascii_hexdigit())
    {
        let bytes = shape::hex_to_bytes(trimmed);
        if let Some(pm) = shape::decode_image_bytes(&bytes) {
            return Some(Arc::new(pm));
        }
    }
    None
}

/// Package footprint container item for the Package Designer.
#[derive(Clone, Debug)]
pub struct SubPackage {
    pub package: Package,
    pub background_pixmap: Option<Arc<ImageBuf>>,
}

impl PartialEq for SubPackage {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package
    }
}

impl crate::canvas::Item {
    pub fn subpackage(
        id: impl Into<String>,
        x: f64,
        y: f64,
        package: crate::package::Package,
    ) -> Self {
        Self::new(id, x, y, SubPackage::new(package))
    }
}

impl Default for SubPackage {
    fn default() -> Self {
        let mut package = Package::default();
        package.logic_symbol = true;
        Self {
            package,
            background_pixmap: None,
        }
    }
}

impl SubPackage {
    pub const TYPE_ID: &'static str = "SubPackage";
    pub fn new(package: Package) -> Self {
        let mut sp = Self {
            package,
            background_pixmap: None,
        };
        sp.resolve_background(None);
        sp
    }

    pub fn resolve_background(&mut self, extra_dir: Option<&Path>) -> bool {
        let mut dirs: Vec<&Path> = Vec::new();
        let pkg_parent = (!self.package.package_file.trim().is_empty())
            .then(|| Path::new(self.package.package_file.trim()).parent())
            .flatten()
            .filter(|p| !p.as_os_str().is_empty());
        if let Some(d) = pkg_parent {
            dirs.push(d);
        }
        if let Some(d) = extra_dir {
            dirs.push(d);
        }
        self.background_pixmap = decode_package_background(&self.package.background, &dirs);
        self.background_pixmap.is_some()
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
        self.resolve_background(None);
        Ok(())
    }

    fn get_package_file(&self) -> PropValue {
        PropValue::String(self.package.package_file.clone())
    }
    fn set_package_file(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.package_file = expect_string("PackageFile", v)?;
        if self.background_pixmap.is_none() && !self.package.background.is_empty() {
            self.resolve_background(None);
        }
        Ok(())
    }

    fn get_pins(&self) -> PropValue {
        PropValue::String(package_pins_to_prop(&self.package.pins))
    }
    fn set_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Pins", v)?;
        self.package.pins = parse_package_pins(&s);
        Ok(())
    }
}

impl Component for SubPackage {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    const DESCRIPTION: &'static str = "Subcircuit IC package footprint.";

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
        super::drawable::paint_dip_package_with_bg(
            d,
            ctx.pal,
            w,
            h,
            label,
            self.package.logic_symbol,
            self.package.custom_color,
            &self.package.bckgndcolor,
            false,
            self.background_pixmap.as_deref(),
            self.package.border,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_subpackage(&mut self, x: f64, y: f64) -> String {
        let id = self.alloc_unique_id("SubPackage");
        let mut part = SubPackage::default();
        part.package.name = id.clone();
        self.items.push(crate::canvas::Item::new(&id, x, y, part));
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
                p.background_pixmap = None;
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
                p.background_pixmap = None;
                return true;
            }
        }
        false
    }

    pub fn load_package_xml(&mut self, uid: &str, xml: &str) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let Part::SubPackage(p) = &mut item.kind {
                let (_, pkg) = crate::package::convert_package(xml);
                p.package = pkg;
                p.resolve_background(None);
                return true;
            }
        }
        false
    }

    pub fn load_package_xml_from_path(
        &mut self,
        uid: &str,
        xml: &str,
        package_file: &str,
        extra_dir: Option<&Path>,
    ) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let Part::SubPackage(p) = &mut item.kind {
                let (_, mut pkg) = crate::package::convert_package(xml);
                pkg.package_file = package_file.to_string();
                p.package = pkg;
                p.resolve_background(extra_dir);
                return true;
            }
        }
        false
    }

    pub fn set_package_file(&mut self, uid: &str, package_file: &str) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let Part::SubPackage(p) = &mut item.kind {
                p.package.package_file = package_file.to_string();
                if p.background_pixmap.is_none() && !p.package.background.is_empty() {
                    p.resolve_background(None);
                }
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
        assert_eq!(sp.get_prop_text("LogicSymbol").unwrap(), "true");
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

    #[test]
    fn subpackage_paint_background_and_border_gating() {
        use crate::canvas::Canvas;
        use crate::canvas::draw::{PaintCtx, Palette};
        use crate::components::Drawable;
        use crate::components::tests::recorder::{DrawOp, DrawRecorder};

        let img = ImageBuf::from_rgba(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            ],
        )
        .unwrap();
        let png = img.encode_png().expect("encode png");
        let hex_png = shape::bytes_to_hex(&png);

        let canvas = Canvas::empty();
        let pal = Palette::light();
        let ctx = PaintCtx {
            canvas: &canvas,
            pal: &pal,
            scale: 1.0,
            item_id: "SubPackage-1",
            on_board: false,
            active_item_id: None,
        };

        // 1. Solid-fill package without background always strokes its outline even when Border=false.
        let mut sp = SubPackage::default();
        assert!(!sp.package.border);
        let mut rec = DrawRecorder::new();
        sp.paint(&mut rec, &ctx);
        assert!(
            rec.ops
                .iter()
                .any(|op| matches!(op, DrawOp::FillRoundRect { .. }))
        );
        assert!(
            rec.ops
                .iter()
                .any(|op| matches!(op, DrawOp::StrokeRoundRect { .. }))
        );
        assert!(
            !rec.ops
                .iter()
                .any(|op| matches!(op, DrawOp::DrawPixmapRect { .. }))
        );

        // 2. Package with Background image and Border=false renders pixmap without border stroke.
        sp.set_prop_text("Background", &hex_png).unwrap();
        assert!(sp.background_pixmap.is_some());
        let mut rec_bg = DrawRecorder::new();
        sp.paint(&mut rec_bg, &ctx);
        assert!(
            rec_bg
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::DrawPixmapRect { .. }))
        );
        assert!(
            !rec_bg
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::StrokeRoundRect { .. }))
        );

        // 3. Enabling Border=true strokes a border around the background image.
        sp.set_prop_text("Border", "true").unwrap();
        let mut rec_bg_border = DrawRecorder::new();
        sp.paint(&mut rec_bg_border, &ctx);
        assert!(
            rec_bg_border
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::DrawPixmapRect { .. }))
        );
        assert!(
            rec_bg_border
                .ops
                .iter()
                .any(|op| matches!(op, DrawOp::StrokeRoundRect { .. }))
        );

        // 4. PartialEq ignores the decoded pixmap cache.
        let mut sp2 = sp.clone();
        sp2.background_pixmap = None;
        assert_eq!(sp, sp2);
    }
}
