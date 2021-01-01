//! Admittance-matrix solver, ported from `src/simulator/circmatrix.cpp`.
//!
//! Islands of interconnected nodes are grouped. Size-1 groups solve as
//! `V = I / G`. Size-2 groups use the closed-form 2×2. Larger groups use
//! Crout LU.

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

    /// Solve into `volts`. Returns `false` if any group was singular.
    pub fn solve(&mut self, volts: &mut [f64]) -> bool {
        debug_assert_eq!(volts.len(), self.n);
        let mut ok = true;

        for &i in &self.singles {
            let g = self.a[i * self.n + i];
            volts[i] = if g > 0.0 { self.b[i] / g } else { 0.0 };
        }

        for g_idx in 0..self.groups.len() {
            let n = self.groups[g_idx].nodes.len();
            if n == 2 {
                let i0 = self.groups[g_idx].nodes[0];
                let i1 = self.groups[g_idx].nodes[1];
                let a00 = self.a[i0 * self.n + i0];
                let a01 = self.a[i0 * self.n + i1];
                let a10 = self.a[i1 * self.n + i0];
                let a11 = self.a[i1 * self.n + i1];
                let det = a00 * a11 - a01 * a10;
                if det == 0.0 {
                    ok = false;
                } else {
                    let bi0 = self.b[i0];
                    let bi1 = self.b[i1];
                    let b0 = bi0 * a11 - a01 * bi1;
                    let b1 = a00 * bi1 - bi0 * a10;
                    volts[i0] = b0 / det;
                    volts[i1] = b1 / det;
                }
            } else if n == 3 {
                let i0 = self.groups[g_idx].nodes[0];
                let i1 = self.groups[g_idx].nodes[1];
                let i2 = self.groups[g_idx].nodes[2];

                let a00 = self.a[i0 * self.n + i0];
                let a01 = self.a[i0 * self.n + i1];
                let a02 = self.a[i0 * self.n + i2];
                let a10 = self.a[i1 * self.n + i0];
                let a11 = self.a[i1 * self.n + i1];
                let a12 = self.a[i1 * self.n + i2];
                let a20 = self.a[i2 * self.n + i0];
                let a21 = self.a[i2 * self.n + i1];
                let a22 = self.a[i2 * self.n + i2];

                let c00 = a11 * a22 - a12 * a21;
                let c01 = a12 * a20 - a10 * a22;
                let c02 = a10 * a21 - a11 * a20;

                let det = a00 * c00 + a01 * c01 + a02 * c02;
                if det == 0.0 {
                    ok = false;
                } else {
                    let c10 = a02 * a21 - a01 * a22;
                    let c11 = a00 * a22 - a02 * a20;
                    let c12 = a01 * a20 - a00 * a21;

                    let c20 = a01 * a12 - a02 * a11;
                    let c21 = a02 * a10 - a00 * a12;
                    let c22 = a00 * a11 - a01 * a10;

                    let b0 = self.b[i0];
                    let b1 = self.b[i1];
                    let b2 = self.b[i2];

                    volts[i0] = (b0 * c00 + b1 * c10 + b2 * c20) / det;
                    volts[i1] = (b0 * c01 + b1 * c11 + b2 * c21) / det;
                    volts[i2] = (b0 * c02 + b1 * c12 + b2 * c22) / det;
                }
            } else {
                let group_nodes = &self.groups[g_idx].nodes;
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
                    group_nodes,
                    volts,
                    &mut self.scratch_a,
                    &mut self.scratch_b,
                ) {
                    ok = false;
                }
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
            if div != 0.0 {
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

    let mut is_ok = true;
    for i in (0..n).rev() {
        let mut tot = b[i];
        for j in i + 1..n {
            tot -= scratch_a[i * n + j] * b[j];
        }
        let div = scratch_a[i * n + i];
        let volt = if div != 0.0 {
            tot / div
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
    fn singular_2x2() {
        let mut m = CircMatrix::new(2);
        m.analyze(&[vec![1], vec![0]]);
        m.stamp_diagonal(0, 1.0);
        m.stamp_matrix(0, 1, -1.0);
        m.stamp_diagonal(1, 1.0);
        m.stamp_matrix(1, 0, -1.0);
        m.stamp_coef(0, 1.0);
        m.stamp_coef(1, -1.0);
        let mut v = vec![0.0; 2];
        assert!(!m.solve(&mut v));
    }
}
