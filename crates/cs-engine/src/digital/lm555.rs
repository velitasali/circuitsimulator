//! 555 Timer IC simulation element matching C++ `Lm555`.

use super::queue::OutQueue;

#[derive(Clone, Debug)]
pub struct Lm555State {
    pub id: String,
    pub out_state: bool,    // Internal latch output (true = High, false = Low)
    pub discharge_on: bool, // Discharge transistor conducting to GND
    pub prop_delay_ps: u64, // Propagation delay in picoseconds (default 10ns = 10,000ps)
    pub out_high_v: f64,    // VCC - 1.3V
    pub out_low_v: f64,     // 0.0V
    pub queue: OutQueue,
}

impl Lm555State {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            out_state: false,
            discharge_on: true,
            prop_delay_ps: 10_000, // 10 ns
            out_high_v: 3.7,       // 5V - 1.3V
            out_low_v: 0.0,
            queue: OutQueue::new(),
        }
    }

    /// Step the 555 analog-to-digital decision logic based on terminal voltages.
    ///
    /// - `v_gnd`: Ground pin voltage
    /// - `v_vcc`: Supply voltage pin
    /// - `v_trg`: Trigger input voltage (pin 2)
    /// - `v_thr`: Threshold input voltage (pin 6)
    /// - `v_cv`: Control Voltage pin (pin 5, optional override)
    /// - `v_rst`: Reset pin (pin 4, active low below ~0.7V)
    pub fn step(
        &mut self,
        v_gnd: f64,
        v_vcc: f64,
        v_trg: f64,
        v_thr: f64,
        v_cv: Option<f64>,
        v_rst: f64,
    ) -> (bool, bool) {
        let v_supply = (v_vcc - v_gnd).max(0.0);
        if v_supply < 1.0 {
            // Unpowered
            self.out_state = false;
            self.discharge_on = false;
            return (false, false);
        }

        // Upper reference voltage (CV pin or 2/3 VCC)
        let v_upper = v_cv.unwrap_or(v_gnd + (2.0 / 3.0) * v_supply);
        // Lower reference voltage (1/2 CV or 1/3 VCC)
        let v_lower = if let Some(cv) = v_cv {
            v_gnd + (cv - v_gnd) * 0.5
        } else {
            v_gnd + (1.0 / 3.0) * v_supply
        };

        // Reset check: active-low below 0.7V relative to GND
        let rst_active = (v_rst - v_gnd) < 0.7;

        if rst_active {
            self.out_state = false;
            self.discharge_on = true;
        } else {
            let trg_fired = v_trg < v_lower;
            let thr_fired = v_thr > v_upper;

            if trg_fired {
                // Trigger sets latch (Out = High, Discharge = Off)
                self.out_state = true;
                self.discharge_on = false;
            } else if thr_fired {
                // Threshold resets latch (Out = Low, Discharge = On)
                self.out_state = false;
                self.discharge_on = true;
            }
        }

        self.out_high_v = (v_vcc - 1.3).max(v_gnd);
        self.out_low_v = v_gnd + 0.1;

        (self.out_state, self.discharge_on)
    }
}
