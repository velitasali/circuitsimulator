//! Diode: semiconductor diode with configurable Shockley model and optional Zener breakdown.

use super::component::{PropGroup, pin_node, stamp_two_terminal};
use super::props::{PropDef, PropError, PropUpdate, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::diode::{
    DIODE_DEFAULT_EM, DIODE_DEFAULT_RS, DIODE_DEFAULT_SAT_NA, DiodeState, ZENER_DEFAULT_BV,
    forward_voltage, sat_current_for_forward_voltage,
};
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.0;
const MAX_V: f64 = 1e6;
const MIN_A: f64 = 1e-18;
const MAX_A: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;
const MIN_EM: f64 = 0.01;
const MAX_EM: f64 = 100.0;

impl crate::canvas::Item {
    pub fn diode(id: impl Into<String>, x: f64, y: f64, zener: bool) -> Self {
        let d = if zener {
            Diode::zener_default()
        } else {
            Diode::default()
        };
        Self::new(id, x, y, d)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn diode_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        zener: bool,
        max_current: f64,
        resistance: f64,
        brkdown_v: f64,
        sat_current: f64,
        em_coef: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Diode {
                zener,
                max_current,
                resistance,
                brkdown_v,
                sat_current,
                em_coef,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diode {
    pub zener: bool,
    pub max_current: f64,
    pub resistance: f64,
    pub brkdown_v: f64,
    pub sat_current: f64,
    pub em_coef: f64,
}

fn diode_sat_current() -> f64 {
    DIODE_DEFAULT_SAT_NA * 1e-9
}

impl Default for Diode {
    fn default() -> Self {
        let sat_current = diode_sat_current();
        Self {
            zener: false,
            max_current: 1.0,
            resistance: DIODE_DEFAULT_RS,
            brkdown_v: 0.0,
            sat_current,
            em_coef: DIODE_DEFAULT_EM,
        }
    }
}

impl Diode {
    pub const TYPE_ID: &'static str = "Diode";
    pub const TYPE_ID_ZENER: &'static str = "Zener";
    pub fn zener_default() -> Self {
        let sat_current = diode_sat_current();
        Self {
            zener: true,
            max_current: 1.0,
            resistance: DIODE_DEFAULT_RS,
            brkdown_v: ZENER_DEFAULT_BV,
            sat_current,
            em_coef: DIODE_DEFAULT_EM,
        }
    }

    /// Forward voltage, SimulIDE `eDiode::threshold` (`m_vCriti`).
    pub fn threshold(&self) -> f64 {
        forward_voltage(self.sat_current, self.em_coef)
    }

    pub fn to_element_kind(&self) -> Kind {
        let mut state = DiodeState::from_model(
            self.sat_current,
            self.em_coef,
            self.brkdown_v,
            self.resistance,
        );
        state.max_current = self.max_current;
        Kind::Diode {
            state,
            zener: self.zener,
        }
    }

    fn get_zener(&self) -> PropValue {
        PropValue::Bool(self.zener)
    }
    fn set_zener(&mut self, v: PropValue) -> Result<(), PropError> {
        self.zener = expect_bool("Zener", v)?;
        Ok(())
    }
    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold())
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        expect_float("Threshold", v)?;
        Ok(())
    }

    fn threshold_sets_sat(diode: &mut Self, v: &PropValue) {
        let PropValue::Float(volts) = v else {
            return;
        };
        let volts = volts.clamp(MIN_V, MAX_V);
        if let Some(sat) = sat_current_for_forward_voltage(volts, diode.em_coef) {
            diode.sat_current = sat.clamp(MIN_A, MAX_A);
        }
    }
    fn get_max_current(&self) -> PropValue {
        PropValue::Float(self.max_current)
    }
    fn set_max_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_current = expect_float("MaxCurrent", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }
    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn get_brkdown_v(&self) -> PropValue {
        PropValue::Float(self.brkdown_v)
    }
    fn set_brkdown_v(&mut self, v: PropValue) -> Result<(), PropError> {
        self.brkdown_v = expect_float("BrkDownV", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
    fn get_sat_current(&self) -> PropValue {
        PropValue::Float(self.sat_current)
    }
    fn set_sat_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.sat_current = expect_float("SatCurrent", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }
    fn get_em_coef(&self) -> PropValue {
        PropValue::Float(self.em_coef)
    }
    fn set_em_coef(&mut self, v: PropValue) -> Result<(), PropError> {
        self.em_coef = expect_float("EmCoef", v)?.clamp(MIN_EM, MAX_EM);
        Ok(())
    }
}

impl Component for Diode {
    fn type_id(&self) -> &'static str {
        if self.zener {
            Self::TYPE_ID_ZENER
        } else {
            Self::TYPE_ID
        }
    }

    fn description(&self) -> &'static str {
        if self.zener {
            "Zener diode."
        } else {
            "Rectifier diode with configurable model."
        }
    }

    fn props() -> &'static [PropDef<Self>] {
        const THRESHOLD_UPDATES: &[PropUpdate<Diode>] =
            &[PropUpdate::always(Diode::threshold_sets_sat)];
        // Computed from saturation current and emission coefficient. Not written or loaded.
        const THRESHOLD: PropDef<Diode> = {
            let mut p = PropDef::float(
                "Threshold",
                "Threshold",
                "V",
                MIN_V,
                MAX_V,
                Diode::get_threshold,
                Diode::set_threshold,
            )
            .updates(THRESHOLD_UPDATES)
            .with_info("Forward voltage, computed from the saturation current and emission coefficient. Editing it changes the saturation current.");
            p.persist = false;
            p
        };
        static PROPS: &[PropDef<Diode>] = &[
            PropDef::bool("Zener", "Zener Diode", Diode::get_zener, Diode::set_zener).with_info(
                "Configure component as a Zener diode instead of a standard rectifier diode.",
            ),
            PropDef::float(
                "MaxCurrent",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                Diode::get_max_current,
                Diode::set_max_current,
            )
            .with_info("Maximum current (it will blink if exceeded)."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Diode::get_resistance,
                Diode::set_resistance,
            )
            .with_info("Series resistance."),
            PropDef::float(
                "BrkDownV",
                "Breakdown Voltage",
                "V",
                MIN_V,
                MAX_V,
                Diode::get_brkdown_v,
                Diode::set_brkdown_v,
            )
            .with_info("Breakdown voltage when reverse biased."),
            PropDef::float(
                "SatCurrent",
                "Saturation Current",
                "A",
                MIN_A,
                MAX_A,
                Diode::get_sat_current,
                Diode::set_sat_current,
            )
            // Eight places: this current is the saved source for forward voltage.
            .with_decimals(8)
            .with_info("Minority charge carriers current when reverse biased."),
            PropDef::float(
                "EmCoef",
                "Emission Coefficient",
                "",
                MIN_EM,
                MAX_EM,
                Diode::get_em_coef,
                Diode::set_em_coef,
            )
            .with_info("Ideality factor."),
            THRESHOLD,
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let mut rows = self.prop_rows();
        for r in &mut rows {
            if r.name == "BrkDownV" {
                r.visible = self.zener;
            }
        }
        super::group_rows_by(
            rows,
            &[
                ("Main", &["Zener"]),
                ("Electric", &["Threshold", "MaxCurrent", "Resistance"]),
                ("Advanced", &["BrkDownV", "SatCurrent", "EmCoef"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-lPin", -16.0, 0.0, 180, 8.0),
            CompPin::new("-rPin", 16.0, 0.0, 0, 9.0),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-10.0, -8.0, 20.0, 16.0)
    }
}

impl TwoTerminal for Diode {}

impl Stampable for Diode {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let mut state = DiodeState::from_model(
            self.sat_current,
            self.em_coef,
            self.brkdown_v,
            self.resistance,
        );
        state.max_current = self.max_current;
        let n = matrix.n();
        let va = pin_node(pin_nodes, 0, n)
            .and_then(|idx| (idx < n).then_some(0.0))
            .unwrap_or(0.0);
        let vk = pin_node(pin_nodes, 1, n)
            .and_then(|idx| (idx < n).then_some(0.0))
            .unwrap_or(0.0);
        state.volt_changed(va, vk);
        let g = state.admit.max(1e-9);
        stamp_two_terminal(matrix, pin_nodes, g, state.i_src());
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Diode {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let fill = ctx.pal.body.fade(COMPONENT_FILL_ALPHA);
        d.fill_poly(&[[7.0, 0.0], [-8.0, -7.0], [-8.0, 7.0]], fill);
        d.stroke_poly(
            &[[7.0, 0.0], [-8.0, -7.0], [-8.0, 7.0]],
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
            true,
        );
        d.line(7.0, -6.0, 7.0, 6.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        if self.zener {
            d.line(7.0, -6.0, 4.0, -6.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(7.0, 6.0, 10.0, 6.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_diode(&mut self, x: f64, y: f64, zener: bool) -> String {
        if zener {
            let id = format!("Zener-{}", self.next_zener);
            self.next_zener += 1;
            self.items.push(crate::canvas::Item::diode(&id, x, y, true));
            id
        } else {
            let id = format!("Diode-{}", self.next_diode);
            self.next_diode += 1;
            self.items
                .push(crate::canvas::Item::diode(&id, x, y, false));
            id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_diode_and_zener() {
        let d = Diode::default();
        assert!(!d.zener);
        assert_eq!(d.type_id(), "Diode");
        let sat = diode_sat_current();
        let v_crit = forward_voltage(sat, DIODE_DEFAULT_EM);
        assert_eq!(d.threshold(), v_crit);
        assert_eq!(d.resistance, DIODE_DEFAULT_RS);
        assert_eq!(d.brkdown_v, 0.0);
        assert_eq!(d.sat_current, sat);
        assert_eq!(d.em_coef, DIODE_DEFAULT_EM);

        let z = Diode::zener_default();
        assert!(z.zener);
        assert_eq!(z.type_id(), "Zener");
        assert_eq!(z.threshold(), v_crit);
        assert_eq!(z.brkdown_v, ZENER_DEFAULT_BV);
        assert_eq!(z.resistance, DIODE_DEFAULT_RS);
        assert_eq!(z.sat_current, sat);
        assert_eq!(z.em_coef, DIODE_DEFAULT_EM);
    }

    #[test]
    fn forward_voltage_follows_saturation_and_emission() {
        let mut d = Diode::default();
        let v0 = d.threshold();
        let sat0 = d.sat_current;

        d.set_prop("SatCurrent", PropValue::Float(1e-9)).unwrap();
        assert!((d.threshold() - v0).abs() > 1e-3);
        assert!((d.sat_current - 1e-9).abs() < 1e-18);

        let v_sat = d.threshold();
        d.set_prop("EmCoef", PropValue::Float(1.5)).unwrap();
        assert!((d.em_coef - 1.5).abs() < 1e-12);
        assert!((d.sat_current - 1e-9).abs() < 1e-18);
        assert!((d.threshold() - v_sat).abs() > 1e-3);

        d.set_prop("Threshold", PropValue::Float(0.8)).unwrap();
        assert!((d.threshold() - 0.8).abs() < 1e-9);
        assert!((d.sat_current - sat0).abs() > 1e-12);
        assert!((d.em_coef - 1.5).abs() < 1e-12);

        let sat = d.sat_current;
        d.set_prop("Threshold", PropValue::Float(0.0)).unwrap();
        assert!((d.sat_current - sat).abs() < 1e-18);
        assert!((d.threshold() - 0.8).abs() < 1e-9);

        d.set_prop("SatCurrent", PropValue::Float(sat0)).unwrap();
        d.set_prop("EmCoef", PropValue::Float(DIODE_DEFAULT_EM))
            .unwrap();
        assert!((d.threshold() - v0).abs() < 1e-9);
    }

    #[test]
    fn stamp_diode_converges() {
        let d = Diode::default();
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        d.stamp(&mut m, &[0, usize::MAX], 0.0);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
    }
}
