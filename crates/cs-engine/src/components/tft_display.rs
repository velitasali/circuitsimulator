//! Color TFT LCD display with SPI interface.

use std::fmt;

use super::component::PropGroup;
use super::drawable::Drawable;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::draw::{Align, Color, Draw, PaintCtx};
use crate::canvas::{ImageBuf, Item, Rect, Scene};
use crate::digital::TftState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 4096;
const MIN_SCALE: f64 = 0.1;
const MAX_SCALE: f64 = 10.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum TftController {
    #[default]
    Ili9341,
    St7735,
    St7789,
    Ssd1283a,
    Ssd1351,
    Gc9a01a,
}

impl TftController {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ili9341 => "ILI9341",
            Self::St7735 => "ST7735",
            Self::St7789 => "ST7789",
            Self::Ssd1283a => "SSD1283A",
            Self::Ssd1351 => "SSD1351",
            Self::Gc9a01a => "GC9A01A",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "st7735" => Self::St7735,
            "st7789" => Self::St7789,
            "ssd1283a" => Self::Ssd1283a,
            "ssd1351" => Self::Ssd1351,
            "gc9a01a" => Self::Gc9a01a,
            _ => Self::Ili9341,
        }
    }
}

impl fmt::Display for TftController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const TFT_CONTROLLER_OPTIONS: &[&str] = &[
    "ILI9341", "ST7735", "ST7789", "SSD1283A", "SSD1351", "GC9A01A",
];

