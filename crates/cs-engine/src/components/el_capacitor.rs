//! Electrolytic capacitor: same companion stamp as [`super::Capacitor`].

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
    pub fn el_capacitor(id: impl Into<String>, x: f64, y: f64, capacitance: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            ElCapacitor {
                capacitance,
                resistance: 1e-3,
                init_volt: 0.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElCapacitor {
    pub capacitance: f64,
    pub resistance: f64,
    pub init_volt: f64,
}

impl Default for ElCapacitor {
    fn default() -> Self {
        Self {
            capacitance: CAPACITOR_DEFAULT_FARADS,
            resistance: DEFAULT_ESR,
            init_volt: 0.0,
        }
    }
}

impl ElCapacitor {
    pub const TYPE_ID: &'static str = "ElCapacitor";
    pub fn new(capacitance: f64) -> Self {
        Self {
            capacitance: capacitance.max(MIN_C),
            resistance: DEFAULT_ESR,
            init_volt: 0.0,
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::ElCapacitor {
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

impl Component for ElCapacitor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Electrolytic capacitor."
    }
    fn props() -> &'static [PropDef<Self>] {
        const CAP: PropDef<ElCapacitor> = {
            let mut p = PropDef::float(
                "Capacitance",
                "Capacitance",
                "F",
                MIN_C,
                MAX_C,
                ElCapacitor::get_capacitance,
                ElCapacitor::set_capacitance,
            )
            .with_info("Capacitance value.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<ElCapacitor>] = &[
            CAP,
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                ElCapacitor::get_resistance,
                ElCapacitor::set_resistance,
            )
            .with_info("Series resistance."),
            PropDef::float(
                "InitVolt",
                "Initial Voltage",
                "V",
                -1e6,
                1e6,
                ElCapacitor::get_init_volt,
                ElCapacitor::set_init_volt,
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

impl TwoTerminal for ElCapacitor {}

impl Stampable for ElCapacitor {
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
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for ElCapacitor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_rect(-3.5, -6.0, 1.0, 12.0, ctx.pal.border);
        d.fill_rect(2.5, -6.0, 1.0, 12.0, ctx.pal.border);
        d.line(
            -9.5,
            -6.0,
            -4.5,
            -6.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(
            -7.0,
            -8.5,
            -7.0,
            -3.5,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_el_capacitor(&mut self, x: f64, y: f64, farads: f64) -> String {
        let id = format!("ElCapacitor-{}", self.next_el_capacitor);
        self.next_el_capacitor += 1;
        self.items
            .push(crate::canvas::Item::el_capacitor(&id, x, y, farads));
        id
    }

    pub fn add_default_el_capacitor(&mut self, x: f64, y: f64) -> String {
        self.add_el_capacitor(x, y, crate::elements::CAPACITOR_DEFAULT_FARADS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_itemtype() {
        let c = ElCapacitor::default();
        assert_eq!(c.type_id(), "ElCapacitor");
        assert_eq!(c.description(), "Electrolytic capacitor.");
    }
}
