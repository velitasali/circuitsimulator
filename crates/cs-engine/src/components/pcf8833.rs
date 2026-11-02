//! PCF8833 132x132 color LCD controller.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::draw::{Align, Color, Draw, PaintCtx};
use crate::canvas::{ImageBuf, Item, Rect, Scene};
use crate::digital::Pcf8833State;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 4096;

impl Item {
    pub fn pcf8833(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(id, x, y, Pcf8833::default())
    }

    pub fn pcf8833_display(id: impl Into<String>, x: f64, y: f64, width: u32, height: u32) -> Self {
        Self::new(
            id,
            x,
            y,
            Pcf8833 {
                width,
                height,
                fb: Vec::new(),
                last_sync_seq: 0,
                image_cache: None,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pcf8833 {
    pub width: u32,
    pub height: u32,
    pub fb: Vec<u16>,
    pub last_sync_seq: u64,
    pub image_cache: Option<ImageBuf>,
}

impl Default for Pcf8833 {
    fn default() -> Self {
        Self {
            width: 132,
            height: 132,
            fb: Vec::new(),
            last_sync_seq: 0,
            image_cache: None,
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

    pub fn update_image_cache(&mut self) {
        let expected_len = (self.width as usize) * (self.height as usize);
        if self.fb.is_empty() || self.fb.len() != expected_len {
            self.image_cache = None;
            return;
        }
        let mut rgba = Vec::with_capacity(self.fb.len() * 4);
        for &pixel in &self.fb {
            let r5 = ((pixel >> 11) & 0x1F) as u8;
            let g6 = ((pixel >> 5) & 0x3F) as u8;
            let b5 = (pixel & 0x1F) as u8;
            let r8 = (r5 << 3) | (r5 >> 2);
            let g8 = (g6 << 2) | (g6 >> 4);
            let b8 = (b5 << 3) | (b5 >> 2);
            rgba.extend_from_slice(&[r8, g8, b8, 255]);
        }
        self.image_cache = ImageBuf::from_rgba(self.width, self.height, rgba);
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Pcf8833Display {
            width: self.width,
            height: self.height,
            pcf: Pcf8833State::new(self.width, self.height),
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

impl Drawable for Pcf8833 {
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
        if let Some(ref img) = self.image_cache {
            d.draw_pixmap_rect(-w / 2.0, -h / 2.0, w, h, img, 1.0);
        }
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

impl Scene {
    pub fn add_pcf8833(&mut self, x: f64, y: f64) -> String {
        let id = format!("PCF8833-{}", self.items.len() + 1);
        self.items.push(Item::new(&id, x, y, Pcf8833::default()));
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
