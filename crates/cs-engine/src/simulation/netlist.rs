//! SPICE and KiCad 6 netlists from a built circuit.
//!
//! Ground pins land on SPICE node 0 and on a KiCad net named `GND`.
//! Two-terminal parts use pin 1 for `lPin` and pin 2 for `rPin`.
//! A fixed voltage source uses its output as pin 1 and node 0 as pin 2.
//! Parts this exporter does not model are listed in a comment and omitted
//! from the netlist body.

use std::collections::HashSet;

use rustc_hash::FxHashMap;

use crate::Error;
use crate::elements::{Comp, Kind, SWITCH_CLOSED_ADMIT};

use super::Circuit;

const OPEN_OHMS: f64 = 1e12;

#[derive(Clone, Debug)]
struct Dev {
    reference: String,
    a: usize,
    b: usize,
    value: String,
    lib: &'static str,
}

impl Circuit {
    /// Text netlist ending in `.end`. Builds nets when they are empty.
    pub fn spice_netlist(&mut self) -> Result<String, Error> {
        self.ensure_nets();
        let (devs, skipped) = self.devices();
        let mut out = String::from("* Circuit Simulator SPICE netlist\n* Ground nets are node 0\n");
        for id in skipped {
            out.push_str(&format!("* skipped {id}\n"));
        }
        for d in &devs {
            let tail = if d.lib == "V" {
                format!("DC {}", d.value)
            } else {
                d.value.clone()
            };
            out.push_str(&format!("{} {} {} {tail}\n", d.reference, d.a, d.b));
        }
        out.push_str(".end\n");
        Ok(out)
    }

    /// KiCad 6 s-expression netlist (`(export (version "D") ...)`).
    pub fn kicad_netlist(&mut self) -> Result<String, Error> {
        self.ensure_nets();
        let (devs, _) = self.devices();
        let code_count = devs
            .iter()
            .flat_map(|d| [d.a, d.b])
            .map(|n| n + 2)
            .max()
            .unwrap_or(2);
        let mut comps = String::new();
        let mut nodes_by_net: Vec<Vec<String>> = vec![Vec::new(); code_count];
        for d in &devs {
            comps.push_str(&format!(
                "    (comp (ref \"{}\")\n      (value \"{}\")\n      (footprint \"\")\n      (libsource (lib \"Device\") (part \"{}\") (description \"\"))\n      (sheetpath (names \"/\") (tstamps \"/\"))\n      (tstamp \"{}\"))\n",
                d.reference, d.value, d.lib, d.reference
            ));
            for (pin, spice_node) in [("1", d.a), ("2", d.b)] {
                let code = spice_node + 1;
                if let Some(bucket) = nodes_by_net.get_mut(code) {
                    bucket.push(format!(
                        "      (node (ref \"{}\") (pin \"{pin}\") (pintype \"passive\"))\n",
                        d.reference
                    ));
                }
            }
        }
        let mut nets = String::new();
        for (code, nodes) in nodes_by_net.iter().enumerate().skip(1) {
            if nodes.is_empty() {
                continue;
            }
            let name = if code == 1 {
                "GND".to_string()
            } else {
                format!("N{}", code - 1)
            };
            nets.push_str(&format!("    (net (code \"{code}\") (name \"{name}\")\n"));
            for node in nodes {
                nets.push_str(node);
            }
            nets.push_str("    )\n");
        }
        Ok(format!(
            "(export (version \"D\")\n  (design\n    (source \"Circuit Simulator\")\n    (date \"\")\n    (tool \"Circuit Simulator\"))\n  (components\n{comps}  )\n  (nets\n{nets}  ))\n"
        ))
    }

    fn ensure_nets(&mut self) {
        if self.pin_net.is_empty() {
            self.build_nets();
        }
    }

