//! Ground: one-terminal 0 V reference.

use super::component::stamp_to_ground;
use super::props::PropDef;
use super::{CompPin, Component, Stampable};
use crate::CERO_DOUB;
use crate::canvas::Rect;
use crate::elements::{Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;

/// Ground `m_area = QRect(-8, -10, 16, 12)`.
pub const GROUND_BODY: Rect = Rect {
    x: -8.0,
    y: -10.0,
    w: 16.0,
    h: 12.0,
};

impl crate::canvas::Item {
    pub fn ground(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(id, x, y, Ground)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ground;

impl Ground {
    pub const TYPE_ID: &'static str = "Ground";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ground
    }
}

impl Component for Ground {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Ground reference (0 V)."
    }
    fn props() -> &'static [PropDef<Self>] {
        &[]
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![CompPin::new("-Gnd", 0.0, -16.0, 90, 8.0)]
    }
    fn body(&self) -> Rect {
        GROUND_BODY
    }
}

impl Stampable for Ground {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_to_ground(matrix, pin_nodes, 0, CERO_DOUB, SOURCE_ADMIT);
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Ground {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_rect(-6.6, -8.5, 13.2, COMPONENT_BORDER_WIDTH, ctx.pal.border);
        d.fill_rect(-4.3, -4.5, 8.6, COMPONENT_BORDER_WIDTH, ctx.pal.border);
        d.fill_rect(-1.9, -0.5, 3.8, COMPONENT_BORDER_WIDTH, ctx.pal.border);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_ground(&mut self, x: f64, y: f64) -> String {
        let id = format!("Ground-{}", self.next_ground);
        self.next_ground += 1;
        self.items.push(crate::canvas::Item::ground(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{GraphicAttrs, write_component_item};

    #[test]
    fn default_has_no_persist_props() {
        let g = Ground;
        assert!(Ground::props().is_empty());
        assert_eq!(g.type_id(), "Ground");
        assert_eq!(g.pin_geoms().len(), 1);
        let line = write_component_item("Ground-1", &g, &GraphicAttrs::at(0.0, 0.0));
        assert!(line.contains("itemtype=\"Ground\""));
        assert!(!line.contains("Voltage="));
    }

    #[test]
    fn stamp_holds_near_zero() {
        let g = Ground;
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        g.stamp(&mut m, &[0], 0.0);
        let mut v = vec![1.0];
        assert!(m.solve(&mut v));
        assert!(v[0].abs() < 1e-8, "{}", v[0]);
    }
}
