//! `Component` and shared behaviour traits.

use super::ComponentChange;
use super::props::{PropDef, PropError, PropValue};
use crate::canvas::{Point, Rect};
use crate::matrix::CircMatrix;

pub use crate::canvas::PinDirection;

/// Pin in component-local coordinates. Scene fills `item_id` when placing.
#[derive(Clone, Debug, PartialEq)]
pub struct CompPin {
    pub suffix: String,
    pub local: Point,
    pub angle: i32,
    pub length: f64,
    pub direction: Option<PinDirection>,
    pub label: String,
}

impl CompPin {
    pub fn new(suffix: impl Into<String>, x: f64, y: f64, angle: i32, length: f64) -> Self {
        Self {
            suffix: suffix.into(),
            local: Point::new(x, y),
            angle,
            length,
            direction: None,
            label: String::new(),
        }
    }

    pub fn with_direction(mut self, dir: PinDirection) -> Self {
        self.direction = Some(dir);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn in_pin(suffix: impl Into<String>, x: f64, y: f64, angle: i32, length: f64) -> Self {
        Self::new(suffix, x, y, angle, length).with_direction(PinDirection::In)
    }

    pub fn out_pin(suffix: impl Into<String>, x: f64, y: f64, angle: i32, length: f64) -> Self {
        Self::new(suffix, x, y, angle, length).with_direction(PinDirection::Out)
    }

    pub fn openco_pin(suffix: impl Into<String>, x: f64, y: f64, angle: i32, length: f64) -> Self {
        Self::new(suffix, x, y, angle, length).with_direction(PinDirection::OpenCo)
    }
}

impl From<crate::canvas::Pin> for CompPin {
    fn from(p: crate::canvas::Pin) -> Self {
        Self {
            suffix: p.id,
            local: p.local,
            angle: p.angle,
            length: p.length,
            direction: p.direction,
            label: p.label,
        }
    }
}

impl From<&crate::canvas::PinGeom> for CompPin {
    fn from(g: &crate::canvas::PinGeom) -> Self {
        Self {
            suffix: g.suffix.to_string(),
            local: Point::new(g.x, g.y),
            angle: g.angle,
            length: g.length,
            direction: g.direction,
            label: String::new(),
        }
    }
}
pub fn find_prop_def<T: Component>(id: &str) -> Option<&'static PropDef<T>> {
    let clean = id.replace('_', "");
    T::props()
        .iter()
        .find(|p| p.id == id)
        .or_else(|| T::props().iter().find(|p| p.id.eq_ignore_ascii_case(id)))
        .or_else(|| {
            T::props()
                .iter()
                .find(|p| p.id.replace('_', "").eq_ignore_ascii_case(&clean))
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropRow {
    pub name: &'static str,
    pub kind: &'static str,
    pub caption: &'static str,
    pub info: &'static str,
    pub unit: &'static str,
    pub options: Vec<String>,
    pub visible: bool,
    pub enabled: bool,
}

impl PropRow {
    pub fn new(
        name: &'static str,
        kind: &'static str,
        caption: &'static str,
        info: &'static str,
        unit: &'static str,
        options: &[&str],
    ) -> Self {
        Self {
            name,
            kind,
            caption,
            info,
            unit,
            options: options.iter().map(|s| s.to_string()).collect(),
            visible: true,
            enabled: true,
        }
    }

    pub fn visible(mut self, v: bool) -> Self {
        self.visible = v;
        self
    }

    pub fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropGroup {
    pub name: &'static str,
    pub rows: Vec<PropRow>,
}

impl PropGroup {
    pub fn new(name: &'static str, rows: Vec<PropRow>) -> Self {
        Self { name, rows }
    }
}

pub fn prop_group(name: &'static str, rows: Vec<PropRow>) -> PropGroup {
    PropGroup { name, rows }
}

/// Convenience helper to split a component's [`PropRow`]s into named [`PropGroup`] tabs.
///
/// Properties are placed into the tab that first lists their name. Any remaining
/// unlisted rows are appended to the first group so properties are never lost.
pub fn group_rows_by(
    rows: Vec<PropRow>,
    groups: &[(&'static str, &[&'static str])],
) -> Vec<PropGroup> {
    let mut used = std::collections::HashSet::new();
    let mut result: Vec<PropGroup> = groups
        .iter()
        .map(|&(group_name, prop_ids)| {
            let mut group_rows = Vec::new();
            for &id in prop_ids {
                if let Some(r) = rows.iter().find(|r| r.name == id) {
                    group_rows.push(r.clone());
                    used.insert(id);
                }
            }
            PropGroup::new(group_name, group_rows)
        })
        .collect();

    let remainder: Vec<PropRow> = rows
        .into_iter()
        .filter(|r| !used.contains(r.name))
        .collect();
    if !remainder.is_empty() {
        if let Some(first) = result.first_mut() {
            first.rows.extend(remainder);
        } else {
            result.push(PropGroup::new("Main", remainder));
        }
    }

    result
}

/// One library type: identity, property table, pins, body.
///
/// Get/set look up [`PropDef`], not a crate-wide match. `Self` is `Sized` on
/// purpose — stamp dispatch goes through [`crate::components::Part`], not a
/// vtable.
pub trait Component: Sized + 'static {
    fn type_id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn props() -> &'static [PropDef<Self>];
    fn pin_geoms(&self) -> Vec<CompPin>;
    fn body(&self) -> Rect;

    /// Local AABB of everything `paint` draws, including glow and rotating
    /// bits that sit outside [`body`]. Hit-testing still uses `body`.
    fn visual_rect(&self) -> Rect {
        self.body()
    }

    fn get_prop(&self, id: &str) -> Option<PropValue> {
        Self::props()
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.get)(self))
    }

