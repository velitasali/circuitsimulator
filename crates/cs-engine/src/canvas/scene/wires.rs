//! Wire management, routing, drafting, splicing, and node cleanup.

use super::Scene;
use crate::canvas::geom::Point;
use crate::canvas::pin::{PIN_HIT_RADIUS, Pin};
use crate::canvas::wire::{Wire, WireHit};

pub(crate) fn pin_belongs(pin_id: &str, item_ids: &[String]) -> bool {
    item_ids
        .iter()
        .any(|id| pin_id.starts_with(id) && pin_id.as_bytes().get(id.len()).copied() == Some(b'-'))
}

pub fn bump_counter(next: &mut u32, id: &str, prefix: &str) {
    if let Some(rest) = id.strip_prefix(prefix) {
        if let Ok(n) = rest.parse::<u32>() {
            if n >= *next {
                *next = n + 1;
            }
        }
    }
}

impl Scene {
    pub fn add_saved_wire(&mut self, wire: Wire) {
        bump_counter(&mut self.next_wire, &wire.id, "Connector-");
        bump_counter(&mut self.next_wire, &wire.id, "connector-");
        self.wires.push(wire);
    }

    /// A 5 V / 2×1 kΩ divider already wired, so power-on has something to solve.
    pub fn add_demo_divider(&mut self) {
        self.add_fixed_volt(-80.0, 0.0, 5.0);
        self.add_resistor(0.0, 0.0, 1000.0);
        self.add_resistor(80.0, 0.0, 1000.0);
        self.add_ground(112.0, 16.0);
        let fv = self.pin_scene("Fixed Voltage-1-outnod").unwrap();
        let r1l = self.pin_scene("Resistor-1-lPin").unwrap();
        let r1r = self.pin_scene("Resistor-1-rPin").unwrap();
        let r2l = self.pin_scene("Resistor-2-lPin").unwrap();
        let r2r = self.pin_scene("Resistor-2-rPin").unwrap();
        let gnd = self.pin_scene("Ground-1-Gnd").unwrap();
        self.connect_pins("Fixed Voltage-1-outnod", fv, "Resistor-1-lPin", r1l);
        self.connect_pins("Resistor-1-rPin", r1r, "Resistor-2-lPin", r2l);
        self.connect_pins("Resistor-2-rPin", r2r, "Ground-1-Gnd", gnd);
    }

    pub fn connect_pins(&mut self, a: &str, ap: Point, b: &str, bp: Point) {
        let id = format!("Connector-{}", self.next_wire);
        self.next_wire += 1;
        let mut w = Wire::start(id, a, ap);
        w.close(b, bp);
        self.wires.push(w);
    }

    pub fn pin_scene(&self, pin_id: &str) -> Option<Point> {
        for it in &self.items {
            for p in it.pins() {
                if p.id == pin_id {
                    return Some(it.pin_scene_pos(&p));
                }
            }
        }
        None
    }

    /// Fast lookup set of all connected pin IDs for O(1) queries during rendering.
    pub fn connected_pins_set(&self) -> rustc_hash::FxHashSet<String> {
        let mut set = rustc_hash::FxHashSet::with_capacity_and_hasher(
            self.wires.len() * 4,
            Default::default(),
        );
        let mut insert_pin = |pin: &str| {
            set.insert(pin.to_string());
            if let Some((comp, suffix)) = pin.split_once('-') {
                match suffix {
                    "lPin0" => {
                        set.insert(format!("{comp}-pinP0"));
                    }
                    "pinP0" => {
                        set.insert(format!("{comp}-lPin0"));
                    }
                    "rPin0" => {
                        set.insert(format!("{comp}-switch0pinN"));
                    }
                    "switch0pinN" => {
                        set.insert(format!("{comp}-rPin0"));
                    }
                    _ => {}
                }
            }
        };
        for w in &self.wires {
            if w.closed() {
                insert_pin(&w.start_pin);
                if let Some(ref ep) = w.end_pin {
                    insert_pin(ep);
                }
            }
        }
        set
    }

    pub fn pin_connected(&self, pin_id: &str) -> bool {
        self.wires.iter().any(|w| {
            w.closed()
                && (w.start_pin == pin_id || w.end_pin.as_deref().map_or(false, |ep| ep == pin_id))
        })
    }

