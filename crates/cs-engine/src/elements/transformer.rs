//! Ferromagnetic transformer physics models:
//! - Ferromagnetic core saturation with non-linear B-H curve and smooth hyperbolic tangent / sech² differential inductance
//! - Magnetic hysteresis dynamics with coercive current displacement and remanence
//! - Frequency-dependent core losses and eddy currents via parallel core resistance R_core
//! - Multi-winding auxiliary secondaries and center-tapped secondary winding models

use crate::elements::PinCache;
use crate::net::ENode;

/// Minimum physical bounds to avoid numerical singularities.
const MIN_L: f64 = 1e-12;
const MIN_R: f64 = 1e-6;
pub const TRANSFORMER_BLEED_ADMIT: f64 = 1e-12;

/// Physical and dynamic runtime state of a transformer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformerState {
    /// Primary winding self-inductance L1 (H).
    pub inductance1: f64,
    /// Secondary winding self-inductance L2 (H).
    pub inductance2: f64,
    /// Magnetic coupling factor k between primary and secondary (0.0 to 1.0).
    pub coupling: f64,
    /// Primary winding series resistance (Ω).
    pub r_coil1: f64,
    /// Secondary winding series resistance (Ω).
    pub r_coil2: f64,
    /// Core loss / eddy current resistance R_core (Ω).
    /// Finite values model real core eddy current heating and in-phase magnetization current.
    pub r_core: f64,
    /// Core saturation current threshold I_sat (A).
    /// 0.0 disables saturation (ideal linear core). When > 0, differential inductance
    /// collapses non-linearly towards residual air-core inductance L_sat as |I_m| > I_sat.
    pub i_sat: f64,
    /// Coercive current width I_c (A) for magnetic hysteresis B-H loop.
    /// 0.0 disables hysteresis.
    pub hysteresis: f64,
    /// Whether the secondary winding has a center tap (adds pin `ct`).
    pub center_tap: bool,
    /// Whether the transformer has an auxiliary secondary winding (adds pins `a1`, `a2`).
    pub aux_secondary: bool,
    /// Auxiliary secondary winding inductance L3 (H).
    pub inductance3: f64,
    /// Auxiliary secondary winding series resistance (Ω).
    pub r_coil3: f64,

    // Dynamic runtime states
    /// Primary current i1 (A).
    pub i1: f64,
    /// Secondary current i2 (A).
    pub i2: f64,
    /// Auxiliary secondary current i3 (A).
    pub i3: f64,
    /// Hysteresis magnetization direction (+1.0 or -1.0).
    pub h_dir: f64,
    /// Previous magnetizing current for dI_m/dt tracking.
    pub prev_i_mag: f64,
    /// Instantaneous magnetic core flux linkage λ (Wb-turns).
    pub flux: f64,
}

impl Default for TransformerState {
    fn default() -> Self {
        Self {
            inductance1: 1.0,
            inductance2: 1.0,
            coupling: 0.99,
            r_coil1: 0.1,
            r_coil2: 0.1,
            r_core: 1e12,
            i_sat: 0.0,
            hysteresis: 0.0,
            center_tap: false,
            aux_secondary: false,
            inductance3: 1.0,
            r_coil3: 0.1,

            i1: 0.0,
            i2: 0.0,
            i3: 0.0,
            h_dir: 1.0,
            prev_i_mag: 0.0,
            flux: 0.0,
        }
    }
}

impl TransformerState {
    pub fn new(
        inductance1: f64,
        inductance2: f64,
        coupling: f64,
        r_coil1: f64,
        r_coil2: f64,
    ) -> Self {
        let mut s = Self::default();
        s.inductance1 = inductance1.max(MIN_L);
        s.inductance2 = inductance2.max(MIN_L);
        s.coupling = coupling.clamp(0.0, 1.0);
        s.r_coil1 = r_coil1.max(MIN_R);
        s.r_coil2 = r_coil2.max(MIN_R);
        s
    }

