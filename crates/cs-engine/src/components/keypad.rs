//! Keypad matrix. Pressed key is live (not saved); Rows/Cols rebuild pins.

use super::component::stamp_conductance_between;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_int, expect_string};
use super::{CompPin, Component, ComponentChange, Stampable};
use crate::canvas::{Point, Rect};
use crate::elements::{Kind, SWITCH_CLOSED_ADMIT};
use crate::matrix::CircMatrix;

const MIN_DIM: i64 = 1;
const MAX_DIM: i64 = 8;

impl crate::canvas::Item {
    pub fn keypad(id: impl Into<String>, x: f64, y: f64, rows: usize, cols: usize) -> Self {
        let key = KeyPad::default_labels_for_cols(cols).to_string();
        Self::keypad_with(id, x, y, rows, cols, key, false, false)
    }

    pub fn keypad_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        rows: usize,
        cols: usize,
        key: String,
        diodes: bool,
        dir: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            KeyPad {
                rows,
                cols,
                key,
                diodes,
                dir,
                pressed: None,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyPad {
    pub rows: usize,
    pub cols: usize,
    pub key: String,
    pub diodes: bool,
    pub dir: bool,
    pub pressed: Option<(usize, usize)>,
}

impl Default for KeyPad {
    fn default() -> Self {
        Self {
            rows: 4,
            cols: 4,
            key: Self::default_labels_for_cols(4).to_string(),
            diodes: false,
            dir: false,
            pressed: None,
        }
    }
}

impl KeyPad {
    pub const TYPE_ID: &'static str = "KeyPad";

    pub fn default_labels_for_cols(cols: usize) -> &'static str {
        if cols == 3 {
            "123456789*0#"
        } else if cols == 4 {
            "123A456B789C*0#D"
        } else {
            "123456789*0#ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        }
    }

    pub fn new(rows: usize, cols: usize) -> Self {
        let rows = rows.clamp(1, MAX_DIM as usize);
        let cols = cols.clamp(1, MAX_DIM as usize);
        Self {
            rows,
            cols,
            key: Self::default_labels_for_cols(cols).to_string(),
            ..Self::default()
        }
    }

    pub fn set_pressed(&mut self, pressed: Option<(usize, usize)>) -> ComponentChange {
        self.pressed = pressed.filter(|&(r, c)| r < self.rows && c < self.cols);
        ComponentChange::live("")
    }

    pub fn key_labels_str(&self) -> &str {
        if !self.key.is_empty() {
            self.key.as_str()
        } else {
            Self::default_labels_for_cols(self.cols)
        }
    }

    pub fn key_label(&self, row: usize, col: usize) -> Option<char> {
        let pos = row * self.cols + col;
        self.key_labels_str().chars().nth(pos)
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::KeyPad {
            rows: self.rows.max(1),
            cols: self.cols.max(1),
            key: self.key.clone(),
            diodes: self.diodes,
            dir: self.dir,
            pressed: self.pressed,
        }
    }

    fn get_rows(&self) -> PropValue {
        PropValue::Int(self.rows as i64)
    }
    fn set_rows(&mut self, v: PropValue) -> Result<(), PropError> {
        self.rows = expect_int("Rows", v)?.clamp(MIN_DIM, MAX_DIM) as usize;
        if let Some((r, _)) = self.pressed {
            if r >= self.rows {
                self.pressed = None;
            }
        }
        Ok(())
    }
    fn get_cols(&self) -> PropValue {
        PropValue::Int(self.cols as i64)
    }
    fn set_cols(&mut self, v: PropValue) -> Result<(), PropError> {
        let old_cols = self.cols;
        self.cols = expect_int("Cols", v)?.clamp(MIN_DIM, MAX_DIM) as usize;
        if self.key.is_empty() || self.key == Self::default_labels_for_cols(old_cols) {
            self.key = Self::default_labels_for_cols(self.cols).to_string();
        }
        if let Some((_, c)) = self.pressed {
            if c >= self.cols {
                self.pressed = None;
            }
        }
        Ok(())
    }
    fn get_key(&self) -> PropValue {
        PropValue::String(self.key_labels_str().to_string())
    }
    fn set_key(&mut self, v: PropValue) -> Result<(), PropError> {
        self.key = expect_string("Key_Labels", v)?;
        Ok(())
    }
    fn get_diodes(&self) -> PropValue {
        PropValue::Bool(self.diodes)
    }
    fn set_diodes(&mut self, v: PropValue) -> Result<(), PropError> {
        self.diodes = expect_bool("Diodes", v)?;
        Ok(())
    }
    fn get_dir(&self) -> PropValue {
        PropValue::Bool(self.dir)
    }
    fn set_dir(&mut self, v: PropValue) -> Result<(), PropError> {
        self.dir = expect_bool("Dir", v)?;
        Ok(())
    }
}

impl Component for KeyPad {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Keypad matrix."
    }
    fn props() -> &'static [PropDef<Self>] {
        const ROWS: PropDef<KeyPad> = {
            let mut p = PropDef::int(
                "Rows",
                "Rows",
                MIN_DIM,
                MAX_DIM,
                KeyPad::get_rows,
                KeyPad::set_rows,
            )
            .with_info("Number of key rows.");
            p.structural = true;
            p
        };
        const COLS: PropDef<KeyPad> = {
            let mut p = PropDef::int(
                "Cols",
                "Columns",
                MIN_DIM,
                MAX_DIM,
                KeyPad::get_cols,
                KeyPad::set_cols,
            )
            .with_info("Number of key columns.");
            p.structural = true;
            p
        };
        const KEY_ALIAS: PropDef<KeyPad> = {
            let mut p = PropDef::string("Key", "Key Labels", KeyPad::get_key, KeyPad::set_key);
            p.show_by_default = false;
            p
        };
        static PROPS: &[PropDef<KeyPad>] = &[
            ROWS,
            COLS,
            PropDef::string(
                "Key_Labels",
                "Key Labels",
                KeyPad::get_key,
                KeyPad::set_key,
            )
            .with_info("List of characters corresponding to each key.\nLeft to right and top to bottom."),
            KEY_ALIAS,
            PropDef::bool(
                "Diodes",
                "Anti-Ghosting Diodes",
                KeyPad::get_diodes,
                KeyPad::set_diodes,
            ).with_info("Add a diode in series with each key, to prevent ghosting when several keys are pressed at once."),
            PropDef::bool("Dir", "Diodes Direction", KeyPad::get_dir, KeyPad::set_dir).with_info("Direction of the key diodes: from row to column, or from column to row."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        let r = self.rows.max(1);
        let c = self.cols.max(1);
        let mut pins = Vec::with_capacity(r + c);
        for row in 0..r {
            pins.push(CompPin::new(
                format!("-Pin{row}"),
                -16.0,
                8.0 + (row as f64) * 16.0,
                180,
                4.0,
            ));
        }
        for col in 0..c {
            pins.push(CompPin::new(
                format!("-Pin{}", r + col),
                (col as f64) * 16.0,
                -8.0,
                90,
                4.0,
            ));
        }
        pins
    }
    fn body(&self) -> Rect {
        Rect::new(
            -12.0,
            -4.0,
            (self.cols as f64).max(1.0) * 16.0 + 8.0,
            (self.rows as f64).max(1.0) * 16.0 + 8.0,
        )
    }
    fn interact_press(&mut self, local: Point) -> bool {
        let r = self.rows.max(1);
        let c = self.cols.max(1);
        for row in 0..r {
            for col in 0..c {
                let bx = -6.0 + (col as f64) * 16.0;
                let by = 2.0 + (row as f64) * 16.0;
                if local.x >= bx && local.x <= bx + 12.0 && local.y >= by && local.y <= by + 12.0 {
                    self.pressed = Some((row, col));
                    return true;
                }
            }
        }
        false
    }
    fn interact_release(&mut self, _local: Point) -> bool {
        if self.pressed.is_some() {
            self.pressed = None;
            true
        } else {
            false
        }
    }
    fn interact_cancel(&mut self) -> bool {
        if self.pressed.is_some() {
            self.pressed = None;
            true
        } else {
            false
        }
    }
}

impl crate::canvas::Scene {
    pub fn add_keypad(&mut self, x: f64, y: f64) -> String {
        let id = format!("KeyPad-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::keypad(&id, x, y, 4, 4));
        id
    }

    pub fn set_keypad_pressed(&mut self, uid: &str, row: usize, col: usize, pressed: bool) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::KeyPad(p) = &mut item.kind {
                if pressed {
                    p.pressed = Some((row, col));
                } else if p.pressed == Some((row, col)) {
                    p.pressed = None;
                }
                return true;
            }
        }
        false
    }
}

impl Stampable for KeyPad {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let Some((r, c)) = self.pressed else {
            return;
        };
        if r < self.rows && c < self.cols {
            let row_i = r;
            let col_i = self.rows + c;
            stamp_conductance_between(matrix, pin_nodes, row_i, col_i, SWITCH_CLOSED_ADMIT);
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for KeyPad {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let r = self.rows.max(1);
        let c = self.cols.max(1);
        let w = (c * 16 + 8) as f64;
        let h = (r * 16 + 8) as f64;
        d.fill_round_rect(-12.0, -4.0, w, h, 2.0, ctx.pal.body.fade(0.3));
        d.stroke_round_rect(
            -12.0,
            -4.0,
            w,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        for row in 0..r {
            for col in 0..c {
                let bx = -6.0 + (col as f64) * 16.0;
                let by = 2.0 + (row as f64) * 16.0;
                let is_pressed = self.pressed == Some((row, col));
                let fill = if is_pressed {
                    ctx.pal.border
                } else {
                    ctx.pal.body.fade(0.6)
                };
                d.fill_round_rect(bx, by, 12.0, 12.0, 2.0, fill);
                d.stroke_round_rect(
                    bx,
                    by,
                    12.0,
                    12.0,
                    2.0,
                    ctx.pal.border,
                    COMPONENT_BORDER_WIDTH,
                );
                if let Some(ch) = self.key_label(row, col) {
                    let text_color = if is_pressed {
                        ctx.pal.canvas
                    } else {
                        ctx.pal.border
                    };
                    let s = ch.to_string();
                    d.text(bx + 6.0, by + 6.0, &s, 7.0, text_color, Align::Center);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_4x4() {
        let k = KeyPad::default();
        assert_eq!(k.rows, 4);
        assert_eq!(k.cols, 4);
        assert!(k.pressed.is_none());
        assert_eq!(k.pin_geoms().len(), 8);
        assert_eq!(k.get_prop_text("Rows").unwrap(), "4");
        assert_eq!(k.get_prop_text("Cols").unwrap(), "4");
        assert_eq!(k.get_prop_text("Key_Labels").unwrap(), "123A456B789C*0#D");
        assert_eq!(
            k.prop_rows()
                .iter()
                .filter(|r| r.caption == "Key Labels")
                .count(),
            1
        );
        // Verify key labels
        assert_eq!(k.key_label(0, 0), Some('1'));
        assert_eq!(k.key_label(0, 3), Some('A'));
        assert_eq!(k.key_label(3, 3), Some('D'));
    }

    #[test]
    fn default_4x3_labels() {
        let k = KeyPad::new(4, 3);
        assert_eq!(k.key_label(0, 0), Some('1'));
        assert_eq!(k.key_label(0, 1), Some('2'));
        assert_eq!(k.key_label(0, 2), Some('3'));
        assert_eq!(k.key_label(1, 0), Some('4'));
        assert_eq!(k.key_label(3, 0), Some('*'));
        assert_eq!(k.key_label(3, 1), Some('0'));
        assert_eq!(k.key_label(3, 2), Some('#'));
    }

    #[test]
    fn custom_key_labels() {
        let mut k = KeyPad::new(2, 2);
        k.key = "ABCD".to_string();
        assert_eq!(k.key_label(0, 0), Some('A'));
        assert_eq!(k.key_label(0, 1), Some('B'));
        assert_eq!(k.key_label(1, 0), Some('C'));
        assert_eq!(k.key_label(1, 1), Some('D'));
    }

    #[test]
    fn press_is_live() {
        let mut k = KeyPad::default();
        let c = k.set_pressed(Some((1, 2)));
        assert!(!c.saved && !c.undo && c.sim);
        assert_eq!(k.pressed, Some((1, 2)));
        k.set_pressed(Some((9, 0)));
        assert!(k.pressed.is_none());
    }

    #[test]
    fn rows_is_structural() {
        let mut k = KeyPad::default();
        let c = k.set_prop_text("Rows", "3").unwrap();
        assert!(c.structural);
        assert_eq!(k.pin_geoms().len(), 7);
    }
}
