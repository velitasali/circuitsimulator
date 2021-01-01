//! Newton-Raphson non-linear companion model iterations and convergence checks.

use crate::elements::Kind;

use super::Circuit;

impl Circuit {
    pub fn scan_has_nonlinear(&self) -> bool {
        self.components.iter().any(|c| {
            matches!(
                &c.kind,
                Kind::Diode { .. }
                    | Kind::Led { .. }
                    | Kind::Bjt { .. }
                    | Kind::Mosfet { .. }
                    | Kind::OpAmp { .. }
                    | Kind::Jfet { .. }
                    | Kind::VoltReg { .. }
                    | Kind::Scr { .. }
                    | Kind::Triac { .. }
                    | Kind::Diac { .. }
                    | Kind::AnalogMux { .. }
            )
        })
    }

    #[inline]

    pub fn has_nonlinear(&self) -> bool {
        self.has_nonlinear_cached
    }

    pub(super) fn reset_device_stamps(&mut self) {
        let slope = self.slope_steps;
        for c in &mut self.components {
            match &mut c.kind {
                Kind::Diode { state, .. } => state.reset_stamp(),
                Kind::Led { state } => state.reset_stamp(),
                Kind::Bjt { state } => state.reset_stamp(),
                Kind::Mosfet { state } => state.reset_stamp(),
                Kind::OpAmp { state } => state.reset_stamp(),
                Kind::Jfet { state } => state.reset_stamp(),
                Kind::Comparator { state } => state.stamp_init(slope),
                Kind::VoltReg { state } => state.reset_stamp(),
                Kind::Gate(g) => g.stamp_init(slope),
                Kind::FlipFlop(f) => f.stamp_init(slope),
                Kind::Latch(l) => l.stamp_init(slope),
                Kind::McuPin(p) => p.stamp_init(slope),
                Kind::Mcu(m) => m.stamp_init(slope),
                Kind::QemuDevice(q) => {
                    q.time_offset = self.circ_time;
                    q.stamp_init(slope);
                }
                Kind::ScriptCpu(s) => s.stamp_init(slope),
                Kind::TestUnit(t) => t.stamp_init(slope),
                _ => {}
            }
        }
    }

    /// Analog Newton + digital `voltChanged` (C++ `solveCircuit` inner loop).
    pub(super) fn update_nonlinear(&mut self) -> bool {
        if !self.has_nonlinear_cached {
            return true;
        }
        let nodes = &self.nodes;
        let mut converged = true;
        for (c, cache) in self.components.iter_mut().zip(&self.pin_caches) {
            if !c.kind.update_nonlinear(cache, nodes) {
                converged = false;
            }
        }
        converged
    }
}
