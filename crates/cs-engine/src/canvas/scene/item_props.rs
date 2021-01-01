//! Circuit scene item property accessors and reflection groups.

use super::item::Item;
use crate::components::*;

pub use crate::components::{PropGroup, PropRow, prop_group};

impl Item {
    pub fn resistance(&self) -> f64 {
        match &self.kind {
            Part::Resistor(p) => p.resistance,
            Part::Led(p) => p.resistance,
            Part::Battery(p) => p.resistance,
            Part::Potentiometer(p) => p.resistance,
            Part::VarResistor(p) => p.resistance,
            Part::ResistorDip(p) => p.resistance,
            Part::Ldr(p) => p.r_light,
            Part::Thermistor(p) => p.r0,
            Part::Rtd(p) => p.r0,
            Part::Strain(p) => p.r0,
            _ => 0.0,
        }
    }

    pub fn voltage(&self) -> f64 {
        match &self.kind {
            Part::Battery(p) => p.voltage,
            Part::FixedVolt(p) => p.voltage,
            Part::Clock(p) => p.voltage,
            Part::Rail(p) => p.voltage,
            Part::Lamp(p) => p.voltage,
            Part::VoltSource(p) => p.value,
            _ => 0.0,
        }
    }

    pub fn capacitance(&self) -> f64 {
        match &self.kind {
            Part::Capacitor(p) => p.capacitance,
            Part::ElCapacitor(p) => p.capacitance,
            Part::VarCapacitor(p) => p.capacitance,
            _ => 0.0,
        }
    }

    pub fn inductance(&self) -> f64 {
        match &self.kind {
            Part::Inductor(p) => p.inductance,
            Part::VarInductor(p) => p.inductance,
            Part::Transformer(p) => p.inductance1,
            Part::Relay(p) => p.inductance,
            _ => 0.0,
        }
    }

    pub fn closed(&self) -> bool {
        match &self.kind {
            Part::Switch(p) => p.checked,
            Part::Push(p) => p.pressed,
            _ => false,
        }
    }

    pub fn pnp(&self) -> bool {
        matches!(&self.kind, Part::Bjt(p) if p.pnp)
    }

    pub fn p_channel(&self) -> bool {
        matches!(&self.kind, Part::Mosfet(p) if p.p_channel)
    }

    pub fn depletion(&self) -> bool {
        matches!(&self.kind, Part::Mosfet(p) if p.depletion)
    }

    pub fn pkg_w(&self) -> i32 {
        self.kind.package().map(|p| p.width).unwrap_or(0)
    }

    pub fn pkg_h(&self) -> i32 {
        self.kind.package().map(|p| p.height).unwrap_or(0)
    }

    pub fn logic_symbol(&self) -> bool {
        match &self.kind {
            Part::Subcircuit(p) => p.logic_symbol,
            Part::Mcu(p) => p.logic_symbol,
            Part::QemuDevice(p) => p.logic_symbol,
            Part::SubPackage(p) => p.package.logic_symbol,
            _ => false,
        }
    }

    pub fn custom_color(&self) -> bool {
        match &self.kind {
            Part::SubPackage(p) => p.package.custom_color,
            _ => false,
        }
    }

    pub fn bckgnd_color(&self) -> &str {
        self.kind
            .package()
            .map(|p| p.bckgndcolor.as_str())
            .unwrap_or("")
    }

    pub fn pkg_name(&self) -> &str {
        self.kind.package().map(|p| p.name.as_str()).unwrap_or("")
    }

    pub fn subc_type(&self) -> &str {
        self.kind
            .package()
            .map(|p| p.subc_type.as_str())
            .unwrap_or("")
    }

    pub fn gate_kind(&self) -> &str {
        match &self.kind {
            Part::AndGate(_) => "And",
            Part::OrGate(_) => "Or",
            Part::XorGate(_) => "Xor",
            Part::NotGate(_) => "Not",
            Part::BufferGate(_) => "Buffer",
            _ => "",
        }
    }

    pub fn num_inputs(&self) -> usize {
        match &self.kind {
            Part::AndGate(p) => p.num_inputs,
            Part::OrGate(p) => p.num_inputs,
            Part::XorGate(p) => p.num_inputs,
            Part::BufferGate(_) | Part::NotGate(_) => 1,
            Part::AnalogMux(p) => p.channels,
            _ => 0,
        }
    }

