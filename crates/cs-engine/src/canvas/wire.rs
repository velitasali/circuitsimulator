//! Orthogonal wire routing matching `Connector::updateConRoute` for a new wire,
//! plus segment / corner drag matching `ConnectorLine`.

use super::geom::{Point, dist_to_segment, project_on_segment, to_grid};

/// Half-width of a wire for hit-test, a bit more than the 1 px stroke.
pub const WIRE_HIT: f64 = 3.5;

/// Hit on a closed wire: a segment, or an interior corner (vertex with
/// neighbours on both sides — the Connector's outer ends belong to Pins).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireHit {
    Segment(usize),
    Corner(usize),
}

#[derive(Clone, Debug)]
pub struct Wire {
    pub id: String,
    pub start_pin: String,
    pub end_pin: Option<String>,
    pub points: Vec<Point>,
    pub selected: bool,
    /// C++ `Connector::m_isBus`. Thicker stroke, no chevrons, no mix with
    /// non-bus pins/wires.
    pub is_bus: bool,
    /// Per-segment: a slant the user asked for (`ConnectorLine::m_freeDiag`).
    /// Routing leaves those alone. Length is `points.len() - 1`.
    free_diag: Vec<bool>,
    /// Index of the segment being stretched while drawing (`Connector::m_actLine`).
    act_line: usize,
}

impl Wire {
    pub fn start(id: impl Into<String>, start_pin: impl Into<String>, at: Point) -> Self {
        Self {
            id: id.into(),
            start_pin: start_pin.into(),
            end_pin: None,
            points: vec![at, at],
            selected: false,
            is_bus: false,
            free_diag: vec![false],
            act_line: 0,
        }
    }

    pub fn from_saved(
        id: impl Into<String>,
        start_pin: impl Into<String>,
        end_pin: impl Into<String>,
        points: Vec<Point>,
    ) -> Self {
        let mut points = points;
        if points.len() < 2 {
            let p = points.first().copied().unwrap_or(Point::zero());
            points = vec![p, p];
        }
        let mut w = Self {
            id: id.into(),
            start_pin: start_pin.into(),
            end_pin: Some(end_pin.into()),
            points,
            selected: false,
            is_bus: false,
            free_diag: Vec::new(),
            act_line: 0,
        };
        // The point list has no record of which slants were deliberate. Anything
        // stored slanted is taken as the user's (`ConnectorLine::markFreeDiag`).
        w.mark_loaded_diags();
        w
    }

    /// Split this closed wire at `at`. The first piece keeps this id and ends
    /// on `mid_end`; the second starts on `mid_start` with `new_id`.
    pub fn split(
        &self,
        at: Point,
        new_id: String,
        mid_end: String,
        mid_start: String,
    ) -> Option<(Wire, Wire)> {
        let end = self.end_pin.clone()?;
        if self.points.len() < 2 {
            return None;
        }
        let mut best_i = 0;
        let mut best_d = f64::MAX;
        let mut best_pt = at;
        for (i, w) in self.points.windows(2).enumerate() {
            let d = dist_to_segment(at, w[0], w[1]);
            if d < best_d {
                best_d = d;
                best_i = i;
                best_pt = project_on_segment(at, w[0], w[1]);
            }
        }
        let mut a_pts = self.points[..=best_i].to_vec();
        a_pts.push(best_pt);
        let mut b_pts = vec![best_pt];
        b_pts.extend_from_slice(&self.points[best_i + 1..]);
        let mut a = Wire::from_saved(&self.id, &self.start_pin, mid_end, a_pts);
        let mut b = Wire::from_saved(new_id, mid_start, end, b_pts);
        a.is_bus = self.is_bus;
        b.is_bus = self.is_bus;
        Some((a, b))
    }

    pub fn drawing(&self) -> bool {
        self.end_pin.is_none()
    }

    pub fn closed(&self) -> bool {
        self.end_pin.is_some()
    }

