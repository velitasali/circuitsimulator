//! HD44780 alphanumeric dot-matrix character LCD display.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_ROWS: i64 = 1;
const MAX_ROWS: i64 = 4;
const MIN_COLS: i64 = 1;
const MAX_COLS: i64 = 80;

#[derive(Clone, Debug, PartialEq)]
pub struct Hd44780 {
    pub rows: usize,
    pub cols: usize,
}

impl crate::canvas::Item {
    pub fn hd44780(id: impl Into<String>, x: f64, y: f64, rows: usize, cols: usize) -> Self {
        Self::new(id, x, y, Hd44780 { rows, cols })
    }
}

impl Default for Hd44780 {
    fn default() -> Self {
        Self { rows: 2, cols: 16 }
    }
}

impl Hd44780 {
    pub const TYPE_ID: &'static str = "Hd44780";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Hd44780(crate::digital::Hd44780State::new("", self.rows, self.cols))
    }

    fn get_rows(&self) -> PropValue {
        PropValue::Int(self.rows as i64)
    }
    fn set_rows(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rows = expect_int("Rows", v)?.clamp(MIN_ROWS, MAX_ROWS) as usize;
        Ok(())
    }

    fn get_cols(&self) -> PropValue {
        PropValue::Int(self.cols as i64)
    }
    fn set_cols(&mut self, v: PropValue) -> Result<(), PropError> {
        self.cols = expect_int("Cols", v)?.clamp(MIN_COLS, MAX_COLS) as usize;
        Ok(())
    }
}

impl Component for Hd44780 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "HD44780-compatible alphanumeric dot-matrix LCD display."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ROWS: PropDef<Hd44780> = {
            let mut p = PropDef::int(
                "Rows",
                "Rows",
                MIN_ROWS,
                MAX_ROWS,
                Hd44780::get_rows,
                Hd44780::set_rows,
            )
            .with_info("Number of rows of characters.");
            p.structural = true;
            p
        };
        const COLS: PropDef<Hd44780> = {
            let mut p = PropDef::int(
                "Cols",
                "Cols",
                MIN_COLS,
                MAX_COLS,
                Hd44780::get_cols,
                Hd44780::set_cols,
            )
            .with_info("Number of characters per row.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Hd44780>] = &[ROWS, COLS];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        hd44780_pins("").into_iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        let w = (self.cols as f64 * 6.0 - 1.0) * 2.0 + 20.0;
        let h = (self.rows as f64 * 9.0 - 1.0) * 2.0 + 33.0;
        Rect::new(0.0, -h, w, h)
    }
}

impl Stampable for Hd44780 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Hd44780 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let rows = self.rows.clamp(1, 4);
        let cols = self.cols.clamp(8, 20);
        let w = (cols as f64 * 6.0 - 1.0) * 2.0 + 20.0;
        let h = (rows as f64 * 9.0 - 1.0) * 2.0 + 33.0;
        d.fill_round_rect(0.0, -h, w, h, 2.0, Color::rgb(50, 70, 100));
        d.stroke_round_rect(0.0, -h, w, h, 2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
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

        // Authentic 5x8 character dot matrix (blank by default until data written)
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
    pub fn add_hd44780(&mut self, x: f64, y: f64) -> String {
        let id = format!("Hd44780-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::hd44780(&id, x, y, 2, 16));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hd44780() {
        let h = Hd44780::default();
        assert_eq!(h.type_id(), "Hd44780");
        assert_eq!(h.rows, 2);
        assert_eq!(h.cols, 16);
        assert_eq!(h.pin_geoms().len(), 11);
        let w = (16.0 * 6.0 - 1.0) * 2.0 + 20.0;
        let ht = (2.0 * 9.0 - 1.0) * 2.0 + 33.0;
        assert_eq!(h.body(), Rect::new(0.0, -ht, w, ht));
    }
}

fn hd44780_pins(id: &str) -> Vec<Pin> {
    let mut pins = Vec::with_capacity(11);
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-PinRS"),
        item_id: id.to_string(),
        local: Point::new(16.0, 8.0),
        angle: 270,
        length: 8.0,
        is_bus: false,
        label: "RS".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-PinRW"),
        item_id: id.to_string(),
        local: Point::new(24.0, 8.0),
        angle: 270,
        length: 8.0,
        is_bus: false,
        label: "RW".into(),
        unused: false,
    });
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-PinEn"),
        item_id: id.to_string(),
        local: Point::new(32.0, 8.0),
        angle: 270,
        length: 8.0,
        is_bus: false,
        label: "En".into(),
        unused: false,
    });
    for i in 0..8 {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-dataPin{i}"),
            item_id: id.to_string(),
            local: Point::new(40.0 + (i as f64) * 8.0, 8.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: format!("D{i}"),
            unused: false,
        });
    }
    pins
}
