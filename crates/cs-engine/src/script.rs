//! Scripted MCU host (C++ `ScriptCpu` + `IoPort` / `IoPin` / `McuPort` / `McuPin`
//! / `Uart` / `SPI` / `TWI` AngelScript APIs).
//!
//! Not drawn (catalog-last).

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::Once;

use cs_script::{Engine, McuApi};

use crate::digital::{
    IoPin, IoPort, PinAction, PinMode, SpiMode, SpiModule, SpiPins, TwiMode, TwiModule, TwiPins,
    TwiState, UsartModule,
};

static INSTALL: Once = Once::new();

fn install_api() {
    INSTALL.call_once(|| {
        crate::sim_log::init_script_logging();
        Engine::set_mcu_api(&McuApi {
            iopin_set_pin_mode: Some(iopin_set_pin_mode),
            iopin_get_inp_state: Some(iopin_get_inp_state),
            iopin_set_out_state: Some(iopin_set_out_state),
            iopin_set_state_z: Some(iopin_set_state_z),
            iopin_set_out_stat_fast: Some(iopin_set_out_stat_fast),
            iopin_schedule_state: Some(iopin_schedule_state),
            iopin_get_voltage: Some(iopin_get_voltage),
            iopin_set_voltage: Some(iopin_set_voltage),
            iopin_set_out_high_v: Some(iopin_set_out_high_v),
            iopin_set_impedance: Some(iopin_set_impedance),
            ioport_set_pin_mode: Some(ioport_set_pin_mode),
            ioport_get_inp_state: Some(ioport_get_inp_state),
            ioport_set_out_state: Some(ioport_set_out_state),
            ioport_schedule_state: Some(ioport_schedule_state),
            ioport_trigger: Some(ioport_trigger),
            cpu_get_pin: Some(cpu_get_pin),
            cpu_get_port: Some(cpu_get_port),
            cpu_circ_time: Some(cpu_circ_time),
            cpu_add_event: Some(cpu_add_event),
            cpu_cancel_events: Some(cpu_cancel_events),
            cpu_read_ram: Some(cpu_read_ram),
            cpu_write_ram: Some(cpu_write_ram),
            cpu_read_pgm: Some(cpu_read_pgm),
            cpu_write_pgm: Some(cpu_write_pgm),
            mcupin_set_direction: Some(mcupin_set_direction),
            mcupin_set_port_state: Some(mcupin_set_port_state),
            mcupin_control_pin: Some(mcupin_control_pin),
            mcupin_set_ext_int: Some(mcupin_set_ext_int),
            mcupin_set_out_state: Some(mcupin_set_out_state),
            mcuport_control_port: Some(mcuport_control_port),
            mcuport_set_direction: Some(mcuport_set_direction),
            mcuport_set_out_state: Some(mcuport_set_out_state),
            cpu_get_mcu_pin: Some(cpu_get_pin),
            cpu_get_mcu_port: Some(cpu_get_port),
            uart_set_baud: Some(uart_set_baud),
            uart_set_data_bits: Some(uart_set_data_bits),
            uart_send_byte: Some(uart_send_byte),
            spi_set_mode: Some(spi_set_mode),
            spi_send_byte: Some(spi_send_byte),
            twi_set_mode: Some(twi_set_mode),
            twi_send_byte: Some(twi_send_byte),
            twi_set_address: Some(twi_set_address),
        });
    });
}

/// C++ `ScriptUsart`.
pub struct ScriptUsart {
    pub name: String,
    pub module: UsartModule,
    tx: Option<usize>,
    rx: Option<usize>,
    cpu: *mut ScriptCpu,
}

/// C++ `ScriptSpi`.
pub struct ScriptSpi {
    pub name: String,
    pub module: SpiModule,
    pins: SpiPins,
    cpu: *mut ScriptCpu,
}

/// C++ `ScriptTwi`.
pub struct ScriptTwi {
    pub name: String,
    pub module: TwiModule,
    pins: TwiPins,
    cpu: *mut ScriptCpu,
}

/// Scripted MCU instance (C++ `ScriptCpu`).
pub struct ScriptCpu {
    pub id: String,
    source: String,
    engine: Option<Engine>,
    pub ports: Vec<IoPort>,
    pub pins: Vec<IoPin>,
    ram: Vec<u8>,
    pgm: Vec<u16>,
    pub circ_time: u64,
    pending_event: Option<u64>,
    cancel_events: bool,
    has_reset: bool,
    has_run_event: bool,
    has_run_step: bool,
    has_volt_changed: bool,
    /// Picoseconds per `runStep` (C++ `cyclesDone * psTick`). Default 1 µs.
    pub ps_tick: u64,
    pub usarts: Vec<ScriptUsart>,
    pub spis: Vec<ScriptSpi>,
    pub twis: Vec<ScriptTwi>,
    has_byte_received: bool,
    has_frame_sent: bool,
    has_slave_write: bool,
}