    pub fn invert_inputs(&self) -> bool {
        match &self.kind {
            Part::AndGate(p) => p.invert_inputs,
            Part::OrGate(p) => p.invert_inputs,
            Part::XorGate(p) => p.invert_inputs,
            Part::BufferGate(p) => p.invert_inputs,
            Part::NotGate(p) => p.invert_inputs,
            Part::Latch(p) => p.invert_inputs,
            _ => false,
        }
    }

    pub fn invert_output(&self) -> bool {
        match &self.kind {
            Part::AndGate(p) => p.invert_output,
            Part::OrGate(p) => p.invert_output,
            Part::XorGate(p) => p.invert_output,
            Part::BufferGate(p) => p.invert_output,
            Part::NotGate(p) => p.invert_output,
            Part::Comparator(p) => p.inverted,
            _ => false,
        }
    }

    pub fn init_high(&self) -> bool {
        match &self.kind {
            Part::AndGate(p) => p.init_high,
            Part::OrGate(p) => p.init_high,
            Part::XorGate(p) => p.init_high,
            Part::BufferGate(p) => p.init_high,
            Part::NotGate(p) => p.init_high,
            _ => false,
        }
    }

    pub fn tristate(&self) -> bool {
        match &self.kind {
            Part::AndGate(p) => p.tristate,
            Part::OrGate(p) => p.tristate,
            Part::XorGate(p) => p.tristate,
            Part::BufferGate(p) => p.tristate,
            Part::NotGate(p) => p.tristate,
            Part::Latch(p) => p.tristate,
            _ => false,
        }
    }

    pub fn small(&self) -> bool {
        match &self.kind {
            Part::AndGate(p) => p.small,
            Part::OrGate(p) => p.small,
            Part::XorGate(p) => p.small,
            Part::BufferGate(p) => p.small,
            Part::NotGate(p) => p.small,
            Part::Probe(p) => p.small,
            Part::Clock(p) => p.small,
            _ => false,
        }
    }

    pub fn ff_kind(&self) -> &str {
        match &self.kind {
            Part::FlipFlop(p) => p.ff_kind.as_str(),
            _ => "",
        }
    }

    pub fn use_rs(&self) -> bool {
        match &self.kind {
            Part::FlipFlop(p) => p.use_rs,
            _ => false,
        }
    }

    pub fn trigger(&self) -> &str {
        match &self.kind {
            Part::FlipFlop(p) => p.trigger.as_str(),
            Part::Latch(p) => p.trigger.as_str(),
            _ => "",
        }
    }

    pub fn channels(&self) -> usize {
        match &self.kind {
            Part::Latch(p) => p.channels,
            Part::AnalogMux(p) => p.channels,
            _ => 0,
        }
    }

    pub fn use_reset(&self) -> bool {
        match &self.kind {
            Part::Latch(p) => p.use_reset,
            _ => false,
        }
    }

    pub fn test_inputs(&self) -> &str {
        match &self.kind {
            Part::TestUnit(p) => p.inputs.as_str(),
            _ => "",
        }
    }

    pub fn test_outputs(&self) -> &str {
        match &self.kind {
            Part::TestUnit(p) => p.outputs.as_str(),
            _ => "",
        }
    }

    pub fn period(&self) -> f64 {
        match &self.kind {
            Part::TestUnit(p) => p.period,
            _ => 0.0,
        }
    }

    pub fn freq_khz(&self) -> f64 {
        match &self.kind {
            Part::Clock(p) => p.frequency / 1000.0,
            Part::Ssd1306(p) => p.freq_khz,
            _ => 0.0,
        }
    }

    pub fn wave_type(&self) -> &str {
        match &self.kind {
            Part::WaveGen(p) => p.wave_type.as_str(),
            _ => "",
        }
    }

    pub fn freq_hz(&self) -> f64 {
        match &self.kind {
            Part::WaveGen(p) => p.freq_hz,
            _ => 0.0,
        }
    }

    pub fn amplitude(&self) -> f64 {
        match &self.kind {
            Part::WaveGen(p) => p.amplitude,
            _ => 0.0,
        }
    }

    pub fn offset(&self) -> f64 {
        match &self.kind {
            Part::WaveGen(p) => p.offset,
            _ => 0.0,
        }
    }

