//! Analog + digital engine: modified nodal analysis, primitive parts, `.sim1`
//! load, the Qt-free circuit canvas, overlay layout, the component catalog,
//! live instruments, plot sample buffers, the event-driven logic engine,
//! TestUnit batch tests, host serial (`serialport`) and audio (`cpal`),
//! PNG/JPEG/BMP/SVG export from the canvas scene.
//! Headless CLI (`-nogui` / `-runcirc` / `-test`) lives in [`headless`].
//!
//! No Qt. PIC12 / PIC14 / AVR / 8051 / 6502 / Z80 cores live in `cs-mcu` and step through [`mcu`].
//! QEMU ESP32/STM32 GPIO / USART / SPI / TWI / timer is [`qemu`] (live
//! `qemu-system-*` when firmware is set); AngelScript `IoPort`/`IoPin`/
//! `McuPort`/`McuPin`/`Uart`/`SPI`/`TWI` host APIs are [`script`]
//! (`.mcu` `core="scripted"` auto-constructs USART/SPI/TWI + the `.as` file).

pub mod audio;
pub mod backup;
pub mod canvas;
pub mod catalog;
pub mod components;
pub mod simulation;
pub use simulation as circuit;
pub mod circ1;
pub mod debug;
pub mod digital;
pub mod editor;
pub mod elements;
pub mod headless;
pub mod highlighter;
pub mod i18n;
pub mod installer;
pub mod instruments;
pub mod library;
pub mod logging;
mod matrix;
pub mod mcu;
pub mod memdata;
mod net;
pub mod overlays;
pub mod overload;
pub mod package;
pub mod plot;
pub mod project;
pub mod qemu;
pub mod script;
pub mod serial;
pub mod settings;
pub mod sim1;
pub use logging as log;
pub mod sim_log;
pub mod subcircuit;
pub mod theme;
pub mod units;
pub mod wav;

pub use logging::{LogCategory, log_compiler, log_default, log_sim};

pub use canvas::PinDirection;
pub use catalog::{Catalog, CatalogItem};
pub use circ1::{ParsedCirc1, parse_circ1, write_circ1};
pub use circuit::Circuit;
pub use digital::TestResult;
pub use elements::pins::*;
pub use elements::{
    ANALOG_DT_DEFAULT, BATTERY_DEFAULT_OHMS, BATTERY_DEFAULT_VOLTS, CAPACITOR_DEFAULT_FARADS,
    COMPARATOR_DEFAULT_OUT_HIGH, COMPARATOR_DEFAULT_OUT_IMP, COMPARATOR_DEFAULT_OUT_LOW,
    FIXED_VOLT_DEFAULT, INDUCTOR_DEFAULT_HENRIES, OPAMP_DEFAULT_GAIN, OPAMP_DEFAULT_OUT_IMP,
    OPAMP_DEFAULT_VOLT_NEG, OPAMP_DEFAULT_VOLT_POS, RESISTOR_DEFAULT_OHMS, SOURCE_ADMIT,
    SWITCH_CLOSED_ADMIT, VOLTREG_DEFAULT_VREF,
};
pub use instruments::{
    AMMETER_OHMS, LA_THRESHOLD_DEFAULT, PROBE_DEFAULT_THRESHOLD, SCOPE_TIME_DIV_DEFAULT,
    VOLTMETER_OHMS,
};
pub use matrix::CircMatrix;
pub use mcu::{McuPoke, McuSnap, monitor_snap, publish_monitor};
pub use package::{Package, PkgPin, packages_from_sim1};
pub use project::ProjectSession;
pub use settings::{AppSettings, CircSettings};
pub use sim1::{ParsedCircuit, parse_legacy_to_circ1, parse_sim1};
pub use subcircuit::{SubcSearch, device_from_id, instantiate};
pub use units::{format_si, join_time, parse_si, si_multiplier, split_time};
pub use wav::WavData;

/// C++ `eElement::cero_doub`. Ground's open-low output sits here, not at 0 V.
pub const CERO_DOUB: f64 = 1e-9;
/// C++ `eElement::low_imp`.
pub const LOW_IMP: f64 = 1e-7;
/// C++ `eElement::high_imp`.
pub const HIGH_IMP: f64 = 1e7;

#[derive(Debug)]
pub enum Error {
    Parse(String),
    Singular,
    NotConverged,
    Export(String),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "{msg}"),
            Error::Singular => write!(f, "could not solve matrix"),
            Error::NotConverged => write!(f, "nonlinear solver did not converge"),
            Error::Export(msg) => write!(f, "{msg}"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
