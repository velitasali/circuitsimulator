//! Comparator: analog differential compare with digital/Thevenin output.

use super::component::stamp_to_ground;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::comparator::{
    COMPARATOR_DEFAULT_OUT_HIGH, COMPARATOR_DEFAULT_OUT_IMP, COMPARATOR_DEFAULT_OUT_LOW,
    ComparatorState,
};
use crate::matrix::CircMatrix;

const MIN_VOLT: f64 = -1000.0;
const MAX_VOLT: f64 = 1000.0;
const MIN_IMP: f64 = 1e-6;
const MAX_IMP: f64 = 1e6;

impl crate::canvas::Item {
    pub fn comparator(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::comparator_with(
            id,
            x,
            y,
            COMPARATOR_DEFAULT_OUT_HIGH,
            COMPARATOR_DEFAULT_OUT_LOW,
            COMPARATOR_DEFAULT_OUT_IMP,
            false,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn comparator_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        out_high: f64,
        out_low: f64,
        out_imp: f64,
        inverted: bool,
        open_col: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Comparator {
                out_high,
                out_low,
                out_imp,
                inverted,
                open_col,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comparator {
    pub out_high: f64,
    pub out_low: f64,
    pub out_imp: f64,
    pub inverted: bool,
    pub open_col: bool,
}

impl Default for Comparator {
    fn default() -> Self {
        Self {
            out_high: COMPARATOR_DEFAULT_OUT_HIGH,
            out_low: COMPARATOR_DEFAULT_OUT_LOW,
            out_imp: COMPARATOR_DEFAULT_OUT_IMP,
            inverted: false,
            open_col: false,
        }
    }
}

impl Comparator {
    pub const TYPE_ID: &'static str = "Comparator";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = ComparatorState::new();
        state.family.out_high_v = self.out_high;
        state.family.out_low_v = self.out_low;
        state.family.out_imp = self.out_imp;
        state.inverted = self.inverted;
        state.open_col = self.open_col;
        Kind::Comparator { state }
    }

    fn get_out_high(&self) -> PropValue {
        PropValue::Float(self.out_high)
    }
    fn set_out_high(&mut self, v: PropValue) -> Result<(), PropError> {
        self.out_high = expect_float("OutHigh", v)?.clamp(MIN_VOLT, MAX_VOLT);
        Ok(())
    }
    fn get_out_low(&self) -> PropValue {
        PropValue::Float(self.out_low)
    }
    fn set_out_low(&mut self, v: PropValue) -> Result<(), PropError> {
        self.out_low = expect_float("OutLow", v)?.clamp(MIN_VOLT, MAX_VOLT);
        Ok(())
    }
    fn get_out_imped(&self) -> PropValue {
        PropValue::Float(self.out_imp)
    }
    fn set_out_imped(&mut self, v: PropValue) -> Result<(), PropError> {
        self.out_imp = expect_float("OutImped", v)?.clamp(MIN_IMP, MAX_IMP);
        Ok(())
    }
    fn get_inverted(&self) -> PropValue {
        PropValue::Bool(self.inverted)
    }
    fn set_inverted(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inverted = expect_bool("Inverted", v)?;
        Ok(())
    }
    fn get_open_collector(&self) -> PropValue {
        PropValue::Bool(self.open_col)
    }
    fn set_open_collector(&mut self, v: PropValue) -> Result<(), PropError> {
        self.open_col = expect_bool("OpenCollector", v)?;
        Ok(())
    }
}

impl Component for Comparator {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Voltage comparator."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Comparator>] = &[
            PropDef::float(
                "OutHigh",
                "Output High",
                "V",
                MIN_VOLT,
                MAX_VOLT,
                Comparator::get_out_high,
                Comparator::set_out_high,
            )
            .with_info("High-level output voltage."),
            PropDef::float(
                "OutLow",
                "Output Low",
                "V",
                MIN_VOLT,
                MAX_VOLT,
                Comparator::get_out_low,
                Comparator::set_out_low,
            )
            .with_info("Low-level output voltage."),
            PropDef::float(
                "OutImped",
                "Output Impedance",
                "Ω",
                MIN_IMP,
                MAX_IMP,
                Comparator::get_out_imped,
                Comparator::set_out_imped,
            )
            .with_info("Impedance of the output pins."),
            PropDef::bool(
                "Inverted",
                "Inverted",
                Comparator::get_inverted,
                Comparator::set_inverted,
            )
            .with_info("Invert output pins."),
            PropDef::bool(
                "OpenCollector",
                "Open Collector",
                Comparator::get_open_collector,
                Comparator::set_open_collector,
            )
            .with_info("Output stage is open collector/drain instead of push-pull."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-in0", -24.0, -8.0, 180, 8.0).with_direction(PinDirection::In),
            CompPin::new("-in1", -24.0, 8.0, 180, 8.0).with_direction(PinDirection::In),
            CompPin::new("-out", 24.0, 0.0, 0, 8.0).with_direction(PinDirection::Out),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-18.0, -16.0, 36.0, 32.0)
    }
}

impl Stampable for Comparator {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin 2 is -out
        let g = 1.0 / self.out_imp.max(MIN_IMP);
        stamp_to_ground(matrix, pin_nodes, 2, self.out_low, g);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Comparator {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let pts = [[-16.0, -16.0], [-16.0, 16.0], [16.0, 1.0], [16.0, -1.0]];
        d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        let (top_sym, bot_sym) = if self.inverted {
            ("-", "+")
        } else {
            ("+", "-")
        };
        d.text(-13.0, -12.0, top_sym, 8.0, ctx.pal.border, Align::TopLeft);
        d.text(-13.0, 3.0, bot_sym, 8.0, ctx.pal.border, Align::TopLeft);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_comparator(&mut self, x: f64, y: f64) -> String {
        let id = format!("Comparator-{}", self.next_comparator);
        self.next_comparator += 1;
        self.items.push(crate::canvas::Item::comparator(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_comparator_properties() {
        let c = Comparator::default();
        assert_eq!(c.type_id(), "Comparator");
        assert_eq!(c.out_high, COMPARATOR_DEFAULT_OUT_HIGH);
        assert_eq!(c.out_low, COMPARATOR_DEFAULT_OUT_LOW);
        assert!(!c.inverted);
        assert!(!c.open_col);
        assert_eq!(c.pin_geoms().len(), 3);
    }
}