    pub fn duty(&self) -> f64 {
        match &self.kind {
            Part::WaveGen(p) => p.duty,
            _ => 0.0,
        }
    }

    pub fn source_value(&self) -> f64 {
        match &self.kind {
            Part::VoltSource(p) => p.value,
            Part::CurrSource(p) => p.value,
            Part::Dial(p) => p.value,
            Part::Potentiometer(p) => p.resistance * p.wiper,
            Part::VarResistor(p) => p.resistance,
            Part::VarCapacitor(p) => p.capacitance,
            Part::VarInductor(p) => p.inductance,
            _ => 0.0,
        }
    }

    pub fn min_value(&self) -> f64 {
        match &self.kind {
            Part::VoltSource(p) => p.min_value,
            Part::CurrSource(p) => p.min_value,
            Part::Dial(p) => p.min_val,
            Part::VarResistor(p) => p.min_r,
            Part::VarCapacitor(p) => p.min_c,
            Part::VarInductor(p) => p.min_l,
            _ => 0.0,
        }
    }

    pub fn max_value(&self) -> f64 {
        match &self.kind {
            Part::VoltSource(p) => p.max_value,
            Part::CurrSource(p) => p.max_value,
            Part::Dial(p) => p.max_val,
            Part::VarResistor(p) => p.max_r,
            Part::VarCapacitor(p) => p.max_c,
            Part::VarInductor(p) => p.max_l,
            _ => 0.0,
        }
    }

    pub fn source_running(&self) -> bool {
        match &self.kind {
            Part::VoltSource(p) => p.running,
            Part::CurrSource(p) => p.running,
            _ => false,
        }
    }

    pub fn control_pins(&self) -> bool {
        match &self.kind {
            Part::Csource(p) => p.control_pins,
            _ => false,
        }
    }

    pub fn is_curr_source(&self) -> bool {
        match &self.kind {
            Part::Csource(p) => p.curr_source,
            _ => false,
        }
    }

    pub fn curr_control(&self) -> bool {
        match &self.kind {
            Part::Csource(p) => p.curr_control,
            _ => false,
        }
    }

    pub fn gain(&self) -> f64 {
        match &self.kind {
            Part::Csource(p) => p.gain,
            Part::OpAmp(p) => p.gain,
            _ => 0.0,
        }
    }

    pub fn norm_close(&self) -> bool {
        match &self.kind {
            Part::Push(p) => p.norm_close,
            Part::Switch(p) => p.norm_close,
            Part::Relay(p) => p.norm_close,
            _ => false,
        }
    }

    pub fn poles(&self) -> usize {
        match &self.kind {
            Part::Push(p) => p.poles,
            Part::Switch(p) => p.poles,
            Part::Relay(p) => p.poles,
            _ => 1,
        }
    }

    pub fn key(&self) -> &str {
        match &self.kind {
            Part::Push(p) => p.key.as_str(),
            Part::Switch(p) => p.key.as_str(),
            Part::KeyPad(p) => p.key_labels_str(),
            Part::Potentiometer(p) => p.dial.key.as_str(),
            Part::VarResistor(p) => p.dial.key.as_str(),
            _ => "",
        }
    }

    pub fn show_button(&self) -> bool {
        match &self.kind {
            Part::Push(p) => p.show_button,
            Part::Switch(p) => p.show_button,
            _ => false,
        }
    }

    pub fn pressed(&self) -> bool {
        match &self.kind {
            Part::Push(p) => p.pressed,
            _ => false,
        }
    }

    pub fn size(&self) -> usize {
        match &self.kind {
            Part::SevenSegment(p) => p.num_displays,
            Part::SwitchDip(p) => p.size,
            Part::ResistorDip(p) => p.size,
            Part::LedBar(p) => p.segments,
            _ => 0,
        }
    }

    pub fn state(&self) -> u32 {
        match &self.kind {
            Part::SwitchDip(p) => p.state,
            _ => 0,
        }
    }

    pub fn exclusive(&self) -> bool {
        match &self.kind {
            Part::SwitchDip(p) => p.exclusive,
            _ => false,
        }
    }

    pub fn common_pin(&self) -> bool {
        match &self.kind {
            Part::SwitchDip(p) => p.common_pin,
            _ => false,
        }
    }

