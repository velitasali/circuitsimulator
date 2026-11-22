//! Mux: digital multiplexer.

use super::drawable::{Drawable, paint_trapezoid_body};
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
const TRAPEZOID_TAPER: f64 = 4.0;

fn mux_dims(addr_bits: usize) -> (usize, f64, f64) {
    let addr_bits = addr_bits.clamp(MIN_ADDR_BITS, MAX_ADDR_BITS);
    let channels = 1 << addr_bits;
    let half_w = if addr_bits >= 5 { 24.0 } else { 16.0 };
    let h = ((channels + 2) as f64) * 4.0;
    (channels, half_w, h)
}

fn mux_trapezoid(addr_bits: usize) -> [[f64; 2]; 4] {
    let (_, half_w, h) = mux_dims(addr_bits);
    [
        [-half_w, -h - 6.0],
        [half_w, -h + 2.0],
        [half_w, h - 2.0],
        [-half_w, h + 6.0],
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mux {
    pub addr_bits: usize,
}

impl crate::canvas::Item {
    pub fn mux(id: impl Into<String>, x: f64, y: f64, addr_bits: usize) -> Self {
        let mut mux = Mux::default();
        mux.addr_bits = addr_bits;
        Self::new(id, x, y, mux)
    }
}

impl Default for Mux {
    fn default() -> Self {
        Self { addr_bits: 3 }
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

    const DESCRIPTION: &'static str = "Digital multiplexer.";

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
        let (_, half_w, h) = mux_dims(self.addr_bits);
        Rect::new(-half_w, -h - 6.0, half_w * 2.0, (h + 6.0) * 2.0)
    }
}

impl Stampable for Mux {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Mux {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_trapezoid_body(d, ctx.pal, &mux_trapezoid(self.addr_bits), "MUX", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_mux(&mut self, x: f64, y: f64) -> String {
        let id = self.alloc_unique_id("Mux");
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Mux::default()));
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
        assert_eq!(m.addr_bits, 3);
        // 8 inputs + 3 addr + 1 enable + 2 outputs = 14 pins
        assert_eq!(m.pin_geoms().len(), 14);
    }

    #[test]
    fn all_pins_end_on_trapezoid_border() {
        for addr_bits in MIN_ADDR_BITS..=MAX_ADDR_BITS {
            let (_, half_w, h) = mux_dims(addr_bits);
            let pins = mux_pins("M", addr_bits);
            for pin in &pins {
                match pin.angle {
                    180 => {
                        let end_x = pin.local.x + pin.length;
                        assert!(
                            (end_x - (-half_w)).abs() < 1e-9,
                            "addr_bits={addr_bits} {}: left pin end_x={end_x} != {}",
                            pin.id,
                            -half_w
                        );
                        assert!(pin.local.y > -h - 6.0 && pin.local.y < h + 6.0);
                    }
                    0 => {
                        let end_x = pin.local.x - pin.length;
                        assert!(
                            (end_x - half_w).abs() < 1e-9,
                            "addr_bits={addr_bits} {}: right pin end_x={end_x} != {half_w}",
                            pin.id
                        );
                        assert!(pin.local.y > -h + 2.0 && pin.local.y < h - 2.0);
                    }
                    90 => {
                        let end_y = pin.local.y + pin.length;
                        let border_y = (-h - 2.0) + TRAPEZOID_TAPER * (pin.local.x / half_w);
                        assert!(
                            (end_y - border_y).abs() < 1e-9,
                            "addr_bits={addr_bits} {}: top pin end_y={end_y} != {border_y}",
                            pin.id
                        );
                        assert!(pin.local.x > -half_w && pin.local.x < half_w);
                    }
                    270 => {
                        let end_y = pin.local.y - pin.length;
                        let border_y = (h + 2.0) - TRAPEZOID_TAPER * (pin.local.x / half_w);
                        assert!(
                            (end_y - border_y).abs() < 1e-9,
                            "addr_bits={addr_bits} {}: bottom pin end_y={end_y} != {border_y}",
                            pin.id
                        );
                        assert!(
                            pin.local.x > -half_w && pin.local.x < half_w,
                            "addr_bits={addr_bits} {}: bottom pin x={} outside ({}, {})",
                            pin.id,
                            pin.local.x,
                            -half_w,
                            half_w
                        );
                        assert!(
                            pin.length >= 3.0,
                            "addr_bits={addr_bits} {}: bottom pin length={} too short",
                            pin.id,
                            pin.length
                        );
                    }
                    other => panic!("unexpected pin angle {other}"),
                }
            }
        }
    }
}

fn mux_pins(id: &str, addr_bits: usize) -> Vec<Pin> {
    let addr_bits = addr_bits.clamp(MIN_ADDR_BITS, MAX_ADDR_BITS);
    let (channels, half_w, h) = mux_dims(addr_bits);
    let num_inputs = channels;
    let mut pins = Vec::with_capacity(num_inputs + addr_bits + 3);
    let y0 = -((channels as f64) * 8.0) / 2.0 + 4.0;

    for i in 0..num_inputs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-half_w - 8.0, y0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: format!("D{i}"),
            unused: false,
        });
    }

    let span = 8.0 * (addr_bits as f64 - 1.0);
    let x0 = span / 2.0;
    for i in 0..addr_bits {
        let x = x0 - 8.0 * (i as f64);
        let length = 6.0 + TRAPEZOID_TAPER * (x / half_w);
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-addr{i}"),
            item_id: id.to_string(),
            local: Point::new(x, h + 8.0),
            angle: 270,
            length,
            is_bus: false,
            label: format!("S{i}"),
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
        local: Point::new(half_w + 8.0, -4.0),
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
        local: Point::new(half_w + 8.0, 4.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "W".into(),
        unused: false,
    });
    pins
}
