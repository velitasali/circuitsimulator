//! BJT: bipolar junction transistor (NPN / PNP) with Ebers-Moll companion model.

use super::component::{stamp_current, stamp_directed};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::transistor::{BJT_DEFAULT_GAIN, BjtState};
use crate::matrix::CircMatrix;

const MIN_GAIN: f64 = 1e-6;
const MAX_GAIN: f64 = 1e6;
const MIN_VCRIT: f64 = 0.0;
const MAX_VCRIT: f64 = 10.0;

impl crate::canvas::Item {
    pub fn bjt(id: impl Into<String>, x: f64, y: f64, pnp: bool) -> Self {
        Self::bjt_with(id, x, y, pnp, BJT_DEFAULT_GAIN, 0.7)
    }

    pub fn bjt_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        pnp: bool,
        gain: f64,
        threshold: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Bjt {
                pnp,
                gain,
                threshold,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bjt {
    pub pnp: bool,
    pub gain: f64,
    pub threshold: f64,
}

impl Default for Bjt {
    fn default() -> Self {
        Self {
            pnp: false,
            gain: BJT_DEFAULT_GAIN,
            threshold: 0.7,
        }
    }
}

impl Bjt {
    pub const TYPE_ID: &'static str = "Bjt";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = BjtState::new(self.pnp);
        state.set_gain(self.gain);
        state.set_threshold(self.threshold);
        Kind::Bjt { state }
    }

    fn get_pnp(&self) -> PropValue {
        PropValue::Bool(self.pnp)
    }
    fn set_pnp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.pnp = expect_bool("PNP", v)?;
        Ok(())
    }
    fn get_gain(&self) -> PropValue {
        PropValue::Float(self.gain)
    }
    fn set_gain(&mut self, v: PropValue) -> Result<(), PropError> {
        self.gain = expect_float("Gain", v)?.clamp(MIN_GAIN, MAX_GAIN);
        Ok(())
    }
    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold)
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        self.threshold = expect_float("Vcrit", v)?.clamp(MIN_VCRIT, MAX_VCRIT);
        Ok(())
    }
}

impl Component for Bjt {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Bipolar junction transistor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Bjt>] = &[
            PropDef::bool("PNP", "PNP", Bjt::get_pnp, Bjt::set_pnp).with_info("PNP or NPN."),
            PropDef::float(
                "Gain",
                "Gain",
                "",
                MIN_GAIN,
                MAX_GAIN,
                Bjt::get_gain,
                Bjt::set_gain,
            )
            .with_info("Current gain."),
            PropDef::float(
                "Vcrit",
                "Threshold",
                "V",
                MIN_VCRIT,
                MAX_VCRIT,
                Bjt::get_threshold,
                Bjt::set_threshold,
            )
            .with_info("Base-Emitter diode threshold."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-collector", 8.0, -16.0, 90, 4.0),
            CompPin::new("-emiter", 8.0, 16.0, 270, 4.0),
            CompPin::new("-base", -16.0, 0.0, 180, 4.0),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-12.0, -14.0, 28.0, 28.0)
    }
}

impl Stampable for Bjt {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let mut state = BjtState::new(self.pnp);
        state.set_gain(self.gain);
        state.set_threshold(self.threshold);
        // pin_nodes: 0 = collector, 1 = emitter, 2 = base
        stamp_directed(matrix, pin_nodes, 2, 0, -state.gec - state.gcc);
        stamp_directed(matrix, pin_nodes, 0, 2, -state.gce - state.gcc);
        stamp_directed(matrix, pin_nodes, 2, 1, -state.gee - state.gce);
        stamp_directed(matrix, pin_nodes, 1, 2, -state.gee - state.gec);
        stamp_directed(matrix, pin_nodes, 0, 1, state.gce);
        stamp_directed(matrix, pin_nodes, 1, 0, state.gec);
        stamp_current(matrix, pin_nodes, 2, state.i_base);
        stamp_current(matrix, pin_nodes, 0, state.i_coll);
        stamp_current(matrix, pin_nodes, 1, state.i_emit);
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Bjt {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_circle(2.0, 0.0, 14.0, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_circle(2.0, 0.0, 14.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(
            -12.0,
            0.0,
            -4.0,
            0.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(
            -4.0,
            -8.0,
            -4.0,
            8.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(
            -4.0,
            -4.0,
            8.0,
            -12.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        if self.pnp {
            d.line(-4.0, 4.0, -0.4, 6.4, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.fill_poly(&[[-0.4, 6.4], [0.89, 9.66], [3.11, 6.34]], ctx.pal.border);
            d.line(2.0, 8.0, 8.0, 12.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        } else {
            d.line(-4.0, 4.0, 2.6, 8.4, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.fill_poly(&[[5.0, 10.0], [1.49, 10.06], [3.71, 6.74]], ctx.pal.border);
            d.line(5.0, 10.0, 8.0, 12.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_bjt(&mut self, x: f64, y: f64, pnp: bool) -> String {
        let id = format!("BJT-{}", self.next_bjt);
        self.next_bjt += 1;
        self.items.push(crate::canvas::Item::bjt(&id, x, y, pnp));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bjt_is_npn() {
        let b = Bjt::default();
        assert!(!b.pnp);
        assert_eq!(b.type_id(), "Bjt");
        assert_eq!(b.gain, BJT_DEFAULT_GAIN);
        assert_eq!(b.threshold, 0.7);
        assert_eq!(b.pin_geoms().len(), 3);
    }

    #[test]
    fn stamp_bjt_solves() {
        let b = Bjt::default();
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        b.stamp(&mut m, &[0, usize::MAX, usize::MAX], 0.0);
        m.add_matrix(0, 0, 1e-3);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
    }
}