    pub fn double_throw(&self) -> bool {
        match &self.kind {
            Part::Switch(p) => p.double_throw,
            Part::Relay(p) => p.double_throw,
            _ => false,
        }
    }

    pub fn active(&self) -> bool {
        match &self.kind {
            Part::Relay(p) => p.active,
            _ => false,
        }
    }

    pub fn rows(&self) -> usize {
        match &self.kind {
            Part::KeyPad(p) => p.rows,
            Part::LedMatrix(p) => p.rows,
            Part::Hd44780(p) => p.rows,
            Part::Ws2812(p) => p.rows,
            _ => 0,
        }
    }

    pub fn cols(&self) -> usize {
        match &self.kind {
            Part::KeyPad(p) => p.cols,
            Part::LedMatrix(p) => p.cols,
            Part::Hd44780(p) => p.cols,
            Part::Ws2812(p) => p.cols,
            _ => 0,
        }
    }

    pub fn wiper(&self) -> f64 {
        self.kind.wiper()
    }

    pub fn min_r(&self) -> f64 {
        match &self.kind {
            Part::VarResistor(p) => p.min_r,
            _ => 0.0,
        }
    }

    pub fn max_r(&self) -> f64 {
        match &self.kind {
            Part::VarResistor(p) => p.max_r,
            _ => 0.0,
        }
    }

    pub fn bussed(&self) -> bool {
        match &self.kind {
            Part::ResistorDip(p) => p.bussed,
            _ => false,
        }
    }

    pub fn lux(&self) -> f64 {
        match &self.kind {
            Part::Ldr(p) => p.lux,
            _ => 0.0,
        }
    }

    pub fn r_dark(&self) -> f64 {
        match &self.kind {
            Part::Ldr(p) => p.r_dark,
            _ => 1e6,
        }
    }

    pub fn r_light(&self) -> f64 {
        match &self.kind {
            Part::Ldr(p) => p.r_light,
            _ => 1000.0,
        }
    }

    pub fn temp_c(&self) -> f64 {
        match &self.kind {
            Part::Thermistor(p) => p.temp_c,
            Part::Rtd(p) => p.temp_c,
            _ => 25.0,
        }
    }

    pub fn r0(&self) -> f64 {
        match &self.kind {
            Part::Thermistor(p) => p.r0,
            Part::Rtd(p) => p.r0,
            Part::Strain(p) => p.r0,
            _ => 1000.0,
        }
    }

    pub fn beta(&self) -> f64 {
        match &self.kind {
            Part::Thermistor(p) => p.beta,
            _ => 3950.0,
        }
    }

    pub fn t0_c(&self) -> f64 {
        match &self.kind {
            Part::Thermistor(p) => p.t0_c,
            _ => 25.0,
        }
    }

    pub fn alpha(&self) -> f64 {
        match &self.kind {
            Part::Rtd(p) => p.alpha,
            _ => 0.00385,
        }
    }

    pub fn strain_val(&self) -> f64 {
        match &self.kind {
            Part::Strain(p) => p.strain,
            _ => 0.0,
        }
    }

    pub fn gauge_factor(&self) -> f64 {
        match &self.kind {
            Part::Strain(p) => p.gauge_factor,
            _ => 2.0,
        }
    }

    pub fn min_c(&self) -> f64 {
        match &self.kind {
            Part::VarCapacitor(p) => p.min_c,
            _ => 0.0,
        }
    }

    pub fn max_c(&self) -> f64 {
        match &self.kind {
            Part::VarCapacitor(p) => p.max_c,
            _ => 0.0,
        }
    }

    pub fn min_l(&self) -> f64 {
        match &self.kind {
            Part::VarInductor(p) => p.min_l,
            _ => 0.0,
        }
    }

    pub fn max_l(&self) -> f64 {
        match &self.kind {
            Part::VarInductor(p) => p.max_l,
            _ => 0.0,
        }
    }

    pub fn inductance1(&self) -> f64 {
        match &self.kind {
            Part::Transformer(p) => p.inductance1,
            _ => 0.0,
        }
    }

    pub fn inductance2(&self) -> f64 {
        match &self.kind {
            Part::Transformer(p) => p.inductance2,
            _ => 0.0,
        }
    }

