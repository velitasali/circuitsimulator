//! Variable inductor: companion stamp plus a dial on Inductance.

use super::component::{DialState, stamp_two_terminal, two_terminal_pins};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_L: f64 = 1e-12;
const MAX_L: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_L: f64 = 1e-3;
const DEFAULT_R: f64 = 1e-3;

impl crate::canvas::Item {
    pub fn var_inductor(id: impl Into<String>, x: f64, y: f64, inductance: f64) -> Self {
        Self::var_inductor_with(id, x, y, inductance, 0.0, inductance * 2.0, 1e-3, 0.0, 0.0)
    }

    pub fn var_inductor_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        inductance: f64,
        min_l: f64,
        max_l: f64,
        resistance: f64,
        init_curr: f64,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            VarInductor {
                inductance,
                min_l,
                max_l,
                resistance,
                init_curr,
                dial: DialState {
                    key: String::new(),
                    step: dial_step,
                },
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarInductor {
    pub inductance: f64,
    pub min_l: f64,
    pub max_l: f64,
    pub resistance: f64,
    pub init_curr: f64,
    pub dial: DialState,
}

impl Default for VarInductor {
    fn default() -> Self {
        Self {
            inductance: DEFAULT_L,
            min_l: 0.0,
            max_l: DEFAULT_L * 2.0,
            resistance: DEFAULT_R,
            init_curr: 0.0,
            dial: DialState::new(1e-6),
        }
    }
}

impl VarInductor {
    pub const TYPE_ID: &'static str = "VarInductor";
    pub fn wiper(&self) -> f64 {
        let span = (self.max_l - self.min_l).max(1e-15);
        ((self.inductance - self.min_l) / span).clamp(0.0, 1.0)
    }

    pub fn new(inductance: f64) -> Self {
        let l = inductance.max(MIN_L);
        Self {
            inductance: l,
            min_l: 0.0,
            max_l: l * 2.0,
            resistance: DEFAULT_R,
            init_curr: 0.0,
            dial: DialState::new(1e-6),
        }
    }

    fn clamp_l(&mut self) {
        let lo = self.min_l.min(self.max_l).max(MIN_L);
        let hi = self.min_l.max(self.max_l).max(lo);
        self.inductance = self.inductance.clamp(lo, hi);
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::VarInductor {
            inductance: self.inductance.max(MIN_L),
            ieq: self.init_curr,
        }
    }

    fn get_inductance(&self) -> PropValue {
        PropValue::Float(self.inductance)
    }
    fn set_inductance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance = expect_float("Inductance", v)?.clamp(MIN_L, MAX_L);
        self.clamp_l();
        Ok(())
    }
    fn get_min_l(&self) -> PropValue {
        PropValue::Float(self.min_l)
    }
    fn set_min_l(&mut self, v: PropValue) -> Result<(), PropError> {
        self.min_l = expect_float("MinInductance", v)?.clamp(0.0, MAX_L);
        self.clamp_l();
        Ok(())
    }
    fn get_max_l(&self) -> PropValue {
        PropValue::Float(self.max_l)
    }
    fn set_max_l(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_l = expect_float("MaxInductance", v)?.clamp(MIN_L, MAX_L);
        self.clamp_l();
        Ok(())
    }
    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn get_init_curr(&self) -> PropValue {
        PropValue::Float(self.init_curr)
    }
    fn set_init_curr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.init_curr = expect_float("InitVolt", v)?;
        Ok(())
    }
    fn get_dial_step(&self) -> PropValue {
        PropValue::Float(self.dial.step)
    }
    fn set_dial_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial.step = expect_float("DialStep", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }
}

