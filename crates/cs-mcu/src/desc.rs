//! Parsed `.mcu` description (C++ `McuCreator` XML).

use crate::dataspace::DataSpace;
use crate::port::PortSpec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreKind {
    Pic12,
    Pic14,
    Avr,
    I51,
    Mcs65,
    Z80,
    Scripted,
    Other,
}

impl CoreKind {
    pub fn from_attr(s: &str) -> Self {
        match s {
            "Pic12" => Self::Pic12,
            "Pic14" | "Pic14e" => Self::Pic14,
            "AVR" => Self::Avr,
            "8051" => Self::I51,
            "6502" => Self::Mcs65,
            "Z80" | "Z80ULA" => Self::Z80,
            "scripted" => Self::Scripted,
            _ => Self::Other,
        }
    }

    pub fn supported(self) -> bool {
        matches!(
            self,
            Self::Pic12
                | Self::Pic14
                | Self::Avr
                | Self::I51
                | Self::Mcs65
                | Self::Z80
                | Self::Scripted
        )
    }
}

/// C++ `McuCreator::createStack`: `spreg` + `increment` (`pre`/`inc`).
#[derive(Clone, Debug, Default)]
pub struct StackSpec {
    pub regs: Vec<String>,
    pub increment: String,
}

/// C++ `<interrupt>` inside `<interrupts>`.
#[derive(Clone, Debug, Default)]
pub struct InterruptSpec {
    pub name: String,
    pub vector: u16,
    pub enable: String,
    pub flag: String,
    pub priority: String,
    pub clear_on_one: bool,
    pub auto_clear: Option<bool>,
    pub pin: Option<String>,
    pub wakeup: u8,
}

/// C++ `<interrupts enable="GIE">`.
#[derive(Clone, Debug, Default)]
pub struct InterruptsSpec {
    pub enable: String,
    pub ints: Vec<InterruptSpec>,
}

/// Shared configregs/configbits attrs (C++ `setConfigRegs`).
#[derive(Clone, Debug, Default)]
pub struct ConfigSpec {
    pub regs_a: String,
    pub regs_b: String,
    pub regs_c: String,
    pub bits_a: String,
    pub bits_b: String,
    pub bits_c: String,
}

/// C++ `<ocunit>` inside `<timer>`.
#[derive(Clone, Debug, Default)]
pub struct OcUnitSpec {
    pub name: String,
    pub pin: String,
    pub ocreg: Vec<String>,
    pub interrupt: String,
    pub bits: String,
    pub config: ConfigSpec,
}

/// C++ `<icunit>` inside `<timer>`.
#[derive(Clone, Debug, Default)]
pub struct IcUnitSpec {
    pub name: String,
    pub pin: String,
    pub icreg: Vec<String>,
    pub interrupt: String,
    pub bits: String,
}

/// C++ `<port><interrupt name mask bitmask>`.
#[derive(Clone, Debug, Default)]
pub struct PortIntSpec {
    pub name: String,
    pub mask: String,
    pub bitmask: String,
}

/// C++ `<port><extint name pin configbits>`.
#[derive(Clone, Debug, Default)]
pub struct ExtIntSpec {
    pub name: String,
    pub pin: String,
    pub config_bits: String,
}

/// C++ `<timer>`.
#[derive(Clone, Debug, Default)]
pub struct TimerSpec {
    pub name: String,
    pub type_id: i32,
    pub counter: Vec<String>,
    pub enable: String,
    pub interrupt: String,
    pub clock_pin: Vec<String>,
    pub top_reg0: Vec<String>,
    pub prescalers: String,
    pub pr_select: String,
    pub config: ConfigSpec,
    pub oc_units: Vec<OcUnitSpec>,
    pub ic_unit: Option<IcUnitSpec>,
}