    pub fn new_saturable(
        inductance1: f64,
        inductance2: f64,
        coupling: f64,
        r_coil1: f64,
        r_coil2: f64,
        i_sat: f64,
        r_core: f64,
        hysteresis: f64,
    ) -> Self {
        let mut s = Self::new(inductance1, inductance2, coupling, r_coil1, r_coil2);
        s.i_sat = i_sat.max(0.0);
        s.r_core = r_core.max(MIN_R);
        s.hysteresis = hysteresis.max(0.0);
        s
    }

    pub fn new_center_tapped(
        inductance1: f64,
        inductance2: f64,
        coupling: f64,
        r_coil1: f64,
        r_coil2: f64,
    ) -> Self {
        let mut s = Self::new(inductance1, inductance2, coupling, r_coil1, r_coil2);
        s.center_tap = true;
        s
    }

    pub fn new_multi_winding(
        inductance1: f64,
        inductance2: f64,
        inductance3: f64,
        coupling: f64,
        r_coil1: f64,
        r_coil2: f64,
        r_coil3: f64,
    ) -> Self {
        let mut s = Self::new(inductance1, inductance2, coupling, r_coil1, r_coil2);
        s.aux_secondary = true;
        s.inductance3 = inductance3.max(MIN_L);
        s.r_coil3 = r_coil3.max(MIN_R);
        s
    }

    pub fn reset(&mut self) {
        self.i1 = 0.0;
        self.i2 = 0.0;
        self.i3 = 0.0;
        self.h_dir = 1.0;
        self.prev_i_mag = 0.0;
        self.flux = 0.0;
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.i1 = prev.i1;
        self.i2 = prev.i2;
        self.i3 = prev.i3;
        self.h_dir = prev.h_dir;
        self.prev_i_mag = prev.prev_i_mag;
        self.flux = prev.flux;
    }

    /// Turns ratio N2 / N1 = sqrt(L2 / L1).
    #[inline]
    pub fn turns_ratio_secondary(&self) -> f64 {
        (self.inductance2.max(MIN_L) / self.inductance1.max(MIN_L)).sqrt()
    }

    /// Turns ratio N3 / N1 = sqrt(L3 / L1).
    #[inline]
    pub fn turns_ratio_aux(&self) -> f64 {
        (self.inductance3.max(MIN_L) / self.inductance1.max(MIN_L)).sqrt()
    }

    /// Calculate total magnetizing current referred to the primary.
    #[inline]
    pub fn magnetizing_current(&self, i1: f64, i2: f64, i3: f64) -> f64 {
        let n2 = self.turns_ratio_secondary();
        let i_sec = if self.center_tap {
            // For center-tapped secondary, i2 is winding A, i3 is winding B
            0.5 * n2 * (i2 + i3)
        } else {
            n2 * i2
        };
        let i_aux = if self.aux_secondary && !self.center_tap {
            self.turns_ratio_aux() * i3
        } else {
            0.0
        };
        i1 + i_sec + i_aux
    }

    /// Non-linear differential magnetizing inductance L_mag(I_m) accounting for core saturation
    /// and B-H hysteresis dynamics.
    pub fn diff_inductance_and_flux(&self, i_mag: f64) -> (f64, f64, f64) {
        let l1 = self.inductance1.max(MIN_L);
        if self.i_sat <= 0.0 {
            // Linear core
            return (l1, 1.0, l1 * i_mag);
        }

        // Hysteresis effective field shift with adaptive deadband filtering
        let d_imag = i_mag - self.prev_i_mag;
        let d_thresh = (self.i_sat * 1e-5).max(1e-7);
        let mut dir = self.h_dir;
        if d_imag > d_thresh {
            dir = 1.0;
        } else if d_imag < -d_thresh {
            dir = -1.0;
        }

        let u_raw = (i_mag / self.i_sat).clamp(-50.0, 50.0);
        let tanh_u = u_raw.tanh();
        // Hysteresis loop smoothly closes at deep saturation (1 - tanh^2)
        let loop_width = (1.0 - tanh_u * tanh_u).max(0.0);
        let i_eff = i_mag - self.hysteresis * dir * loop_width;

        // Differential inductance calculation: L_sat + (L1 - L_sat) * sech²(i_eff / i_sat)
        let l_sat = (l1 * 0.02).max(1e-12);
        let u_eff = (i_eff / self.i_sat).clamp(-50.0, 50.0);
        let cosh_u = u_eff.cosh();
        let sech2 = 1.0 / (cosh_u * cosh_u);
        let l_mag = l_sat + (l1 - l_sat) * sech2;
        let s_sat = (l_mag / l1).clamp(0.01, 1.0);

        let flux = l_sat * i_mag + (l1 - l_sat) * self.i_sat * u_eff.tanh();
        (l_mag, s_sat, flux)
    }

