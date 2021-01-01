//! Host audio output. Sample generation matches C++ `AudioOut::runEvent`;
//! playback is `cpal` (not Qt Multimedia). Catalog drawing stays last.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// C++ Qt6 `AudioOut` format.
pub const SAMPLE_RATE: u32 = 44_100;
/// C++ `m_dataSize = 8000` bytes / 2 = 4000 samples.
pub const CHUNK_SAMPLES: usize = 4000;
/// C++ speaker: `voltPN * 13056 * gain`.
pub const SPEAKER_SCALE: f64 = 13_056.0;
/// C++ buzzer threshold.
pub const BUZZER_THRESHOLD: f64 = 2.5;
pub const DEFAULT_IMPEDANCE: f64 = 8.0;
pub const DEFAULT_VOLUME: f64 = 100.0;
pub const DEFAULT_FREQUENCY: f64 = 1_000.0;

/// One PCM sample as C++ `AudioOut::runEvent` would emit.
pub fn sample(volt_pn: f64, volume: f64, buzzer: bool, frequency: f64, circ_time_ps: u64) -> i16 {
    let gain = (volume / 100.0).clamp(0.0, 1.0);
    let mut v = 0.0;
    if buzzer {
        if volt_pn > BUZZER_THRESHOLD && frequency > 0.0 {
            let steps_pc = 1e12 / frequency;
            let time = ieee_remainder(circ_time_ps as f64, steps_pc);
            let angle = time * std::f64::consts::TAU / steps_pc;
            v += angle.sin() * 32767.0 * gain;
        }
    } else {
        v += volt_pn * SPEAKER_SCALE * gain;
    }
    v.round().clamp(-32768.0, 32767.0) as i16
}

/// C++ `std::remainder` (IEEE 754, used via `<cmath>` / QtMath).
fn ieee_remainder(x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return f64::NAN;
    }
    x - (x / y).round() * y
}

/// Collects samples into C++-sized chunks of 4000 i16s.
#[derive(Clone, Debug)]
pub struct Source {
    pub impedance: f64,
    pub volume: f64,
    pub frequency: f64,
    pub buzzer: bool,
    buf: Vec<i16>,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            impedance: DEFAULT_IMPEDANCE,
            volume: DEFAULT_VOLUME,
            frequency: DEFAULT_FREQUENCY,
            buzzer: false,
            buf: Vec::with_capacity(CHUNK_SAMPLES),
        }
    }
}

impl Source {
    pub fn new(impedance: f64, volume: f64, frequency: f64, buzzer: bool) -> Self {
        Self {
            impedance,
            volume,
            frequency,
            buzzer,
            buf: Vec::with_capacity(CHUNK_SAMPLES),
        }
    }

    pub fn stamp_admit(&self) -> f64 {
        if self.buzzer {
            1e-4
        } else if self.impedance > 0.0 {
            1.0 / self.impedance
        } else {
            1.0 / DEFAULT_IMPEDANCE
        }
    }

    pub fn push(&mut self, volt_pn: f64, circ_time_ps: u64) -> Option<Vec<i16>> {
        let s = sample(
            volt_pn,
            self.volume,
            self.buzzer,
            self.frequency,
            circ_time_ps,
        );
        self.buf.push(s);
        if self.buf.len() >= CHUNK_SAMPLES {
            Some(std::mem::take(&mut self.buf))
        } else {
            None
        }
    }

    pub fn take_samples(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.buf)
    }

    pub fn push_sample(&mut self, volt_pn: f64, circ_time_ps: u64) -> i16 {
        let s = sample(
            volt_pn,
            self.volume,
            self.buzzer,
            self.frequency,
            circ_time_ps,
        );
        self.buf.push(s);
        s
    }

    pub fn reset(&mut self) {
        self.buf.clear();
    }
}

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Output devices from the default `cpal` host. Empty if none / error.
pub fn list_output_devices() -> Vec<DeviceInfo> {
    let host = cpal::default_host();
    let default_name = host.default_output_device().and_then(|d| d.name().ok());
    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| {
            let name = d.name().ok()?;
            let is_default = default_name.as_ref() == Some(&name);
            Some(DeviceInfo { name, is_default })
        })
        .collect()
}

