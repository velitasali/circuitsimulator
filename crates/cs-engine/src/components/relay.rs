//! Electromechanical relay: coil conductance plus contact stamp when active.

use super::component::{PropGroup, resistor_g, stamp_conductance_between};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::{Kind, SWITCH_CLOSED_ADMIT};
use crate::matrix::CircMatrix;

const MIN_POLES: i64 = 1;
const MAX_POLES: i64 = 8;
const MIN_A: f64 = 0.0;
const MAX_A: f64 = 1e3;
const MIN_L: f64 = 1e-12;
const MAX_L: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const DEFAULT_ION: f64 = 0.02;
const DEFAULT_IOFF: f64 = 0.01;
const DEFAULT_L: f64 = 1e-3;
const DEFAULT_R: f64 = 100.0;

impl crate::canvas::Item {
    #[allow(clippy::too_many_arguments)]
    pub fn relay(
        id: impl Into<String>,
        x: f64,
        y: f64,
        norm_close: bool,
        double_throw: bool,
        poles: usize,
        i_on: f64,
        i_off: f64,
        active: bool,
    ) -> Self {
        let _ = active;
        Self::relay_with(
            id,
            x,
            y,
            norm_close,
            double_throw,
            poles,
            i_on,
            i_off,
            1e-3,
            100.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn relay_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        norm_close: bool,
        double_throw: bool,
        poles: usize,
        i_on: f64,
        i_off: f64,
        inductance: f64,
        r_coil: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Relay {
                norm_close,
                double_throw,
                poles,
                i_on,
                i_off,
                inductance,
                r_coil,
                active: false,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Relay {
    pub norm_close: bool,
    pub double_throw: bool,
    pub poles: usize,
    pub i_on: f64,
    pub i_off: f64,
    pub inductance: f64,
    pub r_coil: f64,
    pub active: bool,
}

impl Default for Relay {
    fn default() -> Self {
        Self {
            norm_close: false,
            double_throw: false,
            poles: 1,
            i_on: DEFAULT_ION,
            i_off: DEFAULT_IOFF,
            inductance: DEFAULT_L,
            r_coil: DEFAULT_R,
            active: false,
        }
    }
}

impl Relay {
    pub const TYPE_ID: &'static str = "Relay";
    pub fn new(poles: usize, i_on: f64, i_off: f64) -> Self {
        Self {
            poles: poles.clamp(1, MAX_POLES as usize),
            i_on,
            i_off,
            ..Self::default()
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Relay {
            norm_close: self.norm_close,
            double_throw: self.double_throw,
            poles: self.poles.max(1),
            i_on: self.i_on,
            i_off: self.i_off,
            active: self.active,
        }
    }

    fn get_norm_close(&self) -> PropValue {
        PropValue::Bool(self.norm_close)
    }
    fn set_norm_close(&mut self, v: PropValue) -> Result<(), PropError> {
        self.norm_close = expect_bool("NormClose", v)?;
        Ok(())
    }
    fn get_double_throw(&self) -> PropValue {
        PropValue::Bool(self.double_throw)
    }
    fn set_double_throw(&mut self, v: PropValue) -> Result<(), PropError> {
        self.double_throw = expect_bool("DoubleThrow", v)?;
        Ok(())
    }
    fn get_poles(&self) -> PropValue {
        PropValue::Int(self.poles as i64)
    }
    fn set_poles(&mut self, v: PropValue) -> Result<(), PropError> {
        self.poles = expect_int("Poles", v)?.clamp(MIN_POLES, MAX_POLES) as usize;
        Ok(())
    }
    fn get_ion(&self) -> PropValue {
        PropValue::Float(self.i_on)
    }
    fn set_ion(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_on = expect_float("IOn", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }
    fn get_ioff(&self) -> PropValue {
        PropValue::Float(self.i_off)
    }
    fn set_ioff(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_off = expect_float("IOff", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }
    fn get_inductance(&self) -> PropValue {
        PropValue::Float(self.inductance)
    }
    fn set_inductance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance = expect_float("Inductance", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }
    fn get_rcoil(&self) -> PropValue {
        PropValue::Float(self.r_coil)
    }
    fn set_rcoil(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_coil = expect_float("Rcoil", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
}

impl Component for Relay {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Electromechanical relay."
    }
    fn props() -> &'static [PropDef<Self>] {
        const POLES: PropDef<Relay> = {
            let mut p = PropDef::int(
                "Poles",
                "Poles",
                MIN_POLES,
                MAX_POLES,
                Relay::get_poles,
                Relay::set_poles,
            )
            .with_info("Number of poles controlled by this relay.");
            p.structural = true;
            p
        };
        const DT: PropDef<Relay> = {
            let mut p = PropDef::bool(
                "DoubleThrow",
                "Double Throw",
                Relay::get_double_throw,
                Relay::set_double_throw,
            )
            .with_info("Yes: 2 throws per pole (SPDT/DPDT). No: 1 throw per pole (SPST/DPST).");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Relay>] = &[
            PropDef::bool(
                "NormClose",
                "Normally Closed",
                Relay::get_norm_close,
                Relay::set_norm_close,
            )
            .with_info("State with relay not active."),
            DT,
            POLES,
            PropDef::float(
                "IOn",
                "Turn-on Current",
                "A",
                MIN_A,
                MAX_A,
                Relay::get_ion,
                Relay::set_ion,
            )
            .with_info("Minimum current that activates the relay."),
            PropDef::float(
                "IOff",
                "Holding Current",
                "A",
                MIN_A,
                MAX_A,
                Relay::get_ioff,
                Relay::set_ioff,
            )
            .with_info("Minimum current that holds the relay active."),
            PropDef::float(
                "Inductance",
                "Inductance",
                "H",
                MIN_L,
                MAX_L,
                Relay::get_inductance,
                Relay::set_inductance,
            )
            .with_info("Coil inductance."),
            PropDef::float(
                "Rcoil",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Relay::get_rcoil,
                Relay::set_rcoil,
            )
            .with_info("Coil resistance."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["NormClose", "DoubleThrow", "Poles"]),
                ("Electric", &["IOn", "IOff"]),
                ("Coil", &["Inductance", "Rcoil"]),
            ],
        )
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-lPin", -16.0, 0.0, 180, 4.0),
            CompPin::new("-rPin", 16.0, 0.0, 0, 4.0),
            CompPin::new("-c1", -16.0, -16.0, 180, 4.0),
            CompPin::new("-c2", 16.0, -16.0, 0, 4.0),
        ]
    }
    fn body(&self) -> Rect {
        let p = self.poles.max(1) as f64;
        let h = 20.0 + 16.0 * p;
        let y = -12.0 - 16.0 * p;
        Rect::new(-12.0, y, 24.0, h)
    }
}

impl Stampable for Relay {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_conductance_between(matrix, pin_nodes, 0, 1, resistor_g(self.r_coil));
        if self.active {
            stamp_conductance_between(matrix, pin_nodes, 2, 3, SWITCH_CLOSED_ADMIT);
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Relay {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let p = self.poles as f64;
        let h = 20.0 + 16.0 * p;
        let y = -12.0 - 16.0 * p;

        d.fill_round_rect(-12.0, y, 24.0, h, 2.0, ctx.pal.body.fade(0.2));
        d.stroke_round_rect(
            -12.0,
            y,
            24.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Switch contacts for each pole
        for i in 0..self.poles {
            let sy = -16.0 * ((i + 1) as f64);
            d.line(-12.0, sy, -4.0, sy, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            if self.active {
                d.line(-4.0, sy, 6.0, sy, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            } else {
                d.line(
                    -4.0,
                    sy,
                    6.0,
                    sy - 6.0,
                    ctx.pal.border,
                    COMPONENT_BORDER_WIDTH,
                );
            }
            d.line(6.0, sy, 12.0, sy, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }

        // Inductor coil leads at y = 0
        d.line(
            -12.0,
            0.0,
            -6.0,
            0.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(6.0, 0.0, 12.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);

        // Dashed coupling
        d.line(0.0, -12.0, 0.0, -4.0, ctx.pal.border.fade(0.6), 1.0);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_relay(&mut self, x: f64, y: f64) -> String {
        let id = format!("Relay-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::relay(
            &id, x, y, false, false, 1, 0.02, 0.005, false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let r = Relay::default();
        assert_eq!(r.poles, 1);
        assert!(!r.active);
        assert_eq!(r.get_prop_text("IOn").unwrap(), format_si(DEFAULT_ION, "A"));
        assert_eq!(
            r.get_prop_text("IOff").unwrap(),
            format_si(DEFAULT_IOFF, "A")
        );
        assert_eq!(r.get_prop_text("Rcoil").unwrap(), format_si(DEFAULT_R, "Ω"));
        assert_eq!(r.get_prop_text("NormClose").unwrap(), "false");
    }

    #[test]
    fn coil_stamps_100_ohm() {
        let r = Relay::default();
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        r.stamp(&mut m, &[0, usize::MAX, usize::MAX, usize::MAX], 0.0);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 100.0).abs() < 1e-6, "{}", v[0]);
    }
}
