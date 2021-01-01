//! Multi-segment LED bar graph indicator.

use super::component::{stamp_conductance_between, stamp_to_ground};
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
use crate::elements::pins::{PIN_LEFT, PIN_RGB_C, PIN_RIGHT};
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_SEGMENTS: i64 = 1;
const MAX_SEGMENTS: i64 = 32;

use super::led::{LED_COLOR_OPTIONS, LedColor};

const MIN_V: f64 = 0.0;
const MAX_V: f64 = 100.0;
const MIN_A: f64 = 1e-6;
const MAX_A: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;

impl crate::canvas::Item {
    pub fn led_bar(id: impl Into<String>, x: f64, y: f64, segments: usize) -> Self {
        Self::new(
            id,
            x,
            y,
            LedBar {
                segments,
                color: LedColor::Red,
                grounded: false,
                threshold: 1.8,
                max_current: 0.03,
                resistance: 0.6,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LedBar {
    pub segments: usize,
    pub color: LedColor,
    pub grounded: bool,
    pub threshold: f64,
    pub max_current: f64,
    pub resistance: f64,
}

impl Default for LedBar {
    fn default() -> Self {
        Self {
            segments: 10,
            color: LedColor::Red,
            grounded: false,
            threshold: 1.8,
            max_current: 0.03,
            resistance: 0.6,
        }
    }
}

impl LedBar {
    pub const TYPE_ID: &'static str = "LedBar";
    pub fn to_element_kind(&self) -> Kind {
        Kind::LedBar {
            segments: self.segments,
            states: 0,
            grounded: self.grounded,
        }
    }

    fn get_segments(&self) -> PropValue {
        PropValue::Int(self.segments as i64)
    }
    fn set_segments(&mut self, v: PropValue) -> Result<(), PropError> {
        self.segments = expect_int("Segments", v)?.clamp(MIN_SEGMENTS, MAX_SEGMENTS) as usize;
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

    fn get_grounded(&self) -> PropValue {
        PropValue::Bool(self.grounded)
    }
    fn set_grounded(&mut self, v: PropValue) -> Result<(), PropError> {
        self.grounded = expect_bool("Grounded", v)?;
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

impl Component for LedBar {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "LED bar graph."
    }

    fn props() -> &'static [PropDef<Self>] {
        const SEGS: PropDef<LedBar> = {
            let mut p = PropDef::int(
                "Segments",
                "Segments",
                MIN_SEGMENTS,
                MAX_SEGMENTS,
                LedBar::get_segments,
                LedBar::set_segments,
            )
            .with_info("Number of LED segment elements in the bar array.");
            p.structural = true;
            p
        };
        const GND: PropDef<LedBar> = {
            let mut p = PropDef::bool(
                "Grounded",
                "Grounded",
                LedBar::get_grounded,
                LedBar::set_grounded,
            ).with_info("If yes, connect all cathodes to ground internally and hide cathode pins.\nAny wire already connected to cathodes will be deleted.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<LedBar>] = &[
            PropDef::enumeration("Color", "Color", LED_COLOR_OPTIONS, LedBar::get_color, LedBar::set_color)
                .with_info("Led color."),
            SEGS,
            GND,
            PropDef::float(
                "Threshold",
                "Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                LedBar::get_threshold,
                LedBar::set_threshold,
            ).with_info("Voltage drop when forward biased."),
            PropDef::float(
                "MaxCurrent",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                LedBar::get_max_current,
                LedBar::set_max_current,
            ).with_info("Maximum current (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                LedBar::get_resistance,
                LedBar::set_resistance,
            ).with_info("Series resistance."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Color", "Segments", "Grounded"]),
                ("Electric", &["Threshold", "MaxCurrent", "Resistance"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        led_bar_pins("", self.segments, self.grounded)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = 8.0 * (self.segments as f64).max(1.0);
        Rect::new(-8.0, -28.0, 16.0, h)
    }
}

impl Stampable for LedBar {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if self.grounded {
            for i in 0..self.segments {
                stamp_to_ground(matrix, pin_nodes, i, 1.8, 1.0 / 100.0);
            }
        } else {
            for i in 0..self.segments {
                stamp_conductance_between(matrix, pin_nodes, 2 * i, 2 * i + 1, 1.0 / 100.0);
            }
        }
    }
}

impl Drawable for LedBar {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let n = self.segments.max(1);
        let h = n as f64 * 8.0;
        d.fill_round_rect(-8.0, -28.0, 16.0, h, 2.0, Color::rgb(20, 20, 20));
        d.stroke_round_rect(
            -8.0,
            -28.0,
            16.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        let (lit, unlit) = crate::theme::ColorTheme::led_color_rgba(self.color.as_str());
        let lit_color = Color::rgb(lit.0, lit.1, lit.2);
        let unlit_color = Color::rgb(unlit.0, unlit.1, unlit.2);
        let sim_running = ctx.canvas.sim_running();
        for i in 0..n {
            let y = -26.0 + i as f64 * 8.0;
            let is_lit = if sim_running {
                let va = with_pin_id_idx(ctx.item_id, PIN_LEFT, i, |p| ctx.canvas.pin_voltage(p))
                    .or_else(|| {
                        with_pin_id_idx(ctx.item_id, "aPin", i, |p| ctx.canvas.pin_voltage(p))
                    })
                    .unwrap_or(0.0);
                let vc = if self.grounded {
                    0.0
                } else {
                    with_pin_id_idx(ctx.item_id, PIN_RIGHT, i, |p| ctx.canvas.pin_voltage(p))
                        .or_else(|| {
                            with_pin_id_idx(ctx.item_id, PIN_RGB_C, i, |p| {
                                ctx.canvas.pin_voltage(p)
                            })
                        })
                        .unwrap_or(0.0)
                };

                let cur = with_pin_id_idx(ctx.item_id, PIN_LEFT, i, |p| ctx.canvas.pin_current(p))
                    .or_else(|| {
                        with_pin_id_idx(ctx.item_id, "aPin", i, |p| ctx.canvas.pin_current(p))
                    })
                    .unwrap_or(0.0);

                (va - vc) > 1.5 || cur > 0.0005
            } else {
                false
            };

            if is_lit {
                // Soft glow around segment
                d.fill_round_rect(-7.0, y - 1.0, 14.0, 7.0, 2.0, lit_color.fade(0.35));
                d.fill_round_rect(-6.0, y, 12.0, 5.0, 1.0, lit_color);
            } else {
                d.fill_round_rect(-6.0, y, 12.0, 5.0, 1.0, unlit_color);
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_led_bar(&mut self, x: f64, y: f64) -> String {
        let id = format!("LedBar-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::led_bar(&id, x, y, 10));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_led_bar() {
        let b = LedBar::default();
        assert_eq!(b.type_id(), "LedBar");
        assert_eq!(b.segments, 10);
        assert_eq!(b.color, LedColor::Red);
        assert!(!b.grounded);
        assert_eq!(b.pin_geoms().len(), 20);
        assert_eq!(b.body(), Rect::new(-8.0, -28.0, 16.0, 80.0));
    }

    #[test]
    fn grounded_led_bar_half_pins() {
        let mut b = LedBar::default();
        b.grounded = true;
        assert_eq!(b.pin_geoms().len(), 10);
    }
}

fn led_bar_pins(id: &str, segments: usize, grounded: bool) -> Vec<Pin> {
    let n = segments.max(1);
    let mut pins = Vec::with_capacity(if grounded { n } else { n * 2 });
    for i in 0..n {
        let y = -24.0 + (i as f64) * 8.0;
        pins.push(Pin {
            direction: None,
            id: format!("{id}-lPin{i}"),
            item_id: id.to_string(),
            local: Point::new(-16.0, y),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: String::new(),
            unused: false,
        });
        if !grounded {
            pins.push(Pin {
                direction: None,
                id: format!("{id}-rPin{i}"),
                item_id: id.to_string(),
                local: Point::new(16.0, y),
                angle: 0,
                length: 8.0,
                is_bus: false,
                label: String::new(),
                unused: false,
            });
        }
    }
    pins
}
