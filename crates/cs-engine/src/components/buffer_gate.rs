//! BufferGate: buffer logic gate.

use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::digital::GateState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

#[derive(Clone, Debug, PartialEq)]
pub struct BufferGate {
    pub invert_inputs: bool,
    pub invert_output: bool,
    pub init_high: bool,
    pub tristate: bool,
    pub small: bool,
}

impl Default for BufferGate {
    fn default() -> Self {
        Self {
            invert_inputs: false,
            invert_output: false,
            init_high: false,
            tristate: false,
            small: false,
        }
    }
}

impl BufferGate {
    pub const TYPE_ID: &'static str = "BufferGate";
    pub fn to_element_kind(&self) -> Kind {
        let mut st = GateState::buffer("");
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

impl Component for BufferGate {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Buffer logic gate."
    }

    fn props() -> &'static [PropDef<Self>] {
        const TRISTATE: PropDef<BufferGate> = {
            let mut p = PropDef::bool(
                "Tristate",
                "Tristate",
                BufferGate::get_tristate,
                BufferGate::set_tristate,
            )
            .with_info("High-impedance (tri-state) output stage.");
            p.structural = true;
            p
        };
        const SMALL: PropDef<BufferGate> = {
            let mut p = PropDef::bool(
                "Small",
                "Small size",
                BufferGate::get_small,
                BufferGate::set_small,
            )
            .with_info("Use a smaller body.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<BufferGate>] = &[
            PropDef::bool(
                "InvertInputs",
                "Invert Inputs",
                BufferGate::get_invert_inputs,
                BufferGate::set_invert_inputs,
            )
            .with_info("Invert input pins."),
            PropDef::bool(
                "InvertOutputs",
                "Invert Outputs",
                BufferGate::get_invert_outputs,
                BufferGate::set_invert_outputs,
            )
            .with_info("Invert output pins."),
            PropDef::bool(
                "InitHigh",
                "Init High",
                BufferGate::get_init_high,
                BufferGate::set_init_high,
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

impl Stampable for BufferGate {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for BufferGate {
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

impl crate::canvas::Scene {
    pub fn add_buffer(&mut self, x: f64, y: f64) -> String {
        let id = format!("Buffer-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::gate(
            &id, x, y, "Buffer", 1, false, false, false, false, false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_buffer_gate() {
        let g = BufferGate::default();
        assert_eq!(g.type_id(), "BufferGate");
        assert!(!g.invert_inputs);
        assert!(!g.invert_output);
        assert!(!g.init_high);
        assert!(!g.tristate);
        assert!(!g.small);
        assert_eq!(g.pin_geoms().len(), 2);
    }
}
