//! Live instruments: probes, meters, oscilloscope and logic-analyzer capture.
//! No Qt. Display traces are `PlotBuffer`s drawn by QML Canvas.

use rustc_hash::FxHashMap;

use crate::plot::{Channel, PlotBuffer};

/// C++ Probe default threshold.
pub const PROBE_DEFAULT_THRESHOLD: f64 = 2.5;
/// C++ `IoPin::setImpedance(1e9)` → G = 1e-9 S.
pub const PROBE_ADMIT: f64 = 1e-9;
/// C++ `eElement::high_imp` used as voltmeter resistance.
pub const VOLTMETER_OHMS: f64 = crate::HIGH_IMP;
/// C++ Amperimeter `setResistance(1e-6)`.
pub const AMMETER_OHMS: f64 = 1e-6;
/// C++ PlotBase `m_inputAdmit` (connect-to-ground).
pub const PLOT_INPUT_ADMIT: f64 = 1e-7;
/// C++ Oscope default Time/Div = 1e9 ps = 1 ms.
pub const SCOPE_TIME_DIV_DEFAULT: f64 = 1e-3;
/// C++ LAnalizer default thresholds.
pub const LA_THRESHOLD_DEFAULT: f64 = 2.5;
pub const SCOPE_CHANNELS: usize = 4;
pub const LA_CHANNELS: usize = 8;
const RING: usize = 1048576;
const DISPLAY_N: usize = 512;

