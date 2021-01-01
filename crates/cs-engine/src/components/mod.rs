//! Per-type component ownership: traits, property table, change bus, circ1 1.0.0.
//! All components are represented by modular structs within [`Part`].

mod adc;
mod aip31068;
mod ammeter;
mod analog_mux;
mod and_gate;
mod audio_out;
mod battery;
mod bcd_to_7segment;
mod bcd_to_dec;
mod bin_counter;
mod bjt;
mod buffer_gate;
mod bus;
mod capacitor;
mod change;
mod clock;
mod comparator;
mod component;
mod counter;
mod csource;
mod curr_source;
mod dac;
mod dc_motor;
mod dec_to_bcd;
mod demux;
mod dht22;
mod diac;
mod dial;
mod diode;
pub(crate) mod drawable;
mod ds1307;
mod ds1621;
mod ds18b20;
mod dynamic_memory;
mod el_capacitor;
mod esp01;
mod fixed_volt;
mod flip_flop;
mod freq_meter;
mod full_adder;
mod function;
mod ground;
mod half_adder;
mod hd44780;
mod header;
mod i2c_ram;
mod i2c_to_parallel;
mod inductor;
mod jfet;
mod keypad;
mod ks0108;
mod ky023;
mod ky040;
mod lamp;
mod latch;
mod ldr;
mod led;
mod led_bar;
mod led_matrix;
mod lm555;
mod logic_analyzer;
mod magnitude_comp;
mod max72xx;
mod mcu;
mod memory;
mod mosfet;
mod mux;
mod node;
mod not_gate;
mod opamp;
mod or_gate;
mod oscope;
mod pcd8544;
mod pcf8833;
mod potentiometer;
mod probe;
pub(crate) mod props;
mod push;
mod qemu_device;
mod rail;
mod relay;
mod resistor;
mod resistor_dip;
mod rgb_led;
mod rtd;
mod scr;
mod sd_card;
pub mod ser;
mod serial_port;
mod serial_term;
mod servo;
mod seven_segment;
mod seven_segment_bcd;
mod sh1107;
pub mod shape;
mod shift_reg;
mod socket;
mod sr04;
mod ssd1306;
mod stepper;
mod strain;
mod sub_package;
mod subcircuit;
mod switch;
mod switch_dip;
mod test_unit;
mod tft_display;
mod thermistor;
mod touchpad;
mod transformer;
mod triac;
mod tunnel;
mod var_capacitor;
mod var_inductor;
mod var_resistor;
mod volt_reg;
mod volt_source;
mod voltmeter;
mod wave_gen;
mod ws2812;
mod xor_gate;

use crate::canvas::Point;

pub use adc::Adc;
pub use aip31068::Aip31068;
pub use ammeter::Ammeter;
pub use analog_mux::AnalogMux;
pub use and_gate::AndGate;
pub use audio_out::AudioOut;
pub use battery::{BATTERY_BODY, Battery};
pub use bcd_to_7segment::BcdTo7Segment;
pub use bcd_to_dec::BcdToDec;
pub use bin_counter::BinCounter;
pub use bjt::Bjt;
pub use buffer_gate::BufferGate;
pub use bus::Bus;
pub use capacitor::Capacitor;
pub use change::{ComponentChange, ViewUpdate};
pub use clock::Clock;
pub use comparator::Comparator;
pub use component::{
    CompPin, Component, DialState, Dialed, PinDirection, PropGroup, PropRow, Stampable,
    TwoTerminal, group_rows_by, prop_group,
};
pub use counter::Counter;
pub use csource::Csource;
pub use curr_source::CurrSource;
pub use dac::Dac;
pub use dc_motor::DcMotor;
pub use dec_to_bcd::DecToBcd;
pub use demux::Demux;
pub use dht22::{DHT_MODEL_OPTIONS, DHT22, DhtModel};
pub use diac::Diac;
pub use dial::Dial;
pub use diode::Diode;
pub use drawable::Drawable;
pub use ds18b20::DS18B20;
pub use ds1307::DS1307;
pub use ds1621::DS1621;
pub use dynamic_memory::DynamicMemory;
pub use el_capacitor::ElCapacitor;
pub use esp01::Esp01;
pub use fixed_volt::{FIXED_VOLT_BODY, FixedVolt};
pub use flip_flop::FlipFlop;
pub use freq_meter::FreqMeter;
pub use full_adder::FullAdder;
pub use function::Function;
pub use ground::{GROUND_BODY, Ground};
pub use half_adder::HalfAdder;
pub use hd44780::Hd44780;
pub use header::Header;
pub use i2c_ram::I2CRam;
pub use i2c_to_parallel::I2CToParallel;
pub use inductor::Inductor;
pub use jfet::Jfet;
pub use keypad::KeyPad;
pub use ks0108::Ks0108;
pub use ky023::KY023;
pub use ky040::KY040;
pub use lamp::Lamp;
pub use latch::Latch;
pub use ldr::Ldr;
pub use led::{LED_COLOR_OPTIONS, Led, LedColor};
pub use led_bar::LedBar;
pub use led_matrix::LedMatrix;
pub use lm555::Lm555;
pub use logic_analyzer::LogicAnalyzer;
pub use magnitude_comp::MagnitudeComp;
pub use max72xx::Max72xx;
pub use mcu::Mcu;
pub use memory::Memory;
pub use mosfet::Mosfet;
pub use mux::Mux;
pub use node::Node;
pub use not_gate::NotGate;
pub use opamp::OpAmp;
pub use or_gate::OrGate;
pub use oscope::Oscope;
pub use pcd8544::Pcd8544;
pub use pcf8833::Pcf8833;
pub use potentiometer::Potentiometer;
pub use probe::Probe;
pub use props::{PropDef, PropError, PropKind, PropValue};
pub use push::Push;
pub use qemu_device::QemuDevice;
pub use rail::Rail;
pub use relay::Relay;
pub use resistor::{RESISTOR_BODY, Resistor, resistor_hit_rect, resistor_selection_rect};
pub use resistor_dip::ResistorDip;
pub use rgb_led::RgbLed;
pub use rtd::Rtd;
pub use scr::Scr;
pub use sd_card::SdCard;
pub use ser::{
    FORMAT_VERSION, GraphicAttrs, ParsedV3, apply_item_attrs, create_part, lookup_factory,
    parse_circuit_v3, registered_itemtypes, write_circuit_v3, write_component_item,
};
pub use serial_port::SerialPort;
pub use serial_term::SerialTerm;
pub use servo::Servo;
pub use seven_segment::SevenSegment;
pub use seven_segment_bcd::SevenSegmentBCD;
pub use sh1107::Sh1107;
pub use shape::{SHAPE_KINDS, Shape, ShapeKind};
pub use shift_reg::ShiftReg;
pub use socket::Socket;
pub use sr04::SR04;
pub use ssd1306::{SSD1306_COLOR_OPTIONS, Ssd1306, Ssd1306Color};
pub use stepper::Stepper;
pub use strain::Strain;
pub use sub_package::SubPackage;
pub use subcircuit::Subcircuit;
pub use switch::Switch;
pub use switch_dip::SwitchDip;
pub use test_unit::TestUnit;
pub use tft_display::{TFT_CONTROLLER_OPTIONS, TftController, TftDisplay};
pub use thermistor::Thermistor;
pub use touchpad::TouchPad;
pub use transformer::Transformer;
pub use triac::Triac;
pub use tunnel::Tunnel;
pub use var_capacitor::VarCapacitor;
pub use var_inductor::VarInductor;
pub use var_resistor::VarResistor;
pub use volt_reg::VoltReg;
pub use volt_source::VoltSource;
pub use voltmeter::Voltmeter;
pub use wave_gen::{WAVE_TYPE_OPTIONS, WaveGen, WaveType};
pub use ws2812::Ws2812;
pub use xor_gate::XorGate;

use crate::canvas::Rect;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::elements::Kind;
use crate::matrix::CircMatrix;

