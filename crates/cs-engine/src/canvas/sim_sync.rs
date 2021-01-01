//! Simulation synchronization, runtime stepping loop, MCU cosimulation, and instruments.

use crate::canvas::events::Change;
use crate::canvas::scene::Part;
use crate::canvas::{Canvas, Point, with_pin_id, with_pin_id_idx};
use crate::elements::pins::{
    PIN_IN0, PIN_IN1, PIN_IN2, PIN_IN3, PIN_IN4, PIN_IN5, PIN_INPIN, PIN_LEFT, PIN_RGB_B,
    PIN_RGB_G, PIN_RGB_R, PIN_RIGHT,
};

impl Canvas {
    pub fn power_on(&mut self) -> Change {
        self.sim_running = true;
        let mut circuit = self.scene.to_circuit();
        circuit.slope_steps = crate::settings::get().slope_steps;
        crate::logging::log_sim(format!(
            "Simulation started: {} components, {} wires",
            self.scene.items().len(),
            self.scene.wires().len()
        ));
        if circuit.has_audio_out() {
            if let Some(sink) = &self.audio_sink {
                if let Ok(mut s) = sink.lock() {
                    s.resume();
                }
                circuit.set_audio_sink(sink.clone());
            } else if let Ok(pb) = crate::audio::Playback::try_start() {
                let sink = std::sync::Arc::new(std::sync::Mutex::new(pb));
                self.audio_sink = Some(sink.clone());
                circuit.set_audio_sink(sink);
            }
        }
        let r = circuit.solve();
        self.running = Some(circuit);
        self.dirty.mark_full();
        let mut c = Change {
            sim: true,
            items: true,
            wires: true,
            ..Change::default()
        };
        c.merge(self.publish_solution(r));
        c
    }

    /// SIGSTOP live QEMU children (circuit pause).
    pub fn pause_qemu(&mut self) {
        crate::logging::log_sim("Simulation paused");
        if let Some(c) = self.running.as_mut() {
            c.pause_qemu();
        }
    }

    /// SIGCONT live QEMU children (circuit resume).
    pub fn resume_qemu(&mut self) {
        crate::logging::log_sim("Simulation resumed");
        if let Some(c) = self.running.as_mut() {
            c.resume_qemu();
        }
    }

    pub fn power_off(&mut self) -> Change {
        if !self.sim_running && self.pin_volts.is_empty() && self.sim_error.is_none() {
            self.publish_mcu_snap();
            return Change::default();
        }
        crate::logging::log_sim("Simulation stopped");
        if let Some(c) = self.running.as_ref() {
            self.scene.capture_mcus_from(c);
            self.scene.capture_memories_from(c);
        }
        if let Some(sink) = &self.audio_sink {
            if let Ok(mut s) = sink.lock() {
                s.stop();
            }
        }
        for it in self.scene.items_mut() {
            if let Part::DcMotor(ref mut motor) = it.kind {
                motor.speed = 0.0;
            }
        }
        self.sim_running = false;
        self.running = None;
        self.pin_volts.clear();
        self.wire_currents.clear();
        self.pin_directions.clear();
        self.pin_pullups.clear();
        self.publish_instrument_views();
        self.scope_traces = crate::instruments::empty_scope_traces();
        self.la_traces = crate::instruments::empty_la_traces();
        self.sim_error = None;
        self.sim_warning = None;
        self.sim_warning_ticks = 0;
        self.anim_tick = 0;
        self.wire_chevron_step.clear();
        self.overload_states.clear();
        self.overload_escalated = false;
        self.analog_visual.clear();
        self.publish_mcu_snap();
        self.dirty.mark_full();
        Change {
            sim: true,
            items: true,
            wires: true,
            anim: true,
            plots: true,
            ..Change::default()
        }
    }

    pub fn tick(&mut self) -> Change {
        if !self.sim_running {
            return Change::default();
        }
        let mut c = Change::default();
        if self.sim_warning_ticks > 0 {
            self.sim_warning_ticks -= 1;
            if self.sim_warning_ticks == 0 {
                self.sim_warning = None;
                c.sim = true;
            }
        }
        self.sim_tick = self.sim_tick.wrapping_add(1);
        self.anim_tick = self.anim_tick.wrapping_add(1);
        if self.advance_chevron_steps() {
            c.anim = true;
        }
        if self.sim_tick % 3 == 0 && !self.overload_states.is_empty() {
            c.anim = true;
            let ids: Vec<String> = self.overload_states.keys().cloned().collect();
            for id in ids {
                self.dirty.mark_item(&id);
            }
        }
        let fps = crate::settings::get().fps.clamp(1, 100);
        let ps_per_sec = self.scene.settings().ps_per_sec();
        let ps_target = (ps_per_sec / fps).max(1);
        let frame_budget_ms = (1000 / fps * 3 / 4).max(1);
        let stepped = self.running.as_mut().map(|circuit| {
            circuit.ps_per_sec = ps_per_sec;
            let new_dt = self.scene.settings().analog_dt();
            if (circuit.dt - new_dt).abs() > 1e-15 {
                circuit.dt = new_dt;
                let _ = circuit.re_solve();
            }
            circuit.max_nl_steps = self.scene.settings().nl_steps;
            circuit.run_ps_budgeted(
                ps_target,
                Some(std::time::Duration::from_millis(frame_budget_ms)),
            )
        });
        if let Some(r) = stepped {
            c.merge(self.publish_solution(r));
        } else {
            c.merge(self.publish_instrument_views());
        }
        c.pause_sim |= self.check_probe_pause();
        c
    }

    /// C++ `Connector::updateStep`: `m_step += clamp(speed * current, -4, 4)`.
    /// Returns true if any current-carrying wire advanced (and was dirtied).
    fn advance_chevron_steps(&mut self) -> bool {
        if !self.scene.settings().animate_curr {
            return false;
        }
        let speed = self.curr_speed;
        let moving: Vec<(String, f64)> = self
            .scene
            .wires()
            .iter()
            .filter(|w| !w.drawing() && !w.is_bus)
            .map(|w| (w.id.clone(), self.wire_current(&w.id)))
            .filter(|(_, current)| current.abs() > 1e-12)
            .collect();
        if moving.is_empty() {
            return false;
        }
        for (id, current) in moving {
            let delta = (speed * current).clamp(-4.0, 4.0);
            let step = self.wire_chevron_step.entry(id.clone()).or_insert(0.0);
            *step = crate::canvas::wrap_chevron_step(*step + delta);
            self.dirty.mark_wire(&id);
        }
        true
    }

