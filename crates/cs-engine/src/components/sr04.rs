//! HC-SR04 ultrasonic distance sensor.

use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinDirection;
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

const MIN_DIST_M: f64 = 0.02;
const MAX_DIST_M: f64 = 4.0;

impl crate::canvas::Item {
    pub fn sr04(id: impl Into<String>, x: f64, y: f64, distance: f64, use_slider: bool) -> Self {
        Self::sr04_with(id, x, y, distance, use_slider)
    }

    pub fn sr04_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        distance: f64,
        use_slider: bool,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            SR04 {
                distance,
                use_slider,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SR04 {
    pub distance: f64,
    pub use_slider: bool,
}

impl Default for SR04 {
    fn default() -> Self {
        Self {
            distance: 0.5,
            use_slider: true,
        }
    }
}

impl SR04 {
    pub const TYPE_ID: &'static str = "SR04";
    pub fn to_element_kind(&self) -> Kind {
        Kind::Sr04 {
            distance: self.distance,
            use_slider: self.use_slider,
            trigger_high: false,
            echo_high: false,
        }
    }

    fn get_distance(&self) -> PropValue {
        PropValue::Float(self.distance)
    }
    fn set_distance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.distance = expect_float("Distance", v)?.clamp(MIN_DIST_M, MAX_DIST_M);
        Ok(())
    }

    fn get_slider(&self) -> PropValue {
        PropValue::Bool(self.use_slider)
    }
    fn set_slider(&mut self, v: PropValue) -> Result<(), PropError> {
        self.use_slider = expect_bool("Slider", v)?;
        Ok(())
    }
}

impl Component for SR04 {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Ultrasonic distance sensor. Volts equal to meters: 1.5 V = 1.5 m."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<SR04>] = &[
            PropDef::float(
                "Distance",
                "Distance",
                "m",
                MIN_DIST_M,
                MAX_DIST_M,
                SR04::get_distance,
                SR04::set_distance,
            ).with_info("Simulated target distance."),
            PropDef::bool("Slider", "Use slider", SR04::get_slider, SR04::set_slider)
            .with_info("Show a slider to set the simulated distance directly, instead of using the \"In\" pin voltage.")
            .with_info("Show a slider to set the simulated distance directly, instead of using the \"In\" pin voltage.")
            .with_info("Show a slider to set the simulated distance directly, instead of using the \"In\" pin voltage."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        sr04_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-80.0, -32.0, 168.0, 72.0)
    }
}

impl crate::canvas::Scene {
    pub fn add_sr04(&mut self, x: f64, y: f64) -> String {
        let id = format!("SR04-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::sr04(&id, x, y, 100.0, true));
        id
    }

    pub fn set_sr04_distance(&mut self, uid: &str, dist: f64) -> bool {
        if let Some(item) = self.item_by_id_mut(uid) {
            if let crate::components::Part::SR04(p) = &mut item.kind {
                p.distance = dist.clamp(0.02, 4.0);
                return true;
            }
        }
        false
    }
}

impl Stampable for SR04 {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for SR04 {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // Outer PCB Board (168x72)
        d.fill_round_rect(
            -80.0,
            -32.0,
            168.0,
            72.0,
            4.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -80.0,
            -32.0,
            168.0,
            72.0,
            4.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Header Title
        d.text(4.0, -26.0, "HC-SR04", 8.0, ctx.pal.border, Align::Center);

        // Distance Readout
        let dist_str = format!("{:.1} cm", self.distance * 100.0);
        d.text(4.0, -14.0, &dist_str, 9.0, ctx.pal.border, Align::Center);

        // Transmitter Cylinder (T) at (-66, -16, 44, 44) => center (-44, 6), radius 22
        d.fill_circle(-44.0, 6.0, 22.0, ctx.pal.body.fade(0.8));
        d.stroke_circle(-44.0, 6.0, 22.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.text(-44.0, 6.0, "T", 12.0, ctx.pal.border, Align::Center);

        // Receiver Cylinder (R) at (30, -16, 44, 44) => center (52, 6), radius 22
        d.fill_circle(52.0, 6.0, 22.0, ctx.pal.body.fade(0.8));
        d.stroke_circle(52.0, 6.0, 22.0, ctx.pal.border, COMPONENT_BORDER_WIDTH);
        d.text(52.0, 6.0, "R", 12.0, ctx.pal.border, Align::Center);

        // Interactive Distance Adjustment Slider
        if self.use_slider {
            // Slider track
            d.fill_round_rect(-34.0, 29.5, 76.0, 3.0, 1.5, ctx.pal.border.fade(0.4));

            // Slider handle
            let frac = ((self.distance - 0.02) / 3.98).clamp(0.0, 1.0);
            let hx = -34.0 + frac * 76.0;
            d.fill_circle(hx, 31.0, 4.0, ctx.pal.pin_open_high);
            d.stroke_circle(hx, 31.0, 4.0, ctx.pal.border, 1.0);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sr04() {
        let s = SR04::default();
        assert_eq!(s.type_id(), "SR04");
        assert_eq!(s.distance, 0.5);
        assert!(s.use_slider);
        assert_eq!(s.pin_geoms().len(), 5);
    }
}

const SR04_PINS: [PinGeom; 5] = [
    PinGeom {
        suffix: "-inpin",
        x: -88.0,
        y: -24.0,
        angle: 180,
        length: 8.0,
        direction: Some(PinDirection::In),
    },
    PinGeom {
        suffix: "-vccpin",
        x: -8.0,
        y: 48.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-trigpin",
        x: 0.0,
        y: 48.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-outpin",
        x: 8.0,
        y: 48.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-gndpin",
        x: 16.0,
        y: 48.0,
        angle: 270,
        length: 8.0,
        direction: None,
    },
];

fn sr04_pins() -> &'static [PinGeom] {
    &SR04_PINS
}
