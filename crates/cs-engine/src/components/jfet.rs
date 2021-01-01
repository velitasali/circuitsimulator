//! JFET: junction field-effect transistor with square-law and channel-length correction.

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::transistor::{
    JFET_DEFAULT_IDSS, JFET_DEFAULT_LAMBDA_INV, JFET_DEFAULT_VP, JfetState,
};
use crate::matrix::CircMatrix;

const MIN_IDSS: f64 = 1e-6;
const MAX_IDSS: f64 = 1000.0;
const MIN_VP: f64 = -1000.0;
const MAX_VP: f64 = 1000.0;
const MIN_LAMBDA_INV: f64 = 1.0;
const MAX_LAMBDA_INV: f64 = 1e6;

impl crate::canvas::Item {
    pub fn jfet(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::jfet_with(
            id,
            x,
            y,
            false,
            JFET_DEFAULT_IDSS,
            JFET_DEFAULT_VP,
            JFET_DEFAULT_LAMBDA_INV,
        )
    }

    pub fn jfet_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        p_channel: bool,
        idss: f64,
        vp: f64,
        lambda_inv: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Jfet {
                p_channel,
                idss,
                vp,
                lambda_inv,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Jfet {
    pub p_channel: bool,
    pub idss: f64,
    pub vp: f64,
    pub lambda_inv: f64,
}

impl Default for Jfet {
    fn default() -> Self {
        Self {
            p_channel: false,
            idss: JFET_DEFAULT_IDSS,
            vp: JFET_DEFAULT_VP,
            lambda_inv: JFET_DEFAULT_LAMBDA_INV,
        }
    }
}

impl Jfet {
    pub const TYPE_ID: &'static str = "Jfet";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = JfetState::new();
        state.set_idss(self.idss);
        state.set_vp(self.vp);
        state.set_lambda_inv(self.lambda_inv);
        Kind::Jfet { state }
    }

    fn get_p_channel(&self) -> PropValue {
        PropValue::Bool(self.p_channel)
    }
    fn set_p_channel(&mut self, v: PropValue) -> Result<(), PropError> {
        self.p_channel = expect_bool("PChannel", v)?;
        Ok(())
    }
    fn get_idss(&self) -> PropValue {
        PropValue::Float(self.idss)
    }
    fn set_idss(&mut self, v: PropValue) -> Result<(), PropError> {
        self.idss = expect_float("Idss", v)?.clamp(MIN_IDSS, MAX_IDSS);
        Ok(())
    }
    fn get_vp(&self) -> PropValue {
        PropValue::Float(self.vp)
    }
    fn set_vp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.vp = expect_float("Vp", v)?.clamp(MIN_VP, MAX_VP);
        Ok(())
    }
    fn get_lambda_inv(&self) -> PropValue {
        PropValue::Float(self.lambda_inv)
    }
    fn set_lambda_inv(&mut self, v: PropValue) -> Result<(), PropError> {
        self.lambda_inv = expect_float("LambdaInv", v)?.clamp(MIN_LAMBDA_INV, MAX_LAMBDA_INV);
        Ok(())
    }
}

impl Component for Jfet {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "JFET transistor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Jfet>] = &[
            PropDef::bool(
                "PChannel",
                "P-Channel",
                Jfet::get_p_channel,
                Jfet::set_p_channel,
            )
            .with_info("P channel or N channel."),
            PropDef::float(
                "Idss",
                "Idss",
                "A",
                MIN_IDSS,
                MAX_IDSS,
                Jfet::get_idss,
                Jfet::set_idss,
            )
            .with_info(
                "Drain-Source saturation current: drain current when Gate-Source voltage is 0.",
            ),
            PropDef::float("Vp", "Vp", "V", MIN_VP, MAX_VP, Jfet::get_vp, Jfet::set_vp).with_info(
                "Pinch-off voltage: Gate-Source voltage at which the channel is fully pinched off.",
            ),
            PropDef::float(
                "LambdaInv",
                "1/Lambda",
                "V",
                MIN_LAMBDA_INV,
                MAX_LAMBDA_INV,
                Jfet::get_lambda_inv,
                Jfet::set_lambda_inv,
            )
            .with_info("Channel-length modulation inverse parameter (1/VA)."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-Dren", 8.0, -16.0, 90, 4.0),
            CompPin::new("-Sour", 8.0, 16.0, 270, 8.0),
            CompPin::new("-Gate", -16.0, 0.0, 180, 4.0),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-12.0, -14.0, 28.0, 28.0)
    }
}

impl Stampable for Jfet {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin_nodes: 0 = Drain, 1 = Source, 2 = Gate
        stamp_conductance_between(matrix, pin_nodes, 0, 1, 1e-9);
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Jfet {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_circle(2.0, 0.0, 14.0, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_circle(2.0, 0.0, 14.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(
            -12.0,
            0.0,
            -5.0,
            0.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(0.0, -9.0, 0.0, 9.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(0.0, -7.0, 8.0, -7.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(0.0, 7.0, 8.0, 7.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(
            8.0,
            -12.0,
            8.0,
            -7.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(8.0, 12.0, 8.0, 7.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        if self.p_channel {
            d.fill_poly(&[[-5.0, 0.0], [-1.0, -2.0], [-1.0, 2.0]], ctx.pal.border);
        } else {
            d.fill_poly(&[[-1.0, 0.0], [-5.0, -2.0], [-5.0, 2.0]], ctx.pal.border);
        }
        d.line(-1.0, 0.0, 0.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_jfet(&mut self, x: f64, y: f64) -> String {
        let id = format!("Jfet-{}", self.next_jfet);
        self.next_jfet += 1;
        self.items.push(crate::canvas::Item::jfet(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_jfet_is_n_channel() {
        let j = Jfet::default();
        assert!(!j.p_channel);
        assert_eq!(j.type_id(), "Jfet");
        assert_eq!(j.idss, JFET_DEFAULT_IDSS);
        assert_eq!(j.vp, JFET_DEFAULT_VP);
        assert_eq!(j.lambda_inv, JFET_DEFAULT_LAMBDA_INV);
        assert_eq!(j.pin_geoms().len(), 3);
    }

    #[test]
    fn refuses_legacy_props() {
        let mut j = Jfet::default();
        assert!(j.set_prop_text("P_Channel", "true").is_err());
        assert!(j.set_prop_text("1/Lambda", "500 V").is_err());
        assert!(j.set_prop_text("PChannel", "true").is_ok());
        assert!(j.set_prop_text("LambdaInv", "500 V").is_ok());
        assert_eq!(j.lambda_inv, 500.0);
    }
}
