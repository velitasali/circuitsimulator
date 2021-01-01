//! Shape: schematic visual annotation and geometry shapes.

use std::sync::Arc;

use super::component::PropGroup;
use super::drawable::Drawable;
use super::props::{
    PropDef, PropError, PropValue, expect_bool, expect_float, expect_int, expect_string,
};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx, parse_hex};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::sim1::*;

const PROP_FONT_COLOR: &str = "FontColor";
const PROP_FONT_SIZE: &str = "FontSize";

const MIN_SIZE: f64 = 1.0;
const MAX_SIZE: f64 = 10000.0;
const MIN_FONT_SIZE: i64 = 1;
const MAX_FONT_SIZE: i64 = 144;
const MIN_BORDER: i64 = 0;
const MAX_BORDER: i64 = 100;
const MIN_MARGIN: i64 = 0;
const MAX_MARGIN: i64 = 100;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Rounded,
    Ellipse,
    Line,
    Arc,
    Text,
    Image,
}

impl ShapeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Rounded => "Rounded",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Arc => "Arc",
            Self::Text => "Text",
            Self::Image => "Image",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "rounded" => Self::Rounded,
            "ellipse" => Self::Ellipse,
            "line" => Self::Line,
            "arc" => Self::Arc,
            "text" | "textcomponent" => Self::Text,
            "image" => Self::Image,
            _ => Self::Rectangle,
        }
    }
}

impl std::fmt::Display for ShapeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const SHAPE_KINDS: &[&str] = &[
    "Rectangle",
    "Rounded",
    "Ellipse",
    "Line",
    "Arc",
    "Text",
    "Image",
];

pub fn decode_image_bytes(bytes: &[u8]) -> Option<tiny_skia::Pixmap> {
    if bytes.is_empty() {
        return None;
    }
    if let Ok(pm) = tiny_skia::Pixmap::decode_png(bytes) {
        return Some(pm);
    }
    if let Ok(dyn_img) = image::load_from_memory(bytes) {
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        if let Some(mut pm) = tiny_skia::Pixmap::new(w, h) {
            let src_bytes = rgba.as_raw();
            for (i, pixel) in pm.pixels_mut().iter_mut().enumerate() {
                let r = src_bytes[i * 4];
                let g = src_bytes[i * 4 + 1];
                let b = src_bytes[i * 4 + 2];
                let a = src_bytes[i * 4 + 3];
                if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a) {
                    *pixel = c;
                }
            }
            return Some(pm);
        }
    }
    None
}

pub fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut chars = hex.chars().filter(|c| !c.is_whitespace());
    while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
        if let (Some(d1), Some(d2)) = (c1.to_digit(16), c2.to_digit(16)) {
            bytes.push(((d1 << 4) | d2) as u8);
        }
    }
    bytes
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn measure_text(text: &str, font_size: f64) -> (f64, f64) {
    let lines: Vec<&str> = text.split('\n').collect();
    let num_lines = lines.len().max(1);
    let line_height = font_size * 1.2;
    let mut max_w = 0.0f64;
    for line in lines {
        let w = (line.chars().count() as f64) * (font_size * 0.6);
        if w > max_w {
            max_w = w;
        }
    }
    (max_w.max(font_size * 2.0), (num_lines as f64) * line_height)
}

/// Schematic shape annotation (Rectangle, Ellipse, Line, Text, Image).
#[derive(Clone, Debug)]
pub struct Shape {
    pub shape_kind: ShapeKind,
    pub width: f64,
    pub height: f64,
    pub text: String,
    pub color: String,
    pub font: String,
    pub font_color: String,
    pub font_size: i32,
    pub border: i32,
    pub opacity: f64,
    pub margin: i32,
    pub fixed_w: bool,
    pub embed_bck: bool,
    pub image_file: String,
    pub bck_data: String,
    pub image_pixmap: Option<Arc<tiny_skia::Pixmap>>,
}

impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        self.shape_kind == other.shape_kind
            && (self.width - other.width).abs() < 1e-6
            && (self.height - other.height).abs() < 1e-6
            && self.text == other.text
            && self.color == other.color
            && self.font == other.font
            && self.font_color == other.font_color
            && self.font_size == other.font_size
            && self.border == other.border
            && (self.opacity - other.opacity).abs() < 1e-6
            && self.margin == other.margin
            && self.fixed_w == other.fixed_w
            && self.embed_bck == other.embed_bck
            && self.image_file == other.image_file
            && self.bck_data == other.bck_data
    }
}

