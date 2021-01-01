//! KY-023 dual-axis joystick module.

use super::component::stamp_to_ground;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::canvas::{Point, Rect};
use crate::elements::{Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_AXIS: f64 = -25.0;
const MAX_AXIS: f64 = 25.0;

impl crate::canvas::Item {
    pub fn ky023(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            KY023 {
                stick_x: 0.5,
                stick_y: 0.5,
                btn_down: false,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KY023 {
    pub stick_x: f64,
    pub stick_y: f64,
    pub btn_down: bool,
}

impl Default for KY023 {
    fn default() -> Self {
        Self {
            stick_x: 0.0,
            stick_y: 0.0,
            btn_down: false,
        }
    }
}

impl KY023 {
    pub const TYPE_ID: &'static str = "KY023";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Ky023 {
            stick_x: self.stick_x,
            stick_y: self.stick_y,
            btn_down: self.btn_down,
        }
    }

    fn get_stick_x(&self) -> PropValue {
        PropValue::Float(self.stick_x)
    }
    fn set_stick_x(&mut self, v: PropValue) -> Result<(), PropError> {
        self.stick_x = expect_float("StickX", v)?.clamp(MIN_AXIS, MAX_AXIS);
        Ok(())
    }

    fn get_stick_y(&self) -> PropValue {
        PropValue::Float(self.stick_y)
    }
    fn set_stick_y(&mut self, v: PropValue) -> Result<(), PropError> {
        self.stick_y = expect_float("StickY", v)?.clamp(MIN_AXIS, MAX_AXIS);
        Ok(())
    }

    fn get_btn_down(&self) -> PropValue {
        PropValue::Bool(self.btn_down)
    }
    fn set_btn_down(&mut self, v: PropValue) -> Result<(), PropError> {
        self.btn_down = expect_bool("BtnDown", v)?;
        Ok(())
    }
}

impl Component for KY023 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Resistive joystick."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<KY023>] = &[
            PropDef::float(
                "StickX",
                "Stick X",
                "",
                MIN_AXIS,
                MAX_AXIS,
                KY023::get_stick_x,
                KY023::set_stick_x,
            )
            .with_info("Analog X-axis joystick deflection value."),
            PropDef::float(
                "StickY",
                "Stick Y",
                "",
                MIN_AXIS,
                MAX_AXIS,
                KY023::get_stick_y,
                KY023::set_stick_y,
            )
            .with_info("Analog Y-axis joystick deflection value."),
            PropDef::bool(
                "BtnDown",
                "Button Down",
                KY023::get_btn_down,
                KY023::set_btn_down,
            )
            .with_info("Pushbutton contact state (pressed / released)."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ky023_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-20.0, -28.0, 40.0, 56.0)
    }

    fn interact_press(&mut self, local: Point) -> bool {
        let btn_rect = Rect::new(8.0, 15.0, 10.0, 10.0);
        if btn_rect.contains_point(local) {
            self.btn_down = true;
            return true;
        }
        let d = Point::new(local.x, local.y - (-8.0));
        let stick_pos = Point::new(self.stick_x, -8.0 + self.stick_y);
        let dist_to_stick = (local.x - stick_pos.x).hypot(local.y - stick_pos.y);
        if dist_to_stick <= 10.0 || d.x.hypot(d.y) <= 17.0 {
            let len = d.x.hypot(d.y);
            let (sx, sy) = if len > 13.0 {
                (d.x * 13.0 / len, d.y * 13.0 / len)
            } else {
                (d.x, d.y)
            };
            self.stick_x = sx;
            self.stick_y = sy;
            return true;
        }
        false
    }

    fn interact_move(&mut self, local: Point) -> bool {
        if self.btn_down {
            return false;
        }
        let d = Point::new(local.x, local.y - (-8.0));
        let len = d.x.hypot(d.y);
        let (sx, sy) = if len > 13.0 {
            (d.x * 13.0 / len, d.y * 13.0 / len)
        } else {
            (d.x, d.y)
        };
        if (self.stick_x - sx).abs() > 1e-4 || (self.stick_y - sy).abs() > 1e-4 {
            self.stick_x = sx;
            self.stick_y = sy;
        }
        true
    }

    fn interact_release(&mut self, _local: Point) -> bool {
        if self.btn_down {
            self.btn_down = false;
            true
        } else if self.stick_x != 0.0 || self.stick_y != 0.0 {
            self.stick_x = 0.0;
            self.stick_y = 0.0;
            true
        } else {
            false
        }
    }

    fn interact_cancel(&mut self) -> bool {
        let changed = self.btn_down || self.stick_x != 0.0 || self.stick_y != 0.0;
        self.btn_down = false;
        self.stick_x = 0.0;
        self.stick_y = 0.0;
        changed
    }
}

