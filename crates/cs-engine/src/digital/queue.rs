//! C++ `IoComponent::scheduleOutPuts` / `runOutputs`.

use std::collections::VecDeque;

/// Result of scheduling a new output word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// Delay is 0: bits were applied immediately.
    Immediate,
    /// First pending word: caller should `addEvent(delay)`.
    Arm { delay_ps: u64 },
    /// Extra word queued behind an already-armed event.
    Queued,
    /// Same as the last armed/queued value; nothing to do.
    Unchanged,
}

#[derive(Clone, Debug, Default)]
pub struct OutQueue {
    pub out_value: u32,
    pub next_out_val: u32,
    out_queue: VecDeque<u32>,
    time_queue: VecDeque<u64>,
}

impl OutQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.out_value = 0;
        self.next_out_val = 0;
        self.out_queue.clear();
        self.time_queue.clear();
    }

    pub fn schedule(&mut self, delay_ps: u64, circ_time: u64) -> Schedule {
        if delay_ps == 0 {
            if self.next_out_val == self.out_value {
                return Schedule::Unchanged;
            }
            self.out_value = self.next_out_val;
            return Schedule::Immediate;
        }
        if self.out_queue.is_empty() {
            if self.next_out_val == self.out_value {
                return Schedule::Unchanged;
            }
            self.out_queue.push_back(self.next_out_val);
            return Schedule::Arm { delay_ps };
        }
        if self.next_out_val == *self.out_queue.back().unwrap() {
            return Schedule::Unchanged;
        }
        self.time_queue
            .push_back(circ_time.saturating_add(delay_ps));
        self.out_queue.push_back(self.next_out_val);
        Schedule::Queued
    }

    /// C++ `runOutputs`. Returns the bits to apply and an optional follow-up delay.
    pub fn run_outputs(&mut self, circ_time: u64) -> (u32, Option<u64>) {
        if let Some(v) = self.out_queue.pop_front() {
            self.out_value = v;
        }
        let next = self
            .time_queue
            .pop_front()
            .map(|t| t.saturating_sub(circ_time));
        (self.out_value, next.filter(|d| *d > 0))
    }
}