impl Shape {
    pub fn new_for_kind(kind: ShapeKind) -> Self {
        match kind {
            ShapeKind::Text => Self {
                shape_kind: ShapeKind::Text,
                width: 64.0,
                height: 32.0,
                text: "... TEXT ...".into(),
                color: "#ffffdc".into(),
                font: "SansSerif".into(),
                font_color: "#000000".into(),
                font_size: 10,
                border: 1,
                opacity: 1.0,
                margin: 5,
                fixed_w: false,
                embed_bck: false,
                image_file: String::new(),
                bck_data: String::new(),
                image_pixmap: None,
            },
            ShapeKind::Image => Self {
                shape_kind: ShapeKind::Image,
                width: 80.0,
                height: 80.0,
                text: String::new(),
                color: "#808080".into(),
                font: "SansSerif".into(),
                font_color: "#000000".into(),
                font_size: 10,
                border: 0,
                opacity: 1.0,
                margin: 0,
                fixed_w: false,
                embed_bck: false,
                image_file: String::new(),
                bck_data: String::new(),
                image_pixmap: None,
            },
            _ => Self {
                shape_kind: kind,
                width: 50.0,
                height: 30.0,
                text: String::new(),
                color: "#808080".into(),
                font: "SansSerif".into(),
                font_color: "#000000".into(),
                font_size: 10,
                border: 2,
                opacity: 1.0,
                margin: 0,
                fixed_w: false,
                embed_bck: false,
                image_file: String::new(),
                bck_data: String::new(),
                image_pixmap: None,
            },
        }
    }
}

impl crate::canvas::Item {
    pub fn shape(id: impl Into<String>, x: f64, y: f64, kind: impl AsRef<str>) -> Self {
        let sk = ShapeKind::from_str_name(kind.as_ref());
        Self::new(id, x, y, Shape::new_for_kind(sk))
    }
}

impl Default for Shape {
    fn default() -> Self {
        Self::new_for_kind(ShapeKind::Rectangle)
    }
}

impl Shape {
    pub const TYPE_ID: &'static str = "Shape";

    pub fn to_element_kind(&self) -> Kind {
        Kind::Shape {
            shape_kind: self.shape_kind.as_str().to_string(),
            width: self.width,
            height: self.height,
            text: self.text.clone(),
            color: self.color.clone(),
            font: self.font.clone(),
            font_color: self.font_color.clone(),
            font_size: self.font_size,
            border: self.border,
            opacity: self.opacity,
        }
    }

    fn get_shape_kind(&self) -> PropValue {
        PropValue::Enum(self.shape_kind.as_str().to_string())
    }
    fn set_shape_kind(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("ShapeKind", v)?;
        self.shape_kind = ShapeKind::from_str_name(&s);
        Ok(())
    }

    fn get_width(&self) -> PropValue {
        PropValue::Float(self.width)
    }
    fn set_width(&mut self, v: PropValue) -> Result<(), PropError> {
        self.width = expect_float("Width", v)?.clamp(MIN_SIZE, MAX_SIZE);
        Ok(())
    }

    fn get_height(&self) -> PropValue {
        PropValue::Float(self.height)
    }
    fn set_height(&mut self, v: PropValue) -> Result<(), PropError> {
        self.height = expect_float("Height", v)?.clamp(MIN_SIZE, MAX_SIZE);
        Ok(())
    }

    fn get_text(&self) -> PropValue {
        PropValue::String(self.text.clone())
    }
    fn set_text(&mut self, v: PropValue) -> Result<(), PropError> {
        self.text = expect_string("Text", v)?;
        Ok(())
    }

    fn get_color(&self) -> PropValue {
        PropValue::String(self.color.clone())
    }
    fn set_color(&mut self, v: PropValue) -> Result<(), PropError> {
        self.color = expect_string("Color", v)?;
        Ok(())
    }

    fn get_font(&self) -> PropValue {
        PropValue::String(self.font.clone())
    }
    fn set_font(&mut self, v: PropValue) -> Result<(), PropError> {
        self.font = expect_string("Font", v)?;
        Ok(())
    }