pub fn has_output_device() -> bool {
    cpal::default_host().default_output_device().is_some()
}

#[derive(Debug)]
pub enum AudioError {
    NoDevice,
    Stream(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDevice => write!(f, "No Audio Output Devices Found"),
            Self::Stream(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// Trait for live audio output streams and test sinks.
pub trait AudioSink: Send + Sync + std::fmt::Debug {
    fn write(&mut self, samples: &[i16]);
    fn sample_rate(&self) -> u32;
    fn stop(&mut self);
    fn resume(&mut self) {}
}

/// Headless sink used by tests and by `-nogui` until a device is opened.
#[derive(Clone, Debug, Default)]
pub struct Recorder {
    pub samples: Vec<i16>,
}

impl Recorder {
    pub fn write(&mut self, chunk: &[i16]) {
        self.samples.extend_from_slice(chunk);
    }
}

impl AudioSink for Recorder {
    fn write(&mut self, samples: &[i16]) {
        self.samples.extend_from_slice(samples);
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn stop(&mut self) {}

    fn resume(&mut self) {}
}

/// Live `cpal` output. Samples are i16; the stream converts to the device
/// format. Underruns play silence (0), matching C++ signed-int fill.
pub struct Playback {
    queue: Arc<Mutex<VecDeque<i16>>>,
    _stream: cpal::Stream,
    pub sample_rate: u32,
    active: Arc<AtomicBool>,
    buffering: Arc<AtomicBool>,
}

unsafe impl Send for Playback {}
unsafe impl Sync for Playback {}

impl Playback {
    pub fn try_start() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
        let config = device
            .default_output_config()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        let sample_rate = config.sample_rate().0;
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let q = queue.clone();
        let channels = config.channels() as usize;
        let active = Arc::new(AtomicBool::new(true));
        let buffering = Arc::new(AtomicBool::new(true));
        let prebuffer_samples = ((sample_rate as usize * 50) / 1000).max(512);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config.into(),
                q,
                channels,
                active.clone(),
                buffering.clone(),
                prebuffer_samples,
            )?,
            cpal::SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config.into(),
                q,
                channels,
                active.clone(),
                buffering.clone(),
                prebuffer_samples,
            )?,
            cpal::SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config.into(),
                q,
                channels,
                active.clone(),
                buffering.clone(),
                prebuffer_samples,
            )?,
            fmt => {
                return Err(AudioError::Stream(format!(
                    "unsupported sample format {fmt:?}"
                )));
            }
        };
        stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        Ok(Self {
            queue,
            _stream: stream,
            sample_rate,
            active,
            buffering,
        })
    }

    pub fn write(&self, chunk: &[i16]) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            q.extend(chunk.iter().copied());
            // Bound memory if the sim outruns the device (hold up to 2 seconds).
            let cap = (self.sample_rate as usize * 2).max(SAMPLE_RATE as usize);
            while q.len() > cap {
                q.pop_front();
            }
        }
    }

    pub fn stop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
        self.buffering.store(true, Ordering::Relaxed);
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }

    pub fn resume(&mut self) {
        self.active.store(true, Ordering::Relaxed);
        self.buffering.store(true, Ordering::Relaxed);
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Playback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playback")
            .field("sample_rate", &self.sample_rate)
            .field("active", &self.active.load(Ordering::Relaxed))
            .finish()
    }
}

