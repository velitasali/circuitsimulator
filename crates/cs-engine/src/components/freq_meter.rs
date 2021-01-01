//! FreqMeter: frequency meter with input voltage threshold filter.

use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.0;
const MAX_V: f64 = 1000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct FreqMeter {
    pub filter: f64,
}

impl crate::canvas::Item {
    pub fn freq_meter(id: impl Into<String>, x: f64, y: f64, filter: f64) -> Self {
        Self::new(id, x, y, FreqMeter { filter })
    }
}

impl Default for FreqMeter {
    fn default() -> Self {
        Self { filter: 2.5 }
    }
}

impl FreqMeter {
    pub const TYPE_ID: &'static str = "FreqMeter";
    pub fn to_element_kind(&self) -> Kind {
        Kind::FreqMeter {
            filter: self.filter,
        }
    }

    fn get_filter(&self) -> PropValue {
        PropValue::Float(self.filter)
    }
    fn set_filter(&mut self, v: PropValue) -> Result<(), PropError> {
        self.filter = expect_float("Filter", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
}

impl Component for FreqMeter {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Frequency meter."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<FreqMeter>] = &[PropDef::float(
            "Filter",
            "Filter",
            "V",
            MIN_V,
            MAX_V,
            FreqMeter::get_filter,
            FreqMeter::set_filter,
        )
        .with_info("Filter out any voltage change below this value.")];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![CompPin::new("-lPin", -40.0, 0.0, 180, 8.0).with_direction(PinDirection::In)]
    }

    fn body(&self) -> Rect {
        Rect::new(-32.0, -10.0, 64.0, 20.0)
    }
}

impl Stampable for FreqMeter {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for FreqMeter {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(-32.0, -10.0, 64.0, 20.0, 2.0, ctx.pal.body.fade(0.85));
        d.stroke_round_rect(
            -32.0,
            -10.0,
            64.0,
            20.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        let reading = ctx.canvas.readings().get(ctx.item_id);
        let text = reading
            .map(|r| r.text.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("0 Hz");
        d.text_bold(
            -30.0,
            -6.0,
            text,
            13.0,
            ctx.pal.meter_display,
            Align::TopLeft,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_freq_meter(&mut self, x: f64, y: f64) -> String {
        let id = format!("FreqMeter-{}", self.next_freqmeter);
        self.next_freqmeter += 1;
        self.items
            .push(crate::canvas::Item::freq_meter(&id, x, y, 0.1));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_freq_meter() {
        let fm = FreqMeter::default();
        assert_eq!(fm.type_id(), "FreqMeter");
        assert_eq!(fm.filter, 2.5);
        assert_eq!(fm.pin_geoms().len(), 1);
    }
}
