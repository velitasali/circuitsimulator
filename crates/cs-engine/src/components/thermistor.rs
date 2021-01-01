//! NTC thermistor (temperature-dependent resistor) component.

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
const MIN_BETA: f64 = 0.0;
const MAX_BETA: f64 = 1e6;

impl crate::canvas::Item {
    pub fn thermistor(id: impl Into<String>, x: f64, y: f64, resistance: f64, temp_c: f64) -> Self {
        Self::thermistor_with(id, x, y, temp_c, resistance, 3950.0, 25.0, 5.0)
    }

    pub fn thermistor_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        temp_c: f64,
        r0: f64,
        beta: f64,
        t0_c: f64,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Thermistor {
                temp_c,
                r0,
                beta,
                t0_c,
                dial_step,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Thermistor {
    pub temp_c: f64,
    pub r0: f64,
    pub beta: f64,
    pub t0_c: f64,
    pub dial_step: f64,
}

impl Default for Thermistor {
    fn default() -> Self {
        Self {
            temp_c: 25.0,
            r0: 10_000.0,
            beta: 3950.0,
            t0_c: 25.0,
            dial_step: 1.0,
        }
    }
}

impl Thermistor {
    pub const TYPE_ID: &'static str = "Thermistor";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Thermistor {
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

    fn get_beta(&self) -> PropValue {
        PropValue::Float(self.beta)
    }
    fn set_beta(&mut self, v: PropValue) -> Result<(), PropError> {
        self.beta = expect_float("Beta", v)?.clamp(MIN_BETA, MAX_BETA);
        Ok(())
    }

    fn get_t0(&self) -> PropValue {
        PropValue::Float(self.t0_c)
    }
    fn set_t0(&mut self, v: PropValue) -> Result<(), PropError> {
        self.t0_c = expect_float("T0", v)?.clamp(MIN_TEMP, MAX_TEMP);
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

impl Component for Thermistor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "NTC thermistor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Thermistor>] = &[
            PropDef::float(
                "Temp",
                "Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                Thermistor::get_temp,
                Thermistor::set_temp,
            )
            .with_info("Current temperature."),
            PropDef::float(
                "R0",
                "R25",
                "Ω",
                MIN_R,
                MAX_R,
                Thermistor::get_r0,
                Thermistor::set_r0,
            )
            .with_info("Resistance at 0 ºC"),
            PropDef::float(
                "Beta",
                "Beta (B)",
                "K",
                MIN_BETA,
                MAX_BETA,
                Thermistor::get_beta,
                Thermistor::set_beta,
            )
            .with_info("β (Beta) temperature coefficient parameter in Kelvin."),
            PropDef::float(
                "T0",
                "Reference Temp",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                Thermistor::get_t0,
                Thermistor::set_t0,
            )
            .with_info("Reference temperature T0 in Celsius for nominal resistance."),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "°C",
                0.01,
                100.0,
                Thermistor::get_dial_step,
                Thermistor::set_dial_step,
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

impl TwoTerminal for Thermistor {}

impl Stampable for Thermistor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.r0), 0.0);
    }
}

impl Drawable for Thermistor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, 1000.0, false);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_thermistor(&mut self, x: f64, y: f64) -> String {
        let id = format!("Thermistor-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::thermistor(&id, x, y, 10000.0, 25.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_thermistor() {
        let t = Thermistor::default();
        assert_eq!(t.type_id(), "Thermistor");
        assert_eq!(t.temp_c, 25.0);
        assert_eq!(t.r0, 10_000.0);
        assert_eq!(t.beta, 3950.0);
        assert_eq!(t.t0_c, 25.0);
        assert_eq!(t.dial_step, 1.0);
        assert_eq!(t.pin_geoms().len(), 2);
        assert_eq!(t.body(), Rect::new(-11.0, -4.5, 22.0, 9.0));
    }
}
