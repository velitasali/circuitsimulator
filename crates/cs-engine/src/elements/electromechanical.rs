//! Electromechanical load, inertia, power consumption, and contact dynamics models:
//! - Stepper motor rotor inertia, viscous friction, holding torque, and step-slip under load.
//! - Servo motor dynamic power consumption (idle, moving, stall current draw) and torque-stall modeling.
//! - Relay contact bounce intervals, switching arcing, and contact dielectric breakdown.

use std::f64::consts::PI;

/// Physical and dynamic runtime state of a stepper motor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepperState {
    /// Peak holding torque at rated current T_h (N·m).
    pub holding_torque: f64,
    /// Total mechanical rotor inertia J (kg·m²).
    pub inertia: f64,
    /// Viscous damping friction coefficient B (N·m·s/rad).
    pub friction: f64,
    /// External mechanical load torque T_load (N·m). Opposes rotation.
    pub load_torque: f64,

    // Dynamic runtime states
    /// Instantaneous mechanical rotor angular speed ω_m (rad/s).
    pub speed: f64,
    /// Cumulative count of slipped steps due to stall or excessive torque/acceleration.
    pub step_slip: u32,
    /// Whether the rotor is currently stalled (torque limit exceeded or stuck).
    pub is_stalled: bool,
    /// Instantaneous electromagnetic driving torque T_e (N·m).
    pub elec_torque: f64,
}

impl Default for StepperState {
    fn default() -> Self {
        Self {
            holding_torque: 0.4,
            inertia: 6e-6,
            friction: 0.004,
            load_torque: 0.0,
            speed: 0.0,
            step_slip: 0,
            is_stalled: false,
            elec_torque: 0.0,
        }
    }
}

impl StepperState {
    pub fn new(holding_torque: f64, inertia: f64, friction: f64, load_torque: f64) -> Self {
        Self {
            holding_torque: holding_torque.max(1e-4),
            inertia: inertia.max(1e-8),
            friction: friction.max(0.0),
            load_torque: load_torque.max(0.0),
            ..Default::default()
        }
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.speed = prev.speed;
        self.step_slip = prev.step_slip;
        self.is_stalled = prev.is_stalled;
        self.elec_torque = prev.elec_torque;
    }

    /// Step the mechanical dynamics forward by `dt_s` seconds given phase currents `ia` and `ib`.
    pub fn step_dynamics(
        &mut self,
        angle: &mut f64,
        ia: f64,
        ib: f64,
        steps: i32,
        resistance: f64,
        dt_s: f64,
    ) {
        if dt_s <= 0.0 {
            return;
        }

        // Substep integration to ensure numerical stability for 2nd order mechanics
        const MAX_SUBSTEP: f64 = 50e-6; // 50 microseconds
        let n_steps = ((dt_s / MAX_SUBSTEP).ceil() as usize).clamp(1, 2000);
        let sub_dt = dt_s / n_steps as f64;

        for _ in 0..n_steps {
            self.step_dynamics_sub(angle, ia, ib, steps, resistance, sub_dt);
        }
    }

    fn step_dynamics_sub(
        &mut self,
        angle: &mut f64,
        ia: f64,
        ib: f64,
        steps: i32,
        resistance: f64,
        dt_s: f64,
    ) {
        let pole_pairs = (steps.max(1) as f64 / 4.0).max(1.0);
        let imag = (ia * ia + ib * ib).sqrt();

        if imag < 1e-4 {
            // No current in windings -> decay speed via friction and load torque
            self.elec_torque = 0.0;
            if self.speed.abs() > 1e-4 {
                let decel = (self.friction * self.speed + self.load_torque * self.speed.signum())
                    / self.inertia;
                let next_speed = self.speed - decel * dt_s;
                if next_speed.signum() != self.speed.signum() {
                    self.speed = 0.0;
                } else {
                    self.speed = next_speed;
                }
                *angle = (*angle + (self.speed * dt_s * 180.0 / PI)).rem_euclid(360.0);
            } else {
                self.speed = 0.0;
            }
            return;
        }

        // Stator magnetic field angle in electrical radians: atan2(Ib, Ia)
        let theta_e = ib.atan2(ia);

        // Current mechanical angle of rotor in radians
        let theta_m_rad = *angle * (PI / 180.0);

        // Torque angle: delta_e = theta_e - N_p * theta_m
        let mut delta_e = theta_e - pole_pairs * theta_m_rad;

        // Normalize delta_e into (-PI, PI] and track step slips
        let wraps = ((delta_e + PI) / (2.0 * PI)).floor();
        if wraps.abs() >= 1.0 {
            let slipped = wraps.abs() as u32;
            self.step_slip = self.step_slip.saturating_add(slipped);
            delta_e -= wraps * 2.0 * PI;
        }

        // Current scaling relative to nominal (e.g. 5V / R)
        let rated_current = (5.0 / resistance.max(1e-3)).max(0.05);
        let i_ratio = (imag / rated_current).clamp(0.0, 3.0);

        // Restoring electromagnetic torque: Te = Th * (I / I_rated) * sin(delta_e)
        let t_e = self.holding_torque * i_ratio * delta_e.sin();
        self.elec_torque = t_e;

        // Mechanical load torque opposes direction of motion
        let t_load_eff = if self.speed.abs() > 1e-4 {
            self.load_torque * self.speed.signum()
        } else if t_e.abs() > self.load_torque {
            self.load_torque * t_e.signum()
        } else {
            t_e // Static friction prevents breakaway
        };

        let t_friction = self.friction * self.speed;
        let t_net = t_e - t_friction - t_load_eff;

        let alpha = t_net / self.inertia;
        self.speed += alpha * dt_s;

        // If static friction holds rotor at rest and electrical torque cannot overcome load
        if self.speed.abs() <= 1e-3 && t_e.abs() <= self.load_torque {
            self.speed = 0.0;
        }

        *angle = (*angle + (self.speed * dt_s * 180.0 / PI)).rem_euclid(360.0);

        // Stall detection: load exceeds max holding torque or step slip is occurring at near-zero velocity
        self.is_stalled = (self.load_torque > self.holding_torque * i_ratio)
            || (self.step_slip > 0 && self.speed.abs() < 1e-2);
    }
}

