//! NotGate: inverter / NOT logic gate.

use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::digital::GateState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

#[derive(Clone, Debug, PartialEq)]
pub struct NotGate {
    pub invert_inputs: bool,
    pub invert_output: bool,
    pub init_high: bool,
    pub tristate: bool,
    pub small: bool,
}

impl Default for NotGate {
    fn default() -> Self {
        Self {
            invert_inputs: false,
            invert_output: true,
            init_high: false,
            tristate: false,
            small: false,
        }
    }
}

impl NotGate {
    pub const TYPE_ID: &'static str = "NotGate";
    pub fn to_element_kind(&self) -> Kind {
        let mut st = GateState::inverter("");
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

impl Component for NotGate {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "NOT / Inverter logic gate."
    }

    fn props() -> &'static [PropDef<Self>] {
        const TRISTATE: PropDef<NotGate> = {
            let mut p = PropDef::bool(
                "Tristate",
                "Tristate",
                NotGate::get_tristate,
                NotGate::set_tristate,
            )
            .with_info("High-impedance (tri-state) output stage.");
            p.structural = true;
            p
        };
        const SMALL: PropDef<NotGate> = {
            let mut p = PropDef::bool(
                "Small",
                "Small size",
                NotGate::get_small,
                NotGate::set_small,
            )
            .with_info("Use a smaller body.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<NotGate>] = &[
            PropDef::bool(
                "InvertInputs",
                "Invert Inputs",
                NotGate::get_invert_inputs,
                NotGate::set_invert_inputs,
            )
            .with_info("Invert input pins."),
            PropDef::bool(
                "InvertOutputs",
                "Invert Outputs",
                NotGate::get_invert_outputs,
                NotGate::set_invert_outputs,
            )
            .with_info("Invert output pins."),
            PropDef::bool(
                "InitHigh",
                "Init High",
                NotGate::get_init_high,
                NotGate::set_init_high,
            )
            .with_info("Output state at simulation start."),
            TRISTATE,
            SMALL,
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let mut pins = vec![
            CompPin::new("-in0", -16.0, 0.0, 180, 8.0).with_direction(PinDirection::In),
            CompPin::new("-out", 16.0, 0.0, 0, 8.0).with_direction(PinDirection::Out),
        ];
        if self.tristate {
            pins.push(
                CompPin::new("-Pin_outEnable", 0.0, -12.0, 90, 8.0)
                    .with_direction(PinDirection::In),
            );
        }
        pins
    }

    fn body(&self) -> Rect {
        Rect::new(-12.0, -8.0, 24.0, 16.0)
    }
}

impl Stampable for NotGate {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for NotGate {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let end_x = if self.small { 1.5 } else { 8.0 };
        let (top_y, bot_y) = if self.small { (-5.5, 5.5) } else { (-8.0, 8.0) };
        let pts = [[-8.0, top_y], [-8.0, bot_y], [end_x, 0.0]];
        d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);

        if self.tristate {
            d.line(
                0.0,
                -8.0,
                0.0,
                -12.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }

        if self.invert_output {
            let bx = if self.small { 3.5 } else { 10.0 };
            d.fill_circle(bx, 0.0, 2.0, ctx.pal.body);
            d.stroke_circle(bx, 0.0, 2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_not_gate() {
        let g = NotGate::default();
        assert_eq!(g.type_id(), "NotGate");
        assert!(!g.invert_inputs);
        assert!(g.invert_output);
        assert!(!g.init_high);
        assert!(!g.tristate);
        assert!(!g.small);
        assert_eq!(g.pin_geoms().len(), 2);
    }
}
