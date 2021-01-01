//! AIP31068 dot matrix character LCD with I2C interface.

use super::component::PropGroup;
use super::props::{PropDef, PropError, PropValue, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_ROWS: i64 = 1;
const MAX_ROWS: i64 = 4;
const MIN_COLS: i64 = 1;
const MAX_COLS: i64 = 80;
const MIN_ADDR: i64 = 0;
const MAX_ADDR: i64 = 127;
const MIN_FREQ_KHZ: f64 = 1.0;
const MAX_FREQ_KHZ: f64 = 1000.0;

impl crate::canvas::Item {
    pub fn aip31068(id: impl Into<String>, x: f64, y: f64, rows: u8, cols: u8) -> Self {
        Self::aip31068_display(id, x, y, rows, cols)
    }

    pub fn aip31068_display(id: impl Into<String>, x: f64, y: f64, rows: u8, cols: u8) -> Self {
        Self::new(
            id,
            x,
            y,
            Aip31068 {
                rows,
                cols,
                control_code: 0x3E,
                freq_khz: 100.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aip31068 {
    pub rows: u8,
    pub cols: u8,
    pub control_code: u8,
    pub freq_khz: f64,
}

impl Default for Aip31068 {
    fn default() -> Self {
        Self {
            rows: 2,
            cols: 16,
            control_code: 0x3E,
            freq_khz: 100.0,
        }
    }
}

impl Aip31068 {
    pub const TYPE_ID: &'static str = "Aip31068";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Aip31068(crate::digital::Aip31068State::new(
            "",
            self.rows as usize,
            self.cols as usize,
            self.control_code,
            self.freq_khz,
        ))
    }

    fn get_rows(&self) -> PropValue {
        PropValue::Int(self.rows as i64)
    }
    fn set_rows(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rows = expect_int("Rows", v)?.clamp(MIN_ROWS, MAX_ROWS) as u8;
        Ok(())
    }

    fn get_cols(&self) -> PropValue {
        PropValue::Int(self.cols as i64)
    }
    fn set_cols(&mut self, v: PropValue) -> Result<(), PropError> {
        self.cols = expect_int("Cols", v)?.clamp(MIN_COLS, MAX_COLS) as u8;
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

impl Component for Aip31068 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Dot matrix character LCD with I2C interface."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ROWS: PropDef<Aip31068> = {
            let mut p = PropDef::int(
                "Rows",
                "Rows",
                MIN_ROWS,
                MAX_ROWS,
                Aip31068::get_rows,
                Aip31068::set_rows,
            )
            .with_info("Number of character rows.");
            p.structural = true;
            p
        };
        const COLS: PropDef<Aip31068> = {
            let mut p = PropDef::int(
                "Cols",
                "Cols",
                MIN_COLS,
                MAX_COLS,
                Aip31068::get_cols,
                Aip31068::set_cols,
            )
            .with_info("Number of character columns.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Aip31068>] = &[
            ROWS,
            COLS,
            PropDef::int(
                "Control_Code",
                "I2C Address",
                MIN_ADDR,
                MAX_ADDR,
                Aip31068::get_control_code,
                Aip31068::set_control_code,
            )
            .with_info("Device address."),
            PropDef::float(
                "Frequency",
                "I2C Frequency",
                "_kHz",
                MIN_FREQ_KHZ,
                MAX_FREQ_KHZ,
                Aip31068::get_freq,
                Aip31068::set_freq,
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
                ("Main", &["Rows", "Cols"]),
                ("I2C", &["Control_Code", "Frequency"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::openco_pin("-PinSDA", 16.0, 8.0, 270, 8.0).with_label("SDA"),
            CompPin::openco_pin("-PinSCL", 24.0, 8.0, 270, 8.0).with_label("SCL"),
        ]
    }

    fn body(&self) -> Rect {
        let w = (self.cols as f64 * 6.0 - 1.0) * 2.0 + 20.0;
        let h = (self.rows as f64 * 9.0 - 1.0) * 2.0 + 33.0;
        Rect::new(0.0, -h, w, h)
    }
}

impl Stampable for Aip31068 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl super::drawable::Drawable for Aip31068 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let rows = (self.rows as usize).clamp(1, 4);
        let cols = (self.cols as usize).clamp(8, 20);
        let w = (cols as f64 * 6.0 - 1.0) * 2.0 + 20.0;
        let h = (rows as f64 * 9.0 - 1.0) * 2.0 + 33.0;

        // PCB backing
        d.fill_round_rect(0.0, -h, w, h, 2.0, Color::rgb(50, 70, 100));
        d.stroke_round_rect(0.0, -h, w, h, 2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);

        // Screen glass
        d.fill_round_rect(
            4.0,
            -h + 4.0,
            w - 8.0,
            h - 21.0,
            3.0,
            Color::rgb(200, 220, 180),
        );
        d.stroke_round_rect(
            4.0,
            -h + 4.0,
            w - 8.0,
            h - 21.0,
            3.0,
            ctx.pal.border.fade(0.4),
            1.0,
        );

        // 5x8 character dot matrix
        let start_x = 10.0;
        let start_y = -h + 11.0;
        let unlit = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 15,
        };
        let lit = Color::rgb(30, 40, 30);

        let reading = ctx.canvas.readings().get(ctx.item_id);
        let hex_bitmap = reading.map(|r| r.text.as_str()).unwrap_or("");
        let extra_lines: Vec<&str> = reading
            .map(|r| r.extra.lines().collect())
            .unwrap_or_default();

        for r in 0..rows {
            let char_y = start_y + r as f64 * 18.0;
            for c in 0..cols {
                let char_x = start_x + c as f64 * 12.0;
                let mut pat = [0u8; 8];
                let hex_offset = (r * cols + c) * 16;
                if hex_bitmap.len() >= hex_offset + 16 {
                    for row_idx in 0..8 {
                        let byte_str =
                            &hex_bitmap[hex_offset + row_idx * 2..hex_offset + row_idx * 2 + 2];
                        pat[row_idx] = u8::from_str_radix(byte_str, 16).unwrap_or(0);
                    }
                } else if let Some(line) = extra_lines.get(r) {
                    if let Some(ch) = line.as_bytes().get(c) {
                        pat = crate::digital::hd44780_font::HD44780_ROM[*ch as usize];
                    }
                }

                for dot_row in 0..8 {
                    let dy = char_y + dot_row as f64 * 2.0;
                    let b = pat[dot_row];
                    for dot_col in 0..5 {
                        let dx = char_x + dot_col as f64 * 2.0;
                        if (b & (1 << (4 - dot_col))) != 0 {
                            d.fill_rect(dx, dy, 2.0, 2.0, lit);
                        } else {
                            d.fill_rect(dx, dy, 2.0, 2.0, unlit);
                        }
                    }
                }
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_aip31068(&mut self, x: f64, y: f64, rows: u8, cols: u8) -> String {
        let id = format!("AIP31068-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::aip31068_display(&id, x, y, rows, cols));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    #[test]
    fn default_aip31068() {
        let a = Aip31068::default();
        assert_eq!(a.type_id(), "Aip31068");
        assert_eq!(a.rows, 2);
        assert_eq!(a.cols, 16);
        assert_eq!(a.control_code, 0x3E);
        assert_eq!(a.freq_khz, 100.0);
        let pins = a.pin_geoms();
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].local, Point::new(16.0, 8.0));
        assert_eq!(pins[0].angle, 270);
        assert_eq!(pins[1].local, Point::new(24.0, 8.0));
        assert_eq!(pins[1].angle, 270);
        let w = (16.0 * 6.0 - 1.0) * 2.0 + 20.0;
        let h = (2.0 * 9.0 - 1.0) * 2.0 + 33.0;
        assert_eq!(a.body(), Rect::new(0.0, -h, w, h));
    }
}
