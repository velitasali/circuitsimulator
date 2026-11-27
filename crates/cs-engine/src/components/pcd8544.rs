//! PCD8544 (Nokia 5110) 84x48 monochrome LCD display.

use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_CONTRAST: i64 = 0;
const MAX_CONTRAST: i64 = 127;
const MIN_BIAS: i64 = 0;
const MAX_BIAS: i64 = 7;

impl crate::canvas::Item {
    pub fn pcd8544(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(id, x, y, Pcd8544::default())
    }

    pub fn pcd8544_display(id: impl Into<String>, x: f64, y: f64, contrast: u8, bias: u8) -> Self {
        Self::new(id, x, y, Pcd8544 { contrast, bias })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pcd8544 {
    pub contrast: u8,
    pub bias: u8,
}

impl Default for Pcd8544 {
    fn default() -> Self {
        Self {
            contrast: 50,
            bias: 4,
        }
    }
}

impl Pcd8544 {
    pub const TYPE_ID: &'static str = "Pcd8544";
    pub const WIDTH: usize = 84;
    pub const HEIGHT: usize = 48;

    pub fn disp_width(&self) -> usize {
        Self::WIDTH
    }

    pub fn disp_height(&self) -> usize {
        Self::HEIGHT
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Pcd8544(crate::digital::Pcd8544State::new(
            "",
            self.contrast,
            self.bias,
        ))
    }

    fn get_contrast(&self) -> PropValue {
        PropValue::Int(self.contrast as i64)
    }
    fn set_contrast(&mut self, v: PropValue) -> Result<(), PropError> {
        self.contrast = expect_int("Contrast", v)?.clamp(MIN_CONTRAST, MAX_CONTRAST) as u8;
        Ok(())
    }

    fn get_bias(&self) -> PropValue {
        PropValue::Int(self.bias as i64)
    }
    fn set_bias(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bias = expect_int("Bias", v)?.clamp(MIN_BIAS, MAX_BIAS) as u8;
        Ok(())
    }
}

impl Component for Pcd8544 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    const DESCRIPTION: &'static str = "84x48 pixel monochrome LCD (Nokia 5110).";

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Pcd8544>] = &[
            PropDef::int(
                "Contrast",
                "Contrast",
                MIN_CONTRAST,
                MAX_CONTRAST,
                Pcd8544::get_contrast,
                Pcd8544::set_contrast,
            )
            .with_info("LCD panel contrast level (0-127)."),
            PropDef::int(
                "Bias",
                "Bias",
                MIN_BIAS,
                MAX_BIAS,
                Pcd8544::get_bias,
                Pcd8544::set_bias,
            )
            .with_info("LCD bias voltage ratio configuration."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::in_pin("-PinRst", -32.0, 40.0, 270, 8.0).with_label("RST"),
            CompPin::in_pin("-PinCs", -16.0, 40.0, 270, 8.0).with_label("CS"),
            CompPin::in_pin("-PinDc", 0.0, 40.0, 270, 8.0).with_label("D/C"),
            CompPin::in_pin("-PinSi", 16.0, 40.0, 270, 8.0).with_label("DIN"),
            CompPin::in_pin("-PinScl", 32.0, 40.0, 270, 8.0).with_label("CLK"),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-52.0, -52.0, 104.0, 84.0)
    }
}

impl Stampable for Pcd8544 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl super::drawable::Drawable for Pcd8544 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // PCB backing
        d.fill_round_rect(-52.0, -52.0, 104.0, 84.0, 2.0, Color::rgb(50, 70, 100));
        d.stroke_round_rect(
            -52.0,
            -52.0,
            104.0,
            84.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Frame
        d.fill_round_rect(-48.0, -48.0, 96.0, 60.0, 8.0, Color::rgb(200, 220, 180));
        d.stroke_round_rect(-48.0, -48.0, 96.0, 60.0, 8.0, ctx.pal.border.fade(0.4), 1.0);

        // Screen glass
        let sx = -42.0;
        let sy = -42.0;
        let reading = ctx.canvas.readings().get(ctx.item_id);
        let hex = reading.map(|r| r.text.as_str()).unwrap_or("");
        let (contrast, bias) = reading
            .and_then(|r| parse_contrast_bias(&r.extra))
            .unwrap_or((self.contrast, self.bias));
        let (fg, bg) = pcd8544_colors(contrast, bias);
        super::mono_fb::paint_mono_screen(
            d,
            sx,
            sy,
            84.0,
            48.0,
            84,
            6,
            hex,
            fg,
            bg,
            Color::rgb(140, 160, 144),
            1.0,
            false,
        );
        true
    }
}

fn parse_contrast_bias(extra: &str) -> Option<(u8, u8)> {
    let (c, b) = extra.split_once(',')?;
    let contrast = c.parse::<u8>().ok()?.min(MAX_CONTRAST as u8);
    let bias = b.parse::<u8>().ok()?.min(MAX_BIAS as u8);
    Some((contrast, bias))
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgb(
        lerp_u8(a.r, b.r, t),
        lerp_u8(a.g, b.g, t),
        lerp_u8(a.b, b.b, t),
    )
}

/// Compute LCD foreground and glass background colors from PCD8544 `contrast` (`0..=127`)
/// and `bias` (`0..=7`, nominal `4` = 1:48 multiplex ratio).
pub fn pcd8544_colors(contrast: u8, bias: u8) -> (Color, Color) {
    let bg_base = Color::rgb(200, 215, 180);
    let fg_dark = Color::rgb(28, 44, 32);

    let bias_offset = (bias.min(7) as f64 - 4.0) * 8.0;
    let eff = (contrast.min(127) as f64 + bias_offset).clamp(0.0, 127.0);

    let fg_t = (eff / 60.0).clamp(0.0, 1.0);
    let fg = lerp_color(bg_base, fg_dark, fg_t);

    let bg_t = ((eff - 75.0) / 100.0).clamp(0.0, 0.55);
    let bg = lerp_color(bg_base, fg_dark, bg_t);

    (fg, bg)
}

impl crate::canvas::Scene {
    pub fn add_pcd8544(&mut self, x: f64, y: f64) -> String {
        let id = self.alloc_unique_id("PCD8544");
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Pcd8544::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    #[test]
    fn default_pcd8544() {
        let p = Pcd8544::default();
        assert_eq!(p.type_id(), "Pcd8544");
        assert_eq!(p.contrast, 50);
        assert_eq!(p.bias, 4);
        let pins = p.pin_geoms();
        assert_eq!(pins.len(), 5);
        assert_eq!(pins[0].local, Point::new(-32.0, 40.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[4].local, Point::new(32.0, 40.0));
        assert_eq!(pins[4].angle, 270);
        assert_eq!(p.body(), Rect::new(-52.0, -52.0, 104.0, 84.0));
    }
}