impl AudioSink for Playback {
    fn write(&mut self, samples: &[i16]) {
        Playback::write(self, samples);
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn stop(&mut self) {
        Playback::stop(self);
    }

    fn resume(&mut self) {
        Playback::resume(self);
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    queue: Arc<Mutex<VecDeque<i16>>>,
    channels: usize,
    active: Arc<AtomicBool>,
    buffering: Arc<AtomicBool>,
    prebuffer_samples: usize,
) -> Result<cpal::Stream, AudioError>
where
    T: cpal::SizedSample + FromSample,
{
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                if !active.load(Ordering::Relaxed) {
                    for sample in out.iter_mut() {
                        *sample = T::from_i16(0);
                    }
                    return;
                }
                let mut q = queue.lock().ok();
                if buffering.load(Ordering::Relaxed) {
                    if let Some(ref q) = q {
                        if q.len() >= prebuffer_samples {
                            buffering.store(false, Ordering::Relaxed);
                        }
                    }
                    if buffering.load(Ordering::Relaxed) {
                        for sample in out.iter_mut() {
                            *sample = T::from_i16(0);
                        }
                        return;
                    }
                }
                for frame in out.chunks_mut(channels) {
                    let s = q.as_mut().and_then(|q| q.pop_front()).unwrap_or(0);
                    let v = T::from_i16(s);
                    for ch in frame.iter_mut() {
                        *ch = v;
                    }
                }
            },
            |_| {},
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))
}

trait FromSample {
    fn from_i16(s: i16) -> Self;
}

impl FromSample for f32 {
    fn from_i16(s: i16) -> Self {
        s as f32 / 32768.0
    }
}

impl FromSample for i16 {
    fn from_i16(s: i16) -> Self {
        s
    }
}

impl FromSample for u16 {
    fn from_i16(s: i16) -> Self {
        (s as i32 + 32768) as u16
    }
}

/// Picoseconds between samples. C++ uses `realSpeed * 1e12 / 10000 / sampleRate`
/// while running; at 100% speed (`ps_per_sec = 1e12`) that is `1e12 / sampleRate` ps.
/// At lower simulation speeds, samples are taken more frequently in circuit time
/// so the physical audio playback rate stays continuous and constant in real time.
pub fn sample_period_ps(sample_rate: u32, ps_per_sec: u64) -> u64 {
    let sr = sample_rate.max(1) as f64;
    let ps = ps_per_sec.max(1) as f64;
    (ps / sr).round().max(1.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_scale_near_full_at_2v5() {
        let s = sample(2.5, 100.0, false, 1000.0, 0);
        assert!((s as i32 - 32640).abs() < 2, "sample {s}");
    }

    #[test]
    fn speaker_clips() {
        let s = sample(10.0, 100.0, false, 1000.0, 0);
        assert_eq!(s, 32767);
        let s = sample(-10.0, 100.0, false, 1000.0, 0);
        assert_eq!(s, -32768);
    }

    #[test]
    fn buzzer_silent_below_threshold() {
        let s = sample(1.0, 100.0, true, 1000.0, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn buzzer_sine_at_1khz() {
        let period = 1_000_000_000u64; // 1 kHz in ps
        let a = sample(5.0, 100.0, true, 1000.0, 0);
        let b = sample(5.0, 100.0, true, 1000.0, period / 4);
        let c = sample(5.0, 100.0, true, 1000.0, period / 2);
        assert_eq!(a, 0, "t=0 should be sin(0)");
        assert!((b as i32 - 32767).abs() < 2, "quarter {b}");
        assert!(c.abs() <= 1, "half {c}");
    }

    #[test]
    fn chunk_size() {
        let mut src = Source::default();
        let mut got = None;
        for i in 0..CHUNK_SAMPLES {
            got = src.push(0.0, i as u64);
            if i + 1 < CHUNK_SAMPLES {
                assert!(got.is_none());
            }
        }
        assert_eq!(got.unwrap().len(), CHUNK_SAMPLES);
    }

    #[test]
    fn buzzer_admit() {
        let mut src = Source::default();
        assert!((src.stamp_admit() - 1.0 / 8.0).abs() < 1e-12);
        src.buzzer = true;
        assert!((src.stamp_admit() - 1e-4).abs() < 1e-15);
    }

    #[test]
    fn recorder_keeps_samples() {
        let mut r = Recorder::default();
        r.write(&[1, 2, 3]);
        assert_eq!(r.samples, vec![1, 2, 3]);
    }
}
