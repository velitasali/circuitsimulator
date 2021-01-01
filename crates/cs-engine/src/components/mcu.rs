//! Microcontroller component: MCU instance with package footprint and register/firmware state.

use std::collections::BTreeMap;

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::mcu::McuComp;
use crate::package::Package;

const MIN_FREQ_HZ: f64 = 0.0;
const MAX_FREQ_HZ: f64 = 100.0 * 1e6;

/// MCU instance with chip model, package footprint, and configuration.
#[derive(Clone, Debug)]
pub struct Mcu {
    pub device: String,
    pub mcu: McuComp,
    pub package: Package,
    pub packages: BTreeMap<String, Package>,
    pub logic_symbol: bool,
    pub is_main_comp: bool,
    pub save_eepr: bool,
    pub rst_enabled: bool,
    pub ext_osc: bool,
    pub wdt_enabled: bool,
    pub clk_out: bool,
    pub pgm: String,
}

impl crate::canvas::Item {
    pub fn mcu_from_view(
        id: impl Into<String>,
        x: f64,
        y: f64,
        device: impl Into<String>,
        view: crate::mcu::McuView,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Mcu {
                device: device.into(),
                mcu: view.mcu,
                package: view.package,
                packages: view.packages,
                logic_symbol: view.logic_symbol,
                is_main_comp: false,
                save_eepr: false,
                rst_enabled: true,
                ext_osc: false,
                wdt_enabled: false,
                clk_out: false,
                pgm: String::new(),
            },
        )
    }

    pub fn mcu(id: impl Into<String>, x: f64, y: f64, view: crate::mcu::McuView) -> Self {
        Self::mcu_from_view(id, x, y, "mcu", view)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mcu_full(
        id: impl Into<String>,
        x: f64,
        y: f64,
        device: impl Into<String>,
        mcu: crate::mcu::McuComp,
        package: crate::package::Package,
        packages: std::collections::BTreeMap<String, crate::package::Package>,
        logic_symbol: bool,
        save_eepr: bool,
        rst_enabled: bool,
        ext_osc: bool,
        wdt_enabled: bool,
        clk_out: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Mcu {
                device: device.into(),
                mcu,
                package,
                packages,
                logic_symbol,
                is_main_comp: false,
                save_eepr,
                rst_enabled,
                ext_osc,
                wdt_enabled,
                clk_out,
                pgm: String::new(),
            },
        )
    }
}

impl Default for Mcu {
    fn default() -> Self {
        Self {
            device: String::new(),
            mcu: McuComp::default(),
            package: Package::default(),
            packages: BTreeMap::new(),
            logic_symbol: false,
            is_main_comp: false,
            save_eepr: false,
            rst_enabled: true,
            ext_osc: false,
            wdt_enabled: false,
            clk_out: false,
            pgm: String::new(),
        }
    }
}

impl PartialEq for Mcu {
    fn eq(&self, other: &Self) -> bool {
        self.device == other.device
            && self.mcu.device.id == other.mcu.device.id
            && self.mcu.device.freq == other.mcu.device.freq
            && self.mcu.force_freq == other.mcu.force_freq
            && self.mcu.auto_load == other.mcu.auto_load
            && self.mcu.save_pgm == other.mcu.save_pgm
            && self.mcu.firmware == other.mcu.firmware
            && self.package == other.package
            && self.packages == other.packages
            && self.logic_symbol == other.logic_symbol
            && self.is_main_comp == other.is_main_comp
            && self.save_eepr == other.save_eepr
            && self.rst_enabled == other.rst_enabled
            && self.ext_osc == other.ext_osc
            && self.wdt_enabled == other.wdt_enabled
            && self.clk_out == other.clk_out
            && self.pgm == other.pgm
    }
}

