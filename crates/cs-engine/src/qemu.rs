//! QEMU-backed MCU (C++ `QemuDevice` / `Esp32` / `Stm32`).
//!
//! Tests inject arena doorbells and the GPIO / USART / SPI / TWI / timer
//! modules drive `IoPin`s. Power-on with firmware + a `qemu-system-*` binary
//! spawns the child against a shared-memory arena (C++ `startQemuProcess`:
//! POSIX `shm_open` / Windows `CreateFileMappingA`). Not drawn (catalog-last).

pub mod periph;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cs_qemu::{Arena, QemuProcess, SharedArena, SimAction};

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
const GPIO_ENABLE: u64 = 0x20;
const GPIO_ENABLE_W1TS: u64 = 0x24;
const GPIO_ENABLE_W1TC: u64 = 0x28;
const GPIO_STRAP: u64 = 0x38;
const GPIO_IN: u64 = 0x3C;
const GPIO_IN1: u64 = 0x40;

const CRL_OFFSET: u64 = 0x00;
const CRH_OFFSET: u64 = 0x04;
const IDR_OFFSET: u64 = 0x08;
const ODR_OFFSET: u64 = 0x0C;
const BSRR_OFFSET: u64 = 0x10;
const BRR_OFFSET: u64 = 0x14;
const LCKR_OFFSET: u64 = 0x18;

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

#[derive(Clone, Debug)]
struct Esp32Gpio {
    state: u32,
    enable: u32,
    /// Pad number → index in `QemuComp::pins`.
    pad: [Option<usize>; 40],
}