    /// Solves the multi-winding companion admittance matrix and history current vector.
    pub fn companion(&self, dt: f64) -> MultiWindingCompanion {
        if dt <= 0.0 {
            let mut comp = MultiWindingCompanion::default();
            comp.dim = 2;
            comp.g[0][0] = 1.0 / self.r_coil1.max(MIN_R);
            comp.g[1][1] = 1.0 / self.r_coil2.max(MIN_R);
            return comp;
        }

        let i_m = self.magnetizing_current(self.i1, self.i2, self.i3);
        let (_l_mag, s_sat, flux) = self.diff_inductance_and_flux(i_m);

        let l1 = self.inductance1.max(MIN_L);
        let l2 = self.inductance2.max(MIN_L);
        let k = self.coupling.clamp(0.0, 1.0);
        let r1 = self.r_coil1.max(MIN_R);
        let r2 = self.r_coil2.max(MIN_R);

        // Core loss conductance G_core across primary
        let g_core = if self.r_core > 0.0 && self.r_core < 1e11 {
            1.0 / self.r_core
        } else {
            0.0
        };

        let dim = if self.center_tap && self.aux_secondary {
            4
        } else if self.center_tap || self.aux_secondary {
            3
        } else {
            2
        };

        let mut r = [0.0; 4];
        let mut l = [[0.0; 4]; 4];
        let mut i_prev = [0.0; 4];

        match dim {
            2 => {
                let l1_eff = ((1.0 - k) * l1 * (1.0 - s_sat) + l1 * s_sat).max(MIN_L);
                let l2_eff = (if k > 0.0 {
                    (k * k * s_sat + (1.0 - k) * (1.0 - s_sat)) * l2
                } else {
                    l2 * s_sat
                })
                .max(MIN_L);
                let m = k * (l1 * l2).sqrt() * s_sat;
                let zl1 = l1_eff / dt;
                let zl2 = l2_eff / dt;
                let zm = m / dt;
                let z11 = r1 + zl1;
                let z22 = r2 + zl2;
                let det = (z11 * z22 - zm * zm).max(1e-30);
                let g11 = z22 / det;
                let g22 = z11 / det;
                let g12 = -zm / det;
                let e1 = zl1 * self.i1 + zm * self.i2;
                let e2 = zm * self.i1 + zl2 * self.i2;
                let i_hist1 = g11 * e1 + g12 * e2;
                let i_hist2 = g12 * e1 + g22 * e2;

                let mut comp = MultiWindingCompanion::default();
                comp.dim = 2;
                comp.g[0][0] = g11 + g_core;
                comp.g[1][1] = g22;
                comp.g[0][1] = g12;
                comp.g[1][0] = g12;
                comp.i_hist[0] = i_hist1;
                comp.i_hist[1] = i_hist2;
                comp.flux = flux;
                comp.s_sat = s_sat;
                return comp;
            }
            3 if self.center_tap => {
                // Primary, Sec A (S1-CT), Sec B (CT-S2)
                r[0] = r1;
                r[1] = 0.5 * r2;
                r[2] = 0.5 * r2;
                i_prev[0] = self.i1;
                i_prev[1] = self.i2;
                i_prev[2] = self.i3;

                let l2_half = 0.25 * l2;
                l[0][0] = ((1.0 - k) * l1 + k * l1 * s_sat).max(MIN_L);
                l[1][1] = ((1.0 - k) * l2_half + k * l2_half * s_sat).max(MIN_L);
                l[2][2] = ((1.0 - k) * l2_half + k * l2_half * s_sat).max(MIN_L);

                let m_p_half = 0.5 * k * (l1 * l2).sqrt() * s_sat;
                let m_half_half = k * l2_half * s_sat;

                l[0][1] = m_p_half;
                l[1][0] = m_p_half;
                l[0][2] = m_p_half;
                l[2][0] = m_p_half;
                l[1][2] = m_half_half;
                l[2][1] = m_half_half;
            }
            3 => {
                // Primary, Secondary, Auxiliary
                let l3 = self.inductance3.max(MIN_L);
                let r3 = self.r_coil3.max(MIN_R);

                r[0] = r1;
                r[1] = r2;
                r[2] = r3;
                i_prev[0] = self.i1;
                i_prev[1] = self.i2;
                i_prev[2] = self.i3;

                l[0][0] = ((1.0 - k) * l1 + k * l1 * s_sat).max(MIN_L);
                l[1][1] = ((1.0 - k) * l2 + k * l2 * s_sat).max(MIN_L);
                l[2][2] = ((1.0 - k) * l3 + k * l3 * s_sat).max(MIN_L);

                let m12 = k * (l1 * l2).sqrt() * s_sat;
                let m13 = k * (l1 * l3).sqrt() * s_sat;
                let m23 = k * (l2 * l3).sqrt() * s_sat;

                l[0][1] = m12;
                l[1][0] = m12;
                l[0][2] = m13;
                l[2][0] = m13;
                l[1][2] = m23;
                l[2][1] = m23;
            }
            4 => {
                // Primary, Sec A, Sec B, Auxiliary
                let l3 = self.inductance3.max(MIN_L);
                let r3 = self.r_coil3.max(MIN_R);
                let l2_half = 0.25 * l2;

                r[0] = r1;
                r[1] = 0.5 * r2;
                r[2] = 0.5 * r2;
                r[3] = r3;
                i_prev[0] = self.i1;
                i_prev[1] = self.i2;
                i_prev[2] = self.i3;
                i_prev[3] = 0.0;

                l[0][0] = ((1.0 - k) * l1 + k * l1 * s_sat).max(MIN_L);
                l[1][1] = ((1.0 - k) * l2_half + k * l2_half * s_sat).max(MIN_L);
                l[2][2] = ((1.0 - k) * l2_half + k * l2_half * s_sat).max(MIN_L);
                l[3][3] = ((1.0 - k) * l3 + k * l3 * s_sat).max(MIN_L);

                let m_p_half = 0.5 * k * (l1 * l2).sqrt() * s_sat;
                let m_half_half = k * l2_half * s_sat;
                let m13 = k * (l1 * l3).sqrt() * s_sat;
                let m_half_aux = 0.5 * k * (l2 * l3).sqrt() * s_sat;

                l[0][1] = m_p_half;
                l[1][0] = m_p_half;
                l[0][2] = m_p_half;
                l[2][0] = m_p_half;
                l[1][2] = m_half_half;
                l[2][1] = m_half_half;

                l[0][3] = m13;
                l[3][0] = m13;
                l[1][3] = m_half_aux;
                l[3][1] = m_half_aux;
                l[2][3] = m_half_aux;
                l[3][2] = m_half_aux;
            }
            _ => unreachable!(),
        }

        // Backward Euler impedance matrix: Z_jk = R_k * delta_jk + L_jk / dt
        let mut z = [[0.0; 4]; 4];
        let mut zl = [[0.0; 4]; 4];
        for j in 0..dim {
            for col in 0..dim {
                zl[j][col] = l[j][col] / dt;
                z[j][col] = zl[j][col];
                if j == col {
                    z[j][col] += r[j];
                }
            }
        }

        // History voltages e_j = sum_k (L_jk / dt * i_prev_k)
        let mut e = [0.0; 4];
        for j in 0..dim {
            let mut sum = 0.0;
            for col in 0..dim {
                sum += zl[j][col] * i_prev[col];
            }
            e[j] = sum;
        }

        // Invert Z matrix using Gauss-Jordan elimination
        let g = invert_matrix(z, dim);

        // Calculate companion history currents i_hist = G * e
        let mut i_hist = [0.0; 4];
        for j in 0..dim {
            let mut sum = 0.0;
            for col in 0..dim {
                sum += g[j][col] * e[col];
            }
            i_hist[j] = sum;
        }

        let mut comp = MultiWindingCompanion {
            dim,
            g,
            i_hist,
            flux,
            s_sat,
        };
        // Add core eddy loss conductance to primary winding
        comp.g[0][0] += g_core;

        comp
    }

