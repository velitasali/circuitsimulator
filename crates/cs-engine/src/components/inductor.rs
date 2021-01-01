//! Inductor: backward-Euler companion `G = dt/L`. `InitVolt` is initial current.

use super::component::{stamp_two_terminal, two_terminal_pins};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::{INDUCTOR_DEFAULT_HENRIES, Kind};
use crate::matrix::CircMatrix;

const MIN_L: f64 = 1e-12;
const MAX_L: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_R: f64 = 1e-3;

impl crate::canvas::Item {
    pub fn inductor(id: impl Into<String>, x: f64, y: f64, inductance: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Inductor {
                inductance,
                resistance: 1e-3,
                init_curr: 0.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inductor {
    pub inductance: f64,
    pub resistance: f64,
    pub init_curr: f64,
}

impl Default for Inductor {
    fn default() -> Self {
        Self {
            inductance: INDUCTOR_DEFAULT_HENRIES,
            resistance: DEFAULT_R,
            init_curr: 0.0,
        }
    }
}

impl Inductor {
    pub const TYPE_ID: &'static str = "Inductor";
    pub fn new(inductance: f64) -> Self {
        Self {
            inductance: inductance.max(MIN_L),
            resistance: DEFAULT_R,
            init_curr: 0.0,
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Inductor {
            inductance: self.inductance.max(MIN_L),
            ieq: self.init_curr,
        }
    }

    fn get_inductance(&self) -> PropValue {
        PropValue::Float(self.inductance)
    }
    fn set_inductance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance = expect_float("Inductance", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }
    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn get_init_curr(&self) -> PropValue {
        PropValue::Float(self.init_curr)
    }
    fn set_init_curr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.init_curr = expect_float("InitVolt", v)?;
        Ok(())
    }
}

impl Component for Inductor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Inductor."
    }
    fn props() -> &'static [PropDef<Self>] {
        const IND: PropDef<Inductor> = {
            let mut p = PropDef::float(
                "Inductance",
                "Inductance",
                "H",
                MIN_L,
                MAX_L,
                Inductor::get_inductance,
                Inductor::set_inductance,
            )
            .with_info("Inductance value.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Inductor>] = &[
            IND,
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Inductor::get_resistance,
                Inductor::set_resistance,
            )
            .with_info("Series resistance of the winding."),
            PropDef::float(
                "InitVolt",
                "Initial Current",
                "A",
                -1e6,
                1e6,
                Inductor::get_init_curr,
                Inductor::set_init_curr,
            )
            .with_info("Current at simulation start (initial charge)."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(4.0)
    }
    fn body(&self) -> Rect {
        Rect::new(-12.0, -6.0, 24.0, 12.0)
    }
}

impl TwoTerminal for Inductor {}

impl Stampable for Inductor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let g = dt / self.inductance.max(MIN_L);
        stamp_two_terminal(matrix, pin_nodes, g, self.init_curr);
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Inductor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let deg = std::f64::consts::PI / 180.0;
        d.arc(
            -7.0,
            0.5,
            5.0,
            185.0 * deg,
            405.0 * deg,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.arc(
            0.0,
            0.5,
            5.0,
            135.0 * deg,
            405.0 * deg,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.arc(
            7.0,
            0.5,
            5.0,
            135.0 * deg,
            355.0 * deg,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_inductor(&mut self, x: f64, y: f64, henries: f64) -> String {
        let id = format!("Inductor-{}", self.next_inductor);
        self.next_inductor += 1;
        self.items
            .push(crate::canvas::Item::inductor(&id, x, y, henries));
        id
    }

    pub fn add_default_inductor(&mut self, x: f64, y: f64) -> String {
        self.add_inductor(x, y, crate::elements::INDUCTOR_DEFAULT_HENRIES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let l = Inductor::default();
        assert_eq!(l.inductance, INDUCTOR_DEFAULT_HENRIES);
        assert_eq!(
            l.get_prop_text("Inductance").unwrap(),
            format_si(INDUCTOR_DEFAULT_HENRIES, "H")
        );
        assert_eq!(l.get_prop_text("InitVolt").unwrap(), "0 A");
    }

    #[test]
    fn stamp_companion_init_curr() {
        let mut l = Inductor::new(1.0);
        l.init_curr = 0.01;
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        let dt = 1e-3;
        l.stamp(&mut m, &[0, usize::MAX], dt);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        // G = dt/L = 1e-3, Ieq = 0.01 → V = Ieq/G = 10.
        assert!((v[0] - 10.0).abs() < 1e-6, "{}", v[0]);
    }
}
