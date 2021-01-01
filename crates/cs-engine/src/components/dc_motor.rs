//! Direct current (DC) motor component.

use super::component::{resistor_g, stamp_conductance_between};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_RPM: i64 = 1;
const MAX_RPM: i64 = 100_000;
const MIN_V: f64 = 0.01;
const MAX_V: f64 = 1e4;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;

impl crate::canvas::Item {
    pub fn dc_motor(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::dc_motor_with(id, x, y, 3000, 12.0, 100.0)
    }

    pub fn dcmotor(
        id: impl Into<String>,
        x: f64,
        y: f64,
        rpm_nominal: i32,
        volt_nominal: f64,
        resistance: f64,
    ) -> Self {
        Self::dc_motor_with(id, x, y, rpm_nominal, volt_nominal, resistance)
    }

    pub fn dc_motor_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        rpm_nominal: i32,
        volt_nominal: f64,
        resistance: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            DcMotor {
                rpm_nominal,
                volt_nominal,
                resistance,
                speed: 0.0,
                angle: 0.0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DcMotor {
    pub rpm_nominal: i32,
    pub volt_nominal: f64,
    pub resistance: f64,
    pub speed: f64,
    pub angle: f64,
}

impl Default for DcMotor {
    fn default() -> Self {
        Self {
            rpm_nominal: 60,
            volt_nominal: 5.0,
            resistance: 100.0,
            speed: 0.0,
            angle: 0.0,
        }
    }
}

impl DcMotor {
    pub const TYPE_ID: &'static str = "DcMotor";
    pub fn to_element_kind(&self) -> Kind {
        Kind::DcMotor {
            rpm_nominal: self.rpm_nominal,
            volt_nominal: self.volt_nominal,
            resistance: self.resistance,
            speed: self.speed,
            angle: self.angle,
        }
    }

    fn get_rpm_nominal(&self) -> PropValue {
        PropValue::Int(self.rpm_nominal as i64)
    }
    fn set_rpm_nominal(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rpm_nominal = expect_int("RpmNominal", v)?.clamp(MIN_RPM, MAX_RPM) as i32;
        Ok(())
    }

    fn get_volt_nominal(&self) -> PropValue {
        PropValue::Float(self.volt_nominal)
    }
    fn set_volt_nominal(&mut self, v: PropValue) -> Result<(), PropError> {
        self.volt_nominal = expect_float("VoltNominal", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }
}

impl Component for DcMotor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Configurable DC motor with speed/direction indicator."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<DcMotor>] = &[
            PropDef::int(
                "RpmNominal",
                "Nominal Speed",
                MIN_RPM,
                MAX_RPM,
                DcMotor::get_rpm_nominal,
                DcMotor::set_rpm_nominal,
            )
            .with_info("Speed at nominal voltage."),
            PropDef::float(
                "VoltNominal",
                "Nominal Voltage",
                "V",
                MIN_V,
                MAX_V,
                DcMotor::get_volt_nominal,
                DcMotor::set_volt_nominal,
            )
            .with_info("Voltage to reach nominal speed."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                DcMotor::get_resistance,
                DcMotor::set_resistance,
            )
            .with_info("Resistance of the winding."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        dcmotor_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-35.0, -33.0, 70.0, 66.0)
    }
}

impl Stampable for DcMotor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_conductance_between(matrix, pin_nodes, 0, 1, resistor_g(self.resistance));
    }
}

impl Drawable for DcMotor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Outer stator casing (diameter 64)
        d.fill_circle(0.0, 0.0, 32.0, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_circle(0.0, 0.0, 32.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);

        // Terminal lugs
        d.fill_round_rect(-36.0, -4.0, 8.0, 8.0, 4.0, Color::rgb(220, 40, 40));
        d.stroke_round_rect(-36.0, -4.0, 8.0, 8.0, 4.0, ctx.pal.border, 0.8);

        d.fill_round_rect(28.0, -4.0, 8.0, 8.0, 4.0, ctx.pal.border);
        d.stroke_round_rect(28.0, -4.0, 8.0, 8.0, 4.0, ctx.pal.border, 0.8);

        // Inner cavity (diameter 52)
        d.fill_circle(0.0, 0.0, 26.0, ctx.pal.canvas);
        d.stroke_circle(0.0, 0.0, 26.0, ctx.pal.border, 1.0);

        // Rotating rotor (diameter 40)
        d.push(0.0, 0.0, self.angle, 1.0, 1.0);
        d.fill_circle(0.0, 0.0, 20.0, ctx.pal.body.fade(0.85));
        d.stroke_circle(0.0, 0.0, 20.0, ctx.pal.border, 1.0);
        d.line(0.0, 0.0, 0.0, -20.0, ctx.pal.band, 2.0);
        d.fill_circle(0.0, 0.0, 4.0, ctx.pal.border);
        d.pop();

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_dcmotor(&mut self, x: f64, y: f64) -> String {
        let id = format!("DcMotor-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::dc_motor(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dc_motor() {
        let m = DcMotor::default();
        assert_eq!(m.type_id(), "DcMotor");
        assert_eq!(m.rpm_nominal, 60);
        assert_eq!(m.volt_nominal, 5.0);
        assert_eq!(m.resistance, 100.0);
        assert_eq!(m.pin_geoms().len(), 2);
    }
}

const DCMOTOR_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-lPin",
        x: -40.0,
        y: 0.0,
        angle: 180,
        length: 12.0,
        direction: None,
    },
    PinGeom {
        suffix: "-rPin",
        x: 40.0,
        y: 0.0,
        angle: 0,
        length: 12.0,
        direction: None,
    },
];

fn dcmotor_pins() -> &'static [PinGeom] {
    &DCMOTOR_PINS
}