    fn devices(&self) -> (Vec<Dev>, Vec<String>) {
        let mut map = NetMap::build(self);
        let mut used = HashSet::new();
        let mut devs = Vec::new();
        let mut skipped = Vec::new();
        for comp in &self.components {
            match devices_of(comp, &self.pin_net, &mut map, &mut used) {
                Some(list) => devs.extend(list),
                None => {
                    if !matches!(
                        comp.kind,
                        Kind::Ground | Kind::Junction | Kind::Tunnel { .. }
                    ) {
                        skipped.push(comp.id.clone());
                    }
                }
            }
        }
        (devs, skipped)
    }
}

struct NetMap {
    /// Circuit node index to SPICE node. Ground is 0.
    spice: Vec<usize>,
    next_extra: usize,
}

impl NetMap {
    fn build(c: &Circuit) -> Self {
        let mut ground = HashSet::new();
        for comp in &c.components {
            if matches!(comp.kind, Kind::Ground) {
                if let Some(&n) = c.pin_net.get(&comp.gnd_pin()) {
                    ground.insert(n);
                }
            }
        }
        let mut spice = vec![0; c.nodes.len()];
        let mut next = 1usize;
        for (i, slot) in spice.iter_mut().enumerate() {
            if ground.contains(&i) {
                *slot = 0;
            } else {
                *slot = next;
                next += 1;
            }
        }
        Self {
            spice,
            next_extra: next,
        }
    }

    fn of(&self, node: usize) -> usize {
        self.spice.get(node).copied().unwrap_or(0)
    }

    fn pin(&self, pin_net: &FxHashMap<String, usize>, pin: &str) -> Option<usize> {
        pin_net.get(pin).copied().map(|n| self.of(n))
    }

    fn extra(&mut self) -> usize {
        let n = self.next_extra;
        self.next_extra += 1;
        n
    }
}

fn devices_of(
    comp: &Comp,
    pin_net: &FxHashMap<String, usize>,
    map: &mut NetMap,
    used: &mut HashSet<String>,
) -> Option<Vec<Dev>> {
    let ends = |map: &NetMap| -> Option<(usize, usize)> {
        Some((
            map.pin(pin_net, &comp.left_pin())?,
            map.pin(pin_net, &comp.right_pin())?,
        ))
    };
    match &comp.kind {
        Kind::Resistor { resistance }
        | Kind::VarResistor { resistance }
        | Kind::Ldr { resistance, .. }
        | Kind::Thermistor { resistance, .. }
        | Kind::Rtd { resistance, .. }
        | Kind::Strain { resistance, .. } => {
            let (a, b) = ends(map)?;
            Some(vec![dev("R", &comp.id, a, b, num(*resistance), "R", used)])
        }
        Kind::Capacitor { capacitance, .. }
        | Kind::ElCapacitor { capacitance, .. }
        | Kind::VarCapacitor { capacitance, .. } => {
            let (a, b) = ends(map)?;
            Some(vec![dev("C", &comp.id, a, b, num(*capacitance), "C", used)])
        }
        Kind::Inductor { inductance, .. } | Kind::VarInductor { inductance, .. } => {
            let (a, b) = ends(map)?;
            Some(vec![dev("L", &comp.id, a, b, num(*inductance), "L", used)])
        }
        Kind::FixedVolt { voltage } | Kind::Rail { voltage } | Kind::Clock { voltage, .. } => {
            let n = map.pin(pin_net, &comp.out_pin())?;
            Some(vec![dev("V", &comp.id, n, 0, num(*voltage), "V", used)])
        }
        Kind::VoltSource { value, .. } => {
            let n = map.pin(pin_net, &format!("{}-outPin", comp.id))?;
            Some(vec![dev("V", &comp.id, n, 0, num(*value), "V", used)])
        }
        Kind::Battery {
            voltage,
            resistance,
        } => {
            let plus = map.pin(pin_net, &comp.left_pin())?;
            let minus = map.pin(pin_net, &comp.right_pin())?;
            let mid = map.extra();
            Some(vec![
                dev("V", &comp.id, mid, minus, num(*voltage), "V", used),
                dev(
                    "R",
                    &format!("{}_esr", comp.id),
                    plus,
                    mid,
                    num(*resistance),
                    "R",
                    used,
                ),
            ])
        }
        Kind::Switch { closed, .. } => {
            let (a, b) = ends(map)?;
            let ohms = if *closed {
                1.0 / SWITCH_CLOSED_ADMIT
            } else {
                OPEN_OHMS
            };
            Some(vec![dev("R", &comp.id, a, b, num(ohms), "R", used)])
        }
        Kind::Ground | Kind::Junction | Kind::Tunnel { .. } => None,
        _ => None,
    }
}

