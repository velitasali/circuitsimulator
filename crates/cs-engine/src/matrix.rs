//! Admittance-matrix solver, ported from `src/simulator/circmatrix.cpp`.
//!
//! Islands of interconnected nodes are grouped. Size-1 groups solve as
//! `V = I / G`. Larger groups use Crout LU. A floating group has a zero
//! pivot: one node is held at 0 V and the other voltages are solved. A zero
//! pivot with a nonzero residual is singular.
//!
//! An expanded 2×2 or 3×3 determinant is not used. It cancels when a stiff
//! companion (`C/dt`) shares a group with a tiny conductance to ground, and
//! the leftover microvolts flip a threshold device on and off forever.

#[derive(Clone, Debug)]
struct Group {
    nodes: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct CircMatrix {
    n: usize,
    a: Vec<f64>,
    b: Vec<f64>,
    groups: Vec<Group>,
    singles: Vec<usize>,
    scratch_a: Vec<f64>,
    scratch_b: Vec<f64>,
}

impl CircMatrix {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            a: vec![0.0; n * n],
            b: vec![0.0; n],
            groups: Vec::new(),
            singles: Vec::new(),
            scratch_a: Vec::new(),
            scratch_b: Vec::new(),
        }
    }

    pub fn clear_stamps(&mut self) {
        self.a.fill(0.0);
        self.b.fill(0.0);
    }

    /// Split the node graph into islands. `connections[i]` is the list of
    /// other node indices that share a component with `i`.
    pub fn analyze(&mut self, connections: &[Vec<usize>]) {
        debug_assert_eq!(connections.len(), self.n);
        self.groups.clear();
        self.singles.clear();

        let mut visited = vec![false; self.n];
        let mut queue = Vec::with_capacity(self.n);

        for start in 0..self.n {
            if visited[start] {
                continue;
            }
            visited[start] = true;
            let mut group = Vec::new();
            queue.clear();
            queue.push(start);

            while let Some(curr) = queue.pop() {
                group.push(curr);
                if curr < connections.len() {
                    for &next in &connections[curr] {
                        if next < self.n && !visited[next] {
                            visited[next] = true;
                            queue.push(next);
                        }
                    }
                }
            }

            if group.len() == 1 {
                self.singles.push(group[0]);
            } else {
                self.groups.push(Group { nodes: group });
            }
        }
    }

    #[inline]
    pub fn stamp_diagonal(&mut self, n: usize, value: f64) {
        if n < self.n {
            self.a[n * self.n + n] = value;
        }
    }

    #[inline]
    pub fn stamp_matrix(&mut self, row: usize, col: usize, value: f64) {
        if row < self.n && col < self.n {
            self.a[row * self.n + col] = value;
        }
    }

    #[inline]
    pub fn stamp_coef(&mut self, row: usize, value: f64) {
        if row < self.n {
            self.b[row] = value;
        }
    }

    pub fn n(&self) -> usize {
        self.n
    }

    /// Accumulate a matrix entry. Component stamps add; [`Self::stamp_matrix`]
    /// overwrites (used when eNodes have already summed admitances).
    #[inline]
    pub fn add_matrix(&mut self, row: usize, col: usize, value: f64) {
        if row < self.n && col < self.n {
            self.a[row * self.n + col] += value;
        }
    }

    #[inline]
    pub fn add_coef(&mut self, row: usize, value: f64) {
        if row < self.n {
            self.b[row] += value;
        }
    }

    /// Solve into `volts`. Returns `false` if any group has no solution.
    ///
    /// A group with no path to the implicit ground is still solved. One node
    /// in that group is the 0 V reference, the same rule as a zero pivot in
    /// the LU factorization.
    pub fn solve(&mut self, volts: &mut [f64]) -> bool {
        debug_assert_eq!(volts.len(), self.n);
        let mut ok = true;

        for &i in &self.singles {
            let g = self.a[i * self.n + i];
            volts[i] = if g > 0.0 { self.b[i] / g } else { 0.0 };
        }

        for group in &self.groups {
            let n = group.nodes.len();
            if self.scratch_a.len() < n * n {
                self.scratch_a.resize(n * n, 0.0);
            }
            if self.scratch_b.len() < n {
                self.scratch_b.resize(n, 0.0);
            }
            if !lu_group(
                &self.a,
                self.n,
                &self.b,
                &group.nodes,
                volts,
                &mut self.scratch_a,
                &mut self.scratch_b,
            ) {
                ok = false;
            }
        }
        ok
    }

    pub fn group_sizes(&self) -> Vec<usize> {
        self.groups.iter().map(|g| g.nodes.len()).collect()
    }

    pub fn single_count(&self) -> usize {
        self.singles.len()
    }
}

