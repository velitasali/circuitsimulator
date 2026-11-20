//! Subcircuit: nested schematic item with IC package footprint.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::draw::{Color, Draw, PaintCtx, parse_hex};
use crate::canvas::{Item, Point, Rect, Scene, map_rect, map_scene};
use crate::digital::{
    LOGIC_DELAY, LOGIC_FALL, LOGIC_IN_HIGH, LOGIC_IN_IMP, LOGIC_IN_LOW, LOGIC_OUT_HIGH,
    LOGIC_OUT_IMP, LOGIC_OUT_LOW, LOGIC_RISE, LogicOverride,
};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::package::{Package, SubcType};
use crate::subcircuit::{self, BoardChild, SubcSearch};
use crate::theme::{COMPONENT_BORDER_WIDTH, ColorId, ColorTheme};

const LOGIC_PROP_IDS: &[&str] = &[
    "Input_High_V",
    "Input_Low_V",
    "Input_Imped",
    "Out_High_V",
    "Out_Low_V",
    "Out_Imped",
    "Tpd_ps",
    "Tr_ps",
    "Tf_ps",
];

/// Subcircuit instance referencing an inner schematic or device package.
#[derive(Clone, Debug, PartialEq)]
pub struct Subcircuit {
    pub device: String,
    pub package: Package,
    /// DIP and logic-symbol footprints. Logic Symbol picks between them.
    pub packages: BTreeMap<String, Package>,
    pub nested_src: String,
    pub nested_path: Option<String>,
    pub logic_symbol: bool,
    pub program: String,
    pub firmware_dir: Option<PathBuf>,
    /// SimulIDE `LogicSubc` levels. Pushed onto inner parts only after a field is set.
    pub logic: LogicOverride,
    /// Visual/interactive inner components mounted on a Board/Shield package face.
    pub board_children: Vec<BoardChild>,
    /// Index of board child currently holding an active pointer press/drag.
    pub active_board_child: Option<usize>,
    /// Index of the most recently interacted board child (used for live simulation sync).
    pub last_interacted_child: Option<usize>,
}

impl Item {
    pub fn subcircuit(
        id: impl Into<String>,
        x: f64,
        y: f64,
        device: impl Into<String>,
        package: Package,
        nested_src: impl Into<String>,
        nested_path: Option<String>,
        logic_symbol: bool,
    ) -> Self {
        let nested_src = nested_src.into();
        let board_children = subcircuit::board_children(&nested_src, &package, logic_symbol);
        Self::new(
            id,
            x,
            y,
            Subcircuit {
                device: device.into(),
                package,
                packages: BTreeMap::new(),
                nested_src,
                nested_path,
                logic_symbol,
                program: String::new(),
                firmware_dir: None,
                logic: LogicOverride::default(),
                board_children,
                active_board_child: None,
                last_interacted_child: None,
            },
        )
    }
}

impl Default for Subcircuit {
    fn default() -> Self {
        Self {
            device: String::new(),
            package: Package::default(),
            packages: BTreeMap::new(),
            nested_src: String::new(),
            nested_path: None,
            logic_symbol: false,
            program: String::new(),
            firmware_dir: None,
            logic: LogicOverride::default(),
            board_children: Vec::new(),
            active_board_child: None,
            last_interacted_child: None,
        }
    }
}

impl Subcircuit {
    pub const TYPE_ID: &'static str = "Subcircuit";

    /// Rebuild `board_children` from `nested_src`, `package`, and `logic_symbol`.
    pub fn refresh_board_children(&mut self) {
        self.board_children =
            subcircuit::board_children(&self.nested_src, &self.package, self.logic_symbol);
        self.active_board_child = None;
        self.last_interacted_child = None;
    }

    /// Map a point in `Subcircuit` local coordinates into the local coordinates of `board_children[idx]`.
    pub fn board_child_local(&self, idx: usize, subc_local: Point) -> Option<Point> {
        let ch = self.board_children.get(idx)?;
        Some(map_scene(
            Point::new(ch.placement.x, ch.placement.y),
            subc_local,
            ch.placement.rotation,
            ch.placement.hflip,
            ch.placement.vflip,
        ))
    }