    /// Bounding rectangle encompassing all wire points.
    pub fn bounding_rect(&self) -> super::geom::Rect {
        if self.points.is_empty() {
            return super::geom::Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = self.points[0].x;
        let mut max_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_y = self.points[0].y;
        for p in &self.points[1..] {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        super::geom::Rect::new(
            min_x,
            min_y,
            (max_x - min_x).max(1.0),
            (max_y - min_y).max(1.0),
        )
    }

    /// Mouse-move routing. `shift` keeps a free diagonal (`m_freeLine`).
    pub fn route_end(&mut self, cursor: Point, shift: bool) {
        if self.points.len() < 2 {
            self.points = vec![cursor, cursor];
            self.sync_meta();
            return;
        }
        let p = to_grid(cursor);
        let last = self.points.len() - 1;
        self.points[last] = p;
        if shift {
            self.sync_meta();
            if let Some(f) = self.free_diag.last_mut() {
                *f = is_diag(self.points[last - 1], p);
            }
            return;
        }
        self.square_up();
        self.sync_meta();
    }

    /// Snap the last vertex onto a pin and square the last corner.
    pub fn close(&mut self, end_pin: impl Into<String>, at: Point) {
        self.end_pin = Some(end_pin.into());
        if self.points.len() < 2 {
            let start = self.points.first().copied().unwrap_or(at);
            self.points = vec![start, at];
        } else {
            let last = self.points.len() - 1;
            self.points[last] = at;
        }
        self.square_up();
        if let Some(p) = self.points.last_mut() {
            *p = at;
        }
        self.drop_nulls();
        self.sync_meta();
    }

    /// Left-button release while drawing: freeze the current segment and start
    /// a new one (`Connector::incActLine`).
    pub fn inc_act_line(&mut self) {
        let Some(&last) = self.points.last() else {
            return;
        };
        if self.points.len() >= 2 {
            let prev = self.points[self.points.len() - 2];
            if prev.x == last.x && prev.y == last.y {
                return;
            }
        }
        self.points.push(last);
        self.act_line = self.points.len() - 2;
        self.sync_meta();
    }

    pub fn set_start(&mut self, at: Point) {
        if self.points.is_empty() {
            self.points = vec![at, at];
            self.sync_meta();
            return;
        }
        self.points[0] = at;
        if !self.segment_free(0) {
            self.square_up_from_start();
        }
        self.drop_nulls();
        self.sync_meta();
    }

    pub fn set_end(&mut self, at: Point) {
        if self.points.len() < 2 {
            self.points = vec![at, at];
            self.sync_meta();
            return;
        }
        let last = self.points.len() - 1;
        self.points[last] = at;
        let last_seg = last - 1;
        if !self.segment_free(last_seg) {
            self.square_up();
        }
        if let Some(p) = self.points.last_mut() {
            *p = at;
        }
        self.drop_nulls();
        self.sync_meta();
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        for p in &mut self.points {
            p.x += dx;
            p.y += dy;
        }
    }

    pub fn hits(&self, scene: Point) -> bool {
        let slop = self.hit_width();
        self.points
            .windows(2)
            .any(|w| dist_to_segment(scene, w[0], w[1]) <= slop)
    }

    fn hit_width(&self) -> f64 {
        if self.is_bus { 4.5 } else { WIRE_HIT }
    }

    /// Segment or interior corner under `scene`, matching `ConnectorLine`
    /// mouse-press: grid-snapped cursor equal to an interior vertex is a corner.
    pub fn hit_detail(&self, scene: Point) -> Option<WireHit> {
        let slop = self.hit_width();
        let mut best_i = 0;
        let mut best_d = f64::MAX;
        for (i, w) in self.points.windows(2).enumerate() {
            let d = dist_to_segment(scene, w[0], w[1]);
            if d < best_d {
                best_d = d;
                best_i = i;
            }
        }
        if best_d > slop {
            return None;
        }
        let g = to_grid(scene);
        let p1 = self.points[best_i];
        let p2 = self.points[best_i + 1];
        if g == p1 && best_i > 0 {
            return Some(WireHit::Corner(best_i));
        }
        if g == p2 && best_i + 1 < self.points.len() - 1 {
            return Some(WireHit::Corner(best_i + 1));
        }
        Some(WireHit::Segment(best_i))
    }

    pub fn bounds(&self) -> super::geom::Rect {
        let mut r = super::geom::Rect::default();
        let slop = self.hit_width();
        for w in self.points.windows(2) {
            let x = w[0].x.min(w[1].x) - slop;
            let y = w[0].y.min(w[1].y) - slop;
            let wdt = (w[0].x - w[1].x).abs() + slop * 2.0;
            let h = (w[0].y - w[1].y).abs() + slop * 2.0;
            r = r.united(super::geom::Rect::new(x, y, wdt, h));
        }
        r
    }

    pub fn segment_is_horizontal(&self, seg: usize) -> bool {
        self.points
            .get(seg)
            .zip(self.points.get(seg + 1))
            .is_some_and(|(a, b)| a.y == b.y && a.x != b.x)
    }

    pub fn segment_is_vertical(&self, seg: usize) -> bool {
        self.points
            .get(seg)
            .zip(self.points.get(seg + 1))
            .is_some_and(|(a, b)| a.x == b.x && a.y != b.y)
    }

    /// Slide segment `seg` along its perpendicular (`ConnectorLine::dragLine`).
    /// Inserts a zero-length stub when this is the first or last segment so the
    /// Pin endpoint stays put. Returns the (possibly shifted) segment index.
    pub fn drag_segment(&mut self, mut seg: usize, delta: Point) -> usize {
        if self.points.len() < 2 || delta.x == 0.0 && delta.y == 0.0 {
            return seg;
        }
        let nseg = self.points.len() - 1;
        if seg >= nseg {
            return seg;
        }
        // Tail stub first so `seg` stays valid for the head insert.
        if seg == nseg - 1 {
            self.insert_stub_at_end();
        }
        if seg == 0 {
            self.insert_stub_at_start();
            seg = 1;
        }
        self.move_segment_perp(seg, delta);
        seg
    }

    /// Move interior vertex `vertex`. Shift (`free`) keeps the slant the user
    /// dragged; otherwise both meeting segments stay axis-aligned.
    /// Returns the (possibly shifted) vertex index.
    pub fn drag_corner(&mut self, mut vertex: usize, delta: Point, free: bool) -> usize {
        if vertex == 0 || vertex + 1 >= self.points.len() {
            // Outer end has no neighbour to keep square: drag the whole segment
            // unless Shift asked for a free diagonal.
            if free {
                self.points[vertex] = self.points[vertex].add(delta);
                let seg = if vertex == 0 { 0 } else { vertex - 1 };
                self.mark_free(seg);
            } else if vertex + 1 < self.points.len() {
                return self.drag_segment(vertex, delta);
            } else if vertex > 0 {
                let seg = self.drag_segment(vertex - 1, delta);
                return seg + 1;
            }
            return vertex;
        }
        if delta.x == 0.0 && delta.y == 0.0 {
            return vertex;
        }
        if free {
            self.points[vertex] = self.points[vertex].add(delta);
            self.mark_free(vertex - 1);
            self.mark_free(vertex);
            return vertex;
        }
        if vertex + 1 == self.points.len() - 1 {
            self.insert_stub_at_end();
        }
        if vertex == 1 {
            self.insert_stub_at_start();
            vertex += 1;
        }
        let target = self.points[vertex].add(delta);
        self.move_segment_perp(vertex - 1, delta);
        self.move_segment_perp(vertex, delta);
        self.points[vertex] = target;
        vertex
    }

    /// Put the outer vertices back on their Pins and simplify
    /// (`Connector::isMoved` + `remNullLines` after a wire drag).
    pub fn reanchor(&mut self, start: Point, end: Point) {
        if self.points.is_empty() {
            self.points = vec![start, end];
        } else if self.points.len() == 1 {
            self.points = vec![start, end];
        } else {
            self.points[0] = start;
            let last = self.points.len() - 1;
            self.points[last] = end;
        }
        if !self.segment_free(0) {
            self.square_up_from_start();
        }
        let last_seg = self.points.len().saturating_sub(2);
        if !self.segment_free(last_seg) {
            self.square_up();
        }
        if let Some(p) = self.points.last_mut() {
            *p = end;
        }
        if let Some(p) = self.points.first_mut() {
            *p = start;
        }
        self.rem_null_lines(true);
    }

    /// Merge collinear orthogonal neighbours; optionally drop zero-length
    /// segments (`Connector::remNullLines`).
    pub fn rem_null_lines(&mut self, drop_nulls: bool) {
        if drop_nulls {
            self.drop_nulls();
        }
        self.merge_aligned();
        self.sync_meta();
    }

    fn square_up(&mut self) {
        let n = self.points.len();
        if n < 2 {
            return;
        }
        let last = n - 1;
        let p1 = self.points[last - 1];
        let p2 = self.points[last];
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        if dx == 0.0 || dy == 0.0 {
            return;
        }
        let seg = last - 1;
        if self.segment_free(seg) {
            return;
        }
        if seg == self.act_line || n == 2 {
            let elbow = if dx.abs() > dy.abs() {
                Point::new(p2.x, p1.y)
            } else {
                Point::new(p1.x, p2.y)
            };
            self.points.insert(last, elbow);
        } else {
            let prev = self.points[last - 2];
            let mut corner = p1;
            if (p1.x - prev.x).abs() < 1e-9 {
                corner.y = p2.y;
            } else {
                corner.x = p2.x;
            }
            self.points[last - 1] = corner;
        }
        self.sync_meta();
    }

    fn square_up_from_start(&mut self) {
        // Same as square_up but operating on the first segment.
        let n = self.points.len();
        if n < 2 {
            return;
        }
        if self.segment_free(0) {
            return;
        }
        let p1 = self.points[0];
        let p2 = self.points[1];
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        if dx == 0.0 || dy == 0.0 {
            return;
        }
        if n == 2 {
            let elbow = if dx.abs() > dy.abs() {
                Point::new(p2.x, p1.y)
            } else {
                Point::new(p1.x, p2.y)
            };
            self.points.insert(1, elbow);
        } else {
            let next = self.points[2];
            let mut corner = p2;
            if (next.x - p2.x).abs() < 1e-9 {
                corner.y = p1.y;
            } else {
                corner.x = p1.x;
            }
            self.points[1] = corner;
        }
        self.sync_meta();
    }

    fn drop_nulls(&mut self) {
        let mut i = 1;
        while i < self.points.len() {
            let a = self.points[i - 1];
            let b = self.points[i];
            if a.x == b.x && a.y == b.y && self.points.len() > 2 {
                self.points.remove(i);
                if i - 1 < self.free_diag.len() {
                    self.free_diag.remove(i - 1);
                }
                if self.act_line >= i {
                    self.act_line = self.act_line.saturating_sub(1);
                }
            } else {
                i += 1;
            }
        }
        if self.points.len() < 2 {
            let p = self.points.first().copied().unwrap_or(Point::zero());
            self.points = vec![p, p];
        }
        self.sync_meta();
        if self.act_line + 1 >= self.points.len() {
            self.act_line = self.points.len().saturating_sub(2);
        }
    }

    fn merge_aligned(&mut self) {
        if self.points.len() < 3 {
            return;
        }
        let mut i = self.points.len() - 2; // last interior vertex
        while i >= 1 {
            let p0 = self.points[i - 1];
            let p1 = self.points[i];
            let p2 = self.points[i + 1];
            let d0 = is_diag(p0, p1);
            let d1 = is_diag(p1, p2);
            if d0 || d1 {
                if i == 1 {
                    break;
                }
                i -= 1;
                continue;
            }
            let dx0 = p1.x - p0.x;
            let dy0 = p1.y - p0.y;
            let dx1 = p2.x - p1.x;
            let dy1 = p2.y - p1.y;
            let aligned = (dx0 == 0.0 && dx1 == 0.0) || (dy0 == 0.0 && dy1 == 0.0);
            if aligned && !(dx1 == 0.0 && dy1 == 0.0) {
                self.points.remove(i);
                if i - 1 < self.free_diag.len() {
                    self.free_diag.remove(i - 1);
                }
                if self.act_line >= i {
                    self.act_line = self.act_line.saturating_sub(1);
                }
            }
            if i == 1 {
                break;
            }
            i -= 1;
        }
        self.sync_meta();
    }

    fn insert_stub_at_end(&mut self) {
        let Some(&p) = self.points.last() else {
            return;
        };
        self.points.push(p);
        self.sync_meta();
    }

    fn insert_stub_at_start(&mut self) {
        let Some(&p) = self.points.first() else {
            return;
        };
        self.points.insert(0, p);
        self.free_diag.insert(0, false);
        self.act_line += 1;
        self.sync_meta();
    }

    fn move_segment_perp(&mut self, seg: usize, delta: Point) {
        if seg + 1 >= self.points.len() {
            return;
        }
        let a = self.points[seg];
        let b = self.points[seg + 1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        if dx != 0.0 {
            self.points[seg].y += delta.y;
            self.points[seg + 1].y += delta.y;
        }
        if dy != 0.0 {
            self.points[seg].x += delta.x;
            self.points[seg + 1].x += delta.x;
        }
    }

    fn segment_free(&self, seg: usize) -> bool {
        self.free_diag.get(seg).copied().unwrap_or(false)
    }

    fn mark_free(&mut self, seg: usize) {
        self.sync_meta();
        if let Some(f) = self.free_diag.get_mut(seg) {
            if let (Some(&a), Some(&b)) = (self.points.get(seg), self.points.get(seg + 1)) {
                *f = is_diag(a, b);
            }
        }
    }

    fn mark_loaded_diags(&mut self) {
        self.sync_meta();
        for i in 0..self.free_diag.len() {
            self.free_diag[i] = is_diag(self.points[i], self.points[i + 1]);
        }
    }

    fn sync_meta(&mut self) {
        let n = self.points.len().saturating_sub(1);
        if self.free_diag.len() > n {
            self.free_diag.truncate(n);
        }
        while self.free_diag.len() < n {
            self.free_diag.push(false);
        }
        for i in 0..n {
            if !is_diag(self.points[i], self.points[i + 1]) {
                self.free_diag[i] = false;
            }
        }
        if n == 0 {
            self.act_line = 0;
        } else if self.act_line >= n {
            self.act_line = n - 1;
        }
    }
}

fn is_diag(a: Point, b: Point) -> bool {
    a.x != b.x && a.y != b.y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_grows_a_corner() {
        let mut w = Wire::start("Connector-1", "A", Point::new(0.0, 0.0));
        w.route_end(Point::new(40.0, 24.0), false);
        assert!(w.points.len() >= 3);
        let pts = &w.points;
        // First segment axis-aligned.
        assert!(pts[0].x == pts[1].x || pts[0].y == pts[1].y);
        // Last vertex is grid-snapped cursor.
        assert_eq!(*pts.last().unwrap(), to_grid(Point::new(40.0, 24.0)));
    }

    #[test]
    fn shift_keeps_diagonal() {
        let mut w = Wire::start("Connector-1", "A", Point::new(0.0, 0.0));
        w.route_end(Point::new(40.0, 24.0), true);
        assert_eq!(w.points.len(), 2);
        assert_eq!(w.points[1], to_grid(Point::new(40.0, 24.0)));
    }

    #[test]
    fn drag_horizontal_segment_moves_y() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![Point::new(0.0, 0.0), Point::new(40.0, 0.0)],
        );
        let seg = w.drag_segment(0, Point::new(0.0, 16.0));
        assert_eq!(seg, 1);
        assert_eq!(w.points[0], Point::new(0.0, 0.0));
        assert_eq!(w.points[1], Point::new(0.0, 16.0));
        assert_eq!(w.points[2], Point::new(40.0, 16.0));
        assert_eq!(w.points[3], Point::new(40.0, 0.0));
        w.reanchor(Point::new(0.0, 0.0), Point::new(40.0, 0.0));
        assert_eq!(w.points[0], Point::new(0.0, 0.0));
        assert_eq!(*w.points.last().unwrap(), Point::new(40.0, 0.0));
        assert!(w.points.iter().any(|p| p.y == 16.0));
    }

    #[test]
    fn corner_drag_stays_orthogonal() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![
                Point::new(0.0, 0.0),
                Point::new(0.0, 16.0),
                Point::new(32.0, 16.0),
            ],
        );
        let v = w.drag_corner(1, Point::new(8.0, 8.0), false);
        assert_eq!(w.points[v], Point::new(8.0, 24.0));
        // Both meeting segments still axis-aligned.
        assert!(w.points[v - 1].x == w.points[v].x || w.points[v - 1].y == w.points[v].y);
        assert!(w.points[v].x == w.points[v + 1].x || w.points[v].y == w.points[v + 1].y);
        assert!(!is_diag(w.points[v - 1], w.points[v]));
        assert!(!is_diag(w.points[v], w.points[v + 1]));
    }

    #[test]
    fn shift_corner_allows_diagonal() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![
                Point::new(0.0, 0.0),
                Point::new(0.0, 16.0),
                Point::new(32.0, 16.0),
            ],
        );
        w.drag_corner(1, Point::new(8.0, 8.0), true);
        assert_eq!(w.points[1], Point::new(8.0, 24.0));
        assert!(is_diag(w.points[0], w.points[1]));
        assert!(is_diag(w.points[1], w.points[2]));
    }

    #[test]
    fn rem_nulls_merges_collinear() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![
                Point::new(0.0, 0.0),
                Point::new(16.0, 0.0),
                Point::new(32.0, 0.0),
            ],
        );
        w.rem_null_lines(true);
        assert_eq!(w.points.len(), 2);
        assert_eq!(w.points[0], Point::new(0.0, 0.0));
        assert_eq!(w.points[1], Point::new(32.0, 0.0));
    }

    #[test]
    fn hit_detail_corner_vs_segment() {
        let w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![
                Point::new(0.0, 0.0),
                Point::new(0.0, 16.0),
                Point::new(32.0, 16.0),
            ],
        );
        assert_eq!(
            w.hit_detail(Point::new(0.0, 16.0)),
            Some(WireHit::Corner(1))
        );
        assert_eq!(
            w.hit_detail(Point::new(16.0, 16.0)),
            Some(WireHit::Segment(1))
        );
        assert_eq!(w.hit_detail(Point::new(100.0, 100.0)), None);
    }

    #[test]
    fn split_copies_bus() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![Point::new(0.0, 0.0), Point::new(40.0, 0.0)],
        );
        w.is_bus = true;
        let (a, b) = w
            .split(
                Point::new(16.0, 0.0),
                "Connector-2".into(),
                "N-0".into(),
                "N-1".into(),
            )
            .unwrap();
        assert!(a.is_bus && b.is_bus);
    }
}
