//! Circuit items, wires, hit-test, and selection.

pub mod circ1;
pub mod item;
pub mod item_props;
pub mod mcu;
pub mod selection;
#[cfg(test)]
mod tests;
pub mod wires;

pub use crate::components::*;
pub use item::{CanvasOverflowInfo, Item, ProgrammableDevice, SELECTION_MARGIN};
pub use item_props::{PropGroup, PropRow, prop_group};
pub use selection::HitTarget;

use std::collections::{HashMap, HashSet};

use crate::CircSettings;
use crate::canvas::geom::Point;
use crate::canvas::wire::Wire;
use crate::elements::Comp;

#[derive(Clone, Debug)]
pub struct Scene {
    pub(crate) items: Vec<Item>,
    pub(crate) wires: Vec<Wire>,
    /// Undrawn netlist parts (later gates). MCU chips live in `items`.
    pub(crate) hidden: Vec<Comp>,
    /// Next numeric suffix per component ID prefix (`{prefix}-{n}`).
    pub(crate) next_ids: HashMap<String, u32>,
    pub(crate) next_wire: u32,
    pub(crate) settings: CircSettings,
    pub inverted_pins: HashSet<String>,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            wires: Vec::new(),
            hidden: Vec::new(),
            next_ids: HashMap::new(),
            next_wire: 1,
            settings: CircSettings::default(),
            inverted_pins: HashSet::new(),
        }
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [Item] {
        &mut self.items
    }

    pub fn wires(&self) -> &[Wire] {
        &self.wires
    }

    pub fn wires_mut(&mut self) -> &mut [Wire] {
        &mut self.wires
    }

    pub fn hidden(&self) -> &[crate::elements::Comp] {
        &self.hidden
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.wires.is_empty()
    }

    pub fn settings(&self) -> &crate::CircSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut crate::CircSettings {
        &mut self.settings
    }

    pub fn item_by_id(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|it| it.id == id)
    }

    pub fn item_by_id_mut(&mut self, id: &str) -> Option<&mut Item> {
        self.items.iter_mut().find(|it| it.id == id)
    }

    /// Single O(N) scan to find the owning `&Item` and the local pin name suffix.
    /// Returns `None` when `pin_id` cannot be matched to any item.
    pub fn find_item_for_pin<'a>(&'a self, pin_id: &'a str) -> Option<(&'a Item, &'a str)> {
        self.items.iter().find_map(|it| {
            if pin_id.starts_with(it.id.as_str())
                && (pin_id.len() == it.id.len() || pin_id[it.id.len()..].starts_with('-'))
            {
                let local = pin_id
                    .strip_prefix(it.id.as_str())
                    .unwrap_or("")
                    .strip_prefix('-')
                    .unwrap_or("");
                Some((it, local))
            } else {
                None
            }
        })
    }

    pub fn is_pin_inverted(&self, pin_id: &str) -> bool {
        let default_inv = if let Some((it, local)) = self.find_item_for_pin(pin_id) {
            it.kind.is_pin_default_inverted(local)
        } else if let Some((item_id, local)) = pin_id.rsplit_once('-') {
            self.item_by_id(item_id)
                .map(|it| it.kind.is_pin_default_inverted(local))
                .unwrap_or(false)
        } else {
            false
        };
        if self.inverted_pins.contains(pin_id) {
            !default_inv
        } else {
            default_inv
        }
    }

    pub fn is_pin_pullup(&self, pin_id: &str) -> bool {
        if let Some((it, local)) = self.find_item_for_pin(pin_id) {
            it.default_pin_pullup(local) || it.kind.is_pin_default_pullup(local)
        } else if let Some((item_id, local)) = pin_id.rsplit_once('-') {
            self.item_by_id(item_id)
                .map(|it| it.default_pin_pullup(local) || it.kind.is_pin_default_pullup(local))
                .unwrap_or(false)
        } else {
            false
        }
    }

    pub fn toggle_pin_inverted(&mut self, pin_id: &str) -> bool {
        if self.inverted_pins.contains(pin_id) {
            self.inverted_pins.remove(pin_id);
        } else {
            self.inverted_pins.insert(pin_id.to_string());
        }
        self.is_pin_inverted(pin_id)
    }

    pub fn set_pin_inverted(&mut self, pin_id: &str, inv: bool) {
        let cur = self.is_pin_inverted(pin_id);
        if cur != inv {
            self.toggle_pin_inverted(pin_id);
        }
    }

    pub fn toggle_switch_at(&mut self, index: usize, pos: Point) -> Option<String> {
        let item = self.items.get_mut(index)?;
        let local = item.map_scene(pos);
        if item.kind.interact_toggle(local) {
            Some(item.id.clone())
        } else {
            None
        }
    }

    pub fn wheel_at(&mut self, pos: Point, delta: f64) -> Option<String> {
        if delta == 0.0 {
            return None;
        }
        for item in self.items.iter_mut().rev() {
            let local = item.map_scene(pos);
            if item.kind.interact_wheel(local, delta) {
                return Some(item.id.clone());
            }
        }
        None
    }

    /// Record an existing item ID so future allocations for its prefix will not collide.
    pub(crate) fn record_id(&mut self, id: &str) {
        if let Some((prefix, rest)) = id.rsplit_once('-') {
            if let Ok(n) = rest.parse::<u32>() {
                let next = self.next_ids.entry(prefix.to_string()).or_insert(1);
                if n >= *next {
                    *next = n.saturating_add(1);
                }
                if prefix == "elCapacitor" {
                    let next = self.next_ids.entry("ElCapacitor".to_string()).or_insert(1);
                    if n >= *next {
                        *next = n.saturating_add(1);
                    }
                }
            }
        }
    }

    /// Return the next unused ID for `prefix` without advancing the counter.
    pub fn peek_unique_id(&self, prefix: &str) -> String {
        let mut n = *self.next_ids.get(prefix).unwrap_or(&1);
        loop {
            let candidate = format!("{prefix}-{n}");
            if !self.items.iter().any(|it| it.id == candidate) {
                return candidate;
            }
            n = n.saturating_add(1);
        }
    }

    /// Allocate a unique component ID of the form `{prefix}-{n}` that does not
    /// collide with any existing item in the scene.
    pub fn alloc_unique_id(&mut self, prefix: &str) -> String {
        let mut n = *self.next_ids.get(prefix).unwrap_or(&1);
        loop {
            let candidate = format!("{prefix}-{n}");
            n = n.saturating_add(1);
            if !self.items.iter().any(|it| it.id == candidate) {
                self.next_ids.insert(prefix.to_string(), n);
                return candidate;
            }
        }
    }

    /// Return the next unused wire ID without advancing the counter.
    pub(crate) fn peek_wire_id(&self) -> String {
        let mut n = self.next_wire;
        loop {
            let candidate = format!("Connector-{n}");
            if !self.wires.iter().any(|w| w.id == candidate) {
                return candidate;
            }
            n = n.saturating_add(1);
        }
    }

    /// Allocate a unique wire ID of the form `Connector-{n}`.
    pub(crate) fn alloc_wire_id(&mut self) -> String {
        loop {
            let candidate = format!("Connector-{}", self.next_wire);
            self.next_wire = self.next_wire.saturating_add(1);
            if !self.wires.iter().any(|w| w.id == candidate) {
                return candidate;
            }
        }
    }

    /// Place an item with a saved CircId, bumping the matching counter.
    pub fn add_saved_item(&mut self, item: Item) {
        self.record_id(&item.id);
        self.items.push(item);
    }
}
