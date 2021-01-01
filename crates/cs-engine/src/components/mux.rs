//! Mux: digital multiplexer.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::MuxState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_ADDR_BITS: usize = 1;
const MAX_ADDR_BITS: usize = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct Mux {
    pub addr_bits: usize,
}

impl crate::canvas::Item {
    pub fn mux(id: impl Into<String>, x: f64, y: f64, addr_bits: usize) -> Self {
        Self::new(id, x, y, Mux { addr_bits })
    }
}

impl Default for Mux {
    fn default() -> Self {
        Self { addr_bits: 2 }
    }
}

impl Mux {
    pub const TYPE_ID: &'static str = "Mux";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Mux(MuxState::new("", self.addr_bits))
    }

    fn get_addr_bits(&self) -> PropValue {
        PropValue::Int(self.addr_bits as i64)
    }
    fn set_addr_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.addr_bits =
            expect_int("AddrBits", v)?.clamp(MIN_ADDR_BITS as i64, MAX_ADDR_BITS as i64) as usize;
        Ok(())
    }
}

impl Component for Mux {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Digital multiplexer."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ADDR_BITS: PropDef<Mux> = {
            let mut p = PropDef::int(
                "AddrBits",
                "Address Bits",
                MIN_ADDR_BITS as i64,
                MAX_ADDR_BITS as i64,
                Mux::get_addr_bits,
                Mux::set_addr_bits,
            )
            .with_info("Number of address select bits.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Mux>] = &[ADDR_BITS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        mux_pins("", self.addr_bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let channels = 1 << self.addr_bits.clamp(1, 4);
        let height = channels + 2;
        let h = (height as f64) * 4.0;
        Rect::new(-16.0, -h - 6.0, 32.0, (h + 6.0) * 2.0)
    }
}

impl Stampable for Mux {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Mux {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "MUX", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_mux(&mut self, x: f64, y: f64) -> String {
        let id = format!("Mux-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::mux(&id, x, y, 3));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mux() {
        let m = Mux::default();
        assert_eq!(m.type_id(), "Mux");
        assert_eq!(m.addr_bits, 2);
        // 4 inputs + 2 addr + 1 enable + 2 outputs = 9 pins
        assert_eq!(m.pin_geoms().len(), 9);
    }
}

fn mux_pins(id: &str, addr_bits: usize) -> Vec<Pin> {
    let num_inputs = 1 << addr_bits.clamp(1, 4);
    let mut pins = Vec::with_capacity(num_inputs + addr_bits + 3);
    let channels = num_inputs;
    let height = channels + 2;
    let h = (height as f64) * 4.0;
    let y0 = -((channels as f64) * 8.0) / 2.0 + 4.0;

    for i in 0..num_inputs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, y0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("D{i}"),
            unused: false,
        });
    }

    if addr_bits == 1 {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr0"),
            item_id: id.to_string(),
            local: Point::new(0.0, h + 8.0),
            angle: 270,
            length: 6.0,
            is_bus: false,
            label: "S0".into(),
            unused: false,
        });
    } else if addr_bits == 2 {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr0"),
            item_id: id.to_string(),
            local: Point::new(4.0, h + 8.0),
            angle: 270,
            length: 7.0,
            is_bus: false,
            label: "S0".into(),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr1"),
            item_id: id.to_string(),
            local: Point::new(-4.0, h + 8.0),
            angle: 270,
            length: 5.0,
            is_bus: false,
            label: "S1".into(),
            unused: false,
        });
    } else {
        // addr_bits >= 3 (default 3: S0 at +8, S1 at 0, S2 at -8)
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr0"),
            item_id: id.to_string(),
            local: Point::new(8.0, h + 8.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: "S0".into(),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr1"),
            item_id: id.to_string(),
            local: Point::new(0.0, h + 8.0),
            angle: 270,
            length: 6.0,
            is_bus: false,
            label: "S1".into(),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr2"),
            item_id: id.to_string(),
            local: Point::new(-8.0, h + 8.0),
            angle: 270,
            length: 4.0,
            is_bus: false,
            label: "S2".into(),
            unused: false,
        });
    }

    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-enable"),
        item_id: id.to_string(),
        local: Point::new(0.0, -h - 8.0),
        angle: 90,
        length: 6.0,
        is_bus: false,
        label: "OE".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::Out),
        id: format!("{id}-out"),
        item_id: id.to_string(),
        local: Point::new(24.0, -4.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "Y".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::Out),
        id: format!("{id}-out_inv"),
        item_id: id.to_string(),
        local: Point::new(24.0, 4.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "W".into(),
        unused: false,
    });
    pins
}
