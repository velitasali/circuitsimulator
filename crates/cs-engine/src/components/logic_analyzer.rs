//! 8-channel digital logic analyzer.

use super::component::stamp_to_ground;
use super::drawable::{Drawable, paint_lanalizer};
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

/// 8-channel logic analyzer instrument.
#[derive(Clone, Debug, PartialEq)]
pub struct LogicAnalyzer {
    pub basic_x: i32,
    pub basic_y: i32,
    pub buffer_size: i32,
    pub connect_gnd: bool,
    pub input_imped: f64,
    pub test_time: f64,
    pub do_test: bool,
    pub auto_export: bool,
    pub time_step: i32,
    pub tunnel1: String,
    pub tunnel2: String,
    pub tunnel3: String,
    pub tunnel4: String,
    pub tunnel5: String,
    pub tunnel6: String,
    pub tunnel7: String,
    pub tunnel8: String,
}

impl crate::canvas::Item {
    pub fn lanalizer(id: impl Into<String>, x: f64, y: f64, connect_gnd: bool) -> Self {
        Self::new(
            id,
            x,
            y,
            LogicAnalyzer {
                basic_x: 135,
                basic_y: 135,
                buffer_size: 600000,
                connect_gnd,
                input_imped: 10.0,
                test_time: 0.0,
                do_test: false,
                auto_export: false,
                time_step: 1000,
                tunnel1: String::new(),
                tunnel2: String::new(),
                tunnel3: String::new(),
                tunnel4: String::new(),
                tunnel5: String::new(),
                tunnel6: String::new(),
                tunnel7: String::new(),
                tunnel8: String::new(),
            },
        )
    }
}

impl Default for LogicAnalyzer {
    fn default() -> Self {
        Self {
            basic_x: 135,
            basic_y: 135,
            buffer_size: 600000,
            connect_gnd: true,
            input_imped: 10.0,
            test_time: 0.0,
            do_test: false,
            auto_export: false,
            time_step: 1000,
            tunnel1: String::new(),
            tunnel2: String::new(),
            tunnel3: String::new(),
            tunnel4: String::new(),
            tunnel5: String::new(),
            tunnel6: String::new(),
            tunnel7: String::new(),
            tunnel8: String::new(),
        }
    }
}

