//! QEMU-backed MCU (C++ `QemuDevice` / `Esp32` / `Stm32`).
//!
//! Tests inject arena doorbells and the GPIO / USART / SPI / TWI / timer
//! modules drive `IoPin`s. Power-on with firmware + a `qemu-system-*` binary
//! spawns the child against a shared-memory arena (C++ `startQemuProcess`:
//! POSIX `shm_open` / Windows `CreateFileMappingA`). Not drawn (catalog-last).

pub mod periph;

use std::collections::BTreeMap;
use std::fs;
use std::hint;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use cs_qemu::{Arena, CosimWake, QemuProcess, SharedArena, SimAction};

use crate::debug::{DebugSession, ElfParser, QemuGdbClient};
use crate::digital::{IoPin, PinAction, PinMode, SpiPins, TwiPins};
use crate::package::{Package, PkgPin, convert_package, select_package};
use crate::subcircuit::SubcSearch;

use periph::{
    ESP32_HSPI_START, ESP32_I2C_SIZE, ESP32_I2C0_START, ESP32_I2C1_START, ESP32_IOMUX_SIZE,
    ESP32_IOMUX_START, ESP32_SPI_SIZE, ESP32_UART_SIZE, ESP32_UART0_START, ESP32_UART1_START,
    ESP32_UART2_START, ESP32_VSPI_START, Esp32IoMux, QemuSpi, QemuTimer, QemuTwi, QemuUsart,
    STM32_AFIO_START, STM32_I2C1_START, STM32_I2C2_START, STM32_PERIPH_SIZE, STM32_SPI1_START,
    STM32_SPI2_START, STM32_SPI3_START, STM32_TIM1_START, STM32_TIM2_START, STM32_TIM3_START,
    STM32_TIM4_START, STM32_UART4_START, STM32_UART5_START, STM32_USART1_START, STM32_USART2_START,
    STM32_USART3_START, Stm32Afio,
};

/// ESP32 IOMEM window (`IOMEM_END - IOMEM_BASE`).
pub const ESP32_IOMEM_SIZE: usize = 0x0007_FFFF;
/// STM32 IOMEM window.
pub const STM32_IOMEM_SIZE: usize = 0x0002_3400;
/// ESP32 GPIO module, IOMEM-relative (`0x3FF44000 - 0x3FF00000`).
pub const ESP32_GPIO_START: u64 = 0x0004_4000;
pub const ESP32_GPIO_END: u64 = 0x0004_4FFF;
/// STM32 GPIOA, IOMEM-relative (`0x40010800 - 0x40000000`).
pub const STM32_GPIOA_START: u64 = 0x0001_0800;
pub const STM32_GPIO_SIZE: u64 = 0x0000_0400;

pub const ESP32_FLASH_IMAGE_SIZE: u64 = 4_194_304;
pub const ESP32_BOOTLOADER_ADDR: u64 = 0x1000;
pub const ESP32_OTHER_BOOTLOADER_ADDR: u64 = 0x0;
pub const ESP32_IMAGE_HEADER_MAGIC: u8 = 0xE9;
pub const ESP32_CHIP_ID: u16 = 0x0000;

const GPIO_OUT: u64 = 0x04;
const GPIO_OUT_W1TS: u64 = 0x08;
const GPIO_OUT_W1TC: u64 = 0x0C;
const GPIO_OUT1: u64 = 0x10;
const GPIO_OUT1_W1TS: u64 = 0x14;
const GPIO_OUT1_W1TC: u64 = 0x18;
const GPIO_ENABLE: u64 = 0x20;
const GPIO_ENABLE_W1TS: u64 = 0x24;
const GPIO_ENABLE_W1TC: u64 = 0x28;
const GPIO_ENABLE1: u64 = 0x2C;
const GPIO_ENABLE1_W1TS: u64 = 0x30;
const GPIO_ENABLE1_W1TC: u64 = 0x34;
const GPIO_STRAP: u64 = 0x38;
const GPIO_IN: u64 = 0x3C;
const GPIO_IN1: u64 = 0x40;
const GPIO_FUNC_IN_SEL_CFG_BASE: u64 = 0x130;
const GPIO_FUNC_OUT_SEL_CFG_BASE: u64 = 0x530;
/// Bank 1 output-capable pins: only GPIO32 (bit 0) and GPIO33 (bit 1); GPIO34–39 are input-only.
const ESP32_BANK1_OUT_MASK: u32 = 0x03;
/// ESP32 IO_MUX FUN_WPU bit (internal pull-up enable).
const ESP32_IOMUX_FUN_WPU: u32 = 1 << 8;

const CRL_OFFSET: u64 = 0x00;
const CRH_OFFSET: u64 = 0x04;
const IDR_OFFSET: u64 = 0x08;
const ODR_OFFSET: u64 = 0x0C;
const BSRR_OFFSET: u64 = 0x10;
const BRR_OFFSET: u64 = 0x14;
const LCKR_OFFSET: u64 = 0x18;

/// Guest lead worth parking for. Shorter gaps are one mailbox slice, so a
/// 100 µs guest step does not turn into a 1 ms host wait.
const GUEST_AHEAD_PS: u64 = 50_000_000;
/// A single `qemuTime` sample farther than this ahead of the last accepted
/// value is treated as a torn word until the same value repeats.
const GUEST_JUMP_LIMIT_PS: u64 = 30_000_000_000_000;
/// Headless runs (no frame deadline) park this far ahead when the guest has
/// not published time, so the solver does not spin on a 1 ps reschedule.
const HEADLESS_PARK_PS: u64 = 1_000_000_000;
/// Maximum simulated time a busy-wait UART status poll may be deferred before
/// waking the guest anyway (1 ms), protecting slow baud rates.
const DEFER_POLL_MAX_PS: u64 = 1_000_000_000;
const USART_CATCH_CAP: u32 = 8_192;
const EDGE_CATCH_CAP: u32 = 16_384;

/// Pads without a package pin (C++ `m_pins.resize(40, dummy)`; 20,24,28–31).
const ESP32_UNUSED_PADS: [usize; 6] = [20, 24, 28, 29, 30, 31];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QemuFamily {
    Esp32,
    Stm32,
}

#[derive(Clone, Copy, Debug)]
struct ModuleRange {
    start: u64,
    end: u64,
    kind: ModuleKind,
}

#[derive(Clone, Copy, Debug)]
enum ModuleKind {
    Esp32Gpio,
    Esp32IoMux,
    Esp32Usart(u8),
    Esp32Spi(u8),
    Esp32Twi(u8),
    Stm32Port(u8),
    Stm32Afio,
    Stm32Usart(u8),
    Stm32Spi(u8),
    Stm32Twi(u8),
    Stm32Timer(u8),
    Dummy,
}

#[derive(Clone, Copy, Debug)]
struct PendingEvent {
    kind: ModuleKind,
    address: u64,
    value: u64,
    action: SimAction,
}

/// ESP32 IO_MUX register offset → GPIO pad number (ESP32 TRM Table 20).
fn esp32_iomux_offset_to_gpio(offset: u64) -> Option<usize> {
    match offset {
        0x04 => Some(36),
        0x08 => Some(37),
        0x0C => Some(38),
        0x10 => Some(39),
        0x14 => Some(34),
        0x18 => Some(35),
        0x1C => Some(32),
        0x20 => Some(33),
        0x24 => Some(25),
        0x28 => Some(26),
        0x2C => Some(27),
        0x30 => Some(14),
        0x34 => Some(12),
        0x38 => Some(13),
        0x3C => Some(15),
        0x40 => Some(2),
        0x44 => Some(0),
        0x48 => Some(4),
        0x4C => Some(16),
        0x50 => Some(17),
        0x54 => Some(9),
        0x58 => Some(10),
        0x5C => Some(11),
        0x60 => Some(6),
        0x64 => Some(7),
        0x68 => Some(8),
        0x6C => Some(5),
        0x70 => Some(18),
        0x74 => Some(19),
        0x78 => Some(20),
        0x7C => Some(21),
        0x80 => Some(22),
        0x84 => Some(3),
        0x88 => Some(1),
        0x8C => Some(23),
        0x90 => Some(24),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct Esp32Gpio {
    state: u32,
    enable: u32,
    state1: u32,
    enable1: u32,
    /// Pad number → index in `QemuComp::pins`.
    pad: [Option<usize>; 40],
}

impl Esp32Gpio {
    fn new() -> Self {
        Self {
            state: 0,
            enable: 0,
            state1: 0,
            enable1: 0,
            pad: [None; 40],
        }
    }
}

#[derive(Clone, Debug)]
struct Stm32Port {
    mem_start: u64,
    pin_state: u16,
    /// Pin 0..15 → index in `QemuComp::pins`.
    pins: [Option<usize>; 16],
}

/// A `SIM_READ` of a UART status register held back while TX is actively
/// shifting bits out, avoiding hundreds of back-to-back IPC round-trips per
/// byte during synchronous ROM/bootloader polling loops.
#[derive(Clone, Copy, Debug)]
struct DeferredPoll {
    usart_idx: usize,
    status: (usize, usize, u16),
    irq: (u64, u64),
    max_until: u64,
}

/// Live `qemu-system-*` child, the shm it attached, and the wake events
/// created before the process was spawned.
struct LiveQemu {
    shm: SharedArena,
    process: QemuProcess,
    wake: CosimWake,
    gdb: Option<QemuGdbClient>,
}

/// Circuit wrapper around a QEMU co-sim device.
pub struct QemuComp {
    pub uid: String,
    pub family: QemuFamily,
    pub device: String,
    pub firmware: String,
    pub firmware_dir: Option<PathBuf>,
    pub extra_args: String,
    pub gdb_port: i32,
    pub pins: Vec<IoPin>,
    pub node_indices: Vec<Option<usize>>,
    pub cached_usart_port_ids: Vec<String>,
    pub cached_spi_port_ids: Vec<String>,
    pub cached_twi_port_ids: Vec<String>,
    pub arena: Arena,
    pub time_offset: u64,
    /// Last `start_live` error (empty firmware is not an error).
    pub last_error: Option<String>,
    io_mem: Vec<u32>,
    modules: Vec<ModuleRange>,
    last_module: Option<usize>,
    event: Option<PendingEvent>,
    esp32_gpio: Esp32Gpio,
    pub esp32_iomux: Option<Esp32IoMux>,
    stm32_ports: Vec<Stm32Port>,
    pub stm32_afio: Option<Stm32Afio>,
    rst_pin: Option<usize>,
    cpu_freq: u32,
    apb_freq: u32,
    shm_key: String,
    next_wake: Option<u64>,
    pub pins_dirty: bool,
    pub usarts: Vec<QemuUsart>,
    pub spis: Vec<QemuSpi>,
    pub twis: Vec<QemuTwi>,
    timers: Vec<QemuTimer>,
    live: Option<LiveQemu>,
    /// The GUI frame should stop without jumping circuit time to its target.
    yield_frame: bool,
    /// Last accepted guest `qemuTime` (picoseconds on the guest clock).
    last_guest_raw: Option<u64>,
    /// A `qemuTime` jump past [`GUEST_JUMP_LIMIT_PS`], accepted only if it repeats.
    pending_guest_raw: Option<u64>,
    /// Address of the immediately preceding mailbox `SIM_READ`, used to detect
    /// tight busy-wait polling on a UART status register.
    last_mailbox_read_addr: Option<u64>,
    /// Pending UART status `SIM_READ` deferred until TX/RX/IRQ state changes.
    deferred_poll: Option<DeferredPoll>,
}

impl Clone for QemuComp {
    fn clone(&self) -> Self {
        Self {
            uid: self.uid.clone(),
            family: self.family,
            device: self.device.clone(),
            firmware: self.firmware.clone(),
            firmware_dir: self.firmware_dir.clone(),
            extra_args: self.extra_args.clone(),
            gdb_port: self.gdb_port,
            pins: self.pins.clone(),
            node_indices: self.node_indices.clone(),
            cached_usart_port_ids: self.cached_usart_port_ids.clone(),
            cached_spi_port_ids: self.cached_spi_port_ids.clone(),
            cached_twi_port_ids: self.cached_twi_port_ids.clone(),
            arena: self.arena,
            time_offset: self.time_offset,
            last_error: self.last_error.clone(),
            io_mem: self.io_mem.clone(),
            modules: self.modules.clone(),
            last_module: self.last_module,
            event: self.event,
            esp32_gpio: self.esp32_gpio.clone(),
            esp32_iomux: self.esp32_iomux.clone(),
            stm32_ports: self.stm32_ports.clone(),
            stm32_afio: self.stm32_afio.clone(),
            rst_pin: self.rst_pin,
            cpu_freq: self.cpu_freq,
            apb_freq: self.apb_freq,
            shm_key: self.shm_key.clone(),
            next_wake: self.next_wake,
            pins_dirty: self.pins_dirty,
            usarts: self.usarts.clone(),
            spis: self.spis.clone(),
            twis: self.twis.clone(),
            timers: self.timers.clone(),
            live: None,
            yield_frame: false,
            last_guest_raw: None,
            pending_guest_raw: None,
            last_mailbox_read_addr: None,
            deferred_poll: None,
        }
    }
}

impl Drop for QemuComp {
    fn drop(&mut self) {
        self.stop_live();
    }
}

impl std::fmt::Debug for QemuComp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QemuComp")
            .field("uid", &self.uid)
            .field("family", &self.family)
            .field("firmware", &self.firmware)
            .field("pins", &self.pins.len())
            .field("live", &self.is_live())
            .finish_non_exhaustive()
    }
}

impl Default for QemuComp {
    fn default() -> Self {
        Self::esp32("")
    }
}

impl QemuComp {
    pub fn esp32(id: impl Into<String>) -> Self {
        let id = id.into();
        let mut pins = Vec::new();
        let mut gpio = Esp32Gpio::new();
        for n in 0..40 {
            if ESP32_UNUSED_PADS.contains(&n) {
                continue;
            }
            let mut p = IoPin::input(format!("{id}-G{n:02}"));
            p.set_levels(3.3, 0.0);
            p.set_thresholds(1.65, 1.65);
            gpio.pad[n] = Some(pins.len());
            pins.push(p);
        }
        let mut rst = IoPin::input(format!("{id}-Rst"));
        rst.set_levels(3.3, 0.0);
        rst.set_thresholds(0.65, 0.65);
        rst.set_pullup(1e5);
        let _ = rst.get_inp_state(rst.out_volt());
        let rst_pin = Some(pins.len());
        pins.push(rst);

        let pad = |n: usize| gpio.pad[n];
        let usarts = vec![
            QemuUsart::esp32(ESP32_UART0_START, 0, pad(1), pad(3)),
            QemuUsart::esp32(ESP32_UART1_START, 1, pad(10), pad(9)),
            QemuUsart::esp32(ESP32_UART2_START, 2, pad(17), pad(16)),
        ];
        let spis = vec![
            QemuSpi::esp32(
                ESP32_HSPI_START,
                SpiPins {
                    mosi: pad(13),
                    miso: pad(12),
                    clk: pad(14),
                    ss: pad(15),
                },
            ),
            QemuSpi::esp32(
                ESP32_VSPI_START,
                SpiPins {
                    mosi: pad(23),
                    miso: pad(19),
                    clk: pad(18),
                    ss: pad(5),
                },
            ),
        ];
        let twis = vec![
            QemuTwi::esp32(
                ESP32_I2C0_START,
                0,
                TwiPins {
                    scl: pad(22),
                    sda: pad(21),
                },
            ),
            QemuTwi::esp32(
                ESP32_I2C1_START,
                1,
                TwiPins {
                    scl: pad(22),
                    sda: pad(21),
                },
            ),
        ];
        let mut modules = vec![ModuleRange {
            start: ESP32_GPIO_START,
            end: ESP32_GPIO_END,
            kind: ModuleKind::Esp32Gpio,
        }];
        for (i, u) in usarts.iter().enumerate() {
            modules.push(ModuleRange {
                start: u.mem_start,
                end: u.mem_start + ESP32_UART_SIZE - 1,
                kind: ModuleKind::Esp32Usart(i as u8),
            });
        }
        for (i, s) in spis.iter().enumerate() {
            modules.push(ModuleRange {
                start: s.mem_start,
                end: s.mem_start + ESP32_SPI_SIZE - 1,
                kind: ModuleKind::Esp32Spi(i as u8),
            });
        }
        for (i, t) in twis.iter().enumerate() {
            modules.push(ModuleRange {
                start: t.mem_start,
                end: t.mem_start + ESP32_I2C_SIZE - 1,
                kind: ModuleKind::Esp32Twi(i as u8),
            });
        }
        let iomux = Esp32IoMux::new(ESP32_IOMUX_START);
        modules.push(ModuleRange {
            start: ESP32_IOMUX_START,
            end: ESP32_IOMUX_START + ESP32_IOMUX_SIZE - 1,
            kind: ModuleKind::Esp32IoMux,
        });
        modules.push(ModuleRange {
            start: 0,
            end: ESP32_IOMEM_SIZE as u64,
            kind: ModuleKind::Dummy,
        });

        let n_pins = pins.len();
        let cached_usart_port_ids = (0..usarts.len())
            .map(|i| format!("{id}:USART{}", i + 1))
            .collect();
        let cached_spi_port_ids = (0..spis.len())
            .map(|i| format!("{id}:SPI{}", i + 1))
            .collect();
        let cached_twi_port_ids = (0..twis.len())
            .map(|i| format!("{id}:I2C{}", i + 1))
            .collect();

        Self {
            uid: id,
            family: QemuFamily::Esp32,
            device: "Esp32".into(),
            firmware: String::new(),
            firmware_dir: None,
            extra_args: String::new(),
            gdb_port: 0,
            pins,
            node_indices: vec![None; n_pins],
            cached_usart_port_ids,
            cached_spi_port_ids,
            cached_twi_port_ids,
            arena: Arena::default(),
            time_offset: 0,
            last_error: None,
            io_mem: vec![0; ESP32_IOMEM_SIZE],
            modules,
            last_module: None,
            event: None,
            esp32_gpio: gpio,
            esp32_iomux: Some(iomux),
            stm32_ports: Vec::new(),
            stm32_afio: None,
            rst_pin,
            cpu_freq: 240_000_000,
            apb_freq: 80_000_000,
            shm_key: String::new(),
            next_wake: None,
            pins_dirty: false,
            usarts,
            spis,
            twis,
            timers: Vec::new(),
            live: None,
            yield_frame: false,
            last_guest_raw: None,
            pending_guest_raw: None,
            last_mailbox_read_addr: None,
            deferred_poll: None,
        }
    }

