//! Dynamic time sources and Trapezoidal / Euler numerical integration for reactive elements.

use crate::digital::secs_to_ps;
use crate::elements::{Kind, LAMP_THERMAL_TAU, transformer_companion};

use super::Circuit;

impl Circuit {
    pub fn has_reactive(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(
                &c.kind,
                Kind::Capacitor { .. }
                    | Kind::ElCapacitor { .. }
                    | Kind::VarCapacitor { .. }
                    | Kind::Inductor { .. }
                    | Kind::VarInductor { .. }
                    | Kind::Transformer { .. }
                    | Kind::Lamp { .. }
            ) || matches!(&c.kind, Kind::Relay { inductance, .. } if *inductance > 0.0)
        })
    }

    pub fn has_time_sources(&self) -> bool {
        self.components.iter().any(|c| match &c.kind {
            Kind::WaveGen { freq_hz, .. } => *freq_hz > 0.0,
            Kind::Clock { freq_khz, .. } => *freq_khz > 0.0,
            _ => false,
        })
    }

    pub fn analog_ps(&self) -> u64 {
        secs_to_ps(self.dt).max(1)
    }

    /// Build nets from connectors, stamp, solve. Node voltages are then
    /// available via [`pin_voltage`]. Nonlinear analog parts Newton-iterate
    /// like C++ `Simulator::solveCircuit`. Digital delay events that fire
    /// before the next analog clock are drained so DC matches C++ after
    /// `IoComponent` delay settles.

    pub(super) fn update_sources(&mut self) -> bool {
        if self.time_sources_cached.is_empty() {
            return false;
        }
        let t = self.circ_time;
        let mut changed = false;
        for &idx in &self.time_sources_cached {
            if let Some(c) = self.components.get_mut(idx) {
                if let Kind::WaveGen { .. } = &c.kind {
                    let v = c.wavegen_calc_vout(t);
                    if let Kind::WaveGen { v_out, .. } = &mut c.kind {
                        if (v - *v_out).abs() > 1e-12 {
                            *v_out = v;
                            changed = true;
                        }
                    }
                } else if let Kind::Clock { .. } = &c.kind {
                    let high = c.clock_is_high(t);
                    if let Kind::Clock { state, .. } = &mut c.kind {
                        if *state != high {
                            *state = high;
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }

    pub(super) fn update_reactive(&mut self) -> bool {
        let dt = self.dt;
        if dt <= 0.0 || !self.has_reactive() {
            return false;
        }
        let nodes = &self.nodes;
        let mut changed = false;
        for (c, cache) in self.components.iter_mut().zip(&self.pin_caches) {
            match &mut c.kind {
                Kind::Capacitor { volt, .. }
                | Kind::ElCapacitor { volt, .. }
                | Kind::VarCapacitor { volt, .. } => {
                    let (Some(n0), Some(n1)) = (cache.left, cache.right) else {
                        continue;
                    };
                    if n0 >= nodes.len() || n1 >= nodes.len() {
                        continue;
                    }
                    let new_volt = nodes[n0].volt - nodes[n1].volt;
                    if (*volt - new_volt).abs() > 1e-6 {
                        changed = true;
                    }
                    *volt = new_volt;
                }
                Kind::Inductor {
                    inductance, ieq, ..
                }
                | Kind::VarInductor {
                    inductance, ieq, ..
                } => {
                    let (Some(n0), Some(n1)) = (cache.left, cache.right) else {
                        continue;
                    };
                    if n0 >= nodes.len() || n1 >= nodes.len() {
                        continue;
                    }
                    let g = dt / *inductance;
                    let di = (nodes[n0].volt - nodes[n1].volt) * g;
                    if di.abs() > 1e-8 {
                        changed = true;
                    }
                    *ieq -= di;
                }
                Kind::Transformer {
                    inductance1,
                    inductance2,
                    coupling,
                    r_coil1,
                    r_coil2,
                    i1,
                    i2,
                } => {
                    let vp1 = cache.left.and_then(|n| nodes.get(n)).map(|nd| nd.volt);
                    let vp2 = cache.right.and_then(|n| nodes.get(n)).map(|nd| nd.volt);
                    let vs1 = cache.in0.and_then(|n| nodes.get(n)).map(|nd| nd.volt);
                    let vs2 = cache.in1.and_then(|n| nodes.get(n)).map(|nd| nd.volt);
                    let p_closed = vp1.is_some() && vp2.is_some();
                    let s_closed = vs1.is_some() && vs2.is_some();
                    if !p_closed && !s_closed {
                        continue;
                    }
                    let eff_k = if p_closed && s_closed { *coupling } else { 0.0 };
                    let (g11, g22, g12, i_hist1, i_hist2) = transformer_companion(
                        *inductance1,
                        *inductance2,
                        eff_k,
                        *r_coil1,
                        *r_coil2,
                        if p_closed { *i1 } else { 0.0 },
                        if s_closed { *i2 } else { 0.0 },
                        dt,
                    );
                    let v1 = vp1.zip(vp2).map(|(a, b)| a - b).unwrap_or(0.0);
                    let v2 = vs1.zip(vs2).map(|(a, b)| a - b).unwrap_or(0.0);
                    let new_i1 = if p_closed {
                        g11 * v1 + g12 * v2 + i_hist1
                    } else {
                        0.0
                    };
                    let new_i2 = if s_closed {
                        g12 * v1 + g22 * v2 + i_hist2
                    } else {
                        0.0
                    };
                    if (*i1 - new_i1).abs() > 1e-8 || (*i2 - new_i2).abs() > 1e-8 {
                        changed = true;
                    }
                    *i1 = new_i1;
                    *i2 = new_i2;
                }
                Kind::Relay {
                    r_coil,
                    inductance,
                    coil_i,
                    ..
                } if *inductance > 0.0 => {
                    let (Some(n0), Some(n1)) = (cache.left, cache.right) else {
                        continue;
                    };
                    if n0 >= nodes.len() || n1 >= nodes.len() {
                        continue;
                    }
                    let r = r_coil.max(1e-6);
                    let l_over_dt = *inductance / dt;
                    let g = 1.0 / (r + l_over_dt);
                    let v = nodes[n0].volt - nodes[n1].volt;
                    let new_i = g * v + g * l_over_dt * *coil_i;
                    if (new_i - *coil_i).abs() > 1e-9 {
                        changed = true;
                    }
                    *coil_i = new_i;
                }
                Kind::Lamp {
                    voltage,
                    power,
                    r_cold,
                    resistance,
                    heat,
                    tau,
                } => {
                    let v = match (cache.left, cache.right) {
                        (Some(n0), Some(n1)) if n0 < nodes.len() && n1 < nodes.len() => {
                            nodes[n0].volt - nodes[n1].volt
                        }
                        _ => 0.0,
                    };
                    let r = resistance.max(1e-6);
                    let p_elec = v * v / r;
                    let p_nom = power.max(1e-6);
                    let thermal = if *tau > 0.0 { *tau } else { LAMP_THERMAL_TAU };
                    let du = (p_elec / p_nom - *heat) * (dt / thermal);
                    *heat = (*heat + du).clamp(0.0, 1.5);
                    let r_hot = if *power > 1e-9 {
                        (*voltage * *voltage / *power).max(*r_cold)
                    } else {
                        *r_cold
                    };
                    let new_r = *r_cold + (r_hot - *r_cold) * heat.clamp(0.0, 1.0);
                    if (new_r - *resistance).abs() > 1e-4 {
                        changed = true;
                    }
                    *resistance = new_r;
                }
                _ => {}
            }
        }
        changed
    }
}
