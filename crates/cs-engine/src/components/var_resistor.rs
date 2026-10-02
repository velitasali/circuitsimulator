//! Variable resistor: conductance stamp plus a dial on Resistance.

use super::component::{
    DialState, clamp_positive, resistor_g, stamp_two_terminal, two_terminal_pins,
};
use super::prop_links;
use super::props::{PropDef, PropError, PropUpdate, PropValue, expect_float, expect_string};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_MAX_R: f64 = 1_000.0;

impl crate::canvas::Item {
    pub fn var_resistor(id: impl Into<String>, x: f64, y: f64, resistance: f64) -> Self {
        Self::var_resistor_with(
            id,
            x,
            y,
            resistance,
            0.0,
            resistance * 2.0,
            String::new(),
            10.0,
        )
    }

    pub fn var_resistor_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        resistance: f64,
        min_r: f64,
        max_r: f64,
        key: String,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            VarResistor {
                resistance,
                min_r,
                max_r,
                dial: DialState {
                    key,
                    step: dial_step,
                },
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarResistor {
    pub resistance: f64,
    pub min_r: f64,
    pub max_r: f64,
    pub dial: DialState,
}

impl Default for VarResistor {
    fn default() -> Self {
        Self {
            resistance: 0.0,
            min_r: 0.0,
            max_r: DEFAULT_MAX_R,
            dial: DialState::new(0.0),
        }
    }
}

impl VarResistor {
    pub const TYPE_ID: &'static str = "VarResistor";
    pub fn wiper(&self) -> f64 {
        let span = (self.max_r - self.min_r).max(1e-6);
        ((self.resistance - self.min_r) / span).clamp(0.0, 1.0)
    }

    pub fn new(resistance: f64) -> Self {
        let r = clamp_positive(resistance, MIN_OHMS);
        Self {
            resistance: r,
            min_r: 0.0,
            max_r: r * 2.0,
            dial: DialState::new(1.0),
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::VarResistor {
            resistance: self.resistance.max(MIN_OHMS),
        }
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(0.0, MAX_OHMS);
        Ok(())
    }
    fn value_adjusts(part: &mut Self, _: &PropValue) {
        prop_links::dialed_value(&mut part.resistance, part.min_r, part.max_r);
    }
    fn get_min_r(&self) -> PropValue {
        PropValue::Float(self.min_r)
    }
    fn set_min_r(&mut self, v: PropValue) -> Result<(), PropError> {
        self.min_r = expect_float("MinResistance", v)?.clamp(0.0, MAX_OHMS);
        Ok(())
    }
    fn min_adjusts(part: &mut Self, _: &PropValue) {
        prop_links::dialed_min(&mut part.min_r, part.max_r, &mut part.resistance);
    }
    fn get_max_r(&self) -> PropValue {
        PropValue::Float(self.max_r)
    }
    fn set_max_r(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_r = expect_float("MaxResistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn max_adjusts(part: &mut Self, _: &PropValue) {
        prop_links::dialed_max(&mut part.max_r, part.min_r, &mut part.resistance);
    }
    fn get_key(&self) -> PropValue {
        PropValue::String(self.dial.key.clone())
    }
    fn set_key(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial.key = expect_string("Key", v)?;
        Ok(())
    }
    fn get_dial_step(&self) -> PropValue {
        PropValue::Float(self.dial.step)
    }
    fn set_dial_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial.step = expect_float("DialStep", v)?.clamp(0.0, MAX_OHMS);
        Ok(())
    }
}

impl Component for VarResistor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Variable resistor."
    }
    fn props() -> &'static [PropDef<Self>] {
        const RES_UPDATES: &[PropUpdate<VarResistor>] =
            &[PropUpdate::always(VarResistor::value_adjusts)];
        const MIN_UPDATES: &[PropUpdate<VarResistor>] =
            &[PropUpdate::always(VarResistor::min_adjusts)];
        const MAX_UPDATES: &[PropUpdate<VarResistor>] =
            &[PropUpdate::always(VarResistor::max_adjusts)];
        static PROPS: &[PropDef<VarResistor>] = &[
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                0.0,
                MAX_OHMS,
                VarResistor::get_resistance,
                VarResistor::set_resistance,
            )
            .updates(RES_UPDATES)
            .with_info("Resistance value, in ohms."),
            PropDef::float(
                "MinResistance",
                "Min Resistance",
                "Ω",
                0.0,
                MAX_OHMS,
                VarResistor::get_min_r,
                VarResistor::set_min_r,
            )
            .updates(MIN_UPDATES)
            .with_info("Resistance with dial at the left end."),
            PropDef::float(
                "MaxResistance",
                "Max Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                VarResistor::get_max_r,
                VarResistor::set_max_r,
            )
            .updates(MAX_UPDATES)
            .with_info("Resistance with dial at the right end."),
            PropDef::string("Key", "Key", VarResistor::get_key, VarResistor::set_key).with_info(
                "Character shown in the button.\nCan be activated by keyboard in your PC.",
            ),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "Ω",
                0.0,
                MAX_OHMS,
                VarResistor::get_dial_step,
                VarResistor::set_dial_step,
            )
            .with_info("Minimum step when rotating the dial.\n0 to use default."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(5.0)
    }
    fn body(&self) -> Rect {
        Rect::new(-14.0, -33.0, 28.0, 42.0)
    }
    fn interact_toggle(&mut self, local: crate::canvas::Point) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 16.0;
        if knob_hit {
            let step = (self.max_r - self.min_r) / 20.0;
            let mut val = self.resistance + step;
            if val > self.max_r {
                val = self.min_r;
            }
            self.resistance = val;
            true
        } else {
            false
        }
    }
    fn interact_wheel(&mut self, local: crate::canvas::Point, delta: f64) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 16.0;
        let body_hit = local.x >= -16.0 && local.x <= 16.0 && local.y >= -16.0 && local.y <= 16.0;
        if knob_hit || body_hit {
            let def_step = (self.max_r - self.min_r) / 40.0;
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
            self.resistance = (self.resistance + num_steps * step).clamp(self.min_r, self.max_r);
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_var_resistor(&mut self, x: f64, y: f64) -> String {
        let id = format!("VarResistor-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, VarResistor::default()));
        id
    }
}

impl TwoTerminal for VarResistor {}

impl Stampable for VarResistor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.resistance), 0.0);
    }
}

