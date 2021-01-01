//! OpAmp: operational amplifier with differential input, high gain, and optional supply pins.

use super::component::{PropGroup, stamp_to_ground};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::LOW_IMP;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::opamp::{
    OPAMP_DEFAULT_GAIN, OPAMP_DEFAULT_OUT_IMP, OPAMP_DEFAULT_VOLT_NEG, OPAMP_DEFAULT_VOLT_POS,
    OpAmpState,
};
use crate::matrix::CircMatrix;

const MIN_GAIN: f64 = 1e-12;
const MAX_GAIN: f64 = 1e12;
const MIN_IMP: f64 = 1e-6;
const MAX_IMP: f64 = 1e6;
const MIN_VOLT: f64 = -1000.0;
const MAX_VOLT: f64 = 1000.0;

impl crate::canvas::Item {
    pub fn opamp(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::opamp_with(
            id,
            x,
            y,
            OPAMP_DEFAULT_GAIN,
            OPAMP_DEFAULT_OUT_IMP,
            OPAMP_DEFAULT_VOLT_POS,
            OPAMP_DEFAULT_VOLT_NEG,
            false,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn opamp_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        gain: f64,
        out_imp: f64,
        volt_pos: f64,
        volt_neg: f64,
        power_pins: bool,
        switch_pins: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            OpAmp {
                gain,
                out_imp,
                volt_pos,
                volt_neg,
                power_pins,
                switch_pins,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpAmp {
    pub gain: f64,
    pub out_imp: f64,
    pub volt_pos: f64,
    pub volt_neg: f64,
    pub power_pins: bool,
    pub switch_pins: bool,
}

impl Default for OpAmp {
    fn default() -> Self {
        Self {
            gain: OPAMP_DEFAULT_GAIN,
            out_imp: OPAMP_DEFAULT_OUT_IMP,
            volt_pos: OPAMP_DEFAULT_VOLT_POS,
            volt_neg: OPAMP_DEFAULT_VOLT_NEG,
            power_pins: false,
            switch_pins: false,
        }
    }
}

impl OpAmp {
    pub const TYPE_ID: &'static str = "OpAmp";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = OpAmpState::new();
        state.set_gain(self.gain);
        state.set_out_imp(self.out_imp);
        state.volt_pos = self.volt_pos;
        state.volt_neg = self.volt_neg;
        state.power_pins = self.power_pins;
        state.switch_pins = self.switch_pins;
        Kind::OpAmp { state }
    }

    fn get_gain(&self) -> PropValue {
        PropValue::Float(self.gain)
    }
    fn set_gain(&mut self, v: PropValue) -> Result<(), PropError> {
        self.gain = expect_float("Gain", v)?.clamp(MIN_GAIN, MAX_GAIN);
        Ok(())
    }
    fn get_out_imped(&self) -> PropValue {
        PropValue::Float(self.out_imp)
    }
    fn set_out_imped(&mut self, v: PropValue) -> Result<(), PropError> {
        self.out_imp = expect_float("OutImped", v)?.clamp(MIN_IMP, MAX_IMP);
        Ok(())
    }
    fn get_volt_pos(&self) -> PropValue {
        PropValue::Float(self.volt_pos)
    }
    fn set_volt_pos(&mut self, v: PropValue) -> Result<(), PropError> {
        self.volt_pos = expect_float("VoltPos", v)?.clamp(MIN_VOLT, MAX_VOLT);
        Ok(())
    }
    fn get_volt_neg(&self) -> PropValue {
        PropValue::Float(self.volt_neg)
    }
    fn set_volt_neg(&mut self, v: PropValue) -> Result<(), PropError> {
        self.volt_neg = expect_float("VoltNeg", v)?.clamp(MIN_VOLT, MAX_VOLT);
        Ok(())
    }
    fn get_power_pins(&self) -> PropValue {
        PropValue::Bool(self.power_pins)
    }
    fn set_power_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        self.power_pins = expect_bool("PowerPins", v)?;
        Ok(())
    }
    fn get_switch_pins(&self) -> PropValue {
        PropValue::Bool(self.switch_pins)
    }
    fn set_switch_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        self.switch_pins = expect_bool("SwitchPins", v)?;
        Ok(())
    }
}

impl Component for OpAmp {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Operational amplifier."
    }

