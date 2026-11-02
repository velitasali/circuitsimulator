//! Stepper motor component.

use super::component::{resistor_g, stamp_conductance_between};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::elements::{Kind, StepperState};
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_STEPS: i64 = 1;
const MAX_STEPS: i64 = 10_000;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;
const DEFAULT_HOLDING_TORQUE: f64 = 0.4;
const DEFAULT_INERTIA: f64 = 6e-6;
const DEFAULT_FRICTION: f64 = 0.004;
const DEFAULT_LOAD_TORQUE: f64 = 0.0;

impl crate::canvas::Item {
    pub fn stepper(
        id: impl Into<String>,
        x: f64,
        y: f64,
        bipolar: bool,
        steps: i32,
        resistance: f64,
    ) -> Self {
        Self::stepper_with(id, x, y, bipolar, steps, resistance)
    }

    pub fn stepper_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        bipolar: bool,
        steps: i32,
        resistance: f64,
    ) -> Self {
        let mut motor = Stepper::default();
        motor.bipolar = bipolar;
        motor.steps = steps;
        motor.resistance = resistance;
        Self::new(id, x, y, motor)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stepper {
    pub bipolar: bool,
    pub steps: i32,
    pub resistance: f64,
    pub holding_torque: f64,
    pub inertia: f64,
    pub friction: f64,
    pub load_torque: f64,
    pub angle: f64,
}

impl Default for Stepper {
    fn default() -> Self {
        Self {
            bipolar: false,
            steps: 32,
            resistance: 100.0,
            holding_torque: DEFAULT_HOLDING_TORQUE,
            inertia: DEFAULT_INERTIA,
            friction: DEFAULT_FRICTION,
            load_torque: DEFAULT_LOAD_TORQUE,
            angle: 0.0,
        }
    }
}

impl Stepper {
    pub const TYPE_ID: &'static str = "Stepper";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Stepper {
            bipolar: self.bipolar,
            steps: self.steps,
            resistance: self.resistance,
            angle: self.angle,
            state: StepperState::new(
                self.holding_torque,
                self.inertia,
                self.friction,
                self.load_torque,
            ),
        }
    }

    fn get_bipolar(&self) -> PropValue {
        PropValue::Bool(self.bipolar)
    }
    fn set_bipolar(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bipolar = expect_bool("Bipolar", v)?;
        Ok(())
    }

    fn get_steps(&self) -> PropValue {
        PropValue::Int(self.steps as i64)
    }
    fn set_steps(&mut self, v: PropValue) -> Result<(), PropError> {
        self.steps = expect_int("Steps", v)?.clamp(MIN_STEPS, MAX_STEPS) as i32;
        Ok(())
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_holding_torque(&self) -> PropValue {
        PropValue::Float(self.holding_torque)
    }
    fn set_holding_torque(&mut self, v: PropValue) -> Result<(), PropError> {
        self.holding_torque = expect_float("HoldingTorque", v)?.clamp(0.001, 100.0);
        Ok(())
    }

    fn get_inertia(&self) -> PropValue {
        PropValue::Float(self.inertia)
    }
    fn set_inertia(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inertia = expect_float("Inertia", v)?.clamp(1e-8, 1.0);
        Ok(())
    }

    fn get_friction(&self) -> PropValue {
        PropValue::Float(self.friction)
    }
    fn set_friction(&mut self, v: PropValue) -> Result<(), PropError> {
        self.friction = expect_float("Friction", v)?.clamp(0.0, 10.0);
        Ok(())
    }

    fn get_load_torque(&self) -> PropValue {
        PropValue::Float(self.load_torque)
    }
    fn set_load_torque(&mut self, v: PropValue) -> Result<(), PropError> {
        self.load_torque = expect_float("LoadTorque", v)?.clamp(0.0, 100.0);
        Ok(())
    }
}