    fn get_prop_text(&self, id: &str) -> Option<String> {
        let def = Self::props().iter().find(|p| p.id == id)?;
        Some(def.format(&(def.get)(self)))
    }

    fn set_prop(&mut self, id: &str, value: PropValue) -> Result<ComponentChange, PropError> {
        let def = Self::props()
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| PropError::Unknown(id.to_string()))?;
        (def.set)(self, value)?;
        Ok(def.change())
    }

    fn set_prop_text(&mut self, id: &str, text: &str) -> Result<ComponentChange, PropError> {
        let def = Self::props()
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| PropError::Unknown(id.to_string()))?;
        let value = def.parse_text(text)?;
        (def.set)(self, value)?;
        Ok(def.change())
    }

    fn get_prop_text_alias(&self, id: &str) -> Option<String> {
        let def = find_prop_def::<Self>(id)?;
        Some(def.format(&(def.get)(self)))
    }

    fn set_prop_text_alias(&mut self, id: &str, text: &str) -> Result<ComponentChange, PropError> {
        let def = find_prop_def::<Self>(id).ok_or_else(|| PropError::Unknown(id.to_string()))?;
        let value = def.parse_text(text)?;
        (def.set)(self, value)?;
        Ok(def.change())
    }

    fn prop_rows(&self) -> Vec<PropRow> {
        Self::props()
            .iter()
            .filter(|d| d.show_by_default)
            .map(|d| PropRow {
                name: d.id,
                kind: match d.kind {
                    super::props::PropKind::Float { .. } => "double",
                    super::props::PropKind::Bool => "bool",
                    super::props::PropKind::Int { .. } => "int",
                    super::props::PropKind::Enum { .. } => "enum",
                    super::props::PropKind::String => "string",
                },
                caption: d.caption,
                info: d.info,
                unit: d.unit,
                options: match d.kind {
                    super::props::PropKind::Enum { options } => {
                        options.iter().map(|s| s.to_string()).collect()
                    }
                    _ => Vec::new(),
                },
                visible: true,
                enabled: true,
            })
            .collect()
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        vec![PropGroup {
            name: "Main",
            rows: self.prop_rows(),
        }]
    }

    fn interact_toggle(&mut self, _local: Point) -> bool {
        false
    }

    fn interact_wheel(&mut self, _local: Point, _delta: f64) -> bool {
        false
    }

    fn interact_press(&mut self, _local: Point) -> bool {
        false
    }

    fn interact_move(&mut self, _local: Point) -> bool {
        false
    }

    fn interact_release(&mut self, _local: Point) -> bool {
        false
    }

    fn interact_cancel(&mut self) -> bool {
        false
    }
}

/// Stamp into the analog matrix. Node indices follow [`Component::pin_geoms`].
/// `dt` is the analog timestep (seconds); resistive stamps ignore it.
pub trait Stampable {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64);
}

/// Mouse / wheel value (variable resistor, capacitor, inductor, pot wiper).
pub trait Dialed {
    fn set_value(&mut self, v: f64) -> ComponentChange;
    fn value(&self) -> f64;
    fn min(&self) -> f64;
    fn max(&self) -> f64;
    fn wiper(&self) -> f64 {
        let span = (self.max() - self.min()).max(1e-15);
        ((self.value() - self.min()) / span).clamp(0.0, 1.0)
    }
}

/// Keyboard shortcut + step for a dialed value.
#[derive(Clone, Debug, PartialEq)]
pub struct DialState {
    pub key: String,
    pub step: f64,
}

