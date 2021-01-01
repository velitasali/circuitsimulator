//! Scene-space geometry and C++ `toGrid` / `snapToGrid4`.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    pub fn scale(self, k: f64) -> Self {
        Self {
            x: self.x * k,
            y: self.y * k,
        }
    }

    pub fn distance(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }
}

/// QGraphicsItem order: `transform()` (scale hflip/vflip) then `rotation`,
/// then `pos`. Y-down, so a positive angle is clockwise.
pub fn map_local(origin: Point, local: Point, rotation_deg: f64, hflip: i32, vflip: i32) -> Point {
    let p = Point::new(local.x * hflip as f64, local.y * vflip as f64);
    let rad = rotation_deg.to_radians();
    let (s, c) = (rad.sin(), rad.cos());
    Point::new(origin.x + p.x * c - p.y * s, origin.y + p.x * s + p.y * c)
}

/// Inverse of [`map_local`].
pub fn map_scene(origin: Point, scene: Point, rotation_deg: f64, hflip: i32, vflip: i32) -> Point {
    let dx = scene.x - origin.x;
    let dy = scene.y - origin.y;
    let rad = rotation_deg.to_radians();
    let (s, c) = (rad.sin(), rad.cos());
    let lx = dx * c + dy * s;
    let ly = -dx * s + dy * c;
    Point::new(lx * hflip as f64, ly * vflip as f64)
}

/// Axis-aligned bounds of a local rect after [`map_local`].
pub fn map_rect(origin: Point, local: Rect, rotation_deg: f64, hflip: i32, vflip: i32) -> Rect {
    if rotation_deg == 0.0 && hflip == 1 && vflip == 1 {
        return local.translated(origin.x, origin.y);
    }
    let corners = [
        map_local(
            origin,
            Point::new(local.x, local.y),
            rotation_deg,
            hflip,
            vflip,
        ),
        map_local(
            origin,
            Point::new(local.x + local.w, local.y),
            rotation_deg,
            hflip,
            vflip,
        ),
        map_local(
            origin,
            Point::new(local.x, local.y + local.h),
            rotation_deg,
            hflip,
            vflip,
        ),
        map_local(
            origin,
            Point::new(local.x + local.w, local.y + local.h),
            rotation_deg,
            hflip,
            vflip,
        ),
    ];
    let mut min_x = corners[0].x;
    let mut max_x = corners[0].x;
    let mut min_y = corners[0].y;
    let mut max_y = corners[0].y;
    for c in &corners[1..] {
        min_x = min_x.min(c.x);
        max_x = max_x.max(c.x);
        min_y = min_y.min(c.y);
        max_y = max_y.max(c.y);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Closest point on the segment `a`–`b` to `p`.
pub fn project_on_segment(p: Point, a: Point, b: Point) -> Point {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let len2 = vx * vx + vy * vy;
    if len2 == 0.0 {
        return a;
    }
    let t = ((p.x - a.x) * vx + (p.y - a.y) * vy) / len2;
    let t = t.clamp(0.0, 1.0);
    Point::new(a.x + t * vx, a.y + t * vy)
}

/// Distance from `p` to the segment `a`–`b`.
pub fn dist_to_segment(p: Point, a: Point, b: Point) -> f64 {
    p.distance(project_on_segment(p, a, b))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn area(&self) -> f64 {
        (self.w * self.h).max(0.0)
    }

    pub fn is_null(&self) -> bool {
        self.w == 0.0 && self.h == 0.0
    }

    pub fn left(&self) -> f64 {
        self.x
    }

    pub fn top(&self) -> f64 {
        self.y
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// QRectF::normalized: positive width/height.
    pub fn normalized(self) -> Self {
        let mut x = self.x;
        let mut y = self.y;
        let mut w = self.w;
        let mut h = self.h;
        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }
        Self { x, y, w, h }
    }

    /// QRectF::adjust(dx1, dy1, dx2, dy2).
    pub fn adjust(self, dx1: f64, dy1: f64, dx2: f64, dy2: f64) -> Self {
        Self {
            x: self.x + dx1,
            y: self.y + dy1,
            w: self.w - dx1 + dx2,
            h: self.h - dy1 + dy2,
        }
    }

    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: self.w,
            h: self.h,
        }
    }

    pub fn contains_point(&self, p: Point) -> bool {
        p.x >= self.left() && p.x < self.right() && p.y >= self.top() && p.y < self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        let a = self.normalized();
        let b = other.normalized();
        a.left() < b.right() && b.left() < a.right() && a.top() < b.bottom() && b.top() < a.bottom()
    }

    pub fn united(self, other: Rect) -> Self {
        if self.is_null() {
            return other;
        }
        if other.is_null() {
            return self;
        }
        let a = self.normalized();
        let b = other.normalized();
        let x = a.left().min(b.left());
        let y = a.top().min(b.top());
        let r = a.right().max(b.right());
        let bot = a.bottom().max(b.bottom());
        Self {
            x,
            y,
            w: r - x,
            h: bot - y,
        }
    }
}

/// C++ `roundDown` in `utils.cpp`. Toward-zero division matches C++ `/` for ints.
pub fn round_down(x: i32, roundness: i32) -> i32 {
    if x < 0 {
        (x - roundness + 1) / roundness
    } else {
        x / roundness
    }
}

/// C++ `snapToGrid4`.
pub fn snap_to_grid4(x: i32) -> i32 {
    round_down(x + 2, 4) * 4
}

/// C++ `toGrid(QPointF)`: truncate toward zero, then snap each axis to 4.
pub fn to_grid(p: Point) -> Point {
    Point::new(
        snap_to_grid4(p.x as i32) as f64,
        snap_to_grid4(p.y as i32) as f64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_matches_cpp() {
        assert_eq!(snap_to_grid4(0), 0);
        assert_eq!(snap_to_grid4(1), 0);
        assert_eq!(snap_to_grid4(2), 4);
        assert_eq!(snap_to_grid4(5), 4);
        assert_eq!(snap_to_grid4(6), 8);
        assert_eq!(snap_to_grid4(-1), 0);
        assert_eq!(snap_to_grid4(-2), 0);
        assert_eq!(snap_to_grid4(-3), -4);
    }

    #[test]
    fn rect_normalize_and_hit() {
        let r = Rect::new(10.0, 10.0, -8.0, -6.0).normalized();
        assert_eq!(r, Rect::new(2.0, 4.0, 8.0, 6.0));
        assert!(r.contains_point(Point::new(2.0, 4.0)));
        assert!(!r.contains_point(Point::new(10.0, 4.0)));
        assert!(r.intersects(&Rect::new(9.0, 5.0, 2.0, 2.0)));
        assert!(!r.intersects(&Rect::new(10.0, 4.0, 2.0, 2.0)));
    }

    #[test]
    fn rotate_90_cw_y_down() {
        let origin = Point::zero();
        let left = Point::new(-16.0, 0.0);
        let mapped = map_local(origin, left, 90.0, 1, 1);
        assert!((mapped.x - 0.0).abs() < 1e-9);
        assert!((mapped.y - -16.0).abs() < 1e-9);
        let back = map_scene(origin, mapped, 90.0, 1, 1);
        assert!((back.x - left.x).abs() < 1e-9);
        assert!((back.y - left.y).abs() < 1e-9);
    }

    #[test]
    fn hflip_swaps_x() {
        let origin = Point::new(10.0, 20.0);
        let p = Point::new(-16.0, 4.0);
        let mapped = map_local(origin, p, 0.0, -1, 1);
        assert_eq!(mapped, Point::new(26.0, 24.0));
    }
}