impl Component for VarInductor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Variable inductor."
    }
    fn props() -> &'static [PropDef<Self>] {
        const IND: PropDef<VarInductor> = {
            let mut p = PropDef::float(
                "Inductance",
                "Inductance",
                "H",
                MIN_L,
                MAX_L,
                VarInductor::get_inductance,
                VarInductor::set_inductance,
            )
            .with_info("Value determined by dial position.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<VarInductor>] = &[
            IND,
            PropDef::float(
                "MinInductance",
                "Min Inductance",
                "H",
                0.0,
                MAX_L,
                VarInductor::get_min_l,
                VarInductor::set_min_l,
            )
            .with_info("Inductance with dial at the left end."),
            PropDef::float(
                "MaxInductance",
                "Max Inductance",
                "H",
                MIN_L,
                MAX_L,
                VarInductor::get_max_l,
                VarInductor::set_max_l,
            )
            .with_info("Inductance with dial at the right end."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                VarInductor::get_resistance,
                VarInductor::set_resistance,
            )
            .with_info("Series resistance of the winding."),
            PropDef::float(
                "InitVolt",
                "Initial Current",
                "A",
                -1e6,
                1e6,
                VarInductor::get_init_curr,
                VarInductor::set_init_curr,
            )
            .with_info("Voltage at simulation start (initial charge)."),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "H",
                MIN_L,
                MAX_L,
                VarInductor::get_dial_step,
                VarInductor::set_dial_step,
            )
            .with_info("Minimum step when rotating the dial.\n0 to use default."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(4.0)
    }
    fn body(&self) -> Rect {
        Rect::new(-14.0, -33.0, 28.0, 42.0)
    }
    fn interact_toggle(&mut self, local: crate::canvas::Point) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 16.0;
        if knob_hit {
            let step = (self.max_l - self.min_l) / 20.0;
            let mut val = self.inductance + step;
            if val > self.max_l {
                val = self.min_l;
            }
            self.inductance = val;
            true
        } else {
            false
        }
    }
    fn interact_wheel(&mut self, local: crate::canvas::Point, delta: f64) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 16.0;
        let body_hit = local.x >= -16.0 && local.x <= 16.0 && local.y >= -16.0 && local.y <= 16.0;
        if knob_hit || body_hit {
            let def_step = (self.max_l - self.min_l) / 40.0;
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
            self.inductance = (self.inductance + num_steps * step).clamp(self.min_l, self.max_l);
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_var_inductor(&mut self, x: f64, y: f64) -> String {
        let id = format!("VarInductor-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::var_inductor(&id, x, y, 0.001));
        id
    }
}

impl TwoTerminal for VarInductor {}

impl Stampable for VarInductor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let g = dt / self.inductance.max(MIN_L);
        stamp_two_terminal(matrix, pin_nodes, g, self.init_curr);
    }
}

impl Dialed for VarInductor {
    fn set_value(&mut self, v: f64) -> ComponentChange {
        self.inductance = v.clamp(self.min(), self.max());
        ComponentChange::document("")
    }
    fn value(&self) -> f64 {
        self.inductance
    }
    fn min(&self) -> f64 {
        self.min_l.min(self.max_l).max(MIN_L)
    }
    fn max(&self) -> f64 {
        self.min_l.max(self.max_l).max(MIN_L)
    }
}

use super::drawable::{Drawable, paint_dial};
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for VarInductor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let deg = std::f64::consts::PI / 180.0;
        d.arc(
            -7.0,
            0.5,
            5.0,
            185.0 * deg,
            405.0 * deg,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.arc(
            0.0,
            0.5,
            5.0,
            135.0 * deg,
            405.0 * deg,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.arc(
            7.0,
            0.5,
            5.0,
            135.0 * deg,
            355.0 * deg,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.push(0.0, -19.0, 0.0, 1.0, 1.0);
        paint_dial(d, ctx.pal, self.inductance, self.min_l, self.max_l);
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
        let mut v = VarInductor::default();
        assert_eq!(v.inductance, DEFAULT_L);
        assert_eq!(
            v.get_prop_text("Inductance").unwrap(),
            format_si(DEFAULT_L, "H")
        );
        v.set_value(500e-6);
        assert_eq!(v.get_prop_text("Inductance").unwrap(), "500 µH");
    }
}