impl crate::canvas::Scene {
    pub fn add_ky023(&mut self, x: f64, y: f64) -> String {
        let id = format!("KY023-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::ky023(&id, x, y));
        id
    }

    pub fn set_joystick_pos(&mut self, uid: &str, x: f64, y: f64) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::KY023(p) = &mut item.kind {
                let cx = x.clamp(-25.0, 25.0);
                let cy = y.clamp(-25.0, 25.0);
                if (p.stick_x - cx).abs() < 1e-4 && (p.stick_y - cy).abs() < 1e-4 {
                    return false;
                }
                p.stick_x = cx;
                p.stick_y = cy;
                return true;
            }
        }
        false
    }

    pub fn set_joystick_button(&mut self, uid: &str, down: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::KY023(p) = &mut item.kind {
                if p.btn_down == down {
                    return false;
                }
                p.btn_down = down;
                return true;
            }
        }
        false
    }
}

impl Stampable for KY023 {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let legacy_scale = self.stick_x.abs() > 1.0 || self.stick_y.abs() > 1.0;
        let x_norm = if legacy_scale {
            ((self.stick_x + 25.0) / 50.0).clamp(0.0, 1.0)
        } else {
            ((self.stick_x + 1.0) / 2.0).clamp(0.0, 1.0)
        };
        let y_norm = if legacy_scale {
            ((self.stick_y + 25.0) / 50.0).clamp(0.0, 1.0)
        } else {
            ((self.stick_y + 1.0) / 2.0).clamp(0.0, 1.0)
        };
        stamp_to_ground(matrix, pin_nodes, 0, 5.0 * x_norm, SOURCE_ADMIT);
        stamp_to_ground(matrix, pin_nodes, 1, 5.0 * y_norm, SOURCE_ADMIT);
        if self.btn_down {
            stamp_to_ground(matrix, pin_nodes, 2, 0.0, 1000.0);
        } else {
            stamp_to_ground(matrix, pin_nodes, 2, 5.0, 1.0 / 2000.0);
        }
    }
}

impl Drawable for KY023 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(
            -20.0,
            -28.0,
            40.0,
            56.0,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -20.0,
            -28.0,
            40.0,
            56.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Base circle at (0, -8)
        d.fill_circle(0.0, -8.0, 17.0, ctx.pal.border.fade(0.15));
        d.stroke_circle(0.0, -8.0, 17.0, ctx.pal.border, 1.0);

        // Thumb stick at (0, -8) + (stick_x, stick_y)
        let sx = self.stick_x.clamp(-12.0, 12.0);
        let sy = -8.0 + self.stick_y.clamp(-12.0, 12.0);
        d.fill_circle(sx, sy, 5.1, ctx.pal.border);

        // Push button at (8, 15, 10, 10)
        let btn_fill = if self.btn_down {
            ctx.pal.pin_open_high.fade(0.7)
        } else {
            ctx.pal.body.fade(0.8)
        };
        d.fill_round_rect(8.0, 15.0, 10.0, 10.0, 2.0, btn_fill);
        d.stroke_round_rect(
            8.0,
            15.0,
            10.0,
            10.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ky023() {
        let j = KY023::default();
        assert_eq!(j.type_id(), "KY023");
        assert_eq!(j.stick_x, 0.0);
        assert_eq!(j.stick_y, 0.0);
        assert!(!j.btn_down);
        assert_eq!(j.pin_geoms().len(), 3);
    }
}

const KY023_PINS: [PinGeom; 3] = [
    PinGeom {
        suffix: "-vrx",
        x: -12.0,
        y: 36.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vry",
        x: -4.0,
        y: 36.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-sw",
        x: 4.0,
        y: 36.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
];

fn ky023_pins() -> &'static [PinGeom] {
    &KY023_PINS
}
