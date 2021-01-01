//! PCF8833 132x132 color LCD controller.

use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 4096;

impl crate::canvas::Item {
    pub fn pcf8833(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::pcf8833_display(id, x, y, 132, 132)
    }

    pub fn pcf8833_display(id: impl Into<String>, x: f64, y: f64, width: u32, height: u32) -> Self {
        Self::new(id, x, y, Pcf8833 { width, height })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pcf8833 {
    pub width: u32,
    pub height: u32,
}

impl Default for Pcf8833 {
    fn default() -> Self {
        Self {
            width: 132,
            height: 132,
        }
    }
}

impl Pcf8833 {
    pub const TYPE_ID: &'static str = "Pcf8833";
    pub fn disp_width(&self) -> usize {
        self.width as usize
    }

    pub fn disp_height(&self) -> usize {
        self.height as usize
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Pcf8833Display {
            width: self.width,
            height: self.height,
        }
    }

    fn get_width(&self) -> PropValue {
        PropValue::Int(self.width as i64)
    }
    fn set_width(&mut self, v: PropValue) -> Result<(), PropError> {
        self.width = expect_int("Width", v)?.clamp(MIN_DIM, MAX_DIM) as u32;
        Ok(())
    }

    fn get_height(&self) -> PropValue {
        PropValue::Int(self.height as i64)
    }
    fn set_height(&mut self, v: PropValue) -> Result<(), PropError> {
        self.height = expect_int("Height", v)?.clamp(MIN_DIM, MAX_DIM) as u32;
        Ok(())
    }
}

impl Component for Pcf8833 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "132x132 color LCD controller."
    }

    fn props() -> &'static [PropDef<Self>] {
        const WIDTH: PropDef<Pcf8833> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_DIM,
                MAX_DIM,
                Pcf8833::get_width,
                Pcf8833::set_width,
            )
            .with_info("Width in pixels or grid units.");
            p.structural = true;
            p
        };
        const HEIGHT: PropDef<Pcf8833> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_DIM,
                MAX_DIM,
                Pcf8833::get_height,
                Pcf8833::set_height,
            )
            .with_info("Height in pixels or grid units.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Pcf8833>] = &[WIDTH, HEIGHT];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let h = self.height as f64;
        let pin_y = h / 2.0 + 22.0;
        vec![
            CompPin::in_pin("-PinCS", -12.0, pin_y, 270, 8.0).with_label("CS"),
            CompPin::in_pin("-PinCLK", -4.0, pin_y, 270, 8.0).with_label("CK"),
            CompPin::in_pin("-PinDATA", 4.0, pin_y, 270, 8.0).with_label("DI"),
            CompPin::in_pin("-PinRESET", 12.0, pin_y, 270, 8.0).with_label("RS"),
        ]
    }

    fn body(&self) -> Rect {
        let w = self.width as f64;
        let h = self.height as f64;
        Rect::new(-w / 2.0 - 6.0, -h / 2.0 - 6.0, w + 12.0, h + 20.0)
    }
}

impl Stampable for Pcf8833 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Color, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl super::drawable::Drawable for Pcf8833 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let w = self.width as f64;
        let h = self.height as f64;
        // PCB frame
        d.fill_round_rect(
            -w / 2.0 - 6.0,
            -h / 2.0 - 6.0,
            w + 12.0,
            h + 20.0,
            3.0,
            Color::rgb(40, 55, 80),
        );
        d.stroke_round_rect(
            -w / 2.0 - 6.0,
            -h / 2.0 - 6.0,
            w + 12.0,
            h + 20.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Pin 1 indicator dot
        d.fill_circle(-w / 2.0 - 3.0, -h / 2.0 - 3.0, 1.5, Color::rgb(10, 10, 10));

        // Screen glass
        d.fill_rect(-w / 2.0, -h / 2.0, w, h, Color::rgb(6, 10, 18));
        d.stroke_rect(-w / 2.0, -h / 2.0, w, h, Color::rgb(35, 50, 70), 1.0);

        d.text(
            0.0,
            h / 2.0 + 3.0,
            "PCF8833 132x132",
            7.0,
            Color::rgb(160, 190, 220),
            Align::Center,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_pcf8833(&mut self, x: f64, y: f64) -> String {
        let id = format!("PCF8833-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::pcf8833(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    #[test]
    fn default_pcf8833() {
        let p = Pcf8833::default();
        assert_eq!(p.type_id(), "Pcf8833");
        assert_eq!(p.width, 132);
        assert_eq!(p.height, 132);
        let pins = p.pin_geoms();
        assert_eq!(pins.len(), 4);
        assert_eq!(pins[0].local, Point::new(-12.0, 88.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[3].local, Point::new(12.0, 88.0));
        assert_eq!(pins[3].angle, 270);
        assert_eq!(p.body(), Rect::new(-72.0, -72.0, 144.0, 152.0));
    }
}
