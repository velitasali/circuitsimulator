//! DS1621 I2C digital thermometer.

use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_TEMP: f64 = -55.0;
const MAX_TEMP: f64 = 125.0;
const MIN_INC: f64 = 0.01;
const MAX_INC: f64 = 50.0;

impl crate::canvas::Item {
    pub fn ds1621(id: impl Into<String>, x: f64, y: f64, temp: f64, temp_inc: f64) -> Self {
        Self::new(id, x, y, DS1621 { temp, temp_inc })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DS1621 {
    pub temp: f64,
    pub temp_inc: f64,
}

impl Default for DS1621 {
    fn default() -> Self {
        Self {
            temp: 25.0,
            temp_inc: 0.5,
        }
    }
}

impl DS1621 {
    pub const TYPE_ID: &'static str = "DS1621";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ds1621 {
            temp: self.temp,
            th: 30.0,
            tl: 10.0,
            active: true,
            tout_high: false,
            sda_low: false,
            scl_low: false,
        }
    }

    fn get_temp(&self) -> PropValue {
        PropValue::Float(self.temp)
    }
    fn set_temp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.temp = expect_float("Temp", v)?.clamp(MIN_TEMP, MAX_TEMP);
        Ok(())
    }

    fn get_temp_inc(&self) -> PropValue {
        PropValue::Float(self.temp_inc)
    }
    fn set_temp_inc(&mut self, v: PropValue) -> Result<(), PropError> {
        self.temp_inc = expect_float("TempInc", v)?.clamp(MIN_INC, MAX_INC);
        Ok(())
    }
}

impl Component for DS1621 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Temperature sensor with I2C interface."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<DS1621>] = &[
            PropDef::float(
                "Temp",
                "Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                DS1621::get_temp,
                DS1621::set_temp,
            )
            .with_info("Current temperature."),
            PropDef::float(
                "TempInc",
                "Temp. increment",
                "°C",
                MIN_INC,
                MAX_INC,
                DS1621::get_temp_inc,
                DS1621::set_temp_inc,
            )
            .with_info("Temperature will increment by this value when + or - buttons are pushed."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ds1621_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-20.0, -28.0, 40.0, 56.0)
    }
}

impl Stampable for DS1621 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for DS1621 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // DIP-8 Body (40x56)
        d.fill_round_rect(
            -20.0,
            -28.0,
            40.0,
            56.0,
            3.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -20.0,
            -28.0,
            40.0,
            56.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Top notch for pin 1 orientation
        d.arc(0.0, -28.0, 4.0, 0.0, 180.0, ctx.pal.border, 1.0);

        // Chip title
        d.text(0.0, -18.0, "DS1621", 7.0, ctx.pal.border, Align::Center);

        // Temperature readout banner
        let t_str = format!("{:.1} °C", self.temp);
        d.text(0.0, 0.0, &t_str, 8.0, ctx.pal.border, Align::Center);

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ds1621(&mut self, x: f64, y: f64) -> String {
        let id = format!("DS1621-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::ds1621(&id, x, y, 25.0, 0.5));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ds1621() {
        let d = DS1621::default();
        assert_eq!(d.type_id(), "DS1621");
        assert_eq!(d.temp, 25.0);
        assert_eq!(d.temp_inc, 0.5);
        assert_eq!(d.pin_geoms().len(), 8);
    }
}

const DS1621_PINS: [PinGeom; 8] = [
    PinGeom {
        suffix: "-inPin0",
        x: -24.0,
        y: -8.0,
        angle: 180,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-inPin1",
        x: -24.0,
        y: 0.0,
        angle: 180,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-outPin0",
        x: -24.0,
        y: 8.0,
        angle: 180,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinGnd",
        x: -24.0,
        y: 16.0,
        angle: 180,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinVdd",
        x: 24.0,
        y: -8.0,
        angle: 0,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-inPin2",
        x: 24.0,
        y: 0.0,
        angle: 0,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-inPin3",
        x: 24.0,
        y: 8.0,
        angle: 0,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-inPin4",
        x: 24.0,
        y: 16.0,
        angle: 0,
        length: 4.0,
        direction: None,
    },
];

fn ds1621_pins() -> &'static [PinGeom] {
    &DS1621_PINS
}
