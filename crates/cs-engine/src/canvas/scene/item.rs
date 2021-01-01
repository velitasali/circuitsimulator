use crate::canvas::export::text::text_width;
use crate::canvas::geom::{Point, Rect, map_local, map_rect, map_scene};
use crate::canvas::pin::Pin;
pub use crate::components::*;

/// Canvas overflow status and required dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasOverflowInfo {
    pub has_overflow: bool,
    pub overflowing_count: usize,
    pub required_width: i32,
    pub required_height: i32,
    pub current_width: i32,
    pub current_height: i32,
    pub items_bounds: Rect,
}

/// Selection bounding margin around component body (8px).
pub const SELECTION_MARGIN: f64 = 8.0;

/// A programmable device (MCU, QEMU device, or board-mounted MCU).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProgrammableDevice {
    pub id: String,
    pub uid: String,
    pub label: String,
    pub display_text: String,
    pub kind: String,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub selected: bool,
    /// C++ `Component::rotation()`, degrees, Y-down (clockwise).
    pub rotation: f64,
    /// C++ `hflip`: 1 or -1.
    pub hflip: i32,
    /// C++ `vflip`: 1 or -1.
    pub vflip: i32,
    /// C++ `label` / idLabel.
    pub label: String,
    /// C++ `Show_id`.
    pub show_id: bool,
    pub label_x: f64,
    pub label_y: f64,
    pub label_rot: i32,
    /// C++ `Show_Val`.
    pub show_val: bool,
    /// C++ `ShowProp`.
    pub show_prop: String,
    pub val_x: f64,
    pub val_y: f64,
    pub val_rot: i32,
    pub kind: Part,
    pub custom_label_pos: bool,
    pub custom_val_pos: bool,
}

impl Item {
    pub fn new(id: impl Into<String>, x: f64, y: f64, kind: impl Into<Part>) -> Self {
        let id = id.into();
        let kind = kind.into();
        let default_show_prop = kind.default_show_prop().to_string();
        let mut it = Self {
            id: id.clone(),
            x,
            y,
            selected: false,
            rotation: 0.0,
            hflip: 1,
            vflip: 1,
            label: id,
            show_id: false,
            label_x: 0.0,
            label_y: 0.0,
            label_rot: 0,
            show_val: false,
            show_prop: default_show_prop,
            val_x: 0.0,
            val_y: 0.0,
            val_rot: 0,
            kind,
            custom_label_pos: false,
            custom_val_pos: false,
        };
        let (lx, ly) = it.default_label_pos();
        it.label_x = lx;
        it.label_y = ly;
        let (vx, vy) = it.default_val_pos();
        it.val_x = vx;
        it.val_y = vy;
        it
    }