impl Component for Stepper {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Step by step Motor."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BIPOLAR: PropDef<Stepper> = {
            let mut p = PropDef::bool(
                "Bipolar",
                "Bipolar",
                Stepper::get_bipolar,
                Stepper::set_bipolar,
            ).with_info("True: bipolar winding, no common wire.\nFalse: unipolar winding, with center-tapped common wire.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Stepper>] = &[
            BIPOLAR,
            PropDef::int(
                "Steps",
                "Steps per Rotation",
                MIN_STEPS,
                MAX_STEPS,
                Stepper::get_steps,
                Stepper::set_steps,
            )
            .with_info("Number of steps per full revolution."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Stepper::get_resistance,
                Stepper::set_resistance,
            )
            .with_info("Resistance of each winding."),
            PropDef::float(
                "HoldingTorque",
                "Holding Torque",
                "N·m",
                0.001,
                100.0,
                Stepper::get_holding_torque,
                Stepper::set_holding_torque,
            )
            .with_info("Peak holding torque at rated current."),
            PropDef::float(
                "Inertia",
                "Rotor Inertia",
                "kg·m²",
                1e-8,
                1.0,
                Stepper::get_inertia,
                Stepper::set_inertia,
            )
            .with_info("Mechanical rotor and shaft moment of inertia."),
            PropDef::float(
                "Friction",
                "Viscous Friction",
                "N·m·s/rad",
                0.0,
                10.0,
                Stepper::get_friction,
                Stepper::set_friction,
            )
            .with_info("Rotor bearing and aerodynamic viscous damping coefficient."),
            PropDef::float(
                "LoadTorque",
                "Load Torque",
                "N·m",
                0.0,
                100.0,
                Stepper::get_load_torque,
                Stepper::set_load_torque,
            )
            .with_info("External mechanical load torque opposing rotation."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        stepper_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-40.0, -40.0, 80.0, 80.0)
    }
}

impl Stampable for Stepper {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let g = resistor_g(self.resistance);
        if self.bipolar {
            stamp_conductance_between(matrix, pin_nodes, 0, 3, g);
            stamp_conductance_between(matrix, pin_nodes, 1, 4, g);
        } else {
            stamp_conductance_between(matrix, pin_nodes, 0, 2, g);
            stamp_conductance_between(matrix, pin_nodes, 3, 2, g);
            stamp_conductance_between(matrix, pin_nodes, 1, 2, g);
            stamp_conductance_between(matrix, pin_nodes, 4, 2, g);
        }
    }
}

impl Drawable for Stepper {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(-40.0, -40.0, 80.0, 80.0, 4.0, ctx.pal.body);
        d.stroke_round_rect(
            -40.0,
            -40.0,
            80.0,
            80.0,
            4.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.text(0.0, -34.0, "STEPPER", 8.0, ctx.pal.border, Align::HCenter);
        // Rotor disc
        d.stroke_circle(0.0, 0.0, 24.0, ctx.pal.border, 1.0);
        // Rotating shaft
        d.push(0.0, 0.0, self.angle, 1.0, 1.0);
        d.fill_circle(0.0, 0.0, 12.0, ctx.pal.body);
        d.stroke_circle(0.0, 0.0, 12.0, ctx.pal.border, 1.5);
        d.fill_round_rect(-2.0, -10.0, 4.0, 6.0, 1.0, ctx.pal.band);
        d.pop();
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_stepper(&mut self, x: f64, y: f64) -> String {
        let id = format!("Stepper-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Stepper::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stepper() {
        let s = Stepper::default();
        assert_eq!(s.type_id(), "Stepper");
        assert!(!s.bipolar);
        assert_eq!(s.steps, 32);
        assert_eq!(s.resistance, 100.0);
        assert_eq!(s.pin_geoms().len(), 5);
    }
}

const STEPPER_PINS: [PinGeom; 5] = [
    PinGeom {
        suffix: "-PinA1",
        x: -48.0,
        y: -32.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinB1",
        x: -48.0,
        y: -16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinCo",
        x: -48.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinA2",
        x: -48.0,
        y: 16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinB2",
        x: -48.0,
        y: 32.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn stepper_pins() -> &'static [PinGeom] {
    &STEPPER_PINS
}
