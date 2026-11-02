//! RC servo motor component.

use super::component::{stamp_conductance_between, stamp_to_ground};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Color, Draw, PaintCtx};
use crate::elements::{Kind, ServoState};
use crate::matrix::CircMatrix;
use crate::theme::COMPONENT_BORDER_WIDTH;

const MIN_SPEED: f64 = 0.001;
const MAX_SPEED: f64 = 100.0;
const MIN_PULSE: f64 = 100.0;
const MAX_PULSE: f64 = 10_000.0;
const DEFAULT_IDLE_CURRENT: f64 = 0.015;
const DEFAULT_RUN_CURRENT: f64 = 0.25;
const DEFAULT_STALL_CURRENT: f64 = 1.0;
const DEFAULT_LOAD_TORQUE: f64 = 0.0;
const DEFAULT_STALL_TORQUE: f64 = 3.5;

impl crate::canvas::Item {
    pub fn servo(
        id: impl Into<String>,
        x: f64,
        y: f64,
        speed: f64,
        min_pulse: f64,
        max_pulse: f64,
    ) -> Self {
        Self::servo_with(id, x, y, speed, min_pulse, max_pulse)
    }

    pub fn servo_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        speed: f64,
        min_pulse: f64,
        max_pulse: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Servo {
                speed,
                min_pulse,
                max_pulse,
                idle_current: DEFAULT_IDLE_CURRENT,
                run_current: DEFAULT_RUN_CURRENT,
                stall_current: DEFAULT_STALL_CURRENT,
                load_torque: DEFAULT_LOAD_TORQUE,
                stall_torque: DEFAULT_STALL_TORQUE,
                pos: 90.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Servo {
    pub speed: f64,
    pub min_pulse: f64,
    pub max_pulse: f64,
    pub idle_current: f64,
    pub run_current: f64,
    pub stall_current: f64,
    pub load_torque: f64,
    pub stall_torque: f64,
    pub pos: f64,
}

impl Default for Servo {
    fn default() -> Self {
        Self {
            speed: 0.2,
            min_pulse: 1000.0,
            max_pulse: 2000.0,
            idle_current: DEFAULT_IDLE_CURRENT,
            run_current: DEFAULT_RUN_CURRENT,
            stall_current: DEFAULT_STALL_CURRENT,
            load_torque: DEFAULT_LOAD_TORQUE,
            stall_torque: DEFAULT_STALL_TORQUE,
            pos: 90.0,
        }
    }
}

impl Servo {
    pub const TYPE_ID: &'static str = "Servo";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Servo {
            speed: self.speed,
            min_pulse: self.min_pulse,
            max_pulse: self.max_pulse,
            pos: self.pos,
            target_pos: self.pos,
            pulse_start_ps: 0,
            sig_high: false,
            state: ServoState::new(
                self.idle_current,
                self.run_current,
                self.stall_current,
                self.load_torque,
                self.stall_torque,
            ),
        }
    }

    fn get_speed(&self) -> PropValue {
        PropValue::Float(self.speed)
    }
    fn set_speed(&mut self, v: PropValue) -> Result<(), PropError> {
        self.speed = expect_float("Speed", v)?.clamp(MIN_SPEED, MAX_SPEED);
        Ok(())
    }

    fn get_min_pulse(&self) -> PropValue {
        PropValue::Float(self.min_pulse)
    }
    fn set_min_pulse(&mut self, v: PropValue) -> Result<(), PropError> {
        self.min_pulse = expect_float("MinPulse", v)?.clamp(MIN_PULSE, MAX_PULSE);
        Ok(())
    }

    fn get_max_pulse(&self) -> PropValue {
        PropValue::Float(self.max_pulse)
    }
    fn set_max_pulse(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_pulse = expect_float("MaxPulse", v)?.clamp(MIN_PULSE, MAX_PULSE);
        Ok(())
    }

    fn get_idle_current(&self) -> PropValue {
        PropValue::Float(self.idle_current)
    }
    fn set_idle_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.idle_current = expect_float("IdleCurrent", v)?.clamp(0.0, 1.0);
        Ok(())
    }

    fn get_run_current(&self) -> PropValue {
        PropValue::Float(self.run_current)
    }
    fn set_run_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.run_current = expect_float("RunCurrent", v)?.clamp(0.001, 5.0);
        Ok(())
    }

    fn get_stall_current(&self) -> PropValue {
        PropValue::Float(self.stall_current)
    }
    fn set_stall_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.stall_current = expect_float("StallCurrent", v)?.clamp(0.01, 20.0);
        Ok(())
    }

    fn get_load_torque(&self) -> PropValue {
        PropValue::Float(self.load_torque)
    }
    fn set_load_torque(&mut self, v: PropValue) -> Result<(), PropError> {
        self.load_torque = expect_float("LoadTorque", v)?.clamp(0.0, 50.0);
        Ok(())
    }

    fn get_stall_torque(&self) -> PropValue {
        PropValue::Float(self.stall_torque)
    }
    fn set_stall_torque(&mut self, v: PropValue) -> Result<(), PropError> {
        self.stall_torque = expect_float("StallTorque", v)?.clamp(0.01, 50.0);
        Ok(())
    }
}