impl DialState {
    pub fn new(step: f64) -> Self {
        Self {
            key: String::new(),
            step,
        }
    }
}

const RESISTOR_MIN_OHMS: f64 = 1e-12;

pub(crate) fn pin_node(pin_nodes: &[usize], i: usize, n: usize) -> Option<usize> {
    pin_nodes.get(i).copied().filter(|&p| p < n)
}

/// Two-terminal Norton stamp: conductance `g` between pins 0 and 1, current
/// `i_src` out of pin 0 into pin 1. A missing pin stamps the other to ground.
pub(crate) fn stamp_two_terminal(matrix: &mut CircMatrix, pin_nodes: &[usize], g: f64, i_src: f64) {
    let n = matrix.n();
    let n0 = pin_node(pin_nodes, 0, n);
    let n1 = pin_node(pin_nodes, 1, n);
    match (n0, n1) {
        (Some(a), Some(b)) if a != b => {
            matrix.add_matrix(a, a, g);
            matrix.add_matrix(b, b, g);
            matrix.add_matrix(a, b, -g);
            matrix.add_matrix(b, a, -g);
            if i_src != 0.0 {
                matrix.add_coef(a, i_src);
                matrix.add_coef(b, -i_src);
            }
        }
        (Some(a), None) | (None, Some(a)) | (Some(a), Some(_)) => {
            matrix.add_matrix(a, a, g);
            if i_src != 0.0 {
                matrix.add_coef(a, i_src);
            }
        }
        _ => {}
    }
}

pub(crate) fn stamp_conductance_between(
    matrix: &mut CircMatrix,
    pin_nodes: &[usize],
    i: usize,
    j: usize,
    g: f64,
) {
    let n = matrix.n();
    let n0 = pin_node(pin_nodes, i, n);
    let n1 = pin_node(pin_nodes, j, n);
    match (n0, n1) {
        (Some(a), Some(b)) if a != b => {
            matrix.add_matrix(a, a, g);
            matrix.add_matrix(b, b, g);
            matrix.add_matrix(a, b, -g);
            matrix.add_matrix(b, a, -g);
        }
        (Some(a), None) | (None, Some(a)) | (Some(a), Some(_)) => {
            matrix.add_matrix(a, a, g);
        }
        _ => {}
    }
}

pub(crate) fn stamp_directed(
    matrix: &mut CircMatrix,
    pin_nodes: &[usize],
    from_idx: usize,
    to_idx: usize,
    g: f64,
) {
    let n = matrix.n();
    let from = pin_node(pin_nodes, from_idx, n);
    let to = pin_node(pin_nodes, to_idx, n);
    match (from, to) {
        (Some(a), Some(b)) if a != b => {
            matrix.add_matrix(a, a, g);
            matrix.add_matrix(a, b, -g);
        }
        (Some(a), _) => {
            matrix.add_matrix(a, a, g);
        }
        _ => {}
    }
}

pub(crate) fn two_terminal_pins(length: f64) -> Vec<CompPin> {
    vec![
        CompPin::new("-lPin", -16.0, 0.0, 180, length),
        CompPin::new("-rPin", 16.0, 0.0, 0, length),
    ]
}

pub(crate) fn clamp_positive(v: f64, min: f64) -> f64 {
    v.max(min)
}

pub(crate) fn resistor_g(ohms: f64) -> f64 {
    1.0 / ohms.max(RESISTOR_MIN_OHMS)
}

/// Norton voltage source to implicit ground at `pin_i`.
pub(crate) fn stamp_to_ground(
    matrix: &mut CircMatrix,
    pin_nodes: &[usize],
    pin_i: usize,
    volts: f64,
    g: f64,
) {
    let n = matrix.n();
    if let Some(a) = pin_node(pin_nodes, pin_i, n) {
        matrix.add_matrix(a, a, g);
        let i_src = volts * g;
        if i_src != 0.0 {
            matrix.add_coef(a, i_src);
        }
    }
}

/// Independent current into `pin_i` (from implicit ground).
pub(crate) fn stamp_current(matrix: &mut CircMatrix, pin_nodes: &[usize], pin_i: usize, i: f64) {
    let n = matrix.n();
    if let Some(a) = pin_node(pin_nodes, pin_i, n) {
        if i != 0.0 {
            matrix.add_coef(a, i);
        }
    }
}

/// Two-terminal pin helpers (resistor family).
pub trait TwoTerminal {
    fn left_suffix(&self) -> &'static str {
        "-lPin"
    }
    fn right_suffix(&self) -> &'static str {
        "-rPin"
    }
}