/// Concrete library part. Dispatch stays static; no `Box<dyn Component>` on
/// the stamp path.
#[derive(Clone, Debug)]
pub enum Part {
    Resistor(Resistor),
    VarResistor(VarResistor),
    Potentiometer(Potentiometer),
    Capacitor(Capacitor),
    ElCapacitor(ElCapacitor),
    Inductor(Inductor),
    VarCapacitor(VarCapacitor),
    VarInductor(VarInductor),
    Battery(Battery),
    Ground(Ground),
    Node(Node),
    FixedVolt(FixedVolt),
    Rail(Rail),
    Clock(Clock),
    VoltSource(VoltSource),
    CurrSource(CurrSource),
    Switch(Switch),
    Push(Push),
    SwitchDip(SwitchDip),
    Relay(Relay),
    KeyPad(KeyPad),
    Diode(Diode),
    Led(Led),
    Bjt(Bjt),
    Mosfet(Mosfet),
    Jfet(Jfet),
    OpAmp(OpAmp),
    Comparator(Comparator),
    VoltReg(VoltReg),
    Probe(Probe),
    Voltmeter(Voltmeter),
    Ammeter(Ammeter),
    FreqMeter(FreqMeter),
    AndGate(AndGate),
    OrGate(OrGate),
    XorGate(XorGate),
    NotGate(NotGate),
    BufferGate(BufferGate),
    FlipFlop(FlipFlop),
    Latch(Latch),
    Mux(Mux),
    Demux(Demux),
    AnalogMux(AnalogMux),
    BcdToDec(BcdToDec),
    DecToBcd(DecToBcd),
    BcdTo7Segment(BcdTo7Segment),
    FullAdder(FullAdder),
    HalfAdder(HalfAdder),
    TestUnit(TestUnit),
    LogicAnalyzer(LogicAnalyzer),
    TouchPad(TouchPad),
    KY023(KY023),
    KY040(KY040),
    SR04(SR04),
    DHT22(DHT22),
    DS18B20(DS18B20),
    DS1621(DS1621),
    DS1307(DS1307),
    DcMotor(DcMotor),
    Stepper(Stepper),
    Servo(Servo),
    SdCard(SdCard),
    Esp01(Esp01),
    TftDisplay(TftDisplay),
    Pcd8544(Pcd8544),
    Sh1107(Sh1107),
    Ks0108(Ks0108),
    Pcf8833(Pcf8833),
    Aip31068(Aip31068),
    Hd44780(Hd44780),
    Ssd1306(Ssd1306),
    SevenSegment(SevenSegment),
    SevenSegmentBCD(SevenSegmentBCD),
    LedMatrix(LedMatrix),
    Max72xx(Max72xx),
    LedBar(LedBar),
    RgbLed(RgbLed),
    Ws2812(Ws2812),
    Counter(Counter),
    BinCounter(BinCounter),
    ShiftReg(ShiftReg),
    MagnitudeComp(MagnitudeComp),
    Adc(Adc),
    Dac(Dac),
    I2CToParallel(I2CToParallel),
    Lm555(Lm555),
    Memory(Memory),
    DynamicMemory(DynamicMemory),
    I2CRam(I2CRam),
    Function(Function),
    Transformer(Transformer),
    Scr(Scr),
    Diac(Diac),
    Triac(Triac),
    Csource(Csource),
    ResistorDip(ResistorDip),
    Ldr(Ldr),
    Thermistor(Thermistor),
    Rtd(Rtd),
    Strain(Strain),
    AudioOut(AudioOut),
    Lamp(Lamp),
    WaveGen(WaveGen),
    Oscope(Oscope),
    Tunnel(Tunnel),
    Bus(Bus),
    Socket(Socket),
    Header(Header),
    SerialPort(SerialPort),
    SerialTerm(SerialTerm),
    Subcircuit(Subcircuit),
    SubPackage(SubPackage),
    Dial(Dial),
    Shape(Shape),
    Mcu(Mcu),
    QemuDevice(QemuDevice),
}

macro_rules! each_migrated {
    ($part:expr, |$p:ident| $body:expr) => {
        match $part {
            Part::Resistor($p) => $body,
            Part::VarResistor($p) => $body,
            Part::Potentiometer($p) => $body,
            Part::Capacitor($p) => $body,
            Part::ElCapacitor($p) => $body,
            Part::Inductor($p) => $body,
            Part::VarCapacitor($p) => $body,
            Part::VarInductor($p) => $body,
            Part::Battery($p) => $body,
            Part::Ground($p) => $body,
            Part::Node($p) => $body,
            Part::FixedVolt($p) => $body,
            Part::Rail($p) => $body,
            Part::Clock($p) => $body,
            Part::VoltSource($p) => $body,
            Part::CurrSource($p) => $body,
            Part::Switch($p) => $body,
            Part::Push($p) => $body,
            Part::SwitchDip($p) => $body,
            Part::Relay($p) => $body,
            Part::KeyPad($p) => $body,
            Part::Diode($p) => $body,
            Part::Led($p) => $body,
            Part::Bjt($p) => $body,
            Part::Mosfet($p) => $body,
            Part::Jfet($p) => $body,
            Part::OpAmp($p) => $body,
            Part::Comparator($p) => $body,
            Part::VoltReg($p) => $body,
            Part::Probe($p) => $body,
            Part::Voltmeter($p) => $body,
            Part::Ammeter($p) => $body,
            Part::FreqMeter($p) => $body,
            Part::AndGate($p) => $body,
            Part::OrGate($p) => $body,
            Part::XorGate($p) => $body,
            Part::NotGate($p) => $body,
            Part::BufferGate($p) => $body,
            Part::FlipFlop($p) => $body,
            Part::Latch($p) => $body,
            Part::Mux($p) => $body,
            Part::Demux($p) => $body,
            Part::AnalogMux($p) => $body,
            Part::BcdToDec($p) => $body,
            Part::DecToBcd($p) => $body,
            Part::BcdTo7Segment($p) => $body,
            Part::FullAdder($p) => $body,
            Part::HalfAdder($p) => $body,
            Part::TestUnit($p) => $body,
            Part::LogicAnalyzer($p) => $body,
            Part::TouchPad($p) => $body,
            Part::KY023($p) => $body,
            Part::KY040($p) => $body,
            Part::SR04($p) => $body,
            Part::DHT22($p) => $body,
            Part::DS18B20($p) => $body,
            Part::DS1621($p) => $body,
            Part::DS1307($p) => $body,
            Part::DcMotor($p) => $body,
            Part::Stepper($p) => $body,
            Part::Servo($p) => $body,
            Part::SdCard($p) => $body,
            Part::Esp01($p) => $body,
            Part::TftDisplay($p) => $body,
            Part::Pcd8544($p) => $body,
            Part::Sh1107($p) => $body,
            Part::Ks0108($p) => $body,
            Part::Pcf8833($p) => $body,
            Part::Aip31068($p) => $body,
            Part::Hd44780($p) => $body,
            Part::Ssd1306($p) => $body,
            Part::SevenSegment($p) => $body,
            Part::SevenSegmentBCD($p) => $body,
            Part::LedMatrix($p) => $body,
            Part::Max72xx($p) => $body,
            Part::LedBar($p) => $body,
            Part::RgbLed($p) => $body,
            Part::Ws2812($p) => $body,
            Part::Counter($p) => $body,
            Part::BinCounter($p) => $body,
            Part::ShiftReg($p) => $body,
            Part::MagnitudeComp($p) => $body,
            Part::Adc($p) => $body,
            Part::Dac($p) => $body,
            Part::I2CToParallel($p) => $body,
            Part::Lm555($p) => $body,
            Part::Memory($p) => $body,
            Part::DynamicMemory($p) => $body,
            Part::I2CRam($p) => $body,
            Part::Function($p) => $body,
            Part::Transformer($p) => $body,
            Part::Scr($p) => $body,
            Part::Diac($p) => $body,
            Part::Triac($p) => $body,
            Part::Csource($p) => $body,
            Part::ResistorDip($p) => $body,
            Part::Ldr($p) => $body,
            Part::Thermistor($p) => $body,
            Part::Rtd($p) => $body,
            Part::Strain($p) => $body,
            Part::AudioOut($p) => $body,
            Part::Lamp($p) => $body,
            Part::WaveGen($p) => $body,
            Part::Oscope($p) => $body,
            Part::Tunnel($p) => $body,
            Part::Bus($p) => $body,
            Part::Socket($p) => $body,
            Part::Header($p) => $body,
            Part::SerialPort($p) => $body,
            Part::SerialTerm($p) => $body,
            Part::Subcircuit($p) => $body,
            Part::SubPackage($p) => $body,
            Part::Dial($p) => $body,
            Part::Shape($p) => $body,
            Part::Mcu($p) => $body,
            Part::QemuDevice($p) => $body,
        }
    };
}

