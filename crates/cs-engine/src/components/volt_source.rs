//! Adjustable voltage source with running flag and a dial on Value.

use super::component::stamp_to_ground;
use super::drawable::{Drawable, paint_var_source_body};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, ComponentChange, Dialed, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::{Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;

const MIN_V: f64 = -1e6;
const MAX_V: f64 = 1e6;
const DEFAULT_V: f64 = 5.0;

#[derive(Clone, Debug, PartialEq)]
pub struct VoltSource {
    pub value: f64,
    pub max_value: f64,
    pub min_value: f64,
    pub running: bool,
}

impl crate::canvas::Item {
    pub fn volt_source(id: impl Into<String>, x: f64, y: f64, value: f64, running: bool) -> Self {
        Self::volt_source_with(id, x, y, 5.0, 0.0, value, running)
    }

    pub fn volt_source_with(
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
            VoltSource {
                max_value,
                min_value,
                value,
                running,
            },
        )
    }
}

impl Default for VoltSource {
    fn default() -> Self {
        Self {
            value: DEFAULT_V,
            max_value: DEFAULT_V,
            min_value: 0.0,
            running: true,
        }
    }
}

impl VoltSource {
    pub const TYPE_ID: &'static str = "VoltSource";
    pub fn new(value: f64, running: bool) -> Self {
        Self {
            value,
            max_value: DEFAULT_V,
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
        Kind::VoltSource {
            value: self.value,
            running: self.running,
        }
    }

    fn get_value(&self) -> PropValue {
        PropValue::Float(self.value)
    }
    fn set_value_prop(&mut self, v: PropValue) -> Result<(), PropError> {
        self.value = expect_float("Value", v)?.clamp(MIN_V, MAX_V);
        self.clamp_value();
        Ok(())
    }
    fn get_max(&self) -> PropValue {
        PropValue::Float(self.max_value)
    }
    fn set_max(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_value = expect_float("MaxValue", v)?.clamp(MIN_V, MAX_V);
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
        self.min_value = expect_float("MinValue", v)?.clamp(MIN_V, MAX_V);
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

impl Component for VoltSource {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Adjustable voltage source."
    }
    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<VoltSource>] = &[
            PropDef::float(
                "Value",
                "Voltage",
                "V",
                MIN_V,
                MAX_V,
                VoltSource::get_value,
                VoltSource::set_value_prop,
            )
            .with_info("Output value."),
            PropDef::float(
                "MaxValue",
                "Max Voltage",
                "V",
                MIN_V,
                MAX_V,
                VoltSource::get_max,
                VoltSource::set_max,
            )
            .with_info("Maximum voltage (positive or negative value).\nMust be > Minimum Voltage."),
            PropDef::float(
                "MinValue",
                "Min Voltage",
                "V",
                MIN_V,
                MAX_V,
                VoltSource::get_min,
                VoltSource::set_min,
            )
            .with_info("Minimum voltage (positive or negative value).\nMust be < Maximum Voltage."),
            PropDef::bool(
                "Running",
                "Running",
                VoltSource::get_running,
                VoltSource::set_running,
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
    pub fn add_volt_source(&mut self, x: f64, y: f64) -> String {
        let id = format!("VoltSource-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::volt_source(&id, x, y, 5.0, true));
        id
    }
}

impl Stampable for VoltSource {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let v = if self.running { self.value } else { 0.0 };
        stamp_to_ground(matrix, pin_nodes, 0, v, SOURCE_ADMIT);
    }
}

impl Dialed for VoltSource {
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

impl Drawable for VoltSource {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_var_source_body(
            d,
            ctx.pal,
            self.value,
            self.min_value,
            self.max_value,
            self.running,
            "V",
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
        let v = VoltSource::default();
        assert_eq!(v.value, DEFAULT_V);
        assert!(v.running);
        assert_eq!(v.get_prop_text("Value").unwrap(), format_si(DEFAULT_V, "V"));
        assert_eq!(v.get_prop_text("Running").unwrap(), "true");
    }

    #[test]
    fn dial_writes_value() {
        let mut v = VoltSource::default();
        let c = v.set_value(2.5);
        assert!(c.saved && c.sim);
        assert_eq!(v.get_prop_text("Value").unwrap(), "2.5 V");
    }

    #[test]
    fn stopped_stamps_zero() {
        let mut src = VoltSource::new(5.0, false);
        src.running = false;
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        src.stamp(&mut m, &[0], 0.0);
        let mut volts = vec![1.0];
        assert!(m.solve(&mut volts));
        assert!(volts[0].abs() < 1e-9, "{}", volts[0]);
    }
}
