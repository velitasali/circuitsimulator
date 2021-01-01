//! Named net tunnel connector.

use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl crate::canvas::Item {
    pub fn tunnel(id: impl Into<String>, x: f64, y: f64, name: impl Into<String>) -> Self {
        Self::new(
            id,
            x,
            y,
            Tunnel {
                name: name.into(),
                is_bus: false,
                show: true,
            },
        )
    }
}

/// Net tunnel connector.
#[derive(Clone, Debug, PartialEq)]
pub struct Tunnel {
    pub name: String,
    pub is_bus: bool,
    pub show: bool,
}

impl Default for Tunnel {
    fn default() -> Self {
        Self {
            name: String::new(),
            is_bus: false,
            show: true,
        }
    }
}

impl Tunnel {
    pub const TYPE_ID: &'static str = "Tunnel";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Tunnel {
            name: self.name.clone(),
            pin_id: String::new(),
        }
    }

    fn get_name(&self) -> PropValue {
        PropValue::String(self.name.clone())
    }
    fn set_name(&mut self, v: PropValue) -> Result<(), PropError> {
        self.name = expect_string("Name", v)?;
        Ok(())
    }

    fn get_is_bus(&self) -> PropValue {
        PropValue::Bool(self.is_bus)
    }
    fn set_is_bus(&mut self, v: PropValue) -> Result<(), PropError> {
        self.is_bus = expect_bool("IsBus", v)?;
        Ok(())
    }
}

impl Component for Tunnel {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Net tunnel connector."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Tunnel>] = &[
            PropDef::string("Name", "Net Name", Tunnel::get_name, Tunnel::set_name)
                .with_info("Identifier, all Tunnels with this Id will be connected togeter."),
            PropDef::bool("IsBus", "Bus Net", Tunnel::get_is_bus, Tunnel::set_is_bus)
                .with_info("Whether this tunnel connects to a Bus or not."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        tunnel_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        let size = if self.name.is_empty() {
            20.0
        } else {
            crate::canvas::snap_to_grid4((self.name.len() as i32 * 5 + 4).max(20)) as f64
        };
        Rect::new(-size - 8.0, -4.0, size + 4.0, 8.0)
    }
}

impl Stampable for Tunnel {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for Tunnel {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let size = if self.name.is_empty() {
            20.0
        } else {
            crate::canvas::snap_to_grid4((self.name.len() as i32 * 5 + 4).max(20)) as f64
        };
        let pts = [
            [-size - 8.0, -4.0],
            [-8.0, -4.0],
            [-4.0, 0.0],
            [-8.0, 4.0],
            [-size - 8.0, 4.0],
        ];
        let fill = if !self.name.is_empty() {
            if self.is_bus {
                ctx.pal.tunnel_color1
            } else {
                ctx.pal.tunnel_color2
            }
        } else {
            ctx.pal.tunnel_color3
        };
        d.fill_poly(&pts, fill.fade(COMPONENT_FILL_ALPHA));
        d.stroke_poly(&pts, ctx.pal.border, COMPONENT_BORDER_WIDTH, true);
        let text_x = -(size / 2.0 + 8.0);
        d.text(text_x, 0.0, &self.name, 7.0, ctx.pal.border, Align::Center);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_tunnel(&mut self, x: f64, y: f64) -> String {
        let id = format!("Tunnel-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::tunnel(&id, x, y, "net"));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tunnel() {
        let t = Tunnel::default();
        assert_eq!(t.type_id(), "Tunnel");
        assert!(t.name.is_empty());
        assert!(!t.is_bus);
        assert!(t.show);
        assert_eq!(t.pin_geoms().len(), 1);
    }

    #[test]
    fn tunnel_body_scales_with_name() {
        let mut t = Tunnel::default();
        let b0 = t.body();
        t.name = "LONG_BUS_NET_NAME_12345".into();
        let b1 = t.body();
        assert!(b1.w > b0.w);
    }

    #[test]
    fn tunnel_element_kind() {
        let t = Tunnel {
            name: "CLK".into(),
            is_bus: false,
            show: true,
        };
        match t.to_element_kind() {
            Kind::Tunnel { name, .. } => assert_eq!(name, "CLK"),
            other => panic!("expected Kind::Tunnel, got {other:?}"),
        }
    }
}

const TUNNEL_PINS: [PinGeom; 1] = [PinGeom {
    suffix: "-pin",
    x: 0.0,
    y: 0.0,
    angle: 0,
    length: 4.0,
    direction: None,
}];

fn tunnel_pins() -> &'static [PinGeom] {
    &TUNNEL_PINS
}
