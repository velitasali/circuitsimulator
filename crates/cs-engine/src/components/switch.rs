//! Mechanical switch: poles, double-throw geometry, persisted Checked.

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
use super::{CompPin, Component, ComponentChange, Stampable};
use crate::canvas::Rect;
use crate::elements::{Kind, SWITCH_CLOSED_ADMIT};
use crate::matrix::CircMatrix;

const MIN_POLES: i64 = 1;
const MAX_POLES: i64 = 4;

impl crate::canvas::Item {
    pub fn switch(id: impl Into<String>, x: f64, y: f64, closed: bool) -> Self {
        Self::new(
            id,
            x,
            y,
            Switch {
                checked: closed,
                norm_close: false,
                double_throw: false,
                poles: 1,
                key: String::new(),
                show_button: false,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Switch {
    pub checked: bool,
    pub norm_close: bool,
    pub double_throw: bool,
    pub poles: usize,
    pub key: String,
    pub show_button: bool,
}

impl Default for Switch {
    fn default() -> Self {
        Self {
            checked: true,
            norm_close: false,
            double_throw: false,
            poles: 1,
            key: String::new(),
            show_button: false,
        }
    }
}

impl Switch {
    pub const TYPE_ID: &'static str = "Switch";
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
            ..Self::default()
        }
    }

    pub fn electrically_closed(&self) -> bool {
        if self.norm_close {
            !self.checked
        } else {
            self.checked
        }
    }

    pub fn toggle(&mut self) -> ComponentChange {
        self.checked = !self.checked;
        ComponentChange::document("")
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Switch {
            closed: self.electrically_closed(),
            poles: self.poles.max(1),
            double_throw: self.double_throw,
        }
    }

    fn get_norm_close(&self) -> PropValue {
        PropValue::Bool(self.norm_close)
    }
    fn set_norm_close(&mut self, v: PropValue) -> Result<(), PropError> {
        self.norm_close = expect_bool("NormClose", v)?;
        Ok(())
    }
    fn get_double_throw(&self) -> PropValue {
        PropValue::Bool(self.double_throw)
    }
    fn set_double_throw(&mut self, v: PropValue) -> Result<(), PropError> {
        self.double_throw = expect_bool("DoubleThrow", v)?;
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
        PropValue::Bool(self.checked)
    }
    fn set_checked(&mut self, v: PropValue) -> Result<(), PropError> {
        self.checked = expect_bool("Checked", v)?;
        Ok(())
    }
}

impl Component for Switch {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "SPST switch."
    }
    fn props() -> &'static [PropDef<Self>] {
        const POLES: PropDef<Switch> = {
            let mut p = PropDef::int(
                "Poles",
                "Poles",
                MIN_POLES,
                MAX_POLES,
                Switch::get_poles,
                Switch::set_poles,
            )
            .with_info("Number of poles controlled by this switch.");
            p.structural = true;
            p
        };
        const DT: PropDef<Switch> = {
            let mut p = PropDef::bool(
                "DoubleThrow",
                "Double Throw",
                Switch::get_double_throw,
                Switch::set_double_throw,
            )
            .with_info("Yes: 2 throws per pole (SPDT/DPDT). No: 1 throw per pole (SPST/DPST).");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<Switch>] = &[
            PropDef::bool(
                "NormClose",
                "Normally Closed",
                Switch::get_norm_close,
                Switch::set_norm_close,
            )
            .with_info("State with button not pushed."),
            DT,
            POLES,
            PropDef::string("Key", "Key", Switch::get_key, Switch::set_key).with_info(
                "Character shown in the button.\nCan be activated by keyboard in your PC.",
            ),
            PropDef::bool(
                "ShowButton",
                "Show Button",
                Switch::get_show_button,
                Switch::set_show_button,
            )
            .with_info("Show the physical push-button graphic instead of the bare switch symbol."),
            PropDef::bool(
                "Checked",
                "Closed",
                Switch::get_checked,
                Switch::set_checked,
            )
            .with_info("Initial contact / toggle state."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        let n = self.poles.max(1);
        let mut pins = Vec::with_capacity(if self.double_throw { n * 3 } else { n * 2 });
        for i in 0..n {
            let y = -16.0 * (i as f64);
            pins.push(CompPin::new(format!("-pinP{i}"), -16.0, y, 180, 6.0));
            if self.double_throw {
                pins.push(CompPin::new(
                    format!("-switch{}pinN", 2 * i),
                    16.0,
                    y,
                    0,
                    6.0,
                ));
                pins.push(CompPin::new(
                    format!("-switch{}pinN", 2 * i + 1),
                    16.0,
                    y - 8.0,
                    0,
                    6.0,
                ));
            } else {
                pins.push(CompPin::new(format!("-switch{i}pinN"), 16.0, y, 0, 6.0));
            }
        }
        pins
    }
    fn body(&self) -> Rect {
        let p = self.poles.max(1) as f64;
        let h = 16.0 * p;
        Rect::new(-12.0, 8.0 - h, 24.0, h)
    }
    fn interact_toggle(&mut self, _local: crate::canvas::Point) -> bool {
        self.checked = !self.checked;
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_switch(&mut self, x: f64, y: f64, closed: bool) -> String {
        let id = format!("Switch-{}", self.next_switch);
        self.next_switch += 1;
        self.items
            .push(crate::canvas::Item::switch(&id, x, y, closed));
        id
    }

    pub fn add_default_switch(&mut self, x: f64, y: f64) -> String {
        self.add_switch(x, y, true)
    }

    pub fn toggle_switch(&mut self, index: usize) -> bool {
        if let Some(item) = self.items.get_mut(index) {
            if let crate::components::Part::Switch(p) = &mut item.kind {
                p.checked = !p.checked;
                return true;
            }
        }
        false
    }
}

impl Stampable for Switch {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let n = self.poles.max(1);
        let closed = self.electrically_closed();
        if self.double_throw {
            let stride = 3;
            for i in 0..n {
                let base = i * stride;
                let target_pin = if closed { base + 1 } else { base + 2 };
                stamp_conductance_between(matrix, pin_nodes, base, target_pin, SWITCH_CLOSED_ADMIT);
            }
        } else if closed {
            let stride = 2;
            for i in 0..n {
                let base = i * stride;
                stamp_conductance_between(matrix, pin_nodes, base, base + 1, SWITCH_CLOSED_ADMIT);
            }
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Switch {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let p = self.poles.max(1);
        let closed = self.electrically_closed();
        if !self.show_button {
            if p > 1 {
                let y2 = -16.0 * (p - 1) as f64 - 4.0;
                d.polyline(&[[0.0, 4.0], [0.0, y2]], ctx.pal.border, 1.0, true);
            }
            for i in 0..p {
                let y = -16.0 * (i as f64);
                if closed {
                    d.line(-10.0, y, 10.0, y, ctx.pal.border, COMPONENT_BORDER_WIDTH);
                } else if self.double_throw {
                    d.line(
                        -10.0,
                        y,
                        10.0,
                        y - 8.0,
                        ctx.pal.border,
                        COMPONENT_BORDER_WIDTH,
                    );
                } else {
                    d.line(
                        -10.0,
                        y,
                        8.0,
                        y - 8.0,
                        ctx.pal.border,
                        COMPONENT_BORDER_WIDTH,
                    );
                }
                d.fill_circle(-10.0, y, 1.2, ctx.pal.border);
                d.fill_circle(10.0, y, 1.2, ctx.pal.border);
                if self.double_throw {
                    d.fill_circle(10.0, y - 8.0, 1.2, ctx.pal.border);
                }
            }
        }
        if self.show_button || !self.key.is_empty() {
            let center_y = -16.0 * (p as f64 - 1.0) / 2.0;
            let bg = if self.checked {
                ctx.pal.border
            } else {
                ctx.pal.body.fade(0.85)
            };
            let fg = if self.checked {
                ctx.pal.body
            } else {
                ctx.pal.border
            };
            d.fill_round_rect(-6.0, center_y - 5.0, 12.0, 10.0, 2.0, bg);
            d.stroke_round_rect(
                -6.0,
                center_y - 5.0,
                12.0,
                10.0,
                2.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            let txt = if !self.key.is_empty() {
                &self.key
            } else if self.checked {
                "ON"
            } else {
                "OFF"
            };
            d.text(0.0, center_y, txt, 6.0, fg, Align::Center);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_closed() {
        let s = Switch::default();
        assert!(s.checked);
        assert!(!s.norm_close);
        assert_eq!(s.poles, 1);
        assert_eq!(s.get_prop_text("Checked").unwrap(), "true");
        assert_eq!(s.get_prop_text("NormClose").unwrap(), "false");
        assert!(s.electrically_closed());
    }

    #[test]
    fn toggle_writes_checked() {
        let mut s = Switch::default();
        let c = s.toggle();
        assert!(c.saved && c.undo && c.sim);
        assert!(!s.checked);
        assert_eq!(s.get_prop_text("Checked").unwrap(), "false");
    }

    #[test]
    fn poles_is_structural() {
        let mut s = Switch::default();
        let c = s.set_prop_text("Poles", "2").unwrap();
        assert!(c.structural);
        assert_eq!(s.pin_geoms().len(), 4);
    }

    #[test]
    fn stamp_closed_to_ground() {
        let s = Switch::new(true);
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        s.stamp(&mut m, &[0, usize::MAX], 0.0);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 1.0 / SWITCH_CLOSED_ADMIT).abs() < 1e-9, "{}", v[0]);
    }

    #[test]
    fn pin_length_is_six_preventing_protrusion() {
        let s = Switch::default();
        for p in s.pin_geoms() {
            assert_eq!(
                p.length, 6.0,
                "Pin length must be 6.0 so stems meet dots at x = ±10 without protruding"
            );
        }
        let mut dt_switch = Switch::default();
        dt_switch.double_throw = true;
        for p in dt_switch.pin_geoms() {
            assert_eq!(p.length, 6.0);
        }
    }

    #[test]
    fn double_throw_stamping_alternates_contacts() {
        let mut s = Switch::default();
        s.double_throw = true;
        s.checked = true; // closed -> throw 0

        // pin_nodes: [P0=0, throw0=1, throw1=2]
        let mut m1 = CircMatrix::new(2);
        m1.analyze(&[vec![], vec![]]);
        s.stamp(&mut m1, &[0, 1, usize::MAX], 0.0);
        m1.add_coef(0, 1.0);
        m1.add_coef(1, 1.0);
        let mut v1 = vec![0.0, 0.0];
        assert!(m1.solve(&mut v1));

        // When closed, contact 0 conducts
        let mut m_t0 = CircMatrix::new(1);
        m_t0.analyze(&[vec![]]);
        s.stamp(&mut m_t0, &[0, usize::MAX, 1], 0.0);
        m_t0.add_coef(0, 1.0);
        let mut v_t0 = vec![0.0];
        assert!(m_t0.solve(&mut v_t0));
        assert!((v_t0[0] - 1.0 / SWITCH_CLOSED_ADMIT).abs() < 1e-9);

        // Switch to open (checked = false) -> throw 1 conducts
        s.checked = false;
        let mut m_t1 = CircMatrix::new(1);
        m_t1.analyze(&[vec![]]);
        s.stamp(&mut m_t1, &[0, 1, usize::MAX], 0.0); // connect pole to GND via throw 1
        m_t1.add_coef(0, 1.0);
        let mut v_t1 = vec![0.0];
        assert!(m_t1.solve(&mut v_t1));
        assert!((v_t1[0] - 1.0 / SWITCH_CLOSED_ADMIT).abs() < 1e-9);
    }

    #[test]
    fn normally_closed_double_throw_conducts_throw1_when_open() {
        let mut s = Switch::default();
        s.double_throw = true;
        s.norm_close = true;

        // In resting state (checked = false), normally closed means electrically closed (throw 0)
        s.checked = false;
        assert!(s.electrically_closed());

        // When checked / activated (checked = true), normally closed means electrically open -> conducts throw 1
        s.checked = true;
        assert!(!s.electrically_closed());

        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        s.stamp(&mut m, &[0, 1, usize::MAX], 0.0);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 1.0 / SWITCH_CLOSED_ADMIT).abs() < 1e-9);
    }
}
