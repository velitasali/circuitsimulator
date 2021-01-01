//! 4-channel oscilloscope instrument.

use super::component::stamp_to_ground;
use super::drawable::{Drawable, paint_oscope};
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, PropGroup, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_IMPED_OHMS: f64 = 1e3;
const MAX_IMPED_OHMS: f64 = 1e12;
const MIN_TEST_TIME_S: f64 = 0.0;
const MAX_TEST_TIME_S: f64 = 1e6;

/// 4-channel oscilloscope instrument.
#[derive(Clone, Debug, PartialEq)]
pub struct Oscope {
    pub basic_x: i32,
    pub basic_y: i32,
    pub buffer_size: i32,
    pub connect_gnd: bool,
    pub input_imped: f64,
    pub test_time: f64,
    pub do_test: bool,
    pub tunnel1: String,
    pub tunnel2: String,
    pub tunnel3: String,
    pub tunnel4: String,
}

impl crate::canvas::Item {
    pub fn oscope(id: impl Into<String>, x: f64, y: f64, connect_gnd: bool) -> Self {
        Self::new(
            id,
            x,
            y,
            Oscope {
                basic_x: 135,
                basic_y: 135,
                buffer_size: 600000,
                connect_gnd,
                input_imped: 10.0,
                test_time: 0.0,
                do_test: false,
                tunnel1: String::new(),
                tunnel2: String::new(),
                tunnel3: String::new(),
                tunnel4: String::new(),
            },
        )
    }
}

impl Default for Oscope {
    fn default() -> Self {
        Self {
            basic_x: 135,
            basic_y: 135,
            buffer_size: 600000,
            connect_gnd: true,
            input_imped: 10.0,
            test_time: 0.0,
            do_test: false,
            tunnel1: String::new(),
            tunnel2: String::new(),
            tunnel3: String::new(),
            tunnel4: String::new(),
        }
    }
}

impl Oscope {
    pub const TYPE_ID: &'static str = "Oscope";
    pub fn ch_tunnels(&self) -> Vec<String> {
        vec![
            self.tunnel1.clone(),
            self.tunnel2.clone(),
            self.tunnel3.clone(),
            self.tunnel4.clone(),
        ]
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Oscope {
            connect_gnd: self.connect_gnd,
            input_imped: self.input_imped,
            tunnels: [
                self.tunnel1.clone(),
                self.tunnel2.clone(),
                self.tunnel3.clone(),
                self.tunnel4.clone(),
            ],
        }
    }

    fn get_basic_x(&self) -> PropValue {
        PropValue::Int(self.basic_x as i64)
    }
    fn set_basic_x(&mut self, v: PropValue) -> Result<(), PropError> {
        self.basic_x = expect_int("Basic_X", v)?.clamp(10, 10000) as i32;
        Ok(())
    }

    fn get_basic_y(&self) -> PropValue {
        PropValue::Int(self.basic_y as i64)
    }
    fn set_basic_y(&mut self, v: PropValue) -> Result<(), PropError> {
        self.basic_y = expect_int("Basic_Y", v)?.clamp(10, 10000) as i32;
        Ok(())
    }

    fn get_buffer_size(&self) -> PropValue {
        PropValue::Int(self.buffer_size as i64)
    }
    fn set_buffer_size(&mut self, v: PropValue) -> Result<(), PropError> {
        self.buffer_size = expect_int("BufferSize", v)?.clamp(100, 10_000_000) as i32;
        Ok(())
    }

    fn get_connect_gnd(&self) -> PropValue {
        PropValue::Bool(self.connect_gnd)
    }
    fn set_connect_gnd(&mut self, v: PropValue) -> Result<(), PropError> {
        self.connect_gnd = expect_bool("connectGnd", v)?;
        Ok(())
    }

    fn get_input_imped(&self) -> PropValue {
        PropValue::Float(self.input_imped * 1e6)
    }
    fn set_input_imped(&mut self, v: PropValue) -> Result<(), PropError> {
        let ohms = expect_float("InputImped", v)?.clamp(MIN_IMPED_OHMS, MAX_IMPED_OHMS);
        self.input_imped = ohms / 1e6;
        Ok(())
    }

