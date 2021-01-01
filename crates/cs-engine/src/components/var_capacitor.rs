//! Variable capacitor: companion stamp plus a dial on Capacitance.

use super::component::{DialState, stamp_two_terminal, two_terminal_pins};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_C: f64 = 1e-15;
const MAX_C: f64 = 1e3;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_C: f64 = 100e-12;
const DEFAULT_ESR: f64 = 1e-3;

impl crate::canvas::Item {
    pub fn var_capacitor(id: impl Into<String>, x: f64, y: f64, capacitance: f64) -> Self {
        Self::var_capacitor_with(
            id,
            x,
            y,
            capacitance,
            0.0,
            capacitance * 2.0,
            1e-3,
            0.0,
            0.0,
        )
    }

    pub fn var_capacitor_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        capacitance: f64,
        min_c: f64,
        max_c: f64,
        resistance: f64,
        init_volt: f64,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            VarCapacitor {
                capacitance,
                min_c,
                max_c,
                resistance,
                init_volt,
                dial: DialState {
                    key: String::new(),
                    step: dial_step,
                },
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarCapacitor {
    pub capacitance: f64,
    pub min_c: f64,
    pub max_c: f64,
    pub resistance: f64,
    pub init_volt: f64,
    pub dial: DialState,
}

impl Default for VarCapacitor {
    fn default() -> Self {
        Self {
            capacitance: DEFAULT_C,
            min_c: 0.0,
            max_c: DEFAULT_C * 2.0,
            resistance: DEFAULT_ESR,
            init_volt: 0.0,
            dial: DialState::new(1e-12),
        }
    }
}

impl VarCapacitor {
    pub const TYPE_ID: &'static str = "VarCapacitor";
    pub fn wiper(&self) -> f64 {
        let span = (self.max_c - self.min_c).max(1e-15);
        ((self.capacitance - self.min_c) / span).clamp(0.0, 1.0)
    }

    pub fn new(capacitance: f64) -> Self {
        let c = capacitance.max(MIN_C);
        Self {
            capacitance: c,
            min_c: 0.0,
            max_c: c * 2.0,
            resistance: DEFAULT_ESR,
            init_volt: 0.0,
            dial: DialState::new(1e-12),
        }
    }

    fn clamp_c(&mut self) {
        let lo = self.min_c.min(self.max_c).max(MIN_C);
        let hi = self.min_c.max(self.max_c).max(lo);
        self.capacitance = self.capacitance.clamp(lo, hi);
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::VarCapacitor {
            capacitance: self.capacitance.max(MIN_C),
            volt: self.init_volt,
        }
    }

    fn get_capacitance(&self) -> PropValue {
        PropValue::Float(self.capacitance)
    }
    fn set_capacitance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.capacitance = expect_float("Capacitance", v)?.clamp(MIN_C, MAX_C);
        self.clamp_c();
        Ok(())
    }
    fn get_min_c(&self) -> PropValue {
        PropValue::Float(self.min_c)
    }
    fn set_min_c(&mut self, v: PropValue) -> Result<(), PropError> {
        self.min_c = expect_float("MinCapacitance", v)?.clamp(0.0, MAX_C);
        self.clamp_c();
        Ok(())
    }
    fn get_max_c(&self) -> PropValue {
        PropValue::Float(self.max_c)
    }
    fn set_max_c(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_c = expect_float("MaxCapacitance", v)?.clamp(MIN_C, MAX_C);
        self.clamp_c();
        Ok(())
    }
    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn get_init_volt(&self) -> PropValue {
        PropValue::Float(self.init_volt)
    }
    fn set_init_volt(&mut self, v: PropValue) -> Result<(), PropError> {
        self.init_volt = expect_float("InitVolt", v)?;
        Ok(())
    }
    fn get_dial_step(&self) -> PropValue {
        PropValue::Float(self.dial.step)
    }
    fn set_dial_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial.step = expect_float("DialStep", v)?.clamp(MIN_C, MAX_C);
        Ok(())
    }
}

