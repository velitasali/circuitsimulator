//! Color TFT LCD display with SPI interface.

use super::component::PropGroup;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

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

impl std::fmt::Display for TftController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const TFT_CONTROLLER_OPTIONS: &[&str] = &[
    "ILI9341", "ST7735", "ST7789", "SSD1283A", "SSD1351", "GC9A01A",
];

impl crate::canvas::Item {
    pub fn tft_display(
        id: impl Into<String>,
        x: f64,
        y: f64,
        controller: impl AsRef<str>,
        width: u32,
        height: u32,
    ) -> Self {
        Self::tft_display_with(id, x, y, controller, width, height, 1.0, false)
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
        Self::new(
            id,
            x,
            y,
            TftDisplay {
                controller: TftController::from_str_name(controller.as_ref()),
                width,
                height,
                scale,
                bgr,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TftDisplay {
    pub controller: TftController,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub bgr: bool,
}

impl Default for TftDisplay {
    fn default() -> Self {
        Self {
            controller: TftController::Ili9341,
            width: 320,
            height: 240,
            scale: 1.0,
            bgr: false,
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

    pub fn to_element_kind(&self) -> Kind {
        Kind::TftDisplay {
            controller: self.controller.as_str().to_string(),
            width: self.width,
            height: self.height,
            scale: self.scale,
            bgr: self.bgr,
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

use crate::canvas::draw::{Align, Color, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl super::drawable::Drawable for TftDisplay {
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

impl crate::canvas::Scene {
    pub fn add_tft_display(
        &mut self,
        x: f64,
        y: f64,
        controller: &str,
        width: u32,
        height: u32,
    ) -> String {
        let id = format!("{}-{}", controller, self.items.len() + 1);
        self.items.push(crate::canvas::Item::tft_display_with(
            &id, x, y, controller, width, height, 1.0, false,
        ));
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
        assert_eq!(d.width, 320);
        assert_eq!(d.height, 240);
        assert_eq!(d.scale, 1.0);
        assert!(!d.bgr);
        let pins = d.pin_geoms();
        assert_eq!(pins.len(), 6);
        assert_eq!(pins[0].local, Point::new(-20.0, 144.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[5].local, Point::new(20.0, 144.0));
        assert_eq!(pins[5].angle, 270);
        assert_eq!(d.body(), Rect::new(-166.0, -126.0, 332.0, 262.0));
    }
}