    fn get_test_time(&self) -> PropValue {
        PropValue::Float(self.test_time)
    }
    fn set_test_time(&mut self, v: PropValue) -> Result<(), PropError> {
        self.test_time = expect_float("TestTime", v)?.clamp(MIN_TEST_TIME_S, MAX_TEST_TIME_S);
        Ok(())
    }

    fn get_do_test(&self) -> PropValue {
        PropValue::Bool(self.do_test)
    }
    fn set_do_test(&mut self, v: PropValue) -> Result<(), PropError> {
        self.do_test = expect_bool("DoTest", v)?;
        Ok(())
    }

    fn get_tunnel1(&self) -> PropValue {
        PropValue::String(self.tunnel1.clone())
    }
    fn set_tunnel1(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel1 = expect_string("Tunnel1", v)?;
        Ok(())
    }

    fn get_tunnel2(&self) -> PropValue {
        PropValue::String(self.tunnel2.clone())
    }
    fn set_tunnel2(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel2 = expect_string("Tunnel2", v)?;
        Ok(())
    }

    fn get_tunnel3(&self) -> PropValue {
        PropValue::String(self.tunnel3.clone())
    }
    fn set_tunnel3(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel3 = expect_string("Tunnel3", v)?;
        Ok(())
    }

    fn get_tunnel4(&self) -> PropValue {
        PropValue::String(self.tunnel4.clone())
    }
    fn set_tunnel4(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel4 = expect_string("Tunnel4", v)?;
        Ok(())
    }
}

impl Component for Oscope {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Oscilloscope instrument."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Oscope>] = &[
            PropDef::int(
                "Basic_X",
                "Screen Width",
                10,
                10000,
                Oscope::get_basic_x,
                Oscope::set_basic_x,
            ).with_info("Time base per division in horizontal axis."),
            PropDef::int(
                "Basic_Y",
                "Screen Height",
                10,
                10000,
                Oscope::get_basic_y,
                Oscope::set_basic_y,
            ).with_info("Voltage scale per division in vertical axis."),
            PropDef::int(
                "BufferSize",
                "Buffer Size",
                100,
                10_000_000,
                Oscope::get_buffer_size,
                Oscope::set_buffer_size,
            ).with_info("Sample buffer depth in points."),
            PropDef::bool(
                "connectGnd",
                "Connect to ground",
                Oscope::get_connect_gnd,
                Oscope::set_connect_gnd,
            ).with_info("Internal reference connection to ground."),
            PropDef::float(
                "InputImped",
                "Impedance",
                "Ω",
                MIN_IMPED_OHMS,
                MAX_IMPED_OHMS,
                Oscope::get_input_imped,
                Oscope::set_input_imped,
            ).with_info("Impedance of the input pins."),
            PropDef::float(
                "TestTime",
                "Test Time",
                "s",
                MIN_TEST_TIME_S,
                MAX_TEST_TIME_S,
                Oscope::get_test_time,
                Oscope::set_test_time,
            ).with_info("Automated test acquisition window duration."),
            PropDef::bool("DoTest", "Do Test", Oscope::get_do_test, Oscope::set_do_test).with_info("Perform automated test verification."),
            PropDef::string(
                "Tunnel1",
                "Channel 1",
                Oscope::get_tunnel1,
                Oscope::set_tunnel1,
            ).with_info("Name of the tunnel to connect to Channel 1 wirelessly. Leave empty to use direct probe pin."),
            PropDef::string(
                "Tunnel2",
                "Channel 2",
                Oscope::get_tunnel2,
                Oscope::set_tunnel2,
            ).with_info("Name of the tunnel to connect to Channel 2 wirelessly. Leave empty to use direct probe pin."),
            PropDef::string(
                "Tunnel3",
                "Channel 3",
                Oscope::get_tunnel3,
                Oscope::set_tunnel3,
            ).with_info("Name of the tunnel to connect to Channel 3 wirelessly. Leave empty to use direct probe pin."),
            PropDef::string(
                "Tunnel4",
                "Channel 4",
                Oscope::get_tunnel4,
                Oscope::set_tunnel4,
            ).with_info("Name of the tunnel to connect to Channel 4 wirelessly. Leave empty to use direct probe pin."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        oscope_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-80.0, -72.0, 213.0, 144.0)
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let all_rows = self.prop_rows();
        let mut main_rows = Vec::new();
        let mut tunnels_rows = Vec::new();
        let mut test_rows = Vec::new();
        for r in all_rows {
            if r.name.starts_with("Tunnel") {
                tunnels_rows.push(r);
            } else if r.name == "TestTime" || r.name == "DoTest" {
                test_rows.push(r);
            } else {
                main_rows.push(r);
            }
        }
        vec![
            PropGroup {
                name: "Main",
                rows: main_rows,
            },
            PropGroup {
                name: "Tunnels",
                rows: tunnels_rows,
            },
            PropGroup {
                name: "Test",
                rows: test_rows,
            },
        ]
    }
}

