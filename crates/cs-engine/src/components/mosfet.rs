//! MOSFET: metal-oxide-semiconductor field-effect transistor (N/P channel, enhancement/depletion).

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::transistor::{MOSFET_DEFAULT_RDSON, MOSFET_DEFAULT_VTH, MosfetState};
use crate::matrix::CircMatrix;

const MIN_RDSON: f64 = 1e-6;
const MAX_RDSON: f64 = 1e6;
const MIN_VTH: f64 = 0.01;
const MAX_VTH: f64 = 100.0;

impl crate::canvas::Item {
    pub fn mosfet(id: impl Into<String>, x: f64, y: f64, p_channel: bool, depletion: bool) -> Self {
        Self::mosfet_with(
            id,
            x,
            y,
            p_channel,
            depletion,
            MOSFET_DEFAULT_RDSON,
            MOSFET_DEFAULT_VTH,
        )
    }

    pub fn mosfet_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        p_channel: bool,
        depletion: bool,
        rdson: f64,
        threshold: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Mosfet {
                p_channel,
                depletion,
                rdson,
                threshold,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mosfet {
    pub p_channel: bool,
    pub depletion: bool,
    pub rdson: f64,
    pub threshold: f64,
}

impl Default for Mosfet {
    fn default() -> Self {
        Self {
            p_channel: false,
            depletion: false,
            rdson: MOSFET_DEFAULT_RDSON,
            threshold: MOSFET_DEFAULT_VTH,
        }
    }
}

impl Mosfet {
    pub const TYPE_ID: &'static str = "Mosfet";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = MosfetState::new(self.p_channel, self.depletion);
        state.set_rdson(self.rdson);
        state.set_threshold(self.threshold);
        Kind::Mosfet { state }
    }

    fn get_p_channel(&self) -> PropValue {
        PropValue::Bool(self.p_channel)
    }
    fn set_p_channel(&mut self, v: PropValue) -> Result<(), PropError> {
        self.p_channel = expect_bool("PChannel", v)?;
        Ok(())
    }
    fn get_depletion(&self) -> PropValue {
        PropValue::Bool(self.depletion)
    }
    fn set_depletion(&mut self, v: PropValue) -> Result<(), PropError> {
        self.depletion = expect_bool("Depletion", v)?;
        Ok(())
    }
    fn get_rdson(&self) -> PropValue {
        PropValue::Float(self.rdson)
    }
    fn set_rdson(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rdson = expect_float("RDSon", v)?.clamp(MIN_RDSON, MAX_RDSON);
        Ok(())
    }
    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold)
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        self.threshold = expect_float("Threshold", v)?.clamp(MIN_VTH, MAX_VTH);
        Ok(())
    }
}

impl Component for Mosfet {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "MOSFET transistor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Mosfet>] = &[
            PropDef::bool("PChannel", "P-Channel", Mosfet::get_p_channel, Mosfet::set_p_channel).with_info("P channel or N channel."),
            PropDef::bool(
                "Depletion",
                "Depletion",
                Mosfet::get_depletion,
                Mosfet::set_depletion,
            ).with_info("Depletion mode or enhancement mode."),
            PropDef::float(
                "RDSon",
                "RDSon",
                "Ω",
                MIN_RDSON,
                MAX_RDSON,
                Mosfet::get_rdson,
                Mosfet::set_rdson,
            ).with_info("DS Resistance in the lower part of the linear region when conducting:\nVGS > Vth and VDS < ( VGS – Vth )"),
            PropDef::float(
                "Threshold",
                "Threshold",
                "V",
                MIN_VTH,
                MAX_VTH,
                Mosfet::get_threshold,
                Mosfet::set_threshold,
            ).with_info("Gate-Source Voltage to start conducting."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-Dren", 8.0, -16.0, 90, 4.0),
            CompPin::new("-Sour", 8.0, 16.0, 270, 4.0),
            CompPin::new("-Gate", -16.0, 0.0, 180, 4.0),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-12.0, -14.0, 28.0, 28.0)
    }
}

impl Stampable for Mosfet {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin_nodes: 0 = Drain, 1 = Source, 2 = Gate
        stamp_conductance_between(matrix, pin_nodes, 0, 1, 1.0 / self.rdson.max(MIN_RDSON));
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Mosfet {
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
        d.line(8.0, 12.0, 8.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        if self.p_channel {
            d.fill_poly(&[[7.0, 0.0], [3.0, -2.0], [3.0, 2.0]], ctx.pal.border);
            d.line(0.0, 0.0, 3.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(7.0, 0.0, 8.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        } else {
            d.fill_poly(&[[1.0, 0.0], [5.0, -2.0], [5.0, 2.0]], ctx.pal.border);
            d.line(0.0, 0.0, 1.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(5.0, 0.0, 8.0, 0.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        if self.depletion {
            d.line(
                0.0,
                -9.0,
                0.0,
                9.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH * 2.0,
            );
        } else {
            d.line(0.0, -9.0, 0.0, -5.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(0.0, -2.0, 0.0, 2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(0.0, 5.0, 0.0, 9.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_mosfet(&mut self, x: f64, y: f64, p_channel: bool, depletion: bool) -> String {
        let id = format!("Mosfet-{}", self.next_mosfet);
        self.next_mosfet += 1;
        self.items
            .push(crate::canvas::Item::mosfet(&id, x, y, p_channel, depletion));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mosfet_is_n_enhancement() {
        let m = Mosfet::default();
        assert!(!m.p_channel);
        assert!(!m.depletion);
        assert_eq!(m.type_id(), "Mosfet");
        assert_eq!(m.rdson, MOSFET_DEFAULT_RDSON);
        assert_eq!(m.threshold, MOSFET_DEFAULT_VTH);
        assert_eq!(m.pin_geoms().len(), 3);
    }

    #[test]
    fn refuses_legacy_p_channel_prop() {
        let mut m = Mosfet::default();
        assert!(m.set_prop_text("P_Channel", "true").is_err());
        assert!(m.set_prop_text("PChannel", "true").is_ok());
        assert!(m.p_channel);
    }
}
