//! Adjustable current source with running flag and a dial on Value.

use super::component::stamp_current;
use super::drawable::{Drawable, paint_var_source_body};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_A: f64 = -1e6;
const MAX_A: f64 = 1e6;
const DEFAULT_A: f64 = 0.01;
const DEFAULT_MAX: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct CurrSource {
    pub value: f64,
    pub max_value: f64,
    pub min_value: f64,
    pub running: bool,
}

impl crate::canvas::Item {
    pub fn curr_source(id: impl Into<String>, x: f64, y: f64, value: f64, running: bool) -> Self {
        Self::curr_source_with(id, x, y, 1.0, 0.0, value, running)
    }

    pub fn curr_source_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        max_value: f64,
        min_value: f64,
        value: f64,
        running: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            CurrSource {
                max_value,
                min_value,
                value,
                running,
            },
        )
    }
}

impl Default for CurrSource {
    fn default() -> Self {
        Self {
            value: DEFAULT_A,
            max_value: DEFAULT_MAX,
            min_value: 0.0,
            running: true,
        }
    }
}

impl CurrSource {
    pub const TYPE_ID: &'static str = "CurrSource";
    pub fn new(value: f64, running: bool) -> Self {
        Self {
            value,
            max_value: DEFAULT_MAX,
            min_value: 0.0,
            running,
        }
    }

    fn clamp_value(&mut self) {
        let lo = self.min_value.min(self.max_value);
        let hi = self.min_value.max(self.max_value);
        self.value = self.value.clamp(lo, hi);
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::CurrSource {
            value: self.value,
            running: self.running,
        }
    }

    fn get_value(&self) -> PropValue {
        PropValue::Float(self.value)
    }
    fn set_value_prop(&mut self, v: PropValue) -> Result<(), PropError> {
        self.value = expect_float("Value", v)?.clamp(MIN_A, MAX_A);
        self.clamp_value();
        Ok(())
    }
    fn get_max(&self) -> PropValue {
        PropValue::Float(self.max_value)
    }
    fn set_max(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_value = expect_float("MaxValue", v)?.clamp(MIN_A, MAX_A);
        if self.max_value < self.min_value {
            self.min_value = self.max_value;
        }
        self.clamp_value();
        Ok(())
    }
    fn get_min(&self) -> PropValue {
        PropValue::Float(self.min_value)
    }
    fn set_min(&mut self, v: PropValue) -> Result<(), PropError> {
        self.min_value = expect_float("MinValue", v)?.clamp(MIN_A, MAX_A);
        if self.min_value > self.max_value {
            self.max_value = self.min_value;
        }
        self.clamp_value();
        Ok(())
    }
    fn get_running(&self) -> PropValue {
        PropValue::Bool(self.running)
    }
    fn set_running(&mut self, v: PropValue) -> Result<(), PropError> {
        self.running = expect_bool("Running", v)?;
        Ok(())
    }
}

impl Component for CurrSource {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Adjustable current source."
    }
    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<CurrSource>] = &[
            PropDef::float(
                "Value",
                "Current",
                "A",
                MIN_A,
                MAX_A,
                CurrSource::get_value,
                CurrSource::set_value_prop,
            )
            .with_info("Output value."),
            PropDef::float(
                "MaxValue",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                CurrSource::get_max,
                CurrSource::set_max,
            )
            .with_info("Maximum current (must be a positive value)."),
            PropDef::float(
                "MinValue",
                "Min Current",
                "A",
                MIN_A,
                MAX_A,
                CurrSource::get_min,
                CurrSource::set_min,
            )
            .with_info("Minimum current. Must be less than Maximum Current."),
            PropDef::bool(
                "Running",
                "Running",
                CurrSource::get_running,
                CurrSource::set_running,
            )
            .with_info("Active simulation state of the source."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![CompPin::new("-outPin", 28.0, 16.0, 0, 8.0).with_direction(PinDirection::Out)]
    }
    fn body(&self) -> Rect {
        Rect::new(-20.0, -28.0, 40.0, 56.0)
    }
    fn interact_toggle(&mut self, local: crate::canvas::Point) -> bool {
        let btn_hit = local.x >= -18.0 && local.x <= 18.0 && local.y >= 8.0 && local.y <= 26.0;
        let prog_hit = local.x.abs() < 1e-6 && local.y.abs() < 1e-6;
        if btn_hit || prog_hit {
            self.running = !self.running;
            true
        } else {
            false
        }
    }
    fn interact_wheel(&mut self, local: crate::canvas::Point, delta: f64) -> bool {
        let dx = local.x;
        let dy = local.y - (-8.0);
        let knob_hit = dx.hypot(dy) <= 16.0;
        let body_hit = local.x >= -20.0 && local.x <= 20.0 && local.y >= -28.0 && local.y <= 28.0;
        if knob_hit || body_hit {
            let span = self.max_value - self.min_value;
            if span.abs() > 1e-12 {
                let step = span / 100.0;
                let num_steps = if delta.abs() >= 120.0 {
                    (delta / 120.0).round()
                } else {
                    delta.signum()
                };
                let new_val = self.value + num_steps * step;
                let steps_from_min = ((new_val - self.min_value) / step).round();
                let lo = self.min_value.min(self.max_value);
                let hi = self.min_value.max(self.max_value);
                self.value = (self.min_value + steps_from_min * step).clamp(lo, hi);
                return true;
            }
        }
        false
    }
}

impl crate::canvas::Scene {
    pub fn add_curr_source(&mut self, x: f64, y: f64) -> String {
        let id = format!("CurrSource-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::curr_source(&id, x, y, 0.01, true));
        id
    }
}

impl Stampable for CurrSource {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if self.running {
            stamp_current(matrix, pin_nodes, 0, self.value);
        }
    }
}

impl Dialed for CurrSource {
    fn set_value(&mut self, v: f64) -> ComponentChange {
        self.value = v.clamp(self.min(), self.max());
        ComponentChange::document("")
    }
    fn value(&self) -> f64 {
        self.value
    }
    fn min(&self) -> f64 {
        self.min_value.min(self.max_value)
    }
    fn max(&self) -> f64 {
        self.min_value.max(self.max_value)
    }
}

impl Drawable for CurrSource {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_var_source_body(
            d,
            ctx.pal,
            self.value,
            self.min_value,
            self.max_value,
            self.running,
            "A",
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let c = CurrSource::default();
        assert_eq!(c.value, DEFAULT_A);
        assert_eq!(c.max_value, DEFAULT_MAX);
        assert!(c.running);
        assert_eq!(c.get_prop_text("Value").unwrap(), format_si(DEFAULT_A, "A"));
    }

    #[test]
    fn stamp_current_into_1s() {
        let src = CurrSource::new(0.01, true);
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        m.add_matrix(0, 0, 1.0);
        src.stamp(&mut m, &[0], 0.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 0.01).abs() < 1e-9, "{}", v[0]);
    }
}
