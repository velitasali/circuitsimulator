//! Digital Memory IC state machines matching C++ `Memory`, `DynamicMemory`, and `I2CRam`.

use super::family::LogicFamily;
use super::pin::IoPin;
use super::queue::OutQueue;

// ============================================================================
// Parallel Static RAM / ROM (Memory)
// ============================================================================

#[derive(Clone, Debug)]
pub struct MemoryState {
    pub addr_bits: usize,      // e.g. 8 bits (256 bytes) to 16 bits (64KB)
    pub data_bits: usize,      // e.g. 8 bits
    pub data: Vec<u8>,         // Memory buffer
    pub addr_pins: Vec<IoPin>, // A0..An-1
    pub data_pins: Vec<IoPin>, // D0..Dm-1 (bidirectional)
    pub cs: IoPin,             // Chip Select (active low)
    pub oe: IoPin,             // Output Enable (active low)
    pub we: IoPin,             // Write Enable (active low)
    pub is_rom: bool,          // Read-only memory mode
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl MemoryState {
    pub fn new(id: &str, addr_bits: usize, data_bits: usize, is_rom: bool) -> Self {
        let addr_bits = addr_bits.clamp(1, 16);
        let data_bits = data_bits.clamp(1, 16);
        let size = 1usize << addr_bits;
        let data = vec![0u8; size];

        let mut addr_pins = Vec::with_capacity(addr_bits);
        for i in 0..addr_bits {
            addr_pins.push(IoPin::input(format!("{id}-a{i}")));
        }
        let mut data_pins = Vec::with_capacity(data_bits);
        for i in 0..data_bits {
            data_pins.push(IoPin::output(format!("{id}-d{i}")));
        }
        let cs = IoPin::input(format!("{id}-cs"));
        let oe = IoPin::input(format!("{id}-oe"));
        let we = IoPin::input(format!("{id}-we"));

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
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.addr_pins {
            self.family.apply(p);
        }
        for p in &mut self.data_pins {
            self.family.apply(p);
        }
        self.family.apply(&mut self.cs);
        self.family.apply(&mut self.oe);
        self.family.apply(&mut self.we);
    }

    pub fn eval(&mut self) {
        let cs_active = !self.cs.inp_state(); // active-low
        if !cs_active {
            for p in &mut self.data_pins {
                p.set_out_state(false);
            }
            return;
        }

        let mut addr = 0usize;
        for (i, p) in self.addr_pins.iter().enumerate() {
            if p.inp_state() {
                addr |= 1 << i;
            }
        }
        addr = addr.min(self.data.len() - 1);

        let we_active = !self.is_rom && !self.we.inp_state(); // active-low WE
        if we_active {
            let mut val = 0u8;
            for (i, p) in self.data_pins.iter().enumerate() {
                if p.inp_state() {
                    val |= 1 << i;
                }
            }
            self.data[addr] = val;
        }

        let oe_active = !self.oe.inp_state(); // active-low OE
        if oe_active && !we_active {
            let val = self.data[addr];
            for (i, p) in self.data_pins.iter_mut().enumerate() {
                p.set_out_state(((val >> i) & 1) != 0);
            }
        } else if !we_active {
            for p in &mut self.data_pins {
                p.set_out_state(false);
            }
        }
    }
}

// ============================================================================
// Dynamic RAM (DynamicMemory)
// ============================================================================

#[derive(Clone, Debug)]
pub struct DynamicMemoryState {
    pub addr_bits: usize, // Multiplexed address bits (e.g. 8 bits -> 64K matrix)
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
            addr_pins.push(IoPin::input(format!("{id}-a{i}")));
        }
        let mut data_pins = Vec::with_capacity(data_bits);
        for i in 0..data_bits {
            data_pins.push(IoPin::output(format!("{id}-d{i}")));
        }
        let ras = IoPin::input(format!("{id}-ras"));
        let cas = IoPin::input(format!("{id}-cas"));
        let we = IoPin::input(format!("{id}-we"));
        let oe = IoPin::input(format!("{id}-oe"));

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
        st
    }

    pub fn apply_family(&mut self) {
        for p in &mut self.addr_pins {
            self.family.apply(p);
        }
        for p in &mut self.data_pins {
            self.family.apply(p);
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
        // Falling edge on CAS latches col address and performs read/write
        if !cas_now && self.last_cas {
            self.col_addr = curr_addr;
            let full_addr = (self.row_addr << self.addr_bits) | self.col_addr;
            let full_addr = full_addr.min(self.data.len() - 1);

            let we_active = !self.we.inp_state(); // active low WE
            if we_active {
                let mut val = 0u8;
                for (i, p) in self.data_pins.iter().enumerate() {
                    if p.inp_state() {
                        val |= 1 << i;
                    }
                }
                self.data[full_addr] = val;
            } else {
                let oe_active = !self.oe.inp_state(); // active low OE
                let val = if oe_active { self.data[full_addr] } else { 0 };
                for (i, p) in self.data_pins.iter_mut().enumerate() {
                    p.set_out_state(oe_active && (val & (1 << i) != 0));
                }
            }
        } else if cas_now || self.oe.inp_state() {
            for p in &mut self.data_pins {
                p.set_out_state(false);
            }
        }
        self.last_cas = cas_now;
    }
}

