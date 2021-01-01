//! Serial terminal interface.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::draw::{Align, Color, Draw, PaintCtx};
use crate::canvas::{PinGeom, Point, Rect};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_BAUD: i64 = 300;
const MAX_BAUD: i64 = 1_000_000;
const MIN_DATA_BITS: i64 = 5;
const MAX_DATA_BITS: i64 = 9;
const MIN_STOP_BITS: i64 = 1;
const MAX_STOP_BITS: i64 = 2;

/// Serial terminal instrument interface.
#[derive(Clone, Debug, PartialEq)]
pub struct SerialTerm {
    pub baud_rate: u32,
    pub data_bits: usize,
    pub stop_bits: usize,
}

impl crate::canvas::Item {
    pub fn serial_term(id: impl Into<String>, x: f64, y: f64, baud_rate: u32) -> Self {
        Self::new(
            id,
            x,
            y,
            SerialTerm {
                baud_rate,
                data_bits: 8,
                stop_bits: 1,
            },
        )
    }
}

impl Default for SerialTerm {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
        }
    }
}

impl SerialTerm {
    pub const TYPE_ID: &'static str = "SerialTerm";
    pub fn to_element_kind(&self) -> Kind {
        Kind::SerialTerm {
            baud_rate: self.baud_rate,
        }
    }

    fn get_baud(&self) -> PropValue {
        PropValue::Int(self.baud_rate as i64)
    }
    fn set_baud(&mut self, v: PropValue) -> Result<(), PropError> {
        self.baud_rate = expect_int("Baud", v)?.clamp(MIN_BAUD, MAX_BAUD) as u32;
        Ok(())
    }

    fn get_data_bits(&self) -> PropValue {
        PropValue::Int(self.data_bits as i64)
    }
    fn set_data_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.data_bits = expect_int("DataBits", v)?.clamp(MIN_DATA_BITS, MAX_DATA_BITS) as usize;
        Ok(())
    }

    fn get_stop_bits(&self) -> PropValue {
        PropValue::Int(self.stop_bits as i64)
    }
    fn set_stop_bits(&mut self, v: PropValue) -> Result<(), PropError> {
        self.stop_bits = expect_int("StopBits", v)?.clamp(MIN_STOP_BITS, MAX_STOP_BITS) as usize;
        Ok(())
    }
}

impl Component for SerialTerm {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Serial terminal interface."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<SerialTerm>] = &[
            PropDef::int(
                "Baud",
                "Baud Rate",
                MIN_BAUD,
                MAX_BAUD,
                SerialTerm::get_baud,
                SerialTerm::set_baud,
            )
            .with_info("Serial communication baud rate in bits per second."),
            PropDef::int(
                "DataBits",
                "Data Bits",
                MIN_DATA_BITS,
                MAX_DATA_BITS,
                SerialTerm::get_data_bits,
                SerialTerm::set_data_bits,
            )
            .with_info("Number of data bits."),
            PropDef::int(
                "StopBits",
                "Stop Bits",
                MIN_STOP_BITS,
                MAX_STOP_BITS,
                SerialTerm::get_stop_bits,
                SerialTerm::set_stop_bits,
            )
            .with_info("Number of stop bits."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        serial_term_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -16.0, 72.0, 32.0)
    }

    fn interact_toggle(&mut self, local: Point) -> bool {
        let btn_hit = local.x >= 8.0 && local.x <= 52.0 && local.y >= -10.0 && local.y <= 10.0;
        let prog_hit = local.x.abs() < 1e-6 && local.y.abs() < 1e-6;
        btn_hit || prog_hit
    }
}

impl Stampable for SerialTerm {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for SerialTerm {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Module enclosure (-16, -16, 72, 32)
        d.fill_round_rect(
            -16.0,
            -16.0,
            72.0,
            32.0,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -16.0,
            -16.0,
            72.0,
            32.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Tx LED (-5, -11, 8, 6)
        d.fill_round_rect(-5.0, -11.0, 8.0, 6.0, 1.5, Color::rgb(220, 40, 40));
        d.stroke_round_rect(-5.0, -11.0, 8.0, 6.0, 1.5, ctx.pal.border, 0.5);

        // Rx LED (-5, 5, 8, 6)
        d.fill_round_rect(-5.0, 5.0, 8.0, 6.0, 1.5, Color::rgb(40, 220, 40));
        d.stroke_round_rect(-5.0, 5.0, 8.0, 6.0, 1.5, ctx.pal.border, 0.5);

        // Open/Close button (8, -10, 44, 20)
        d.fill_round_rect(8.0, -10.0, 44.0, 20.0, 3.0, ctx.pal.body.fade(0.85));
        d.stroke_round_rect(8.0, -10.0, 44.0, 20.0, 3.0, ctx.pal.border, 1.0);
        d.text(30.0, 0.0, "Open", 8.0, ctx.pal.border, Align::Center);

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_serial_term(&mut self, x: f64, y: f64) -> String {
        let id = format!("SerialTerm-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::serial_term(&id, x, y, 9600));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_serial_term() {
        let st = SerialTerm::default();
        assert_eq!(st.type_id(), "SerialTerm");
        assert_eq!(st.baud_rate, 9600);
        assert_eq!(st.data_bits, 8);
        assert_eq!(st.stop_bits, 1);
        assert_eq!(st.pin_geoms().len(), 2);
    }

    #[test]
    fn serial_term_element_kind() {
        let st = SerialTerm {
            baud_rate: 57600,
            data_bits: 7,
            stop_bits: 2,
        };
        match st.to_element_kind() {
            Kind::SerialTerm { baud_rate } => assert_eq!(baud_rate, 57600),
            other => panic!("expected Kind::SerialTerm, got {other:?}"),
        }
    }
}

const SERIAL_TERM_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-rx",
        x: -24.0,
        y: -8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-tx",
        x: -24.0,
        y: 8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn serial_term_pins() -> &'static [PinGeom] {
    &SERIAL_TERM_PINS
}
