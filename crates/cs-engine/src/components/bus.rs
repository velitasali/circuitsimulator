//! Multi-line bus connector.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_WIDTH: i64 = 1;
const MAX_WIDTH: i64 = 64;

impl crate::canvas::Item {
    pub fn bus(id: impl Into<String>, x: f64, y: f64, width: usize) -> Self {
        Self::new(id, x, y, Bus { width })
    }
}

/// Bus connector with dynamic line count.
#[derive(Clone, Debug, PartialEq)]
pub struct Bus {
    pub width: usize,
}

impl Default for Bus {
    fn default() -> Self {
        Self { width: 8 }
    }
}

impl Bus {
    pub const TYPE_ID: &'static str = "Bus";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Bus { width: self.width }
    }

    fn get_width(&self) -> PropValue {
        PropValue::Int(self.width as i64)
    }
    fn set_width(&mut self, v: PropValue) -> Result<(), PropError> {
        self.width = expect_int("Width", v)?.clamp(MIN_WIDTH, MAX_WIDTH) as usize;
        Ok(())
    }
}

impl Component for Bus {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Bus connection."
    }

    fn props() -> &'static [PropDef<Self>] {
        const WIDTH: PropDef<Bus> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_WIDTH,
                MAX_WIDTH,
                Bus::get_width,
                Bus::set_width,
            )
            .with_info("Width in pixels or grid units.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Bus>] = &[WIDTH];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let lines = self.width.max(1);
        let mut pins = Vec::with_capacity(lines + 2);
        pins.push(CompPin::new("-ePin0", 0.0, 0.0, 90, 1.0));
        for i in 1..=lines {
            let y = -8.0 * (lines as f64) + (i as f64) * 8.0;
            pins.push(CompPin::new(format!("-ePin{i}"), -8.0, y, 180, 8.0));
        }
        pins.push(CompPin::new(
            "-busPinI",
            0.0,
            -((lines - 1) as f64) * 8.0,
            270,
            1.0,
        ));
        pins
    }

    fn body(&self) -> Rect {
        let lines = self.width.max(1);
        let m_height = (lines - 1) as f64;
        Rect::new(-3.0, -m_height * 8.0 - 2.0, 6.0, m_height * 8.0 + 4.0)
    }
}

impl Stampable for Bus {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Bus {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let lines = self.width.max(1);
        let h = ((lines - 1) as f64) * 8.0;
        d.line(0.0, -h, 0.0, 0.0, ctx.pal.border, 3.0);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_bus(&mut self, x: f64, y: f64) -> String {
        let id = format!("Bus-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::bus(&id, x, y, 8));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bus() {
        let b = Bus::default();
        assert_eq!(b.type_id(), "Bus");
        assert_eq!(b.width, 8);
        assert_eq!(b.pin_geoms().len(), 10); // 8 lines + 2 trunk pins
    }

    #[test]
    fn bus_width_changes_pins() {
        let mut b = Bus::default();
        let ch = b.set_prop("Width", PropValue::Int(4)).unwrap();
        assert!(ch.structural);
        assert_eq!(b.width, 4);
        assert_eq!(b.pin_geoms().len(), 6); // 4 lines + 2 trunk pins
    }

    #[test]
    fn bus_element_kind() {
        let b = Bus { width: 16 };
        match b.to_element_kind() {
            Kind::Bus { width } => assert_eq!(width, 16),
            other => panic!("expected Kind::Bus, got {other:?}"),
        }
    }
}