/// Physical and dynamic runtime state of an RC servo motor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ServoState {
    /// Quiescent / idle current draw when holding steady state (A).
    pub idle_current: f64,
    /// Operating current draw while rotating towards target (A).
    pub run_current: f64,
    /// Maximum stall current draw when jammed or under max load (A).
    pub stall_current: f64,
    /// External load torque (kg·cm).
    pub load_torque: f64,
    /// Maximum stall torque of servo (kg·cm).
    pub stall_torque: f64,

    // Dynamic runtime states
    /// Instantaneous electrical current drawn through V+ and GND (A).
    pub current_draw: f64,
    /// Whether the servo is currently in a stalled condition.
    pub is_stalled: bool,
}

impl Default for ServoState {
    fn default() -> Self {
        Self {
            idle_current: 0.015,
            run_current: 0.25,
            stall_current: 1.0,
            load_torque: 0.0,
            stall_torque: 3.5,
            current_draw: 0.015,
            is_stalled: false,
        }
    }
}

impl ServoState {
    pub fn new(
        idle_current: f64,
        run_current: f64,
        stall_current: f64,
        load_torque: f64,
        stall_torque: f64,
    ) -> Self {
        Self {
            idle_current: idle_current.max(0.0),
            run_current: run_current.max(1e-3),
            stall_current: stall_current.max(1e-3),
            load_torque: load_torque.max(0.0),
            stall_torque: stall_torque.max(1e-3),
            current_draw: idle_current.max(0.0),
            is_stalled: false,
        }
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.current_draw = prev.current_draw;
        self.is_stalled = prev.is_stalled;
    }

    /// Advance mechanical position and calculate electrical current consumption.
    pub fn step_dynamics(
        &mut self,
        pos: &mut f64,
        target_pos: f64,
        speed: f64,
        v_supply: f64,
        dt_s: f64,
    ) {
        // Under-voltage lockout: typical RC servos brownout / shut down below 2.5V
        if v_supply < 2.5 {
            self.current_draw = 0.0;
            self.is_stalled = false;
            return;
        }

        let kv = (v_supply / 5.0).clamp(0.4, 1.5);
        let torque_ratio = (self.load_torque / self.stall_torque.max(1e-3)).clamp(0.0, 2.0);
        let delta = target_pos - *pos;
        let abs_delta = delta.abs();

        if torque_ratio >= 1.0 {
            // Stalled under excessive mechanical load
            self.is_stalled = true;
            self.current_draw = self.stall_current * kv;
        } else {
            self.is_stalled = false;
            if abs_delta > 1e-4 && dt_s > 0.0 {
                // Moving towards target angle
                // Speed slows down under mechanical load and lower supply voltage
                let speed_factor = (1.0 - torque_ratio * 0.7).max(0.1) * kv;
                let max_move = dt_s / speed.max(1e-4) * 60.0 * speed_factor;
                let step = if abs_delta > max_move {
                    delta.signum() * max_move
                } else {
                    delta
                };
                *pos += step;
                self.current_draw = (self.run_current
                    + (self.stall_current - self.run_current) * torque_ratio)
                    * kv;
            } else {
                // Idle holding target position
                self.current_draw = (self.idle_current
                    + (self.stall_current - self.idle_current) * (torque_ratio * 0.2))
                    * kv;
            }
        }
    }

    /// Equivalent conductance stamped between V+ and GND pins for electrical power draw.
    pub fn power_conductance(&self, v_supply: f64) -> f64 {
        if v_supply < 2.5 {
            1e-7
        } else {
            (self.current_draw / v_supply.max(0.5)).clamp(1e-5, 100.0)
        }
    }
}

