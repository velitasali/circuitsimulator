//! 4-bit binary / decade counter (7490 / 7493).

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::BinCounterState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 32;

/// Binary or decade counter (N-bit, optional parallel load, RCO, and up/down direction).
#[derive(Clone, Debug, PartialEq)]
pub struct BinCounter {
    pub is_decade: bool,
    pub bits: usize,
    pub max_value: u32,
    pub parallel_input: bool,
    pub use_rco: bool,
    pub bidirectional: bool,
}

impl crate::canvas::Item {
    pub fn bin_counter(id: impl Into<String>, x: f64, y: f64, is_decade: bool) -> Self {
        Self::new(
            id,
            x,
            y,
            BinCounter {
                is_decade,
                ..BinCounter::default()
            },
        )
    }
}

impl Default for BinCounter {
    fn default() -> Self {
        Self {
            is_decade: false,
            bits: 4,
            max_value: 0,
            parallel_input: false,
            use_rco: false,
            bidirectional: false,
        }
    }
}

impl BinCounter {
    pub const TYPE_ID: &'static str = "BinCounter";
    pub fn to_element_kind(&self) -> Kind {
        Kind::BinCounter(BinCounterState::new_configured(
            "",
            self.is_decade,
            self.bits,
            self.max_value,
            self.parallel_input,
            self.use_rco,
            self.bidirectional,
        ))
    }

    fn get_is_decade(&self) -> PropValue {
        PropValue::Bool(self.is_decade)
    }
    fn set_is_decade(&mut self, v: PropValue) -> Result<(), PropError> {
        self.is_decade = expect_bool("IsDecade", v)?;
        Ok(())
    }

    fn get_bits(&self) -> PropValue {
        PropValue::Int(self.bits as i64)
    }
    fn set_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bits = expect_int("Bits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }

    fn get_max_value(&self) -> PropValue {
        PropValue::Int(self.max_value as i64)
    }
    fn set_max_value(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_value = expect_int("MaxValue", v)?.max(0) as u32;
        Ok(())
    }

    fn get_parallel_input(&self) -> PropValue {
        PropValue::Bool(self.parallel_input)
    }
    fn set_parallel_input(&mut self, v: PropValue) -> Result<(), PropError> {
        self.parallel_input = expect_bool("ParallelInput", v)?;
        Ok(())
    }

    fn get_use_rco(&self) -> PropValue {
        PropValue::Bool(self.use_rco)
    }
    fn set_use_rco(&mut self, v: PropValue) -> Result<(), PropError> {
        self.use_rco = expect_bool("UseRco", v)?;
        Ok(())
    }

    fn get_bidirectional(&self) -> PropValue {
        PropValue::Bool(self.bidirectional)
    }
    fn set_bidirectional(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bidirectional = expect_bool("Bidirectional", v)?;
        Ok(())
    }
}

impl Component for BinCounter {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    const DESCRIPTION: &'static str = "Binary / Decade Counter.";

    fn props() -> &'static [PropDef<Self>] {
        const BITS: PropDef<BinCounter> = {
            let mut p = PropDef::int(
                "Bits",
                "Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                BinCounter::get_bits,
                BinCounter::set_bits,
            )
            .with_info("Counter width in bits.");
            p.structural = true;
            p
        };
        const PARALLEL_INPUT: PropDef<BinCounter> = {
            let mut p = PropDef::bool(
                "ParallelInput",
                "Parallel Input",
                BinCounter::get_parallel_input,
                BinCounter::set_parallel_input,
            )
            .with_info("Exposes parallel load (Pin_ldp) and data input pins.");
            p.structural = true;
            p
        };
        const USE_RCO: PropDef<BinCounter> = {
            let mut p = PropDef::bool(
                "UseRco",
                "Ripple Carry Out",
                BinCounter::get_use_rco,
                BinCounter::set_use_rco,
            )
            .with_info("Exposes the Ripple Carry Output (Pin_rco) pin.");
            p.structural = true;
            p
        };
        const BIDIRECTIONAL: PropDef<BinCounter> = {
            let mut p = PropDef::bool(
                "Bidirectional",
                "Bidirectional",
                BinCounter::get_bidirectional,
                BinCounter::set_bidirectional,
            )
            .with_info("Exposes the Up/Down Direction (Pin_dir) pin.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<BinCounter>] = &[
            PropDef::bool(
                "IsDecade",
                "Decade Counter",
                BinCounter::get_is_decade,
                BinCounter::set_is_decade,
            )
            .with_info("Decade counter mode (counts 0 to 9) instead of binary 4-bit (0 to 15)."),
            BITS,
            PropDef::int(
                "MaxValue",
                "Max Value",
                0,
                i64::MAX,
                BinCounter::get_max_value,
                BinCounter::set_max_value,
            )
            .with_info("Maximum counter value before wrapping (0 uses 2^Bits - 1 or 9 if decade)."),
            PARALLEL_INPUT,
            USE_RCO,
            BIDIRECTIONAL,
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        bin_counter_pins(
            "",
            self.bits,
            self.bidirectional,
            self.parallel_input,
            self.use_rco,
        )
        .into_iter()
        .map(CompPin::from)
        .collect()
    }

    fn body(&self) -> Rect {
        let rows = bin_counter_rows(
            self.bits,
            self.bidirectional,
            self.parallel_input,
            self.use_rco,
        );
        let h = rows as f64 * 8.0;
        Rect::new(-16.0, -h / 2.0, 32.0, h)
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
        let id = self.alloc_unique_id("BinCounter");
        self.items
            .push(crate::canvas::Item::new(&id, x, y, BinCounter::default()));
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

fn bin_counter_rows(bits: usize, bidirectional: bool, parallel_in: bool, use_rco: bool) -> usize {
    let b = bits.max(1);
    let left_count = 2 + usize::from(bidirectional) + usize::from(parallel_in) * (1 + b);
    let right_count = b + usize::from(use_rco) * (1 + usize::from(bidirectional));
    (left_count.max(right_count) + 2).max(6)
}

fn bin_counter_pins(
    id: &str,
    bits: usize,
    bidirectional: bool,
    parallel_in: bool,
    use_rco: bool,
) -> Vec<Pin> {
    let b = bits.max(1);
    let h = bin_counter_rows(b, bidirectional, parallel_in, use_rco);
    let half_h = (h as f64 / 2.0) * 8.0;
    let mut pins = Vec::with_capacity(b * 2 + 6);
    // Left pins
    let mut left_y = -half_h + 8.0;
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_clk"),
        item_id: id.to_string(),
        local: Point::new(-24.0, left_y),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: ">".into(),
        unused: false,
    });
    left_y += 8.0;
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_rst"),
        item_id: id.to_string(),
        local: Point::new(-24.0, left_y),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "Rst".into(),
        unused: false,
    });
    left_y += 8.0;
    if bidirectional {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-Pin_dir"),
            item_id: id.to_string(),
            local: Point::new(-24.0, left_y),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "Dir".into(),
            unused: false,
        });
        left_y += 8.0;
    }
    if parallel_in {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-Pin_ldp"),
            item_id: id.to_string(),
            local: Point::new(-24.0, left_y),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "LD".into(),
            unused: false,
        });
        left_y += 8.0;
        for i in 0..b {
            pins.push(Pin {
                direction: Some(PinDirection::In),
                id: format!("{id}-in{}", 10 + i),
                item_id: id.to_string(),
                local: Point::new(-24.0, left_y + (i as f64) * 8.0),
                angle: 180,
                length: 8.0,
                is_bus: false,
                label: format!("D{i}"),
                unused: false,
            });
        }
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
