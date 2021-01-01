//! Picosecond event queue matching C++ `Simulator::addEvent` / `cancelEvents`.
//!
//! One pending event per [`EventTarget`] (C++ one slot per `eElement`).

use rustc_hash::FxHashMap;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventTarget {
    AnalogClock,
    /// `components[i]` runEvent (gate / FF / comparator / latch).
    Device(usize),
    /// Slope step on a device pin (`pin` indexes [`DevicePins`]).
    Pin {
        comp: usize,
        pin: usize,
    },
}

#[derive(Clone, Debug, Default)]
pub struct EventQueue {
    /// `(time, seq, target)` min-heap. `seq` breaks ties in insertion order.
    heap: BinaryHeap<Reverse<(u64, u64, EventTarget)>>,
    /// Fast generation slot for `Device(i)`.
    device_seq: Vec<u64>,
    /// Fast generation slot for `AnalogClock`.
    analog_clock_seq: u64,
    /// Fallback generation slot for `Pin { comp, pin }`.
    pin_seq: FxHashMap<(usize, usize), u64>,
    live_count: usize,
    seq: u64,
}

impl EventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.heap.clear();
        self.device_seq.clear();
        self.analog_clock_seq = 0;
        self.pin_seq.clear();
        self.live_count = 0;
        self.seq = 0;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    #[inline]
    pub fn has_event(&self, target: &EventTarget) -> bool {
        self.get_live_seq(target) != 0
    }

    #[inline]
    fn get_live_seq(&self, target: &EventTarget) -> u64 {
        match target {
            EventTarget::AnalogClock => self.analog_clock_seq,
            EventTarget::Device(i) => self.device_seq.get(*i).copied().unwrap_or(0),
            EventTarget::Pin { comp, pin } => {
                self.pin_seq.get(&(*comp, *pin)).copied().unwrap_or(0)
            }
        }
    }

    #[inline]
    fn set_live_seq(&mut self, target: EventTarget, seq: u64) {
        match target {
            EventTarget::AnalogClock => {
                if (self.analog_clock_seq != 0) != (seq != 0) {
                    if seq != 0 {
                        self.live_count += 1;
                    } else {
                        self.live_count -= 1;
                    }
                }
                self.analog_clock_seq = seq;
            }
            EventTarget::Device(i) => {
                if i >= self.device_seq.len() {
                    self.device_seq.resize(i + 1, 0);
                }
                let old = self.device_seq[i];
                if (old != 0) != (seq != 0) {
                    if seq != 0 {
                        self.live_count += 1;
                    } else {
                        self.live_count -= 1;
                    }
                }
                self.device_seq[i] = seq;
            }
            EventTarget::Pin { comp, pin } => {
                let key = (comp, pin);
                if seq != 0 {
                    if self.pin_seq.insert(key, seq).is_none() {
                        self.live_count += 1;
                    }
                } else if self.pin_seq.remove(&key).is_some() {
                    self.live_count -= 1;
                }
            }
        }
    }

    /// C++ `addEventAt`: ignored if this target already has an event.
    #[inline]
    pub fn add(&mut self, time: u64, target: EventTarget) -> bool {
        if self.get_live_seq(&target) != 0 {
            return false;
        }
        self.seq += 1;
        self.set_live_seq(target, self.seq);
        self.heap.push(Reverse((time, self.seq, target)));
        true
    }

    #[inline]
    pub fn cancel(&mut self, target: EventTarget) {
        self.set_live_seq(target, 0);
    }

    pub fn peek_time(&mut self) -> Option<u64> {
        self.drop_stale();
        self.heap.peek().map(|Reverse((t, _, _))| *t)
    }

    pub fn peek_target(&mut self) -> Option<EventTarget> {
        self.drop_stale();
        self.heap.peek().map(|Reverse((_, _, t))| *t)
    }

    /// Pop the next live event with `time <= end`.
    pub fn pop_due(&mut self, end: u64) -> Option<(u64, EventTarget)> {
        self.drop_stale();
        let Reverse((time, seq, target)) = self.heap.peek().copied()?;
        if time > end {
            return None;
        }
        self.heap.pop();
        if self.get_live_seq(&target) == seq {
            self.set_live_seq(target, 0);
            Some((time, target))
        } else {
            self.pop_due(end)
        }
    }

    #[inline]
    fn drop_stale(&mut self) {
        while let Some(Reverse((_, seq, target))) = self.heap.peek().copied() {
            if self.get_live_seq(&target) == seq {
                break;
            }
            self.heap.pop();
        }
    }
}
