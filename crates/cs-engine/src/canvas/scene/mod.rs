//! Circuit items, wires, hit-test, and selection.

pub mod item;
pub mod item_props;
pub mod mcu;
pub mod selection;
pub mod sim1;
#[cfg(test)]
mod tests;
pub mod wires;

pub use crate::components::*;
pub use item::{CanvasOverflowInfo, Item, ProgrammableDevice, SELECTION_MARGIN};
pub use item_props::{PropGroup, PropRow, prop_group};
pub use wires::bump_counter;

use crate::canvas::geom::Point;
use crate::canvas::wire::Wire;

#[derive(Clone, Debug)]
pub struct Scene {
    pub(crate) items: Vec<Item>,
    pub(crate) wires: Vec<Wire>,
    /// Undrawn netlist parts (later gates). MCU chips live in `items`.
    pub(crate) hidden: Vec<crate::elements::Comp>,
    pub(crate) next_resistor: u32,
    pub(crate) next_battery: u32,
    pub(crate) next_ground: u32,
    pub(crate) next_fixed: u32,
    pub(crate) next_node: u32,
    pub(crate) next_capacitor: u32,
    pub(crate) next_el_capacitor: u32,
    pub(crate) next_inductor: u32,
    pub(crate) next_switch: u32,
    pub(crate) next_diode: u32,
    pub(crate) next_zener: u32,
    pub(crate) next_led: u32,
    pub(crate) next_bjt: u32,
    pub(crate) next_mosfet: u32,
    pub(crate) next_opamp: u32,
    pub(crate) next_jfet: u32,
    pub(crate) next_comparator: u32,
    pub(crate) next_voltreg: u32,
    pub(crate) next_probe: u32,
    pub(crate) next_voltmeter: u32,
    pub(crate) next_ammeter: u32,
    pub(crate) next_freqmeter: u32,
    pub(crate) next_oscope: u32,
    pub(crate) next_lanalizer: u32,
    pub(crate) next_subc: u32,
    pub(crate) next_mcu: u32,
    pub(crate) next_wire: u32,
    pub(crate) settings: crate::CircSettings,
    pub inverted_pins: std::collections::HashSet<String>,
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
            next_resistor: 1,
            next_battery: 1,
            next_ground: 1,
            next_fixed: 1,
            next_node: 1,
            next_capacitor: 1,
            next_el_capacitor: 1,
            next_inductor: 1,
            next_switch: 1,
            next_diode: 1,
            next_zener: 1,
            next_led: 1,
            next_bjt: 1,
            next_mosfet: 1,
            next_opamp: 1,
            next_jfet: 1,
            next_comparator: 1,
            next_voltreg: 1,
            next_probe: 1,
            next_voltmeter: 1,
            next_ammeter: 1,
            next_freqmeter: 1,
            next_oscope: 1,
            next_lanalizer: 1,
            next_subc: 1,
            next_mcu: 1,
            next_wire: 1,
            settings: crate::CircSettings::default(),
            inverted_pins: std::collections::HashSet::new(),
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

    /// Place an item with a saved CircId, bumping the matching counter.
    pub fn add_saved_item(&mut self, item: Item) {
        bump_counter(&mut self.next_resistor, &item.id, "Resistor-");
        bump_counter(&mut self.next_battery, &item.id, "Battery-");
        bump_counter(&mut self.next_ground, &item.id, "Ground-");
        bump_counter(&mut self.next_fixed, &item.id, "Fixed Voltage-");
        bump_counter(&mut self.next_node, &item.id, "Node-");
        bump_counter(&mut self.next_capacitor, &item.id, "Capacitor-");
        bump_counter(&mut self.next_el_capacitor, &item.id, "ElCapacitor-");
        bump_counter(&mut self.next_el_capacitor, &item.id, "elCapacitor-");
        bump_counter(&mut self.next_inductor, &item.id, "Inductor-");
        bump_counter(&mut self.next_switch, &item.id, "Switch-");
        bump_counter(&mut self.next_diode, &item.id, "Diode-");
        bump_counter(&mut self.next_zener, &item.id, "Zener-");
        bump_counter(&mut self.next_led, &item.id, "Led-");
        bump_counter(&mut self.next_bjt, &item.id, "BJT-");
        bump_counter(&mut self.next_mosfet, &item.id, "Mosfet-");
        bump_counter(&mut self.next_opamp, &item.id, "opAmp-");
        bump_counter(&mut self.next_jfet, &item.id, "Jfet-");
        bump_counter(&mut self.next_comparator, &item.id, "Comparator-");
        bump_counter(&mut self.next_voltreg, &item.id, "VoltReg-");
        bump_counter(&mut self.next_probe, &item.id, "Probe-");
        bump_counter(&mut self.next_voltmeter, &item.id, "Voltimeter-");
        bump_counter(&mut self.next_ammeter, &item.id, "Amperimeter-");
        bump_counter(&mut self.next_freqmeter, &item.id, "FreqMeter-");
        bump_counter(&mut self.next_oscope, &item.id, "Oscope-");
        bump_counter(&mut self.next_lanalizer, &item.id, "LAnalizer-");
        if matches!(&item.kind, Part::Subcircuit(_)) {
            if let Some(n) = item
                .id
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
            {
                if n >= self.next_subc {
                    self.next_subc = n + 1;
                }
            }
        }
        if matches!(&item.kind, Part::Mcu(_)) {
            if let Some(n) = item
                .id
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
            {
                if n >= self.next_mcu {
                    self.next_mcu = n + 1;
                }
            }
        }
        self.items.push(item);
    }
}
