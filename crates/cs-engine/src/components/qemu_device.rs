//! QEMU co-simulation device component (ESP32/STM32).

use std::collections::BTreeMap;

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::package::Package;
use crate::qemu::QemuComp;

/// QEMU device instance with co-simulation process state, package footprint, and configuration.
#[derive(Clone, Debug)]
pub struct QemuDevice {
    pub qemu: QemuComp,
    pub package: Package,
    pub packages: BTreeMap<String, Package>,
    pub logic_symbol: bool,
    pub active: bool,
}

impl crate::canvas::Item {
    pub fn qemu_device_from_view(
        id: impl Into<String>,
        x: f64,
        y: f64,
        view: crate::qemu::QemuView,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            QemuDevice {
                qemu: view.qemu,
                package: view.package,
                packages: view.packages,
                logic_symbol: view.logic_symbol,
                active: true,
            },
        )
    }

    pub fn qemu_device(id: impl Into<String>, x: f64, y: f64, view: crate::qemu::QemuView) -> Self {
        Self::qemu_device_from_view(id, x, y, view)
    }

    pub fn qemu_device_full(
        id: impl Into<String>,
        x: f64,
        y: f64,
        qemu: crate::qemu::QemuComp,
        package: crate::package::Package,
        packages: std::collections::BTreeMap<String, crate::package::Package>,
        logic_symbol: bool,
        active: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            QemuDevice {
                qemu,
                package,
                packages,
                logic_symbol,
                active,
            },
        )
    }
}

impl Default for QemuDevice {
    fn default() -> Self {
        Self {
            qemu: QemuComp::default(),
            package: Package::default(),
            packages: BTreeMap::new(),
            logic_symbol: false,
            active: true,
        }
    }
}

impl PartialEq for QemuDevice {
    fn eq(&self, other: &Self) -> bool {
        self.qemu.device == other.qemu.device
            && self.qemu.firmware == other.qemu.firmware
            && self.qemu.extra_args == other.qemu.extra_args
            && self.package == other.package
            && self.packages == other.packages
            && self.logic_symbol == other.logic_symbol
            && self.active == other.active
    }
}

impl QemuDevice {
    pub const TYPE_ID: &'static str = "QemuDevice";
    pub fn to_element_kind(&self) -> Kind {
        Kind::QemuDevice(self.qemu.clone())
    }

    fn get_device(&self) -> PropValue {
        PropValue::String(self.qemu.device.clone())
    }
    fn set_device(&mut self, v: PropValue) -> Result<(), PropError> {
        self.qemu.device = expect_string("Device", v)?;
        Ok(())
    }

    fn get_program(&self) -> PropValue {
        PropValue::String(self.qemu.firmware.clone())
    }
    fn set_program(&mut self, v: PropValue) -> Result<(), PropError> {
        self.qemu.firmware = expect_string("Program", v)?;
        Ok(())
    }

    fn get_args(&self) -> PropValue {
        PropValue::String(self.qemu.extra_args.clone())
    }
    fn set_args(&mut self, v: PropValue) -> Result<(), PropError> {
        self.qemu.extra_args = expect_string("Args", v)?;
        Ok(())
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

    fn get_active(&self) -> PropValue {
        PropValue::Bool(self.active)
    }
    fn set_active(&mut self, v: PropValue) -> Result<(), PropError> {
        self.active = expect_bool("Active", v)?;
        Ok(())
    }
}

impl Component for QemuDevice {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "QEMU co-simulation device (ESP32/STM32)."
    }