    pub(crate) fn check_probe_pause(&mut self) -> bool {
        let mut pause = false;
        let mut updates: Vec<(String, bool)> = Vec::new();
        for it in self.scene.items() {
            let Part::Probe(ref probe) = it.kind else {
                continue;
            };
            if !probe.pause_at_change {
                continue;
            }
            let threshold = probe.threshold;
            let connected_and_volt = with_pin_id(&it.id, PIN_INPIN, |pin_id| {
                if self.scene.pin_connected(pin_id) {
                    Some(self.pin_volts.get(pin_id).copied().unwrap_or(0.0))
                } else {
                    None
                }
            });
            let Some(volt) = connected_and_volt else {
                continue;
            };
            let high = volt > threshold;
            if let Some(prev) = self.probe_last_high.get(&it.id) {
                if *prev != high {
                    pause = true;
                }
            }
            updates.push((it.id.clone(), high));
        }
        for (id, high) in updates {
            self.probe_last_high.insert(id, high);
        }
        pause
    }

    pub fn refresh_sim_component(&mut self, id: &str) -> Change {
        if !self.sim_running {
            return Change::default();
        }
        if let Some(circuit) = self.running.as_mut() {
            if let Some(item) = self.scene.item_by_id(id) {
                if let Some(kind) = item.to_element_kind() {
                    if matches!(kind, crate::elements::Kind::AudioOut { .. })
                        && circuit.has_audio_out()
                        && self.audio_sink.is_none()
                    {
                        return self.refresh_sim();
                    }
                    if circuit.update_component_kind(id, &kind) {
                        let r = circuit.re_solve();
                        return self.publish_solution(r);
                    }
                }
            }
        }
        self.refresh_sim()
    }

    pub(crate) fn refresh_sim(&mut self) -> Change {
        if !self.sim_running {
            return Change::default();
        }
        let mut circuit = self.scene.to_circuit();
        circuit.slope_steps = crate::settings::get().slope_steps;
        if circuit.has_audio_out() {
            if let Some(sink) = &self.audio_sink {
                if let Ok(mut s) = sink.lock() {
                    s.resume();
                }
                circuit.set_audio_sink(sink.clone());
            } else if let Ok(pb) = crate::audio::Playback::try_start() {
                let sink = std::sync::Arc::new(std::sync::Mutex::new(pb));
                self.audio_sink = Some(sink.clone());
                circuit.set_audio_sink(sink);
            }
        } else if let Some(sink) = &self.audio_sink {
            if let Ok(mut s) = sink.lock() {
                s.stop();
            }
        }
        let r = circuit.solve();
        self.running = Some(circuit);
        self.publish_solution(r)
    }

    pub(crate) fn publish_solution(&mut self, r: crate::Result<()>) -> Change {
        match r {
            Ok(()) => {
                let mut new_volts = std::collections::HashMap::new();
                let mut new_currents = std::collections::HashMap::new();
                let mut new_directions = std::collections::HashMap::new();
                let mut new_pullups = std::collections::HashSet::new();
                let mut motor_anim = false;
                if let Some(circuit) = self.running.as_ref() {
                    for it in self.scene.items_mut() {
                        for p in it.pins() {
                            if let Some(v) = circuit.pin_voltage(&p.id) {
                                new_volts.insert(p.id.clone(), v);
                            }
                            if let Some(dir) = circuit.pin_direction(&p.id) {
                                new_directions.insert(p.id.clone(), dir);
                            }
                            if circuit.pin_has_pullup(&p.id) {
                                new_pullups.insert(p.id.clone());
                            }
                        }
                        if let Part::DcMotor(ref mut motor) = it.kind {
                            if let Some(comp) = circuit.components().iter().find(|c| c.id == it.id)
                            {
                                if let crate::elements::Kind::DcMotor {
                                    speed: s, angle: a, ..
                                } = comp.kind
                                {
                                    if (motor.speed - s).abs() > 1e-6
                                        || (motor.angle - a).abs() > 1e-4
                                    {
                                        motor.speed = s;
                                        motor.angle = a;
                                        motor_anim = true;
                                    }
                                }
                            }
                        } else if let Part::Servo(ref mut servo) = it.kind {
                            if let Some(comp) = circuit.components().iter().find(|c| c.id == it.id)
                            {
                                if let crate::elements::Kind::Servo { pos: p, .. } = comp.kind {
                                    if (servo.pos - p).abs() > 1e-3 {
                                        servo.pos = p;
                                        self.dirty.mark_item(&it.id);
                                        motor_anim = true;
                                    }
                                }
                            }
                        }
                    }
                    new_currents = compute_wire_currents(&self.scene, circuit);
                }
                let animate_logic = self.scene.settings().animate_logic;
                let animate_curr = self.scene.settings().animate_curr;
                if motor_anim {
                    for it in self.scene.items() {
                        if matches!(&it.kind, Part::DcMotor(_)) {
                            self.dirty.mark_item(&it.id);
                        }
                    }
                }
                for it in self.scene.items() {
                    let mut item_dirty = false;
                    for p in it.pins() {
                        let old_high = self
                            .pin_volts
                            .get(&p.id)
                            .map(|v| crate::canvas::dirty::pin_logic_high(*v));
                        let new_high = new_volts
                            .get(&p.id)
                            .map(|v| crate::canvas::dirty::pin_logic_high(*v));
                        if old_high != new_high {
                            item_dirty = true;
                        }
                        let old_dir = self.pin_directions.get(&p.id).copied();
                        let new_dir = new_directions.get(&p.id).copied();
                        if old_dir != new_dir {
                            item_dirty = true;
                        }
                        let old_pu = self.pin_pullups.contains(&p.id);
                        let new_pu = new_pullups.contains(&p.id);
                        if old_pu != new_pu {
                            item_dirty = true;
                        }
                    }
                    if let Some(circuit) = self.running.as_ref() {
                        if let Some(bucket) =
                            crate::canvas::dirty::analog_visual_bucket(it, circuit)
                        {
                            if self.analog_visual.get(&it.id) != Some(&bucket) {
                                item_dirty = true;
                                self.analog_visual.insert(it.id.clone(), bucket);
                            }
                        }
                    }
                    if item_dirty {
                        self.dirty.mark_item(&it.id);
                    }
                }
                for w in self.scene.wires() {
                    if animate_logic {
                        let old_high = self
                            .pin_volts
                            .get(&w.start_pin)
                            .map(|v| crate::canvas::dirty::pin_logic_high(*v));
                        let new_high = new_volts
                            .get(&w.start_pin)
                            .map(|v| crate::canvas::dirty::pin_logic_high(*v));
                        if old_high != new_high {
                            self.dirty.mark_wire(&w.id);
                        }
                    }
                    if animate_curr {
                        let old_on =
                            self.wire_currents.get(&w.id).copied().unwrap_or(0.0).abs() > 1e-12;
                        let new_on = new_currents.get(&w.id).copied().unwrap_or(0.0).abs() > 1e-12;
                        if old_on != new_on {
                            self.dirty.mark_wire(&w.id);
                        }
                    }
                }
                self.pin_volts = new_volts;
                self.wire_currents = new_currents;
                self.pin_directions = new_directions;
                self.pin_pullups = new_pullups;
                let error_changed = self.sim_error.is_some();
                self.sim_error = None;
                let mut c = Change {
                    sim: error_changed,
                    items: false,
                    anim: motor_anim,
                    ..Change::default()
                };
                c.merge(self.publish_instrument_views());
                let _overloads_changed = self.evaluate_overloads();
                if !self.overload_states.is_empty() {
                    c.anim = true;
                }
                c
            }
            Err(crate::Error::NotConverged) => {
                let warn_str = "NonLinear Not Converging".to_string();
                let warn_changed = self.sim_warning.as_deref() != Some(&warn_str);
                if warn_changed {
                    crate::logging::log_sim("Warning: NonLinear Not Converging");
                }
                self.sim_warning = Some(warn_str);
                self.sim_warning_ticks = 10;
                let error_changed = self.sim_error.is_some();
                self.sim_error = None;
                Change {
                    sim: warn_changed || error_changed,
                    items: false,
                    wires: false,
                    plots: false,
                    ..Change::default()
                }
            }
            Err(e) => {
                let had_data = !self.pin_volts.is_empty()
                    || !self.wire_currents.is_empty()
                    || !self.pin_directions.is_empty()
                    || !self.pin_pullups.is_empty()
                    || !self.readings.is_empty();
                self.pin_volts.clear();
                self.wire_currents.clear();
                self.pin_directions.clear();
                self.pin_pullups.clear();
                self.readings.clear();
                self.overload_states.clear();
                self.overload_escalated = false;
                self.analog_visual.clear();
                self.sim_warning = None;
                self.sim_warning_ticks = 0;
                let err_str = e.to_string();
                let error_changed = self.sim_error.as_deref() != Some(&err_str);
                if error_changed {
                    crate::logging::log_sim(format!("ERROR: {err_str}"));
                }
                self.sim_error = Some(err_str);
                if had_data {
                    self.dirty.mark_full();
                }
                Change {
                    sim: error_changed,
                    items: false,
                    wires: false,
                    plots: had_data,
                    ..Change::default()
                }
            }
        }
    }

