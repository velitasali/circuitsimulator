//! Runtime component linking dispatch (`Links="target0,target1,"`).
//!
//! Port of C++ `Linker` / `LinkerComponent` value, string, and property forwarding:
//! - `Led` forwards forward current (A) to `Csource` (`out = I * Gain`), normalized
//!   brightness (`0..1000 lux`) to `Ldr`, and current (A) to scripted/other targets.
//! - `Dial` forwards its numeric value to linked targets (`Potentiometer`, `VarResistor`,
//!   `VarCapacitor`, `VarInductor`, `Ldr`, `VoltSource`, `CurrSource`, `Csource`, `Dial`,
//!   `Shape(Text)`, `ScriptCpu`).
//! - `DcMotor` / `Stepper` forwards rotation angle scaled `0..1000` (`1000 = 360°`).
//! - `ScriptCpu` dispatches queued `setLinkedValue`, `setLinkedString`, and `setPropStr`
//!   commands to its indexed targets.

use rustc_hash::FxHashMap;

use crate::components::Ldr;
use crate::elements::Kind;
use crate::script::ScriptLinkCmd;
use crate::units::parse_si;

use super::Circuit;

enum SourceEmission {
    Value {
        val: f64,
        led_brightness: Option<f64>,
    },
    ScriptCmds(Vec<ScriptLinkCmd>),
    None,
}

impl Circuit {
    /// Register ordered linked target IDs for component `from`.
    pub fn set_links(&mut self, from: impl Into<String>, targets: Vec<String>) {
        let from = from.into();
        if targets.is_empty() {
            self.comp_links.remove(&from);
        } else {
            self.comp_links.insert(from, targets);
        }
        self.last_linked_sent.clear();
        if !self.components.is_empty() {
            self.rebuild_links();
        }
    }

    /// Remove all registered component links.
    pub fn clear_links(&mut self) {
        self.comp_links.clear();
        self.resolved_links.clear();
        self.last_linked_sent.clear();
    }

    /// Resolve `comp_links` IDs into component indices after `components` are populated.
    pub(super) fn rebuild_links(&mut self) {
        self.resolved_links.clear();
        if self.comp_links.is_empty() {
            return;
        }
        let mut by_id = FxHashMap::default();
        for (i, c) in self.components.iter().enumerate() {
            by_id.insert(c.id.as_str(), i);
        }
        for (src_idx, c) in self.components.iter().enumerate() {
            let Some(targets) = self.comp_links.get(&c.id) else {
                continue;
            };
            let mut resolved = Vec::with_capacity(targets.len());
            for tid in targets {
                if let Some(&dst_idx) = by_id.get(tid.as_str()) {
                    if dst_idx != src_idx {
                        resolved.push(dst_idx);
                    }
                }
            }
            if !resolved.is_empty() {
                self.resolved_links.push((src_idx, resolved));
            }
        }
    }

