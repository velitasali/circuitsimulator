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
        Self::pcd8544_display(id, x, y, 50, 4)
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

    fn description(&self) -> &'static str {
        "84x48 pixel monochrome LCD (Nokia 5110)."
    }

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
        d.fill_rect(sx, sy, 84.0, 48.0, Color::rgb(200, 215, 180));
        d.stroke_rect(sx, sy, 84.0, 48.0, Color::rgb(140, 160, 144), 1.0);

        let pixel_color = Color::rgb(28, 44, 32);

        if let Some(reading) = ctx.canvas.readings().get(ctx.item_id) {
            if !reading.text.is_empty() {
                let hex_bytes = reading.text.as_bytes();
                for page in 0..6 {
                    let page_offset = page * 84 * 2;
                    let py_base = sy + (page * 8) as f64;
                    for col in 0..84 {
                        let idx = page_offset + col * 2;
                        if idx + 2 <= hex_bytes.len() {
                            let hi = hex_nibble(hex_bytes[idx]);
                            let lo = hex_nibble(hex_bytes[idx + 1]);
                            let val = (hi << 4) | lo;
                            if val == 0 {
                                continue;
                            }
                            let px = sx + col as f64;
                            for bit in 0..8 {
                                if (val & (1 << bit)) != 0 {
                                    d.fill_rect(px, py_base + bit as f64, 1.0, 1.0, pixel_color);
                                }
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

#[inline]
fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

impl crate::canvas::Scene {
    pub fn add_pcd8544(&mut self, x: f64, y: f64) -> String {
        let id = format!("PCD8544-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::pcd8544(&id, x, y));
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