/// Crout factorization + forward/back substitution, same arithmetic as
/// `CircMatrix::factorMatrix` / `luSolve`.
fn lu_group(
    a_full: &[f64],
    n_full: usize,
    b_full: &[f64],
    group: &[usize],
    volts: &mut [f64],
    scratch_a: &mut [f64],
    scratch_b: &mut [f64],
) -> bool {
    let n = group.len();
    for (row, &gi) in group.iter().enumerate() {
        for (col, &gj) in group.iter().enumerate() {
            scratch_a[row * n + col] = a_full[gi * n_full + gj];
        }
    }

    // factorMatrix
    for col in 0..n {
        for row in 0..col {
            let mut q = scratch_a[row * n + col];
            for k in 0..row {
                q -= scratch_a[row * n + k] * scratch_a[k * n + col];
            }
            scratch_a[row * n + col] = q;
        }
        for row in col..n {
            let mut q = scratch_a[row * n + col];
            for k in 0..col {
                q -= scratch_a[row * n + k] * scratch_a[k * n + col];
            }
            scratch_a[row * n + col] = q;
        }
        if col != n - 1 {
            let div = scratch_a[col * n + col];
            let diag = a_full[group[col] * n_full + group[col]].abs();
            if div.abs() > 16.0 * f64::EPSILON * diag {
                for row in col + 1..n {
                    scratch_a[row * n + col] /= div;
                }
            }
        }
    }

    // luSolve
    let b = &mut scratch_b[..n];
    let mut i = 0;
    while i < n {
        let tot = b_full[group[i]];
        b[i] = tot;
        if tot != 0.0 {
            break;
        }
        i += 1;
    }
    let bi = i;
    i += 1;
    while i < n {
        let mut tot = b_full[group[i]];
        for j in bi..i {
            tot -= scratch_a[i * n + j] * b[j];
        }
        b[i] = tot;
        i += 1;
    }

    let max_b = group
        .iter()
        .map(|&gi| b_full[gi].abs())
        .fold(0.0_f64, f64::max);
    let rhs_tol = 64.0 * f64::EPSILON * max_b;

    let mut is_ok = true;
    for i in (0..n).rev() {
        let mut tot = b[i];
        for j in i + 1..n {
            tot -= scratch_a[i * n + j] * b[j];
        }
        let div = scratch_a[i * n + i];
        let diag = a_full[group[i] * n_full + group[i]].abs();
        let pivot_tol = 16.0 * f64::EPSILON * diag;
        // A zero pivot with a zero residual is a free reference. Hold that
        // node at 0 V. A nonzero residual has no solution.
        let volt = if div.abs() > pivot_tol {
            tot / div
        } else if tot.abs() <= rhs_tol {
            0.0
        } else {
            is_ok = false;
            0.0
        };
        b[i] = volt;
        volts[group[i]] = volt;
    }
    is_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn closed_form_2x2() {
        let mut m = CircMatrix::new(2);
        m.analyze(&[vec![1], vec![0]]);
        m.stamp_diagonal(0, 2.0);
        m.stamp_matrix(0, 1, 1.0);
        m.stamp_diagonal(1, 2.0);
        m.stamp_matrix(1, 0, 1.0);
        m.stamp_coef(0, 3.0);
        m.stamp_coef(1, 3.0);
        let mut v = vec![0.0; 2];
        assert!(m.solve(&mut v));
        approx(v[0], 1.0);
        approx(v[1], 1.0);
        assert_eq!(m.group_sizes(), vec![2]);
    }

    #[test]
    fn identity_3x3_lu() {
        let mut m = CircMatrix::new(3);
        m.analyze(&[vec![1], vec![0, 2], vec![1]]);
        for i in 0..3 {
            m.stamp_diagonal(i, 1.0);
        }
        m.stamp_coef(0, 1.0);
        m.stamp_coef(1, 2.0);
        m.stamp_coef(2, 3.0);
        let mut v = vec![0.0; 3];
        assert!(m.solve(&mut v));
        approx(v[0], 1.0);
        approx(v[1], 2.0);
        approx(v[2], 3.0);
        assert_eq!(m.group_sizes(), vec![3]);
    }

    #[test]
    fn resistor_chain_lu() {
        // 1 Ω chain: I=1 A into node 0, node 3 grounded with G=1e9.
        // Three series 1 Ω + ground → voltages 3, 2, 1, ~0.
        let mut m = CircMatrix::new(4);
        m.analyze(&[vec![1], vec![0, 2], vec![1, 3], vec![2]]);
        // node 0: G=1 to node 1, I=1
        m.stamp_diagonal(0, 1.0);
        m.stamp_matrix(0, 1, -1.0);
        m.stamp_coef(0, 1.0);
        // node 1: G=1 to 0 and 2
        m.stamp_diagonal(1, 2.0);
        m.stamp_matrix(1, 0, -1.0);
        m.stamp_matrix(1, 2, -1.0);
        // node 2: G=1 to 1 and 3
        m.stamp_diagonal(2, 2.0);
        m.stamp_matrix(2, 1, -1.0);
        m.stamp_matrix(2, 3, -1.0);
        // node 3: G=1 to 2 plus 1e9 to gnd
        m.stamp_diagonal(3, 1.0 + 1e9);
        m.stamp_matrix(3, 2, -1.0);
        let mut v = vec![0.0; 4];
        assert!(m.solve(&mut v));
        // Ground G=1e9 vs 1 Ω chain: voltages sit a few pV above the ideal 3/2/1/0.
        assert!((v[0] - 3.0).abs() < 1e-8, "{}", v[0]);
        assert!((v[1] - 2.0).abs() < 1e-8, "{}", v[1]);
        assert!((v[2] - 1.0).abs() < 1e-8, "{}", v[2]);
        assert!(v[3].abs() < 1e-8, "{}", v[3]);
        assert_eq!(m.group_sizes(), vec![4]);
    }

    #[test]
    fn single_node() {
        let mut m = CircMatrix::new(1);
        m.analyze(&[vec![]]);
        m.stamp_diagonal(0, 1e9);
        m.stamp_coef(0, 5.0 * 1e9);
        let mut v = vec![0.0];
        assert!(m.solve(&mut v));
        approx(v[0], 5.0);
        assert_eq!(m.single_count(), 1);
    }

    #[test]
    fn floating_2x2_uses_one_node_as_reference() {
        let mut m = CircMatrix::new(2);
        m.analyze(&[vec![1], vec![0]]);
        m.stamp_diagonal(0, 1.0);
        m.stamp_matrix(0, 1, -1.0);
        m.stamp_diagonal(1, 1.0);
        m.stamp_matrix(1, 0, -1.0);
        m.stamp_coef(0, 1.0);
        m.stamp_coef(1, -1.0);
        let mut v = vec![0.0; 2];
        assert!(m.solve(&mut v));
        approx(v[0], 1.0);
        approx(v[1], 0.0);
    }

    #[test]
    fn inconsistent_2x2_is_singular() {
        let mut m = CircMatrix::new(2);
        m.analyze(&[vec![1], vec![0]]);
        m.stamp_diagonal(0, 1.0);
        m.stamp_matrix(0, 1, -1.0);
        m.stamp_diagonal(1, 1.0);
        m.stamp_matrix(1, 0, -1.0);
        m.stamp_coef(0, 1.0);
        m.stamp_coef(1, 1.0);
        let mut v = vec![0.0; 2];
        assert!(!m.solve(&mut v));
    }

    #[test]
    fn stiff_led_knee_stays_above_threshold() {
        // capacitor.circ1, right island, one step before the green LED should
        // turn off. G_c = 1 mF / 1 µs. The scope's 10 MΩ is the only path to
        // ground. The expanded 3×3 determinant reported Vled = 3.499998 V
        // here, which the piecewise LED reads as off, and the next iteration
        // turns it back on.
        let mut m = CircMatrix::new(3);
        m.analyze(&[vec![1, 2], vec![0, 2], vec![0, 1]]);
        let g_c = 1000.0;
        let v_prev = 3.500926;
        let g_r = 0.01;
        let g_scope = 1e-7;
        let g_led = 10.0;
        let i_led = 35.0;
        m.add_matrix(0, 0, g_c);
        m.add_matrix(2, 2, g_c);
        m.add_matrix(0, 2, -g_c);
        m.add_matrix(2, 0, -g_c);
        m.add_coef(0, v_prev * g_c);
        m.add_coef(2, -v_prev * g_c);
        m.add_matrix(0, 0, g_r);
        m.add_matrix(2, 2, g_r);
        m.add_matrix(0, 2, -g_r);
        m.add_matrix(2, 0, -g_r);
        m.add_matrix(0, 0, g_r);
        m.add_matrix(1, 1, g_r);
        m.add_matrix(0, 1, -g_r);
        m.add_matrix(1, 0, -g_r);
        m.add_matrix(0, 0, g_scope);
        m.add_matrix(1, 1, g_scope);
        m.add_matrix(1, 1, g_led);
        m.add_matrix(2, 2, g_led);
        m.add_matrix(1, 2, -g_led);
        m.add_matrix(2, 1, -g_led);
        m.add_coef(1, i_led);
        m.add_coef(2, -i_led);
        let mut v = vec![0.0; 3];
        assert!(m.solve(&mut v));
        let v_led = v[1] - v[2];
        assert!(
            v_led > 3.5,
            "forward companion must stay above 3.5 V, got {v_led}"
        );
    }

    #[test]
    fn floating_3x3_chain() {
        // 1 A through two series 1 Ω, no path to ground. Last node is 0 V.
        let mut m = CircMatrix::new(3);
        m.analyze(&[vec![1], vec![0, 2], vec![1]]);
        m.stamp_diagonal(0, 1.0);
        m.stamp_matrix(0, 1, -1.0);
        m.stamp_coef(0, 1.0);
        m.stamp_diagonal(1, 2.0);
        m.stamp_matrix(1, 0, -1.0);
        m.stamp_matrix(1, 2, -1.0);
        m.stamp_diagonal(2, 1.0);
        m.stamp_matrix(2, 1, -1.0);
        m.stamp_coef(2, -1.0);
        let mut v = vec![0.0; 3];
        assert!(m.solve(&mut v));
        approx(v[0], 2.0);
        approx(v[1], 1.0);
        approx(v[2], 0.0);
    }

    #[test]
    fn floating_3x3_triangle_with_ulp_residual() {
        // Floating resistor triangle (G01 = 1, G02 = 2, G12 = 1) with a 0.7 A
        // source between nodes 0 and 1. Elimination by 1/3 leaves a 1-ULP
        // residual (~5.55e-17) on the zero-pivot row that must still solve.
        let mut m = CircMatrix::new(3);
        m.analyze(&[vec![1, 2], vec![0, 2], vec![0, 1]]);
        m.stamp_diagonal(0, 3.0);
        m.stamp_matrix(0, 1, -1.0);
        m.stamp_matrix(0, 2, -2.0);
        m.stamp_coef(0, 0.7);
        m.stamp_diagonal(1, 2.0);
        m.stamp_matrix(1, 0, -1.0);
        m.stamp_matrix(1, 2, -1.0);
        m.stamp_coef(1, -0.7);
        m.stamp_diagonal(2, 3.0);
        m.stamp_matrix(2, 0, -2.0);
        m.stamp_matrix(2, 1, -1.0);
        m.stamp_coef(2, 0.0);
        let mut v = vec![0.0; 3];
        assert!(m.solve(&mut v));
        // Equivalent conductance between 0 and 1 is 1 + (2*1)/(2+1) = 5/3 S,
        // so V0 - V1 = 0.7 / (5/3) = 0.42 V, and V2 - V1 = (2/3) * 0.42 = 0.28 V.
        approx(v[0] - v[1], 0.42);
        approx(v[2] - v[1], 0.28);
        approx(v[1], 0.0);
    }
}
