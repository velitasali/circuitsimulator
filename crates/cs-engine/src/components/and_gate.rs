//! AndGate: combinational AND logic gate.

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::digital::GateState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_INPUTS: usize = 1;
const MAX_INPUTS: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct AndGate {
    pub num_inputs: usize,
    pub invert_inputs: bool,
    pub invert_output: bool,
    pub init_high: bool,
    pub tristate: bool,
    pub small: bool,
}

use super::{BufferGate, NotGate, OrGate, XorGate};

impl crate::canvas::Item {
    pub fn gate(
        id: impl Into<String>,
        x: f64,
        y: f64,
        kind: &str,
        num_inputs: usize,
        invert_inputs: bool,
        invert_output: bool,
        init_high: bool,
        tristate: bool,
        small: bool,
    ) -> Self {
        match kind.to_ascii_lowercase().as_str() {
            "and" => Self::new(
                id,
                x,
                y,
                AndGate {
                    num_inputs,
                    invert_inputs,
                    invert_output,
                    init_high,
                    tristate,
                    small,
                },
            ),
            "or" => Self::new(
                id,
                x,
                y,
                OrGate {
                    num_inputs,
                    invert_inputs,
                    invert_output,
                    init_high,
                    tristate,
                    small,
                },
            ),
            "xor" => Self::new(
                id,
                x,
                y,
                XorGate {
                    num_inputs,
                    invert_inputs,
                    invert_output,
                    init_high,
                    tristate,
                    small,
                },
            ),
            "not" => Self::new(
                id,
                x,
                y,
                NotGate {
                    invert_inputs,
                    invert_output,
                    init_high,
                    tristate,
                    small,
                },
            ),
            _ => Self::new(
                id,
                x,
                y,
                BufferGate {
                    invert_inputs,
                    invert_output,
                    init_high,
                    tristate,
                    small,
                },
            ),
        }
    }
}

impl Default for AndGate {
    fn default() -> Self {
        Self {
            num_inputs: 2,
            invert_inputs: false,
            invert_output: false,
            init_high: false,
            tristate: false,
            small: false,
        }
    }
}

impl AndGate {
    pub const TYPE_ID: &'static str = "AndGate";
    pub fn to_element_kind(&self) -> Kind {
        let mut st = GateState::and("", self.num_inputs);
        if self.invert_inputs {
            for p in &mut st.inputs {
                p.set_inverted(true);
            }
        }
        st.output.set_inverted(self.invert_output);
        st.init_high = self.init_high;
        st.set_tristate(self.tristate);
        Kind::Gate(st)
    }

    fn get_num_inputs(&self) -> PropValue {
        PropValue::Int(self.num_inputs as i64)
    }
    fn set_num_inputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.num_inputs =
            expect_int("NumInputs", v)?.clamp(MIN_INPUTS as i64, MAX_INPUTS as i64) as usize;
        Ok(())
    }
    fn get_invert_inputs(&self) -> PropValue {
        PropValue::Bool(self.invert_inputs)
    }
    fn set_invert_inputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.invert_inputs = expect_bool("InvertInputs", v)?;
        Ok(())
    }
    fn get_invert_outputs(&self) -> PropValue {
        PropValue::Bool(self.invert_output)
    }
    fn set_invert_outputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.invert_output = expect_bool("InvertOutputs", v)?;
        Ok(())
    }
    fn get_init_high(&self) -> PropValue {
        PropValue::Bool(self.init_high)
    }
    fn set_init_high(&mut self, v: PropValue) -> Result<(), PropError> {
        self.init_high = expect_bool("InitHigh", v)?;
        Ok(())
    }
    fn get_tristate(&self) -> PropValue {
        PropValue::Bool(self.tristate)
    }
    fn set_tristate(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tristate = expect_bool("Tristate", v)?;
        Ok(())
    }
    fn get_small(&self) -> PropValue {
        PropValue::Bool(self.small)
    }
    fn set_small(&mut self, v: PropValue) -> Result<(), PropError> {
        self.small = expect_bool("Small", v)?;
        Ok(())
    }
}

