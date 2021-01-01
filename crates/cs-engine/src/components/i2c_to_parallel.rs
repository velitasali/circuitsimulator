//! I2C to 8-Bit Parallel I/O Expander (PCF8574).

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::I2CToParallelState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

/// I2C to 8-Bit Parallel I/O Expander (PCF8574).
#[derive(Clone, Debug, PartialEq)]
pub struct I2CToParallel {
    pub address: u8,
}

impl crate::canvas::Item {
    pub fn i2c_to_parallel(id: impl Into<String>, x: f64, y: f64, address: u8) -> Self {
        Self::new(id, x, y, I2CToParallel { address })
    }
}

impl Default for I2CToParallel {
    fn default() -> Self {
        Self { address: 0x20 }
    }
}

impl I2CToParallel {
    pub const TYPE_ID: &'static str = "I2CToParallel";
    pub fn to_element_kind(&self) -> Kind {
        Kind::I2CToParallel(I2CToParallelState::new("", self.address))
    }

    fn get_address(&self) -> PropValue {
        PropValue::Int(self.address as i64)
    }
    fn set_address(&mut self, v: PropValue) -> Result<(), PropError> {
        self.address = expect_int("Address", v)?.clamp(0, 127) as u8;
        Ok(())
    }
}

impl Component for I2CToParallel {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "I2C to 8-Bit Parallel I/O Expander (PCF8574)."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<I2CToParallel>] = &[PropDef::int(
            "Address",
            "I2C Address",
            0,
            127,
            I2CToParallel::get_address,
            I2CToParallel::set_address,
        )
        .with_info("7-bit I2C slave device address.")];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        i2c_to_parallel_pins("")
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -32.0, 32.0, 72.0)
    }
}

impl Stampable for I2CToParallel {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for I2CToParallel {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "PCF8574", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_i2c_to_parallel(&mut self, x: f64, y: f64) -> String {
        let id = format!("I2CToParallel-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::i2c_to_parallel(&id, x, y, 0x20));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_i2c_to_parallel() {
        let p = I2CToParallel::default();
        assert_eq!(p.type_id(), "I2CToParallel");
        assert_eq!(p.address, 0x20);
        // 6 left + 8 right = 14 pins
        assert_eq!(p.pin_geoms().len(), 14);
        assert_eq!(p.body(), Rect::new(-16.0, -32.0, 32.0, 72.0));
    }

    #[test]
    fn set_address() {
        let mut p = I2CToParallel::default();
        p.set_prop("Address", PropValue::Int(0x27)).unwrap();
        assert_eq!(p.address, 0x27);
    }
}

fn i2c_to_parallel_pins(id: &str) -> Vec<Pin> {
    let mut pins = Vec::with_capacity(14);
    // Left inputs (angle 180, x = -24.0): SDA (-24), SCL (-16), INT (0), A0 (16), A1 (24), A2 (32)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in0"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -24.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "SDA".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in1"),
        item_id: id.to_string(),
        local: Point::new(-24.0, -16.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "SCL".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in5"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 0.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "INT".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in2"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 16.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "A0".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in3"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 24.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "A1".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in4"),
        item_id: id.to_string(),
        local: Point::new(-24.0, 32.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "A2".into(),
        unused: false,
    });
    // Right outputs (angle 0, x = 24.0): D0..D7
    for i in 0..8 {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, -24.0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: format!("D{i}"),
            unused: false,
        });
    }
    pins
}