    pub fn firmware_path(&self) -> Option<PathBuf> {
        if self.firmware.is_empty() {
            return None;
        }
        crate::mcu::resolve_firmware(self.firmware_dir.as_deref(), &self.firmware).or_else(|| {
            let p = PathBuf::from(&self.firmware);
            if p.exists() { Some(p) } else { None }
        })
    }

    /// `port_n` is C++ `m_portN` (GPIOA..). Default STM32F103C8 is 5.
    pub fn stm32(id: impl Into<String>, port_n: u8) -> Self {
        let id = id.into();
        let port_n = port_n.clamp(1, 7);
        let mut pins = Vec::new();
        let mut ports = Vec::new();
        let mut modules = Vec::new();
        let mut start = STM32_GPIOA_START;
        for p in 0..port_n {
            let letter = (b'A' + p) as char;
            let mut port = Stm32Port {
                mem_start: start,
                pin_state: 0,
                pins: [None; 16],
            };
            for i in 0..16 {
                let mut pin = IoPin::input(format!("{id}-P{letter}{i}"));
                pin.set_levels(3.3, 0.0);
                pin.set_thresholds(1.65, 1.65);
                port.pins[i] = Some(pins.len());
                pins.push(pin);
            }
            modules.push(ModuleRange {
                start,
                end: start + STM32_GPIO_SIZE - 1,
                kind: ModuleKind::Stm32Port(p),
            });
            ports.push(port);
            start += STM32_GPIO_SIZE;
        }
        let pin_at = |port: usize, bit: usize| ports.get(port).and_then(|p| p.pins[bit]);
        let usart_n = match port_n {
            1..=4 => 2,
            5 => 3,
            _ => 5,
        };
        let spi_n = match port_n {
            1..=4 => 1,
            5 => 2,
            _ => 3,
        };
        let i2c_n = if port_n <= 4 { 1 } else { 2 };
        let mut usarts = Vec::new();
        let usart_map = [
            (STM32_USART1_START, pin_at(0, 9), pin_at(0, 10)),
            (STM32_USART2_START, pin_at(0, 2), pin_at(0, 3)),
            (STM32_USART3_START, pin_at(1, 10), pin_at(1, 11)),
            (STM32_UART4_START, pin_at(0, 0), pin_at(0, 1)),
            (STM32_UART5_START, pin_at(2, 12), pin_at(3, 2)),
        ];
        for (i, &(base, tx, rx)) in usart_map.iter().enumerate().take(usart_n) {
            usarts.push(QemuUsart::stm32(base, i as u8, tx, rx));
            modules.push(ModuleRange {
                start: base,
                end: base + STM32_PERIPH_SIZE - 1,
                kind: ModuleKind::Stm32Usart(i as u8),
            });
        }
        let mut spis = Vec::new();
        let spi_map = [
            (
                STM32_SPI1_START,
                SpiPins {
                    mosi: pin_at(0, 7),
                    miso: pin_at(0, 6),
                    clk: pin_at(0, 5),
                    ss: pin_at(0, 4),
                },
            ),
            (
                STM32_SPI2_START,
                SpiPins {
                    mosi: pin_at(1, 15),
                    miso: pin_at(1, 14),
                    clk: pin_at(1, 13),
                    ss: pin_at(1, 12),
                },
            ),
            (
                STM32_SPI3_START,
                SpiPins {
                    mosi: pin_at(1, 5),
                    miso: pin_at(1, 4),
                    clk: pin_at(1, 3),
                    ss: pin_at(0, 15),
                },
            ),
        ];
        for (i, &(base, map)) in spi_map.iter().enumerate().take(spi_n) {
            spis.push(QemuSpi::stm32(base, map));
            modules.push(ModuleRange {
                start: base,
                end: base + STM32_PERIPH_SIZE - 1,
                kind: ModuleKind::Stm32Spi(i as u8),
            });
        }
        let mut twis = Vec::new();
        let twi_map = [
            (
                STM32_I2C1_START,
                TwiPins {
                    scl: pin_at(1, 6),
                    sda: pin_at(1, 7),
                },
            ),
            (
                STM32_I2C2_START,
                TwiPins {
                    scl: pin_at(1, 10),
                    sda: pin_at(1, 11),
                },
            ),
        ];
        for (i, &(base, map)) in twi_map.iter().enumerate().take(i2c_n) {
            twis.push(QemuTwi::stm32(base, i as u8, map));
            modules.push(ModuleRange {
                start: base,
                end: base + STM32_PERIPH_SIZE - 1,
                kind: ModuleKind::Stm32Twi(i as u8),
            });
        }
        let mut timers = Vec::new();
        let timer_map = [
            (
                STM32_TIM1_START,
                [pin_at(0, 8), pin_at(0, 9), pin_at(0, 10), pin_at(0, 11)],
            ),
            (
                STM32_TIM2_START,
                [pin_at(0, 0), pin_at(0, 1), pin_at(0, 2), pin_at(0, 3)],
            ),
            (
                STM32_TIM3_START,
                [pin_at(0, 6), pin_at(0, 7), pin_at(1, 0), pin_at(1, 1)],
            ),
            (
                STM32_TIM4_START,
                [pin_at(1, 6), pin_at(1, 7), pin_at(1, 8), pin_at(1, 9)],
            ),
        ];
        for (i, &(base, oc)) in timer_map.iter().enumerate() {
            timers.push(QemuTimer::new(base, oc));
            modules.push(ModuleRange {
                start: base,
                end: base + STM32_PERIPH_SIZE - 1,
                kind: ModuleKind::Stm32Timer(i as u8),
            });
        }
        let afio = Stm32Afio::new(STM32_AFIO_START);
        modules.push(ModuleRange {
            start: STM32_AFIO_START,
            end: STM32_AFIO_START + STM32_PERIPH_SIZE - 1,
            kind: ModuleKind::Stm32Afio,
        });
        modules.push(ModuleRange {
            start: 0,
            end: STM32_IOMEM_SIZE as u64,
            kind: ModuleKind::Dummy,
        });
        let n_pins = pins.len();
        let cached_usart_port_ids = (0..usarts.len())
            .map(|i| format!("{id}:USART{}", i + 1))
            .collect();
        let cached_spi_port_ids = (0..spis.len())
            .map(|i| format!("{id}:SPI{}", i + 1))
            .collect();
        let cached_twi_port_ids = (0..twis.len())
            .map(|i| format!("{id}:I2C{}", i + 1))
            .collect();

        Self {
            uid: id,
            family: QemuFamily::Stm32,
            device: format!("STM32F103C8"),
            firmware: String::new(),
            firmware_dir: None,
            extra_args: String::new(),
            gdb_port: 0,
            pins,
            node_indices: vec![None; n_pins],
            cached_usart_port_ids,
            cached_spi_port_ids,
            cached_twi_port_ids,
            arena: Arena::default(),
            time_offset: 0,
            last_error: None,
            io_mem: vec![0; STM32_IOMEM_SIZE],
            modules,
            last_module: None,
            event: None,
            esp32_gpio: Esp32Gpio::new(),
            esp32_iomux: None,
            stm32_ports: ports,
            stm32_afio: Some(afio),
            rst_pin: None,
            cpu_freq: 72_000_000,
            apb_freq: 72_000_000,
            shm_key: String::new(),
            next_wake: None,
            pins_dirty: false,
            usarts,
            spis,
            twis,
            timers,
            live: None,
            yield_frame: false,
            last_guest_raw: None,
            pending_guest_raw: None,
            last_mailbox_read_addr: None,
            deferred_poll: None,
        }
    }

    pub fn refresh_cached_port_ids(&mut self) {
        self.cached_usart_port_ids = (0..self.usarts.len())
            .map(|i| format!("{}:USART{}", self.uid, i + 1))
            .collect();
        self.cached_spi_port_ids = (0..self.spis.len())
            .map(|i| format!("{}:SPI{}", self.uid, i + 1))
            .collect();
        self.cached_twi_port_ids = (0..self.twis.len())
            .map(|i| format!("{}:I2C{}", self.uid, i + 1))
            .collect();
    }

    /// Rebinds logical/serial identity without altering electrical pin IDs.
    pub fn set_serial_id(&mut self, id: &str) {
        self.uid = id.to_string();
        self.refresh_cached_port_ids();
    }

    pub fn rebind_id(&mut self, new_id: &str) {
        let old = self.uid.clone();
        if old == new_id {
            return;
        }
        self.uid = new_id.to_string();
        self.refresh_cached_port_ids();
        for p in &mut self.pins {
            if let Some(rest) = p.id.strip_prefix(&old) {
                p.id = format!("{new_id}{rest}");
            }
        }
    }

    pub fn update_node_indices(&mut self, pin_net: &rustc_hash::FxHashMap<String, usize>) {
        self.node_indices = self
            .pins
            .iter_mut()
            .map(|p| {
                let n = pin_net.get(&p.id).copied();
                p.node_idx = n;
                n
            })
            .collect();
    }

    pub fn sample_inputs_nodes(&mut self, nodes: &[crate::net::ENode]) {
        for (i, p) in self.pins.iter_mut().enumerate() {
            let volt = self
                .node_indices
                .get(i)
                .and_then(|&idx| idx)
                .and_then(|node_idx| nodes.get(node_idx))
                .map(|n| n.volt)
                .unwrap_or_else(|| p.out_volt());
            let _ = p.get_inp_state(volt);
        }
    }

    #[inline]
    pub fn mark_pins_dirty(&mut self) {
        self.pins_dirty = true;
    }

    #[inline]
    pub fn take_pins_dirty(&mut self) -> bool {
        let dirty = self.pins_dirty;
        self.pins_dirty = false;
        dirty
    }

    #[inline]
    pub fn take_yield_frame(&mut self) -> bool {
        let yield_frame = self.yield_frame;
        self.yield_frame = false;
        yield_frame
    }

    pub fn pin_ids(&self) -> Vec<String> {
        self.pins.iter().map(|p| p.id.clone()).collect()
    }

    pub fn set_shm_key(&mut self, key: impl Into<String>) {
        self.shm_key = key.into();
    }

    pub fn shm_key(&self) -> &str {
        &self.shm_key
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        self.stop_live();
        self.arena.reset();
        self.event = None;
        self.last_module = None;
        self.next_wake = None;
        self.yield_frame = false;
        self.last_guest_raw = None;
        self.pending_guest_raw = None;
        self.last_mailbox_read_addr = None;
        self.deferred_poll = None;
        self.last_error = None;
        self.esp32_gpio.state = 0;
        self.esp32_gpio.enable = 0;
        for u in &mut self.usarts {
            u.reset();
        }
        for s in &mut self.spis {
            s.reset();
        }
        for t in &mut self.twis {
            t.reset();
        }
        for tm in &mut self.timers {
            tm.reset();
        }
        let stm_starts: Vec<u64> = self.stm32_ports.iter().map(|p| p.mem_start).collect();
        for p in &mut self.stm32_ports {
            p.pin_state = 0;
        }
        for start in stm_starts {
            let cr = 0x4444_4444;
            self.write_mem(start + CRL_OFFSET, cr);
            self.write_mem(start + CRH_OFFSET, cr);
            self.write_mem(start + ODR_OFFSET, 0);
        }
        for pin in &mut self.pins {
            pin.initialize(slope_steps);
            pin.set_pin_mode(PinMode::Input);
        }
        if self.family == QemuFamily::Esp32 {
            if let Some(i) = self.rst_pin {
                self.pins[i].set_pullup(1e5);
                let v = self.pins[i].out_volt();
                let _ = self.pins[i].get_inp_state(v);
            }
            // Only UART0 (G01/G03) is active on boot for the ESP32 ROM banner.
            // UART1 (G10/G09) and UART2 (G17/G16) stay unconfigured until
            // firmware writes their registers so those pads remain usable as GPIO.
            if let Some(u) = self.usarts.first_mut() {
                let tx = u.tx;
                u.module.enable_tx(true, &mut self.pins, tx);
                u.module.enable_rx(true);
            }
        }
        if let Err(e) = self.start_live() {
            crate::logging::log_sim_err(format!("[QEMU] {e}"));
            self.last_error = Some(e);
        }
    }

    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// Kill the child (C++ `killQemuProcess`) and drop the shm.
    pub fn stop_live(&mut self) {
        if let Some(mut live) = self.live.take() {
            live.process.kill();
            if let Some(mut gdb) = live.gdb.take() {
                gdb.shutdown();
            }
        }
        self.next_wake = None;
        self.last_mailbox_read_addr = None;
        self.deferred_poll = None;
    }

    /// SIGSTOP (C++ `stopQemuProcess`). No-op without a live child or with GDB.
    /// On Windows freeze-on-pause is a no-op (`SIGSTOP` has no equivalent).
    pub fn pause(&mut self) {
        if let Some(live) = &mut self.live {
            live.process.gdb = self.gdb_port != 0 || live.gdb.is_some();
            live.process.stop();
        }
    }

    /// SIGCONT (C++ `resumeQemuProcess`).
    pub fn resume(&mut self) {
        if let Some(live) = &mut self.live {
            live.process.resume();
        }
    }
}

static SHM_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn next_shm_key(uid: &str) -> String {
    let seq = SHM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    cs_qemu::shm_key(std::process::id(), &format!("{uid}-{seq}"))
}

impl QemuComp {
    /// Spawn `qemu-system-*` if firmware is set and the binary exists.
    /// Empty firmware is a no-op (inject-doorbell tests).
    pub fn start_live(&mut self) -> Result<(), String> {
        self.start_live_timeout(Duration::from_secs(5))
    }

    pub fn start_live_timeout(&mut self, timeout: Duration) -> Result<(), String> {
        if self.firmware.is_empty() {
            return Ok(());
        }
        let exe = qemu_executable(self.family).ok_or_else(|| {
            format!(
                "emulator executable not found: {}",
                qemu_exe_name(self.family)
            )
        })?;
        if self.shm_key.is_empty() {
            self.shm_key = next_shm_key(&self.uid);
        }
        let debug_active = DebugSession::is_active_global();
        let (mut args, firm_path) = self.build_args(&exe)?;

        // When starting in Debug mode, automatically load companion ELF symbols
        // if none are loaded yet, and attach an internal GDB client over an
        // ephemeral loopback TCP port.
        let mut internal_gdb_port: Option<u16> = None;
        if debug_active {
            load_companion_elf_if_needed(&firm_path);
            if self.gdb_port > 0 && self.gdb_port <= i32::from(u16::MAX) {
                internal_gdb_port = Some(self.gdb_port as u16);
            } else if let Some(port) = pick_free_loopback_port() {
                args.push("-gdb".into());
                args.push(format!("tcp:127.0.0.1:{port}"));
                args.push("-S".into());
                internal_gdb_port = Some(port);
            }
        }

        self.spawn_live_inner(&exe, &args, timeout, internal_gdb_port)
    }

    /// Spawn a specific binary (tests use the cosim stub / `/bin/true`).
    pub fn start_live_with(
        &mut self,
        executable: &Path,
        args: &[String],
        timeout: Duration,
    ) -> Result<(), String> {
        self.spawn_live_inner(executable, args, timeout, None)
    }

