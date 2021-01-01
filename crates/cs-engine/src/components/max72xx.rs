//! MAX7219/MAX7221 8x8 LED matrix display module.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float, expect_int, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_MODULES: i64 = 1;
const MAX_MODULES: i64 = 16;

use super::led::{LED_COLOR_OPTIONS, LedColor};

const MIN_V: f64 = 0.0;
const MAX_V: f64 = 100.0;
const MIN_A: f64 = 1e-6;
const MAX_A: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;

impl crate::canvas::Item {
    pub fn max72xx(id: impl Into<String>, x: f64, y: f64, modules: usize) -> Self {
        Self::new(
            id,
            x,
            y,
            Max72xx {
                modules,
                color: LedColor::Red,
                threshold: 1.8,
                max_current: 0.03,
                resistance: 0.6,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Max72xx {
    pub modules: usize,
    pub color: LedColor,
    pub threshold: f64,
    pub max_current: f64,
    pub resistance: f64,
}

impl Default for Max72xx {
    fn default() -> Self {
        Self {
            modules: 1,
            color: LedColor::Red,
            threshold: 1.8,
            max_current: 0.03,
            resistance: 0.6,
        }
    }
}

impl Max72xx {
    pub const TYPE_ID: &'static str = "Max72xx";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Max72xx {
            modules: self.modules,
            color: self.color.as_str().to_string(),
        }
    }

    fn get_modules(&self) -> PropValue {
        PropValue::Int(self.modules as i64)
    }
    fn set_modules(&mut self, v: PropValue) -> Result<(), PropError> {
        self.modules = expect_int("Modules", v)?.clamp(MIN_MODULES, MAX_MODULES) as usize;
        Ok(())
    }

    fn get_color(&self) -> PropValue {
        PropValue::Enum(self.color.as_str().to_string())
    }
    fn set_color(&mut self, v: PropValue) -> Result<(), PropError> {
        let color_name = expect_string("Color", v)?;
        self.color = LedColor::from_str_name(&color_name);
        self.threshold = self.color.threshold();
        Ok(())
    }

    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold)
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        self.threshold = expect_float("Threshold", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_max_current(&self) -> PropValue {
        PropValue::Float(self.max_current)
    }
    fn set_max_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_current = expect_float("MaxCurrent", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
}

impl Component for Max72xx {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "MAX7219/MAX7221 8x8 LED matrix display module."
    }

    fn props() -> &'static [PropDef<Self>] {
        const MODS: PropDef<Max72xx> = {
            let mut p = PropDef::int(
                "Modules",
                "Modules",
                MIN_MODULES,
                MAX_MODULES,
                Max72xx::get_modules,
                Max72xx::set_modules,
            )
            .with_info("Number of daisy-chained LED matrix modules.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Max72xx>] = &[
            PropDef::enumeration("Color", "Color", LED_COLOR_OPTIONS, Max72xx::get_color, Max72xx::set_color)
                .with_info("Display color."),
            MODS,
            PropDef::float(
                "Threshold",
                "Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                Max72xx::get_threshold,
                Max72xx::set_threshold,
            ).with_info("Voltage drop when forward biased."),
            PropDef::float(
                "MaxCurrent",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                Max72xx::get_max_current,
                Max72xx::set_max_current,
            ).with_info("Maximum current (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Max72xx::get_resistance,
                Max72xx::set_resistance,
            ).with_info("Series resistance."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Color", "Modules"]),
                ("Electric", &["Threshold", "MaxCurrent", "Resistance"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        max72xx_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(
            -36.0,
            -44.0,
            8.0 + 64.0 * (self.modules as f64).max(1.0),
            88.0,
        )
    }
}

impl Stampable for Max72xx {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Max72xx {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let disp = self.modules.max(1);
        let w = (4 + 64 * disp + 4) as f64;
        d.fill_round_rect(-36.0, -44.0, w, 88.0, 3.0, Color::rgb(15, 15, 15));
        d.stroke_round_rect(
            -36.0,
            -44.0,
            w,
            88.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        for di in 0..disp {
            let x0 = -32.0 + (di as f64) * 64.0;
            d.stroke_rect(x0, -40.0, 60.0, 80.0, ctx.pal.border.fade(0.4), 1.0);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_max72xx(&mut self, x: f64, y: f64) -> String {
        let id = format!("Max72xx-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::max72xx(&id, x, y, 1));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_max72xx() {
        let m = Max72xx::default();
        assert_eq!(m.type_id(), "Max72xx");
        assert_eq!(m.modules, 1);
        assert_eq!(m.color, LedColor::Red);
        assert_eq!(m.pin_geoms().len(), 5);
        assert_eq!(m.body(), Rect::new(-36.0, -44.0, 72.0, 88.0));
    }
}

const MAX72XX_PINS: [PinGeom; 5] = [
    PinGeom {
        suffix: "-din",
        x: -44.0,
        y: -16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-cs",
        x: -44.0,
        y: -8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-clk",
        x: -44.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vcc",
        x: -44.0,
        y: 8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-gnd",
        x: -44.0,
        y: 16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn max72xx_pins() -> &'static [PinGeom] {
    &MAX72XX_PINS
}
