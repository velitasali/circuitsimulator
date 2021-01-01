//! Automated digital testbench unit.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float, expect_string};
use super::{CompPin, Component, PropGroup, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::digital::TestUnitState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_PERIOD_S: f64 = 1e-12;
const MAX_PERIOD_S: f64 = 1e3;

/// Test unit for automated truth-table verification.
#[derive(Clone, Debug, PartialEq)]
pub struct TestUnit {
    pub inputs: String,
    pub outputs: String,
    pub period: f64,
    pub truth: Vec<u32>,
}

impl crate::canvas::Item {
    pub fn test_unit(
        id: impl Into<String>,
        x: f64,
        y: f64,
        inputs: &str,
        outputs: &str,
        period: f64,
        truth: Vec<u32>,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            TestUnit {
                inputs: inputs.into(),
                outputs: outputs.into(),
                period,
                truth,
            },
        )
    }
}

impl Default for TestUnit {
    fn default() -> Self {
        Self {
            inputs: "in0,in1".to_string(),
            outputs: "out0".to_string(),
            period: 1e-7,
            truth: Vec::new(),
        }
    }
}

impl TestUnit {
    pub const TYPE_ID: &'static str = "TestUnit";
    pub fn to_element_kind(&self) -> Kind {
        let mut tu = TestUnitState::new("");
        tu.set_inputs("", &self.inputs);
        tu.set_outputs("", &self.outputs);
        tu.period = self.period;
        tu.truth = self.truth.clone();
        Kind::TestUnit(tu)
    }

    fn get_inputs(&self) -> PropValue {
        PropValue::String(self.inputs.clone())
    }
    fn set_inputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inputs = expect_string("Inputs", v)?;
        Ok(())
    }

    fn get_outputs(&self) -> PropValue {
        PropValue::String(self.outputs.clone())
    }
    fn set_outputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.outputs = expect_string("Outputs", v)?;
        Ok(())
    }

    fn get_period(&self) -> PropValue {
        PropValue::Float(self.period)
    }
    fn set_period(&mut self, v: PropValue) -> Result<(), PropError> {
        self.period = expect_float("Period", v)?.clamp(MIN_PERIOD_S, MAX_PERIOD_S);
        Ok(())
    }

    fn get_truth(&self) -> PropValue {
        let s = self
            .truth
            .iter()
            .map(|v| format!("{:x}", v))
            .collect::<Vec<_>>()
            .join(",");
        PropValue::String(if s.is_empty() { String::new() } else { s + "," })
    }
    fn set_truth(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Truth", v)?;
        self.truth.clear();
        for part in s.split(',') {
            let p = part.trim();
            if !p.is_empty() {
                if let Ok(val) = u32::from_str_radix(p, 16) {
                    self.truth.push(val);
                }
            }
        }
        Ok(())
    }
}

impl Component for TestUnit {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Automated digital unit tester."
    }

    fn props() -> &'static [PropDef<Self>] {
        const INPUTS: PropDef<TestUnit> = {
            let mut p = PropDef::string(
                "Inputs",
                "Inputs",
                TestUnit::get_inputs,
                TestUnit::set_inputs,
            ).with_info("Comma-separated names of the pins that drive the inputs of the circuit under test.");
            p.structural = true;
            p
        };
        const OUTPUTS: PropDef<TestUnit> = {
            let mut p = PropDef::string(
                "Outputs",
                "Outputs",
                TestUnit::get_outputs,
                TestUnit::set_outputs,
            ).with_info("Comma-separated names of the pins that read the outputs of the circuit under test.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<TestUnit>] = &[
            INPUTS,
            OUTPUTS,
            PropDef::float(
                "Period",
                "Period",
                "ns",
                MIN_PERIOD_S,
                MAX_PERIOD_S,
                TestUnit::get_period,
                TestUnit::set_period,
            )
            .with_info("Time between test steps."),
            PropDef::string("Truth", "Truth", TestUnit::get_truth, TestUnit::set_truth)
                .with_info("Truth table definition strings for automated verification."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        test_unit_pins("", &self.inputs, &self.outputs)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let inp_list: Vec<&str> = self
            .inputs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let out_list: Vec<&str> = self
            .outputs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let max_n = inp_list.len().max(out_list.len()).max(1);
        let h = ((max_n + 1) as f64) * 8.0;
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let all_rows = self.prop_rows();
        let mut main_rows = Vec::new();
        let mut test_rows = Vec::new();
        for r in all_rows {
            if r.name == "Period" {
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
                name: "Test",
                rows: test_rows,
            },
        ]
    }
}

impl Stampable for TestUnit {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for TestUnit {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let inp_list: Vec<&str> = self
            .inputs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let out_list: Vec<&str> = self
            .outputs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let max_n = inp_list.len().max(out_list.len()).max(1);
        let h = ((max_n + 1) as f64) * 8.0;
        d.fill_round_rect(
            -16.0,
            -h / 2.0,
            32.0,
            h,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -16.0,
            -h / 2.0,
            32.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.text(0.0, -4.0, "TEST", 6.0, ctx.pal.border, Align::Center);
        d.text(0.0, 4.0, "UNIT", 6.0, ctx.pal.border, Align::Center);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_test_unit(&mut self, x: f64, y: f64) -> String {
        let id = format!("TestUnit-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::test_unit(
            &id,
            x,
            y,
            "O",
            "I0,I1",
            1e-7,
            Vec::new(),
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_test_unit() {
        let tu = TestUnit::default();
        assert_eq!(tu.type_id(), "TestUnit");
        assert_eq!(tu.inputs, "in0,in1");
        assert_eq!(tu.outputs, "out0");
        assert_eq!(tu.period, 1e-7);
        assert!(tu.truth.is_empty());
        // 1 input pin (reads out0) + 2 output pins (drive in0, in1) = 3 pins
        assert_eq!(tu.pin_geoms().len(), 3);
    }

    #[test]
    fn truth_table_round_trip() {
        let mut tu = TestUnit::default();
        tu.set_prop("Truth", PropValue::String("0,0,0,1,".into()))
            .unwrap();
        assert_eq!(tu.truth, vec![0, 0, 0, 1]);
        assert_eq!(
            tu.get_prop("Truth"),
            Some(PropValue::String("0,0,0,1,".into()))
        );
    }
}

fn test_unit_pins(id: &str, inputs: &str, outputs: &str) -> Vec<Pin> {
    let inp_list: Vec<&str> = inputs
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let out_list: Vec<&str> = outputs
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let n_in = out_list.len().max(1);
    let n_out = inp_list.len().max(1);
    let max_n = n_in.max(n_out);
    let h = max_n + 1;
    let y0 = -((h as f64) / 2.0) * 8.0 + 8.0;
    let mut pins = Vec::with_capacity(n_in + n_out);
    // Left pins: TestUnit inputs that read DUT outputs (labels from outputs property, e.g. "I0, I1")
    for (i, label) in out_list.iter().enumerate() {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, y0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: (*label).to_string(),
            unused: false,
        });
    }
    // Right pins: TestUnit outputs that drive DUT inputs (labels from inputs property, e.g. "O")
    for (i, label) in inp_list.iter().enumerate() {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-out{i}"),
            item_id: id.to_string(),
            local: Point::new(24.0, y0 + (i as f64) * 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: (*label).to_string(),
            unused: false,
        });
    }
    pins
}