impl Clone for ScriptCpu {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            source: self.source.clone(),
            engine: None,
            ports: self.ports.clone(),
            pins: self.pins.clone(),
            ram: self.ram.clone(),
            pgm: self.pgm.clone(),
            circ_time: self.circ_time,
            pending_event: None,
            cancel_events: false,
            has_reset: false,
            has_run_event: false,
            has_run_step: false,
            has_volt_changed: false,
            ps_tick: self.ps_tick,
            usarts: self
                .usarts
                .iter()
                .map(|u| ScriptUsart {
                    name: u.name.clone(),
                    module: u.module.clone(),
                    tx: u.tx,
                    rx: u.rx,
                    cpu: std::ptr::null_mut(),
                })
                .collect(),
            spis: self
                .spis
                .iter()
                .map(|s| ScriptSpi {
                    name: s.name.clone(),
                    module: s.module.clone(),
                    pins: s.pins,
                    cpu: std::ptr::null_mut(),
                })
                .collect(),
            twis: self
                .twis
                .iter()
                .map(|t| ScriptTwi {
                    name: t.name.clone(),
                    module: t.module.clone(),
                    pins: t.pins,
                    cpu: std::ptr::null_mut(),
                })
                .collect(),
            has_byte_received: false,
            has_frame_sent: false,
            has_slave_write: false,
        }
    }
}

impl std::fmt::Debug for ScriptCpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptCpu")
            .field("id", &self.id)
            .field("ports", &self.ports.len())
            .field("pins", &self.pins.len())
            .field("usarts", &self.usarts.len())
            .field("spis", &self.spis.len())
            .field("twis", &self.twis.len())
            .finish_non_exhaustive()
    }
}