    fn get_font_color(&self) -> PropValue {
        PropValue::String(self.font_color.clone())
    }
    fn set_font_color(&mut self, v: PropValue) -> Result<(), PropError> {
        self.font_color = expect_string("FontColor", v)?;
        Ok(())
    }

    fn get_font_size(&self) -> PropValue {
        PropValue::Int(self.font_size as i64)
    }
    fn set_font_size(&mut self, v: PropValue) -> Result<(), PropError> {
        self.font_size = expect_int("FontSize", v)?.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE) as i32;
        Ok(())
    }

    fn get_border(&self) -> PropValue {
        PropValue::Int(self.border as i64)
    }
    fn set_border(&mut self, v: PropValue) -> Result<(), PropError> {
        self.border = expect_int("Border", v)?.clamp(MIN_BORDER, MAX_BORDER) as i32;
        Ok(())
    }

    fn get_opacity(&self) -> PropValue {
        PropValue::Float(self.opacity)
    }
    fn set_opacity(&mut self, v: PropValue) -> Result<(), PropError> {
        self.opacity = expect_float("Opacity", v)?.clamp(0.0, 1.0);
        Ok(())
    }

    fn get_margin(&self) -> PropValue {
        PropValue::Int(self.margin as i64)
    }
    fn set_margin(&mut self, v: PropValue) -> Result<(), PropError> {
        self.margin = expect_int("Margin", v)?.clamp(MIN_MARGIN, MAX_MARGIN) as i32;
        Ok(())
    }

    fn get_fixed_w(&self) -> PropValue {
        PropValue::Bool(self.fixed_w)
    }
    fn set_fixed_w(&mut self, v: PropValue) -> Result<(), PropError> {
        self.fixed_w = expect_bool("FixedWidth", v)?;
        Ok(())
    }

    fn get_embed_bck(&self) -> PropValue {
        PropValue::Bool(self.embed_bck)
    }
    fn set_embed_bck(&mut self, v: PropValue) -> Result<(), PropError> {
        self.embed_bck = expect_bool("EmbedBck", v)?;
        Ok(())
    }

    fn get_image_file(&self) -> PropValue {
        PropValue::String(self.image_file.clone())
    }
    fn set_image_file(&mut self, v: PropValue) -> Result<(), PropError> {
        self.image_file = expect_string("ImageFile", v)?;
        Ok(())
    }

    fn get_bck_data(&self) -> PropValue {
        PropValue::String(self.bck_data.clone())
    }
    fn set_bck_data(&mut self, v: PropValue) -> Result<(), PropError> {
        let hex = expect_string("BckGndData", v)?;
        self.bck_data = hex.clone();
        if !hex.is_empty() {
            let bytes = hex_to_bytes(&hex);
            if let Some(pm) = decode_image_bytes(&bytes) {
                self.image_pixmap = Some(Arc::new(pm));
            }
        }
        Ok(())
    }

    pub fn write_item(&self, circ_id: &str, graphic: &crate::circ1::GraphicAttrs) -> String {
        let mut attrs = Vec::new();
        attrs.push(("ShapeKind", self.shape_kind.as_str().to_string()));
        match self.shape_kind {
            ShapeKind::Rectangle
            | ShapeKind::Rounded
            | ShapeKind::Ellipse
            | ShapeKind::Line
            | ShapeKind::Arc => {
                attrs.push((PROP_H_SIZE, self.width.to_string()));
                attrs.push((PROP_V_SIZE, self.height.to_string()));
                attrs.push((PROP_BORDER, self.border.to_string()));
                attrs.push((PROP_COLOR, self.color.clone()));
                attrs.push((PROP_OPACITY, self.opacity.to_string()));
            }
            ShapeKind::Text => {
                attrs.push((PROP_MARGIN, self.margin.to_string()));
                attrs.push((PROP_BORDER, self.border.to_string()));
                attrs.push((PROP_COLOR, self.color.clone()));
                attrs.push((PROP_OPACITY, self.opacity.to_string()));
                attrs.push((PROP_FONT, self.font.clone()));
                attrs.push((PROP_FONT_COLOR, self.font_color.clone()));
                attrs.push((PROP_FONT_SIZE, self.font_size.to_string()));
                attrs.push((PROP_FIXED_WIDTH, self.fixed_w.to_string()));
                attrs.push((
                    PROP_TEXT,
                    self.text
                        .replace('\n', "&#xa;")
                        .replace('"', "&#x22;")
                        .replace('=', "&#x3D;")
                        .replace('<', "&#x3C;")
                        .replace('>', "&#x3E;"),
                ));
            }
            ShapeKind::Image => {
                attrs.push((PROP_H_SIZE, self.width.to_string()));
                attrs.push((PROP_V_SIZE, self.height.to_string()));
                attrs.push((PROP_BORDER, self.border.to_string()));
                attrs.push((PROP_OPACITY, self.opacity.to_string()));
                attrs.push((PROP_EMBEED_BCK, self.embed_bck.to_string()));
                attrs.push((PROP_IMAGE_FILE, self.image_file.clone()));
                attrs.push((PROP_BCKGND_DATA, self.bck_data.clone()));
            }
        }
        let attr_refs: Vec<(&str, String)> = attrs;
        crate::circ1::write_item_line(self.type_id(), circ_id, &attr_refs, graphic)
    }
}

