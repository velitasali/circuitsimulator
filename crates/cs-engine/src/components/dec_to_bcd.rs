//! Decimal to BCD priority encoder.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::DecToBcdState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

/// Decimal to BCD priority encoder.
#[derive(Clone, Debug, PartialEq)]
pub struct DecToBcd {
    pub sixteen: bool,
    pub active_low: bool,
}

impl crate::canvas::Item {
    pub fn dec_to_bcd(
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
            DecToBcd {
                sixteen,
                active_low,
            },
        )
    }
}

impl Default for DecToBcd {
    fn default() -> Self {
        Self {
            sixteen: false,
            active_low: false,
        }
    }
}

impl DecToBcd {
    pub const TYPE_ID: &'static str = "DecToBcd";
    pub fn to_element_kind(&self) -> Kind {
        Kind::DecToBcd(DecToBcdState::new("", self.sixteen, self.active_low))
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

impl Component for DecToBcd {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Decimal to BCD Priority Encoder."
    }

    fn props() -> &'static [PropDef<Self>] {
        const SIXTEEN: PropDef<DecToBcd> = {
            let mut p = PropDef::bool(
                "Sixteen",
                "16 Inputs",
                DecToBcd::get_sixteen,
                DecToBcd::set_sixteen,
            )
            .with_info("Enable 16-line (4-to-16 / 16-to-4) mode instead of 10-line (BCD) mode.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<DecToBcd>] = &[
            SIXTEEN,
            PropDef::bool(
                "ActiveLow",
                "Active Low",
                DecToBcd::get_active_low,
                DecToBcd::set_active_low,
            )
            .with_info("Active-low polarity."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        dec_to_bcd_pins("", self.sixteen)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = if self.sixteen { 16.0 * 8.0 } else { 11.0 * 8.0 };
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for DecToBcd {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for DecToBcd {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "DEC-BCD", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_dec_to_bcd(&mut self, x: f64, y: f64) -> String {
        let id = format!("DecToBcd-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::dec_to_bcd(&id, x, y, false, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dec_to_bcd() {
        let d = DecToBcd::default();
        assert_eq!(d.type_id(), "DecToBcd");
        assert!(!d.sixteen);
        assert!(!d.active_low);
        // 9 in + 1 OE + 4 out = 14 pins
        assert_eq!(d.pin_geoms().len(), 14);
    }

    #[test]
    fn sixteen_inputs() {
        let mut d = DecToBcd::default();
        d.set_prop("Sixteen", PropValue::Bool(true)).unwrap();
        assert!(d.sixteen);
        // 15 in + 1 OE + 4 out = 20 pins
        assert_eq!(d.pin_geoms().len(), 20);
    }
}

fn dec_to_bcd_pins(id: &str, sixteen: bool) -> Vec<Pin> {
    let count = if sixteen { 15 } else { 9 }; // D1..D9 or D1..D15 (0 is all-low state)
    let mut pins = Vec::with_capacity(count + 5);
    let body_h = if sixteen { 16.0 * 8.0 } else { 11.0 * 8.0 };
    let y0 = -((count as f64) / 2.0) * 8.0 + 4.0;
    // Left inputs: D1..D(count)
    for i in 0..count {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, y0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("D{}", i + 1),
            unused: false,
        });
    }
    // Top pin: Output Enable (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in15"),
        item_id: id.to_string(),
        local: Point::new(0.0, -body_h / 2.0 - 8.0),
        angle: 90,
        length: 8.0,
        is_bus: false,
        label: "OE".into(),
        unused: false,
    });
    // Right outputs: A, B, C, D (weights 8, 4, 2, 1)
    let out_labels = ["A", "B", "C", "D"];
    let out_y0 = -12.0;
    for (i, label) in out_labels.iter().enumerate() {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, out_y0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: (*label).to_string(),
            unused: false,
        });
    }
    pins
}
