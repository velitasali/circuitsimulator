//! LED: light emitting diode with color, forward drop, and optional internal ground.

use super::component::{stamp_to_ground, stamp_two_terminal};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_string};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::elements::diode::LedState;
use crate::elements::pins::{PIN_LEFT, PIN_RIGHT};
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.0;
const MAX_V: f64 = 100.0;
const MIN_A: f64 = 1e-6;
const MAX_A: f64 = 1e6;
const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;

impl crate::canvas::Item {
    pub fn led(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Led {
                color: LedColor::Yellow,
                grounded: false,
                threshold: 2.4,
                max_current: 0.03,
                resistance: 0.6,
            },
        )
    }
}

pub const LED_COLOR_OPTIONS: &[&str] = &[
    "Yellow", "Red", "Green", "Blue", "Orange", "Purple", "White",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LedColor {
    #[default]
    Yellow,
    Red,
    Green,
    Blue,
    Orange,
    Purple,
    White,
}

impl LedColor {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Yellow => "Yellow",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Orange => "Orange",
            Self::Purple => "Purple",
            Self::White => "White",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "red" => Self::Red,
            "green" => Self::Green,
            "blue" => Self::Blue,
            "orange" => Self::Orange,
            "purple" => Self::Purple,
            "white" => Self::White,
            _ => Self::Yellow,
        }
    }

    pub const fn threshold(&self) -> f64 {
        match self {
            Self::Red => 1.8,
            Self::Green => 3.5,
            Self::Blue => 3.6,
            Self::Orange => 2.0,
            Self::Purple => 3.5,
            Self::White => 4.0,
            Self::Yellow => 2.4,
        }
    }
}

impl std::fmt::Display for LedColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Led {
    pub color: LedColor,
    pub grounded: bool,
    pub threshold: f64,
    pub max_current: f64,
    pub resistance: f64,
}

impl Default for Led {
    fn default() -> Self {
        Self {
            color: LedColor::Yellow,
            grounded: false,
            threshold: 2.4,
            max_current: 0.03,
            resistance: 0.6,
        }
    }
}

impl Led {
    pub const TYPE_ID: &'static str = "Led";
    pub fn to_element_kind(&self) -> Kind {
        let mut state = LedState::default_led();
        state.threshold = self.threshold;
        state.impedance = self.resistance;
        state.max_current = self.max_current;
        state.grounded = self.grounded;
        Kind::Led { state }
    }

