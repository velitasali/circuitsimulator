//! DrawRecorder for testing component canvas drawing commands deterministically.

use crate::canvas::Canvas;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Color, Draw, PaintCtx, Palette};
use crate::canvas::scene::Item;
use crate::components::{Drawable, Part};

#[derive(Clone, Debug, PartialEq)]
pub enum DrawOp {
    FillRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color,
    },
    StrokeRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color,
        width: f64,
    },
    FillRoundRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: f64,
        color: Color,
    },
    StrokeRoundRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: f64,
        color: Color,
        width: f64,
    },
    FillCircle {
        cx: f64,
        cy: f64,
        r: f64,
        color: Color,
    },
    StrokeCircle {
        cx: f64,
        cy: f64,
        r: f64,
        color: Color,
        width: f64,
    },
    FillEllipse {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color,
    },
    StrokeEllipse {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color,
        width: f64,
    },
    FillPoly {
        pts: Vec<[f64; 2]>,
        color: Color,
    },
    StrokePoly {
        pts: Vec<[f64; 2]>,
        color: Color,
        width: f64,
        close: bool,
    },
    Polyline {
        pts: Vec<[f64; 2]>,
        color: Color,
        width: f64,
        dash: bool,
    },
    Line {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        color: Color,
        width: f64,
    },
    Arc {
        cx: f64,
        cy: f64,
        r: f64,
        a0: f64,
        a1: f64,
        color: Color,
        width: f64,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        color: Color,
        align: Align,
    },
    TextBold {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        color: Color,
        align: Align,
    },
    GridDots {
        scene: Rect,
        step: i32,
        color: Color,
    },
    Push {
        x: f64,
        y: f64,
        rot: f64,
        sx: f64,
        sy: f64,
    },
    Pop,
    PushClipRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    PopClip,
    BlitPixmap {
        x: f64,
        y: f64,
        pm_scale: f64,
    },
    DrawPixmapRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        opacity: f64,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawRecorder {
    pub ops: Vec<DrawOp>,
}

impl Draw for DrawRecorder {
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color) {
        self.ops.push(DrawOp::FillRect { x, y, w, h, color });
    }

    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color, width: f64) {
        self.ops.push(DrawOp::StrokeRect {
            x,
            y,
            w,
            h,
            color,
            width,
        });
    }

    fn fill_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, color: Color) {
        self.ops.push(DrawOp::FillRoundRect {
            x,
            y,
            w,
            h,
            r,
            color,
        });
    }

    fn stroke_round_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: f64,
        color: Color,
        width: f64,
    ) {
        self.ops.push(DrawOp::StrokeRoundRect {
            x,
            y,
            w,
            h,
            r,
            color,
            width,
        });
    }

    fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, color: Color) {
        self.ops.push(DrawOp::FillCircle { cx, cy, r, color });
    }

    fn stroke_circle(&mut self, cx: f64, cy: f64, r: f64, color: Color, width: f64) {
        self.ops.push(DrawOp::StrokeCircle {
            cx,
            cy,
            r,
            color,
            width,
        });
    }

    fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color) {
        self.ops.push(DrawOp::FillEllipse { x, y, w, h, color });
    }

    fn stroke_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color, width: f64) {
        self.ops.push(DrawOp::StrokeEllipse {
            x,
            y,
            w,
            h,
            color,
            width,
        });
    }

    fn fill_poly(&mut self, pts: &[[f64; 2]], color: Color) {
        self.ops.push(DrawOp::FillPoly {
            pts: pts.to_vec(),
            color,
        });
    }

    fn stroke_poly(&mut self, pts: &[[f64; 2]], color: Color, width: f64, close: bool) {
        self.ops.push(DrawOp::StrokePoly {
            pts: pts.to_vec(),
            color,
            width,
            close,
        });
    }

    fn polyline(&mut self, pts: &[[f64; 2]], color: Color, width: f64, dash: bool) {
        self.ops.push(DrawOp::Polyline {
            pts: pts.to_vec(),
            color,
            width,
            dash,
        });
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: Color, width: f64) {
        self.ops.push(DrawOp::Line {
            x0,
            y0,
            x1,
            y1,
            color,
            width,
        });
    }

    fn arc(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64, color: Color, width: f64) {
        self.ops.push(DrawOp::Arc {
            cx,
            cy,
            r,
            a0,
            a1,
            color,
            width,
        });
    }

    fn text(&mut self, x: f64, y: f64, s: &str, size: f64, color: Color, align: Align) {
        self.ops.push(DrawOp::Text {
            x,
            y,
            text: s.to_string(),
            size,
            color,
            align,
        });
    }

    fn text_bold(&mut self, x: f64, y: f64, s: &str, size: f64, color: Color, align: Align) {
        self.ops.push(DrawOp::TextBold {
            x,
            y,
            text: s.to_string(),
            size,
            color,
            align,
        });
    }

    fn grid_dots(&mut self, scene: Rect, step: i32, color: Color) {
        self.ops.push(DrawOp::GridDots { scene, step, color });
    }

    fn push(&mut self, x: f64, y: f64, rot: f64, sx: f64, sy: f64) {
        self.ops.push(DrawOp::Push { x, y, rot, sx, sy });
    }

    fn pop(&mut self) {
        self.ops.push(DrawOp::Pop);
    }

    fn push_clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.ops.push(DrawOp::PushClipRect { x, y, w, h });
    }

    fn pop_clip(&mut self) {
        self.ops.push(DrawOp::PopClip);
    }

    fn blit_pixmap(&mut self, x: f64, y: f64, _pm: &tiny_skia::Pixmap, pm_scale: f64) -> bool {
        self.ops.push(DrawOp::BlitPixmap { x, y, pm_scale });
        true
    }

    fn draw_pixmap_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        _pm: &tiny_skia::Pixmap,
        opacity: f64,
    ) -> bool {
        self.ops.push(DrawOp::DrawPixmapRect {
            x,
            y,
            w,
            h,
            opacity,
        });
        true
    }
}

