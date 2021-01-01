//! Resistor: two-terminal conductance plus color-band display flag.

use super::component::{clamp_positive, resistor_g, stamp_two_terminal, two_terminal_pins};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable, TwoTerminal};
use crate::canvas::Rect;
use crate::elements::{Kind, RESISTOR_DEFAULT_OHMS};
use crate::matrix::CircMatrix;

const MIN_OHMS: f64 = 1e-12;
const MAX_OHMS: f64 = 1e12;

/// Comp2Pin body `QRectF(-11, -4.5, 22, 9)`.
pub const RESISTOR_BODY: Rect = Rect {
    x: -11.0,
    y: -4.5,
    w: 22.0,
    h: 9.0,
};

/// `Component::boundingRect`: body inflated by SELECTION_MARGIN. Selection outline uses this.
pub fn resistor_selection_rect() -> Rect {
    RESISTOR_BODY.adjust(
        -crate::canvas::scene::SELECTION_MARGIN,
        -crate::canvas::scene::SELECTION_MARGIN,
        crate::canvas::scene::SELECTION_MARGIN,
        crate::canvas::scene::SELECTION_MARGIN,
    )
}

pub fn resistor_hit_rect() -> Rect {
    RESISTOR_BODY
}

impl crate::canvas::Item {
    pub fn resistor(id: impl Into<String>, x: f64, y: f64, resistance: f64) -> Self {
        Self::new(
            id,
            x,
            y,
            Resistor {
                resistance,
                show_bands: true,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resistor {
    pub resistance: f64,
    pub show_bands: bool,
}

impl Default for Resistor {
    fn default() -> Self {
        Self {
            resistance: RESISTOR_DEFAULT_OHMS,
            show_bands: true,
        }
    }
}

impl Resistor {
    pub const TYPE_ID: &'static str = "Resistor";
    pub fn new(resistance: f64) -> Self {
        Self {
            resistance: clamp_positive(resistance, MIN_OHMS),
            show_bands: true,
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Resistor {
            resistance: self.resistance.max(MIN_OHMS),
        }
    }

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        self.resistance = expect_float("Resistance", v)?.clamp(MIN_OHMS, MAX_OHMS);
        Ok(())
    }
    fn get_show_bands(&self) -> PropValue {
        PropValue::Bool(self.show_bands)
    }
    fn set_show_bands(&mut self, v: PropValue) -> Result<(), PropError> {
        self.show_bands = expect_bool("ShowBands", v)?;
        Ok(())
    }
}

impl Component for Resistor {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }
    fn description(&self) -> &'static str {
        "Simple resistance."
    }
    fn props() -> &'static [PropDef<Self>] {
        const RES: PropDef<Resistor> = {
            let mut p = PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                MIN_OHMS,
                MAX_OHMS,
                Resistor::get_resistance,
                Resistor::set_resistance,
            )
            .with_info("Resistance value, in ohms.");
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Resistor>] = &[
            RES,
            PropDef::bool(
                "ShowBands",
                "Show Bands",
                Resistor::get_show_bands,
                Resistor::set_show_bands,
            )
            .with_info("Show the resistance as color bands instead of a value label."),
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<CompPin> {
        two_terminal_pins(5.0)
    }
    fn body(&self) -> Rect {
        RESISTOR_BODY
    }
}

impl TwoTerminal for Resistor {}

impl Stampable for Resistor {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], _dt: f64) {
        stamp_two_terminal(matrix, pin_nodes, resistor_g(self.resistance), 0.0);
    }
}

use super::Drawable;
use crate::canvas::draw::{Color, Draw, PaintCtx, parse_hex};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

pub fn bands_of(r: f64) -> [i32; 4] {
    if r <= 0.0 {
        return [0, 0, 0, -1];
    }
    let exp = r.log10().floor();
    let mut val = (r / 10f64.powf(exp) * 10.0).round() as i32;
    let mut mult = exp as i32 - 1;
    if val >= 100 {
        val /= 10;
        mult += 1;
    }
    [val / 10, val % 10, mult, -1]
}

pub fn band_color(digit: i32) -> Color {
    match digit {
        -2 => parse_hex("#c0c0c0"),
        -1 => parse_hex("#cfb53b"),
        0 => Color::rgb(0, 0, 0),
        1 => parse_hex("#8b4513"),
        2 => Color::rgb(255, 0, 0),
        3 => parse_hex("#ffa500"),
        4 => Color::rgb(255, 255, 0),
        5 => parse_hex("#008000"),
        6 => Color::rgb(0, 0, 255),
        7 => parse_hex("#ee82ee"),
        8 => parse_hex("#808080"),
        9 => Color::rgb(255, 255, 255),
        _ => Color::rgb(0, 0, 0),
    }
}

pub fn paint_resistor_body(
    d: &mut dyn Draw,
    pal: &crate::canvas::draw::Palette,
    resistance: f64,
    show_bands: bool,
) {
    d.fill_rect(-11.0, -4.5, 22.0, 9.0, pal.body.fade(COMPONENT_FILL_ALPHA));
    if show_bands {
        let bands = bands_of(resistance);
        let xs = [-8.0, -4.0, 0.0, 6.0];
        for (i, x) in xs.iter().enumerate() {
            d.fill_rect(*x, -4.5, 1.0, 9.0, band_color(bands[i]));
        }
    }
    d.stroke_rect(-11.0, -4.5, 22.0, 9.0, pal.border, COMPONENT_BORDER_WIDTH);
}

impl Drawable for Resistor {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        paint_resistor_body(d, ctx.pal, self.resistance, self.show_bands);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_resistor(&mut self, x: f64, y: f64, resistance: f64) -> String {
        let id = format!("Resistor-{}", self.next_resistor);
        self.next_resistor += 1;
        self.items.push(crate::canvas::Item::resistor(
            &id,
            x,
            y,
            resistance.max(1e-12),
        ));
        id
    }

    pub fn add_default_resistor(&mut self, x: f64, y: f64) -> String {
        self.add_resistor(x, y, crate::elements::RESISTOR_DEFAULT_OHMS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let r = Resistor::default();
        assert_eq!(r.resistance, RESISTOR_DEFAULT_OHMS);
        assert!(r.show_bands);
        assert_eq!(
            r.get_prop_text("Resistance").unwrap(),
            format_si(RESISTOR_DEFAULT_OHMS, "Ω")
        );
    }

    #[test]
    fn stamp_100_ohm_to_ground() {
        let r = Resistor::new(100.0);
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        r.stamp(&mut m, &[0, usize::MAX], 0.0);
        m.add_coef(0, 1.0);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        assert!((v[0] - 100.0).abs() < 1e-6, "{}", v[0]);
    }
}
