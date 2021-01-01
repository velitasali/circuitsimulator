//! SerialPort communication interface.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
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

impl crate::canvas::Item {
    pub fn serial_port(
        id: impl Into<String>,
        x: f64,
        y: f64,
        port_name: &str,
        baud_rate: u32,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            SerialPort {
                port_name: port_name.into(),
                baud_rate,
                data_bits: 8,
                stop_bits: 1,
                auto_open: false,
            },
        )
    }
}

/// Hardware serial port interface.
#[derive(Clone, Debug, PartialEq)]
pub struct SerialPort {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: usize,
    pub stop_bits: usize,
    pub auto_open: bool,
}

impl Default for SerialPort {
    fn default() -> Self {
        Self {
            port_name: "/dev/ttyUSB0".into(),
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            auto_open: false,
        }
    }
}

impl SerialPort {
    pub const TYPE_ID: &'static str = "SerialPort";
    pub fn to_element_kind(&self) -> Kind {
        Kind::SerialPort {
            port_name: self.port_name.clone(),
            baud_rate: self.baud_rate,
        }
    }

    fn get_port(&self) -> PropValue {
        PropValue::String(self.port_name.clone())
    }
    fn set_port(&mut self, v: PropValue) -> Result<(), PropError> {
        self.port_name = expect_string("Port", v)?;
        Ok(())
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

    fn get_auto(&self) -> PropValue {
        PropValue::Bool(self.auto_open)
    }
    fn set_auto(&mut self, v: PropValue) -> Result<(), PropError> {
        self.auto_open = expect_bool("Auto", v)?;
        Ok(())
    }
}

impl Component for SerialPort {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "SerialPort interface."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<SerialPort>] = &[
            PropDef::string(
                "Port",
                "Port Name",
                SerialPort::get_port,
                SerialPort::set_port,
            )
            .with_info("Name of the Serial Port to connect to."),
            PropDef::int(
                "Baud",
                "Baud Rate",
                MIN_BAUD,
                MAX_BAUD,
                SerialPort::get_baud,
                SerialPort::set_baud,
            )
            .with_info("Serial communication baud rate in bits per second."),
            PropDef::int(
                "DataBits",
                "Data Bits",
                MIN_DATA_BITS,
                MAX_DATA_BITS,
                SerialPort::get_data_bits,
                SerialPort::set_data_bits,
            )
            .with_info("Number of data bits."),
            PropDef::int(
                "StopBits",
                "Stop Bits",
                MIN_STOP_BITS,
                MAX_STOP_BITS,
                SerialPort::get_stop_bits,
                SerialPort::set_stop_bits,
            )
            .with_info("Number of stop bits."),
            PropDef::bool(
                "Auto",
                "Auto Open",
                SerialPort::get_auto,
                SerialPort::set_auto,
            )
            .with_info("Open port automatically at Simulation start."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        serial_port_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-32.0, -16.0, 160.0, 32.0)
    }

    fn interact_toggle(&mut self, local: Point) -> bool {
        let btn_hit = local.x >= -8.0 && local.x <= 36.0 && local.y >= -10.0 && local.y <= 10.0;
        let prog_hit = local.x.abs() < 1e-6 && local.y.abs() < 1e-6;
        btn_hit || prog_hit
    }
}

impl Stampable for SerialPort {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for SerialPort {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Module enclosure (-32, -16, 160, 32)
        d.fill_round_rect(
            -32.0,
            -16.0,
            160.0,
            32.0,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -32.0,
            -16.0,
            160.0,
            32.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Tx LED (-21, -11, 8, 6)
        d.fill_round_rect(-21.0, -11.0, 8.0, 6.0, 1.5, Color::rgb(220, 40, 40));
        d.stroke_round_rect(-21.0, -11.0, 8.0, 6.0, 1.5, ctx.pal.border, 0.5);

        // Rx LED (-21, 5, 8, 6)
        d.fill_round_rect(-21.0, 5.0, 8.0, 6.0, 1.5, Color::rgb(40, 220, 40));
        d.stroke_round_rect(-21.0, 5.0, 8.0, 6.0, 1.5, ctx.pal.border, 0.5);

        // Open/Close button (-8, -10, 44, 20)
        d.fill_round_rect(-8.0, -10.0, 44.0, 20.0, 3.0, ctx.pal.body.fade(0.85));
        d.stroke_round_rect(-8.0, -10.0, 44.0, 20.0, 3.0, ctx.pal.border, 1.0);
        d.text(14.0, 0.0, "Open", 8.0, ctx.pal.border, Align::Center);

        // Port text
        let p_name = if self.port_name.is_empty() {
            "/dev/ttyUSB0"
        } else {
            &self.port_name
        };
        d.text(42.0, -4.0, p_name, 8.0, ctx.pal.border, Align::TopLeft);

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_serial_port(&mut self, x: f64, y: f64) -> String {
        let id = format!("SerialPort-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::serial_port(
            &id,
            x,
            y,
            "/dev/ttyUSB0",
            9600,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_serial_port() {
        let sp = SerialPort::default();
        assert_eq!(sp.type_id(), "SerialPort");
        assert_eq!(sp.port_name, "/dev/ttyUSB0");
        assert_eq!(sp.baud_rate, 9600);
        assert_eq!(sp.data_bits, 8);
        assert_eq!(sp.stop_bits, 1);
        assert!(!sp.auto_open);
        assert_eq!(sp.pin_geoms().len(), 4);
    }

    #[test]
    fn serial_port_element_kind() {
        let sp = SerialPort {
            port_name: "COM3".into(),
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            auto_open: true,
        };
        match sp.to_element_kind() {
            Kind::SerialPort {
                port_name,
                baud_rate,
            } => {
                assert_eq!(port_name, "COM3");
                assert_eq!(baud_rate, 115200);
            }
            other => panic!("expected Kind::SerialPort, got {other:?}"),
        }
    }
}

const SERIAL_PORT_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-rx",
        x: -40.0,
        y: -8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-tx",
        x: -40.0,
        y: 8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-gnd",
        x: 136.0,
        y: -8.0,
        angle: 0,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vcc",
        x: 136.0,
        y: 8.0,
        angle: 0,
        length: 8.0,
        direction: None,
    },
];

fn serial_port_pins() -> &'static [PinGeom] {
    &SERIAL_PORT_PINS
}
