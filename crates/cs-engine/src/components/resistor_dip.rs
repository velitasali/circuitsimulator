//! Resistor DIP network component.

use super::component::{PropGroup, resistor_g, stamp_conductance_between};
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float, expect_int};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_SIZE: i64 = 1;
const MAX_SIZE: i64 = 16;
const MIN_R: f64 = 1e-12;
const MAX_R: f64 = 1e12;
const MIN_V: f64 = -1e4;
const MAX_V: f64 = 1e4;

impl crate::canvas::Item {
    pub fn resistor_dip(
        id: impl Into<String>,
        x: f64,
        y: f64,
        size: usize,
        resistance: f64,
        bussed: bool,
    ) -> Self {
        Self::resistor_dip_with(id, x, y, size, resistance, bussed, 5.0)
    }

    pub fn resistor_dip_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        size: usize,
        resistance: f64,
        bussed: bool,
        pu_volt: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            ResistorDip {
                size,
                resistance,
                bussed,
                pu_volt,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResistorDip {
    pub size: usize,
    pub resistance: f64,
    pub bussed: bool,
    pub pu_volt: f64,
}

impl Default for ResistorDip {
    fn default() -> Self {
        Self {
            size: 8,
            resistance: 100.0,
            bussed: false,
            pu_volt: 5.0,
        }
    }
}

impl ResistorDip {
    pub const TYPE_ID: &'static str = "ResistorDip";
    pub fn to_element_kind(&self) -> Kind {
        Kind::ResistorDip {
            size: self.size,
            resistance: self.resistance,
            bussed: self.bussed,
        }
    }

    fn get_size(&self) -> PropValue {
        PropValue::Int(self.size as i64)
    }
    fn set_size(&mut self, v: PropValue) -> Result<(), PropError> {
        self.size = expect_int("Size", v)?.clamp(MIN_SIZE, MAX_SIZE) as usize;
        Ok(())
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_bussed(&self) -> PropValue {
        PropValue::Bool(self.bussed)
    }
    fn set_bussed(&mut self, v: PropValue) -> Result<(), PropError> {
        self.bussed = expect_bool("Bussed", v)?;
        Ok(())
    }

    fn get_pu_volt(&self) -> PropValue {
        PropValue::Float(self.pu_volt)
    }
    fn set_pu_volt(&mut self, v: PropValue) -> Result<(), PropError> {
        self.pu_volt = expect_float("PuVolt", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }
}

impl Component for ResistorDip {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Resistor network package."
    }

    fn props() -> &'static [PropDef<Self>] {
        const SIZE: PropDef<ResistorDip> = {
            let mut p = PropDef::int(
                "Size",
                "Size",
                MIN_SIZE,
                MAX_SIZE,
                ResistorDip::get_size,
                ResistorDip::set_size,
            )
            .with_info("Number of resistors.");
            p.structural = true;
            p
        };
        const BUSSED: PropDef<ResistorDip> = {
            let mut p = PropDef::bool(
                "Bussed",
                "Bussed (Pullup)",
                ResistorDip::get_bussed,
                ResistorDip::set_bussed,
            )
            .with_info("Bussed array mode: common pin connected to all resistors.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<ResistorDip>] = &[
            SIZE,
            PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                ResistorDip::get_resistance,
                ResistorDip::set_resistance,
            )
            .with_info("Resistance value of each resistor, in ohms."),
            BUSSED,
            PropDef::float(
                "PuVolt",
                "Pullup Voltage",
                "V",
                MIN_V,
                MAX_V,
                ResistorDip::get_pu_volt,
                ResistorDip::set_pu_volt,
            )
            .with_info(
                "Voltage to connect the resistors.\nShown only if \"Pullup\" property is set.",
            ),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let mut rows = self.prop_rows();
        for r in &mut rows {
            if r.name == "PuVolt" {
                r.visible = self.bussed;
            }
        }
        vec![PropGroup::new("Main", rows)]
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        resistor_dip_pins("", self.size, self.bussed)
            .into_iter()
            .map(CompPin::from)
            .collect()
    }

    fn body(&self) -> Rect {
        let h = 8.0 * (self.size as f64).max(1.0);
        Rect::new(-9.0, -28.0, 18.0, h)
    }
}

impl Stampable for ResistorDip {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        let g = resistor_g(self.resistance);
        if self.bussed {
            for i in 0..self.size {
                stamp_conductance_between(matrix, pin_nodes, 0, i + 1, g);
            }
        } else {
            for i in 0..self.size {
                stamp_conductance_between(matrix, pin_nodes, 2 * i, 2 * i + 1, g);
            }
        }
    }
}

impl Drawable for ResistorDip {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let n = self.size.max(1);
        let h = n as f64 * 8.0;
        d.fill_round_rect(
            -9.0,
            -28.0,
            18.0,
            h,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -9.0,
            -28.0,
            18.0,
            h,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        for i in 0..n {
            let y = -24.0 + i as f64 * 8.0;
            d.stroke_rect(
                -6.0,
                y - 2.0,
                12.0,
                4.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_resistor_dip(&mut self, x: f64, y: f64) -> String {
        let id = format!("ResistorDip-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::resistor_dip(
            &id, x, y, 8, 100.0, false,
        ));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resistor_dip() {
        let r = ResistorDip::default();
        assert_eq!(r.type_id(), "ResistorDip");
        assert_eq!(r.size, 8);
        assert_eq!(r.resistance, 100.0);
        assert!(!r.bussed);
        assert_eq!(r.pu_volt, 5.0);
        assert_eq!(r.pin_geoms().len(), 16);
        assert_eq!(r.body(), Rect::new(-9.0, -28.0, 18.0, 64.0));
    }

    #[test]
    fn bussed_resistor_dip_pins() {
        let mut r = ResistorDip::default();
        r.bussed = true;
        // 1 COM pin + 8 individual pins = 9 pins
        assert_eq!(r.pin_geoms().len(), 9);
    }
}

fn resistor_dip_pins(id: &str, size: usize, bussed: bool) -> Vec<Pin> {
    let n = size.max(1);
    let mut pins = Vec::with_capacity(n * 2 + 1);
    if bussed {
        pins.push(Pin {
            direction: None,
            id: format!("{id}-com"),
            item_id: id.to_string(),
            local: Point::new(-16.0, -24.0),
            angle: 180,
            length: 7.0,
            is_bus: false,
            label: "COM".into(),
            unused: false,
        });
        for i in 0..n {
            let y = -24.0 + (i as f64) * 8.0;
            pins.push(Pin {
                direction: None,
                id: format!("{id}-pin{i}"),
                item_id: id.to_string(),
                local: Point::new(16.0, y),
                angle: 0,
                length: 7.0,
                is_bus: false,
                label: format!("{}", i + 1),
                unused: false,
            });
        }
    } else {
        for i in 0..n {
            let y = -24.0 + (i as f64) * 8.0;
            pins.push(Pin {
                direction: None,
                id: format!("{id}-lPin{i}"),
                item_id: id.to_string(),
                local: Point::new(-16.0, y),
                angle: 180,
                length: 7.0,
                is_bus: false,
                label: String::new(),
                unused: false,
            });
            pins.push(Pin {
                direction: None,
                id: format!("{id}-rPin{i}"),
                item_id: id.to_string(),
                local: Point::new(16.0, y),
                angle: 0,
                length: 7.0,
                is_bus: false,
                label: String::new(),
                unused: false,
            });
        }
    }
    pins
}