impl ScriptCpu {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: String::new(),
            engine: None,
            ports: Vec::new(),
            pins: Vec::new(),
            ram: vec![0; 256],
            pgm: vec![0; 64],
            circ_time: 1,
            pending_event: None,
            cancel_events: false,
            has_reset: false,
            has_run_event: false,
            has_run_step: false,
            has_volt_changed: false,
            ps_tick: 1_000_000,
            usarts: Vec::new(),
            spis: Vec::new(),
            twis: Vec::new(),
            has_byte_received: false,
            has_frame_sent: false,
            has_slave_write: false,
        }
    }

    fn pin_index(&self, name: &str) -> Option<usize> {
        if name.is_empty() {
            return None;
        }
        let mut i = 0;
        for p in &self.pins {
            if pin_matches(&p.id, name) {
                return Some(i);
            }
            i += 1;
        }
        for port in &self.ports {
            for p in &port.pins {
                if pin_matches(&p.id, name) || pin_name_on_port(&p.id, &port.name, name) {
                    return Some(i);
                }
                i += 1;
            }
        }
        None
    }

    fn take_flat_pins(&mut self) -> (Vec<IoPin>, Vec<usize>) {
        let mut all = std::mem::take(&mut self.pins);
        let mut lens = Vec::with_capacity(self.ports.len());
        for port in &mut self.ports {
            lens.push(port.pins.len());
            all.append(&mut port.pins);
        }
        (all, lens)
    }

    fn restore_flat_pins(&mut self, mut all: Vec<IoPin>, lens: &[usize]) {
        for (i, port) in self.ports.iter_mut().enumerate().rev() {
            let n = *lens.get(i).unwrap_or(&0);
            let split = all.len().saturating_sub(n);
            port.pins = all.split_off(split);
        }
        self.pins = all;
    }

    fn with_flat_pins<R>(&mut self, f: impl FnOnce(&mut Vec<IoPin>) -> R) -> R {
        let (mut all, lens) = self.take_flat_pins();
        let r = f(&mut all);
        self.restore_flat_pins(all, &lens);
        r
    }

    /// C++ `McuCreator` scripted core: `<port>`/`<ioport>` plus `<usart>`/`<spi>`/`<twi>`
    /// globals, then the `.as` from `script=""`.
    pub fn from_mcu_desc(id: &str, desc: &cs_mcu::McuDesc, script: impl Into<String>) -> Self {
        let mut cpu = Self::new(id);
        if desc.data_size > 0 {
            cpu.ram = vec![0; desc.data_size as usize];
        }
        if desc.prog_size > 0 {
            cpu.pgm = vec![0; desc.prog_size as usize];
        }
        if desc.freq > 0.0 {
            let c_per = if desc.cpu_cycle > 0.0 {
                desc.cpu_cycle
            } else {
                desc.inst_cycle.max(1.0)
            };
            cpu.ps_tick = ((1e12 * c_per / desc.freq).round() as u64).max(1);
        }
        for spec in &desc.ports {
            cpu.ports.push(io_port_from_spec(id, spec));
        }
        for u in &desc.usarts {
            let tx =
                u.tx.as_ref()
                    .and_then(|t| t.pins.first())
                    .map(String::as_str)
                    .unwrap_or("");
            let rx =
                u.rx.as_ref()
                    .and_then(|t| t.pins.first())
                    .map(String::as_str)
                    .unwrap_or("");
            cpu.add_uart(&u.name, tx, rx);
        }
        for s in &desc.spis {
            cpu.add_spi(
                &s.name,
                s.pins.first().map(String::as_str).unwrap_or(""),
                s.pins.get(1).map(String::as_str).unwrap_or(""),
                s.pins.get(2).map(String::as_str).unwrap_or(""),
                s.pins.get(3).map(String::as_str).unwrap_or(""),
            );
        }
        for t in &desc.twis {
            // C++ `pins.value(0)` = SDA, `pins.value(1)` = SCL.
            let sda = t.pins.first().map(String::as_str).unwrap_or("");
            let scl = t.pins.get(1).map(String::as_str).unwrap_or("");
            cpu.add_twi(&t.name, scl, sda);
        }
        cpu.set_script(script);
        cpu
    }

    /// C++ `McuCreator` `<usart>` for a scripted core: global `Uart {name}`.
    pub fn add_uart(&mut self, name: impl Into<String>, tx: &str, rx: &str) {
        let mut module = UsartModule::new();
        module.enable_tx(true, &mut [], None);
        module.enable_rx(true);
        self.usarts.push(ScriptUsart {
            name: name.into(),
            module,
            tx: self.pin_index(tx),
            rx: self.pin_index(rx),
            cpu: std::ptr::null_mut(),
        });
    }

    /// C++ `McuCreator` `<spi>`: global `SPI {name}`.
    pub fn add_spi(
        &mut self,
        name: impl Into<String>,
        mosi: &str,
        miso: &str,
        clk: &str,
        ss: &str,
    ) {
        self.spis.push(ScriptSpi {
            name: name.into(),
            module: SpiModule::new(),
            pins: SpiPins {
                mosi: self.pin_index(mosi),
                miso: self.pin_index(miso),
                clk: self.pin_index(clk),
                ss: self.pin_index(ss),
            },
            cpu: std::ptr::null_mut(),
        });
    }

    /// C++ `McuCreator` `<twi>`: global `TWI {name}`.
    pub fn add_twi(&mut self, name: impl Into<String>, scl: &str, sda: &str) {
        self.twis.push(ScriptTwi {
            name: name.into(),
            module: TwiModule::new(),
            pins: TwiPins {
                scl: self.pin_index(scl),
                sda: self.pin_index(sda),
            },
            cpu: std::ptr::null_mut(),
        });
    }

    pub fn with_port(mut self, port: IoPort) -> Self {
        self.ports.push(port);
        self
    }

    pub fn with_pin(mut self, pin: IoPin) -> Self {
        self.pins.push(pin);
        self
    }

    pub fn set_script(&mut self, source: impl Into<String>) {
        self.source = source.into();
        self.engine = None;
    }

    pub fn pin_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.pins.iter().map(|p| p.id.clone()).collect();
        for port in &self.ports {
            ids.extend(port.pins.iter().map(|p| p.id.clone()));
        }
        ids
    }

    pub fn pins_iter_mut(&mut self) -> impl Iterator<Item = &mut IoPin> {
        self.pins
            .iter_mut()
            .chain(self.ports.iter_mut().flat_map(|p| p.pins.iter_mut()))
    }

    pub fn compile(&mut self) -> crate::Result<()> {
        install_api();
        let ptr = self as *mut Self;
        let mut engine = Engine::new().map_err(|e| crate::Error::Parse(e.to_string()))?;
        engine
            .register_mcu(ptr as *mut c_void)
            .map_err(|e| crate::Error::Parse(e.to_string()))?;
        unsafe {
            for u in &mut (*ptr).usarts {
                u.cpu = ptr;
                let decl = format!("Uart {}", u.name);
                engine
                    .register_global(&decl, (u as *mut ScriptUsart).cast())
                    .map_err(|e| crate::Error::Parse(e.to_string()))?;
            }
            for s in &mut (*ptr).spis {
                s.cpu = ptr;
                let decl = format!("SPI {}", s.name);
                engine
                    .register_global(&decl, (s as *mut ScriptSpi).cast())
                    .map_err(|e| crate::Error::Parse(e.to_string()))?;
            }
            for t in &mut (*ptr).twis {
                t.cpu = ptr;
                let decl = format!("TWI {}", t.name);
                engine
                    .register_global(&decl, (t as *mut ScriptTwi).cast())
                    .map_err(|e| crate::Error::Parse(e.to_string()))?;
            }
        }
        if !self.source.is_empty() {
            crate::logging::log_sim("Compiling AngelScript MCU module...");
            if let Err(e) = engine.compile("mcu", &self.source) {
                crate::logging::log_sim(format!("Script error: {e}"));
                return Err(crate::Error::Parse(e.to_string()));
            }
            crate::logging::log_sim("AngelScript MCU module compiled successfully.");
        }
        self.has_reset = engine.has_function("void reset()");
        self.has_run_event = engine.has_function("void runEvent()");
        self.has_run_step = engine.has_function("void runStep()");
        self.has_volt_changed = engine.has_function("void voltChanged()");
        self.has_byte_received = engine.has_function("void byteReceived( uint d )");
        self.has_frame_sent = engine.has_function("void frameSent( uint data )");
        self.has_slave_write = engine.has_function("uint slaveWrite()");
        self.engine = Some(engine);
        Ok(())
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        for p in self
            .pins
            .iter_mut()
            .chain(self.ports.iter_mut().flat_map(|po| po.pins.iter_mut()))
        {
            p.initialize(slope_steps);
        }
        for po in &mut self.ports {
            po.reset();
        }
        if self.engine.is_none() && !self.source.is_empty() {
            let _ = self.compile();
        }
        let ptr = self as *mut Self;
        self.with_flat_pins(|pins| unsafe {
            for u in &mut (*ptr).usarts {
                let tx = u.tx;
                u.module.enable_tx(true, pins, tx);
                u.module.enable_rx(true);
            }
        });
        if self.has_reset {
            let _ = self.call_void("void reset()");
        }
        if let Some(engine) = &mut self.engine {
            if engine.has_function("void setup()") {
                let _ = engine.call_void0("void setup()");
            }
        }
    }

    fn call_void(&mut self, decl: &str) -> crate::Result<()> {
        let Some(engine) = self.engine.as_mut() else {
            return Ok(());
        };
        engine
            .call_void0(decl)
            .map_err(|e| crate::Error::Parse(e.to_string()))
    }

    pub fn sample_inputs(&mut self, v_of: &impl Fn(&str) -> f64) {
        for p in self
            .pins
            .iter_mut()
            .chain(self.ports.iter_mut().flat_map(|po| po.pins.iter_mut()))
        {
            let v = v_of(&p.id);
            let _ = p.get_inp_state(v);
        }
    }

    pub fn run_event(&mut self, v_of: &impl Fn(&str) -> f64) -> Vec<(usize, PinAction)> {
        self.sample_inputs(v_of);
        self.tick_peripherals();
        if self.has_run_event {
            let _ = self.call_void("void runEvent()");
        } else if self.has_run_step {
            let _ = self.call_void("void runStep()");
        }
        Vec::new()
    }

    pub fn volt_changed(&mut self, v_of: &impl Fn(&str) -> f64) {
        self.sample_inputs(v_of);
        self.sample_uart_rx();
        self.tick_slave_peripherals();
        if self.has_volt_changed {
            let _ = self.call_void("void voltChanged()");
        }
    }

    pub fn take_event(&mut self) -> Option<u64> {
        if self.cancel_events {
            self.cancel_events = false;
            self.pending_event = None;
        }
        let proto = self.protocol_remain();
        match (self.pending_event, proto) {
            (Some(a), Some(b)) if a <= b => {
                self.pending_event = None;
                Some(a)
            }
            (Some(a), Some(b)) => {
                self.pending_event = Some(a.saturating_sub(b));
                Some(b)
            }
            (Some(a), None) => {
                self.pending_event = None;
                Some(a)
            }
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    fn protocol_remain(&self) -> Option<u64> {
        let now = self.circ_time;
        let mut best: Option<u64> = None;
        let take = |best: &mut Option<u64>, v: Option<u64>| {
            if let Some(v) = v {
                *best = Some(best.map_or(v, |b| b.min(v)));
            }
        };
        for u in &self.usarts {
            take(&mut best, u.module.remain(now));
        }
        for s in &self.spis {
            take(&mut best, s.module.remain(now));
        }
        for t in &self.twis {
            take(&mut best, t.module.remain(now));
        }
        best
    }

    fn tick_peripherals(&mut self) {
        let now = self.circ_time;
        let ptr = self as *mut Self;

        // Drain pending host send bytes into ScriptUsart
        unsafe {
            for (i, u) in (*ptr).usarts.iter_mut().enumerate() {
                let port_id = format!("{}:{}", (*ptr).id, u.name);
                let mut pending_rx = crate::serial::take_send(&port_id);
                if i == 0 {
                    pending_rx.extend(crate::serial::take_send(&(*ptr).id));
                    pending_rx.extend(crate::serial::take_send("default"));
                }
                for b in pending_rx {
                    crate::serial::publish_in(&port_id, b);
                    if i == 0 {
                        crate::serial::publish_in(&(*ptr).id, b);
                        crate::serial::publish_in("default", b);
                    }
                    (*ptr).call_script_u("void byteReceived( uint d )", u32::from(b));
                }
            }
        }

        let mut uart_out = Vec::new();
        let mut spi_trans = Vec::new();
        let mut twi_ticks = Vec::new();
        self.with_flat_pins(|pins| unsafe {
            for (i, u) in (*ptr).usarts.iter_mut().enumerate() {
                uart_out.push((i, u.name.clone(), u.module.tick(now, pins, u.tx, u.rx)));
            }
            for (i, s) in (*ptr).spis.iter_mut().enumerate() {
                if s.module.due.is_some_and(|t| t <= now) && s.module.run_event(now, pins, s.pins) {
                    spi_trans.push((i, s.name.clone(), s.module.tx_reg, s.module.data_reg));
                }
            }
            for (i, t) in (*ptr).twis.iter_mut().enumerate() {
                if t.module.due.is_some_and(|d| d <= now) {
                    twi_ticks.push((i, t.name.clone(), t.module.run_event(now, pins, t.pins)));
                }
            }
        });
        for (i, name, t) in uart_out {
            let port_id = format!("{}:{}", self.id, name);
            if let Some(b) = t.frame_sent {
                crate::serial::publish_out(&port_id, b);
                if i == 0 {
                    crate::serial::publish_out(&self.id, b);
                    crate::serial::publish_out("default", b);
                }
                self.call_script_u("void frameSent( uint data )", u32::from(b));
            }
            if let Some(b) = t.byte_received {
                crate::serial::publish_in(&port_id, b);
                if i == 0 {
                    crate::serial::publish_in(&self.id, b);
                    crate::serial::publish_in("default", b);
                }
                self.call_script_u("void byteReceived( uint d )", u32::from(b));
            }
        }
        for (_i, name, tx_b, rx_b) in spi_trans {
            let port_id = format!("{}:{}", self.id, name);
            crate::serial::publish_out(&port_id, tx_b);
            crate::serial::publish_in(&port_id, rx_b);
            self.call_script_u("void byteReceived( uint d )", u32::from(rx_b));
        }
        for (i, _name, tick) in twi_ticks {
            let port_id = format!("{}:{}", self.id, _name);
            if let Some(b) = tick.byte_sent {
                crate::serial::publish_out(&port_id, b);
            }
            if let Some(b) = tick.byte_received {
                crate::serial::publish_in(&port_id, b);
            }
            self.apply_twi_tick(i, tick);
        }
    }

    fn sample_uart_rx(&mut self) {
        let now = self.circ_time;
        let ptr = self as *mut Self;
        self.with_flat_pins(|pins| unsafe {
            for u in &mut (*ptr).usarts {
                u.module.rx_pin_changed(now, pins, u.rx);
            }
        });
    }

    fn tick_slave_peripherals(&mut self) {
        let now = self.circ_time;
        let ptr = self as *mut Self;
        let mut spi_bytes = Vec::new();
        let mut twi_ticks = Vec::new();
        self.with_flat_pins(|pins| unsafe {
            for s in &mut (*ptr).spis {
                if s.module.volt_changed(now, pins, s.pins) {
                    spi_bytes.push(s.module.sr_reg);
                }
            }
            for (i, t) in (*ptr).twis.iter_mut().enumerate() {
                twi_ticks.push((i, t.module.volt_changed(pins, t.pins)));
            }
        });
        for b in spi_bytes {
            self.call_script_u("void byteReceived( uint d )", u32::from(b));
        }
        for (i, tick) in twi_ticks {
            self.apply_twi_tick(i, tick);
        }
    }

    fn apply_twi_tick(&mut self, i: usize, tick: crate::digital::TwiTick) {
        if let Some(b) = tick.byte_received {
            self.call_script_u("void byteReceived( uint d )", u32::from(b));
        }
        if tick.need_slave_write {
            let data = self.call_slave_write();
            if let Some(t) = self.twis.get_mut(i) {
                t.module.tx_reg = data;
            }
        }
        let _ = tick.state;
    }

    fn call_script_u(&mut self, decl: &str, arg: u32) {
        let want = if decl.starts_with("void frameSent") {
            self.has_frame_sent
        } else {
            self.has_byte_received
        };
        if !want {
            return;
        }
        if let Some(engine) = self.engine.as_mut() {
            let _ = engine.call_void1u(decl, arg);
        }
    }

    fn call_slave_write(&mut self) -> u8 {
        if !self.has_slave_write {
            return 0;
        }
        self.engine
            .as_mut()
            .and_then(|e| e.call_int0("uint slaveWrite()").ok())
            .unwrap_or(0) as u8
    }

    pub fn get_pin_mut(&mut self, name: &str) -> Option<&mut IoPin> {
        if let Some(p) = self.pins.iter_mut().find(|p| pin_matches(&p.id, name)) {
            return Some(p);
        }
        for port in &mut self.ports {
            if let Some(p) = port
                .pins
                .iter_mut()
                .find(|p| pin_matches(&p.id, name) || pin_name_on_port(&p.id, &port.name, name))
            {
                return Some(p);
            }
        }
        None
    }

    pub fn get_port_mut(&mut self, name: &str) -> Option<&mut IoPort> {
        self.ports
            .iter_mut()
            .find(|p| p.name == name || p.name.eq_ignore_ascii_case(name))
    }
}

fn io_port_from_spec(owner: &str, spec: &cs_mcu::PortSpec) -> IoPort {
    if spec.pin_labels.is_empty() {
        let mut port = IoPort::new(spec.name.clone());
        for i in 0..spec.n_pins {
            if spec.pin_mask & (1u32 << i) == 0 {
                continue;
            }
            let mut pin = IoPin::input(format!("{owner}-{}{i}", spec.name));
            pin.set_levels(5.0, 0.0);
            port.pins.push(pin);
        }
        port
    } else {
        IoPort::with_named_pins(&spec.name, owner, &spec.pin_labels)
    }
}

fn pin_matches(id: &str, name: &str) -> bool {
    id == name
        || id.strip_suffix(name).is_some_and(|p| p.ends_with('-'))
        || id.rsplit('-').next() == Some(name)
}

fn pin_name_on_port(id: &str, port: &str, name: &str) -> bool {
    let last = id.rsplit('-').next().unwrap_or(id);
    last.strip_prefix(port) == Some(name) || last == name
}

unsafe extern "C" fn iopin_set_pin_mode(pin: *mut c_void, m: u32) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_pin_mode(PinMode::from_u32(m)) };
}
unsafe extern "C" fn iopin_get_inp_state(pin: *mut c_void) -> c_int {
    if pin.is_null() {
        return 0;
    }
    unsafe { (*(pin as *mut IoPin)).last_inp_state() as c_int }
}
unsafe extern "C" fn iopin_set_out_state(pin: *mut c_void, s: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_out_state(s != 0) };
}
unsafe extern "C" fn iopin_set_state_z(pin: *mut c_void, z: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_state_z(z != 0) };
}
unsafe extern "C" fn iopin_set_out_stat_fast(pin: *mut c_void, s: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_out_stat_fast(s != 0) };
}
unsafe extern "C" fn iopin_schedule_state(pin: *mut c_void, s: c_int, time: u64) {
    if pin.is_null() {
        return;
    }
    let _ = unsafe { (*(pin as *mut IoPin)).schedule_state(s != 0, time) };
}
unsafe extern "C" fn iopin_get_voltage(pin: *mut c_void) -> f64 {
    if pin.is_null() {
        return 0.0;
    }
    unsafe { (*(pin as *mut IoPin)).get_voltage() }
}
unsafe extern "C" fn iopin_set_voltage(pin: *mut c_void, v: f64) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_voltage(v) };
}
unsafe extern "C" fn iopin_set_out_high_v(pin: *mut c_void, v: f64) {
    if pin.is_null() {
        return;
    }
    let pin = unsafe { &mut *(pin as *mut IoPin) };
    let low = pin.out_low_v;
    pin.set_levels(v, low);
}
unsafe extern "C" fn iopin_set_impedance(pin: *mut c_void, imp: f64) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_output_imp(imp) };
}

