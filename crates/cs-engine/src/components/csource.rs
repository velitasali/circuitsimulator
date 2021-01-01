//! Controlled source component (VCVS, VCCS, CCVS, CCCS).

use super::component::{PropGroup, pin_node};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_GAIN: f64 = -1e9;
const MAX_GAIN: f64 = 1e9;
const MIN_V: f64 = -1e6;
const MAX_V: f64 = 1e6;
const MIN_I: f64 = -1e6;
const MAX_I: f64 = 1e6;

#[derive(Clone, Debug, PartialEq)]
pub struct Csource {
    pub control_pins: bool,
    pub curr_source: bool,
    pub curr_control: bool,
    pub gain: f64,
    pub volt: f64,
    pub current: f64,
}

impl crate::canvas::Item {
    pub fn csource(
        id: impl Into<String>,
        x: f64,
        y: f64,
        control_pins: bool,
        curr_source: bool,
        curr_control: bool,
        gain: f64,
        volt: f64,
        current: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Csource {
                control_pins,
                curr_source,
                curr_control,
                gain,
                volt,
                current,
            },
        )
    }
}

impl Default for Csource {
    fn default() -> Self {
        Self {
            control_pins: true,
            curr_source: false,
            curr_control: false,
            gain: 1.0,
            volt: 0.0,
            current: 0.0,
        }
    }
}

impl Csource {
    pub const TYPE_ID: &'static str = "Csource";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Csource {
            control_pins: self.control_pins,
            curr_source: self.curr_source,
            curr_control: self.curr_control,
            gain: self.gain,
            volt: self.volt,
            current: self.current,
        }
    }

    fn get_curr_source(&self) -> PropValue {
        PropValue::Bool(self.curr_source)
    }
    fn set_curr_source(&mut self, v: PropValue) -> Result<(), PropError> {
        self.curr_source = expect_bool("CurrSource", v)?;
        Ok(())
    }

    fn get_control_pins(&self) -> PropValue {
        PropValue::Bool(self.control_pins)
    }
    fn set_control_pins(&mut self, v: PropValue) -> Result<(), PropError> {
        self.control_pins = expect_bool("ControlPins", v)?;
        Ok(())
    }

    fn get_curr_control(&self) -> PropValue {
        PropValue::Bool(self.curr_control)
    }
    fn set_curr_control(&mut self, v: PropValue) -> Result<(), PropError> {
        self.curr_control = expect_bool("CurrControl", v)?;
        Ok(())
    }

    fn get_gain(&self) -> PropValue {
        PropValue::Float(self.gain)
    }
    fn set_gain(&mut self, v: PropValue) -> Result<(), PropError> {
        self.gain = expect_float("Gain", v)?.clamp(MIN_GAIN, MAX_GAIN);
        Ok(())
    }

    fn get_voltage(&self) -> PropValue {
        PropValue::Float(self.volt)
    }
    fn set_voltage(&mut self, v: PropValue) -> Result<(), PropError> {
        self.volt = expect_float("Voltage", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_current(&self) -> PropValue {
        PropValue::Float(self.current)
    }
    fn set_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.current = expect_float("Current", v)?.clamp(MIN_I, MAX_I);
        Ok(())
    }
}

impl Component for Csource {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Controlled source."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Csource>] = &[
            PropDef::bool(
                "CurrSource",
                "Current Source",
                Csource::get_curr_source,
                Csource::set_curr_source,
            ).with_info("true: Current source.\nfalse: Voltage source."),
            PropDef::bool(
                "ControlPins",
                "Control Pins",
                Csource::get_control_pins,
                Csource::set_control_pins,
            ).with_info("true: Variable Source controlled by Pins.\nfalse: Fixed value Source or controlled by linker."),
            PropDef::bool(
                "CurrControl",
                "Current Controlled",
                Csource::get_curr_control,
                Csource::set_curr_control,
            ).with_info("true: Current controlled.\nfalse: Voltage controlled.\nOnly active when controlled by Pins."),
            PropDef::float(
                "Gain",
                "Gain",
                "",
                MIN_GAIN,
                MAX_GAIN,
                Csource::get_gain,
                Csource::set_gain,
            ).with_info("Gain respect to controlling value( current or voltage).\nOnly active when controlled by Pins."),
            PropDef::float(
                "Voltage",
                "Voltage",
                "V",
                MIN_V,
                MAX_V,
                Csource::get_voltage,
                Csource::set_voltage,
            ).with_info("Fixed voltage value for voltage source.\nOnly active when not controlled by Pins."),
            PropDef::float(
                "Current",
                "Current",
                "A",
                MIN_I,
                MAX_I,
                Csource::get_current,
                Csource::set_current,
            ).with_info("Fixed current value for current source.\nOnly active when not controlled by Pins."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let mut rows = self.prop_rows();
        for r in &mut rows {
            match r.name {
                "CurrControl" | "Gain" => r.visible = self.control_pins,
                "Voltage" => r.visible = !self.control_pins && !self.curr_source,
                "Current" => r.visible = !self.control_pins && self.curr_source,
                _ => {}
            }
        }
        vec![PropGroup::new("Main", rows)]
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        csource_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -16.0, 32.0, 32.0)
    }
}

