//! C++ `eClockedDevice` trigger / clock edge.

use super::pin::IoPin;

/// C++ `clkState_t`. `Clock_Allow` shares the Rising discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClkState {
    Low = 0,
    Rising = 1,
    High = 2,
    Falling = 3,
}

/// C++ `trigger_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    None,
    Clock,
    Enable,
}

impl Trigger {
    pub fn from_str_name(s: &str) -> Self {
        match s {
            "None" => Trigger::None,
            "Enable" => Trigger::Enable,
            _ => Trigger::Clock,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Trigger::None => "None",
            Trigger::Clock => "Clock",
            Trigger::Enable => "Enable",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Clocked {
    pub trigger: Trigger,
    clock: bool,
    pub clk_state: ClkState,
}

impl Default for Clocked {
    fn default() -> Self {
        Self {
            trigger: Trigger::Clock,
            clock: false,
            clk_state: ClkState::Low,
        }
    }
}

impl Clocked {
    pub fn stamp(&mut self, clk: Option<&IoPin>) {
        self.clock = clk.map(|p| p.inverted).unwrap_or(false);
        self.clk_state = ClkState::Low;
    }

    /// C++ `updateClock`. `clk_level` is `getInpState()` of the clock pin.
    pub fn update(&mut self, clk_level: Option<bool>) {
        let Some(clock) = clk_level else {
            self.clk_state = ClkState::Rising; // Clock_Allow
            return;
        };
        self.clk_state = ClkState::Low;
        match self.trigger {
            Trigger::Enable => {
                if clock {
                    self.clk_state = ClkState::Rising; // Allow
                }
            }
            Trigger::Clock => {
                if !self.clock && clock {
                    self.clk_state = ClkState::Rising;
                } else if self.clock && clock {
                    self.clk_state = ClkState::High;
                } else if self.clock && !clock {
                    self.clk_state = ClkState::Falling;
                }
            }
            Trigger::None => {
                self.clk_state = ClkState::Rising; // Allow
            }
        }
        self.clock = clock;
    }

    pub fn allow(&self) -> bool {
        self.clk_state == ClkState::Rising
    }
}