    /// Propagate values, strings, and property updates across all linked components.
    /// Returns `true` if any electrical target's parameters changed (needing re-stamp/re-solve).
    pub(super) fn propagate_links(&mut self) -> bool {
        if self.resolved_links.is_empty() {
            return false;
        }
        let mut electrical_changed = false;
        // Up to two passes so `Dial -> ScriptCpu -> Target` settles in a single call.
        for _pass in 0..2 {
            let mut pass_activity = false;
            let n = self.resolved_links.len();
            for link_idx in 0..n {
                let src_idx = self.resolved_links[link_idx].0;
                let emission = match self.components.get_mut(src_idx).map(|c| &mut c.kind) {
                    Some(Kind::Dial { value, .. }) => {
                        let v = *value;
                        if should_emit(&mut self.last_linked_sent, src_idx, v, 1e-12) {
                            SourceEmission::Value {
                                val: v,
                                led_brightness: None,
                            }
                        } else {
                            SourceEmission::None
                        }
                    }
                    Some(Kind::Led { state }) => {
                        let curr = state.current.max(0.0);
                        if should_emit(&mut self.last_linked_sent, src_idx, curr, 1e-12) {
                            let max_i = state.max_current.max(1e-9);
                            let brightness = (curr / max_i).clamp(0.0, 1.0);
                            SourceEmission::Value {
                                val: curr,
                                led_brightness: Some(brightness),
                            }
                        } else {
                            SourceEmission::None
                        }
                    }
                    Some(Kind::DcMotor { angle, .. } | Kind::Stepper { angle, .. }) => {
                        let scaled = (angle.rem_euclid(360.0) / 360.0) * 1000.0;
                        if should_emit(&mut self.last_linked_sent, src_idx, scaled, 1e-6) {
                            SourceEmission::Value {
                                val: scaled,
                                led_brightness: None,
                            }
                        } else {
                            SourceEmission::None
                        }
                    }
                    Some(Kind::ScriptCpu(cpu)) => {
                        let cmds = cpu.take_link_cmds();
                        if cmds.is_empty() {
                            SourceEmission::None
                        } else {
                            SourceEmission::ScriptCmds(cmds)
                        }
                    }
                    _ => SourceEmission::None,
                };

                match emission {
                    SourceEmission::None => {}
                    SourceEmission::Value {
                        val,
                        led_brightness,
                    } => {
                        let targets = self.resolved_links[link_idx].1.clone();
                        for dst_idx in targets {
                            let (elec, acted) =
                                self.apply_linked_value(dst_idx, val, 0, led_brightness);
                            electrical_changed |= elec;
                            pass_activity |= acted;
                        }
                    }
                    SourceEmission::ScriptCmds(cmds) => {
                        let targets = self.resolved_links[link_idx].1.clone();
                        for cmd in cmds {
                            match cmd {
                                ScriptLinkCmd::SetValue { index, val, i } => {
                                    if let Some(&dst_idx) = targets.get(index) {
                                        let (elec, acted) =
                                            self.apply_linked_value(dst_idx, val, i, None);
                                        electrical_changed |= elec;
                                        pass_activity |= acted;
                                    }
                                }
                                ScriptLinkCmd::SetString { index, text, i } => {
                                    if let Some(&dst_idx) = targets.get(index) {
                                        let acted = self.apply_linked_string(dst_idx, &text, i);
                                        pass_activity |= acted;
                                    }
                                }
                                ScriptLinkCmd::SetPropStr { index, prop, val } => {
                                    if let Some(&dst_idx) = targets.get(index) {
                                        let (elec, acted) =
                                            self.apply_linked_prop_str(dst_idx, &prop, &val);
                                        electrical_changed |= elec;
                                        pass_activity |= acted;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !pass_activity {
                break;
            }
        }
        electrical_changed
    }

    /// Apply a linked numeric value to target component `dst_idx`.
    /// Returns `(electrical_changed, any_activity)`.
    fn apply_linked_value(
        &mut self,
        dst_idx: usize,
        val: f64,
        i: i32,
        led_brightness: Option<f64>,
    ) -> (bool, bool) {
        let Some(comp) = self.components.get_mut(dst_idx) else {
            return (false, false);
        };
        match &mut comp.kind {
            Kind::Csource {
                control_pins,
                curr_source,
                gain,
                volt,
                current,
                ctrl,
                ..
            } => {
                if *control_pins {
                    return (false, false);
                }
                *ctrl = val;
                let out = val * *gain;
                if *curr_source {
                    let changed = (*current - out).abs() > 1e-9;
                    *current = out;
                    (changed, changed)
                } else {
                    let changed = (*volt - out).abs() > 1e-9;
                    *volt = out;
                    (changed, changed)
                }
            }
            Kind::Potentiometer { resistance, wiper } => {
                if i == 1 {
                    let r = val.clamp(1e-12, 1e12);
                    let changed = (*resistance - r).abs() > 1e-9;
                    *resistance = r;
                    (changed, changed)
                } else {
                    let w = (val / 1000.0).clamp(0.0, 1.0);
                    let changed = (*wiper - w).abs() > 1e-9;
                    *wiper = w;
                    (changed, changed)
                }
            }
            Kind::VarResistor { resistance } => {
                let r = val.clamp(1e-12, 1e12);
                let changed = (*resistance - r).abs() > 1e-9;
                *resistance = r;
                (changed, changed)
            }
            Kind::VarCapacitor { capacitance, .. } => {
                let c = val.clamp(1e-15, 1e3);
                let changed = (*capacitance - c).abs() > 1e-15;
                *capacitance = c;
                (changed, changed)
            }
            Kind::VarInductor { inductance, .. } => {
                let l = val.clamp(1e-15, 1e6);
                let changed = (*inductance - l).abs() > 1e-15;
                *inductance = l;
                (changed, changed)
            }
            Kind::Ldr {
                resistance,
                lux,
                r_dark,
                r_light,
            } => {
                let new_lux = match led_brightness {
                    Some(b) => (b * 1000.0).clamp(0.1, 1e8),
                    None => val.clamp(0.1, 1e8),
                };
                let new_r = Ldr {
                    lux: new_lux,
                    r_dark: *r_dark,
                    r_light: *r_light,
                    dial_step: 0.0,
                }
                .resistance();
                let changed = (*lux - new_lux).abs() > 1e-6 || (*resistance - new_r).abs() > 1e-6;
                *lux = new_lux;
                *resistance = new_r;
                (changed, changed)
            }
            Kind::VoltSource { value, .. } => {
                let v = val.clamp(-1e6, 1e6);
                let changed = (*value - v).abs() > 1e-9;
                *value = v;
                (changed, changed)
            }
            Kind::CurrSource { value, .. } => {
                let a = val.clamp(-1e6, 1e6);
                let changed = (*value - a).abs() > 1e-9;
                *value = a;
                (changed, changed)
            }
            Kind::Dial {
                value,
                min_val,
                max_val,
                ..
            } => {
                let lo = min_val.min(*max_val);
                let hi = min_val.max(*max_val);
                let next = val.clamp(lo, hi);
                let changed = (*value - next).abs() > 1e-9;
                *value = next;
                (false, changed)
            }
            Kind::Shape {
                shape_kind, text, ..
            } if shape_kind == "Text" => {
                let formatted = format_linked_number(val);
                let changed = *text != formatted;
                *text = formatted;
                (false, changed)
            }
            Kind::ScriptCpu(cpu) => {
                cpu.call_set_linked_value(val, i);
                (true, true)
            }
            _ => (false, false),
        }
    }

    /// Apply a linked string to target component `dst_idx`.
    fn apply_linked_string(&mut self, dst_idx: usize, text: &str, i: i32) -> bool {
        let Some(comp) = self.components.get_mut(dst_idx) else {
            return false;
        };
        match &mut comp.kind {
            Kind::Shape {
                shape_kind,
                text: dst,
                ..
            } if shape_kind == "Text" => {
                if i == 1 {
                    dst.push_str(text);
                    !text.is_empty()
                } else {
                    let changed = dst != text;
                    *dst = text.to_string();
                    changed
                }
            }
            Kind::ScriptCpu(cpu) => {
                cpu.call_set_linked_string(text, i);
                true
            }
            _ => false,
        }
    }

    /// Apply a property string update from a scripted linker (`component.setPropStr`).
    fn apply_linked_prop_str(&mut self, dst_idx: usize, prop: &str, val: &str) -> (bool, bool) {
        let Some(comp) = self.components.get_mut(dst_idx) else {
            return (false, false);
        };
        let key = prop.replace('_', "").to_ascii_lowercase();
        match &mut comp.kind {
            Kind::Led { state } => match key.as_str() {
                "threshold" => {
                    let v = parse_si(val, "V").clamp(0.0, 100.0);
                    let changed = (state.threshold - v).abs() > 1e-9;
                    state.threshold = v;
                    (changed, changed)
                }
                "maxcurrent" => {
                    let v = parse_si(val, "A").clamp(1e-6, 1e6);
                    let changed = (state.max_current - v).abs() > 1e-9;
                    state.max_current = v;
                    (changed, changed)
                }
                "resistance" => {
                    let v = parse_si(val, "Ω").clamp(1e-12, 1e12);
                    let changed = (state.impedance - v).abs() > 1e-9;
                    state.impedance = v;
                    (changed, changed)
                }
                _ => (false, false),
            },
            Kind::Mosfet { state } => match key.as_str() {
                "resistance" | "rdson" => {
                    let v = parse_si(val, "Ω").clamp(1e-6, 1e9);
                    let changed = (state.rdson - v).abs() > 1e-9;
                    state.rdson = v;
                    (changed, changed)
                }
                "threshold" => {
                    let v = parse_si(val, "V").clamp(-1e3, 1e3);
                    let changed = (state.threshold - v).abs() > 1e-9;
                    state.threshold = v;
                    (changed, changed)
                }
                _ => (false, false),
            },
            Kind::Potentiometer { resistance, wiper } => match key.as_str() {
                "resistance" => {
                    let v = parse_si(val, "Ω").clamp(1e-12, 1e12);
                    let changed = (*resistance - v).abs() > 1e-9;
                    *resistance = v;
                    (changed, changed)
                }
                "wiper" => {
                    let v = val.trim().parse::<f64>().unwrap_or(*wiper).clamp(0.0, 1.0);
                    let changed = (*wiper - v).abs() > 1e-9;
                    *wiper = v;
                    (changed, changed)
                }
                _ => (false, false),
            },
            Kind::Dial {
                value,
                min_val,
                max_val,
                step,
            } => match key.as_str() {
                "value" => {
                    if let Ok(v) = val.trim().parse::<f64>() {
                        let lo = min_val.min(*max_val);
                        let hi = min_val.max(*max_val);
                        let next = v.clamp(lo, hi);
                        let changed = (*value - next).abs() > 1e-9;
                        *value = next;
                        return (false, changed);
                    }
                    (false, false)
                }
                "minval" => {
                    if let Ok(v) = val.trim().parse::<f64>() {
                        let changed = (*min_val - v).abs() > 1e-9;
                        *min_val = v;
                        return (false, changed);
                    }
                    (false, false)
                }
                "maxval" => {
                    if let Ok(v) = val.trim().parse::<f64>() {
                        let changed = (*max_val - v).abs() > 1e-9;
                        *max_val = v;
                        return (false, changed);
                    }
                    (false, false)
                }
                "step" | "steps" => {
                    if let Ok(v) = val.trim().parse::<f64>() {
                        let changed = (*step - v).abs() > 1e-9;
                        *step = v;
                        return (false, changed);
                    }
                    (false, false)
                }
                _ => (false, false),
            },
            Kind::Shape {
                shape_kind, text, ..
            } if shape_kind == "Text" && key == "text" => {
                let changed = text != val;
                *text = val.to_string();
                (false, changed)
            }
            _ => (false, false),
        }
    }
}

fn should_emit(last: &mut FxHashMap<usize, f64>, src_idx: usize, val: f64, eps: f64) -> bool {
    if let Some(&prev) = last.get(&src_idx) {
        if (prev - val).abs() <= eps {
            return false;
        }
    }
    last.insert(src_idx, val);
    true
}

fn format_linked_number(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 && v.abs() < 1e12 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.3}")
    }
}