impl Item {
    pub fn tft_display(
        id: impl Into<String>,
        x: f64,
        y: f64,
        controller: impl AsRef<str>,
        width: u32,
        height: u32,
    ) -> Self {
        let mut display = TftDisplay::default();
        display.controller = TftController::from_str_name(controller.as_ref());
        display.width = width;
        display.height = height;
        Self::new(id, x, y, display)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tft_display_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        controller: impl AsRef<str>,
        width: u32,
        height: u32,
        scale: f64,
        bgr: bool,
    ) -> Self {
        let mut display = TftDisplay::default();
        display.controller = TftController::from_str_name(controller.as_ref());
        display.width = width;
        display.height = height;
        display.scale = scale;
        display.bgr = bgr;
        Self::new(id, x, y, display)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TftDisplay {
    pub controller: TftController,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub bgr: bool,
    pub fb: Vec<u16>,
    pub last_sync_seq: u64,
    pub image_cache: Option<ImageBuf>,
}

impl Default for TftDisplay {
    fn default() -> Self {
        Self {
            controller: TftController::Ili9341,
            width: 240,
            height: 320,
            scale: 1.0,
            bgr: false,
            fb: Vec::new(),
            last_sync_seq: 0,
            image_cache: None,
        }
    }
}

impl TftDisplay {
    pub const TYPE_ID: &'static str = "TftDisplay";
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
        if self.controller == TftController::Gc9a01a {
            let cx = self.width as f64 / 2.0;
            let cy = self.height as f64 / 2.0;
            for (i, &pixel) in self.fb.iter().enumerate() {
                let px = (i % (self.width as usize)) as f64 + 0.5;
                let py = (i / (self.width as usize)) as f64 + 0.5;
                let nx = (px - cx) / cx.max(0.5);
                let ny = (py - cy) / cy.max(0.5);
                if nx * nx + ny * ny > 1.0 {
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                } else {
                    let r5 = ((pixel >> 11) & 0x1F) as u8;
                    let g6 = ((pixel >> 5) & 0x3F) as u8;
                    let b5 = (pixel & 0x1F) as u8;
                    let r8 = (r5 << 3) | (r5 >> 2);
                    let g8 = (g6 << 2) | (g6 >> 4);
                    let b8 = (b5 << 3) | (b5 >> 2);
                    if self.bgr {
                        rgba.extend_from_slice(&[b8, g8, r8, 255]);
                    } else {
                        rgba.extend_from_slice(&[r8, g8, b8, 255]);
                    }
                }
            }
        } else {
            for &pixel in &self.fb {
                let r5 = ((pixel >> 11) & 0x1F) as u8;
                let g6 = ((pixel >> 5) & 0x3F) as u8;
                let b5 = (pixel & 0x1F) as u8;
                let r8 = (r5 << 3) | (r5 >> 2);
                let g8 = (g6 << 2) | (g6 >> 4);
                let b8 = (b5 << 3) | (b5 >> 2);
                if self.bgr {
                    rgba.extend_from_slice(&[b8, g8, r8, 255]);
                } else {
                    rgba.extend_from_slice(&[r8, g8, b8, 255]);
                }
            }
        }
        self.image_cache = ImageBuf::from_rgba(self.width, self.height, rgba);
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::TftDisplay {
            controller: self.controller.as_str().to_string(),
            width: self.width,
            height: self.height,
            scale: self.scale,
            bgr: self.bgr,
            tft: TftState::new(self.width, self.height, self.bgr),
        }
    }

    fn get_controller(&self) -> PropValue {
        PropValue::Enum(self.controller.as_str().to_string())
    }
    fn set_controller(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Controller", v)?;
        self.controller = TftController::from_str_name(&s);
        Ok(())
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

    fn get_scale(&self) -> PropValue {
        PropValue::Float(self.scale)
    }
    fn set_scale(&mut self, v: PropValue) -> Result<(), PropError> {
        self.scale = expect_float("Scale", v)?.clamp(MIN_SCALE, MAX_SCALE);
        Ok(())
    }

    fn get_bgr(&self) -> PropValue {
        PropValue::Bool(self.bgr)
    }
    fn set_bgr(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bgr = expect_bool("BGR", v)?;
        Ok(())
    }
}

impl Component for TftDisplay {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Color TFT LCD display with SPI interface."
    }

    fn props() -> &'static [PropDef<Self>] {
        const WIDTH: PropDef<TftDisplay> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_DIM,
                MAX_DIM,
                TftDisplay::get_width,
                TftDisplay::set_width,
            )
            .with_info("Width in pixels or grid units.");
            p.structural = true;
            p
        };
        const HEIGHT: PropDef<TftDisplay> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_DIM,
                MAX_DIM,
                TftDisplay::get_height,
                TftDisplay::set_height,
            )
            .with_info("Height in pixels or grid units.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<TftDisplay>] = &[
            PropDef::enumeration(
                "Controller",
                "Controller",
                TFT_CONTROLLER_OPTIONS,
                TftDisplay::get_controller,
                TftDisplay::set_controller,
            )
            .with_info("Graphics display driver controller type (e.g. ST7735, ILI9341)."),
            WIDTH,
            HEIGHT,
            PropDef::float(
                "Scale",
                "Scale",
                "",
                MIN_SCALE,
                MAX_SCALE,
                TftDisplay::get_scale,
                TftDisplay::set_scale,
            )
            .with_info("Scale factor applied when rendering the screen."),
            PropDef::bool("BGR", "BGR Color", TftDisplay::get_bgr, TftDisplay::set_bgr)
                .with_info("Color byte order (true for BGR, false for RGB)."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        vec![PropGroup::new("Main", self.prop_rows())]
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let sh = (self.height as f64 * self.scale).max(32.0);
        let pin_y = sh / 2.0 + 24.0;
        vec![
            CompPin::in_pin("-PinDC", -20.0, pin_y, 270, 8.0).with_label("DC"),
            CompPin::in_pin("-PinRS", -12.0, pin_y, 270, 8.0).with_label("RS"),
            CompPin::in_pin("-PinCS", -4.0, pin_y, 270, 8.0).with_label("CS"),
            CompPin::in_pin("-PinDI", 4.0, pin_y, 270, 8.0).with_label("DI"),
            CompPin::in_pin("-PinCK", 12.0, pin_y, 270, 8.0).with_label("CK"),
            CompPin::out_pin("-PinDO", 20.0, pin_y, 270, 8.0).with_label("DO"),
        ]
    }

    fn body(&self) -> Rect {
        let sw = (self.width as f64 * self.scale).max(32.0);
        let sh = (self.height as f64 * self.scale).max(32.0);
        Rect::new(-sw / 2.0 - 6.0, -sh / 2.0 - 6.0, sw + 12.0, sh + 22.0)
    }
}

