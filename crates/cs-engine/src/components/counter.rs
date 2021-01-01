//! Binary counter / frequency divider component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::CounterState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 32;

/// Binary counter / frequency divider.
#[derive(Clone, Debug, PartialEq)]
pub struct Counter {
    pub bits: usize,
    pub max_count: u32,
}

impl crate::canvas::Item {
    pub fn counter(id: impl Into<String>, x: f64, y: f64, bits: usize, max_count: u32) -> Self {
        Self::new(id, x, y, Counter { bits, max_count })
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            bits: 4,
            max_count: 15,
        }
    }
}

impl Counter {
    pub const TYPE_ID: &'static str = "Counter";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Counter(CounterState::new("", self.bits, self.max_count))
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }

    fn get_max_count(&self) -> PropValue {
        PropValue::Int(self.max_count as i64)
    }
    fn set_max_count(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_count = expect_int("MaxCount", v)?.max(0) as u32;
        Ok(())
    }
}

impl Component for Counter {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Binary Counter."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Counter>] = &[
            PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                Counter::get_bits,
                Counter::set_bits,
            )
            .with_info("Number of bits of the counter."),
            PropDef::int(
                "MaxCount",
                "Max Count",
                0,
                i64::MAX,
                Counter::get_max_count,
                Counter::set_max_count,
            )
            .with_info("Maximum counter value before wrapping or resetting to zero."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        counter_pins("", false)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-12.0, -12.0, 24.0, 24.0)
    }
}

impl Stampable for Counter {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Counter {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "CTR", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_counter(&mut self, x: f64, y: f64) -> String {
        let id = format!("Counter-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::counter(&id, x, y, 4, 15));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_counter() {
        let c = Counter::default();
        assert_eq!(c.type_id(), "Counter");
        assert_eq!(c.bits, 4);
        assert_eq!(c.max_count, 15);
        assert_eq!(c.pin_geoms().len(), 3);
        assert_eq!(c.body(), Rect::new(-12.0, -12.0, 24.0, 24.0));
    }

    #[test]
    fn set_props() {
        let mut c = Counter::default();
        c.set_prop("Bits", PropValue::Int(8)).unwrap();
        c.set_prop("MaxCount", PropValue::Int(100)).unwrap();
        assert_eq!(c.bits, 8);
        assert_eq!(c.max_count, 100);
    }
}

fn counter_pins(id: &str, use_set: bool) -> Vec<Pin> {
    let mut pins = Vec::with_capacity(if use_set { 4 } else { 3 });
    // Left pos 1: Clock
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in0"),
        item_id: id.to_string(),
        local: Point::new(-20.0, -4.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: ">".into(),
        unused: false,
    });
    // Left pos 2: Reset (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in1"),
        item_id: id.to_string(),
        local: Point::new(-20.0, 4.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "R".into(),
        unused: false,
    });
    // Top pos 1: Set (optional, active low)
    if use_set {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in2"),
            item_id: id.to_string(),
            local: Point::new(-4.0, -20.0),
            angle: 90,
            length: 8.0,
            is_bus: false,
            label: "S".into(),
            unused: false,
        });
    }
    // Right pos 1: Q
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-out0"),
        item_id: id.to_string(),
        local: Point::new(20.0, 0.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "Q".into(),
        unused: false,
    });
    pins
}