    pub(crate) fn evaluate_overloads(&mut self) -> bool {
        let mut any_changed = false;
        let mut new_states = std::collections::HashMap::new();
        let mut to_log = Vec::new();

        for it in self.scene.items() {
            let prev_state = self
                .overload_states
                .get(&it.id)
                .cloned()
                .unwrap_or_default();
            let new_state = match &it.kind {
                Part::ElCapacitor(_) => {
                    let v_pos = self.item_pin_voltage(&it.id, PIN_LEFT).unwrap_or(0.0);
                    let v_neg = self.item_pin_voltage(&it.id, PIN_RIGHT).unwrap_or(0.0);
                    crate::overload::eval_el_capacitor(v_pos, v_neg)
                }
                Part::Led(p) => {
                    let current = self.item_pin_current(&it.id, PIN_LEFT).unwrap_or(0.0).abs();
                    crate::overload::eval_led(
                        current,
                        p.max_current,
                        prev_state.warning,
                        prev_state.crashed,
                    )
                }
                Part::RgbLed(_) => {
                    let r_cur = self
                        .item_pin_current(&it.id, PIN_RGB_R)
                        .unwrap_or(0.0)
                        .abs();
                    let g_cur = self
                        .item_pin_current(&it.id, PIN_RGB_G)
                        .unwrap_or(0.0)
                        .abs();
                    let b_cur = self
                        .item_pin_current(&it.id, PIN_RGB_B)
                        .unwrap_or(0.0)
                        .abs();
                    crate::overload::eval_rgb_led(
                        [r_cur, g_cur, b_cur],
                        0.03,
                        prev_state.warning,
                        prev_state.crashed,
                    )
                }
                Part::Lamp(p) => {
                    let current = self.item_pin_current(&it.id, PIN_LEFT).unwrap_or(0.0).abs();
                    let max_cur = if p.voltage > 1e-6 {
                        p.power / p.voltage
                    } else {
                        1.0
                    };
                    crate::overload::eval_lamp(
                        current,
                        max_cur,
                        prev_state.warning,
                        prev_state.crashed,
                    )
                }
                Part::Diode(p) => {
                    let current = self.item_pin_current(&it.id, PIN_LEFT).unwrap_or(0.0).abs();
                    crate::overload::eval_diode(
                        current,
                        p.max_current,
                        prev_state.warning,
                        prev_state.crashed,
                    )
                }
                Part::Voltmeter(_) | Part::Ammeter(_) => {
                    let meter_val = self
                        .running
                        .as_ref()
                        .and_then(|c| c.instruments.meters.get(&it.id))
                        .map(|s| s.value)
                        .unwrap_or_else(|| {
                            let v_pos = self.item_pin_voltage(&it.id, PIN_LEFT).unwrap_or(0.0);
                            let v_neg = self.item_pin_voltage(&it.id, PIN_RIGHT).unwrap_or(0.0);
                            v_pos - v_neg
                        });
                    crate::overload::eval_meter(meter_val.abs())
                }
                _ => crate::overload::ItemOverloadState::default(),
            };

            let was_active = prev_state.warning || prev_state.crashed;
            let is_active = new_state.warning || new_state.crashed;
            let got_worse = new_state.crashed && !prev_state.crashed;

            if is_active && (!was_active || got_worse) {
                let text = format!("{}: {}", it.id, new_state.reason);
                to_log.push((it.id.clone(), text, new_state.crashed));
            }

            if new_state != prev_state {
                any_changed = true;
                self.dirty.mark_item(&it.id);
            }
            if is_active {
                new_states.insert(it.id.clone(), new_state);
            }
        }

        for (comp_uid, text, crashed) in to_log {
            self.log_overload(&comp_uid, text, crashed);
            self.overload_escalated = true;
        }

        self.overload_states = new_states;
        any_changed
    }

