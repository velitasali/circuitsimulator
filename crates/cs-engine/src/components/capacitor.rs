//! Capacitor: backward-Euler companion `G = C/dt`. ESR is persisted, not stamped.

use super::component::{stamp_two_terminal, two_terminal_pins};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::{CAPACITOR_DEFAULT_FARADS, Kind};
use crate::matrix::CircMatrix;

const MIN_C: f64 = 1e-15;
const MAX_C: f64 = 1e3;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_ESR: f64 = 1e-3;

impl crate::canvas::Item {
    pub fn capacitor(id: impl Into<String>, x: f64, y: f64, capacitance: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Capacitor {
                capacitance,
                resistance: 1e-3,
                init_volt: 0.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Capacitor {
    pub capacitance: f64,
    pub resistance: f64,
    pub init_volt: f64,
}

impl Default for Capacitor {
    fn default() -> Self {
        Self {
            capacitance: CAPACITOR_DEFAULT_FARADS,
            resistance: DEFAULT_ESR,
            init_volt: 0.0,
        }
    }
}

impl Capacitor {
    pub const TYPE_ID: &'static str = "Capacitor";
    pub fn new(capacitance: f64) -> Self {
        Self {
            capacitance: capacitance.max(MIN_C),
            resistance: DEFAULT_ESR,
            init_volt: 0.0,
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Capacitor {
            capacitance: self.capacitance.max(MIN_C),
            volt: self.init_volt,
        }
    }

    fn get_capacitance(&self) -> PropValue {
        PropValue::Float(self.capacitance)
    }
    fn set_capacitance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.capacitance = expect_float("Capacitance", v)?.clamp(MIN_C, MAX_C);
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
}

impl Component for Capacitor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Capacitor."
    }
    fn props() -> &'static [PropDef<Self>] {
        const CAP: PropDef<Capacitor> = {
            let mut p = PropDef::float(
                "Capacitance",
                "Capacitance",
                "F",
                MIN_C,
                MAX_C,
                Capacitor::get_capacitance,
                Capacitor::set_capacitance,
            )
            .with_info("Capacitance value.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Capacitor>] = &[
            CAP,
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Capacitor::get_resistance,
                Capacitor::set_resistance,
            )
            .with_info("Series resistance."),
            PropDef::float(
                "InitVolt",
                "Initial Voltage",
                "V",
                -1e6,
                1e6,
                Capacitor::get_init_volt,
                Capacitor::set_init_volt,
            )
            .with_info("Voltage at simulation start (initial charge)."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(13.5)
    }
    fn body(&self) -> Rect {
        Rect::new(-10.0, -8.0, 20.0, 16.0)
    }
}

impl TwoTerminal for Capacitor {}

impl Stampable for Capacitor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let g = self.capacitance.max(MIN_C) / dt;
        stamp_two_terminal(matrix, pin_nodes, g, self.init_volt * g);
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};

impl Drawable for Capacitor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_rect(-3.5, -6.0, 1.0, 12.0, ctx.pal.border);
        d.fill_rect(2.5, -6.0, 1.0, 12.0, ctx.pal.border);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_capacitor(&mut self, x: f64, y: f64, farads: f64) -> String {
        let id = format!("Capacitor-{}", self.next_capacitor);
        self.next_capacitor += 1;
        self.items
            .push(crate::canvas::Item::capacitor(&id, x, y, farads));
        id
    }

    pub fn add_default_capacitor(&mut self, x: f64, y: f64) -> String {
        self.add_capacitor(x, y, crate::elements::CAPACITOR_DEFAULT_FARADS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let c = Capacitor::default();
        assert_eq!(c.capacitance, CAPACITOR_DEFAULT_FARADS);
        assert_eq!(c.resistance, DEFAULT_ESR);
        assert_eq!(c.init_volt, 0.0);
        assert_eq!(
            c.get_prop_text("Capacitance").unwrap(),
            format_si(CAPACITOR_DEFAULT_FARADS, "F")
        );
    }

    #[test]
    fn stamp_companion_init_volt() {
        let mut c = Capacitor::new(10e-6);
        c.init_volt = 1.0;
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        c.stamp(&mut m, &[0, usize::MAX], 1e-6);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 1.0).abs() < 1e-6, "{}", v[0]);
    }
}
