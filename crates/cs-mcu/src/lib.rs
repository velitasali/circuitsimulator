//! MCU cores: data space, Intel HEX, `.mcu` XML, PIC12 / PIC14 / AVR /
//! 8051 / 6502 / Z80 instruction decode, GPIO ports, interrupt controller,
//! timers, OC / PWM / IC, PIC CCP PWM, Tinyx5 T1 PWM1A/B, USART TX/RX,
//! scripted `<usart>`/`<spi>`/`<twi>` + `script` attrs, 8051 EA/ALE/PSEN bus
//! and MOVX XRAM, 6502 / Z80 address / data bus GPIO.
//! No Qt. QEMU devices come after this core.
//!
//! Pin analog stamps stay in `cs-engine`; this crate reports GPIO direction
//! and level so the host can drive `McuPin` / `IoPin`.

mod ccp;
mod cpu;
mod dataspace;
mod desc;
mod device;
mod hex;
mod icunit;
mod interrupts;
mod ocunit;
mod port;
mod timer;
pub mod twi;
mod usart;
mod xml;

pub use dataspace::{DataSpace, RegInfo, Watch};
pub use desc::{CoreKind, McuDesc, StackSpec};
pub use device::{Device, Mcu, McuState};
pub use hex::{HexError, encode_hex, load_hex};
pub use interrupts::Interrupts;
pub use port::{GpioPin, Port, PortSpec};
pub use twi::Twi;
pub use xml::{parse_mcu_file, parse_mcu_xml};

#[derive(Debug)]
pub enum Error {
    Parse(String),
    Hex(HexError),
    Io(std::io::Error),
    Unsupported(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(s) | Error::Unsupported(s) => write!(f, "{s}"),
            Error::Hex(e) => write!(f, "{e}"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Hex(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<HexError> for Error {
    fn from(e: HexError) -> Self {
        Error::Hex(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