// ============================================================================
// 24Cxx I2C Serial EEPROM (I2CRam)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I2CRamStateMode {
    Idle,
    RecvAddress,
    RecvMemAddress,
    WriteData,
    ReadData,
}

#[derive(Clone, Debug)]
pub struct I2CRamState {
    pub size_bytes: usize,
    pub dev_address: u8, // Base I2C address, e.g. 0x50
    pub data: Vec<u8>,   // e.g. 256 bytes (24C02) up to 8KB (24C64)
    pub mem_ptr: u16,
    pub mode: I2CRamStateMode,
    pub bit_count: u8,
    pub rx_byte: u8,
    pub scl: IoPin,
    pub sda: IoPin,
    pub last_scl: bool,
    pub last_sda: bool,
    pub family: LogicFamily,
    pub queue: OutQueue,
}

impl I2CRamState {
    pub fn new(id: &str, size_bytes: usize, dev_address: u8) -> Self {
        let size_bytes = size_bytes.clamp(128, 65536);
        let data = vec![0u8; size_bytes];
        let scl = IoPin::input(format!("{id}-scl"));
        let sda = IoPin::open_collector(format!("{id}-sda"));

        let mut st = Self {
            size_bytes,
            dev_address: dev_address & 0x78,
            data,
            mem_ptr: 0,
            mode: I2CRamStateMode::Idle,
            bit_count: 0,
            rx_byte: 0,
            scl,
            sda,
            last_scl: true,
            last_sda: true,
            family: LogicFamily::default(),
            queue: OutQueue::new(),
        };
        st.apply_family();
        st
    }

    pub fn apply_family(&mut self) {
        self.family.apply(&mut self.scl);
        self.family.apply(&mut self.sda);
    }

    pub fn tick(&mut self) {
        let sda_in = self.sda.inp_state();
        let scl_in = self.scl.inp_state();

        // I2C Start condition: SDA falling while SCL is high
        if scl_in && self.last_sda && !sda_in {
            self.mode = I2CRamStateMode::RecvAddress;
            self.bit_count = 0;
            self.rx_byte = 0;
        }
        // I2C Stop condition: SDA rising while SCL is high
        if scl_in && !self.last_sda && sda_in {
            self.mode = I2CRamStateMode::Idle;
        }

        // Clock edge processing
        if !self.last_scl && scl_in {
            // Rising edge of SCL: Sample SDA
            if self.mode == I2CRamStateMode::RecvAddress {
                self.rx_byte = (self.rx_byte << 1) | (if sda_in { 1 } else { 0 });
                self.bit_count += 1;
                if self.bit_count == 8 {
                    let addr = self.rx_byte >> 1;
                    let is_read = (self.rx_byte & 1) != 0;
                    if (addr & 0x78) == self.dev_address {
                        // ACK and transition
                        if is_read {
                            self.mode = I2CRamStateMode::ReadData;
                        } else {
                            self.mode = I2CRamStateMode::RecvMemAddress;
                        }
                    } else {
                        self.mode = I2CRamStateMode::Idle;
                    }
                    self.bit_count = 0;
                }
            } else if self.mode == I2CRamStateMode::RecvMemAddress {
                self.rx_byte = (self.rx_byte << 1) | (if sda_in { 1 } else { 0 });
                self.bit_count += 1;
                if self.bit_count == 8 {
                    self.mem_ptr = self.rx_byte as u16 % self.data.len() as u16;
                    self.mode = I2CRamStateMode::WriteData;
                    self.bit_count = 0;
                }
            } else if self.mode == I2CRamStateMode::WriteData {
                self.rx_byte = (self.rx_byte << 1) | (if sda_in { 1 } else { 0 });
                self.bit_count += 1;
                if self.bit_count == 8 {
                    let idx = self.mem_ptr as usize % self.data.len();
                    self.data[idx] = self.rx_byte;
                    self.mem_ptr = (self.mem_ptr + 1) % self.data.len() as u16;
                    self.bit_count = 0;
                }
            }
        }

        self.last_sda = sda_in;
        self.last_scl = scl_in;
    }
}
