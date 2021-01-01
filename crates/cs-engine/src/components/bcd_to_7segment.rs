//! BCD to 7-segment display decoder.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::BcdTo7SState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

/// BCD to 7-segment display decoder.
#[derive(Clone, Debug, PartialEq)]
pub struct BcdTo7Segment {
    pub common_anode: bool,
}

impl crate::canvas::Item {
    pub fn bcd_to_7s(id: impl Into<String>, x: f64, y: f64, common_anode: bool) -> Self {
        Self::new(id, x, y, BcdTo7Segment { common_anode })
    }
}

impl Default for BcdTo7Segment {
    fn default() -> Self {
        Self {
            common_anode: false,
        }
    }
}

impl BcdTo7Segment {
    pub const TYPE_ID: &'static str = "BcdTo7Segment";
    pub fn to_element_kind(&self) -> Kind {
        Kind::BcdTo7S(BcdTo7SState::new("", self.common_anode))
    }

    fn get_common_anode(&self) -> PropValue {
        PropValue::Bool(self.common_anode)
    }
    fn set_common_anode(&mut self, v: PropValue) -> Result<(), PropError> {
        self.common_anode = expect_bool("CommonAnode", v)?;
        Ok(())
    }
}

impl Component for BcdTo7Segment {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "BCD to 7-Segment Decoder."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<BcdTo7Segment>] = &[PropDef::bool(
            "CommonAnode",
            "Common Anode",
            BcdTo7Segment::get_common_anode,
            BcdTo7Segment::set_common_anode,
        )
        .with_info("Determines common anode (true) or common cathode (false) pin configuration.")];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        bcd_to_7s_pins("", false)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -32.0, 32.0, 64.0)
    }
}

impl Stampable for BcdTo7Segment {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for BcdTo7Segment {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "BCD-7S", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_bcd_to_7s(&mut self, x: f64, y: f64) -> String {
        let id = format!("BcdTo7S-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::bcd_to_7s(&id, x, y, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bcd_to_7segment() {
        let b = BcdTo7Segment::default();
        assert_eq!(b.type_id(), "BcdTo7Segment");
        assert!(!b.common_anode);
        // 4 in + 1 OE + 7 out = 12 pins
        assert_eq!(b.pin_geoms().len(), 12);
    }

    #[test]
    fn common_anode_toggle() {
        let mut b = BcdTo7Segment::default();
        b.set_prop("CommonAnode", PropValue::Bool(true)).unwrap();
        assert!(b.common_anode);
    }
}

fn bcd_to_7s_pins(id: &str, use_reset: bool) -> Vec<Pin> {
    let mut pins = Vec::with_capacity(13);
    // Inputs (left side, angle 180): S0, S1, S2, S3
    let in_labels = ["S0", "S1", "S2", "S3"];
    let in_y = [-8.0, 0.0, 8.0, 16.0];
    for (i, (lbl, y)) in in_labels.iter().zip(in_y.iter()).enumerate() {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, *y),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: lbl.to_string(),
            unused: false,
        });
    }
    // Output Enable (top, angle 90)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in4"),
        item_id: id.to_string(),
        local: Point::new(-8.0, -40.0),
        angle: 90,
        length: 8.0,
        is_bus: false,
        label: "OE".into(),
        unused: false,
    });
    // Optional Reset Pin (left, angle 180)
    if use_reset {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-rst"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -16.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "RST".into(),
            unused: false,
        });
    }
    // Outputs (right side, angle 0): a, b, c, d, e, f, g
    let seg_labels = ["a", "b", "c", "d", "e", "f", "g"];
    for (i, lbl) in seg_labels.iter().enumerate() {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, -24.0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: lbl.to_string(),
            unused: false,
        });
    }
    pins
}