    /// Find the topmost board child whose body contains `subc_local`.
    pub fn hit_board_child(&self, subc_local: Point) -> Option<(usize, Point)> {
        if self.logic_symbol || self.board_children.is_empty() {
            return None;
        }
        for (idx, ch) in self.board_children.iter().enumerate().rev() {
            let local = map_scene(
                Point::new(ch.placement.x, ch.placement.y),
                subc_local,
                ch.placement.rotation,
                ch.placement.hflip,
                ch.placement.vflip,
            );
            if ch.part.body().contains_point(local) {
                return Some((idx, local));
            }
        }
        None
    }

    /// Prefixed simulation component ID for an inner component `inner_id` inside this instance.
    pub fn child_sim_id(&self, instance_id: &str, inner_id: &str) -> String {
        let num_id = subcircuit::instance_num_id(instance_id, &self.device);
        format!("{num_id}@{inner_id}")
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Subcircuit {
            device: self.device.clone(),
            logic_symbol: self.logic_symbol,
            package_name: if self.package.name.is_empty() {
                None
            } else {
                Some(self.package.name.clone())
            },
        }
    }

    fn get_logic_symbol(&self) -> PropValue {
        PropValue::Bool(self.logic_symbol)
    }
    fn set_logic_symbol(&mut self, v: PropValue) -> Result<(), PropError> {
        let ls = expect_bool("LogicSymbol", v)?;
        let (pkg, is_ls) =
            crate::package::retarget_package(&self.package, &self.packages, ls, None);
        self.logic_symbol = is_ls;
        self.package = pkg;
        self.refresh_board_children();
        Ok(())
    }

    fn get_package(&self) -> PropValue {
        PropValue::String(self.package.name.clone())
    }
    fn set_package(&mut self, v: PropValue) -> Result<(), PropError> {
        let name = expect_string("Package", v)?;
        let (pkg, is_ls) = crate::package::retarget_package(
            &self.package,
            &self.packages,
            self.logic_symbol,
            Some(&name),
        );
        self.logic_symbol = is_ls;
        self.package = pkg;
        self.refresh_board_children();
        Ok(())
    }

    fn get_device(&self) -> PropValue {
        PropValue::String(self.device.clone())
    }
    fn set_device(&mut self, v: PropValue) -> Result<(), PropError> {
        self.device = expect_string("Device", v)?;
        Ok(())
    }

    fn get_program(&self) -> PropValue {
        PropValue::String(self.program.clone())
    }
    fn set_program(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Program", v)?;
        self.program = s.clone();
        if !self.nested_src.is_empty() {
            self.nested_src = subcircuit::set_nested_mcu_program(&self.nested_src, None, &s);
        }
        Ok(())
    }

    /// Levels to push while flattening. `None` until a logic property is set.
    pub fn simulation_logic(&self) -> Option<LogicOverride> {
        if self.package.subc_type == SubcType::Logic && self.logic.mask != 0 {
            Some(self.logic)
        } else {
            None
        }
    }

    fn is_logic(&self) -> bool {
        self.package.subc_type == SubcType::Logic
    }

    fn mark(&mut self, bit: u16) {
        self.logic.mask |= bit;
    }