impl Component for Shape {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Configurable schematic shape annotation."
    }

    fn props() -> &'static [PropDef<Self>] {
        const W: PropDef<Shape> = {
            let mut p = PropDef::float(
                PROP_H_SIZE,
                "Width",
                "_px",
                MIN_SIZE,
                MAX_SIZE,
                Shape::get_width,
                Shape::set_width,
            )
            .with_info("Width of the shape, in pixels.");
            p.structural = true;
            p
        };
        const H: PropDef<Shape> = {
            let mut p = PropDef::float(
                PROP_V_SIZE,
                "Height",
                "_px",
                MIN_SIZE,
                MAX_SIZE,
                Shape::get_height,
                Shape::set_height,
            )
            .with_info("Height of the shape, in pixels.");
            p.structural = true;
            p
        };
        const SK: PropDef<Shape> = {
            let mut p = PropDef::enumeration(
                "ShapeKind",
                "Shape",
                SHAPE_KINDS,
                Shape::get_shape_kind,
                Shape::set_shape_kind,
            )
            .with_info("Geometric primitive shape type (e.g. Rectangle, Ellipse, Line, Text).");
            p.show_by_default = false;
            p
        };
        static PROPS: &[PropDef<Shape>] = &[
            SK,
            W,
            H,
            PropDef::string(PROP_TEXT, "Text", Shape::get_text, Shape::set_text)
                .with_info("Text content displayed in the text box."),
            PropDef::string(PROP_COLOR, "Color", Shape::get_color, Shape::set_color)
                .with_info("Click on the color box to choose a new color."),
            PropDef::string(PROP_FONT, "Font", Shape::get_font, Shape::set_font)
                .with_info("Font family name."),
            PropDef::string(
                PROP_FONT_COLOR,
                "Font Color",
                Shape::get_font_color,
                Shape::set_font_color,
            )
            .with_info("Click on the color box to choose a new font color."),
            PropDef::int(
                PROP_FONT_SIZE,
                "Font Size",
                MIN_FONT_SIZE,
                MAX_FONT_SIZE,
                Shape::get_font_size,
                Shape::set_font_size,
            )
            .with_info("Font size in pixels.")
            .with_unit("_px"),
            PropDef::int(
                PROP_BORDER,
                "Border",
                MIN_BORDER,
                MAX_BORDER,
                Shape::get_border,
                Shape::set_border,
            )
            .with_info("Width of the border line.")
            .with_unit("_px"),
            PropDef::float(
                PROP_OPACITY,
                "Opacity",
                "",
                0.0,
                1.0,
                Shape::get_opacity,
                Shape::set_opacity,
            )
            .with_info("Value from 0 to 1: 0 for transparent to 1 for 100% opaque."),
            PropDef::int(
                PROP_MARGIN,
                "Margin",
                MIN_MARGIN,
                MAX_MARGIN,
                Shape::get_margin,
                Shape::set_margin,
            )
            .with_info("Space between text and border.")
            .with_unit("_px"),
            PropDef::bool(
                PROP_FIXED_WIDTH,
                "Fixed Width",
                Shape::get_fixed_w,
                Shape::set_fixed_w,
            )
            .with_info("Characters have constant width."),
            PropDef::bool(
                PROP_EMBEED_BCK,
                "Embeed background",
                Shape::get_embed_bck,
                Shape::set_embed_bck,
            )
            .with_info(
                "Embed the image data in the circuit file instead of referencing the external file path.",
            ),
            PropDef::string(
                PROP_IMAGE_FILE,
                "Image File",
                Shape::get_image_file,
                Shape::set_image_file,
            )
            .with_info("Path to external image file."),
            PropDef::string(
                PROP_BCKGND_DATA,
                "BckGndData",
                Shape::get_bck_data,
                Shape::set_bck_data,
            )
            .with_info("Embedded image hex data."),
        ];
        PROPS
    }

    fn prop_rows(&self) -> Vec<super::component::PropRow> {
        let mut rows = Vec::new();
        for def in Self::props() {
            let (visible, enabled) = match (self.shape_kind, def.id) {
                (
                    ShapeKind::Text,
                    PROP_H_SIZE | PROP_V_SIZE | PROP_EMBEED_BCK | PROP_IMAGE_FILE
                    | PROP_BCKGND_DATA,
                ) => (false, false),
                (ShapeKind::Text, PROP_COLOR) => (true, true),
                (
                    ShapeKind::Image,
                    PROP_MARGIN | PROP_TEXT | PROP_FONT | PROP_FONT_COLOR | PROP_FONT_SIZE
                    | PROP_FIXED_WIDTH | PROP_COLOR | PROP_IMAGE_FILE | PROP_BCKGND_DATA,
                ) => (false, false),
                (
                    ShapeKind::Rectangle
                    | ShapeKind::Rounded
                    | ShapeKind::Ellipse
                    | ShapeKind::Line
                    | ShapeKind::Arc,
                    PROP_MARGIN | PROP_TEXT | PROP_FONT | PROP_FONT_COLOR | PROP_FONT_SIZE
                    | PROP_FIXED_WIDTH | PROP_EMBEED_BCK | PROP_IMAGE_FILE | PROP_BCKGND_DATA,
                ) => (false, false),
                (_, "ShapeKind") => (false, false),
                _ => (true, true),
            };

            let caption = if self.shape_kind == ShapeKind::Text && def.id == PROP_COLOR {
                "Background Color"
            } else {
                def.caption
            };

            let kind = match def.kind {
                super::props::PropKind::Float { .. } => "double",
                super::props::PropKind::Int { .. } => "int",
                super::props::PropKind::Bool => "bool",
                super::props::PropKind::String
                    if def.id == PROP_COLOR || def.id == PROP_FONT_COLOR =>
                {
                    "color"
                }
                super::props::PropKind::String if def.id == PROP_TEXT => "textEdit",
                super::props::PropKind::String => "string",
                super::props::PropKind::Enum { .. } => "enum",
            };

            rows.push(super::component::PropRow {
                name: def.id,
                kind,
                caption,
                info: def.info,
                unit: def.unit,
                options: match def.kind {
                    super::props::PropKind::Enum { options } => {
                        options.iter().map(|s| s.to_string()).collect()
                    }
                    _ => Vec::new(),
                },
                visible,
                enabled,
            });
        }
        rows
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let rows = self.prop_rows();
        match self.shape_kind {
            ShapeKind::Text => super::group_rows_by(
                rows,
                &[
                    (
                        "Main",
                        &[PROP_MARGIN, PROP_BORDER, PROP_COLOR, PROP_OPACITY],
                    ),
                    (
                        "Text",
                        &[
                            PROP_FONT,
                            PROP_FONT_COLOR,
                            PROP_FONT_SIZE,
                            PROP_FIXED_WIDTH,
                            PROP_TEXT,
                        ],
                    ),
                ],
            ),
            ShapeKind::Image => super::group_rows_by(
                rows,
                &[
                    (
                        "Main",
                        &[PROP_H_SIZE, PROP_V_SIZE, PROP_EMBEED_BCK, PROP_BORDER],
                    ),
                    ("Color", &[PROP_OPACITY]),
                ],
            ),
            _ => super::group_rows_by(
                rows,
                &[
                    ("Main", &[PROP_H_SIZE, PROP_V_SIZE, PROP_BORDER]),
                    ("Color", &[PROP_COLOR, PROP_OPACITY]),
                ],
            ),
        }
    }

    fn get_prop_text(&self, name: &str) -> Option<String> {
        let clean = name.replace('_', "").to_ascii_lowercase();
        match clean.as_str() {
            "hsize" | "width" => Some(self.width.to_string()),
            "vsize" | "height" => Some(self.height.to_string()),
            "border" => Some(self.border.to_string()),
            "color" => Some(self.color.clone()),
            "opacity" => Some(self.opacity.to_string()),
            "margin" => Some(self.margin.to_string()),
            "font" => Some(self.font.clone()),
            "fontcolor" => Some(self.font_color.clone()),
            "fontsize" => Some(self.font_size.to_string()),
            "fixedwidth" => Some(self.fixed_w.to_string()),
            "text" => Some(
                self.text
                    .replace('\n', "&#xa;")
                    .replace('"', "&#x22;")
                    .replace('=', "&#x3D;")
                    .replace('<', "&#x3C;")
                    .replace('>', "&#x3E;"),
            ),
            "embeedbck" | "embedbck" | "embeedbackground" | "embedbackground" => {
                Some(self.embed_bck.to_string())
            }
            "imagefile" => Some(self.image_file.clone()),
            "bckgnddata" | "bckdata" => Some(self.bck_data.clone()),
            "shapekind" | "shape" => Some(self.shape_kind.as_str().to_string()),
            _ => None,
        }
    }

    fn set_prop_text(
        &mut self,
        name: &str,
        text: &str,
    ) -> Result<super::ComponentChange, PropError> {
        let err = || PropError::Invalid {
            id: name.to_string(),
            value: text.to_string(),
        };
        let parse_f = |s: &str| -> Option<f64> {
            let t = s.trim();
            let t = t.strip_suffix("_px").unwrap_or(t).trim();
            let t = t.strip_suffix("px").unwrap_or(t).trim();
            t.parse::<f64>().ok()
        };
        let parse_i = |s: &str| -> Option<i32> {
            let t = s.trim();
            let t = t.strip_suffix("_px").unwrap_or(t).trim();
            let t = t.strip_suffix("px").unwrap_or(t).trim();
            t.parse::<i32>().ok()
        };
        let clean = name.replace('_', "").to_ascii_lowercase();
        match clean.as_str() {
            "hsize" | "width" => {
                self.width = parse_f(text).ok_or_else(err)?.clamp(MIN_SIZE, MAX_SIZE);
                Ok(super::change::ComponentChange::document(""))
            }
            "vsize" | "height" => {
                self.height = parse_f(text).ok_or_else(err)?.clamp(MIN_SIZE, MAX_SIZE);
                Ok(super::change::ComponentChange::document(""))
            }
            "border" => {
                self.border = parse_i(text)
                    .ok_or_else(err)?
                    .clamp(MIN_BORDER as i32, MAX_BORDER as i32);
                Ok(super::change::ComponentChange::document(""))
            }
            "color" => {
                self.color = text.trim().to_string();
                Ok(super::change::ComponentChange::document(""))
            }
            "opacity" => {
                self.opacity = text
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| err())?
                    .clamp(0.0, 1.0);
                Ok(super::change::ComponentChange::document(""))
            }
            "margin" => {
                self.margin = parse_i(text)
                    .ok_or_else(err)?
                    .clamp(MIN_MARGIN as i32, MAX_MARGIN as i32);
                Ok(super::change::ComponentChange::document(""))
            }
            "font" => {
                self.font = text.trim().to_string();
                Ok(super::change::ComponentChange::document(""))
            }
            "fontcolor" => {
                self.font_color = text.trim().to_string();
                Ok(super::change::ComponentChange::document(""))
            }
            "fontsize" => {
                self.font_size = parse_i(text)
                    .ok_or_else(err)?
                    .clamp(MIN_FONT_SIZE as i32, MAX_FONT_SIZE as i32);
                Ok(super::change::ComponentChange::document(""))
            }
            "fixedwidth" => {
                self.fixed_w = matches!(text.trim(), "true" | "1" | "yes");
                Ok(super::change::ComponentChange::document(""))
            }
            "text" => {
                self.text = text
                    .replace("&#xa;", "\n")
                    .replace("&#x22;", "\"")
                    .replace("&#x3D;", "=")
                    .replace("&#x3C;", "<")
                    .replace("&#x3E;", ">");
                Ok(super::change::ComponentChange::document(""))
            }
            "embeedbck" | "embedbck" | "embeedbackground" | "embedbackground" => {
                self.embed_bck = matches!(text.trim(), "true" | "1" | "yes");
                Ok(super::change::ComponentChange::document(""))
            }
            "imagefile" => {
                self.image_file = text.trim().to_string();
                Ok(super::change::ComponentChange::document(""))
            }
            "bckgnddata" | "bckdata" => {
                self.bck_data = text.trim().to_string();
                if !self.bck_data.is_empty() {
                    let bytes = hex_to_bytes(&self.bck_data);
                    if let Some(pm) = decode_image_bytes(&bytes) {
                        self.image_pixmap = Some(Arc::new(pm));
                    }
                }
                Ok(super::change::ComponentChange::document(""))
            }
            "shapekind" | "shape" => {
                self.shape_kind = ShapeKind::from_str_name(text);
                Ok(super::change::ComponentChange::document(""))
            }
            _ => Err(PropError::Unknown(name.into())),
        }
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        Vec::new()
    }

    fn body(&self) -> Rect {
        match self.shape_kind {
            ShapeKind::Text => {
                let (tw, th) = measure_text(&self.text, self.font_size as f64);
                let m = self.margin.max(0) as f64;
                Rect::new(-m, -m, tw + 2.0 * m, th + 2.0 * m)
            }
            _ => Rect::new(
                -self.width / 2.0,
                -self.height / 2.0,
                self.width,
                self.height,
            ),
        }
    }
}

