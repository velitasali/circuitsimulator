//! Oscilloscope spectrum and dual-cursor measurements.
//!
//! The FFT is a radix-2 transform of the samples, zero-padded up to the next
//! power of two. One-sided bins are peak amplitudes: a sine of amplitude 1
//! that lands on a bin reads back as magnitude 1.

/// One bin of a one-sided magnitude spectrum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectrumBin {
    pub hz: f64,
    pub mag: f64,
}

/// Readings between two sample indices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorMeasure {
    pub dt: f64,
    pub freq: f64,
    pub dv: f64,
    pub v1: f64,
    pub v2: f64,
    pub vrms: f64,
    pub vpp: f64,
}

/// One-sided magnitude spectrum. `sample_hz` is the sample rate.
/// Returns an empty list when there are fewer than two samples.
pub fn fft_mag(samples: &[f64], sample_hz: f64) -> Vec<SpectrumBin> {
    if samples.len() < 2 || !sample_hz.is_finite() || sample_hz <= 0.0 {
        return Vec::new();
    }
    let n = samples.len().next_power_of_two();
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    re[..samples.len()].copy_from_slice(samples);
    fft(&mut re, &mut im);
    let mut out = Vec::with_capacity(n / 2 + 1);
    let scale = n as f64;
    for k in 0..=(n / 2) {
        let mag = re[k].hypot(im[k]) / scale;
        let mag = if k == 0 || k == n / 2 { mag } else { mag * 2.0 };
        out.push(SpectrumBin {
            hz: sample_hz * k as f64 / scale,
            mag,
        });
    }
    out
}

/// The bin with the largest magnitude above DC. DC is the answer when the
/// rest of the spectrum is empty.
pub fn peak_bin(bins: &[SpectrumBin]) -> Option<SpectrumBin> {
    let rest = bins.get(1..).unwrap_or(&[]);
    let best = rest.iter().max_by(|a, b| {
        a.mag
            .partial_cmp(&b.mag)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    best.or_else(|| bins.first()).copied()
}

/// `v1` is `samples[i0]` and `v2` is `samples[i1]`. `dt` is signed.
/// RMS and peak-to-peak cover the inclusive span.
pub fn measure(samples: &[f64], sample_dt: f64, i0: usize, i1: usize) -> Option<CursorMeasure> {
    if samples.is_empty() || !sample_dt.is_finite() || sample_dt < 0.0 {
        return None;
    }
    if i0 >= samples.len() || i1 >= samples.len() {
        return None;
    }
    let v1 = samples[i0];
    let v2 = samples[i1];
    let dt = (i1 as f64 - i0 as f64) * sample_dt;
    let freq = if dt.abs() > 0.0 { 1.0 / dt.abs() } else { 0.0 };
    let lo = i0.min(i1);
    let hi = i0.max(i1);
    let window = &samples[lo..=hi];
    let mean_sq = window.iter().map(|v| v * v).sum::<f64>() / window.len() as f64;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for v in window {
        min_v = min_v.min(*v);
        max_v = max_v.max(*v);
    }
    Some(CursorMeasure {
        dt,
        freq,
        dv: v2 - v1,
        v1,
        v2,
        vrms: mean_sq.sqrt(),
        vpp: max_v - min_v,
    })
}

fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -std::f64::consts::TAU / len as f64;
        let wlen_re = ang.cos();
        let wlen_im = ang.sin();
        let mut i = 0usize;
        while i < n {
            let mut w_re = 1.0;
            let mut w_im = 0.0;
            for k in 0..(len / 2) {
                let u_re = re[i + k];
                let u_im = im[i + k];
                let v_re = re[i + k + len / 2] * w_re - im[i + k + len / 2] * w_im;
                let v_im = re[i + k + len / 2] * w_im + im[i + k + len / 2] * w_re;
                re[i + k] = u_re + v_re;
                im[i + k] = u_im + v_im;
                re[i + k + len / 2] = u_re - v_re;
                im[i + k + len / 2] = u_im - v_im;
                let next_re = w_re * wlen_re - w_im * wlen_im;
                w_im = w_re * wlen_im + w_im * wlen_re;
                w_re = next_re;
            }
            i += len;
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_lands_on_its_bin() {
        let n = 64usize;
        let k = 3usize;
        let samples: Vec<f64> = (0..n)
            .map(|i| (std::f64::consts::TAU * k as f64 * i as f64 / n as f64).sin())
            .collect();
        let bins = fft_mag(&samples, n as f64);
        let peak = peak_bin(&bins).unwrap();
        assert!((peak.hz - k as f64).abs() < 1e-9, "hz {}", peak.hz);
        assert!((peak.mag - 1.0).abs() < 1e-6, "mag {}", peak.mag);
        let other: f64 = bins
            .iter()
            .filter(|b| (b.hz - peak.hz).abs() > 0.5)
            .map(|b| b.mag)
            .fold(0.0, f64::max);
        assert!(other < 1e-6, "leak {other}");
    }

    #[test]
    fn cursors_report_delta_and_rms() {
        let samples = [0.0, 1.0, 0.0, -1.0];
        let m = measure(&samples, 0.25, 1, 3).unwrap();
        assert!((m.dt - 0.5).abs() < 1e-12);
        assert!((m.freq - 2.0).abs() < 1e-12);
        assert!((m.dv - (-2.0)).abs() < 1e-12);
        assert!((m.v1 - 1.0).abs() < 1e-12);
        assert!((m.v2 - (-1.0)).abs() < 1e-12);
        assert!((m.vpp - 2.0).abs() < 1e-12);
        assert!((m.vrms - (2.0_f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!(measure(&samples, 0.25, 0, 9).is_none());
    }
}