impl Stampable for Oscope {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if self.connect_gnd {
            let admit = if self.input_imped > 0.0 {
                1.0 / (self.input_imped * 1e6)
            } else {
                crate::instruments::PLOT_INPUT_ADMIT
            };
            for i in 0..5 {
                stamp_to_ground(matrix, pin_nodes, i, 0.0, admit);
            }
        }
    }
}

impl Drawable for Oscope {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let tunnels = [
            self.tunnel1.as_str(),
            self.tunnel2.as_str(),
            self.tunnel3.as_str(),
            self.tunnel4.as_str(),
        ];
        let freq_owned: Vec<String> = ctx
            .canvas
            .readings()
            .get(ctx.item_id)
            .map(|r| r.text.split(';').map(str::to_string).collect())
            .unwrap_or_default();
        let freqs: Vec<&str> = freq_owned.iter().map(String::as_str).collect();
        let live = ctx.canvas.live_scope_traces(ctx.item_id);
        paint_oscope(
            d,
            ctx.pal,
            &tunnels,
            &freqs,
            live.as_ref(),
            &ctx.canvas.scope_volt_div(),
            &ctx.canvas.scope_volt_pos(),
            ctx.canvas.scope_tracks(),
            &ctx.canvas.scope_hidden(),
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_oscope(&mut self, x: f64, y: f64) -> String {
        let id = format!("Oscope-{}", self.next_oscope);
        self.next_oscope += 1;
        self.items
            .push(crate::canvas::Item::oscope(&id, x, y, true));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_oscope() {
        let osc = Oscope::default();
        assert_eq!(osc.type_id(), "Oscope");
        assert_eq!(osc.pin_geoms().len(), 5);
        assert!(osc.connect_gnd);
        assert_eq!(osc.input_imped, 10.0);
        assert_eq!(osc.buffer_size, 600000);
    }

    #[test]
    fn oscope_element_kind() {
        let osc = Oscope::default();
        match osc.to_element_kind() {
            Kind::Oscope {
                connect_gnd,
                input_imped,
                tunnels,
            } => {
                assert!(connect_gnd);
                assert_eq!(input_imped, 10.0);
                assert_eq!(tunnels.len(), 4);
            }
            other => panic!("expected Kind::Oscope, got {other:?}"),
        }
    }
}

const OSCOPE_PINS: [PinGeom; 5] = [
    PinGeom {
        suffix: "-Pin0",
        x: -88.0,
        y: -48.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin1",
        x: -88.0,
        y: -16.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin2",
        x: -88.0,
        y: 16.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin3",
        x: -88.0,
        y: 48.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-PinG",
        x: -88.0,
        y: 64.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn oscope_pins() -> &'static [PinGeom] {
    &OSCOPE_PINS
}