unsafe extern "C" fn ioport_set_pin_mode(port: *mut c_void, m: u32) {
    if port.is_null() {
        return;
    }
    unsafe { (*(port as *mut IoPort)).set_pin_mode(PinMode::from_u32(m)) };
}
unsafe extern "C" fn ioport_get_inp_state(port: *mut c_void) -> u32 {
    if port.is_null() {
        return 0;
    }
    unsafe { (*(port as *mut IoPort)).get_inp_state() }
}
unsafe extern "C" fn ioport_set_out_state(port: *mut c_void, s: u32) {
    if port.is_null() {
        return;
    }
    unsafe { (*(port as *mut IoPort)).set_out_state(s) };
}
unsafe extern "C" fn ioport_schedule_state(port: *mut c_void, s: u32, time: u64) {
    if port.is_null() {
        return;
    }
    let _ = unsafe { (*(port as *mut IoPort)).schedule_state(s, time) };
}
unsafe extern "C" fn ioport_trigger(port: *mut c_void, n: u32) {
    if port.is_null() {
        return;
    }
    let _ = unsafe { (*(port as *mut IoPort)).trigger(n) };
}

unsafe extern "C" fn cpu_get_pin(cpu: *mut c_void, name: *const c_char) -> *mut c_void {
    if cpu.is_null() || name.is_null() {
        return std::ptr::null_mut();
    }
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    match unsafe { (*(cpu as *mut ScriptCpu)).get_pin_mut(&name) } {
        Some(p) => p as *mut IoPin as *mut c_void,
        None => std::ptr::null_mut(),
    }
}
unsafe extern "C" fn cpu_get_port(cpu: *mut c_void, name: *const c_char) -> *mut c_void {
    if cpu.is_null() || name.is_null() {
        return std::ptr::null_mut();
    }
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    match unsafe { (*(cpu as *mut ScriptCpu)).get_port_mut(&name) } {
        Some(p) => p as *mut IoPort as *mut c_void,
        None => std::ptr::null_mut(),
    }
}
unsafe extern "C" fn cpu_circ_time(cpu: *mut c_void) -> u64 {
    if cpu.is_null() {
        return 0;
    }
    unsafe { (*(cpu as *mut ScriptCpu)).circ_time }
}
unsafe extern "C" fn cpu_add_event(cpu: *mut c_void, time: u64) {
    if cpu.is_null() {
        return;
    }
    unsafe { (*(cpu as *mut ScriptCpu)).pending_event = Some(time) };
}
unsafe extern "C" fn cpu_cancel_events(cpu: *mut c_void) {
    if cpu.is_null() {
        return;
    }
    unsafe {
        let c = &mut *(cpu as *mut ScriptCpu);
        c.cancel_events = true;
        c.pending_event = None;
    }
}
unsafe extern "C" fn cpu_read_ram(cpu: *mut c_void, addr: u32) -> c_int {
    if cpu.is_null() {
        return -1;
    }
    let c = unsafe { &*(cpu as *mut ScriptCpu) };
    c.ram
        .get(addr as usize)
        .copied()
        .map(i32::from)
        .unwrap_or(-1)
}
unsafe extern "C" fn cpu_write_ram(cpu: *mut c_void, addr: u32, v: c_int) {
    if cpu.is_null() {
        return;
    }
    let c = unsafe { &mut *(cpu as *mut ScriptCpu) };
    if let Some(slot) = c.ram.get_mut(addr as usize) {
        *slot = v as u8;
    }
}
unsafe extern "C" fn cpu_read_pgm(cpu: *mut c_void, addr: u32) -> c_int {
    if cpu.is_null() {
        return -1;
    }
    let c = unsafe { &*(cpu as *mut ScriptCpu) };
    c.pgm
        .get(addr as usize)
        .copied()
        .map(i32::from)
        .unwrap_or(-1)
}
unsafe extern "C" fn cpu_write_pgm(cpu: *mut c_void, addr: u32, v: c_int) {
    if cpu.is_null() {
        return;
    }
    let c = unsafe { &mut *(cpu as *mut ScriptCpu) };
    if let Some(slot) = c.pgm.get_mut(addr as usize) {
        *slot = v as u16;
    }
}

