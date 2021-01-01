//! Component placement and library instantiation on the canvas.

use crate::canvas::events::Change;
use crate::canvas::geom::{self, to_grid as snap_point};
use crate::canvas::{Canvas, Point};
use crate::components::ComponentChange;

/// C++ mime `name,compType`, or a bare type for double-click of library items.
fn split_place(spec: &str) -> (&str, &str) {
    if let Some((name, typ)) = spec.split_once(',') {
        let name = name.trim();
        let typ = typ.trim();
        if !name.is_empty() && !typ.is_empty() {
            return (name, typ);
        }
    }
    let s = spec.trim();
    (s, s)
}

/// Placeable catalog component types.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    Resistor,
    Battery,
    Ground,
    FixedVolt,
    Capacitor,
    ElCapacitor,
    Inductor,
    Switch,
    Diode,
    Zener,
    Led,
    Bjt,
    Mosfet,
    OpAmp,
    Jfet,
    Comparator,
    VoltReg,
    Probe,
    Voltmeter,
    Ammeter,
    FreqMeter,
    Oscope,
    LogicAnalyzer,
    Clock,
    Rail,
    WaveGen,
    VoltSource,
    CurrSource,
    Csource,
    Push,
    SwitchDip,
    Relay,
    KeyPad,
    TouchPad,
    KY023,
    KY040,
    SR04,
    DHT22,
    DS18B20,
    DS1621,
    DS1307,
    DcMotor,
    Stepper,
    Servo,
    SdCard,
    Esp01,
    ILI9341,
    ST7789,
    ST7735,
    GC9A01A,
    TFTDisplay,
    PCD8544,
    SH1107,
    KS0108,
    PCF8833,
    AIP31068,
    Potentiometer,
    VarResistor,
    ResistorDip,
    Ldr,
    Thermistor,
    Rtd,
    Strain,
    VarCapacitor,
    VarInductor,
    Transformer,
    Scr,
    Diac,
    Triac,
    AnalogMux,
    RgbLed,
    LedBar,
    SevenSegment,
    LedMatrix,
    Max72xx,
    Ws2812,
    Hd44780,
    SSD1306,
    AudioOut,
    Lamp,
    Tunnel,
    Bus,
    Socket,
    Header,
    SerialPort,
    SerialTerm,
    Dial,
    SubPackage,
    AndGate,
    OrGate,
    XorGate,
    BufferGate,
    FlipFlopD,
    FlipFlopJK,
    FlipFlopRS,
    FlipFlopT,
    LatchD,
    TestUnit,
    Mux,
    Demux,
    BcdToDec,
    DecToBcd,
    BcdTo7Segment,
    SevenSegmentBCD,
    I2CToParallel,
    Adc,
    Dac,
    Counter,
    BinCounter,
    FullAdder,
    MagnitudeComp,
    ShiftReg,
    Function,
    Memory,
    DynamicMemory,
    I2CRam,
    Lm555,
    Rectangle,
    Ellipse,
    Line,
    TextComponent,
    Image,
    Mcu,
    QemuDevice,
}