impl Stampable for TftDisplay {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for TftDisplay {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let sw = (self.width as f64 * self.scale).max(32.0);
        let sh = (self.height as f64 * self.scale).max(32.0);
        let pcb_color = Color::rgb(50, 70, 100);
        let screen_color = Color::rgb(4, 6, 12);
        let screen_border = Color::rgb(30, 45, 60);

        if self.controller == TftController::Gc9a01a {
            let r = sw / 2.0;
            d.fill_round_rect(
                -sw * 3.0 / 8.0,
                0.0,
                sw * 0.75,
                sh / 2.0 + 16.0,
                5.0,
                pcb_color,
            );
            d.stroke_round_rect(
                -sw * 3.0 / 8.0,
                0.0,
                sw * 0.75,
                sh / 2.0 + 16.0,
                5.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            d.fill_circle(0.0, 0.0, r + 6.0, pcb_color);
            d.stroke_circle(0.0, 0.0, r + 6.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.fill_circle(0.0, 0.0, r, screen_color);
            if let Some(ref img) = self.image_cache {
                d.draw_pixmap_rect(-sw / 2.0, -sh / 2.0, sw, sh, img, 1.0);
            }
            d.stroke_circle(0.0, 0.0, r, screen_border, 1.0);
        } else {
            // Rectangular TFT (ILI9341, ST7789, ST7735, etc.)
            d.fill_round_rect(
                -sw / 2.0 - 6.0,
                -sh / 2.0 - 6.0,
                sw + 12.0,
                sh + 22.0,
                3.0,
                pcb_color,
            );
            d.stroke_round_rect(
                -sw / 2.0 - 6.0,
                -sh / 2.0 - 6.0,
                sw + 12.0,
                sh + 22.0,
                3.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            // Pin 1 indicator dot
            d.fill_circle(
                -sw / 2.0 - 3.0,
                -sh / 2.0 - 3.0,
                1.5,
                Color::rgb(10, 10, 10),
            );
            // Screen glass
            d.fill_rect(-sw / 2.0, -sh / 2.0, sw, sh, screen_color);
            if let Some(ref img) = self.image_cache {
                d.draw_pixmap_rect(-sw / 2.0, -sh / 2.0, sw, sh, img, 1.0);
            }
            d.stroke_rect(-sw / 2.0, -sh / 2.0, sw, sh, screen_border, 1.0);
            // Label
            d.text(
                0.0,
                sh / 2.0 + 4.0,
                self.controller.as_str(),
                7.0,
                Color::rgb(180, 200, 220),
                Align::Center,
            );
        }
        true
    }
}

impl Scene {
    pub fn add_tft_display(
        &mut self,
        x: f64,
        y: f64,
        controller: &str,
        width: u32,
        height: u32,
    ) -> String {
        let id = format!("{}-{}", controller, self.items.len() + 1);
        let mut display = TftDisplay::default();
        display.controller = TftController::from_str_name(controller);
        display.width = width;
        display.height = height;
        self.items.push(Item::new(&id, x, y, display));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    #[test]
    fn default_tft_display() {
        let d = TftDisplay::default();
        assert_eq!(d.type_id(), "TftDisplay");
        assert_eq!(d.controller, TftController::Ili9341);
        assert_eq!(d.width, 240);
        assert_eq!(d.height, 320);
        assert_eq!(d.scale, 1.0);
        assert!(!d.bgr);
        let pins = d.pin_geoms();
        assert_eq!(pins.len(), 6);
        assert_eq!(pins[0].local, Point::new(-20.0, 184.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[5].local, Point::new(20.0, 184.0));
        assert_eq!(pins[5].angle, 270);
        assert_eq!(d.body(), Rect::new(-126.0, -166.0, 252.0, 342.0));
    }
}
