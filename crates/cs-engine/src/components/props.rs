//! Typed property table: dialog, undo, file, and tests share one list.

use super::ComponentChange;
use crate::units::{format_si, parse_si};

#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    Float(f64),
    Bool(bool),
    Int(i64),
    Enum(String),
    String(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind {
    Float { min: f64, max: f64 },
    Bool,
    Int { min: i64, max: i64 },
    Enum { options: &'static [&'static str] },
    String,
}

/// One row in a type's property table. Ids are PascalCase English (`PChannel`,
/// not `P_Channel`).
pub struct PropDef<T> {
    pub id: &'static str,
    pub caption: &'static str,
    pub unit: &'static str,
    pub info: &'static str,
    pub kind: PropKind,
    pub show_by_default: bool,
    pub persist: bool,
    pub required: bool,
    pub structural: bool,
    pub get: fn(&T) -> PropValue,
    pub set: fn(&mut T, PropValue) -> Result<(), PropError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropError {
    Unknown(String),
    Invalid { id: String, value: String },
}

impl std::fmt::Display for PropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "unknown property '{id}'"),
            Self::Invalid { id, value } => write!(f, "invalid value for '{id}': {value}"),
        }
    }
}

impl std::error::Error for PropError {}

pub(crate) fn expect_float(id: &str, v: PropValue) -> Result<f64, PropError> {
    match v {
        PropValue::Float(x) => Ok(x),
        other => Err(PropError::Invalid {
            id: id.to_string(),
            value: format!("{other:?}"),
        }),
    }
}

pub(crate) fn expect_bool(id: &str, v: PropValue) -> Result<bool, PropError> {
    match v {
        PropValue::Bool(x) => Ok(x),
        other => Err(PropError::Invalid {
            id: id.to_string(),
            value: format!("{other:?}"),
        }),
    }
}

pub(crate) fn expect_string(id: &str, v: PropValue) -> Result<String, PropError> {
    match v {
        PropValue::String(x) | PropValue::Enum(x) => Ok(x),
        other => Err(PropError::Invalid {
            id: id.to_string(),
            value: format!("{other:?}"),
        }),
    }
}

pub(crate) fn expect_int(id: &str, v: PropValue) -> Result<i64, PropError> {
    match v {
        PropValue::Int(x) => Ok(x),
        other => Err(PropError::Invalid {
            id: id.to_string(),
            value: format!("{other:?}"),
        }),
    }
}

impl<T> PropDef<T> {
    pub const fn float(
        id: &'static str,
        caption: &'static str,
        unit: &'static str,
        min: f64,
        max: f64,
        get: fn(&T) -> PropValue,
        set: fn(&mut T, PropValue) -> Result<(), PropError>,
    ) -> Self {
        Self {
            id,
            caption,
            unit,
            info: "",
            kind: PropKind::Float { min, max },
            show_by_default: true,
            persist: true,
            required: false,
            structural: false,
            get,
            set,
        }
    }

    pub const fn bool(
        id: &'static str,
        caption: &'static str,
        get: fn(&T) -> PropValue,
        set: fn(&mut T, PropValue) -> Result<(), PropError>,
    ) -> Self {
        Self {
            id,
            caption,
            unit: "",
            info: "",
            kind: PropKind::Bool,
            show_by_default: true,
            persist: true,
            required: false,
            structural: false,
            get,
            set,
        }
    }

    pub const fn int(
        id: &'static str,
        caption: &'static str,
        min: i64,
        max: i64,
        get: fn(&T) -> PropValue,
        set: fn(&mut T, PropValue) -> Result<(), PropError>,
    ) -> Self {
        Self {
            id,
            caption,
            unit: "",
            info: "",
            kind: PropKind::Int { min, max },
            show_by_default: true,
            persist: true,
            required: false,
            structural: false,
            get,
            set,
        }
    }