    pub fn log_overload(&mut self, comp_uid: &str, text: String, crashed: bool) {
        if let Some(pos) = self
            .overload_log
            .iter()
            .position(|e| e.comp_uid == comp_uid)
        {
            self.overload_log.remove(pos);
        }
        self.overload_log.insert(
            0,
            crate::overload::OverloadLogEntry {
                comp_uid: comp_uid.to_string(),
                text,
                crashed,
            },
        );
    }

    pub fn clear_overload_log(&mut self) -> Change {
        self.overload_log.clear();
        Change {
            items: true,
            ..Change::default()
        }
    }

    pub fn select_and_center_item(&mut self, uid: &str) -> Change {
        if let Some(idx) = self.scene.items().iter().position(|it| it.id == uid) {
            let x = self.scene.items()[idx].x;
            let y = self.scene.items()[idx].y;
            self.select_only(idx);
            let mut c = self.set_center(x, y);
            c.items = true;
            return c;
        }
        Change::default()
    }

    pub fn item_overload_state(&self, id: &str) -> Option<&crate::overload::ItemOverloadState> {
        self.overload_states.get(id)
    }

    pub fn overload_states(
        &self,
    ) -> &std::collections::HashMap<String, crate::overload::ItemOverloadState> {
        &self.overload_states
    }

    pub fn overload_log(&self) -> &[crate::overload::OverloadLogEntry] {
        &self.overload_log
    }

    pub fn take_overload_escalated(&mut self) -> bool {
        let esc = self.overload_escalated;
        self.overload_escalated = false;
        esc
    }

    pub fn current_mcu_snap(&self) -> crate::mcu::McuSnap {
        if let Some(c) = &self.running {
            c.mcu_snap()
        } else {
            self.scene.mcu_snap()
        }
    }

    pub(crate) fn publish_mcu_snap(&self) {
        crate::mcu::publish_monitor(self.current_mcu_snap());
    }

    pub(crate) fn apply_mcu_poke(&mut self, poke: crate::mcu::McuPoke) {
        if let Some(c) = self.running.as_mut() {
            c.apply_mcu_poke(poke);
        } else {
            self.scene.poke_mcu(poke);
        }
    }

    /// Drain monitor pokes and publish RAM / PC / STATUS for `McuMonitor`.
    pub fn sync_mcu(&mut self) -> Change {
        for p in crate::mcu::take_pokes() {
            self.apply_mcu_poke(p);
        }
        self.publish_mcu_snap();
        Change::default()
    }

    pub fn poke_mcu_ram(&mut self, addr: u16, value: u8) -> Change {
        self.apply_mcu_poke(crate::mcu::McuPoke::Ram { addr, value });
        self.publish_mcu_snap();
        Change::default()
    }

    pub fn poke_mcu_flash(&mut self, addr: u32, value: u16) -> Change {
        self.apply_mcu_poke(crate::mcu::McuPoke::Flash { addr, value });
        self.publish_mcu_snap();
        Change::default()
    }

    pub fn poke_mcu_eeprom(&mut self, addr: u16, value: u8) -> Change {
        self.apply_mcu_poke(crate::mcu::McuPoke::Eeprom { addr, value });
        self.publish_mcu_snap();
        Change::default()
    }

