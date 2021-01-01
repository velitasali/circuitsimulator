//! Digital-to-analog converter component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::DacState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 16;
const MIN_VREF: f64 = -100.0;
const MAX_VREF: f64 = 100.0;

/// Digital-to-analog converter.
#[derive(Clone, Debug, PartialEq)]
pub struct Dac {
    pub bits: usize,
    pub vref: f64,
}

impl crate::canvas::Item {
    pub fn dac(id: impl Into<String>, x: f64, y: f64, bits: usize, vref: f64) -> Self {
        Self::new(id, x, y, Dac { bits, vref })
    }
}

impl Default for Dac {
    fn default() -> Self {
        Self { bits: 8, vref: 5.0 }
    }
}

impl Dac {
    pub const TYPE_ID: &'static str = "Dac";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Dac(DacState::new("", self.bits, self.vref))
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }

    fn get_vref(&self) -> PropValue {
        PropValue::Float(self.vref)
    }
    fn set_vref(&mut self, v: PropValue) -> Result<(), PropError> {
        self.vref = expect_float("Vref", v)?.clamp(MIN_VREF, MAX_VREF);
        Ok(())
    }
}

impl Component for Dac {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Digital to Analog Converter."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BITS: PropDef<Dac> = {
            let mut p = PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                Dac::get_bits,
                Dac::set_bits,
            )
            .with_info("Number of bits of the counter.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Dac>] = &[
            BITS,
            PropDef::float(
                "Vref",
                "Vref",
                "V",
                MIN_VREF,
                MAX_VREF,
                Dac::get_vref,
                Dac::set_vref,
            )
            .with_info("Voltage assigned to the maximum value."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        dac_pins("", self.bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = (self.bits.max(8) as f64 * 8.0 + 8.0).max(72.0);
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for Dac {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Dac {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "DAC", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_dac(&mut self, x: f64, y: f64) -> String {
        let id = format!("DAC-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::dac(&id, x, y, 8, 5.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dac() {
        let d = Dac::default();
        assert_eq!(d.type_id(), "Dac");
        assert_eq!(d.bits, 8);
        assert_eq!(d.vref, 5.0);
        // 8 in + 1 out = 9 pins
        assert_eq!(d.pin_geoms().len(), 9);
        assert_eq!(d.body(), Rect::new(-16.0, -36.0, 32.0, 72.0));
    }

    #[test]
    fn set_props() {
        let mut d = Dac::default();
        d.set_prop("Bits", PropValue::Int(12)).unwrap();
        d.set_prop("Vref", PropValue::Float(2.5)).unwrap();
        assert_eq!(d.bits, 12);
        assert_eq!(d.vref, 2.5);
        assert_eq!(d.pin_geoms().len(), 13);
    }
}

fn dac_pins(id: &str, bits: usize) -> Vec<Pin> {
    let b = bits.max(1);
    let mut pins = Vec::with_capacity(b + 1);
    // Left: bits digital inputs 0..b-1
    let y0 = -((b as f64) * 4.0) + 4.0;
    for i in 0..b {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, y0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("{i}"),
            unused: false,
        });
    }
    // Right: 1 analog output "Out"
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-out0"),
        item_id: id.to_string(),
        local: Point::new(24.0, 0.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "Out".into(),
        unused: false,
    });
    pins
}