impl Component for Servo {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Configurable servo motor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Servo>] = &[
            PropDef::float(
                "Speed",
                "Speed (s/60°)",
                "",
                MIN_SPEED,
                MAX_SPEED,
                Servo::get_speed,
                Servo::set_speed,
            )
            .with_info("Time to rotate 60º."),
            PropDef::float(
                "MinPulse",
                "Min. Pulse Width (µs)",
                "",
                MIN_PULSE,
                MAX_PULSE,
                Servo::get_min_pulse,
                Servo::set_min_pulse,
            )
            .with_info("Pulse width for rotation = 0º"),
            PropDef::float(
                "MaxPulse",
                "Max. Pulse Width (µs)",
                "",
                MIN_PULSE,
                MAX_PULSE,
                Servo::get_max_pulse,
                Servo::set_max_pulse,
            )
            .with_info("Pulse width for rotation = 180º"),
            PropDef::float(
                "IdleCurrent",
                "Idle Current",
                "A",
                0.0,
                1.0,
                Servo::get_idle_current,
                Servo::set_idle_current,
            )
            .with_info("Quiescent current when holding steady state."),
            PropDef::float(
                "RunCurrent",
                "Running Current",
                "A",
                0.001,
                5.0,
                Servo::get_run_current,
                Servo::set_run_current,
            )
            .with_info("Operating current while rotating towards target."),
            PropDef::float(
                "StallCurrent",
                "Stall Current",
                "A",
                0.01,
                20.0,
                Servo::get_stall_current,
                Servo::set_stall_current,
            )
            .with_info("Maximum current when jammed or under max stall torque."),
            PropDef::float(
                "LoadTorque",
                "Load Torque",
                "kg·cm",
                0.0,
                50.0,
                Servo::get_load_torque,
                Servo::set_load_torque,
            )
            .with_info("External mechanical load torque opposing rotation."),
            PropDef::float(
                "StallTorque",
                "Stall Torque",
                "kg·cm",
                0.01,
                50.0,
                Servo::get_stall_torque,
                Servo::set_stall_torque,
            )
            .with_info("Maximum holding / stall torque capability of the servo."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        servo_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-40.0, -24.0, 80.0, 48.0)
    }

    fn visual_rect(&self) -> Rect {
        // Horn is a stadium about (16, 0): local (-8, -8, 48, 16). The right
        // cap sits at x=32 with r=8; dirty regions must cover every angle so
        // a rotate does not leave a ghost outside the enclosure.
        const HORN_PIVOT_X: f64 = 16.0;
        const HORN_REACH: f64 = 42.0;
        self.body().united(Rect::new(
            HORN_PIVOT_X - HORN_REACH,
            -HORN_REACH,
            HORN_REACH * 2.0,
            HORN_REACH * 2.0,
        ))
    }
}

impl Stampable for Servo {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // High input impedance on Sig pin (index 2)
        stamp_to_ground(matrix, pin_nodes, 2, 0.0, 1e-7);
        // Quiescent power consumption across V+ (index 0) and Gnd (index 1)
        let g_idle = (self.idle_current / 5.0).max(1e-6);
        stamp_conductance_between(matrix, pin_nodes, 0, 1, g_idle);
    }
}

impl Drawable for Servo {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Main servo enclosure
        d.fill_round_rect(-40.0, -24.0, 80.0, 48.0, 4.0, Color::rgb(50, 70, 100));
        d.stroke_round_rect(
            -40.0,
            -24.0,
            80.0,
            48.0,
            4.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Horn shaft circle at (16, 0)
        d.stroke_circle(16.0, 0.0, 16.0, Color::rgb(255, 255, 255), 1.0);

        // Rotating horn at (16, 0) rotated by pos - 90
        d.push(16.0, 0.0, self.pos - 90.0, 1.0, 1.0);
        d.fill_round_rect(-8.0, -8.0, 48.0, 16.0, 8.0, Color::rgb(255, 255, 255));
        d.stroke_round_rect(-8.0, -8.0, 48.0, 16.0, 8.0, ctx.pal.border, 1.0);
        d.fill_circle(0.0, 0.0, 3.0, ctx.pal.border);
        d.pop();

        d.text(
            -18.0,
            0.0,
            "SERVO",
            7.0,
            Color::rgb(255, 255, 255),
            Align::Center,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_servo(&mut self, x: f64, y: f64) -> String {
        let id = format!("Servo-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Servo::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_servo() {
        let s = Servo::default();
        assert_eq!(s.type_id(), "Servo");
        assert_eq!(s.speed, 0.2);
        assert_eq!(s.min_pulse, 1000.0);
        assert_eq!(s.max_pulse, 2000.0);
        assert_eq!(s.pin_geoms().len(), 3);
        let v = s.visual_rect();
        assert!(
            v.contains_point(crate::canvas::Point::new(56.0, 0.0)),
            "horn tip at 90° must sit in visual_rect, got {v:?}"
        );
        assert!(
            v.contains_point(crate::canvas::Point::new(16.0, 40.0)),
            "horn tip at 180° must sit in visual_rect, got {v:?}"
        );
    }
}

const SERVO_PINS: [PinGeom; 3] = [
    PinGeom {
        suffix: "-PinV+",
        x: -48.0,
        y: -16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinGnd",
        x: -48.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinSig",
        x: -48.0,
        y: 16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn servo_pins() -> &'static [PinGeom] {
    &SERVO_PINS
}