    pub(crate) fn publish_instrument_views(&mut self) -> Change {
        let prev_readings = std::mem::take(&mut self.readings);
        self.apply_probe_hover();
        let mut plots_changed = false;
        for it in self.scene.items() {
            match &it.kind {
                Part::Probe(p) => {
                    let v = self.item_pin_voltage(&it.id, PIN_INPIN).unwrap_or(0.0);
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::probe_reading(v, p.threshold),
                    );
                }
                Part::Voltmeter(p) => {
                    let default_stats = crate::instruments::MeterStats::default();
                    let stats = self
                        .running
                        .as_ref()
                        .and_then(|c| c.instruments.meters.get(&it.id))
                        .unwrap_or(&default_stats);
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::meter_reading(stats, p.rms, "V"),
                    );
                }
                Part::Ammeter(p) => {
                    let default_stats = crate::instruments::MeterStats::default();
                    let stats = self
                        .running
                        .as_ref()
                        .and_then(|c| c.instruments.meters.get(&it.id))
                        .unwrap_or(&default_stats);
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::meter_reading(stats, p.rms, "A"),
                    );
                }
                Part::FreqMeter(_) => {
                    let default_freq = crate::instruments::FreqMeterState::default();
                    let st = self
                        .running
                        .as_ref()
                        .and_then(|c| c.instruments.freq.get(&it.id))
                        .unwrap_or(&default_freq);
                    self.readings
                        .insert(it.id.clone(), crate::instruments::freq_reading(st));
                }
                Part::Hd44780(_) => {
                    let (hex, lines) = self
                        .running
                        .as_ref()
                        .and_then(|c| {
                            c.components()
                                .iter()
                                .find(|comp| comp.id == it.id)
                                .and_then(|comp| {
                                    if let crate::elements::Kind::Hd44780(h) = &comp.kind {
                                        Some((h.hex_bitmap(), h.lines().join("\n")))
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_default();
                    let changed = self.readings.get(&it.id).map(|r| (&r.text, &r.extra))
                        != Some((&hex, &lines));
                    if changed {
                        self.dirty.mark_item(&it.id);
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: hex,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: lines,
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::Ssd1306(_) => {
                    let text = self
                        .running
                        .as_ref()
                        .and_then(|c| {
                            c.components()
                                .iter()
                                .find(|comp| comp.id == it.id)
                                .and_then(|comp| {
                                    if let crate::elements::Kind::Ssd1306(s) = &comp.kind {
                                        Some(s.hex_bitmap())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_default();
                    let changed = self.readings.get(&it.id).map(|r| &r.text) != Some(&text);
                    if changed {
                        self.dirty.mark_item(&it.id);
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::Aip31068(_) => {
                    let (hex, lines) = self
                        .running
                        .as_ref()
                        .and_then(|c| {
                            c.components()
                                .iter()
                                .find(|comp| comp.id == it.id)
                                .and_then(|comp| {
                                    if let crate::elements::Kind::Aip31068(a) = &comp.kind {
                                        Some((a.hex_bitmap(), a.lines().join("\n")))
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_default();
                    let changed = self.readings.get(&it.id).map(|r| (&r.text, &r.extra))
                        != Some((&hex, &lines));
                    if changed {
                        self.dirty.mark_item(&it.id);
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: hex,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: lines,
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::Sh1107(_) => {
                    let text = self
                        .running
                        .as_ref()
                        .and_then(|c| {
                            c.components()
                                .iter()
                                .find(|comp| comp.id == it.id)
                                .and_then(|comp| {
                                    if let crate::elements::Kind::Sh1107(s) = &comp.kind {
                                        Some(s.hex_bitmap())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_default();
                    let changed = self.readings.get(&it.id).map(|r| &r.text) != Some(&text);
                    if changed {
                        self.dirty.mark_item(&it.id);
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::Pcd8544(_) => {
                    let text = self
                        .running
                        .as_ref()
                        .and_then(|c| {
                            c.components()
                                .iter()
                                .find(|comp| comp.id == it.id)
                                .and_then(|comp| {
                                    if let crate::elements::Kind::Pcd8544(p) = &comp.kind {
                                        Some(p.hex_bitmap())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_default();
                    let changed = self.readings.get(&it.id).map(|r| &r.text) != Some(&text);
                    if changed {
                        self.dirty.mark_item(&it.id);
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::Ks0108(_) => {
                    let text = self
                        .running
                        .as_ref()
                        .and_then(|c| {
                            c.components()
                                .iter()
                                .find(|comp| comp.id == it.id)
                                .and_then(|comp| {
                                    if let crate::elements::Kind::Ks0108(k) = &comp.kind {
                                        Some(k.hex_bitmap())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_default();
                    let changed = self.readings.get(&it.id).map(|r| &r.text) != Some(&text);
                    if changed {
                        self.dirty.mark_item(&it.id);
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::Oscope(_) => {
                    let freqs = self
                        .running
                        .as_ref()
                        .and_then(|c| c.instruments.scopes.get(&it.id))
                        .map(|s| s.channel_freqs().join(";"))
                        .unwrap_or_else(|| "0 Hz;0 Hz;0 Hz;0 Hz".to_string());
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: freqs,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::LogicAnalyzer(_) => {
                    let freqs = self
                        .running
                        .as_ref()
                        .and_then(|c| c.instruments.las.get(&it.id))
                        .map(|s| s.channel_freqs().join(";"))
                        .unwrap_or_else(|| "0 Hz;0 Hz;0 Hz;0 Hz;0 Hz;0 Hz;0 Hz;0 Hz".to_string());
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: freqs,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::LedBar(p) => {
                    let mut bitmask_str = String::with_capacity(p.segments);
                    for i in 0..p.segments {
                        let vl = with_pin_id_idx(&it.id, PIN_LEFT, i, |pin_l| {
                            self.pin_volts.get(pin_l).copied().unwrap_or(0.0)
                        });
                        let vr = if p.grounded {
                            0.0
                        } else {
                            with_pin_id_idx(&it.id, PIN_RIGHT, i, |pin_r| {
                                self.pin_volts.get(pin_r).copied().unwrap_or(0.0)
                            })
                        };
                        let on = (vl - vr) >= 1.4;
                        bitmask_str.push(if on { '1' } else { '0' });
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: bitmask_str,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::SevenSegment(p) => {
                    let n = p.num_displays.clamp(1, 8);
                    let mut bitmask_str = String::with_capacity(n * 8);
                    let th = if p.threshold > 0.1 { p.threshold } else { 1.4 };
                    let seg_suffixes = [
                        "pin_a", "pin_b", "pin_c", "pin_d", "pin_e", "pin_f", "pin_g", "pin_dot",
                    ];
                    for d in 0..n {
                        let com_ch = (b'a' + d as u8) as char;
                        let v_com = with_pin_id_idx(&it.id, "pin_common", com_ch, |com_id| {
                            self.pin_volts.get(com_id).copied().unwrap_or(0.0)
                        });
                        for s in &seg_suffixes {
                            let v_seg = self.item_pin_voltage(&it.id, s).unwrap_or(0.0);
                            let on = if p.common_anode {
                                (v_com - v_seg) >= th
                            } else {
                                (v_seg - v_com) >= th
                            };
                            bitmask_str.push(if on { '1' } else { '0' });
                        }
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: bitmask_str,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                Part::SevenSegmentBCD(_) => {
                    static BCD_SEGS: [u8; 16] = [
                        0b00111111, 0b00000110, 0b01011011, 0b01001111, 0b01100110, 0b01101101,
                        0b01111101, 0b00000111, 0b01111111, 0b01101111, 0b01110111, 0b01111100,
                        0b00111001, 0b01011110, 0b01111001, 0b01110001,
                    ];
                    let b0 = self.item_pin_voltage(&it.id, PIN_IN0).unwrap_or(0.0) >= 2.0;
                    let b1 = self.item_pin_voltage(&it.id, PIN_IN1).unwrap_or(0.0) >= 2.0;
                    let b2 = self.item_pin_voltage(&it.id, PIN_IN2).unwrap_or(0.0) >= 2.0;
                    let b3 = self.item_pin_voltage(&it.id, PIN_IN3).unwrap_or(0.0) >= 2.0;
                    let en_v = self.item_pin_voltage(&it.id, PIN_IN4);
                    let en_active = en_v.map_or(true, |v| v < 1.5);
                    let dp = self.item_pin_voltage(&it.id, PIN_IN5).unwrap_or(0.0) >= 2.0;

                    let mut digit = if en_active {
                        let idx = (b0 as usize)
                            | ((b1 as usize) << 1)
                            | ((b2 as usize) << 2)
                            | ((b3 as usize) << 3);
                        BCD_SEGS[idx.min(15)]
                    } else {
                        0
                    };
                    if en_active && dp {
                        digit |= 0x80;
                    }

                    let mut bitmask_str = String::with_capacity(8);
                    for bit in 0..8 {
                        bitmask_str.push(if (digit & (1 << bit)) != 0 { '1' } else { '0' });
                    }
                    self.readings.insert(
                        it.id.clone(),
                        crate::instruments::ReadingView {
                            text: bitmask_str,
                            max_text: String::new(),
                            avg_text: String::new(),
                            extra: String::new(),
                            high: false,
                            low: false,
                            hz_text: String::new(),
                        },
                    );
                }
                _ => {}
            }
        }
        let visual_tick = !self.sim_running || self.sim_tick % 3 == 0;
        if visual_tick {
            if let Some(circuit) = self.running.as_ref() {
                if !circuit.instruments.scopes.is_empty() {
                    let new_scope = circuit
                        .instruments
                        .first_scope()
                        .map(|s| {
                            s.traces(
                                self.scope_time_div,
                                self.scope_time_pos,
                                self.scope_trigger,
                                self.scope_trig_level,
                                &crate::instruments::SCOPE_COLORS,
                                None,
                            )
                        })
                        .unwrap_or_else(crate::instruments::empty_scope_traces);
                    if self.scope_traces != new_scope {
                        self.scope_traces = new_scope;
                        plots_changed = true;
                    }
                } else if !self.scope_traces.channels.is_empty() && self.scope_traces.n > 0 {
                    self.scope_traces = crate::instruments::empty_scope_traces();
                    plots_changed = true;
                }

                if !circuit.instruments.las.is_empty() {
                    let la_thr = 0.5 * (self.la_threshold_r + self.la_threshold_f);
                    let new_la = circuit
                        .instruments
                        .first_la()
                        .map(|s| {
                            s.traces(
                                self.la_time_div,
                                self.la_time_pos,
                                self.la_trigger,
                                la_thr,
                                &crate::instruments::LA_COLORS,
                                Some(la_thr),
                            )
                        })
                        .unwrap_or_else(crate::instruments::empty_la_traces);
                    if self.la_traces != new_la {
                        self.la_traces = new_la;
                        plots_changed = true;
                    }
                } else if !self.la_traces.channels.is_empty() && self.la_traces.n > 0 {
                    self.la_traces = crate::instruments::empty_la_traces();
                    plots_changed = true;
                }
            }
        }
        if visual_tick {
            let changed_ids: Vec<String> = self
                .readings
                .iter()
                .filter(|(id, reading)| prev_readings.get(*id) != Some(*reading))
                .map(|(id, _)| id.clone())
                .chain(
                    prev_readings
                        .keys()
                        .filter(|id| !self.readings.contains_key(*id))
                        .cloned(),
                )
                .collect();
            for id in changed_ids {
                self.dirty.mark_item(&id);
            }
            if plots_changed {
                self.dirty_plot_items();
            }
        }
        Change {
            plots: plots_changed,
            ..Change::default()
        }
    }

    fn dirty_plot_items(&mut self) {
        let plot_ids: Vec<String> = self
            .scene
            .items()
            .iter()
            .filter(|it| matches!(&it.kind, Part::Oscope(_) | Part::LogicAnalyzer(_)))
            .map(|it| it.id.clone())
            .collect();
        for id in plot_ids {
            self.dirty.mark_item(&id);
        }
    }

    /// C++ Probe::updateStep: an unconnected tip reads the pin or wire under it.
    pub(crate) fn apply_probe_hover(&mut self) {
        let probes: Vec<(String, String, f64, f64)> = self
            .scene
            .items()
            .iter()
            .filter_map(|it| {
                if !matches!(&it.kind, Part::Probe(_)) {
                    return None;
                }
                let pin = with_pin_id(&it.id, PIN_INPIN, |pin_id| {
                    if self.scene.pin_connected(pin_id) {
                        return None;
                    }
                    it.pins().into_iter().find(|p| p.id == pin_id)
                })?;
                let tip = it.pin_scene_pos(&pin);
                Some((it.id.clone(), pin.id, tip.x, tip.y))
            })
            .collect();
        for (id, pin_id, x, y) in probes {
            let tip = Point::new(x, y);
            let v = self.hover_voltage_at(tip, &pin_id).unwrap_or(0.0);
            self.pin_volts.insert(pin_id, v);
            let _ = id;
        }
    }

    pub(crate) fn hover_voltage_at(&self, tip: Point, except_pin: &str) -> Option<f64> {
        if let Some(p) = self.scene.hit_pin(tip) {
            if p.id != except_pin {
                return self.pin_volts.get(&p.id).copied();
            }
        }
        if let Some(widx) = self.scene.hit_wire(tip) {
            let start = &self.scene.wires()[widx].start_pin;
            return self.pin_volts.get(start).copied();
        }
        Some(0.0)
    }

    pub fn readings(&self) -> &std::collections::HashMap<String, crate::instruments::ReadingView> {
        &self.readings
    }

    pub fn set_reading(&mut self, id: impl Into<String>, reading: crate::instruments::ReadingView) {
        self.readings.insert(id.into(), reading);
    }

    pub fn scope_traces(&self) -> &crate::plot::PlotBuffer {
        &self.scope_traces
    }

    /// Live samples for the on-canvas oscope screen. `None` when this item
    /// has no sampler (idle / not in the running circuit).
    pub fn live_scope_traces(&self, item_id: &str) -> Option<crate::plot::PlotBuffer> {
        let circuit = self.running.as_ref()?;
        let sampler = circuit.instruments.scopes.get(item_id)?;
        Some(sampler.traces_with_count(
            self.scope_time_div,
            self.scope_time_pos,
            self.scope_trigger,
            self.scope_trig_level,
            &crate::instruments::SCOPE_COLORS,
            None,
            96,
        ))
    }

    /// Live samples for the on-canvas logic analyzer screen. `None` when this item
    /// has no sampler (idle / not in the running circuit).
    pub fn live_la_traces(&self, item_id: &str) -> Option<crate::plot::PlotBuffer> {
        let circuit = self.running.as_ref()?;
        let sampler = circuit.instruments.las.get(item_id)?;
        let la_thr = 0.5 * (self.la_threshold_r + self.la_threshold_f);
        Some(sampler.traces_with_count(
            self.la_time_div,
            self.la_time_pos,
            self.la_trigger,
            la_thr,
            &crate::instruments::LA_COLORS,
            Some(la_thr),
            96,
        ))
    }

    pub fn la_traces(&self) -> &crate::plot::PlotBuffer {
        &self.la_traces
    }

    pub fn set_scope_time_div(&mut self, v: f64) -> Change {
        self.scope_time_div = v.max(1e-12);
        self.publish_instrument_views()
    }

    pub fn set_scope_time_pos(&mut self, v: f64) -> Change {
        self.scope_time_pos = v;
        self.publish_instrument_views()
    }

    pub fn set_scope_volt_div(&mut self, ch: usize, v: f64) -> Change {
        if ch < 4 {
            self.scope_volt_div[ch] = v.max(1e-12);
        }
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_scope_volt_pos(&mut self, ch: usize, v: f64) -> Change {
        if ch < 4 {
            self.scope_volt_pos[ch] = v;
        }
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_scope_tracks(&mut self, tracks: i32) -> Change {
        self.scope_tracks = match tracks {
            2 => 2,
            4 => 4,
            _ => 1,
        };
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_scope_hidden(&mut self, ch: usize, hide: bool) -> Change {
        if ch < 4 {
            self.scope_hidden[ch] = hide;
        }
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_scope_trigger(&mut self, ch: i32) -> Change {
        self.scope_trigger = ch;
        self.publish_instrument_views()
    }

    pub fn set_scope_trig_level(&mut self, level: f64) -> Change {
        self.scope_trig_level = level;
        self.publish_instrument_views()
    }

    pub fn set_scope_filter(&mut self, v: f64) -> Change {
        self.scope_filter = v.max(0.0);
        Change::default()
    }

    pub fn set_la_time_div(&mut self, v: f64) -> Change {
        self.la_time_div = v.max(1e-12);
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_la_time_pos(&mut self, v: f64) -> Change {
        self.la_time_pos = v;
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_la_trigger(&mut self, ch: i32) -> Change {
        self.la_trigger = ch;
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn set_la_thresholds(&mut self, rise: f64, fall: f64) -> Change {
        self.la_threshold_r = rise;
        self.la_threshold_f = fall;
        let c = self.publish_instrument_views();
        self.dirty_plot_items();
        c
    }

    pub fn scope_time_div(&self) -> f64 {
        self.scope_time_div
    }

    pub fn scope_time_pos(&self) -> f64 {
        self.scope_time_pos
    }

    pub fn scope_volt_div(&self) -> [f64; 4] {
        self.scope_volt_div
    }

    pub fn scope_volt_pos(&self) -> [f64; 4] {
        self.scope_volt_pos
    }

    pub fn scope_tracks(&self) -> i32 {
        self.scope_tracks
    }

    pub fn scope_hidden(&self) -> [bool; 4] {
        self.scope_hidden
    }

    pub fn scope_trigger(&self) -> i32 {
        self.scope_trigger
    }

    pub fn scope_trig_level(&self) -> f64 {
        self.scope_trig_level
    }

    pub fn auto_scale_scope(&self, ch: usize) -> Option<crate::instruments::AutoScaleResult> {
        if let Some(scope) = self
            .running
            .as_ref()
            .and_then(|c| c.instruments.first_scope())
        {
            if let Some(res) = scope.auto_scale(ch) {
                return Some(res);
            }
        }
        if let Some(channel) = self.scope_traces.channels.get(ch) {
            if channel.samples.len() >= 2 && channel.connected {
                let mut min_v = f64::INFINITY;
                let mut max_v = f64::NEG_INFINITY;
                for &v in &channel.samples {
                    if v < min_v {
                        min_v = v;
                    }
                    if v > max_v {
                        max_v = v;
                    }
                }
                let ampli = max_v - min_v;
                if ampli > 1e-6 {
                    let mid = min_v + ampli * 0.5;
                    let volt_pos = mid;
                    let raw_volt_div = ampli / 8.0;
                    let volt_div = crate::instruments::snap_to_step(
                        raw_volt_div,
                        &crate::instruments::SCOPE_VOLT_DIV_STEPS,
                    );
                    return Some(crate::instruments::AutoScaleResult {
                        volt_div,
                        volt_pos,
                        time_div: None,
                    });
                } else {
                    let volt_pos = max_v;
                    let raw_volt_div = if max_v.abs() > 1e-6 {
                        max_v.abs() / 4.0
                    } else {
                        1.0
                    };
                    let volt_div = crate::instruments::snap_to_step(
                        raw_volt_div,
                        &crate::instruments::SCOPE_VOLT_DIV_STEPS,
                    );
                    return Some(crate::instruments::AutoScaleResult {
                        volt_div,
                        volt_pos,
                        time_div: None,
                    });
                }
            }
        }
        None
    }

    pub fn auto_scale_scope_all(&self) -> crate::instruments::AutoScaleAllResult {
        if let Some(scope) = self
            .running
            .as_ref()
            .and_then(|c| c.instruments.first_scope())
        {
            return scope.auto_scale_all();
        }
        let mut results = Vec::new();
        for ch in 0..4 {
            results.push(self.auto_scale_scope(ch));
        }
        crate::instruments::AutoScaleAllResult {
            channels: results,
            time_div: None,
        }
    }

    pub fn la_time_div(&self) -> f64 {
        self.la_time_div
    }

    pub fn la_time_pos(&self) -> f64 {
        self.la_time_pos
    }

    pub fn la_trigger(&self) -> i32 {
        self.la_trigger
    }

    pub fn la_threshold_r(&self) -> f64 {
        self.la_threshold_r
    }

    pub fn la_threshold_f(&self) -> f64 {
        self.la_threshold_f
    }
}

fn compute_wire_currents(
    scene: &crate::canvas::Scene,
    circuit: &crate::Circuit,
) -> std::collections::HashMap<String, f64> {
    use crate::canvas::scene::wires::pin_belongs;
    use rustc_hash::{FxHashMap, FxHashSet};

    let mut result = std::collections::HashMap::new();

    let node_ids: FxHashSet<String> = scene
        .items()
        .iter()
        .filter(|it| it.is_node())
        .map(|it| it.id.clone())
        .collect();

    let pin_to_vertex = |pin_id: &str| -> String {
        for nid in &node_ids {
            if pin_belongs(pin_id, std::slice::from_ref(nid)) {
                return format!("node:{nid}");
            }
        }
        format!("pin:{pin_id}")
    };

    let mut vertex_to_idx: FxHashMap<String, usize> = FxHashMap::default();
    let mut vertex_keys: Vec<String> = Vec::new();

    let mut get_vertex = |key: String| -> usize {
        if let Some(&idx) = vertex_to_idx.get(&key) {
            idx
        } else {
            let idx = vertex_keys.len();
            vertex_to_idx.insert(key.clone(), idx);
            vertex_keys.push(key);
            idx
        }
    };

    let mut edges: Vec<(String, usize, usize)> = Vec::new();
    for w in scene.wires() {
        if w.closed() {
            if let Some(end_pin) = &w.end_pin {
                let u = get_vertex(pin_to_vertex(&w.start_pin));
                let v = get_vertex(pin_to_vertex(end_pin));
                edges.push((w.id.clone(), u, v));
            }
        }
    }

    let num_v = vertex_keys.len();
    if num_v == 0 || edges.is_empty() {
        return result;
    }

    let mut inj_current = vec![0.0; num_v];
    for (i, key) in vertex_keys.iter().enumerate() {
        if let Some(pin_id) = key.strip_prefix("pin:") {
            inj_current[i] = circuit.current_out_of_pin(pin_id).unwrap_or(0.0);
        }
    }

    // Union-Find to partition into connected components
    let mut parent: Vec<usize> = (0..num_v).collect();
    fn find(p: &mut [usize], i: usize) -> usize {
        if p[i] != i {
            let root = find(p, p[i]);
            p[i] = root;
        }
        p[i]
    }
    for &(_, u, v) in &edges {
        let root_u = find(&mut parent, u);
        let root_v = find(&mut parent, v);
        if root_u != root_v {
            parent[root_u] = root_v;
        }
    }

    let mut comp_verts: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    for i in 0..num_v {
        let root = find(&mut parent, i);
        comp_verts.entry(root).or_default().push(i);
    }

    let mut comp_edges: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    for (e_idx, &(_, u, _)) in edges.iter().enumerate() {
        let root = find(&mut parent, u);
        comp_edges.entry(root).or_default().push(e_idx);
    }

    for (root, verts) in comp_verts {
        let Some(c_edges) = comp_edges.get(&root) else {
            continue;
        };
        let nv = verts.len();
        if nv < 2 || c_edges.is_empty() {
            continue;
        }

        if nv == 2 && c_edges.len() == 1 {
            let (wire_id, u, v) = &edges[c_edges[0]];
            let i_u = inj_current[*u];
            let i_v = inj_current[*v];
            let curr = if i_u.abs() > 1e-12 {
                i_u
            } else if i_v.abs() > 1e-12 {
                -i_v
            } else {
                0.0
            };
            if curr.abs() > 1e-12 {
                result.insert(wire_id.clone(), curr);
            }
            continue;
        }

        let mut local_map = FxHashMap::default();
        for (local_i, &v) in verts.iter().enumerate() {
            local_map.insert(v, local_i);
        }

        let mut b = vec![0.0; nv];
        for (local_i, &v) in verts.iter().enumerate() {
            b[local_i] = inj_current[v];
        }

        let sum_b: f64 = b.iter().sum();
        let mean_b = sum_b / (nv as f64);
        for val in &mut b {
            *val -= mean_b;
        }

        let mut l = vec![vec![0.0; nv]; nv];
        for &e_idx in c_edges {
            let (_, u, v) = &edges[e_idx];
            let lu = local_map[u];
            let lv = local_map[v];
            if lu != lv {
                l[lu][lu] += 1.0;
                l[lv][lv] += 1.0;
                l[lu][lv] -= 1.0;
                l[lv][lu] -= 1.0;
            }
        }

        let m = nv - 1;
        let mut a = vec![vec![0.0; m]; m];
        let mut rhs = vec![0.0; m];
        for i in 0..m {
            rhs[i] = b[i];
            for j in 0..m {
                a[i][j] = l[i][j];
            }
        }

        if let Some(x) = solve_linear(&a, &rhs) {
            let mut v_pot = vec![0.0; nv];
            for i in 0..m {
                v_pot[i] = x[i];
            }
            v_pot[nv - 1] = 0.0;
            for &e_idx in c_edges {
                let (wire_id, u, v) = &edges[e_idx];
                let lu = local_map[u];
                let lv = local_map[v];
                let curr = v_pot[lu] - v_pot[lv];
                if curr.abs() > 1e-12 {
                    result.insert(wire_id.clone(), curr);
                }
            }
        } else {
            for &e_idx in c_edges {
                let (wire_id, u, v) = &edges[e_idx];
                let i_u = inj_current[*u];
                let i_v = inj_current[*v];
                let curr = if i_u.abs() > 1e-12 {
                    i_u
                } else if i_v.abs() > 1e-12 {
                    -i_v
                } else {
                    0.0
                };
                if curr.abs() > 1e-12 {
                    result.insert(wire_id.clone(), curr);
                }
            }
        }
    }

    result
}

fn solve_linear(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    if n == 0 {
        return Some(Vec::new());
    }
    let mut mat: Vec<Vec<f64>> = a.iter().map(|row| row[..n].to_vec()).collect();
    let mut rhs = b.to_vec();

    for i in 0..n {
        let mut max_row = i;
        let mut max_val = mat[i][i].abs();
        for k in (i + 1)..n {
            if mat[k][i].abs() > max_val {
                max_val = mat[k][i].abs();
                max_row = k;
            }
        }
        if max_val < 1e-12 {
            return None;
        }
        mat.swap(i, max_row);
        rhs.swap(i, max_row);

        for k in (i + 1)..n {
            let factor = mat[k][i] / mat[i][i];
            rhs[k] -= factor * rhs[i];
            for j in i..n {
                mat[k][j] -= factor * mat[i][j];
            }
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = rhs[i];
        for j in (i + 1)..n {
            sum -= mat[i][j] * x[j];
        }
        x[i] = sum / mat[i][i];
    }
    Some(x)
}
