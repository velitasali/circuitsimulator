//! Clock: square-wave voltage source. Solver `state` is transient, not saved.

use super::component::stamp_to_ground;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::{FIXED_VOLT_DEFAULT, Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;

const MIN_V: f64 = 0.0;
const MAX_V: f64 = 1e6;
const MIN_HZ: f64 = 1e-6;
const MAX_HZ: f64 = 1e12;
const DEFAULT_HZ: f64 = 1_000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Clock {
    pub voltage: f64,
    pub frequency: f64,
    pub small: bool,
}

impl crate::canvas::Item {
    pub fn clock(id: impl Into<String>, x: f64, y: f64, voltage: f64, freq_khz: f64) -> Self {
        Self::clock_with(id, x, y, voltage, freq_khz, false)
    }

    pub fn clock_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        voltage: f64,
        freq_khz: f64,
        small: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Clock {
                voltage,
                frequency: freq_khz * 1e3,
                small,
            },
        )
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            voltage: FIXED_VOLT_DEFAULT,
            frequency: DEFAULT_HZ,
            small: false,
        }
    }
}

impl Clock {
    pub const TYPE_ID: &'static str = "Clock";
    pub fn new(voltage: f64, frequency: f64) -> Self {
        Self {
            voltage,
            frequency: frequency.max(MIN_HZ),
            small: false,
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Clock {
            voltage: self.voltage,
            freq_khz: self.frequency / 1e3,
            state: false,
        }
    }

    fn get_voltage(&self) -> PropValue {
        PropValue::Float(self.voltage)
    }
    fn set_voltage(&mut self, v: PropValue) -> Result<(), PropError> {
        self.voltage = expect_float("Voltage", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
    fn get_frequency(&self) -> PropValue {
        PropValue::Float(self.frequency)
    }
    fn set_frequency(&mut self, v: PropValue) -> Result<(), PropError> {
        self.frequency = expect_float("Frequency", v)?.clamp(MIN_HZ, MAX_HZ);
        Ok(())
    }
    fn get_small(&self) -> PropValue {
        PropValue::Bool(self.small)
    }
    fn set_small(&mut self, v: PropValue) -> Result<(), PropError> {
        self.small = expect_bool("Small", v)?;
        Ok(())
    }
}

impl Component for Clock {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Clock signal generator."
    }
    fn props() -> &'static [PropDef<Self>] {
        const FREQ: PropDef<Clock> = {
            let mut p = PropDef::float(
                "Frequency",
                "Frequency",
                "Hz",
                MIN_HZ,
                MAX_HZ,
                Clock::get_frequency,
                Clock::set_frequency,
            )
            .with_info("Set output frequency.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Clock>] = &[
            PropDef::float(
                "Voltage",
                "Voltage",
                "V",
                MIN_V,
                MAX_V,
                Clock::get_voltage,
                Clock::set_voltage,
            )
            .with_info("Set output voltage."),
            FREQ,
            PropDef::bool("Small", "Small size", Clock::get_small, Clock::set_small)
                .with_info("Use a smaller body."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        let plen = if self.small { 12.0 } else { 8.0 };
        vec![CompPin::new("-outnod", 16.0, 0.0, 0, plen).with_direction(PinDirection::Out)]
    }
    fn body(&self) -> Rect {
        if self.small {
            Rect::new(-4.0, -4.0, 8.0, 8.0)
        } else {
            Rect::new(-8.0, -8.0, 16.0, 16.0)
        }
    }
}

impl Stampable for Clock {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_to_ground(matrix, pin_nodes, 0, self.voltage, SOURCE_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Clock {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let r = if self.small { 4.0 } else { 8.0 };
        d.fill_circle(0.0, 0.0, r, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_circle(0.0, 0.0, r, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        let s = r / 8.0;
        let pts = [
            [-5.0 * s, 3.0 * s],
            [-2.0 * s, 3.0 * s],
            [-2.0 * s, -3.0 * s],
            [2.0 * s, -3.0 * s],
            [2.0 * s, 3.0 * s],
            [5.0 * s, 3.0 * s],
        ];
        d.polyline(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, false);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_clock(&mut self, x: f64, y: f64) -> String {
        let id = format!("Clock-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::clock(&id, x, y, 5.0, 1.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let c = Clock::default();
        assert_eq!(c.voltage, FIXED_VOLT_DEFAULT);
        assert_eq!(c.frequency, DEFAULT_HZ);
        assert!(!c.small);
        assert_eq!(
            c.get_prop_text("Frequency").unwrap(),
            format_si(DEFAULT_HZ, "Hz")
        );
    }

    #[test]
    fn element_kind_uses_khz() {
        let c = Clock::new(5.0, 1_000.0);
        match c.to_element_kind() {
            Kind::Clock {
                freq_khz, state, ..
            } => {
                assert!((freq_khz - 1.0).abs() < 1e-12);
                assert!(!state);
            }
            other => panic!("{other:?}"),
        }
    }
}