impl Component for VarCapacitor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Variable capacitor."
    }
    fn props() -> &'static [PropDef<Self>] {
        const CAP: PropDef<VarCapacitor> = {
            let mut p = PropDef::float(
                "Capacitance",
                "Capacitance",
                "F",
                MIN_C,
                MAX_C,
                VarCapacitor::get_capacitance,
                VarCapacitor::set_capacitance,
            )
            .with_info("Value determined by dial position.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<VarCapacitor>] = &[
            CAP,
            PropDef::float(
                "MinCapacitance",
                "Min Capacitance",
                "F",
                0.0,
                MAX_C,
                VarCapacitor::get_min_c,
                VarCapacitor::set_min_c,
            )
            .with_info("Capacitance with dial at the left end."),
            PropDef::float(
                "MaxCapacitance",
                "Max Capacitance",
                "F",
                MIN_C,
                MAX_C,
                VarCapacitor::get_max_c,
                VarCapacitor::set_max_c,
            )
            .with_info("Capacitance with dial at the right end."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                VarCapacitor::get_resistance,
                VarCapacitor::set_resistance,
            )
            .with_info("Series resistance."),
            PropDef::float(
                "InitVolt",
                "Initial Voltage",
                "V",
                -1e6,
                1e6,
                VarCapacitor::get_init_volt,
                VarCapacitor::set_init_volt,
            )
            .with_info("Voltage at simulation start (initial charge)."),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "F",
                MIN_C,
                MAX_C,
                VarCapacitor::get_dial_step,
                VarCapacitor::set_dial_step,
            )
            .with_info("Minimum step when rotating the dial.\n0 to use default."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(13.5)
    }
    fn body(&self) -> Rect {
        Rect::new(-14.0, -33.0, 28.0, 42.0)
    }
    fn interact_toggle(&mut self, local: crate::canvas::Point) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 16.0;
        if knob_hit {
            let step = (self.max_c - self.min_c) / 20.0;
            let mut val = self.capacitance + step;
            if val > self.max_c {
                val = self.min_c;
            }
            self.capacitance = val;
            true
        } else {
            false
        }
    }
    fn interact_wheel(&mut self, local: crate::canvas::Point, delta: f64) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 16.0;
        let body_hit = local.x >= -16.0 && local.x <= 16.0 && local.y >= -16.0 && local.y <= 16.0;
        if knob_hit || body_hit {
            let def_step = (self.max_c - self.min_c) / 40.0;
            let step = if self.dial.step > 0.0 && self.dial.step >= def_step * 0.05 {
                self.dial.step
            } else {
                def_step
            };
            let num_steps = if delta.abs() >= 120.0 {
                (delta / 120.0).round()
            } else {
                delta.signum()
            };
            self.capacitance = (self.capacitance + num_steps * step).clamp(self.min_c, self.max_c);
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_var_capacitor(&mut self, x: f64, y: f64) -> String {
        let id = format!("VarCapacitor-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::var_capacitor(&id, x, y, 10e-6));
        id
    }
}

impl TwoTerminal for VarCapacitor {}

impl Stampable for VarCapacitor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let g = self.capacitance.max(MIN_C) / dt;
        stamp_two_terminal(matrix, pin_nodes, g, self.init_volt * g);
    }
}

impl Dialed for VarCapacitor {
    fn set_value(&mut self, v: f64) -> ComponentChange {
        self.capacitance = v.clamp(self.min(), self.max());
        ComponentChange::document("")
    }
    fn value(&self) -> f64 {
        self.capacitance
    }
    fn min(&self) -> f64 {
        self.min_c.min(self.max_c).max(MIN_C)
    }
    fn max(&self) -> f64 {
        self.min_c.max(self.max_c).max(MIN_C)
    }
}

use super::drawable::{Drawable, paint_dial};
use crate::canvas::draw::{Draw, PaintCtx};

impl Drawable for VarCapacitor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_rect(-3.5, -6.0, 1.0, 12.0, ctx.pal.border);
        d.fill_rect(2.5, -6.0, 1.0, 12.0, ctx.pal.border);
        d.push(0.0, -19.0, 0.0, 1.0, 1.0);
        paint_dial(d, ctx.pal, self.capacitance, self.min_c, self.max_c);
        d.pop();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_and_dial() {
        let mut v = VarCapacitor::default();
        assert_eq!(v.capacitance, DEFAULT_C);
        assert_eq!(
            v.get_prop_text("Capacitance").unwrap(),
            format_si(DEFAULT_C, "F")
        );
        v.set_value(50e-12);
        assert_eq!(v.get_prop_text("Capacitance").unwrap(), "50 pF");
    }
}
