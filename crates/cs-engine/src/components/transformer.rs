//! Coupled inductor transformer component.

use super::component::{resistor_g, stamp_conductance_between};
use super::props::{PropDef, PropError, PropValue, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_L: f64 = 1e-12;
const MAX_L: f64 = 1e6;
const MIN_R: f64 = 1e-6;
const MAX_R: f64 = 1e6;

impl crate::canvas::Item {
    pub fn transformer(
        id: impl Into<String>,
        x: f64,
        y: f64,
        ind1: f64,
        ind2: f64,
        coupling: f64,
    ) -> Self {
        Self::transformer_with(id, x, y, ind1, ind2, coupling, 0.1, 0.1)
    }

    pub fn transformer_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        inductance1: f64,
        inductance2: f64,
        coupling: f64,
        r_coil1: f64,
        r_coil2: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Transformer {
                inductance1,
                inductance2,
                coupling,
                r_coil1,
                r_coil2,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transformer {
    pub inductance1: f64,
    pub inductance2: f64,
    pub coupling: f64,
    pub r_coil1: f64,
    pub r_coil2: f64,
}

impl Default for Transformer {
    fn default() -> Self {
        Self {
            inductance1: 1.0,
            inductance2: 1.0,
            coupling: 0.99,
            r_coil1: 0.1,
            r_coil2: 0.1,
        }
    }
}

impl Transformer {
    pub const TYPE_ID: &'static str = "Transformer";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Transformer {
            inductance1: self.inductance1,
            inductance2: self.inductance2,
            coupling: self.coupling,
        }
    }

    fn get_inductance1(&self) -> PropValue {
        PropValue::Float(self.inductance1)
    }
    fn set_inductance1(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance1 = expect_float("Inductance1", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }

    fn get_inductance2(&self) -> PropValue {
        PropValue::Float(self.inductance2)
    }
    fn set_inductance2(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance2 = expect_float("Inductance2", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }

    fn get_coupling(&self) -> PropValue {
        PropValue::Float(self.coupling)
    }
    fn set_coupling(&mut self, v: PropValue) -> Result<(), PropError> {
        self.coupling = expect_float("Coupling", v)?.clamp(0.0, 1.0);
        Ok(())
    }

    fn get_r_coil1(&self) -> PropValue {
        PropValue::Float(self.r_coil1)
    }
    fn set_r_coil1(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_coil1 = expect_float("Rcoil1", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_r_coil2(&self) -> PropValue {
        PropValue::Float(self.r_coil2)
    }
    fn set_r_coil2(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_coil2 = expect_float("Rcoil2", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }
}

impl Component for Transformer {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Coupled inductor transformer."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Transformer>] = &[
            PropDef::float(
                "Inductance1",
                "Primary Inductance",
                "H",
                MIN_L,
                MAX_L,
                Transformer::get_inductance1,
                Transformer::set_inductance1,
            )
            .with_info("Primary winding inductance in Henrys."),
            PropDef::float(
                "Inductance2",
                "Secondary Inductance",
                "H",
                MIN_L,
                MAX_L,
                Transformer::get_inductance2,
                Transformer::set_inductance2,
            )
            .with_info("Secondary winding inductance in Henrys."),
            PropDef::float(
                "Coupling",
                "Coupling Factor",
                "",
                0.0,
                1.0,
                Transformer::get_coupling,
                Transformer::set_coupling,
            )
            .with_info("Magnetic coupling coefficient (k) between windings (0.0 to 1.0)."),
            PropDef::float(
                "Rcoil1",
                "Primary Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Transformer::get_r_coil1,
                Transformer::set_r_coil1,
            )
            .with_info("Primary winding series winding resistance in Ohms."),
            PropDef::float(
                "Rcoil2",
                "Secondary Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Transformer::get_r_coil2,
                Transformer::set_r_coil2,
            )
            .with_info("Secondary winding series winding resistance in Ohms."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        transformer_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-12.0, 0.0, 24.0, 24.0)
    }
}

impl Stampable for Transformer {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        if dt > 0.0 {
            stamp_conductance_between(matrix, pin_nodes, 0, 1, dt / self.inductance1.max(MIN_L));
            stamp_conductance_between(matrix, pin_nodes, 2, 3, dt / self.inductance2.max(MIN_L));
        } else {
            stamp_conductance_between(matrix, pin_nodes, 0, 1, resistor_g(self.r_coil1));
            stamp_conductance_between(matrix, pin_nodes, 2, 3, resistor_g(self.r_coil2));
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Transformer {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.line(
            -2.0,
            0.0,
            -2.0,
            24.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(2.0, 0.0, 2.0, 24.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        for y in [4.0, 12.0, 20.0] {
            d.arc(
                -10.0,
                y,
                4.0,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }
        for y in [4.0, 12.0, 20.0] {
            d.arc(
                10.0,
                y,
                4.0,
                3.0 * std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }
        d.fill_circle(-12.0, 4.0, 1.4, ctx.pal.border);
        d.fill_circle(12.0, 4.0, 1.4, ctx.pal.border);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_transformer(&mut self, x: f64, y: f64) -> String {
        let id = format!("Transformer-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::transformer(&id, x, y, 1.0, 1.0, 0.99));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_transformer() {
        let t = Transformer::default();
        assert_eq!(t.type_id(), "Transformer");
        assert_eq!(t.inductance1, 1.0);
        assert_eq!(t.inductance2, 1.0);
        assert_eq!(t.coupling, 0.99);
        assert_eq!(t.r_coil1, 0.1);
        assert_eq!(t.r_coil2, 0.1);
        let pins = t.pin_geoms();
        assert_eq!(pins.len(), 4);
        assert_eq!(t.body(), Rect::new(-12.0, 0.0, 24.0, 24.0));
        assert_eq!(pins[0].local.x, -16.0);
        assert_eq!(pins[0].local.y, 0.0);
        assert_eq!(pins[1].local.x, -16.0);
        assert_eq!(pins[1].local.y, 24.0);
        assert_eq!(pins[2].local.x, 16.0);
        assert_eq!(pins[2].local.y, 0.0);
        assert_eq!(pins[3].local.x, 16.0);
        assert_eq!(pins[3].local.y, 24.0);
    }
}

const TRANSFORMER_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-p1",
        x: -16.0,
        y: 0.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-p2",
        x: -16.0,
        y: 24.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-s1",
        x: 16.0,
        y: 0.0,
        angle: 0,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-s2",
        x: 16.0,
        y: 24.0,
        angle: 0,
        length: 6.0,
        direction: None,
    },
];

fn transformer_pins() -> &'static [PinGeom] {
    &TRANSFORMER_PINS
}