    /// Stamp into canvas CircMatrix.
    /// Stamp into circuit simulation ENode array.
    pub fn stamp_nodes(&self, cache: &PinCache, nodes: &mut [ENode], dt: f64) {
        let comp = self.companion(dt);
        let dim = comp.dim;

        let p1 = cache.left;
        let p2 = cache.right;
        let s1 = cache.in0;
        let s2 = cache.in1;
        let ct = cache.mid;
        let a1 = cache.collector;
        let a2 = cache.emitter;

        let pairs: [(Option<usize>, Option<usize>); 4] = match dim {
            2 => [(p1, p2), (s1, s2), (None, None), (None, None)],
            3 if self.center_tap => [(p1, p2), (s1, ct), (ct, s2), (None, None)],
            3 => [(p1, p2), (s1, s2), (a1, a2), (None, None)],
            4 => [(p1, p2), (s1, ct), (ct, s2), (a1, a2)],
            _ => return,
        };

        // Stamp self conductances and history currents
        for w in 0..dim {
            let (n_a, n_b) = pairs[w];
            if n_a.is_some() && n_b.is_some() {
                super::stamp::stamp_cached_two_terminal(
                    n_a,
                    n_b,
                    nodes,
                    comp.g[w][w],
                    -comp.i_hist[w],
                );
            }
        }

        // Stamp mutual conductances
        for j in 0..dim {
            for col in (j + 1)..dim {
                let (na1, na2) = pairs[j];
                let (nb1, nb2) = pairs[col];
                let g = comp.g[j][col];
                if na1.is_some() && na2.is_some() && nb1.is_some() && nb2.is_some() && g != 0.0 {
                    super::stamp::stamp_cached_admit_directed(na1, nb1, nodes, -g);
                    super::stamp::stamp_cached_admit_directed(na1, nb2, nodes, g);
                    super::stamp::stamp_cached_admit_directed(na2, nb1, nodes, g);
                    super::stamp::stamp_cached_admit_directed(na2, nb2, nodes, -g);

                    super::stamp::stamp_cached_admit_directed(nb1, na1, nodes, -g);
                    super::stamp::stamp_cached_admit_directed(nb1, na2, nodes, g);
                    super::stamp::stamp_cached_admit_directed(nb2, na1, nodes, g);
                    super::stamp::stamp_cached_admit_directed(nb2, na2, nodes, -g);
                }
            }
        }

        // Prevent floating node singularity on reference pins
        if let Some(n) = p2 {
            if n < nodes.len() {
                nodes[n].add_admit(None, TRANSFORMER_BLEED_ADMIT);
            }
        }
        if let Some(n) = s2 {
            if n < nodes.len() {
                nodes[n].add_admit(None, TRANSFORMER_BLEED_ADMIT);
            }
        }
        if let Some(n) = a2 {
            if n < nodes.len() {
                nodes[n].add_admit(None, TRANSFORMER_BLEED_ADMIT);
            }
        }
    }

