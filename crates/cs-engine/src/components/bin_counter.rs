//! 4-bit binary / decade counter (7490 / 7493).

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::BinCounterState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

/// 4-bit binary or decade counter.
#[derive(Clone, Debug, PartialEq)]
pub struct BinCounter {
    pub is_decade: bool,
}

impl crate::canvas::Item {
    pub fn bin_counter(id: impl Into<String>, x: f64, y: f64, is_decade: bool) -> Self {
        Self::new(id, x, y, BinCounter { is_decade })
    }
}

impl Default for BinCounter {
    fn default() -> Self {
        Self { is_decade: false }
    }
}

impl BinCounter {
    pub const TYPE_ID: &'static str = "BinCounter";
    pub fn to_element_kind(&self) -> Kind {
        Kind::BinCounter(BinCounterState::new("", self.is_decade))
    }

    fn get_is_decade(&self) -> PropValue {
        PropValue::Bool(self.is_decade)
    }
    fn set_is_decade(&mut self, v: PropValue) -> Result<(), PropError> {
        self.is_decade = expect_bool("IsDecade", v)?;
        Ok(())
    }
}

impl Component for BinCounter {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "4-Bit Binary / Decade Counter."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<BinCounter>] = &[PropDef::bool(
            "IsDecade",
            "Decade Counter",
            BinCounter::get_is_decade,
            BinCounter::set_is_decade,
        )
        .with_info("Decade counter mode (counts 0 to 9) instead of binary 4-bit (0 to 15).")];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        bin_counter_pins("", 4, false, false, false)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -24.0, 32.0, 48.0)
    }
}

impl Stampable for BinCounter {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for BinCounter {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "BIN CTR", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_bin_counter(&mut self, x: f64, y: f64) -> String {
        let id = format!("BinCounter-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::bin_counter(&id, x, y, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bin_counter() {
        let b = BinCounter::default();
        assert_eq!(b.type_id(), "BinCounter");
        assert!(!b.is_decade);
        // clk_a, clk_b, r0_1, r0_2, r9_1, r9_2 ... wait!
        // bin_counter_pins("", 4, false, false, false):
        // left: clk, rst (2 pins) + right: out0..out3 (4 pins) = 6 pins
        assert_eq!(b.pin_geoms().len(), 6);
        assert_eq!(b.body(), Rect::new(-16.0, -24.0, 32.0, 48.0));
    }

    #[test]
    fn set_is_decade() {
        let mut b = BinCounter::default();
        b.set_prop("IsDecade", PropValue::Bool(true)).unwrap();
        assert!(b.is_decade);
    }
}

fn bin_counter_pins(
    id: &str,
    bits: usize,
    bidirectional: bool,
    parallel_in: bool,
    use_rco: bool,
) -> Vec<Pin> {
    let b = bits.max(1);
    let h = (b + 2).max(6);
    let half_h = (h / 2) as f64 * 8.0;
    let mut pins = Vec::with_capacity(b + 6);
    // Left pins
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_clk"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -half_h + 8.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: ">".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_rst"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -half_h + 16.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "Rst".into(),
        unused: false,
    });
    if bidirectional {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-Pin_dir"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -half_h + 24.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "Dir".into(),
            unused: false,
        });
    }
    if parallel_in {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-Pin_ldp"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -half_h + 32.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "LD".into(),
            unused: false,
        });
    }
    // Right outputs: Q0..Q(b-1)
    let y_q0 = -half_h + 8.0;
    for i in 0..b {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, y_q0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: format!("{i}"),
            unused: false,
        });
    }
    if use_rco {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-Pin_rco"),
            item_id: id.to_string(),
            local: Point::new(24.0, y_q0 + (b as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "CO".into(),
            unused: false,
        });
        if bidirectional {
            pins.push(Pin {
                direction: Some(PinDirection::In),
                id: format!("{id}-Pin_rbo"),
                item_id: id.to_string(),
                local: Point::new(24.0, y_q0 + ((b + 1) as f64) * 8.0),
                angle: 0,
                length: 8.0,
                is_bus: false,
                label: "BO".into(),
                unused: false,
            });
        }
    }
    pins
}
