//! Static RAM / ROM memory block component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::MemoryState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 16;

/// Static RAM / ROM memory block.
#[derive(Clone, Debug, PartialEq)]
pub struct Memory {
    pub addr_bits: usize,
    pub data_bits: usize,
    pub is_rom: bool,
    pub data: Vec<u8>,
}

impl crate::canvas::Item {
    pub fn memory(
        id: impl Into<String>,
        x: f64,
        y: f64,
        addr_bits: usize,
        data_bits: usize,
        is_rom: bool,
        data: Vec<u8>,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Memory {
                addr_bits,
                data_bits,
                is_rom,
                data,
            },
        )
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            addr_bits: 8,
            data_bits: 8,
            is_rom: false,
            data: Vec::new(),
        }
    }
}

impl Memory {
    pub const TYPE_ID: &'static str = "Memory";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Memory(MemoryState::new(
            "",
            self.addr_bits,
            self.data_bits,
            self.is_rom,
        ))
    }

    fn get_addr_bits(&self) -> PropValue {
        PropValue::Int(self.addr_bits as i64)
    }
    fn set_addr_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.addr_bits =
            expect_int("AddrBits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }

    fn get_data_bits(&self) -> PropValue {
        PropValue::Int(self.data_bits as i64)
    }
    fn set_data_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.data_bits =
            expect_int("DataBits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }

    fn get_is_rom(&self) -> PropValue {
        PropValue::Bool(self.is_rom)
    }
    fn set_is_rom(&mut self, v: PropValue) -> Result<(), PropError> {
        self.is_rom = expect_bool("IsRom", v)?;
        Ok(())
    }
}

impl Component for Memory {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Static RAM / ROM Memory Block."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ADDR_BITS: PropDef<Memory> = {
            let mut p = PropDef::int(
                "AddrBits",
                "Address Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                Memory::get_addr_bits,
                Memory::set_addr_bits,
            )
            .with_info("Number of address select bits.");
            p.structural = true;
            p
        };
        const DATA_BITS: PropDef<Memory> = {
            let mut p = PropDef::int(
                "DataBits",
                "Data Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                Memory::get_data_bits,
                Memory::set_data_bits,
            )
            .with_info("Number of bits of data bus.\nThis determines the length of a memory word.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Memory>] = &[
            ADDR_BITS,
            DATA_BITS,
            PropDef::bool(
                "IsRom",
                "ROM / Persistent",
                Memory::get_is_rom,
                Memory::set_is_rom,
            )
            .with_info("Configure as Read-Only Memory (ROM) or Random-Access Memory (RAM)."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        memory_pins("", self.addr_bits, self.data_bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let mut h = self.addr_bits + 1;
        if self.data_bits > h {
            h = self.data_bits;
        }
        let height = h + 2;
        let orig_y = -((height / 2) as f64) * 8.0;
        Rect::new(-16.0, orig_y, 32.0, (height as f64) * 8.0)
    }
}

impl Stampable for Memory {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Memory {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "RAM", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_memory(&mut self, x: f64, y: f64) -> String {
        let id = format!("Memory-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::memory(
            &id,
            x,
            y,
            8,
            8,
            false,
            Vec::new(),
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_memory() {
        let m = Memory::default();
        assert_eq!(m.type_id(), "Memory");
        assert_eq!(m.addr_bits, 8);
        assert_eq!(m.data_bits, 8);
        assert!(!m.is_rom);
        // addr (8) + we + oe + data (8) + cs = 19 pins
        assert_eq!(m.pin_geoms().len(), 19);
    }

    #[test]
    fn set_props() {
        let mut m = Memory::default();
        m.set_prop("AddrBits", PropValue::Int(4)).unwrap();
        m.set_prop("DataBits", PropValue::Int(4)).unwrap();
        m.set_prop("IsRom", PropValue::Bool(true)).unwrap();
        assert_eq!(m.addr_bits, 4);
        assert_eq!(m.data_bits, 4);
        assert!(m.is_rom);
        // 4 addr + we + oe + 4 data + cs = 11 pins
        assert_eq!(m.pin_geoms().len(), 11);
    }
}

fn memory_pins(id: &str, addr_bits: usize, data_bits: usize) -> Vec<Pin> {
    let mut h = addr_bits + 1;
    if data_bits > h {
        h = data_bits;
    }
    let height = h + 2;
    let orig_y = -((height / 2) as f64) * 8.0;
    let mut pins = Vec::with_capacity(addr_bits + data_bits + 3);

    // Left pins: Address inputs
    for i in 0..addr_bits {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, orig_y + 8.0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("A{i}"),
            unused: false,
        });
    }
    // Left pins: WE and OE (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_We"),
        item_id: id.to_string(),
        local: Point::new(-24.0, orig_y + (h as f64) * 8.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "WE".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_outEnable"),
        item_id: id.to_string(),
        local: Point::new(-24.0, orig_y + 8.0 + (h as f64) * 8.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "OE".into(),
        unused: false,
    });

    // Right pins: Data bits
    for i in 0..data_bits {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, orig_y + 8.0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: format!("D{i}"),
            unused: false,
        });
    }
    // Right pins: CS (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_Cs"),
        item_id: id.to_string(),
        local: Point::new(24.0, orig_y + 8.0 + (h as f64) * 8.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "CS".into(),
        unused: false,
    });

    pins
}
