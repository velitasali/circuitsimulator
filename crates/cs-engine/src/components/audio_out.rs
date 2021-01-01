//! Audio output (speaker / buzzer) transducer component.

use super::component::stamp_two_terminal;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::audio::Source;
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_IMP: f64 = 0.1;
const MAX_IMP: f64 = 1e6;
const MIN_VOL: f64 = 0.0;
const MAX_VOL: f64 = 100.0;
const MIN_FREQ: f64 = 1.0;
const MAX_FREQ: f64 = 100_000.0;

impl crate::canvas::Item {
    pub fn audio_out(id: impl Into<String>, x: f64, y: f64, impedance: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            AudioOut {
                impedance,
                volume: 100.0,
                buzzer: false,
                frequency: 1000.0,
            },
        )
    }

    pub fn audio_out_full(
        id: impl Into<String>,
        x: f64,
        y: f64,
        impedance: f64,
        volume: f64,
        buzzer: bool,
        frequency: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            AudioOut {
                impedance,
                volume,
                buzzer,
                frequency,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioOut {
    pub impedance: f64,
    pub volume: f64,
    pub buzzer: bool,
    pub frequency: f64,
}

impl Default for AudioOut {
    fn default() -> Self {
        Self {
            impedance: 8.0,
            volume: 100.0,
            buzzer: false,
            frequency: 1000.0,
        }
    }
}

impl AudioOut {
    pub const TYPE_ID: &'static str = "AudioOut";
    pub fn to_element_kind(&self) -> Kind {
        Kind::AudioOut {
            source: Source::new(self.impedance, self.volume, self.frequency, self.buzzer),
            last_v: 0.0,
        }
    }

    fn get_impedance(&self) -> PropValue {
        PropValue::Float(self.impedance)
    }
    fn set_impedance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.impedance = expect_float("Impedance", v)?.clamp(MIN_IMP, MAX_IMP);
        Ok(())
    }

    fn get_volume(&self) -> PropValue {
        PropValue::Float(self.volume)
    }
    fn set_volume(&mut self, v: PropValue) -> Result<(), PropError> {
        self.volume = expect_float("Volume", v)?.clamp(MIN_VOL, MAX_VOL);
        Ok(())
    }

    fn get_buzzer(&self) -> PropValue {
        PropValue::Bool(self.buzzer)
    }
    fn set_buzzer(&mut self, v: PropValue) -> Result<(), PropError> {
        self.buzzer = expect_bool("Buzzer", v)?;
        Ok(())
    }

    fn get_frequency(&self) -> PropValue {
        PropValue::Float(self.frequency)
    }
    fn set_frequency(&mut self, v: PropValue) -> Result<(), PropError> {
        self.frequency = expect_float("Frequency", v)?.clamp(MIN_FREQ, MAX_FREQ);
        Ok(())
    }
}

impl Component for AudioOut {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Audio speaker / buzzer output."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<AudioOut>] = &[
            PropDef::bool(
                "Buzzer",
                "Buzzer",
                AudioOut::get_buzzer,
                AudioOut::set_buzzer,
            )
            .with_info("Act as a buzzer instead of as a speaker."),
            PropDef::float(
                "Impedance",
                "Impedance",
                "Ω",
                MIN_IMP,
                MAX_IMP,
                AudioOut::get_impedance,
                AudioOut::set_impedance,
            )
            .with_info("Audio output impedance."),
            PropDef::float(
                "Frequency",
                "Frequency",
                "Hz",
                MIN_FREQ,
                MAX_FREQ,
                AudioOut::get_frequency,
                AudioOut::set_frequency,
            )
            .with_info("Buzzer frequency."),
            PropDef::float(
                "Volume",
                "Volume",
                "%",
                MIN_VOL,
                MAX_VOL,
                AudioOut::get_volume,
                AudioOut::set_volume,
            )
            .with_info("Audio playback volume level."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        let mut rows = self.prop_rows();
        for r in &mut rows {
            match r.name {
                "Frequency" => r.visible = self.buzzer,
                "Impedance" => r.visible = !self.buzzer,
                _ => {}
            }
        }
        vec![super::PropGroup::new("Main", rows)]
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        audio_out_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-10.0, -24.0, 20.0, 40.0)
    }
}

impl TwoTerminal for AudioOut {}

impl Stampable for AudioOut {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let source = Source::new(self.impedance, self.volume, self.frequency, self.buzzer);
        stamp_two_terminal(matrix, pin_nodes, source.stamp_admit(), 0.0);
    }
}

impl Drawable for AudioOut {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        if self.buzzer {
            let mut pts = Vec::with_capacity(18);
            pts.push([-10.0, -24.0]);
            for i in 0..=16 {
                let angle =
                    -std::f64::consts::FRAC_PI_2 + (std::f64::consts::PI * (i as f64) / 16.0);
                let x = -10.0 + 20.0 * angle.cos();
                let y = -4.0 + 20.0 * angle.sin();
                pts.push([x, y]);
            }
            pts.push([-10.0, 16.0]);
            d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
            d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        } else {
            let pts = [
                [-10.0, -12.0],
                [-10.0, 4.0],
                [0.0, 4.0],
                [10.0, 16.0],
                [10.0, -24.0],
                [0.0, -12.0],
            ];
            d.fill_poly(&pts, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
            d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_audio_out(&mut self, x: f64, y: f64) -> String {
        let id = format!("AudioOut-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::audio_out(&id, x, y, 8.0));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_audio_out() {
        let a = AudioOut::default();
        assert_eq!(a.type_id(), "AudioOut");
        assert_eq!(a.impedance, 8.0);
        assert_eq!(a.volume, 100.0);
        assert!(!a.buzzer);
        assert_eq!(a.frequency, 1000.0);
        assert_eq!(a.pin_geoms().len(), 2);
        assert_eq!(a.body(), Rect::new(-10.0, -24.0, 20.0, 40.0));
    }
}

const AUDIO_OUT_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-lPin",
        x: -16.0,
        y: -8.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-rPin",
        x: -16.0,
        y: 0.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
];

fn audio_out_pins() -> &'static [PinGeom] {
    &AUDIO_OUT_PINS
}
