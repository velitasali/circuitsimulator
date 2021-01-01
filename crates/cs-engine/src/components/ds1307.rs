//! DS1307 real-time clock (RTC) module with I2C interface.

use super::props::{PropDef, PropError, PropValue, expect_bool};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

impl crate::canvas::Item {
    pub fn ds1307(id: impl Into<String>, x: f64, y: f64, time_updated: bool) -> Self {
        Self::new(id, x, y, DS1307 { time_updated })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DS1307 {
    pub time_updated: bool,
}

impl Default for DS1307 {
    fn default() -> Self {
        Self { time_updated: true }
    }
}

impl DS1307 {
    pub const TYPE_ID: &'static str = "DS1307";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ds1307 {
            time_updated: self.time_updated,
            sqw_freq: 1.0,
            sqw_enabled: true,
            sqw_state: false,
            sda_low: false,
            scl_low: false,
        }
    }

    fn get_time_updated(&self) -> PropValue {
        PropValue::Bool(self.time_updated)
    }
    fn set_time_updated(&mut self, v: PropValue) -> Result<(), PropError> {
        self.time_updated = expect_bool("TimeUpdated", v)?;
        Ok(())
    }
}

impl Component for DS1307 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Serial I2C real time clock."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<DS1307>] = &[PropDef::bool(
            "TimeUpdated",
            "Set current time at start",
            DS1307::get_time_updated,
            DS1307::set_time_updated,
        )
        .with_info("Initialize RTC simulation with current host system time.")];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ds1307_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-28.0, -20.0, 56.0, 40.0)
    }
}

impl Stampable for DS1307 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for DS1307 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Module Body (56x40)
        d.fill_round_rect(
            -28.0,
            -20.0,
            56.0,
            40.0,
            3.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -28.0,
            -20.0,
            56.0,
            40.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Header Title
        d.text(0.0, -13.0, "DS1307 RTC", 7.0, ctx.pal.border, Align::Center);

        // Crystal Oscillator graphic (-22, -2, 14, 16)
        d.fill_round_rect(-22.0, -2.0, 14.0, 16.0, 7.0, ctx.pal.body.fade(0.8));
        d.stroke_round_rect(-22.0, -2.0, 14.0, 16.0, 7.0, ctx.pal.border, 1.0);
        d.text(-15.0, 6.0, "32k", 5.0, ctx.pal.border, Align::Center);

        // Clock IC graphic (-2, -2, 24, 16)
        d.fill_round_rect(-2.0, -2.0, 24.0, 16.0, 2.0, ctx.pal.body.fade(0.8));
        d.stroke_round_rect(-2.0, -2.0, 24.0, 16.0, 2.0, ctx.pal.border, 1.0);
        d.text(10.0, 6.0, "I2C", 6.0, ctx.pal.border, Align::Center);

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ds1307(&mut self, x: f64, y: f64) -> String {
        let id = format!("DS1307-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::ds1307(&id, x, y, true));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ds1307() {
        let d = DS1307::default();
        assert_eq!(d.type_id(), "DS1307");
        assert!(d.time_updated);
        assert_eq!(d.pin_geoms().len(), 3);
    }
}

const DS1307_PINS: [PinGeom; 3] = [
    PinGeom {
        suffix: "-PinSDA",
        x: -36.0,
        y: -12.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinSCL",
        x: -36.0,
        y: -4.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinSQW",
        x: -36.0,
        y: 12.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn ds1307_pins() -> &'static [PinGeom] {
    &DS1307_PINS
}
