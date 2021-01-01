//! Battery: two-terminal Norton voltage source with series resistance.

use super::component::{clamp_positive, resistor_g, stamp_two_terminal};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::{BATTERY_DEFAULT_OHMS, BATTERY_DEFAULT_VOLTS, Kind};
use crate::matrix::CircMatrix;

const MIN_V: f64 = 1e-12;
const MAX_V: f64 = 1e6;
const MIN_OHMS: f64 = 1e-14;
const MAX_OHMS: f64 = 1e12;

/// Battery constructor `m_area = QRect(-10, -10, 20, 20)`.
pub const BATTERY_BODY: Rect = Rect {
    x: -10.0,
    y: -10.0,
    w: 20.0,
    h: 20.0,
};

impl crate::canvas::Item {
    pub fn battery(id: impl Into<String>, x: f64, y: f64, voltage: f64, resistance: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Battery {
                voltage,
                resistance,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Battery {
    pub voltage: f64,
    pub resistance: f64,
}

impl Default for Battery {
    fn default() -> Self {
        Self {
            voltage: BATTERY_DEFAULT_VOLTS,
            resistance: BATTERY_DEFAULT_OHMS,
        }
    }
}

impl Battery {
    pub const TYPE_ID: &'static str = "Battery";
    pub fn new(voltage: f64, resistance: f64) -> Self {
        Self {
            voltage: voltage.max(MIN_V),
            resistance: clamp_positive(resistance, MIN_OHMS),
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Battery {
            voltage: self.voltage.max(MIN_V),
            resistance: self.resistance.max(MIN_OHMS),
        }
    }

    fn get_voltage(&self) -> PropValue {
        PropValue::Float(self.voltage)
    }
    fn set_voltage(&mut self, v: PropValue) -> Result<(), PropError> {
        self.voltage = expect_float("Voltage", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
}

impl Component for Battery {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Simple battery."
    }
    fn props() -> &'static [PropDef<Self>] {
        const VOLT: PropDef<Battery> = {
            let mut p = PropDef::float(
                "Voltage",
                "Voltage",
                "V",
                MIN_V,
                MAX_V,
                Battery::get_voltage,
                Battery::set_voltage,
            )
            .with_info("Battery voltage.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Battery>] = &[
            VOLT,
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Battery::get_resistance,
                Battery::set_resistance,
            )
            .with_info("Internal resistance."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-lPin", -16.0, 0.0, 180, 9.5),
            CompPin::new("-rPin", 16.0, 0.0, 0, 8.5),
        ]
    }
    fn body(&self) -> Rect {
        BATTERY_BODY
    }
}

impl TwoTerminal for Battery {}

impl Stampable for Battery {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let g = resistor_g(self.resistance.max(MIN_OHMS));
        stamp_two_terminal(matrix, pin_nodes, g, self.voltage * g);
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Battery {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_rect(-7.5, -8.0, COMPONENT_BORDER_WIDTH, 16.0, ctx.pal.border);
        d.fill_rect(-2.5, -3.0, COMPONENT_BORDER_WIDTH, 6.0, ctx.pal.border);
        d.fill_rect(2.5, -8.0, COMPONENT_BORDER_WIDTH, 16.0, ctx.pal.border);
        d.fill_rect(7.5, -3.0, COMPONENT_BORDER_WIDTH, 6.0, ctx.pal.border);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_battery(&mut self, x: f64, y: f64, voltage: f64, resistance: f64) -> String {
        let id = format!("Battery-{}", self.next_battery);
        self.next_battery += 1;
        self.items
            .push(crate::canvas::Item::battery(&id, x, y, voltage, resistance));
        id
    }

    pub fn add_default_battery(&mut self, x: f64, y: f64) -> String {
        self.add_battery(
            x,
            y,
            crate::elements::BATTERY_DEFAULT_VOLTS,
            crate::elements::BATTERY_DEFAULT_OHMS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let b = Battery::default();
        assert_eq!(b.voltage, BATTERY_DEFAULT_VOLTS);
        assert_eq!(b.resistance, BATTERY_DEFAULT_OHMS);
        assert_eq!(
            b.get_prop_text("Voltage").unwrap(),
            format_si(BATTERY_DEFAULT_VOLTS, "V")
        );
        assert_eq!(
            b.get_prop_text("Resistance").unwrap(),
            format_si(BATTERY_DEFAULT_OHMS, "Ω")
        );
    }

    #[test]
    fn stamp_5v_to_ground() {
        let b = Battery::new(5.0, 1e-3);
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        b.stamp(&mut m, &[0, usize::MAX], 0.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 5.0).abs() < 1e-6, "{}", v[0]);
    }
}
