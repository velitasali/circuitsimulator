//! DHT22 / DHT11 temperature and humidity sensor.

use super::component::PropGroup;
use super::props::{PropDef, PropError, PropValue, expect_float, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_TEMP: f64 = -40.0;
const MAX_TEMP: f64 = 125.0;
const MIN_HUMI: f64 = 0.0;
const MAX_HUMI: f64 = 100.0;
const MIN_INC: f64 = 0.01;
const MAX_INC: f64 = 50.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum DhtModel {
    #[default]
    Dht22,
    Dht11,
}

impl DhtModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dht22 => "DHT22",
            Self::Dht11 => "DHT11",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "dht11" => Self::Dht11,
            _ => Self::Dht22,
        }
    }
}

impl std::fmt::Display for DhtModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const DHT_MODEL_OPTIONS: &[&str] = &["DHT11", "DHT22"];

impl crate::canvas::Item {
    pub fn dht22(
        id: impl Into<String>,
        x: f64,
        y: f64,
        model: impl AsRef<str>,
        temp: f64,
        humi: f64,
        temp_inc: f64,
        humi_inc: f64,
    ) -> Self {
        Self::dht22_with(id, x, y, model, temp, humi, temp_inc, humi_inc)
    }

    pub fn dht22_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        model: impl AsRef<str>,
        temp: f64,
        humi: f64,
        temp_inc: f64,
        humi_inc: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            DHT22 {
                model: DhtModel::from_str_name(model.as_ref()),
                temp,
                humi,
                temp_inc,
                humi_inc,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DHT22 {
    pub model: DhtModel,
    pub temp: f64,
    pub humi: f64,
    pub temp_inc: f64,
    pub humi_inc: f64,
}

impl Default for DHT22 {
    fn default() -> Self {
        Self {
            model: DhtModel::Dht22,
            temp: 22.5,
            humi: 68.5,
            temp_inc: 0.5,
            humi_inc: 5.0,
        }
    }
}

impl DHT22 {
    pub const TYPE_ID: &'static str = "DHT22";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Dht22 {
            model: self.model.as_str().to_string(),
            temp: self.temp,
            humi: self.humi,
            out_state: true,
            pin_driven: false,
        }
    }

    fn get_model(&self) -> PropValue {
        PropValue::Enum(self.model.as_str().to_string())
    }
    fn set_model(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Model", v)?;
        self.model = DhtModel::from_str_name(&s);
        Ok(())
    }

    fn get_temp(&self) -> PropValue {
        PropValue::Float(self.temp)
    }
    fn set_temp(&mut self, v: PropValue) -> Result<(), PropError> {
        self.temp = expect_float("Temp", v)?.clamp(MIN_TEMP, MAX_TEMP);
        Ok(())
    }

    fn get_humi(&self) -> PropValue {
        PropValue::Float(self.humi)
    }
    fn set_humi(&mut self, v: PropValue) -> Result<(), PropError> {
        self.humi = expect_float("Humi", v)?.clamp(MIN_HUMI, MAX_HUMI);
        Ok(())
    }

    fn get_temp_inc(&self) -> PropValue {
        PropValue::Float(self.temp_inc)
    }
    fn set_temp_inc(&mut self, v: PropValue) -> Result<(), PropError> {
        self.temp_inc = expect_float("TempInc", v)?.clamp(MIN_INC, MAX_INC);
        Ok(())
    }

    fn get_humi_inc(&self) -> PropValue {
        PropValue::Float(self.humi_inc)
    }
    fn set_humi_inc(&mut self, v: PropValue) -> Result<(), PropError> {
        self.humi_inc = expect_float("HumiInc", v)?.clamp(MIN_INC, MAX_INC);
        Ok(())
    }
}

