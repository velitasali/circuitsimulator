//! Polyline approximation of a circular arc. Shared by the software raster,
//! SVG export, and the Vello recorder.

pub(crate) fn arc_pts(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> Vec<[f64; 2]> {
    let sweep = a1 - a0;
    let n = ((sweep.abs() / std::f64::consts::PI) * 16.0)
        .ceil()
        .max(4.0) as usize;
    (0..=n)
        .map(|i| {
            let a = a0 + sweep * (i as f64 / n as f64);
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}