macro_rules! each_migrated_mut {
    ($part:expr, |$p:ident| $body:expr) => {
        match $part {
            Part::Resistor($p) => $body,
            Part::VarResistor($p) => $body,
            Part::Potentiometer($p) => $body,
            Part::Capacitor($p) => $body,
            Part::ElCapacitor($p) => $body,
            Part::Inductor($p) => $body,
            Part::VarCapacitor($p) => $body,
            Part::VarInductor($p) => $body,
            Part::Battery($p) => $body,
            Part::Ground($p) => $body,
            Part::Node($p) => $body,
            Part::FixedVolt($p) => $body,
            Part::Rail($p) => $body,
            Part::Clock($p) => $body,
            Part::VoltSource($p) => $body,
            Part::CurrSource($p) => $body,
            Part::Switch($p) => $body,
            Part::Push($p) => $body,
            Part::SwitchDip($p) => $body,
            Part::Relay($p) => $body,
            Part::KeyPad($p) => $body,
            Part::Diode($p) => $body,
            Part::Led($p) => $body,
            Part::Bjt($p) => $body,
            Part::Mosfet($p) => $body,
            Part::Jfet($p) => $body,
            Part::OpAmp($p) => $body,
            Part::Comparator($p) => $body,
            Part::VoltReg($p) => $body,
            Part::Probe($p) => $body,
            Part::Voltmeter($p) => $body,
            Part::Ammeter($p) => $body,
            Part::FreqMeter($p) => $body,
            Part::AndGate($p) => $body,
            Part::OrGate($p) => $body,
            Part::XorGate($p) => $body,
            Part::NotGate($p) => $body,
            Part::BufferGate($p) => $body,
            Part::FlipFlop($p) => $body,
            Part::Latch($p) => $body,
            Part::Mux($p) => $body,
            Part::Demux($p) => $body,
            Part::AnalogMux($p) => $body,
            Part::BcdToDec($p) => $body,
            Part::DecToBcd($p) => $body,
            Part::BcdTo7Segment($p) => $body,
            Part::FullAdder($p) => $body,
            Part::HalfAdder($p) => $body,
            Part::TestUnit($p) => $body,
            Part::LogicAnalyzer($p) => $body,
            Part::TouchPad($p) => $body,
            Part::KY023($p) => $body,
            Part::KY040($p) => $body,
            Part::SR04($p) => $body,
            Part::DHT22($p) => $body,
            Part::DS18B20($p) => $body,
            Part::DS1621($p) => $body,
            Part::DS1307($p) => $body,
            Part::DcMotor($p) => $body,
            Part::Stepper($p) => $body,
            Part::Servo($p) => $body,
            Part::SdCard($p) => $body,
            Part::Esp01($p) => $body,
            Part::TftDisplay($p) => $body,
            Part::Pcd8544($p) => $body,
            Part::Sh1107($p) => $body,
            Part::Ks0108($p) => $body,
            Part::Pcf8833($p) => $body,
            Part::Aip31068($p) => $body,
            Part::Hd44780($p) => $body,
            Part::Ssd1306($p) => $body,
            Part::SevenSegment($p) => $body,
            Part::SevenSegmentBCD($p) => $body,
            Part::LedMatrix($p) => $body,
            Part::Max72xx($p) => $body,
            Part::LedBar($p) => $body,
            Part::RgbLed($p) => $body,
            Part::Ws2812($p) => $body,
            Part::Counter($p) => $body,
            Part::BinCounter($p) => $body,
            Part::ShiftReg($p) => $body,
            Part::MagnitudeComp($p) => $body,
            Part::Adc($p) => $body,
            Part::Dac($p) => $body,
            Part::I2CToParallel($p) => $body,
            Part::Lm555($p) => $body,
            Part::Memory($p) => $body,
            Part::DynamicMemory($p) => $body,
            Part::I2CRam($p) => $body,
            Part::Function($p) => $body,
            Part::Transformer($p) => $body,
            Part::Scr($p) => $body,
            Part::Diac($p) => $body,
            Part::Triac($p) => $body,
            Part::Csource($p) => $body,
            Part::ResistorDip($p) => $body,
            Part::Ldr($p) => $body,
            Part::Thermistor($p) => $body,
            Part::Rtd($p) => $body,
            Part::Strain($p) => $body,
            Part::AudioOut($p) => $body,
            Part::Lamp($p) => $body,
            Part::WaveGen($p) => $body,
            Part::Oscope($p) => $body,
            Part::Tunnel($p) => $body,
            Part::Bus($p) => $body,
            Part::Socket($p) => $body,
            Part::Header($p) => $body,
            Part::SerialPort($p) => $body,
            Part::SerialTerm($p) => $body,
            Part::Subcircuit($p) => $body,
            Part::SubPackage($p) => $body,
            Part::Dial($p) => $body,
            Part::Shape($p) => $body,
            Part::Mcu($p) => $body,
            Part::QemuDevice($p) => $body,
        }
    };
}

impl Part {
    pub fn interact_toggle(&mut self, local: Point) -> bool {
        each_migrated_mut!(self, |p| p.interact_toggle(local))
    }

    pub fn interact_wheel(&mut self, local: Point, delta: f64) -> bool {
        each_migrated_mut!(self, |p| p.interact_wheel(local, delta))
    }

    pub fn interact_press(&mut self, local: Point) -> bool {
        each_migrated_mut!(self, |p| p.interact_press(local))
    }

    pub fn interact_move(&mut self, local: Point) -> bool {
        each_migrated_mut!(self, |p| p.interact_move(local))
    }

    pub fn interact_release(&mut self, local: Point) -> bool {
        each_migrated_mut!(self, |p| p.interact_release(local))
    }

    pub fn interact_cancel(&mut self) -> bool {
        each_migrated_mut!(self, |p| p.interact_cancel())
    }