    fn get_in_high(&self) -> PropValue {
        PropValue::Float(self.logic.in_high_v)
    }
    fn set_in_high(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.in_high_v = expect_float("Input_High_V", v)?.clamp(-1e3, 1e3);
        self.mark(LOGIC_IN_HIGH);
        Ok(())
    }
    fn get_in_low(&self) -> PropValue {
        PropValue::Float(self.logic.in_low_v)
    }
    fn set_in_low(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.in_low_v = expect_float("Input_Low_V", v)?.clamp(-1e3, 1e3);
        self.mark(LOGIC_IN_LOW);
        Ok(())
    }
    fn get_in_imp(&self) -> PropValue {
        PropValue::Float(self.logic.in_imp)
    }
    fn set_in_imp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.in_imp = expect_float("Input_Imped", v)?.clamp(1e-12, 1e15);
        self.mark(LOGIC_IN_IMP);
        Ok(())
    }
    fn get_out_high(&self) -> PropValue {
        PropValue::Float(self.logic.out_high_v)
    }
    fn set_out_high(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.out_high_v = expect_float("Out_High_V", v)?.clamp(-1e3, 1e3);
        self.mark(LOGIC_OUT_HIGH);
        Ok(())
    }
    fn get_out_low(&self) -> PropValue {
        PropValue::Float(self.logic.out_low_v)
    }
    fn set_out_low(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.out_low_v = expect_float("Out_Low_V", v)?.clamp(-1e3, 1e3);
        self.mark(LOGIC_OUT_LOW);
        Ok(())
    }
    fn get_out_imp(&self) -> PropValue {
        PropValue::Float(self.logic.out_imp)
    }
    fn set_out_imp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.out_imp = expect_float("Out_Imped", v)?.clamp(1e-12, 1e15);
        self.mark(LOGIC_OUT_IMP);
        Ok(())
    }
    fn get_delay(&self) -> PropValue {
        PropValue::Float(self.logic.delay_s)
    }
    fn set_delay(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.delay_s = expect_float("Tpd_ps", v)?.clamp(0.0, 1e6);
        self.mark(LOGIC_DELAY);
        Ok(())
    }
    fn get_rise(&self) -> PropValue {
        PropValue::Float(self.logic.rise_s)
    }
    fn set_rise(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.rise_s = expect_float("Tr_ps", v)?.clamp(1e-12, 1e6);
        self.mark(LOGIC_RISE);
        Ok(())
    }
    fn get_fall(&self) -> PropValue {
        PropValue::Float(self.logic.fall_s)
    }
    fn set_fall(&mut self, v: PropValue) -> Result<(), PropError> {
        self.logic.fall_s = expect_float("Tf_ps", v)?.clamp(1e-12, 1e6);
        self.mark(LOGIC_FALL);
        Ok(())
    }
}

