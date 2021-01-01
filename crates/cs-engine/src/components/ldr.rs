//! Light-dependent resistor (LDR) component.

use super::component::{resistor_g, stamp_two_terminal, two_terminal_pins};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::resistor::paint_resistor_body;
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_LUX: f64 = 0.1;
const MAX_LUX: f64 = 1e8;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e12;

impl crate::canvas::Item {
    pub fn ldr(id: impl Into<String>, x: f64, y: f64, resistance: f64, lux: f64) -> Self {
        Self::ldr_with(id, x, y, lux, 1e6, resistance, 10.0)
    }

    pub fn ldr_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        lux: f64,
        r_dark: f64,
        r_light: f64,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Ldr {
                lux,
                r_dark,
                r_light,
                dial_step,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ldr {
    pub lux: f64,
    pub r_dark: f64,
    pub r_light: f64,
    pub dial_step: f64,
}

impl Default for Ldr {
    fn default() -> Self {
        Self {
            lux: 100.0,
            r_dark: 1e6,
            r_light: 1_000.0,
            dial_step: 1.0,
        }
    }
}

impl Ldr {
    pub const TYPE_ID: &'static str = "Ldr";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ldr {
            resistance: self.r_light,
            lux: self.lux,
        }
    }

    fn get_lux(&self) -> PropValue {
        PropValue::Float(self.lux)
    }
    fn set_lux(&mut self, v: PropValue) -> Result<(), PropError> {
        self.lux = expect_float("Lux", v)?.clamp(MIN_LUX, MAX_LUX);
        Ok(())
    }

    fn get_r_dark(&self) -> PropValue {
        PropValue::Float(self.r_dark)
    }
    fn set_r_dark(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_dark = expect_float("RDark", v)?.clamp(1.0, MAX_R);
        Ok(())
    }

    fn get_r_light(&self) -> PropValue {
        PropValue::Float(self.r_light)
    }
    fn set_r_light(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_light = expect_float("RLight", v)?.clamp(MIN_R, 1e9);
        Ok(())
    }

    fn get_dial_step(&self) -> PropValue {
        PropValue::Float(self.dial_step)
    }
    fn set_dial_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial_step = expect_float("DialStep", v)?.clamp(MIN_LUX, 1e6);
        Ok(())
    }
}

impl Component for Ldr {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Light-dependent resistor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Ldr>] = &[
            PropDef::float(
                "Lux",
                "Light Level",
                "lux",
                MIN_LUX,
                MAX_LUX,
                Ldr::get_lux,
                Ldr::set_lux,
            )
            .with_info("Value determined by dial position."),
            PropDef::float(
                "RDark",
                "Dark Resistance",
                "Ω",
                1.0,
                MAX_R,
                Ldr::get_r_dark,
                Ldr::set_r_dark,
            )
            .with_info("Dark resistance in Ohms when unilluminated."),
            PropDef::float(
                "RLight",
                "Light Resistance",
                "Ω",
                MIN_R,
                1e9,
                Ldr::get_r_light,
                Ldr::set_r_light,
            )
            .with_info("Light resistance in Ohms at reference illumination (1000 Lux)."),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "lux",
                MIN_LUX,
                1e6,
                Ldr::get_dial_step,
                Ldr::set_dial_step,
            )
            .with_info("Minimum step when rotating the dial.\n0 to use default."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(5.0)
    }

    fn body(&self) -> Rect {
        Rect::new(-11.0, -4.5, 22.0, 9.0)
    }
}

impl TwoTerminal for Ldr {}

impl Stampable for Ldr {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.r_light), 0.0);
    }
}

impl Drawable for Ldr {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, 1000.0, false);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ldr(&mut self, x: f64, y: f64) -> String {
        let id = format!("LDR-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::ldr(&id, x, y, 1000.0, 100.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ldr() {
        let l = Ldr::default();
        assert_eq!(l.type_id(), "Ldr");
        assert_eq!(l.lux, 100.0);
        assert_eq!(l.r_dark, 1e6);
        assert_eq!(l.r_light, 1000.0);
        assert_eq!(l.dial_step, 1.0);
        assert_eq!(l.pin_geoms().len(), 2);
        assert_eq!(l.body(), Rect::new(-11.0, -4.5, 22.0, 9.0));
    }
}