    fn props() -> &'static [PropDef<Self>] {
        const POWER_PINS: PropDef<OpAmp> = {
            let mut p = PropDef::bool(
                "PowerPins",
                "Power Pins",
                OpAmp::get_power_pins,
                OpAmp::set_power_pins,
            )
            .with_info("Use supply pins instead of values above.");
            p.structural = true;
            p
        };
        const SWITCH_PINS: PropDef<OpAmp> = {
            let mut p = PropDef::bool(
                "SwitchPins",
                "Switch Pins",
                OpAmp::get_switch_pins,
                OpAmp::set_switch_pins,
            )
            .with_info("Use supply pins instead of values above.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<OpAmp>] = &[
            PropDef::float(
                "Gain",
                "Gain",
                "",
                MIN_GAIN,
                MAX_GAIN,
                OpAmp::get_gain,
                OpAmp::set_gain,
            )
            .with_info("Voltage gain."),
            PropDef::float(
                "OutImped",
                "Output Impedance",
                "Ω",
                MIN_IMP,
                MAX_IMP,
                OpAmp::get_out_imped,
                OpAmp::set_out_imped,
            )
            .with_info("Impedance of the output stage."),
            PropDef::float(
                "VoltPos",
                "Positive Supply",
                "V",
                MIN_VOLT,
                MAX_VOLT,
                OpAmp::get_volt_pos,
                OpAmp::set_volt_pos,
            )
            .with_info("Positive supply voltage if supply pins not used."),
            PropDef::float(
                "VoltNeg",
                "Negative Supply",
                "V",
                MIN_VOLT,
                MAX_VOLT,
                OpAmp::get_volt_neg,
                OpAmp::set_volt_neg,
            )
            .with_info("Negative supply voltage if supply pins not used."),
            POWER_PINS,
            SWITCH_PINS,
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let mut rows = self.prop_rows();
        for r in &mut rows {
            match r.name {
                "SwitchPins" => r.visible = self.power_pins,
                "VoltPos" | "VoltNeg" => r.visible = !self.power_pins,
                _ => {}
            }
        }
        super::group_rows_by(
            rows,
            &[
                ("Main", &["Gain", "OutImped"]),
                ("Supply", &["PowerPins", "SwitchPins", "VoltPos", "VoltNeg"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let mut pins = vec![
            CompPin::new("-inputNinv", -24.0, -8.0, 180, 8.0).with_direction(PinDirection::In),
            CompPin::new("-inputInv", -24.0, 8.0, 180, 8.0).with_direction(PinDirection::In),
            CompPin::new("-output", 24.0, 0.0, 0, 8.0).with_direction(PinDirection::Out),
        ];
        if self.power_pins {
            if self.switch_pins {
                pins.push(CompPin::new("-powerPos", 0.0, 16.0, 270, 8.0));
                pins.push(CompPin::new("-powerNeg", 0.0, -16.0, 90, 8.0));
            } else {
                pins.push(CompPin::new("-powerPos", 0.0, -16.0, 90, 8.0));
                pins.push(CompPin::new("-powerNeg", 0.0, 16.0, 270, 8.0));
            }
        }
        pins
    }

    fn body(&self) -> Rect {
        Rect::new(-18.0, -16.0, 36.0, 32.0)
    }
}

impl Stampable for OpAmp {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        // pin 2 is -output
        let g = 1.0 / self.out_imp.max(LOW_IMP);
        stamp_to_ground(matrix, pin_nodes, 2, 0.0, g);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for OpAmp {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let pts = [[-16.0, -16.0], [-16.0, 16.0], [16.0, 1.0], [16.0, -1.0]];
        d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        d.text(-13.0, -12.0, "+", 8.0, ctx.pal.border, Align::TopLeft);
        d.text(-13.0, 3.0, "-", 8.0, ctx.pal.border, Align::TopLeft);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_opamp(&mut self, x: f64, y: f64) -> String {
        let id = format!("opAmp-{}", self.next_opamp);
        self.next_opamp += 1;
        self.items.push(crate::canvas::Item::opamp(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_opamp_has_3_pins() {
        let op = OpAmp::default();
        assert_eq!(op.type_id(), "OpAmp");
        assert_eq!(op.pin_geoms().len(), 3);
    }

    #[test]
    fn power_pins_adds_2_pins() {
        let mut op = OpAmp::default();
        let change = op.set_prop("PowerPins", PropValue::Bool(true)).unwrap();
        assert!(change.structural);
        assert_eq!(op.pin_geoms().len(), 5);
        assert_eq!(op.pin_geoms()[3].suffix, "-powerPos");
        assert_eq!(op.pin_geoms()[4].suffix, "-powerNeg");
    }
}