pub const SCOPE_COLORS: [&str; SCOPE_CHANNELS] = ["#ffff00", "#00ff00", "#00ffff", "#ff00ff"];
pub const LA_COLORS: [&str; LA_CHANNELS] = [
    "#e74c3c", "#e67e22", "#f1c40f", "#2ecc71", "#1abc9c", "#3498db", "#9b59b6", "#ecf0f1",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoScaleResult {
    pub volt_div: f64,
    pub volt_pos: f64,
    pub time_div: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AutoScaleAllResult {
    pub channels: Vec<Option<AutoScaleResult>>,
    pub time_div: Option<f64>,
}

pub const SCOPE_VOLT_DIV_STEPS: [f64; 33] = [
    1e-4, 2e-4, 5e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2, 5e-2, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0,
    50.0, 100.0, 200.0, 500.0, 1e3, 2e3, 5e3, 1e4, 2e4, 5e4, 1e5, 2e5, 5e5, 1e6, 2e6, 5e6,
];

pub const SCOPE_TIME_DIV_STEPS: [f64; 34] = [
    1e-9, 2e-9, 5e-9, 1e-8, 2e-8, 5e-8, 1e-7, 2e-7, 5e-7, 1e-6, 2e-6, 5e-6, 1e-5, 2e-5, 5e-5, 1e-4,
    2e-4, 5e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2, 5e-2, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0,
    100.0,
];

pub fn snap_to_step(val: f64, steps: &[f64]) -> f64 {
    if steps.is_empty() {
        return val;
    }
    for &s in steps {
        if s >= val * 0.999 {
            return s;
        }
    }
    *steps.last().unwrap()
}

pub fn snap_closest_step(val: f64, steps: &[f64]) -> f64 {
    if steps.is_empty() {
        return val;
    }
    let mut best = steps[0];
    let mut min_diff = f64::INFINITY;
    let log_val = val.max(1e-15).ln();
    for &s in steps {
        let diff = (s.max(1e-15).ln() - log_val).abs();
        if diff < min_diff {
            min_diff = diff;
            best = s;
        }
    }
    best
}

#[derive(Clone, Debug, Default)]
pub struct MeterStats {
    pub value: f64,
    pub max: f64,
    pub sum: f64,
    pub n: u64,
    pub sum_sq: f64,
    pub n_sq: u64,
    pub freq: f64,
    pub last: f64,
    pub last_cross_t: f64,
    pub had_cross: bool,
}

impl MeterStats {
    pub fn sample(&mut self, x: f64, rms: bool, t: f64) {
        let stat = if rms { x.abs() } else { x };
        if self.n == 0 || stat > self.max {
            self.max = stat;
        }
        self.sum += stat;
        self.n += 1;
        if rms {
            self.sum_sq += x * x;
            self.n_sq += 1;
            if self.last <= 0.0 && x > 0.0 {
                if self.had_cross {
                    let period = t - self.last_cross_t;
                    if period > 0.0 {
                        self.freq = 1.0 / period;
                    }
                }
                self.last_cross_t = t;
                self.had_cross = true;
            } else if self.had_cross && self.freq > 0.0 {
                let last_period = 1.0 / self.freq;
                if t - self.last_cross_t > last_period * 3.0 {
                    self.freq = 0.0;
                    self.had_cross = false;
                }
            }
        }
        self.last = x;
        self.value = x;
    }

    pub fn avg(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.sum / self.n as f64
        }
    }

    pub fn rms(&self) -> f64 {
        if self.n_sq == 0 {
            0.0
        } else {
            (self.sum_sq / self.n_sq as f64).sqrt()
        }
    }

    pub fn display_value(&self, rms: bool) -> f64 {
        if rms { self.rms() } else { self.value }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FreqMeterState {
    pub freq: f64,
    pub last: f64,
    pub rising: bool,
    pub falling: bool,
    pub num_max: u32,
    pub last_max: f64,
    pub total_p: f64,
    pub period: f64,
}

impl FreqMeterState {
    pub fn sample(&mut self, data: f64, filter: f64, t: f64) {
        let delta = data - self.last;
        if delta > filter {
            if self.falling && !self.rising {
                self.falling = false;
            }
            self.rising = true;
            self.last = data;
        } else if delta < -filter {
            if self.rising && !self.falling {
                if self.num_max > 0 {
                    self.period = t - self.last_max;
                    self.total_p += self.period;
                }
                self.last_max = t;
                self.num_max += 1;
                self.rising = false;
            }
            self.falling = true;
            self.last = data;
        }
        if self.num_max > 1 && self.total_p > 0.0 {
            let avg = self.total_p / (self.num_max as f64 - 1.0);
            if avg > 0.0 {
                self.freq = 1.0 / avg;
            }
            if self.num_max > 8 {
                self.total_p = 0.0;
                self.num_max = 1;
                self.last_max = t;
            }
        }
        if self.period > 0.0 && t - self.last_max > self.period * 4.0 {
            self.freq = 0.0;
            self.period = 0.0;
            self.total_p = 0.0;
            self.num_max = 0;
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScopeSampler {
    t: Vec<f64>,
    ch: Vec<Vec<f64>>,
    i: usize,
    n: usize,
    cap: usize,
    nch: usize,
    trig_t: f64,
    prev: Vec<f64>,
    high: Vec<bool>,
    pub connected: Vec<bool>,
    pub freq_meters: Vec<FreqMeterState>,
}

impl ScopeSampler {
    pub fn scope() -> Self {
        Self::with_channels(SCOPE_CHANNELS)
    }

    pub fn analyzer() -> Self {
        Self::with_channels(LA_CHANNELS)
    }

    fn with_channels(nch: usize) -> Self {
        Self {
            t: vec![0.0; RING],
            ch: vec![vec![0.0; RING]; nch],
            i: 0,
            n: 0,
            cap: RING,
            nch,
            trig_t: 0.0,
            prev: vec![0.0; nch],
            high: vec![false; nch],
            connected: vec![false; nch],
            freq_meters: vec![FreqMeterState::default(); nch],
        }
    }

    fn record_raw(
        &mut self,
        t: f64,
        values: &[f64],
        connected: &[bool],
        trigger: i32,
        level: f64,
        rising: bool,
        filter: f64,
    ) {
        let idx = self.i;
        self.t[idx] = t;
        for c in 0..self.nch {
            let is_conn = connected.get(c).copied().unwrap_or(true);
            self.connected[c] = is_conn;
            let v = if is_conn {
                values.get(c).copied().unwrap_or(0.0)
            } else {
                0.0
            };
            self.ch[c][idx] = v;
            if is_conn {
                self.freq_meters[c].sample(v, filter, t);
            } else {
                self.freq_meters[c] = FreqMeterState::default();
            }
            if trigger == c as i32 && is_conn {
                let prev = self.prev[c];
                let crossed = if rising {
                    prev < level && v >= level
                } else {
                    prev > level && v <= level
                };
                if crossed {
                    self.trig_t = t;
                }
            }
            self.prev[c] = v;
        }
        self.i = (self.i + 1) % self.cap;
        if self.n < self.cap {
            self.n += 1;
        }
    }

    pub fn push(
        &mut self,
        t: f64,
        values: &[f64],
        connected: &[bool],
        trigger: i32,
        level: f64,
        rising: bool,
        filter: f64,
    ) {
        let mut changed = self.n == 0;
        if !changed {
            let last_idx = if self.i == 0 {
                self.cap - 1
            } else {
                self.i - 1
            };
            for c in 0..self.nch {
                let is_conn = connected.get(c).copied().unwrap_or(true);
                let v = if is_conn {
                    values.get(c).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                if (self.ch[c][last_idx] - v).abs() > 1e-6 || (self.connected[c] != is_conn) {
                    changed = true;
                    break;
                }
            }
        }

        if changed {
            if self.n > 0 {
                let last_idx = if self.i == 0 {
                    self.cap - 1
                } else {
                    self.i - 1
                };
                self.t[last_idx] = self.t[last_idx].max(t);
            }
            self.record_raw(t, values, connected, trigger, level, rising, filter);
        } else {
            // Signal has not changed. If the last recorded sample is a hold sample, update its timestamp to t.
            // Otherwise, record a hold sample at t.
            let last_idx = if self.i == 0 {
                self.cap - 1
            } else {
                self.i - 1
            };
            let prev_idx = if last_idx == 0 {
                self.cap - 1
            } else {
                last_idx - 1
            };

            let is_hold_sample = self.n >= 2 && {
                let mut same_as_prev = true;
                for c in 0..self.nch {
                    if (self.ch[c][last_idx] - self.ch[c][prev_idx]).abs() > 1e-6 {
                        same_as_prev = false;
                        break;
                    }
                }
                same_as_prev
            };

            if is_hold_sample {
                self.t[last_idx] = t;
            } else {
                self.record_raw(t, values, connected, trigger, level, rising, filter);
            }

            // Update frequency meters
            for c in 0..self.nch {
                let is_conn = connected.get(c).copied().unwrap_or(true);
                if is_conn {
                    let v = values.get(c).copied().unwrap_or(0.0);
                    self.freq_meters[c].sample(v, filter, t);
                }
            }
        }
    }

    pub fn push_digital(
        &mut self,
        t: f64,
        values: &[f64],
        connected: &[bool],
        thr_r: f64,
        thr_f: f64,
        trigger: i32,
    ) {
        let mut bits = vec![0.0; self.nch];
        for c in 0..self.nch {
            let v = values.get(c).copied().unwrap_or(0.0);
            if v >= thr_r {
                self.high[c] = true;
            } else if v <= thr_f {
                self.high[c] = false;
            }
            bits[c] = if self.high[c] { 1.0 } else { 0.0 };
        }
        self.push(t, &bits, connected, trigger, 0.5, true, 0.1);
    }

    pub fn channel_freqs(&self) -> Vec<String> {
        (0..self.nch)
            .map(|c| {
                if self.connected.get(c).copied().unwrap_or(false) {
                    let f = self.freq_meters.get(c).map(|m| m.freq).unwrap_or(0.0);
                    if f >= 1.0 {
                        crate::units::format_si_compact(f, "Hz")
                    } else {
                        "0 Hz".to_string()
                    }
                } else {
                    "0 Hz".to_string()
                }
            })
            .collect()
    }

    pub fn channel_connected(&self) -> Vec<bool> {
        self.connected.clone()
    }

    pub fn last_values(&self) -> Vec<f64> {
        if self.n == 0 {
            return vec![0.0; self.nch];
        }
        let idx = if self.i == 0 {
            self.cap - 1
        } else {
            self.i - 1
        };
        (0..self.nch).map(|c| self.ch[c][idx]).collect()
    }

    fn last_time(&self) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        let idx = if self.i == 0 {
            self.cap - 1
        } else {
            self.i - 1
        };
        self.t[idx]
    }

    pub fn sample_at(&self, t: f64) -> Vec<f64> {
        if self.n == 0 {
            return vec![0.0; self.nch];
        }
        if self.n == 1 {
            let idx = if self.i == 0 {
                self.cap - 1
            } else {
                self.i - 1
            };
            return (0..self.nch).map(|c| self.ch[c][idx]).collect();
        }
        let start = if self.n < self.cap { 0 } else { self.i };
        let t_first = self.t[start];
        let t_last = self.t[(start + self.n - 1) % self.cap];

        if t <= t_first {
            return (0..self.nch).map(|c| self.ch[c][start]).collect();
        }
        if t >= t_last {
            let last_idx = (start + self.n - 1) % self.cap;
            return (0..self.nch).map(|c| self.ch[c][last_idx]).collect();
        }

        // Binary search for the first sample with timestamp >= t
        let mut low = 0usize;
        let mut high = self.n - 1;
        while low < high {
            let mid = (low + high) / 2;
            let mid_idx = (start + mid) % self.cap;
            if self.t[mid_idx] < t {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        let k1 = low;
        if k1 == 0 {
            return (0..self.nch).map(|c| self.ch[c][start]).collect();
        }
        // Zero-order hold: the last sample at or before `t` is held until the
        // next recorded sample. The ring only stores value *changes* (plus a
        // hold timestamp), so linear interpolation of those points turns a
        // square wave into a ramp — the same artifact C++ PlotDisplay avoids
        // by inserting a corner point before each new sample.
        let idx1 = (start + k1) % self.cap;
        if self.t[idx1] <= t {
            return (0..self.nch).map(|c| self.ch[c][idx1]).collect();
        }
        let idx0 = (start + k1 - 1) % self.cap;
        (0..self.nch).map(|c| self.ch[c][idx0]).collect()
    }

    fn last_crossing(&self, ch: usize, level: f64) -> Option<f64> {
        if self.n < 2 || ch >= self.nch {
            return None;
        }
        let start = if self.n < self.cap { 0 } else { self.i };

        // Determine channel signal range across recent samples (up to 20,000 samples)
        let search_n = self.n.min(20_000);
        let offset = self.n - search_n;
        let mut min_v = f64::INFINITY;
        let mut max_v = f64::NEG_INFINITY;
        for k in offset..self.n {
            let idx = (start + k) % self.cap;
            let v = self.ch[ch][idx];
            if v < min_v {
                min_v = v;
            }
            if v > max_v {
                max_v = v;
            }
        }
        let span = max_v - min_v;
        if span < 1e-6 {
            return None;
        }

        // If specified trigger level is not within the signal swing, use the signal midpoint.
        let eff_level = if level > min_v && level < max_v {
            level
        } else {
            (min_v + max_v) * 0.5
        };

        // Search backwards from the latest sample to find the last rising edge immediately
        for k in (offset.max(1)..self.n).rev() {
            let idx = (start + k) % self.cap;
            let prev_idx = (start + k - 1) % self.cap;
            let cur = self.ch[ch][idx];
            let prev = self.ch[ch][prev_idx];
            if prev < eff_level && cur >= eff_level {
                return Some(self.t[idx]);
            }
        }
        None
    }

    /// Window of `time_div * 10` seconds, 512 display samples. Trigger channel
    /// (if 0..nch-1 and an edge was seen) centres the window on that edge.
    /// `time_pos` shifts the view window back/forward in simulation history.
    pub fn traces(
        &self,
        time_div: f64,
        time_pos: f64,
        trigger: i32,
        trig_level: f64,
        colors: &[&str],
        digital_thr: Option<f64>,
    ) -> PlotBuffer {
        self.traces_with_count(
            time_div,
            time_pos,
            trigger,
            trig_level,
            colors,
            digital_thr,
            DISPLAY_N,
        )
    }

    /// Window of `time_div * 10` seconds, `n_display` samples.
    pub fn traces_with_count(
        &self,
        time_div: f64,
        time_pos: f64,
        trigger: i32,
        trig_level: f64,
        colors: &[&str],
        digital_thr: Option<f64>,
        n_display: usize,
    ) -> PlotBuffer {
        let n = n_display.max(2);
        let window = (time_div.max(1e-12) * 10.0).max(1e-12);
        let t_last = self.last_time();
        let t_base = if trigger >= 0 && (trigger as usize) < self.nch {
            self.last_crossing(trigger as usize, trig_level)
                .map(|t| (t + window * 0.5).min(t_last))
                .unwrap_or(t_last)
        } else {
            t_last
        };
        // time_pos shifts the window back/forward in time without buffer loss
        let t_end = t_base - time_pos;
        let t_start = t_end - window;

        let mut channels = Vec::with_capacity(self.nch);
        for c in 0..self.nch {
            let color = colors.get(c).copied().unwrap_or("#ffffff").to_string();
            let is_conn = self.connected.get(c).copied().unwrap_or(false);
            if !is_conn {
                channels.push(Channel {
                    color,
                    samples: Vec::new(),
                    connected: false,
                });
            } else {
                channels.push(Channel {
                    color,
                    samples: Vec::with_capacity(n),
                    connected: true,
                });
            }
        }

        if self.n == 0 {
            for ch in &mut channels {
                if ch.connected {
                    ch.samples.resize(n, 0.0);
                }
            }
            return PlotBuffer { channels, n };
        }

        let start = if self.n < self.cap { 0 } else { self.i };
        let mut k = if self.n > 1 && t_start > self.t[start] {
            let mut low = 0usize;
            let mut high = self.n - 1;
            while low < high {
                let mid = (low + high + 1) / 2;
                let mid_idx = (start + mid) % self.cap;
                if self.t[mid_idx] <= t_start {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            low
        } else {
            0
        };

        for i in 0..n {
            let t = t_start + window * (i as f64) / ((n - 1) as f64);
            let sample_idx = if self.n == 1 || t <= self.t[start] {
                start
            } else {
                let last_k = self.n - 1;
                let last_idx = (start + last_k) % self.cap;
                if t >= self.t[last_idx] {
                    last_idx
                } else {
                    while k + 1 < self.n {
                        let next_idx = (start + k + 1) % self.cap;
                        if self.t[next_idx] <= t {
                            k += 1;
                        } else {
                            break;
                        }
                    }
                    (start + k) % self.cap
                }
            };

            for c in 0..self.nch {
                if channels[c].connected {
                    let mut val = self.ch[c][sample_idx];
                    if let Some(thr) = digital_thr {
                        val = if val >= thr { 1.0 } else { 0.0 };
                    }
                    channels[c].samples.push(val);
                }
            }
        }

        PlotBuffer { channels, n }
    }

    /// Automatically measures the channel signal and calculates the ideal Volt/Div,
    /// Volt Pos (vertical offset), and Time/Div to fit the waveform on screen.
    pub fn auto_scale(&self, ch: usize) -> Option<AutoScaleResult> {
        if ch >= self.nch || self.n < 2 || !self.connected.get(ch).copied().unwrap_or(false) {
            return None;
        }
        let start = if self.n < self.cap { 0 } else { self.i };
        let mut min_v = f64::INFINITY;
        let mut max_v = f64::NEG_INFINITY;
        for k in 0..self.n {
            let idx = (start + k) % self.cap;
            let v = self.ch[ch][idx];
            if v < min_v {
                min_v = v;
            }
            if v > max_v {
                max_v = v;
            }
        }
        let ampli = max_v - min_v;

        // If DC signal (no significant AC swing)
        if ampli <= 1e-6 {
            let volt_pos = max_v;
            let raw_volt_div = if max_v.abs() > 1e-6 {
                max_v.abs() / 4.0
            } else {
                1.0
            };
            let volt_div = snap_to_step(raw_volt_div, &SCOPE_VOLT_DIV_STEPS);
            return Some(AutoScaleResult {
                volt_div,
                volt_pos,
                time_div: None,
            });
        }

        let mid = min_v + ampli * 0.5;
        let volt_pos = mid;
        let raw_volt_div = ampli / 8.0;
        let volt_div = snap_to_step(raw_volt_div, &SCOPE_VOLT_DIV_STEPS);

        // Period measurement using hysteresis around mid
        let hyst = (ampli * 0.1).max(1e-6);
        let mut below = false;
        let mut last_rise = 0.0;
        let mut total_period = 0.0;
        let mut n_edges = 0;

        for k in 0..self.n {
            let idx = (start + k) % self.cap;
            let v = self.ch[ch][idx];
            let t = self.t[idx];
            if v > mid + hyst {
                if below {
                    if last_rise > 0.0 {
                        let dt = t - last_rise;
                        if dt > 1e-12 {
                            total_period += dt;
                            n_edges += 1;
                        }
                    }
                    last_rise = t;
                    below = false;
                }
            } else if v < mid - hyst {
                below = true;
            }
        }

        let time_div = if n_edges > 0 {
            let avg_period = total_period / n_edges as f64;
            if avg_period > 1e-12 {
                // Show ~2.5 full cycles across 10 horizontal screen divisions (time_div = avg_period / 4.0)
                let raw_time_div = avg_period / 4.0;
                Some(snap_closest_step(raw_time_div, &SCOPE_TIME_DIV_STEPS))
            } else {
                None
            }
        } else {
            // Fallback: check real-time freq meter if available
            let f = self.freq_meters.get(ch).map(|m| m.freq).unwrap_or(0.0);
            if f > 1e-6 {
                let period = 1.0 / f;
                let raw_time_div = period / 4.0;
                Some(snap_closest_step(raw_time_div, &SCOPE_TIME_DIV_STEPS))
            } else {
                None
            }
        };

        Some(AutoScaleResult {
            volt_div,
            volt_pos,
            time_div,
        })
    }

    /// Automatically measures all connected channels and calculates per-channel
    /// Volt/Div, Volt Pos, and the optimal global Time/Div based on the lowest
    /// detected frequency (longest period).
    pub fn auto_scale_all(&self) -> AutoScaleAllResult {
        let mut results = Vec::with_capacity(self.nch);
        let mut max_period: Option<f64> = None;

        for ch in 0..self.nch {
            if let Some(res) = self.auto_scale(ch) {
                if let Some(td) = res.time_div {
                    let est_period = td * 4.0;
                    match max_period {
                        Some(cur_max) => {
                            if est_period > cur_max {
                                max_period = Some(est_period);
                            }
                        }
                        None => {
                            max_period = Some(est_period);
                        }
                    }
                }
                results.push(Some(res));
            } else {
                results.push(None);
            }
        }

        let time_div = max_period.map(|p| snap_closest_step(p / 4.0, &SCOPE_TIME_DIV_STEPS));

        AutoScaleAllResult {
            channels: results,
            time_div,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InstrumentState {
    pub meters: FxHashMap<String, MeterStats>,
    pub freq: FxHashMap<String, FreqMeterState>,
    pub scopes: FxHashMap<String, ScopeSampler>,
    pub las: FxHashMap<String, ScopeSampler>,
}

impl InstrumentState {
    pub fn clear(&mut self) {
        self.meters.clear();
        self.freq.clear();
        self.scopes.clear();
        self.las.clear();
    }

    pub fn first_scope(&self) -> Option<&ScopeSampler> {
        self.scopes.values().next()
    }

    pub fn first_la(&self) -> Option<&ScopeSampler> {
        self.las.values().next()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReadingView {
    pub text: String,
    pub max_text: String,
    pub avg_text: String,
    pub extra: String,
    pub high: bool,
    pub low: bool,
    pub hz_text: String,
}

pub fn format_reading(value: f64, unit: &str) -> String {
    if value.abs() < 1e-12 {
        return format!("0{unit}");
    }
    crate::units::format_si_compact_precision(value, unit, 2)
}

pub fn format_hz(freq: f64) -> String {
    if freq <= 0.0 {
        return "--Hz".into();
    }
    crate::units::format_si_compact_precision(freq, "Hz", 2)
}

pub fn probe_reading(volt: f64, threshold: f64) -> ReadingView {
    let high = volt > threshold;
    let low = volt < -threshold;
    ReadingView {
        text: format_reading(volt, "V"),
        max_text: String::new(),
        avg_text: String::new(),
        extra: String::new(),
        high,
        low,
        hz_text: String::new(),
    }
}

pub fn meter_reading(stats: &MeterStats, rms: bool, unit: &str) -> ReadingView {
    let shown = stats.display_value(rms);
    let text = format_reading(shown, unit);
    let hz_text = if rms {
        format_hz(stats.freq)
    } else {
        String::new()
    };
    let max_val = format_reading(stats.max, unit);
    let avg_val = format_reading(stats.avg(), unit);
    let max_text = format!("MAX {max_val}");
    let avg_text = format!("AVG {avg_val}");
    let extra = if rms {
        format!(
            "MAX {}\nAVG {}\nRMS {}\n{}",
            format_reading(stats.max, unit),
            format_reading(stats.avg(), unit),
            format_reading(stats.rms(), unit),
            format_hz(stats.freq)
        )
    } else {
        format!(
            "MAX {}\nAVG {}",
            format_reading(stats.max, unit),
            format_reading(stats.avg(), unit)
        )
    };
    ReadingView {
        text,
        max_text,
        avg_text,
        extra,
        high: false,
        low: false,
        hz_text,
    }
}

pub fn freq_reading(state: &FreqMeterState) -> ReadingView {
    ReadingView {
        text: format_hz(state.freq),
        max_text: String::new(),
        avg_text: String::new(),
        extra: String::new(),
        high: false,
        low: false,
        hz_text: String::new(),
    }
}

pub fn empty_scope_traces() -> PlotBuffer {
    PlotBuffer {
        channels: SCOPE_COLORS
            .iter()
            .map(|c| Channel {
                color: (*c).into(),
                samples: vec![],
                connected: false,
            })
            .collect(),
        n: 0,
    }
}

pub fn empty_la_traces() -> PlotBuffer {
    PlotBuffer {
        channels: LA_COLORS
            .iter()
            .map(|c| Channel {
                color: (*c).into(),
                samples: vec![],
                connected: false,
            })
            .collect(),
        n: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_avg_and_rms() {
        let mut s = MeterStats::default();
        s.sample(3.0, true, 0.0);
        s.sample(-3.0, true, 0.001);
        s.sample(3.0, true, 0.002);
        assert!((s.avg() - 3.0).abs() < 1e-12);
        assert!((s.rms() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn scope_trigger_centres_edge() {
        let mut s = ScopeSampler::scope();
        for i in 0..200 {
            let t = i as f64 * 1e-6;
            let v = if i < 100 { 0.0 } else { 5.0 };
            s.push(
                t,
                &[v, 0.0, 0.0, 0.0],
                &[true, false, false, false],
                0,
                2.5,
                true,
                0.1,
            );
        }
        // 50 µs window centred on the 100 µs edge.
        let buf = s.traces(5e-6, 0.0, 0, 2.5, &SCOPE_COLORS, None);
        assert_eq!(buf.channels.len(), 4);
        assert_eq!(buf.channels[0].samples.len(), DISPLAY_N);
        assert!(buf.channels[0].connected);
        assert!(!buf.channels[1].connected);
        let mid = buf.channels[0].samples[DISPLAY_N / 2];
        assert!(mid > 2.0, "trigger centre {mid}");

        // Negative trigger (-1) indicates free-running mode without centering
        let buf_free = s.traces(5e-6, 0.0, -1, 2.5, &SCOPE_COLORS, None);
        assert_eq!(buf_free.channels.len(), 4);
        assert_eq!(buf_free.channels[0].samples.len(), DISPLAY_N);

        // Historical time_pos panning test: go back 150 µs into history
        let buf_history = s.traces(5e-6, 150e-6, -1, 2.5, &SCOPE_COLORS, None);
        assert_eq!(buf_history.channels.len(), 4);
        assert_eq!(buf_history.channels[0].samples.len(), DISPLAY_N);
        // At t = 200µs - 150µs = 50µs, v was 0.0
        assert_eq!(buf_history.channels[0].samples[DISPLAY_N / 2], 0.0);
    }

    #[test]
    fn scope_auto_scale_sine_wave() {
        let mut s = ScopeSampler::scope();
        let freq = 1000.0; // 1 kHz -> 1 ms period
        for i in 0..1000 {
            let t = i as f64 * 10e-6; // 10 µs step -> 10 ms total (10 cycles)
            let v = 2.5 + 2.0 * (2.0 * std::f64::consts::PI * freq * t).sin(); // 0.5V .. 4.5V swing (4Vpp), mid = 2.5V
            s.push(
                t,
                &[v, 0.0, 0.0, 0.0],
                &[true, false, false, false],
                0,
                2.5,
                true,
                0.1,
            );
        }

        let res = s
            .auto_scale(0)
            .expect("auto scale should succeed on active sine wave");
        assert!(
            (res.volt_pos - 2.5).abs() < 0.1,
            "volt_pos should center near 2.5V, got {}",
            res.volt_pos
        );
        assert!(
            res.volt_div >= 0.5 && res.volt_div <= 1.0,
            "volt_div should be reasonable for 4Vpp, got {}",
            res.volt_div
        );
        let td = res.time_div.expect("periodic wave should have time_div");
        assert!(
            td >= 1e-4 && td <= 1e-3,
            "time_div should fit ~1ms period, got {}",
            td
        );

        // Unconnected channel should return None
        assert_eq!(s.auto_scale(1), None);
    }

    #[test]
    fn scope_square_hold_not_ramp() {
        let mut s = ScopeSampler::scope();
        // Sparse 1 kHz square: two updates per period, the same recording
        // pattern analog change-detect uses. Linear interpolation of those
        // points is a triangle; a scope must hold.
        for cycle in 0..20 {
            let t0 = cycle as f64 * 1e-3;
            s.push(
                t0,
                &[5.0, 0.0, 0.0, 0.0],
                &[true, false, false, false],
                -1,
                0.0,
                true,
                0.1,
            );
            s.push(
                t0 + 0.5e-3,
                &[0.0, 0.0, 0.0, 0.0],
                &[true, false, false, false],
                -1,
                0.0,
                true,
                0.1,
            );
        }
        let buf = s.traces(1e-3, 0.0, -1, 2.5, &SCOPE_COLORS, None);
        let ch = &buf.channels[0].samples;
        let high = ch.iter().filter(|&&v| v > 4.5).count();
        let low = ch.iter().filter(|&&v| v < 0.5).count();
        let mid = ch.iter().filter(|&&v| v > 1.0 && v < 4.0).count();
        assert!(high > 80, "high rail samples {high}");
        assert!(low > 80, "low rail samples {low}");
        assert!(
            mid < 20,
            "square display must not ramp through mid-scale, got {mid}"
        );
    }
}
