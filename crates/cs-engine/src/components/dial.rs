//! Dial: rotary schematic control with value, min, max, and step.

use super::drawable::{Drawable, paint_dial};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const DEFAULT_VAL: f64 = 0.0;
const DEFAULT_MIN: f64 = 0.0;
const DEFAULT_MAX: f64 = 100.0;
const DEFAULT_STEP: f64 = 1.0;

/// Rotary control dial.
#[derive(Clone, Debug, PartialEq)]
pub struct Dial {
    pub value: f64,
    pub min_val: f64,
    pub max_val: f64,
    pub step: f64,
}

impl crate::canvas::Item {
    pub fn dial(id: impl Into<String>, x: f64, y: f64, val: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Dial {
                value: val,
                min_val: 0.0,
                max_val: 100.0,
                step: 1.0,
            },
        )
    }
}

impl Default for Dial {
    fn default() -> Self {
        Self {
            value: DEFAULT_VAL,
            min_val: DEFAULT_MIN,
            max_val: DEFAULT_MAX,
            step: DEFAULT_STEP,
        }
    }
}

impl Dial {
    pub const TYPE_ID: &'static str = "Dial";
    pub fn wiper(&self) -> f64 {
        let span = (self.max_val - self.min_val).max(1e-6);
        ((self.value - self.min_val) / span).clamp(0.0, 1.0)
    }

    pub fn new(val: f64) -> Self {
        Self {
            value: val,
            min_val: DEFAULT_MIN,
            max_val: DEFAULT_MAX,
            step: DEFAULT_STEP,
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Dial {
            value: self.value,
            min_val: self.min_val,
            max_val: self.max_val,
            step: self.step,
        }
    }

    fn clamp_val(&mut self) {
        let lo = self.min_val.min(self.max_val);
        let hi = self.min_val.max(self.max_val);
        self.value = self.value.clamp(lo, hi);
    }

    fn get_value(&self) -> PropValue {
        PropValue::Float(self.value)
    }
    fn set_value_prop(&mut self, v: PropValue) -> Result<(), PropError> {
        let val = expect_float("Value", v)?;
        self.value = val;
        self.clamp_val();
        Ok(())
    }

    fn get_min_val(&self) -> PropValue {
        PropValue::Float(self.min_val)
    }
    fn set_min_val(&mut self, v: PropValue) -> Result<(), PropError> {
        self.min_val = expect_float("MinVal", v)?;
        self.clamp_val();
        Ok(())
    }

    fn get_max_val(&self) -> PropValue {
        PropValue::Float(self.max_val)
    }
    fn set_max_val(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_val = expect_float("MaxVal", v)?;
        self.clamp_val();
        Ok(())
    }

    fn get_step(&self) -> PropValue {
        PropValue::Float(self.step)
    }
    fn set_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.step = expect_float("Step", v)?;
        Ok(())
    }
}

impl Dialed for Dial {
    fn set_value(&mut self, v: f64) -> ComponentChange {
        let lo = self.min_val.min(self.max_val);
        let hi = self.min_val.max(self.max_val);
        self.value = v.clamp(lo, hi);
        ComponentChange::document("")
    }

    fn value(&self) -> f64 {
        self.value
    }

    fn min(&self) -> f64 {
        self.min_val
    }

    fn max(&self) -> f64 {
        self.max_val
    }
}

