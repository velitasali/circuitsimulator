//! Demux: digital demultiplexer.

use super::drawable::{Drawable, paint_trapezoid_body};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::DemuxState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_ADDR_BITS: usize = 1;
const MAX_ADDR_BITS: usize = 5;
const TRAPEZOID_TAPER: f64 = 4.0;

fn demux_dims(addr_bits: usize) -> (usize, f64, f64) {
    let addr_bits = addr_bits.clamp(MIN_ADDR_BITS, MAX_ADDR_BITS);
    let channels = 1 << addr_bits;
    let half_w = if addr_bits >= 5 { 24.0 } else { 16.0 };
    let h = ((channels + 2) as f64) * 4.0;
    (channels, half_w, h)
}

fn demux_trapezoid(addr_bits: usize) -> [[f64; 2]; 4] {
    let (_, half_w, h) = demux_dims(addr_bits);
    [
        [-half_w, -h + 2.0],
        [half_w, -h - 6.0],
        [half_w, h + 6.0],
        [-half_w, h - 2.0],
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct Demux {
    pub addr_bits: usize,
    pub inverted: bool,
}

impl crate::canvas::Item {
    pub fn demux(id: impl Into<String>, x: f64, y: f64, addr_bits: usize, inverted: bool) -> Self {
        let mut demux = Demux::default();
        demux.addr_bits = addr_bits;
        demux.inverted = inverted;
        Self::new(id, x, y, demux)
    }
}

impl Default for Demux {
    fn default() -> Self {
        Self {
            addr_bits: 3,
            inverted: false,
        }
    }
}

impl Demux {
    pub const TYPE_ID: &'static str = "Demux";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Demux(DemuxState::new("", self.addr_bits, self.inverted))
    }

    fn get_addr_bits(&self) -> PropValue {
        PropValue::Int(self.addr_bits as i64)
    }
    fn set_addr_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.addr_bits =
            expect_int("AddrBits", v)?.clamp(MIN_ADDR_BITS as i64, MAX_ADDR_BITS as i64) as usize;
        Ok(())
    }
    fn get_inverted(&self) -> PropValue {
        PropValue::Bool(self.inverted)
    }
    fn set_inverted(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inverted = expect_bool("Inverted", v)?;
        Ok(())
    }
}

impl Component for Demux {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    const DESCRIPTION: &'static str = "Digital demultiplexer.";

    fn props() -> &'static [PropDef<Self>] {
        const ADDR_BITS: PropDef<Demux> = {
            let mut p = PropDef::int(
                "AddrBits",
                "Address Bits",
                MIN_ADDR_BITS as i64,
                MAX_ADDR_BITS as i64,
                Demux::get_addr_bits,
                Demux::set_addr_bits,
            )
            .with_info("Number of address select bits.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Demux>] = &[
            ADDR_BITS,
            PropDef::bool(
                "Inverted",
                "Inverted",
                Demux::get_inverted,
                Demux::set_inverted,
            )
            .with_info("Invert output pins."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        demux_pins("", self.addr_bits)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let (_, half_w, h) = demux_dims(self.addr_bits);
        Rect::new(-half_w, -h - 6.0, half_w * 2.0, (h + 6.0) * 2.0)
    }
}

impl Stampable for Demux {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Demux {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_trapezoid_body(d, ctx.pal, &demux_trapezoid(self.addr_bits), "DEMUX", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_demux(&mut self, x: f64, y: f64) -> String {
        let id = self.alloc_unique_id("Demux");
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Demux::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_demux() {
        let d = Demux::default();
        assert_eq!(d.type_id(), "Demux");
        assert_eq!(d.addr_bits, 3);
        assert!(!d.inverted);
        // 1 in + 3 addr + 1 enable + 8 outputs = 13 pins
        assert_eq!(d.pin_geoms().len(), 13);
    }

    #[test]
    fn all_pins_end_on_trapezoid_border() {
        for addr_bits in MIN_ADDR_BITS..=MAX_ADDR_BITS {
            let (_, half_w, h) = demux_dims(addr_bits);
            let pins = demux_pins("D", addr_bits);
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
                        assert!(pin.local.y > -h + 2.0 && pin.local.y < h - 2.0);
                    }
                    0 => {
                        let end_x = pin.local.x - pin.length;
                        assert!(
                            (end_x - half_w).abs() < 1e-9,
                            "addr_bits={addr_bits} {}: right pin end_x={end_x} != {half_w}",
                            pin.id
                        );
                        assert!(pin.local.y > -h - 6.0 && pin.local.y < h + 6.0);
                    }
                    90 => {
                        let end_y = pin.local.y + pin.length;
                        let border_y = (-h - 2.0) - TRAPEZOID_TAPER * (pin.local.x / half_w);
                        assert!(
                            (end_y - border_y).abs() < 1e-9,
                            "addr_bits={addr_bits} {}: top pin end_y={end_y} != {border_y}",
                            pin.id
                        );
                        assert!(pin.local.x > -half_w && pin.local.x < half_w);
                    }
                    270 => {
                        let end_y = pin.local.y - pin.length;
                        let border_y = (h + 2.0) + TRAPEZOID_TAPER * (pin.local.x / half_w);
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

fn demux_pins(id: &str, addr_bits: usize) -> Vec<Pin> {
    let addr_bits = addr_bits.clamp(MIN_ADDR_BITS, MAX_ADDR_BITS);
    let (channels, half_w, h) = demux_dims(addr_bits);
    let num_outputs = channels;
    let mut pins = Vec::with_capacity(num_outputs + addr_bits + 2);
    let y0 = -((channels as f64) * 8.0) / 2.0 + 4.0;

    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-in"),
        item_id: id.to_string(),
        local: Point::new(-half_w - 8.0, 0.0),
        angle: 180,
        length: 8.0,
        is_bus: false,
        label: "D".into(),
        unused: false,
    });

    let span = 8.0 * (addr_bits as f64 - 1.0);
    let x0 = span / 2.0;
    for i in 0..addr_bits {
        let x = x0 - 8.0 * (i as f64);
        let length = 6.0 - TRAPEZOID_TAPER * (x / half_w);
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
    for i in 0..num_outputs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(half_w + 8.0, y0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: format!("D{i}"),
            unused: false,
        });
    }
    pins
}