    fn spawn_live_inner(
        &mut self,
        executable: &Path,
        args: &[String],
        timeout: Duration,
        internal_gdb_port: Option<u16>,
    ) -> Result<(), String> {
        self.stop_live();
        if self.shm_key.is_empty() {
            self.shm_key = next_shm_key(&self.uid);
        }
        crate::logging::log_sim(format!(
            "[QEMU] Spawning emulator: {} with key: {}",
            executable.display(),
            self.shm_key
        ));
        crate::logging::log_sim(format!("[QEMU] Args: {:?}", args));
        let mut shm = SharedArena::create(&self.shm_key).map_err(|e| e.to_string())?;
        shm.as_mut().reset();
        // Events exist before the child starts so it can open them by name.
        let wake = CosimWake::create(&self.shm_key).unwrap_or_else(|e| {
            crate::logging::log_sim(format!("[QEMU] mailbox wake unavailable: {e}"));
            CosimWake::sleeping()
        });
        let mut process = QemuProcess::spawn(executable, args).map_err(|e| e.to_string())?;
        process.gdb = self.gdb_port != 0 || internal_gdb_port.is_some();
        let start = Instant::now();
        while shm.load_running() == 0 {
            if !process.is_running() {
                crate::logging::log_sim("[QEMU] Process terminated early during spawn");
                return Err(format!(
                    "emulator did not start: {} (process terminated early)",
                    executable.display()
                ));
            }
            if start.elapsed() > timeout {
                crate::logging::log_sim("[QEMU] Timed out waiting for arena->running");
                process.kill();
                return Err(format!(
                    "emulator did not start: {} (timed out waiting for arena->running)",
                    executable.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        crate::logging::log_sim(format!(
            "[QEMU] Attached and running successfully (startup took {:?})",
            start.elapsed()
        ));
        let gdb = if let Some(port) = internal_gdb_port {
            match QemuGdbClient::connect(port, self.family) {
                Ok(client) => Some(client),
                Err(e) => {
                    crate::logging::log_sim_err(format!("[GDB] {e}"));
                    None
                }
            }
        } else {
            None
        };
        self.arena = *shm.as_ref();
        self.live = Some(LiveQemu {
            shm,
            process,
            wake,
            gdb,
        });
        Ok(())
    }

    fn build_args(&self, _executable: &Path) -> Result<(Vec<String>, PathBuf), String> {
        let firm = self
            .firmware_path()
            .unwrap_or_else(|| PathBuf::from(&self.firmware));
        match self.family {
            QemuFamily::Esp32 => {
                let bytes = std::fs::read(&firm).map_err(|e| {
                    format!("Could not open firmware file:\n{}\n{e}", firm.display())
                })?;
                check_flash_image(&bytes)?;
                let key = if self.shm_key.is_empty() {
                    next_shm_key(&self.uid)
                } else {
                    self.shm_key.clone()
                };
                let rom = qemu_rom_dir().ok_or_else(|| {
                    "ESP32 ROM directory not found (esp32/rom/bin or qemu pc-bios)".to_string()
                })?;
                let efuse = ensure_esp32_efuse(&firm)?;
                let args = self.esp32_args(
                    firm.to_str().unwrap_or(""),
                    &key,
                    rom.to_str().unwrap_or(""),
                    efuse.to_str().unwrap_or(""),
                );
                Ok((args, firm))
            }
            QemuFamily::Stm32 => {
                let key = if self.shm_key.is_empty() {
                    next_shm_key(&self.uid)
                } else {
                    self.shm_key.clone()
                };
                Ok((self.stm32_args(firm.to_str().unwrap_or(""), &key), firm))
            }
        }
    }

    fn pull_live(&mut self) {
        let Some(view) = self.live.as_ref().map(|live| live.shm.load_mailbox()) else {
            return;
        };
        self.arena.simu_action = view.simu_action;
        self.arena.simu_time = view.simu_time;
        self.arena.qemu_time = view.qemu_time;
        self.arena.reg_addr = view.reg_addr;
        self.arena.reg_data = view.reg_data;
        self.arena.running = view.running;
        self.arena.qemu_action = view.qemu_action;
    }

    fn push_live(&mut self) {
        let Some(live) = &mut self.live else {
            return;
        };
        live.shm.store_response(&self.arena);
        live.wake.signal_ack();
    }

    fn push_live_irq(&mut self) {
        let Some(live) = &mut self.live else {
            return;
        };
        live.shm
            .store_irq(self.arena.irq_number, self.arena.irq_level);
    }

    fn load_simu_action(&self) -> u64 {
        self.live
            .as_ref()
            .map(|live| live.shm.load_simu_action())
            .unwrap_or(0)
    }

    fn wait_request(&self, timeout_ms: u32) -> bool {
        self.live
            .as_ref()
            .is_some_and(|live| live.wake.wait_request(timeout_ms))
    }

    fn is_process_running(&mut self) -> bool {
        self.live
            .as_mut()
            .map(|l| l.process.is_running())
            .unwrap_or(false)
    }

    /// Delay until the next QEMU wake, or `None` if the child is gone.
    pub fn next_event_delay(&self, now: u64) -> Option<u64> {
        let mut best: Option<u64> = None;
        let take = |best: &mut Option<u64>, v: Option<u64>| {
            if let Some(v) = v {
                *best = Some(best.map_or(v, |b| b.min(v)));
            }
        };
        for u in &self.usarts {
            take(&mut best, u.remain(now));
        }
        for s in &self.spis {
            take(&mut best, s.module.remain(now));
        }
        for t in &self.twis {
            take(&mut best, t.module.remain(now));
        }
        if self.is_live() {
            take(
                &mut best,
                self.next_wake.map(|t| t.saturating_sub(now).max(1)),
            );
        }
        best
    }

    /// Earliest absolute peripheral deadline (USART TX/RX/timeout, SPI, TWI).
    fn next_periph_due(&self) -> Option<u64> {
        let mut best: Option<u64> = None;
        let take = |best: &mut Option<u64>, v: Option<u64>| {
            if let Some(v) = v {
                *best = Some(best.map_or(v, |b| b.min(v)));
            }
        };
        for u in &self.usarts {
            take(&mut best, u.next_due());
        }
        for s in &self.spis {
            take(&mut best, s.module.due);
        }
        for t in &self.twis {
            take(&mut best, t.module.due);
        }
        best
    }

    /// Observe updated input pin levels immediately after a circuit solve.
    /// Returns `true` when any UART RX pin just detected a falling start-bit
    /// edge and armed a future receive deadline.
    pub fn on_input_pins_changed(&mut self, now: u64) -> bool {
        let mut armed = false;
        for u in &mut self.usarts {
            if u.module.rx_pin_changed(now, &self.pins, u.rx) {
                armed = true;
            }
        }
        armed
    }

    pub fn sample_inputs(&mut self, v_of: &impl Fn(&str) -> f64) {
        for p in &mut self.pins {
            let v = v_of(&p.id);
            let _ = p.get_inp_state(v);
        }
    }

    /// Inject a doorbell the same way QEMU posts `simuAction`.
    pub fn post(&mut self, action: SimAction, addr: u64, data: u64, time_ps: u64) {
        self.arena.post(action, addr, data, time_ps);
    }

    /// Consume a posted action (C++ `runEvent` body when `nextTime <= now`).
    pub fn process_pending(&mut self) -> Vec<(usize, PinAction)> {
        self.pull_live();
        if self.arena.simu_action == 0 {
            return Vec::new();
        }
        let action = SimAction::from_u64(self.arena.simu_action);
        let event_time = self.arena.simu_time;
        self.arena.qemu_action = 0;
        if action == SimAction::Freq {
            self.updt_frequency();
        } else if action != SimAction::Event {
            self.do_action();
            self.run_module_event(event_time);
        }
        self.arena.simu_action = 0;
        self.arena.simu_time = 0;
        self.push_live();
        Vec::new()
    }

    /// One simulator wake. The inject path ticks peripherals due at `now`.
    /// A live child drains the mailbox, then catches peripherals up toward
    /// the guest clock. It does not spin.
    pub fn run_event(&mut self, v_of: &impl Fn(&str) -> f64, now: u64) -> Vec<(usize, PinAction)> {
        self.sample_inputs(v_of);
        if self.is_live() {
            self.run_live_event(now, now, None)
        } else {
            self.tick_peripherals(now);
            self.process_pending()
        }
    }

    /// Fast path: zero-allocation execution using pre-resolved node indices.
    ///
    /// `horizon` is the frame's absolute end. `deadline` is the wall-clock
    /// budget; `None` is a headless run and must not yield the caller.
    pub fn run_event_nodes_buf(
        &mut self,
        nodes: &[crate::net::ENode],
        now: u64,
        horizon: u64,
        deadline: Option<Instant>,
        actions: &mut Vec<(usize, PinAction)>,
    ) {
        self.sample_inputs_nodes(nodes);
        if self.is_live() {
            let act = self.run_live_event(now, horizon, deadline);
            actions.extend(act);
        } else {
            self.tick_peripherals(now);
            let act = self.process_pending();
            actions.extend(act);
        }
    }

    fn run_live_event(
        &mut self,
        now: u64,
        _horizon: u64,
        deadline: Option<Instant>,
    ) -> Vec<(usize, PinAction)> {
        self.yield_frame = false;
        self.run_module_event(now);
        // Service peripheral edges due at `now` so bit-by-bit TX/RX stays on
        // the circuit event queue and propagates through `solve_circuit()`.
        self.catch_up_peripherals(now);

        // `true` once this wake has applied a mailbox post. A hot guest is
        // drained until the frame budget; an idle guest gets one short wait.
        let mut hot = false;
        if let Some(dp) = self.deferred_poll {
            if !self.deferred_poll_ready(dp, now) {
                if !self.is_process_running() {
                    crate::logging::log_sim_err("[QEMU] Emulator process exited unexpectedly");
                    self.stop_live();
                }
                return Vec::new();
            }
            self.complete_deferred_poll(now);
            hot = true;
        }

        // Peripheral-only wake before the guest's scheduled wake time: drain
        // any immediately ready mailbox slots without blocking on `wait_request`,
        // then return so the circuit advances to the next bit edge.
        if let Some(saved_wake) = self.next_wake
            && saved_wake > now
        {
            while self.load_simu_action() != 0 {
                if deadline.is_some_and(|dl| Instant::now() >= dl) {
                    break;
                }
                if let Some(event_time) = self.pending_mailbox_future_time(now) {
                    self.next_wake = Some(event_time);
                    break;
                }
                if let Some(dp) = self.try_defer_uart_status_poll(now) {
                    self.deferred_poll = Some(dp);
                    self.next_wake = Some(dp.max_until);
                    return Vec::new();
                }
                if self.consume_mailbox(now) {
                    // Return to the circuit solver immediately after a pin change
                    // so edge-sensitive peripherals see every transition.
                    self.next_wake = Some(now.saturating_add(1));
                    break;
                }
                if let Some(pd) = self.next_periph_due()
                    && self.guest_circuit_time(now).is_some_and(|g| g >= pd)
                {
                    break;
                }
            }
            if !self.is_process_running() {
                crate::logging::log_sim_err("[QEMU] Emulator process exited unexpectedly");
                self.stop_live();
            }
            return Vec::new();
        }

        self.next_wake = None;
        loop {
            if self.load_simu_action() != 0 {
                if let Some(event_time) = self.pending_mailbox_future_time(now) {
                    self.park_at_guest(now, Some(event_time));
                    return Vec::new();
                }
                // The first post is always taken. Further posts stop when the
                // frame budget is gone so a hot guest cannot hold the UI thread.
                // The slot stays posted; the next frame consumes it.
                if hot && deadline.is_some_and(|dl| Instant::now() >= dl) {
                    self.yield_behind(now);
                    return Vec::new();
                }
                if let Some(dp) = self.try_defer_uart_status_poll(now) {
                    self.deferred_poll = Some(dp);
                    self.next_wake = Some(dp.max_until);
                    return Vec::new();
                }
                let pin_changed = self.consume_mailbox(now);
                hot = true;
                if pin_changed {
                    // Yield to the circuit solver after any pin-changing MMIO
                    // write so rapid GPIO pulses (e.g. HD44780 En strobes) are
                    // solved individually rather than coalesced inside one wake.
                    self.next_wake = Some(now.saturating_add(1));
                    return Vec::new();
                }
                // If an active peripheral bit edge is pending and the guest has
                // already reached or passed it (e.g. polling UART_STATUS in
                // Serial.flush()), yield back to the circuit solver so the bit
                // edge can fire instead of spinning until frame timeout.
                if let Some(pd) = self.next_periph_due()
                    && pd > now
                {
                    let guest = self.guest_circuit_time(now);
                    if guest.is_some_and(|g| g >= pd) {
                        self.park_at_guest(now, Some(pd));
                        return Vec::new();
                    }
                }
                continue;
            }
            if !self.is_process_running() {
                crate::logging::log_sim_err("[QEMU] Emulator process exited unexpectedly");
                self.stop_live();
                return Vec::new();
            }
            let ahead_limit = self
                .next_periph_due()
                .map_or(now.saturating_add(GUEST_AHEAD_PS), |pd| {
                    now.saturating_add(GUEST_AHEAD_PS).min(pd)
                });
            let guest = self.guest_circuit_time(now);
            // Re-check mailbox after sampling `qemu_time`: a fire-and-forget
            // `SIM_WRITE` may have posted an earlier `simu_time` while `qemu_time`
            // kept advancing, and we must not park past that write.
            if self.load_simu_action() != 0 {
                continue;
            }
            if guest.is_some_and(|g| g >= ahead_limit && g > now) {
                self.park_at_guest(now, guest);
                return Vec::new();
            }
            if deadline.is_none() {
                self.next_wake = Some(now.saturating_add(HEADLESS_PARK_PS));
                self.catch_up_peripherals(now);
                return Vec::new();
            }
            if deadline.is_some_and(|dl| Instant::now() >= dl) {
                self.yield_behind(now);
                return Vec::new();
            }
            // QEMU updates `qemuTime` every 100 µs without signalling `req_event`,
            // and on Windows `WaitForSingleObject(..., 1)` sleeps for a full
            // ~15.6 ms timer tick on timeout. Briefly poll + yield first so fast
            // MMIO replies and non-MMIO `qemuTime` advances don't stall a frame.
            let mut poll_end = Instant::now() + Duration::from_micros(250);
            if let Some(dl) = deadline {
                poll_end = poll_end.min(dl);
            }
            let mut spins = 0u32;
            let mut got_action = false;
            loop {
                if self.load_simu_action() != 0 {
                    got_action = true;
                    break;
                }
                let guest = self.guest_circuit_time(now);
                if self.load_simu_action() != 0 {
                    got_action = true;
                    break;
                }
                if guest.is_some_and(|g| g >= ahead_limit && g > now) {
                    self.park_at_guest(now, guest);
                    return Vec::new();
                }
                if spins < 64 {
                    hint::spin_loop();
                } else {
                    if Instant::now() >= poll_end {
                        break;
                    }
                    thread::yield_now();
                }
                spins += 1;
            }
            if got_action {
                continue;
            }
            // One millisecond. An old guest never signals the request event,
            // and the mailbox flag is rechecked when this returns.
            let _ = self.wait_request(1);
            if self.load_simu_action() != 0 {
                continue;
            }
            let guest = self.guest_circuit_time(now);
            if self.load_simu_action() != 0 {
                continue;
            }
            if guest.is_some_and(|g| g >= ahead_limit && g > now) {
                self.park_at_guest(now, guest);
                return Vec::new();
            }
            if !hot {
                self.yield_behind(now);
                return Vec::new();
            }
            if deadline.is_some_and(|dl| Instant::now() >= dl) {
                self.yield_behind(now);
                return Vec::new();
            }
        }
    }

    /// If the mailbox holds a posted action whose guest timestamp
    /// (`simu_time + time_offset`) is still in the future relative to `now`,
    /// return that absolute circuit time so the solver advances to it before
    /// applying the MMIO access.
    fn pending_mailbox_future_time(&self, now: u64) -> Option<u64> {
        let view = self.live.as_ref()?.shm.load_mailbox();
        if view.simu_action == 0 || view.simu_time == 0 {
            return None;
        }
        let event_time = view.simu_time.saturating_add(self.time_offset);
        (event_time > now).then_some(event_time)
    }

    fn find_busy_usart_status(&self, addr: u64) -> Option<usize> {
        self.usarts.iter().position(|u| {
            addr >= u.mem_start && u.is_status_reg_offset(addr - u.mem_start) && u.is_tx_busy()
        })
    }

    /// Detect a consecutive busy-wait `SIM_READ` on a UART status register
    /// while TX is actively shifting bits out. Holding the response until the
    /// status changes lets the circuit queue step every TX/RX bit edge without
    /// hundreds of cross-process IPC round-trips per byte.
    fn try_defer_uart_status_poll(&mut self, now: u64) -> Option<DeferredPoll> {
        let view = self.live.as_ref()?.shm.load_mailbox();
        if view.simu_action != SimAction::Read as u64 {
            return None;
        }
        if self.last_mailbox_read_addr != Some(view.reg_addr) {
            return None;
        }
        let usart_idx = self.find_busy_usart_status(view.reg_addr)?;
        self.observe_guest_raw(view.qemu_time, now);
        Some(DeferredPoll {
            usart_idx,
            status: self.usarts[usart_idx].status_snapshot(),
            irq: (self.arena.irq_number, self.arena.irq_level),
            max_until: now.saturating_add(DEFER_POLL_MAX_PS),
        })
    }

    fn deferred_poll_ready(&self, dp: DeferredPoll, now: u64) -> bool {
        if now >= dp.max_until {
            return true;
        }
        if (self.arena.irq_number, self.arena.irq_level) != dp.irq {
            return true;
        }
        self.usarts
            .get(dp.usart_idx)
            .is_none_or(|u| u.status_snapshot() != dp.status || !u.is_tx_busy())
    }

    fn complete_deferred_poll(&mut self, now: u64) {
        self.deferred_poll = None;
        self.next_wake = None;
        if let Some(raw) = self.last_guest_raw {
            let guest = raw.saturating_add(self.time_offset);
            if now > guest {
                self.time_offset = self.time_offset.saturating_add(now - guest);
            }
        }
        let _ = self.consume_mailbox(now);
    }

    /// Apply the posted mailbox slot immediately. Reads stay blocked in the
    /// guest until `push_live` publishes `qemu_action`; writes are applied
    /// before the single pending-event slot can be overwritten.
    ///
    /// Returns `true` if this mailbox action dirtied any device pin state.
    fn consume_mailbox(&mut self, now: u64) -> bool {
        let prev_dirty = self.pins_dirty;
        self.pins_dirty = false;
        self.pull_live();
        self.observe_guest_raw(self.arena.qemu_time, now);
        let action = SimAction::from_u64(self.arena.simu_action);
        self.last_mailbox_read_addr = if action == SimAction::Read {
            Some(self.arena.reg_addr)
        } else {
            None
        };
        self.arena.qemu_action = 0;
        if action == SimAction::Freq {
            self.updt_frequency();
        } else if action != SimAction::Event {
            self.do_action();
            self.run_module_event(now);
        }
        self.arena.simu_action = 0;
        self.arena.simu_time = 0;
        self.push_live();
        let changed = self.pins_dirty;
        self.pins_dirty |= prev_dirty;
        changed
    }

    fn yield_behind(&mut self, now: u64) {
        self.catch_up_peripherals(now);
        self.yield_frame = true;
        // The next GUI tick must find this device due. Circuit time stays.
        self.next_wake = Some(now.saturating_add(1));
    }

    /// Guest clock is ahead of `now`. Leave future peripheral bit edges on the
    /// circuit event queue (via `next_event_delay`) so each pin transition is
    /// solved and visible to connected components and instruments.
    fn park_at_guest(&mut self, now: u64, guest: Option<u64>) {
        let guest_at = guest.unwrap_or(now);
        self.catch_up_peripherals(now);
        self.yield_frame = false;
        if guest_at > now {
            self.next_wake = Some(guest_at);
        } else {
            self.next_wake = Some(now.saturating_add(1));
        }
    }

    fn guest_circuit_time(&mut self, circuit_now: u64) -> Option<u64> {
        let raw = self.live.as_ref().map(|live| live.shm.load_qemu_time());
        if let Some(raw) = raw {
            self.observe_guest_raw(raw, circuit_now);
        }
        self.last_guest_raw
            .map(|raw| raw.saturating_add(self.time_offset))
    }

    fn observe_guest_raw(&mut self, raw: u64, circuit_now: u64) {
        if raw == 0 {
            return;
        }
        if let Some(prev) = self.last_guest_raw {
            if raw < prev {
                return;
            }
            if raw.saturating_sub(prev) > GUEST_JUMP_LIMIT_PS {
                if self.pending_guest_raw == Some(raw) {
                    self.last_guest_raw = Some(raw);
                    self.pending_guest_raw = None;
                } else {
                    self.pending_guest_raw = Some(raw);
                }
                return;
            }
        } else {
            let circuit = raw.saturating_add(self.time_offset);
            if circuit > circuit_now.saturating_add(GUEST_JUMP_LIMIT_PS) {
                if self.pending_guest_raw == Some(raw) {
                    self.last_guest_raw = Some(raw);
                    self.pending_guest_raw = None;
                } else {
                    self.pending_guest_raw = Some(raw);
                }
                return;
            }
        }
        self.pending_guest_raw = None;
        self.last_guest_raw = Some(raw);
    }

    /// C++ `QemuDevice::voltChanged` on the reset pin: low kills the child.
    pub fn check_reset(&mut self) {
        self.check_reset_at(self.time_offset);
    }

    pub fn check_reset_at(&mut self, circuit_now: u64) {
        let Some(i) = self.rst_pin else {
            return;
        };
        let reset = !self.pins[i].last_inp_state();
        if reset {
            self.stop_live();
        } else if !self.is_live() && !self.firmware.is_empty() && self.last_error.is_none() {
            self.time_offset = circuit_now;
            self.last_guest_raw = None;
            self.pending_guest_raw = None;
            self.last_mailbox_read_addr = None;
            self.deferred_poll = None;
            self.esp32_gpio.state = 0;
            self.esp32_gpio.enable = 0;
            for u in &mut self.usarts {
                u.reset();
            }
            for s in &mut self.spis {
                s.reset();
            }
            for t in &mut self.twis {
                t.reset();
            }
            for (idx, pin) in self.pins.iter_mut().enumerate() {
                if idx != i {
                    pin.set_pin_mode(PinMode::Input);
                    pin.set_out_state(false);
                }
            }
            if self.family == QemuFamily::Esp32
                && let Some(u) = self.usarts.first_mut()
            {
                let tx = u.tx;
                u.module.enable_tx(true, &mut self.pins, tx);
                u.module.enable_rx(true);
            }
            self.pins_dirty = true;
            if let Err(e) = self.start_live() {
                crate::logging::log_sim_err(format!("[QEMU] {e}"));
                self.last_error = Some(e);
            }
        }
    }

    fn updt_frequency(&mut self) {
        match self.family {
            QemuFamily::Esp32 => {
                self.cpu_freq = self.arena.reg_data as u32;
                self.apb_freq = self.arena.reg_addr as u32;
            }
            QemuFamily::Stm32 => {
                self.apb_freq = self.arena.reg_data as u32;
            }
        }
    }

    fn do_action(&mut self) {
        let address = self.arena.reg_addr;
        if let Some(i) = self.last_module {
            let m = self.modules[i];
            if address >= m.start && address <= m.end && !matches!(m.kind, ModuleKind::Dummy) {
                self.event = Some(PendingEvent {
                    kind: m.kind,
                    address,
                    value: self.arena.reg_data,
                    action: SimAction::from_u64(self.arena.simu_action),
                });
                return;
            }
        }
        for (i, m) in self.modules.iter().enumerate() {
            if address < m.start || address > m.end {
                continue;
            }
            if matches!(m.kind, ModuleKind::Dummy) {
                continue;
            }
            self.last_module = Some(i);
            self.event = Some(PendingEvent {
                kind: m.kind,
                address,
                value: self.arena.reg_data,
                action: SimAction::from_u64(self.arena.simu_action),
            });
            return;
        }
        self.last_module = None;
        self.event = None;
        let action = SimAction::from_u64(self.arena.simu_action);
        if action == SimAction::Read {
            let val = self.read_mem(address);
            self.arena.reg_data = val as u64;
            self.arena.qemu_action = SimAction::Read as u64;
        } else if action == SimAction::Write {
            self.write_mem(address, self.arena.reg_data as u32);
        }
    }

    fn run_module_event(&mut self, now: u64) {
        let Some(ev) = self.event.take() else {
            return;
        };
        match ev.action {
            SimAction::Read => self.read_register(ev),
            SimAction::Write => self.write_register(ev, now),
            _ => {}
        }
    }

    fn read_register(&mut self, ev: PendingEvent) {
        let val = match ev.kind {
            ModuleKind::Esp32Gpio => self.esp32_read(ev.address),
            ModuleKind::Esp32IoMux => self
                .esp32_iomux
                .as_ref()
                .map(|m| m.read(ev.address.saturating_sub(m.mem_start)))
                .unwrap_or(0),
            ModuleKind::Stm32Port(n) => self.stm32_read(n as usize, ev.address),
            ModuleKind::Stm32Afio => self
                .stm32_afio
                .as_ref()
                .map(|m| m.read(ev.address.saturating_sub(m.mem_start)))
                .unwrap_or(0),
            ModuleKind::Esp32Usart(n) | ModuleKind::Stm32Usart(n) => self.usart_read(n, ev.address),
            ModuleKind::Esp32Spi(n) | ModuleKind::Stm32Spi(n) => self.spi_read(n, ev.address),
            ModuleKind::Esp32Twi(n) | ModuleKind::Stm32Twi(n) => self.twi_read(n, ev.address),
            ModuleKind::Stm32Timer(n) => self.timer_read(n, ev.address),
            ModuleKind::Dummy => self.read_mem(ev.address),
        };
        self.arena.reg_data = val as u64;
        self.arena.qemu_action = SimAction::Read as u64;
    }

    fn write_register(&mut self, ev: PendingEvent, now: u64) {
        match ev.kind {
            ModuleKind::Esp32Gpio => self.esp32_write(ev.address, ev.value as u32),
            ModuleKind::Esp32IoMux => self.esp32_iomux_write(ev.address, ev.value as u32),
            ModuleKind::Stm32Port(n) => self.stm32_write(n as usize, ev.address, ev.value as u32),
            ModuleKind::Stm32Afio => self.stm32_afio_write(ev.address, ev.value as u32),
            ModuleKind::Esp32Usart(n) | ModuleKind::Stm32Usart(n) => {
                self.usart_write(n, ev.address, ev.value as u32, now);
            }
            ModuleKind::Esp32Spi(n) | ModuleKind::Stm32Spi(n) => {
                self.spi_write(n, ev.address, ev.value as u32, now);
            }
            ModuleKind::Esp32Twi(n) | ModuleKind::Stm32Twi(n) => {
                self.twi_write(n, ev.address, ev.value as u32, now);
            }
            ModuleKind::Stm32Timer(n) => self.timer_write(n, ev.address, ev.value as u32),
            ModuleKind::Dummy => self.write_mem(ev.address, ev.value as u32),
        }
    }

    fn stm32_afio_write(&mut self, address: u64, value: u32) {
        let Some(ref mut afio) = self.stm32_afio else {
            return;
        };
        let offset = address.saturating_sub(afio.mem_start);
        afio.write(offset, value);

        let pin_at = |ports: &[Stm32Port], port: usize, bit: usize| {
            ports.get(port).and_then(|p| p.pins[bit])
        };

        // Remap USART1
        if let Some(u) = self.usarts.first_mut() {
            if afio.usart1_remap() {
                u.tx = pin_at(&self.stm32_ports, 1, 6); // PB6
                u.rx = pin_at(&self.stm32_ports, 1, 7); // PB7
            } else {
                u.tx = pin_at(&self.stm32_ports, 0, 9); // PA9
                u.rx = pin_at(&self.stm32_ports, 0, 10); // PA10
            }
        }
        // Remap USART2
        if let Some(u) = self.usarts.get_mut(1) {
            if afio.usart2_remap() {
                u.tx = pin_at(&self.stm32_ports, 3, 5); // PD5
                u.rx = pin_at(&self.stm32_ports, 3, 6); // PD6
            } else {
                u.tx = pin_at(&self.stm32_ports, 0, 2); // PA2
                u.rx = pin_at(&self.stm32_ports, 0, 3); // PA3
            }
        }
        // Remap USART3
        if let Some(u) = self.usarts.get_mut(2) {
            match afio.usart3_remap() {
                1 => {
                    u.tx = pin_at(&self.stm32_ports, 2, 10); // PC10
                    u.rx = pin_at(&self.stm32_ports, 2, 11); // PC11
                }
                3 => {
                    u.tx = pin_at(&self.stm32_ports, 3, 8); // PD8
                    u.rx = pin_at(&self.stm32_ports, 3, 9); // PD9
                }
                _ => {
                    u.tx = pin_at(&self.stm32_ports, 1, 10); // PB10
                    u.rx = pin_at(&self.stm32_ports, 1, 11); // PB11
                }
            }
        }
        // Remap SPI1
        if let Some(s) = self.spis.first_mut() {
            if afio.spi1_remap() {
                s.pins = SpiPins {
                    mosi: pin_at(&self.stm32_ports, 1, 5), // PB5
                    miso: pin_at(&self.stm32_ports, 1, 4), // PB4
                    clk: pin_at(&self.stm32_ports, 1, 3),  // PB3
                    ss: pin_at(&self.stm32_ports, 0, 15),  // PA15
                };
            } else {
                s.pins = SpiPins {
                    mosi: pin_at(&self.stm32_ports, 0, 7), // PA7
                    miso: pin_at(&self.stm32_ports, 0, 6), // PA6
                    clk: pin_at(&self.stm32_ports, 0, 5),  // PA5
                    ss: pin_at(&self.stm32_ports, 0, 4),   // PA4
                };
            }
        }
        // Remap I2C1
        if let Some(t) = self.twis.first_mut() {
            if afio.i2c1_remap() {
                t.pins = TwiPins {
                    scl: pin_at(&self.stm32_ports, 1, 8), // PB8
                    sda: pin_at(&self.stm32_ports, 1, 9), // PB9
                };
            } else {
                t.pins = TwiPins {
                    scl: pin_at(&self.stm32_ports, 1, 6), // PB6
                    sda: pin_at(&self.stm32_ports, 1, 7), // PB7
                };
            }
        }
    }

    fn usart_read(&mut self, n: u8, address: u64) -> u32 {
        let (val, irq) = {
            let Some(u) = self.usarts.get_mut(n as usize) else {
                return 0;
            };
            let offset = address.saturating_sub(u.mem_start);
            u.read(offset)
        };
        // Publish with the read data. Signaling here would ack the guest
        // before `reg_data` is stored.
        if let Some((irq, level)) = irq {
            self.note_interrupt(irq, level);
        }
        val.unwrap_or_else(|| self.read_mem(address))
    }

    fn usart_write(&mut self, n: u8, address: u64, value: u32, now: u64) {
        let apb = self.apb_freq;
        let Some(u) = self.usarts.get_mut(n as usize) else {
            return;
        };
        let offset = address.saturating_sub(u.mem_start);
        if let Some((irq, level)) = u.write(offset, value, now, &mut self.pins, apb) {
            self.set_interrupt(irq, level);
        }
        self.pins_dirty = true;
        self.write_mem(address, value);
    }

    fn spi_read(&mut self, n: u8, address: u64) -> u32 {
        let Some(s) = self.spis.get_mut(n as usize) else {
            return 0;
        };
        let offset = address.saturating_sub(s.mem_start);
        match s.family {
            periph::SpiFamily::Esp32 => {
                if offset == 0x80 {
                    u32::from(s.module.data_reg)
                } else {
                    self.read_mem(address)
                }
            }
            periph::SpiFamily::Stm32 => s.read(offset),
        }
    }

    fn spi_write(&mut self, n: u8, address: u64, value: u32, now: u64) {
        let ps = self.arena.ps_per_inst;
        let Some(s) = self.spis.get_mut(n as usize) else {
            return;
        };
        let offset = address.saturating_sub(s.mem_start);
        s.write(offset, value, now, &mut self.pins, ps);
        self.pins_dirty = true;
        self.write_mem(address, value);
    }

    fn twi_read(&mut self, n: u8, address: u64) -> u32 {
        let Some(t) = self.twis.get_mut(n as usize) else {
            return 0;
        };
        let offset = address.saturating_sub(t.mem_start);
        match t.family {
            periph::TwiFamily::Esp32 => match offset {
                0x00 | 0x04 | 0x08 | 0x18 | 0x20 | 0x28 | 0x2C | 0x38 => t.read(offset),
                0x58..=0x97 => t.read(offset),
                0x1C..=0x1F | 0x100..=0x1FF => t.read(offset),
                _ => self.read_mem(address),
            },
            periph::TwiFamily::Stm32 => t.read(offset),
        }
    }

    fn twi_write(&mut self, n: u8, address: u64, value: u32, now: u64) {
        let Some(t) = self.twis.get_mut(n as usize) else {
            return;
        };
        let offset = address.saturating_sub(t.mem_start);
        if let Some((irq, level)) = t.write(offset, value, now, &mut self.pins) {
            self.set_interrupt(irq, level);
        }
        self.pins_dirty = true;
        self.write_mem(address, value);
    }

    fn timer_read(&mut self, n: u8, address: u64) -> u32 {
        let Some(t) = self.timers.get(n as usize) else {
            return 0;
        };
        let offset = address.saturating_sub(t.mem_start);
        t.read(offset)
    }

    fn timer_write(&mut self, n: u8, address: u64, value: u32) {
        let Some(t) = self.timers.get_mut(n as usize) else {
            return;
        };
        let offset = address.saturating_sub(t.mem_start);
        t.write(offset, value, &mut self.pins);
        self.pins_dirty = true;
        self.write_mem(address, value);
    }

    fn note_interrupt(&mut self, number: u8, level: u8) {
        self.arena.irq_number = u64::from(number);
        self.arena.irq_level = u64::from(level);
    }

    fn set_interrupt(&mut self, number: u8, level: u8) {
        self.note_interrupt(number, level);
        self.push_live_irq();
    }

    fn tick_peripherals(&mut self, now: u64) {
        self.catch_up_peripherals(now);
    }

    /// Walk USART, SPI, and TWI edges whose deadlines fall at or before `until`.
    /// Each edge is scheduled from its own due time, so a frame of bits lands
    /// in one device wake. Host serial input is taken once, not per bit.
    fn catch_up_peripherals(&mut self, until: u64) {
        self.ingest_host_serial(until);
        self.advance_usarts(until);
        self.advance_spis(until);
        self.advance_twis(until);
    }

    fn ingest_host_serial(&mut self, now: u64) {
        if !crate::serial::has_pending_send() {
            return;
        }
        for i in 0..self.usarts.len() {
            let port_id = self.usart_port(i).to_string();
            let mut pending_rx = crate::serial::take_send(&port_id);
            if i == 0 {
                pending_rx.extend(crate::serial::take_send(&self.uid));
                pending_rx.extend(crate::serial::take_send("default"));
            }
            if pending_rx.is_empty() {
                continue;
            }
            let mut irq_to_set = None;
            for b in pending_rx {
                crate::serial::publish_in(&port_id, b);
                if i == 0 {
                    crate::serial::publish_in(&self.uid, b);
                    crate::serial::publish_in("default", b);
                }
                if let Some((irq, level)) = self.usarts[i].on_byte_received_at(b, now) {
                    irq_to_set = Some((irq, level));
                }
            }
            if let Some((irq, level)) = irq_to_set {
                self.set_interrupt(irq, level);
            }
        }
    }

    fn advance_usarts(&mut self, until: u64) {
        for i in 0..self.usarts.len() {
            let tx = self.usarts[i].tx;
            let rx = self.usarts[i].rx;
            let sample = self.usarts[i]
                .next_due()
                .filter(|t| *t <= until)
                .unwrap_or(until);
            let _ = self.usarts[i].module.rx_pin_changed(sample, &self.pins, rx);
            let mut n = 0u32;
            loop {
                if n >= USART_CATCH_CAP {
                    break;
                }
                let Some(due) = self.usarts[i].next_due() else {
                    break;
                };
                if due > until {
                    break;
                }
                let tick = self.usarts[i].module.tick(due, &mut self.pins, tx, rx);
                n += 1;
                if tick.frame_sent.is_some()
                    || self.usarts[i].module.tx_due.is_some()
                    || self.usarts[i].module.sending()
                {
                    self.pins_dirty = true;
                }
                let mut irq_to_set = None;
                if let Some(b) = tick.frame_sent {
                    self.publish_usart_out(i, b);
                    // The next FIFO byte starts at this bit's due time. Passing
                    // `until` would collapse the rest of the frame into one step.
                    if let Some((irq, level)) = self.usarts[i].on_frame_sent(due, &mut self.pins) {
                        irq_to_set = Some((irq, level));
                    }
                    self.pins_dirty = true;
                }
                if let Some((irq, level)) = irq_to_set.take() {
                    self.set_interrupt(irq, level);
                }
                if let Some(b) = tick.byte_received {
                    self.publish_usart_in(i, b);
                    if let Some((irq, level)) = self.usarts[i].on_byte_received_at(b, due) {
                        irq_to_set = Some((irq, level));
                    }
                }
                if let Some((irq, level)) = irq_to_set.take() {
                    self.set_interrupt(irq, level);
                }
                if let Some((irq, level)) = self.usarts[i].tick_rx_timeout(due) {
                    self.set_interrupt(irq, level);
                }
                if self.usarts[i].next_due().is_some_and(|next| next <= due) {
                    break;
                }
            }
        }
    }

    fn advance_spis(&mut self, until: u64) {
        for i in 0..self.spis.len() {
            let mut n = 0u32;
            loop {
                if n >= EDGE_CATCH_CAP {
                    break;
                }
                let Some(due) = self.spis[i].module.due else {
                    break;
                };
                if due > until {
                    break;
                }
                let map = self.spis[i].pins;
                let finished = self.spis[i].module.run_event(due, &mut self.pins, map);
                n += 1;
                self.pins_dirty = true;
                if finished {
                    let port_id = self
                        .cached_spi_port_ids
                        .get(i)
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let tx_b = self.spis[i].module.tx_reg;
                    let rx_b = self.spis[i].module.data_reg;
                    crate::serial::publish_out(port_id, tx_b);
                    crate::serial::publish_in(port_id, rx_b);
                    self.spis[i].on_end();
                }
                if self.spis[i].module.due.is_some_and(|next| next <= due) {
                    break;
                }
            }
        }
    }

    fn advance_twis(&mut self, until: u64) {
        for i in 0..self.twis.len() {
            let mut n = 0u32;
            loop {
                if n >= EDGE_CATCH_CAP {
                    break;
                }
                let Some(due) = self.twis[i].module.due else {
                    break;
                };
                if due > until {
                    break;
                }
                let map = self.twis[i].pins;
                let tick = self.twis[i].module.run_event(due, &mut self.pins, map);
                n += 1;
                self.pins_dirty = true;
                let port_id = self
                    .cached_twi_port_ids
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(b) = tick.byte_sent {
                    crate::serial::publish_out(&port_id, b);
                }
                if let Some(b) = tick.byte_received {
                    crate::serial::publish_in(&port_id, b);
                }
                if let Some(st) = tick.state {
                    self.twis[i].on_state(st, due, &mut self.pins);
                }
                if self.twis[i].module.due.is_some_and(|next| next <= due) {
                    break;
                }
            }
        }
    }

    fn usart_port(&self, i: usize) -> &str {
        self.cached_usart_port_ids
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    fn publish_usart_out(&self, i: usize, byte: u8) {
        let port_id = self.usart_port(i);
        crate::serial::publish_out(port_id, byte);
        if i == 0 {
            crate::serial::publish_out(&self.uid, byte);
            crate::serial::publish_out("default", byte);
        }
    }

    fn publish_usart_in(&self, i: usize, byte: u8) {
        let port_id = self.usart_port(i);
        crate::serial::publish_in(port_id, byte);
        if i == 0 {
            crate::serial::publish_in(&self.uid, byte);
            crate::serial::publish_in("default", byte);
        }
    }

    fn esp32_iomux_write(&mut self, address: u64, value: u32) {
        let Some(ref mut m) = self.esp32_iomux else {
            return;
        };
        let offset = address.saturating_sub(m.mem_start);
        m.write(offset, value);
        if let Some(gpio) = esp32_iomux_offset_to_gpio(offset)
            && gpio < 34
            && let Some(idx) = self.esp32_gpio.pad[gpio]
        {
            let wpu = (value & ESP32_IOMUX_FUN_WPU) != 0;
            self.pins[idx].set_pullup(if wpu { 1e5 } else { 0.0 });
            self.pins_dirty = true;
        }
    }

    fn esp32_read(&mut self, address: u64) -> u32 {
        let offset = address.saturating_sub(ESP32_GPIO_START);
        match offset {
            GPIO_OUT => self.esp32_gpio.state,
            GPIO_OUT1 => self.esp32_gpio.state1,
            GPIO_ENABLE => self.esp32_gpio.enable,
            GPIO_ENABLE1 => self.esp32_gpio.enable1,
            GPIO_STRAP => 0x13,
            GPIO_IN => self.read_esp32_port(0),
            GPIO_IN1 => self.read_esp32_port(1),
            _ => self.read_mem(address),
        }
    }

    fn esp32_write(&mut self, address: u64, value: u32) {
        let offset = address.saturating_sub(ESP32_GPIO_START);
        match offset {
            GPIO_OUT => self.set_gpio_state(value),
            GPIO_OUT_W1TS => self.set_gpio_state(self.esp32_gpio.state | value),
            GPIO_OUT_W1TC => self.set_gpio_state(self.esp32_gpio.state & !value),
            GPIO_OUT1 => self.set_gpio_state1(value),
            GPIO_OUT1_W1TS => self.set_gpio_state1(self.esp32_gpio.state1 | value),
            GPIO_OUT1_W1TC => self.set_gpio_state1(self.esp32_gpio.state1 & !value),
            GPIO_ENABLE => self.set_gpio_dir(value),
            GPIO_ENABLE_W1TS => self.set_gpio_dir(self.esp32_gpio.enable | value),
            GPIO_ENABLE_W1TC => self.set_gpio_dir(self.esp32_gpio.enable & !value),
            GPIO_ENABLE1 => self.set_gpio_dir1(value),
            GPIO_ENABLE1_W1TS => self.set_gpio_dir1(self.esp32_gpio.enable1 | value),
            GPIO_ENABLE1_W1TC => self.set_gpio_dir1(self.esp32_gpio.enable1 & !value),
            off if (GPIO_FUNC_IN_SEL_CFG_BASE..GPIO_FUNC_OUT_SEL_CFG_BASE).contains(&off) => {
                let sig = ((off - GPIO_FUNC_IN_SEL_CFG_BASE) / 4) as usize;
                if let Some(ref mut m) = self.esp32_iomux {
                    m.set_func_in(sig, value);
                }
                self.apply_esp32_func_in(sig, value);
            }
            off if (GPIO_FUNC_OUT_SEL_CFG_BASE..GPIO_FUNC_OUT_SEL_CFG_BASE + 40 * 4)
                .contains(&off) =>
            {
                let gpio = ((off - GPIO_FUNC_OUT_SEL_CFG_BASE) / 4) as usize;
                if let Some(ref mut m) = self.esp32_iomux {
                    m.set_func_out(gpio, value);
                }
                self.apply_esp32_func_out(gpio, value);
            }
            _ => {}
        }
        self.write_mem(address, value);
    }

    fn apply_esp32_func_in(&mut self, sig: usize, val: u32) {
        let gpio = (val & 0x3F) as usize;
        if gpio >= 40 {
            return;
        }
        let pin = self.esp32_gpio.pad[gpio];
        match sig {
            14 => {
                if let Some(u) = self.usarts.first_mut() {
                    u.rx = pin;
                }
            }
            17 => {
                if let Some(u) = self.usarts.get_mut(1) {
                    u.rx = pin;
                }
            }
            198 => {
                if let Some(u) = self.usarts.get_mut(2) {
                    u.rx = pin;
                }
            }
            9 => {
                if let Some(s) = self.spis.first_mut() {
                    s.pins.miso = pin;
                }
            }
            64 => {
                if let Some(s) = self.spis.get_mut(1) {
                    s.pins.miso = pin;
                }
            }
            29 => {
                if let Some(t) = self.twis.first_mut() {
                    t.pins.scl = pin;
                }
            }
            30 => {
                if let Some(t) = self.twis.first_mut() {
                    t.pins.sda = pin;
                }
            }
            95 => {
                if let Some(t) = self.twis.get_mut(1) {
                    t.pins.scl = pin;
                }
            }
            96 => {
                if let Some(t) = self.twis.get_mut(1) {
                    t.pins.sda = pin;
                }
            }
            _ => {}
        }
    }

    fn apply_esp32_func_out(&mut self, gpio: usize, val: u32) {
        if gpio >= 34 {
            return;
        }
        let sig = (val & 0x1FF) as usize;
        let pin = self.esp32_gpio.pad[gpio];
        match sig {
            14 => {
                if let Some(u) = self.usarts.first_mut() {
                    u.tx = pin;
                }
            }
            17 => {
                if let Some(u) = self.usarts.get_mut(1) {
                    u.tx = pin;
                }
            }
            198 => {
                if let Some(u) = self.usarts.get_mut(2) {
                    u.tx = pin;
                }
            }
            8 => {
                if let Some(s) = self.spis.first_mut() {
                    s.pins.clk = pin;
                }
            }
            10 => {
                if let Some(s) = self.spis.first_mut() {
                    s.pins.mosi = pin;
                }
            }
            11 => {
                if let Some(s) = self.spis.first_mut() {
                    s.pins.ss = pin;
                }
            }
            63 => {
                if let Some(s) = self.spis.get_mut(1) {
                    s.pins.clk = pin;
                }
            }
            65 => {
                if let Some(s) = self.spis.get_mut(1) {
                    s.pins.mosi = pin;
                }
            }
            68 => {
                if let Some(s) = self.spis.get_mut(1) {
                    s.pins.ss = pin;
                }
            }
            29 => {
                if let Some(t) = self.twis.first_mut() {
                    t.pins.scl = pin;
                }
            }
            30 => {
                if let Some(t) = self.twis.first_mut() {
                    t.pins.sda = pin;
                }
            }
            95 => {
                if let Some(t) = self.twis.get_mut(1) {
                    t.pins.scl = pin;
                }
            }
            96 => {
                if let Some(t) = self.twis.get_mut(1) {
                    t.pins.sda = pin;
                }
            }
            _ => {}
        }
    }

    fn read_esp32_port(&self, bank: i32) -> u32 {
        let mut data = 0u32;
        if bank == 0 {
            for i in 0..32 {
                if let Some(idx) = self.esp32_gpio.pad[i] {
                    if self.pin_inp(idx) {
                        data |= 1 << i;
                    }
                }
            }
        } else {
            for i in 32..40 {
                if let Some(idx) = self.esp32_gpio.pad[i] {
                    if self.pin_inp(idx) {
                        data |= 1 << (i - 32);
                    }
                }
            }
        }
        data
    }

    fn pin_inp(&self, idx: usize) -> bool {
        self.pins[idx].last_inp_state()
    }

    fn set_gpio_state(&mut self, new_state: u32) {
        if self.esp32_gpio.state == new_state {
            return;
        }
        let mut changed = self.esp32_gpio.state ^ new_state;
        self.esp32_gpio.state = new_state;
        self.pins_dirty = true;
        while changed != 0 {
            let i = changed.trailing_zeros() as usize;
            let mask = 1u32 << i;
            if let Some(idx) = self.esp32_gpio.pad[i] {
                self.pins[idx].set_out_state(new_state & mask != 0);
            }
            changed &= !mask;
        }
    }

    fn set_gpio_dir(&mut self, new_enable: u32) {
        if self.esp32_gpio.enable == new_enable {
            return;
        }
        let mut changed = self.esp32_gpio.enable ^ new_enable;
        self.esp32_gpio.enable = new_enable;
        self.pins_dirty = true;
        while changed != 0 {
            let i = changed.trailing_zeros() as usize;
            let mask = 1u32 << i;
            if let Some(idx) = self.esp32_gpio.pad[i] {
                if new_enable & mask != 0 {
                    self.pins[idx].set_pin_mode(PinMode::Output);
                    self.pins[idx].set_out_state(self.esp32_gpio.state & mask != 0);
                } else {
                    self.pins[idx].set_pin_mode(PinMode::Input);
                }
            }
            changed &= !mask;
        }
    }

    fn set_gpio_state1(&mut self, raw_state: u32) {
        let new_state = raw_state & ESP32_BANK1_OUT_MASK;
        if self.esp32_gpio.state1 == new_state {
            return;
        }
        let mut changed = self.esp32_gpio.state1 ^ new_state;
        self.esp32_gpio.state1 = new_state;
        self.pins_dirty = true;
        while changed != 0 {
            let bit = changed.trailing_zeros() as usize;
            let mask = 1u32 << bit;
            if let Some(idx) = self.esp32_gpio.pad[32 + bit] {
                self.pins[idx].set_out_state(new_state & mask != 0);
            }
            changed &= !mask;
        }
    }

    fn set_gpio_dir1(&mut self, raw_enable: u32) {
        let new_enable = raw_enable & ESP32_BANK1_OUT_MASK;
        if self.esp32_gpio.enable1 == new_enable {
            return;
        }
        let mut changed = self.esp32_gpio.enable1 ^ new_enable;
        self.esp32_gpio.enable1 = new_enable;
        self.pins_dirty = true;
        while changed != 0 {
            let bit = changed.trailing_zeros() as usize;
            let mask = 1u32 << bit;
            if let Some(idx) = self.esp32_gpio.pad[32 + bit] {
                if new_enable & mask != 0 {
                    self.pins[idx].set_pin_mode(PinMode::Output);
                    self.pins[idx].set_out_state(self.esp32_gpio.state1 & mask != 0);
                } else {
                    self.pins[idx].set_pin_mode(PinMode::Input);
                }
            }
            changed &= !mask;
        }
    }

    #[cfg(test)]
    pub(crate) fn esp32_write_for_test(&mut self, address: u64, value: u32) {
        self.esp32_write(address, value);
    }

    #[cfg(test)]
    pub(crate) fn esp32_read_for_test(&mut self, address: u64) -> u32 {
        self.esp32_read(address)
    }

    #[cfg(test)]
    pub(crate) fn esp32_iomux_write_for_test(&mut self, address: u64, value: u32) {
        self.esp32_iomux_write(address, value);
    }

    #[cfg(test)]
    pub(crate) fn pin_by_suffix(&self, suffix: &str) -> Option<&IoPin> {
        let want = format!("{}-{suffix}", self.uid);
        self.pins.iter().find(|p| p.id == want)
    }

    #[cfg(test)]
    pub(crate) fn set_pin_input_for_test(&mut self, suffix: &str, volt: f64) {
        let want = format!("{}-{suffix}", self.uid);
        if let Some(p) = self.pins.iter_mut().find(|p| p.id == want) {
            p.get_inp_state(volt);
        }
    }

    #[cfg(test)]
    pub(crate) fn usart_tx_pin_name(&self, idx: usize) -> Option<String> {
        let pin_idx = self.usarts.get(idx)?.tx?;
        self.pins.get(pin_idx).map(|p| p.id.clone())
    }

    fn stm32_read(&mut self, port: usize, address: u64) -> u32 {
        let Some(p) = self.stm32_ports.get(port) else {
            return 0;
        };
        let offset = address.saturating_sub(p.mem_start);
        match offset {
            IDR_OFFSET => self.stm32_read_port(port),
            BSRR_OFFSET | BRR_OFFSET => 0,
            _ => self.read_mem(address),
        }
    }

    fn stm32_write(&mut self, port: usize, address: u64, value: u32) {
        let Some(p) = self.stm32_ports.get(port) else {
            return;
        };
        let start = p.mem_start;
        let offset = address.saturating_sub(start);
        match offset {
            CRL_OFFSET => {
                if value == self.read_mem(address) {
                    return;
                }
                self.write_mem(address, value);
                self.config_port(port, value, 0);
            }
            CRH_OFFSET => {
                if value == self.read_mem(address) {
                    return;
                }
                self.write_mem(address, value);
                self.config_port(port, value, 8);
            }
            IDR_OFFSET => {}
            ODR_OFFSET => {
                if value == self.read_mem(address) {
                    return;
                }
                self.write_mem(address, value);
                self.set_port_state(port, value as u16);
            }
            BSRR_OFFSET => {
                let set_mask = value & 0x0000_FFFF;
                let reset_mask = !(value >> 16) & 0x0000_FFFF;
                let odr = self.read_mem(start + ODR_OFFSET);
                let new = (odr & reset_mask) | set_mask;
                if new == odr {
                    return;
                }
                self.write_mem(start + ODR_OFFSET, new);
                self.set_port_state(port, new as u16);
            }
            BRR_OFFSET => {
                let reset_mask = !value & 0x0000_FFFF;
                let odr = self.read_mem(start + ODR_OFFSET);
                let new = odr & reset_mask;
                if new == odr {
                    return;
                }
                self.write_mem(start + ODR_OFFSET, new);
                self.set_port_state(port, new as u16);
            }
            LCKR_OFFSET => self.write_mem(address, value),
            _ => self.write_mem(address, value),
        }
    }

    fn stm32_read_port(&self, port: usize) -> u32 {
        let Some(p) = self.stm32_ports.get(port) else {
            return 0;
        };
        let mut data = 0u32;
        for i in 0..16 {
            if let Some(idx) = p.pins[i] {
                if self.pin_inp(idx) {
                    data |= 1 << i;
                }
            }
        }
        data
    }

    fn set_port_state(&mut self, port: usize, state: u16) {
        let Some(p) = self.stm32_ports.get(port) else {
            return;
        };
        let pins = p.pins;
        if self.stm32_ports[port].pin_state == state {
            return;
        }
        self.stm32_ports[port].pin_state = state;
        self.pins_dirty = true;
        for i in 0..16 {
            if let Some(idx) = pins[i] {
                self.pins[idx].set_out_state(state & (1 << i) != 0);
            }
        }
    }

    fn config_port(&mut self, port: usize, config: u32, shift: u8) {
        let Some(p) = self.stm32_ports.get(port) else {
            return;
        };
        let pins = p.pins;
        self.pins_dirty = true;
        for i in shift..(shift + 8) {
            let Some(idx) = pins[i as usize] else {
                continue;
            };
            let cfg_shift = (i - shift) * 4;
            let cfg_bits = (config >> cfg_shift) & 0b1111;
            let is_output = cfg_bits & 0b0011;
            if is_output != 0 {
                let open = cfg_bits & 0b0100;
                self.pins[idx].set_pin_mode(if open != 0 {
                    PinMode::OpenCo
                } else {
                    PinMode::Output
                });
            } else {
                let pull = cfg_bits & 0b1000;
                self.pins[idx].set_pin_mode(PinMode::Input);
                self.pins[idx].set_pullup(if pull != 0 { 1e5 } else { 0.0 });
            }
        }
    }

    fn read_mem(&self, address: u64) -> u32 {
        let i = address as usize;
        self.io_mem.get(i).copied().unwrap_or(0)
    }

    fn write_mem(&mut self, address: u64, value: u32) {
        let i = address as usize;
        if i < self.io_mem.len() {
            self.io_mem[i] = value;
        }
    }

    /// C++ `Esp32::createArgs` argv (without the executable).
    pub fn esp32_args(
        &self,
        firm_path: &str,
        shm_key: &str,
        rom_dir: &str,
        efuse: &str,
    ) -> Vec<String> {
        let mut args = Vec::new();
        for arg in self.extra_args.split(',') {
            if !arg.is_empty() {
                args.push(arg.to_string());
            }
        }
        if !has_display_arg(&args) {
            args.push("-display".into());
            args.push("none".into());
        }
        args.push("-M".into());
        args.push(format!("esp32-cs,cosim-shm={shm_key}"));
        args.push("-L".into());
        args.push(rom_dir.into());
        let firmware = firm_path
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(firm_path);
        args.push("-drive".into());
        args.push(format!("file={firmware}.bin,if=mtd,format=raw"));
        args.push("-drive".into());
        args.push(format!("file={efuse},if=none,format=raw,id=efuse"));
        args.push("-global".into());
        args.push("driver=nvram.esp32.efuse,property=drive,value=efuse".into());
        args.push("-global".into());
        args.push("driver=timer.esp32.timg,property=wdt_disable,value=true".into());
        args.push("-icount".into());
        args.push("shift=4,align=off,sleep=off".into());
        if self.gdb_port != 0 {
            args.push("-gdb".into());
            args.push(format!("tcp::{}", self.gdb_port));
            args.push("-S".into());
        }
        args
    }

    /// C++ `Stm32::createArgs` argv (without the executable).
    pub fn stm32_args(&self, firmware: &str, shm_key: &str) -> Vec<String> {
        let mut args = Vec::new();
        args.push(shm_key.into());
        args.push("qemu-system-arm".into());
        for arg in self.extra_args.split(',') {
            if !arg.is_empty() {
                args.push(arg.to_string());
            }
        }
        if !has_display_arg(&args) {
            args.push("-display".into());
            args.push("none".into());
        }
        args.push("-M".into());
        args.push("stm32-f10xx".into());
        args.push("-drive".into());
        args.push(format!("file={firmware},if=pflash,format=raw"));
        args.push("-icount".into());
        args.push("shift=0,align=off,sleep=off".into());
        if self.gdb_port != 0 {
            args.push("-gdb".into());
            args.push(format!("tcp::{}", self.gdb_port));
            args.push("-S".into());
        }
        args
    }
}

fn pick_free_loopback_port() -> Option<u16> {
    TcpListener::bind(("127.0.0.1", 0))
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
}

fn load_companion_elf_if_needed(firmware: &Path) {
    let session = DebugSession::global();
    if !session.symbols().is_empty() {
        return;
    }
    let mut candidates = vec![
        firmware.with_extension("elf"),
        firmware.with_extension("ino.elf"),
    ];
    let s = firmware.to_string_lossy();
    if let Some(prefix) = s.strip_suffix(".merged.bin") {
        candidates.push(PathBuf::from(format!("{prefix}.elf")));
    }
    for cand in candidates {
        if !cand.is_file() {
            continue;
        }
        if let Ok(bytes) = fs::read(&cand) {
            if let Some(syms) = ElfParser::parse(&bytes) {
                crate::logging::log_sim(format!(
                    "[GDB] Loaded debug symbols from {}",
                    cand.display()
                ));
                session.load_symbols(syms);
                return;
            }
        }
    }
}

fn has_display_arg(args: &[String]) -> bool {
    args.iter().any(|a| {
        matches!(a.as_str(), "-display" | "-nographic" | "-vnc")
            || a.starts_with("-display=")
            || a.starts_with("-vnc=")
    })
}

fn qemu_exe_name(family: QemuFamily) -> &'static str {
    match family {
        QemuFamily::Esp32 => "qemu-system-xtensa",
        QemuFamily::Stm32 => "qemu-system-arm",
    }
}

fn qemu_executable(family: QemuFamily) -> Option<PathBuf> {
    match family {
        QemuFamily::Esp32 => cs_qemu::qemu_xtensa_path(),
        QemuFamily::Stm32 => cs_qemu::qemu_arm_path(),
    }
}

fn find_data_path(rel: &str) -> Option<PathBuf> {
    for dir in cs_qemu::qemu_search_dirs() {
        let p = dir.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn qemu_rom_dir() -> Option<PathBuf> {
    let dirs = cs_qemu::qemu_search_dirs();
    for suffix in ["esp32/rom/bin", "esp32", ""] {
        for dir in &dirs {
            let candidate = if suffix.is_empty() {
                dir.clone()
            } else {
                dir.join(suffix)
            };
            if candidate.join("esp32-v3-rom.bin").is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// C++ uses `{firmware}.efuse` or `data/bin/esp32/esp32.efuse`. A blank 1 KiB
/// image is enough for QEMU's `nvram.esp32.efuse` (`Esp32EfuseRegs` is 124 bytes).
fn ensure_esp32_efuse(firmware: &Path) -> Result<PathBuf, String> {
    let next_to = firmware.with_extension("efuse");
    if next_to.is_file() {
        return Ok(next_to);
    }
    if let Some(p) = find_data_path("esp32/esp32.efuse") {
        return Ok(p);
    }
    let tmp = std::env::temp_dir().join("circuitsimulator-esp32.efuse");
    if !tmp.is_file() {
        std::fs::write(&tmp, [0u8; 1024])
            .map_err(|e| format!("could not create blank eFuse image: {e}"))?;
    }
    Ok(tmp)
}

/// C++ `Esp32::checkFlashImage`.
pub fn check_flash_image(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Err(
            "No firmware loaded. Compile a sketch, or load a .bin with \"Load firmware\".".into(),
        );
    }
    if bytes.len() as u64 != ESP32_FLASH_IMAGE_SIZE {
        return Err(format!(
            "Firmware must be a whole 4 MB flash image, this one is: {} bytes",
            bytes.len()
        ));
    }
    let Some((magic, chip)) = read_image_header(bytes, ESP32_BOOTLOADER_ADDR as usize) else {
        return Err("Could not read the image header".into());
    };
    if magic != ESP32_IMAGE_HEADER_MAGIC {
        if let Some((magic0, chip0)) =
            read_image_header(bytes, ESP32_OTHER_BOOTLOADER_ADDR as usize)
        {
            if magic0 == ESP32_IMAGE_HEADER_MAGIC {
                return Err(format!(
                    "This firmware is built for {}, which places its bootloader at 0x0. Circuit Simulator emulates the original ESP32, whose ROM boots from 0x1000, so this image can only reset-loop reporting an invalid header.",
                    chip_name(chip0)
                ));
            }
        }
        return Err(format!(
            "No image header at 0x1000 (found magic byte 0x{magic:x}, expected 0xe9). This is not a flashable ESP32 image."
        ));
    }
    if chip == ESP32_CHIP_ID {
        return Ok(());
    }
    Err(format!(
        "This firmware is built for {}. Circuit Simulator only emulates the original ESP32.",
        chip_name(chip)
    ))
}

fn read_image_header(bytes: &[u8], offset: usize) -> Option<(u8, u16)> {
    let hdr = bytes.get(offset..offset + 16)?;
    let magic = hdr[0];
    let chip = hdr[12] as u16 | ((hdr[13] as u16) << 8);
    Some((magic, chip))
}

fn chip_name(id: u16) -> String {
    match id {
        0x0000 => "ESP32".into(),
        0x0002 => "ESP32-S2".into(),
        0x0005 => "ESP32-C3".into(),
        0x0009 => "ESP32-S3".into(),
        0x000C => "ESP32-C2".into(),
        0x000D => "ESP32-C6".into(),
        0x0010 => "ESP32-H2".into(),
        0x0012 => "ESP32-P4".into(),
        0x0017 => "ESP32-C5".into(),
        other => format!("unknown chip id 0x{other:x}"),
    }
}

pub const ESP32_PACKAGE_XML: &str = include_str!("../../../resources/data/esp32/esp32.package");

/// Layout + live device for a canvas QEMU device chip (ESP32/STM32).
#[derive(Clone, Debug)]
pub struct QemuView {
    pub qemu: QemuComp,
    pub package: Package,
    pub packages: BTreeMap<String, Package>,
    pub logic_symbol: bool,
}

pub fn load_packages(device: &str, search: &SubcSearch) -> BTreeMap<String, Package> {
    let mut list = BTreeMap::new();
    let dev_upper = device.to_ascii_uppercase();
    if dev_upper.contains("ESP32") {
        let (_, mut pkg) = convert_package(ESP32_PACKAGE_XML);
        let key = "1- ESP32_DIP".to_string();
        if pkg.name.is_empty() {
            pkg.name = "ESP32".to_string();
        }
        list.insert(key, pkg);
        return list;
    }
    let mut bases = Vec::new();
    if let Some(d) = &search.circuit_dir {
        bases.push(d.join(device).join(device));
        bases.push(d.join("data").join(device).join(device));
        bases.push(d.join(device));
    }
    for dir in &search.data_dirs {
        bases.push(dir.join(device).join(device));
        bases.push(dir.join(device));
    }
    for base in bases {
        let mut dip = base.as_os_str().to_os_string();
        dip.push(".package");
        let dip = PathBuf::from(dip);
        if dip.is_file()
            && let Ok(text) = std::fs::read_to_string(&dip)
        {
            let (_, mut pkg) = convert_package(&text);
            let key = format!("1- {device}_DIP");
            if pkg.name.is_empty() {
                pkg.name = device.to_string();
            }
            list.insert(key, pkg);
            break;
        }
    }
    list
}

/// Fallback DIP package generated from QEMU IO pins when no package XML is found.
pub fn gpio_dip_package(qemu: &QemuComp, instance_id: &str) -> Package {
    let n = qemu.pins.len().max(1);
    let per_side = n.div_ceil(2).max(2);
    let width = 4;
    let height = (per_side as i32 + 1).max(4);
    let mut pins = Vec::with_capacity(n);
    for (i, p) in qemu.pins.iter().enumerate() {
        let suffix =
            p.id.strip_prefix(instance_id)
                .and_then(|s| s.strip_prefix('-'))
                .unwrap_or(&p.id)
                .to_string();
        let (xpos, ypos, angle) = if i < per_side {
            (-8, 8 + (i as i32) * 8, 180)
        } else {
            let right_i = i - per_side;
            ((width + 1) * 8, 8 + (right_i as i32) * 8, 0)
        };
        pins.push(PkgPin {
            id: suffix.clone(),
            label: suffix,
            pin_type: String::new(),
            xpos,
            ypos,
            angle,
            length: 8,
            space: 0,
        });
    }
    Package {
        name: crate::subcircuit::device_from_id(instance_id),
        width,
        height,
        logic_symbol: false,
        border: false,
        pins,
        ..Package::default()
    }
}

pub fn canvas_package(
    qemu: &QemuComp,
    instance_id: &str,
    packages: &BTreeMap<String, Package>,
    logic_symbol: bool,
    named: Option<&str>,
) -> Package {
    select_package(packages, logic_symbol, named)
        .cloned()
        .unwrap_or_else(|| gpio_dip_package(qemu, instance_id))
}

pub fn instantiate_qemu_view(id: &str, device: &str, search: &SubcSearch) -> QemuView {
    let dev_upper = device.to_ascii_uppercase();
    let qemu = if dev_upper.starts_with("STM32") {
        QemuComp::stm32(id, 5)
    } else {
        QemuComp::esp32(id)
    };
    let packages = load_packages(device, search);
    let package = canvas_package(&qemu, id, &packages, false, None);
    QemuView {
        qemu,
        package,
        packages,
        logic_symbol: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Circuit, serial};

    #[test]
    fn flash_image_rejects_wrong_size() {
        let err = check_flash_image(&[0xE9; 16]).unwrap_err();
        assert!(err.contains("4 MB"), "{err}");
    }

    #[test]
    fn flash_image_accepts_esp32_header() {
        let mut img = vec![0u8; ESP32_FLASH_IMAGE_SIZE as usize];
        img[ESP32_BOOTLOADER_ADDR as usize] = ESP32_IMAGE_HEADER_MAGIC;
        img[ESP32_BOOTLOADER_ADDR as usize + 12] = 0;
        img[ESP32_BOOTLOADER_ADDR as usize + 13] = 0;
        check_flash_image(&img).unwrap();
    }

    #[test]
    fn flash_image_rejects_s3_at_0x0() {
        let mut img = vec![0u8; ESP32_FLASH_IMAGE_SIZE as usize];
        img[0] = ESP32_IMAGE_HEADER_MAGIC;
        img[12] = 0x09;
        img[13] = 0x00;
        let err = check_flash_image(&img).unwrap_err();
        assert!(err.contains("ESP32-S3"), "{err}");
        assert!(err.contains("0x0"), "{err}");
    }

    #[test]
    fn esp32_args_match_create_args() {
        let q = QemuComp::esp32("Esp32-1");
        let args = q.esp32_args("/tmp/app.bin", "/123Esp32-1", "/rom", "/efuse");
        assert!(args.iter().any(|a| a.starts_with("esp32-cs,cosim-shm=")));
        assert!(args.contains(&"-icount".to_string()));
        assert!(args.iter().any(|a| a.contains("if=mtd")));
        assert!(args.iter().any(|a| a.contains("wdt_disable")));
        assert!(
            args.windows(2).any(|w| w == ["-display", "none"]),
            "esp32 should default to headless display: {args:?}"
        );
    }

    #[test]
    fn esp32_args_respect_custom_display_in_extra_args() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.extra_args = "-vnc,:0".into();
        let args = q.esp32_args("/tmp/app.bin", "/123Esp32-1", "/rom", "/efuse");
        assert!(
            !args.windows(2).any(|w| w == ["-display", "none"]),
            "explicit -vnc should suppress -display none: {args:?}"
        );

        q.extra_args = "-nographic".into();
        let args = q.esp32_args("/tmp/app.bin", "/123Esp32-1", "/rom", "/efuse");
        assert!(!args.windows(2).any(|w| w == ["-display", "none"]));
    }

    #[test]
    fn stm32_args_start_with_shm_key() {
        let q = QemuComp::stm32("STM32-1", 2);
        let args = q.stm32_args("/tmp/fw.bin", "/pidSTM32-1");
        assert_eq!(args[0], "/pidSTM32-1");
        assert_eq!(args[1], "qemu-system-arm");
        assert!(args.contains(&"stm32-f10xx".to_string()));
        assert!(args.windows(2).any(|w| w == ["-display", "none"]));
    }

    #[test]
    fn start_live_empty_firmware_is_noop() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.start_live().unwrap();
        assert!(!q.is_live());
        assert!(q.last_error.is_none());
    }

    #[test]
    fn start_live_missing_binary() {
        let mut q = QemuComp::esp32("Esp32-1");
        let err = q
            .start_live_with(
                Path::new("/no/such/qemu-system-xtensa"),
                &[],
                Duration::from_millis(50),
            )
            .unwrap_err();
        assert!(err.contains("not found"), "{err}");
        assert!(!q.is_live());
    }

    #[cfg(unix)]
    #[test]
    fn start_live_child_exit_is_reported() {
        let mut q = QemuComp::esp32("Esp32-exit");
        q.set_shm_key(cs_qemu::shm_key(std::process::id(), "qexit"));
        let err = q
            .start_live_with(Path::new("/usr/bin/true"), &[], Duration::from_secs(2))
            .unwrap_err();
        assert!(
            err.contains("terminated early") || err.contains("did not start"),
            "{err}"
        );
        assert!(!q.is_live());
    }

    #[cfg(unix)]
    #[test]
    fn start_live_timeout_kills_child() {
        let mut q = QemuComp::esp32("Esp32-to");
        q.set_shm_key(cs_qemu::shm_key(std::process::id(), "qto"));
        let err = q
            .start_live_with(
                Path::new("/bin/sleep"),
                &["8".into()],
                Duration::from_millis(80),
            )
            .unwrap_err();
        assert!(err.contains("timed out"), "{err}");
        assert!(!q.is_live());
    }

    #[test]
    fn start_live_stub_sets_running() {
        let Some(stub) = qemu_stub_path() else {
            return;
        };
        let mut q = QemuComp::esp32("Esp32-1");
        q.set_shm_key(cs_qemu::shm_key(std::process::id(), "qlive"));
        let args = vec![format!("esp32-cs,cosim-shm={}", q.shm_key())];
        q.start_live_with(&stub, &args, Duration::from_secs(2))
            .expect("stub should set arena.running");
        assert!(q.is_live());
        assert_eq!(q.arena.running, 1);
        q.pause();
        q.resume();
        q.stop_live();
        assert!(!q.is_live());
    }

    fn qemu_stub_path() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?.parent()?;
        let p = dir.join(format!(
            "qemu-cosim-stub{}",
            if cfg!(windows) { ".exe" } else { "" }
        ));
        p.is_file().then_some(p)
    }

    #[test]
    fn strap_reg_defaults_to_flash_boot() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        q.post(SimAction::Read, ESP32_GPIO_START + GPIO_STRAP, 0, 1);
        q.process_pending();
        assert_eq!(q.arena.reg_data, 0x13);
        assert_eq!(q.arena.qemu_action, SimAction::Read as u64);
    }

    #[test]
    fn unwired_rst_pin_does_not_reset_esp32_when_g02_drives_led() {
        let mut c = Circuit::new();
        c.add_qemu_esp32("Esp32-1")
            .add_resistor("Resistor-1", 330.0)
            .add_led("Led-1")
            .add_ground("Ground-1")
            .connect("Resistor-1-lPin", "Esp32-1-G02")
            .connect("Led-1-lPin", "Resistor-1-rPin")
            .connect("Led-1-rPin", "Ground-1-Gnd");
        c.configure_led("Led-1", 2.4, 0.03, 0.6, false);
        c.solve().unwrap();

        if let Some(stub) = qemu_stub_path() {
            let q = c.qemu_mut("Esp32-1").unwrap();
            q.set_shm_key(cs_qemu::shm_key(std::process::id(), "qrst"));
            let args = vec![format!("esp32-cs,cosim-shm={}", q.shm_key())];
            q.start_live_with(&stub, &args, Duration::from_secs(2))
                .expect("stub should start");
            assert!(q.is_live());
        }

        // Re-solving the circuit (which runs digital_volt_changed -> sample_inputs_nodes -> check_reset)
        // must keep the unwired Rst pin pulled high rather than killing the live QEMU process.
        c.update().unwrap();
        if qemu_stub_path().is_some() {
            let q = c.qemu_mut("Esp32-1").unwrap();
            assert!(
                q.is_live(),
                "live QEMU process must not be killed when Rst is unwired"
            );
            q.stop_live();
        }

        {
            let q = c.qemu_mut("Esp32-1").unwrap();
            let rst_idx = q.rst_pin.expect("esp32 rst pin");
            assert!(
                q.pins[rst_idx].last_inp_state(),
                "unwired Rst pin with pullup must sample high"
            );
            q.post(
                SimAction::Write,
                ESP32_GPIO_START + GPIO_ENABLE_W1TS,
                1 << 2,
                1,
            );
            q.process_pending();
            q.post(
                SimAction::Write,
                ESP32_GPIO_START + GPIO_OUT_W1TS,
                1 << 2,
                2,
            );
            q.process_pending();
        }
        c.update().unwrap();

        let v_g02 = c.pin_voltage("Esp32-1-G02").expect("G02 voltage");
        assert!(
            v_g02 > 3.0,
            "G02 should drive high into resistor+LED, got {v_g02}"
        );
    }

    #[test]
    fn esp32_usart_fifo_write_starts_tx() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        q.post(SimAction::Write, ESP32_UART0_START, 0x41, 1);
        q.process_pending();
        assert_eq!(q.usarts[0].tx_fifo.len(), 1);
        let tx = q.usarts[0].tx.expect("UART0 TX GPIO1");
        assert_eq!(q.pins[tx].mode, PinMode::Output);
        assert!(!q.pins[tx].get_out_state()); // start bit of 0x41
    }

    #[test]
    fn esp32_usart_rx_fifo_caps_and_read_clears_full_irq() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        // UART_RXFIFO_FULL_INT_ENA
        q.post(SimAction::Write, ESP32_UART0_START + 0x0C, 1, 1);
        q.process_pending();
        for byte in 0..200u16 {
            if let Some((irq, level)) = q.usarts[0].on_byte_received(byte as u8) {
                q.set_interrupt(irq, level);
            }
        }
        assert_eq!(q.usarts[0].rx_fifo.len(), 128);
        assert_eq!(q.usarts[0].int_raw & 1, 1);
        assert_eq!(q.arena.irq_level, 1);

        let mut last = 0u32;
        for _ in 0..118 {
            q.post(SimAction::Read, ESP32_UART0_START, 0, 2);
            q.process_pending();
            last = q.arena.reg_data as u32;
        }
        // 128 - 118 = 10, under the default full threshold of 11.
        assert_eq!(q.usarts[0].rx_fifo.len(), 10);
        assert_eq!(last, 117);
        assert_eq!(q.usarts[0].int_raw & 1, 0);
        assert_eq!(q.arena.irq_level, 0);
    }

    #[test]
    fn esp32_usart_status_counts_fifo() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        q.post(SimAction::Write, ESP32_UART0_START, 0x42, 1);
        q.process_pending();
        q.post(SimAction::Read, ESP32_UART0_START + 0x1C, 0, 1);
        q.process_pending();
        assert_eq!(q.arena.reg_data & 0xFF, 0); // rx empty
        assert_eq!((q.arena.reg_data >> 16) & 0xFF, 1); // one tx byte
    }

    #[test]
    fn stm32_usart_cr1_enables_tx() {
        let mut q = QemuComp::stm32("STM32-1", 5);
        q.stamp_init(0);
        // UE | TE
        q.post(
            SimAction::Write,
            STM32_USART1_START + 0x0C,
            (1 << 13) | (1 << 3),
            1,
        );
        q.process_pending();
        assert!(q.usarts[0].module.is_tx_enabled());
        let tx = q.usarts[0].tx.expect("USART1 PA9");
        assert_eq!(q.pins[tx].mode, PinMode::Output);
    }

    #[test]
    fn stm32_spi_cr1_master_enable() {
        let mut q = QemuComp::stm32("STM32-1", 5);
        q.stamp_init(0);
        // SPE | MSTR
        q.post(SimAction::Write, STM32_SPI1_START, (1 << 6) | (1 << 2), 1);
        q.process_pending();
        assert_eq!(q.spis[0].module.mode, crate::digital::SpiMode::Master);
        q.post(SimAction::Read, STM32_SPI1_START + 0x08, 0, 1);
        q.process_pending();
        assert_eq!(q.arena.reg_data & 0x02, 0x02); // TXE
    }

    #[test]
    fn stm32_timer_oc_toggles_pin() {
        let mut q = QemuComp::stm32("STM32-1", 5);
        q.stamp_init(0);
        // TIM2 CH0 high
        q.post(SimAction::Write, STM32_TIM2_START, 1 << 8, 1);
        q.process_pending();
        let pa0 = q.timers[1].oc_pins[0].expect("TIM2 CH1 PA0");
        assert_eq!(q.pins[pa0].mode, PinMode::Output);
        assert!(q.pins[pa0].get_out_state());
    }

    #[test]
    fn esp32_twi_ctr_starts_master() {
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        // MS_MODE | TRANS_START
        q.post(
            SimAction::Write,
            ESP32_I2C0_START + 0x04,
            (1 << 4) | (1 << 5),
            1,
        );
        q.process_pending();
        assert_eq!(q.twis[0].module.mode, crate::digital::TwiMode::Master);
        assert!(q.twis[0].busy);
        q.post(SimAction::Read, ESP32_I2C0_START + 0x08, 0, 1);
        q.process_pending();
        assert_ne!(q.arena.reg_data & (1 << 4), 0); // BUS_BUSY
    }

    #[test]
    fn stm32_timer_arr_readback() {
        let mut q = QemuComp::stm32("STM32-1", 5);
        q.stamp_init(0);
        q.post(SimAction::Write, STM32_TIM2_START + 0x2C, 1000, 1);
        q.process_pending();
        q.post(SimAction::Read, STM32_TIM2_START + 0x2C, 0, 1);
        q.process_pending();
        assert_eq!(q.arena.reg_data, 1000);
    }

    #[test]
    fn esp32_serial_monitor_publish_and_receive() {
        let _guard = serial::test_lock();
        serial::clear("Esp32-1:USART1");
        serial::clear("default");
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        q.post(SimAction::Write, ESP32_UART0_START, b'H' as u64, 1);
        q.process_pending();

        let mut now = 1;
        while let Some(delay) = q.next_event_delay(now) {
            now += delay;
            q.run_event(&|_| 0.0, now);
            if !serial::out_text("Esp32-1:USART1").is_empty() {
                break;
            }
        }

        assert_eq!(serial::out_text("Esp32-1:USART1"), "H");
        assert_eq!(serial::out_text("default"), "H");

        // Host UI sends character 'X'
        serial::send_text("Esp32-1:USART1", "X");
        q.run_event(&|_| 0.0, now + 1);

        assert_eq!(serial::in_text("Esp32-1:USART1"), "X");
        assert_eq!(q.usarts[0].rx_fifo.pop_front(), Some(b'X'));
    }

    #[test]
    fn esp32_usart_catch_up_drains_fifo() {
        let _guard = serial::test_lock();
        serial::clear("Esp32-1:USART1");
        serial::clear("Esp32-1");
        serial::clear("default");
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        for byte in [b'A', b'B', b'C'] {
            q.post(SimAction::Write, ESP32_UART0_START, u64::from(byte), 1);
            q.process_pending();
        }
        let period = q.usarts[0].module.period;
        q.catch_up_peripherals(1 + period * 40);
        assert_eq!(serial::out_text("Esp32-1:USART1"), "ABC");
        assert_eq!(serial::out_text("default"), "ABC");
        assert!(q.usarts[0].tx_fifo.is_empty());
    }

    #[test]
    fn stm32_serial_monitor_publish_and_receive() {
        let _guard = serial::test_lock();
        serial::clear("STM32-1:USART1");
        serial::clear("default");
        let mut q = QemuComp::stm32("STM32-1", 5);
        q.stamp_init(0);
        // Enable UE | TE | RE
        q.post(
            SimAction::Write,
            STM32_USART1_START + 0x0C,
            (1 << 13) | (1 << 3) | (1 << 2),
            1,
        );
        q.process_pending();
        // Write byte 'A' to DR
        q.post(SimAction::Write, STM32_USART1_START + 0x04, b'A' as u64, 2);
        q.process_pending();

        let mut now = 2;
        while let Some(delay) = q.next_event_delay(now) {
            now += delay;
            q.run_event(&|_| 0.0, now);
            if !serial::out_text("STM32-1:USART1").is_empty() {
                break;
            }
        }

        assert_eq!(serial::out_text("STM32-1:USART1"), "A");
        assert_eq!(serial::out_text("default"), "A");

        // Host UI sends character 'Y'
        serial::send_text("STM32-1:USART1", "Y");
        q.run_event(&|_| 0.0, now + 1);

        assert_eq!(serial::in_text("STM32-1:USART1"), "Y");
        // Read USART_DR
        q.post(SimAction::Read, STM32_USART1_START + 0x04, 0, now + 2);
        q.process_pending();
        assert_eq!(q.arena.reg_data & 0xFF, b'Y' as u64);
    }

    #[test]
    fn esp32_uart_rmw_preserves_clkdiv_conf0_and_triggers_rx_timeout() {
        let mut q = QemuComp::esp32("Esp32-rmw");
        q.stamp_init(0);

        // Read-modify-write UART_CLKDIV_REG (0x14): set div_int then div_frag
        q.post(SimAction::Write, ESP32_UART0_START + 0x14, 694, 1);
        q.process_pending();
        q.post(SimAction::Read, ESP32_UART0_START + 0x14, 0, 2);
        q.process_pending();
        assert_eq!(q.arena.reg_data, 694);
        let clkdiv = (q.arena.reg_data as u32) | (7 << 20);
        q.post(
            SimAction::Write,
            ESP32_UART0_START + 0x14,
            u64::from(clkdiv),
            3,
        );
        q.process_pending();
        assert!((q.usarts[0].module.baud_rate - 115200).abs() <= 2);

        // Read-modify-write UART_CONF0_REG (0x20): toggle rxfifo_rst without losing 8N1
        q.post(SimAction::Read, ESP32_UART0_START + 0x20, 0, 4);
        q.process_pending();
        let conf0 = q.arena.reg_data as u32;
        assert_eq!((conf0 >> 2) & 0b11, 3, "default conf0 must report 8-bit");
        q.post(
            SimAction::Write,
            ESP32_UART0_START + 0x20,
            u64::from(conf0 | (1 << 17)),
            5,
        );
        q.process_pending();
        q.post(
            SimAction::Write,
            ESP32_UART0_START + 0x20,
            u64::from(conf0),
            6,
        );
        q.process_pending();
        assert_eq!(q.usarts[0].module.data_bits, 8);

        // Enable RX_TOUT with high RX_FULL threshold (112) as Arduino-ESP32 does
        let conf1 = 112u32 | (10 << 8) | (2 << 24) | (1 << 31);
        q.post(
            SimAction::Write,
            ESP32_UART0_START + 0x24,
            u64::from(conf1),
            7,
        );
        q.process_pending();
        q.post(
            SimAction::Write,
            ESP32_UART0_START + 0x0C,
            (1 << 0) | (1 << 8),
            8,
        );
        q.process_pending();

        // Receive 3 bytes (below 112 full threshold) -> arms rx_tout_due
        let mut t = 1_000u64;
        for &b in b"Hi!" {
            let irq = q.usarts[0].on_byte_received_at(b, t);
            assert!(irq.is_none());
            t += 100;
        }
        assert_eq!(q.arena.irq_level, 0);
        let due = q.usarts[0].rx_tout_due.expect("rx_tout_due must be armed");
        q.advance_usarts(due);
        assert_ne!(
            q.usarts[0].int_raw & (1 << 8),
            0,
            "RXFIFO_TOUT_INT must assert"
        );
        assert_eq!(q.arena.irq_level, 1);

        // Clear RXFIFO_TOUT_INT via INT_CLR (0x10)
        q.post(SimAction::Write, ESP32_UART0_START + 0x10, 1 << 8, due + 1);
        q.process_pending();
        assert_eq!(q.usarts[0].int_raw & (1 << 8), 0);
        assert_eq!(q.arena.irq_level, 0);

        // Verify UART_MEM_RX_STATUS_REG (0x60) rd_addr/wr_addr match ESP-IDF
        // `uart_ll_get_rxfifo_len` across partial fill, wraparound, and empty.
        let read_mem_rx_len = |q: &mut QemuComp| -> u32 {
            q.post(SimAction::Read, ESP32_UART0_START + 0x1C, 0, due + 2);
            q.process_pending();
            let fifo_cnt = (q.arena.reg_data as u32) & 0xFF;
            q.post(SimAction::Read, ESP32_UART0_START + 0x60, 0, due + 3);
            q.process_pending();
            let mem_rx = q.arena.reg_data as u32;
            let rd_addr = (mem_rx >> 2) & 0x7FF;
            let wr_addr = (mem_rx >> 13) & 0x7FF;
            if wr_addr > rd_addr {
                wr_addr - rd_addr
            } else if wr_addr < rd_addr {
                (wr_addr + 128) - rd_addr
            } else if fifo_cnt > 0 {
                128
            } else {
                0
            }
        };
        assert_eq!(read_mem_rx_len(&mut q), 3);
        for _ in 0..3 {
            q.post(SimAction::Read, ESP32_UART0_START, 0, due + 4);
            q.process_pending();
        }
        assert_eq!(read_mem_rx_len(&mut q), 0);

        // Push 120 bytes and drain 120 bytes, then push 15 bytes so wr_addr wraps past 128.
        for b in 0..120u8 {
            q.usarts[0].on_byte_received_at(b, due + 5);
        }
        assert_eq!(read_mem_rx_len(&mut q), 120);
        for _ in 0..120 {
            q.post(SimAction::Read, ESP32_UART0_START, 0, due + 6);
            q.process_pending();
        }
        assert_eq!(read_mem_rx_len(&mut q), 0);
        for b in 0..15u8 {
            q.usarts[0].on_byte_received_at(b, due + 7);
        }
        assert_eq!(read_mem_rx_len(&mut q), 15);
    }

    #[test]
    fn two_esp32_exchange_uart_over_wired_pins() {
        let _guard = serial::test_lock();
        serial::clear("Esp32-1:USART1");
        serial::clear("Esp32-2:USART1");
        serial::clear("Esp32-1");
        serial::clear("Esp32-2");
        serial::clear("default");

        let mut c = Circuit::new();
        c.add_qemu_esp32("Esp32-1")
            .add_qemu_esp32("Esp32-2")
            .connect("Esp32-1-G01", "Esp32-2-G03")
            .connect("Esp32-2-G01", "Esp32-1-G03");
        c.solve().unwrap();

        // Start simulation so event queue is active, then inject TX writes on both ESP32s
        c.run_ps(1_000).unwrap();
        let t_now = c.circ_time;
        for (uid, msg) in [
            ("Esp32-1", b"Hello mars!\n".as_slice()),
            ("Esp32-2", b"Hello world\n".as_slice()),
        ] {
            let q = c.qemu_mut(uid).unwrap();
            for &b in msg {
                q.post(SimAction::Write, ESP32_UART0_START, u64::from(b), t_now);
                q.process_pending();
            }
        }
        // Kick circuit solve + schedule bit edges
        c.update().unwrap();
        // Run 2 ms of circuit time (12 bytes @ 115200 baud ≈ 1.04 ms)
        c.run_ps(2_000_000_000).unwrap();

        assert_eq!(serial::out_text("Esp32-1:USART1"), "Hello mars!\n");
        assert_eq!(serial::out_text("Esp32-2:USART1"), "Hello world\n");
        assert_eq!(serial::in_text("Esp32-2:USART1"), "Hello mars!\n");
        assert_eq!(serial::in_text("Esp32-1:USART1"), "Hello world\n");

        let rx1: Vec<u8> = c.qemu_mut("Esp32-1").unwrap().usarts[0]
            .rx_fifo
            .iter()
            .copied()
            .collect();
        let rx2: Vec<u8> = c.qemu_mut("Esp32-2").unwrap().usarts[0]
            .rx_fifo
            .iter()
            .copied()
            .collect();
        assert_eq!(rx1, b"Hello world\n");
        assert_eq!(rx2, b"Hello mars!\n");
    }

    #[test]
    fn deferred_uart_status_poll_wakes_on_tx_rx_irq_or_timeout() {
        let _guard = serial::test_lock();
        let mut q = QemuComp::esp32("Esp32-defer");
        q.stamp_init(1);
        let status_addr = ESP32_UART0_START + 0x1C;

        // While TX is idle, UART_STATUS_REG is never considered busy.
        assert_eq!(q.find_busy_usart_status(status_addr), None);

        // Start shifting two bytes on UART0.
        q.post(SimAction::Write, ESP32_UART0_START, u64::from(b'A'), 1_000);
        q.process_pending();
        q.post(SimAction::Write, ESP32_UART0_START, u64::from(b'B'), 1_001);
        q.process_pending();
        assert_eq!(q.find_busy_usart_status(status_addr), Some(0));
        assert_eq!(q.find_busy_usart_status(ESP32_UART0_START + 0x20), None);

        // Snapshot a deferred poll while 2 bytes are in TX FIFO.
        let dp = DeferredPoll {
            usart_idx: 0,
            status: q.usarts[0].status_snapshot(),
            irq: (q.arena.irq_number, q.arena.irq_level),
            max_until: 1_000 + DEFER_POLL_MAX_PS,
        };
        assert!(!q.deferred_poll_ready(dp, 1_000));

        // Mid-byte bit edges do not wake the deferred poll yet.
        let bit_due = q.usarts[0].module.tx_due.unwrap();
        q.advance_usarts(bit_due);
        assert!(!q.deferred_poll_ready(dp, bit_due));

        // Completing the first byte decrements tx_fifo.len() and wakes immediately.
        let mut t = bit_due;
        for _ in 0..16 {
            if q.deferred_poll_ready(dp, t) {
                break;
            }
            t = q.usarts[0].module.tx_due.unwrap();
            q.advance_usarts(t);
        }
        assert!(q.deferred_poll_ready(dp, t));
        assert_eq!(q.usarts[0].tx_fifo.len(), 1);

        // Receiving a byte on RX also wakes a deferred TX status poll immediately.
        let dp2 = DeferredPoll {
            usart_idx: 0,
            status: q.usarts[0].status_snapshot(),
            irq: (q.arena.irq_number, q.arena.irq_level),
            max_until: t + DEFER_POLL_MAX_PS,
        };
        assert!(!q.deferred_poll_ready(dp2, t));
        q.usarts[0].on_byte_received_at(b'Z', t + 10);
        assert!(q.deferred_poll_ready(dp2, t + 10));

        // Timeout cap always wakes even if status did not change.
        let dp3 = DeferredPoll {
            usart_idx: 0,
            status: q.usarts[0].status_snapshot(),
            irq: (q.arena.irq_number, q.arena.irq_level),
            max_until: t + DEFER_POLL_MAX_PS,
        };
        assert!(!q.deferred_poll_ready(dp3, dp3.max_until - 1));
        assert!(q.deferred_poll_ready(dp3, dp3.max_until));
    }

    #[test]
    fn esp32_uart2_pins_stay_gpio_until_configured() {
        let mut q = QemuComp::esp32("Esp32-uart2-gpio");
        q.stamp_init(0);

        let g17 = q.esp32_gpio.pad[17].expect("G17");
        let g16 = q.esp32_gpio.pad[16].expect("G16");
        let g01 = q.esp32_gpio.pad[1].expect("G01");

        // UART0 TX is enabled for ROM boot, while UART1/UART2 leave their pads as GPIO.
        assert_eq!(q.pins[g01].mode, PinMode::Output);
        assert!(q.pins[g01].get_out_state());
        assert_eq!(q.pins[g17].mode, PinMode::Input);
        assert!(!q.pins[g17].get_out_state());
        assert!(!q.usarts[2].module.is_tx_enabled());
        assert!(!q.usarts[2].module.is_rx_enabled());

        // Configuring G17 as GPIO output and writing LOW must drive LOW, not stay stuck HIGH.
        q.post(
            SimAction::Write,
            ESP32_GPIO_START + GPIO_ENABLE_W1TS,
            1 << 17,
            1,
        );
        q.process_pending();
        assert_eq!(q.pins[g17].mode, PinMode::Output);
        assert!(!q.pins[g17].get_out_state());

        q.post(
            SimAction::Write,
            ESP32_GPIO_START + GPIO_OUT_W1TC,
            1 << 17,
            2,
        );
        q.process_pending();
        assert!(!q.pins[g17].get_out_state());

        // Toggling G16 low as a GPIO must not trigger bogus UART2 RX start bits.
        q.pins[g16].get_inp_state(3.3);
        assert!(!q.on_input_pins_changed(10));
        q.pins[g16].get_inp_state(0.0);
        assert!(!q.on_input_pins_changed(20));

        // Writing to UART2 registers lazily activates UART2 TX/RX.
        q.post(SimAction::Write, ESP32_UART2_START, b'Z' as u64, 30);
        q.process_pending();
        assert!(q.usarts[2].module.is_tx_enabled());
        assert!(q.usarts[2].module.is_rx_enabled());
        assert_eq!(q.usarts[2].tx_fifo.len(), 1);
    }

    #[test]
    fn live_gpio_enable_pulses_drive_hd44780_without_coalescing() {
        use std::ptr;
        use std::sync::atomic::{AtomicU64, Ordering};

        use crate::elements::{Comp, Kind};

        let Some(stub) = qemu_stub_path() else {
            return;
        };

        let mut c = Circuit::new();
        c.add_qemu_esp32("Esp32-1")
            .add_comp(Comp::hd44780("Lcd-1", 2, 16))
            .add_ground("Gnd-1")
            .connect("Lcd-1-PinRS", "Esp32-1-G19")
            .connect("Lcd-1-PinEn", "Esp32-1-G18")
            .connect("Lcd-1-PinRW", "Gnd-1-Gnd")
            .connect("Lcd-1-dataPin4", "Esp32-1-G05")
            .connect("Lcd-1-dataPin5", "Esp32-1-G17")
            .connect("Lcd-1-dataPin6", "Esp32-1-G16")
            .connect("Lcd-1-dataPin7", "Esp32-1-G04");
        c.solve().unwrap();

        let key = cs_qemu::shm_key(std::process::id(), "qhd44780");
        {
            let q = c.qemu_mut("Esp32-1").unwrap();
            q.set_shm_key(key.clone());
            let args = vec![format!("esp32-cs,cosim-shm={key}")];
            q.start_live_with(&stub, &args, Duration::from_secs(2))
                .expect("stub should start");
        }

        // Open the same shared arena from a worker thread acting like QEMU
        // executing LiquidCrystal 4-bit initialization and writing "Hi!".
        let guest_shm = SharedArena::open(&key).expect("open shm");
        let guest = thread::spawn(move || {
            let post_write = |addr: u64, data: u32, t_ps: u64| {
                while guest_shm.load_simu_action() != 0 {
                    thread::yield_now();
                }
                unsafe {
                    let p = guest_shm.as_ref() as *const Arena as *mut Arena;
                    (*p).reg_addr = addr;
                    (*p).reg_data = u64::from(data);
                    (*p).qemu_time = t_ps;
                    (*p).simu_time = t_ps;
                    (&*ptr::addr_of!((*p).simu_action).cast::<AtomicU64>())
                        .store(SimAction::Write as u64, Ordering::Release);
                }
            };

            let rs_bit = 1u32 << 19;
            let en_bit = 1u32 << 18;
            let data_bits = |nibble: u8| -> u32 {
                let mut m = 0u32;
                if nibble & 1 != 0 {
                    m |= 1 << 5;
                }
                if nibble & 2 != 0 {
                    m |= 1 << 17;
                }
                if nibble & 4 != 0 {
                    m |= 1 << 16;
                }
                if nibble & 8 != 0 {
                    m |= 1 << 4;
                }
                m
            };
            let all_pins = rs_bit | en_bit | (1 << 5) | (1 << 17) | (1 << 16) | (1 << 4);
            let mut t = 1_000_000u64;
            post_write(ESP32_GPIO_START + GPIO_ENABLE_W1TS, all_pins, t);

            let mut pulse_nibble = |rs: bool, nibble: u8| {
                let mut out = data_bits(nibble);
                if rs {
                    out |= rs_bit;
                }
                t += 1_000_000;
                post_write(ESP32_GPIO_START + GPIO_OUT, out, t);
                // 1 us Enable pulse (far shorter than GUEST_AHEAD_PS = 50 us)
                t += 1_000_000;
                post_write(ESP32_GPIO_START + GPIO_OUT_W1TS, en_bit, t);
                t += 1_000_000;
                post_write(ESP32_GPIO_START + GPIO_OUT_W1TC, en_bit, t);
            };
            let send_byte = |rs: bool, byte: u8, pulse: &mut dyn FnMut(bool, u8)| {
                pulse(rs, (byte >> 4) & 0x0F);
                pulse(rs, byte & 0x0F);
            };

            // Standard HD44780 4-bit init sequence
            pulse_nibble(false, 0x03);
            pulse_nibble(false, 0x03);
            pulse_nibble(false, 0x03);
            pulse_nibble(false, 0x02);
            send_byte(false, 0x28, &mut pulse_nibble); // 4-bit, 2 lines
            send_byte(false, 0x0C, &mut pulse_nibble); // display on
            send_byte(false, 0x01, &mut pulse_nibble); // clear
            send_byte(false, 0x06, &mut pulse_nibble); // entry mode
            for &ch in b"Hi!" {
                send_byte(true, ch, &mut pulse_nibble);
            }
            while guest_shm.load_simu_action() != 0 {
                thread::yield_now();
            }
        });

        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            c.run_ps_budgeted(16_666_666_667, Some(Duration::from_millis(12)))
                .unwrap();
            let done = c.components().iter().any(|comp| {
                if let Kind::Hd44780(h) = &comp.kind {
                    h.disp_on && h.lines()[0].starts_with("Hi!")
                } else {
                    false
                }
            });
            if done {
                break;
            }
        }

        guest.join().expect("guest thread finished");
        if let Some(q) = c.qemu_mut("Esp32-1") {
            q.stop_live();
        }

        let lcd = c
            .components()
            .iter()
            .find_map(|comp| match &comp.kind {
                Kind::Hd44780(h) => Some(h),
                _ => None,
            })
            .expect("Lcd-1");
        assert!(lcd.disp_on, "HD44780 display should be turned on");
        assert_eq!(lcd.data_length, 4);
        assert_eq!(lcd.line_length, 40);
        assert!(
            lcd.lines()[0].starts_with("Hi!"),
            "expected 'Hi!' on LCD row 0, got {:?}",
            lcd.lines()
        );
    }