impl Component for Dial {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Rotary control dial."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Dial>] = &[
            PropDef::float(
                "Value",
                "Value",
                "",
                -1e12,
                1e12,
                Dial::get_value,
                Dial::set_value_prop,
            )
            .with_info("Output value."),
            PropDef::float(
                "MinVal",
                "Minimum",
                "",
                -1e12,
                1e12,
                Dial::get_min_val,
                Dial::set_min_val,
            )
            .with_info("Value with dial at the left end."),
            PropDef::float(
                "MaxVal",
                "Maximum",
                "",
                -1e12,
                1e12,
                Dial::get_max_val,
                Dial::set_max_val,
            )
            .with_info("Value with dial at the right end."),
            PropDef::float(
                "Step",
                "Step",
                "",
                1e-6,
                1e6,
                Dial::get_step,
                Dial::set_step,
            )
            .with_info("Step increment per dial tick or scroll."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        Vec::new()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -16.0, 32.0, 32.0)
    }

    fn interact_toggle(&mut self, _local: crate::canvas::Point) -> bool {
        let mut v = self.value + self.step;
        if v > self.max_val {
            v = self.min_val;
        }
        self.value = v;
        true
    }

    fn interact_wheel(&mut self, local: crate::canvas::Point, delta: f64) -> bool {
        let r = local.x.hypot(local.y);
        if r <= 14.0 {
            let step_size = if self.step > 0.0 {
                self.step
            } else {
                (self.max_val - self.min_val) / 40.0
            };
            let num_steps = if delta.abs() >= 120.0 {
                (delta / 120.0).round()
            } else {
                delta.signum()
            };
            self.value = (self.value + num_steps * step_size).clamp(self.min_val, self.max_val);
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_dial(&mut self, x: f64, y: f64) -> String {
        let id = format!("Dial-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::dial(&id, x, y, 50.0));
        id
    }

    pub fn set_dial_val(&mut self, uid: &str, val: f64) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            match &mut item.kind {
                crate::components::Part::Dial(p) => {
                    p.value = val.clamp(p.min_val, p.max_val);
                    return true;
                }
                crate::components::Part::VoltSource(p) => {
                    p.value = val.clamp(p.min_value, p.max_value);
                    return true;
                }
                crate::components::Part::CurrSource(p) => {
                    p.value = val.clamp(p.min_value, p.max_value);
                    return true;
                }
                crate::components::Part::VarResistor(p) => {
                    p.resistance = val.clamp(p.min_r.min(p.max_r), p.min_r.max(p.max_r));
                    return true;
                }
                crate::components::Part::VarCapacitor(p) => {
                    p.capacitance = val.clamp(p.min_c.min(p.max_c), p.min_c.max(p.max_c));
                    return true;
                }
                crate::components::Part::VarInductor(p) => {
                    p.inductance = val.clamp(p.min_l.min(p.max_l), p.min_l.max(p.max_l));
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}

impl Stampable for Dial {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // Dial is a user-interaction schematic control, not an electrical element.
    }
}

impl Drawable for Dial {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_dial(d, ctx.pal, self.value, self.min_val, self.max_val);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dial_defaults_and_props() {
        let mut dial = Dial::default();
        assert_eq!(dial.type_id(), "Dial");
        assert_eq!(dial.get_prop_text("Value").unwrap(), "0");
        assert_eq!(dial.get_prop_text("MinVal").unwrap(), "0");
        assert_eq!(dial.get_prop_text("MaxVal").unwrap(), "100");
        assert_eq!(dial.get_prop_text("Step").unwrap(), "1");

        dial.set_prop_text("Value", "42").unwrap();
        assert_eq!(dial.value, 42.0);
        dial.set_prop_text("MinVal", "10").unwrap();
        assert_eq!(dial.min_val, 10.0);
        dial.set_prop_text("MaxVal", "200").unwrap();
        assert_eq!(dial.max_val, 200.0);
        dial.set_prop_text("Step", "5").unwrap();
        assert_eq!(dial.step, 5.0);
    }

    #[test]
    fn dial_dialed_interaction() {
        let mut dial = Dial::default();
        let change = dial.set_value(55.0);
        assert!(change.saved);
        assert_eq!(dial.value(), 55.0);

        // Clamping to [min, max]
        dial.set_value(150.0);
        assert_eq!(dial.value(), 100.0);
        dial.set_value(-10.0);
        assert_eq!(dial.value(), 0.0);
    }

    #[test]
    fn dial_body_and_element_kind() {
        let dial = Dial::default();
        assert_eq!(dial.body(), Rect::new(-16.0, -16.0, 32.0, 32.0));
        assert_eq!(dial.pin_geoms().len(), 0);
        match dial.to_element_kind() {
            Kind::Dial {
                value,
                min_val,
                max_val,
                step,
            } => {
                assert_eq!(value, 0.0);
                assert_eq!(min_val, 0.0);
                assert_eq!(max_val, 100.0);
                assert_eq!(step, 1.0);
            }
            other => panic!("expected Kind::Dial, got {other:?}"),
        }
    }
}
