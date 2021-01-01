//! Resistive touchpad component.

use super::component::{resistor_g, stamp_conductance_between};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::canvas::{Point, Rect};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_DIM: i64 = 10;
const MAX_DIM: i64 = 10_000;
const MIN_R: f64 = 1e-3;
const MAX_R: f64 = 1e9;

impl crate::canvas::Item {
    #[allow(clippy::too_many_arguments)]
    pub fn touchpad(
        id: impl Into<String>,
        x: f64,
        y: f64,
        width: i32,
        height: i32,
        transparent: bool,
        rx_min: f64,
        rx_max: f64,
        ry_min: f64,
        ry_max: f64,
    ) -> Self {
        Self::touchpad_with(
            id,
            x,
            y,
            width,
            height,
            transparent,
            rx_min,
            rx_max,
            ry_min,
            ry_max,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn touchpad_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        width: i32,
        height: i32,
        transparent: bool,
        rx_min: f64,
        rx_max: f64,
        ry_min: f64,
        ry_max: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            TouchPad {
                width,
                height,
                transparent,
                rx_min,
                rx_max,
                ry_min,
                ry_max,
                x_pos: 0,
                y_pos: 0,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TouchPad {
    pub width: i32,
    pub height: i32,
    pub transparent: bool,
    pub rx_min: f64,
    pub rx_max: f64,
    pub ry_min: f64,
    pub ry_max: f64,
    pub x_pos: i32,
    pub y_pos: i32,
}

impl Default for TouchPad {
    fn default() -> Self {
        Self {
            width: 240,
            height: 320,
            transparent: false,
            rx_min: 100.0,
            rx_max: 500.0,
            ry_min: 100.0,
            ry_max: 500.0,
            x_pos: -1,
            y_pos: -1,
        }
    }
}

impl TouchPad {
    pub const TYPE_ID: &'static str = "TouchPad";
    pub fn to_element_kind(&self) -> Kind {
        Kind::TouchPad {
            width: self.width,
            height: self.height,
            rx_min: self.rx_min,
            rx_max: self.rx_max,
            ry_min: self.ry_min,
            ry_max: self.ry_max,
            x_pos: self.x_pos,
            y_pos: self.y_pos,
        }
    }

    fn get_width(&self) -> PropValue {
        PropValue::Int(self.width as i64)
    }
    fn set_width(&mut self, v: PropValue) -> Result<(), PropError> {
        self.width = expect_int("Width", v)?.clamp(MIN_DIM, MAX_DIM) as i32;
        Ok(())
    }

    fn get_height(&self) -> PropValue {
        PropValue::Int(self.height as i64)
    }
    fn set_height(&mut self, v: PropValue) -> Result<(), PropError> {
        self.height = expect_int("Height", v)?.clamp(MIN_DIM, MAX_DIM) as i32;
        Ok(())
    }

    fn get_transparent(&self) -> PropValue {
        PropValue::Bool(self.transparent)
    }
    fn set_transparent(&mut self, v: PropValue) -> Result<(), PropError> {
        self.transparent = expect_bool("Transparent", v)?;
        Ok(())
    }

    fn get_rx_min(&self) -> PropValue {
        PropValue::Float(self.rx_min)
    }
    fn set_rx_min(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rx_min = expect_float("RxMin", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_rx_max(&self) -> PropValue {
        PropValue::Float(self.rx_max)
    }
    fn set_rx_max(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rx_max = expect_float("RxMax", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_ry_min(&self) -> PropValue {
        PropValue::Float(self.ry_min)
    }
    fn set_ry_min(&mut self, v: PropValue) -> Result<(), PropError> {
        self.ry_min = expect_float("RyMin", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_ry_max(&self) -> PropValue {
        PropValue::Float(self.ry_max)
    }
    fn set_ry_max(&mut self, v: PropValue) -> Result<(), PropError> {
        self.ry_max = expect_float("RyMax", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }
}

impl Component for TouchPad {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Resistive touchpad."
    }

    fn props() -> &'static [PropDef<Self>] {
        const WIDTH: PropDef<TouchPad> = {
            let mut p = PropDef::int(
                "Width",
                "Width",
                MIN_DIM,
                MAX_DIM,
                TouchPad::get_width,
                TouchPad::set_width,
            )
            .with_info("Touchpad width, in pixels.");
            p.structural = true;
            p
        };
        const HEIGHT: PropDef<TouchPad> = {
            let mut p = PropDef::int(
                "Height",
                "Height",
                MIN_DIM,
                MAX_DIM,
                TouchPad::get_height,
                TouchPad::set_height,
            )
            .with_info("Touchpad height, in pixels.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<TouchPad>] = &[
            WIDTH,
            HEIGHT,
            PropDef::bool(
                "Transparent",
                "Transparent",
                TouchPad::get_transparent,
                TouchPad::set_transparent,
            ).with_info("Draw the touchpad body as transparent, so it can be overlaid on a background image."),
            PropDef::float(
                "RxMin",
                "RxMin",
                "Ω",
                MIN_R,
                MAX_R,
                TouchPad::get_rx_min,
                TouchPad::set_rx_min,
            ).with_info("Minimum resistance in X axis (right side)."),
            PropDef::float(
                "RxMax",
                "RxMax",
                "Ω",
                MIN_R,
                MAX_R,
                TouchPad::get_rx_max,
                TouchPad::set_rx_max,
            ).with_info("Maximum resistance in X axis (left side)."),
            PropDef::float(
                "RyMin",
                "RyMin",
                "Ω",
                MIN_R,
                MAX_R,
                TouchPad::get_ry_min,
                TouchPad::set_ry_min,
            ).with_info("Minimum resistance in Y axis (top side)."),
            PropDef::float(
                "RyMax",
                "RyMax",
                "Ω",
                MIN_R,
                MAX_R,
                TouchPad::get_ry_max,
                TouchPad::set_ry_max,
            ).with_info("Maximum resistance in Y axis (bottom side)."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        touchpad_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        let w = self.width as f64;
        let h = self.height as f64;
        Rect::new(-w / 2.0, -h, w, h + 16.0)
    }

    fn interact_press(&mut self, local: Point) -> bool {
        let w = self.width as f64;
        let h = self.height as f64;
        let touch_rect = Rect::new(-w / 2.0, -h, w, h);
        if touch_rect.contains_point(local) {
            self.x_pos = (local.x - (-w / 2.0)).round().clamp(0.0, w) as i32;
            self.y_pos = (local.y - (-h)).round().clamp(0.0, h) as i32;
            true
        } else {
            false
        }
    }

    fn interact_move(&mut self, local: Point) -> bool {
        if self.x_pos >= 0 && self.y_pos >= 0 {
            let w = self.width as f64;
            let h = self.height as f64;
            let nx = (local.x - (-w / 2.0)).round().clamp(0.0, w) as i32;
            let ny = (local.y - (-h)).round().clamp(0.0, h) as i32;
            if nx != self.x_pos || ny != self.y_pos {
                self.x_pos = nx;
                self.y_pos = ny;
            }
            true
        } else {
            false
        }
    }

    fn interact_release(&mut self, _local: Point) -> bool {
        if self.x_pos >= 0 || self.y_pos >= 0 {
            self.x_pos = -1;
            self.y_pos = -1;
            true
        } else {
            false
        }
    }

    fn interact_cancel(&mut self) -> bool {
        if self.x_pos >= 0 || self.y_pos >= 0 {
            self.x_pos = -1;
            self.y_pos = -1;
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_touchpad(&mut self, x: f64, y: f64) -> String {
        let id = format!("TouchPad-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::touchpad(
            &id, x, y, 240, 320, false, 100.0, 500.0, 100.0, 500.0,
        ));
        id
    }

    pub fn set_touchpad_pos(&mut self, uid: &str, x: i32, y: i32) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::TouchPad(p) = &mut item.kind {
                p.x_pos = x.clamp(0, p.width);
                p.y_pos = y.clamp(0, p.height);
                return true;
            }
        }
        false
    }

    pub fn reset_touchpad(&mut self, uid: &str) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::TouchPad(p) = &mut item.kind {
                p.x_pos = -1;
                p.y_pos = -1;
                return true;
            }
        }
        false
    }
}

impl Stampable for TouchPad {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_conductance_between(
            matrix,
            pin_nodes,
            0,
            1,
            resistor_g(self.rx_min + self.rx_max),
        );
        stamp_conductance_between(
            matrix,
            pin_nodes,
            2,
            3,
            resistor_g(self.ry_min + self.ry_max),
        );
    }
}

impl Drawable for TouchPad {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let w = self.width as f64;
        let h = self.height as f64;
        if !self.transparent {
            d.fill_round_rect(
                -w / 2.0,
                -h,
                w,
                h,
                2.0,
                ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
            );
        }
        d.stroke_round_rect(
            -w / 2.0,
            -h,
            w,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.fill_round_rect(
            -20.0,
            0.0,
            40.0,
            16.0,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -20.0,
            0.0,
            40.0,
            16.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.text(0.0, 8.0, "Touch", 8.0, ctx.pal.border, Align::Center);

        if self.x_pos >= 0 && self.y_pos >= 0 {
            let cx = -w / 2.0 + (self.x_pos as f64);
            let cy = -h + (self.y_pos as f64);
            d.line(cx - 6.0, cy, cx + 6.0, cy, ctx.pal.border, 1.5);
            d.line(cx, cy - 6.0, cx, cy + 6.0, ctx.pal.border, 1.5);
            d.stroke_circle(cx, cy, 4.0, ctx.pal.border, 1.0);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_touchpad() {
        let tp = TouchPad::default();
        assert_eq!(tp.type_id(), "TouchPad");
        assert_eq!(tp.width, 240);
        assert_eq!(tp.height, 320);
        assert!(!tp.transparent);
        assert_eq!(tp.rx_min, 100.0);
        assert_eq!(tp.rx_max, 500.0);
        assert_eq!(tp.pin_geoms().len(), 4);
        assert_eq!(tp.body(), Rect::new(-120.0, -320.0, 240.0, 336.0));
    }
}

const TOUCHPAD_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-vrx_p",
        x: -12.0,
        y: 24.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vrx_m",
        x: -4.0,
        y: 24.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vry_p",
        x: 4.0,
        y: 24.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-vry_m",
        x: 12.0,
        y: 24.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
];

fn touchpad_pins() -> &'static [PinGeom] {
    &TOUCHPAD_PINS
}
