//! Analog-to-digital converter component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::AdcState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 16;
const MIN_VREF: f64 = -100.0;
const MAX_VREF: f64 = 100.0;

/// Analog-to-digital converter.
#[derive(Clone, Debug, PartialEq)]
pub struct Adc {
    pub bits: usize,
    pub vref_pos: f64,
    pub vref_neg: f64,
}

impl crate::canvas::Item {
    pub fn adc(
        id: impl Into<String>,
        x: f64,
        y: f64,
        bits: usize,
        vref_pos: f64,
        vref_neg: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Adc {
                bits,
                vref_pos,
                vref_neg,
            },
        )
    }
}

impl Default for Adc {
    fn default() -> Self {
        Self {
            bits: 8,
            vref_pos: 5.0,
            vref_neg: 0.0,
        }
    }
}

impl Adc {
    pub const TYPE_ID: &'static str = "Adc";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Adc(AdcState::new("", self.bits, self.vref_pos, self.vref_neg))
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }

    fn get_vref_pos(&self) -> PropValue {
        PropValue::Float(self.vref_pos)
    }
    fn set_vref_pos(&mut self, v: PropValue) -> Result<(), PropError> {
        self.vref_pos = expect_float("VrefPos", v)?.clamp(MIN_VREF, MAX_VREF);
        Ok(())
    }

    fn get_vref_neg(&self) -> PropValue {
        PropValue::Float(self.vref_neg)
    }
    fn set_vref_neg(&mut self, v: PropValue) -> Result<(), PropError> {
        self.vref_neg = expect_float("VrefNeg", v)?.clamp(MIN_VREF, MAX_VREF);
        Ok(())
    }
}

impl Component for Adc {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Analog to Digital Converter."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BITS: PropDef<Adc> = {
            let mut p = PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                Adc::get_bits,
                Adc::set_bits,
            )
            .with_info("Number of bits of the counter.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Adc>] = &[
            BITS,
            PropDef::float(
                "VrefPos",
                "Vref +",
                "V",
                MIN_VREF,
                MAX_VREF,
                Adc::get_vref_pos,
                Adc::set_vref_pos,
            )
            .with_info("Positive reference voltage for the converter."),
            PropDef::float(
                "VrefNeg",
                "Vref -",
                "V",
                MIN_VREF,
                MAX_VREF,
                Adc::get_vref_neg,
                Adc::set_vref_neg,
            )
            .with_info("Negative reference voltage for the converter."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        adc_pins("", self.bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = (self.bits.max(8) as f64 * 8.0 + 8.0).max(72.0);
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for Adc {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Adc {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "ADC", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_adc(&mut self, x: f64, y: f64) -> String {
        let id = format!("ADC-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::adc(&id, x, y, 8, 5.0, 0.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_adc() {
        let a = Adc::default();
        assert_eq!(a.type_id(), "Adc");
        assert_eq!(a.bits, 8);
        assert_eq!(a.vref_pos, 5.0);
        assert_eq!(a.vref_neg, 0.0);
        // 1 in + 8 out = 9 pins
        assert_eq!(a.pin_geoms().len(), 9);
        assert_eq!(a.body(), Rect::new(-16.0, -36.0, 32.0, 72.0));
    }

    #[test]
    fn set_props() {
        let mut a = Adc::default();
        a.set_prop("Bits", PropValue::Int(10)).unwrap();
        a.set_prop("VrefPos", PropValue::Float(3.3)).unwrap();
        a.set_prop("VrefNeg", PropValue::Float(-3.3)).unwrap();
        assert_eq!(a.bits, 10);
        assert_eq!(a.vref_pos, 3.3);
        assert_eq!(a.vref_neg, -3.3);
        assert_eq!(a.pin_geoms().len(), 11);
    }
}

fn adc_pins(id: &str, bits: usize) -> Vec<Pin> {
    let b = bits.max(1);
    let mut pins = Vec::with_capacity(b + 1);
    // Left: 1 analog input "In"
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in0"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 0.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "In".into(),
        unused: false,
    });
    // Right: bits digital outputs 0..b-1
    let y0 = -((b as f64) * 4.0) + 4.0;
    for i in 0..b {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, y0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: format!("{i}"),
            unused: false,
        });
    }
    pins
}
