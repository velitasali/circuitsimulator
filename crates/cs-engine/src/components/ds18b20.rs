//! DS18B20 1-wire digital thermometer.

use super::props::{PropDef, PropError, PropValue, expect_float, expect_string};
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
    pub fn ds18b20(
        id: impl Into<String>,
        x: f64,
        y: f64,
        rom: impl Into<String>,
        temp: f64,
        temp_inc: f64,
    ) -> Self {
        Self::ds18b20_with(id, x, y, rom, temp, temp_inc)
    }

    pub fn ds18b20_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        rom: impl Into<String>,
        temp: f64,
        temp_inc: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            DS18B20 {
                rom: rom.into(),
                temp,
                temp_inc,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DS18B20 {
    pub rom: String,
    pub temp: f64,
    pub temp_inc: f64,
}

impl Default for DS18B20 {
    fn default() -> Self {
        Self {
            rom: "28FF2B450000".into(),
            temp: 25.0,
            temp_inc: 0.5,
        }
    }
}

impl DS18B20 {
    pub const TYPE_ID: &'static str = "DS18B20";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ds18b20 {
            rom: self.rom.clone(),
            temp: self.temp,
            dq_low: false,
        }
    }

    fn get_rom(&self) -> PropValue {
        PropValue::String(self.rom.clone())
    }
    fn set_rom(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rom = expect_string("Rom", v)?;
        Ok(())
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

impl Component for DS18B20 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Programmable Resolution 1-Wire Digital Thermometer."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<DS18B20>] = &[
            PropDef::string("Rom", "ROM", DS18B20::get_rom, DS18B20::set_rom)
                .with_info("ROM address."),
            PropDef::float(
                "Temp",
                "Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                DS18B20::get_temp,
                DS18B20::set_temp,
            )
            .with_info("Current temperature."),
            PropDef::float(
                "TempInc",
                "Temp. increment",
                "°C",
                MIN_INC,
                MAX_INC,
                DS18B20::get_temp_inc,
                DS18B20::set_temp_inc,
            )
            .with_info("Temperature will increment by this value when + or - buttons are pushed."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ds18b20_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-20.0, -20.0, 40.0, 40.0)
    }
}

impl Stampable for DS18B20 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for DS18B20 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Module body (40x40)
        d.fill_round_rect(
            -20.0,
            -20.0,
            40.0,
            40.0,
            3.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -20.0,
            -20.0,
            40.0,
            40.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Chip title
        d.text(0.0, -12.0, "DS18B20", 7.0, ctx.pal.border, Align::Center);

        // Temperature readout
        let t_str = format!("{:.1} °C", self.temp);
        d.text(0.0, 2.0, &t_str, 8.0, ctx.pal.border, Align::Center);

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ds18b20(&mut self, x: f64, y: f64) -> String {
        let id = format!("DS18B20-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::ds18b20(
            &id,
            x,
            y,
            "2800000000000001",
            25.0,
            1.0,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ds18b20() {
        let d = DS18B20::default();
        assert_eq!(d.type_id(), "DS18B20");
        assert_eq!(d.rom, "28FF2B450000");
        assert_eq!(d.temp, 25.0);
        assert_eq!(d.temp_inc, 0.5);
        assert_eq!(d.pin_geoms().len(), 3);
    }
}

const DS18B20_PINS: [PinGeom; 3] = [
    PinGeom {
        suffix: "-gndPin",
        x: -8.0,
        y: 24.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-inPin",
        x: 0.0,
        y: 24.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vddPin",
        x: 8.0,
        y: 24.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
];

fn ds18b20_pins() -> &'static [PinGeom] {
    &DS18B20_PINS
}
