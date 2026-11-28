//! Digital Memory IC state machines matching C++ `Memory`, `DynamicMemory`, and `I2CRam`.

use super::TwiModule;
use super::family::LogicFamily;
use super::pin::IoPin;
use super::queue::OutQueue;

// ============================================================================
// Parallel Static RAM / ROM (Memory)
// ============================================================================

#[derive(Clone, Debug)]
pub struct MemoryState {
    pub addr_bits: usize,      // e.g. 8 bits (256 words) to 16 bits (64K words)
    pub data_bits: usize,      // 1..=16 bits per word
    pub data: Vec<u8>,         // Little-endian packed word buffer (1 or 2 bytes/word)
    pub addr_pins: Vec<IoPin>, // A0..An-1
    pub data_pins: Vec<IoPin>, // D0..Dm-1 (bidirectional, tri-stated when not reading)
    pub cs: IoPin,             // Chip Select (active low)
    pub oe: IoPin,             // Output Enable (active low)
    pub we: IoPin,             // Write Enable (active low)
    pub is_rom: bool,          // Read-only memory mode
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl MemoryState {
    pub fn word_bytes(&self) -> usize {
        self.data_bits.div_ceil(8).max(1)
    }

    pub fn new(id: &str, addr_bits: usize, data_bits: usize, is_rom: bool) -> Self {
        let addr_bits = addr_bits.clamp(1, 16);
        let data_bits = data_bits.clamp(1, 16);
        let word_bytes = data_bits.div_ceil(8).max(1);
        let size = (1usize << addr_bits) * word_bytes;
        let data = vec![0u8; size];

        let mut addr_pins = Vec::with_capacity(addr_bits);
        for i in 0..addr_bits {
            addr_pins.push(IoPin::input(format!("{id}-in{i}")));
        }
        let mut data_pins = Vec::with_capacity(data_bits);
        for i in 0..data_bits {
            let mut p = IoPin::output(format!("{id}-out{i}"));
            p.set_state_z(true);
            data_pins.push(p);
        }
        let cs = IoPin::input(format!("{id}-Pin_Cs"));
        let oe = IoPin::input(format!("{id}-Pin_outEnable"));
        let we = IoPin::input(format!("{id}-Pin_We"));

        let mut st = Self {
            addr_bits,
            data_bits,
            data,
            addr_pins,
            data_pins,
            cs,
            oe,
            we,
            is_rom,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        for p in &mut st.data_pins {
            p.set_state_z(true);
        }
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.addr_pins {
            self.family.apply(p);
        }
        for p in &mut self.data_pins {
            let z = p.is_state_z();
            self.family.apply(p);
            if z {
                p.set_state_z(true);
            }
        }
        self.family.apply(&mut self.cs);
        self.family.apply(&mut self.oe);
        self.family.apply(&mut self.we);
    }

    fn read_word(&self, addr: usize) -> u16 {
        let wb = self.word_bytes();
        let off = addr * wb;
        if wb == 1 {
            u16::from(self.data.get(off).copied().unwrap_or(0))
        } else {
            let lo = self.data.get(off).copied().unwrap_or(0);
            let hi = self.data.get(off + 1).copied().unwrap_or(0);
            u16::from_le_bytes([lo, hi])
        }
    }

    fn write_word(&mut self, addr: usize, val: u16) {
        let wb = self.word_bytes();
        let off = addr * wb;
        if wb == 1 {
            if let Some(slot) = self.data.get_mut(off) {
                *slot = val as u8;
            }
        } else {
            let [lo, hi] = val.to_le_bytes();
            if off + 1 < self.data.len() {
                self.data[off] = lo;
                self.data[off + 1] = hi;
            }
        }
    }

    pub fn eval(&mut self) {
        let cs_active = !self.cs.inp_state(); // active-low
        if !cs_active {
            for p in &mut self.data_pins {
                p.set_state_z(true);
            }
            return;
        }

        let word_count = (self.data.len() / self.word_bytes()).max(1);
        let mut addr = 0usize;
        for (i, p) in self.addr_pins.iter().enumerate() {
            if p.inp_state() {
                addr |= 1 << i;
            }
        }
        addr = addr.min(word_count - 1);

        let we_active = !self.is_rom && !self.we.inp_state(); // active-low WE
        if we_active {
            // Release data drivers before sampling so external bus drivers aren't fought.
            let was_driving = self.data_pins.iter().any(|p| !p.is_state_z());
            for p in &mut self.data_pins {
                p.set_state_z(true);
            }
            if !was_driving {
                let mut val = 0u16;
                for (i, p) in self.data_pins.iter().enumerate() {
                    if i < 16 && p.inp_state() {
                        val |= 1u16 << i;
                    }
                }
                self.write_word(addr, val);
            }
            return;
        }

        let oe_active = !self.oe.inp_state(); // active-low OE
        if oe_active {
            let val = self.read_word(addr);
            for (i, p) in self.data_pins.iter_mut().enumerate() {
                if p.is_state_z() {
                    p.set_state_z(false);
                }
                p.set_out_state(i < 16 && ((val >> i) & 1) != 0);
            }
        } else {
            for p in &mut self.data_pins {
                p.set_state_z(true);
            }
        }
    }
}

// ============================================================================
// Dynamic RAM (DynamicMemory)
// ============================================================================

#[derive(Clone, Debug)]
pub struct DynamicMemoryState {
    pub addr_bits: usize, // Multiplexed address bits (4..=12 -> 1 << (addr_bits * 2) bytes)
    pub data_bits: usize, // Data bus width (default 8)
    pub data: Vec<u8>,    // Memory array
    pub row_addr: usize,  // Latched row address (on RAS falling)
    pub col_addr: usize,  // Latched column address (on CAS falling)
    pub addr_pins: Vec<IoPin>,
    pub data_pins: Vec<IoPin>,
    pub ras: IoPin, // Row Address Strobe (active low)
    pub cas: IoPin, // Column Address Strobe (active low)
    pub we: IoPin,  // Write Enable (active low)
    pub oe: IoPin,  // Output Enable (active low)
    pub last_ras: bool,
    pub last_cas: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl DynamicMemoryState {
    pub fn new(id: &str, addr_bits: usize) -> Self {
        let addr_bits = addr_bits.clamp(4, 12);
        let data_bits = 8;
        let size = 1 << (addr_bits * 2);
        let data = vec![0u8; size];

        let mut addr_pins = Vec::with_capacity(addr_bits);
        for i in 0..addr_bits {
            addr_pins.push(IoPin::input(format!("{id}-in{i}")));
        }
        let mut data_pins = Vec::with_capacity(data_bits);
        for i in 0..data_bits {
            let mut p = IoPin::output(format!("{id}-out{i}"));
            p.set_state_z(true);
            data_pins.push(p);
        }
        let ras = IoPin::input(format!("{id}-Pin_Ras"));
        let cas = IoPin::input(format!("{id}-Pin_Cas"));
        let we = IoPin::input(format!("{id}-Pin_We"));
        let oe = IoPin::input(format!("{id}-Pin_outEnable"));

        let mut st = Self {
            addr_bits,
            data_bits,
            data,
            row_addr: 0,
            col_addr: 0,
            addr_pins,
            data_pins,
            ras,
            cas,
            we,
            oe,
            last_ras: true,
            last_cas: true,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        for p in &mut st.data_pins {
            p.set_state_z(true);
        }
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.addr_pins {
            self.family.apply(p);
        }
        for p in &mut self.data_pins {
            let z = p.is_state_z();
            self.family.apply(p);
            if z {
                p.set_state_z(true);
            }
        }
        self.family.apply(&mut self.ras);
        self.family.apply(&mut self.cas);
        self.family.apply(&mut self.we);
        self.family.apply(&mut self.oe);
    }

    pub fn eval(&mut self) {
        let mut curr_addr = 0usize;
        for (i, p) in self.addr_pins.iter().enumerate() {
            if p.inp_state() {
                curr_addr |= 1 << i;
            }
        }

        let ras_now = self.ras.inp_state();
        // Falling edge on RAS latches row address
        if !ras_now && self.last_ras {
            self.row_addr = curr_addr;
        }
        self.last_ras = ras_now;

        let cas_now = self.cas.inp_state();
        let we_active = !self.we.inp_state(); // active low WE
        let oe_active = !self.oe.inp_state(); // active low OE

        if !cas_now && self.last_cas {
            self.col_addr = curr_addr;
            let full_addr = (self.row_addr << self.addr_bits) | self.col_addr;
            let full_addr = full_addr.min(self.data.len() - 1);

            if we_active {
                for p in &mut self.data_pins {
                    p.set_state_z(true);
                }
                let mut val = 0u8;
                for (i, p) in self.data_pins.iter().enumerate() {
                    if p.inp_state() {
                        val |= 1 << i;
                    }
                }
                self.data[full_addr] = val;
            } else if oe_active {
                let val = self.data[full_addr];
                for (i, p) in self.data_pins.iter_mut().enumerate() {
                    if p.is_state_z() {
                        p.set_state_z(false);
                    }
                    p.set_out_state((val & (1 << i)) != 0);
                }
            } else {
                for p in &mut self.data_pins {
                    p.set_state_z(true);
                }
            }
        } else if !cas_now && we_active {
            // Keep drivers released while WE is low during an active CAS cycle,
            // and re-sample external data once released.
            let was_driving = self.data_pins.iter().any(|p| !p.is_state_z());
            for p in &mut self.data_pins {
                p.set_state_z(true);
            }
            if !was_driving {
                let full_addr =
                    ((self.row_addr << self.addr_bits) | self.col_addr).min(self.data.len() - 1);
                let mut val = 0u8;
                for (i, p) in self.data_pins.iter().enumerate() {
                    if p.inp_state() {
                        val |= 1 << i;
                    }
                }
                self.data[full_addr] = val;
            }
        } else if !cas_now && oe_active {
            let full_addr =
                ((self.row_addr << self.addr_bits) | self.col_addr).min(self.data.len() - 1);
            let val = self.data[full_addr];
            for (i, p) in self.data_pins.iter_mut().enumerate() {
                if p.is_state_z() {
                    p.set_state_z(false);
                }
                p.set_out_state((val & (1 << i)) != 0);
            }
        } else {
            for p in &mut self.data_pins {
                p.set_state_z(true);
            }
        }
        self.last_cas = cas_now;
    }
}

// ============================================================================
// 24Cxx I2C Serial EEPROM (I2CRam)
// ============================================================================

#[derive(Clone, Debug)]
pub struct I2CRamState {
    pub size_bytes: usize,
    pub dev_address: u8, // 7-bit base I2C address, e.g. 0x50
    pub data: Vec<u8>,   // 1..=65536 bytes
    pub mem_ptr: u16,
    pub addr_bytes_left: u8,
    pub addr_acc: u16,
    pub scl: IoPin,
    pub sda: IoPin,
    pub addr_pins: [IoPin; 3], // A0, A1, A2 strap inputs (in2, in3, in4)
    pub twi: TwiModule,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl I2CRamState {
    pub fn new(id: &str, size_bytes: usize, dev_address: u8) -> Self {
        let size_bytes = size_bytes.clamp(1, 65536);
        let data = vec![0u8; size_bytes];
        let scl = IoPin::input(format!("{id}-in1"));
        let sda = IoPin::open_collector(format!("{id}-in0"));
        let addr_pins = [
            IoPin::input(format!("{id}-in2")),
            IoPin::input(format!("{id}-in3")),
            IoPin::input(format!("{id}-in4")),
        ];
        let mut twi = TwiModule::new();
        twi.set_slave(dev_address & 0x7F);

        let mut st = Self {
            size_bytes,
            dev_address: dev_address & 0x7F,
            data,
            mem_ptr: 0,
            addr_bytes_left: 0,
            addr_acc: 0,
            scl,
            sda,
            addr_pins,
            twi,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        // Open-collector SDA must idle released (high-Z) so the host can drive the bus.
        st.sda.set_out_state(true);
        st
    }

    pub fn apply_family(&mut self) {
        self.family.apply(&mut self.scl);
        self.family.apply(&mut self.sda);
        for p in &mut self.addr_pins {
            self.family.apply(p);
        }
    }

    pub fn effective_address(&self) -> u8 {
        let mut strap = 0u8;
        for (i, pin) in self.addr_pins.iter().enumerate() {
            if pin.inp_state() {
                strap |= 1 << i;
            }
        }
        (self.dev_address & 0x7F) | strap
    }

    pub fn addr_width_bytes(&self) -> u8 {
        if self.size_bytes > 256 { 2 } else { 1 }
    }

    pub fn tick(&mut self) {
        let addr = self.effective_address();
        if self.twi.address != addr {
            self.twi.set_slave(addr);
        }
        let tick = self.twi.volt_changed_pins(&mut self.scl, &mut self.sda);
        if tick.need_start_write {
            self.addr_bytes_left = self.addr_width_bytes();
            self.addr_acc = 0;
        }
        if let Some(byte) = tick.byte_received {
            if self.addr_bytes_left > 0 {
                self.addr_acc = (self.addr_acc << 8) | u16::from(byte);
                self.addr_bytes_left -= 1;
                if self.addr_bytes_left == 0 {
                    self.mem_ptr = (self.addr_acc as usize % self.data.len()) as u16;
                }
            } else {
                let idx = self.mem_ptr as usize % self.data.len();
                self.data[idx] = byte;
                self.mem_ptr = ((idx + 1) % self.data.len()) as u16;
            }
        }
        if tick.need_slave_write {
            let idx = self.mem_ptr as usize % self.data.len();
            self.twi.tx_reg = self.data[idx];
            self.mem_ptr = ((idx + 1) % self.data.len()) as u16;
        }
    }
}