    pub const fn enumeration(
        id: &'static str,
        caption: &'static str,
        options: &'static [&'static str],
        get: fn(&T) -> PropValue,
        set: fn(&mut T, PropValue) -> Result<(), PropError>,
    ) -> Self {
        Self {
            id,
            caption,
            unit: "",
            info: "",
            kind: PropKind::Enum { options },
            show_by_default: true,
            persist: true,
            required: false,
            structural: false,
            get,
            set,
        }
    }

    pub const fn string(
        id: &'static str,
        caption: &'static str,
        get: fn(&T) -> PropValue,
        set: fn(&mut T, PropValue) -> Result<(), PropError>,
    ) -> Self {
        Self {
            id,
            caption,
            unit: "",
            info: "",
            kind: PropKind::String,
            show_by_default: true,
            persist: true,
            required: false,
            structural: false,
            get,
            set,
        }
    }

    pub const fn with_info(mut self, info: &'static str) -> Self {
        self.info = info;
        self
    }

    pub const fn with_unit(mut self, unit: &'static str) -> Self {
        self.unit = unit;
        self
    }

    pub fn parse_text(&self, text: &str) -> Result<PropValue, PropError> {
        let invalid = || PropError::Invalid {
            id: self.id.to_string(),
            value: text.to_string(),
        };
        match self.kind {
            PropKind::Float { min, max } => {
                let v = parse_si(text, self.unit).clamp(min, max);
                Ok(PropValue::Float(v))
            }
            PropKind::Bool => match text.trim() {
                "true" | "1" => Ok(PropValue::Bool(true)),
                "false" | "0" => Ok(PropValue::Bool(false)),
                _ => Err(invalid()),
            },
            PropKind::Int { min, max } => {
                let trimmed = text.trim();
                let v: i64 = if let Ok(n) = trimmed.parse() {
                    n
                } else {
                    let num_str: String = trimmed
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
                        .collect();
                    num_str.parse().map_err(|_| invalid())?
                };
                Ok(PropValue::Int(v.clamp(min, max)))
            }
            PropKind::Enum { options } => {
                let t = text.trim();
                if let Some(matched) = options.iter().find(|o| o.eq_ignore_ascii_case(t)) {
                    Ok(PropValue::Enum((*matched).to_string()))
                } else {
                    Err(invalid())
                }
            }
            PropKind::String => Ok(PropValue::String(text.to_string())),
        }
    }

    pub fn format(&self, value: &PropValue) -> String {
        match (self.kind, value) {
            (PropKind::Float { .. }, PropValue::Float(v)) => {
                if self.unit.is_empty() {
                    format!("{v}")
                } else {
                    let base_unit = match self.unit {
                        "ns" | "µs" | "ms" | "ps" => "s",
                        "mV" | "kV" => "V",
                        "mA" | "µA" => "A",
                        "mΩ" | "kΩ" | "MΩ" => "Ω",
                        other => other,
                    };
                    format_si(*v, base_unit)
                }
            }
            (PropKind::Bool, PropValue::Bool(v)) => {
                if *v {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            (PropKind::Int { .. }, PropValue::Int(v)) => v.to_string(),
            (PropKind::Enum { .. }, PropValue::Enum(v))
            | (PropKind::Enum { .. }, PropValue::String(v)) => v.clone(),
            (PropKind::String, PropValue::String(v)) | (PropKind::String, PropValue::Enum(v)) => {
                v.clone()
            }
            _ => String::new(),
        }
    }

    pub fn change(&self) -> ComponentChange {
        if self.structural {
            ComponentChange::structural("")
        } else if self.persist {
            ComponentChange::document("")
        } else {
            ComponentChange::live("")
        }
    }
}

#[cfg(test)]
pub(crate) struct Harness {
    pub resistance: f64,
    pub enabled: bool,
    pub mode: String,
    pub note: String,
}

#[cfg(test)]
impl Harness {
    pub const DEFAULT_R: f64 = 100.0;

    fn get_resistance(&self) -> PropValue {
        PropValue::Float(self.resistance)
    }
    fn set_resistance(&mut self, v: PropValue) -> Result<(), PropError> {
        match v {
            PropValue::Float(x) => {
                self.resistance = x;
                Ok(())
            }
            _ => Err(PropError::Invalid {
                id: "Resistance".into(),
                value: format!("{v:?}"),
            }),
        }
    }
    fn get_enabled(&self) -> PropValue {
        PropValue::Bool(self.enabled)
    }
    fn set_enabled(&mut self, v: PropValue) -> Result<(), PropError> {
        match v {
            PropValue::Bool(x) => {
                self.enabled = x;
                Ok(())
            }
            _ => Err(PropError::Invalid {
                id: "Enabled".into(),
                value: format!("{v:?}"),
            }),
        }
    }
    fn get_mode(&self) -> PropValue {
        PropValue::Enum(self.mode.clone())
    }
    fn set_mode(&mut self, v: PropValue) -> Result<(), PropError> {
        match v {
            PropValue::Enum(x) => {
                self.mode = x;
                Ok(())
            }
            _ => Err(PropError::Invalid {
                id: "Mode".into(),
                value: format!("{v:?}"),
            }),
        }
    }
    fn get_note(&self) -> PropValue {
        PropValue::String(self.note.clone())
    }
    fn set_note(&mut self, v: PropValue) -> Result<(), PropError> {
        match v {
            PropValue::String(x) => {
                self.note = x;
                Ok(())
            }
            _ => Err(PropError::Invalid {
                id: "Note".into(),
                value: format!("{v:?}"),
            }),
        }
    }
}

#[cfg(test)]
impl Default for Harness {
    fn default() -> Self {
        Self {
            resistance: Self::DEFAULT_R,
            enabled: true,
            mode: "A".into(),
            note: String::new(),
        }
    }
}

#[cfg(test)]
impl super::Component for Harness {
    fn type_id(&self) -> &'static str {
        "Harness"
    }
    fn description(&self) -> &'static str {
        "Test harness part."
    }
    fn props() -> &'static [PropDef<Self>] {
        const NOTE: PropDef<Harness> = {
            let mut p = PropDef::string("Note", "Note", Harness::get_note, Harness::set_note);
            p.persist = false;
            p.show_by_default = false;
            p
        };
        const RES: PropDef<Harness> = {
            let mut p = PropDef::float(
                "Resistance",
                "Resistance",
                "Ω",
                1e-12,
                1e12,
                Harness::get_resistance,
                Harness::set_resistance,
            );
            p.required = true;
            p
        };
        static PROPS: &[PropDef<Harness>] = &[
            RES,
            PropDef::bool(
                "Enabled",
                "Enabled",
                Harness::get_enabled,
                Harness::set_enabled,
            ),
            PropDef::enumeration(
                "Mode",
                "Mode",
                &["A", "B"],
                Harness::get_mode,
                Harness::set_mode,
            ),
            NOTE,
        ];
        PROPS
    }
    fn pin_geoms(&self) -> Vec<super::CompPin> {
        vec![
            super::CompPin::new("-lPin", -16.0, 0.0, 180, 5.0),
            super::CompPin::new("-rPin", 16.0, 0.0, 0, 5.0),
        ]
    }
    fn body(&self) -> crate::canvas::Rect {
        crate::canvas::Rect::new(-11.0, -4.5, 22.0, 9.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Component;
    use crate::units::format_si;

    #[test]
    fn default_matches_constructor() {
        let h = Harness::default();
        assert_eq!(
            h.get_prop_text("Resistance").unwrap(),
            format_si(Harness::DEFAULT_R, "Ω")
        );
        assert_eq!(h.get_prop_text("Enabled").unwrap(), "true");
        assert_eq!(h.get_prop_text("Mode").unwrap(), "A");
    }

    #[test]
    fn set_second_and_third_values() {
        let mut h = Harness::default();
        h.set_prop_text("Resistance", "4.7 kΩ").unwrap();
        assert_eq!(h.get_prop_text("Resistance").unwrap(), "4.7 kΩ");
        h.set_prop_text("Resistance", "10 Ω").unwrap();
        assert_eq!(h.get_prop_text("Resistance").unwrap(), "10 Ω");

        h.set_prop_text("Enabled", "false").unwrap();
        assert_eq!(h.get_prop_text("Enabled").unwrap(), "false");
        h.set_prop_text("Enabled", "true").unwrap();
        assert_eq!(h.get_prop("Enabled"), Some(PropValue::Bool(true)));

        h.set_prop_text("Mode", "B").unwrap();
        assert_eq!(h.get_prop_text("Mode").unwrap(), "B");
        assert!(h.set_prop_text("Mode", "Z").is_err());
    }

    #[test]
    fn unknown_prop_is_error() {
        let mut h = Harness::default();
        match h.set_prop_text("Nope", "1") {
            Err(PropError::Unknown(id)) => assert_eq!(id, "Nope"),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn persist_false_emits_live_change() {
        let mut h = Harness::default();
        let c = h.set_prop_text("Note", "scratch").unwrap();
        assert!(!c.saved);
        assert!(!c.undo);
        assert_eq!(h.get_prop_text("Note").unwrap(), "scratch");
    }

    #[test]
    fn persist_true_emits_document_change() {
        let mut h = Harness::default();
        let c = h.set_prop_text("Resistance", "1 kΩ").unwrap();
        assert!(c.saved && c.undo && c.sim);
    }
}
