//! WaveGen: waveform signal generator instrument.

use crate::canvas::PinDirection;
use crate::canvas::PinGeom;
use std::sync::Arc;

use super::component::{stamp_to_ground, stamp_two_terminal};
use super::drawable::Drawable;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::{Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_FREQ_HZ: f64 = 1e-3;
const MAX_FREQ_HZ: f64 = 1e12;
const MIN_AMPLITUDE_V: f64 = 0.0;
const MAX_AMPLITUDE_V: f64 = 1e6;
const MIN_OFFSET_V: f64 = -1e6;
const MAX_OFFSET_V: f64 = 1e6;
const MIN_PHASE_DEG: f64 = -360.0;
const MAX_PHASE_DEG: f64 = 360.0;
const MIN_STEPS: i64 = 10;
const MAX_STEPS: i64 = 10000;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum WaveType {
    #[default]
    Sine,
    Saw,
    Triangle,
    Square,
    Random,
    Wav,
}

impl WaveType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sine => "Sine",
            Self::Saw => "Saw",
            Self::Triangle => "Triangle",
            Self::Square => "Square",
            Self::Random => "Random",
            Self::Wav => "Wav",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "saw" | "sawtooth" => Self::Saw,
            "triangle" => Self::Triangle,
            "square" => Self::Square,
            "random" => Self::Random,
            "wav" => Self::Wav,
            _ => Self::Sine,
        }
    }
}

impl std::fmt::Display for WaveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const WAVE_TYPE_OPTIONS: &[&str] = &["Sine", "Saw", "Triangle", "Square", "Random", "Wav"];

/// Duty is stored as a 0..=1 fraction. Palette/legacy values were sometimes
/// written as 0..=100 percent; `duty.clamp(0, 1)` then treated 50% as 100%
/// (triangle collapsed to saw, square stuck high).
pub fn normalize_duty(duty: f64) -> f64 {
    if duty > 1.0 {
        (duty / 100.0).clamp(0.0, 1.0)
    } else {
        duty.clamp(0.0, 1.0)
    }
}

/// Function and waveform signal generator.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveGen {
    pub wave_type: WaveType,
    pub freq_hz: f64,
    pub amplitude: f64,
    pub offset: f64,
    pub duty: f64,
    pub phase: f64,
    pub steps: i32,
    pub bipolar: bool,
    pub floating: bool,
    pub file: String,
    pub wav_data: Option<Arc<Vec<f64>>>,
}

impl crate::canvas::Item {
    #[allow(clippy::too_many_arguments)]
    pub fn wave_gen(
        id: impl Into<String>,
        x: f64,
        y: f64,
        wave_type: impl AsRef<str>,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
    ) -> Self {
        Self::wave_gen_full(
            id,
            x,
            y,
            wave_type,
            freq_hz,
            amplitude,
            offset,
            duty,
            0.0,
            64,
            false,
            false,
            String::new(),
            Vec::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wave_gen_full(
        id: impl Into<String>,
        x: f64,
        y: f64,
        wave_type: impl AsRef<str>,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
        phase: f64,
        steps: usize,
        bipolar: bool,
        floating: bool,
        file: String,
        wav_data: Vec<f64>,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            WaveGen {
                wave_type: WaveType::from_str_name(wave_type.as_ref()),
                freq_hz,
                amplitude,
                offset,
                duty: normalize_duty(duty),
                phase,
                steps: steps as i32,
                bipolar,
                floating,
                file,
                wav_data: if wav_data.is_empty() {
                    None
                } else {
                    Some(Arc::new(wav_data))
                },
            },
        )
    }
}

impl Default for WaveGen {
    fn default() -> Self {
        Self {
            wave_type: WaveType::Sine,
            freq_hz: 1000.0,
            amplitude: 5.0,
            offset: 0.0,
            duty: 0.5,
            phase: 0.0,
            steps: 100,
            bipolar: false,
            floating: false,
            file: String::new(),
            wav_data: None,
        }
    }
}

impl WaveGen {
    pub const TYPE_ID: &'static str = "WaveGen";
    pub fn to_element_kind(&self) -> Kind {
        Kind::WaveGen {
            wave_type: self.wave_type.as_str().to_string(),
            freq_hz: self.freq_hz,
            amplitude: self.amplitude,
            offset: self.offset,
            duty: normalize_duty(self.duty),
            phase: self.phase,
            steps: self.steps.max(10),
            bipolar: self.bipolar,
            floating: self.floating,
            file: self.file.clone(),
            wav_data: self.wav_data.clone(),
            wav_index: 0,
            v_out: 0.0,
        }
    }