impl Component for Subcircuit {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Subcircuit."
    }

    fn props() -> &'static [PropDef<Self>] {
        const LS: PropDef<Subcircuit> = {
            let mut p = PropDef::bool(
                "LogicSymbol",
                "Logic Symbol",
                Subcircuit::get_logic_symbol,
                Subcircuit::set_logic_symbol,
            )
            .with_info("Render pins grouped by logic ports instead of physical IC layout.")
            .with_info(
                "If yes, use A logic symbol representation.\nIf no, use a \"Chip\" representation.",
            )
            .with_info(
                "If yes, use A logic symbol representation.\nIf no, use a \"Chip\" representation.",
            );
            p.structural = true;
            p
        };
        const PKG: PropDef<Subcircuit> = {
            let mut p = PropDef::string(
                "Package",
                "Package",
                Subcircuit::get_package,
                Subcircuit::set_package,
            )
            .with_info("Package used to represent this subcircuit.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Subcircuit>] = &[
            LS,
            PKG,
            PropDef::string(
                "Device",
                "Device",
                Subcircuit::get_device,
                Subcircuit::set_device,
            )
            .with_info("Target device part name and model."),
            PropDef::string(
                "Program",
                "Program",
                Subcircuit::get_program,
                Subcircuit::set_program,
            )
            .with_info("Path to the compiled firmware binary for nested MCUs."),
            PropDef::float(
                "Input_High_V",
                "Low to High Threshold",
                "V",
                -1e3,
                1e3,
                Subcircuit::get_in_high,
                Subcircuit::set_in_high,
            )
            .with_info("Input threshold copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Input_Low_V",
                "High to Low Threshold",
                "V",
                -1e3,
                1e3,
                Subcircuit::get_in_low,
                Subcircuit::set_in_low,
            )
            .with_info("Input threshold copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Input_Imped",
                "Input Impedance",
                "Ω",
                1e-12,
                1e15,
                Subcircuit::get_in_imp,
                Subcircuit::set_in_imp,
            )
            .with_info("Input impedance copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Out_High_V",
                "Output High Voltage",
                "V",
                -1e3,
                1e3,
                Subcircuit::get_out_high,
                Subcircuit::set_out_high,
            )
            .with_info("Output high voltage copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Out_Low_V",
                "Output Low Voltage",
                "V",
                -1e3,
                1e3,
                Subcircuit::get_out_low,
                Subcircuit::set_out_low,
            )
            .with_info("Output low voltage copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Out_Imped",
                "Output Impedance",
                "Ω",
                1e-12,
                1e15,
                Subcircuit::get_out_imp,
                Subcircuit::set_out_imp,
            )
            .with_info("Output impedance copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Tpd_ps",
                "Gate Delay",
                "s",
                0.0,
                1e6,
                Subcircuit::get_delay,
                Subcircuit::set_delay,
            )
            .with_info("Propagation delay copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Tr_ps",
                "Rise Time",
                "s",
                1e-12,
                1e6,
                Subcircuit::get_rise,
                Subcircuit::set_rise,
            )
            .with_info("Rise time copied onto logic parts inside this subcircuit."),
            PropDef::float(
                "Tf_ps",
                "Fall Time",
                "s",
                1e-12,
                1e6,
                Subcircuit::get_fall,
                Subcircuit::set_fall,
            )
            .with_info("Fall time copied onto logic parts inside this subcircuit."),
        ];
        PROPS
    }

    fn saves_prop(&self, id: &str) -> bool {
        if LOGIC_PROP_IDS.contains(&id) {
            self.is_logic()
        } else if id == "Program" {
            !self.program.is_empty()
        } else {
            true
        }
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        let has_mcu = !self.program.is_empty() || subcircuit::has_nested_mcu(&self.nested_src);
        let rows: Vec<_> = self
            .prop_rows()
            .into_iter()
            .filter(|r| self.is_logic() || !LOGIC_PROP_IDS.contains(&r.name))
            .filter(|r| has_mcu || r.name != "Program")
            .collect();
        if !self.is_logic() {
            return vec![super::PropGroup::new("Main", rows)];
        }
        super::group_rows_by(
            rows,
            &[
                ("Main", &["LogicSymbol", "Package", "Device", "Program"]),
                (
                    "Electric",
                    &[
                        "Input_High_V",
                        "Input_Low_V",
                        "Input_Imped",
                        "Out_High_V",
                        "Out_Low_V",
                        "Out_Imped",
                    ],
                ),
                ("Timing", &["Tpd_ps", "Tr_ps", "Tf_ps"]),
            ],
        )
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

    fn visual_rect(&self) -> Rect {
        let mut r = self.body();
        if !self.logic_symbol {
            for child in &self.board_children {
                let child_vis = map_rect(
                    Point::new(child.placement.x, child.placement.y),
                    child.part.visual_rect(),
                    child.placement.rotation,
                    child.placement.hflip,
                    child.placement.vflip,
                );
                r = r.united(child_vis);
            }
        }
        r
    }

    fn interact_press(&mut self, local: Point) -> bool {
        if self.logic_symbol || self.board_children.is_empty() {
            return false;
        }
        for idx in (0..self.board_children.len()).rev() {
            let Some(child_local) = self.board_child_local(idx, local) else {
                continue;
            };
            if !self.board_children[idx]
                .part
                .body()
                .contains_point(child_local)
            {
                continue;
            }
            if self.board_children[idx].part.interact_press(child_local) {
                self.active_board_child = Some(idx);
                self.last_interacted_child = Some(idx);
                return true;
            }
        }
        false
    }

    fn interact_move(&mut self, local: Point) -> bool {
        let Some(idx) = self.active_board_child else {
            return false;
        };
        let Some(child_local) = self.board_child_local(idx, local) else {
            return false;
        };
        if self.board_children[idx].part.interact_move(child_local) {
            self.last_interacted_child = Some(idx);
            true
        } else {
            false
        }
    }

    fn interact_release(&mut self, local: Point) -> bool {
        let Some(idx) = self.active_board_child.take() else {
            return false;
        };
        let Some(child_local) = self.board_child_local(idx, local) else {
            return false;
        };
        if self.board_children[idx].part.interact_release(child_local) {
            self.last_interacted_child = Some(idx);
            true
        } else {
            false
        }
    }

    fn interact_cancel(&mut self) -> bool {
        let Some(idx) = self.active_board_child.take() else {
            return false;
        };
        if let Some(ch) = self.board_children.get_mut(idx) {
            let changed = ch.part.interact_cancel();
            if changed {
                self.last_interacted_child = Some(idx);
            }
            changed
        } else {
            false
        }
    }

    fn interact_toggle(&mut self, local: Point) -> bool {
        if self.logic_symbol || self.board_children.is_empty() {
            return false;
        }
        for idx in (0..self.board_children.len()).rev() {
            let Some(child_local) = self.board_child_local(idx, local) else {
                continue;
            };
            if !self.board_children[idx]
                .part
                .body()
                .contains_point(child_local)
            {
                continue;
            }
            if self.board_children[idx].part.interact_toggle(child_local) {
                self.last_interacted_child = Some(idx);
                return true;
            }
        }
        false
    }

    fn interact_wheel(&mut self, local: Point, delta: f64) -> bool {
        if self.logic_symbol || self.board_children.is_empty() {
            return false;
        }
        for idx in (0..self.board_children.len()).rev() {
            let Some(child_local) = self.board_child_local(idx, local) else {
                continue;
            };
            if !self.board_children[idx]
                .part
                .body()
                .contains_point(child_local)
            {
                continue;
            }
            if self.board_children[idx]
                .part
                .interact_wheel(child_local, delta)
            {
                self.last_interacted_child = Some(idx);
                return true;
            }
        }
        false
    }
}