    fn props() -> &'static [PropDef<Self>] {
        const DEV: PropDef<QemuDevice> = {
            let mut p = PropDef::string(
                "Device",
                "Device",
                QemuDevice::get_device,
                QemuDevice::set_device,
            )
            .with_info("Target device part name and model.");
            p.structural = true;
            p
        };
        const LS: PropDef<QemuDevice> = {
            let mut p = PropDef::bool(
                "LogicSymbol",
                "Logic Symbol",
                QemuDevice::get_logic_symbol,
                QemuDevice::set_logic_symbol,
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
        const PKG: PropDef<QemuDevice> = {
            let mut p = PropDef::string(
                "Package",
                "Package",
                QemuDevice::get_package,
                QemuDevice::set_package,
            )
            .with_info("Package used to represent this subcircuit.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<QemuDevice>] = &[
            DEV,
            PropDef::string(
                "Program",
                "Program File",
                QemuDevice::get_program,
                QemuDevice::set_program,
            )
            .with_info("Path to the firmware binary image file."),
            PropDef::string(
                "Args",
                "Extra Arguments",
                QemuDevice::get_args,
                QemuDevice::set_args,
            )
            .with_info("Additional command line arguments passed to emulator."),
            LS,
            PKG,
            PropDef::bool(
                "Active",
                "Active",
                QemuDevice::get_active,
                QemuDevice::set_active,
            )
            .with_info("Enable co-simulation with external emulator."),
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

impl Stampable for QemuDevice {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // QEMU pins are stamped by the circuit solver.
    }
}

impl super::drawable::Drawable for QemuDevice {
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
        } else if !self.qemu.device.is_empty() {
            &self.qemu.device
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

impl crate::canvas::Scene {
    pub fn add_qemu(
        &mut self,
        device: &str,
        x: f64,
        y: f64,
        search: &crate::subcircuit::SubcSearch,
    ) -> Option<String> {
        if device.is_empty() {
            return None;
        }
        let id = format!("{device}-{}", self.next_mcu);
        self.next_mcu += 1;
        let view = crate::qemu::instantiate_qemu_view(&id, device, search);
        let item = crate::canvas::Item::qemu_device_from_view(id.clone(), x, y, view);
        self.add_saved_item(item);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::PkgPin;

    #[test]
    fn qemu_device_defaults_and_props() {
        let mut qemu = QemuDevice::default();
        assert_eq!(qemu.type_id(), "QemuDevice");
        assert_eq!(
            qemu.description(),
            "QEMU co-simulation device (ESP32/STM32)."
        );
        assert_eq!(qemu.get_prop_text("Device").unwrap(), "Esp32");
        assert_eq!(qemu.get_prop_text("Program").unwrap(), "");
        assert_eq!(qemu.get_prop_text("Args").unwrap(), "");
        assert_eq!(qemu.get_prop_text("LogicSymbol").unwrap(), "false");
        assert_eq!(qemu.get_prop_text("Package").unwrap(), "");
        assert_eq!(qemu.get_prop_text("Active").unwrap(), "true");

        qemu.set_prop_text("Device", "STM32F103C8").unwrap();
        assert_eq!(qemu.qemu.device, "STM32F103C8");
        assert_eq!(qemu.get_prop_text("Device").unwrap(), "STM32F103C8");

        qemu.set_prop_text("Program", "firmware.bin").unwrap();
        assert_eq!(qemu.qemu.firmware, "firmware.bin");
        assert_eq!(qemu.get_prop_text("Program").unwrap(), "firmware.bin");

        qemu.set_prop_text("Args", "-d in_asm").unwrap();
        assert_eq!(qemu.qemu.extra_args, "-d in_asm");
        assert_eq!(qemu.get_prop_text("Args").unwrap(), "-d in_asm");

        qemu.set_prop_text("LogicSymbol", "true").unwrap();
        assert_eq!(qemu.logic_symbol, true);

        qemu.set_prop_text("Package", "LQFP48").unwrap();
        assert_eq!(qemu.package.name, "LQFP48");

        qemu.set_prop_text("Active", "false").unwrap();
        assert_eq!(qemu.active, false);
    }

    #[test]
    fn qemu_device_pin_geoms_and_body() {
        let mut qemu = QemuDevice::default();
        assert_eq!(qemu.pin_geoms().len(), 0);
        assert!(qemu.body().w > 0.0 && qemu.body().h > 0.0);

        qemu.package
            .pins
            .push(PkgPin::new("PA0", "IO", "", -8, 8, 180));
        assert_eq!(qemu.pin_geoms().len(), 1);
        assert_eq!(qemu.pin_geoms()[0].suffix, "PA0");
        assert_eq!(qemu.pin_geoms()[0].local.x, -8.0);
    }

    #[test]
    fn qemu_device_to_element_kind() {
        let qemu = QemuDevice::default();
        match qemu.to_element_kind() {
            Kind::QemuDevice(q) => {
                assert_eq!(q.family, crate::qemu::QemuFamily::Esp32);
            }
            other => panic!("expected Kind::QemuDevice, got {other:?}"),
        }
    }
}