impl PlaceKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Resistor => "Resistor",
            Self::Battery => "Battery",
            Self::Ground => "Ground",
            Self::FixedVolt => "FixedVolt",
            Self::Capacitor => "Capacitor",
            Self::ElCapacitor => "ElCapacitor",
            Self::Inductor => "Inductor",
            Self::Switch => "Switch",
            Self::Diode => "Diode",
            Self::Zener => "Zener",
            Self::Led => "Led",
            Self::Bjt => "Bjt",
            Self::Mosfet => "Mosfet",
            Self::OpAmp => "OpAmp",
            Self::Jfet => "Jfet",
            Self::Comparator => "Comparator",
            Self::VoltReg => "VoltReg",
            Self::Probe => "Probe",
            Self::Voltmeter => "Voltmeter",
            Self::Ammeter => "Ammeter",
            Self::FreqMeter => "FreqMeter",
            Self::Oscope => "Oscope",
            Self::LogicAnalyzer => "LogicAnalyzer",
            Self::Clock => "Clock",
            Self::Rail => "Rail",
            Self::WaveGen => "WaveGen",
            Self::VoltSource => "VoltSource",
            Self::CurrSource => "CurrSource",
            Self::Csource => "Csource",
            Self::Push => "Push",
            Self::SwitchDip => "SwitchDip",
            Self::Relay => "Relay",
            Self::KeyPad => "KeyPad",
            Self::TouchPad => "TouchPad",
            Self::KY023 => "KY023",
            Self::KY040 => "KY040",
            Self::SR04 => "SR04",
            Self::DHT22 => "DHT22",
            Self::DS18B20 => "DS18B20",
            Self::DS1621 => "DS1621",
            Self::DS1307 => "DS1307",
            Self::DcMotor => "DcMotor",
            Self::Stepper => "Stepper",
            Self::Servo => "Servo",
            Self::SdCard => "SdCard",
            Self::Esp01 => "Esp01",
            Self::ILI9341 => "ILI9341",
            Self::ST7789 => "ST7789",
            Self::ST7735 => "ST7735",
            Self::GC9A01A => "GC9A01A",
            Self::TFTDisplay => "TFTDisplay",
            Self::PCD8544 => "PCD8544",
            Self::SH1107 => "SH1107",
            Self::KS0108 => "KS0108",
            Self::PCF8833 => "PCF8833",
            Self::AIP31068 => "AIP31068",
            Self::Potentiometer => "Potentiometer",
            Self::VarResistor => "VarResistor",
            Self::ResistorDip => "ResistorDip",
            Self::Ldr => "Ldr",
            Self::Thermistor => "Thermistor",
            Self::Rtd => "Rtd",
            Self::Strain => "Strain",
            Self::VarCapacitor => "VarCapacitor",
            Self::VarInductor => "VarInductor",
            Self::Transformer => "Transformer",
            Self::Scr => "Scr",
            Self::Diac => "Diac",
            Self::Triac => "Triac",
            Self::AnalogMux => "AnalogMux",
            Self::RgbLed => "RgbLed",
            Self::LedBar => "LedBar",
            Self::SevenSegment => "SevenSegment",
            Self::LedMatrix => "LedMatrix",
            Self::Max72xx => "Max72xx",
            Self::Ws2812 => "Ws2812",
            Self::Hd44780 => "Hd44780",
            Self::SSD1306 => "SSD1306",
            Self::AudioOut => "AudioOut",
            Self::Lamp => "Lamp",
            Self::Tunnel => "Tunnel",
            Self::Bus => "Bus",
            Self::Socket => "Socket",
            Self::Header => "Header",
            Self::SerialPort => "SerialPort",
            Self::SerialTerm => "SerialTerm",
            Self::Dial => "Dial",
            Self::SubPackage => "SubPackage",
            Self::AndGate => "AndGate",
            Self::OrGate => "OrGate",
            Self::XorGate => "XorGate",
            Self::BufferGate => "BufferGate",
            Self::FlipFlopD => "FlipFlopD",
            Self::FlipFlopJK => "FlipFlopJK",
            Self::FlipFlopRS => "FlipFlopRS",
            Self::FlipFlopT => "FlipFlopT",
            Self::LatchD => "LatchD",
            Self::TestUnit => "TestUnit",
            Self::Mux => "Mux",
            Self::Demux => "Demux",
            Self::BcdToDec => "BcdToDec",
            Self::DecToBcd => "DecToBcd",
            Self::BcdTo7Segment => "BcdTo7Segment",
            Self::SevenSegmentBCD => "SevenSegmentBCD",
            Self::I2CToParallel => "I2CToParallel",
            Self::Adc => "Adc",
            Self::Dac => "Dac",
            Self::Counter => "Counter",
            Self::BinCounter => "BinCounter",
            Self::FullAdder => "FullAdder",
            Self::MagnitudeComp => "MagnitudeComp",
            Self::ShiftReg => "ShiftReg",
            Self::Function => "Function",
            Self::Memory => "Memory",
            Self::DynamicMemory => "DynamicMemory",
            Self::I2CRam => "I2CRam",
            Self::Lm555 => "Lm555",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::TextComponent => "TextComponent",
            Self::Image => "Image",
            Self::Mcu => "MCU",
            Self::QemuDevice => "QemuDevice",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "Resistor" => Some(Self::Resistor),
            "Battery" => Some(Self::Battery),
            "Ground" => Some(Self::Ground),
            "FixedVolt" => Some(Self::FixedVolt),
            "Capacitor" => Some(Self::Capacitor),
            "ElCapacitor" => Some(Self::ElCapacitor),
            "Inductor" => Some(Self::Inductor),
            "Switch" => Some(Self::Switch),
            "Diode" => Some(Self::Diode),
            "Zener" => Some(Self::Zener),
            "Led" => Some(Self::Led),
            "Bjt" => Some(Self::Bjt),
            "Mosfet" => Some(Self::Mosfet),
            "OpAmp" => Some(Self::OpAmp),
            "Jfet" => Some(Self::Jfet),
            "Comparator" => Some(Self::Comparator),
            "VoltReg" => Some(Self::VoltReg),
            "Probe" => Some(Self::Probe),
            "Voltmeter" => Some(Self::Voltmeter),
            "Ammeter" => Some(Self::Ammeter),
            "FreqMeter" => Some(Self::FreqMeter),
            "Oscope" => Some(Self::Oscope),
            "LogicAnalyzer" => Some(Self::LogicAnalyzer),
            "Clock" => Some(Self::Clock),
            "Rail" => Some(Self::Rail),
            "WaveGen" => Some(Self::WaveGen),
            "VoltSource" => Some(Self::VoltSource),
            "CurrSource" => Some(Self::CurrSource),
            "Csource" => Some(Self::Csource),
            "Push" => Some(Self::Push),
            "SwitchDip" => Some(Self::SwitchDip),
            "Relay" => Some(Self::Relay),
            "KeyPad" => Some(Self::KeyPad),
            "TouchPad" => Some(Self::TouchPad),
            "KY023" => Some(Self::KY023),
            "KY040" => Some(Self::KY040),
            "SR04" => Some(Self::SR04),
            "DHT22" => Some(Self::DHT22),
            "DS18B20" => Some(Self::DS18B20),
            "DS1621" => Some(Self::DS1621),
            "DS1307" => Some(Self::DS1307),
            "DcMotor" => Some(Self::DcMotor),
            "Stepper" => Some(Self::Stepper),
            "Servo" => Some(Self::Servo),
            "SdCard" => Some(Self::SdCard),
            "Esp01" => Some(Self::Esp01),
            "ILI9341" => Some(Self::ILI9341),
            "ST7789" => Some(Self::ST7789),
            "ST7735" => Some(Self::ST7735),
            "GC9A01A" => Some(Self::GC9A01A),
            "TFTDisplay" => Some(Self::TFTDisplay),
            "PCD8544" => Some(Self::PCD8544),
            "SH1107" => Some(Self::SH1107),
            "KS0108" => Some(Self::KS0108),
            "PCF8833" => Some(Self::PCF8833),
            "AIP31068" => Some(Self::AIP31068),
            "Potentiometer" => Some(Self::Potentiometer),
            "VarResistor" => Some(Self::VarResistor),
            "ResistorDip" => Some(Self::ResistorDip),
            "Ldr" => Some(Self::Ldr),
            "Thermistor" => Some(Self::Thermistor),
            "Rtd" => Some(Self::Rtd),
            "Strain" => Some(Self::Strain),
            "VarCapacitor" => Some(Self::VarCapacitor),
            "VarInductor" => Some(Self::VarInductor),
            "Transformer" => Some(Self::Transformer),
            "Scr" => Some(Self::Scr),
            "Diac" => Some(Self::Diac),
            "Triac" => Some(Self::Triac),
            "AnalogMux" => Some(Self::AnalogMux),
            "RgbLed" => Some(Self::RgbLed),
            "LedBar" => Some(Self::LedBar),
            "SevenSegment" => Some(Self::SevenSegment),
            "LedMatrix" => Some(Self::LedMatrix),
            "Max72xx" => Some(Self::Max72xx),
            "Ws2812" => Some(Self::Ws2812),
            "Hd44780" => Some(Self::Hd44780),
            "SSD1306" => Some(Self::SSD1306),
            "AudioOut" => Some(Self::AudioOut),
            "Lamp" => Some(Self::Lamp),
            "Tunnel" => Some(Self::Tunnel),
            "Bus" => Some(Self::Bus),
            "Socket" => Some(Self::Socket),
            "Header" => Some(Self::Header),
            "SerialPort" => Some(Self::SerialPort),
            "SerialTerm" => Some(Self::SerialTerm),
            "Dial" => Some(Self::Dial),
            "SubPackage" => Some(Self::SubPackage),
            "AndGate" => Some(Self::AndGate),
            "OrGate" => Some(Self::OrGate),
            "XorGate" => Some(Self::XorGate),
            "BufferGate" => Some(Self::BufferGate),
            "FlipFlopD" => Some(Self::FlipFlopD),
            "FlipFlopJK" => Some(Self::FlipFlopJK),
            "FlipFlopRS" => Some(Self::FlipFlopRS),
            "FlipFlopT" => Some(Self::FlipFlopT),
            "LatchD" => Some(Self::LatchD),
            "TestUnit" => Some(Self::TestUnit),
            "Mux" => Some(Self::Mux),
            "Demux" => Some(Self::Demux),
            "BcdToDec" => Some(Self::BcdToDec),
            "DecToBcd" => Some(Self::DecToBcd),
            "BcdTo7Segment" => Some(Self::BcdTo7Segment),
            "SevenSegmentBCD" => Some(Self::SevenSegmentBCD),
            "I2CToParallel" => Some(Self::I2CToParallel),
            "Adc" => Some(Self::Adc),
            "Dac" => Some(Self::Dac),
            "Counter" => Some(Self::Counter),
            "BinCounter" => Some(Self::BinCounter),
            "FullAdder" => Some(Self::FullAdder),
            "MagnitudeComp" => Some(Self::MagnitudeComp),
            "ShiftReg" => Some(Self::ShiftReg),
            "Function" => Some(Self::Function),
            "Memory" => Some(Self::Memory),
            "DynamicMemory" => Some(Self::DynamicMemory),
            "I2CRam" => Some(Self::I2CRam),
            "Lm555" => Some(Self::Lm555),
            "Rectangle" => Some(Self::Rectangle),
            "Ellipse" => Some(Self::Ellipse),
            "Line" => Some(Self::Line),
            "TextComponent" | "Text" => Some(Self::TextComponent),
            "Image" => Some(Self::Image),
            "MCU" => Some(Self::Mcu),
            "QemuDevice" => Some(Self::QemuDevice),
            _ => None,
        }
    }
}

