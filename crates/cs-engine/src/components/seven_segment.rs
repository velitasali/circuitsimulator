//! Seven-segment display indicator.

use super::component::stamp_two_terminal;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::with_pin_id_idx;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_DISPLAYS: i64 = 1;
const MAX_DISPLAYS: i64 = 8;
const MIN_V: f64 = 0.1;
const MAX_V: f64 = 10.0;
const MIN_A: f64 = 1e-3;
const MAX_A: f64 = 1.0;
const MIN_OHMS: f64 = 1e-3;
const MAX_OHMS: f64 = 1e6;

use super::led::{LED_COLOR_OPTIONS, LedColor};

impl crate::canvas::Item {
    pub fn seven_segment(id: impl Into<String>, x: f64, y: f64, common_anode: bool) -> Self {
        Self::new(
            id,
            x,
            y,
            SevenSegment {
                common_anode,
                num_displays: 1,
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
pub struct SevenSegment {
    pub common_anode: bool,
    pub num_displays: usize,
    pub vertical_pins: bool,
    pub color: LedColor,
    pub threshold: f64,
    pub max_current: f64,
    pub resistance: f64,
}

impl Default for SevenSegment {
    fn default() -> Self {
        Self {
            common_anode: false,
            num_displays: 1,
            vertical_pins: false,
            color: LedColor::Red,
            threshold: 2.0,
            max_current: 0.03,
            resistance: 0.6,
        }
    }
}

impl SevenSegment {
    pub const TYPE_ID: &'static str = "SevenSegment";
    pub fn to_element_kind(&self) -> Kind {
        Kind::SevenSegment {
            common_anode: self.common_anode,
            segments: 0,
        }
    }

    fn get_common_anode(&self) -> PropValue {
        PropValue::Bool(self.common_anode)
    }
    fn set_common_anode(&mut self, v: PropValue) -> Result<(), PropError> {
        self.common_anode = expect_bool("CommonAnode", v)?;
        Ok(())
    }

    fn get_num_displays(&self) -> PropValue {
        PropValue::Int(self.num_displays as i64)
    }
    fn set_num_displays(&mut self, v: PropValue) -> Result<(), PropError> {
        self.num_displays =
            expect_int("NumDisplays", v)?.clamp(MIN_DISPLAYS, MAX_DISPLAYS) as usize;
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

impl Component for SevenSegment {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Seven-segment display."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ANODE: PropDef<SevenSegment> = {
            let mut p = PropDef::bool(
                "CommonAnode",
                "Common Anode",
                SevenSegment::get_common_anode,
                SevenSegment::set_common_anode,
            )
            .with_info(
                "Determines common anode (true) or common cathode (false) pin configuration.",
            );
            p.structural = true;
            p
        };
        const NUM_DISP: PropDef<SevenSegment> = {
            let mut p = PropDef::int(
                "NumDisplays",
                "Displays",
                MIN_DISPLAYS,
                MAX_DISPLAYS,
                SevenSegment::get_num_displays,
                SevenSegment::set_num_displays,
            )
            .with_info("Number of displays.");
            p.structural = true;
            p
        };
        const VERT_PINS: PropDef<SevenSegment> = {
            let mut p = PropDef::bool(
                "VerticalPins",
                "Vertical Pins",
                SevenSegment::get_vertical_pins,
                SevenSegment::set_vertical_pins,
            )
            .with_info("If yes, All pins will be positioned at the top and bottom of the display.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<SevenSegment>] = &[
            PropDef::enumeration("Color", "Color", LED_COLOR_OPTIONS, SevenSegment::get_color, SevenSegment::set_color).with_info("Led color."),
            ANODE,
            NUM_DISP,
            VERT_PINS,
            PropDef::float(
                "Threshold",
                "Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                SevenSegment::get_threshold,
                SevenSegment::set_threshold,
            ).with_info("Voltage drop when forward biased."),
            PropDef::float(
                "MaxCurrent",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                SevenSegment::get_max_current,
                SevenSegment::set_max_current,
            ).with_info("Maximum current (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                SevenSegment::get_resistance,
                SevenSegment::set_resistance,
            ).with_info("Series resistance."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                (
                    "Main",
                    &["Color", "CommonAnode", "NumDisplays", "VerticalPins"],
                ),
                ("Electric", &["Threshold", "MaxCurrent", "Resistance"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        seven_segment_dynamic_pins("", self.num_displays, self.vertical_pins, self.common_anode)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-18.0, -28.0, 32.0 * (self.num_displays as f64) + 4.0, 56.0)
    }
}

impl Stampable for SevenSegment {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, 1.0 / 100.0, 1.8 / 100.0);
    }
}

use super::Drawable;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA, ColorTheme};

impl Drawable for SevenSegment {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let n = self.num_displays.clamp(1, 8);
        let w = 32.0 * n as f64 + 4.0;
        d.fill_rect(
            -18.0,
            -28.0,
            w,
            56.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_rect(
            -18.0,
            -28.0,
            w,
            56.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        let (lit_rgba, unlit_rgba) = ColorTheme::led_color_rgba(self.color.as_str());
        let lit_c = Color::rgb(lit_rgba.0, lit_rgba.1, lit_rgba.2);
        let unlit_c = Color::rgb(unlit_rgba.0, unlit_rgba.1, unlit_rgba.2);
        let sim_running = ctx.canvas.sim_running();

        for idx in 0..n {
            let x_off = 32.0 * idx as f64;
            let seg_color = |seg_name: &str| -> Color {
                if sim_running {
                    let v = with_pin_id_idx(ctx.item_id, seg_name, idx, |pin_id| {
                        ctx.canvas.pin_voltage(pin_id)
                    });
                    if let Some(v) = v {
                        let is_on = if self.common_anode { v < 1.0 } else { v > 1.5 };
                        if is_on {
                            return lit_c;
                        }
                    }
                    unlit_c
                } else {
                    lit_c
                }
            };

            d.line(x_off - 5.0, -18.0, x_off + 7.0, -18.0, seg_color("a"), 3.5);
            d.line(x_off + 11.0, -14.0, x_off + 10.0, -4.0, seg_color("b"), 3.5);
            d.line(x_off + 10.0, 4.0, x_off + 9.0, 14.0, seg_color("c"), 3.5);
            d.line(x_off + 5.0, 18.0, x_off - 7.0, 18.0, seg_color("d"), 3.5);
            d.line(x_off - 11.0, 14.0, x_off - 10.0, 4.0, seg_color("e"), 3.5);
            d.line(x_off - 10.0, -4.0, x_off - 9.0, -14.0, seg_color("f"), 3.5);
            d.line(x_off - 6.0, 0.0, x_off + 6.0, 0.0, seg_color("g"), 3.5);
            d.fill_circle(x_off + 11.0, 18.0, 2.0, seg_color("dp"));
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_seven_segment(&mut self, x: f64, y: f64) -> String {
        let id = format!("SevenSegment-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::seven_segment(&id, x, y, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seven_segment() {
        let s = SevenSegment::default();
        assert_eq!(s.type_id(), "SevenSegment");
        assert!(!s.common_anode);
        assert_eq!(s.num_displays, 1);
        assert!(!s.vertical_pins);
        assert_eq!(s.color, LedColor::Red);
        assert_eq!(s.threshold, 2.0);
        assert_eq!(s.max_current, 0.03);
        assert_eq!(s.resistance, 0.6);
        assert_eq!(s.body(), Rect::new(-18.0, -28.0, 36.0, 56.0));
        assert_eq!(s.pin_geoms().len(), 9);
    }
}

fn seven_segment_dynamic_pins(
    id: &str,
    num_displays: usize,
    vertical_pins: bool,
    common_anode: bool,
) -> Vec<Pin> {
    let n_disp = num_displays.clamp(1, 8);
    let mut pins = Vec::with_capacity(8 + n_disp);

    if !vertical_pins {
        // Pins a..g on Left Border (angle 180, x = -24.0, length 6.0 reaches border at x = -18.0)
        for i in 0..7 {
            let ch = (b'a' + i as u8) as char;
            pins.push(Pin {
                direction: None,
                id: format!("{id}-pin_{ch}"),
                item_id: id.to_string(),
                local: Point::new(-24.0, -24.0 + i as f64 * 8.0),
                angle: 180,
                length: 6.0,
                is_bus: false,
                label: ch.to_string(),
                unused: false,
            });
        }
        // Pin dot on Bottom Border (angle 270, y = 32.0, length 4.0 reaches border at y = 28.0)
        pins.push(Pin {
            direction: None,
            id: format!("{id}-pin_dot"),
            item_id: id.to_string(),
            local: Point::new(-8.0, 32.0),
            angle: 270,
            length: 4.0,
            is_bus: false,
            label: ".".to_string(),
            unused: false,
        });
    } else {
        // Top Pins 0..4 (a..e) on Top Border (angle 90, y = -32.0, length 4.0 reaches border at y = -28.0)
        for i in 0..5 {
            let ch = (b'a' + i as u8) as char;
            pins.push(Pin {
                direction: None,
                id: format!("{id}-pin_{ch}"),
                item_id: id.to_string(),
                local: Point::new(-16.0 + 8.0 * i as f64, -32.0),
                angle: 90,
                length: 4.0,
                is_bus: false,
                label: ch.to_string(),
                unused: false,
            });
        }
        // Bottom Pins 5..7 (f, g, dot) on Bottom Border (angle 270, y = 32.0, length 4.0 reaches border at y = 28.0)
        for i in 5..7 {
            let ch = (b'a' + i as u8) as char;
            pins.push(Pin {
                direction: None,
                id: format!("{id}-pin_{ch}"),
                item_id: id.to_string(),
                local: Point::new(-16.0 + 8.0 * (i - 5) as f64, 32.0),
                angle: 270,
                length: 4.0,
                is_bus: false,
                label: ch.to_string(),
                unused: false,
            });
        }
        pins.push(Pin {
            direction: None,
            id: format!("{id}-pin_dot"),
            item_id: id.to_string(),
            local: Point::new(0.0, 32.0),
            angle: 270,
            length: 4.0,
            is_bus: false,
            label: ".".to_string(),
            unused: false,
        });
    }

    // Common pin(s) on Bottom Border (angle 270, y = 32.0, length 4.0 reaches border at y = 28.0)
    let com_label = if common_anode { "+" } else { "|" };
    for n in 0..n_disp {
        let ch = (b'a' + n as u8) as char;
        pins.push(Pin {
            direction: None,
            id: format!("{id}-pin_common{ch}"),
            item_id: id.to_string(),
            local: Point::new(32.0 * n as f64 + 8.0, 32.0),
            angle: 270,
            length: 4.0,
            is_bus: false,
            label: com_label.to_string(),
            unused: false,
        });
    }

    pins
}
