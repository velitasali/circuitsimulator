//! KY-040 rotary encoder module.

use super::component::stamp_to_ground;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::canvas::{Point, Rect};
use crate::elements::{Kind, SOURCE_ADMIT};
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_STEPS: i64 = 1;
const MAX_STEPS: i64 = 1000;
const MIN_DIAL: i64 = -1_000_000;
const MAX_DIAL: i64 = 1_000_000;

impl crate::canvas::Item {
    pub fn ky040(id: impl Into<String>, x: f64, y: f64, steps: usize) -> Self {
        Self::ky040_with(id, x, y, steps)
    }

    pub fn ky040_with(id: impl Into<String>, x: f64, y: f64, steps: usize) -> Self {
        Self::new(
            id,
            x,
            y,
            KY040 {
                steps: steps as u32,
                dial_val: 1,
                btn_closed: false,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KY040 {
    pub steps: u32,
    pub dial_val: i32,
    pub btn_closed: bool,
}

impl Default for KY040 {
    fn default() -> Self {
        Self {
            steps: 20,
            dial_val: 1,
            btn_closed: false,
        }
    }
}

impl KY040 {
    pub const TYPE_ID: &'static str = "KY040";
    pub fn to_element_kind(&self) -> Kind {
        let k = self.dial_val.rem_euclid(4);
        Kind::Ky040 {
            steps: self.steps,
            dial_val: self.dial_val,
            btn_closed: self.btn_closed,
            state_a: k == 1 || k == 2,
            state_b: k == 2 || k == 3,
        }
    }

    fn get_steps(&self) -> PropValue {
        PropValue::Int(self.steps as i64)
    }
    fn set_steps(&mut self, v: PropValue) -> Result<(), PropError> {
        self.steps = expect_int("Steps", v)?.clamp(MIN_STEPS, MAX_STEPS) as u32;
        Ok(())
    }

    fn get_dial_val(&self) -> PropValue {
        PropValue::Int(self.dial_val as i64)
    }
    fn set_dial_val(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dial_val = expect_int("DialVal", v)?.clamp(MIN_DIAL, MAX_DIAL) as i32;
        Ok(())
    }

    fn get_btn_closed(&self) -> PropValue {
        PropValue::Bool(self.btn_closed)
    }
    fn set_btn_closed(&mut self, v: PropValue) -> Result<(), PropError> {
        self.btn_closed = expect_bool("BtnClosed", v)?;
        Ok(())
    }
}

impl Component for KY040 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Relative rotary encoder."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<KY040>] = &[
            PropDef::int(
                "Steps",
                "Steps per Rotation",
                MIN_STEPS,
                MAX_STEPS,
                KY040::get_steps,
                KY040::set_steps,
            )
            .with_info("Encoder steps per dial rotation."),
            PropDef::int(
                "DialVal",
                "Dial Value",
                MIN_DIAL,
                MAX_DIAL,
                KY040::get_dial_val,
                KY040::set_dial_val,
            )
            .with_info("Rotary encoder position count."),
            PropDef::bool(
                "BtnClosed",
                "Button Closed",
                KY040::get_btn_closed,
                KY040::set_btn_closed,
            )
            .with_info("Integrated pushbutton contact state (pressed / released)."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        ky040_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-20.0, -28.0, 40.0, 56.0)
    }

    fn interact_wheel(&mut self, local: Point, delta: f64) -> bool {
        let knob_hit = local.x.hypot(local.y) <= 18.0;
        if knob_hit {
            let step = if delta.abs() >= 120.0 {
                (delta / 120.0).round() as i32
            } else {
                delta.signum() as i32
            };
            self.dial_val += step;
            true
        } else {
            false
        }
    }

    fn interact_press(&mut self, local: Point) -> bool {
        if local.x.hypot(local.y) <= 12.0 {
            self.btn_closed = true;
            true
        } else {
            false
        }
    }

    fn interact_release(&mut self, _local: Point) -> bool {
        if self.btn_closed {
            self.btn_closed = false;
            true
        } else {
            false
        }
    }

    fn interact_cancel(&mut self) -> bool {
        if self.btn_closed {
            self.btn_closed = false;
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_ky040(&mut self, x: f64, y: f64) -> String {
        let id = format!("KY040-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::ky040(&id, x, y, 20));
        id
    }

    pub fn set_rotary_dial(&mut self, uid: &str, val: i32) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::KY040(p) = &mut item.kind {
                p.dial_val = val;
                return true;
            }
        }
        false
    }

    pub fn set_rotary_button(&mut self, uid: &str, closed: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::KY040(p) = &mut item.kind {
                p.btn_closed = closed;
                return true;
            }
        }
        false
    }
}

impl Stampable for KY040 {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let k = self.dial_val.rem_euclid(4);
        let state_a = k == 1 || k == 2;
        let state_b = k == 2 || k == 3;
        let va = if state_a { 5.0 } else { 0.0 };
        let vb = if state_b { 5.0 } else { 0.0 };
        stamp_to_ground(matrix, pin_nodes, 0, va, SOURCE_ADMIT);
        stamp_to_ground(matrix, pin_nodes, 1, vb, SOURCE_ADMIT);
        if self.btn_closed {
            stamp_to_ground(matrix, pin_nodes, 2, 0.0, 1000.0);
        } else {
            stamp_to_ground(matrix, pin_nodes, 2, 5.0, 1.0 / 2000.0);
        }
    }
}

impl Drawable for KY040 {
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
        d.text(-16.0, -22.0, "ENCODER", 6.0, ctx.pal.border, Align::TopLeft);

        let r_track = 15.0;
        d.arc(
            0.0,
            0.0,
            r_track,
            135.0_f64.to_radians(),
            405.0_f64.to_radians(),
            ctx.pal.border.fade(0.6),
            1.0,
        );
        for &deg in &[135.0, 180.0, 225.0, 270.0, 315.0, 360.0, 405.0] {
            let rad = (deg as f64).to_radians();
            let c = rad.cos();
            let s = rad.sin();
            d.line(
                (r_track - 1.5) * c,
                (r_track - 1.5) * s,
                (r_track + 1.5) * c,
                (r_track + 1.5) * s,
                ctx.pal.border.fade(0.7),
                1.0,
            );
        }

        let knob_fill = if self.btn_closed {
            ctx.pal.pin_open_high.fade(0.7)
        } else {
            ctx.pal.body.fade(0.5)
        };
        d.fill_circle(0.0, 0.0, 12.0, knob_fill);
        d.stroke_circle(0.0, 0.0, 12.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);

        let step_deg = 360.0 / (self.steps.max(1) as f64);
        let angle = (self.dial_val as f64 * step_deg - 90.0).to_radians();
        d.line(
            2.0 * angle.cos(),
            2.0 * angle.sin(),
            10.0 * angle.cos(),
            10.0 * angle.sin(),
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH * 1.5,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ky040() {
        let enc = KY040::default();
        assert_eq!(enc.type_id(), "KY040");
        assert_eq!(enc.steps, 20);
        assert_eq!(enc.dial_val, 1);
        assert!(!enc.btn_closed);
        assert_eq!(enc.pin_geoms().len(), 3);
    }
}

const KY040_PINS: [PinGeom; 3] = [
    PinGeom {
        suffix: "-clk",
        x: 4.0,
        y: 36.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-dt",
        x: -4.0,
        y: 36.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-sw",
        x: -12.0,
        y: 36.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
];

fn ky040_pins() -> &'static [PinGeom] {
    &KY040_PINS
}
