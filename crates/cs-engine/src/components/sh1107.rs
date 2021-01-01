//! SH1107 128x128 monochrome OLED display.

use super::component::PropGroup;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 4096;
const MIN_ADDR: i64 = 0;
const MAX_ADDR: i64 = 127;
const MIN_FREQ_KHZ: f64 = 1.0;
const MAX_FREQ_KHZ: f64 = 1000.0;

impl crate::canvas::Item {
    pub fn sh1107(id: impl Into<String>, x: f64, y: f64, width: u32, height: u32) -> Self {
        Self::sh1107_display(id, x, y, width, height)
    }

    pub fn sh1107_display(id: impl Into<String>, x: f64, y: f64, width: u32, height: u32) -> Self {
        Self::new(
            id,
            x,
            y,
            Sh1107 {
                width,
                height,
                x_offset: true,
                control_code: 0x3C,
                freq_khz: 100.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sh1107 {
    pub width: u32,
    pub height: u32,
    pub x_offset: bool,
    pub control_code: u8,
    pub freq_khz: f64,
}

impl Default for Sh1107 {
    fn default() -> Self {
        Self {
            width: 128,
            height: 128,
            x_offset: true,
            control_code: 0x3C,
            freq_khz: 100.0,
        }
    }
}

impl Sh1107 {
    pub const TYPE_ID: &'static str = "Sh1107";
    pub fn disp_width(&self) -> usize {
        self.width as usize
    }

    pub fn disp_height(&self) -> usize {
        self.height as usize
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Sh1107(crate::digital::Sh1107State::new(
            "",
            self.width as usize,
            self.height as usize,
            self.control_code,
            self.x_offset,
            self.freq_khz,
        ))
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

    fn get_x_offset(&self) -> PropValue {
        PropValue::Bool(self.x_offset)
    }
    fn set_x_offset(&mut self, v: PropValue) -> Result<(), PropError> {
        self.x_offset = expect_bool("Xoffset", v)?;
        Ok(())
    }

    fn get_control_code(&self) -> PropValue {
        PropValue::Int(self.control_code as i64)
    }
    fn set_control_code(&mut self, v: PropValue) -> Result<(), PropError> {
        self.control_code = expect_int("Control_Code", v)?.clamp(MIN_ADDR, MAX_ADDR) as u8;
        Ok(())
    }

    fn get_freq(&self) -> PropValue {
        PropValue::Float(self.freq_khz)
    }
    fn set_freq(&mut self, v: PropValue) -> Result<(), PropError> {
        self.freq_khz = expect_float("Frequency", v)?.clamp(MIN_FREQ_KHZ, MAX_FREQ_KHZ);
        Ok(())
    }
}

impl Component for Sh1107 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "128x128 monochrome OLED display."
    }

    fn props() -> &'static [PropDef<Self>] {
        const WIDTH: PropDef<Sh1107> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_DIM,
                MAX_DIM,
                Sh1107::get_width,
                Sh1107::set_width,
            )
            .with_info("Screen width in pixels.");
            p.structural = true;
            p
        };
        const HEIGHT: PropDef<Sh1107> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_DIM,
                MAX_DIM,
                Sh1107::get_height,
                Sh1107::set_height,
            )
            .with_info("Screen height in pixels.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Sh1107>] = &[
            WIDTH,
            HEIGHT,
            PropDef::bool("Xoffset", "X Offset", Sh1107::get_x_offset, Sh1107::set_x_offset)
                .with_info("Shift the display RAM columns by 96."),
            PropDef::int(
                "Control_Code",
                "I2C Address",
                MIN_ADDR,
                MAX_ADDR,
                Sh1107::get_control_code,
                Sh1107::set_control_code,
            )
            .with_info("Device address."),
            PropDef::float(
                "Frequency",
                "I2C Frequency",
                "_kHz",
                MIN_FREQ_KHZ,
                MAX_FREQ_KHZ,
                Sh1107::get_freq,
                Sh1107::set_freq,
            )
            .with_info(
                "It is better to be similar to I2C Master frequency, but not critical in most cases.",
            ),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Width", "Height", "Xoffset"]),
                ("I2C", &["Control_Code", "Frequency"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let h = self.height as f64;
        let pin_y = h / 2.0 + 16.0;
        vec![
            CompPin::openco_pin("-PinSck", -48.0, pin_y, 270, 8.0).with_label("SCL"),
            CompPin::openco_pin("-PinSda", -40.0, pin_y, 270, 8.0).with_label("SDA"),
        ]
    }

    fn body(&self) -> Rect {
        let w = self.width as f64;
        let h = self.height as f64;
        Rect::new(-70.0, -h / 2.0 - 16.0, w + 12.0, h + 24.0)
    }
}

impl Stampable for Sh1107 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl super::drawable::Drawable for Sh1107 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let w = self.width as f64;
        let h = self.height as f64;
        // Module casing
        d.fill_round_rect(
            -70.0,
            -h / 2.0 - 16.0,
            w + 12.0,
            h + 24.0,
            2.0,
            Color::rgb(50, 70, 100),
        );
        d.stroke_round_rect(
            -70.0,
            -h / 2.0 - 16.0,
            w + 12.0,
            h + 24.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        let screen_x = -64.0;
        let screen_y = -h / 2.0 - 10.0;
        // Deep-black OLED screen
        d.fill_rect(screen_x, screen_y, w, h, Color::rgb(0, 0, 0));
        d.stroke_rect(screen_x, screen_y, w, h, ctx.pal.border.fade(0.4), 1.0);

        if let Some(reading) = ctx.canvas.readings().get(ctx.item_id) {
            if !reading.text.is_empty() {
                let fg = Color::rgb(0, 170, 255);
                let hex_bytes = reading.text.as_bytes();
                let num_pages = (self.height as usize) / 8;
                let cols = self.width as usize;
                for page in 0..num_pages {
                    let page_offset = page * cols * 2;
                    let py_base = screen_y + (page * 8) as f64;
                    for col in 0..cols {
                        let idx = page_offset + col * 2;
                        if idx + 2 <= hex_bytes.len() {
                            let hi = hex_nibble(hex_bytes[idx]);
                            let lo = hex_nibble(hex_bytes[idx + 1]);
                            let val = (hi << 4) | lo;
                            if val == 0 {
                                continue;
                            }
                            let px = screen_x + col as f64;
                            if val == 0xFF {
                                d.fill_rect(px, py_base, 1.0, 8.0, fg);
                            } else {
                                for bit in 0..8 {
                                    if (val & (1 << bit)) != 0 {
                                        d.fill_rect(px, py_base + bit as f64, 1.0, 1.0, fg);
                                    }
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
    pub fn add_sh1107(&mut self, x: f64, y: f64, width: u32, height: u32) -> String {
        let id = format!("SH1107-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::sh1107(&id, x, y, width, height));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    #[test]
    fn default_sh1107() {
        let s = Sh1107::default();
        assert_eq!(s.type_id(), "Sh1107");
        assert_eq!(s.width, 128);
        assert_eq!(s.height, 128);
        assert_eq!(s.control_code, 0x3C);
        assert_eq!(s.freq_khz, 100.0);
        assert!(s.x_offset);
        let pins = s.pin_geoms();
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].local, Point::new(-48.0, 80.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[1].local, Point::new(-40.0, 80.0));
        assert_eq!(pins[1].angle, 270);
        assert_eq!(s.body(), Rect::new(-70.0, -80.0, 140.0, 152.0));
    }
}
