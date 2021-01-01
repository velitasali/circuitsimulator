//! Pin geometry primitives and hit-testing constants.

use super::geom::Point;

/// Scene-space radius for pin hit-test (ring plus a bit of slop).
pub const PIN_HIT_RADIUS: f64 = 6.0;

/// Pin logical direction for chevrons and port orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PinDirection {
    In,
    Out,
    OpenCo,
}

impl PinDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
            Self::OpenCo => "openco",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "in" | "input" => Some(Self::In),
            "out" | "output" => Some(Self::Out),
            "openco" | "oc" | "open_collector" | "open-collector" => Some(Self::OpenCo),
            _ => None,
        }
    }

    pub const fn is_in(self) -> bool {
        matches!(self, Self::In)
    }

    pub const fn is_out(self) -> bool {
        matches!(self, Self::Out)
    }

    pub const fn is_openco(self) -> bool {
        matches!(self, Self::OpenCo)
    }
}

impl std::fmt::Display for PinDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PinGeom {
    pub suffix: &'static str,
    pub x: f64,
    pub y: f64,
    /// C++ `Pin` constructor angle (0 / 90 / 180 / 270).
    pub angle: i32,
    pub length: f64,
    pub direction: Option<PinDirection>,
}

#[derive(Clone, Debug)]
pub struct Pin {
    pub id: String,
    pub item_id: String,
    pub local: Point,
    pub angle: i32,
    pub length: f64,
    pub is_bus: bool,
    pub label: String,
    pub unused: bool,
    pub direction: Option<PinDirection>,
}

impl Pin {
    pub fn scene_pos(&self, item: Point) -> Point {
        Point::new(item.x + self.local.x, item.y + self.local.y)
    }
}