    /// Step update at end of transient iteration. Returns true if currents changed.
    pub fn update_step(&mut self, cache: &PinCache, nodes: &[ENode], dt: f64) -> bool {
        let comp = self.companion(dt);
        let dim = comp.dim;

        let p1 = cache.left;
        let p2 = cache.right;
        let s1 = cache.in0;
        let s2 = cache.in1;
        let ct = cache.mid;
        let a1 = cache.collector;
        let a2 = cache.emitter;

        let pairs: [(Option<usize>, Option<usize>); 4] = match dim {
            2 => [(p1, p2), (s1, s2), (None, None), (None, None)],
            3 if self.center_tap => [(p1, p2), (s1, ct), (ct, s2), (None, None)],
            3 => [(p1, p2), (s1, s2), (a1, a2), (None, None)],
            4 => [(p1, p2), (s1, ct), (ct, s2), (a1, a2)],
            _ => return false,
        };

        let mut v = [0.0; 4];
        let mut closed = [false; 4];
        for w in 0..dim {
            let (na, nb) = pairs[w];
            if let (Some(a), Some(b)) = (na, nb) {
                if a < nodes.len() && b < nodes.len() {
                    v[w] = nodes[a].volt - nodes[b].volt;
                    closed[w] = true;
                }
            }
        }

        let mut new_i = [0.0; 4];
        for j in 0..dim {
            if closed[j] {
                let mut sum = comp.i_hist[j];
                for col in 0..dim {
                    if closed[col] {
                        sum += comp.g[j][col] * v[col];
                    }
                }
                new_i[j] = sum;
            }
        }

        let mut changed = false;
        if (self.i1 - new_i[0]).abs() > 1e-8
            || (self.i2 - new_i[1]).abs() > 1e-8
            || (self.i3 - new_i[2]).abs() > 1e-8
        {
            changed = true;
        }

        self.i1 = new_i[0];
        self.i2 = new_i[1];
        self.i3 = new_i[2];

        let i_m = self.magnetizing_current(self.i1, self.i2, self.i3);
        let d_imag = i_m - self.prev_i_mag;
        let d_thresh = (self.i_sat * 1e-5).max(1e-7);
        if d_imag > d_thresh {
            self.h_dir = 1.0;
        } else if d_imag < -d_thresh {
            self.h_dir = -1.0;
        }
        self.prev_i_mag = i_m;
        self.flux = comp.flux;

        changed
    }
}

