//! ESP-01 WiFi serial module (AT commands over UART).

use super::component::stamp_to_ground;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::{Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;

const MIN_BAUD: i64 = 300;
const MAX_BAUD: i64 = 10_000_000;

impl crate::canvas::Item {
    pub fn esp01(id: impl Into<String>, x: f64, y: f64, baud_rate: u32, debug: bool) -> Self {
        Self::esp01_with(id, x, y, baud_rate, debug)
    }

    pub fn esp01_with(id: impl Into<String>, x: f64, y: f64, baud_rate: u32, debug: bool) -> Self {
        Self::new(id, x, y, Esp01 { baud_rate, debug })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Esp01 {
    pub baud_rate: u32,
    pub debug: bool,
}

impl Default for Esp01 {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            debug: false,
        }
    }
}

impl Esp01 {
    pub const TYPE_ID: &'static str = "Esp01";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Esp01 {
            baud_rate: self.baud_rate,
            debug: self.debug,
        }
    }

    fn get_baud_rate(&self) -> PropValue {
        PropValue::Int(self.baud_rate as i64)
    }
    fn set_baud_rate(&mut self, v: PropValue) -> Result<(), PropError> {
        self.baud_rate = expect_int("Baudrate", v)?.clamp(MIN_BAUD, MAX_BAUD) as u32;
        Ok(())
    }

    fn get_debug(&self) -> PropValue {
        PropValue::Bool(self.debug)
    }
    fn set_debug(&mut self, v: PropValue) -> Result<(), PropError> {
        self.debug = expect_bool("Debug", v)?;
        Ok(())
    }
}

impl Component for Esp01 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Emulated ESP-01 WiFi module driven through AT command set over UART."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Esp01>] = &[
            PropDef::int(
                "Baudrate",
                "Baudrate",
                MIN_BAUD,
                MAX_BAUD,
                Esp01::get_baud_rate,
                Esp01::set_baud_rate,
            )
            .with_info("Transmission speed."),
            PropDef::bool(
                "Debug",
                "Show Debug messages",
                Esp01::get_debug,
                Esp01::set_debug,
            )
            .with_info("Show debug messages in bottom panel."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        esp01_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-28.0, -20.0, 56.0, 40.0)
    }
}

impl Stampable for Esp01 {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_to_ground(matrix, pin_nodes, 0, 3.3, SOURCE_ADMIT);
    }
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for Esp01 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // PCB Board (56x40)
        d.fill_round_rect(
            -28.0,
            -20.0,
            56.0,
            40.0,
            3.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -28.0,
            -20.0,
            56.0,
            40.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Header Title
        d.text(0.0, -12.0, "ESP-01", 8.0, ctx.pal.border, Align::Center);

        // Wi-Fi Label
        d.text(0.0, 0.0, "Wi-Fi", 8.0, ctx.pal.pin_open_high, Align::Center);

        // PCB Meander Antenna trace (-24, -14, 8, 28)
        d.fill_round_rect(
            -24.0,
            -14.0,
            8.0,
            28.0,
            1.0,
            ctx.pal.pin_open_high.fade(0.6),
        );
        d.stroke_round_rect(-24.0, -14.0, 8.0, 28.0, 1.0, ctx.pal.border, 0.5);

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_esp01(&mut self, x: f64, y: f64) -> String {
        let id = format!("Esp01-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::esp01(&id, x, y, 115200, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_esp01() {
        let e = Esp01::default();
        assert_eq!(e.type_id(), "Esp01");
        assert_eq!(e.baud_rate, 115200);
        assert!(!e.debug);
        assert_eq!(e.pin_geoms().len(), 2);
    }
}

const ESP01_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-pin0",
        x: -36.0,
        y: -8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-pin1",
        x: -36.0,
        y: 8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn esp01_pins() -> &'static [PinGeom] {
    &ESP01_PINS
}