    fn get_wave_type(&self) -> PropValue {
        PropValue::Enum(self.wave_type.as_str().to_string())
    }
    fn set_wave_type(&mut self, v: PropValue) -> Result<(), PropError> {
        let x = expect_string("WaveType", v)?;
        self.wave_type = WaveType::from_str_name(&x);
        Ok(())
    }

    fn get_frequency(&self) -> PropValue {
        PropValue::Float(self.freq_hz)
    }
    fn set_frequency(&mut self, v: PropValue) -> Result<(), PropError> {
        self.freq_hz = expect_float("Frequency", v)?.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ);
        Ok(())
    }

    fn get_amplitude(&self) -> PropValue {
        PropValue::Float(self.amplitude)
    }
    fn set_amplitude(&mut self, v: PropValue) -> Result<(), PropError> {
        self.amplitude = expect_float("Amplitude", v)?.clamp(MIN_AMPLITUDE_V, MAX_AMPLITUDE_V);
        Ok(())
    }

    fn get_offset(&self) -> PropValue {
        PropValue::Float(self.offset)
    }
    fn set_offset(&mut self, v: PropValue) -> Result<(), PropError> {
        self.offset = expect_float("Offset", v)?.clamp(MIN_OFFSET_V, MAX_OFFSET_V);
        Ok(())
    }

    fn get_duty(&self) -> PropValue {
        PropValue::Float(normalize_duty(self.duty) * 100.0)
    }
    fn set_duty(&mut self, v: PropValue) -> Result<(), PropError> {
        self.duty = expect_float("Duty", v)?.clamp(0.0, 100.0) / 100.0;
        Ok(())
    }

    fn get_phase(&self) -> PropValue {
        PropValue::Float(self.phase)
    }
    fn set_phase(&mut self, v: PropValue) -> Result<(), PropError> {
        self.phase = expect_float("Phase", v)?.clamp(MIN_PHASE_DEG, MAX_PHASE_DEG);
        Ok(())
    }

    fn get_steps(&self) -> PropValue {
        PropValue::Int(self.steps as i64)
    }
    fn set_steps(&mut self, v: PropValue) -> Result<(), PropError> {
        self.steps = expect_int("Steps", v)?.clamp(MIN_STEPS, MAX_STEPS) as i32;
        Ok(())
    }

    fn get_bipolar(&self) -> PropValue {
        PropValue::Bool(self.bipolar)
    }
    fn set_bipolar(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bipolar = expect_bool("Bipolar", v)?;
        Ok(())
    }

    fn get_floating(&self) -> PropValue {
        PropValue::Bool(self.floating)
    }
    fn set_floating(&mut self, v: PropValue) -> Result<(), PropError> {
        self.floating = expect_bool("Floating", v)?;
        Ok(())
    }

    fn get_file(&self) -> PropValue {
        PropValue::String(self.file.clone())
    }
    fn set_file(&mut self, v: PropValue) -> Result<(), PropError> {
        let path = expect_string("File", v)?;
        self.file = path.clone();
        if !path.is_empty() {
            if let Ok(wav) = crate::wav::WavData::from_file(&path) {
                self.wav_data = Some(wav.samples);
                if self.wave_type == WaveType::Wav {
                    self.freq_hz = wav.sample_rate as f64;
                }
            }
        }
        Ok(())
    }
}

