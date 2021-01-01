//! DIP switch package. `State` is a persisted bitfield (one bit per switch).

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int};
use super::{CompPin, Component, ComponentChange, Stampable};
use crate::canvas::Rect;
use crate::elements::{Kind, SWITCH_CLOSED_ADMIT};
use crate::matrix::CircMatrix;

const MIN_SIZE: i64 = 1;
const MAX_SIZE: i64 = 16;

impl crate::canvas::Item {
    pub fn switch_dip(
        id: impl Into<String>,
        x: f64,
        y: f64,
        size: usize,
        state: u32,
        exclusive: bool,
    ) -> Self {
        Self::switch_dip_with(id, x, y, size, state, exclusive, false)
    }

    pub fn switch_dip_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        size: usize,
        state: u32,
        exclusive: bool,
        common_pin: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            SwitchDip {
                size,
                state,
                exclusive,
                common_pin,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwitchDip {
    pub size: usize,
    pub state: u32,
    pub exclusive: bool,
    pub common_pin: bool,
}

impl Default for SwitchDip {
    fn default() -> Self {
        Self {
            size: 4,
            state: 0b1111,
            exclusive: false,
            common_pin: false,
        }
    }
}

impl SwitchDip {
    pub const TYPE_ID: &'static str = "SwitchDip";
    pub fn new(size: usize, state: u32, common_pin: bool) -> Self {
        let size = size.clamp(1, MAX_SIZE as usize);
        let mut s = Self {
            size,
            state,
            exclusive: false,
            common_pin,
        };
        s.clamp_state();
        s
    }

    fn mask(&self) -> u32 {
        if self.size >= 32 {
            u32::MAX
        } else {
            (1u32 << self.size) - 1
        }
    }

    fn clamp_state(&mut self) {
        self.state &= self.mask();
        if self.exclusive {
            if self.state == 0 {
                self.state = 1;
            } else {
                self.state &= self.state.wrapping_neg();
            }
        }
    }

    pub fn set_state(&mut self, state: u32) -> ComponentChange {
        self.state = state;
        self.clamp_state();
        ComponentChange::document("")
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::SwitchDip {
            size: self.size.max(1),
            state: self.state,
            common_pin: self.common_pin,
        }
    }

    fn get_size(&self) -> PropValue {
        PropValue::Int(self.size as i64)
    }
    fn set_size(&mut self, v: PropValue) -> Result<(), PropError> {
        self.size = expect_int("Size", v)?.clamp(MIN_SIZE, MAX_SIZE) as usize;
        self.clamp_state();
        Ok(())
    }
    fn get_exclusive(&self) -> PropValue {
        PropValue::Bool(self.exclusive)
    }
    fn set_exclusive(&mut self, v: PropValue) -> Result<(), PropError> {
        self.exclusive = expect_bool("Exclusive", v)?;
        self.clamp_state();
        Ok(())
    }
    fn get_common_pin(&self) -> PropValue {
        PropValue::Bool(self.common_pin)
    }
    fn set_common_pin(&mut self, v: PropValue) -> Result<(), PropError> {
        self.common_pin = expect_bool("CommonPin", v)?;
        Ok(())
    }
    fn get_state(&self) -> PropValue {
        PropValue::Int(self.state as i64)
    }
    fn set_state_prop(&mut self, v: PropValue) -> Result<(), PropError> {
        self.state = expect_int("State", v)?.clamp(0, u32::MAX as i64) as u32;
        self.clamp_state();
        Ok(())
    }
}

impl Component for SwitchDip {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "DIP switch package."
    }
    fn props() -> &'static [PropDef<Self>] {
        const SIZE: PropDef<SwitchDip> = {
            let mut p = PropDef::int(
                "Size",
                "Size",
                MIN_SIZE,
                MAX_SIZE,
                SwitchDip::get_size,
                SwitchDip::set_size,
            )
            .with_info("Number of switches.");
            p.structural = true;
            p
        };
        const COMMON: PropDef<SwitchDip> = {
            let mut p = PropDef::bool(
                "CommonPin",
                "Common Pin",
                SwitchDip::get_common_pin,
                SwitchDip::set_common_pin,
            )
            .with_info("Connect all Switches to a common Pin.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<SwitchDip>] = &[
            SIZE,
            PropDef::bool(
                "Exclusive",
                "Exclusive",
                SwitchDip::get_exclusive,
                SwitchDip::set_exclusive,
            )
            .with_info("Only one Switch can be closed at a time."),
            COMMON,
            PropDef::int(
                "State",
                "State",
                0,
                u32::MAX as i64,
                SwitchDip::get_state,
                SwitchDip::set_state_prop,
            )
            .with_info("Switch %1, terminal 2"),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        let n = self.size.max(1);
        let mut pins = Vec::new();
        if self.common_pin {
            pins.push(CompPin::new("-com", -8.0, -24.0, 180, 5.0));
            for i in 0..n {
                let y = -24.0 + (i as f64) * 8.0;
                pins.push(CompPin::new(format!("-pin{i}"), 16.0, y, 0, 5.0));
            }
        } else {
            for i in 0..n {
                let y = -24.0 + (i as f64) * 8.0;
                pins.push(CompPin::new(format!("-pin{}", 2 * i), -8.0, y, 180, 5.0));
                pins.push(CompPin::new(format!("-pin{}", 2 * i + 1), 16.0, y, 0, 5.0));
            }
        }
        pins
    }
    fn body(&self) -> Rect {
        let h = 8.0 * (self.size as f64).max(1.0);
        Rect::new(-3.0, -28.0, 14.0, h)
    }
    fn interact_toggle(&mut self, local: crate::canvas::Point) -> bool {
        if local.x < -6.0 || local.x > 14.0 {
            return false;
        }
        let idx = ((local.y - (-28.0)) / 8.0).floor() as isize;
        if idx >= 0 && (idx as usize) < self.size {
            let bit = idx as usize;
            if self.exclusive {
                self.state = 1 << bit;
            } else {
                self.state ^= 1 << bit;
            }
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_switch_dip(&mut self, x: f64, y: f64) -> String {
        let id = format!("SwitchDip-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::switch_dip(
            &id,
            x,
            y,
            4,
            (1 << 4) - 1,
            false,
        ));
        id
    }

    pub fn set_dip_switch(&mut self, uid: &str, switch_idx: usize, on: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::SwitchDip(p) = &mut item.kind {
                if switch_idx < p.size {
                    if p.exclusive {
                        if on {
                            p.state = 1 << switch_idx;
                        }
                    } else if on {
                        p.state |= 1 << switch_idx;
                    } else {
                        p.state &= !(1 << switch_idx);
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn toggle_dip_switch(&mut self, uid: &str, switch_idx: usize) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::SwitchDip(p) = &mut item.kind {
                if switch_idx < p.size {
                    if p.exclusive {
                        p.state = 1 << switch_idx;
                    } else {
                        p.state ^= 1 << switch_idx;
                    }
                    return true;
                }
            }
        }
        false
    }
}

impl Stampable for SwitchDip {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        for i in 0..self.size.max(1) {
            if (self.state & (1 << i)) == 0 {
                continue;
            }
            if self.common_pin {
                stamp_conductance_between(matrix, pin_nodes, 0, i + 1, SWITCH_CLOSED_ADMIT);
            } else {
                stamp_conductance_between(matrix, pin_nodes, 2 * i, 2 * i + 1, SWITCH_CLOSED_ADMIT);
            }
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for SwitchDip {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let n = self.size.max(1);
        let h = n as f64 * 8.0;
        d.fill_round_rect(-3.0, -28.0, 14.0, h, 2.0, ctx.pal.body.fade(0.4));
        d.stroke_round_rect(
            -3.0,
            -28.0,
            14.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        for i in 0..n {
            let on = (self.state & (1 << i)) != 0;
            let ty = -27.0 + i as f64 * 8.0;
            d.stroke_rect(1.0, ty, 6.0, 6.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
            let sy = if on { ty + 0.5 } else { ty + 2.9 };
            d.fill_round_rect(1.6, sy, 4.8, 2.6, 0.8, ctx.pal.border);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_all_closed() {
        let d = SwitchDip::default();
        assert_eq!(d.size, 4);
        assert_eq!(d.state, 15);
        assert_eq!(d.get_prop_text("State").unwrap(), "15");
        assert_eq!(d.get_prop_text("Size").unwrap(), "4");
    }

    #[test]
    fn set_state_marks_document() {
        let mut d = SwitchDip::default();
        let c = d.set_state(5);
        assert!(c.saved && c.sim);
        assert_eq!(d.state, 5);
    }

    #[test]
    fn exclusive_keeps_one_bit() {
        let mut d = SwitchDip::default();
        d.set_prop_text("Exclusive", "true").unwrap();
        assert_eq!(d.state, 1);
        d.set_state(0b1100);
        assert_eq!(d.state, 0b0100);
    }

    #[test]
    fn common_pin_is_structural() {
        let mut d = SwitchDip::default();
        let before = d.pin_geoms().len();
        let c = d.set_prop_text("CommonPin", "true").unwrap();
        assert!(c.structural);
        assert_eq!(before, 8);
        assert_eq!(d.pin_geoms().len(), 5);
    }
}
