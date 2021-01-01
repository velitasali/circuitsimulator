//! Multi-pin pin header connector.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_PINS: i64 = 1;
const MAX_PINS: i64 = 64;

impl crate::canvas::Item {
    pub fn header(id: impl Into<String>, x: f64, y: f64, pins_count: usize) -> Self {
        Self::new(id, x, y, Header { pins_count })
    }
}

/// Male pin header connector with dynamic pin count.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub pins_count: usize,
}

impl Default for Header {
    fn default() -> Self {
        Self { pins_count: 8 }
    }
}

impl Header {
    pub const TYPE_ID: &'static str = "Header";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Header {
            pins_count: self.pins_count,
        }
    }

    fn get_pins(&self) -> PropValue {
        PropValue::Int(self.pins_count as i64)
    }
    fn set_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        self.pins_count = expect_int("Pins", v)?.clamp(MIN_PINS, MAX_PINS) as usize;
        Ok(())
    }
}

impl Component for Header {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Pin header connector."
    }

    fn props() -> &'static [PropDef<Self>] {
        const PINS: PropDef<Header> = {
            let mut p = PropDef::int(
                "Pins",
                "Pins",
                MIN_PINS,
                MAX_PINS,
                Header::get_pins,
                Header::set_pins,
            )
            .with_info("Generate Pins");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Header>] = &[PINS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let n = self.pins_count.max(1);
        let mut pins = Vec::with_capacity(n * 2);
        let y0 = -4.0 * (n as f64 - 1.0);
        for i in 0..n {
            pins.push(CompPin::new(
                format!("-pin{i}"),
                -8.0,
                y0 + (i as f64) * 8.0,
                180,
                8.0,
            ));
        }
        for i in 0..n {
            pins.push(CompPin::new(
                format!("-pin{}", i + n),
                0.0,
                y0 + (i as f64) * 8.0,
                90,
                1.0,
            ));
        }
        pins
    }

    fn body(&self) -> Rect {
        let h = 8.0 * (self.pins_count as f64);
        Rect::new(-4.0, -h / 2.0, 8.0, h)
    }
}

impl Stampable for Header {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Header {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let n = self.pins_count.max(1);
        let h = 8.0 * n as f64;
        d.fill_rect(
            -4.0,
            -h / 2.0,
            8.0,
            h,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_rect(
            -4.0,
            -h / 2.0,
            8.0,
            h,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        let y0 = -4.0 * (n as f64 - 1.0);
        for i in 0..n {
            let py = y0 + i as f64 * 8.0;
            d.fill_circle(0.0, py, 2.5, ctx.pal.border);
            if i > 0 {
                let ly = -h / 2.0 + i as f64 * 8.0;
                d.line(-4.0, ly, 4.0, ly, ctx.pal.border, 0.5);
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_header(&mut self, x: f64, y: f64) -> String {
        let id = format!("Header-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::header(&id, x, y, 8));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_header() {
        let h = Header::default();
        assert_eq!(h.type_id(), "Header");
        assert_eq!(h.pins_count, 8);
        assert_eq!(h.pin_geoms().len(), 16); // 8 outer pins + 8 mating pins
    }

    #[test]
    fn header_pins_prop() {
        let mut h = Header::default();
        let ch = h.set_prop("Pins", PropValue::Int(12)).unwrap();
        assert!(ch.structural);
        assert_eq!(h.pins_count, 12);
        assert_eq!(h.pin_geoms().len(), 24);
    }

    #[test]
    fn header_element_kind() {
        let h = Header { pins_count: 6 };
        match h.to_element_kind() {
            Kind::Header { pins_count } => assert_eq!(pins_count, 6),
            other => panic!("expected Kind::Header, got {other:?}"),
        }
    }
}
