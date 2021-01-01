//! 7-Segment Display with BCD Decoder.

use super::led::{LED_COLOR_OPTIONS, LedColor};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::pins::{PIN_IN0, PIN_IN1, PIN_IN2, PIN_IN3};
use crate::matrix::CircMatrix;

impl crate::canvas::Item {
    pub fn seven_segment_bcd(
        id: impl Into<String>,
        x: f64,
        y: f64,
        color: impl AsRef<str>,
        common_anode: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            SevenSegmentBCD {
                color: LedColor::from_str_name(color.as_ref()),
                common_anode,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SevenSegmentBCD {
    pub color: LedColor,
    pub common_anode: bool,
}

impl Default for SevenSegmentBCD {
    fn default() -> Self {
        Self {
            color: LedColor::Red,
            common_anode: false,
        }
    }
}

impl SevenSegmentBCD {
    pub const TYPE_ID: &'static str = "SevenSegmentBCD";
    pub fn to_element_kind(&self) -> Kind {
        Kind::SevenSegmentBCD {
            color: self.color.as_str().to_string(),
            common_anode: self.common_anode,
        }
    }

    fn get_color(&self) -> PropValue {
        PropValue::Enum(self.color.as_str().to_string())
    }
    fn set_color(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Color", v)?;
        self.color = LedColor::from_str_name(&s);
        Ok(())
    }

    fn get_common_anode(&self) -> PropValue {
        PropValue::Bool(self.common_anode)
    }
    fn set_common_anode(&mut self, v: PropValue) -> Result<(), PropError> {
        self.common_anode = expect_bool("CommonAnode", v)?;
        Ok(())
    }
}

impl Component for SevenSegmentBCD {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "7-Segment Display with BCD Decoder."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<SevenSegmentBCD>] = &[
            PropDef::enumeration(
                "Color",
                "Color",
                LED_COLOR_OPTIONS,
                SevenSegmentBCD::get_color,
                SevenSegmentBCD::set_color,
            )
            .with_info("Led color."),
            PropDef::bool(
                "CommonAnode",
                "Common Anode",
                SevenSegmentBCD::get_common_anode,
                SevenSegmentBCD::set_common_anode,
            )
            .with_info(
                "Determines common anode (true) or common cathode (false) pin configuration.",
            ),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        seven_segment_bcd_pins("", false, false)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -24.0, 32.0, 48.0)
    }
}

impl Stampable for SevenSegmentBCD {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use super::Drawable;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_SYMBOL_ALPHA, ColorTheme};

impl Drawable for SevenSegmentBCD {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_rect(
            -16.0,
            -24.0,
            32.0,
            48.0,
            ctx.pal.border.fade(COMPONENT_SYMBOL_ALPHA),
        );
        d.stroke_rect(
            -16.0,
            -24.0,
            32.0,
            48.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        let (lit_rgba, unlit_rgba) = ColorTheme::led_color_rgba(self.color.as_str());
        let lit_c = Color::rgb(lit_rgba.0, lit_rgba.1, lit_rgba.2);
        let unlit_c = Color::rgb(unlit_rgba.0, unlit_rgba.1, unlit_rgba.2);
        let sim_running = ctx.canvas.sim_running();

        // 7-segment table for digits 0..15 (0..9 standard decimal, A..F hex)
        // bit 0: a, bit 1: b, bit 2: c, bit 3: d, bit 4: e, bit 5: f, bit 6: g, bit 7: dp
        static SEGS_BCD: [u8; 16] = [
            0b0011_1111, // 0: a,b,c,d,e,f
            0b0000_0110, // 1: b,c
            0b0101_1011, // 2: a,b,d,e,g
            0b0100_1111, // 3: a,b,c,d,g
            0b0110_0110, // 4: b,c,f,g
            0b0110_1101, // 5: a,c,d,f,g
            0b0111_1101, // 6: a,c,d,e,f,g
            0b0000_0111, // 7: a,b,c
            0b0111_1111, // 8: a,b,c,d,e,f,g
            0b0110_1111, // 9: a,b,c,d,f,g
            0b0111_0111, // A: a,b,c,e,f,g
            0b0111_1100, // b: c,d,e,f,g
            0b0011_1001, // C: a,d,e,f
            0b0101_1110, // d: b,c,d,e,g
            0b0111_1001, // E: a,d,e,f,g
            0b0111_0001, // F: a,e,f,g
        ];

        let seg_mask = if sim_running {
            let v0 = ctx.pin_voltage(PIN_IN0).unwrap_or(0.0) > 2.5;
            let v1 = ctx.pin_voltage(PIN_IN1).unwrap_or(0.0) > 2.5;
            let v2 = ctx.pin_voltage(PIN_IN2).unwrap_or(0.0) > 2.5;
            let v3 = ctx.pin_voltage(PIN_IN3).unwrap_or(0.0) > 2.5;
            let val = (if v0 { 1 } else { 0 })
                | (if v1 { 2 } else { 0 })
                | (if v2 { 4 } else { 0 })
                | (if v3 { 8 } else { 0 });
            SEGS_BCD[val & 0x0F]
        } else {
            0b0111_1111
        };

        let seg_color = |bit: u8| -> Color {
            if (seg_mask & (1 << bit)) != 0 {
                lit_c
            } else {
                unlit_c
            }
        };

        d.line(-5.0, -18.0, 7.0, -18.0, seg_color(0), 3.5);
        d.line(11.0, -14.0, 10.0, -4.0, seg_color(1), 3.5);
        d.line(10.0, 4.0, 9.0, 14.0, seg_color(2), 3.5);
        d.line(5.0, 18.0, -7.0, 18.0, seg_color(3), 3.5);
        d.line(-11.0, 14.0, -10.0, 4.0, seg_color(4), 3.5);
        d.line(-10.0, -4.0, -9.0, -14.0, seg_color(5), 3.5);
        d.line(-6.0, 0.0, 6.0, 0.0, seg_color(6), 3.5);
        d.fill_circle(11.0, 18.0, 2.0, seg_color(7));
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_seven_segment_bcd(&mut self, x: f64, y: f64) -> String {
        let id = format!("SevenSegmentBCD-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::seven_segment_bcd(
            &id, x, y, "Red", false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seven_segment_bcd() {
        let s = SevenSegmentBCD::default();
        assert_eq!(s.type_id(), "SevenSegmentBCD");
        assert_eq!(s.color, LedColor::Red);
        assert!(!s.common_anode);
        assert_eq!(s.pin_geoms().len(), 4);
        assert_eq!(s.body(), Rect::new(-16.0, -24.0, 32.0, 48.0));
    }
}

fn seven_segment_bcd_pins(id: &str, show_enable: bool, show_dot: bool) -> Vec<Pin> {
    let mut pins = Vec::with_capacity(6);
    // BCD Inputs on Bottom (angle 270, y = 32.0, weights 1, 2, 4, 8)
    let bottom_inputs = [
        ("1", 12.0, 0),
        ("2", 4.0, 1),
        ("4", -4.0, 2),
        ("8", -12.0, 3),
    ];
    for (lbl, x, idx) in bottom_inputs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{idx}"),
            item_id: id.to_string(),
            local: Point::new(x, 32.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: lbl.into(),
            unused: false,
        });
    }
    // Top Pins (angle 90, y = -32.0)
    if show_enable {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in4"),
            item_id: id.to_string(),
            local: Point::new(12.0, -32.0),
            angle: 90,
            length: 8.0,
            is_bus: false,
            label: "E".into(),
            unused: false,
        });
    }
    if show_dot {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in5"),
            item_id: id.to_string(),
            local: Point::new(-12.0, -32.0),
            angle: 90,
            length: 8.0,
            is_bus: false,
            label: ".".into(),
            unused: false,
        });
    }
    pins
}