impl Component for DHT22 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Temperature and humidity sensor."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<DHT22>] = &[
            PropDef::enumeration(
                "Model",
                "Model",
                DHT_MODEL_OPTIONS,
                DHT22::get_model,
                DHT22::set_model,
            )
            .with_info("Choose model: DHT11 or DHT22."),
            PropDef::float(
                "Temp",
                "Temperature",
                "°C",
                MIN_TEMP,
                MAX_TEMP,
                DHT22::get_temp,
                DHT22::set_temp,
            )
            .with_info("Current temperature."),
            PropDef::float(
                "Humi",
                "Humidity",
                "%",
                MIN_HUMI,
                MAX_HUMI,
                DHT22::get_humi,
                DHT22::set_humi,
            )
            .with_info("Current humidity."),
            PropDef::float(
                "TempInc",
                "Temp. increment",
                "°C",
                MIN_INC,
                MAX_INC,
                DHT22::get_temp_inc,
                DHT22::set_temp_inc,
            )
            .with_info("Temperature will increment by this value when + or - buttons are pushed."),
            PropDef::float(
                "HumiInc",
                "Humid. increment",
                "%",
                MIN_INC,
                MAX_INC,
                DHT22::get_humi_inc,
                DHT22::set_humi_inc,
            )
            .with_info("Humidity will increment by this value when + or - buttons are pushed."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        vec![PropGroup::new("Main", self.prop_rows())]
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        dht22_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-20.0, -40.0, 40.0, 68.0)
    }
}

impl crate::canvas::Scene {
    pub fn add_dht22(&mut self, x: f64, y: f64) -> String {
        let id = format!("DHT22-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::dht22(
            &id, x, y, "DHT22", 22.5, 68.5, 0.5, 5.0,
        ));
        id
    }

    pub fn step_sensor_temp(&mut self, uid: &str, up: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            match &mut item.kind {
                crate::components::Part::DHT22(p) => {
                    if up {
                        p.temp += p.temp_inc;
                    } else {
                        p.temp -= p.temp_inc;
                    }
                    return true;
                }
                crate::components::Part::DS18B20(p) => {
                    if up {
                        p.temp += p.temp_inc;
                    } else {
                        p.temp -= p.temp_inc;
                    }
                    return true;
                }
                crate::components::Part::DS1621(p) => {
                    if up {
                        p.temp += p.temp_inc;
                    } else {
                        p.temp -= p.temp_inc;
                    }
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub fn step_sensor_humi(&mut self, uid: &str, up: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::DHT22(p) = &mut item.kind {
                if up {
                    p.humi = (p.humi + p.humi_inc).min(100.0);
                } else {
                    p.humi = (p.humi - p.humi_inc).max(0.0);
                }
                return true;
            }
        }
        false
    }
}

impl Stampable for DHT22 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for DHT22 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Enclosure (40x68)
        d.fill_round_rect(
            -20.0,
            -40.0,
            40.0,
            68.0,
            3.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -20.0,
            -40.0,
            40.0,
            68.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Model name
        d.text(
            0.0,
            -36.0,
            self.model.as_str(),
            7.0,
            ctx.pal.border,
            Align::Center,
        );

        // Horizontal grill slots
        let slot_color = ctx.pal.border.fade(0.35);
        for i in 0..4 {
            let y = -26.0 + i as f64 * 4.0;
            d.fill_round_rect(-14.0, y, 28.0, 2.0, 1.0, slot_color);
        }

        // Temperature & Humidity readouts
        let t_str = format!("{:.1} °C", self.temp);
        let h_str = format!("{:.1} %", self.humi);
        d.text(0.0, -7.0, &t_str, 7.0, ctx.pal.border, Align::Center);
        d.text(0.0, 5.0, &h_str, 7.0, ctx.pal.border, Align::Center);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dht22() {
        let d = DHT22::default();
        assert_eq!(d.type_id(), "DHT22");
        assert_eq!(d.model, DhtModel::Dht22);
        assert_eq!(d.temp, 22.5);
        assert_eq!(d.humi, 68.5);
        assert_eq!(d.temp_inc, 0.5);
        assert_eq!(d.humi_inc, 5.0);
        assert_eq!(d.pin_geoms().len(), 4);
    }
}

const DHT22_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-vccPin",
        x: -12.0,
        y: 32.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-inPin",
        x: -4.0,
        y: 32.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-ncPin",
        x: 4.0,
        y: 32.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
    PinGeom {
        suffix: "-gdnPin",
        x: 12.0,
        y: 32.0,
        angle: 270,
        length: 4.0,
        direction: None,
    },
];

fn dht22_pins() -> &'static [PinGeom] {
    &DHT22_PINS
}