impl std::str::FromStr for PlaceKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or(())
    }
}

impl Canvas {
    pub fn add_resistor_at(&mut self, scene: Point) -> Change {
        self.add_component_at("Resistor", scene)
    }

    pub fn add_resistor_at_cursor(&mut self) -> Change {
        self.add_resistor_at(self.last_scene())
    }

    /// Instantiate a component by spec (`name,compType` or just type) at point `p`.
    /// Returns true if the component was recognized and added to `self.scene`.
    pub fn scene_add_component_spec(&mut self, spec: &str, p: Point) -> bool {
        let (name, typ) = split_place(spec);
        let Some(kind) = PlaceKind::from_name(typ) else {
            return false;
        };
        self.scene_add_component_kind(name, kind, p)
    }

    /// Instantiate a component by typed [`PlaceKind`] and `name` at point `p`.
    pub fn scene_add_component_kind(&mut self, name: &str, kind: PlaceKind, p: Point) -> bool {
        match kind {
            PlaceKind::Resistor => {
                self.scene.add_default_resistor(p.x, p.y);
                true
            }
            PlaceKind::Battery => {
                self.scene.add_default_battery(p.x, p.y);
                true
            }
            PlaceKind::Ground => {
                self.scene.add_ground(p.x, p.y);
                true
            }
            PlaceKind::FixedVolt => {
                self.scene.add_default_fixed_volt(p.x, p.y);
                true
            }
            PlaceKind::Capacitor => {
                self.scene.add_default_capacitor(p.x, p.y);
                true
            }
            PlaceKind::ElCapacitor => {
                self.scene.add_default_el_capacitor(p.x, p.y);
                true
            }
            PlaceKind::Inductor => {
                self.scene.add_default_inductor(p.x, p.y);
                true
            }
            PlaceKind::Switch => {
                self.scene.add_switch(p.x, p.y, true);
                true
            }
            PlaceKind::Diode => {
                self.scene.add_diode(p.x, p.y, false);
                true
            }
            PlaceKind::Zener => {
                self.scene.add_diode(p.x, p.y, true);
                true
            }
            PlaceKind::Led => {
                self.scene.add_led(p.x, p.y);
                true
            }
            PlaceKind::Bjt => {
                self.scene.add_bjt(p.x, p.y, false);
                true
            }
            PlaceKind::Mosfet => {
                self.scene.add_mosfet(p.x, p.y, false, false);
                true
            }
            PlaceKind::OpAmp => {
                self.scene.add_opamp(p.x, p.y);
                true
            }
            PlaceKind::Jfet => {
                self.scene.add_jfet(p.x, p.y);
                true
            }
            PlaceKind::Comparator => {
                self.scene.add_comparator(p.x, p.y);
                true
            }
            PlaceKind::VoltReg => {
                self.scene.add_volt_reg(p.x, p.y);
                true
            }
            PlaceKind::Probe => {
                self.scene.add_probe(p.x, p.y);
                true
            }
            PlaceKind::Voltmeter => {
                self.scene.add_voltmeter(p.x, p.y);
                true
            }
            PlaceKind::Ammeter => {
                self.scene.add_ammeter(p.x, p.y);
                true
            }
            PlaceKind::FreqMeter => {
                self.scene.add_freq_meter(p.x, p.y);
                true
            }
            PlaceKind::Oscope => {
                self.scene.add_oscope(p.x, p.y);
                true
            }
            PlaceKind::LogicAnalyzer => {
                self.scene.add_lanalizer(p.x, p.y);
                true
            }
            PlaceKind::Clock => {
                self.scene.add_clock(p.x, p.y);
                true
            }
            PlaceKind::Rail => {
                self.scene.add_rail(p.x, p.y);
                true
            }
            PlaceKind::WaveGen => {
                self.scene.add_wave_gen(p.x, p.y);
                true
            }
            PlaceKind::VoltSource => {
                self.scene.add_volt_source(p.x, p.y);
                true
            }
            PlaceKind::CurrSource => {
                self.scene.add_curr_source(p.x, p.y);
                true
            }
            PlaceKind::Csource => {
                self.scene.add_csource(p.x, p.y);
                true
            }
            PlaceKind::Push => {
                self.scene.add_push(p.x, p.y);
                true
            }
            PlaceKind::SwitchDip => {
                self.scene.add_switch_dip(p.x, p.y);
                true
            }
            PlaceKind::Relay => {
                self.scene.add_relay(p.x, p.y);
                true
            }
            PlaceKind::KeyPad => {
                self.scene.add_keypad(p.x, p.y);
                true
            }
            PlaceKind::TouchPad => {
                self.scene.add_touchpad(p.x, p.y);
                true
            }
            PlaceKind::KY023 => {
                self.scene.add_ky023(p.x, p.y);
                true
            }
            PlaceKind::KY040 => {
                self.scene.add_ky040(p.x, p.y);
                true
            }
            PlaceKind::SR04 => {
                self.scene.add_sr04(p.x, p.y);
                true
            }
            PlaceKind::DHT22 => {
                self.scene.add_dht22(p.x, p.y);
                true
            }
            PlaceKind::DS18B20 => {
                self.scene.add_ds18b20(p.x, p.y);
                true
            }
            PlaceKind::DS1621 => {
                self.scene.add_ds1621(p.x, p.y);
                true
            }
            PlaceKind::DS1307 => {
                self.scene.add_ds1307(p.x, p.y);
                true
            }
            PlaceKind::DcMotor => {
                self.scene.add_dcmotor(p.x, p.y);
                true
            }
            PlaceKind::Stepper => {
                self.scene.add_stepper(p.x, p.y);
                true
            }
            PlaceKind::Servo => {
                self.scene.add_servo(p.x, p.y);
                true
            }
            PlaceKind::SdCard => {
                self.scene.add_sdcard(p.x, p.y);
                true
            }
            PlaceKind::Esp01 => {
                self.scene.add_esp01(p.x, p.y);
                true
            }
            PlaceKind::ILI9341 => {
                self.scene.add_tft_display(p.x, p.y, "ILI9341", 320, 240);
                true
            }
            PlaceKind::ST7789 => {
                self.scene.add_tft_display(p.x, p.y, "ST7789", 240, 240);
                true
            }
            PlaceKind::ST7735 => {
                self.scene.add_tft_display(p.x, p.y, "ST7735", 160, 128);
                true
            }
            PlaceKind::GC9A01A => {
                self.scene.add_tft_display(p.x, p.y, "GC9A01A", 240, 240);
                true
            }
            PlaceKind::TFTDisplay => {
                self.scene.add_tft_display(p.x, p.y, "ILI9341", 320, 240);
                true
            }
            PlaceKind::PCD8544 => {
                self.scene.add_pcd8544(p.x, p.y);
                true
            }
            PlaceKind::SH1107 => {
                self.scene.add_sh1107(p.x, p.y, 128, 128);
                true
            }
            PlaceKind::KS0108 => {
                self.scene.add_ks0108(p.x, p.y);
                true
            }
            PlaceKind::PCF8833 => {
                self.scene.add_pcf8833(p.x, p.y);
                true
            }
            PlaceKind::AIP31068 => {
                self.scene.add_aip31068(p.x, p.y, 2, 16);
                true
            }
            PlaceKind::Potentiometer => {
                self.scene.add_potentiometer(p.x, p.y);
                true
            }
            PlaceKind::VarResistor => {
                self.scene.add_var_resistor(p.x, p.y);
                true
            }
            PlaceKind::ResistorDip => {
                self.scene.add_resistor_dip(p.x, p.y);
                true
            }
            PlaceKind::Ldr => {
                self.scene.add_ldr(p.x, p.y);
                true
            }
            PlaceKind::Thermistor => {
                self.scene.add_thermistor(p.x, p.y);
                true
            }
            PlaceKind::Rtd => {
                self.scene.add_rtd(p.x, p.y);
                true
            }
            PlaceKind::Strain => {
                self.scene.add_strain(p.x, p.y);
                true
            }
            PlaceKind::VarCapacitor => {
                self.scene.add_var_capacitor(p.x, p.y);
                true
            }
            PlaceKind::VarInductor => {
                self.scene.add_var_inductor(p.x, p.y);
                true
            }
            PlaceKind::Transformer => {
                self.scene.add_transformer(p.x, p.y);
                true
            }
            PlaceKind::Scr => {
                self.scene.add_scr(p.x, p.y);
                true
            }
            PlaceKind::Diac => {
                self.scene.add_diac(p.x, p.y);
                true
            }
            PlaceKind::Triac => {
                self.scene.add_triac(p.x, p.y);
                true
            }
            PlaceKind::AnalogMux => {
                self.scene.add_analog_mux(p.x, p.y);
                true
            }
            PlaceKind::RgbLed => {
                self.scene.add_rgb_led(p.x, p.y);
                true
            }
            PlaceKind::LedBar => {
                self.scene.add_led_bar(p.x, p.y);
                true
            }
            PlaceKind::SevenSegment => {
                self.scene.add_seven_segment(p.x, p.y);
                true
            }
            PlaceKind::LedMatrix => {
                self.scene.add_led_matrix(p.x, p.y);
                true
            }
            PlaceKind::Max72xx => {
                self.scene.add_max72xx(p.x, p.y);
                true
            }
            PlaceKind::Ws2812 => {
                self.scene.add_ws2812(p.x, p.y);
                true
            }
            PlaceKind::Hd44780 => {
                self.scene.add_hd44780(p.x, p.y);
                true
            }
            PlaceKind::SSD1306 => {
                self.scene.add_ssd1306(p.x, p.y);
                true
            }
            PlaceKind::AudioOut => {
                self.scene.add_audio_out(p.x, p.y);
                true
            }
            PlaceKind::Lamp => {
                self.scene.add_lamp(p.x, p.y);
                true
            }
            PlaceKind::Tunnel => {
                self.scene.add_tunnel(p.x, p.y);
                true
            }
            PlaceKind::Bus => {
                self.scene.add_bus(p.x, p.y);
                true
            }
            PlaceKind::Socket => {
                self.scene.add_socket(p.x, p.y);
                true
            }
            PlaceKind::Header => {
                self.scene.add_header(p.x, p.y);
                true
            }
            PlaceKind::SerialPort => {
                self.scene.add_serial_port(p.x, p.y);
                true
            }
            PlaceKind::SerialTerm => {
                self.scene.add_serial_term(p.x, p.y);
                true
            }
            PlaceKind::Dial => {
                self.scene.add_dial(p.x, p.y);
                true
            }
            PlaceKind::SubPackage => {
                self.scene.add_subpackage(p.x, p.y);
                true
            }
            PlaceKind::AndGate => {
                self.scene.add_and_gate(p.x, p.y);
                true
            }
            PlaceKind::OrGate => {
                self.scene.add_or_gate(p.x, p.y);
                true
            }
            PlaceKind::XorGate => {
                self.scene.add_xor_gate(p.x, p.y);
                true
            }
            PlaceKind::BufferGate => {
                self.scene.add_buffer(p.x, p.y);
                true
            }
            PlaceKind::FlipFlopD => {
                self.scene.add_flipflop_d(p.x, p.y);
                true
            }
            PlaceKind::FlipFlopJK => {
                self.scene.add_flipflop_jk(p.x, p.y);
                true
            }
            PlaceKind::FlipFlopRS => {
                self.scene.add_flipflop_rs(p.x, p.y);
                true
            }
            PlaceKind::FlipFlopT => {
                self.scene.add_flipflop_t(p.x, p.y);
                true
            }
            PlaceKind::LatchD => {
                self.scene.add_latch_d(p.x, p.y);
                true
            }
            PlaceKind::TestUnit => {
                self.scene.add_test_unit(p.x, p.y);
                true
            }
            PlaceKind::Mux => {
                self.scene.add_mux(p.x, p.y);
                true
            }
            PlaceKind::Demux => {
                self.scene.add_demux(p.x, p.y);
                true
            }
            PlaceKind::BcdToDec => {
                self.scene.add_bcd_to_dec(p.x, p.y);
                true
            }
            PlaceKind::DecToBcd => {
                self.scene.add_dec_to_bcd(p.x, p.y);
                true
            }
            PlaceKind::BcdTo7Segment => {
                self.scene.add_bcd_to_7s(p.x, p.y);
                true
            }
            PlaceKind::SevenSegmentBCD => {
                self.scene.add_seven_segment_bcd(p.x, p.y);
                true
            }
            PlaceKind::I2CToParallel => {
                self.scene.add_i2c_to_parallel(p.x, p.y);
                true
            }
            PlaceKind::Adc => {
                self.scene.add_adc(p.x, p.y);
                true
            }
            PlaceKind::Dac => {
                self.scene.add_dac(p.x, p.y);
                true
            }
            PlaceKind::Counter => {
                self.scene.add_counter(p.x, p.y);
                true
            }
            PlaceKind::BinCounter => {
                self.scene.add_bin_counter(p.x, p.y);
                true
            }
            PlaceKind::FullAdder => {
                self.scene.add_full_adder(p.x, p.y);
                true
            }
            PlaceKind::MagnitudeComp => {
                self.scene.add_magnitude_comp(p.x, p.y);
                true
            }
            PlaceKind::ShiftReg => {
                self.scene.add_shift_reg(p.x, p.y);
                true
            }
            PlaceKind::Function => {
                self.scene.add_function(p.x, p.y);
                true
            }
            PlaceKind::Memory => {
                self.scene.add_memory(p.x, p.y);
                true
            }
            PlaceKind::DynamicMemory => {
                self.scene.add_dynamic_memory(p.x, p.y);
                true
            }
            PlaceKind::I2CRam => {
                self.scene.add_i2c_ram(p.x, p.y);
                true
            }
            PlaceKind::Lm555 => {
                self.scene.add_lm555(p.x, p.y);
                true
            }
            PlaceKind::Rectangle
            | PlaceKind::Ellipse
            | PlaceKind::Line
            | PlaceKind::TextComponent
            | PlaceKind::Image => {
                self.scene.add_shape(name, p.x, p.y);
                true
            }
            PlaceKind::Mcu if name != "MCU" && name != "NEW_MCU" => {
                self.scene.add_mcu(name, p.x, p.y, &self.search()).is_some()
            }
            PlaceKind::Mcu => false,
            PlaceKind::QemuDevice => self
                .scene
                .add_qemu(name, p.x, p.y, &self.search())
                .is_some(),
        }
    }

