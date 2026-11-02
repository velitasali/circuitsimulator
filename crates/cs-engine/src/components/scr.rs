//! Silicon controlled rectifier (thyristor) component.

use super::component::stamp_conductance_between;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_V: f64 = 0.01;
const MAX_V: f64 = 1e3;
const MIN_I: f64 = 1e-6;
const MAX_I: f64 = 1e3;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;
const MIN_DVDT: f64 = 1e3;
const MAX_DVDT: f64 = 1e12;
const MIN_TIME: f64 = 1e-9;
const MAX_TIME: f64 = 1.0;

impl crate::canvas::Item {
    pub fn scr(id: impl Into<String>, x: f64, y: f64, trig_volt: f64, trig_curr: f64) -> Self {
        Self::scr_with(id, x, y, trig_volt, 0.0082, trig_curr, 100.0)
    }

    pub fn scr_with(
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
            Scr {
                v_gate_th,
                i_hold,
                i_trig,
                i_latch: (i_hold * 1.8).max(i_hold),
                r_gate,
                crit_dv_dt: 50.0e6,
                t_q: 30.0e-6,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scr {
    pub v_gate_th: f64,
    pub i_hold: f64,
    pub i_trig: f64,
    pub i_latch: f64,
    pub r_gate: f64,
    pub crit_dv_dt: f64,
    pub t_q: f64,
}

impl Default for Scr {
    fn default() -> Self {
        Self {
            v_gate_th: 0.7,
            i_hold: 0.0082,
            i_trig: 0.01,
            i_latch: 0.015,
            r_gate: 100.0,
            crit_dv_dt: 50.0e6,
            t_q: 30.0e-6,
        }
    }
}

impl Scr {
    pub const TYPE_ID: &'static str = "Scr";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Scr {
            state: crate::elements::ScrState {
                v_gate_th: self.v_gate_th,
                i_hold: self.i_hold,
                i_trig: self.i_trig,
                i_latch: self.i_latch,
                r_gate: self.r_gate,
                crit_dv_dt: self.crit_dv_dt,
                t_q: self.t_q,
                ..Default::default()
            },
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

    fn get_latch_curr(&self) -> PropValue {
        PropValue::Float(self.i_latch)
    }
    fn set_latch_curr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_latch = expect_float("LatchCurr", v)?.clamp(MIN_I, MAX_I);
        Ok(())
    }

    fn get_gate_res(&self) -> PropValue {
        PropValue::Float(self.r_gate)
    }
    fn set_gate_res(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_gate = expect_float("GateRes", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_crit_dv_dt(&self) -> PropValue {
        PropValue::Float(self.crit_dv_dt)
    }
    fn set_crit_dv_dt(&mut self, v: PropValue) -> Result<(), PropError> {
        self.crit_dv_dt = expect_float("CritDvDt", v)?.clamp(MIN_DVDT, MAX_DVDT);
        Ok(())
    }

    fn get_t_q(&self) -> PropValue {
        PropValue::Float(self.t_q)
    }
    fn set_t_q(&mut self, v: PropValue) -> Result<(), PropError> {
        self.t_q = expect_float("TurnOffTime", v)?.clamp(MIN_TIME, MAX_TIME);
        Ok(())
    }
}

impl Component for Scr {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Silicon controlled rectifier (thyristor)."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Scr>] = &[
            PropDef::float(
                "GateTh",
                "Gate Threshold",
                "V",
                MIN_V,
                MAX_V,
                Scr::get_gate_th,
                Scr::set_gate_th,
            )
            .with_info("Gate trigger voltage threshold required to latch conduction."),
            PropDef::float(
                "HoldCurr",
                "Holding Current",
                "A",
                MIN_I,
                MAX_I,
                Scr::get_hold_curr,
                Scr::set_hold_curr,
            )
            .with_info("Minimum current to keep conducting."),
            PropDef::float(
                "TrigCurr",
                "Trigger Current",
                "A",
                MIN_I,
                MAX_I,
                Scr::get_trig_curr,
                Scr::set_trig_curr,
            )
            .with_info("Minimum Gate current needed to trigger conduction."),
            PropDef::float(
                "LatchCurr",
                "Latching Current",
                "A",
                MIN_I,
                MAX_I,
                Scr::get_latch_curr,
                Scr::set_latch_curr,
            )
            .with_info("Minimum anode current to keep conducting after gate trigger drops."),
            PropDef::float(
                "GateRes",
                "Gate Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Scr::get_gate_res,
                Scr::set_gate_res,
            )
            .with_info("Gate to Cathode resistance."),
            PropDef::float(
                "CritDvDt",
                "Critical dV/dt",
                "V/s",
                MIN_DVDT,
                MAX_DVDT,
                Scr::get_crit_dv_dt,
                Scr::set_crit_dv_dt,
            )
            .with_info("Critical rate of off-state voltage rise triggering conduction."),
            PropDef::float(
                "TurnOffTime",
                "Turn-Off Time (tq)",
                "s",
                MIN_TIME,
                MAX_TIME,
                Scr::get_t_q,
                Scr::set_t_q,
            )
            .with_info("Circuit-commutated recovery time required before forward voltage can be reapplied."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        scr_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-10.0, -8.0, 20.0, 16.0)
    }
}

impl Stampable for Scr {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin 0: anode, pin 1: cathode, pin 2: gate
        stamp_conductance_between(matrix, pin_nodes, 2, 1, 1.0 / self.r_gate.max(MIN_R));
        stamp_conductance_between(matrix, pin_nodes, 0, 1, 1e-9);
    }
}

impl Drawable for Scr {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let tri = [[-8.0, -7.0], [7.0, 0.0], [-8.0, 7.0]];
        d.fill_poly(&tri, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&tri, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        d.line(7.0, -7.0, 7.0, 7.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_scr(&mut self, x: f64, y: f64) -> String {
        let id = format!("SCR-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Scr::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scr() {
        let s = Scr::default();
        assert_eq!(s.type_id(), "Scr");
        assert_eq!(s.v_gate_th, 0.7);
        assert_eq!(s.i_hold, 0.0082);
        assert_eq!(s.i_trig, 0.01);
        assert_eq!(s.i_latch, 0.015);
        assert_eq!(s.r_gate, 100.0);
        assert_eq!(s.crit_dv_dt, 50.0e6);
        assert_eq!(s.t_q, 30.0e-6);
        assert_eq!(s.pin_geoms().len(), 3);
        assert_eq!(s.body(), Rect::new(-10.0, -8.0, 20.0, 16.0));
    }
}

const SCR_PINS: [PinGeom; 3] = [
    PinGeom {
        suffix: "-aPin",
        x: -16.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-kPin",
        x: 16.0,
        y: 0.0,
        angle: 0,
        length: 9.0,
        direction: None,
    },
    PinGeom {
        suffix: "-gPin",
        x: 16.0,
        y: 8.0,
        angle: -40,
        length: 12.0,
        direction: None,
    },
];

fn scr_pins() -> &'static [PinGeom] {
    &SCR_PINS
}
