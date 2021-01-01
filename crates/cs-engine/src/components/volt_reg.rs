//! VoltReg: linear voltage regulator with reference pin.

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::volt_reg::{VOLTREG_ADMIT, VOLTREG_DEFAULT_VREF, VoltRegState};
use crate::matrix::CircMatrix;

const MIN_VOLT: f64 = 0.0;
const MAX_VOLT: f64 = 1000.0;

impl crate::canvas::Item {
    pub fn volt_reg(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::volt_reg_with(id, x, y, VOLTREG_DEFAULT_VREF)
    }

    pub fn volt_reg_with(id: impl Into<String>, x: f64, y: f64, voltage: f64) -> Self {
        Self::new(id, x, y, VoltReg { voltage })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VoltReg {
    pub voltage: f64,
}

impl Default for VoltReg {
    fn default() -> Self {
        Self {
            voltage: VOLTREG_DEFAULT_VREF,
        }
    }
}

impl VoltReg {
    pub const TYPE_ID: &'static str = "VoltReg";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = VoltRegState::new();
        state.set_out_volt(self.voltage);
        Kind::VoltReg { state }
    }

    fn get_voltage(&self) -> PropValue {
        PropValue::Float(self.voltage)
    }
    fn set_voltage(&mut self, v: PropValue) -> Result<(), PropError> {
        self.voltage = expect_float("Voltage", v)?.clamp(MIN_VOLT, MAX_VOLT);
        Ok(())
    }
}

impl Component for VoltReg {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Voltage regulator."
    }

    fn props() -> &'static [PropDef<Self>] {
        const VOLT: PropDef<VoltReg> = {
            let mut p = PropDef::float(
                "Voltage",
                "Voltage",
                "V",
                MIN_VOLT,
                MAX_VOLT,
                VoltReg::get_voltage,
                VoltReg::set_voltage,
            )
            .with_info("Output voltage in reference to \"R\" pin.")
            .with_info("Output voltage in reference to \"R\" pin.")
            .with_info("Output voltage in reference to \"R\" pin.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<VoltReg>] = &[VOLT];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-input", -16.0, 0.0, 180, 5.0).with_direction(PinDirection::In),
            CompPin::new("-output", 16.0, 0.0, 0, 5.0).with_direction(PinDirection::Out),
            CompPin::new("-ref", 0.0, 16.0, 270, 8.0).with_direction(PinDirection::In),
        ]
    }

    fn body(&self) -> Rect {
        Rect::new(-11.0, -8.0, 22.0, 16.0)
    }
}

impl Stampable for VoltReg {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin 0 = input, pin 1 = output, pin 2 = ref
        stamp_conductance_between(matrix, pin_nodes, 0, 1, VOLTREG_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for VoltReg {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(
            -11.0,
            -8.0,
            22.0,
            16.0,
            1.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -11.0,
            -8.0,
            22.0,
            16.0,
            1.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.text(-8.0, -7.0, "I", 7.0, ctx.pal.border, Align::TopLeft);
        d.text(3.0, -7.0, "O", 7.0, ctx.pal.border, Align::TopLeft);
        d.text(-2.0, 1.0, "R", 7.0, ctx.pal.border, Align::TopLeft);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_volt_reg(&mut self, x: f64, y: f64) -> String {
        let id = format!("VoltReg-{}", self.next_voltreg);
        self.next_voltreg += 1;
        self.items.push(crate::canvas::Item::volt_reg(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_volt_reg() {
        let vr = VoltReg::default();
        assert_eq!(vr.type_id(), "VoltReg");
        assert_eq!(vr.voltage, VOLTREG_DEFAULT_VREF);
        assert_eq!(vr.pin_geoms().len(), 3);
    }
}