    /// Place a catalog type at a scene point. `spec` is `name,compType` (C++
    /// mime) or just the type. Unknown types are ignored.
    pub fn add_component_at(&mut self, spec: &str, scene: Point) -> Change {
        self.push_undo();
        let p = snap_point(scene);
        let added = self.scene_add_component_spec(spec, p);
        if !added {
            self.history.undo.pop();
            return Change::default();
        }
        let id = self
            .scene
            .items()
            .last()
            .map(|it| it.id.clone())
            .unwrap_or_default();
        self.apply_component_change(ComponentChange::document(id).no_undo())
    }

    /// Place all available catalog components onto the canvas in an organized grid/row layout
    /// starting at `self.last_scene()`.
    pub fn add_all_components(&mut self) -> Change {
        self.add_all_components_at(self.last_scene())
    }

    /// Place all available catalog components onto the canvas in an organized grid/row layout
    /// starting at the specified `start` scene coordinate.
    /// Computes each component's width and height, aligns top-left coordinates to grid points,
    /// and ensures clear spacing between all components without overlapping.
    pub fn add_all_components_at(&mut self, start: Point) -> Change {
        self.push_undo();

        let start = snap_point(start);
        let start_x = start.x;
        let start_y = start.y;
        let max_row_width = 1200.0;
        let spacing_x = 48.0;
        let spacing_y = 48.0;

        let mut cur_x = start_x;
        let mut cur_y = start_y;
        let mut row_max_h = 0.0f64;

        let mut all_specs: Vec<String> = crate::library::all_placeable_specs()
            .into_iter()
            .map(|(caption, typ)| {
                if typ == "QemuDevice" || typ == "MCU" {
                    format!("{caption},{typ}")
                } else {
                    typ
                }
            })
            .collect();

        // If MCU definitions are present in catalog, include a sample MCU
        let cat = crate::catalog::standard();
        if let Some(mcu) = cat.mcu_items().next() {
            let mcu_spec = format!("{},MCU", mcu.name);
            if !all_specs.contains(&mcu_spec) {
                all_specs.push(mcu_spec);
            }
        }

        for spec in &all_specs {
            let prev_len = self.scene.items().len();
            if !self.scene_add_component_spec(spec, Point::zero())
                || self.scene.items().len() == prev_len
            {
                continue;
            }

            let item = self.scene.items_mut().last_mut().unwrap();
            let local_rect = item.local_hit_rect();
            let comp_w = local_rect.w.max(24.0);
            let comp_h = local_rect.h.max(24.0);

            // Wrap to next row if this component would exceed the maximum row width
            if cur_x > start_x && (cur_x + comp_w - start_x) > max_row_width {
                cur_x = start_x;
                cur_y += row_max_h + spacing_y;
                cur_y = (geom::snap_to_grid4(cur_y as i32) as f64).max(cur_y);
                row_max_h = 0.0;
            }

            // Position component so its bounding box starts at (cur_x, cur_y)
            let scene_x = geom::snap_to_grid4((cur_x - local_rect.x) as i32) as f64;
            let scene_y = geom::snap_to_grid4((cur_y - local_rect.y) as i32) as f64;
            item.x = scene_x;
            item.y = scene_y;

            row_max_h = row_max_h.max(comp_h);
            cur_x += comp_w + spacing_x;
            cur_x = (geom::snap_to_grid4(cur_x as i32) as f64).max(cur_x);
        }

        self.dirty.mark_full();
        let mut c = Change::edit();
        c.merge(self.refresh_sim());
        c
    }
}