    #[test]
    fn live_future_gpio_writes_wait_for_simu_time_without_collapsing() {
        use std::ptr;
        use std::sync::atomic::{AtomicU64, Ordering};

        let Some(stub) = qemu_stub_path() else {
            return;
        };

        let mut c = Circuit::new();
        c.add_qemu_esp32("Esp32-1")
            .add_resistor("Resistor-1", 100.0)
            .add_led("Led-1")
            .add_ground("Ground-1")
            .connect("Resistor-1-lPin", "Esp32-1-G02")
            .connect("Led-1-lPin", "Resistor-1-rPin")
            .connect("Led-1-rPin", "Ground-1-Gnd");
        c.configure_led("Led-1", 2.4, 0.03, 0.6, false);
        c.solve().unwrap();

        let key = cs_qemu::shm_key(std::process::id(), "qblink");
        {
            let q = c.qemu_mut("Esp32-1").unwrap();
            q.set_shm_key(key.clone());
            let args = vec![format!("esp32-cs,cosim-shm={key}")];
            q.start_live_with(&stub, &args, Duration::from_secs(2))
                .expect("stub should start");
        }

        // Simulate QEMU running -icount sleep=off: the guest executes 100 ms
        // delays almost instantaneously in wall time, posting future simu_time
        // writes and advancing qemu_time ahead while the circuit steps in 10 ms
        // frames.
        const PERIOD_PS: u64 = 100_000_000_000; // 100 ms per half-cycle
        const FRAME_PS: u64 = 10_000_000_000; // 10 ms per frame
        let guest_shm = SharedArena::open(&key).expect("open shm");
        let guest = thread::spawn(move || {
            let post_write = |addr: u64, data: u32, t_ps: u64| {
                while guest_shm.load_simu_action() != 0 {
                    thread::yield_now();
                }
                unsafe {
                    let p = guest_shm.as_ref() as *const Arena as *mut Arena;
                    (*p).reg_addr = addr;
                    (*p).reg_data = u64::from(data);
                    (*p).qemu_time = t_ps;
                    (*p).simu_time = t_ps;
                    (&*ptr::addr_of!((*p).simu_action).cast::<AtomicU64>())
                        .store(SimAction::Write as u64, Ordering::Release);
                }
            };
            let set_qemu_time = |t_ps: u64| unsafe {
                let p = guest_shm.as_ref() as *const Arena as *mut Arena;
                (&*ptr::addr_of!((*p).qemu_time).cast::<AtomicU64>())
                    .store(t_ps, Ordering::Release);
            };

            let bit = 1u32 << 2;
            post_write(ESP32_GPIO_START + GPIO_ENABLE_W1TS, bit, 10_000_000_000);
            // Blink ON at 100 ms, OFF at 200 ms, ON at 300 ms, OFF at 400 ms.
            // Advance qemu_time ahead after each write to mimic unthrottled
            // irq_poll_timer_cb ticking during delay().
            post_write(ESP32_GPIO_START + GPIO_OUT_W1TS, bit, PERIOD_PS);
            set_qemu_time(2 * PERIOD_PS);
            post_write(ESP32_GPIO_START + GPIO_OUT_W1TC, bit, 2 * PERIOD_PS);
            set_qemu_time(3 * PERIOD_PS);
            post_write(ESP32_GPIO_START + GPIO_OUT_W1TS, bit, 3 * PERIOD_PS);
            set_qemu_time(4 * PERIOD_PS);
            post_write(ESP32_GPIO_START + GPIO_OUT_W1TC, bit, 4 * PERIOD_PS);
            while guest_shm.load_simu_action() != 0 {
                thread::yield_now();
            }
            set_qemu_time(5 * PERIOD_PS);
        });

        let mut edges: Vec<(bool, u64)> = Vec::new();
        let mut last_high = false;
        let deadline = Instant::now() + Duration::from_secs(5);
        while c.circ_time < 450_000_000_000 && Instant::now() < deadline {
            c.run_ps_budgeted(FRAME_PS, Some(Duration::from_millis(10)))
                .unwrap();
            let high = c.pin_voltage("Esp32-1-G02").unwrap_or(0.0) > 2.0;
            if high != last_high {
                edges.push((high, c.circ_time));
                last_high = high;
            }
        }

        if let Some(q) = c.qemu_mut("Esp32-1") {
            q.stop_live();
        }
        // Unblock the guest thread if the test stopped early so join cannot hang.
        if let Ok(shm) = SharedArena::open(&key) {
            unsafe {
                let arena = shm.as_ref() as *const Arena as *mut Arena;
                (&*ptr::addr_of!((*arena).simu_action).cast::<AtomicU64>())
                    .store(0, Ordering::Release);
            }
        }
        let _ = guest.join();

        assert_eq!(
            edges.len(),
            4,
            "expected 4 LED transitions (ON/OFF/ON/OFF), got {edges:?}"
        );
        let states: Vec<bool> = edges.iter().map(|(s, _)| *s).collect();
        assert_eq!(states, vec![true, false, true, false]);
        for pair in edges.windows(2) {
            let dt = pair[1].1.saturating_sub(pair[0].1);
            assert!(
                dt.abs_diff(PERIOD_PS) <= FRAME_PS,
                "each blink half-period should be ~100 ms (within 1 frame), got {dt} ps in {edges:?}"
            );
        }
    }
}
