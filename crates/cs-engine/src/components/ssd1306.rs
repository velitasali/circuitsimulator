//! SSD1306 128x64 OLED graphic display with I2C interface.

use super::component::PropGroup;
use super::drawable::Drawable;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Ssd1306Color {
    #[default]
    White,
    Blue,
    Yellow,
}

impl Ssd1306Color {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Blue => "Blue",
            Self::Yellow => "Yellow",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "blue" => Self::Blue,
            "yellow" => Self::Yellow,
            _ => Self::White,
        }
    }
}

impl std::fmt::Display for Ssd1306Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const SSD1306_COLOR_OPTIONS: &[&str] = &["White", "Blue", "Yellow"];
const MIN_WIDTH: i64 = 16;
const MAX_WIDTH: i64 = 256;
const MIN_HEIGHT: i64 = 16;
const MAX_HEIGHT: i64 = 128;
const MIN_ADDR: i64 = 0;
const MAX_ADDR: i64 = 127;
const MIN_FREQ_HZ: f64 = 1e3;
const MAX_FREQ_HZ: f64 = 10e6;

#[derive(Clone, Debug, PartialEq)]
pub struct Ssd1306 {
    pub color: Ssd1306Color,
    pub width: usize,
    pub height: usize,
    pub rotate: bool,
    pub control_code: u8,
    pub freq_khz: f64,
}

impl crate::canvas::Item {
    pub fn ssd1306(
        id: impl Into<String>,
        x: f64,
        y: f64,
        width: usize,
        height: usize,
        control_code: u8,
        color: impl AsRef<str>,
        rotate: bool,
        freq_khz: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Ssd1306 {
                color: Ssd1306Color::from_str_name(color.as_ref()),
                width,
                height,
                rotate,
                control_code,
                freq_khz,
            },
        )
    }
}

impl Default for Ssd1306 {
    fn default() -> Self {
        Self {
            color: Ssd1306Color::White,
            width: 128,
            height: 64,
            rotate: true,
            control_code: 0x3C,
            freq_khz: 100.0,
        }
    }
}

impl Ssd1306 {
    pub const TYPE_ID: &'static str = "Ssd1306";
    pub fn disp_width(&self) -> usize {
        self.width
    }

    pub fn disp_height(&self) -> usize {
        self.height
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Ssd1306(crate::digital::Ssd1306State::new(
            "",
            self.width,
            self.height,
            self.control_code,
            self.color.as_str(),
            self.rotate,
            self.freq_khz,
        ))
    }

    fn get_color(&self) -> PropValue {
        PropValue::Enum(self.color.as_str().to_string())
    }
    fn set_color(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Color", v)?;
        self.color = Ssd1306Color::from_str_name(&s);
        Ok(())
    }

    fn get_width(&self) -> PropValue {
        PropValue::Int(self.width as i64)
    }
    fn set_width(&mut self, v: PropValue) -> Result<(), PropError> {
        self.width = expect_int("Width", v)?.clamp(MIN_WIDTH, MAX_WIDTH) as usize;
        Ok(())
    }

    fn get_height(&self) -> PropValue {
        PropValue::Int(self.height as i64)
    }
    fn set_height(&mut self, v: PropValue) -> Result<(), PropError> {
        self.height = expect_int("Height", v)?.clamp(MIN_HEIGHT, MAX_HEIGHT) as usize;
        Ok(())
    }

    fn get_rotate(&self) -> PropValue {
        PropValue::Bool(self.rotate)
    }
    fn set_rotate(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rotate = expect_bool("Rotate", v)?;
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
        PropValue::Float(self.freq_khz * 1e3)
    }
    fn set_freq(&mut self, v: PropValue) -> Result<(), PropError> {
        let hz = expect_float("Frequency", v)?.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ);
        self.freq_khz = hz / 1e3;
        Ok(())
    }
}

