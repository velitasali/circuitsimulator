//! Plot sample buffers. `PlotDisplay` was a `QQuickPaintedItem`; QML Canvas
//! draws these traces instead.

#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub color: String,
    pub samples: Vec<f64>,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotBuffer {
    pub channels: Vec<Channel>,
    pub n: usize,
}

impl PlotBuffer {
    /// Four analogue traces, as the scope's default demo.
    pub fn scope_demo(n: usize) -> Self {
        let n = n.max(2);
        let colors = ["#ffff00", "#00ff00", "#00ffff", "#ff00ff"];
        let mut channels = Vec::with_capacity(4);
        for (ch, color) in colors.iter().enumerate() {
            let mut samples = Vec::with_capacity(n);
            for i in 0..n {
                let t = i as f64 / (n - 1) as f64;
                let v = match ch {
                    0 => (t * std::f64::consts::TAU * 2.0).sin(),
                    1 => {
                        let x = (t * 4.0).fract();
                        if x < 0.5 { 1.0 } else { -1.0 }
                    }
                    2 => 2.0 * (t * 2.0).fract() - 1.0,
                    _ => 1.0 - 4.0 * (t - 0.5).abs(),
                };
                samples.push(v);
            }
            channels.push(Channel {
                color: (*color).into(),
                samples,
                connected: true,
            });
        }
        Self { channels, n }
    }

    /// Eight digital traces for the logic analyzer.
    pub fn analyzer_demo(n: usize) -> Self {
        let n = n.max(2);
        let colors = [
            "#e74c3c", "#e67e22", "#f1c40f", "#2ecc71", "#1abc9c", "#3498db", "#9b59b6", "#ecf0f1",
        ];
        let mut channels = Vec::with_capacity(8);
        for (ch, color) in colors.iter().enumerate() {
            let mut samples = Vec::with_capacity(n);
            let period = 8 << (ch.min(4));
            for i in 0..n {
                let bit = ((i / period) % 2) as f64;
                samples.push(bit);
            }
            channels.push(Channel {
                color: (*color).into(),
                samples,
                connected: true,
            });
        }
        Self { channels, n }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_has_four_channels() {
        let p = PlotBuffer::scope_demo(64);
        assert_eq!(p.channels.len(), 4);
        assert_eq!(p.channels[0].samples.len(), 64);
        assert!(p.channels[0].samples.iter().any(|v| *v > 0.5));
        assert!(p.channels[0].samples.iter().any(|v| *v < -0.5));
    }

    #[test]
    fn analyzer_is_digital() {
        let p = PlotBuffer::analyzer_demo(32);
        assert_eq!(p.channels.len(), 8);
        for ch in &p.channels {
            assert!(ch.samples.iter().all(|v| *v == 0.0 || *v == 1.0));
        }
    }
}