unsafe extern "C" fn mcupin_set_direction(pin: *mut c_void, o: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_direction(o != 0) };
}
unsafe extern "C" fn mcupin_set_port_state(pin: *mut c_void, s: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_port_state(s != 0) };
}
unsafe extern "C" fn mcupin_control_pin(pin: *mut c_void, out_ctrl: c_int, dir_ctrl: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).control_pin(out_ctrl != 0, dir_ctrl != 0) };
}
unsafe extern "C" fn mcupin_set_ext_int(pin: *mut c_void, mode: u32) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).set_ext_int(mode) };
}
unsafe extern "C" fn mcupin_set_out_state(pin: *mut c_void, s: c_int) {
    if pin.is_null() {
        return;
    }
    unsafe { (*(pin as *mut IoPin)).mcu_set_out_state(s != 0) };
}
unsafe extern "C" fn mcuport_control_port(port: *mut c_void, o: c_int, d: c_int) {
    if port.is_null() {
        return;
    }
    unsafe { (*(port as *mut IoPort)).control_port(o != 0, d != 0) };
}
unsafe extern "C" fn mcuport_set_direction(port: *mut c_void, d: u32) {
    if port.is_null() {
        return;
    }
    unsafe { (*(port as *mut IoPort)).mcu_set_direction(d) };
}
unsafe extern "C" fn mcuport_set_out_state(port: *mut c_void, s: u32) {
    if port.is_null() {
        return;
    }
    unsafe { (*(port as *mut IoPort)).mcu_set_out_state(s) };
}

