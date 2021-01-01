//! Standalone MCU pin as a digital `IoPin`. A full MCU uses `Kind::Mcu`
//! (`cs-mcu::Device`) with one `IoPin` per GPIO.

use super::pin::{IoPin, PinAction, PinMode};

#[derive(Clone, Debug)]
pub struct McuPinState {
    pub pin: IoPin,
}

impl McuPinState {
    pub fn new(id: &str) -> Self {
        // Default input, like a reset MCU pin before DDR is written.
        let mut pin = IoPin::input(format!("{id}-pin"));
        pin.out_high_v = 5.0;
        pin.out_low_v = 0.0;
        Self { pin }
    }

    pub fn pin_ids(&self) -> Vec<String> {
        vec![self.pin.id.clone()]
    }

    pub fn stamp_init(&mut self, slope_steps: i32) {
        self.pin.initialize(slope_steps);
        match self.pin.mode {
            PinMode::Output | PinMode::OpenCo | PinMode::Source => {
                self.pin.set_out_state(self.pin.get_out_state());
            }
            _ => {}
        }
    }

    /// MCU core writes the pin (output / open-collector).
    pub fn drive(&mut self, high: bool) -> PinAction {
        if matches!(self.pin.mode, PinMode::Input | PinMode::Undef) {
            self.pin.set_pin_mode(PinMode::Output);
        }
        self.pin.schedule_state(high, 0)
    }

    /// MCU core reads the analog net as a logic level.
    pub fn read(&mut self, volt: f64) -> bool {
        self.pin.get_inp_state(volt)
    }

    pub fn set_mode(&mut self, mode: PinMode) {
        self.pin.set_pin_mode(mode);
    }
}
