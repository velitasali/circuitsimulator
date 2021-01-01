//! Incandescent lamp component.

use super::component::{resistor_g, stamp_two_terminal};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.01;
const MAX_V: f64 = 1e4;
const MIN_P: f64 = 0.01;
const MAX_P: f64 = 1e4;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e6;

impl crate::canvas::Item {
    pub fn lamp(
        id: impl Into<String>,
        x: f64,
        y: f64,
        voltage: f64,
        power: f64,
        r_cold: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Lamp {
                voltage,
                power,
                r_cold,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lamp {
    pub voltage: f64,
    pub power: f64,
    pub r_cold: f64,
}

impl Default for Lamp {
    fn default() -> Self {
        Self {
            voltage: 12.0,
            power: 24.0,
            r_cold: 1.0,
        }
    }
}

impl Lamp {
    pub const TYPE_ID: &'static str = "Lamp";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Lamp {
            voltage: self.voltage,
            power: self.power,
            r_cold: self.r_cold,
            resistance: self.r_cold,
        }
    }

    fn get_voltage(&self) -> PropValue {
        PropValue::Float(self.voltage)
    }
    fn set_voltage(&mut self, v: PropValue) -> Result<(), PropError> {
        self.voltage = expect_float("Voltage", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_power(&self) -> PropValue {
        PropValue::Float(self.power)
    }
    fn set_power(&mut self, v: PropValue) -> Result<(), PropError> {
        self.power = expect_float("Power", v)?.clamp(MIN_P, MAX_P);
        Ok(())
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.r_cold)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_cold = expect_float("Resistance", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }
}

impl Component for Lamp {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Incandescent lamp indicator."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Lamp>] = &[
            PropDef::float(
                "Voltage",
                "Nominal Voltage",
                "V",
                MIN_V,
                MAX_V,
                Lamp::get_voltage,
                Lamp::set_voltage,
            )
            .with_info("Output voltage."),
            PropDef::float(
                "Power",
                "Nominal Power",
                "W",
                MIN_P,
                MAX_P,
                Lamp::get_power,
                Lamp::set_power,
            )
            .with_info("Nominal electric power rating in Watts."),
            PropDef::float(
                "Resistance",
                "Cold Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Lamp::get_resistance,
                Lamp::set_resistance,
            )
            .with_info("Resistance of the filament."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        lamp_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-8.0, -8.0, 16.0, 16.0)
    }

    fn visual_rect(&self) -> Rect {
        self.body()
            .united(super::drawable::indicator_bulb_visual_rect(0.0, 0.0, 8.0))
    }
}

impl TwoTerminal for Lamp {}

impl Stampable for Lamp {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.r_cold), 0.0);
    }
}

use super::drawable::{BulbState, Drawable, paint_indicator_bulb};
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::pins::{PIN_LEFT, PIN_RIGHT};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Lamp {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let (state, bulb_c) = if ctx.canvas.sim_running() {
            let overload = ctx.canvas.item_overload_state(ctx.item_id);
            let is_warning = overload.is_some_and(|st| st.warning);
            let is_crashed = overload.is_some_and(|st| st.crashed);

            let cur = ctx.pin_current(PIN_LEFT).map(|i| i.abs()).unwrap_or(0.0);
            let vl = ctx.pin_voltage(PIN_LEFT).unwrap_or(0.0);
            let vr = ctx.pin_voltage(PIN_RIGHT).unwrap_or(0.0);
            let v_diff = (vl - vr).abs();

            let max_cur = if self.voltage > 1e-6 {
                self.power / self.voltage
            } else {
                1.0
            };
            let nom_cur = max_cur.max(0.001);

            let ratio = if cur > 0.0 {
                cur / nom_cur
            } else if v_diff > 0.05 {
                (v_diff / self.r_cold.max(1e-3)) / nom_cur
            } else {
                0.0
            };

            let lit = is_warning || ratio > 0.01 || cur > 0.001 || v_diff > 0.5;
            let intensity = if is_warning {
                1.5
            } else if ratio > 0.0 {
                ratio.sqrt().clamp(0.1, 1.5)
            } else if lit {
                (v_diff / self.voltage.max(1.0)).sqrt().clamp(0.1, 1.0)
            } else {
                0.0
            };

            let g_scale = intensity.clamp(0.2, 1.0);
            let bulb_c = Color::rgb(
                255,
                (230.0 * g_scale).round() as u8,
                (70.0 * g_scale).round() as u8,
            );

            (
                BulbState {
                    is_lit: lit && !is_crashed,
                    intensity,
                    is_warning,
                    is_crashed,
                },
                bulb_c,
            )
        } else {
            (BulbState::unlit(), Color::rgb(255, 230, 80))
        };

        paint_indicator_bulb(d, ctx, 0.0, 0.0, 8.0, bulb_c, &state);

        // Filament cross
        d.line(-5.0, -5.0, 5.0, 5.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.line(-5.0, 5.0, 5.0, -5.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_lamp(&mut self, x: f64, y: f64) -> String {
        let id = format!("Lamp-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::lamp(&id, x, y, 12.0, 24.0, 1.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lamp() {
        let l = Lamp::default();
        assert_eq!(l.type_id(), "Lamp");
        assert_eq!(l.voltage, 12.0);
        assert_eq!(l.power, 24.0);
        assert_eq!(l.r_cold, 1.0);
        assert_eq!(l.pin_geoms().len(), 2);
        assert_eq!(l.body(), Rect::new(-8.0, -8.0, 16.0, 16.0));
    }
}

const LAMP_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-lPin",
        x: -16.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-rPin",
        x: 16.0,
        y: 0.0,
        angle: 0,
        length: 8.0,
        direction: None,
    },
];

fn lamp_pins() -> &'static [PinGeom] {
    &LAMP_PINS
}
