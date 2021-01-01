//! Digital arithmetic, counting, and logic function state machines matching
//! C++ `Counter`, `BinCounter`, `FullAdder`, `MagnitudeComp`, `ShiftReg`, `Function`.

use super::family::LogicFamily;
use super::pin::IoPin;
use super::queue::OutQueue;

// ============================================================================
// Simple Clock Counter / Frequency Divider (Counter)
// ============================================================================

#[derive(Clone, Debug)]
pub struct CounterState {
    pub bits: usize,
    pub max_count: u32,
    pub count: u32,
    pub clock: IoPin,
    pub reset: IoPin,
    pub enable: IoPin,
    pub output: IoPin,
    pub outputs: Vec<IoPin>, // Q0..Qn-1 binary outputs
    pub last_clk: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl CounterState {
    pub fn new(id: &str, bits: usize, max_count: u32) -> Self {
        let bits = bits.clamp(1, 16);
        let mut outputs = Vec::with_capacity(bits);
        for i in 0..bits {
            outputs.push(IoPin::output(format!("{id}-q{i}")));
        }
        let clock = IoPin::input(format!("{id}-clk"));
        let reset = IoPin::input(format!("{id}-rst"));
        let enable = IoPin::input(format!("{id}-en"));
        let output = IoPin::output(format!("{id}-out"));

        let mut st = Self {
            bits,
            max_count: if max_count == 0 {
                (1u32 << bits) - 1
            } else {
                max_count
            },
            count: 0,
            clock,
            reset,
            enable,
            output,
            outputs,
            last_clk: false,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        self.family.apply(&mut self.clock);
        self.family.apply(&mut self.reset);
        self.family.apply(&mut self.enable);
        self.family.apply(&mut self.output);
        for pin in &mut self.outputs {
            self.family.apply(pin);
        }
    }

    pub fn eval(&mut self) {
        if self.reset.inp_state() {
            self.count = 0;
            self.output.set_out_state(false);
            for p in &mut self.outputs {
                p.set_out_state(false);
            }
            return;
        }

        let clk = self.clock.inp_state();
        let en = self.enable.inp_state();

        // Rising edge clock
        if clk && !self.last_clk && en {
            if self.count >= self.max_count {
                self.count = 0;
            } else {
                self.count += 1;
            }
        }
        self.last_clk = clk;

        for (i, p) in self.outputs.iter_mut().enumerate() {
            p.set_out_state(((self.count >> i) & 1) != 0);
        }
        self.output.set_out_state(self.count == self.max_count);
    }
}

// ============================================================================
// 4-bit Binary / Decade Counter (BinCounter)
// ============================================================================

#[derive(Clone, Debug)]
pub struct BinCounterState {
    pub is_decade: bool, // 7490 (decade) vs 7493 (4-bit binary)
    pub count_a: u8,
    pub count_b: u8,
    pub clk_a: IoPin,        // Input A
    pub clk_b: IoPin,        // Input B
    pub r0_1: IoPin,         // Reset 0 (1)
    pub r0_2: IoPin,         // Reset 0 (2)
    pub r9_1: IoPin,         // Set 9 (1) - for decade counter
    pub r9_2: IoPin,         // Set 9 (2) - for decade counter
    pub outputs: Vec<IoPin>, // Q0, Q1, Q2, Q3
    pub last_clk_a: bool,
    pub last_clk_b: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl BinCounterState {
    pub fn new(id: &str, is_decade: bool) -> Self {
        let mut outputs = Vec::with_capacity(4);
        for i in 0..4 {
            outputs.push(IoPin::output(format!("{id}-q{i}")));
        }
        let clk_a = IoPin::input(format!("{id}-clka"));
        let clk_b = IoPin::input(format!("{id}-clkb"));
        let r0_1 = IoPin::input(format!("{id}-r0_1"));
        let r0_2 = IoPin::input(format!("{id}-r0_2"));
        let r9_1 = IoPin::input(format!("{id}-r9_1"));
        let r9_2 = IoPin::input(format!("{id}-r9_2"));

        let mut st = Self {
            is_decade,
            count_a: 0,
            count_b: 0,
            clk_a,
            clk_b,
            r0_1,
            r0_2,
            r9_1,
            r9_2,
            outputs,
            last_clk_a: false,
            last_clk_b: false,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        self.family.apply(&mut self.clk_a);
        self.family.apply(&mut self.clk_b);
        self.family.apply(&mut self.r0_1);
        self.family.apply(&mut self.r0_2);
        self.family.apply(&mut self.r9_1);
        self.family.apply(&mut self.r9_2);
        for p in &mut self.outputs {
            self.family.apply(p);
        }
    }

    pub fn eval(&mut self) {
        // Set 9 override (if both R9 inputs are high on decade counter)
        if self.is_decade && self.r9_1.inp_state() && self.r9_2.inp_state() {
            self.count_a = 1;
            self.count_b = 4; // Q3=1, Q2=0, Q1=0 -> binary 1001 (9)
        } else if self.r0_1.inp_state() && self.r0_2.inp_state() {
            self.count_a = 0;
            self.count_b = 0;
        } else {
            // Negative-edge triggered Clock A
            let clk_a = self.clk_a.inp_state();
            if !clk_a && self.last_clk_a {
                self.count_a = (self.count_a + 1) & 1;
            }
            self.last_clk_a = clk_a;

            // Negative-edge triggered Clock B
            let clk_b = self.clk_b.inp_state();
            if !clk_b && self.last_clk_b {
                if self.is_decade {
                    self.count_b = if self.count_b >= 4 {
                        0
                    } else {
                        self.count_b + 1
                    };
                } else {
                    self.count_b = (self.count_b + 1) & 7;
                }
            }
            self.last_clk_b = clk_b;
        }

        let q0 = self.count_a != 0;
        let q1 = (self.count_b & 1) != 0;
        let q2 = ((self.count_b >> 1) & 1) != 0;
        let q3 = ((self.count_b >> 2) & 1) != 0;

        self.outputs[0].set_out_state(q0);
        self.outputs[1].set_out_state(q1);
        self.outputs[2].set_out_state(q2);
        self.outputs[3].set_out_state(q3);
    }
}

// ============================================================================
// Binary Full Adder (FullAdder)
// ============================================================================

#[derive(Clone, Debug)]
pub struct FullAdderState {
    pub bits: usize,
    pub a_inputs: Vec<IoPin>,
    pub b_inputs: Vec<IoPin>,
    pub ci: IoPin, // Carry In
    pub sum_outputs: Vec<IoPin>,
    pub co: IoPin, // Carry Out
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl FullAdderState {
    pub fn new(id: &str, bits: usize) -> Self {
        let bits = bits.clamp(1, 16);
        let mut a_inputs = Vec::with_capacity(bits);
        let mut b_inputs = Vec::with_capacity(bits);
        let mut sum_outputs = Vec::with_capacity(bits);
        for i in 0..bits {
            a_inputs.push(IoPin::input(format!("{id}-a{i}")));
            b_inputs.push(IoPin::input(format!("{id}-b{i}")));
            sum_outputs.push(IoPin::output(format!("{id}-s{i}")));
        }
        let ci = IoPin::input(format!("{id}-ci"));
        let co = IoPin::output(format!("{id}-co"));

        let mut st = Self {
            bits,
            a_inputs,
            b_inputs,
            ci,
            sum_outputs,
            co,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.a_inputs {
            self.family.apply(p);
        }
        for p in &mut self.b_inputs {
            self.family.apply(p);
        }
        self.family.apply(&mut self.ci);
        for p in &mut self.sum_outputs {
            self.family.apply(p);
        }
        self.family.apply(&mut self.co);
    }

    pub fn eval(&mut self) {
        let mut carry = self.ci.inp_state();
        for i in 0..self.bits {
            let a = self.a_inputs[i].inp_state();
            let b = self.b_inputs[i].inp_state();
            let sum = (a ^ b) ^ carry;
            carry = (a && b) || (carry && (a ^ b));
            self.sum_outputs[i].set_out_state(sum);
        }
        self.co.set_out_state(carry);
    }
}

// ============================================================================
// Magnitude Comparator (MagnitudeComp)
// ============================================================================

#[derive(Clone, Debug)]
pub struct MagnitudeCompState {
    pub bits: usize,
    pub a_inputs: Vec<IoPin>,
    pub b_inputs: Vec<IoPin>,
    pub cascade_gt: IoPin, // A > B cascade input
    pub cascade_eq: IoPin, // A = B cascade input
    pub cascade_lt: IoPin, // A < B cascade input
    pub out_gt: IoPin,     // A > B output
    pub out_eq: IoPin,     // A = B output
    pub out_lt: IoPin,     // A < B output
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl MagnitudeCompState {
    pub fn new(id: &str, bits: usize) -> Self {
        let bits = bits.clamp(1, 16);
        let mut a_inputs = Vec::with_capacity(bits);
        let mut b_inputs = Vec::with_capacity(bits);
        for i in 0..bits {
            a_inputs.push(IoPin::input(format!("{id}-a{i}")));
            b_inputs.push(IoPin::input(format!("{id}-b{i}")));
        }
        let cascade_gt = IoPin::input(format!("{id}-in_gt"));
        let cascade_eq = IoPin::input(format!("{id}-in_eq"));
        let cascade_lt = IoPin::input(format!("{id}-in_lt"));

        let out_gt = IoPin::output(format!("{id}-out_gt"));
        let out_eq = IoPin::output(format!("{id}-out_eq"));
        let out_lt = IoPin::output(format!("{id}-out_lt"));

        let mut st = Self {
            bits,
            a_inputs,
            b_inputs,
            cascade_gt,
            cascade_eq,
            cascade_lt,
            out_gt,
            out_eq,
            out_lt,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.a_inputs {
            self.family.apply(p);
        }
        for p in &mut self.b_inputs {
            self.family.apply(p);
        }
        self.family.apply(&mut self.cascade_gt);
        self.family.apply(&mut self.cascade_eq);
        self.family.apply(&mut self.cascade_lt);
        self.family.apply(&mut self.out_gt);
        self.family.apply(&mut self.out_eq);
        self.family.apply(&mut self.out_lt);
    }

    pub fn eval(&mut self) {
        let mut a_val = 0u32;
        let mut b_val = 0u32;
        for (i, p) in self.a_inputs.iter().enumerate() {
            if p.inp_state() {
                a_val |= 1 << i;
            }
        }
        for (i, p) in self.b_inputs.iter().enumerate() {
            if p.inp_state() {
                b_val |= 1 << i;
            }
        }

        let (gt, eq, lt) = if a_val > b_val {
            (true, false, false)
        } else if a_val < b_val {
            (false, false, true)
        } else {
            // Equal: follow cascade inputs
            let c_gt = self.cascade_gt.inp_state();
            let c_eq = self.cascade_eq.inp_state();
            let c_lt = self.cascade_lt.inp_state();
            if c_gt && !c_lt {
                (true, false, false)
            } else if c_lt && !c_gt {
                (false, false, true)
            } else {
                (false, c_eq, false)
            }
        };

        self.out_gt.set_out_state(gt);
        self.out_eq.set_out_state(eq);
        self.out_lt.set_out_state(lt);
    }
}

// ============================================================================
// Shift Register (ShiftReg)
// ============================================================================

#[derive(Clone, Debug)]
pub struct ShiftRegState {
    pub bits: usize,
    pub shift_reg: u32,
    pub latch_reg: u32,
    pub ser_in: IoPin,       // Serial data input (DS)
    pub clk_shift: IoPin,    // Shift clock (SH_CP)
    pub clk_latch: IoPin,    // Storage / Latch clock (ST_CP)
    pub master_reset: IoPin, // Reset (MR, active low)
    pub oe: IoPin,           // Output enable (OE, active low)
    pub outputs: Vec<IoPin>, // Q0..Q7 parallel outputs
    pub ser_out: IoPin,      // Serial cascade output (Q7')
    pub last_clk_shift: bool,
    pub last_clk_latch: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl ShiftRegState {
    pub fn new(id: &str, bits: usize) -> Self {
        let bits = bits.clamp(1, 32);
        let mut outputs = Vec::with_capacity(bits);
        for i in 0..bits {
            outputs.push(IoPin::output(format!("{id}-q{i}")));
        }
        let ser_in = IoPin::input(format!("{id}-ds"));
        let clk_shift = IoPin::input(format!("{id}-sh_cp"));
        let clk_latch = IoPin::input(format!("{id}-st_cp"));
        let master_reset = IoPin::input(format!("{id}-mr"));
        let oe = IoPin::input(format!("{id}-oe"));
        let ser_out = IoPin::output(format!("{id}-q7s"));

        let mut st = Self {
            bits,
            shift_reg: 0,
            latch_reg: 0,
            ser_in,
            clk_shift,
            clk_latch,
            master_reset,
            oe,
            outputs,
            ser_out,
            last_clk_shift: false,
            last_clk_latch: false,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        self.family.apply(&mut self.ser_in);
        self.family.apply(&mut self.clk_shift);
        self.family.apply(&mut self.clk_latch);
        self.family.apply(&mut self.master_reset);
        self.family.apply(&mut self.oe);
        for p in &mut self.outputs {
            self.family.apply(p);
        }
        self.family.apply(&mut self.ser_out);
    }

    pub fn eval(&mut self) {
        // Master reset: active low
        if !self.master_reset.inp_state() {
            self.shift_reg = 0;
        } else {
            let clk_s = self.clk_shift.inp_state();
            if clk_s && !self.last_clk_shift {
                let bit = if self.ser_in.inp_state() { 1 } else { 0 };
                let mask = (1u32 << self.bits) - 1;
                self.shift_reg = ((self.shift_reg << 1) | bit) & mask;
            }
            self.last_clk_shift = clk_s;
        }

        // Latch clock
        let clk_l = self.clk_latch.inp_state();
        if clk_l && !self.last_clk_latch {
            self.latch_reg = self.shift_reg;
        }
        self.last_clk_latch = clk_l;

        let oe_active = !self.oe.inp_state(); // active-low OE
        for (i, p) in self.outputs.iter_mut().enumerate() {
            p.set_out_state(oe_active && (((self.latch_reg >> i) & 1) != 0));
        }
        self.ser_out
            .set_out_state(((self.shift_reg >> (self.bits - 1)) & 1) != 0);
    }
}

// ============================================================================
// Logic Function Evaluator (Function)
// ============================================================================

#[derive(Clone, Debug)]
pub struct FunctionState {
    pub expression: String,
    pub inputs: Vec<IoPin>,
    pub output: IoPin,
    pub n_inputs: usize,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl FunctionState {
    pub fn new(id: &str, n_inputs: usize, expression: &str) -> Self {
        let n_inputs = n_inputs.clamp(1, 8);
        let mut inputs = Vec::with_capacity(n_inputs);
        for i in 0..n_inputs {
            let ch = (b'A' + i as u8) as char;
            inputs.push(IoPin::input(format!("{id}-{ch}")));
        }
        let output = IoPin::output(format!("{id}-out"));

        let mut st = Self {
            expression: expression.to_string(),
            inputs,
            output,
            n_inputs,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.inputs {
            self.family.apply(p);
        }
        self.family.apply(&mut self.output);
    }

    pub fn eval(&mut self) -> bool {
        let mut vars = [false; 8];
        for (i, p) in self.inputs.iter().enumerate() {
            if i < 8 {
                vars[i] = p.inp_state();
            }
        }
        let res = eval_boolean_expr(&self.expression, &vars);
        self.output.set_out_state(res);
        res
    }
}

/// Simple Boolean expression evaluator supporting A..H, !, ~, &, |, ^, (, )
fn eval_boolean_expr(expr: &str, vars: &[bool; 8]) -> bool {
    let mut clean = String::new();
    for c in expr.chars() {
        if !c.is_whitespace() {
            clean.push(c);
        }
    }
    if clean.is_empty() {
        return vars[0];
    }
    parse_or(&clean, vars).0
}

fn parse_or(s: &str, vars: &[bool; 8]) -> (bool, usize) {
    let (mut left, mut idx) = parse_xor(s, vars);
    while idx < s.len() && (s.as_bytes()[idx] == b'|' || s.as_bytes()[idx] == b'+') {
        let (right, next_idx) = parse_xor(&s[idx + 1..], vars);
        left = left || right;
        idx += 1 + next_idx;
    }
    (left, idx)
}

fn parse_xor(s: &str, vars: &[bool; 8]) -> (bool, usize) {
    let (mut left, mut idx) = parse_and(s, vars);
    while idx < s.len() && s.as_bytes()[idx] == b'^' {
        let (right, next_idx) = parse_and(&s[idx + 1..], vars);
        left = left ^ right;
        idx += 1 + next_idx;
    }
    (left, idx)
}

fn parse_and(s: &str, vars: &[bool; 8]) -> (bool, usize) {
    let (mut left, mut idx) = parse_primary(s, vars);
    while idx < s.len() && (s.as_bytes()[idx] == b'&' || s.as_bytes()[idx] == b'*') {
        let (right, next_idx) = parse_primary(&s[idx + 1..], vars);
        left = left && right;
        idx += 1 + next_idx;
    }
    (left, idx)
}

fn parse_primary(s: &str, vars: &[bool; 8]) -> (bool, usize) {
    if s.is_empty() {
        return (false, 0);
    }
    let b = s.as_bytes()[0];
    if b == b'!' || b == b'~' {
        let (val, idx) = parse_primary(&s[1..], vars);
        (!val, 1 + idx)
    } else if b == b'(' {
        let (val, idx) = parse_or(&s[1..], vars);
        let end = if idx + 1 < s.len() && s.as_bytes()[idx + 1] == b')' {
            idx + 2
        } else {
            idx + 1
        };
        (val, end)
    } else if b >= b'A' && b <= b'H' {
        let var_idx = (b - b'A') as usize;
        (vars[var_idx], 1)
    } else if b >= b'a' && b <= b'h' {
        let var_idx = (b - b'a') as usize;
        (vars[var_idx], 1)
    } else if b == b'1' {
        (true, 1)
    } else {
        (false, 1)
    }
}