    pub fn type_id(&self) -> &'static str {
        each_migrated!(self, |p| p.type_id())
    }

    pub fn type_name(&self) -> &'static str {
        self.type_id()
    }

    pub fn description(&self) -> &'static str {
        each_migrated!(self, |p| p.description())
    }

    pub fn pin_geoms(&self) -> Vec<CompPin> {
        each_migrated!(self, |p| p.pin_geoms())
    }

    pub fn body(&self) -> Rect {
        each_migrated!(self, |p| p.body())
    }

    pub fn visual_rect(&self) -> Rect {
        each_migrated!(self, |p| p.visual_rect())
    }

    pub fn package(&self) -> Option<&crate::package::Package> {
        match self {
            Self::Subcircuit(p) => Some(&p.package),
            Self::SubPackage(p) => Some(&p.package),
            Self::Mcu(p) => Some(&p.package),
            Self::QemuDevice(p) => Some(&p.package),
            _ => None,
        }
    }

    pub fn prop_groups(&self) -> Vec<PropGroup> {
        each_migrated!(self, |p| p.prop_groups())
    }

    pub fn wiper(&self) -> f64 {
        match self {
            Self::Potentiometer(p) => p.wiper,
            Self::VarResistor(p) => p.wiper(),
            Self::VarCapacitor(p) => p.wiper(),
            Self::VarInductor(p) => p.wiper(),
            Self::Dial(p) => p.wiper(),
            _ => 0.5,
        }
    }

    pub fn ch_tunnels(&self) -> Vec<String> {
        match self {
            Self::Oscope(p) => p.ch_tunnels(),
            Self::LogicAnalyzer(p) => p.ch_tunnels(),
            _ => Vec::new(),
        }
    }

    pub fn disp_width(&self) -> usize {
        match self {
            Self::Ssd1306(p) => p.disp_width(),
            Self::TftDisplay(p) => p.disp_width(),
            Self::Pcd8544(p) => p.disp_width(),
            Self::Sh1107(p) => p.disp_width(),
            Self::Ks0108(p) => p.disp_width(),
            Self::Pcf8833(p) => p.disp_width(),
            _ => 128,
        }
    }

    pub fn disp_height(&self) -> usize {
        match self {
            Self::Ssd1306(p) => p.disp_height(),
            Self::TftDisplay(p) => p.disp_height(),
            Self::Pcd8544(p) => p.disp_height(),
            Self::Sh1107(p) => p.disp_height(),
            Self::Ks0108(p) => p.disp_height(),
            Self::Pcf8833(p) => p.disp_height(),
            _ => 64,
        }
    }

    pub fn human_name(&self) -> &str {
        match self {
            Self::Subcircuit(p) => {
                let clean = crate::package::clean_chip_label(&p.package.name);
                if !clean.is_empty() {
                    clean
                } else if !p.device.is_empty() {
                    p.device.as_str()
                } else {
                    "Subcircuit"
                }
            }
            Self::Mcu(p) => {
                let clean = crate::package::clean_chip_label(&p.package.name);
                if !clean.is_empty() {
                    clean
                } else if !p.device.is_empty() {
                    p.device.as_str()
                } else {
                    "Microcontroller (MCU)"
                }
            }
            Self::QemuDevice(p) => {
                let clean = crate::package::clean_chip_label(&p.package.name);
                if !clean.is_empty() {
                    clean
                } else if !p.qemu.device.is_empty() {
                    p.qemu.device.as_str()
                } else {
                    "QEMU Device"
                }
            }
            Self::TouchPad(_) => "TouchPad (Resistive)",
            Self::KY023(_) => "Joystick Dual Axis (KY-023)",
            Self::KY040(_) => "Rotary Encoder (KY-040)",
            Self::SR04(_) => "HC-SR04 Ultrasonic Sensor",
            Self::DHT22(_) => "DHT22 Temperature & Humidity Sensor",
            Self::DS18B20(_) => "DS18B20 1-Wire Temperature Sensor",
            Self::DS1621(_) => "DS1621 I2C Temperature Sensor",
            Self::DS1307(_) => "DS1307 Real Time Clock",
            Self::DcMotor(_) => "DC Motor",
            Self::Stepper(_) => "Stepper Motor",
            Self::Servo(_) => "Servo Motor",
            Self::SdCard(_) => "SD Card Reader",
            Self::Esp01(_) => "ESP-01 Wi-Fi Module",
            Self::TftDisplay(_) => "TFT Display",
            Self::Pcd8544(_) => "PCD8544 LCD (Nokia 5110)",
            Self::Sh1107(_) => "SH1107 OLED",
            Self::Ks0108(_) => "KS0108 Graphic LCD",
            Self::Pcf8833(_) => "PCF8833 Color LCD",
            Self::Aip31068(_) => "AIP31068 I2C LCD",
            Self::Resistor(_) => "Resistor",
            Self::Battery(_) => "Battery",
            Self::Ground(_) => "Ground (0 V)",
            Self::FixedVolt(_) => "Fixed Voltage",
            Self::Node(_) => "Node",
            Self::Capacitor(_) => "Capacitor",
            Self::ElCapacitor(_) => "Electrolytic Capacitor",
            Self::VarCapacitor(_) => "Variable Capacitor",
            Self::Inductor(_) => "Inductor",
            Self::VarInductor(_) => "Variable Inductor",
            Self::Hd44780(_) => "HD44780 LCD",
            Self::Ssd1306(_) => "SSD1306 OLED",
            Self::Switch(_) => "Switch",
            Self::Diode(d) => {
                if d.zener {
                    "Zener Diode"
                } else {
                    "Diode"
                }
            }
            Self::Led(_) => "LED",
            Self::Bjt(_) => "BJT",
            Self::Mosfet(_) => "MOSFET",
            Self::OpAmp(_) => "OpAmp",
            Self::Jfet(_) => "JFET",
            Self::Comparator(_) => "Comparator",
            Self::VoltReg(_) => "Voltage Regulator",
            Self::Probe(_) => "Probe",
            Self::Voltmeter(_) => "Voltmeter",
            Self::Ammeter(_) => "Ampmeter",
            Self::FreqMeter(_) => "Frequency Meter",
            Self::Oscope(_) => "Oscilloscope",
            Self::LogicAnalyzer(_) => "Logic Analyzer",
            Self::AndGate(_) => "And Gate",
            Self::OrGate(_) => "Or Gate",
            Self::XorGate(_) => "Xor Gate",
            Self::NotGate(_) => "Not Gate",
            Self::BufferGate(_) => "Buffer",
            Self::FlipFlop(_) => "Flip-Flop",
            Self::Latch(_) => "Latch D",
            Self::TestUnit(_) => "Test Unit",
            Self::Clock(_) => "Clock",
            Self::Rail(_) => "Rail",
            Self::WaveGen(_) => "Wave Generator",
            Self::VoltSource(_) => "Voltage Source",
            Self::CurrSource(_) => "Current Source",
            Self::Csource(_) => "Current Controlled Source",
            Self::Push(_) => "Push Button",
            Self::SwitchDip(_) => "DIP Switch",
            Self::Relay(_) => "Relay",
            Self::KeyPad(_) => "Keypad",
            Self::Potentiometer(_) => "Potentiometer",
            Self::VarResistor(_) => "Variable Resistor",
            Self::ResistorDip(_) => "Resistor DIP",
            Self::Ldr(_) => "LDR",
            Self::Thermistor(_) => "Thermistor",
            Self::Rtd(_) => "RTD",
            Self::Strain(_) => "Strain Gauge",
            Self::AudioOut(_) => "Audio Output",
            Self::Lamp(_) => "Incandescent Lamp",
            Self::Tunnel(_) => "Tunnel",
            Self::Mux(_) => "Multiplexer",
            Self::Demux(_) => "Demultiplexer",
            Self::AnalogMux(_) => "Analog Multiplexer",
            Self::BcdToDec(_) => "BCD to Decimal Decoder",
            Self::DecToBcd(_) => "Decimal to BCD Encoder",
            Self::BcdTo7Segment(_) => "BCD to 7-Segment Decoder",
            Self::SevenSegment(_) => "7-Segment Display",
            Self::SevenSegmentBCD(_) => "7-Segment Display with BCD Decoder",
            Self::LedMatrix(_) => "LED Matrix",
            Self::Max72xx(_) => "MAX7219/MAX7221 Display",
            Self::LedBar(_) => "LED Bar Graph",
            Self::RgbLed(_) => "RGB LED",
            Self::Ws2812(_) => "WS2812 RGB LED",
            Self::Counter(_) => "Simple Counter",
            Self::BinCounter(_) => "Binary Counter",
            Self::ShiftReg(_) => "Shift Register",
            Self::MagnitudeComp(_) => "Magnitude Comparator",
            Self::FullAdder(_) => "Full Adder",
            Self::HalfAdder(_) => "Half Adder",
            Self::Adc(_) => "ADC",
            Self::Dac(_) => "DAC",
            Self::I2CToParallel(_) => "I2C to Parallel (PCF8574)",
            Self::Lm555(_) => "LM555 Timer",
            Self::Memory(_) => "RAM/ROM Memory",
            Self::DynamicMemory(_) => "Dynamic RAM",
            Self::I2CRam(_) => "I2C 24Cxx EEPROM",
            Self::Function(_) => "Logic Function",
            Self::Transformer(_) => "Transformer",
            Self::Scr(_) => "SCR",
            Self::Diac(_) => "DIAC",
            Self::Triac(_) => "TRIAC",
            Self::Bus(_) => "Bus",
            Self::Socket(_) => "Socket",
            Self::Header(_) => "Header",
            Self::SerialPort(_) => "Serial Port",
            Self::SerialTerm(_) => "Serial Terminal",
            Self::SubPackage(_) => "Subcircuit Package",
            Self::Dial(_) => "Dial",
            Self::Shape(_) => "Shape",
        }
    }

    pub fn default_show_prop(&self) -> &'static str {
        match self {
            Self::Resistor(_)
            | Self::Potentiometer(_)
            | Self::VarResistor(_)
            | Self::ResistorDip(_) => "Resistance",
            Self::Capacitor(_) | Self::ElCapacitor(_) | Self::VarCapacitor(_) => "Capacitance",
            Self::Inductor(_) | Self::VarInductor(_) => "Inductance",
            Self::Battery(_) | Self::FixedVolt(_) | Self::VoltReg(_) | Self::Rail(_) => "Voltage",
            Self::Clock(_) => "Frequency",
            Self::WaveGen(_) => "Wave_Type",
            Self::VoltSource(_) => "MaxValue",
            Self::CurrSource(_) => "MaxValue",
            Self::Probe(_) => "Threshold",
            Self::Led(_)
            | Self::LedBar(_)
            | Self::LedMatrix(_)
            | Self::SevenSegment(_)
            | Self::Ssd1306(_) => "Color",
            Self::SubPackage(_) => "SubcType",
            Self::Dial(_) => "Value",
            Self::Ldr(_) => "Lux",
            Self::Thermistor(_) | Self::Rtd(_) => "Temp",
            Self::Strain(_) => "Strain",
            _ => "",
        }
    }

    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node(_))
    }

    /// Whether the named pin is inverted by default for this component type.
    /// Matches directly on `&self` variants without cloning any simulation state.
    pub fn is_pin_default_inverted(&self, pin_id_local: &str) -> bool {
        let ll = pin_id_local.to_ascii_lowercase();
        match self {
            Self::Mux(_) => ll == "enable" || ll == "oe" || ll == "out_inv" || ll == "in11",
            Self::Demux(p) => {
                ll == "enable" || ll == "oe" || ll == "in4" || (p.inverted && ll.starts_with("out"))
            }
            Self::AnalogMux(_) => ll == "pinenable" || ll == "enable" || ll == "en",
            Self::AndGate(p) => {
                if ll == "out" || ll == "outpin" {
                    p.invert_output || false // "And" never inverts by default; only invert_output
                } else {
                    ll == "pin_outenable" || ll == "enable" || ll == "oe"
                }
            }
            Self::OrGate(p) => {
                if ll == "out" || ll == "outpin" {
                    p.invert_output
                } else {
                    ll == "pin_outenable" || ll == "enable" || ll == "oe"
                }
            }
            Self::XorGate(p) => {
                if ll == "out" || ll == "outpin" {
                    p.invert_output
                } else {
                    ll == "pin_outenable" || ll == "enable" || ll == "oe"
                }
            }
            Self::NotGate(_) => {
                if ll == "out" || ll == "outpin" {
                    true // NOT always inverts output
                } else {
                    ll == "pin_outenable" || ll == "enable" || ll == "oe"
                }
            }
            Self::BufferGate(p) => {
                if ll == "out" || ll == "outpin" {
                    p.invert_output
                } else {
                    ll == "pin_outenable" || ll == "enable" || ll == "oe"
                }
            }
            Self::FlipFlop(p) => {
                ll == "out1"
                    || ll == "set"
                    || ll == "in1"
                    || ll == "in2"
                    || (ll == "rst" && p.reset_inverted)
                    || (ll == "in3" && (p.reset_inverted || p.clock_inverted))
            }
            Self::Counter(_) => ll == "rst" || ll == "set" || ll == "in1" || ll == "in2",
            Self::BinCounter(_) => ll == "rst" || ll == "dir" || ll == "pin_rst" || ll == "pin_dir",
            Self::Latch(_) => {
                ll == "oe"
                    || ll == "rst"
                    || ll == "pin_outenable"
                    || ll == "pin_reset"
                    || ll == "out1"
            }
            Self::ShiftReg(_) => {
                ll == "oe"
                    || ll == "rst"
                    || ll == "mr"
                    || ll == "ser"
                    || ll == "in2"
                    || ll == "in3"
                    || ll == "in4"
            }
            Self::BcdTo7Segment(_) => {
                ll == "enable" || ll == "oe" || ll == "rst" || ll == "in4" || ll == "pin_reset"
            }
            Self::BcdToDec(p) => {
                ll == "enable"
                    || ll == "oe"
                    || ll == "in4"
                    || (p.active_low && ll.starts_with("out"))
            }
            Self::DecToBcd(p) => {
                ll == "enable"
                    || ll == "oe"
                    || ll == "in15"
                    || (p.active_low && ll.starts_with("out"))
            }
            Self::DynamicMemory(_) => {
                ll == "ras"
                    || ll == "cas"
                    || ll == "we"
                    || ll == "oe"
                    || ll == "pin_ras"
                    || ll == "pin_cas"
                    || ll == "pin_we"
                    || ll == "pin_outenable"
            }
            Self::Memory(_) => {
                ll == "cs"
                    || ll == "we"
                    || ll == "oe"
                    || ll == "pin_cs"
                    || ll == "pin_we"
                    || ll == "pin_outenable"
            }
            Self::Comparator(p) => (ll == "out" || ll == "outpin") && p.inverted,
            Self::SevenSegmentBCD(_) => ll == "enable" || ll == "en" || ll == "e" || ll == "in4",
            Self::I2CToParallel(_) => false,
            _ => false,
        }
    }

    /// Whether the named pin has a default pull-up for this component type.
    /// Matches directly on `&self` variants without cloning any simulation state.
    pub fn is_pin_default_pullup(&self, pin_id_local: &str) -> bool {
        let ll = pin_id_local.to_ascii_lowercase();
        match self {
            Self::I2CToParallel(_) => {
                ll == "int"
                    || ll == "in5"
                    || ll.starts_with("out")
                    || ll.starts_with('d')
                    || ll.starts_with('p')
            }
            _ => false,
        }
    }

    pub fn prop_rows(&self) -> Vec<crate::canvas::PropRow> {
        each_migrated!(self, |p| p.prop_rows())
    }

    pub fn get_prop_text(&self, id: &str) -> Option<String> {
        each_migrated!(self, |p| p
            .get_prop_text(id)
            .or_else(|| p.get_prop_text_alias(id)))
    }

    pub fn set_prop_text(&mut self, id: &str, text: &str) -> Result<ComponentChange, PropError> {
        each_migrated!(self, |p| {
            p.set_prop_text(id, text)
                .or_else(|_| p.set_prop_text_alias(id, text))
        })
    }

    pub fn write_item(&self, circ_id: &str, graphic: &GraphicAttrs) -> String {
        each_migrated!(self, |p| write_component_item(circ_id, p, graphic))
    }

    pub fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        each_migrated!(self, |p| p.stamp(matrix, pin_nodes, dt))
    }

    pub fn to_element_kind(&self) -> Option<Kind> {
        each_migrated!(self, |p| Some(p.to_element_kind()))
    }

    pub fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        match self {
            Self::Resistor(p) => p.paint(d, ctx),
            Self::Led(p) => p.paint(d, ctx),
            Self::Push(p) => p.paint(d, ctx),
            Self::Battery(p) => p.paint(d, ctx),
            Self::Ground(p) => p.paint(d, ctx),
            Self::Node(p) => p.paint(d, ctx),
            Self::FixedVolt(p) => p.paint(d, ctx),
            Self::Rail(p) => p.paint(d, ctx),
            Self::Clock(p) => p.paint(d, ctx),
            Self::Capacitor(p) => p.paint(d, ctx),
            Self::ElCapacitor(p) => p.paint(d, ctx),
            Self::Inductor(p) => p.paint(d, ctx),
            Self::Switch(p) => p.paint(d, ctx),
            Self::Diode(p) => p.paint(d, ctx),
            Self::OpAmp(p) => p.paint(d, ctx),
            Self::Comparator(p) => p.paint(d, ctx),
            Self::VoltReg(p) => p.paint(d, ctx),
            Self::Bjt(p) => p.paint(d, ctx),
            Self::Mosfet(p) => p.paint(d, ctx),
            Self::Jfet(p) => p.paint(d, ctx),
            Self::AndGate(p) => p.paint(d, ctx),
            Self::OrGate(p) => p.paint(d, ctx),
            Self::XorGate(p) => p.paint(d, ctx),
            Self::NotGate(p) => p.paint(d, ctx),
            Self::BufferGate(p) => p.paint(d, ctx),
            Self::VarResistor(p) => p.paint(d, ctx),
            Self::Potentiometer(p) => p.paint(d, ctx),
            Self::VarCapacitor(p) => p.paint(d, ctx),
            Self::VarInductor(p) => p.paint(d, ctx),
            Self::Probe(p) => p.paint(d, ctx),
            Self::Voltmeter(p) => p.paint(d, ctx),
            Self::Ammeter(p) => p.paint(d, ctx),
            Self::FreqMeter(p) => p.paint(d, ctx),
            Self::SevenSegment(p) => p.paint(d, ctx),
            Self::SevenSegmentBCD(p) => p.paint(d, ctx),
            Self::Lamp(p) => p.paint(d, ctx),
            Self::Relay(p) => p.paint(d, ctx),
            Self::Transformer(p) => p.paint(d, ctx),
            Self::SwitchDip(p) => p.paint(d, ctx),
            Self::KeyPad(p) => p.paint(d, ctx),
            Self::Scr(p) => p.paint(d, ctx),
            Self::Diac(p) => p.paint(d, ctx),
            Self::Triac(p) => p.paint(d, ctx),
            Self::AnalogMux(p) => p.paint(d, ctx),
            Self::AudioOut(p) => p.paint(d, ctx),
            Self::Tunnel(p) => p.paint(d, ctx),
            Self::LedBar(p) => p.paint(d, ctx),
            Self::LedMatrix(p) => p.paint(d, ctx),
            Self::RgbLed(p) => p.paint(d, ctx),
            Self::Ws2812(p) => p.paint(d, ctx),
            Self::Max72xx(p) => p.paint(d, ctx),
            Self::KY040(p) => p.paint(d, ctx),
            Self::KY023(p) => p.paint(d, ctx),
            Self::TouchPad(p) => p.paint(d, ctx),
            Self::VoltSource(p) => p.paint(d, ctx),
            Self::CurrSource(p) => p.paint(d, ctx),
            Self::Csource(p) => p.paint(d, ctx),
            Self::WaveGen(p) => p.paint(d, ctx),
            Self::Dial(p) => p.paint(d, ctx),
            Self::ResistorDip(p) => p.paint(d, ctx),
            Self::Ldr(p) => p.paint(d, ctx),
            Self::Thermistor(p) => p.paint(d, ctx),
            Self::Rtd(p) => p.paint(d, ctx),
            Self::Strain(p) => p.paint(d, ctx),
            Self::FlipFlop(p) => p.paint(d, ctx),
            Self::Latch(p) => p.paint(d, ctx),
            Self::TestUnit(p) => p.paint(d, ctx),
            Self::LogicAnalyzer(p) => p.paint(d, ctx),
            Self::Oscope(p) => p.paint(d, ctx),
            Self::Bus(p) => p.paint(d, ctx),
            Self::Socket(p) => p.paint(d, ctx),
            Self::Header(p) => p.paint(d, ctx),
            Self::SerialPort(p) => p.paint(d, ctx),
            Self::SerialTerm(p) => p.paint(d, ctx),
            Self::Stepper(p) => p.paint(d, ctx),
            Self::DcMotor(p) => p.paint(d, ctx),
            Self::Servo(p) => p.paint(d, ctx),
            Self::Hd44780(p) => p.paint(d, ctx),
            Self::Ssd1306(p) => p.paint(d, ctx),
            Self::Mux(p) => p.paint(d, ctx),
            Self::Demux(p) => p.paint(d, ctx),
            Self::BcdToDec(p) => p.paint(d, ctx),
            Self::DecToBcd(p) => p.paint(d, ctx),
            Self::BcdTo7Segment(p) => p.paint(d, ctx),
            Self::FullAdder(p) => p.paint(d, ctx),
            Self::HalfAdder(p) => p.paint(d, ctx),
            Self::Counter(p) => p.paint(d, ctx),
            Self::BinCounter(p) => p.paint(d, ctx),
            Self::ShiftReg(p) => p.paint(d, ctx),
            Self::MagnitudeComp(p) => p.paint(d, ctx),
            Self::Adc(p) => p.paint(d, ctx),
            Self::Dac(p) => p.paint(d, ctx),
            Self::I2CToParallel(p) => p.paint(d, ctx),
            Self::Lm555(p) => p.paint(d, ctx),
            Self::Memory(p) => p.paint(d, ctx),
            Self::DynamicMemory(p) => p.paint(d, ctx),
            Self::I2CRam(p) => p.paint(d, ctx),
            Self::Function(p) => p.paint(d, ctx),
            Self::Shape(p) => p.paint(d, ctx),
            Self::Subcircuit(p) => p.paint(d, ctx),
            Self::SubPackage(p) => p.paint(d, ctx),
            Self::Mcu(p) => p.paint(d, ctx),
            Self::QemuDevice(p) => p.paint(d, ctx),
            Self::SR04(p) => p.paint(d, ctx),
            Self::DHT22(p) => p.paint(d, ctx),
            Self::DS18B20(p) => p.paint(d, ctx),
            Self::DS1621(p) => p.paint(d, ctx),
            Self::DS1307(p) => p.paint(d, ctx),
            Self::SdCard(p) => p.paint(d, ctx),
            Self::Esp01(p) => p.paint(d, ctx),
            Self::TftDisplay(p) => p.paint(d, ctx),
            Self::Pcd8544(p) => p.paint(d, ctx),
            Self::Sh1107(p) => p.paint(d, ctx),
            Self::Ks0108(p) => p.paint(d, ctx),
            Self::Pcf8833(p) => p.paint(d, ctx),
            Self::Aip31068(p) => p.paint(d, ctx),
        }
    }

    pub fn set_dial(&mut self, v: f64) -> Option<ComponentChange> {
        match self {
            Self::VarResistor(p) => Some(p.set_value(v)),
            Self::Potentiometer(p) => Some(p.set_value(v)),
            Self::VarCapacitor(p) => Some(p.set_value(v)),
            Self::VarInductor(p) => Some(p.set_value(v)),
            Self::VoltSource(p) => Some(p.set_value(v)),
            Self::CurrSource(p) => Some(p.set_value(v)),
            Self::Dial(p) => Some(p.set_value(v)),
            _ => None,
        }
    }

    pub fn dial(&self) -> Option<(f64, f64, f64)> {
        match self {
            Self::VarResistor(p) => Some((p.value(), p.min(), p.max())),
            Self::Potentiometer(p) => Some((p.value(), p.min(), p.max())),
            Self::VarCapacitor(p) => Some((p.value(), p.min(), p.max())),
            Self::VarInductor(p) => Some((p.value(), p.min(), p.max())),
            Self::VoltSource(p) => Some((p.value(), p.min(), p.max())),
            Self::CurrSource(p) => Some((p.value(), p.min(), p.max())),
            Self::Dial(p) => Some((p.value(), p.min(), p.max())),
            _ => None,
        }
    }

    pub fn add_to_circuit(&self, id: &str, c: &mut crate::Circuit) {
        match self {
            Self::Resistor(p) => {
                c.add_resistor(id, p.resistance);
            }
            Self::Battery(p) => {
                c.add_battery(id, p.voltage, p.resistance);
            }
            Self::Ground(_) => {
                c.add_ground(id);
            }
            Self::FixedVolt(p) => {
                c.add_fixed_volt(id, p.voltage);
            }
            Self::Node(_) => {
                c.add_node(id);
            }
            Self::Capacitor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: p.to_element_kind(),
                });
            }
            Self::ElCapacitor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: p.to_element_kind(),
                });
            }
            Self::VarCapacitor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: p.to_element_kind(),
                });
            }
            Self::Inductor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: p.to_element_kind(),
                });
            }
            Self::VarInductor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: p.to_element_kind(),
                });
            }
            Self::Switch(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: p.to_element_kind(),
                });
            }
            Self::Diode(p) => {
                c.add_diode_with(
                    id,
                    p.zener,
                    p.threshold,
                    p.max_current,
                    p.resistance,
                    p.brkdown_v,
                    p.sat_current,
                    p.em_coef,
                );
            }
            Self::Led(p) => {
                c.add_led(id);
                c.configure_led(id, p.threshold, p.max_current, p.resistance, p.grounded);
            }
            Self::Bjt(p) => {
                c.add_bjt_with(id, p.pnp, p.gain, p.threshold);
            }
            Self::Mosfet(p) => {
                c.add_mosfet_with(id, p.p_channel, p.depletion, p.rdson, p.threshold);
            }
            Self::OpAmp(p) => {
                c.add_opamp(id);
                c.configure_opamp(id, p.gain, p.out_imp, p.volt_pos, p.volt_neg, p.power_pins);
            }
            Self::Jfet(p) => {
                c.add_jfet(id);
                c.configure_jfet(id, p.idss, p.vp, p.lambda_inv);
            }
            Self::Comparator(p) => {
                c.add_comparator(id);
                c.configure_comparator(
                    id, p.out_high, p.out_low, p.out_imp, p.inverted, p.open_col,
                );
            }
            Self::VoltReg(p) => {
                c.add_volt_reg(id);
                c.configure_volt_reg(id, p.voltage);
            }
            Self::Probe(p) => {
                c.add_probe_with(id, p.threshold, p.small);
            }
            Self::Voltmeter(p) => {
                c.add_voltmeter(id, p.rms);
            }
            Self::Ammeter(p) => {
                c.add_ammeter(id, p.rms);
            }
            Self::FreqMeter(p) => {
                c.add_freq_meter_with(id, p.filter);
            }
            Self::Oscope(p) => {
                c.add_oscope_full(
                    id,
                    p.connect_gnd,
                    p.input_imped,
                    [
                        p.tunnel1.clone(),
                        p.tunnel2.clone(),
                        p.tunnel3.clone(),
                        p.tunnel4.clone(),
                    ],
                );
            }
            Self::LogicAnalyzer(p) => {
                c.add_lanalizer_full(
                    id,
                    p.connect_gnd,
                    p.input_imped,
                    [
                        p.tunnel1.clone(),
                        p.tunnel2.clone(),
                        p.tunnel3.clone(),
                        p.tunnel4.clone(),
                        p.tunnel5.clone(),
                        p.tunnel6.clone(),
                        p.tunnel7.clone(),
                        p.tunnel8.clone(),
                    ],
                );
            }
            Self::Subcircuit(p) => {
                let search =
                    crate::subcircuit::SubcSearch::from_circuit_path(p.nested_path.as_deref())
                        .with_memory(&p.device, p.nested_src.clone());
                if let Ok(inst) = crate::subcircuit::instantiate(
                    id,
                    &p.device,
                    &p.nested_src,
                    &search,
                    p.logic_symbol,
                    Some(p.package.name.as_str()),
                    0,
                ) {
                    for comp in inst.components {
                        c.add_comp(comp);
                    }
                    for (a, b) in inst.connectors {
                        c.connect(a, b);
                    }
                }
            }
            Self::Mcu(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Mcu(p.mcu.clone()),
                });
            }
            Self::QemuDevice(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::QemuDevice(p.qemu.clone()),
                });
            }
            Self::Clock(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Clock {
                        voltage: p.voltage,
                        freq_khz: p.frequency / 1e3,
                        state: false,
                    },
                });
            }
            Self::Rail(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Rail { voltage: p.voltage },
                });
            }
            Self::WaveGen(p) => {
                c.add_comp(crate::elements::Comp::wave_gen_with_wav_data(
                    id,
                    p.wave_type.as_str(),
                    p.freq_hz,
                    p.amplitude,
                    p.offset,
                    p.duty,
                    p.phase,
                    p.steps,
                    p.bipolar,
                    p.floating,
                    &p.file,
                    p.wav_data.clone(),
                ));
            }
            Self::VoltSource(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::VoltSource {
                        value: p.value,
                        running: p.running,
                    },
                });
            }
            Self::CurrSource(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::CurrSource {
                        value: p.value,
                        running: p.running,
                    },
                });
            }
            Self::Csource(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Csource {
                        control_pins: p.control_pins,
                        curr_source: p.curr_source,
                        curr_control: p.curr_control,
                        gain: p.gain,
                        volt: p.volt,
                        current: p.current,
                    },
                });
            }
            Self::Push(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Push {
                        closed: if p.norm_close { !p.pressed } else { p.pressed },
                        poles: p.poles,
                    },
                });
            }
            Self::SwitchDip(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::SwitchDip {
                        size: p.size,
                        state: p.state,
                        common_pin: p.common_pin,
                    },
                });
            }
            Self::KeyPad(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::KeyPad {
                        rows: p.rows,
                        cols: p.cols,
                        key: p.key.clone(),
                        diodes: p.diodes,
                        dir: p.dir,
                        pressed: p.pressed,
                    },
                });
            }
            Self::Relay(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Relay {
                        norm_close: p.norm_close,
                        double_throw: p.double_throw,
                        poles: p.poles,
                        i_on: p.i_on,
                        i_off: p.i_off,
                        active: p.active,
                    },
                });
            }
            Self::Potentiometer(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Potentiometer {
                        resistance: p.resistance,
                        wiper: p.wiper,
                    },
                });
            }
            Self::TouchPad(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::TouchPad {
                        width: p.width,
                        height: p.height,
                        rx_min: p.rx_min,
                        rx_max: p.rx_max,
                        ry_min: p.ry_min,
                        ry_max: p.ry_max,
                        x_pos: p.x_pos,
                        y_pos: p.y_pos,
                    },
                });
            }
            Self::KY023(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Ky023 {
                        stick_x: p.stick_x,
                        stick_y: p.stick_y,
                        btn_down: p.btn_down,
                    },
                });
            }
            Self::KY040(p) => {
                let k = p.dial_val.rem_euclid(4);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Ky040 {
                        steps: p.steps,
                        dial_val: p.dial_val,
                        btn_closed: p.btn_closed,
                        state_a: k == 1 || k == 2,
                        state_b: k == 2 || k == 3,
                    },
                });
            }
            Self::SR04(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Sr04 {
                        distance: p.distance,
                        use_slider: p.use_slider,
                        trigger_high: false,
                        echo_high: false,
                    },
                });
            }
            Self::DHT22(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Dht22 {
                        model: p.model.as_str().to_string(),
                        temp: p.temp,
                        humi: p.humi,
                        out_state: true,
                        pin_driven: false,
                    },
                });
            }
            Self::DS18B20(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Ds18b20 {
                        rom: p.rom.clone(),
                        temp: p.temp,
                        dq_low: false,
                    },
                });
            }
            Self::DS1621(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Ds1621 {
                        temp: p.temp,
                        th: 30.0,
                        tl: 10.0,
                        active: true,
                        tout_high: false,
                        sda_low: false,
                        scl_low: false,
                    },
                });
            }
            Self::DS1307(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Ds1307 {
                        time_updated: p.time_updated,
                        sqw_freq: 1.0,
                        sqw_enabled: true,
                        sqw_state: false,
                        sda_low: false,
                        scl_low: false,
                    },
                });
            }
            Self::DcMotor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::DcMotor {
                        rpm_nominal: p.rpm_nominal,
                        volt_nominal: p.volt_nominal,
                        resistance: p.resistance,
                        speed: p.speed,
                        angle: p.angle,
                    },
                });
            }
            Self::Stepper(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Stepper {
                        bipolar: p.bipolar,
                        steps: p.steps,
                        resistance: p.resistance,
                        angle: p.angle,
                    },
                });
            }
            Self::Servo(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Servo {
                        speed: p.speed,
                        min_pulse: p.min_pulse,
                        max_pulse: p.max_pulse,
                        pos: p.pos,
                        target_pos: p.pos,
                        pulse_start_ps: 0,
                        sig_high: false,
                    },
                });
            }
            Self::SdCard(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::SdCard {
                        file: p.file.clone(),
                        card_inserted: true,
                    },
                });
            }
            Self::Esp01(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Esp01 {
                        baud_rate: p.baud_rate,
                        debug: p.debug,
                    },
                });
            }
            Self::TftDisplay(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::TftDisplay {
                        controller: p.controller.as_str().to_string(),
                        width: p.width,
                        height: p.height,
                        scale: p.scale,
                        bgr: p.bgr,
                    },
                });
            }
            Self::Pcd8544(p) => {
                let mut kind = p.to_element_kind();
                if let crate::elements::Kind::Pcd8544(st) = &mut kind {
                    st.id = id.to_string();
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind,
                });
            }
            Self::Sh1107(p) => {
                let mut kind = p.to_element_kind();
                if let crate::elements::Kind::Sh1107(st) = &mut kind {
                    st.id = id.to_string();
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind,
                });
            }
            Self::Ks0108(p) => {
                let mut kind = p.to_element_kind();
                if let crate::elements::Kind::Ks0108(st) = &mut kind {
                    st.id = id.to_string();
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind,
                });
            }
            Self::Pcf8833(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Pcf8833Display {
                        width: p.width,
                        height: p.height,
                    },
                });
            }
            Self::Aip31068(p) => {
                let mut kind = p.to_element_kind();
                if let crate::elements::Kind::Aip31068(st) = &mut kind {
                    st.id = id.to_string();
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind,
                });
            }
            Self::VarResistor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::VarResistor {
                        resistance: p.resistance,
                    },
                });
            }
            Self::ResistorDip(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::ResistorDip {
                        size: p.size,
                        resistance: p.resistance,
                        bussed: false,
                    },
                });
            }
            Self::Ldr(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Ldr {
                        resistance: p.r_light,
                        lux: p.lux,
                    },
                });
            }
            Self::Thermistor(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Thermistor {
                        resistance: p.r0,
                        temp_c: p.temp_c,
                    },
                });
            }
            Self::Rtd(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Rtd {
                        resistance: p.r0,
                        temp_c: p.temp_c,
                    },
                });
            }
            Self::Strain(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Strain {
                        resistance: p.r0,
                        strain: p.strain,
                    },
                });
            }
            Self::Transformer(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Transformer {
                        inductance1: p.inductance1,
                        inductance2: p.inductance2,
                        coupling: p.coupling,
                    },
                });
            }
            Self::Scr(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Scr {
                        v_gate_th: p.v_gate_th,
                        i_hold: p.i_hold,
                        conducting: false,
                    },
                });
            }
            Self::Diac(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Diac {
                        v_breakover: p.v_breakover,
                        conducting: false,
                    },
                });
            }
            Self::Triac(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Triac {
                        v_gate_th: p.v_gate_th,
                        i_hold: p.i_hold,
                        conducting: false,
                    },
                });
            }
            Self::AnalogMux(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::AnalogMux {
                        channels: p.channels,
                        selected: 0,
                        on_res: p.on_resistance,
                    },
                });
            }
            Self::RgbLed(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::RgbLed {
                        common_anode: p.common_anode,
                        state_r: 0.0,
                        state_g: 0.0,
                        state_b: 0.0,
                    },
                });
            }
            Self::LedBar(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::LedBar {
                        segments: p.segments,
                        states: 0,
                        grounded: p.grounded,
                    },
                });
            }
            Self::SevenSegment(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::SevenSegment {
                        common_anode: p.common_anode,
                        segments: 0,
                    },
                });
            }
            Self::AudioOut(p) => {
                let source =
                    crate::audio::Source::new(p.impedance, p.volume, p.frequency, p.buzzer);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::AudioOut {
                        source,
                        last_v: 0.0,
                    },
                });
            }
            Self::Lamp(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Lamp {
                        voltage: p.voltage,
                        power: p.power,
                        r_cold: p.r_cold,
                        resistance: p.r_cold,
                    },
                });
            }
            Self::Tunnel(p) => {
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Tunnel {
                        name: p.name.clone(),
                        pin_id: format!("{id}-pin"),
                    },
                });
            }
            Self::AndGate(p) => {
                let mut g = crate::digital::GateState::and(id, p.num_inputs);
                g.output.set_inverted(p.invert_output);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Gate(g),
                });
            }
            Self::OrGate(p) => {
                let mut g = crate::digital::GateState::or(id, p.num_inputs);
                g.output.set_inverted(p.invert_output);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Gate(g),
                });
            }
            Self::XorGate(p) => {
                let mut g = crate::digital::GateState::xor(id, p.num_inputs);
                g.output.set_inverted(p.invert_output);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Gate(g),
                });
            }
            Self::NotGate(_) => {
                let mut g = crate::digital::GateState::buffer(id);
                g.output.set_inverted(true);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Gate(g),
                });
            }
            Self::BufferGate(p) => {
                let mut g = crate::digital::GateState::buffer(id);
                g.output.set_inverted(p.invert_output);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Gate(g),
                });
            }
            Self::FlipFlop(p) => {
                let f = match p.ff_kind.as_str() {
                    "D" => crate::digital::FlipFlopState::d(id),
                    "JK" => crate::digital::FlipFlopState::jk(id),
                    "RS" => crate::digital::FlipFlopState::rs(id),
                    _ => crate::digital::FlipFlopState::t(id),
                };
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::FlipFlop(f),
                });
            }
            Self::Latch(p) => {
                let l = crate::digital::LatchState::new(id, p.channels);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Latch(l),
                });
            }
            Self::TestUnit(p) => {
                let mut tu = crate::digital::TestUnitState::new(id);
                tu.set_inputs(id, &p.inputs);
                tu.set_outputs(id, &p.outputs);
                tu.period = p.period;
                tu.truth = p.truth.clone();
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::TestUnit(tu),
                });
            }
            Self::Hd44780(p) => {
                c.add_comp(crate::elements::Comp::hd44780(
                    id.to_string(),
                    p.rows,
                    p.cols,
                ));
            }
            Self::Ssd1306(p) => {
                c.add_comp(crate::elements::Comp::ssd1306(
                    id.to_string(),
                    p.width,
                    p.height,
                    p.control_code,
                    p.color.as_str(),
                    p.rotate,
                    p.freq_khz,
                ));
            }
            Self::Mux(p) => {
                let m = crate::digital::MuxState::new(id, p.addr_bits);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Mux(m),
                });
            }
            Self::Demux(p) => {
                let d = crate::digital::DemuxState::new(id, p.addr_bits, p.inverted);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Demux(d),
                });
            }
            Self::BcdToDec(p) => {
                let b = crate::digital::BcdToDecState::new(id, p.sixteen, p.active_low);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::BcdToDec(b),
                });
            }
            Self::DecToBcd(p) => {
                let d = crate::digital::DecToBcdState::new(id, p.sixteen, p.active_low);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::DecToBcd(d),
                });
            }
            Self::BcdTo7Segment(p) => {
                let b = crate::digital::BcdTo7SState::new(id, p.common_anode);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::BcdTo7S(b),
                });
            }
            Self::SevenSegmentBCD(p) => {
                c.add_comp(crate::elements::Comp::seven_segment_bcd(
                    id.to_string(),
                    p.color.as_str(),
                    p.common_anode,
                ));
            }
            Self::LedMatrix(p) => {
                c.add_comp(crate::elements::Comp::led_matrix(
                    id.to_string(),
                    p.rows,
                    p.cols,
                    p.vertical_pins,
                    p.color.as_str(),
                    p.threshold,
                    p.max_current,
                    p.resistance,
                ));
            }
            Self::Max72xx(p) => {
                c.add_comp(crate::elements::Comp::max72xx(
                    id.to_string(),
                    p.modules,
                    p.color.as_str(),
                ));
            }
            Self::Ws2812(p) => {
                c.add_comp(crate::elements::Comp::ws2812(
                    id.to_string(),
                    p.count,
                    p.rows,
                    p.cols,
                    p.rst_time_ns,
                    p.t0h_ns,
                    p.t0l_ns,
                    p.t1h_ns,
                    p.t1l_ns,
                ));
            }
            Self::Dial(p) => {
                c.add_comp(crate::elements::Comp::dial(
                    id.to_string(),
                    p.value,
                    p.min_val,
                    p.max_val,
                    p.step,
                ));
            }
            Self::Shape(p) => {
                c.add_comp(crate::elements::Comp::shape(
                    id.to_string(),
                    p.shape_kind.as_str(),
                    p.width,
                    p.height,
                    p.text.clone(),
                    p.color.clone(),
                    p.font.clone(),
                    p.font_color.clone(),
                    p.font_size,
                    p.border,
                    p.opacity,
                ));
            }
            Self::I2CToParallel(p) => {
                let ptr = crate::digital::I2CToParallelState::new(id, p.address);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::I2CToParallel(ptr),
                });
            }
            Self::Adc(p) => {
                let a = crate::digital::AdcState::new(id, p.bits, p.vref_pos, p.vref_neg);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Adc(a),
                });
            }
            Self::Dac(p) => {
                let d = crate::digital::DacState::new(id, p.bits, p.vref);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Dac(d),
                });
            }
            Self::Counter(p) => {
                let cnt = crate::digital::CounterState::new(id, p.bits, p.max_count);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Counter(cnt),
                });
            }
            Self::BinCounter(p) => {
                let b = crate::digital::BinCounterState::new(id, p.is_decade);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::BinCounter(b),
                });
            }
            Self::FullAdder(p) => {
                let fa = crate::digital::FullAdderState::new(id, p.bits);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::FullAdder(fa),
                });
            }
            Self::HalfAdder(p) => {
                let fa = crate::digital::FullAdderState::new(id, p.bits);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::HalfAdder(fa),
                });
            }
            Self::MagnitudeComp(p) => {
                let mc = crate::digital::MagnitudeCompState::new(id, p.bits);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::MagnitudeComp(mc),
                });
            }
            Self::ShiftReg(p) => {
                let sr = crate::digital::ShiftRegState::new(id, p.bits);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::ShiftReg(sr),
                });
            }
            Self::Function(p) => {
                let f = crate::digital::FunctionState::new(id, p.n_inputs, &p.expression);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Function(f),
                });
            }
            Self::Memory(p) => {
                let mut m =
                    crate::digital::MemoryState::new(id, p.addr_bits, p.data_bits, p.is_rom);
                if !p.data.is_empty() {
                    let n = m.data.len().min(p.data.len());
                    m.data[..n].copy_from_slice(&p.data[..n]);
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Memory(m),
                });
            }
            Self::DynamicMemory(p) => {
                let mut dm = crate::digital::DynamicMemoryState::new(id, p.addr_bits);
                if !p.data.is_empty() {
                    let n = dm.data.len().min(p.data.len());
                    dm.data[..n].copy_from_slice(&p.data[..n]);
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::DynamicMemory(dm),
                });
            }
            Self::I2CRam(p) => {
                let mut r = crate::digital::I2CRamState::new(id, p.size_bytes, p.dev_address);
                if !p.data.is_empty() {
                    let n = r.data.len().min(p.data.len());
                    r.data[..n].copy_from_slice(&p.data[..n]);
                }
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::I2CRam(r),
                });
            }
            Self::Lm555(_) => {
                let lm = crate::digital::Lm555State::new(id);
                c.add_comp(crate::elements::Comp {
                    id: id.to_string(),
                    kind: crate::elements::Kind::Lm555(lm),
                });
            }
            Self::Bus(_)
            | Self::Socket(_)
            | Self::Header(_)
            | Self::SerialPort(_)
            | Self::SerialTerm(_)
            | Self::SubPackage(_) => {}
        }
    }
}