impl Mcu {
    pub const TYPE_ID: &'static str = "Mcu";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Mcu(self.mcu.clone())
    }

    fn get_device(&self) -> PropValue {
        PropValue::String(self.device.clone())
    }
    fn set_device(&mut self, v: PropValue) -> Result<(), PropError> {
        self.device = expect_string("Device", v)?;
        Ok(())
    }

    fn get_frequency(&self) -> PropValue {
        PropValue::Float(self.mcu.device.freq)
    }
    fn set_frequency(&mut self, v: PropValue) -> Result<(), PropError> {
        let f = expect_float("Frequency", v)?;
        self.mcu.device.force_freq(f);
        Ok(())
    }

    fn get_force_freq(&self) -> PropValue {
        PropValue::Bool(self.mcu.force_freq)
    }
    fn set_force_freq(&mut self, v: PropValue) -> Result<(), PropError> {
        self.mcu.force_freq = expect_bool("ForceFreq", v)?;
        Ok(())
    }

    fn get_program(&self) -> PropValue {
        PropValue::String(self.mcu.firmware.clone().unwrap_or_default())
    }
    fn set_program(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Program", v)?;
        self.mcu.firmware = if s.trim().is_empty() {
            None
        } else {
            Some(s.trim().to_string())
        };
        Ok(())
    }

    fn get_auto_load(&self) -> PropValue {
        PropValue::Bool(self.mcu.auto_load)
    }
    fn set_auto_load(&mut self, v: PropValue) -> Result<(), PropError> {
        self.mcu.auto_load = expect_bool("AutoLoad", v)?;
        Ok(())
    }

    fn get_save_pgm(&self) -> PropValue {
        PropValue::Bool(self.mcu.save_pgm)
    }
    fn set_save_pgm(&mut self, v: PropValue) -> Result<(), PropError> {
        self.mcu.save_pgm = expect_bool("SavePgm", v)?;
        Ok(())
    }

    fn get_pgm(&self) -> PropValue {
        PropValue::String(self.pgm.clone())
    }
    fn set_pgm(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Pgm", v)?;
        self.mcu.load_pgm_str(&s);
        self.pgm = s;
        Ok(())
    }

    fn get_logic_symbol(&self) -> PropValue {
        PropValue::Bool(self.logic_symbol)
    }
    fn set_logic_symbol(&mut self, v: PropValue) -> Result<(), PropError> {
        let ls = expect_bool("LogicSymbol", v)?;
        self.logic_symbol = ls;
        self.package =
            crate::mcu::canvas_package(&self.mcu, &self.mcu.device.id, &self.packages, ls, None);
        self.package.logic_symbol = ls;
        Ok(())
    }

    fn get_package(&self) -> PropValue {
        if self.packages.contains_key(&self.package.name) {
            return PropValue::String(self.package.name.clone());
        }
        for (key, _) in &self.packages {
            if self.logic_symbol && key.ends_with("LS") {
                return PropValue::String(key.clone());
            }
            if !self.logic_symbol && key.ends_with("DIP") {
                return PropValue::String(key.clone());
            }
        }
        if let Some((first_key, _)) = self.packages.iter().next() {
            return PropValue::String(first_key.clone());
        }
        PropValue::String(self.package.name.clone())
    }
    fn set_package(&mut self, v: PropValue) -> Result<(), PropError> {
        let name = match v {
            PropValue::String(s) | PropValue::Enum(s) => s,
            _ => expect_string("Package", v)?,
        };
        let is_ls = if name.ends_with("LS") || name.ends_with("_LS") || name.ends_with("-LS") {
            true
        } else if name.ends_with("DIP")
            || name.ends_with("_DIP")
            || name.ends_with("-DIP")
            || name.contains("DIP")
        {
            false
        } else {
            self.logic_symbol
        };
        self.logic_symbol = is_ls;
        self.package = crate::mcu::canvas_package(
            &self.mcu,
            &self.mcu.device.id,
            &self.packages,
            is_ls,
            if name.is_empty() { None } else { Some(&name) },
        );
        self.package.name = name;
        self.package.logic_symbol = is_ls;
        Ok(())
    }

    fn get_save_eepr(&self) -> PropValue {
        PropValue::Bool(self.save_eepr)
    }
    fn set_save_eepr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.save_eepr = expect_bool("SaveEepr", v)?;
        Ok(())
    }

    fn get_main_comp(&self) -> PropValue {
        PropValue::Bool(self.is_main_comp)
    }
    fn set_main_comp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.is_main_comp = expect_bool("MainComp", v)?;
        Ok(())
    }

    fn get_rst_enabled(&self) -> PropValue {
        PropValue::Bool(self.rst_enabled)
    }
    fn set_rst_enabled(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rst_enabled = expect_bool("RstEnabled", v)?;
        Ok(())
    }

    fn get_ext_osc(&self) -> PropValue {
        PropValue::Bool(self.ext_osc)
    }
    fn set_ext_osc(&mut self, v: PropValue) -> Result<(), PropError> {
        self.ext_osc = expect_bool("ExtOsc", v)?;
        Ok(())
    }

    fn get_wdt_enabled(&self) -> PropValue {
        PropValue::Bool(self.wdt_enabled)
    }
    fn set_wdt_enabled(&mut self, v: PropValue) -> Result<(), PropError> {
        self.wdt_enabled = expect_bool("WdtEnabled", v)?;
        Ok(())
    }

    fn get_clk_out(&self) -> PropValue {
        PropValue::Bool(self.clk_out)
    }
    fn set_clk_out(&mut self, v: PropValue) -> Result<(), PropError> {
        self.clk_out = expect_bool("ClkOut", v)?;
        Ok(())
    }
}

