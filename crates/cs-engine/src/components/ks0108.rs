//! KS0108 128x64 graphic LCD module.

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 4096;

impl crate::canvas::Item {
    pub fn ks0108(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::ks0108_display(id, x, y, 128, 64)
    }

    pub fn ks0108_display(id: impl Into<String>, x: f64, y: f64, width: u32, height: u32) -> Self {
        Self::new(
            id,
            x,
            y,
            Ks0108 {
                width,
                height,
                cs_act_low: false,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ks0108 {
    pub width: u32,
    pub height: u32,
    pub cs_act_low: bool,
}

impl Default for Ks0108 {
    fn default() -> Self {
        Self {
            width: 128,
            height: 64,
            cs_act_low: false,
        }
    }
}

impl Ks0108 {
    pub const TYPE_ID: &'static str = "Ks0108";
    pub fn disp_width(&self) -> usize {
        self.width as usize
    }

    pub fn disp_height(&self) -> usize {
        self.height as usize
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Ks0108(crate::digital::Ks0108State::new("", self.cs_act_low))
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

    fn get_cs_act_low(&self) -> PropValue {
        PropValue::Bool(self.cs_act_low)
    }
    fn set_cs_act_low(&mut self, v: PropValue) -> Result<(), PropError> {
        self.cs_act_low = expect_bool("CS_Active_Low", v)?;
        Ok(())
    }
}

impl Component for Ks0108 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "128x64 graphical LCD module."
    }

    fn props() -> &'static [PropDef<Self>] {
        const WIDTH: PropDef<Ks0108> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_DIM,
                MAX_DIM,
                Ks0108::get_width,
                Ks0108::set_width,
            )
            .with_info("Width in pixels or grid units.");
            p.structural = true;
            p
        };
        const HEIGHT: PropDef<Ks0108> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_DIM,
                MAX_DIM,
                Ks0108::get_height,
                Ks0108::set_height,
            )
            .with_info("Height in pixels or grid units.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Ks0108>] = &[
            WIDTH,
            HEIGHT,
            PropDef::bool(
                "CS_Active_Low",
                "CS Active Low",
                Ks0108::get_cs_act_low,
                Ks0108::set_cs_act_low,
            )
            .with_info("Whether the CS pin is active low."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::in_pin("-PinRst", -56.0, 56.0, 270, 8.0).with_label("RST"),
            CompPin::in_pin("-PinCs2", -48.0, 56.0, 270, 8.0).with_label("CS2"),
            CompPin::in_pin("-PinCs1", -40.0, 56.0, 270, 8.0).with_label("CS1"),
            CompPin::in_pin("-dataPin7", -32.0, 56.0, 270, 8.0).with_label("D7"),
            CompPin::in_pin("-dataPin6", -24.0, 56.0, 270, 8.0).with_label("D6"),
            CompPin::in_pin("-dataPin5", -16.0, 56.0, 270, 8.0).with_label("D5"),
            CompPin::in_pin("-dataPin4", -8.0, 56.0, 270, 8.0).with_label("D4"),
            CompPin::in_pin("-dataPin3", 0.0, 56.0, 270, 8.0).with_label("D3"),
            CompPin::in_pin("-dataPin2", 8.0, 56.0, 270, 8.0).with_label("D2"),
            CompPin::in_pin("-dataPin1", 16.0, 56.0, 270, 8.0).with_label("D1"),
            CompPin::in_pin("-dataPin0", 24.0, 56.0, 270, 8.0).with_label("D0"),
            CompPin::in_pin("-PinEn", 32.0, 56.0, 270, 8.0).with_label("En"),
            CompPin::in_pin("-PinRW", 40.0, 56.0, 270, 8.0).with_label("RW"),
            CompPin::in_pin("-PinDC", 48.0, 56.0, 270, 8.0).with_label("RS"),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-74.0, -52.0, 148.0, 100.0)
    }
}

impl Stampable for Ks0108 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl super::drawable::Drawable for Ks0108 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // PCB backing
        d.fill_round_rect(-74.0, -52.0, 148.0, 100.0, 2.0, Color::rgb(50, 70, 100));
        d.stroke_round_rect(
            -74.0,
            -52.0,
            148.0,
            100.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Frame
        d.fill_round_rect(-70.0, -48.0, 140.0, 76.0, 8.0, Color::rgb(200, 220, 180));
        d.stroke_round_rect(
            -70.0,
            -48.0,
            140.0,
            76.0,
            8.0,
            ctx.pal.border.fade(0.4),
            1.0,
        );

        // Screen glass
        let sx = -64.0;
        let sy = -42.0;
        d.fill_rect(sx, sy, 128.0, 64.0, Color::rgb(200, 215, 180));
        d.stroke_rect(sx, sy, 128.0, 64.0, Color::rgb(140, 160, 144), 1.0);

        let pixel_color = Color::rgb(28, 44, 32);

        if let Some(reading) = ctx.canvas.readings().get(ctx.item_id) {
            if !reading.text.is_empty() {
                let hex_bytes = reading.text.as_bytes();
                for page in 0..8 {
                    let page_offset = page * 128 * 2;
                    let py_base = sy + (page * 8) as f64;
                    for col in 0..128 {
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
    pub fn add_ks0108(&mut self, x: f64, y: f64) -> String {
        let id = format!("KS0108-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::ks0108(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    #[test]
    fn default_ks0108() {
        let k = Ks0108::default();
        assert_eq!(k.type_id(), "Ks0108");
        assert_eq!(k.width, 128);
        assert_eq!(k.height, 64);
        assert!(!k.cs_act_low);
        let pins = k.pin_geoms();
        assert_eq!(pins.len(), 14);
        assert_eq!(pins[0].local, Point::new(-56.0, 56.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[13].local, Point::new(48.0, 56.0));
        assert_eq!(pins[13].angle, 270);
        assert_eq!(k.body(), Rect::new(-74.0, -52.0, 148.0, 100.0));
    }
}
