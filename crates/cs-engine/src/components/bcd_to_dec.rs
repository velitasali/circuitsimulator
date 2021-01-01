//! BCD to decimal (or 4-to-16) decoder.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::BcdToDecState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

/// BCD to decimal / 4-to-16 line decoder.
#[derive(Clone, Debug, PartialEq)]
pub struct BcdToDec {
    pub sixteen: bool,
    pub active_low: bool,
}

impl crate::canvas::Item {
    pub fn bcd_to_dec(
        id: impl Into<String>,
        x: f64,
        y: f64,
        sixteen: bool,
        active_low: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            BcdToDec {
                sixteen,
                active_low,
            },
        )
    }
}

impl Default for BcdToDec {
    fn default() -> Self {
        Self {
            sixteen: false,
            active_low: false,
        }
    }
}

impl BcdToDec {
    pub const TYPE_ID: &'static str = "BcdToDec";
    pub fn to_element_kind(&self) -> Kind {
        Kind::BcdToDec(BcdToDecState::new("", self.sixteen, self.active_low))
    }

    fn get_sixteen(&self) -> PropValue {
        PropValue::Bool(self.sixteen)
    }
    fn set_sixteen(&mut self, v: PropValue) -> Result<(), PropError> {
        self.sixteen = expect_bool("Sixteen", v)?;
        Ok(())
    }

    fn get_active_low(&self) -> PropValue {
        PropValue::Bool(self.active_low)
    }
    fn set_active_low(&mut self, v: PropValue) -> Result<(), PropError> {
        self.active_low = expect_bool("ActiveLow", v)?;
        Ok(())
    }
}

impl Component for BcdToDec {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "BCD to Decimal Decoder."
    }

    fn props() -> &'static [PropDef<Self>] {
        const SIXTEEN: PropDef<BcdToDec> = {
            let mut p = PropDef::bool(
                "Sixteen",
                "16 Outputs",
                BcdToDec::get_sixteen,
                BcdToDec::set_sixteen,
            )
            .with_info("Enable 16-line (4-to-16 / 16-to-4) mode instead of 10-line (BCD) mode.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<BcdToDec>] = &[
            SIXTEEN,
            PropDef::bool(
                "ActiveLow",
                "Active Low",
                BcdToDec::get_active_low,
                BcdToDec::set_active_low,
            )
            .with_info("Active-low polarity."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        bcd_to_dec_pins("", self.sixteen)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = if self.sixteen { 16.0 * 8.0 } else { 11.0 * 8.0 };
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for BcdToDec {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for BcdToDec {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "BCD-DEC", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_bcd_to_dec(&mut self, x: f64, y: f64) -> String {
        let id = format!("BcdToDec-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::bcd_to_dec(&id, x, y, false, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bcd_to_dec() {
        let b = BcdToDec::default();
        assert_eq!(b.type_id(), "BcdToDec");
        assert!(!b.sixteen);
        assert!(!b.active_low);
        // 4 in + 1 OE + 10 out = 15 pins
        assert_eq!(b.pin_geoms().len(), 15);
    }

    #[test]
    fn sixteen_outputs() {
        let mut b = BcdToDec::default();
        b.set_prop("Sixteen", PropValue::Bool(true)).unwrap();
        assert!(b.sixteen);
        // 4 in + 1 OE + 16 out = 21 pins
        assert_eq!(b.pin_geoms().len(), 21);
    }
}

fn bcd_to_dec_pins(id: &str, sixteen: bool) -> Vec<Pin> {
    let count = if sixteen { 16 } else { 10 };
    let mut pins = Vec::with_capacity(5 + count);
    // Left inputs: S0..S3
    for i in 0..4 {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -12.0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("S{i}"),
            unused: false,
        });
    }
    // Top pin: Output Enable (active low)
    let body_h = if sixteen { 16.0 * 8.0 } else { 11.0 * 8.0 };
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in4"),
        item_id: id.to_string(),
        local: Point::new(-8.0, -body_h / 2.0 - 8.0),
        angle: 90,
        length: 8.0,
        is_bus: false,
        label: "OE".into(),
        unused: false,
    });
    // Right outputs: 0..count-1
    let y0 = -((count as f64) / 2.0) * 8.0 + 4.0;
    for i in 0..count {
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