impl Component for AndGate {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "AND logic gate."
    }

    fn props() -> &'static [PropDef<Self>] {
        const NUM_INPUTS: PropDef<AndGate> = {
            let mut p = PropDef::int(
                "NumInputs",
                "Number of Inputs",
                MIN_INPUTS as i64,
                MAX_INPUTS as i64,
                AndGate::get_num_inputs,
                AndGate::set_num_inputs,
            )
            .with_info("Number of input pins.");
            p.structural = true;
            p
        };
        const TRISTATE: PropDef<AndGate> = {
            let mut p = PropDef::bool(
                "Tristate",
                "Tristate",
                AndGate::get_tristate,
                AndGate::set_tristate,
            )
            .with_info("High-impedance (tri-state) output stage.");
            p.structural = true;
            p
        };
        const SMALL: PropDef<AndGate> = {
            let mut p = PropDef::bool(
                "Small",
                "Small size",
                AndGate::get_small,
                AndGate::set_small,
            )
            .with_info("Use a smaller body.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<AndGate>] = &[
            NUM_INPUTS,
            PropDef::bool(
                "InvertInputs",
                "Invert Inputs",
                AndGate::get_invert_inputs,
                AndGate::set_invert_inputs,
            )
            .with_info("Invert input pins."),
            PropDef::bool(
                "InvertOutputs",
                "Invert Outputs",
                AndGate::get_invert_outputs,
                AndGate::set_invert_outputs,
            )
            .with_info("Invert output pins."),
            PropDef::bool(
                "InitHigh",
                "Init High",
                AndGate::get_init_high,
                AndGate::set_init_high,
            )
            .with_info("Output state at simulation start."),
            TRISTATE,
            SMALL,
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let n = self.num_inputs.max(1);
        let mut pins = Vec::with_capacity(n + 2);
        for i in 0..n {
            let y = -4.0 * (n as f64) + (i as f64) * 8.0 + 4.0;
            pins.push(
                CompPin::new(format!("-in{i}"), -16.0, y, 180, 8.0)
                    .with_direction(PinDirection::In),
            );
        }
        pins.push(CompPin::new("-out", 16.0, 0.0, 0, 8.0).with_direction(PinDirection::Out));
        if self.tristate {
            pins.push(
                CompPin::new("-Pin_outEnable", 0.0, -12.0, 90, 8.0)
                    .with_direction(PinDirection::In),
            );
        }
        pins
    }

    fn body(&self) -> Rect {
        let h = ((self.num_inputs as f64) * 8.0).max(8.0);
        Rect::new(-12.0, -h / 2.0, 24.0, h)
    }
}

impl Stampable for AndGate {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for AndGate {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let inps = self.num_inputs.max(1);
        let end_y = inps as f64 * 4.0;
        let mut pts = Vec::with_capacity(32);
        pts.push([-8.0, -end_y]);
        pts.push([-4.0, -end_y]);
        for i in 1..=12 {
            let t = i as f64 / 12.0;
            let omt = 1.0 - t;
            let x = omt * omt * (-4.0) + 2.0 * omt * t * 8.0 + t * t * 8.0;
            let y = omt * omt * (-end_y) + 2.0 * omt * t * (-end_y) + t * t * 0.0;
            pts.push([x, y]);
        }
        for i in 1..=12 {
            let t = i as f64 / 12.0;
            let omt = 1.0 - t;
            let x = omt * omt * 8.0 + 2.0 * omt * t * 8.0 + t * t * (-4.0);
            let y = omt * omt * 0.0 + 2.0 * omt * t * end_y + t * t * end_y;
            pts.push([x, y]);
        }
        pts.push([-8.0, end_y]);

        d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);

        if self.tristate {
            d.line(
                0.0,
                -end_y,
                0.0,
                -12.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }

        if self.invert_output {
            d.fill_circle(10.0, 0.0, 2.0, ctx.pal.body);
            d.stroke_circle(10.0, 0.0, 2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_and_gate(&mut self, x: f64, y: f64) -> String {
        let id = format!("AndGate-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::gate(
            &id, x, y, "And", 2, false, false, false, false, false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_gate() {
        let g = AndGate::default();
        assert_eq!(g.type_id(), "AndGate");
        assert_eq!(g.num_inputs, 2);
        assert!(!g.invert_inputs);
        assert!(!g.invert_output);
        assert!(!g.init_high);
        assert!(!g.tristate);
        assert!(!g.small);
        assert_eq!(g.pin_geoms().len(), 3);
    }

    #[test]
    fn and_gate_tristate_pin() {
        let mut g = AndGate::default();
        g.tristate = true;
        assert_eq!(g.pin_geoms().len(), 4);
    }
}
