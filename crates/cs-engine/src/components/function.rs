//! Combinational boolean logic function component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_int, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::FunctionState;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_INPUTS: usize = 1;
const MAX_INPUTS: usize = 16;

/// Combinational boolean logic function.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub n_inputs: usize,
    pub expression: String,
}

impl crate::canvas::Item {
    pub fn function(
        id: impl Into<String>,
        x: f64,
        y: f64,
        n_inputs: usize,
        expression: impl Into<String>,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Function {
                n_inputs,
                expression: expression.into(),
            },
        )
    }
}

impl Default for Function {
    fn default() -> Self {
        Self {
            n_inputs: 3,
            expression: String::new(),
        }
    }
}

impl Function {
    pub const TYPE_ID: &'static str = "Function";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Function(FunctionState::new("", self.n_inputs, &self.expression))
    }

    fn get_inputs(&self) -> PropValue {
        PropValue::Int(self.n_inputs as i64)
    }
    fn set_inputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.n_inputs =
            expect_int("Inputs", v)?.clamp(MIN_INPUTS as i64, MAX_INPUTS as i64) as usize;
        Ok(())
    }

    fn get_expression(&self) -> PropValue {
        PropValue::String(self.expression.clone())
    }
    fn set_expression(&mut self, v: PropValue) -> Result<(), PropError> {
        self.expression = expect_string("Expression", v)?;
        Ok(())
    }
}

impl Component for Function {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Combinational Boolean Logic Function."
    }

    fn props() -> &'static [PropDef<Self>] {
        const INPUTS: PropDef<Function> = {
            let mut p = PropDef::int(
                "Inputs",
                "Inputs",
                MIN_INPUTS as i64,
                MAX_INPUTS as i64,
                Function::get_inputs,
                Function::set_inputs,
            ).with_info("Comma-separated names of the pins that drive the inputs of the circuit under test.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Function>] = &[
            INPUTS,
            PropDef::string(
                "Expression",
                "Expression",
                Function::get_expression,
                Function::set_expression,
            )
            .with_info("Mathematical evaluation expression for output voltage."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        function_pins("", self.n_inputs)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = (self.n_inputs as f64 * 8.0).max(24.0);
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for Function {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Function {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "FUNC", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_function(&mut self, x: f64, y: f64) -> String {
        let id = format!("Function-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::function(&id, x, y, 2, "A & B"));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_function() {
        let f = Function::default();
        assert_eq!(f.type_id(), "Function");
        assert_eq!(f.n_inputs, 3);
        assert_eq!(f.expression, "");
        // 3 in + 1 out = 4 pins
        assert_eq!(f.pin_geoms().len(), 4);
        assert_eq!(f.body(), Rect::new(-16.0, -12.0, 32.0, 24.0));
    }

    #[test]
    fn set_props() {
        let mut f = Function::default();
        f.set_prop("Inputs", PropValue::Int(4)).unwrap();
        f.set_prop("Expression", PropValue::String("!I0 & (I1 | I2)".into()))
            .unwrap();
        assert_eq!(f.n_inputs, 4);
        assert_eq!(f.expression, "!I0 & (I1 | I2)");
        assert_eq!(f.pin_geoms().len(), 5);
    }
}

fn function_pins(id: &str, n_inputs: usize) -> Vec<Pin> {
    let n = n_inputs.max(1);
    let mut pins = Vec::with_capacity(n + 1);
    let h = (n as f64) * 8.0;
    let y0 = -h / 2.0 + 4.0;
    // Left inputs: I0, I1... or I
    for i in 0..n {
        let label = if n > 1 { format!("I{i}") } else { "I".into() };
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in{i}"),
            item_id: id.to_string(),
            local: Point::new(-24.0, y0 + (i as f64) * 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label,
            unused: false,
        });
    }
    // Right output: Out
    pins.push(Pin {
        direction: Some(PinDirection::In),
        id: format!("{id}-out0"),
        item_id: id.to_string(),
        local: Point::new(24.0, 0.0),
        angle: 0,
        length: 8.0,
        is_bus: false,
        label: "Out".into(),
        unused: false,
    });
    pins
}