impl Dialed for VarResistor {
    fn set_value(&mut self, v: f64) -> ComponentChange {
        self.resistance = v.clamp(self.min(), self.max());
        ComponentChange::document("")
    }
    fn value(&self) -> f64 {
        self.resistance
    }
    fn min(&self) -> f64 {
        self.min_r.min(self.max_r)
    }
    fn max(&self) -> f64 {
        self.min_r.max(self.max_r)
    }
}

use super::drawable::{Drawable, paint_dial};
use super::resistor::paint_resistor_body;
use crate::canvas::draw::{Draw, PaintCtx};

impl Drawable for VarResistor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, self.resistance, true);
        d.push(0.0, -19.0, 0.0, 1.0, 1.0);
        paint_dial(d, ctx.pal, self.resistance, self.min_r, self.max_r);
        d.pop();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let mut v = VarResistor::default();
        assert_eq!(v.resistance, 0.0);
        assert_eq!(v.min_r, 0.0);
        assert_eq!(v.max_r, DEFAULT_MAX_R);
        assert_eq!(v.dial.step, 0.0);
        assert_eq!(v.get_prop_text("Resistance").unwrap(), format_si(0.0, "Ω"));
        assert_eq!(v.min(), 0.0);
        v.set_value(0.0);
        assert_eq!(v.resistance, 0.0);
    }

    #[test]
    fn dial_writes_resistance() {
        let mut v = VarResistor::default();
        let c = v.set_value(500.0);
        assert!(c.saved && c.undo && c.sim);
        assert_eq!(v.get_prop_text("Resistance").unwrap(), "500 Ω");
        v.set_value(10_000.0);
        assert_eq!(v.value(), v.max());
    }
}