/// Physical contact dynamics, bounce, arcing, and dielectric breakdown models for electromechanical relays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelayState {
    /// Contact bounce duration upon switching (seconds, default 0.002 = 2 ms).
    pub bounce_time: f64,
    /// Contact dielectric breakdown voltage (V, default 500 V).
    pub breakdown_v: f64,
    /// Closed contact series resistance (Ω, default 0.05 Ω = 50 mΩ).
    pub contact_r: f64,
    /// Minimum voltage to sustain an electrical arc across opening contacts (V, default 15 V).
    pub arc_voltage: f64,
    /// Minimum current to sustain an electrical arc (A, default 0.1 A).
    pub arc_min_current: f64,

    // Dynamic runtime states
    /// Simulation time when the most recent armature switch occurred (seconds).
    pub last_switch_t: f64,
    /// Whether currently within the contact bounce window.
    pub in_bounce: bool,
    /// Whether the contact is physically conducting (closed).
    pub contact_closed: bool,
    /// Whether an electrical arc is currently conducting across the contacts.
    pub arcing: bool,
    /// Simulation time when the active arc began (seconds).
    pub arc_start_t: f64,
}

impl Default for RelayState {
    fn default() -> Self {
        Self {
            bounce_time: 0.002,
            breakdown_v: 500.0,
            contact_r: 0.05,
            arc_voltage: 15.0,
            arc_min_current: 0.1,
            last_switch_t: 0.0,
            in_bounce: false,
            contact_closed: true,
            arcing: false,
            arc_start_t: 0.0,
        }
    }
}

impl RelayState {
    pub fn new(bounce_time: f64, breakdown_v: f64, contact_r: f64) -> Self {
        Self {
            bounce_time: bounce_time.max(0.0),
            breakdown_v: breakdown_v.max(10.0),
            contact_r: contact_r.max(1e-6),
            ..Default::default()
        }
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.last_switch_t = prev.last_switch_t;
        self.in_bounce = prev.in_bounce;
        self.contact_closed = prev.contact_closed;
        self.arcing = prev.arcing;
        self.arc_start_t = prev.arc_start_t;
    }

    /// Trigger armature switch at simulation time `sim_time` (seconds).
    pub fn on_armature_switch(&mut self, sim_time: f64) {
        if self.bounce_time > 0.0 && sim_time > 0.0 {
            self.in_bounce = true;
            self.last_switch_t = sim_time;
            self.contact_closed = false;
        } else {
            self.in_bounce = false;
            self.contact_closed = true;
        }
    }

    /// Update contact state given whether relay is active/energized, contact voltage, contact current, and time.
    /// Returns true if contact conduction state changed.
    pub fn update_contact_state(
        &mut self,
        is_energized: bool,
        v_contact: f64,
        i_contact: f64,
        sim_time: f64,
    ) -> bool {
        let prev_closed = self.contact_closed;
        let prev_arcing = self.arcing;

        if !is_energized {
            // Armature released / de-energized
            self.in_bounce = false;
            self.contact_closed = false;

            // Check arcing upon contact opening or dielectric breakdown
            if self.arcing {
                let arc_dur = sim_time - self.arc_start_t;
                if i_contact.abs() < self.arc_min_current
                    || (arc_dur > 0.003 && v_contact.abs() < self.breakdown_v)
                {
                    self.arcing = false;
                }
            } else if (i_contact.abs() >= self.arc_min_current
                && v_contact.abs() >= self.arc_voltage)
                || v_contact.abs() >= self.breakdown_v
            {
                // Arc ignited by inductive flyback or dielectric breakdown!
                self.arcing = true;
                self.arc_start_t = sim_time;
            }
        } else if self.in_bounce && self.bounce_time > 0.0 && sim_time > 0.0 {
            let elapsed = sim_time - self.last_switch_t;
            if elapsed >= self.bounce_time {
                self.in_bounce = false;
                self.contact_closed = true;
                self.arcing = false;
            } else {
                // Physical bounce pattern: 3 bounces before continuous settle
                let tau = (elapsed / self.bounce_time).clamp(0.0, 1.0);
                let touching =
                    (0.0..=0.35).contains(&tau) || (0.52..=0.75).contains(&tau) || (tau >= 0.85);
                self.contact_closed = touching;
                self.arcing = false;
            }
        } else {
            self.contact_closed = true;
            self.arcing = false;
        }

        self.contact_closed != prev_closed || self.arcing != prev_arcing
    }

    /// Effective contact conductance in MNA.
    pub fn contact_conductance(&self) -> f64 {
        if self.contact_closed {
            1.0 / self.contact_r.max(1e-6)
        } else if self.arcing {
            0.1 // 10 Ω dynamic arc resistance
        } else {
            1e-12 // Open circuit leakage
        }
    }
}