/// C++ `<ccpunit>` (PIC Capture/Compare/PWM).
#[derive(Clone, Debug, Default)]
pub struct CcpSpec {
    pub name: String,
    pub type_id: i32,
    pub pin: String,
    pub ccpreg: Vec<String>,
    pub interrupt: String,
    pub config: ConfigSpec,
}

/// C++ `<trunit type="tx|rx">`.
#[derive(Clone, Debug, Default)]
pub struct TrUnitSpec {
    pub register: String,
    pub pins: Vec<String>,
    pub enable: String,
    pub interrupt: String,
    pub config: ConfigSpec,
}

/// C++ `<usart>`.
#[derive(Clone, Debug, Default)]
pub struct UsartSpec {
    pub name: String,
    pub number: i32,
    pub core: Option<String>,
    pub interrupt: String,
    pub config: ConfigSpec,
    pub tx: Option<TrUnitSpec>,
    pub rx: Option<TrUnitSpec>,
}

/// C++ `<spi>` (`McuCreator::createSpi`). `pins` is MOSI, MISO, SCK, SS.
#[derive(Clone, Debug, Default)]
pub struct SpiSpec {
    pub name: String,
    pub pins: Vec<String>,
    pub data_reg: String,
    pub status_reg: String,
    pub interrupt: String,
    pub prescalers: String,
    pub config: ConfigSpec,
}

/// C++ `<twi>` (`McuCreator::createTwi`). `pins` is SDA, SCL.
#[derive(Clone, Debug, Default)]
pub struct TwiSpec {
    pub name: String,
    pub pins: Vec<String>,
    pub data_reg: String,
    pub addr_reg: String,
    pub status_reg: String,
    pub interrupt: String,
    pub prescalers: String,
    pub config: ConfigSpec,
}

impl StackSpec {
    pub fn pre(&self) -> bool {
        self.increment.contains("pre")
    }

    pub fn inc(&self) -> i16 {
        if self.increment.contains("inc") {
            1
        } else {
            -1
        }
    }
}

#[derive(Clone, Debug)]
pub struct McuDesc {
    pub core: CoreKind,
    pub core_name: String,
    pub data_size: u32,
    pub prog_size: u32,
    pub word_size: u8,
    pub eeprom_size: u32,
    pub inst_cycle: f64,
    pub cpu_cycle: f64,
    pub freq: f64,
    pub clkpin: Option<String>,
    pub data: DataSpace,
    pub ports: Vec<PortSpec>,
    pub prog_fill: u16,
    pub prog_init: Vec<(u16, u16)>,
    pub stack: Option<StackSpec>,
    pub prog_page: u8,
    pub interrupts: InterruptsSpec,
    pub timers: Vec<TimerSpec>,
    pub ccps: Vec<CcpSpec>,
    pub usarts: Vec<UsartSpec>,
    pub spis: Vec<SpiSpec>,
    pub twis: Vec<TwiSpec>,
    /// C++ `<mcu script="…">` next to the `.mcu` file (scripted cores).
    pub script: Option<String>,
}

impl McuDesc {
    pub fn new(core: CoreKind, core_name: impl Into<String>) -> Self {
        Self {
            core,
            core_name: core_name.into(),
            data_size: 0,
            prog_size: 0,
            word_size: 2,
            eeprom_size: 0,
            inst_cycle: 1.0,
            cpu_cycle: 1.0,
            freq: 0.0,
            clkpin: None,
            data: DataSpace::new(0),
            ports: Vec::new(),
            prog_fill: match core {
                CoreKind::Pic12 => 0x0FFF,
                CoreKind::Pic14 => 0x3FFF,
                _ => 0xFFFF,
            },
            prog_init: Vec::new(),
            stack: None,
            prog_page: 0,
            interrupts: InterruptsSpec::default(),
            timers: Vec::new(),
            ccps: Vec::new(),
            usarts: Vec::new(),
            spis: Vec::new(),
            twis: Vec::new(),
            script: None,
        }
    }
}
