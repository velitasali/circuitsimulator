//! LM555 timer IC component.

use super::drawable::{Drawable, paint_chip_body};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::digital::Lm555State;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_V: f64 = -100.0;
const MAX_V: f64 = 100.0;

/// LM555 timer IC.
#[derive(Clone, Debug, PartialEq)]
pub struct Lm555 {
    pub out_high_v: f64,
    pub out_low_v: f64,
}

impl crate::canvas::Item {
    pub fn lm555(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Lm555 {
                out_high_v: 5.0,
                out_low_v: 0.0,
            },
        )
    }
}

impl Default for Lm555 {
    fn default() -> Self {
        Self {
            out_high_v: 5.0,
            out_low_v: 0.0,
        }
    }
}

impl Lm555 {
    pub const TYPE_ID: &'static str = "Lm555";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Lm555(Lm555State::new(""))
    }

    fn get_out_high_v(&self) -> PropValue {
        PropValue::Float(self.out_high_v)
    }
    fn set_out_high_v(&mut self, v: PropValue) -> Result<(), PropError> {
        self.out_high_v = expect_float("OutHighV", v)?.clamp(0.0, MAX_V);
        Ok(())
    }

    fn get_out_low_v(&self) -> PropValue {
        PropValue::Float(self.out_low_v)
    }
    fn set_out_low_v(&mut self, v: PropValue) -> Result<(), PropError> {
        self.out_low_v = expect_float("OutLowV", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
}

impl Component for Lm555 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "LM555 Timer IC."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Lm555>] = &[
            PropDef::float(
                "OutHighV",
                "High Output Voltage",
                "V",
                0.0,
                MAX_V,
                Lm555::get_out_high_v,
                Lm555::set_out_high_v,
            )
            .with_info("Output voltage for a logic High."),
            PropDef::float(
                "OutLowV",
                "Low Output Voltage",
                "V",
                MIN_V,
                MAX_V,
                Lm555::get_out_low_v,
                Lm555::set_out_low_v,
            )
            .with_info("Output voltage for a logic Low."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        lm555_pins("").into_iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -20.0, 32.0, 40.0)
    }
}

impl Stampable for Lm555 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Lm555 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_chip_body(d, ctx.pal, self.body(), "555", true);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_lm555(&mut self, x: f64, y: f64) -> String {
        let id = format!("Lm555-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::lm555(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lm555() {
        let lm = Lm555::default();
        assert_eq!(lm.type_id(), "Lm555");
        assert_eq!(lm.out_high_v, 5.0);
        assert_eq!(lm.out_low_v, 0.0);
        assert_eq!(lm.pin_geoms().len(), 8);
        assert_eq!(lm.body(), Rect::new(-16.0, -20.0, 32.0, 40.0));
    }

    #[test]
    fn set_voltages() {
        let mut lm = Lm555::default();
        lm.set_prop("OutHighV", PropValue::Float(3.3)).unwrap();
        lm.set_prop("OutLowV", PropValue::Float(0.2)).unwrap();
        assert_eq!(lm.out_high_v, 3.3);
        assert_eq!(lm.out_low_v, 0.2);
    }
}

fn lm555_pins(id: &str) -> Vec<Pin> {
    vec![
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin0"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -12.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "Gnd".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin1"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -4.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "Trg".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin2"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 4.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "Out".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin3"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 12.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "Rst".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin4"),
            item_id: id.to_string(),
            local: Point::new(24.0, 12.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "CV".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin5"),
            item_id: id.to_string(),
            local: Point::new(24.0, 4.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Thr".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin6"),
            item_id: id.to_string(),
            local: Point::new(24.0, -4.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Dis".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-ePin7"),
            item_id: id.to_string(),
            local: Point::new(24.0, -12.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Vcc".into(),
            unused: false,
        },
    ]
}
