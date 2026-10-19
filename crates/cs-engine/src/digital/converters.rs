//! Digital converters matching C++ `Mux`, `Demux`, `BcdToDec`, `DecToBcd`,
//! `BcdTo7S`, `I2CToParallel`, `ADC`, and `DAC`.

use super::TwiModule;
use super::family::LogicFamily;
use super::pin::IoPin;
use super::queue::OutQueue;

// ============================================================================
// Digital Multiplexer (Mux)
// ============================================================================

#[derive(Clone, Debug)]
pub struct MuxState {
    pub addr_bits: usize,
    pub inputs: Vec<IoPin>,
    pub addr_pins: Vec<IoPin>,
    pub enable: Option<IoPin>,
    pub output: IoPin,
    pub out_inverted: IoPin,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl MuxState {
    pub fn new(id: &str, addr_bits: usize) -> Self {
        let addr_bits = addr_bits.clamp(1, 5);
        let num_inputs = 1 << addr_bits;
        let mut inputs = Vec::with_capacity(num_inputs);
        for i in 0..num_inputs {
            inputs.push(IoPin::input(format!("{id}-in{i}")));
        }
        let mut addr_pins = Vec::with_capacity(addr_bits);
        for i in 0..addr_bits {
            addr_pins.push(IoPin::input(format!("{id}-addr{i}")));
        }
        let enable = Some(IoPin::input(format!("{id}-enable")));
        let output = IoPin::output(format!("{id}-out"));
        let out_inverted = IoPin::output(format!("{id}-out_inv"));

        let mut st = Self {
            addr_bits,
            inputs,
            addr_pins,
            enable,
            output,
            out_inverted,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.inputs {
            self.family.apply(pin);
        }
        for pin in &mut self.addr_pins {
            self.family.apply(pin);
        }
        if let Some(en) = &mut self.enable {
            self.family.apply(en);
        }
        self.family.apply(&mut self.output);
        self.family.apply(&mut self.out_inverted);
    }

    pub fn eval(&mut self) -> bool {
        // Active-low enable: if high, output is 0 (or disabled)
        if let Some(en) = &self.enable {
            if en.inp_state() {
                self.output.set_out_state(false);
                self.out_inverted.set_out_state(true);
                return false;
            }
        }
        let mut sel = 0usize;
        for (i, p) in self.addr_pins.iter().enumerate() {
            if p.inp_state() {
                sel |= 1 << i;
            }
        }
        let val = if sel < self.inputs.len() {
            self.inputs[sel].inp_state()
        } else {
            false
        };
        self.output.set_out_state(val);
        self.out_inverted.set_out_state(!val);
        val
    }
}

// ============================================================================
// Digital Demultiplexer / Decoder (Demux)
// ============================================================================

#[derive(Clone, Debug)]
pub struct DemuxState {
    pub addr_bits: usize,
    pub input: IoPin,
    pub addr_pins: Vec<IoPin>,
    pub enable: Option<IoPin>,
    pub outputs: Vec<IoPin>,
    pub inverted: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl DemuxState {
    pub fn new(id: &str, addr_bits: usize, inverted: bool) -> Self {
        let addr_bits = addr_bits.clamp(1, 5);
        let num_outputs = 1 << addr_bits;
        let mut outputs = Vec::with_capacity(num_outputs);
        for i in 0..num_outputs {
            let mut pin = IoPin::output(format!("{id}-out{i}"));
            if inverted {
                pin.set_inverted(true);
            }
            outputs.push(pin);
        }
        let mut addr_pins = Vec::with_capacity(addr_bits);
        for i in 0..addr_bits {
            addr_pins.push(IoPin::input(format!("{id}-addr{i}")));
        }
        let input = IoPin::input(format!("{id}-in"));
        let enable = Some(IoPin::input(format!("{id}-enable")));

        let mut st = Self {
            addr_bits,
            input,
            addr_pins,
            enable,
            outputs,
            inverted,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        self.family.apply(&mut self.input);
        for pin in &mut self.addr_pins {
            self.family.apply(pin);
        }
        if let Some(en) = &mut self.enable {
            self.family.apply(en);
        }
        for pin in &mut self.outputs {
            self.family.apply(pin);
        }
    }

    pub fn eval(&mut self) {
        let enabled = self
            .enable
            .as_ref()
            .map(|en| !en.inp_state())
            .unwrap_or(true);
        let in_val = self.input.inp_state();
        let mut sel = 0usize;
        for (i, p) in self.addr_pins.iter().enumerate() {
            if p.inp_state() {
                sel |= 1 << i;
            }
        }
        for (i, out) in self.outputs.iter_mut().enumerate() {
            let high = enabled && (i == sel) && in_val;
            out.set_out_state(high);
        }
    }
}

// ============================================================================
// BCD to Decimal Decoder (BcdToDec)
// ============================================================================

#[derive(Clone, Debug)]
pub struct BcdToDecState {
    pub inputs: Vec<IoPin>,  // 4 inputs: A0, A1, A2, A3
    pub outputs: Vec<IoPin>, // 10 or 16 outputs
    /// Output enable, schematic pin `in4`. Active-low: a high level blanks
    /// every output. Unconnected (sampled low) stays enabled.
    pub oe: IoPin,
    pub sixteen: bool,
    pub active_low: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl BcdToDecState {
    pub fn new(id: &str, sixteen: bool, active_low: bool) -> Self {
        let count = if sixteen { 16 } else { 10 };
        let mut inputs = Vec::with_capacity(4);
        for i in 0..4 {
            inputs.push(IoPin::input(format!("{id}-in{i}")));
        }
        let mut outputs = Vec::with_capacity(count);
        for i in 0..count {
            let mut pin = IoPin::output(format!("{id}-out{i}"));
            if active_low {
                pin.set_inverted(true);
            }
            outputs.push(pin);
        }
        let oe = IoPin::input(format!("{id}-in4"));
        let mut st = Self {
            inputs,
            outputs,
            oe,
            sixteen,
            active_low,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.inputs {
            self.family.apply(pin);
        }
        for pin in &mut self.outputs {
            self.family.apply(pin);
        }
        self.family.apply(&mut self.oe);
    }

    pub fn eval(&mut self) {
        // High OE disables the decoder. `set_out_state(false)` is the
        // deasserted logical level, so active-low outputs rest high.
        if self.oe.inp_state() {
            for out in &mut self.outputs {
                out.set_out_state(false);
            }
            return;
        }
        let mut val = 0usize;
        for (i, p) in self.inputs.iter().enumerate() {
            if p.inp_state() {
                val |= 1 << i;
            }
        }
        for (i, out) in self.outputs.iter_mut().enumerate() {
            out.set_out_state(i == val);
        }
    }
}

// ============================================================================
// Decimal to BCD Priority Encoder (DecToBcd)
// ============================================================================

#[derive(Clone, Debug)]
pub struct DecToBcdState {
    pub inputs: Vec<IoPin>,  // 10 or 16 inputs
    pub outputs: Vec<IoPin>, // 4 BCD outputs
    pub gs: IoPin,           // Group select / Any input active
    pub eo: IoPin,           // Enable output
    pub ei: IoPin,           // Enable input
    pub sixteen: bool,
    pub active_low: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl DecToBcdState {
    pub fn new(id: &str, sixteen: bool, active_low: bool) -> Self {
        // Schematic data pins are in0..in8 (or in0..in14). Output enable is always in15.
        let count = if sixteen { 15 } else { 9 };
        let mut inputs = Vec::with_capacity(count);
        for i in 0..count {
            let mut pin = IoPin::input(format!("{id}-in{i}"));
            if active_low {
                pin.set_inverted(true);
            }
            inputs.push(pin);
        }
        let mut outputs = Vec::with_capacity(4);
        for i in 0..4 {
            let mut pin = IoPin::output(format!("{id}-out{i}"));
            if active_low {
                pin.set_inverted(true);
            }
            outputs.push(pin);
        }
        let gs = IoPin::output(format!("{id}-gs"));
        let eo = IoPin::output(format!("{id}-eo"));
        let ei = IoPin::input(format!("{id}-in15"));

        let mut st = Self {
            inputs,
            outputs,
            gs,
            eo,
            ei,
            sixteen,
            active_low,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.inputs {
            self.family.apply(pin);
        }
        for pin in &mut self.outputs {
            self.family.apply(pin);
        }
        self.family.apply(&mut self.gs);
        self.family.apply(&mut self.eo);
        self.family.apply(&mut self.ei);
    }

    pub fn eval(&mut self) {
        let enabled = !self.ei.inp_state(); // active-low enable
        if !enabled {
            for out in &mut self.outputs {
                out.set_out_state(false);
            }
            self.gs.set_out_state(false);
            self.eo.set_out_state(false);
            return;
        }

        // Highest active input wins (priority encoding)
        let mut highest = None;
        for (i, p) in self.inputs.iter().enumerate().rev() {
            if p.inp_state() {
                highest = Some(i);
                break;
            }
        }

        if let Some(val) = highest {
            for (bit, out) in self.outputs.iter_mut().enumerate() {
                out.set_out_state((val & (1 << bit)) != 0);
            }
            self.gs.set_out_state(true);
            self.eo.set_out_state(false);
        } else {
            for out in &mut self.outputs {
                out.set_out_state(false);
            }
            self.gs.set_out_state(false);
            self.eo.set_out_state(true);
        }
    }
}

// ============================================================================
// BCD to 7-Segment Decoder (BcdTo7S)
// ============================================================================

#[derive(Clone, Debug)]
pub struct BcdTo7SState {
    pub inputs: Vec<IoPin>,   // 4 BCD inputs: A, B, C, D
    pub lt: IoPin,            // Lamp test (active-low)
    pub rbi: IoPin,           // Ripple blanking input (active-low)
    pub bi_rbo: IoPin,        // Blanking input / Ripple blanking output
    pub segments: Vec<IoPin>, // 7 segment outputs: a, b, c, d, e, f, g
    pub common_anode: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl BcdTo7SState {
    pub const FONT_7SEG: [u8; 16] = [
        0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F, 0x77, 0x7C, 0x39, 0x5E, 0x79,
        0x71,
    ];

    pub fn new(id: &str, common_anode: bool) -> Self {
        let mut inputs = Vec::with_capacity(4);
        for i in 0..4 {
            inputs.push(IoPin::input(format!("{id}-in{i}")));
        }
        // Lamp test and ripple-blanking input are not schematic pins.
        let lt = IoPin::input(format!("{id}-lt"));
        let rbi = IoPin::input(format!("{id}-rbi"));
        // Blanking / output-enable pin is schematic `in4`.
        let bi_rbo = IoPin::input(format!("{id}-in4"));

        let mut segments = Vec::with_capacity(7);
        for i in 0..7 {
            let mut pin = IoPin::output(format!("{id}-out{i}"));
            if common_anode {
                pin.set_inverted(true);
            }
            segments.push(pin);
        }

        let mut st = Self {
            inputs,
            lt,
            rbi,
            bi_rbo,
            segments,
            common_anode,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.inputs {
            self.family.apply(pin);
        }
        self.family.apply(&mut self.lt);
        self.family.apply(&mut self.rbi);
        self.family.apply(&mut self.bi_rbo);
        for pin in &mut self.segments {
            self.family.apply(pin);
        }
    }

    pub fn eval(&mut self) {
        // Lamp test: active-low, turns on all segments
        if !self.lt.inp_state() && self.lt.last_inp_state() != self.lt.inp_state() {
            // Checked below
        }
        if !self.lt.inp_state() {
            for seg in &mut self.segments {
                seg.set_out_state(true);
            }
            return;
        }

        // Blanking input: active-low, turns off all segments
        if !self.bi_rbo.inp_state() {
            for seg in &mut self.segments {
                seg.set_out_state(false);
            }
            return;
        }

        let mut val = 0usize;
        for (i, p) in self.inputs.iter().enumerate() {
            if p.inp_state() {
                val |= 1 << i;
            }
        }

        // Ripple blanking for zero
        if val == 0 && !self.rbi.inp_state() {
            for seg in &mut self.segments {
                seg.set_out_state(false);
            }
            return;
        }

        let mask = Self::FONT_7SEG[val.min(15)];
        for (i, seg) in self.segments.iter_mut().enumerate() {
            seg.set_out_state((mask & (1 << i)) != 0);
        }
    }
}

// ============================================================================
// I2C to 8-Bit Parallel I/O Expander (PCF8574 / I2CToParallel)
// ============================================================================

#[derive(Clone, Debug)]
pub struct I2CToParallelState {
    pub address: u8,       // 7-bit I2C address (0x20..0x27 for PCF8574)
    pub ports: Vec<IoPin>, // P0..P7 quasi-bidirectional pins
    pub int_pin: IoPin,    // Open-drain interrupt output (active low)
    pub scl: IoPin,        // I2C clock
    pub sda: IoPin,        // I2C data
    pub in_latch: u8,
    pub out_latch: u8,
    pub family: LogicFamily,
    pub queue: OutQueue,
    pub twi: TwiModule,
}

impl I2CToParallelState {
    pub fn new(id: &str, address: u8) -> Self {
        let mut ports = Vec::with_capacity(8);
        for i in 0..8 {
            ports.push(IoPin::output(format!("{id}-out{i}")));
        }
        let int_pin = IoPin::open_collector(format!("{id}-in5"));
        let scl = IoPin::input(format!("{id}-in1"));
        let sda = IoPin::open_collector(format!("{id}-in0"));

        let mut twi = TwiModule::new();
        twi.set_slave(address & 0x7F);
        let mut st = Self {
            address,
            ports,
            int_pin,
            scl,
            sda,
            in_latch: 0xFF,
            out_latch: 0xFF,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
            twi,
        };
        st.apply_family();
        st.write_byte(0xFF);
        // Open-collector pins are born sinking. A slave that holds SDA low
        // hides every host bit, so the address "ack" is just a stuck bus.
        st.sda.set_out_state(true);
        st.int_pin.set_out_state(true);
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.ports {
            self.family.apply(pin);
        }
        self.family.apply(&mut self.int_pin);
        self.family.apply(&mut self.scl);
        self.family.apply(&mut self.sda);
    }

    pub fn write_byte(&mut self, data: u8) {
        self.out_latch = data;
        for (i, p) in self.ports.iter_mut().enumerate() {
            p.set_out_state((data & (1 << i)) != 0);
        }
    }

    pub fn read_byte(&self) -> u8 {
        let mut val = 0u8;
        for (i, p) in self.ports.iter().enumerate() {
            if p.inp_state() {
                val |= 1 << i;
            }
        }
        val
    }

    /// Sample the existing TWI slave. A received byte drives the port pins.
    /// A slave read returns the sampled port byte. INT is the open-drain
    /// alert when the port byte differs from the last bus transfer.
    pub fn poll(&mut self) {
        if self.twi.address != (self.address & 0x7F) {
            self.twi.set_slave(self.address & 0x7F);
        }
        let tick = self.twi.volt_changed_pins(&mut self.scl, &mut self.sda);
        if let Some(byte) = tick.byte_received {
            self.write_byte(byte);
            self.in_latch = byte;
        }
        if tick.need_slave_write {
            let byte = self.read_byte();
            self.twi.tx_reg = byte;
            self.in_latch = byte;
        }
        let alert = self.read_byte() != self.in_latch;
        self.int_pin.set_out_state(!alert);
    }
}

// ============================================================================
// Analog-to-Digital Converter (ADC)
// ============================================================================

#[derive(Clone, Debug)]
pub struct AdcState {
    pub bits: usize,         // 8, 10, or 12 bits
    pub vref_pos: f64,       // Positive reference voltage (default 5.0V)
    pub vref_neg: f64,       // Negative reference voltage (default 0.0V)
    pub outputs: Vec<IoPin>, // D0..Dn-1 digital outputs
    pub soc: IoPin,          // Start of conversion (input)
    pub eoc: IoPin,          // End of conversion (output)
    pub oe: IoPin,           // Output enable (input)
    pub current_code: u32,   // Converted discrete digital value
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl AdcState {
    pub fn new(id: &str, bits: usize, vref_pos: f64, vref_neg: f64) -> Self {
        let bits = bits.clamp(4, 16);
        let mut outputs = Vec::with_capacity(bits);
        for i in 0..bits {
            outputs.push(IoPin::output(format!("{id}-out{i}")));
        }
        let soc = IoPin::input(format!("{id}-soc"));
        let eoc = IoPin::output(format!("{id}-eoc"));
        let oe = IoPin::input(format!("{id}-oe"));

        let mut st = Self {
            bits,
            vref_pos,
            vref_neg,
            outputs,
            soc,
            eoc,
            oe,
            current_code: 0,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.outputs {
            self.family.apply(pin);
        }
        self.family.apply(&mut self.soc);
        self.family.apply(&mut self.eoc);
        self.family.apply(&mut self.oe);
    }

    pub fn convert(&mut self, vin: f64) -> u32 {
        let max_code = (1u32 << self.bits) - 1;
        let vspan = (self.vref_pos - self.vref_neg).max(1e-6);
        let norm = ((vin - self.vref_neg) / vspan).clamp(0.0, 1.0);
        let code = (norm * max_code as f64).round() as u32;
        self.current_code = code;

        let oe_active = self.oe.inp_state();
        for (i, p) in self.outputs.iter_mut().enumerate() {
            p.set_out_state(oe_active && ((code & (1 << i)) != 0));
        }
        self.eoc.set_out_state(true);
        code
    }
}

// ============================================================================
// Digital-to-Analog Converter (DAC)
// ============================================================================

#[derive(Clone, Debug)]
pub struct DacState {
    pub bits: usize,        // 8, 10, or 12 bits
    pub vref: f64,          // Reference voltage (default 5.0V)
    pub inputs: Vec<IoPin>, // D0..Dn-1 digital inputs
    pub vout: f64,          // Output analog voltage
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl DacState {
    pub fn new(id: &str, bits: usize, vref: f64) -> Self {
        let bits = bits.clamp(4, 16);
        let mut inputs = Vec::with_capacity(bits);
        for i in 0..bits {
            inputs.push(IoPin::input(format!("{id}-in{i}")));
        }
        let mut st = Self {
            bits,
            vref,
            inputs,
            vout: 0.0,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        for pin in &mut self.inputs {
            self.family.apply(pin);
        }
    }

    pub fn eval(&mut self) -> f64 {
        let mut val = 0u32;
        for (i, p) in self.inputs.iter().enumerate() {
            if p.inp_state() {
                val |= 1 << i;
            }
        }
        let max_val = (1u32 << self.bits) - 1;
        let vout = self.vref * (val as f64 / max_val as f64);
        self.vout = vout;
        vout
    }
}

#[cfg(test)]
mod tests {
    use super::I2CToParallelState;

    /// Host levels, then one extra sample when the slave changes SDA.
    /// That second sample is what the solver does after a stamp change.
    fn sample(dev: &mut I2CToParallelState, scl_high: bool, host_sda_high: bool) {
        for _ in 0..4 {
            let sda_high = host_sda_high && dev.sda.get_out_state();
            let before = dev.sda.get_out_state();
            dev.scl.get_inp_state(if scl_high { 5.0 } else { 0.0 });
            dev.sda.get_inp_state(if sda_high { 5.0 } else { 0.0 });
            dev.poll();
            if dev.sda.get_out_state() == before {
                break;
            }
        }
    }

    #[test]
    fn i2c_slave_releases_sda_and_a_write_updates_the_port_latch() {
        let mut dev = I2CToParallelState::new("IO", 0x50);
        assert!(
            dev.sda.get_out_state(),
            "SDA must idle released or the host cannot send a byte"
        );
        sample(&mut dev, true, true);
        sample(&mut dev, true, true);
        sample(&mut dev, true, false);
        for byte in [0xA0u8, 0x0F] {
            for i in (0..8).rev() {
                let one = byte & (1 << i) != 0;
                sample(&mut dev, false, true);
                sample(&mut dev, false, one);
                sample(&mut dev, true, one);
            }
            sample(&mut dev, false, true);
            sample(&mut dev, true, true);
        }
        assert_eq!(dev.out_latch, 0x0F);
        assert!(!dev.ports[4].get_out_state());
        assert!(dev.ports[0].get_out_state());
    }
}
