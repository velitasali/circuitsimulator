//! Shift register with parallel outputs (74164 / 74595).

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::ShiftRegState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 32;

/// Shift register with parallel outputs.
#[derive(Clone, Debug, PartialEq)]
pub struct ShiftReg {
    pub bits: usize,
}

impl crate::canvas::Item {
    pub fn shift_reg(id: impl Into<String>, x: f64, y: f64, bits: usize) -> Self {
        Self::new(id, x, y, ShiftReg { bits })
    }
}

impl Default for ShiftReg {
    fn default() -> Self {
        Self { bits: 8 }
    }
}

impl ShiftReg {
    pub const TYPE_ID: &'static str = "ShiftReg";
    pub fn to_element_kind(&self) -> Kind {
        Kind::ShiftReg(ShiftRegState::new("", self.bits))
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }
}

impl Component for ShiftReg {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Shift register with parallel outputs."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BITS: PropDef<ShiftReg> = {
            let mut p = PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                ShiftReg::get_bits,
                ShiftReg::set_bits,
            )
            .with_info("Register width in bits.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<ShiftReg>] = &[BITS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        shift_reg_pins("", self.bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = (self.bits as f64 * 8.0 + 8.0).max(72.0);
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for ShiftReg {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for ShiftReg {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "SHIFT REG", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_shift_reg(&mut self, x: f64, y: f64) -> String {
        let id = format!("ShiftReg-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::shift_reg(&id, x, y, 8));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shift_reg() {
        let s = ShiftReg::default();
        assert_eq!(s.type_id(), "ShiftReg");
        assert_eq!(s.bits, 8);
        // DI, clk, rst, oe + 8 outputs = 12 pins
        assert_eq!(s.pin_geoms().len(), 12);
        assert_eq!(s.body(), Rect::new(-16.0, -36.0, 32.0, 72.0));
    }

    #[test]
    fn change_bits() {
        let mut s = ShiftReg::default();
        s.set_prop("Bits", PropValue::Int(16)).unwrap();
        assert_eq!(s.bits, 16);
        // 4 control + 16 outputs = 20 pins
        assert_eq!(s.pin_geoms().len(), 20);
    }
}

fn shift_reg_pins(id: &str, bits: usize) -> Vec<Pin> {
    let b = bits.max(1);
    let mut pins = Vec::with_capacity(b + 4);
    // Left DI pos 3
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in0"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -8.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "DI".into(),
        unused: false,
    });
    // Left Clock pos 5
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in1"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 0.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: ">".into(),
        unused: false,
    });
    // Left Reset pos 7 (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in2"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 8.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "Rst".into(),
        unused: false,
    });
    // Top OE pin (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in3"),
        item_id: id.to_string(),
        local: Point::new(-8.0, -40.0),
        angle: 90,
        length: 8.0,
        is_bus: false,
        label: "OE".into(),
        unused: false,
    });
    // Right outputs: Q0..Q(b-1)
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
