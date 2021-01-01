//! Magnitude comparator (7485).

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::MagnitudeCompState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 16;

/// Magnitude comparator.
#[derive(Clone, Debug, PartialEq)]
pub struct MagnitudeComp {
    pub bits: usize,
}

impl crate::canvas::Item {
    pub fn magnitude_comp(id: impl Into<String>, x: f64, y: f64, bits: usize) -> Self {
        Self::new(id, x, y, MagnitudeComp { bits })
    }
}

impl Default for MagnitudeComp {
    fn default() -> Self {
        Self { bits: 4 }
    }
}

impl MagnitudeComp {
    pub const TYPE_ID: &'static str = "MagnitudeComp";
    pub fn to_element_kind(&self) -> Kind {
        Kind::MagnitudeComp(MagnitudeCompState::new("", self.bits))
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }
}

impl Component for MagnitudeComp {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Magnitude comparator."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BITS: PropDef<MagnitudeComp> = {
            let mut p = PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                MagnitudeComp::get_bits,
                MagnitudeComp::set_bits,
            )
            .with_info("Number of bits of each operand being compared.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<MagnitudeComp>] = &[BITS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        magnitude_comp_pins("", self.bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let b = self.bits.max(1);
        let h = ((b * 2 + 4).max(6) as f64) * 8.0;
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for MagnitudeComp {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for MagnitudeComp {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "MAG COMP", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_magnitude_comp(&mut self, x: f64, y: f64) -> String {
        let id = format!("MagnitudeComp-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::magnitude_comp(&id, x, y, 4));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_magnitude_comp() {
        let m = MagnitudeComp::default();
        assert_eq!(m.type_id(), "MagnitudeComp");
        assert_eq!(m.bits, 4);
        // 3 cascade in + 4 A + 4 B + 3 out = 14 pins
        assert_eq!(m.pin_geoms().len(), 14);
        // h = (4*2 + 4)*8 = 96
        assert_eq!(m.body(), Rect::new(-16.0, -48.0, 32.0, 96.0));
    }

    #[test]
    fn change_bits() {
        let mut m = MagnitudeComp::default();
        m.set_prop("Bits", PropValue::Int(8)).unwrap();
        assert_eq!(m.bits, 8);
        // 3 cascade + 8 A + 8 B + 3 out = 22 pins
        assert_eq!(m.pin_geoms().len(), 22);
    }
}

fn magnitude_comp_pins(id: &str, bits: usize) -> Vec<Pin> {
    let b = bits.max(1);
    let h = (b * 2 + 3 + 1).max(6);
    let half_h = (h / 2) as f64 * 8.0;
    let mut pins = Vec::with_capacity(b * 2 + 6);
    // Left cascade inputs
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in0"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -half_h + 8.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "iA>B".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in1"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -half_h + 16.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "iA=B".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in2"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -half_h + 24.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "iA<B".into(),
        unused: false,
    });
    // Left operand inputs: A0..A(b-1), B0..B(b-1)
    for i in 0..b {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{}", 3 + i),
            item_id: id.to_string(),
            local: Point::new(-24.0, -half_h + 32.0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("A{i}"),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{}", 3 + b + i),
            item_id: id.to_string(),
            local: Point::new(-24.0, -half_h + 32.0 + ((b + i) as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("B{i}"),
            unused: false,
        });
    }
    // Right outputs: A>B, A=B, A<B
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-out0"),
        item_id: id.to_string(),
        local: Point::new(24.0, -half_h + 8.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "A>B".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-out1"),
        item_id: id.to_string(),
        local: Point::new(24.0, -half_h + 16.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "A=B".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-out2"),
        item_id: id.to_string(),
        local: Point::new(24.0, -half_h + 24.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "A<B".into(),
        unused: false,
    });
    pins
}