macro_rules! impl_from_part {
    ($($variant:ident ($type:ident)),* $(,)?) => {
        $(
            impl From<$type> for Part {
                fn from(c: $type) -> Self {
                    Part::$variant(c)
                }
            }
        )*
    };
}

impl_from_part! {
    Resistor(Resistor),
    VarResistor(VarResistor),
    Potentiometer(Potentiometer),
    Capacitor(Capacitor),
    ElCapacitor(ElCapacitor),
    Inductor(Inductor),
    VarCapacitor(VarCapacitor),
    VarInductor(VarInductor),
    Battery(Battery),
    Ground(Ground),
    Node(Node),
    FixedVolt(FixedVolt),
    Rail(Rail),
    Clock(Clock),
    VoltSource(VoltSource),
    CurrSource(CurrSource),
    Switch(Switch),
    Push(Push),
    SwitchDip(SwitchDip),
    Relay(Relay),
    KeyPad(KeyPad),
    Diode(Diode),
    Led(Led),
    Bjt(Bjt),
    Mosfet(Mosfet),
    Jfet(Jfet),
    OpAmp(OpAmp),
    Comparator(Comparator),
    VoltReg(VoltReg),
    Probe(Probe),
    Voltmeter(Voltmeter),
    Ammeter(Ammeter),
    FreqMeter(FreqMeter),
    AndGate(AndGate),
    OrGate(OrGate),
    XorGate(XorGate),
    NotGate(NotGate),
    BufferGate(BufferGate),
    FlipFlop(FlipFlop),
    Latch(Latch),
    Mux(Mux),
    Demux(Demux),
    AnalogMux(AnalogMux),
    BcdToDec(BcdToDec),
    DecToBcd(DecToBcd),
    BcdTo7Segment(BcdTo7Segment),
    FullAdder(FullAdder),
    HalfAdder(HalfAdder),
    TestUnit(TestUnit),
    LogicAnalyzer(LogicAnalyzer),
    TouchPad(TouchPad),
    KY023(KY023),
    KY040(KY040),
    SR04(SR04),
    DHT22(DHT22),
    DS18B20(DS18B20),
    DS1621(DS1621),
    DS1307(DS1307),
    DcMotor(DcMotor),
    Stepper(Stepper),
    Servo(Servo),
    SdCard(SdCard),
    Esp01(Esp01),
    TftDisplay(TftDisplay),
    Pcd8544(Pcd8544),
    Sh1107(Sh1107),
    Ks0108(Ks0108),
    Pcf8833(Pcf8833),
    Aip31068(Aip31068),
    Hd44780(Hd44780),
    Ssd1306(Ssd1306),
    SevenSegment(SevenSegment),
    SevenSegmentBCD(SevenSegmentBCD),
    LedMatrix(LedMatrix),
    Max72xx(Max72xx),
    LedBar(LedBar),
    RgbLed(RgbLed),
    Ws2812(Ws2812),
    Counter(Counter),
    BinCounter(BinCounter),
    ShiftReg(ShiftReg),
    MagnitudeComp(MagnitudeComp),
    Adc(Adc),
    Dac(Dac),
    I2CToParallel(I2CToParallel),
    Lm555(Lm555),
    Memory(Memory),
    DynamicMemory(DynamicMemory),
    I2CRam(I2CRam),
    Function(Function),
    Transformer(Transformer),
    Scr(Scr),
    Diac(Diac),
    Triac(Triac),
    Csource(Csource),
    ResistorDip(ResistorDip),
    Ldr(Ldr),
    Thermistor(Thermistor),
    Rtd(Rtd),
    Strain(Strain),
    AudioOut(AudioOut),
    Lamp(Lamp),
    WaveGen(WaveGen),
    Oscope(Oscope),
    Tunnel(Tunnel),
    Bus(Bus),
    Socket(Socket),
    Header(Header),
    SerialPort(SerialPort),
    SerialTerm(SerialTerm),
    Subcircuit(Subcircuit),
    SubPackage(SubPackage),
    Dial(Dial),
    Shape(Shape),
    Mcu(Mcu),
    QemuDevice(QemuDevice),
}

#[cfg(test)]
mod harness;

#[cfg(test)]
mod tests;
