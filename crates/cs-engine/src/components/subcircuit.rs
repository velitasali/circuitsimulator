//! Subcircuit: nested schematic item with IC package footprint.

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::package::Package;

/// Subcircuit instance referencing an inner schematic or device package.
#[derive(Clone, Debug, PartialEq)]
pub struct Subcircuit {
    pub device: String,
    pub package: Package,
    pub nested_src: String,
    pub nested_path: Option<String>,
    pub logic_symbol: bool,
}

impl crate::canvas::Item {
    pub fn subcircuit(
        id: impl Into<String>,
        x: f64,
        y: f64,
        device: impl Into<String>,
        package: crate::package::Package,
        nested_src: impl Into<String>,
        nested_path: Option<String>,
        logic_symbol: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Subcircuit {
                device: device.into(),
                package,
                nested_src: nested_src.into(),
                nested_path,
                logic_symbol,
            },
        )
    }
}

impl Default for Subcircuit {
    fn default() -> Self {
        Self {
            device: String::new(),
            package: Package::default(),
            nested_src: String::new(),
            nested_path: None,
            logic_symbol: false,
        }
    }
}

impl Subcircuit {
    pub const TYPE_ID: &'static str = "Subcircuit";
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
        self.logic_symbol = expect_bool("LogicSymbol", v)?;
        Ok(())
    }

    fn get_package(&self) -> PropValue {
        PropValue::String(self.package.name.clone())
    }
    fn set_package(&mut self, v: PropValue) -> Result<(), PropError> {
        self.package.name = expect_string("Package", v)?;
        Ok(())
    }

    fn get_device(&self) -> PropValue {
        PropValue::String(self.device.clone())
    }
    fn set_device(&mut self, v: PropValue) -> Result<(), PropError> {
        self.device = expect_string("Device", v)?;
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

impl Stampable for Subcircuit {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // Subcircuits are flattened into primitive components prior to matrix stamping.
    }
}

impl super::drawable::Drawable for Subcircuit {
    fn paint(
        &self,
        d: &mut dyn crate::canvas::draw::Draw,
        ctx: &crate::canvas::draw::PaintCtx,
    ) -> bool {
        let w = self.package.body_w().max(8.0);
        let h = self.package.body_h().max(8.0);
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
            nested_src: String::new(),
            nested_path: None,
            logic_symbol: true,
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
}