impl Stampable for Subcircuit {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // Subcircuits are flattened into primitive components prior to matrix stamping.
    }
}

impl super::drawable::Drawable for Subcircuit {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let w = self.package.body_w().max(8.0);
        let h = self.package.body_h().max(8.0);
        if !self.board_children.is_empty() && !self.logic_symbol {
            let fill = if self.package.custom_color && !self.package.bckgndcolor.is_empty() {
                parse_hex(&self.package.bckgndcolor)
            } else {
                let (r, g, b, a) = ColorTheme::get_rgba(ColorId::ChipBody, false);
                Color::rgba(r, g, b, a)
            };
            d.fill_round_rect(0.0, 0.0, w, h, 2.0, fill);
            d.stroke_round_rect(0.0, 0.0, w, h, 2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            if ctx.is_active() {
                d.fill_round_rect(4.0, 4.0, 4.0, 4.0, 2.0, ctx.pal.exposed_highlight);
                d.stroke_round_rect(4.0, 4.0, 4.0, 4.0, 2.0, ctx.pal.border, 0.5);
            }
            for child in &self.board_children {
                let sim_id = self.child_sim_id(ctx.item_id, &child.inner_id);
                let child_ctx = PaintCtx {
                    canvas: ctx.canvas,
                    pal: ctx.pal,
                    scale: ctx.scale,
                    item_id: &sim_id,
                    on_board: true,
                    active_item_id: None,
                };
                d.push(
                    child.placement.x,
                    child.placement.y,
                    child.placement.rotation,
                    child.placement.hflip as f64,
                    child.placement.vflip as f64,
                );
                child.part.paint(d, &child_ctx);
                d.pop();
            }
            return true;
        }
        let clean_pkg = crate::package::clean_chip_label(&self.package.name);
        let label = if !clean_pkg.is_empty() {
            clean_pkg
        } else if !self.device.is_empty() {
            &self.device
        } else {
            ""
        };
        super::drawable::paint_dip_package(
            d,
            ctx.pal,
            w,
            h,
            label,
            self.logic_symbol,
            self.package.custom_color,
            &self.package.bckgndcolor,
            ctx.is_active(),
        );
        true
    }
}