    pub fn type_name(&self) -> &'static str {
        self.kind.type_name()
    }

    pub fn package(&self) -> Option<&crate::package::Package> {
        self.kind.package()
    }

    pub fn is_node(&self) -> bool {
        self.kind.is_node()
    }

    pub fn graphic_attrs(&self) -> crate::components::ser::GraphicAttrs {
        crate::components::ser::GraphicAttrs {
            pos: Some((self.x, self.y)),
            rotation: self.rotation,
            hflip: self.hflip,
            vflip: self.vflip,
            label: if self.label != self.id {
                Some(self.label.clone())
            } else {
                None
            },
            show_id: self.show_id,
            label_pos: if self.custom_label_pos {
                Some((self.label_x, self.label_y))
            } else {
                None
            },
            label_rot: self.label_rot,
            show_val: self.show_val,
            show_prop: if !self.show_prop.is_empty() {
                Some(self.show_prop.clone())
            } else {
                None
            },
            val_pos: if self.custom_val_pos {
                Some((self.val_x, self.val_y))
            } else {
                None
            },
            val_rot: self.val_rot,
        }
    }

    /// Calculate default label position horizontally centered above the component's clickable area.
    pub fn default_label_pos(&self) -> (f64, f64) {
        let rect = self.body_rect();
        let cx = rect.x + rect.w * 0.5;
        let tw = text_width(&self.label, 9.0, false);
        let x = cx - tw * 0.5;
        let y = rect.y - 12.0;
        (x, y)
    }

    /// Calculate default value position horizontally centered below the component's clickable area.
    pub fn default_val_pos(&self) -> (f64, f64) {
        let rect = self.body_rect();
        let cx = rect.x + rect.w * 0.5;
        let val_text = self.val_label_text();
        let tw = text_width(&val_text, 9.0, false);
        let x = cx - tw * 0.5;
        let y = rect.y + rect.h + 2.0;
        (x, y)
    }

    pub fn label_pos(&self) -> (f64, f64) {
        if self.custom_label_pos {
            (self.label_x, self.label_y)
        } else {
            self.default_label_pos()
        }
    }

    pub fn val_pos(&self) -> (f64, f64) {
        if self.custom_val_pos {
            (self.val_x, self.val_y)
        } else {
            self.default_val_pos()
        }
    }

    /// Convert current Part to simulation Kind for in-place parameter updates.
    pub fn to_element_kind(&self) -> Option<crate::elements::Kind> {
        self.kind.to_element_kind()
    }

    pub fn val_label_text(&self) -> String {
        if self.show_prop.is_empty() {
            return String::new();
        }
        self.prop_text(&self.show_prop).unwrap_or_default()
    }

    pub fn set_show_prop(&mut self, prop: &str, show: bool) -> bool {
        if show {
            self.show_prop = prop.to_string();
            self.show_val = true;
        } else {
            self.show_val = false;
            self.show_prop.clear();
        }
        true
    }

    pub fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn map_local(&self, local: Point) -> Point {
        map_local(
            self.position(),
            local,
            self.rotation,
            self.hflip,
            self.vflip,
        )
    }

    pub fn map_scene(&self, scene: Point) -> Point {
        map_scene(
            self.position(),
            scene,
            self.rotation,
            self.hflip,
            self.vflip,
        )
    }

    pub fn pin_scene_pos(&self, pin: &Pin) -> Point {
        self.map_local(pin.local)
    }

    pub fn body_rect(&self) -> Rect {
        self.kind.body()
    }

    pub fn local_hit_rect(&self) -> Rect {
        self.body_rect()
    }

    pub fn local_selection_rect(&self) -> Rect {
        self.body_rect().adjust(
            -SELECTION_MARGIN,
            -SELECTION_MARGIN,
            SELECTION_MARGIN,
            SELECTION_MARGIN,
        )
    }

    /// Iterate over local pin coordinate points with zero heap allocations when package is present.
    pub fn for_each_pin_point<F: FnMut(f64, f64)>(&self, mut f: F) {
        if let Some(package) = self.kind.package() {
            for p in &package.pins {
                f(p.xpos as f64, p.ypos as f64);
            }
        } else {
            for pin in self.pins() {
                f(pin.local.x, pin.local.y);
            }
        }
    }

    /// Local bounding rectangle encompassing the painted area and pin contacts.
    pub fn local_total_bounding_rect(&self) -> Rect {
        let body = self.body_rect().united(self.kind.visual_rect());
        let mut min_x = body.left();
        let mut max_x = body.right();
        let mut min_y = body.top();
        let mut max_y = body.bottom();
        self.for_each_pin_point(|px, py| {
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_y = min_y.min(py);
            max_y = max_y.max(py);
        });
        Rect::new(
            min_x,
            min_y,
            (max_x - min_x).max(1.0),
            (max_y - min_y).max(1.0),
        )
    }

    pub fn pins(&self) -> Vec<Pin> {
        if let Some(package) = self.kind.package() {
            return package
                .pins
                .iter()
                .map(|p| Pin {
                    id: format!("{}-{}", self.id, p.id),
                    item_id: self.id.clone(),
                    local: Point::new(p.xpos as f64, p.ypos as f64),
                    angle: p.angle,
                    length: p.length as f64,
                    is_bus: p.is_bus(),
                    label: p.label.clone(),
                    unused: p.unused(),
                    direction: self.default_pin_direction(&p.id),
                })
                .collect();
        }
        let is_tunnel_bus = match &self.kind {
            Part::Tunnel(t) => t.is_bus,
            _ => false,
        };
        self.kind
            .pin_geoms()
            .into_iter()
            .map(|cp| {
                let id = if cp.suffix.starts_with('-') {
                    format!("{}{}", self.id, cp.suffix)
                } else {
                    format!("{}-{}", self.id, cp.suffix)
                };
                let is_bus = is_tunnel_bus || cp.suffix == "-ePin0" || cp.suffix == "-busPinI";
                Pin {
                    id,
                    item_id: self.id.clone(),
                    local: cp.local,
                    angle: cp.angle,
                    length: cp.length,
                    is_bus,
                    label: cp.label,
                    unused: false,
                    direction: cp.direction,
                }
            })
            .collect()
    }

    pub fn default_pin_direction(&self, pin_id_local: &str) -> Option<PinDirection> {
        let pkg_pin = self
            .kind
            .package()
            .and_then(|pkg| pkg.find_pin(pin_id_local));
        if let Some(pp) = pkg_pin {
            if pp.unused()
                || pp.pin_type.eq_ignore_ascii_case("nc")
                || pp.pin_type.eq_ignore_ascii_case("unused")
                || pp.pin_type.eq_ignore_ascii_case("rst")
            {
                return None;
            }
            let t = pp.pin_type.to_ascii_lowercase();
            if t == "in" || t == "input" {
                return Some(PinDirection::In);
            }
            if t == "out" || t == "output" {
                return Some(PinDirection::Out);
            }
            if t == "openco" || t == "oc" {
                return Some(PinDirection::OpenCo);
            }
            let lbl = pp.label.to_ascii_uppercase();
            if lbl.starts_with("IN")
                || lbl.starts_with("CLK")
                || lbl.starts_with("RST")
                || lbl.starts_with("RX")
                || lbl == "D"
                || lbl == "J"
                || lbl == "K"
                || lbl == "S"
                || lbl == "R"
                || lbl == "T"
                || lbl == "OE"
                || lbl == "EN"
            {
                return Some(PinDirection::In);
            }
            if lbl.starts_with("OUT")
                || lbl.starts_with("TX")
                || lbl == "Q"
                || lbl == "!Q"
                || lbl == "~Q"
            {
                return Some(PinDirection::Out);
            }
        }
        match &self.kind {
            Part::Mcu(mcu) => {
                if let Some(g) = crate::mcu::match_gpio(&mcu.mcu, &self.id, pin_id_local) {
                    if g.is_out {
                        Some(PinDirection::Out)
                    } else {
                        Some(PinDirection::In)
                    }
                } else {
                    None
                }
            }
            Part::QemuDevice(qemu) => {
                let full = format!("{}-{}", self.id, pin_id_local);
                if let Some(p) = qemu
                    .qemu
                    .pins
                    .iter()
                    .find(|p| p.id == full || p.id.ends_with(pin_id_local))
                {
                    match p.mode {
                        crate::digital::PinMode::Output | crate::digital::PinMode::Source => {
                            Some(PinDirection::Out)
                        }
                        crate::digital::PinMode::OpenCo => Some(PinDirection::OpenCo),
                        crate::digital::PinMode::Input => Some(PinDirection::In),
                        crate::digital::PinMode::Undef => None,
                    }
                } else {
                    None
                }
            }
            Part::Subcircuit(_) | Part::SubPackage(_) => None,
            _ => {
                let norm_local = pin_id_local.trim_start_matches('-');
                if let Some(pin) = self.pins().iter().find(|p| {
                    let p_local =
                        p.id.strip_prefix(&self.id)
                            .unwrap_or(&p.id)
                            .trim_start_matches('-');
                    p_local.eq_ignore_ascii_case(norm_local)
                }) {
                    if pin.direction.is_some() {
                        return pin.direction;
                    }
                }
                None
            }
        }
    }

    pub fn default_pin_pullup(&self, pin_id_local: &str) -> bool {
        match &self.kind {
            Part::Mcu(mcu) => {
                if let Some(g) = crate::mcu::match_gpio(&mcu.mcu, &self.id, pin_id_local) {
                    g.pullup
                } else {
                    false
                }
            }
            Part::QemuDevice(qemu) => {
                let full = format!("{}-{}", self.id, pin_id_local);
                if let Some(p) = qemu
                    .qemu
                    .pins
                    .iter()
                    .find(|p| p.id == full || p.id.ends_with(pin_id_local))
                {
                    p.has_pullup()
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn hit_rect(&self) -> Rect {
        map_rect(
            self.position(),
            self.local_hit_rect(),
            self.rotation,
            self.hflip,
            self.vflip,
        )
    }

    pub fn selection_rect(&self) -> Rect {
        map_rect(
            self.position(),
            self.local_selection_rect(),
            self.rotation,
            self.hflip,
            self.vflip,
        )
    }

    /// Scene-space AABB of the painted area, pins, glow, and rotating bits.
    pub fn total_bounding_rect(&self) -> Rect {
        map_rect(
            self.position(),
            self.local_total_bounding_rect(),
            self.rotation,
            self.hflip,
            self.vflip,
        )
    }

    pub fn full_rect(&self) -> Rect {
        self.hit_rect()
    }

    pub fn contains(&self, scene: Point) -> bool {
        self.local_hit_rect().contains_point(self.map_scene(scene))
    }

    pub fn is_visible(&self) -> bool {
        !matches!(&self.kind, Part::Tunnel(p) if !p.show)
    }

    pub fn tunnel_visible(&self) -> bool {
        match &self.kind {
            Part::Tunnel(p) => p.show,
            _ => true,
        }
    }

    pub fn probe_pause(&self) -> bool {
        match &self.kind {
            Part::Probe(p) => p.pause_at_change,
            _ => false,
        }
    }

    /// Check if a scene point hits this item's ID label (returns `Some(false)`)
    /// or value label (returns `Some(true)`).
    pub fn hit_label(&self, scene: Point) -> Option<bool> {
        if matches!(
            &self.kind,
            Part::Subcircuit(_) | Part::Mcu(_) | Part::SubPackage(_) | Part::Node(_)
        ) {
            return None;
        }

        let local_pt = self.map_scene(scene);

        // 1. Check ID Label
        if self.show_id && !self.label.is_empty() {
            let (lx, ly) = self.label_pos();
            let tw = text_width(&self.label, 9.0, false).max(12.0);
            let approx_h = 14.0;
            let dx = local_pt.x - lx;
            let dy = local_pt.y - ly;
            let rad = (-self.label_rot as f64).to_radians();
            let (s, c) = (rad.sin(), rad.cos());
            let rx = dx * c - dy * s;
            let ry = dx * s + dy * c;
            if rx >= -4.0 && rx <= tw + 4.0 && ry >= -4.0 && ry <= approx_h + 4.0 {
                return Some(false);
            }
        }

        // 2. Check Value Label
        if self.show_val {
            let val_text = self.val_label_text();
            if !val_text.is_empty() {
                let (vx, vy) = self.val_pos();
                let tw = text_width(&val_text, 9.0, false).max(12.0);
                let approx_h = 14.0;
                let dx = local_pt.x - vx;
                let dy = local_pt.y - vy;
                let rad = (-self.val_rot as f64).to_radians();
                let (s, c) = (rad.sin(), rad.cos());
                let rx = dx * c - dy * s;
                let ry = dx * s + dy * c;
                if rx >= -4.0 && rx <= tw + 4.0 && ry >= -4.0 && ry <= approx_h + 4.0 {
                    return Some(true);
                }
            }
        }

        None
    }
}
