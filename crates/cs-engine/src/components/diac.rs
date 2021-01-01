//! Diac (bidirectional trigger diode) component.

use super::component::stamp_two_terminal;
use super::drawable::{Drawable, paint_diac_body};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.1;
const MAX_V: f64 = 1e4;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e12;
const MIN_I: f64 = 1e-6;
const MAX_I: f64 = 1e3;

impl crate::canvas::Item {
    pub fn diac(id: impl Into<String>, x: f64, y: f64, break_volt: f64) -> Self {
        Self::diac_with(id, x, y, break_volt, 1e8, 0.01, 500.0)
    }

    pub fn diac_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        v_breakover: f64,
        res_off: f64,
        hold_curr: f64,
        res_on: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Diac {
                v_breakover,
                res_off,
                hold_curr,
                res_on,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diac {
    pub v_breakover: f64,
    pub res_off: f64,
    pub hold_curr: f64,
    pub res_on: f64,
}

impl Default for Diac {
    fn default() -> Self {
        Self {
            v_breakover: 30.0,
            res_off: 1e8,
            hold_curr: 0.01,
            res_on: 500.0,
        }
    }
}

impl Diac {
    pub const TYPE_ID: &'static str = "Diac";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Diac {
            v_breakover: self.v_breakover,
            conducting: false,
        }
    }

    fn get_breakover(&self) -> PropValue {
        PropValue::Float(self.v_breakover)
    }
    fn set_breakover(&mut self, v: PropValue) -> Result<(), PropError> {
        self.v_breakover = expect_float("Breakover", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_res_off(&self) -> PropValue {
        PropValue::Float(self.res_off)
    }
    fn set_res_off(&mut self, v: PropValue) -> Result<(), PropError> {
        self.res_off = expect_float("ResOff", v)?.clamp(1.0, MAX_R);
        Ok(())
    }

    fn get_hold_curr(&self) -> PropValue {
        PropValue::Float(self.hold_curr)
    }
    fn set_hold_curr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.hold_curr = expect_float("HoldCurr", v)?.clamp(MIN_I, MAX_I);
        Ok(())
    }

    fn get_res_on(&self) -> PropValue {
        PropValue::Float(self.res_on)
    }
    fn set_res_on(&mut self, v: PropValue) -> Result<(), PropError> {
        self.res_on = expect_float("ResOn", v)?.clamp(MIN_R, 1e6);
        Ok(())
    }
}

impl Component for Diac {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Diac bidirectional trigger diode."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Diac>] = &[
            PropDef::float(
                "Breakover",
                "Breakover Voltage",
                "V",
                MIN_V,
                MAX_V,
                Diac::get_breakover,
                Diac::set_breakover,
            )
            .with_info("Breakover voltage (Vbo) at which conduction begins."),
            PropDef::float(
                "ResOff",
                "Off Resistance",
                "Ω",
                1.0,
                MAX_R,
                Diac::get_res_off,
                Diac::set_res_off,
            )
            .with_info("Resistance when not conducting."),
            PropDef::float(
                "HoldCurr",
                "Holding Current",
                "A",
                MIN_I,
                MAX_I,
                Diac::get_hold_curr,
                Diac::set_hold_curr,
            )
            .with_info("Minimum current to keep conducting."),
            PropDef::float(
                "ResOn",
                "On Resistance",
                "Ω",
                MIN_R,
                1e6,
                Diac::get_res_on,
                Diac::set_res_on,
            )
            .with_info("Resistance when conducting."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        diac_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-8.0, -16.0, 16.0, 32.0)
    }
}

impl TwoTerminal for Diac {}

impl Stampable for Diac {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, 1.0 / self.res_off.max(1.0), 0.0);
    }
}

impl Drawable for Diac {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_diac_body(d, ctx.pal);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_diac(&mut self, x: f64, y: f64) -> String {
        let id = format!("Diac-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::diac(&id, x, y, 30.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_diac() {
        let d = Diac::default();
        assert_eq!(d.type_id(), "Diac");
        assert_eq!(d.v_breakover, 30.0);
        assert_eq!(d.res_off, 1e8);
        assert_eq!(d.hold_curr, 0.01);
        assert_eq!(d.res_on, 500.0);
        assert_eq!(d.pin_geoms().len(), 2);
        assert_eq!(d.body(), Rect::new(-8.0, -16.0, 16.0, 32.0));
    }
}

const DIAC_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-lPin",
        x: -16.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-rPin",
        x: 16.0,
        y: 0.0,
        angle: 0,
        length: 8.0,
        direction: None,
    },
];

fn diac_pins() -> &'static [PinGeom] {
    &DIAC_PINS
}