#[allow(dead_code)]
impl DrawRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn contains_text(&self, needle: &str) -> bool {
        self.ops.iter().any(|op| match op {
            DrawOp::Text { text, .. } | DrawOp::TextBold { text, .. } => text.contains(needle),
            _ => false,
        })
    }

    pub fn all_text(&self) -> Vec<String> {
        self.ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Text { text, .. } | DrawOp::TextBold { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn has_color(&self, color: Color) -> bool {
        self.ops.iter().any(|op| match op {
            DrawOp::FillRect { color: c, .. }
            | DrawOp::StrokeRect { color: c, .. }
            | DrawOp::FillRoundRect { color: c, .. }
            | DrawOp::StrokeRoundRect { color: c, .. }
            | DrawOp::FillCircle { color: c, .. }
            | DrawOp::StrokeCircle { color: c, .. }
            | DrawOp::FillEllipse { color: c, .. }
            | DrawOp::StrokeEllipse { color: c, .. }
            | DrawOp::FillPoly { color: c, .. }
            | DrawOp::StrokePoly { color: c, .. }
            | DrawOp::Polyline { color: c, .. }
            | DrawOp::Line { color: c, .. }
            | DrawOp::Arc { color: c, .. }
            | DrawOp::Text { color: c, .. }
            | DrawOp::TextBold { color: c, .. }
            | DrawOp::GridDots { color: c, .. } => *c == color,
            DrawOp::Push { .. }
            | DrawOp::Pop
            | DrawOp::PushClipRect { .. }
            | DrawOp::PopClip
            | DrawOp::BlitPixmap { .. }
            | DrawOp::DrawPixmapRect { .. } => false,
        })
    }

    pub fn is_all_finite(&self) -> bool {
        self.ops.iter().all(|op| match op {
            DrawOp::FillRect { x, y, w, h, .. } | DrawOp::StrokeRect { x, y, w, h, .. } => {
                x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()
            }
            DrawOp::FillRoundRect { x, y, w, h, r, .. }
            | DrawOp::StrokeRoundRect { x, y, w, h, r, .. } => {
                x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite() && r.is_finite()
            }
            DrawOp::FillCircle { cx, cy, r, .. } | DrawOp::StrokeCircle { cx, cy, r, .. } => {
                cx.is_finite() && cy.is_finite() && r.is_finite()
            }
            DrawOp::FillEllipse { x, y, w, h, .. } | DrawOp::StrokeEllipse { x, y, w, h, .. } => {
                x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()
            }
            DrawOp::FillPoly { pts, .. }
            | DrawOp::StrokePoly { pts, .. }
            | DrawOp::Polyline { pts, .. } => {
                pts.iter().all(|[x, y]| x.is_finite() && y.is_finite())
            }
            DrawOp::Line {
                x0,
                y0,
                x1,
                y1,
                width,
                ..
            } => {
                x0.is_finite()
                    && y0.is_finite()
                    && x1.is_finite()
                    && y1.is_finite()
                    && width.is_finite()
            }
            DrawOp::Arc {
                cx,
                cy,
                r,
                a0,
                a1,
                width,
                ..
            } => {
                cx.is_finite()
                    && cy.is_finite()
                    && r.is_finite()
                    && a0.is_finite()
                    && a1.is_finite()
                    && width.is_finite()
            }
            DrawOp::Text { x, y, size, .. } | DrawOp::TextBold { x, y, size, .. } => {
                x.is_finite() && y.is_finite() && size.is_finite()
            }
            DrawOp::GridDots { scene, .. } => {
                scene.x.is_finite()
                    && scene.y.is_finite()
                    && scene.w.is_finite()
                    && scene.h.is_finite()
            }
            DrawOp::Push { x, y, rot, sx, sy } => {
                x.is_finite()
                    && y.is_finite()
                    && rot.is_finite()
                    && sx.is_finite()
                    && sy.is_finite()
            }
            DrawOp::Pop => true,
            DrawOp::PushClipRect { x, y, w, h } => {
                x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()
            }
            DrawOp::PopClip => true,
            DrawOp::BlitPixmap { x, y, pm_scale } => {
                x.is_finite() && y.is_finite() && pm_scale.is_finite()
            }
            DrawOp::DrawPixmapRect {
                x,
                y,
                w,
                h,
                opacity,
            } => {
                x.is_finite()
                    && y.is_finite()
                    && w.is_finite()
                    && h.is_finite()
                    && opacity.is_finite()
            }
        })
    }
}

pub fn record_part_paint(part: &Part) -> DrawRecorder {
    let canvas = Canvas::new();
    let pal = Palette::light();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: "test-item",
    };
    let mut recorder = DrawRecorder::new();
    part.paint(&mut recorder, &ctx);
    recorder
}

pub fn record_drawable_paint<T: Drawable>(drawable: &T) -> DrawRecorder {
    let canvas = Canvas::new();
    let pal = Palette::light();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: "test-item",
    };
    let mut recorder = DrawRecorder::new();
    drawable.paint(&mut recorder, &ctx);
    recorder
}

#[allow(dead_code)]
pub fn record_item_paint(item: &Item) -> DrawRecorder {
    let canvas = Canvas::new();
    let pal = Palette::light();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: &item.id,
    };
    let mut recorder = DrawRecorder::new();
    let connected = rustc_hash::FxHashSet::default();
    crate::canvas::export::paint::paint_item(&mut recorder, &ctx, canvas.scene(), item, &connected);
    recorder
}
