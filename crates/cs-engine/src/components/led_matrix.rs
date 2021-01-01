//! Dot matrix LED display.

use super::drawable::Drawable;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::canvas::with_pin_id_idx;
use crate::elements::Kind;
use crate::elements::pins::{PIN_RGB_C, PIN_RIGHT};
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 64;
const MIN_V: f64 = 0.1;
const MAX_V: f64 = 10.0;
const MIN_A: f64 = 1e-3;
const MAX_A: f64 = 1.0;
const MIN_OHMS: f64 = 1e-3;
const MAX_OHMS: f64 = 1e6;

use super::led::{LED_COLOR_OPTIONS, LedColor};

impl crate::canvas::Item {
    pub fn led_matrix(id: impl Into<String>, x: f64, y: f64, rows: usize, cols: usize) -> Self {
        Self::new(
            id,
            x,
            y,
            LedMatrix {
                rows,
                cols,
                vertical_pins: false,
                color: LedColor::Red,
                threshold: 2.0,
                max_current: 0.03,
                resistance: 0.6,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LedMatrix {
    pub rows: usize,
    pub cols: usize,
    pub vertical_pins: bool,
    pub color: LedColor,
    pub threshold: f64,
    pub max_current: f64,
    pub resistance: f64,
}

impl Default for LedMatrix {
    fn default() -> Self {
        Self {
            rows: 8,
            cols: 8,
            vertical_pins: false,
            color: LedColor::Red,
            threshold: 2.0,
            max_current: 0.03,
            resistance: 0.6,
        }
    }
}

impl LedMatrix {
    pub const TYPE_ID: &'static str = "LedMatrix";
    pub fn to_element_kind(&self) -> Kind {
        Kind::LedMatrix {
            rows: self.rows,
            cols: self.cols,
            vertical_pins: self.vertical_pins,
            color: self.color.as_str().to_string(),
            threshold: self.threshold,
            max_current: self.max_current,
            resistance: self.resistance,
        }
    }

    fn get_rows(&self) -> PropValue {
        PropValue::Int(self.rows as i64)
    }
    fn set_rows(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rows = expect_int("Rows", v)?.clamp(MIN_DIM, MAX_DIM) as usize;
        Ok(())
    }

    fn get_cols(&self) -> PropValue {
        PropValue::Int(self.cols as i64)
    }
    fn set_cols(&mut self, v: PropValue) -> Result<(), PropError> {
        self.cols = expect_int("Cols", v)?.clamp(MIN_DIM, MAX_DIM) as usize;
        Ok(())
    }

    fn get_vertical_pins(&self) -> PropValue {
        PropValue::Bool(self.vertical_pins)
    }
    fn set_vertical_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        self.vertical_pins = expect_bool("VerticalPins", v)?;
        Ok(())
    }

    fn get_color(&self) -> PropValue {
        PropValue::Enum(self.color.as_str().to_string())
    }
    fn set_color(&mut self, v: PropValue) -> Result<(), PropError> {
        let color_name = expect_string("Color", v)?;
        self.color = LedColor::from_str_name(&color_name);
        self.threshold = self.color.threshold();
        Ok(())
    }

    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold)
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        self.threshold = expect_float("Threshold", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_max_current(&self) -> PropValue {
        PropValue::Float(self.max_current)
    }
    fn set_max_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_current = expect_float("MaxCurrent", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
}

impl Component for LedMatrix {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Dot matrix LED display."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ROWS: PropDef<LedMatrix> = {
            let mut p = PropDef::int(
                "Rows",
                "Rows",
                MIN_DIM,
                MAX_DIM,
                LedMatrix::get_rows,
                LedMatrix::set_rows,
            )
            .with_info("Number of rows.");
            p.structural = true;
            p
        };
        const COLS: PropDef<LedMatrix> = {
            let mut p = PropDef::int(
                "Cols",
                "Columns",
                MIN_DIM,
                MAX_DIM,
                LedMatrix::get_cols,
                LedMatrix::set_cols,
            )
            .with_info("Number of columns.");
            p.structural = true;
            p
        };
        const VERT_PINS: PropDef<LedMatrix> = {
            let mut p = PropDef::bool(
                "VerticalPins",
                "Vertical Pins",
                LedMatrix::get_vertical_pins,
                LedMatrix::set_vertical_pins,
            )
            .with_info("If yes, All pins will be positioned at the top and bottom of the display.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<LedMatrix>] = &[
            PropDef::enumeration("Color", "Color", LED_COLOR_OPTIONS, LedMatrix::get_color, LedMatrix::set_color).with_info("Led color."),
            ROWS,
            COLS,
            VERT_PINS,
            PropDef::float(
                "Threshold",
                "Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                LedMatrix::get_threshold,
                LedMatrix::set_threshold,
            ).with_info("Voltage drop when forward biased."),
            PropDef::float(
                "MaxCurrent",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                LedMatrix::get_max_current,
                LedMatrix::set_max_current,
            ).with_info("Maximum current (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                LedMatrix::get_resistance,
                LedMatrix::set_resistance,
            ).with_info("Series resistance."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Color", "Rows", "Cols", "VerticalPins"]),
                ("Electric", &["Threshold", "MaxCurrent", "Resistance"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        led_matrix_pins("", self.rows, self.cols, self.vertical_pins)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(
            -8.0,
            -8.0,
            (self.cols as f64).max(1.0) * 8.0 + 8.0,
            (self.rows as f64).max(1.0) * 8.0 + 8.0,
        )
    }
}

impl Stampable for LedMatrix {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for LedMatrix {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let r = self.rows.max(1);
        let c = self.cols.max(1);
        let w = (c * 8 + 8) as f64;
        let h = (r * 8 + 8) as f64;
        d.fill_round_rect(-8.0, -8.0, w, h, 2.0, Color::rgb(20, 20, 20));
        d.stroke_round_rect(
            -8.0,
            -8.0,
            w,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        let (lit_rgba, unlit_rgba) = crate::theme::ColorTheme::led_color_rgba(self.color.as_str());
        let lit_color = Color::rgb(lit_rgba.0, lit_rgba.1, lit_rgba.2);
        let unlit_color = Color::rgb(unlit_rgba.0, unlit_rgba.1, unlit_rgba.2);
        let sim_running = ctx.canvas.sim_running();

        for row in 0..r {
            let vr = if sim_running {
                with_pin_id_idx(ctx.item_id, PIN_RIGHT, row, |p| ctx.canvas.pin_voltage(p))
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            for col in 0..c {
                let vc = if sim_running {
                    with_pin_id_idx(ctx.item_id, PIN_RGB_C, col, |p| ctx.canvas.pin_voltage(p))
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                let is_on = sim_running && (vr - vc) > self.threshold * 0.8;
                let cx = (col as f64) * 8.0;
                let cy = (row as f64) * 8.0;

                if is_on {
                    d.fill_circle(cx, cy, 4.0, lit_color.fade(0.35));
                    d.fill_circle(cx, cy, 2.5, lit_color);
                } else {
                    d.fill_circle(cx, cy, 2.5, unlit_color);
                }
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_led_matrix(&mut self, x: f64, y: f64) -> String {
        let id = format!("LedMatrix-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::led_matrix(&id, x, y, 8, 8));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_led_matrix() {
        let m = LedMatrix::default();
        assert_eq!(m.type_id(), "LedMatrix");
        assert_eq!(m.rows, 8);
        assert_eq!(m.cols, 8);
        assert!(!m.vertical_pins);
        assert_eq!(m.color, LedColor::Red);
        assert_eq!(m.threshold, 2.0);
        assert_eq!(m.max_current, 0.03);
        assert_eq!(m.resistance, 0.6);
        assert_eq!(m.pin_geoms().len(), 16);
        assert_eq!(m.body(), Rect::new(-8.0, -8.0, 72.0, 72.0));
    }
}

fn led_matrix_pins(id: &str, rows: usize, cols: usize, vertical_pins: bool) -> Vec<Pin> {
    let r = rows.max(1);
    let c = cols.max(1);
    let mut pins = Vec::with_capacity(r + c);
    if vertical_pins {
        for row in 0..r {
            pins.push(Pin {
                direction: None,
                id: format!("{id}-rPin{row}"),
                item_id: id.to_string(),
                local: Point::new((row as f64) * 8.0, -16.0),
                angle: 90,
                length: 8.0,
                is_bus: false,
                label: String::new(),
                unused: false,
            });
        }
    } else {
        for row in 0..r {
            pins.push(Pin {
                direction: None,
                id: format!("{id}-rPin{row}"),
                item_id: id.to_string(),
                local: Point::new(-16.0, (row as f64) * 8.0),
                angle: 180,
                length: 8.0,
                is_bus: false,
                label: String::new(),
                unused: false,
            });
        }
    }
    for col in 0..c {
        pins.push(Pin {
            direction: None,
            id: format!("{id}-cPin{col}"),
            item_id: id.to_string(),
            local: Point::new((col as f64) * 8.0, (r as f64) * 8.0 + 8.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: String::new(),
            unused: false,
        });
    }
    pins
}