impl Stampable for Csource {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let g = if self.curr_source { 1e-6 } else { 1e7 };
        let i_src = if self.curr_source {
            self.current
        } else {
            self.volt * 1e7
        };
        let n = matrix.n();
        let n2 = pin_node(pin_nodes, 2, n);
        let n3 = pin_node(pin_nodes, 3, n);
        match (n2, n3) {
            (Some(a), Some(b)) if a != b => {
                matrix.add_matrix(a, a, g);
                matrix.add_matrix(b, b, g);
                matrix.add_matrix(a, b, -g);
                matrix.add_matrix(b, a, -g);
                if i_src != 0.0 {
                    matrix.add_coef(a, i_src);
                    matrix.add_coef(b, -i_src);
                }
            }
            (Some(a), None) | (None, Some(a)) | (Some(a), Some(_)) => {
                matrix.add_matrix(a, a, g);
                if i_src != 0.0 {
                    matrix.add_coef(a, i_src);
                }
            }
            _ => {}
        }
    }
}

impl Drawable for Csource {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        if self.control_pins {
            d.fill_rect(
                -16.0,
                -16.0,
                32.0,
                32.0,
                ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
            );
            d.stroke_rect(
                -16.0,
                -16.0,
                32.0,
                32.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }
        if !self.control_pins {
            d.fill_circle(0.0, 0.0, 10.0, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
            d.stroke_circle(0.0, 0.0, 10.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        } else {
            let diamond = [[0.0, -13.0], [8.0, 0.0], [0.0, 13.0], [-8.0, 0.0]];
            d.fill_poly(&diamond, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
            d.stroke_poly(&diamond, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        }
        if self.curr_source {
            d.line(0.0, -5.0, 0.0, 5.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(-2.0, 2.0, 0.0, 5.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(2.0, 2.0, 0.0, 5.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        } else {
            d.line(
                -2.0,
                -4.0,
                2.0,
                -4.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            d.line(0.0, -6.0, 0.0, -2.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            d.line(-2.0, 4.0, 2.0, 4.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }
        if self.curr_control && self.control_pins {
            d.line(-12.0, -7.0, -12.0, 7.0, ctx.pal.border, 0.6);
            d.line(-13.0, 5.0, -12.0, 7.0, ctx.pal.border, 0.6);
            d.line(-11.0, 5.0, -12.0, 7.0, ctx.pal.border, 0.6);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_csource(&mut self, x: f64, y: f64) -> String {
        let id = format!("Csource-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::csource(
            &id, x, y, true, false, false, 1.0, 0.0, 0.0,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_csource() {
        let c = Csource::default();
        assert_eq!(c.type_id(), "Csource");
        assert!(c.control_pins);
        assert!(!c.curr_source);
        assert!(!c.curr_control);
        assert_eq!(c.gain, 1.0);
        assert_eq!(c.volt, 0.0);
        assert_eq!(c.current, 0.0);
        assert_eq!(c.pin_geoms().len(), 4);
        assert_eq!(c.body(), Rect::new(-16.0, -16.0, 32.0, 32.0));
    }
}

const CSOURCE_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-cpPin",
        x: -24.0,
        y: -8.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-cmPin",
        x: -24.0,
        y: 8.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-s1Pin",
        x: 0.0,
        y: -20.0,
        angle: 90,
        length: 8.0,
        direction: Some(PinDirection::Out),
    },
    PinGeom {
        suffix: "-s2Pin",
        x: 0.0,
        y: 20.0,
        angle: 270,
        length: 8.0,
        direction: Some(PinDirection::Out),
    },
];

fn csource_pins() -> &'static [PinGeom] {
    &CSOURCE_PINS
}
