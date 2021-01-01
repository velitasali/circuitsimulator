//! Dynamic RAM (DRAM) memory block component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::DynamicMemoryState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_BITS: usize = 1;
const MAX_BITS: usize = 16;

/// Dynamic RAM (DRAM) memory block.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicMemory {
    pub addr_bits: usize,
    pub data: Vec<u8>,
}

impl crate::canvas::Item {
    pub fn dynamic_memory(
        id: impl Into<String>,
        x: f64,
        y: f64,
        addr_bits: usize,
        data: Vec<u8>,
    ) -> Self {
        Self::new(id, x, y, DynamicMemory { addr_bits, data })
    }
}

impl Default for DynamicMemory {
    fn default() -> Self {
        Self {
            addr_bits: 8,
            data: Vec::new(),
        }
    }
}

impl DynamicMemory {
    pub const TYPE_ID: &'static str = "DynamicMemory";
    pub fn to_element_kind(&self) -> Kind {
        Kind::DynamicMemory(DynamicMemoryState::new("", self.addr_bits))
    }

    fn get_addr_bits(&self) -> PropValue {
        PropValue::Int(self.addr_bits as i64)
    }
    fn set_addr_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.addr_bits =
            expect_int("AddrBits", v)?.clamp(MIN_BITS as i64, MAX_BITS as i64) as usize;
        Ok(())
    }
}

impl Component for DynamicMemory {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Dynamic RAM (DRAM) Memory Block."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ADDR_BITS: PropDef<DynamicMemory> = {
            let mut p = PropDef::int(
                "AddrBits",
                "Address Bits",
                MIN_BITS as i64,
                MAX_BITS as i64,
                DynamicMemory::get_addr_bits,
                DynamicMemory::set_addr_bits,
            )
            .with_info("Number of address select bits.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<DynamicMemory>] = &[ADDR_BITS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        dynamic_memory_pins("", self.addr_bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let data_bits = 8;
        let h = self.addr_bits.max(data_bits) + 1;
        let height = h + 2;
        let orig_y = -((height / 2) as f64) * 8.0;
        Rect::new(-16.0, orig_y, 32.0, (height as f64) * 8.0)
    }
}

impl Stampable for DynamicMemory {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for DynamicMemory {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "DRAM", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_dynamic_memory(&mut self, x: f64, y: f64) -> String {
        let id = format!("DynamicMemory-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::dynamic_memory(
            &id,
            x,
            y,
            8,
            Vec::new(),
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dynamic_memory() {
        let dm = DynamicMemory::default();
        assert_eq!(dm.type_id(), "DynamicMemory");
        assert_eq!(dm.addr_bits, 8);
        // 8 addr + we + oe + 8 data + ras + cas = 20 pins
        assert_eq!(dm.pin_geoms().len(), 20);
    }

    #[test]
    fn change_addr_bits() {
        let mut dm = DynamicMemory::default();
        dm.set_prop("AddrBits", PropValue::Int(10)).unwrap();
        assert_eq!(dm.addr_bits, 10);
        // 10 addr + we + oe + 8 data + ras + cas = 22 pins
        assert_eq!(dm.pin_geoms().len(), 22);
    }
}

fn dynamic_memory_pins(id: &str, addr_bits: usize) -> Vec<Pin> {
    let data_bits = 8;
    let h = addr_bits.max(data_bits) + 1;
    let height = h + 2;
    let orig_y = -((height / 2) as f64) * 8.0;
    let mut pins = Vec::with_capacity(addr_bits + data_bits + 4);

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
    // Right pins: RAS and CAS (active low)
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_Ras"),
        item_id: id.to_string(),
        local: Point::new(24.0, orig_y + (h as f64) * 8.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "RAS".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-Pin_Cas"),
        item_id: id.to_string(),
        local: Point::new(24.0, orig_y + 8.0 + (h as f64) * 8.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "CAS".into(),
        unused: false,
    });

    pins
}