    pub fn drawing(&self) -> bool {
        self.wires.iter().any(Wire::drawing)
    }

    pub fn draft_mut(&mut self) -> Option<&mut Wire> {
        self.wires.iter_mut().rev().find(|w| w.drawing())
    }

    /// Topmost pin first. Connected pins are still returned (caller decides).
    /// A Node returns its first unconnected pin so a T-junction can form.
    pub fn hit_pin(&self, scene: Point) -> Option<Pin> {
        for it in self.items.iter().rev() {
            let origin = it.position();
            if it.is_node() {
                if origin.distance(scene) > PIN_HIT_RADIUS {
                    continue;
                }
                let pins = it.pins();
                if let Some(p) = pins.into_iter().find(|p| !self.pin_connected(&p.id)) {
                    return Some(p);
                }
                continue;
            }
            let b = it.body_rect();
            let max_r = (b.w.abs() + b.h.abs()).max(40.0) + 40.0;
            if (origin.x - scene.x).abs() > max_r || (origin.y - scene.y).abs() > max_r {
                continue;
            }
            let pins = it.pins();
            let mut best: Option<(f64, Pin)> = None;
            for p in pins {
                let d = it.pin_scene_pos(&p).distance(scene);
                if d <= PIN_HIT_RADIUS {
                    if best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) {
                        best = Some((d, p));
                    }
                }
            }
            if let Some((_, p)) = best {
                return Some(p);
            }
        }
        None
    }

    pub fn hit_wire(&self, scene: Point) -> Option<usize> {
        self.hit_wire_detail(scene).map(|(i, _)| i)
    }

    pub fn hit_wire_detail(&self, scene: Point) -> Option<(usize, WireHit)> {
        self.wires
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, w)| w.closed())
            .filter(|(_, w)| w.bounds().contains_point(scene))
            .find_map(|(i, w)| w.hit_detail(scene).map(|h| (i, h)))
    }

    pub fn selected_count(&self) -> usize {
        self.items.iter().filter(|it| it.selected).count()
            + self
                .wires
                .iter()
                .filter(|w| w.selected && w.closed())
                .count()
    }

    /// A pin is a bus pin when it belongs to a Node that already has a bus
    /// wire, or (later) when the part's pin is inherently a bus pin.
    pub fn pin_is_bus(&self, pin_id: &str) -> bool {
        let Some(it) = self
            .items
            .iter()
            .find(|it| pin_belongs(pin_id, std::slice::from_ref(&it.id)))
        else {
            return false;
        };
        if it.is_node() {
            return self.item_has_bus_wire(&it.id);
        }
        false
    }

    fn item_has_bus_wire(&self, item_id: &str) -> bool {
        let ids = [item_id.to_string()];
        self.wires.iter().any(|w| {
            w.is_bus
                && w.closed()
                && (pin_belongs(&w.start_pin, &ids)
                    || w.end_pin.as_ref().is_some_and(|e| pin_belongs(e, &ids)))
        })
    }

    pub fn start_wire(&mut self, pin: &Pin, at: Point) -> bool {
        if pin.unused || self.pin_connected(&pin.id) || self.drawing() {
            return false;
        }
        let id = format!("Connector-{}", self.next_wire);
        self.next_wire += 1;
        let mut w = Wire::start(id, pin.id.clone(), at);
        w.is_bus = self.pin_is_bus(&pin.id);
        self.wires.push(w);
        true
    }

    pub fn close_wire(&mut self, pin: &Pin, at: Point) -> bool {
        if pin.unused || self.pin_connected(&pin.id) {
            return false;
        }
        let is_bus = self.pin_is_bus(&pin.id);
        let Some(draft) = self.draft_mut() else {
            return false;
        };
        if draft.start_pin == pin.id {
            return false;
        }
        if draft.is_bus != is_bus {
            return false;
        }
        draft.close(pin.id.clone(), at);
        true
    }

    pub fn cancel_wire(&mut self) -> bool {
        let before = self.wires.len();
        self.wires.retain(|w| w.closed());
        let cleaned = self.cleanup_nodes();
        self.wires.len() != before || cleaned
    }

    pub fn route_draft(&mut self, cursor: Point, shift: bool) -> bool {
        if let Some(w) = self.draft_mut() {
            w.route_end(cursor, shift);
            return true;
        }
        false
    }

    pub fn inc_draft_corner(&mut self) -> bool {
        if let Some(w) = self.draft_mut() {
            w.inc_act_line();
            return true;
        }
        false
    }

    /// Parity with C++ `Node::checkRemove` and `Node::joinConns`.
    /// When a junction `Node` has fewer than 3 connected wires:
    /// - If 2 wires: merge them into a single continuous wire from start to end,
    ///   remove the two segments and the node.
    /// - If 1 wire: remove the orphaned wire and the node.
    /// - If 0 wires: remove the node.
    /// Runs until a fixed point so that cascading node removals are resolved.
    pub fn cleanup_nodes(&mut self) -> bool {
        let mut changed_overall = false;
        loop {
            let mut changed_this_pass = false;

            let node_ids: Vec<(String, bool)> = self
                .items
                .iter()
                .filter(|it| it.is_node())
                .map(|it| (it.id.clone(), it.selected))
                .collect();

            for (node_id, node_selected) in node_ids {
                if !self.items.iter().any(|it| it.id == node_id) {
                    continue;
                }

                let p0 = format!("{node_id}-0");
                let p1 = format!("{node_id}-1");
                let p2 = format!("{node_id}-2");
                let node_pins = [&p0, &p1, &p2];

                // 1. Remove self-loop wires on this node (both start & end on this node)
                let before_wires = self.wires.len();
                self.wires.retain(|w| {
                    if !w.closed() {
                        return true;
                    }
                    let start_on_node = node_pins.iter().any(|&p| p == &w.start_pin);
                    let end_on_node = w
                        .end_pin
                        .as_ref()
                        .is_some_and(|e| node_pins.iter().any(|&p| p == e));
                    !(start_on_node && end_on_node)
                });
                if self.wires.len() != before_wires {
                    changed_this_pass = true;
                }

                // 2. Find all closed wires connected to this node
                // (wire_idx, node_pin_id, other_pin_id, node_is_start)
                let mut connected: Vec<(usize, String, String, bool)> = Vec::new();
                for (w_idx, w) in self.wires.iter().enumerate() {
                    if !w.closed() {
                        continue;
                    }
                    if let Some(np) = node_pins.iter().find(|&&p| p == &w.start_pin) {
                        let other = w.end_pin.clone().unwrap_or_default();
                        connected.push((w_idx, (*np).clone(), other, true));
                    } else if let Some(end) = &w.end_pin {
                        if let Some(np) = node_pins.iter().find(|&&p| p == end) {
                            let other = w.start_pin.clone();
                            connected.push((w_idx, (*np).clone(), other, false));
                        }
                    }
                }

                if connected.len() >= 3 {
                    continue;
                }

                changed_this_pass = true;
                if connected.len() == 2 {
                    let (idx0, _np0, other0, node_is_start0) = &connected[0];
                    let (idx1, _np1, other1, node_is_start1) = &connected[1];
                    let w0 = self.wires[*idx0].clone();
                    let w1 = self.wires[*idx1].clone();

                    if other0 != other1 {
                        // Points from other0 to node
                        let mut pts0 = if *node_is_start0 {
                            let mut p = w0.points.clone();
                            p.reverse();
                            p
                        } else {
                            w0.points.clone()
                        };

                        // Points from node to other1
                        let pts1 = if *node_is_start1 {
                            w1.points.clone()
                        } else {
                            let mut p = w1.points.clone();
                            p.reverse();
                            p
                        };

                        pts0.extend(pts1);
                        let mut merged =
                            Wire::from_saved(&w0.id, other0.clone(), other1.clone(), pts0);
                        merged.is_bus = w0.is_bus || w1.is_bus;
                        merged.selected = w0.selected || w1.selected || node_selected;
                        merged.rem_null_lines(true);

                        let remove_indices = if idx0 > idx1 {
                            vec![*idx0, *idx1]
                        } else {
                            vec![*idx1, *idx0]
                        };
                        for r_idx in remove_indices {
                            self.wires.remove(r_idx);
                        }
                        self.wires.push(merged);
                    } else {
                        let remove_indices = if idx0 > idx1 {
                            vec![*idx0, *idx1]
                        } else {
                            vec![*idx1, *idx0]
                        };
                        for r_idx in remove_indices {
                            self.wires.remove(r_idx);
                        }
                    }
                } else if connected.len() == 1 {
                    let (idx0, _, _, _) = connected[0];
                    self.wires.remove(idx0);
                }

                // Remove the node item
                if let Some(pos) = self.items.iter().position(|it| it.id == node_id) {
                    self.items.remove(pos);
                }

                // Break to restart pass after modifying items & wires
                break;
            }

            if !changed_this_pass {
                break;
            }
            changed_overall = true;
        }
        changed_overall
    }

    pub fn splice_and_close(&mut self, wire_idx: usize, at: Point) -> bool {
        if wire_idx >= self.wires.len() || self.wires[wire_idx].drawing() {
            return false;
        }
        let target_bus = self.wires[wire_idx].is_bus;
        if self
            .wires
            .iter()
            .rev()
            .find(|w| w.drawing())
            .is_some_and(|w| w.is_bus != target_bus)
        {
            return false;
        }
        let Some((node_id, _p0, p1, _p2)) = self.splice_wire(wire_idx, at) else {
            return false;
        };
        let pin = Pin {
            id: p1,
            item_id: node_id,
            local: Point::zero(),
            angle: 90,
            length: 0.0,
            is_bus: target_bus,
            label: String::new(),
            unused: false,
            direction: None,
        };
        self.close_wire(&pin, at)
    }

    /// Alt-click on a closed wire: splice a Node and start a new wire from it
    /// (`ConnectorLine::connectToWire` when no connector is in progress).
    pub fn splice_and_start(&mut self, wire_idx: usize, at: Point) -> bool {
        if self.drawing() {
            return false;
        }
        if wire_idx >= self.wires.len() || self.wires[wire_idx].drawing() {
            return false;
        }
        let is_bus = self.wires[wire_idx].is_bus;
        let Some((node_id, _p0, p1, _p2)) = self.splice_wire(wire_idx, at) else {
            return false;
        };
        let pin = Pin {
            id: p1,
            item_id: node_id,
            local: Point::zero(),
            angle: 90,
            length: 0.0,
            is_bus,
            label: String::new(),
            unused: false,
            direction: None,
        };
        self.start_wire(&pin, at)
    }

    fn splice_wire(
        &mut self,
        wire_idx: usize,
        at: Point,
    ) -> Option<(String, String, String, String)> {
        let node_id = self.add_node(at.x, at.y);
        let p0 = format!("{node_id}-0");
        let p1 = format!("{node_id}-1");
        let p2 = format!("{node_id}-2");
        let new_id = format!("Connector-{}", self.next_wire);
        self.next_wire += 1;
        let old = self.wires[wire_idx].clone();
        let Some((a, b)) = old.split(at, new_id, p0.clone(), p2.clone()) else {
            self.items.pop();
            self.next_node = self.next_node.saturating_sub(1);
            return None;
        };
        self.wires[wire_idx] = a;
        self.wires.push(b);
        Some((node_id, p0, p1, p2))
    }

    pub fn drag_wire_segment(&mut self, idx: usize, seg: usize, delta: Point) -> Option<usize> {
        let w = self.wires.get_mut(idx)?;
        if w.drawing() {
            return None;
        }
        Some(w.drag_segment(seg, delta))
    }

    pub fn drag_wire_corner(
        &mut self,
        idx: usize,
        vertex: usize,
        delta: Point,
        free: bool,
    ) -> Option<usize> {
        let w = self.wires.get_mut(idx)?;
        if w.drawing() {
            return None;
        }
        Some(w.drag_corner(vertex, delta, free))
    }

    pub fn finish_wire_drag(&mut self, idx: usize) -> bool {
        let Some(w) = self.wires.get(idx) else {
            return false;
        };
        if w.drawing() {
            return false;
        }
        let start_id = w.start_pin.clone();
        let end_id = w.end_pin.clone();
        let start = self.pin_scene(&start_id);
        let end = end_id.as_ref().and_then(|e| self.pin_scene(e));
        let Some(w) = self.wires.get_mut(idx) else {
            return false;
        };
        match (start, end) {
            (Some(s), Some(e)) => w.reanchor(s, e),
            _ => w.rem_null_lines(true),
        }
        true
    }
}