impl Component for WaveGen {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Function / waveform generator."
    }

    fn props() -> &'static [PropDef<Self>] {
        const BIPOLAR: PropDef<WaveGen> = {
            let mut p = PropDef::bool(
                "Bipolar",
                "Bipolar",
                WaveGen::get_bipolar,
                WaveGen::set_bipolar,
            )
            .with_info("If true, use 2 output pins.");
            p.structural = true;
            p
        };

        static PROPS: &[PropDef<WaveGen>] = &[
            PropDef::enumeration(
                "WaveType",
                "Wave Type",
                WAVE_TYPE_OPTIONS,
                WaveGen::get_wave_type,
                WaveGen::set_wave_type,
            ).with_info("Set output wave type."),
            PropDef::float(
                "Frequency",
                "Frequency",
                "Hz",
                MIN_FREQ_HZ,
                MAX_FREQ_HZ,
                WaveGen::get_frequency,
                WaveGen::set_frequency,
            ).with_info("Set output frequency."),
            PropDef::float(
                "Amplitude",
                "Semi Amplitude",
                "V",
                MIN_AMPLITUDE_V,
                MAX_AMPLITUDE_V,
                WaveGen::get_amplitude,
                WaveGen::set_amplitude,
            ).with_info("Output wave peak voltage amplitude."),
            PropDef::float(
                "Offset",
                "Middle Voltage",
                "V",
                MIN_OFFSET_V,
                MAX_OFFSET_V,
                WaveGen::get_offset,
                WaveGen::set_offset,
            ).with_info("DC offset voltage of the output waveform."),
            PropDef::float(
                "Duty",
                "Duty Cycle",
                "%",
                0.0,
                100.0,
                WaveGen::get_duty,
                WaveGen::set_duty,
            ).with_info("Duty cycle.\nNot available for all types."),
            PropDef::float(
                "Phase",
                "Phase Shift",
                "°",
                MIN_PHASE_DEG,
                MAX_PHASE_DEG,
                WaveGen::get_phase,
                WaveGen::set_phase,
            ).with_info("Phase shift relative to wave start."),
            PropDef::int(
                "Steps",
                "Minimum Steps",
                MIN_STEPS,
                MAX_STEPS,
                WaveGen::get_steps,
                WaveGen::set_steps,
            ).with_info("Encoder steps per dial rotation."),
            BIPOLAR,
            PropDef::bool(
                "Floating",
                "Floating",
                WaveGen::get_floating,
                WaveGen::set_floating,
            ).with_info("If true, output voltages are not referred to ground.\nOnly visible if Bipolar is true."),
            PropDef::string("File", "File", WaveGen::get_file, WaveGen::set_file).with_info("Wav file to play. Only for Wav type."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<super::PropGroup> {
        let show_duty = self.wave_type == WaveType::Triangle || self.wave_type == WaveType::Square;
        let show_file = self.wave_type == WaveType::Wav;
        let show_floating = self.bipolar;

        let mut rows = self.prop_rows();
        for r in &mut rows {
            match r.name {
                "Duty" => r.visible = show_duty,
                "File" => r.visible = show_file,
                "Floating" => r.visible = show_floating,
                _ => {}
            }
        }

        super::group_rows_by(
            rows,
            &[
                (
                    "Main",
                    &["WaveType", "Frequency", "Phase", "Duty", "File", "Steps"],
                ),
                ("Electric", &["Bipolar", "Amplitude", "Offset", "Floating"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        if self.bipolar {
            wavegen_bipolar_pins().iter().map(CompPin::from).collect()
        } else {
            wavegen_unipolar_pins().iter().map(CompPin::from).collect()
        }
    }

    fn body(&self) -> Rect {
        Rect::new(-8.0, -8.0, 16.0, 16.0)
    }
}

impl Stampable for WaveGen {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if self.bipolar {
            let volt = 2.0 * self.amplitude * (0.0 - 0.5);
            if self.floating {
                stamp_two_terminal(matrix, pin_nodes, SOURCE_ADMIT, volt * SOURCE_ADMIT);
            } else {
                let half_v = volt / 2.0;
                stamp_to_ground(matrix, pin_nodes, 0, self.offset + half_v, SOURCE_ADMIT);
                stamp_to_ground(matrix, pin_nodes, 1, self.offset - half_v, SOURCE_ADMIT);
            }
        } else {
            let volt_base = self.offset - self.amplitude;
            let v = volt_base + 2.0 * self.amplitude * 0.0;
            stamp_to_ground(matrix, pin_nodes, 0, v, SOURCE_ADMIT);
        }
    }
}

impl Drawable for WaveGen {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_circle(0.0, 0.0, 8.0, ctx.pal.body.fade(COMPONENT_FILL_ALPHA));
        d.stroke_circle(0.0, 0.0, 8.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        match self.wave_type {
            WaveType::Square => {
                let pts = [
                    [-4.5, 0.0],
                    [-4.5, -3.6],
                    [0.0, -3.6],
                    [0.0, 3.6],
                    [4.5, 3.6],
                    [4.5, 0.0],
                ];
                d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, false);
            }
            WaveType::Saw => {
                let pts = [[-4.1, 3.6], [4.1, -3.6], [4.1, 3.6]];
                d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, false);
            }
            WaveType::Triangle => {
                let pts = [[-4.5, 2.7], [0.0, -2.7], [4.5, 2.7]];
                d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, false);
            }
            WaveType::Random => {
                let pts = [
                    [-4.5, 1.8],
                    [-4.5, -1.8],
                    [-2.7, -1.8],
                    [-2.7, -3.6],
                    [-0.9, -3.6],
                    [-0.9, 1.8],
                    [0.9, 1.8],
                    [0.9, -0.9],
                    [2.7, -0.9],
                    [2.7, 3.6],
                    [4.5, 3.6],
                ];
                d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, false);
            }
            WaveType::Wav => {
                d.line(
                    -4.5,
                    -1.5,
                    -4.5,
                    1.5,
                    ctx.pal.border,
                    COMPONENT_BORDER_WIDTH,
                );
                d.line(
                    -2.7,
                    -3.2,
                    -2.7,
                    3.2,
                    ctx.pal.border,
                    COMPONENT_BORDER_WIDTH,
                );
                d.line(
                    -0.9,
                    -3.6,
                    -0.9,
                    3.6,
                    ctx.pal.border,
                    COMPONENT_BORDER_WIDTH,
                );
                d.line(0.9, -2.5, 0.9, 2.5, ctx.pal.border, COMPONENT_BORDER_WIDTH);
                d.line(2.7, -3.2, 2.7, 3.2, ctx.pal.border, COMPONENT_BORDER_WIDTH);
                d.line(4.5, -1.6, 4.5, 1.6, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            }
            WaveType::Sine => {
                let pts = [
                    [-4.5, 0.0],
                    [-4.1, -1.0],
                    [-3.0, -2.6],
                    [-2.25, -3.4],
                    [-1.5, -3.4],
                    [-0.75, -2.5],
                    [0.0, 0.0],
                    [0.75, 2.5],
                    [1.5, 3.4],
                    [2.25, 3.4],
                    [3.0, 2.6],
                    [4.1, 1.0],
                    [4.5, 0.0],
                ];
                d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, false);
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_wave_gen(&mut self, x: f64, y: f64) -> String {
        let id = format!("WaveGen-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::wave_gen(
            &id, x, y, "Sine", 1000.0, 5.0, 0.0, 0.5,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_wavegen() {
        let wg = WaveGen::default();
        assert_eq!(wg.type_id(), "WaveGen");
        assert_eq!(wg.pin_geoms().len(), 1);
        assert_eq!(wg.wave_type, WaveType::Sine);
        assert_eq!(wg.freq_hz, 1000.0);
        assert_eq!(wg.amplitude, 5.0);
        assert_eq!(wg.offset, 0.0);
        assert_eq!(wg.duty, 0.5);
        assert_eq!(wg.phase, 0.0);
        assert_eq!(wg.steps, 100);
        assert!(!wg.bipolar);
        assert!(!wg.floating);
    }

    #[test]
    fn bipolar_changes_pins() {
        let mut wg = WaveGen::default();
        assert_eq!(wg.pin_geoms().len(), 1);
        let ch = wg.set_prop("Bipolar", PropValue::Bool(true)).unwrap();
        assert!(ch.structural);
        assert!(wg.bipolar);
        assert_eq!(wg.pin_geoms().len(), 2);
    }

    #[test]
    fn duty_percentage_conversion() {
        let mut wg = WaveGen::default();
        wg.set_prop_text("Duty", "75 %").unwrap();
        assert_eq!(wg.duty, 0.75);
        assert_eq!(wg.get_prop_text("Duty").unwrap(), "75 %");
    }

    #[test]
    fn duty_percent_construct_and_palette() {
        assert!((normalize_duty(50.0) - 0.5).abs() < 1e-12);
        assert!((normalize_duty(0.25) - 0.25).abs() < 1e-12);
        let item =
            crate::canvas::Item::wave_gen("wg", 0.0, 0.0, "Triangle", 1000.0, 5.0, 0.0, 50.0);
        assert!((item.duty() - 0.5).abs() < 1e-12);
        let mut scene = crate::canvas::Scene::default();
        let id = scene.add_wave_gen(0.0, 0.0);
        let placed = scene.item_by_id(&id).expect("palette WaveGen");
        assert!((placed.duty() - 0.5).abs() < 1e-12);
        assert_eq!(placed.prop_text("Duty").as_deref(), Some("50 %"));
    }

    #[test]
    fn element_kind_mapping() {
        let wg = WaveGen::default();
        match wg.to_element_kind() {
            Kind::WaveGen {
                wave_type, freq_hz, ..
            } => {
                assert_eq!(wave_type, "Sine");
                assert_eq!(freq_hz, 1000.0);
            }
            other => panic!("expected Kind::WaveGen, got {other:?}"),
        }
    }
}

const WAVEGEN_BIPOLAR_PINS: [PinGeom; 2] = [
    PinGeom {
        suffix: "-outnod",
        x: 16.0,
        y: -4.0,
        angle: 0,
        length: 9.07,
        direction: Some(PinDirection::Out),
    },
    PinGeom {
        suffix: "-gndnod",
        x: 16.0,
        y: 4.0,
        angle: 0,
        length: 9.07,
        direction: None,
    },
];

fn wavegen_bipolar_pins() -> &'static [PinGeom] {
    &WAVEGEN_BIPOLAR_PINS
}

const WAVEGEN_UNIPOLAR_PINS: [PinGeom; 1] = [PinGeom {
    suffix: "-outnod",
    x: 16.0,
    y: 0.0,
    angle: 0,
    length: 8.0,
    direction: Some(PinDirection::Out),
}];

fn wavegen_unipolar_pins() -> &'static [PinGeom] {
    &WAVEGEN_UNIPOLAR_PINS
}