impl Component for Ssd1306 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "128x64 OLED monochrome graphic display."
    }

    fn props() -> &'static [PropDef<Self>] {
        const HEIGHT: PropDef<Ssd1306> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_HEIGHT,
                MAX_HEIGHT,
                Ssd1306::get_height,
                Ssd1306::set_height,
            )
            .with_info("Screen height in pixels");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Ssd1306>] = &[
            PropDef::enumeration(
                "Color",
                "Color",
                SSD1306_COLOR_OPTIONS,
                Ssd1306::get_color,
                Ssd1306::set_color,
            )
            .with_info("Pixel color."),
            PropDef::int(
                "Width",
                "Width",
                MIN_WIDTH,
                MAX_WIDTH,
                Ssd1306::get_width,
                Ssd1306::set_width,
            ).with_info("Screen width in pixels"),
            HEIGHT,
            PropDef::bool("Rotate", "Rotate", Ssd1306::get_rotate, Ssd1306::set_rotate).with_info("Rotate screen 180º.\nSome libraries use screen rotation, others don't use it.\nUsing this you can keep the display in same position."),
            PropDef::int(
                "Control_Code",
                "I2C Address",
                MIN_ADDR,
                MAX_ADDR,
                Ssd1306::get_control_code,
                Ssd1306::set_control_code,
            ).with_info("Device address."),
            PropDef::float(
                "Frequency",
                "I2C Frequency",
                "Hz",
                MIN_FREQ_HZ,
                MAX_FREQ_HZ,
                Ssd1306::get_freq,
                Ssd1306::set_freq,
            ).with_info("It is better to be similar to I2C Master frequency, but not critical in most cases."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Color", "Width", "Height", "Rotate"]),
                ("I2C", &["Control_Code", "Frequency"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ssd1306_pins("", self.height)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let w = self.width as f64;
        let h = self.height as f64;
        Rect::new(-70.0, -h / 2.0 - 16.0, w + 12.0, h + 24.0)
    }
}

impl Stampable for Ssd1306 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Ssd1306 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let w = self.width as f64;
        let h = self.height as f64;
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
        d.fill_rect(screen_x, screen_y, w, h, Color::rgb(0, 0, 0));
        d.stroke_rect(screen_x, screen_y, w, h, ctx.pal.border.fade(0.4), 1.0);

        if let Some(reading) = ctx.canvas.readings().get(ctx.item_id) {
            if !reading.text.is_empty() {
                let fg = match self.color {
                    Ssd1306Color::White => Color::rgb(245, 245, 245),
                    Ssd1306Color::Blue => Color::rgb(200, 200, 255),
                    Ssd1306Color::Yellow => Color::rgb(245, 245, 100),
                };
                let hex_bytes = reading.text.as_bytes();
                let num_pages = self.height / 8;
                let cols = self.width;
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
    pub fn add_ssd1306(&mut self, x: f64, y: f64) -> String {
        let id = format!("Ssd1306-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::ssd1306(
            &id, x, y, 128, 64, 0x3C, "White", true, 100.0,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ssd1306() {
        let s = Ssd1306::default();
        assert_eq!(s.type_id(), "Ssd1306");
        assert_eq!(s.color, Ssd1306Color::White);
        assert_eq!(s.width, 128);
        assert_eq!(s.height, 64);
        assert!(s.rotate);
        assert_eq!(s.control_code, 0x3C);
        assert_eq!(s.freq_khz, 100.0);
        assert_eq!(s.pin_geoms().len(), 2);
        assert_eq!(s.body(), Rect::new(-70.0, -48.0, 140.0, 88.0));

        let groups = s.prop_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Main");
        assert_eq!(
            groups[0].rows.iter().map(|r| r.name).collect::<Vec<_>>(),
            vec!["Color", "Width", "Height", "Rotate"]
        );
        assert_eq!(groups[1].name, "I2C");
        assert_eq!(
            groups[1].rows.iter().map(|r| r.name).collect::<Vec<_>>(),
            vec!["Control_Code", "Frequency"]
        );
    }
}

fn ssd1306_pins(id: &str, height: usize) -> Vec<Pin> {
    let y = (height as f64) / 2.0 + 16.0;
    vec![
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-PinSck"),
            item_id: id.to_string(),
            local: Point::new(-48.0, y),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: "SCL".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-PinSda"),
            item_id: id.to_string(),
            local: Point::new(-40.0, y),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: "SDA".into(),
            unused: false,
        },
    ]
}
