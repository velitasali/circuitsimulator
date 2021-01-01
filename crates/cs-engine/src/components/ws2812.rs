//! WS2812 addressable RGB LED chain or matrix.

use super::component::PropGroup;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_LEDS: i64 = 1;
const MAX_LEDS: i64 = 64;
const MIN_TIME_NS: i64 = 1;
const MAX_RST_NS: i64 = 1_000_000;
const MAX_BIT_NS: i64 = 100_000;

impl crate::canvas::Item {
    pub fn ws2812(id: impl Into<String>, x: f64, y: f64, count: usize) -> Self {
        Self::new(
            id,
            x,
            y,
            Ws2812 {
                count,
                rows: 1,
                cols: count,
                rst_time_ns: 50000,
                t0h_ns: 400,
                t0l_ns: 850,
                t1h_ns: 850,
                t1l_ns: 400,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ws2812 {
    pub count: usize,
    pub rows: usize,
    pub cols: usize,
    pub rst_time_ns: u32,
    pub t0h_ns: u32,
    pub t0l_ns: u32,
    pub t1h_ns: u32,
    pub t1l_ns: u32,
}

impl Default for Ws2812 {
    fn default() -> Self {
        Self {
            count: 8,
            rows: 1,
            cols: 8,
            rst_time_ns: 50000,
            t0h_ns: 400,
            t0l_ns: 850,
            t1h_ns: 850,
            t1l_ns: 400,
        }
    }
}

impl Ws2812 {
    pub const TYPE_ID: &'static str = "Ws2812";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ws2812 {
            count: self.count,
            rows: self.rows,
            cols: self.cols,
            rst_time_ns: self.rst_time_ns,
            t0h_ns: self.t0h_ns,
            t0l_ns: self.t0l_ns,
            t1h_ns: self.t1h_ns,
            t1l_ns: self.t1l_ns,
        }
    }

    fn get_count(&self) -> PropValue {
        PropValue::Int(self.count as i64)
    }
    fn set_count(&mut self, v: PropValue) -> Result<(), PropError> {
        self.count = expect_int("Count", v)?.clamp(MIN_LEDS, MAX_LEDS) as usize;
        Ok(())
    }

    fn get_rows(&self) -> PropValue {
        PropValue::Int(self.rows as i64)
    }
    fn set_rows(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rows = expect_int("Rows", v)?.clamp(MIN_LEDS, MAX_LEDS) as usize;
        Ok(())
    }

    fn get_cols(&self) -> PropValue {
        PropValue::Int(self.cols as i64)
    }
    fn set_cols(&mut self, v: PropValue) -> Result<(), PropError> {
        self.cols = expect_int("Cols", v)?.clamp(MIN_LEDS, MAX_LEDS) as usize;
        Ok(())
    }

    fn get_rst_time(&self) -> PropValue {
        PropValue::Int(self.rst_time_ns as i64)
    }
    fn set_rst_time(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rst_time_ns = expect_int("RstTime", v)?.clamp(MIN_TIME_NS, MAX_RST_NS) as u32;
        Ok(())
    }

    fn get_t0h(&self) -> PropValue {
        PropValue::Int(self.t0h_ns as i64)
    }
    fn set_t0h(&mut self, v: PropValue) -> Result<(), PropError> {
        self.t0h_ns = expect_int("T0H", v)?.clamp(MIN_TIME_NS, MAX_BIT_NS) as u32;
        Ok(())
    }

    fn get_t0l(&self) -> PropValue {
        PropValue::Int(self.t0l_ns as i64)
    }
    fn set_t0l(&mut self, v: PropValue) -> Result<(), PropError> {
        self.t0l_ns = expect_int("T0L", v)?.clamp(MIN_TIME_NS, MAX_BIT_NS) as u32;
        Ok(())
    }

    fn get_t1h(&self) -> PropValue {
        PropValue::Int(self.t1h_ns as i64)
    }
    fn set_t1h(&mut self, v: PropValue) -> Result<(), PropError> {
        self.t1h_ns = expect_int("T1H", v)?.clamp(MIN_TIME_NS, MAX_BIT_NS) as u32;
        Ok(())
    }

    fn get_t1l(&self) -> PropValue {
        PropValue::Int(self.t1l_ns as i64)
    }
    fn set_t1l(&mut self, v: PropValue) -> Result<(), PropError> {
        self.t1l_ns = expect_int("T1L", v)?.clamp(MIN_TIME_NS, MAX_BIT_NS) as u32;
        Ok(())
    }
}

impl Component for Ws2812 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "WS2812 addressable RGB LED chain/matrix."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ROWS: PropDef<Ws2812> = {
            let mut p = PropDef::int(
                "Rows",
                "Rows",
                MIN_LEDS,
                MAX_LEDS,
                Ws2812::get_rows,
                Ws2812::set_rows,
            )
            .with_info("Number of rows.");
            p.structural = true;
            p
        };
        const COLS: PropDef<Ws2812> = {
            let mut p = PropDef::int(
                "Cols",
                "Columns",
                MIN_LEDS,
                MAX_LEDS,
                Ws2812::get_cols,
                Ws2812::set_cols,
            )
            .with_info("Number of columns.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Ws2812>] = &[
            PropDef::int("Count", "Count", MIN_LEDS, MAX_LEDS, Ws2812::get_count, Ws2812::set_count).with_info("Number of addressable RGB LEDs in the strip."),
            ROWS,
            COLS,
            PropDef::int(
                "RstTime",
                "Reset Pulse (ns)",
                MIN_TIME_NS,
                MAX_RST_NS,
                Ws2812::get_rst_time,
                Ws2812::set_rst_time,
            ).with_info("Minimum time the data line must stay low between frames to latch the data (reset/latch time)."),
            PropDef::int("T0H", "T0H (ns)", MIN_TIME_NS, MAX_BIT_NS, Ws2812::get_t0h, Ws2812::set_t0h).with_info("High pulse duration encoding a logic '0' bit."),
            PropDef::int("T0L", "T0L (ns)", MIN_TIME_NS, MAX_BIT_NS, Ws2812::get_t0l, Ws2812::set_t0l).with_info("Low pulse duration encoding a logic '0' bit."),
            PropDef::int("T1H", "T1H (ns)", MIN_TIME_NS, MAX_BIT_NS, Ws2812::get_t1h, Ws2812::set_t1h).with_info("High pulse duration encoding a logic '1' bit."),
            PropDef::int("T1L", "T1L (ns)", MIN_TIME_NS, MAX_BIT_NS, Ws2812::get_t1l, Ws2812::set_t1l).with_info("Low pulse duration encoding a logic '1' bit."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Rows", "Cols", "Count"]),
                ("Timing", &["RstTime", "T0H", "T0L", "T1H", "T1L"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ws2812_pins("", self.rows, self.cols)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(
            -6.0,
            -6.0,
            (self.cols as f64).max(1.0) * 12.0,
            (self.rows as f64).max(1.0) * 12.0,
        )
    }
}

impl Stampable for Ws2812 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Ws2812 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let r = self.rows.max(1);
        let c = self.cols.max(1);
        let w = (c * 12) as f64;
        let h = (r * 12) as f64;
        d.fill_round_rect(-6.0, -6.0, w, h, 2.0, Color::rgb(25, 25, 25));
        d.stroke_round_rect(
            -6.0,
            -6.0,
            w,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        for row in 0..r {
            for col in 0..c {
                d.fill_circle(
                    (col as f64) * 12.0,
                    (row as f64) * 12.0,
                    3.5,
                    Color::rgb(200, 200, 200),
                );
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ws2812(&mut self, x: f64, y: f64) -> String {
        let id = format!("WS2812-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::ws2812(&id, x, y, 8));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ws2812() {
        let w = Ws2812::default();
        assert_eq!(w.type_id(), "Ws2812");
        assert_eq!(w.count, 8);
        assert_eq!(w.rows, 1);
        assert_eq!(w.cols, 8);
        assert_eq!(w.rst_time_ns, 50000);
        assert_eq!(w.t0h_ns, 400);
        assert_eq!(w.t0l_ns, 850);
        assert_eq!(w.t1h_ns, 850);
        assert_eq!(w.t1l_ns, 400);
        assert_eq!(w.pin_geoms().len(), 4);
        assert_eq!(w.body(), Rect::new(-6.0, -6.0, 96.0, 12.0));
    }
}

fn ws2812_pins(id: &str, rows: usize, cols: usize) -> Vec<Pin> {
    let r = rows.max(1);
    let c = cols.max(1);
    let right_pin_x = (c as f64) * 12.0;
    let (y_top, y_bot) = if r == 1 {
        (-4.0, 4.0)
    } else {
        (0.0, ((r - 1) as f64) * 12.0)
    };
    vec![
        Pin {
            direction: None,
            id: format!("{id}-din"),
            item_id: id.to_string(),
            local: Point::new(-16.0, y_top),
            angle: 180,
            length: 10.0,
            is_bus: false,
            label: String::new(),
            unused: false,
        },
        Pin {
            direction: None,
            id: format!("{id}-vcc"),
            item_id: id.to_string(),
            local: Point::new(-16.0, y_bot),
            angle: 180,
            length: 10.0,
            is_bus: false,
            label: String::new(),
            unused: false,
        },
        Pin {
            direction: None,
            id: format!("{id}-dout"),
            item_id: id.to_string(),
            local: Point::new(right_pin_x, y_top),
            angle: 0,
            length: 6.0,
            is_bus: false,
            label: String::new(),
            unused: false,
        },
        Pin {
            direction: None,
            id: format!("{id}-gnd"),
            item_id: id.to_string(),
            local: Point::new(right_pin_x, y_bot),
            angle: 0,
            length: 6.0,
            is_bus: false,
            label: String::new(),
            unused: false,
        },
    ]
}