impl Component for Mcu {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Microcontroller."
    }

    fn props() -> &'static [PropDef<Self>] {
        const DEV: PropDef<Mcu> = {
            let mut p = PropDef::string("Device", "Device", Mcu::get_device, Mcu::set_device)
                .with_info("Target device part name and model.");
            p.structural = true;
            p.show_by_default = false;
            p
        };
        const MAIN_COMP: PropDef<Mcu> = {
            let mut p = PropDef::bool(
                "MainComp",
                "Main Component",
                Mcu::get_main_comp,
                Mcu::set_main_comp,
            )
            .with_info("Whether this MCU is the main component of a composite subcircuit.");
            p.show_by_default = false;
            p
        };
        const LS: PropDef<Mcu> = {
            let mut p = PropDef::bool(
                "LogicSymbol",
                "Logic Symbol",
                Mcu::get_logic_symbol,
                Mcu::set_logic_symbol,
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
        const PKG: PropDef<Mcu> = {
            let mut p = PropDef::string("Package", "Package", Mcu::get_package, Mcu::set_package)
                .with_info("Package used to represent this subcircuit.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Mcu>] = &[
            DEV,
            MAIN_COMP,
            PropDef::float(
                "Frequency",
                "Frequency",
                "Hz",
                MIN_FREQ_HZ,
                MAX_FREQ_HZ,
                Mcu::get_frequency,
                Mcu::set_frequency,
            )
            .with_info("Set output frequency."),
            PropDef::bool(
                "ForceFreq",
                "Force Frequency",
                Mcu::get_force_freq,
                Mcu::set_force_freq,
            )
            .with_info("Override firmware frequency with configured clock frequency."),
            PropDef::string(
                "Program",
                "Program File",
                Mcu::get_program,
                Mcu::set_program,
            )
            .with_info("Path to the firmware binary image file."),
            PropDef::bool(
                "AutoLoad",
                "Auto Load",
                Mcu::get_auto_load,
                Mcu::set_auto_load,
            )
            .with_info("Automatically reload firmware when the file changes on disk."),
            PropDef::bool(
                "SavePgm",
                "Save Program",
                Mcu::get_save_pgm,
                Mcu::set_save_pgm,
            )
            .with_info("Save firmware binary into the circuit file."),
            PropDef::string("Pgm", "Flash Words", Mcu::get_pgm, Mcu::set_pgm)
                .with_info("Embedded firmware hex data stored within the circuit."),
            LS,
            PKG,
            PropDef::bool(
                "SaveEepr",
                "Save EEPROM",
                Mcu::get_save_eepr,
                Mcu::set_save_eepr,
            )
            .with_info("Save EEPROM non-volatile data into the circuit file."),
            PropDef::bool(
                "RstEnabled",
                "Reset Enabled",
                Mcu::get_rst_enabled,
                Mcu::set_rst_enabled,
            )
            .with_info("Enable external Reset pin function."),
            PropDef::bool(
                "ExtOsc",
                "External Oscillator",
                Mcu::get_ext_osc,
                Mcu::set_ext_osc,
            )
            .with_info("Enable external oscillator inputs."),
            PropDef::bool(
                "WdtEnabled",
                "Watchdog Timer Enabled",
                Mcu::get_wdt_enabled,
                Mcu::set_wdt_enabled,
            )
            .with_info("Enable internal Watchdog Timer hardware."),
            PropDef::bool("ClkOut", "Clock Output", Mcu::get_clk_out, Mcu::set_clk_out)
                .with_info("Enable system clock output on clock out pin."),
        ];
        PROPS
    }

    fn prop_rows(&self) -> Vec<super::PropRow> {
        let has_flash = self.mcu.device.flash_size() > 0;
        let has_eeprom = !self.mcu.device.eeprom().is_empty();
        let pkg_options: Vec<String> = self.packages.keys().cloned().collect();

        Self::props()
            .iter()
            .filter(|d| d.show_by_default)
            .map(|d| {
                let mut visible = true;
                if matches!(d.id, "Program" | "AutoLoad" | "SavePgm" | "Pgm") && !has_flash {
                    visible = false;
                }
                if d.id == "SaveEepr" && !has_eeprom {
                    visible = false;
                }

                let mut enabled = true;
                if self.is_main_comp
                    && matches!(
                        d.id,
                        "Package" | "RstEnabled" | "ExtOsc" | "WdtEnabled" | "ClkOut"
                    )
                {
                    enabled = false;
                }

                let (kind, options) = if d.id == "Package" && !pkg_options.is_empty() {
                    ("enum", pkg_options.clone())
                } else {
                    (
                        match d.kind {
                            super::props::PropKind::Float { .. } => "double",
                            super::props::PropKind::Bool => "bool",
                            super::props::PropKind::Int { .. } => "int",
                            super::props::PropKind::Enum { .. } => "enum",
                            super::props::PropKind::String => "string",
                        },
                        match d.kind {
                            super::props::PropKind::Enum { options } => {
                                options.iter().map(|s| s.to_string()).collect()
                            }
                            _ => Vec::new(),
                        },
                    )
                };

                super::PropRow {
                    name: d.id,
                    kind,
                    caption: d.caption,
                    info: d.info,
                    unit: d.unit,
                    options,
                    visible,
                    enabled,
                }
            })
            .collect()
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                (
                    "Main",
                    &[
                        "Package",
                        "LogicSymbol",
                        "Frequency",
                        "ForceFreq",
                        "Program",
                        "AutoLoad",
                        "SavePgm",
                        "Pgm",
                        "SaveEepr",
                    ],
                ),
                ("Config", &["RstEnabled", "ExtOsc", "WdtEnabled", "ClkOut"]),
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
}

impl Stampable for Mcu {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // MCU IO pins are stamped by the circuit solver.
    }
}

impl super::drawable::Drawable for Mcu {
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
        } else if !self.mcu.device.id.is_empty() {
            &self.mcu.device.id
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
    pub fn add_mcu(
        &mut self,
        device: &str,
        x: f64,
        y: f64,
        search: &crate::subcircuit::SubcSearch,
    ) -> Option<String> {
        let mut device = crate::mcu::canonicalize_device(device);
        if device.is_empty() || device == "MCU" || device == "NEW_MCU" {
            device = "mega328".to_string();
        }
        let id = format!("{device}-{}", self.next_mcu);
        let spec = crate::mcu::McuItemSpec {
            device: device.clone(),
            force_freq: true,
            ..Default::default()
        };
        let view = crate::mcu::instantiate_view(&id, &spec, search).ok()?;
        let item = crate::canvas::Item::mcu_from_view(id.clone(), x, y, device, view);
        self.add_saved_item(item);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::PkgPin;

    #[test]
    fn mcu_defaults_and_props() {
        let mut mcu = Mcu::default();
        assert_eq!(mcu.type_id(), "Mcu");
        assert_eq!(mcu.description(), "Microcontroller.");
        assert_eq!(mcu.get_prop_text("Device").unwrap(), "");
        assert_eq!(mcu.get_prop_text("Frequency").unwrap(), "4 MHz");
        assert_eq!(mcu.get_prop_text("ForceFreq").unwrap(), "true");
        assert_eq!(mcu.get_prop_text("Program").unwrap(), "");
        assert_eq!(mcu.get_prop_text("AutoLoad").unwrap(), "false");
        assert_eq!(mcu.get_prop_text("SavePgm").unwrap(), "false");
        assert_eq!(mcu.get_prop_text("Pgm").unwrap(), "");
        assert_eq!(mcu.get_prop_text("LogicSymbol").unwrap(), "false");
        assert_eq!(mcu.get_prop_text("Package").unwrap(), "");
        assert_eq!(mcu.get_prop_text("SaveEepr").unwrap(), "false");
        assert_eq!(mcu.get_prop_text("RstEnabled").unwrap(), "true");
        assert_eq!(mcu.get_prop_text("ExtOsc").unwrap(), "false");
        assert_eq!(mcu.get_prop_text("WdtEnabled").unwrap(), "false");
        assert_eq!(mcu.get_prop_text("ClkOut").unwrap(), "false");

        mcu.set_prop_text("Device", "p16f84").unwrap();
        assert_eq!(mcu.device, "p16f84");
        assert_eq!(mcu.get_prop_text("Device").unwrap(), "p16f84");

        mcu.set_prop_text("Frequency", "8 MHz").unwrap();
        assert_eq!(mcu.mcu.device.freq, 8e6);
        assert_eq!(mcu.get_prop_text("Frequency").unwrap(), "8 MHz");

        mcu.set_prop_text("ForceFreq", "false").unwrap();
        assert_eq!(mcu.mcu.force_freq, false);

        mcu.set_prop_text("Program", "firmware.hex").unwrap();
        assert_eq!(mcu.mcu.firmware.as_deref(), Some("firmware.hex"));

        mcu.set_prop_text("AutoLoad", "true").unwrap();
        assert_eq!(mcu.mcu.auto_load, true);

        mcu.set_prop_text("SavePgm", "true").unwrap();
        assert_eq!(mcu.mcu.save_pgm, true);

        mcu.set_prop_text("LogicSymbol", "true").unwrap();
        assert_eq!(mcu.logic_symbol, true);

        mcu.set_prop_text("Package", "DIP18").unwrap();
        assert_eq!(mcu.package.name, "DIP18");

        mcu.set_prop_text("SaveEepr", "true").unwrap();
        assert_eq!(mcu.save_eepr, true);

        mcu.set_prop_text("RstEnabled", "false").unwrap();
        assert_eq!(mcu.rst_enabled, false);

        mcu.set_prop_text("ExtOsc", "true").unwrap();
        assert_eq!(mcu.ext_osc, true);

        mcu.set_prop_text("WdtEnabled", "true").unwrap();
        assert_eq!(mcu.wdt_enabled, true);

        mcu.set_prop_text("ClkOut", "true").unwrap();
        assert_eq!(mcu.clk_out, true);

        assert_eq!(mcu.get_prop_text("MainComp").unwrap(), "false");
        mcu.set_prop_text("MainComp", "true").unwrap();
        assert_eq!(mcu.is_main_comp, true);
        assert_eq!(mcu.get_prop_text("MainComp").unwrap(), "true");
    }

    #[test]
    fn mcu_prop_groups_and_main_comp_disabled_state() {
        let mut mcu = Mcu::default();
        // Standalone Mcu (not main_comp)
        let groups = mcu.prop_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Main");
        assert_eq!(groups[1].name, "Config");

        // All rows are enabled when not main_comp
        for g in &groups {
            for r in &g.rows {
                assert!(r.enabled, "Row {} should be enabled", r.name);
            }
        }

        // Set is_main_comp = true (as part of a subcircuit / board)
        mcu.is_main_comp = true;
        let groups_board = mcu.prop_groups();
        assert_eq!(groups_board.len(), 2);

        // In Main tab: Package should be disabled, others enabled
        let main_rows = &groups_board[0].rows;
        let pkg_row = main_rows.iter().find(|r| r.name == "Package").unwrap();
        assert!(!pkg_row.enabled, "Package should be disabled on board MCU");
        let freq_row = main_rows.iter().find(|r| r.name == "Frequency").unwrap();
        assert!(freq_row.enabled, "Frequency should remain enabled");
        let pgm_row = main_rows.iter().find(|r| r.name == "Program").unwrap();
        assert!(pgm_row.enabled, "Program should remain enabled");

        // In Config tab: all properties (RstEnabled, ExtOsc, WdtEnabled, ClkOut) should be disabled
        let config_rows = &groups_board[1].rows;
        for r in config_rows {
            assert!(
                !r.enabled,
                "Config prop {} should be disabled on board MCU",
                r.name
            );
        }
    }

    #[test]
    fn mcu_pin_geoms_and_body() {
        let mut mcu = Mcu::default();
        assert_eq!(mcu.pin_geoms().len(), 0);
        assert!(mcu.body().w > 0.0 && mcu.body().h > 0.0);

        mcu.package
            .pins
            .push(PkgPin::new("RA0", "IO", "", -8, 8, 180));
        assert_eq!(mcu.pin_geoms().len(), 1);
        assert_eq!(mcu.pin_geoms()[0].suffix, "RA0");
        assert_eq!(mcu.pin_geoms()[0].local.x, -8.0);
    }

    #[test]
    fn mcu_to_element_kind() {
        let mcu = Mcu::default();
        match mcu.to_element_kind() {
            Kind::Mcu(m) => {
                assert_eq!(m.device.freq, 4e6);
            }
            other => panic!("expected Kind::Mcu, got {other:?}"),
        }
    }

    #[test]
    fn mcu_load_pgm() {
        let mut mcu = Mcu::default();
        mcu.set_prop_text("Pgm", "10,20,30,").unwrap();
        assert_eq!(mcu.get_prop_text("Pgm").unwrap(), "10,20,30,");
        assert_eq!(mcu.mcu.device.flash_word(0), Some(10));
        assert_eq!(mcu.mcu.device.flash_word(1), Some(20));
        assert_eq!(mcu.mcu.device.flash_word(2), Some(30));
    }
}
