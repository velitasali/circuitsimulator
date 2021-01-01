//! Byte-for-byte `qemuArena_t` / `Esp32CsArena`.

/// Size of the shared-memory object (`sizeof(qemuArena_t)`).
pub const ARENA_SIZE: usize = 88;

/// Co-sim mailbox. Layout must never drift from
/// `src/microsim/cores/qemu/qemudevice.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arena {
    /// Circuit time in picoseconds. QEMU sets this *before* `simu_action`.
    /// Zero means "no event" on the consumer side.
    pub simu_time: u64,
    pub qemu_time: u64,
    pub reg_data: u64,
    /// IOMEM-relative address (absolute minus the device's MMIO base).
    pub reg_addr: u64,
    pub irq_number: u64,
    pub irq_level: u64,
    /// Doorbell. Must be the **last** field written when posting a request.
    pub simu_action: u64,
    /// Simulator response. Only `SimAction::Read` produces one.
    pub qemu_action: u64,
    /// QEMU sets this to 1 once attached and ready.
    pub running: u64,
    pub loop_timeout_ns: i64,
    pub ps_per_inst: f64,
}

impl Arena {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Post a request. `simu_action` is stored last so the consumer cannot
    /// observe a doorbell with torn payload.
    pub fn post(&mut self, action: SimAction, addr: u64, data: u64, time_ps: u64) {
        self.simu_time = if time_ps == 0 { 1 } else { time_ps };
        self.reg_addr = addr;
        self.reg_data = data;
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
        self.simu_action = action as u64;
    }

    pub fn take_simu_action(&mut self) -> SimAction {
        let a = SimAction::from_u64(self.simu_action);
        self.simu_action = 0;
        self.simu_time = 0;
        a
    }
}

/// `enum simuAction` in `qemudevice.h`.
#[repr(u64)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimAction {
    None = 0,
    Read = 1,
    Write = 2,
    Freq = 3,
    Interrupt = 4,
    I2c = 10,
    Spi = 11,
    Usart = 12,
    Timer = 13,
    GpioIn = 14,
    Event = 1 << 7,
}

impl SimAction {
    pub fn from_u64(v: u64) -> Self {
        match v {
            1 => Self::Read,
            2 => Self::Write,
            3 => Self::Freq,
            4 => Self::Interrupt,
            10 => Self::I2c,
            11 => Self::Spi,
            12 => Self::Usart,
            13 => Self::Timer,
            14 => Self::GpioIn,
            128 => Self::Event,
            _ => Self::None,
        }
    }
}
