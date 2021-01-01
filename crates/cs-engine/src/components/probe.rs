//! Probe: logic level and voltage probe with normal or compact display.

use super::component::stamp_to_ground;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::instruments::PROBE_ADMIT;
use crate::matrix::CircMatrix;

const MIN_VOLT: f64 = -1000.0;
const MAX_VOLT: f64 = 1000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    pub threshold: f64,
    pub small: bool,
    pub show_volt: bool,
    pub pause_at_change: bool,
}

impl crate::canvas::Item {
    pub fn probe(id: impl Into<String>, x: f64, y: f64, threshold: f64, small: bool) -> Self {
        Self::probe_with(id, x, y, threshold, small, false)
    }

    pub fn probe_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        threshold: f64,
        small: bool,
        show_volt: bool,
    ) -> Self {
        let mut it = Self::new(
            id,
            x,
            y,
            Probe {
                threshold,
                small,
                show_volt,
                pause_at_change: false,
            },
        );
        it.rotation = -45.0;
        it
    }
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            threshold: 2.5,
            small: false,
            show_volt: false,
            pause_at_change: false,
        }
    }
}

impl Probe {
    pub const TYPE_ID: &'static str = "Probe";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Probe {
            threshold: self.threshold,
            small: self.small,
        }
    }

    fn get_threshold(&self) -> PropValue {
        PropValue::Float(self.threshold)
    }
    fn set_threshold(&mut self, v: PropValue) -> Result<(), PropError> {
        self.threshold = expect_float("Threshold", v)?.clamp(MIN_VOLT, MAX_VOLT);
        Ok(())
    }
    fn get_small(&self) -> PropValue {
        PropValue::Bool(self.small)
    }
    fn set_small(&mut self, v: PropValue) -> Result<(), PropError> {
        self.small = expect_bool("Small", v)?;
        Ok(())
    }
    fn get_show_volt(&self) -> PropValue {
        PropValue::Bool(self.show_volt)
    }
    fn set_show_volt(&mut self, v: PropValue) -> Result<(), PropError> {
        self.show_volt = expect_bool("ShowVolt", v)?;
        Ok(())
    }
    fn get_pause_at_change(&self) -> PropValue {
        PropValue::Bool(self.pause_at_change)
    }
    fn set_pause_at_change(&mut self, v: PropValue) -> Result<(), PropError> {
        self.pause_at_change = expect_bool("PauseAtChange", v)?;
        Ok(())
    }
}

impl Component for Probe {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Voltage logic probe."
    }

    fn props() -> &'static [PropDef<Self>] {
        const SMALL: PropDef<Probe> = {
            let mut p = PropDef::bool("Small", "Small size", Probe::get_small, Probe::set_small)
                .with_info("Use a smaller body.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Probe>] = &[
            PropDef::float(
                "Threshold",
                "Threshold",
                "V",
                MIN_VOLT,
                MAX_VOLT,
                Probe::get_threshold,
                Probe::set_threshold,
            )
            .with_info("Voltage above which the probe reads as a logic High."),
            SMALL,
            PropDef::bool(
                "ShowVolt",
                "Show Voltage",
                Probe::get_show_volt,
                Probe::set_show_volt,
            )
            .with_info("Show the measured voltage next to the probe."),
            PropDef::bool(
                "PauseAtChange",
                "Pause At Change",
                Probe::get_pause_at_change,
                Probe::set_pause_at_change,
            )
            .with_info("Automatically pause simulation when probed logic state changes."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        if self.small {
            vec![CompPin::new("-inpin", -24.0, 0.0, 180, 8.0).with_direction(PinDirection::In)]
        } else {
            vec![CompPin::new("-inpin", -24.0, 0.0, 180, 16.0).with_direction(PinDirection::In)]
        }
    }

    fn body(&self) -> Rect {
        if self.small {
            Rect::new(-16.0, -4.0, 8.0, 8.0)
        } else {
            Rect::new(-8.0, -8.0, 16.0, 16.0)
        }
    }
}

impl Stampable for Probe {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_to_ground(matrix, pin_nodes, 0, 0.0, PROBE_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::PinDirection;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl Drawable for Probe {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let reading = ctx.canvas.readings().get(ctx.item_id);
        let fill = if reading.is_some_and(|r| r.high) {
            ctx.pal.pin_high
        } else if reading.is_some_and(|r| r.low) {
            ctx.pal.pin_low
        } else {
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA)
        };

        if self.small {
            d.fill_circle(-12.0, 0.0, 4.0, fill);
            d.stroke_circle(-12.0, 0.0, 4.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        } else {
            d.fill_circle(0.0, 0.0, 8.0, fill);
            d.stroke_circle(0.0, 0.0, 8.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        }

        if let Some(r) = reading.filter(|r| !r.text.is_empty()) {
            let tx = if self.small { -4.0 } else { 10.0 };
            let font_size = if self.small { 8.0 } else { 10.0 };
            d.text(tx, -6.0, &r.text, font_size, ctx.pal.border, Align::TopLeft);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_probe(&mut self, x: f64, y: f64) -> String {
        let id = format!("Probe-{}", self.next_probe);
        self.next_probe += 1;
        self.items.push(crate::canvas::Item::probe(
            &id,
            x,
            y,
            crate::PROBE_DEFAULT_THRESHOLD,
            false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_probe() {
        let p = Probe::default();
        assert_eq!(p.type_id(), "Probe");
        assert_eq!(p.threshold, 2.5);
        assert!(!p.small);
        assert_eq!(p.pin_geoms().len(), 1);
        assert_eq!(p.pin_geoms()[0].length, 16.0);
        assert_eq!(p.body(), Rect::new(-8.0, -8.0, 16.0, 16.0));
    }

    #[test]
    fn small_probe_changes_body_and_pin() {
        let mut p = Probe::default();
        let change = p.set_prop("Small", PropValue::Bool(true)).unwrap();
        assert!(change.structural);
        assert_eq!(p.pin_geoms()[0].length, 8.0);
        assert_eq!(p.body().w, 8.0);
        assert_eq!(p.body().h, 8.0);
    }
}
