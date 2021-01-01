//! SD card reader module accessed over SPI.

use super::props::{PropDef, PropError, PropValue, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::PinGeom;
use crate::canvas::Rect;
use crate::elements::Kind;
use crate::matrix::CircMatrix;

impl crate::canvas::Item {
    pub fn sd_card(id: impl Into<String>, x: f64, y: f64, file: impl Into<String>) -> Self {
        Self::new(id, x, y, SdCard { file: file.into() })
    }

    pub fn sdcard(id: impl Into<String>, x: f64, y: f64, file: impl Into<String>) -> Self {
        Self::sd_card(id, x, y, file)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SdCard {
    pub file: String,
}

impl Default for SdCard {
    fn default() -> Self {
        Self {
            file: String::new(),
        }
    }
}

impl SdCard {
    pub const TYPE_ID: &'static str = "SdCard";
    pub fn to_element_kind(&self) -> Kind {
        Kind::SdCard {
            file: self.file.clone(),
            card_inserted: true,
        }
    }

    fn get_file(&self) -> PropValue {
        PropValue::String(self.file.clone())
    }
    fn set_file(&mut self, v: PropValue) -> Result<(), PropError> {
        self.file = expect_string("File", v)?;
        Ok(())
    }
}

impl Component for SdCard {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "SD card reader, accessed over SPI."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<SdCard>] =
            &[
                PropDef::string("File", "File", SdCard::get_file, SdCard::set_file)
                    .with_info(
                        "Path to the \".img\" file used as the disk image for this card's storage.",
                    )
                    .with_info(
                        "Path to the \".img\" file used as the disk image for this card's storage.",
                    )
                    .with_info(
                        "Path to the \".img\" file used as the disk image for this card's storage.",
                    ),
            ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        sdcard_pins().iter().map(CompPin::from).collect()
    }

    fn body(&self) -> Rect {
        Rect::new(-24.0, -16.0, 56.0, 40.0)
    }
}

impl Stampable for SdCard {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

impl super::drawable::Drawable for SdCard {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        // PCB Board (56x40)
        d.fill_round_rect(
            -24.0,
            -16.0,
            56.0,
            40.0,
            3.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -24.0,
            -16.0,
            56.0,
            40.0,
            3.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Metal SD Socket cage
        d.fill_round_rect(4.0, -9.0, 24.0, 26.0, 1.0, ctx.pal.body.fade(0.8));
        d.stroke_round_rect(4.0, -9.0, 24.0, 26.0, 1.0, ctx.pal.border, 1.0);

        d.text(16.0, 4.0, "SD", 8.0, ctx.pal.border, Align::Center);
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_sdcard(&mut self, x: f64, y: f64) -> String {
        let id = format!("SdCard-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::sdcard(&id, x, y, ""));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sd_card() {
        let s = SdCard::default();
        assert_eq!(s.type_id(), "SdCard");
        assert!(s.file.is_empty());
        assert_eq!(s.pin_geoms().len(), 4);
    }
}

const SDCARD_PINS: [PinGeom; 4] = [
    PinGeom {
        suffix: "-PinCS",
        x: -32.0,
        y: -8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinDI",
        x: -32.0,
        y: 0.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinCK",
        x: -32.0,
        y: 8.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
    PinGeom {
        suffix: "-PinDO",
        x: -32.0,
        y: 16.0,
        angle: 180,
        length: 8.0,
        direction: None,
    },
];

fn sdcard_pins() -> &'static [PinGeom] {
    &SDCARD_PINS
}