impl Stampable for Shape {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {
        // Shape is a visual schematic annotation.
    }
}

impl Drawable for Shape {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        match self.shape_kind {
            ShapeKind::Rectangle | ShapeKind::Rounded => {
                let col = parse_hex(&self.color).fade(self.opacity);
                let x = -self.width / 2.0;
                let y = -self.height / 2.0;
                d.fill_round_rect(x, y, self.width, self.height, 3.5, col);
                if self.border > 0 {
                    d.stroke_round_rect(
                        x,
                        y,
                        self.width,
                        self.height,
                        3.5,
                        ctx.pal.border,
                        self.border as f64,
                    );
                }
            }
            ShapeKind::Ellipse => {
                let col = parse_hex(&self.color).fade(self.opacity);
                let x = -self.width / 2.0;
                let y = -self.height / 2.0;
                d.fill_ellipse(x, y, self.width, self.height, col);
                if self.border > 0 {
                    d.stroke_ellipse(
                        x,
                        y,
                        self.width,
                        self.height,
                        ctx.pal.border,
                        self.border as f64,
                    );
                }
            }
            ShapeKind::Line | ShapeKind::Arc => {
                let col = parse_hex(&self.color).fade(self.opacity);
                let w2 = self.width / 2.0;
                let h2 = self.height / 2.0;
                d.line(-w2, h2, w2, -h2, col, (self.border as f64).max(1.0));
            }
            ShapeKind::Text => {
                let bg = parse_hex(&self.color).fade(self.opacity);
                let (tw, th) = measure_text(&self.text, self.font_size as f64);
                let m = self.margin.max(0) as f64;
                d.fill_rect(-m, -m, tw + 2.0 * m, th + 2.0 * m, bg);
                if self.border > 0 {
                    d.stroke_rect(
                        -m,
                        -m,
                        tw + 2.0 * m,
                        th + 2.0 * m,
                        ctx.pal.border,
                        self.border as f64,
                    );
                }
                let fg = parse_hex(&self.font_color).fade(self.opacity);
                d.text(
                    0.0,
                    0.0,
                    &self.text,
                    self.font_size as f64,
                    fg,
                    Align::TopLeft,
                );
            }
            ShapeKind::Image => {
                let x = -self.width / 2.0;
                let y = -self.height / 2.0;
                if let Some(pm) = &self.image_pixmap {
                    d.draw_pixmap_rect(x, y, self.width, self.height, pm, self.opacity);
                } else {
                    d.fill_rect(
                        x,
                        y,
                        self.width,
                        self.height,
                        ctx.pal.body.fade(0.15 * self.opacity),
                    );
                    let icon_col = ctx.pal.border.fade(0.4 * self.opacity);
                    d.text(0.0, 0.0, "Image", 10.0, icon_col, Align::Center);
                }
                if self.border > 0 {
                    d.stroke_rect(
                        x,
                        y,
                        self.width,
                        self.height,
                        ctx.pal.border,
                        self.border as f64,
                    );
                }
            }
        }
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_shape(&mut self, kind: &str, x: f64, y: f64) -> String {
        let sk = ShapeKind::from_str_name(kind);
        let id_prefix = match sk {
            ShapeKind::Rectangle | ShapeKind::Rounded => "Rectangle",
            ShapeKind::Ellipse => "Ellipse",
            ShapeKind::Line | ShapeKind::Arc => "Line",
            ShapeKind::Text => "TextComponent",
            ShapeKind::Image => "Image",
        };
        let id = format!("{id_prefix}-{}", self.items.len() + 1);
        self.items.push(crate::canvas::Item::shape(&id, x, y, kind));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_defaults_and_props() {
        let mut shape = Shape::default();
        assert_eq!(shape.type_id(), "Shape");
        assert_eq!(shape.get_prop_text("ShapeKind").unwrap(), "Rectangle");
        assert_eq!(shape.get_prop_text("Width").unwrap(), "50");
        assert_eq!(shape.get_prop_text("Height").unwrap(), "30");
        assert_eq!(shape.get_prop_text("Color").unwrap(), "#808080");
        assert_eq!(shape.get_prop_text("Border").unwrap(), "2");
        assert_eq!(shape.get_prop_text("Opacity").unwrap(), "1");

        shape.set_prop_text("ShapeKind", "Ellipse").unwrap();
        assert_eq!(shape.shape_kind, ShapeKind::Ellipse);
        assert_eq!(shape.type_id(), "Shape");
        shape.set_prop_text("Width", "100").unwrap();
        assert_eq!(shape.width, 100.0);
        shape.set_prop_text("Height", "50").unwrap();
        assert_eq!(shape.height, 50.0);
        shape.set_prop_text("Color", "#ffffff").unwrap();
        assert_eq!(shape.color, "#ffffff");
        shape.set_prop_text("Border", "3").unwrap();
        assert_eq!(shape.border, 3);
        shape.set_prop_text("Opacity", "0.5").unwrap();
        assert_eq!(shape.opacity, 0.5);

        // Text properties
        let text_shape = Shape::new_for_kind(ShapeKind::Text);
        assert_eq!(text_shape.type_id(), "Shape");
        assert_eq!(text_shape.get_prop_text("Margin").unwrap(), "5");
        assert_eq!(text_shape.get_prop_text("Border").unwrap(), "1");
        assert_eq!(text_shape.get_prop_text("Color").unwrap(), "#ffffdc");
        assert_eq!(text_shape.get_prop_text("Font").unwrap(), "SansSerif");
        assert_eq!(text_shape.get_prop_text("FontColor").unwrap(), "#000000");
        assert_eq!(text_shape.get_prop_text("FontSize").unwrap(), "10");
        assert_eq!(text_shape.get_prop_text("Fixed_Width").unwrap(), "false");
        assert_eq!(text_shape.text, "... TEXT ...");
    }

    #[test]
    fn shape_tabs_separation() {
        let rect = Shape::new_for_kind(ShapeKind::Rectangle);
        let groups = rect.prop_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Main");
        assert_eq!(groups[1].name, "Color");
        assert!(groups.iter().all(|g| g.name != "Text"));

        let line = Shape::new_for_kind(ShapeKind::Line);
        let groups = line.prop_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Main");
        assert_eq!(groups[1].name, "Color");
        assert!(groups.iter().all(|g| g.name != "Text"));

        let text = Shape::new_for_kind(ShapeKind::Text);
        let groups = text.prop_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Main");
        assert_eq!(groups[1].name, "Text");

        let img = Shape::new_for_kind(ShapeKind::Image);
        let groups = img.prop_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Main");
        assert_eq!(groups[1].name, "Color");
        assert!(groups.iter().all(|g| g.name != "Text"));
    }
}
