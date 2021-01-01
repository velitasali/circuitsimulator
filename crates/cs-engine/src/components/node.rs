//! Node: wire junction with 3 zero-length pins.

use super::props::PropDef;
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

impl crate::canvas::Item {
    pub fn node(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(id, x, y, Node)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Node;

impl Node {
    pub const TYPE_ID: &'static str = "Node";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Junction
    }
}

impl Component for Node {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Wire junction."
    }
    fn props() -> &'static [PropDef<Self>] {
        &[]
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        vec![
            CompPin::new("-0", 0.0, 0.0, 0, 0.0),
            CompPin::new("-1", 0.0, 0.0, 90, 0.0),
            CompPin::new("-2", 0.0, 0.0, 180, 0.0),
        ]
    }
    fn body(&self) -> Rect {
        Rect::new(-4.0, -4.0, 8.0, 8.0)
    }
}

impl Stampable for Node {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};

impl Drawable for Node {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let is_bus = ctx.canvas.scene().pin_is_bus(&format!("{}-0", ctx.item_id));
        let r = if is_bus { 2.4 } else { 1.5 };
        let is_selected = ctx
            .canvas
            .scene()
            .item_by_id(ctx.item_id)
            .is_some_and(|it| it.selected);
        let color = if is_selected {
            ctx.pal.band
        } else if is_bus {
            ctx.pal.wire
        } else if ctx.canvas.sim_running() && ctx.canvas.circ_settings().animate_logic {
            let v = ctx
                .canvas
                .pin_voltage(&format!("{}-0", ctx.item_id))
                .or_else(|| ctx.canvas.pin_voltage(&format!("{}-1", ctx.item_id)))
                .or_else(|| ctx.canvas.pin_voltage(&format!("{}-2", ctx.item_id)))
                .unwrap_or(0.0);
            if v > 2.5 {
                ctx.pal.pin_high
            } else {
                ctx.pal.pin_low
            }
        } else {
            ctx.pal.wire
        };
        d.fill_circle(0.0, 0.0, r, color);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_node(&mut self, x: f64, y: f64) -> String {
        let id = format!("Node-{}", self.next_node);
        self.next_node += 1;
        self.items.push(crate::canvas::Item::node(&id, x, y));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{GraphicAttrs, write_component_item};

    #[test]
    fn default_node_properties() {
        let n = Node;
        assert!(Node::props().is_empty());
        assert_eq!(n.type_id(), "Node");
        assert_eq!(n.pin_geoms().len(), 3);
        let line = write_component_item("Node-1", &n, &GraphicAttrs::at(0.0, 0.0));
        assert!(line.contains("itemtype=\"Node\""));
    }
}