fn dev(
    prefix: &str,
    id: &str,
    a: usize,
    b: usize,
    value: String,
    lib: &'static str,
    used: &mut HashSet<String>,
) -> Dev {
    Dev {
        reference: spice_ref(prefix, id, used),
        a,
        b,
        value,
        lib,
    }
}

fn num(v: f64) -> String {
    let s = format!("{v:.12}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn spice_ref(prefix: &str, id: &str, used: &mut HashSet<String>) -> String {
    let body: String = id
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    let lead = prefix.chars().next().unwrap_or('X');
    let mut name = if body
        .chars()
        .next()
        .is_some_and(|c| c.eq_ignore_ascii_case(&lead))
    {
        body
    } else if body.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}_{body}")
    };
    if used.contains(&name) {
        let base = name.clone();
        let mut n = 2u32;
        while used.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
    }
    used.insert(name.clone());
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn divider() -> Circuit {
        let mut c = Circuit::new();
        c.add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 1000.0)
            .add_resistor("R2", 1000.0)
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "R2-lPin")
            .connect("R2-rPin", "GND-Gnd");
        c
    }

    fn endpoints(line: &str) -> Vec<usize> {
        line.split_whitespace()
            .skip(1)
            .filter_map(|w| w.parse().ok())
            .take(2)
            .collect()
    }

    #[test]
    fn spice_divider_shares_the_middle_and_grounds_the_bottom() {
        let text = divider().spice_netlist().unwrap();
        assert!(text.contains(".end"), "{text}");
        assert!(text.contains("Ground nets are node 0"), "{text}");
        let resistors: Vec<Vec<usize>> = text
            .lines()
            .filter(|l| l.starts_with('R'))
            .map(endpoints)
            .collect();
        assert_eq!(resistors.len(), 2, "{text}");
        assert!(resistors.iter().any(|n| n.contains(&0)), "{text}");
        let shared: Vec<usize> = resistors[0]
            .iter()
            .copied()
            .filter(|n| resistors[1].contains(n))
            .collect();
        assert!(
            shared.iter().any(|n| *n != 0),
            "resistors do not share a node: {text}"
        );
        assert!(
            text.lines()
                .any(|l| l.starts_with('V') && l.contains(" 0 ")),
            "{text}"
        );
        assert!(text.contains("DC 5"), "{text}");
    }

    #[test]
    fn spice_lists_capacitor_and_inductor() {
        let mut c = Circuit::new();
        c.add_fixed_volt("V1", 5.0)
            .add_capacitor("C1", 1e-6)
            .add_inductor("L1", 1e-3)
            .add_ground("GND")
            .connect("V1-outnod", "C1-lPin")
            .connect("C1-rPin", "L1-lPin")
            .connect("L1-rPin", "GND-Gnd");
        let text = c.spice_netlist().unwrap();
        assert!(
            text.lines()
                .any(|l| l.starts_with('C') && l.contains("0.000001")),
            "{text}"
        );
        assert!(
            text.lines()
                .any(|l| l.starts_with('L') && l.contains("0.001")),
            "{text}"
        );
    }

    #[test]
    fn kicad_export_names_ground_and_lists_the_resistor() {
        let text = divider().kicad_netlist().unwrap();
        assert!(text.contains("(export (version \"D\")"), "{text}");
        assert!(text.contains("(name \"GND\")"), "{text}");
        assert!(text.contains("(part \"R\")"), "{text}");
        assert!(text.contains("(pin \"1\")"), "{text}");
        assert!(text.contains("(pin \"2\")"), "{text}");
    }
}
