//! Ammeter: series current meter with low shunt resistance and optional RMS display.

use super::component::{stamp_conductance_between, stamp_to_ground};
use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::SOURCE_ADMIT;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::instruments::AMMETER_OHMS;
use crate::matrix::CircMatrix;

#[derive(Clone, Debug, PartialEq)]
pub struct Ammeter {
    pub rms: bool,
}

impl crate::canvas::Item {
    pub fn ammeter(id: impl Into<String>, x: f64, y: f64, rms: bool) -> Self {
        Self::new(id, x, y, Ammeter { rms })
    }
}

impl Default for Ammeter {
    fn default() -> Self {
        Self { rms: false }
    }
}

impl Ammeter {
    pub const TYPE_ID: &'static str = "Ammeter";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ammeter {
            rms: self.rms,
            last_out: 0.0,
        }
    }

    fn get_rms(&self) -> PropValue {
        PropValue::Bool(self.rms)
    }
    fn set_rms(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rms = expect_bool("Rms", v)?;
        Ok(())
    }
}

impl Component for Ammeter {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Current meter."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Ammeter>] = &[PropDef::bool(
            "Rms",
            "AC (RMS)",
            Ammeter::get_rms,
            Ammeter::set_rms,
        ).with_info("Shows the RMS (root-mean-square) value of the current instead of the instantaneous value.")];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-lPin", -8.0, 16.0, 270, 8.0).with_direction(PinDirection::In),
            CompPin::new("-rPin", 8.0, 16.0, 270, 8.0).with_direction(PinDirection::In),
            CompPin::new("-outnod", 32.0, -8.0, 0, 8.0).with_direction(PinDirection::Out),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-24.0, -24.0, 48.0, 32.0)
    }
}

impl Stampable for Ammeter {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_conductance_between(matrix, pin_nodes, 0, 1, 1.0 / AMMETER_OHMS);
        stamp_to_ground(matrix, pin_nodes, 2, 0.0, SOURCE_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Ammeter {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(-24.0, -24.0, 48.0, 32.0, 2.0, ctx.pal.body.fade(0.85));
        d.stroke_round_rect(
            -24.0,
            -24.0,
            48.0,
            32.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        let reading = ctx.canvas.readings().get(ctx.item_id);
        let main_text = reading
            .map(|r| r.text.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("0.00 A");
        let hz_text = reading
            .map(|r| r.hz_text.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(if self.rms { "--Hz" } else { "" });
        let max_text = reading
            .map(|r| r.max_text.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("MAX 0.00 A");
        let avg_text = reading
            .map(|r| r.avg_text.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("AVG 0.00 A");

        d.text_bold(
            -20.0,
            -22.0,
            main_text,
            9.0,
            ctx.pal.meter_display,
            Align::TopLeft,
        );

        if !hz_text.is_empty() {
            let main_w = crate::canvas::export::text_width(main_text, 9.0, true);
            d.text(
                -20.0 + main_w + 2.0,
                -21.0,
                hz_text,
                4.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
        }

        let max_prefix_w = crate::canvas::export::text_width("MAX ", 6.0, false);
        if let Some(val) = max_text.strip_prefix("MAX ") {
            d.text(
                -20.0,
                -10.5,
                "MAX ",
                6.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
            d.text_bold(
                -20.0 + max_prefix_w,
                -10.5,
                val,
                6.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
        } else {
            d.text_bold(
                -20.0,
                -10.5,
                max_text,
                6.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
        }

        let avg_prefix_w = crate::canvas::export::text_width("AVG ", 6.0, false);
        if let Some(val) = avg_text.strip_prefix("AVG ") {
            d.text(
                -20.0,
                -2.5,
                "AVG ",
                6.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
            d.text_bold(
                -20.0 + avg_prefix_w,
                -2.5,
                val,
                6.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
        } else {
            d.text_bold(
                -20.0,
                -2.5,
                avg_text,
                6.0,
                ctx.pal.meter_display,
                Align::TopLeft,
            );
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ammeter(&mut self, x: f64, y: f64) -> String {
        let id = format!("Amperimeter-{}", self.next_ammeter);
        self.next_ammeter += 1;
        self.items
            .push(crate::canvas::Item::ammeter(&id, x, y, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ammeter() {
        let am = Ammeter::default();
        assert_eq!(am.type_id(), "Ammeter");
        assert!(!am.rms);
        assert_eq!(am.pin_geoms().len(), 3);
    }
}