    fn get_color(&self) -> PropValue {
        PropValue::Enum(self.color.as_str().to_string())
    }
    fn set_color(&mut self, v: PropValue) -> Result<(), PropError> {
        let color_name = expect_string("Color", v)?;
        self.color = LedColor::from_str_name(&color_name);
        self.threshold = self.color.threshold();
        Ok(())
    }
    fn get_grounded(&self) -> PropValue {
        PropValue::Bool(self.grounded)
    }
    fn set_grounded(&mut self, v: PropValue) -> Result<(), PropError> {
        self.grounded = expect_bool("Grounded", v)?;
        Ok(())
    }
    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold)
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        self.threshold = expect_float("Threshold", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
    fn get_max_current(&self) -> PropValue {
        PropValue::Float(self.max_current)
    }
    fn set_max_current(&mut self, v: PropValue) -> Result<(), PropError> {
        self.max_current = expect_float("MaxCurrent", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }
    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
}

impl Component for Led {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Light-emitting diode."
    }

    fn props() -> &'static [PropDef<Self>] {
        const GROUNDED: PropDef<Led> = {
            let mut p = PropDef::bool("Grounded", "Grounded", Led::get_grounded, Led::set_grounded).with_info("If yes, connect the cathode to ground internally and hide the cathode pin.\nAny wire already connected to the cathode will be deleted.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Led>] = &[
            PropDef::enumeration("Color", "Color", LED_COLOR_OPTIONS, Led::get_color, Led::set_color).with_info("Led color."),
            GROUNDED,
            PropDef::float(
                "Threshold",
                "Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                Led::get_threshold,
                Led::set_threshold,
            ).with_info("Voltage drop when forward biased."),
            PropDef::float(
                "MaxCurrent",
                "Max Current",
                "A",
                MIN_A,
                MAX_A,
                Led::get_max_current,
                Led::set_max_current,
            ).with_info("Maximum current (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Led::get_resistance,
                Led::set_resistance,
            ).with_info("Series resistance."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Color", "Grounded"]),
                ("Electric", &["Threshold", "MaxCurrent", "Resistance"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        if self.grounded {
            vec![CompPin::new("-lPin", -16.0, 0.0, 180, 10.0)]
        } else {
            vec![
                CompPin::new("-lPin", -16.0, 0.0, 180, 10.0),
                CompPin::new("-rPin", 16.0, 0.0, 0, 6.0),
            ]
        }
    }

    fn body(&self) -> Rect {
        Rect::new(-6.0, -8.0, 16.0, 16.0)
    }

    fn visual_rect(&self) -> Rect {
        self.body()
            .united(super::drawable::indicator_bulb_visual_rect(2.0, 0.0, 8.0))
    }
}

impl TwoTerminal for Led {}

impl Stampable for Led {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if self.grounded {
            stamp_to_ground(matrix, pin_nodes, 0, 0.0, 1e-9);
        } else {
            stamp_two_terminal(matrix, pin_nodes, 1e-9, 0.0);
        }
    }
}

use super::drawable::{BulbState, Drawable, paint_indicator_bulb};
use crate::canvas::draw::{Color, Draw, PaintCtx};

pub fn led_fore_color(color_name: &str) -> Color {
    match color_name.to_ascii_lowercase().as_str() {
        "green" => Color::rgb(34, 255, 68),
        "blue" | "blue_super" => Color::rgb(34, 136, 255),
        "yellow" => Color::rgb(255, 238, 34),
        "orange" => Color::rgb(255, 136, 0),
        "white" => Color::rgb(255, 255, 255),
        "purple" | "uv" => Color::rgb(170, 68, 255),
        "infrared" => Color::rgb(255, 68, 102),
        _ => Color::rgb(255, 34, 34), // default red
    }
}

impl Drawable for Led {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let state = if ctx.canvas.sim_running() {
            let overload = ctx.canvas.item_overload_state(ctx.item_id);
            let is_warning = overload.is_some_and(|st| st.warning);
            let is_crashed = overload.is_some_and(|st| st.crashed);

            let current = ctx.pin_current(PIN_LEFT).unwrap_or(0.0).max(0.0);
            let max_i = self.max_current.max(0.001);
            let ratio = (current / max_i).max(0.0);
            let v_anode = ctx.pin_voltage(PIN_LEFT);
            let v_cathode = if self.grounded {
                Some(0.0)
            } else {
                ctx.pin_voltage(PIN_RIGHT)
            };
            let v_diff = match (v_anode, v_cathode) {
                (Some(va), Some(vc)) => (va - vc).max(0.0),
                _ => 0.0,
            };

            let effective_ratio = if ratio > 0.001 {
                ratio
            } else if v_diff > self.threshold * 0.95 {
                ((v_diff - self.threshold).max(0.0) / (self.resistance.max(0.1) * max_i))
                    .clamp(0.05, 1.5)
            } else {
                0.0
            };

            let is_lit = is_warning || effective_ratio > 0.001 || current > 1e-5;

            let intensity = if is_warning {
                1.5
            } else if effective_ratio > 0.0 {
                effective_ratio.sqrt().clamp(0.15, 1.5)
            } else {
                0.0
            };

            BulbState {
                is_lit: is_lit && !is_crashed,
                intensity,
                is_warning,
                is_crashed,
            }
        } else {
            BulbState::unlit()
        };

        let fore = led_fore_color(self.color.as_str());
        paint_indicator_bulb(d, ctx, 2.0, 0.0, 8.0, fore, &state);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_led(&mut self, x: f64, y: f64) -> String {
        let id = format!("Led-{}", self.next_led);
        self.next_led += 1;
        self.items.push(crate::canvas::Item::led(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_led_has_two_pins() {
        let led = Led::default();
        assert_eq!(led.color, LedColor::Yellow);
        assert!(!led.grounded);
        assert_eq!(led.pin_geoms().len(), 2);
    }

    #[test]
    fn grounded_led_has_one_pin() {
        let mut led = Led::default();
        let change = led.set_prop("Grounded", PropValue::Bool(true)).unwrap();
        assert!(change.structural);
        assert_eq!(led.pin_geoms().len(), 1);
        assert_eq!(led.pin_geoms()[0].suffix, "-lPin");
    }
}