impl LogicAnalyzer {
    pub const TYPE_ID: &'static str = "LogicAnalyzer";
    pub fn ch_tunnels(&self) -> Vec<String> {
        vec![
            self.tunnel1.clone(),
            self.tunnel2.clone(),
            self.tunnel3.clone(),
            self.tunnel4.clone(),
            self.tunnel5.clone(),
            self.tunnel6.clone(),
            self.tunnel7.clone(),
            self.tunnel8.clone(),
        ]
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::LAnalizer {
            connect_gnd: self.connect_gnd,
            input_imped: self.input_imped,
            tunnels: [
                self.tunnel1.clone(),
                self.tunnel2.clone(),
                self.tunnel3.clone(),
                self.tunnel4.clone(),
                self.tunnel5.clone(),
                self.tunnel6.clone(),
                self.tunnel7.clone(),
                self.tunnel8.clone(),
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

    fn get_auto_export(&self) -> PropValue {
        PropValue::Bool(self.auto_export)
    }
    fn set_auto_export(&mut self, v: PropValue) -> Result<(), PropError> {
        self.auto_export = expect_bool("AutoExport", v)?;
        Ok(())
    }

    fn get_time_step(&self) -> PropValue {
        PropValue::Int(self.time_step as i64)
    }
    fn set_time_step(&mut self, v: PropValue) -> Result<(), PropError> {
        self.time_step = expect_int("TimeStep", v)?.clamp(1, 1_000_000) as i32;
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

    fn get_tunnel5(&self) -> PropValue {
        PropValue::String(self.tunnel5.clone())
    }
    fn set_tunnel5(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel5 = expect_string("Tunnel5", v)?;
        Ok(())
    }

    fn get_tunnel6(&self) -> PropValue {
        PropValue::String(self.tunnel6.clone())
    }
    fn set_tunnel6(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel6 = expect_string("Tunnel6", v)?;
        Ok(())
    }

    fn get_tunnel7(&self) -> PropValue {
        PropValue::String(self.tunnel7.clone())
    }
    fn set_tunnel7(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel7 = expect_string("Tunnel7", v)?;
        Ok(())
    }

    fn get_tunnel8(&self) -> PropValue {
        PropValue::String(self.tunnel8.clone())
    }
    fn set_tunnel8(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tunnel8 = expect_string("Tunnel8", v)?;
        Ok(())
    }
}

impl Component for LogicAnalyzer {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Logic analyzer."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<LogicAnalyzer>] = &[
            PropDef::int(
                "Basic_X",
                "Screen Width",
                10,
                10000,
                LogicAnalyzer::get_basic_x,
                LogicAnalyzer::set_basic_x,
            ).with_info("Time base per division in horizontal axis."),
            PropDef::int(
                "Basic_Y",
                "Screen Height",
                10,
                10000,
                LogicAnalyzer::get_basic_y,
                LogicAnalyzer::set_basic_y,
            ).with_info("Voltage scale per division in vertical axis."),
            PropDef::int(
                "BufferSize",
                "Buffer Size",
                100,
                10_000_000,
                LogicAnalyzer::get_buffer_size,
                LogicAnalyzer::set_buffer_size,
            ).with_info("Sample buffer depth in points."),
            PropDef::bool(
                "connectGnd",
                "Connect to ground",
                LogicAnalyzer::get_connect_gnd,
                LogicAnalyzer::set_connect_gnd,
            ).with_info("Internal reference connection to ground."),
            PropDef::float(
                "InputImped",
                "Impedance",
                "Ω",
                MIN_IMPED_OHMS,
                MAX_IMPED_OHMS,
                LogicAnalyzer::get_input_imped,
                LogicAnalyzer::set_input_imped,
            ).with_info("Impedance of the input pins."),
            PropDef::float(
                "TestTime",
                "Test Time",
                "s",
                MIN_TEST_TIME_S,
                MAX_TEST_TIME_S,
                LogicAnalyzer::get_test_time,
                LogicAnalyzer::set_test_time,
            ).with_info("Automated test acquisition window duration."),
            PropDef::bool(
                "DoTest",
                "Do Test",
                LogicAnalyzer::get_do_test,
                LogicAnalyzer::set_do_test,
            ).with_info("Perform automated test verification."),
            PropDef::bool(
                "AutoExport",
                "Export at pause",
                LogicAnalyzer::get_auto_export,
                LogicAnalyzer::set_auto_export,
            ).with_info("Export to VCD file at Simulation pause."),
            PropDef::int(
                "TimeStep",
                "Base Time Step",
                1,
                1_000_000,
                LogicAnalyzer::get_time_step,
                LogicAnalyzer::set_time_step,
            ).with_info("Base time used in VCD file."),
            PropDef::string(
                "Tunnel1",
                "Channel 1",
                LogicAnalyzer::get_tunnel1,
                LogicAnalyzer::set_tunnel1,
            ).with_info("Name of tunnel to connect to Channel 1 wirelessly."),
            PropDef::string(
                "Tunnel2",
                "Channel 2",
                LogicAnalyzer::get_tunnel2,
                LogicAnalyzer::set_tunnel2,
            ).with_info("Name of tunnel to connect to Channel 2 wirelessly."),
            PropDef::string(
                "Tunnel3",
                "Channel 3",
                LogicAnalyzer::get_tunnel3,
                LogicAnalyzer::set_tunnel3,
            ).with_info("Name of tunnel to connect to Channel 3 wirelessly."),
            PropDef::string(
                "Tunnel4",
                "Channel 4",
                LogicAnalyzer::get_tunnel4,
                LogicAnalyzer::set_tunnel4,
            ).with_info("Name of tunnel to connect to Channel 4 wirelessly."),
            PropDef::string(
                "Tunnel5",
                "Channel 5",
                LogicAnalyzer::get_tunnel5,
                LogicAnalyzer::set_tunnel5,
            ).with_info("Name of the tunnel to connect to Channel 5 wirelessly. Leave empty to use direct probe pin."),
            PropDef::string(
                "Tunnel6",
                "Channel 6",
                LogicAnalyzer::get_tunnel6,
                LogicAnalyzer::set_tunnel6,
            ).with_info("Name of the tunnel to connect to Channel 6 wirelessly. Leave empty to use direct probe pin."),
            PropDef::string(
                "Tunnel7",
                "Channel 7",
                LogicAnalyzer::get_tunnel7,
                LogicAnalyzer::set_tunnel7,
            ).with_info("Name of the tunnel to connect to Channel 7 wirelessly. Leave empty to use direct probe pin."),
            PropDef::string(
                "Tunnel8",
                "Channel 8",
                LogicAnalyzer::get_tunnel8,
                LogicAnalyzer::set_tunnel8,
            ).with_info("Name of the tunnel to connect to Channel 8 wirelessly. Leave empty to use direct probe pin."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        la_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-80.0, -72.0, 213.0, 144.0)
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let all_rows = self.prop_rows();
        let mut main_rows = Vec::new();
        let mut tunnels_rows = Vec::new();
        let mut export_rows = Vec::new();
        let mut test_rows = Vec::new();
        for r in all_rows {
            if r.name.starts_with("Tunnel") {
                tunnels_rows.push(r);
            } else if r.name == "TimeStep" || r.name == "AutoExport" {
                export_rows.push(r);
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
                name: "Export",
                rows: export_rows,
            },
            PropGroup {
                name: "Test",
                rows: test_rows,
            },
        ]
    }
}

impl Stampable for LogicAnalyzer {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if self.connect_gnd {
            let admit = if self.input_imped > 0.0 {
                1.0 / (self.input_imped * 1e6)
            } else {
                crate::instruments::PLOT_INPUT_ADMIT
            };
            for i in 0..8 {
                stamp_to_ground(matrix, pin_nodes, i, 0.0, admit);
            }
        }
    }
}

impl Drawable for LogicAnalyzer {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let tunnels = [
            self.tunnel1.as_str(),
            self.tunnel2.as_str(),
            self.tunnel3.as_str(),
            self.tunnel4.as_str(),
            self.tunnel5.as_str(),
            self.tunnel6.as_str(),
            self.tunnel7.as_str(),
            self.tunnel8.as_str(),
        ];
        let live = ctx.canvas.live_la_traces(ctx.item_id);
        paint_lanalizer(d, ctx.pal, &tunnels, live.as_ref());
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_lanalizer(&mut self, x: f64, y: f64) -> String {
        let id = format!("LAnalizer-{}", self.next_lanalizer);
        self.next_lanalizer += 1;
        self.items
            .push(crate::canvas::Item::lanalizer(&id, x, y, true));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_logic_analyzer() {
        let la = LogicAnalyzer::default();
        assert_eq!(la.type_id(), "LogicAnalyzer");
        assert_eq!(la.pin_geoms().len(), 8);
        assert!(la.connect_gnd);
        assert_eq!(la.input_imped, 10.0);
    }
}

const LA_PINS: [PinGeom; 8] = [
    PinGeom {
        suffix: "-Pin0",
        x: -88.0,
        y: -64.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin1",
        x: -88.0,
        y: -48.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin2",
        x: -88.0,
        y: -32.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin3",
        x: -88.0,
        y: -16.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin4",
        x: -88.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin5",
        x: -88.0,
        y: 16.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin6",
        x: -88.0,
        y: 32.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-Pin7",
        x: -88.0,
        y: 48.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
];

fn la_pins() -> &'static [PinGeom] {
    &LA_PINS
}
