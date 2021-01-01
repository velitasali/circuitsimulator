//! AnalogMux: analog multiplexer.

use super::component::stamp_conductance_between;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_CHANNELS: usize = 2;
const MAX_CHANNELS: usize = 16;
const MIN_RDSON: f64 = 1e-6;
const MAX_RDSON: f64 = 1e6;

#[derive(Clone, Debug, PartialEq)]
pub struct AnalogMux {
    pub channels: usize,
    pub on_resistance: f64,
}

impl crate::canvas::Item {
    pub fn analog_mux(
        id: impl Into<String>,
        x: f64,
        y: f64,
        channels: usize,
        on_resistance: f64,
    ) -> Self {
        Self::analog_mux_with(id, x, y, channels, on_resistance)
    }

    pub fn analog_mux_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        channels: usize,
        on_resistance: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            AnalogMux {
                channels,
                on_resistance,
            },
        )
    }
}

impl Default for AnalogMux {
    fn default() -> Self {
        Self {
            channels: 4,
            on_resistance: 50.0,
        }
    }
}

impl AnalogMux {
    pub const TYPE_ID: &'static str = "AnalogMux";
    pub fn to_element_kind(&self) -> Kind {
        Kind::AnalogMux {
            channels: self.channels,
            selected: 0,
            on_res: self.on_resistance,
        }
    }

    fn get_channels(&self) -> PropValue {
        PropValue::Int(self.channels as i64)
    }
    fn set_channels(&mut self, v: PropValue) -> Result<(), PropError> {
        self.channels =
            expect_int("Channels", v)?.clamp(MIN_CHANNELS as i64, MAX_CHANNELS as i64) as usize;
        Ok(())
    }
    fn get_rdson(&self) -> PropValue {
        PropValue::Float(self.on_resistance)
    }
    fn set_rdson(&mut self, v: PropValue) -> Result<(), PropError> {
        self.on_resistance = expect_float("RDSon", v)?.clamp(MIN_RDSON, MAX_RDSON);
        Ok(())
    }
}

impl Component for AnalogMux {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Analog multiplexer."
    }

    fn props() -> &'static [PropDef<Self>] {
        const CHANNELS: PropDef<AnalogMux> = {
            let mut p = PropDef::int(
                "Channels",
                "Channels",
                MIN_CHANNELS as i64,
                MAX_CHANNELS as i64,
                AnalogMux::get_channels,
                AnalogMux::set_channels,
            )
            .with_info("Number of latch channels/bits.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<AnalogMux>] = &[
            CHANNELS,
            PropDef::float(
                "RDSon",
                "On Resistance",
                "Ω",
                MIN_RDSON,
                MAX_RDSON,
                AnalogMux::get_rdson,
                AnalogMux::set_rdson,
            )
            .with_info("Drain-Source ON resistance."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let ch = self.channels.max(2);
        let bits = ((ch as f64).log2().ceil() as usize).max(1);
        let mut pins = Vec::with_capacity(ch + bits + 2);

        pins.push(CompPin::new("-PinInput", -24.0, 8.0, 180, 8.0).with_direction(PinDirection::In));

        for i in 0..bits {
            pins.push(
                CompPin::new(
                    format!("-pinAddr{i}"),
                    -24.0,
                    24.0 + (i as f64) * 8.0,
                    180,
                    8.0,
                )
                .with_direction(PinDirection::In),
            );
        }

        pins.push(
            CompPin::new("-PinEnable", -24.0, 32.0 + (bits as f64) * 8.0, 180, 8.0)
                .with_direction(PinDirection::In),
        );

        for i in 0..ch {
            pins.push(
                CompPin::new(format!("-pinY{i}"), 24.0, 8.0 + (i as f64) * 8.0, 0, 8.0)
                    .with_direction(PinDirection::Out),
            );
        }
        pins
    }

    fn body(&self) -> Rect {
        let ch = self.channels.max(2);
        let bits = ((ch as f64).log2().ceil() as usize).max(1);
        let rside = ch * 8 + 8;
        let size = (40 + bits * 8).max(rside) as f64;
        Rect::new(-16.0, 0.0, 32.0, size)
    }
}

impl Stampable for AnalogMux {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let ch = self.channels.max(2);
        let bits = ((ch as f64).log2().ceil() as usize).max(1);
        let y0_idx = 2 + bits;
        let g = 1.0 / self.on_resistance.max(MIN_RDSON);
        stamp_conductance_between(matrix, pin_nodes, 0, y0_idx, g);
    }
}

impl Drawable for AnalogMux {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let ch = self.channels.max(2);
        let bits = ((ch as f64).log2().ceil() as usize).max(1);
        let rside = ch * 8 + 8;
        let h = (40 + bits * 8).max(rside) as f64;
        d.fill_round_rect(
            -16.0,
            0.0,
            32.0,
            h,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -16.0,
            0.0,
            32.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.text(
            0.0,
            h / 2.0 - 4.0,
            "MUX",
            7.0,
            ctx.pal.border,
            Align::Center,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_analog_mux(&mut self, x: f64, y: f64) -> String {
        let id = format!("AnalogMux-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::analog_mux(&id, x, y, 4, 100.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_analog_mux() {
        let m = AnalogMux::default();
        assert_eq!(m.type_id(), "AnalogMux");
        assert_eq!(m.channels, 4);
        assert_eq!(m.on_resistance, 50.0);
        // 1 in + 2 addr + 1 en + 4 y = 8 pins
        assert_eq!(m.pin_geoms().len(), 8);
    }
}
