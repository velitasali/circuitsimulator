//! Push button: momentary `Checked` is live, not saved. Resting state is NormClose.

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
use super::{CompPin, Component, ComponentChange, Stampable};
use crate::canvas::{Point, Rect};
use crate::elements::{Kind, SWITCH_CLOSED_ADMIT};
use crate::matrix::CircMatrix;

const MIN_POLES: i64 = 1;
const MAX_POLES: i64 = 4;

impl crate::canvas::Item {
    pub fn push(id: impl Into<String>, x: f64, y: f64, norm_close: bool, poles: usize) -> Self {
        Self::push_with(id, x, y, norm_close, poles, String::new(), false)
    }

    pub fn push_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        norm_close: bool,
        poles: usize,
        key: String,
        show_button: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Push {
                norm_close,
                poles,
                key,
                show_button,
                pressed: false,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Push {
    pub norm_close: bool,
    pub poles: usize,
    pub key: String,
    pub show_button: bool,
    pub pressed: bool,
}

impl Default for Push {
    fn default() -> Self {
        Self {
            norm_close: false,
            poles: 1,
            key: String::new(),
            show_button: false,
            pressed: false,
        }
    }
}

impl Push {
    pub const TYPE_ID: &'static str = "Push";
    pub fn new(pressed: bool, poles: usize) -> Self {
        Self {
            pressed,
            poles: poles.max(1),
            ..Self::default()
        }
    }

    pub fn electrically_closed(&self) -> bool {
        if self.norm_close {
            !self.pressed
        } else {
            self.pressed
        }
    }

    pub fn set_pressed(&mut self, pressed: bool) -> ComponentChange {
        self.pressed = pressed;
        ComponentChange::live("")
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Push {
            closed: self.electrically_closed(),
            poles: self.poles.max(1),
        }
    }

    fn get_norm_close(&self) -> PropValue {
        PropValue::Bool(self.norm_close)
    }
    fn set_norm_close(&mut self, v: PropValue) -> Result<(), PropError> {
        self.norm_close = expect_bool("NormClose", v)?;
        Ok(())
    }
    fn get_poles(&self) -> PropValue {
        PropValue::Int(self.poles as i64)
    }
    fn set_poles(&mut self, v: PropValue) -> Result<(), PropError> {
        self.poles = expect_int("Poles", v)?.clamp(MIN_POLES, MAX_POLES) as usize;
        Ok(())
    }
    fn get_key(&self) -> PropValue {
        PropValue::String(self.key.clone())
    }
    fn set_key(&mut self, v: PropValue) -> Result<(), PropError> {
        self.key = expect_string("Key", v)?;
        Ok(())
    }
    fn get_show_button(&self) -> PropValue {
        PropValue::Bool(self.show_button)
    }
    fn set_show_button(&mut self, v: PropValue) -> Result<(), PropError> {
        self.show_button = expect_bool("ShowButton", v)?;
        Ok(())
    }
    fn get_checked(&self) -> PropValue {
        PropValue::Bool(self.pressed)
    }
    fn set_checked(&mut self, v: PropValue) -> Result<(), PropError> {
        self.pressed = expect_bool("Checked", v)?;
        Ok(())
    }
}

impl Component for Push {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Push button switch."
    }
    fn props() -> &'static [PropDef<Self>] {
        const POLES: PropDef<Push> = {
            let mut p = PropDef::int(
                "Poles",
                "Poles",
                MIN_POLES,
                MAX_POLES,
                Push::get_poles,
                Push::set_poles,
            )
            .with_info("Number of poles controlled by this push button.");
            p.structural = true;
            p
        };
        const CHECKED: PropDef<Push> = {
            let mut p = PropDef::bool("Checked", "Pressed", Push::get_checked, Push::set_checked)
                .with_info("Initial contact / toggle state.");
            p.persist = false;
            p
        };
        static PROPS: &[PropDef<Push>] = &[
            PropDef::bool(
                "NormClose",
                "Normally Closed",
                Push::get_norm_close,
                Push::set_norm_close,
            )
            .with_info("Push to open (yes) or push to close (no)."),
            POLES,
            PropDef::string("Key", "Key", Push::get_key, Push::set_key).with_info(
                "Character shown in the button.\nCan be activated by keyboard in your PC.",
            ),
            PropDef::bool(
                "ShowButton",
                "Show Button",
                Push::get_show_button,
                Push::set_show_button,
            )
            .with_info("Show the physical push-button graphic instead of the bare switch symbol."),
            CHECKED,
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        let n = self.poles.max(1);
        let mut pins = Vec::with_capacity(n * 2);
        for i in 0..n {
            let y = -16.0 * (i as f64);
            pins.push(CompPin::new(format!("-lPin{i}"), -16.0, y, 180, 6.0));
            pins.push(CompPin::new(format!("-rPin{i}"), 16.0, y, 0, 6.0));
        }
        pins
    }
    fn body(&self) -> Rect {
        let p = self.poles.max(1) as f64;
        let h = 16.0 * p;
        Rect::new(-12.0, 8.0 - h, 24.0, h)
    }

    fn interact_toggle(&mut self, _local: Point) -> bool {
        false
    }

    fn interact_press(&mut self, _local: Point) -> bool {
        self.pressed = true;
        true
    }

    fn interact_release(&mut self, _local: Point) -> bool {
        if self.pressed {
            self.pressed = false;
            true
        } else {
            false
        }
    }

    fn interact_cancel(&mut self) -> bool {
        if self.pressed {
            self.pressed = false;
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_push(&mut self, x: f64, y: f64) -> String {
        let id = format!("Push-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Push::default()));
        id
    }

    pub fn set_push_state(&mut self, uid: &str, pressed: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::Push(p) = &mut item.kind {
                p.pressed = pressed;
                return true;
            }
        }
        false
    }
}

impl Stampable for Push {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        if !self.electrically_closed() {
            return;
        }
        for i in 0..self.poles.max(1) {
            stamp_conductance_between(matrix, pin_nodes, 2 * i, 2 * i + 1, SWITCH_CLOSED_ADMIT);
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Push {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let p = self.poles.max(1);
        let show_as_button = self.show_button || ctx.on_board || !self.key.is_empty();

        if show_as_button {
            let outer_top = -16.0 * (p - 1) as f64 - 7.5;
            let outer_bot = 7.5;
            let outer_h = outer_bot - outer_top;
            let center_y = (outer_top + outer_bot) * 0.5;

            // Outer tactile switch housing: spans x = -10..+10 to meet pin stems
            // cleanly on schematics with zero gap and no stray side dots on boards.
            d.fill_round_rect(-10.0, outer_top, 20.0, outer_h, 2.4, ctx.pal.body);
            d.fill_round_rect(
                -10.0,
                outer_top,
                20.0,
                outer_h,
                2.4,
                ctx.pal.border.fade(0.08),
            );
            d.stroke_round_rect(
                -10.0,
                outer_top,
                20.0,
                outer_h,
                2.4,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );

            // Subtle corner mounting posts inside the tactile switch bezel.
            let rivet_col = ctx.pal.border.fade(0.38);
            for (rx, ry) in [
                (-8.3, outer_top + 1.8),
                (8.3, outer_top + 1.8),
                (-8.3, outer_bot - 1.8),
                (8.3, outer_bot - 1.8),
            ] {
                d.fill_circle(rx, ry, 0.65, rivet_col);
            }

            let txt = if !self.key.is_empty() {
                self.key.as_str()
            } else {
                "PUSH"
            };
            let char_count = txt.chars().count();
            let font_size = match char_count {
                0 | 1 => 6.4,
                2 => 5.6,
                3 => 5.0,
                4 => 4.6,
                _ => (12.0 / (char_count as f64 * 0.65)).clamp(3.2, 4.4),
            };

            if self.pressed {
                // Recessed well + depressed plunger cap
                d.fill_round_rect(
                    -7.4,
                    center_y - 5.0,
                    14.8,
                    10.0,
                    2.0,
                    ctx.pal.border.fade(0.18),
                );
                d.fill_round_rect(-7.0, center_y - 4.2, 14.0, 9.2, 1.6, ctx.pal.border);
                d.text_bold(
                    0.0,
                    center_y + 0.4,
                    txt,
                    font_size,
                    ctx.pal.body,
                    Align::Center,
                );
            } else {
                // Raised plunger cap with bottom 3D lip shadow
                d.fill_round_rect(
                    -7.2,
                    center_y - 3.8,
                    14.4,
                    9.6,
                    1.8,
                    ctx.pal.border.fade(0.26),
                );
                d.fill_round_rect(-7.2, center_y - 4.8, 14.4, 9.6, 1.8, ctx.pal.body);
                d.stroke_round_rect(-7.2, center_y - 4.8, 14.4, 9.6, 1.8, ctx.pal.border, 0.75);
                d.text_bold(0.0, center_y, txt, font_size, ctx.pal.border, Align::Center);
            }
            return true;
        }

        // Bare schematic push-button symbol (ShowButton = false, off-board, no key).
        for i in 0..p {
            let y = -16.0 * i as f64;
            d.fill_circle(-10.0, y, 1.3, ctx.pal.border);
            d.fill_circle(10.0, y, 1.3, ctx.pal.border);
            let (plate_y, cap_y) = if self.norm_close {
                if self.pressed {
                    (y + 5.0, y - 2.5)
                } else {
                    (y + 1.2, y - 5.5)
                }
            } else if self.pressed {
                (y - 1.2, y - 5.0)
            } else {
                (y - 4.5, y - 8.0)
            };
            d.line(
                -10.0,
                plate_y,
                10.0,
                plate_y,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            d.line(
                0.0,
                plate_y,
                0.0,
                cap_y,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            d.line(
                -4.0,
                cap_y,
                4.0,
                cap_y,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }
        if p > 1 {
            d.polyline(
                &[[0.0, 2.0], [0.0, -16.0 * (p - 1) as f64 - 4.0]],
                ctx.pal.border,
                1.0,
                true,
            );
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{GraphicAttrs, write_component_item};

    #[test]
    fn default_is_released() {
        let p = Push::default();
        assert!(!p.pressed);
        assert!(!p.electrically_closed());
        assert_eq!(p.get_prop_text("Checked").unwrap(), "false");
    }

    #[test]
    fn pressed_is_live_not_saved() {
        let mut p = Push::default();
        let c = p.set_pressed(true);
        assert!(!c.saved && !c.undo && c.sim);
        let via_prop = p.set_prop_text("Checked", "true").unwrap();
        assert!(!via_prop.saved);
        let line = write_component_item("Push-1", &p, &GraphicAttrs::default());
        assert!(!line.contains("Checked="), "{line}");
    }

    #[test]
    fn norm_close_inverts() {
        let mut p = Push::default();
        p.set_prop_text("NormClose", "true").unwrap();
        assert!(p.electrically_closed());
        p.set_pressed(true);
        assert!(!p.electrically_closed());
    }
}
