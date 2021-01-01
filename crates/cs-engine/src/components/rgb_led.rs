//! RGB LED indicator with independent red, green, and blue channels.

use super::component::stamp_two_terminal;
use super::drawable::{BulbState, Drawable, indicator_bulb_visual_rect, paint_indicator_bulb};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx};
use crate::elements::Kind;
use crate::elements::pins::{PIN_RGB_B, PIN_RGB_C, PIN_RGB_G, PIN_RGB_R};
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

const MIN_V: f64 = 0.1;
const MAX_V: f64 = 10.0;
const MIN_A: f64 = 1e-3;
const MAX_A: f64 = 1.0;
const MIN_OHMS: f64 = 1e-3;
const MAX_OHMS: f64 = 1e6;

impl crate::canvas::Item {
    pub fn rgb_led(id: impl Into<String>, x: f64, y: f64, common_anode: bool) -> Self {
        Self::new(
            id,
            x,
            y,
            RgbLed {
                common_anode,
                v_th_r: 2.0,
                i_max_r: 0.03,
                r_res_r: 0.6,
                v_th_g: 3.0,
                i_max_g: 0.03,
                r_res_g: 0.6,
                v_th_b: 3.0,
                i_max_b: 0.03,
                r_res_b: 0.6,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RgbLed {
    pub common_anode: bool,
    pub v_th_r: f64,
    pub i_max_r: f64,
    pub r_res_r: f64,
    pub v_th_g: f64,
    pub i_max_g: f64,
    pub r_res_g: f64,
    pub v_th_b: f64,
    pub i_max_b: f64,
    pub r_res_b: f64,
}

impl Default for RgbLed {
    fn default() -> Self {
        Self {
            common_anode: false,
            v_th_r: 2.0,
            i_max_r: 0.03,
            r_res_r: 0.6,
            v_th_g: 3.0,
            i_max_g: 0.03,
            r_res_g: 0.6,
            v_th_b: 3.0,
            i_max_b: 0.03,
            r_res_b: 0.6,
        }
    }
}

impl RgbLed {
    pub const TYPE_ID: &'static str = "RgbLed";
    pub fn to_element_kind(&self) -> Kind {
        Kind::RgbLed {
            common_anode: self.common_anode,
            state_r: 0.0,
            state_g: 0.0,
            state_b: 0.0,
        }
    }

    fn get_common_anode(&self) -> PropValue {
        PropValue::Bool(self.common_anode)
    }
    fn set_common_anode(&mut self, v: PropValue) -> Result<(), PropError> {
        self.common_anode = expect_bool("CommonAnode", v)?;
        Ok(())
    }

    fn get_v_th_r(&self) -> PropValue {
        PropValue::Float(self.v_th_r)
    }
    fn set_v_th_r(&mut self, v: PropValue) -> Result<(), PropError> {
        self.v_th_r = expect_float("ThresholdR", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_i_max_r(&self) -> PropValue {
        PropValue::Float(self.i_max_r)
    }
    fn set_i_max_r(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_max_r = expect_float("MaxCurrentR", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }

    fn get_r_res_r(&self) -> PropValue {
        PropValue::Float(self.r_res_r)
    }
    fn set_r_res_r(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_res_r = expect_float("ResistanceR", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }

    fn get_v_th_g(&self) -> PropValue {
        PropValue::Float(self.v_th_g)
    }
    fn set_v_th_g(&mut self, v: PropValue) -> Result<(), PropError> {
        self.v_th_g = expect_float("ThresholdG", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_i_max_g(&self) -> PropValue {
        PropValue::Float(self.i_max_g)
    }
    fn set_i_max_g(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_max_g = expect_float("MaxCurrentG", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }

    fn get_r_res_g(&self) -> PropValue {
        PropValue::Float(self.r_res_g)
    }
    fn set_r_res_g(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_res_g = expect_float("ResistanceG", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }

    fn get_v_th_b(&self) -> PropValue {
        PropValue::Float(self.v_th_b)
    }
    fn set_v_th_b(&mut self, v: PropValue) -> Result<(), PropError> {
        self.v_th_b = expect_float("ThresholdB", v)?.clamp(MIN_V, MAX_V);
        Ok(())
    }

    fn get_i_max_b(&self) -> PropValue {
        PropValue::Float(self.i_max_b)
    }
    fn set_i_max_b(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_max_b = expect_float("MaxCurrentB", v)?.clamp(MIN_A, MAX_A);
        Ok(())
    }

    fn get_r_res_b(&self) -> PropValue {
        PropValue::Float(self.r_res_b)
    }
    fn set_r_res_b(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_res_b = expect_float("ResistanceB", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
}

impl Component for RgbLed {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "RGB LED."
    }

    fn props() -> &'static [PropDef<Self>] {
        const ANODE: PropDef<RgbLed> = {
            let mut p = PropDef::bool(
                "CommonAnode",
                "Common Anode",
                RgbLed::get_common_anode,
                RgbLed::set_common_anode,
            )
            .with_info(
                "Determines common anode (true) or common cathode (false) pin configuration.",
            );
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<RgbLed>] = &[
            ANODE,
            PropDef::float(
                "ThresholdR",
                "Red Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                RgbLed::get_v_th_r,
                RgbLed::set_v_th_r,
            ).with_info("Voltage drop for the red channel when forward biased."),
            PropDef::float(
                "MaxCurrentR",
                "Red Max Current",
                "A",
                MIN_A,
                MAX_A,
                RgbLed::get_i_max_r,
                RgbLed::set_i_max_r,
            ).with_info("Maximum current for the red channel (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "ResistanceR",
                "Red Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                RgbLed::get_r_res_r,
                RgbLed::set_r_res_r,
            ).with_info("Series resistance for the red channel."),
            PropDef::float(
                "ThresholdG",
                "Green Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                RgbLed::get_v_th_g,
                RgbLed::set_v_th_g,
            ).with_info("Voltage drop for the green channel when forward biased."),
            PropDef::float(
                "MaxCurrentG",
                "Green Max Current",
                "A",
                MIN_A,
                MAX_A,
                RgbLed::get_i_max_g,
                RgbLed::set_i_max_g,
            ).with_info("Maximum current for the green channel (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "ResistanceG",
                "Green Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                RgbLed::get_r_res_g,
                RgbLed::set_r_res_g,
            ).with_info("Series resistance for the green channel."),
            PropDef::float(
                "ThresholdB",
                "Blue Forward Voltage",
                "V",
                MIN_V,
                MAX_V,
                RgbLed::get_v_th_b,
                RgbLed::set_v_th_b,
            ).with_info("Voltage drop for the blue channel when forward biased."),
            PropDef::float(
                "MaxCurrentB",
                "Blue Max Current",
                "A",
                MIN_A,
                MAX_A,
                RgbLed::get_i_max_b,
                RgbLed::set_i_max_b,
            ).with_info("Maximum current for the blue channel (it will blink if exceeded).\nMaximum brightness is reached at this current."),
            PropDef::float(
                "ResistanceB",
                "Blue Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                RgbLed::get_r_res_b,
                RgbLed::set_r_res_b,
            ).with_info("Series resistance for the blue channel."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        rgb_led_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-10.0, -10.0, 20.0, 20.0)
    }

    fn visual_rect(&self) -> Rect {
        self.body()
            .united(indicator_bulb_visual_rect(0.0, 0.0, 6.0))
    }
}

impl Stampable for RgbLed {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, 1.0 / 100.0, 1.8 / 100.0);
    }
}

impl Drawable for RgbLed {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(
            -10.0,
            -10.0,
            20.0,
            20.0,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -10.0,
            -10.0,
            20.0,
            20.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        let (state, lit_c) = if ctx.canvas.sim_running() {
            let overload = ctx.canvas.item_overload_state(ctx.item_id);
            let is_warning = overload.is_some_and(|st| st.warning);
            let is_crashed = overload.is_some_and(|st| st.crashed);

            let ir_cur = ctx.pin_current(PIN_RGB_R).unwrap_or(0.0).max(0.0);
            let ig_cur = ctx.pin_current(PIN_RGB_G).unwrap_or(0.0).max(0.0);
            let ib_cur = ctx.pin_current(PIN_RGB_B).unwrap_or(0.0).max(0.0);

            let vc = ctx.pin_voltage(PIN_RGB_C).unwrap_or(0.0);
            let vr = ctx.pin_voltage(PIN_RGB_R).unwrap_or(0.0);
            let vg = ctx.pin_voltage(PIN_RGB_G).unwrap_or(0.0);
            let vb = ctx.pin_voltage(PIN_RGB_B).unwrap_or(0.0);

            let (v_diff_r, v_diff_g, v_diff_b) = if self.common_anode {
                ((vc - vr).max(0.0), (vc - vg).max(0.0), (vc - vb).max(0.0))
            } else {
                ((vr - vc).max(0.0), (vg - vc).max(0.0), (vb - vc).max(0.0))
            };

            let ir = ir_cur / self.i_max_r.max(0.001);
            let ig = ig_cur / self.i_max_g.max(0.001);
            let ib = ib_cur / self.i_max_b.max(0.001);

            let fr = if ir > 0.0 {
                ir
            } else if v_diff_r > self.v_th_r * 0.95 {
                ((v_diff_r - self.v_th_r).max(0.0)
                    / (self.r_res_r.max(0.1) * self.i_max_r.max(0.001)))
                .clamp(0.05, 1.0)
            } else {
                0.0
            };
            let fg = if ig > 0.0 {
                ig
            } else if v_diff_g > self.v_th_g * 0.95 {
                ((v_diff_g - self.v_th_g).max(0.0)
                    / (self.r_res_g.max(0.1) * self.i_max_g.max(0.001)))
                .clamp(0.05, 1.0)
            } else {
                0.0
            };
            let fb = if ib > 0.0 {
                ib
            } else if v_diff_b > self.v_th_b * 0.95 {
                ((v_diff_b - self.v_th_b).max(0.0)
                    / (self.r_res_b.max(0.1) * self.i_max_b.max(0.001)))
                .clamp(0.05, 1.0)
            } else {
                0.0
            };

            let lit = is_warning || fr > 0.005 || fg > 0.005 || fb > 0.005;
            let final_r = if is_warning {
                255
            } else {
                (fr.clamp(0.0, 1.0) * 255.0).round() as u8
            };
            let final_g = if is_warning {
                150
            } else {
                (fg.clamp(0.0, 1.0) * 255.0).round() as u8
            };
            let final_b = if is_warning {
                0
            } else {
                (fb.clamp(0.0, 1.0) * 255.0).round() as u8
            };

            let intensity = (fr.max(fg).max(fb)).clamp(0.15, 1.5);
            (
                BulbState {
                    is_lit: lit && !is_crashed,
                    intensity: if is_warning { 1.5 } else { intensity },
                    is_warning,
                    is_crashed,
                },
                Color::rgb(final_r, final_g, final_b),
            )
        } else {
            (BulbState::unlit(), Color::rgb(0, 0, 0))
        };

        paint_indicator_bulb(d, ctx, 0.0, 0.0, 6.0, lit_c, &state);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_rgb_led(&mut self, x: f64, y: f64) -> String {
        let id = format!("RGBLed-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::rgb_led(&id, x, y, false));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rgb_led() {
        let r = RgbLed::default();
        assert_eq!(r.type_id(), "RgbLed");
        assert!(!r.common_anode);
        assert_eq!(r.v_th_r, 2.0);
        assert_eq!(r.v_th_g, 3.0);
        assert_eq!(r.v_th_b, 3.0);
        assert_eq!(r.pin_geoms().len(), 4);
        assert_eq!(r.body(), Rect::new(-10.0, -10.0, 20.0, 20.0));
    }
}

const RGB_LED_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-rPin",
        x: -16.0,
        y: -8.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-gPin",
        x: -16.0,
        y: 0.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-bPin",
        x: -16.0,
        y: 8.0,
        angle: 180,
        length: 6.0,
        direction: None,
    },
    PinGeom {
        suffix: "-cPin",
        x: 16.0,
        y: 0.0,
        angle: 0,
        length: 6.0,
        direction: None,
    },
];

fn rgb_led_pins() -> &'static [PinGeom] {
    &RGB_LED_PINS
}
