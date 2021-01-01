//! Latch: digital transparent latch / register.

use super::component::PropGroup;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::digital::{LatchState, Trigger};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_CHANNELS: usize = 1;
const MAX_CHANNELS: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct Latch {
    pub channels: usize,
    pub use_reset: bool,
    pub tristate: bool,
    pub trigger: Trigger,
    pub invert_inputs: bool,
}

impl crate::canvas::Item {
    pub fn latch(
        id: impl Into<String>,
        x: f64,
        y: f64,
        channels: usize,
        use_reset: bool,
        tristate: bool,
        trigger: &str,
        invert_inputs: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Latch {
                channels,
                use_reset,
                tristate,
                trigger: Trigger::from_str_name(trigger),
                invert_inputs,
            },
        )
    }
}

impl Default for Latch {
    fn default() -> Self {
        Self {
            channels: 4,
            use_reset: false,
            tristate: false,
            trigger: Trigger::Clock,
            invert_inputs: false,
        }
    }
}

impl Latch {
    pub const TYPE_ID: &'static str = "Latch";
    pub fn to_element_kind(&self) -> Kind {
        let l = LatchState::new("", self.channels);
        Kind::Latch(l)
    }

    fn get_channels(&self) -> PropValue {
        PropValue::Int(self.channels as i64)
    }
    fn set_channels(&mut self, v: PropValue) -> Result<(), PropError> {
        self.channels =
            expect_int("Channels", v)?.clamp(MIN_CHANNELS as i64, MAX_CHANNELS as i64) as usize;
        Ok(())
    }
    fn get_use_reset(&self) -> PropValue {
        PropValue::Bool(self.use_reset)
    }
    fn set_use_reset(&mut self, v: PropValue) -> Result<(), PropError> {
        self.use_reset = expect_bool("UseReset", v)?;
        Ok(())
    }
    fn get_tristate(&self) -> PropValue {
        PropValue::Bool(self.tristate)
    }
    fn set_tristate(&mut self, v: PropValue) -> Result<(), PropError> {
        self.tristate = expect_bool("Tristate", v)?;
        Ok(())
    }
    fn get_trigger(&self) -> PropValue {
        PropValue::String(self.trigger.as_str().to_string())
    }
    fn set_trigger(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Trigger", v)?;
        self.trigger = Trigger::from_str_name(&s);
        Ok(())
    }
    fn get_invert_inputs(&self) -> PropValue {
        PropValue::Bool(self.invert_inputs)
    }
    fn set_invert_inputs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.invert_inputs = expect_bool("InvertInputs", v)?;
        Ok(())
    }
}

impl Component for Latch {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Digital transparent latch / register."
    }

    fn props() -> &'static [PropDef<Self>] {
        const TRIGGER_OPTIONS: &[&str] = &["None", "Clock", "Enable"];
        const CHANNELS: PropDef<Latch> = {
            let mut p = PropDef::int(
                "Channels",
                "Channels",
                MIN_CHANNELS as i64,
                MAX_CHANNELS as i64,
                Latch::get_channels,
                Latch::set_channels,
            )
            .with_info("Number of latch channels/bits.");
            p.structural = true;
            p
        };
        const USE_RESET: PropDef<Latch> = {
            let mut p = PropDef::bool(
                "UseReset",
                "Use Reset",
                Latch::get_use_reset,
                Latch::set_use_reset,
            )
            .with_info("Shows/hides the asynchronous Reset pin.");
            p.structural = true;
            p
        };
        const TRISTATE: PropDef<Latch> = {
            let mut p = PropDef::bool(
                "Tristate",
                "Tristate",
                Latch::get_tristate,
                Latch::set_tristate,
            )
            .with_info("High-impedance (tri-state) output stage.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Latch>] = &[
            CHANNELS,
            USE_RESET,
            TRISTATE,
            PropDef::enumeration(
                "Trigger",
                "Trigger",
                TRIGGER_OPTIONS,
                Latch::get_trigger,
                Latch::set_trigger,
            )
            .with_info(
                "\"Clock\" triggers every active edge.\n\"Enable\" any change during active state.\n\"None\" hides Clock pin.",
            ),
            PropDef::bool(
                "InvertInputs",
                "Invert Inputs",
                Latch::get_invert_inputs,
                Latch::set_invert_inputs,
            )
            .with_info("Invert input pins."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        super::group_rows_by(
            self.prop_rows(),
            &[
                ("Main", &["Channels", "UseReset", "Tristate", "Trigger"]),
                ("Inputs", &["InvertInputs"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let ch = self.channels.max(1);
        let h = ch + 2;
        let y0 = -((h as f64) / 2.0) * 8.0 + 8.0;
        let mut pins = Vec::with_capacity(ch * 2 + 3);
        for i in 0..ch {
            let y = y0 + (i as f64) * 8.0;
            pins.push(
                CompPin::new(format!("-in{i}"), -24.0, y, 180, 8.0)
                    .with_direction(PinDirection::In),
            );
            pins.push(
                CompPin::new(format!("-out{i}"), 24.0, y, 0, 8.0).with_direction(PinDirection::Out),
            );
        }
        pins.push(
            CompPin::new("-clk", -24.0, y0 + (ch as f64) * 8.0, 180, 8.0)
                .with_direction(PinDirection::In),
        );
        if self.use_reset {
            pins.push(
                CompPin::new("-rst", 0.0, -((h as f64) / 2.0) * 8.0, 90, 8.0)
                    .with_direction(PinDirection::In),
            );
        }
        if self.tristate {
            pins.push(
                CompPin::new("-oe", 0.0, ((h as f64) / 2.0) * 8.0, 270, 8.0)
                    .with_direction(PinDirection::In),
            );
        }
        pins
    }

    fn body(&self) -> Rect {
        let h = ((self.channels + 2) as f64) * 8.0;
        Rect::new(-16.0, -h / 2.0, 32.0, h)
    }
}

impl Stampable for Latch {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Latch {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let ch = self.channels.max(1);
        let h = (ch + 2) as f64 * 8.0;
        d.fill_round_rect(
            -16.0,
            -h / 2.0,
            32.0,
            h,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -16.0,
            -h / 2.0,
            32.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.push(0.0, 0.0, -90.0, 1.0, 1.0);
        d.text(0.0, 0.0, "LATCH", 7.0, ctx.pal.border, Align::Center);
        d.pop();
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_latch_d(&mut self, x: f64, y: f64) -> String {
        let id = format!("LatchD-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::latch(
            &id, x, y, 8, false, false, "pos", false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_latch() {
        let l = Latch::default();
        assert_eq!(l.type_id(), "Latch");
        assert_eq!(l.channels, 4);
        assert!(!l.use_reset);
        assert!(!l.tristate);
        assert_eq!(l.pin_geoms().len(), 4 * 2 + 1); // 4 in + 4 out + 1 clk
    }

    #[test]
    fn latch_with_reset_and_oe() {
        let mut l = Latch::default();
        l.use_reset = true;
        l.tristate = true;
        assert_eq!(l.pin_geoms().len(), 4 * 2 + 3);
    }
}
