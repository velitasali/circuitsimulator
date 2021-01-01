//! Potentiometer: two series resistors sharing a wiper pin.

use super::component::{DialState, clamp_positive, resistor_g, stamp_conductance_between};
use super::props::{PropDef, PropError, PropValue, expect_float, expect_string};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_R: f64 = 1_000.0;

impl crate::canvas::Item {
    pub fn potentiometer(
        id: impl Into<String>,
        x: f64,
        y: f64,
        resistance: f64,
        wiper: f64,
    ) -> Self {
        Self::potentiometer_with(id, x, y, resistance, wiper, String::new(), 1.0)
    }

    pub fn potentiometer_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        resistance: f64,
        wiper: f64,
        key: String,
        dial_step: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Potentiometer {
                resistance,
                wiper,
                dial: DialState {
                    key,
                    step: dial_step,
                },
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Potentiometer {
    pub resistance: f64,
    pub wiper: f64,
    pub dial: DialState,
}

impl Default for Potentiometer {
    fn default() -> Self {
        Self {
            resistance: DEFAULT_R,
            wiper: 0.5,
            dial: DialState::new(1.0),
        }
    }
}

impl Potentiometer {
    pub const TYPE_ID: &'static str = "Potentiometer";
    pub fn new(resistance: f64, wiper: f64) -> Self {
        Self {
            resistance: clamp_positive(resistance, MIN_OHMS),
            wiper: wiper.clamp(0.0, 1.0),
            dial: DialState::new(1.0),
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Potentiometer {
            resistance: self.resistance.max(MIN_OHMS),
            wiper: self.wiper.clamp(0.0, 1.0),
        }
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn get_wiper(&self) -> PropValue {
        PropValue::Float(self.wiper)
    }
    fn set_wiper(&mut self, v: PropValue) -> Result<(), PropError> {
        self.wiper = expect_float("Wiper", v)?.clamp(0.0, 1.0);
        Ok(())
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
        self.dial.step = expect_float("DialStep", v)?.clamp(0.0, 100.0);
        Ok(())
    }
    fn get_value_ohm(&self) -> PropValue {
        PropValue::Float(self.resistance * self.wiper)
    }
    fn set_value_ohm(&mut self, v: PropValue) -> Result<(), PropError> {
        let val = expect_float("Value_Ohm", v)?.max(0.0);
        self.wiper = if self.resistance > 1e-12 {
            (val / self.resistance).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Ok(())
    }
}

impl Component for Potentiometer {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Potentiometer."
    }
    fn props() -> &'static [PropDef<Self>] {
        const VAL_OHM: PropDef<Potentiometer> = {
            let mut p = PropDef::float(
                "Value_Ohm",
                "Current Value",
                "Ω",
                0.0,
                MAX_OHMS,
                Potentiometer::get_value_ohm,
                Potentiometer::set_value_ohm,
            )
            .with_info("Value determined by dial position.");
            p.persist = false;
            p.show_by_default = false;
            p
        };
        static PROPS: &[PropDef<Potentiometer>] = &[
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Potentiometer::get_resistance,
                Potentiometer::set_resistance,
            )
            .with_info("Total end-to-end resistance value, in ohms."),
            VAL_OHM,
            PropDef::float(
                "Wiper",
                "Wiper (0-1)",
                "",
                0.0,
                1.0,
                Potentiometer::get_wiper,
                Potentiometer::set_wiper,
            )
            .with_info("Wiper position fraction from 0.0 to 1.0."),
            PropDef::string("Key", "Key", Potentiometer::get_key, Potentiometer::set_key)
                .with_info(
                    "Character shown in the button.\nCan be activated by keyboard in your PC.",
                ),
            PropDef::float(
                "DialStep",
                "Dial Step",
                "%",
                0.0,
                100.0,
                Potentiometer::get_dial_step,
                Potentiometer::set_dial_step,
            )
            .with_info("Minimum step when rotating the dial.\n0 to use default."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-lPin", -16.0, 0.0, 180, 5.0),
            CompPin::new("-rPin", 16.0, 0.0, 0, 5.0),
            CompPin::new("-wPin", 0.0, 16.0, 270, 6.0),
        ]
    }
    fn body(&self) -> Rect {
        Rect::new(-14.0, -33.0, 28.0, 45.0)
    }
    fn interact_toggle(&mut self, local: crate::canvas::Point) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 14.0;
        if knob_hit {
            let step = if self.dial.step > 0.0 {
                self.dial.step / 100.0
            } else {
                0.05
            };
            let mut w = self.wiper + step;
            if w > 1.0 {
                w = 0.0;
            }
            self.wiper = (w * 100.0).round() / 100.0;
            true
        } else {
            false
        }
    }
    fn interact_wheel(&mut self, local: crate::canvas::Point, delta: f64) -> bool {
        let knob_hit = local.x.hypot(local.y - (-19.0)) <= 14.0;
        let body_hit = local.x >= -12.0 && local.x <= 12.0 && local.y >= -8.0 && local.y <= 12.0;
        if knob_hit || body_hit {
            let step = 0.01;
            let num_steps = if delta.abs() >= 120.0 {
                (delta / 120.0).round()
            } else {
                delta.signum()
            };
            self.wiper = (self.wiper + num_steps * step).clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_potentiometer(&mut self, x: f64, y: f64) -> String {
        let id = format!("Potentiometer-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::potentiometer(&id, x, y, 1000.0, 0.5));
        id
    }

    pub fn set_pot_wiper(&mut self, uid: &str, wiper_val: f64) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::Potentiometer(p) = &mut item.kind {
                p.wiper = wiper_val.clamp(0.0, 1.0);
                return true;
            }
        }
        false
    }
}

impl Stampable for Potentiometer {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let r = self.resistance.max(MIN_OHMS);
        let w = self.wiper.clamp(0.0, 1.0);
        let g1 = resistor_g(w * r);
        let g2 = resistor_g((1.0 - w) * r);
        stamp_conductance_between(matrix, pin_nodes, 0, 2, g1);
        stamp_conductance_between(matrix, pin_nodes, 2, 1, g2);
    }
}

impl Dialed for Potentiometer {
    fn set_value(&mut self, v: f64) -> ComponentChange {
        self.wiper = v.clamp(0.0, 1.0);
        ComponentChange::document("")
    }
    fn value(&self) -> f64 {
        self.wiper
    }
    fn min(&self) -> f64 {
        0.0
    }
    fn max(&self) -> f64 {
        1.0
    }
}

use super::drawable::{Drawable, paint_dial};
use super::resistor::paint_resistor_body;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Potentiometer {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, self.resistance, true);
        d.line(0.0, 10.0, 0.0, 4.5, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(-2.5, 7.0, 0.0, 4.5, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(2.5, 7.0, 0.0, 4.5, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.push(0.0, -19.0, 0.0, 1.0, 1.0);
        paint_dial(d, ctx.pal, self.wiper, 0.0, 1.0);
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
        let p = Potentiometer::default();
        assert_eq!(p.resistance, DEFAULT_R);
        assert_eq!(p.wiper, 0.5);
        assert_eq!(
            p.get_prop_text("Resistance").unwrap(),
            format_si(DEFAULT_R, "Ω")
        );
        assert_eq!(p.get_prop_text("Wiper").unwrap(), "0.5");
    }

    #[test]
    fn dial_writes_wiper() {
        let mut p = Potentiometer::default();
        let c = p.set_value(0.25);
        assert!(c.saved);
        assert_eq!(p.wiper, 0.25);
        assert_eq!(p.get_prop_text("Wiper").unwrap(), "0.25");
    }

    #[test]
    fn stamp_mid_wiper_divides() {
        let p = Potentiometer::new(1_000.0, 0.5);
        let mut m = CircMatrix::new(2);
        m.analyze(&[vec![], vec![]]);
        // lPin=0 grounded through wiper at 1; rPin open (MAX).
        p.stamp(&mut m, &[usize::MAX, usize::MAX, 0], 0.0);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0; 2];
        assert!(m.solve(&mut v));
        // two 500 Ω to ground in parallel from the wiper → 250 Ω, V = 250.
        assert!((v[0] - 250.0).abs() < 1e-3, "{}", v[0]);
    }
}
