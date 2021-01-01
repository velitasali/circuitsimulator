//! Fixed voltage source: one-terminal Norton source to ground.

use super::component::stamp_to_ground;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::{FIXED_VOLT_DEFAULT, Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;

const MIN_V: f64 = -1e6;
const MAX_V: f64 = 1e6;

/// Fixed Voltage (large) `m_area = QRect(-8, -8, 16, 16)`.
pub const FIXED_VOLT_BODY: Rect = Rect {
    x: -8.0,
    y: -8.0,
    w: 16.0,
    h: 16.0,
};

impl crate::canvas::Item {
    pub fn fixed_volt(id: impl Into<String>, x: f64, y: f64, voltage: f64) -> Self {
        Self::fixed_volt_with(id, x, y, voltage, false)
    }

    pub fn fixed_volt_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        voltage: f64,
        small: bool,
    ) -> Self {
        Self::new(id, x, y, FixedVolt { voltage, small })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedVolt {
    pub voltage: f64,
    pub small: bool,
}

impl Default for FixedVolt {
    fn default() -> Self {
        Self {
            voltage: FIXED_VOLT_DEFAULT,
            small: false,
        }
    }
}

impl FixedVolt {
    pub const TYPE_ID: &'static str = "FixedVolt";
    pub fn new(voltage: f64, small: bool) -> Self {
        Self { voltage, small }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::FixedVolt {
            voltage: self.voltage,
        }
    }

    fn get_voltage(&self) -> PropValue {
        PropValue::Float(self.voltage)
    }
    fn set_voltage(&mut self, v: PropValue) -> Result<(), PropError> {
        self.voltage = expect_float("Voltage", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
    fn get_small(&self) -> PropValue {
        PropValue::Bool(self.small)
    }
    fn set_small(&mut self, v: PropValue) -> Result<(), PropError> {
        self.small = expect_bool("Small", v)?;
        Ok(())
    }
}

impl Component for FixedVolt {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Fixed voltage source."
    }
    fn props() -> &'static [PropDef<Self>] {
        const VOLT: PropDef<FixedVolt> = {
            let mut p = PropDef::float(
                "Voltage",
                "Voltage",
                "V",
                MIN_V,
                MAX_V,
                FixedVolt::get_voltage,
                FixedVolt::set_voltage,
            )
            .with_info("Output voltage.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<FixedVolt>] = &[
            VOLT,
            PropDef::bool(
                "Small",
                "Small size",
                FixedVolt::get_small,
                FixedVolt::set_small,
            )
            .with_info("Use a smaller body."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        let plen = if self.small { 12.0 } else { 8.0 };
        vec![CompPin::new("-outnod", 16.0, 0.0, 0, plen).with_direction(PinDirection::Out)]
    }
    fn body(&self) -> Rect {
        if self.small {
            Rect::new(-4.0, -4.0, 8.0, 8.0)
        } else {
            FIXED_VOLT_BODY
        }
    }
}

impl Stampable for FixedVolt {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_to_ground(matrix, pin_nodes, 0, self.voltage, SOURCE_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for FixedVolt {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let r = if self.small { 4.0 } else { 8.0 };
        d.fill_circle(0.0, 0.0, r, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_circle(0.0, 0.0, r, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        let s = r * 0.45;
        d.line(-s, 0.0, s, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(0.0, -s, 0.0, s, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_fixed_volt(&mut self, x: f64, y: f64, voltage: f64) -> String {
        let id = format!("Fixed Voltage-{}", self.next_fixed);
        self.next_fixed += 1;
        self.items
            .push(crate::canvas::Item::fixed_volt(&id, x, y, voltage));
        id
    }

    pub fn add_default_fixed_volt(&mut self, x: f64, y: f64) -> String {
        self.add_fixed_volt(x, y, crate::elements::FIXED_VOLT_DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let v = FixedVolt::default();
        assert_eq!(v.voltage, FIXED_VOLT_DEFAULT);
        assert!(!v.small);
        assert_eq!(
            v.get_prop_text("Voltage").unwrap(),
            format_si(FIXED_VOLT_DEFAULT, "V")
        );
        assert_eq!(v.type_id(), "FixedVolt");
    }

    #[test]
    fn small_shrinks_body() {
        let mut v = FixedVolt::default();
        let large = v.body();
        v.set_prop_text("Small", "true").unwrap();
        let small = v.body();
        assert!(small.w < large.w && small.h < large.h);
    }

    #[test]
    fn stamp_5v() {
        let v = FixedVolt::new(5.0, false);
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        v.stamp(&mut m, &[0], 0.0);
        let mut volts = vec![0.0];
        assert!(m.solve(&mut volts));
        assert!((volts[0] - 5.0).abs() < 1e-6, "{}", volts[0]);
    }
}