unsafe extern "C" fn uart_set_baud(uart: *mut c_void, baud: c_int) {
    if uart.is_null() {
        return;
    }
    unsafe { (*(uart as *mut ScriptUsart)).module.set_baud_rate(baud) };
}
unsafe extern "C" fn uart_set_data_bits(uart: *mut c_void, bits: u32) {
    if uart.is_null() {
        return;
    }
    unsafe {
        (*(uart as *mut ScriptUsart))
            .module
            .set_data_bits(bits as u8)
    };
}
unsafe extern "C" fn uart_send_byte(uart: *mut c_void, b: u32) {
    if uart.is_null() {
        return;
    }
    unsafe {
        let u = uart as *mut ScriptUsart;
        if (*u).cpu.is_null() {
            return;
        }
        let cpu = (*u).cpu;
        let tx = (*u).tx;
        let now = (*cpu).circ_time;
        (*cpu).with_flat_pins(|pins| {
            (*u).module.send_byte(b as u8, now, pins, tx);
        });
    }
}
unsafe extern "C" fn spi_set_mode(spi: *mut c_void, mode: c_int) {
    if spi.is_null() {
        return;
    }
    unsafe {
        let s = spi as *mut ScriptSpi;
        if (*s).cpu.is_null() {
            return;
        }
        let mode = match mode {
            1 => SpiMode::Master,
            2 => SpiMode::Slave,
            _ => SpiMode::Off,
        };
        let map = (*s).pins;
        (*(*s).cpu).with_flat_pins(|pins| {
            (*s).module.set_mode(mode, pins, map);
        });
    }
}
unsafe extern "C" fn spi_send_byte(spi: *mut c_void, b: u32) {
    if spi.is_null() {
        return;
    }
    unsafe {
        let s = spi as *mut ScriptSpi;
        if (*s).cpu.is_null() {
            return;
        }
        (*s).module.sr_reg = b as u8;
        if (*s).module.mode == SpiMode::Master {
            let now = (*(*s).cpu).circ_time;
            let map = (*s).pins;
            (*(*s).cpu).with_flat_pins(|pins| {
                (*s).module.start_transaction(now, pins, map);
            });
        }
    }
}
unsafe extern "C" fn twi_set_mode(twi: *mut c_void, mode: c_int) {
    if twi.is_null() {
        return;
    }
    unsafe {
        let t = twi as *mut ScriptTwi;
        if (*t).cpu.is_null() {
            return;
        }
        let mode = match mode {
            1 => TwiMode::Master,
            2 => TwiMode::Slave,
            _ => TwiMode::Off,
        };
        let now = (*(*t).cpu).circ_time;
        let map = (*t).pins;
        (*(*t).cpu).with_flat_pins(|pins| {
            (*t).module.set_mode(mode, now, pins, map);
        });
    }
}
unsafe extern "C" fn twi_send_byte(twi: *mut c_void, b: u32) {
    if twi.is_null() {
        return;
    }
    unsafe {
        let t = twi as *mut ScriptTwi;
        if (*t).cpu.is_null() {
            return;
        }
        let data = b as u8;
        if (*t).module.mode == TwiMode::Slave {
            (*t).module.tx_reg = data;
            return;
        }
        if (*t).module.mode != TwiMode::Master {
            return;
        }
        let status = (*t).module.status();
        let is_addr = status == TwiState::Start as u8 || status == TwiState::RepStart as u8;
        let write = if is_addr { data & 1 == 0 } else { true };
        let map = (*t).pins;
        (*(*t).cpu).with_flat_pins(|pins| {
            (*t).module.master_write(data, is_addr, write, pins, map);
        });
    }
}
unsafe extern "C" fn twi_set_address(twi: *mut c_void, a: u32) {
    if twi.is_null() {
        return;
    }
    unsafe { (*(twi as *mut ScriptTwi)).module.set_address(a as u8) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_sets_pin_high() {
        let mut cpu = ScriptCpu::new("Script-1");
        cpu.pins.push(IoPin::input("Script-1-OUT"));
        cpu.set_script(
            r#"
            void reset() {
                IoPin@ p = component.getPin("OUT");
                p.setPinMode(3);
                p.setOutState(true);
            }
            "#,
        );
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        let pin = cpu.get_pin_mut("OUT").unwrap();
        assert_eq!(pin.mode, PinMode::Output);
        assert!(pin.get_out_state());
    }

    #[test]
    fn script_port_and_ram() {
        let mut cpu =
            ScriptCpu::new("Script-1").with_port(IoPort::with_pins("PORTA", "Script-1", 8));
        cpu.set_script(
            r#"
            void reset() {
                IoPort@ p = component.getPort("PORTA");
                p.setPinMode(3);
                p.setOutState(1);
                component.writeRAM(4, 42);
            }
            int run() { return component.readRAM(4); }
            "#,
        );
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        assert_eq!(
            cpu.engine.as_mut().unwrap().call_int0("int run()").unwrap(),
            42
        );
        assert_eq!(cpu.ports[0].pins[0].mode, PinMode::Output);
        assert!(cpu.ports[0].pins[0].get_out_state());
        assert!(!cpu.ports[0].pins[1].get_out_state());
    }

    #[test]
    fn script_mcu_pin_and_port() {
        let mut cpu =
            ScriptCpu::new("Script-1").with_port(IoPort::with_pins("PORTA", "Script-1", 8));
        cpu.set_script(
            r#"
            void reset() {
                McuPort@ port = component.getMcuPort("PORTA");
                port.setDirection(1);
                McuPin@ p = component.getMcuPin("PORTA0");
                p.setDirection(true);
                p.setPortState(true);
                p.setExtInt(3);
                p.controlPin(true, true);
                p.setOutState(true);
            }
            "#,
        );
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        let pin = cpu.get_pin_mut("PORTA0").unwrap();
        assert!(pin.is_out);
        assert!(pin.out_ctrl);
        assert!(pin.dir_ctrl);
        assert_eq!(pin.ext_int_trigger, 3);
        assert!(pin.port_state);
        assert!(pin.get_out_state());
        assert_eq!(pin.mode, PinMode::Output);
    }

    #[test]
    fn script_uart_send_byte_drives_tx() {
        let mut cpu = ScriptCpu::new("Script-1");
        cpu.pins.push(IoPin::input("Script-1-TX"));
        cpu.pins.push(IoPin::input("Script-1-RX"));
        cpu.add_uart("UART0", "TX", "RX");
        cpu.set_script(
            r#"
            void reset() {
                UART0.setBaudRate(1000000);
                UART0.setDataBits(8);
                UART0.sendByte(0x01);
            }
            "#,
        );
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        let pin = cpu.get_pin_mut("TX").unwrap();
        assert_eq!(pin.mode, PinMode::Output);
        assert!(!pin.get_out_state()); // start bit
    }

    #[test]
    fn script_spi_set_mode_master() {
        let mut cpu = ScriptCpu::new("Script-1");
        cpu.pins.push(IoPin::input("Script-1-MOSI"));
        cpu.pins.push(IoPin::input("Script-1-MISO"));
        cpu.pins.push(IoPin::input("Script-1-SCK"));
        cpu.pins.push(IoPin::input("Script-1-SS"));
        cpu.add_spi("SPI0", "MOSI", "MISO", "SCK", "SS");
        cpu.set_script(
            r#"
            void reset() {
                SPI0.setMode(1);
                SPI0.sendByte(0xA5);
            }
            "#,
        );
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        assert_eq!(cpu.spis[0].module.mode, SpiMode::Master);
        assert_eq!(cpu.spis[0].module.tx_reg, 0xA5);
        assert_eq!(cpu.get_pin_mut("MOSI").unwrap().mode, PinMode::Output);
    }

    #[test]
    fn script_twi_set_mode_and_address() {
        let mut cpu = ScriptCpu::new("Script-1");
        cpu.pins.push(IoPin::input("Script-1-SCL"));
        cpu.pins.push(IoPin::input("Script-1-SDA"));
        cpu.add_twi("TWI0", "SCL", "SDA");
        cpu.set_script(
            r#"
            void reset() {
                TWI0.setAddress(0x50);
                TWI0.setMode(1);
            }
            "#,
        );
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        assert_eq!(cpu.twis[0].module.address, 0x50);
        assert_eq!(cpu.twis[0].module.mode, TwiMode::Master);
    }

    const SCRIPTED_MCU: &str = r#"
<mcu core="scripted" script="cpu.as" data="256" prog="64" inst_cycle="1" freq="16000000">
  <ioport name="P" pins="TX,RX,MOSI,MISO,SCK,SS,SDA,SCL"/>
  <usart name="UART0" number="0">
    <trunit type="tx" pin="TX"/>
    <trunit type="rx" pin="RX"/>
  </usart>
  <spi name="SPI0" pins="MOSI,MISO,SCK,SS"/>
  <twi name="TWI0" pins="SDA,SCL"/>
</mcu>
"#;

    #[test]
    fn from_mcu_xml_constructs_usart_spi_twi() {
        let desc = cs_mcu::parse_mcu_xml(SCRIPTED_MCU).unwrap();
        let mut cpu = ScriptCpu::from_mcu_desc(
            "scriptuart-1",
            &desc,
            r#"
            void reset() {
                UART0.setBaudRate(1000000);
                UART0.setDataBits(8);
                UART0.sendByte(0x01);
                SPI0.setMode(1);
                TWI0.setAddress(0x50);
            }
            "#,
        );
        assert_eq!(cpu.usarts.len(), 1);
        assert_eq!(cpu.usarts[0].name, "UART0");
        assert!(cpu.usarts[0].tx.is_some(), "TX pin from <trunit>");
        assert!(cpu.usarts[0].rx.is_some(), "RX pin from <trunit>");
        assert_eq!(cpu.spis.len(), 1);
        assert_eq!(cpu.spis[0].name, "SPI0");
        assert!(cpu.spis[0].pins.mosi.is_some());
        assert_eq!(cpu.twis.len(), 1);
        assert_eq!(cpu.twis[0].name, "TWI0");
        assert!(cpu.twis[0].pins.sda.is_some());
        assert!(cpu.twis[0].pins.scl.is_some());
        cpu.compile().unwrap();
        cpu.stamp_init(0);
        let tx = cpu.get_pin_mut("TX").unwrap();
        assert_eq!(tx.mode, PinMode::Output);
        assert!(!tx.get_out_state()); // start bit
        assert_eq!(cpu.spis[0].module.mode, SpiMode::Master);
        assert_eq!(cpu.twis[0].module.address, 0x50);
    }
}
