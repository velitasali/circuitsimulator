//! Dynamic time sources and Trapezoidal / Euler numerical integration for reactive elements.

use crate::digital::secs_to_ps;
use crate::elements::Kind;

use super::Circuit;

impl Circuit {
    pub fn has_reactive(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(
                &c.kind,
                Kind::Capacitor { .. } | Kind::ElCapacitor { .. } | Kind::Inductor { .. }
            )
        })
    }

    pub fn has_time_sources(&self) -> bool {
        self.components.iter().any(|c| match &c.kind {
            Kind::WaveGen { freq_hz, .. } => *freq_hz > 0.0,
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
            let (Some(n0), Some(n1)) = (cache.left, cache.right) else {
                continue;
            };
            if n0 >= nodes.len() || n1 >= nodes.len() {
                continue;
            }
            let (v0, v1) = (nodes[n0].volt, nodes[n1].volt);
            match &mut c.kind {
                Kind::Capacitor { volt, .. } | Kind::ElCapacitor { volt, .. } => {
                    let new_volt = v0 - v1;
                    if (*volt - new_volt).abs() > 1e-6 {
                        changed = true;
                    }
                    *volt = new_volt;
                }
                Kind::Inductor {
                    inductance, ieq, ..
                } => {
                    let g = dt / *inductance;
                    let di = (v0 - v1) * g;
                    if di.abs() > 1e-8 {
                        changed = true;
                    }
                    *ieq -= di;
                }
                _ => {}
            }
        }
        changed
    }
}