    pub fn coupling(&self) -> f64 {
        match &self.kind {
            Part::Transformer(p) => p.coupling,
            _ => 0.0,
        }
    }

    pub fn v_gate_th(&self) -> f64 {
        match &self.kind {
            Part::Scr(p) => p.v_gate_th,
            Part::Triac(p) => p.v_gate_th,
            _ => 0.7,
        }
    }

    pub fn i_hold(&self) -> f64 {
        match &self.kind {
            Part::Scr(p) => p.i_hold,
            Part::Triac(p) => p.i_hold,
            _ => 0.01,
        }
    }

    pub fn v_breakover(&self) -> f64 {
        match &self.kind {
            Part::Diac(p) => p.v_breakover,
            _ => 30.0,
        }
    }

    pub fn on_resistance(&self) -> f64 {
        match &self.kind {
            Part::AnalogMux(p) => p.on_resistance,
            _ => 50.0,
        }
    }

    pub fn transparent(&self) -> bool {
        match &self.kind {
            Part::TouchPad(p) => p.transparent,
            _ => false,
        }
    }

    pub fn rx_min(&self) -> f64 {
        match &self.kind {
            Part::TouchPad(p) => p.rx_min,
            _ => 100.0,
        }
    }

    pub fn rx_max(&self) -> f64 {
        match &self.kind {
            Part::TouchPad(p) => p.rx_max,
            _ => 500.0,
        }
    }

    pub fn ry_min(&self) -> f64 {
        match &self.kind {
            Part::TouchPad(p) => p.ry_min,
            _ => 100.0,
        }
    }

    pub fn ry_max(&self) -> f64 {
        match &self.kind {
            Part::TouchPad(p) => p.ry_max,
            _ => 500.0,
        }
    }

    pub fn x_pos(&self) -> i32 {
        match &self.kind {
            Part::TouchPad(p) => p.x_pos,
            _ => -1,
        }
    }

    pub fn y_pos(&self) -> i32 {
        match &self.kind {
            Part::TouchPad(p) => p.y_pos,
            _ => -1,
        }
    }

    pub fn width_px(&self) -> i32 {
        match &self.kind {
            Part::TouchPad(p) => p.width,
            _ => 0,
        }
    }

    pub fn height_px(&self) -> i32 {
        match &self.kind {
            Part::TouchPad(p) => p.height,
            _ => 0,
        }
    }

    pub fn stick_x(&self) -> f64 {
        match &self.kind {
            Part::KY023(p) => p.stick_x,
            _ => 0.0,
        }
    }

    pub fn stick_y(&self) -> f64 {
        match &self.kind {
            Part::KY023(p) => p.stick_y,
            _ => 0.0,
        }
    }

    pub fn btn_down(&self) -> bool {
        match &self.kind {
            Part::KY023(p) => p.btn_down,
            _ => false,
        }
    }

    pub fn dial_val(&self) -> i32 {
        match &self.kind {
            Part::KY040(p) => p.dial_val,
            _ => 1,
        }
    }

    pub fn btn_closed(&self) -> bool {
        match &self.kind {
            Part::KY040(p) => p.btn_closed,
            _ => false,
        }
    }

    pub fn distance(&self) -> f64 {
        match &self.kind {
            Part::SR04(p) => p.distance,
            _ => 0.0,
        }
    }

    pub fn use_slider(&self) -> bool {
        match &self.kind {
            Part::SR04(p) => p.use_slider,
            _ => false,
        }
    }

    pub fn model(&self) -> &str {
        match &self.kind {
            Part::DHT22(p) => p.model.as_str(),
            _ => "",
        }
    }

    pub fn temp(&self) -> f64 {
        match &self.kind {
            Part::DHT22(p) => p.temp,
            Part::DS18B20(p) => p.temp,
            Part::DS1621(p) => p.temp,
            _ => 25.0,
        }
    }

    pub fn humi(&self) -> f64 {
        match &self.kind {
            Part::DHT22(p) => p.humi,
            _ => 50.0,
        }
    }

    pub fn temp_inc(&self) -> f64 {
        match &self.kind {
            Part::DHT22(p) => p.temp_inc,
            Part::DS18B20(p) => p.temp_inc,
            Part::DS1621(p) => p.temp_inc,
            _ => 0.5,
        }
    }

