//! Binary full adder.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::FullAdderState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 32;

/// Binary full adder.
#[derive(Clone, Debug, PartialEq)]
pub struct FullAdder {
    pub bits: usize,
}

impl crate::canvas::Item {
    pub fn full_adder(id: impl Into<String>, x: f64, y: f64, bits: usize) -> Self {
        Self::new(id, x, y, FullAdder { bits })
    }
}

impl Default for FullAdder {
    fn default() -> Self {
        Self { bits: 4 }
    }
}

impl FullAdder {
    pub const TYPE_ID: &'static str = "FullAdder";
    pub fn to_element_kind(&self) -> Kind {
        Kind::FullAdder(FullAdderState::new("", self.bits))
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }
}

impl Component for FullAdder {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Binary Full Adder."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BITS: PropDef<FullAdder> = {
            let mut p = PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                FullAdder::get_bits,
                FullAdder::set_bits,
            )
            .with_info("Number of bits of the operands.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<FullAdder>] = &[BITS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        full_adder_pins("", self.bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let b = self.bits.max(1);
        let h = ((b * 2 + 2).max(4) as f64) * 8.0;
        Rect::new(-12.0, -h / 2.0, 24.0, h)
    }
}

impl Stampable for FullAdder {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for FullAdder {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "ADDER", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_full_adder(&mut self, x: f64, y: f64) -> String {
        let id = format!("FullAdder-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::full_adder(&id, x, y, 4));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_full_adder() {
        let a = FullAdder::default();
        assert_eq!(a.type_id(), "FullAdder");
        assert_eq!(a.bits, 4);
        // 4 A + 4 B + 1 Ci + 4 S + 1 Co = 14 pins
        assert_eq!(a.pin_geoms().len(), 14);
    }

    #[test]
    fn set_bits() {
        let mut a = FullAdder::default();
        a.set_prop("Bits", PropValue::Int(1)).unwrap();
        assert_eq!(a.bits, 1);
        // 1 A + 1 B + 1 Ci + 1 S + 1 Co = 5 pins
        assert_eq!(a.pin_geoms().len(), 5);
    }
}

fn full_adder_pins(id: &str, bits: usize) -> Vec<Pin> {
    let b = bits.max(1);
    let h = (b * 2 + 2).max(4);
    let half_h = (h / 2) as f64 * 8.0;
    let mut pins = Vec::with_capacity(b * 3 + 2);
    // Left inputs: A0..A(b-1), B0..B(b-1)
    for i in 0..b {
        let i_str = if b > 1 { format!("{i}") } else { String::new() };
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{}", 1 + i),
            item_id: id.to_string(),
            local: Point::new(-20.0, -half_h + 8.0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("A{i_str}"),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{}", 1 + b + i),
            item_id: id.to_string(),
            local: Point::new(-20.0, -half_h + 8.0 + ((b + i) as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("B{i_str}"),
            unused: false,
        });
    }
    // Right carry in Ci at pos 1
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-ci"),
        item_id: id.to_string(),
        local: Point::new(20.0, -half_h + 8.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "Ci".into(),
        unused: false,
    });
    // Right sum outputs: S0..S(b-1)
    for i in 0..b {
        let i_str = if b > 1 { format!("{i}") } else { String::new() };
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{}", 1 + i),
            item_id: id.to_string(),
            local: Point::new(20.0, -half_h + 16.0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: format!("S{i_str}"),
            unused: false,
        });
    }
    // Right carry out Co at bottom
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-co"),
        item_id: id.to_string(),
        local: Point::new(20.0, -half_h + 16.0 + (b as f64) * 8.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "Co".into(),
        unused: false,
    });
    pins
}
