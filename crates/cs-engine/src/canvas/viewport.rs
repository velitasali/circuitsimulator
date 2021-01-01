//! Viewport transform, pan, zoom. Matches `CircuitCanvas` / `CircuitViewport`.

use super::geom::{Point, Rect};

pub const MIN_ZOOM: f64 = 0.5;
pub const MAX_ZOOM: f64 = 50.0;
pub const ZOOM_STEP: f64 = 1.2;
/// C++ default `CircuitDefaults::canvasWidth`.
pub const DEFAULT_SCENE_WIDTH: f64 = 3200.0;
/// C++ default `CircuitDefaults::canvasHeight`.
pub const DEFAULT_SCENE_HEIGHT: f64 = 2400.0;

#[derive(Clone, Debug)]
pub struct Viewport {
    zoom: f64,
    center: Point,
    view_w: f64,
    view_h: f64,
    scene: Rect,
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(DEFAULT_SCENE_WIDTH, DEFAULT_SCENE_HEIGHT)
    }
}

impl Viewport {
    pub fn new(scene_w: f64, scene_h: f64) -> Self {
        Self {
            zoom: 1.0,
            center: Point::zero(),
            view_w: 1200.0,
            view_h: 800.0,
            scene: Rect::new(-scene_w * 0.5, -scene_h * 0.5, scene_w, scene_h),
        }
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    pub fn center(&self) -> Point {
        self.center
    }

    pub fn view_size(&self) -> (f64, f64) {
        (self.view_w, self.view_h)
    }

    pub fn scene_rect(&self) -> Rect {
        self.scene
    }

    /// C++ `Circuit::setSize`: scene centred on origin.
    pub fn set_scene_size(&mut self, w: f64, h: f64) -> bool {
        let w = w.max(1.0);
        let h = h.max(1.0);
        let scene = Rect::new(-w * 0.5, -h * 0.5, w, h);
        if self.scene == scene {
            return false;
        }
        self.scene = scene;
        self.clamp_center();
        true
    }

    /// Scene rectangle currently covering the item.
    pub fn visible_rect(&self) -> Rect {
        if self.zoom <= 0.0 {
            return Rect::default();
        }
        let w = self.view_w / self.zoom;
        let h = self.view_h / self.zoom;
        Rect::new(self.center.x - w * 0.5, self.center.y - h * 0.5, w, h)
    }

    /// Circuit → item. C++ `viewTransform`.
    pub fn map_from_circuit(&self, scene: Point) -> Point {
        Point::new(
            (scene.x - self.center.x) * self.zoom + self.view_w * 0.5,
            (scene.y - self.center.y) * self.zoom + self.view_h * 0.5,
        )
    }

    /// Item → circuit.
    pub fn map_to_circuit(&self, item: Point) -> Point {
        if self.zoom == 0.0 {
            return self.center;
        }
        Point::new(
            (item.x - self.view_w * 0.5) / self.zoom + self.center.x,
            (item.y - self.view_h * 0.5) / self.zoom + self.center.y,
        )
    }

    pub fn map_rect_from_circuit(&self, r: Rect) -> Rect {
        let p0 = self.map_from_circuit(Point::new(r.left(), r.top()));
        let p1 = self.map_from_circuit(Point::new(r.right(), r.bottom()));
        Rect::new(p0.x, p0.y, p1.x - p0.x, p1.y - p0.y).normalized()
    }

    /// Returns whether the value changed.
    pub fn set_view_size(&mut self, w: f64, h: f64) -> bool {
        if self.view_w == w && self.view_h == h {
            return false;
        }
        self.view_w = w.max(1.0);
        self.view_h = h.max(1.0);
        self.clamp_center();
        true
    }

    /// Returns whether zoom changed. May also move `center` via clamp.
    pub fn set_zoom(&mut self, z: f64) -> bool {
        let z = z.clamp(MIN_ZOOM, MAX_ZOOM);
        if approx_eq(z, self.zoom) {
            return false;
        }
        self.zoom = z;
        self.clamp_center();
        true
    }

    pub fn set_center(&mut self, c: Point) -> bool {
        if c == self.center {
            return false;
        }
        self.center = c;
        self.clamp_center();
        true
    }

    /// C++ `zoomBy`: keep the scene point under `anchor_item` pinned there.
    pub fn zoom_by(&mut self, factor: f64, anchor_item: Point) -> bool {
        let anchor_scene = self.map_to_circuit(anchor_item);
        let target = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if approx_eq(target, self.zoom) {
            return false;
        }
        self.zoom = target;
        let viewport_offset = Point::new(
            anchor_item.x - self.view_w * 0.5,
            anchor_item.y - self.view_h * 0.5,
        );
        self.center = Point::new(
            anchor_scene.x - viewport_offset.x / self.zoom,
            anchor_scene.y - viewport_offset.y / self.zoom,
        );
        self.clamp_center();
        true
    }

    pub fn zoom_in(&mut self) -> bool {
        let mid = Point::new(self.view_w * 0.5, self.view_h * 0.5);
        self.zoom_by(ZOOM_STEP, mid)
    }

    pub fn zoom_out(&mut self) -> bool {
        let mid = Point::new(self.view_w * 0.5, self.view_h * 0.5);
        self.zoom_by(1.0 / ZOOM_STEP, mid)
    }

    pub fn zoom_one(&mut self) -> bool {
        self.set_zoom(1.0)
    }

    /// Fit `r` (already padded) in the view. C++ `zoomToFit` / `zoomSelected`.
    pub fn zoom_to_rect(&mut self, r: Rect) -> bool {
        if r.is_empty() || r.is_null() {
            return false;
        }
        let z = (self.view_w / r.w).min(self.view_h / r.h);
        let z_changed = self.set_zoom(z);
        let c_changed = self.set_center(r.center());
        z_changed || c_changed
    }

    /// Keep the viewport inside the scene. C++ `clampCenter`.
    pub fn clamp_center(&mut self) -> bool {
        let sr = self.scene;
        if sr.is_null() {
            return false;
        }
        let vis = self.visible_rect();
        let mut x = self.center.x;
        let mut y = self.center.y;

        if vis.w >= sr.w {
            x = sr.center().x;
        } else {
            let lo = sr.left() + vis.w * 0.5;
            let hi = sr.right() - vis.w * 0.5;
            x = x.clamp(lo, hi);
        }
        if vis.h >= sr.h {
            y = sr.center().y;
        } else {
            let lo = sr.top() + vis.h * 0.5;
            let hi = sr.bottom() - vis.h * 0.5;
            y = y.clamp(lo, hi);
        }

        let clamped = Point::new(x, y);
        if clamped == self.center {
            return false;
        }
        self.center = clamped;
        true
    }
}

fn approx_eq(a: f64, b: f64) -> bool {
    // qFuzzyCompare for non-zero; treat exact 0 as equal.
    if a == b {
        return true;
    }
    let diff = (a - b).abs();
    diff <= 1e-12 * a.abs().max(b.abs()).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp() -> Viewport {
        let mut v = Viewport::new(3200.0, 2400.0);
        v.set_view_size(800.0, 600.0);
        v
    }

    #[test]
    fn map_roundtrip() {
        let v = vp();
        let s = Point::new(40.0, -12.0);
        let i = v.map_from_circuit(s);
        let back = v.map_to_circuit(i);
        assert!((back.x - s.x).abs() < 1e-9);
        assert!((back.y - s.y).abs() < 1e-9);
        // Scene origin sits at the item centre at zoom 1, center 0.
        let mid = v.map_from_circuit(Point::zero());
        assert!((mid.x - 400.0).abs() < 1e-9);
        assert!((mid.y - 300.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_by_keeps_anchor() {
        let mut v = vp();
        let anchor = Point::new(500.0, 200.0);
        let scene_before = v.map_to_circuit(anchor);
        assert!(v.zoom_by(2.0, anchor));
        assert!((v.zoom() - 2.0).abs() < 1e-12);
        let scene_after = v.map_to_circuit(anchor);
        assert!((scene_after.x - scene_before.x).abs() < 1e-9);
        assert!((scene_after.y - scene_before.y).abs() < 1e-9);
    }

    #[test]
    fn zoom_clamped() {
        let mut v = vp();
        assert!(v.set_zoom(100.0));
        assert_eq!(v.zoom(), MAX_ZOOM);
        assert!(!v.set_zoom(MAX_ZOOM));
        assert!(v.set_zoom(0.01));
        assert_eq!(v.zoom(), MIN_ZOOM);
    }

    #[test]
    fn clamp_when_scene_smaller_than_view() {
        let mut v = Viewport::new(100.0, 80.0);
        v.set_view_size(800.0, 600.0);
        v.set_center(Point::new(50.0, 50.0));
        // Visible span is larger than the scene on both axes → pin to scene centre.
        assert_eq!(v.center(), Point::zero());
    }

    #[test]
    fn clamp_pan_to_scene_edge() {
        let mut v = vp();
        assert!(v.set_center(Point::new(10_000.0, 0.0)));
        let vis = v.visible_rect();
        let sr = v.scene_rect();
        assert!((v.center().x - (sr.right() - vis.w * 0.5)).abs() < 1e-9);
    }

    #[test]
    fn visible_rect_at_default() {
        let v = vp();
        let r = v.visible_rect();
        assert!((r.w - 800.0).abs() < 1e-9);
        assert!((r.h - 600.0).abs() < 1e-9);
        assert!((r.center().x).abs() < 1e-9);
        assert!((r.center().y).abs() < 1e-9);
    }

    #[test]
    fn test_scrollbar_calculations() {
        let v = vp();
        let sr = v.scene_rect();
        let vr = v.visible_rect();
        assert!(sr.w > 0.0 && sr.h > 0.0);
        let v_scroll_size = (vr.h / sr.h).clamp(0.0, 1.0);
        let h_scroll_size = (vr.w / sr.w).clamp(0.0, 1.0);
        let v_scroll_pos = ((vr.y - sr.y) / sr.h).clamp(0.0, 1.0);
        let h_scroll_pos = ((vr.x - sr.x) / sr.w).clamp(0.0, 1.0);
        assert!(v_scroll_size > 0.0 && v_scroll_size <= 1.0);
        assert!(h_scroll_size > 0.0 && h_scroll_size <= 1.0);
        assert!(v_scroll_pos >= 0.0 && v_scroll_pos <= 1.0);
        assert!(h_scroll_pos >= 0.0 && h_scroll_pos <= 1.0);
    }
}