impl Scene {
    pub fn add_subcircuit(
        &mut self,
        device: &str,
        x: f64,
        y: f64,
        search: &SubcSearch,
    ) -> Option<String> {
        let id = format!("{device}-{}", self.next_subc);
        let view = subcircuit::load_view(&id, device, search, false, None).ok()?;
        let mut item = Item::subcircuit(
            id.clone(),
            x,
            y,
            view.device,
            view.package,
            view.nested_src,
            view.nested_path,
            view.logic_symbol,
        );
        if let super::Part::Subcircuit(subc) = &mut item.kind {
            subc.packages = view.packages;
            if let Some(prog) = subcircuit::nested_mcu_program(&subc.nested_src) {
                subc.program = prog;
            }
        }
        self.add_saved_item(item);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::PkgPin;

    #[test]
    fn subcircuit_defaults_and_props() {
        let mut sub = Subcircuit::default();
        assert_eq!(sub.type_id(), "Subcircuit");
        assert_eq!(sub.get_prop_text("LogicSymbol").unwrap(), "false");
        assert_eq!(sub.get_prop_text("Package").unwrap(), "");
        assert_eq!(sub.get_prop_text("Device").unwrap(), "");

        sub.set_prop_text("LogicSymbol", "true").unwrap();
        assert_eq!(sub.logic_symbol, true);
        assert_eq!(sub.get_prop_text("LogicSymbol").unwrap(), "true");

        sub.set_prop_text("Package", "DIP8").unwrap();
        assert_eq!(sub.package.name, "DIP8");
        assert_eq!(sub.get_prop_text("Package").unwrap(), "DIP8");

        sub.set_prop_text("Device", "74HC00").unwrap();
        assert_eq!(sub.device, "74HC00");
        assert_eq!(sub.get_prop_text("Device").unwrap(), "74HC00");
    }

    #[test]
    fn subcircuit_pin_geoms_and_body() {
        let mut sub = Subcircuit::default();
        assert_eq!(sub.pin_geoms().len(), 0);
        assert!(sub.body().w > 0.0 && sub.body().h > 0.0);

        sub.package
            .pins
            .push(PkgPin::new("1", "IN", "", -8, 8, 180));
        assert_eq!(sub.pin_geoms().len(), 1);
        assert_eq!(sub.pin_geoms()[0].suffix, "1");
        assert_eq!(sub.pin_geoms()[0].local.x, -8.0);
    }

    #[test]
    fn subcircuit_to_element_kind() {
        let sub = Subcircuit {
            device: "74HC00".into(),
            package: Package {
                name: "DIP8".into(),
                ..Package::default()
            },
            packages: BTreeMap::new(),
            nested_src: String::new(),
            nested_path: None,
            logic_symbol: true,
            program: String::new(),
            firmware_dir: None,
            logic: LogicOverride::default(),
            board_children: Vec::new(),
            active_board_child: None,
            last_interacted_child: None,
        };
        match sub.to_element_kind() {
            Kind::Subcircuit {
                device,
                logic_symbol,
                package_name,
            } => {
                assert_eq!(device, "74HC00");
                assert_eq!(logic_symbol, true);
                assert_eq!(package_name, Some("DIP8".into()));
            }
            other => panic!("expected Kind::Subcircuit, got {other:?}"),
        }
    }

    #[test]
    fn subcircuit_logic_symbol_selects_package() {
        let mut dip = Package::default();
        dip.pins.push(PkgPin::new("1", "IN", "", -8, 8, 180));
        let mut ls = Package::default();
        ls.pins.push(PkgPin::new("A", "IN", "", -8, 8, 180));
        let mut packages = BTreeMap::new();
        packages.insert("1- demo_DIP".into(), dip);
        packages.insert("2- demo_LS".into(), ls);

        let mut sub = Subcircuit::default();
        sub.packages = packages;
        sub.package.name = "1- demo_DIP".into();
        sub.set_prop_text("LogicSymbol", "true").unwrap();
        assert!(sub.logic_symbol);
        assert_eq!(sub.package.name, "2- demo_LS");
        assert_eq!(sub.pin_geoms()[0].suffix, "A");

        sub.set_prop_text("Package", "1- demo_DIP").unwrap();
        assert!(!sub.logic_symbol);
        assert_eq!(sub.package.name, "1- demo_DIP");
        assert_eq!(sub.pin_geoms()[0].suffix, "1");
    }

    fn package_item(typ: &str) -> String {
        format!(
            r#"<item itemtype="Package" CircId="Package-1" label="DIP" width="2" height="2" SubcType="{typ}" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=A; label=A" />"#
        )
    }

    fn and_item(id: &str) -> String {
        format!(r#"<item itemtype="And Gate" CircId="{id}" Num_Inputs="2" Tpd_ps="20 ns" />"#)
    }

    #[test]
    fn logic_properties_belong_to_logic_packages() {
        use crate::circ1::{GraphicAttrs, write_component_item};

        let plain = Subcircuit::default();
        assert!(plain.prop_groups().iter().all(|g| g.name == "Main"));
        assert!(
            plain
                .prop_groups()
                .iter()
                .flat_map(|g| g.rows.iter())
                .all(|r| r.name != "Input_High_V")
        );
        let xml = write_component_item("S", &plain, &GraphicAttrs::at(0.0, 0.0));
        assert!(!xml.contains("Input_High_V"), "{xml}");
        assert!(plain.simulation_logic().is_none());

        let mut logic = Subcircuit::default();
        logic.package.subc_type = SubcType::Logic;
        let names: Vec<_> = logic.prop_groups().iter().map(|g| g.name).collect();
        assert_eq!(names, ["Main", "Electric", "Timing"]);
        assert_eq!(logic.get_prop_text("Input_High_V").unwrap(), "2.5 V");
        assert_eq!(logic.get_prop_text("Tpd_ps").unwrap(), "10 ns");
        assert_eq!(logic.get_prop_text("Tr_ps").unwrap(), "3 ns");
        assert_eq!(logic.get_prop_text("Input_Imped").unwrap(), "1 GΩ");
        assert!(logic.simulation_logic().is_none());

        logic.set_prop_text("Input_High_V", "3.3 V").unwrap();
        assert_eq!(logic.simulation_logic().unwrap().in_high_v, 3.3);
        let xml = write_component_item("S", &logic, &GraphicAttrs::at(0.0, 0.0));
        assert!(xml.contains("Input_High_V=\"3.3 V\""), "{xml}");
    }

    #[test]
    fn logic_override_copies_onto_logic_children() {
        use crate::elements::Kind;
        use crate::subcircuit::{SubcSearch, instantiate};

        let board = format!("{}\n{}", package_item("None"), and_item("BoardAnd"));
        let nest = format!("{}\n{}", package_item("Logic"), and_item("NestAnd"));
        let parent = format!(
            "{}\n{}\n<item itemtype=\"Resistor\" CircId=\"R-1\" Resistance=\"1 kΩ\" />\n<item itemtype=\"Subcircuit\" CircId=\"board-1\" Device=\"board\" />\n<item itemtype=\"Subcircuit\" CircId=\"nest-1\" Device=\"nest\" />\n",
            package_item("Logic"),
            and_item("TopAnd")
        );
        let search = SubcSearch::empty()
            .with_memory("board", board)
            .with_memory("nest", nest);

        let mut sub = Subcircuit::default();
        sub.package.subc_type = SubcType::Logic;
        sub.nested_src = parent.clone();
        let untouched = instantiate(
            "chip-1",
            "chip",
            &parent,
            &search,
            false,
            None,
            0,
            sub.simulation_logic(),
        )
        .expect("untouched logic subcircuit");
        let top = gate_high(&untouched.components, "TopAnd");
        assert!((top.0 - 2.5).abs() < 1e-9, "{}", top.0);
        assert_eq!(top.1, 20_000);

        sub.set_prop_text("Input_High_V", "3.3 V").unwrap();
        let inst = instantiate(
            "chip-1",
            "chip",
            &sub.nested_src,
            &search,
            false,
            None,
            0,
            sub.simulation_logic(),
        )
        .expect("logic subcircuit");

        let top = gate_high(&inst.components, "TopAnd");
        assert!((top.0 - 3.3).abs() < 1e-9, "{}", top.0);
        assert!((top.2 - 3.3).abs() < 1e-9, "pin {}", top.2);
        assert_eq!(top.1, 20_000, "unset delay stays on the child");

        let board_gate = gate_high(&inst.components, "BoardAnd");
        assert!(
            (board_gate.0 - 2.5).abs() < 1e-9,
            "plain nested subcircuit keeps {}",
            board_gate.0
        );

        let nest_gate = gate_high(&inst.components, "NestAnd");
        assert!(
            (nest_gate.0 - 3.3).abs() < 1e-9,
            "nested logic subcircuit got {}",
            nest_gate.0
        );

        let resistor = inst
            .components
            .iter()
            .find(|c| matches!(c.kind, Kind::Resistor { .. }));
        match resistor.map(|c| &c.kind) {
            Some(Kind::Resistor { resistance }) => assert!((resistance - 1000.0).abs() < 1e-6),
            other => panic!("expected resistor, got {other:?}"),
        }
    }

    fn gate_high(comps: &[crate::elements::Comp], id_frag: &str) -> (f64, u64, f64) {
        let gate = comps
            .iter()
            .find(|c| c.id.contains(id_frag))
            .unwrap_or_else(|| {
                panic!("missing {id_frag}");
            });
        match &gate.kind {
            Kind::Gate(g) => (
                g.family.inp_high_v,
                g.family.delay_ps(),
                g.inputs[0].inp_high_v,
            ),
            other => panic!("{id_frag} is {other:?}"),
        }
    }
}
