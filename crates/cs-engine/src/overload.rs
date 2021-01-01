//! Component overload, crash, and warning detection with C++ SimulIDE parity.

#[derive(Clone, Debug, PartialEq)]
pub struct ItemOverloadState {
    pub warning: bool,
    pub crashed: bool,
    pub reason: String,
}

impl Default for ItemOverloadState {
    fn default() -> Self {
        Self {
            warning: false,
            crashed: false,
            reason: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverloadLogEntry {
    pub comp_uid: String,
    pub text: String,
    pub crashed: bool,
}

/// Evaluates electrolytic capacitor reverse polarity.
/// In C++ `elCapacitor::updateStep`:
/// `volt = pin[0].voltage - pin[1].voltage;`
/// `crashed = (volt < -1e-6);`
pub fn eval_el_capacitor(v_pos: f64, v_neg: f64) -> ItemOverloadState {
    let volt = v_pos - v_neg;
    if volt < -1e-6 {
        ItemOverloadState {
            warning: false,
            crashed: true,
            reason: format!("Reverse polarity: {:.2} V", -volt),
        }
    } else {
        ItemOverloadState::default()
    }
}

/// Evaluates LED overcurrent with 10% hysteresis band.
/// In C++ `LedBase::updateStep`:
/// `oc = overCurrent();`
/// `warning = oc > (m_warning ? 1.35 : 1.5);`
/// `crashed = warning && (oc > (m_crashed ? 1.8 : 2.0));`
pub fn eval_led(
    current: f64,
    max_current: f64,
    prev_warning: bool,
    prev_crashed: bool,
) -> ItemOverloadState {
    let max_i = max_current.max(0.001);
    let cur = current.max(0.0);
    let oc = cur / max_i;

    let warn_thresh = if prev_warning { 1.35 } else { 1.5 };
    let crash_thresh = if prev_crashed { 1.8 } else { 2.0 };

    let warning = oc > warn_thresh;
    let crashed = warning && (oc > crash_thresh);

    if warning {
        ItemOverloadState {
            warning: true,
            crashed,
            reason: format!(
                "Overcurrent: {:.1} mA (max {:.1} mA)",
                cur * 1e3,
                max_i * 1e3
            ),
        }
    } else {
        ItemOverloadState::default()
    }
}

/// Evaluates RGB LED overcurrent per channel (R, G, B) with 10% hysteresis band.
/// In C++ `LedRgb::updateStep`.
pub fn eval_rgb_led(
    currents: [f64; 3],
    max_current: f64,
    prev_warning: bool,
    prev_crashed: bool,
) -> ItemOverloadState {
    const CHAN_NAMES: [&str; 3] = ["R", "G", "B"];
    let max_i = max_current.max(0.001);

    let warn_thresh = if prev_warning { 1.35 } else { 1.5 };
    let crash_thresh = if prev_crashed { 1.8 } else { 2.0 };

    let mut warning = false;
    let mut crashed = false;
    let mut worst_idx = 0;
    let mut max_oc = 0.0;

    for i in 0..3 {
        let cur = currents[i].max(0.0);
        let oc = cur / max_i;
        if oc > warn_thresh {
            warning = true;
            if oc > crash_thresh {
                crashed = true;
            }
            if oc > max_oc {
                max_oc = oc;
                worst_idx = i;
            }
        }
    }

    if warning {
        let worst_cur = currents[worst_idx].max(0.0);
        ItemOverloadState {
            warning: true,
            crashed,
            reason: format!(
                "Overcurrent on {} channel: {:.1} mA (max {:.1} mA)",
                CHAN_NAMES[worst_idx],
                worst_cur * 1e3,
                max_i * 1e3
            ),
        }
    } else {
        ItemOverloadState::default()
    }
}

/// Evaluates Lamp overcurrent with 10% hysteresis band.
/// In C++ `Lamp::updateStep`:
/// `oc = overCurrent();`
/// `warning = oc > (m_warning ? 1.35 : 1.5);`
/// `crashed = warning && (oc > (m_crashed ? 1.8 : 2.0));`
pub fn eval_lamp(
    current: f64,
    max_current: f64,
    prev_warning: bool,
    prev_crashed: bool,
) -> ItemOverloadState {
    let max_i = max_current.max(0.001);
    let cur = current.abs();
    let oc = cur / max_i;

    let warn_thresh = if prev_warning { 1.35 } else { 1.5 };
    let crash_thresh = if prev_crashed { 1.8 } else { 2.0 };

    let warning = oc > warn_thresh;
    let crashed = warning && (oc > crash_thresh);

    if warning {
        ItemOverloadState {
            warning: true,
            crashed,
            reason: format!("Overcurrent: {:.2} A (max {:.2} A)", cur, max_i),
        }
    } else {
        ItemOverloadState::default()
    }
}

/// Evaluates Diode / ZenerDiode overcurrent.
/// In C++ `Diode::updateStep`:
/// `warning = m_current > (m_warning ? m_maxCur * 0.9 : m_maxCur);`
/// `crashed = warning && (m_current > (m_crashed ? m_maxCur * 1.8 : m_maxCur * 2));`
pub fn eval_diode(
    current: f64,
    max_current: f64,
    prev_warning: bool,
    prev_crashed: bool,
) -> ItemOverloadState {
    let max_cur = max_current.max(0.001);
    let cur = current.max(0.0);

    let warn_thresh = if prev_warning { max_cur * 0.9 } else { max_cur };
    let crash_thresh = if prev_crashed {
        max_cur * 1.8
    } else {
        max_cur * 2.0
    };

    let warning = cur > warn_thresh;
    let crashed = warning && (cur > crash_thresh);

    if warning {
        ItemOverloadState {
            warning: true,
            crashed,
            reason: format!("Overcurrent: {:.3} A (max {:.3} A)", cur, max_cur),
        }
    } else {
        ItemOverloadState::default()
    }
}

/// Evaluates Voltmeter / Ammeter out of range overflow.
/// In C++ `Meter::updateStep`:
/// `if (value > 999) m_crashed = true;`
pub fn eval_meter(value: f64) -> ItemOverloadState {
    if value.abs() > 999.0 {
        ItemOverloadState {
            warning: false,
            crashed: true,
            reason: "Out of range: meter overload".to_string(),
        }
    } else {
        ItemOverloadState::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_el_capacitor_reverse_polarity() {
        let normal = eval_el_capacitor(5.0, 0.0);
        assert!(!normal.crashed);
        assert!(!normal.warning);

        let reverse = eval_el_capacitor(0.0, 5.0);
        assert!(reverse.crashed);
        assert!(!reverse.warning);
        assert_eq!(reverse.reason, "Reverse polarity: 5.00 V");
    }

    #[test]
    fn test_led_overcurrent_hysteresis() {
        let max_i = 0.03; // 30mA
        // Below 1.5x (45mA) -> normal
        let s1 = eval_led(0.040, max_i, false, false);
        assert!(!s1.warning);
        assert!(!s1.crashed);

        // Above 1.5x (46mA) -> warning
        let s2 = eval_led(0.046, max_i, false, false);
        assert!(s2.warning);
        assert!(!s2.crashed);
        assert!(s2.reason.contains("Overcurrent: 46.0 mA"));

        // Drops to 42mA (> 1.35x = 40.5mA) with prev_warning=true -> stays warning
        let s3 = eval_led(0.042, max_i, true, false);
        assert!(s3.warning);

        // Drops below 1.35x (39mA) -> clears
        let s4 = eval_led(0.039, max_i, true, false);
        assert!(!s4.warning);

        // Above 2.0x (65mA) -> crashed
        let s5 = eval_led(0.065, max_i, false, false);
        assert!(s5.warning);
        assert!(s5.crashed);
    }

    #[test]
    fn test_diode_overcurrent() {
        let max_cur = 1.0;
        let s1 = eval_diode(0.8, max_cur, false, false);
        assert!(!s1.warning);

        let s2 = eval_diode(1.2, max_cur, false, false);
        assert!(s2.warning);
        assert!(!s2.crashed);

        let s3 = eval_diode(2.2, max_cur, true, false);
        assert!(s3.warning);
        assert!(s3.crashed);
    }
}