    pub fn humi_inc(&self) -> f64 {
        match &self.kind {
            Part::DHT22(p) => p.humi_inc,
            _ => 5.0,
        }
    }

    pub fn rom(&self) -> &str {
        match &self.kind {
            Part::DS18B20(p) => p.rom.as_str(),
            _ => "",
        }
    }

    pub fn time_updated(&self) -> bool {
        match &self.kind {
            Part::DS1307(p) => p.time_updated,
            _ => true,
        }
    }

    pub fn rpm_nominal(&self) -> i32 {
        match &self.kind {
            Part::DcMotor(p) => p.rpm_nominal,
            _ => 60,
        }
    }

    pub fn volt_nominal(&self) -> f64 {
        match &self.kind {
            Part::DcMotor(p) => p.volt_nominal,
            _ => 5.0,
        }
    }

    pub fn speed(&self) -> f64 {
        match &self.kind {
            Part::DcMotor(p) => p.speed,
            Part::Servo(p) => p.speed,
            _ => 0.0,
        }
    }

    pub fn angle(&self) -> f64 {
        match &self.kind {
            Part::DcMotor(p) => p.angle,
            Part::Stepper(p) => p.angle,
            _ => 0.0,
        }
    }

    pub fn bipolar(&self) -> bool {
        match &self.kind {
            Part::Stepper(p) => p.bipolar,
            _ => false,
        }
    }

    pub fn min_pulse(&self) -> f64 {
        match &self.kind {
            Part::Servo(p) => p.min_pulse,
            _ => 1000.0,
        }
    }

    pub fn max_pulse(&self) -> f64 {
        match &self.kind {
            Part::Servo(p) => p.max_pulse,
            _ => 2000.0,
        }
    }

    pub fn pos(&self) -> f64 {
        match &self.kind {
            Part::Servo(p) => p.pos,
            _ => 90.0,
        }
    }

    pub fn human_name(&self) -> &str {
        self.kind.human_name()
    }

    pub fn file(&self) -> &str {
        match &self.kind {
            Part::SdCard(p) => p.file.as_str(),
            _ => "",
        }
    }

    pub fn debug(&self) -> bool {
        match &self.kind {
            Part::Esp01(p) => p.debug,
            _ => false,
        }
    }

    pub fn common_anode(&self) -> bool {
        match &self.kind {
            Part::RgbLed(p) => p.common_anode,
            Part::SevenSegment(p) => p.common_anode,
            _ => false,
        }
    }

    pub fn color_str(&self) -> &str {
        match &self.kind {
            Part::Led(p) => p.color.as_str(),
            Part::LedBar(p) => p.color.as_str(),
            Part::Shape(p) => p.color.as_str(),
            Part::Ssd1306(p) => p.color.as_str(),
            _ => "",
        }
    }

    pub fn disp_width(&self) -> usize {
        self.kind.disp_width()
    }

    pub fn disp_height(&self) -> usize {
        self.kind.disp_height()
    }

    pub fn controller(&self) -> &str {
        match &self.kind {
            Part::TftDisplay(p) => p.controller.as_str(),
            _ => "",
        }
    }

    pub fn contrast(&self) -> u8 {
        match &self.kind {
            Part::Pcd8544(p) => p.contrast,
            _ => 50,
        }
    }

    pub fn bias(&self) -> u8 {
        match &self.kind {
            Part::Pcd8544(p) => p.bias,
            _ => 4,
        }
    }

    pub fn disp_rotate(&self) -> bool {
        match &self.kind {
            Part::Ssd1306(p) => p.rotate,
            _ => true,
        }
    }

    pub fn control_code(&self) -> u8 {
        match &self.kind {
            Part::Ssd1306(p) => p.control_code,
            _ => 60,
        }
    }

    pub fn threshold(&self) -> f64 {
        match &self.kind {
            Part::Led(p) => p.threshold,
            Part::Probe(p) => p.threshold,
            _ => 0.0,
        }
    }

    pub fn max_current(&self) -> f64 {
        match &self.kind {
            Part::Led(p) => p.max_current,
            _ => 0.03,
        }
    }

    pub fn grounded(&self) -> bool {
        match &self.kind {
            Part::Led(p) => p.grounded,
            _ => false,
        }
    }

