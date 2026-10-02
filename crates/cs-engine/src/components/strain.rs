//! Strain gauge sensor component.

use super::component::{resistor_g, stamp_two_terminal, two_terminal_pins};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::resistor::paint_resistor_body;
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_STRAIN: f64 = -1.0;
const MAX_STRAIN: f64 = 1.0;
const MIN_GF: f64 = -100.0;
const MAX_GF: f64 = 100.0;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;
const MIN_TEMP: f64 = -273.15;
const MAX_TEMP: f64 = 1000.0;

impl crate::canvas::Item {
    pub fn strain(id: impl Into<String>, x: f64, y: f64, resistance: f64, strain: f64) -> Self {
        let mut s = Strain::default();
        s.strain = strain;
        s.r0 = resistance;
        Self::new(id, x, y, s)
    }

    pub fn strain_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        strain: f64,
        gauge_factor: f64,
        r0: f64,
        temp_c: f64,
        ref_temp: f64,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Strain {
                strain,
                gauge_factor,
                r0,
                temp_c,
                ref_temp,
                dial_step,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Strain {
    pub strain: f64,
    pub gauge_factor: f64,
    pub r0: f64,
    pub temp_c: f64,
    pub ref_temp: f64,
    pub dial_step: f64,
}

impl Default for Strain {
    fn default() -> Self {
        Self {
            strain: 0.0,
            gauge_factor: 2.0,
            r0: 350.0,
            temp_c: 20.0,
            ref_temp: 20.0,
            dial_step: 0.0,
        }
    }
}

impl Strain {
    pub const TYPE_ID: &'static str = "Strain";

    /// Gauge equation from the stored gauge factor. R = R0 * (1 + GF * ε).
    pub fn resistance_of(r0: f64, gauge_factor: f64, strain: f64) -> f64 {
        let res = r0 * (1.0 + gauge_factor * strain);
        if res.is_finite() && res > 0.0 {
            res.max(1e-12)
        } else {
            r0.max(1e-12)
        }
    }

    pub fn resistance(&self) -> f64 {
        Self::resistance_of(self.r0, self.gauge_factor, self.strain)
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Strain {
            resistance: self.resistance(),
            strain: self.strain,
        }
    }

    fn get_strain(&self) -> PropValue {
        PropValue::Float(self.strain)
    }
    fn set_strain(&mut self, v: PropValue) -> Result<(), PropError> {
        self.strain = expect_float("Strain", v)?.clamp(MIN_STRAIN, MAX_STRAIN);
        Ok(())
    }

    fn get_gauge_factor(&self) -> PropValue {
        PropValue::Float(self.gauge_factor)
    }
    fn set_gauge_factor(&mut self, v: PropValue) -> Result<(), PropError> {
        self.gauge_factor = expect_float("GaugeFactor", v)?.clamp(MIN_GF, MAX_GF);
        Ok(())
    }

    fn get_r0(&self) -> PropValue {
        PropValue::Float(self.r0)
    }
    fn set_r0(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r0 = expect_float("R0", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_temp(&self) -> PropValue {
        PropValue::Float(self.temp_c)
    }
    fn set_temp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.temp_c = expect_float("Temp", v)?.clamp(MIN_TEMP, MAX_TEMP);
        Ok(())
    }

    fn get_ref_temp(&self) -> PropValue {
        PropValue::Float(self.ref_temp)
    }
    fn set_ref_temp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.ref_temp = expect_float("RefTemp", v)?.clamp(MIN_TEMP, MAX_TEMP);
        Ok(())
    }

    fn get_dial_step(&self) -> PropValue {
        PropValue::Float(self.dial_step)
    }
    fn set_dial_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial_step = expect_float("DialStep", v)?.clamp(0.0, 1.0);
        Ok(())
    }
}

impl Component for Strain {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Strain gauge sensor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Strain>] = &[
            PropDef::float(
                "Strain",
                "Strain / Force",
                "ε",
                MIN_STRAIN,
                MAX_STRAIN,
                Strain::get_strain,
                Strain::set_strain,
            )
            .with_info("Mechanical strain or applied force value."),
            PropDef::float(
                "GaugeFactor",
                "Gauge Factor",
                "",
                MIN_GF,
                MAX_GF,
                Strain::get_gauge_factor,
                Strain::set_gauge_factor,
            )
            .with_info("Gauge factor sensitivity ratio of the strain gauge."),
            PropDef::float(
                "R0",
                "Nominal Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Strain::get_r0,
                Strain::set_r0,
            )
            .with_info("Resistance at 0 ºC"),
            PropDef::float(
                "Temp",
                "Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                Strain::get_temp,
                Strain::set_temp,
            )
            .with_info("Current Temperature."),
            PropDef::float(
                "RefTemp",
                "Ref Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                Strain::get_ref_temp,
                Strain::set_ref_temp,
            )
            .with_info("Reference Temperature."),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "ε",
                0.0,
                1.0,
                Strain::get_dial_step,
                Strain::set_dial_step,
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

impl TwoTerminal for Strain {}

impl Stampable for Strain {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.resistance()), 0.0);
    }
}

impl Drawable for Strain {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, 1000.0, false);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_strain(&mut self, x: f64, y: f64) -> String {
        let id = format!("Strain-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Strain::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strain() {
        let s = Strain::default();
        assert_eq!(s.type_id(), "Strain");
        assert_eq!(s.strain, 0.0);
        assert_eq!(s.gauge_factor, 2.0);
        assert_eq!(s.r0, 350.0);
        assert_eq!(s.temp_c, 20.0);
        assert_eq!(s.ref_temp, 20.0);
        assert_eq!(s.dial_step, 0.0);
        assert_eq!(s.pin_geoms().len(), 2);
        assert_eq!(s.body(), Rect::new(-11.0, -4.5, 22.0, 9.0));
        assert!((s.resistance() - 350.0).abs() < 1e-9);
        assert!((Strain::resistance_of(350.0, 2.0, 0.001) - 350.7).abs() < 1e-9);
    }
}
