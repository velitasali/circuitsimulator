//! I2C serial EEPROM / RAM (24C04 compatible) component.

use super::drawable::{Drawable, paint_chip_body};
use super::memory::{decode_mem_hex, encode_mem_hex};
use super::props::{PropDef, PropError, PropValue, expect_int, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::I2CRamState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_SIZE: usize = 1;
const MAX_SIZE: usize = 65536;

/// I2C serial EEPROM / RAM (24C04 compatible).
#[derive(Clone, Debug, PartialEq)]
pub struct I2CRam {
    pub size_bytes: usize,
    pub dev_address: u8,
    pub data: Vec<u8>,
}

impl crate::canvas::Item {
    pub fn i2c_ram(
        id: impl Into<String>,
        x: f64,
        y: f64,
        size_bytes: usize,
        dev_address: u8,
        data: Vec<u8>,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            I2CRam {
                size_bytes,
                dev_address,
                data,
            },
        )
    }
}

impl Default for I2CRam {
    fn default() -> Self {
        Self {
            size_bytes: 65536,
            dev_address: 0x50,
            data: Vec::new(),
        }
    }
}

impl I2CRam {
    pub const TYPE_ID: &'static str = "I2CRam";
    pub fn to_element_kind(&self) -> Kind {
        let mut st = I2CRamState::new("", self.size_bytes, self.dev_address);
        let n = st.data.len().min(self.data.len());
        st.data[..n].copy_from_slice(&self.data[..n]);
        Kind::I2CRam(st)
    }

    fn get_size_bytes(&self) -> PropValue {
        PropValue::Int(self.size_bytes as i64)
    }
    fn set_size_bytes(&mut self, v: PropValue) -> Result<(), PropError> {
        self.size_bytes =
            expect_int("SizeBytes", v)?.clamp(MIN_SIZE as i64, MAX_SIZE as i64) as usize;
        Ok(())
    }

    fn get_dev_address(&self) -> PropValue {
        PropValue::Int(self.dev_address as i64)
    }
    fn set_dev_address(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dev_address = expect_int("DevAddress", v)?.clamp(0, 127) as u8;
        Ok(())
    }

    fn get_mem(&self) -> PropValue {
        PropValue::String(encode_mem_hex(&self.data))
    }
    fn set_mem(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Mem", v)?;
        self.data = decode_mem_hex(&s);
        Ok(())
    }
}

impl Component for I2CRam {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    const DESCRIPTION: &'static str = "I2C Serial EEPROM / RAM (24C04 compatible).";

    fn props() -> &'static [PropDef<Self>] {
        const MEM: PropDef<I2CRam> = {
            let mut p = PropDef::string("Mem", "Memory Data", I2CRam::get_mem, I2CRam::set_mem);
            p.show_by_default = false;
            p.persist = true;
            p
        };
        static PROPS: &[PropDef<I2CRam>] = &[
            PropDef::int(
                "SizeBytes",
                "Size (Bytes)",
                MIN_SIZE as i64,
                MAX_SIZE as i64,
                I2CRam::get_size_bytes,
                I2CRam::set_size_bytes,
            )
            .with_info("Size in bytes."),
            PropDef::int(
                "DevAddress",
                "I2C Address",
                0,
                127,
                I2CRam::get_dev_address,
                I2CRam::set_dev_address,
            )
            .with_info("7-bit I2C slave device address."),
            MEM,
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        i2c_ram_pins("").into_iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -16.0, 32.0, 32.0)
    }
}

impl Stampable for I2CRam {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for I2CRam {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "24Cxx", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_i2c_ram(&mut self, x: f64, y: f64) -> String {
        let id = self.alloc_unique_id("I2CRam");
        self.items
            .push(crate::canvas::Item::new(&id, x, y, I2CRam::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_i2c_ram() {
        let r = I2CRam::default();
        assert_eq!(r.type_id(), "I2CRam");
        assert_eq!(r.size_bytes, 65536);
        assert_eq!(r.dev_address, 0x50);
        // SDA, SCL, A0, A1, A2 = 5 pins
        assert_eq!(r.pin_geoms().len(), 5);
        assert_eq!(r.body(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    }

    #[test]
    fn set_props() {
        let mut r = I2CRam::default();
        r.set_prop("SizeBytes", PropValue::Int(512)).unwrap();
        r.set_prop("DevAddress", PropValue::Int(0x52)).unwrap();
        assert_eq!(r.size_bytes, 512);
        assert_eq!(r.dev_address, 0x52);
    }
}

fn i2c_ram_pins(id: &str) -> Vec<Pin> {
    vec![
        // Left: SDA, SCL (open collector)
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in0"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "SDA".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in1"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "SCL".into(),
            unused: false,
        },
        // Right: Address bits A0, A1, A2
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in2"),
            item_id: id.to_string(),
            local: Point::new(24.0, -8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "A0".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in3"),
            item_id: id.to_string(),
            local: Point::new(24.0, 0.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "A1".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in4"),
            item_id: id.to_string(),
            local: Point::new(24.0, 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "A2".into(),
            unused: false,
        },
    ]
}