impl Esp32Gpio {
    fn new() -> Self {
        Self {
            state: 0,
            enable: 0,
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

/// Live `qemu-system-*` child + the shm it attached.
struct LiveQemu {
    shm: SharedArena,
    process: QemuProcess,
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
        }
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
            if let Some(&Some(node_idx)) = self.node_indices.get(i) {
                if let Some(n) = nodes.get(node_idx) {
                    let _ = p.get_inp_state(n.volt);
                }
            }
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
            }
            for i in 0..self.usarts.len() {
                let tx = self.usarts[i].tx;
                self.usarts[i].module.enable_tx(true, &mut self.pins, tx);
                self.usarts[i].module.enable_rx(true);
            }
        }
        if let Err(e) = self.start_live() {
            self.last_error = Some(e);
        }
    }

    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// Kill the child (C++ `killQemuProcess`) and drop the shm.
    pub fn stop_live(&mut self) {
        self.live = None;
        self.next_wake = None;
    }

    /// SIGSTOP (C++ `stopQemuProcess`). No-op without a live child or with GDB.
    /// On Windows freeze-on-pause is a no-op (`SIGSTOP` has no equivalent).
    pub fn pause(&mut self) {
        if let Some(live) = &mut self.live {
            live.process.gdb = self.gdb_port != 0;
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
        let (args, _) = self.build_args(&exe)?;
        self.spawn_live(&exe, &args, timeout)
    }

    /// Spawn a specific binary (tests use the cosim stub / `/bin/true`).
    pub fn start_live_with(
        &mut self,
        executable: &Path,
        args: &[String],
        timeout: Duration,
    ) -> Result<(), String> {
        self.spawn_live(executable, args, timeout)
    }

    fn spawn_live(
        &mut self,
        executable: &Path,
        args: &[String],
        timeout: Duration,
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
        let mut process = QemuProcess::spawn(executable, args).map_err(|e| e.to_string())?;
        process.gdb = self.gdb_port != 0;
        let start = Instant::now();
        while shm.as_ref().running == 0 {
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
        self.arena = *shm.as_ref();
        self.live = Some(LiveQemu { shm, process });
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
        if let Some(live) = &self.live {
            let shm = live.shm.as_ref();
            self.arena.simu_action = shm.simu_action;
            self.arena.simu_time = shm.simu_time;
            self.arena.qemu_time = shm.qemu_time;
            self.arena.reg_addr = shm.reg_addr;
            self.arena.reg_data = shm.reg_data;
            self.arena.running = shm.running;
            self.arena.qemu_action = shm.qemu_action;
        }
    }

    fn push_live(&mut self) {
        if let Some(live) = &mut self.live {
            let shm = live.shm.as_mut();
            shm.simu_action = self.arena.simu_action;
            shm.simu_time = self.arena.simu_time;
            shm.qemu_action = self.arena.qemu_action;
            shm.reg_data = self.arena.reg_data;
            shm.reg_addr = self.arena.reg_addr;
            shm.irq_number = self.arena.irq_number;
            shm.irq_level = self.arena.irq_level;
        }
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
            take(&mut best, u.module.remain(now));
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
        if action == SimAction::Freq {
            self.updt_frequency();
        } else if action != SimAction::Event {
            self.do_action();
            self.run_module_event();
        }
        self.arena.simu_action = 0;
        self.arena.simu_time = 0;
        self.push_live();
        Vec::new()
    }

    /// One simulator wake. Inject path processes a posted doorbell; a live
    /// child spin-waits like C++ `QemuDevice::runEvent`.
    pub fn run_event(&mut self, v_of: &impl Fn(&str) -> f64, now: u64) -> Vec<(usize, PinAction)> {
        self.sample_inputs(v_of);
        self.tick_peripherals(now);
        if self.is_live() {
            self.run_live_event(now)
        } else {
            self.process_pending()
        }
    }

    /// Fast path: zero-allocation execution using pre-resolved node indices.
    pub fn run_event_nodes_buf(
        &mut self,
        nodes: &[crate::net::ENode],
        now: u64,
        actions: &mut Vec<(usize, PinAction)>,
    ) {
        self.sample_inputs_nodes(nodes);
        self.tick_peripherals(now);
        if self.is_live() {
            let act = self.run_live_event(now);
            actions.extend(act);
        } else {
            let act = self.process_pending();
            actions.extend(act);
        }
    }

    fn run_live_event(&mut self, now: u64) -> Vec<(usize, PinAction)> {
        self.run_module_event();
        self.next_wake = None;
        let mut spin_count = 0u32;
        let deadline = Instant::now() + Duration::from_millis(50);
        loop {
            let action_u64 = if let Some(live) = &self.live {
                live.shm.as_ref().simu_action
            } else {
                0
            };

            if action_u64 == 0 {
                spin_count += 1;
                if spin_count < 256 {
                    std::hint::spin_loop();
                } else {
                    std::thread::yield_now();
                }

                if (spin_count & 0x0FFF == 0) || Instant::now() >= deadline {
                    if !self.is_process_running() {
                        self.stop_live();
                        return Vec::new();
                    }
                    if Instant::now() >= deadline {
                        self.next_wake = Some(now.saturating_add(1_000_000));
                        return Vec::new();
                    }
                }
                continue;
            }

            self.pull_live();
            let next_time = self.arena.simu_time.saturating_add(self.time_offset);
            if next_time <= now {
                let _ = self.process_pending();
                spin_count = 0;
                continue;
            }
            let action = SimAction::from_u64(self.arena.simu_action);
            if action == SimAction::Freq {
                self.updt_frequency();
            } else if action != SimAction::Event {
                self.do_action();
                self.run_module_event();
            }
            self.arena.simu_action = 0;
            self.arena.simu_time = 0;
            self.push_live();
            self.next_wake = Some(next_time);
            return Vec::new();
        }
    }

    /// C++ `QemuDevice::voltChanged` on the reset pin: low kills the child.
    pub fn check_reset(&mut self) {
        let Some(i) = self.rst_pin else {
            return;
        };
        let reset = !self.pins[i].last_inp_state();
        if reset {
            self.stop_live();
        } else if !self.is_live() && !self.firmware.is_empty() && self.last_error.is_none() {
            if let Err(e) = self.start_live() {
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
            self.push_live();
        } else if action == SimAction::Write {
            self.write_mem(address, self.arena.reg_data as u32);
        }
    }

    fn run_module_event(&mut self) {
        let Some(ev) = self.event.take() else {
            return;
        };
        match ev.action {
            SimAction::Read => self.read_register(ev),
            SimAction::Write => self.write_register(ev),
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
        self.push_live();
    }

    fn write_register(&mut self, ev: PendingEvent) {
        match ev.kind {
            ModuleKind::Esp32Gpio => self.esp32_write(ev.address, ev.value as u32),
            ModuleKind::Esp32IoMux => {
                if let Some(ref mut m) = self.esp32_iomux {
                    m.write(ev.address.saturating_sub(m.mem_start), ev.value as u32);
                }
            }
            ModuleKind::Stm32Port(n) => self.stm32_write(n as usize, ev.address, ev.value as u32),
            ModuleKind::Stm32Afio => self.stm32_afio_write(ev.address, ev.value as u32),
            ModuleKind::Esp32Usart(n) | ModuleKind::Stm32Usart(n) => {
                self.usart_write(n, ev.address, ev.value as u32);
            }
            ModuleKind::Esp32Spi(n) | ModuleKind::Stm32Spi(n) => {
                self.spi_write(n, ev.address, ev.value as u32);
            }
            ModuleKind::Esp32Twi(n) | ModuleKind::Stm32Twi(n) => {
                self.twi_write(n, ev.address, ev.value as u32);
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
        let Some(u) = self.usarts.get_mut(n as usize) else {
            return 0;
        };
        let offset = address.saturating_sub(u.mem_start);
        u.read(offset)
    }

    fn usart_write(&mut self, n: u8, address: u64, value: u32) {
        let now = self.arena.simu_time;
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

    fn spi_write(&mut self, n: u8, address: u64, value: u32) {
        let now = self.arena.simu_time;
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

    fn twi_write(&mut self, n: u8, address: u64, value: u32) {
        let now = self.arena.simu_time;
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

    fn set_interrupt(&mut self, number: u8, level: u8) {
        self.arena.irq_number = u64::from(number);
        self.arena.irq_level = u64::from(level);
        self.push_live();
    }

    fn tick_peripherals(&mut self, now: u64) {
        let has_send = crate::serial::has_pending_send();
        for i in 0..self.usarts.len() {
            let tx = self.usarts[i].tx;
            let rx = self.usarts[i].rx;
            let mut irq_to_set = None;

            if has_send {
                let port_id = self
                    .cached_usart_port_ids
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let mut pending_rx = crate::serial::take_send(port_id);
                if i == 0 {
                    pending_rx.extend(crate::serial::take_send(&self.uid));
                    pending_rx.extend(crate::serial::take_send("default"));
                }
                for b in pending_rx {
                    crate::serial::publish_in(port_id, b);
                    if i == 0 {
                        crate::serial::publish_in(&self.uid, b);
                        crate::serial::publish_in("default", b);
                    }
                    if let Some((irq, level)) = self.usarts[i].on_byte_received(b) {
                        irq_to_set = Some((irq, level));
                    }
                }
            }

            if let Some((irq, level)) = irq_to_set.take() {
                self.set_interrupt(irq, level);
            }

            self.usarts[i].module.rx_pin_changed(now, &self.pins, rx);
            let tick = self.usarts[i].module.tick(now, &mut self.pins, tx, rx);
            if let Some(b) = tick.frame_sent {
                let port_id = self
                    .cached_usart_port_ids
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                crate::serial::publish_out(port_id, b);
                if i == 0 {
                    crate::serial::publish_out(&self.uid, b);
                    crate::serial::publish_out("default", b);
                }
                if let Some((irq, level)) = self.usarts[i].on_frame_sent(now, &mut self.pins) {
                    irq_to_set = Some((irq, level));
                }
                self.pins_dirty = true;
            }
            if let Some((irq, level)) = irq_to_set.take() {
                self.set_interrupt(irq, level);
            }
            if let Some(b) = tick.byte_received {
                let port_id = self
                    .cached_usart_port_ids
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                crate::serial::publish_in(port_id, b);
                if i == 0 {
                    crate::serial::publish_in(&self.uid, b);
                    crate::serial::publish_in("default", b);
                }
                if let Some((irq, level)) = self.usarts[i].on_byte_received(b) {
                    irq_to_set = Some((irq, level));
                }
                self.pins_dirty = true;
            }
            if let Some((irq, level)) = irq_to_set {
                self.set_interrupt(irq, level);
            }
        }
        for i in 0..self.spis.len() {
            let map = self.spis[i].pins;
            if self.spis[i].module.due.is_some_and(|t| t <= now)
                && self.spis[i].module.run_event(now, &mut self.pins, map)
            {
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
                self.pins_dirty = true;
            }
        }
        for i in 0..self.twis.len() {
            let map = self.twis[i].pins;
            if self.twis[i].module.due.is_some_and(|t| t <= now) {
                let tick = self.twis[i].module.run_event(now, &mut self.pins, map);
                let port_id = self
                    .cached_twi_port_ids
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if let Some(b) = tick.byte_sent {
                    crate::serial::publish_out(port_id, b);
                }
                if let Some(b) = tick.byte_received {
                    crate::serial::publish_in(port_id, b);
                }
                if let Some(st) = tick.state {
                    self.twis[i].on_state(st, now, &mut self.pins);
                }
                self.pins_dirty = true;
            }
        }
    }

    fn esp32_read(&mut self, address: u64) -> u32 {
        let offset = address.saturating_sub(ESP32_GPIO_START);
        match offset {
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
            GPIO_ENABLE => self.set_gpio_dir(value),
            GPIO_ENABLE_W1TS => self.set_gpio_dir(self.esp32_gpio.enable | value),
            GPIO_ENABLE_W1TC => self.set_gpio_dir(self.esp32_gpio.enable & !value),
            _ => {}
        }
        self.write_mem(address, value);
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
            for i in 33..40 {
                if let Some(idx) = self.esp32_gpio.pad[i] {
                    if self.pin_inp(idx) {
                        data |= 1 << (i - 33);
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
                } else {
                    self.pins[idx].set_pin_mode(PinMode::Input);
                }
            }
            changed &= !mask;
        }
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
        args.push("-M".into());
        args.push("stm32-f10xx".into());
        args.push("-drive".into());
        args.push(format!("file={firmware},if=pflash,format=raw"));
        args.push("-icount".into());
        args.push("shift=0,align=off,sleep=off".into());
        args
    }
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
    find_data_path("esp32/rom/bin")
        .or_else(|| find_data_path("esp32"))
        .or_else(|| {
            for dir in cs_qemu::qemu_search_dirs() {
                if dir.join("esp32-v3-rom.bin").is_file() {
                    return Some(dir);
                }
            }
            None
        })
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
    }

    #[test]
    fn stm32_args_start_with_shm_key() {
        let q = QemuComp::stm32("STM32-1", 2);
        let args = q.stm32_args("/tmp/fw.bin", "/pidSTM32-1");
        assert_eq!(args[0], "/pidSTM32-1");
        assert_eq!(args[1], "qemu-system-arm");
        assert!(args.contains(&"stm32-f10xx".to_string()));
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

    static TEST_SERIAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn esp32_serial_monitor_publish_and_receive() {
        let _guard = TEST_SERIAL_LOCK.lock().unwrap();
        crate::serial::clear("Esp32-1:USART1");
        crate::serial::clear("default");
        let mut q = QemuComp::esp32("Esp32-1");
        q.stamp_init(0);
        q.post(SimAction::Write, ESP32_UART0_START, b'H' as u64, 1);
        q.process_pending();

        let mut now = 1;
        while let Some(delay) = q.next_event_delay(now) {
            now += delay;
            q.run_event(&|_| 0.0, now);
            if !crate::serial::out_text("Esp32-1:USART1").is_empty() {
                break;
            }
        }

        assert_eq!(crate::serial::out_text("Esp32-1:USART1"), "H");
        assert_eq!(crate::serial::out_text("default"), "H");

        // Host UI sends character 'X'
        crate::serial::send_text("Esp32-1:USART1", "X");
        q.run_event(&|_| 0.0, now + 1);

        assert_eq!(crate::serial::in_text("Esp32-1:USART1"), "X");
        assert_eq!(q.usarts[0].rx_fifo.pop_front(), Some(b'X'));
    }

    #[test]
    fn stm32_serial_monitor_publish_and_receive() {
        let _guard = TEST_SERIAL_LOCK.lock().unwrap();
        crate::serial::clear("STM32-1:USART1");
        crate::serial::clear("default");
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
            if !crate::serial::out_text("STM32-1:USART1").is_empty() {
                break;
            }
        }

        assert_eq!(crate::serial::out_text("STM32-1:USART1"), "A");
        assert_eq!(crate::serial::out_text("default"), "A");

        // Host UI sends character 'Y'
        crate::serial::send_text("STM32-1:USART1", "Y");
        q.run_event(&|_| 0.0, now + 1);

        assert_eq!(crate::serial::in_text("STM32-1:USART1"), "Y");
        // Read USART_DR
        q.post(SimAction::Read, STM32_USART1_START + 0x04, 0, now + 2);
        q.process_pending();
        assert_eq!(q.arena.reg_data & 0xFF, b'Y' as u64);
    }
}
