//! Power rail: one-terminal Norton voltage source to ground.

use super::component::stamp_to_ground;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::{FIXED_VOLT_DEFAULT, Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;

const MIN_V: f64 = -1e6;
const MAX_V: f64 = 1e6;

#[derive(Clone, Debug, PartialEq)]
pub struct Rail {
    pub voltage: f64,
}

impl crate::canvas::Item {
    pub fn rail(id: impl Into<String>, x: f64, y: f64, voltage: f64) -> Self {
        Self::new(id, x, y, Rail { voltage })
    }
}

impl Default for Rail {
    fn default() -> Self {
        Self {
            voltage: FIXED_VOLT_DEFAULT,
        }
    }
}

impl Rail {
    pub const TYPE_ID: &'static str = "Rail";
    pub fn new(voltage: f64) -> Self {
        Self { voltage }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Rail {
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
}

impl Component for Rail {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Power supply rail."
    }
    fn props() -> &'static [PropDef<Self>] {
        const VOLT: PropDef<Rail> = {
            let mut p = PropDef::float(
                "Voltage",
                "Voltage",
                "V",
                MIN_V,
                MAX_V,
                Rail::get_voltage,
                Rail::set_voltage,
            )
            .with_info("Rail voltage.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Rail>] = &[VOLT];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![CompPin::new("-outnod", 16.0, 0.0, 0, 7.0).with_direction(PinDirection::Out)]
    }
    fn body(&self) -> Rect {
        Rect::new(-2.0, -7.0, 12.0, 14.0)
    }
}

impl Stampable for Rail {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_to_ground(matrix, pin_nodes, 0, self.voltage, SOURCE_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Rail {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let pts = [[-1.5, -6.5], [-1.5, 6.5], [9.0, 1.0], [9.0, -1.0]];
        d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_rail(&mut self, x: f64, y: f64) -> String {
        let id = format!("Rail-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::rail(&id, x, y, 5.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let r = Rail::default();
        assert_eq!(r.voltage, FIXED_VOLT_DEFAULT);
        assert_eq!(
            r.get_prop_text("Voltage").unwrap(),
            format_si(FIXED_VOLT_DEFAULT, "V")
        );
    }

    #[test]
    fn stamp_12v() {
        let r = Rail::new(12.0);
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        r.stamp(&mut m, &[0], 0.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 12.0).abs() < 1e-6, "{}", v[0]);
    }
}
