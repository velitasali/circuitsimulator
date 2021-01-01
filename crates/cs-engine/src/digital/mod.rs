//! Event-driven digital engine: IoPin, logic family, gates, flip-flops,
//! latch, MCU pins as digital, TestUnit, USART / SPI / TWI bit-bang.
//! Analog stamps the Thevenin equivalent; voltChanged / delayed outputs run
//! on the picosecond event queue.

mod aip31068;
mod arithmetic;
mod clock;
mod converters;
mod event;
mod family;
mod flipflop;
mod gate;
mod hd44780;
pub mod hd44780_font;
mod ks0108;
mod lm555;
mod mcu_pin;
mod memory_ic;
mod pcd8544;
mod pin;
mod port;
mod queue;
mod sh1107;
mod spi;
mod ssd1306;
mod testunit;
mod twi;
mod usart;

pub use aip31068::Aip31068State;
pub use ks0108::Ks0108State;
pub use pcd8544::Pcd8544State;
pub use sh1107::Sh1107State;

pub use arithmetic::{
    BinCounterState, CounterState, FullAdderState, FunctionState, MagnitudeCompState, ShiftRegState,
};
pub use clock::{ClkState, Clocked, Trigger};
pub use converters::{
    AdcState, BcdTo7SState, BcdToDecState, DacState, DecToBcdState, DemuxState, I2CToParallelState,
    MuxState,
};
pub use event::{EventQueue, EventTarget};
pub use family::LogicFamily;
pub use flipflop::{FlipFlopKind, FlipFlopState, LatchState};
pub use gate::{GateOp, GateState, GateUpdate, apply_family_out};
pub use hd44780::Hd44780State;
pub use lm555::Lm555State;
pub use mcu_pin::McuPinState;
pub use memory_ic::{DynamicMemoryState, I2CRamState, MemoryState};
pub use pin::{IoPin, OC_HIGH_ADMIT, PinAction, PinMode};
pub use port::{IoPort, OutState};
pub use queue::{OutQueue, Schedule};
pub use spi::{SpiMode, SpiModule, SpiPins};
pub use ssd1306::Ssd1306State;
pub use testunit::{DEFAULT_PERIOD as TESTUNIT_DEFAULT_PERIOD, TestResult, TestUnitState};
pub use twi::{TwiMode, TwiModule, TwiPins, TwiState, TwiTick};
pub use usart::{Parity, UsartModule, UsartTick};

/// Seconds → picoseconds (C++ `Simulator` timeline).
pub fn secs_to_ps(s: f64) -> u64 {
    (s * 1e12).round().max(0.0) as u64
}

pub fn ps_to_secs(ps: u64) -> f64 {
    ps as f64 * 1e-12
}
