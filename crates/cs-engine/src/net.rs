//! Electrical nets (C++ `eNode`) and union-find over pin IDs.

use std::collections::HashMap;

const MAX_NODE_ADMIT: usize = 8;

#[derive(Clone, Debug)]
pub struct ENode {
    #[allow(dead_code)]
    pub id: String,
    pub num: usize,
    pub volt: f64,
    total_admit: f64,
    total_curr: f64,
    admit_entries: [(usize, f64); MAX_NODE_ADMIT],
    admit_count: usize,
    extra_admit: Vec<(usize, f64)>,
    pub pin_ids: Vec<String>,
}

impl ENode {
    pub fn new(id: String, num: usize) -> Self {
        Self {
            id,
            num,
            volt: 0.0,
            total_admit: 0.0,
            total_curr: 0.0,
            admit_entries: [(0, 0.0); MAX_NODE_ADMIT],
            admit_count: 0,
            extra_admit: Vec::new(),
            pin_ids: Vec::new(),
        }
    }

    #[inline]
    pub fn add_admit(&mut self, other: Option<usize>, g: f64) {
        self.total_admit += g;
        if let Some(n) = other {
            for i in 0..self.admit_count {
                if self.admit_entries[i].0 == n {
                    self.admit_entries[i].1 += g;
                    return;
                }
            }
            if self.admit_count < MAX_NODE_ADMIT {
                self.admit_entries[self.admit_count] = (n, g);
                self.admit_count += 1;
                return;
            }
            if let Some(entry) = self.extra_admit.iter_mut().find(|(idx, _)| *idx == n) {
                entry.1 += g;
            } else {
                self.extra_admit.push((n, g));
            }
        }
    }

    #[inline]
    pub fn add_current(&mut self, i: f64) {
        self.total_curr += i;
    }

    #[inline]
    pub fn clear_stamps(&mut self) {
        self.volt = 0.0;
        self.total_admit = 0.0;
        self.total_curr = 0.0;
        self.admit_count = 0;
        if !self.extra_admit.is_empty() {
            self.extra_admit.clear();
        }
    }

    pub fn connections(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(self.admit_count + self.extra_admit.len());
        for i in 0..self.admit_count {
            v.push(self.admit_entries[i].0);
        }
        for &(idx, _) in &self.extra_admit {
            v.push(idx);
        }
        v
    }

    #[inline]
    pub fn stamp_into(&self, matrix: &mut crate::matrix::CircMatrix) {
        matrix.stamp_diagonal(self.num, self.total_admit);
        for i in 0..self.admit_count {
            let (other, g) = self.admit_entries[i];
            matrix.stamp_matrix(self.num, other, -g);
        }
        for &(other, g) in &self.extra_admit {
            matrix.stamp_matrix(self.num, other, -g);
        }
        matrix.stamp_coef(self.num, self.total_curr);
    }
}

#[derive(Default)]
pub struct UnionFind {
    parent: HashMap<String, String>,
}

impl UnionFind {
    pub fn add(&mut self, x: &str) {
        self.parent
            .entry(x.to_string())
            .or_insert_with(|| x.to_string());
    }

    pub fn contains(&self, x: &str) -> bool {
        self.parent.contains_key(x)
    }

    pub fn find(&mut self, x: &str) -> String {
        let p = self.parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        if p != x {
            let root = self.find(&p);
            self.parent.insert(x.to_string(), root.clone());
            root
        } else {
            p
        }
    }

    pub fn union(&mut self, a: &str, b: &str) {
        self.add(a);
        self.add(b);
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent.insert(ra, rb);
        }
    }

    pub fn groups(&mut self) -> Vec<Vec<String>> {
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for k in keys {
            let r = self.find(&k);
            map.entry(r).or_default().push(k);
        }
        map.into_values().collect()
    }
}
