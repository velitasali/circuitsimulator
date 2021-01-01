//! Triac (bidirectional thyristor) component.

use super::component::stamp_conductance_between;
use super::drawable::{Drawable, paint_diac_body};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.01;
const MAX_V: f64 = 1e3;
const MIN_I: f64 = 1e-6;
const MAX_I: f64 = 1e3;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;

impl crate::canvas::Item {
    pub fn triac(id: impl Into<String>, x: f64, y: f64, trig_volt: f64, trig_curr: f64) -> Self {
        Self::triac_with(id, x, y, trig_volt, trig_curr, 0.01, 100.0)
    }

    pub fn triac_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        v_gate_th: f64,
        i_hold: f64,
        i_trig: f64,
        r_gate: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Triac {
                v_gate_th,
                i_hold,
                i_trig,
                r_gate,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Triac {
    pub v_gate_th: f64,
    pub i_hold: f64,
    pub i_trig: f64,
    pub r_gate: f64,
}

impl Default for Triac {
    fn default() -> Self {
        Self {
            v_gate_th: 0.7,
            i_hold: 0.0082,
            i_trig: 0.01,
            r_gate: 100.0,
        }
    }
}

impl Triac {
    pub const TYPE_ID: &'static str = "Triac";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Triac {
            v_gate_th: self.v_gate_th,
            i_hold: self.i_hold,
            conducting: false,
        }
    }

    fn get_gate_th(&self) -> PropValue {
        PropValue::Float(self.v_gate_th)
    }
    fn set_gate_th(&mut self, v: PropValue) -> Result<(), PropError> {
        self.v_gate_th = expect_float("GateTh", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_hold_curr(&self) -> PropValue {
        PropValue::Float(self.i_hold)
    }
    fn set_hold_curr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_hold = expect_float("HoldCurr", v)?.clamp(MIN_I, MAX_I);
        Ok(())
    }

    fn get_trig_curr(&self) -> PropValue {
        PropValue::Float(self.i_trig)
    }
    fn set_trig_curr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_trig = expect_float("TrigCurr", v)?.clamp(MIN_I, MAX_I);
        Ok(())
    }

    fn get_gate_res(&self) -> PropValue {
        PropValue::Float(self.r_gate)
    }
    fn set_gate_res(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_gate = expect_float("GateRes", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }
}

impl Component for Triac {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Triac bidirectional thyristor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Triac>] = &[
            PropDef::float(
                "GateTh",
                "Gate Threshold",
                "V",
                MIN_V,
                MAX_V,
                Triac::get_gate_th,
                Triac::set_gate_th,
            )
            .with_info("Gate trigger voltage threshold required to latch conduction."),
            PropDef::float(
                "HoldCurr",
                "Holding Current",
                "A",
                MIN_I,
                MAX_I,
                Triac::get_hold_curr,
                Triac::set_hold_curr,
            )
            .with_info("Minimum current to keep conducting."),
            PropDef::float(
                "TrigCurr",
                "Trigger Current",
                "A",
                MIN_I,
                MAX_I,
                Triac::get_trig_curr,
                Triac::set_trig_curr,
            )
            .with_info("Minimum Gate current needed to trigger conduction."),
            PropDef::float(
                "GateRes",
                "Gate Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Triac::get_gate_res,
                Triac::set_gate_res,
            )
            .with_info("Gate to MT1 resistance."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        triac_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-8.0, -16.0, 16.0, 32.0)
    }
}

impl Stampable for Triac {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin 0: MT1, pin 1: MT2, pin 2: Gate
        stamp_conductance_between(matrix, pin_nodes, 2, 0, 1.0 / self.r_gate.max(MIN_R));
        stamp_conductance_between(matrix, pin_nodes, 0, 1, 1e-9);
    }
}

impl Drawable for Triac {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_diac_body(d, ctx.pal);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_triac(&mut self, x: f64, y: f64) -> String {
        let id = format!("Triac-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::triac(&id, x, y, 0.7, 0.0082));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_triac() {
        let t = Triac::default();
        assert_eq!(t.type_id(), "Triac");
        assert_eq!(t.v_gate_th, 0.7);
        assert_eq!(t.i_hold, 0.0082);
        assert_eq!(t.i_trig, 0.01);
        assert_eq!(t.r_gate, 100.0);
        assert_eq!(t.pin_geoms().len(), 3);
        assert_eq!(t.body(), Rect::new(-8.0, -16.0, 16.0, 32.0));
    }
}

const TRIAC_PINS: [PinGeom; 3] = [
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
    PinGeom {
        suffix: "-gPin",
        x: 16.0,
        y: 12.0,
        angle: -26,
        length: 8.9,
        direction: None,
    },
];

fn triac_pins() -> &'static [PinGeom] {
    &TRIAC_PINS
}
