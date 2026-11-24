//! Orthogonal wire routing matching `Connector::updateConRoute` for a new wire,
//! plus segment / corner drag matching `ConnectorLine`.

use super::geom::{Point, Rect, dist_to_segment, project_on_segment, to_grid};

/// Half-width of a wire for hit-test, a bit more than the 1 px stroke.
pub const WIRE_HIT: f64 = 3.5;

/// Floating-point tolerance for axis-alignment and coincident-point checks.
const EPS: f64 = 1e-6;

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
    /// Index of the last committed vertex while drawing (`Connector::m_actLine`).
    /// Everything after `points[act_line]` is the active uncommitted leg.
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

    /// Project `at` onto the closest segment of this wire, producing a point
    /// guaranteed to lie on the wire. Axis-aligned segments snap along their
    /// running axis to the 4-unit grid (clamped to the segment endpoints) while
    /// locking the perpendicular axis to the segment; angled segments project
    /// orthogonally onto the segment.
    pub fn project_splice_point(&self, at: Point) -> Option<(usize, Point)> {
        if self.points.len() < 2 {
            return None;
        }
        let mut best_i = 0;
        let mut best_d = f64::MAX;
        for (i, w) in self.points.windows(2).enumerate() {
            let d = dist_to_segment(at, w[0], w[1]);
            if d < best_d {
                best_d = d;
                best_i = i;
            }
        }
        let a = self.points[best_i];
        let b = self.points[best_i + 1];
        let split_pt = if is_horiz(a, b) {
            let min_x = a.x.min(b.x);
            let max_x = a.x.max(b.x);
            let x = to_grid(at).x.clamp(min_x, max_x);
            Point::new(x, a.y)
        } else if is_vert(a, b) {
            let min_y = a.y.min(b.y);
            let max_y = a.y.max(b.y);
            let y = to_grid(at).y.clamp(min_y, max_y);
            Point::new(a.x, y)
        } else {
            let proj = project_on_segment(at, a, b);
            if pts_eq(proj, a) {
                a
            } else if pts_eq(proj, b) {
                b
            } else {
                proj
            }
        };
        Some((best_i, split_pt))
    }

    /// Split this closed wire at `at`. The first piece keeps this id and ends
    /// on `mid_end`; the second starts on `mid_start` with `new_id`.
    /// Returns the two half-wires and the exact on-wire split point where the
    /// junction Node must be placed.
    pub fn split(
        &self,
        at: Point,
        new_id: String,
        mid_end: String,
        mid_start: String,
    ) -> Option<(Wire, Wire, Point)> {
        let end = self.end_pin.clone()?;
        let (best_i, split_pt) = self.project_splice_point(at)?;
        let mut a_pts = self.points[..=best_i].to_vec();
        if !a_pts.last().is_some_and(|&p| pts_eq(p, split_pt)) || a_pts.len() < 2 {
            a_pts.push(split_pt);
        }
        let mut b_pts = vec![split_pt];
        let tail = &self.points[best_i + 1..];
        if tail.first().is_some_and(|&p| pts_eq(p, split_pt)) && tail.len() >= 2 {
            b_pts.extend_from_slice(&tail[1..]);
        } else {
            b_pts.extend_from_slice(tail);
        }
        let mut a = Wire::from_saved(&self.id, &self.start_pin, mid_end, a_pts);
        let mut b = Wire::from_saved(new_id, mid_start, end, b_pts);
        a.is_bus = self.is_bus;
        b.is_bus = self.is_bus;
        Some((a, b, split_pt))
    }

    pub fn drawing(&self) -> bool {
        self.end_pin.is_none()
    }

    pub fn closed(&self) -> bool {
        self.end_pin.is_some()
    }

    /// Bounding rectangle encompassing all wire points.
    pub fn bounding_rect(&self) -> Rect {
        if self.points.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
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
        Rect::new(
            min_x,
            min_y,
            (max_x - min_x).max(1.0),
            (max_y - min_y).max(1.0),
        )
    }

    /// Mouse-move routing snapped to the 4-unit grid. `shift` allows an angled
    /// (free diagonal) active leg; otherwise the active leg is strictly
    /// orthogonal (0°/90°/180°/270°).
    pub fn route_end(&mut self, cursor: Point, shift: bool) {
        self.route_end_exact(to_grid(cursor), shift);
    }

    /// Mouse-move routing to an exact target point (e.g. a snapped pin,
    /// on-wire splice point, or grid-snapped cursor).
    pub fn route_end_exact(&mut self, target: Point, shift: bool) {
        if self.points.is_empty() {
            self.points = vec![target, target];
            self.act_line = 0;
            self.sync_meta();
            return;
        }
        self.route_active_leg(target, shift);
    }

    /// Rebuild the uncommitted active leg (`points[act_line..]`) cleanly from
    /// `points[act_line]` to `target`.
    ///
    /// - When `shift` is true: single straight segment `anchor -> target`,
    ///   marked `free_diag` if angled.
    /// - When `shift` is false: strictly orthogonal (1 segment if aligned with
    ///   `anchor`, otherwise a clean 2-segment L-bend preserving the current
    ///   leg's initial H/V orientation so it never jitters or doubles back).
    fn route_active_leg(&mut self, target: Point, shift: bool) {
        if self.act_line >= self.points.len() {
            self.act_line = self.points.len().saturating_sub(1);
        }
        let anchor = self.points[self.act_line];

        // Determine whether the active leg already had an established H or V
        // initial direction before we replace it.
        let prev_horiz_first = if self.points.len() > self.act_line + 1 {
            let next = self.points[self.act_line + 1];
            if is_horiz(anchor, next) {
                Some(true)
            } else if is_vert(anchor, next) {
                Some(false)
            } else {
                None
            }
        } else {
            None
        };

        self.points.truncate(self.act_line + 1);
        self.free_diag.truncate(self.act_line);

        if shift {
            self.points.push(target);
            self.free_diag.push(is_diag(anchor, target));
            self.sync_meta();
            return;
        }

        let dx = target.x - anchor.x;
        let dy = target.y - anchor.y;
        if pts_eq(anchor, target) || dx.abs() < EPS || dy.abs() < EPS {
            self.points.push(target);
            self.free_diag.push(false);
        } else {
            let horiz_first = prev_horiz_first.unwrap_or_else(|| dx.abs() > dy.abs());
            let elbow = if horiz_first {
                Point::new(target.x, anchor.y)
            } else {
                Point::new(anchor.x, target.y)
            };
            self.points.push(elbow);
            self.points.push(target);
            self.free_diag.push(false);
            self.free_diag.push(false);
        }
        self.sync_meta();
    }

    /// Snap the last vertex onto a pin (or splice Node) and square the active
    /// leg unless `shift` is held.
    pub fn close(&mut self, end_pin: impl Into<String>, at: Point) {
        self.close_with_shift(end_pin, at, false);
    }

    /// Snap the last vertex onto `at`, routing the final leg orthogonally
    /// unless `shift` is pressed, then remove any redundant collinear/null
    /// segments.
    pub fn close_with_shift(&mut self, end_pin: impl Into<String>, at: Point, shift: bool) {
        if self.points.is_empty() {
            self.points = vec![at, at];
            self.act_line = 0;
        } else {
            self.route_active_leg(at, shift);
        }
        self.end_pin = Some(end_pin.into());
        if let Some(p) = self.points.last_mut() {
            *p = at;
        }
        self.rem_null_lines(true);
    }

    /// Left-button release on empty canvas while drawing: freeze the current
    /// active leg up to the cursor and begin a new active leg from there
    /// (`Connector::incActLine`).
    pub fn inc_act_line(&mut self) {
        let Some(&last) = self.points.last() else {
            return;
        };
        if self.act_line >= self.points.len() {
            self.act_line = self.points.len().saturating_sub(1);
        }
        // Ignore clicks that have not moved away from the current anchor.
        if pts_eq(self.points[self.act_line], last) {
            return;
        }
        // Drop any zero-length trailing point before advancing.
        while self.points.len() > self.act_line + 2
            && pts_eq(
                self.points[self.points.len() - 2],
                self.points[self.points.len() - 1],
            )
        {
            self.points.pop();
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
        if let Some(p) = self.points.first_mut() {
            *p = at;
        }
        self.rem_null_lines(true);
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
        self.rem_null_lines(true);
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        for p in &mut self.points {
            p.x += dx;
            p.y += dy;
        }
    }

    /// Translate the wire by `(dx, dy)` while keeping any stationary endpoint
    /// (`anchor_start` / `anchor_end`) attached live to its pin or Node.
    pub fn translate_with_anchors(
        &mut self,
        dx: f64,
        dy: f64,
        anchor_start: Option<Point>,
        anchor_end: Option<Point>,
    ) {
        if anchor_start.is_none() && anchor_end.is_none() {
            self.translate(dx, dy);
            return;
        }
        if self.points.len() < 2 {
            return;
        }
        let n = self.points.len();
        if n == 2 {
            if anchor_end.is_some() && !self.segment_free(0) {
                self.insert_stub_at_end();
            }
            if anchor_start.is_some() && !self.segment_free(0) {
                self.insert_stub_at_start();
            }
        }
        let last = self.points.len() - 1;
        for (i, p) in self.points.iter_mut().enumerate() {
            if i == 0 && anchor_start.is_some() {
                continue;
            }
            if i == last && anchor_end.is_some() {
                continue;
            }
            p.x += dx;
            p.y += dy;
        }
        if let Some(s) = anchor_start {
            self.points[0] = s;
            if !self.segment_free(0) {
                self.square_up_from_start();
            }
        }
        if let Some(e) = anchor_end {
            let last = self.points.len() - 1;
            self.points[last] = e;
            let last_seg = last.saturating_sub(1);
            if !self.segment_free(last_seg) {
                self.square_up();
            }
        }
        if let Some(s) = anchor_start
            && let Some(p) = self.points.first_mut()
        {
            *p = s;
        }
        if let Some(e) = anchor_end
            && let Some(p) = self.points.last_mut()
        {
            *p = e;
        }
        self.rem_null_lines(true);
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

    /// Segment or interior corner under `scene`. Interior vertices hit within
    /// a small radius (or grid-exact match) so both on-grid and off-grid
    /// angled corners are reliably grabbable.
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
        let seg_len = p1.distance(p2);
        let corner_r = slop.min(seg_len * 0.35);
        let d1 = scene.distance(p1);
        let d2 = scene.distance(p2);
        let hit_p1 = best_i > 0 && (pts_eq(g, p1) || d1 <= corner_r);
        let hit_p2 = best_i + 1 < self.points.len() - 1 && (pts_eq(g, p2) || d2 <= corner_r);
        match (hit_p1, hit_p2) {
            (true, true) => {
                if d1 <= d2 {
                    Some(WireHit::Corner(best_i))
                } else {
                    Some(WireHit::Corner(best_i + 1))
                }
            }
            (true, false) => Some(WireHit::Corner(best_i)),
            (false, true) => Some(WireHit::Corner(best_i + 1)),
            (false, false) => Some(WireHit::Segment(best_i)),
        }
    }

    pub fn bounds(&self) -> Rect {
        let mut r = Rect::default();
        let slop = self.hit_width();
        for w in self.points.windows(2) {
            let x = w[0].x.min(w[1].x) - slop;
            let y = w[0].y.min(w[1].y) - slop;
            let wdt = (w[0].x - w[1].x).abs() + slop * 2.0;
            let h = (w[0].y - w[1].y).abs() + slop * 2.0;
            r = r.united(Rect::new(x, y, wdt, h));
        }
        r
    }

    pub fn segment_is_horizontal(&self, seg: usize) -> bool {
        self.points
            .get(seg)
            .zip(self.points.get(seg + 1))
            .is_some_and(|(&a, &b)| is_horiz(a, b))
    }

    pub fn segment_is_vertical(&self, seg: usize) -> bool {
        self.points
            .get(seg)
            .zip(self.points.get(seg + 1))
            .is_some_and(|(&a, &b)| is_vert(a, b))
    }

    pub fn segment_is_diagonal(&self, seg: usize) -> bool {
        self.points
            .get(seg)
            .zip(self.points.get(seg + 1))
            .is_some_and(|(&a, &b)| is_diag(a, b))
    }

    /// Slide orthogonal segment `seg` along its perpendicular (`ConnectorLine::dragLine`).
    /// Inserts a zero-length stub when this is the first or last segment so the
    /// Pin endpoint stays put. Returns the (possibly shifted) segment index.
    pub fn drag_segment(&mut self, mut seg: usize, delta: Point) -> usize {
        if self.points.len() < 2 || (delta.x == 0.0 && delta.y == 0.0) {
            return seg;
        }
        let nseg = self.points.len() - 1;
        if seg >= nseg {
            return seg;
        }
        if self.segment_is_diagonal(seg) {
            if self.can_parallel_slide_diag(seg) {
                self.drag_diag_segment_parallel(seg, delta);
            }
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

    /// Interactive segment drag supporting both orthogonal and angled segments:
    /// - Orthogonal segment: slides perpendicularly, keeping pins anchored.
    /// - Interior angled segment between orthogonal neighbours (when `!free`):
    ///   slides parallel along its orthogonal neighbours, preserving its angle.
    /// - Pinned angled segment (or angled segment with `free`/Shift): inserts a
    ///   pivot corner at `grab_pos`, moves it by `delta`, and transitions to
    ///   `WireHit::Corner(new_vertex)` so the user bends the angled wire smoothly.
    pub fn drag_segment_at(
        &mut self,
        seg: usize,
        delta: Point,
        grab_pos: Point,
        free: bool,
    ) -> WireHit {
        if self.points.len() < 2 || (delta.x == 0.0 && delta.y == 0.0) {
            return WireHit::Segment(seg);
        }
        let nseg = self.points.len() - 1;
        if seg >= nseg {
            return WireHit::Segment(seg);
        }
        if self.segment_is_diagonal(seg) {
            if !free && self.can_parallel_slide_diag(seg) {
                self.drag_diag_segment_parallel(seg, delta);
                return WireHit::Segment(seg);
            }
            let a = self.points[seg];
            let b = self.points[seg + 1];
            let pivot = project_on_segment(grab_pos, a, b);
            let moved = pivot.add(delta);
            let new_v = seg + 1;
            self.points.insert(new_v, moved);
            self.sync_meta();
            self.mark_free(seg);
            self.mark_free(new_v);
            return WireHit::Corner(new_v);
        }
        WireHit::Segment(self.drag_segment(seg, delta))
    }

    fn can_parallel_slide_diag(&self, seg: usize) -> bool {
        if seg == 0 || seg + 1 >= self.points.len().saturating_sub(1) {
            return false;
        }
        if !self.segment_is_diagonal(seg) {
            return false;
        }
        let left_ortho = self.segment_is_horizontal(seg - 1) || self.segment_is_vertical(seg - 1);
        let right_ortho = self.segment_is_horizontal(seg + 1) || self.segment_is_vertical(seg + 1);
        left_ortho && right_ortho
    }

    fn drag_diag_segment_parallel(&mut self, seg: usize, delta: Point) {
        let left_h = self.segment_is_horizontal(seg - 1);
        let right_h = self.segment_is_horizontal(seg + 1);
        let a = self.points[seg];
        let b = self.points[seg + 1];
        let vx = b.x - a.x;
        let vy = b.y - a.y;
        if vx.abs() < EPS || vy.abs() < EPS {
            return;
        }
        match (left_h, right_h) {
            (true, true) => {
                // H - Diag - H: slide horizontally along both horizontal neighbors.
                self.points[seg].x += delta.x;
                self.points[seg + 1].x += delta.x;
            }
            (false, false) => {
                // V - Diag - V: slide vertically along both vertical neighbors.
                self.points[seg].y += delta.y;
                self.points[seg + 1].y += delta.y;
            }
            (true, false) => {
                // H - Diag - V miter: slide parallel keeping `a` on H and `b` on V.
                let dx_a = delta.x - (vx / vy) * delta.y;
                let dy_b = delta.y - (vy / vx) * delta.x;
                let new_a = Point::new(a.x + dx_a, a.y);
                let new_b = Point::new(b.x, b.y + dy_b);
                let nvx = new_b.x - new_a.x;
                let nvy = new_b.y - new_a.y;
                if nvx * vx <= EPS || nvy * vy <= EPS {
                    let corner = Point::new(b.x, a.y);
                    self.points[seg] = corner;
                    self.points[seg + 1] = corner;
                } else {
                    self.points[seg] = new_a;
                    self.points[seg + 1] = new_b;
                }
            }
            (false, true) => {
                // V - Diag - H miter: slide parallel keeping `a` on V and `b` on H.
                let dy_a = delta.y - (vy / vx) * delta.x;
                let dx_b = delta.x - (vx / vy) * delta.y;
                let new_a = Point::new(a.x, a.y + dy_a);
                let new_b = Point::new(b.x + dx_b, b.y);
                let nvx = new_b.x - new_a.x;
                let nvy = new_b.y - new_a.y;
                if nvx * vx <= EPS || nvy * vy <= EPS {
                    let corner = Point::new(a.x, b.y);
                    self.points[seg] = corner;
                    self.points[seg + 1] = corner;
                } else {
                    self.points[seg] = new_a;
                    self.points[seg + 1] = new_b;
                }
            }
        }
        self.mark_free(seg);
    }

    /// Move interior vertex `vertex`.
    /// - With `free` (Shift): moves freely in 2D and marks both incident
    ///   segments as free diagonals.
    /// - Without `free`:
    ///   - Incident angled segments pivot smoothly with the corner.
    ///   - Incident orthogonal segments remain strictly axis-aligned (inserting
    ///     end stubs when touching pins and handling zero-length segments
    ///     without ever creating accidental diagonals).
    /// Returns the (possibly shifted) vertex index.
    pub fn drag_corner(&mut self, mut vertex: usize, delta: Point, free: bool) -> usize {
        if vertex == 0 || vertex + 1 >= self.points.len() {
            // Outer ends belong to Pins and must never detach.
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

        let left_diag = self.segment_free(vertex - 1) || self.segment_is_diagonal(vertex - 1);
        let right_diag = self.segment_free(vertex) || self.segment_is_diagonal(vertex);

        if left_diag && right_diag {
            self.points[vertex] = self.points[vertex].add(delta);
            self.mark_free(vertex - 1);
            self.mark_free(vertex);
            return vertex;
        }

        if !right_diag && vertex + 1 == self.points.len() - 1 {
            self.insert_stub_at_end();
        }
        if !left_diag && vertex == 1 {
            self.insert_stub_at_start();
            vertex += 1;
        }

        let target = self.points[vertex].add(delta);
        let left_axis = if left_diag {
            None
        } else {
            Some(self.infer_ortho_axis(vertex - 1, Some(vertex)))
        };
        let right_axis = if right_diag {
            None
        } else {
            Some(self.infer_ortho_axis(vertex, Some(vertex - 1)))
        };

        if let Some(is_h) = left_axis {
            if is_h {
                self.points[vertex - 1].y = target.y;
            } else {
                self.points[vertex - 1].x = target.x;
            }
        }
        if let Some(is_h) = right_axis {
            if is_h {
                self.points[vertex + 1].y = target.y;
            } else {
                self.points[vertex + 1].x = target.x;
            }
        }
        self.points[vertex] = target;
        if left_diag {
            self.mark_free(vertex - 1);
        }
        if right_diag {
            self.mark_free(vertex);
        }
        self.sync_meta();
        vertex
    }

    /// Infer whether orthogonal segment `seg` is horizontal (`true`) or
    /// vertical (`false`), even when it has momentarily collapsed to zero
    /// length during a corner drag.
    fn infer_ortho_axis(&self, seg: usize, sibling: Option<usize>) -> bool {
        if self.segment_is_horizontal(seg) {
            return true;
        }
        if self.segment_is_vertical(seg) {
            return false;
        }
        if let Some(sib) = sibling {
            if self.segment_is_horizontal(sib) {
                return false;
            }
            if self.segment_is_vertical(sib) {
                return true;
            }
        }
        if seg > 0 {
            if self.segment_is_horizontal(seg - 1) {
                return false;
            }
            if self.segment_is_vertical(seg - 1) {
                return true;
            }
        }
        if seg + 2 < self.points.len() {
            if self.segment_is_horizontal(seg + 1) {
                return false;
            }
            if self.segment_is_vertical(seg + 1) {
                return true;
            }
        }
        true
    }

    /// Put the outer vertices back on their Pins and simplify
    /// (`Connector::isMoved` + `remNullLines` after a wire drag).
    pub fn reanchor(&mut self, start: Point, end: Point) {
        if self.points.len() < 2 {
            self.points = vec![start, end];
        } else {
            self.points[0] = start;
            let last = self.points.len() - 1;
            self.points[last] = end;
        }
        self.drop_nulls();
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

    /// Merge collinear neighbours; optionally drop zero-length segments
    /// (`Connector::remNullLines`). Runs to a fixed point so nested fold-backs
    /// collapse completely.
    pub fn rem_null_lines(&mut self, drop_nulls: bool) {
        loop {
            let before = self.points.len();
            if drop_nulls {
                self.drop_nulls();
            }
            self.merge_aligned();
            if self.points.len() == before {
                break;
            }
        }
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
        if dx.abs() < EPS || dy.abs() < EPS {
            return;
        }
        let seg = last - 1;
        if self.segment_free(seg) {
            return;
        }
        if (self.drawing() && seg == self.act_line) || n == 2 {
            let elbow = if dx.abs() > dy.abs() {
                Point::new(p2.x, p1.y)
            } else {
                Point::new(p1.x, p2.y)
            };
            self.points.insert(last, elbow);
        } else {
            let prev = self.points[last - 2];
            if is_vert(prev, p1) {
                self.points[last - 1] = Point::new(p1.x, p2.y);
            } else if is_horiz(prev, p1) {
                self.points[last - 1] = Point::new(p2.x, p1.y);
            } else {
                // Neighbor is diagonal or zero-length: insert an orthogonal
                // elbow so we never warp an angled segment.
                let elbow = if dx.abs() > dy.abs() {
                    Point::new(p2.x, p1.y)
                } else {
                    Point::new(p1.x, p2.y)
                };
                self.points.insert(last, elbow);
            }
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
        if dx.abs() < EPS || dy.abs() < EPS {
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
            if is_vert(p2, next) {
                self.points[1] = Point::new(p2.x, p1.y);
            } else if is_horiz(p2, next) {
                self.points[1] = Point::new(p1.x, p2.y);
            } else {
                // Neighbor is diagonal or zero-length: insert an orthogonal
                // elbow so we never warp an angled segment.
                let elbow = if dx.abs() > dy.abs() {
                    Point::new(p2.x, p1.y)
                } else {
                    Point::new(p1.x, p2.y)
                };
                self.points.insert(1, elbow);
                self.free_diag.insert(0, false);
            }
        }
        self.sync_meta();
    }

    fn drop_nulls(&mut self) {
        let mut i = 1;
        while i < self.points.len() {
            let a = self.points[i - 1];
            let b = self.points[i];
            if pts_eq(a, b) && self.points.len() > 2 {
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
            let merge = if d0 && d1 {
                is_collinear_diag(p0, p1, p2)
            } else if !d0 && !d1 {
                let dx0 = p1.x - p0.x;
                let dy0 = p1.y - p0.y;
                let dx1 = p2.x - p1.x;
                let dy1 = p2.y - p1.y;
                let aligned =
                    (dx0.abs() < EPS && dx1.abs() < EPS) || (dy0.abs() < EPS && dy1.abs() < EPS);
                aligned && !pts_eq(p1, p2)
            } else {
                false
            };
            if merge {
                self.points.remove(i);
                if i < self.free_diag.len() {
                    let keep_diag = d0 && d1;
                    self.free_diag.remove(i);
                    if keep_diag && i - 1 < self.free_diag.len() {
                        self.free_diag[i - 1] = true;
                    }
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
        if is_horiz(a, b) {
            self.points[seg].y += delta.y;
            self.points[seg + 1].y += delta.y;
        } else if is_vert(a, b) {
            self.points[seg].x += delta.x;
            self.points[seg + 1].x += delta.x;
        }
    }

    fn segment_free(&self, seg: usize) -> bool {
        self.free_diag.get(seg).copied().unwrap_or(false)
    }

    fn mark_free(&mut self, seg: usize) {
        self.sync_meta();
        if let Some(f) = self.free_diag.get_mut(seg)
            && let (Some(&a), Some(&b)) = (self.points.get(seg), self.points.get(seg + 1))
        {
            *f = is_diag(a, b);
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

#[inline]
fn pts_eq(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

#[inline]
fn is_horiz(a: Point, b: Point) -> bool {
    (a.y - b.y).abs() < EPS && (a.x - b.x).abs() >= EPS
}

#[inline]
fn is_vert(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() >= EPS
}

#[inline]
fn is_diag(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() >= EPS && (a.y - b.y).abs() >= EPS
}

fn is_collinear_diag(a: Point, b: Point, c: Point) -> bool {
    let v0x = b.x - a.x;
    let v0y = b.y - a.y;
    let v1x = c.x - b.x;
    let v1y = c.y - b.y;
    let len0 = v0x.hypot(v0y);
    let len1 = v1x.hypot(v1y);
    if len0 < EPS || len1 < EPS {
        return false;
    }
    let dot = v0x * v1x + v0y * v1y;
    if dot <= 0.0 {
        return false;
    }
    let cross = (v0x * v1y - v0y * v1x).abs();
    cross / (len0 * len1) < 1e-3
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
        let (a, b, pt) = w
            .split(
                Point::new(16.0, 0.0),
                "Connector-2".into(),
                "N-0".into(),
                "N-1".into(),
            )
            .unwrap();
        assert!(a.is_bus && b.is_bus);
        assert_eq!(pt, Point::new(16.0, 0.0));
    }

    #[test]
    fn shift_release_squares_up_immediately() {
        let mut w = Wire::start("Connector-1", "A", Point::new(0.0, 0.0));
        w.route_end(Point::new(40.0, 24.0), true);
        assert_eq!(w.points.len(), 2);
        assert!(is_diag(w.points[0], w.points[1]));

        // Releasing Shift must immediately snap back to 90° orthogonal.
        w.route_end(Point::new(44.0, 24.0), false);
        assert_eq!(w.points.len(), 3);
        for seg in w.points.windows(2) {
            assert!(
                !is_diag(seg[0], seg[1]),
                "unexpected diagonal: {:?}",
                w.points
            );
        }
    }

    #[test]
    fn shift_press_collapses_uncommitted_elbow() {
        let mut w = Wire::start("Connector-1", "A", Point::new(0.0, 0.0));
        w.route_end(Point::new(40.0, 24.0), false);
        assert_eq!(w.points.len(), 3);

        // Pressing Shift collapses the uncommitted L-bend into one straight angled segment.
        w.route_end(Point::new(40.0, 24.0), true);
        assert_eq!(w.points, vec![Point::new(0.0, 0.0), Point::new(40.0, 24.0)]);
    }

    #[test]
    fn route_no_overshoot_backtrack() {
        let mut w = Wire::start("Connector-1", "A", Point::new(16.0, 0.0));
        w.route_end(Point::new(80.0, 16.0), false);
        assert_eq!(
            w.points,
            vec![
                Point::new(16.0, 0.0),
                Point::new(80.0, 0.0),
                Point::new(80.0, 16.0)
            ]
        );
        // Returning back onto y=0 at x=64 must NOT leave a spike out to x=80.
        w.route_end(Point::new(64.0, 0.0), false);
        assert_eq!(w.points, vec![Point::new(16.0, 0.0), Point::new(64.0, 0.0)]);
    }

    #[test]
    fn close_without_shift_never_creates_diagonal() {
        let mut w = Wire::start("Connector-1", "A", Point::new(0.0, 0.0));
        w.route_end(Point::new(40.0, 24.0), true);
        w.close_with_shift("B", Point::new(40.0, 24.0), false);
        for seg in w.points.windows(2) {
            assert!(
                !is_diag(seg[0], seg[1]),
                "unexpected diagonal: {:?}",
                w.points
            );
        }
        assert_eq!(*w.points.last().unwrap(), Point::new(40.0, 24.0));
    }

    #[test]
    fn corner_drag_zero_length_never_creates_diagonal() {
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
        // Collapse the vertical segment to zero length, then drag diagonally without Shift.
        let v = w.drag_corner(1, Point::new(0.0, -16.0), false);
        let _v = w.drag_corner(v, Point::new(8.0, 8.0), false);
        for seg in w.points.windows(2) {
            assert!(
                !is_diag(seg[0], seg[1]),
                "unexpected diagonal: {:?}",
                w.points
            );
        }
        w.reanchor(Point::new(0.0, 0.0), Point::new(32.0, 16.0));
        for seg in w.points.windows(2) {
            assert!(
                !is_diag(seg[0], seg[1]),
                "diagonal after reanchor: {:?}",
                w.points
            );
        }
    }

    #[test]
    fn angled_segment_bend_and_straighten() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![Point::new(0.0, 0.0), Point::new(40.0, 20.0)],
        );
        let hit = w.drag_segment_at(0, Point::new(0.0, 12.0), Point::new(20.0, 10.0), false);
        assert_eq!(hit, WireHit::Corner(1));
        assert_eq!(w.points[0], Point::new(0.0, 0.0));
        assert_eq!(w.points[1], Point::new(20.0, 22.0));
        assert_eq!(w.points[2], Point::new(40.0, 20.0));

        // Dragging the bent corner back onto the straight line merges back to 2 points on reanchor.
        w.drag_corner(1, Point::new(0.0, -12.0), false);
        w.reanchor(Point::new(0.0, 0.0), Point::new(40.0, 20.0));
        assert_eq!(w.points, vec![Point::new(0.0, 0.0), Point::new(40.0, 20.0)]);
    }

    #[test]
    fn angled_chamfer_parallel_slide_preserves_slope() {
        // H - 45° Diag - V miter
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![
                Point::new(0.0, 0.0),
                Point::new(16.0, 0.0),
                Point::new(32.0, 16.0),
                Point::new(32.0, 48.0),
            ],
        );
        let hit = w.drag_segment_at(1, Point::new(4.0, 0.0), Point::new(24.0, 8.0), false);
        assert_eq!(hit, WireHit::Segment(1));
        assert_eq!(w.points[1], Point::new(20.0, 0.0));
        assert_eq!(w.points[2], Point::new(32.0, 12.0));
        assert!(w.segment_is_horizontal(0));
        assert!(w.segment_is_vertical(2));
        // Slope remains 1:1 (45°).
        let dx = w.points[2].x - w.points[1].x;
        let dy = w.points[2].y - w.points[1].y;
        assert!((dx - dy).abs() < 1e-6);

        // Sliding all the way past the corner collapses cleanly into a 90° corner.
        w.drag_segment_at(1, Point::new(20.0, 0.0), Point::new(24.0, 8.0), false);
        w.reanchor(Point::new(0.0, 0.0), Point::new(32.0, 48.0));
        assert_eq!(
            w.points,
            vec![
                Point::new(0.0, 0.0),
                Point::new(32.0, 0.0),
                Point::new(32.0, 48.0)
            ]
        );
    }

    #[test]
    fn mixed_angled_and_orthogonal_corner_drag() {
        let mut w = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![
                Point::new(0.0, 0.0),
                Point::new(16.0, 16.0),
                Point::new(48.0, 16.0),
            ],
        );
        let v = w.drag_corner(1, Point::new(8.0, 8.0), false);
        assert_eq!(w.points[0], Point::new(0.0, 0.0));
        assert_eq!(w.points[v], Point::new(24.0, 24.0));
        // Angled segment pivoted; orthogonal segment stayed horizontal (with stub to B).
        assert!(w.segment_is_diagonal(v - 1));
        assert!(w.segment_is_horizontal(v));
        assert_eq!(*w.points.last().unwrap(), Point::new(48.0, 16.0));
    }

    #[test]
    fn split_projects_onto_horizontal_and_angled_wires() {
        let h = Wire::from_saved(
            "Connector-1",
            "A",
            "B",
            vec![Point::new(16.0, 0.0), Point::new(64.0, 0.0)],
        );
        let (a, b, pt) = h
            .split(
                Point::new(40.0, 3.0),
                "Connector-2".into(),
                "N-0".into(),
                "N-2".into(),
            )
            .unwrap();
        assert_eq!(pt, Point::new(40.0, 0.0));
        assert_eq!(*a.points.last().unwrap(), pt);
        assert_eq!(b.points[0], pt);

        let diag = Wire::from_saved(
            "Connector-3",
            "A",
            "B",
            vec![Point::new(0.0, 0.0), Point::new(40.0, 20.0)],
        );
        let (da, db, dpt) = diag
            .split(
                Point::new(20.0, 12.0),
                "Connector-4".into(),
                "N2-0".into(),
                "N2-2".into(),
            )
            .unwrap();
        // Projected point lies strictly on y = 0.5 * x.
        assert!(
            (dpt.y - 0.5 * dpt.x).abs() < 1e-6,
            "off angled wire: {dpt:?}"
        );
        assert_eq!(*da.points.last().unwrap(), dpt);
        assert_eq!(db.points[0], dpt);
    }
}