    pub fn modules(&self) -> usize {
        match &self.kind {
            Part::SevenSegment(p) => p.num_displays,
            Part::Max72xx(p) => p.modules,
            _ => 1,
        }
    }

    pub fn count(&self) -> usize {
        match &self.kind {
            Part::Ws2812(p) => p.count,
            _ => 1,
        }
    }

    pub fn volume(&self) -> f64 {
        match &self.kind {
            Part::AudioOut(p) => p.volume,
            _ => 100.0,
        }
    }

    pub fn impedance(&self) -> f64 {
        match &self.kind {
            Part::AudioOut(p) => p.impedance,
            _ => 8.0,
        }
    }

    pub fn buzzer(&self) -> bool {
        match &self.kind {
            Part::AudioOut(p) => p.buzzer,
            _ => false,
        }
    }

    pub fn frequency(&self) -> f64 {
        match &self.kind {
            Part::AudioOut(p) => p.frequency,
            _ => 1000.0,
        }
    }

    pub fn power(&self) -> f64 {
        match &self.kind {
            Part::Lamp(p) => p.power,
            _ => 0.0,
        }
    }

    pub fn r_cold(&self) -> f64 {
        match &self.kind {
            Part::Lamp(p) => p.r_cold,
            _ => 1.0,
        }
    }

    pub fn tunnel_name(&self) -> &str {
        match &self.kind {
            Part::Tunnel(p) => p.name.as_str(),
            _ => "",
        }
    }

    pub fn ch_tunnels(&self) -> Vec<String> {
        self.kind.ch_tunnels()
    }

    pub fn is_bus(&self) -> bool {
        match &self.kind {
            Part::Tunnel(p) => p.is_bus,
            Part::Bus(_) => true,
            _ => false,
        }
    }

    pub fn bus_width(&self) -> usize {
        match &self.kind {
            Part::Bus(p) => p.width,
            _ => 8,
        }
    }

    pub fn pins_count(&self) -> usize {
        match &self.kind {
            Part::Socket(p) => p.pins_count,
            Part::Header(p) => p.pins_count,
            _ => 2,
        }
    }

    pub fn port_name(&self) -> &str {
        match &self.kind {
            Part::SerialPort(p) => p.port_name.as_str(),
            _ => "",
        }
    }

    pub fn baud_rate(&self) -> u32 {
        match &self.kind {
            Part::SerialPort(p) => p.baud_rate,
            Part::SerialTerm(p) => p.baud_rate,
            Part::Esp01(p) => p.baud_rate,
            _ => 9600,
        }
    }

    pub fn dial_step(&self) -> f64 {
        match &self.kind {
            Part::Dial(p) => p.step,
            _ => 1.0,
        }
    }

    pub fn shape_kind(&self) -> &str {
        match &self.kind {
            Part::Shape(p) => p.shape_kind.as_str(),
            _ => "",
        }
    }

    pub fn shape_width(&self) -> f64 {
        match &self.kind {
            Part::Shape(p) => p.width,
            _ => 0.0,
        }
    }

    pub fn shape_height(&self) -> f64 {
        match &self.kind {
            Part::Shape(p) => p.height,
            _ => 0.0,
        }
    }

    pub fn shape_text(&self) -> &str {
        match &self.kind {
            Part::Shape(p) => p.text.as_str(),
            _ => "",
        }
    }

    pub fn description(&self) -> &'static str {
        self.kind.description()
    }

    pub fn prop_rows(&self) -> Vec<PropRow> {
        self.kind.prop_rows()
    }

    pub fn prop_groups(&self) -> Vec<PropGroup> {
        self.kind.prop_groups()
    }

    pub fn prop_text(&self, name: &str) -> Option<String> {
        self.kind.get_prop_text(name)
    }

    pub fn prop_bool(&self, name: &str) -> Option<bool> {
        match self.kind.get_prop_text(name).as_deref() {
            Some("true" | "1") => Some(true),
            Some("false" | "0") => Some(false),
            _ => None,
        }
    }

    pub fn set_prop_text(&mut self, name: &str, text: &str) -> bool {
        self.kind.set_prop_text(name, text).is_ok()
    }

    pub fn set_prop_bool(&mut self, name: &str, v: bool) -> bool {
        self.set_prop_text(name, if v { "true" } else { "false" })
    }
}