/// Companion matrix and vector for multi-winding coupled inductors.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiWindingCompanion {
    pub dim: usize,
    pub g: [[f64; 4]; 4],
    pub i_hist: [f64; 4],
    pub flux: f64,
    pub s_sat: f64,
}

/// Inverts a small square matrix (dim <= 4) using Gauss-Jordan elimination with partial pivoting.
fn invert_matrix(mat: [[f64; 4]; 4], dim: usize) -> [[f64; 4]; 4] {
    let mut a = [[0.0; 8]; 4];
    for r in 0..dim {
        for c in 0..dim {
            a[r][c] = mat[r][c];
        }
        a[r][dim + r] = 1.0;
    }

    for i in 0..dim {
        let mut pivot_row = i;
        let mut max_val = a[i][i].abs();
        for r in (i + 1)..dim {
            if a[r][i].abs() > max_val {
                max_val = a[r][i].abs();
                pivot_row = r;
            }
        }

        if pivot_row != i {
            a.swap(i, pivot_row);
        }

        let mut pivot = a[i][i];
        if pivot.abs() < 1e-15 {
            pivot = if pivot >= 0.0 { 1e-12 } else { -1e-12 };
            a[i][i] = pivot;
        }

        let inv_pivot = 1.0 / pivot;
        for c in 0..(2 * dim) {
            a[i][c] *= inv_pivot;
        }

        for r in 0..dim {
            if r != i {
                let factor = a[r][i];
                if factor != 0.0 {
                    for c in 0..(2 * dim) {
                        a[r][c] -= factor * a[i][c];
                    }
                }
            }
        }
    }

    let mut inv = [[0.0; 4]; 4];
    for r in 0..dim {
        for c in 0..dim {
            inv[r][c] = a[r][dim + c];
        }
    }
    inv
}
