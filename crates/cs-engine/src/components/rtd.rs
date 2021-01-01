//! Resistance temperature detector (RTD, e.g. PT100) component.

use super::component::{resistor_g, stamp_two_terminal, two_terminal_pins};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::resistor::paint_resistor_body;
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_TEMP: f64 = -273.15;
const MAX_TEMP: f64 = 1000.0;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;

impl crate::canvas::Item {
    pub fn rtd(id: impl Into<String>, x: f64, y: f64, resistance: f64, temp_c: f64) -> Self {
        Self::rtd_with(id, x, y, temp_c, resistance, 0.00385, 5.0)
    }

    pub fn rtd_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        temp_c: f64,
        r0: f64,
        alpha: f64,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Rtd {
                temp_c,
                r0,
                alpha,
                dial_step,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rtd {
    pub temp_c: f64,
    pub r0: f64,
    pub alpha: f64,
    pub dial_step: f64,
}

impl Default for Rtd {
    fn default() -> Self {
        Self {
            temp_c: 25.0,
            r0: 100.0,
            alpha: 0.00385,
            dial_step: 1.0,
        }
    }
}

impl Rtd {
    pub const TYPE_ID: &'static str = "Rtd";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Rtd {
            resistance: self.r0,
            temp_c: self.temp_c,
        }
    }

    fn get_temp(&self) -> PropValue {
        PropValue::Float(self.temp_c)
    }
    fn set_temp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.temp_c = expect_float("Temp", v)?.clamp(MIN_TEMP, MAX_TEMP);
        Ok(())
    }

    fn get_r0(&self) -> PropValue {
        PropValue::Float(self.r0)
    }
    fn set_r0(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r0 = expect_float("R0", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_alpha(&self) -> PropValue {
        PropValue::Float(self.alpha)
    }
    fn set_alpha(&mut self, v: PropValue) -> Result<(), PropError> {
        self.alpha = expect_float("Alpha", v)?.clamp(0.0, 1.0);
        Ok(())
    }

    fn get_dial_step(&self) -> PropValue {
        PropValue::Float(self.dial_step)
    }
    fn set_dial_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial_step = expect_float("DialStep", v)?.clamp(0.01, 100.0);
        Ok(())
    }
}

impl Component for Rtd {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Resistance temperature detector (RTD)."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Rtd>] = &[
            PropDef::float(
                "Temp",
                "Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                Rtd::get_temp,
                Rtd::set_temp,
            )
            .with_info("Current temperature."),
            PropDef::float("R0", "R0", "Ω", MIN_R, MAX_R, Rtd::get_r0, Rtd::set_r0)
                .with_info("Resistance at 0 ºC"),
            PropDef::float(
                "Alpha",
                "Alpha",
                "",
                0.0,
                1.0,
                Rtd::get_alpha,
                Rtd::set_alpha,
            )
            .with_info("Temperature coefficient of resistance (alpha) per °C."),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "°C",
                0.01,
                100.0,
                Rtd::get_dial_step,
                Rtd::set_dial_step,
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

impl TwoTerminal for Rtd {}

impl Stampable for Rtd {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.r0), 0.0);
    }
}

impl Drawable for Rtd {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, 1000.0, false);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_rtd(&mut self, x: f64, y: f64) -> String {
        let id = format!("RTD-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::rtd(&id, x, y, 100.0, 25.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rtd() {
        let r = Rtd::default();
        assert_eq!(r.type_id(), "Rtd");
        assert_eq!(r.temp_c, 25.0);
        assert_eq!(r.r0, 100.0);
        assert_eq!(r.alpha, 0.00385);
        assert_eq!(r.dial_step, 1.0);
        assert_eq!(r.pin_geoms().len(), 2);
        assert_eq!(r.body(), Rect::new(-11.0, -4.5, 22.0, 9.0));
    }
}
