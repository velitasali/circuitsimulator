//! Per-property harness: every persist prop, two extra values, circ1 1.0.0, dial.

use super::*;
use crate::elements::Kind;
use crate::settings::CircSettings;

pub(crate) struct Case {
    pub(crate) itemtype: &'static str,
    pub(crate) make: fn() -> Part,
    pub(crate) extras: &'static [(&'static str, &'static str, &'static str)],
    pub(crate) dial_prop: Option<&'static str>,
    pub(crate) dial_value: f64,
}

pub(crate) const CASES: &[Case] = &[
    Case {
        itemtype: "Resistor",
        make: || Part::Resistor(Resistor::default()),
        extras: &[
            ("Resistance", "4.7 kΩ", "10 Ω"),
            ("ShowBands", "false", "true"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "VarResistor",
        make: || Part::VarResistor(VarResistor::default()),
        extras: &[
            ("Resistance", "1.5 kΩ", "500 Ω"),
            ("MinResistance", "10 Ω", "50 Ω"),
            ("MaxResistance", "5 kΩ", "10 kΩ"),
            ("Key", "A", "B"),
            ("DialStep", "10 Ω", "100 Ω"),
        ],
        dial_prop: Some("Resistance"),
        dial_value: 1_500.0,
    },
    Case {
        itemtype: "Potentiometer",
        make: || Part::Potentiometer(Potentiometer::default()),
        extras: &[
            ("Resistance", "4.7 kΩ", "10 kΩ"),
            ("Wiper", "0.25", "0.75"),
            ("Key", "A", "B"),
            ("DialStep", "5 %", "2 %"),
        ],
        dial_prop: Some("Wiper"),
        dial_value: 0.25,
    },
    Case {
        itemtype: "Capacitor",
        make: || Part::Capacitor(Capacitor::default()),
        extras: &[
            ("Capacitance", "22 µF", "100 nF"),
            ("Resistance", "10 mΩ", "1 Ω"),
            ("InitVolt", "1 V", "0 V"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "ElCapacitor",
        make: || Part::ElCapacitor(ElCapacitor::default()),
        extras: &[
            ("Capacitance", "100 µF", "47 µF"),
            ("Resistance", "50 mΩ", "1 Ω"),
            ("InitVolt", "5 V", "0 V"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Inductor",
        make: || Part::Inductor(Inductor::default()),
        extras: &[
            ("Inductance", "10 mH", "2 H"),
            ("Resistance", "2 Ω", "10 mΩ"),
            ("InitVolt", "10 mA", "0 A"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "VarCapacitor",
        make: || Part::VarCapacitor(VarCapacitor::default()),
        extras: &[
            ("Capacitance", "50 pF", "150 pF"),
            ("MinCapacitance", "10 pF", "20 pF"),
            ("MaxCapacitance", "300 pF", "1 nF"),
            ("Resistance", "10 mΩ", "1 Ω"),
            ("InitVolt", "1 V", "0 V"),
            ("DialStep", "2 pF", "5 pF"),
        ],
        dial_prop: Some("Capacitance"),
        dial_value: 50e-12,
    },
    Case {
        itemtype: "VarInductor",
        make: || Part::VarInductor(VarInductor::default()),
        extras: &[
            ("Inductance", "500 µH", "1.5 mH"),
            ("MinInductance", "100 µH", "200 µH"),
            ("MaxInductance", "3 mH", "10 mH"),
            ("Resistance", "10 mΩ", "1 Ω"),
            ("InitVolt", "1 mA", "0 A"),
            ("DialStep", "2 µH", "5 µH"),
        ],
        dial_prop: Some("Inductance"),
        dial_value: 500e-6,
    },
    Case {
        itemtype: "Battery",
        make: || Part::Battery(Battery::default()),
        extras: &[("Voltage", "9 V", "1.5 V"), ("Resistance", "1 Ω", "10 mΩ")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Ground",
        make: || Part::Ground(Ground),
        extras: &[],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Node",
        make: || Part::Node(Node),
        extras: &[],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "FixedVolt",
        make: || Part::FixedVolt(FixedVolt::default()),
        extras: &[("Voltage", "3.3 V", "12 V"), ("Small", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Rail",
        make: || Part::Rail(Rail::default()),
        extras: &[("Voltage", "12 V", "3.3 V")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Clock",
        make: || Part::Clock(Clock::default()),
        extras: &[
            ("Voltage", "3.3 V", "12 V"),
            ("Frequency", "10 kHz", "100 Hz"),
            ("Small", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "VoltSource",
        make: || Part::VoltSource(VoltSource::default()),
        extras: &[
            ("Value", "2.5 V", "1 V"),
            ("MaxValue", "10 V", "5 V"),
            ("MinValue", "1 V", "0 V"),
            ("Running", "false", "true"),
        ],
        dial_prop: Some("Value"),
        dial_value: 2.5,
    },
    Case {
        itemtype: "CurrSource",
        make: || Part::CurrSource(CurrSource::default()),
        extras: &[
            ("Value", "100 mA", "1 A"),
            ("MaxValue", "2 A", "500 mA"),
            ("MinValue", "1 mA", "0 A"),
            ("Running", "false", "true"),
        ],
        dial_prop: Some("Value"),
        dial_value: 0.1,
    },
    Case {
        itemtype: "Switch",
        make: || Part::Switch(Switch::default()),
        extras: &[
            ("NormClose", "true", "false"),
            ("DoubleThrow", "true", "false"),
            ("Poles", "2", "3"),
            ("Key", "A", "B"),
            ("ShowButton", "true", "false"),
            ("Checked", "false", "true"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Push",
        make: || Part::Push(Push::default()),
        extras: &[
            ("NormClose", "true", "false"),
            ("Poles", "2", "4"),
            ("Key", "A", "B"),
            ("ShowButton", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SwitchDip",
        make: || Part::SwitchDip(SwitchDip::default()),
        extras: &[
            ("Size", "8", "2"),
            ("Exclusive", "true", "false"),
            ("CommonPin", "true", "false"),
            ("State", "5", "0"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Relay",
        make: || Part::Relay(Relay::default()),
        extras: &[
            ("NormClose", "true", "false"),
            ("DoubleThrow", "true", "false"),
            ("Poles", "2", "4"),
            ("IOn", "50 mA", "10 mA"),
            ("IOff", "5 mA", "20 mA"),
            ("Inductance", "10 mH", "1 mH"),
            ("Rcoil", "200 Ω", "50 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "KeyPad",
        make: || Part::KeyPad(KeyPad::default()),
        extras: &[
            ("Rows", "3", "5"),
            ("Cols", "3", "5"),
            ("Key", "ABCD", "1234"),
            ("Diodes", "true", "false"),
            ("Dir", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Diode",
        make: || Part::Diode(Diode::default()),
        extras: &[
            ("Threshold", "800 mV", "1.2 V"),
            ("MaxCurrent", "500 mA", "2 A"),
            ("Resistance", "500 mΩ", "20 mΩ"),
            ("BrkDownV", "75 V", "100 V"),
            ("SatCurrent", "10 nA", "50 nA"),
            ("EmCoef", "1.5", "2"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Zener",
        make: || Part::Diode(Diode::zener_default()),
        extras: &[
            ("Threshold", "5.1 V", "3.3 V"),
            ("MaxCurrent", "500 mA", "2 A"),
            ("Resistance", "500 mΩ", "20 mΩ"),
            ("BrkDownV", "5.1 V", "3.3 V"),
            ("SatCurrent", "10 nA", "50 nA"),
            ("EmCoef", "1.5", "2"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Led",
        make: || Part::Led(Led::default()),
        extras: &[
            ("Color", "Red", "Blue"),
            ("Grounded", "true", "false"),
            ("Threshold", "1.8 V", "3.2 V"),
            ("MaxCurrent", "20 mA", "50 mA"),
            ("Resistance", "1 Ω", "200 mΩ"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Bjt",
        make: || Part::Bjt(Bjt::default()),
        extras: &[
            ("PNP", "true", "false"),
            ("Gain", "200", "50"),
            ("Vcrit", "800 mV", "600 mV"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Mosfet",
        make: || Part::Mosfet(Mosfet::default()),
        extras: &[
            ("PChannel", "true", "false"),
            ("Depletion", "true", "false"),
            ("RDSon", "500 mΩ", "2 Ω"),
            ("Threshold", "2 V", "4.5 V"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Jfet",
        make: || Part::Jfet(Jfet::default()),
        extras: &[
            ("PChannel", "true", "false"),
            ("Idss", "20 mA", "100 mA"),
            ("Vp", "-2 V", "-4 V"),
            ("LambdaInv", "500 V", "200 V"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "OpAmp",
        make: || Part::OpAmp(OpAmp::default()),
        extras: &[
            ("Gain", "5000", "20000"),
            ("OutImped", "50 Ω", "10 Ω"),
            ("VoltPos", "15 V", "12 V"),
            ("VoltNeg", "-15 V", "-12 V"),
            ("PowerPins", "true", "false"),
            ("SwitchPins", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Comparator",
        make: || Part::Comparator(Comparator::default()),
        extras: &[
            ("OutHigh", "3.3 V", "12 V"),
            ("OutLow", "500 mV", "-5 V"),
            ("OutImped", "50 Ω", "20 Ω"),
            ("Inverted", "true", "false"),
            ("OpenCollector", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "VoltReg",
        make: || Part::VoltReg(VoltReg::default()),
        extras: &[("Voltage", "5 V", "3.3 V")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Probe",
        make: || Part::Probe(Probe::default()),
        extras: &[
            ("Threshold", "1.5 V", "3.3 V"),
            ("Small", "true", "false"),
            ("ShowVolt", "true", "false"),
            ("PauseAtChange", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Voltmeter",
        make: || Part::Voltmeter(Voltmeter::default()),
        extras: &[("Rms", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Ammeter",
        make: || Part::Ammeter(Ammeter::default()),
        extras: &[("Rms", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "FreqMeter",
        make: || Part::FreqMeter(FreqMeter::default()),
        extras: &[("Filter", "1 V", "500 mV")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "AndGate",
        make: || Part::AndGate(AndGate::default()),
        extras: &[
            ("NumInputs", "3", "4"),
            ("InvertInputs", "true", "false"),
            ("InvertOutputs", "true", "false"),
            ("InitHigh", "true", "false"),
            ("Tristate", "true", "false"),
            ("Small", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "OrGate",
        make: || Part::OrGate(OrGate::default()),
        extras: &[
            ("NumInputs", "3", "4"),
            ("InvertInputs", "true", "false"),
            ("InvertOutputs", "true", "false"),
            ("InitHigh", "true", "false"),
            ("Tristate", "true", "false"),
            ("Small", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "XorGate",
        make: || Part::XorGate(XorGate::default()),
        extras: &[
            ("NumInputs", "3", "4"),
            ("InvertInputs", "true", "false"),
            ("InvertOutputs", "true", "false"),
            ("InitHigh", "true", "false"),
            ("Tristate", "true", "false"),
            ("Small", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "NotGate",
        make: || Part::NotGate(NotGate::default()),
        extras: &[
            ("InvertOutputs", "false", "true"),
            ("InitHigh", "true", "false"),
            ("Tristate", "true", "false"),
            ("Small", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "BufferGate",
        make: || Part::BufferGate(BufferGate::default()),
        extras: &[
            ("InvertOutputs", "true", "false"),
            ("InitHigh", "true", "false"),
            ("Tristate", "true", "false"),
            ("Small", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "FlipFlop",
        make: || Part::FlipFlop(FlipFlop::default()),
        extras: &[
            ("Kind", "JK", "T"),
            ("UseRS", "true", "false"),
            ("Trigger", "Enable", "None"),
            ("ResetInverted", "false", "true"),
            ("ClockInverted", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Latch",
        make: || Part::Latch(Latch::default()),
        extras: &[
            ("Channels", "4", "16"),
            ("UseReset", "true", "false"),
            ("Tristate", "true", "false"),
            ("Trigger", "Enable", "None"),
            ("InvertInputs", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Mux",
        make: || Part::Mux(Mux::default()),
        extras: &[("AddrBits", "3", "4")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Demux",
        make: || Part::Demux(Demux::default()),
        extras: &[("AddrBits", "3", "4"), ("Inverted", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "AnalogMux",
        make: || Part::AnalogMux(AnalogMux::default()),
        extras: &[("Channels", "8", "2"), ("RDSon", "100 Ω", "20 Ω")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "BcdToDec",
        make: || Part::BcdToDec(BcdToDec::default()),
        extras: &[("Sixteen", "true", "false"), ("ActiveLow", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DecToBcd",
        make: || Part::DecToBcd(DecToBcd::default()),
        extras: &[("Sixteen", "true", "false"), ("ActiveLow", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "BcdTo7Segment",
        make: || Part::BcdTo7Segment(BcdTo7Segment::default()),
        extras: &[("CommonAnode", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "FullAdder",
        make: || Part::FullAdder(FullAdder::default()),
        extras: &[("Bits", "8", "2")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "HalfAdder",
        make: || Part::HalfAdder(HalfAdder::default()),
        extras: &[("Bits", "8", "2")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "TestUnit",
        make: || Part::TestUnit(TestUnit::default()),
        extras: &[
            ("Inputs", "A,B,C", "X"),
            ("Outputs", "Y0,Y1", "Z"),
            ("Period", "50 ns", "200 ns"),
            ("Truth", "1,2,3,", "0,f,"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "LogicAnalyzer",
        make: || Part::LogicAnalyzer(LogicAnalyzer::default()),
        extras: &[
            ("connectGnd", "false", "true"),
            ("InputImped", "1 MΩ", "500 kΩ"),
            ("BufferSize", "100000", "50000"),
            ("Basic_X", "200", "300"),
            ("Basic_Y", "150", "250"),
            ("TimeStep", "500", "2000"),
            ("DoTest", "true", "false"),
            ("AutoExport", "true", "false"),
            ("TestTime", "10 µs", "100 µs"),
            ("Tunnel1", "SIG_A", "CLK"),
            ("Tunnel2", "SIG_B", "DATA"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "TouchPad",
        make: || Part::TouchPad(TouchPad::default()),
        extras: &[
            ("Width", "320", "480"),
            ("Height", "240", "640"),
            ("Transparent", "true", "false"),
            ("RxMin", "200 Ω", "50 Ω"),
            ("RxMax", "1 kΩ", "2 kΩ"),
            ("RyMin", "200 Ω", "50 Ω"),
            ("RyMax", "1 kΩ", "2 kΩ"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "KY023",
        make: || Part::KY023(KY023::default()),
        extras: &[
            ("StickX", "0.5", "-0.5"),
            ("StickY", "0.25", "-0.75"),
            ("BtnDown", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "KY040",
        make: || Part::KY040(KY040::default()),
        extras: &[
            ("Steps", "30", "12"),
            ("DialVal", "5", "-3"),
            ("BtnClosed", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SR04",
        make: || Part::SR04(SR04::default()),
        extras: &[("Distance", "1.5 m", "200 mm"), ("Slider", "false", "true")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DHT22",
        make: || Part::DHT22(DHT22::default()),
        extras: &[
            ("Model", "DHT11", "DHT22"),
            ("Temp", "30 °C", "0 °C"),
            ("Humi", "60 %", "45 %"),
            ("TempInc", "1 °C", "2 °C"),
            ("HumiInc", "2 %", "5 %"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DS18B20",
        make: || Part::DS18B20(DS18B20::default()),
        extras: &[
            ("Rom", "28AA33110000", "280000000000"),
            ("Temp", "30 °C", "0 °C"),
            ("TempInc", "1 °C", "2 °C"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DS1621",
        make: || Part::DS1621(DS1621::default()),
        extras: &[("Temp", "30 °C", "0 °C"), ("TempInc", "1 °C", "2 °C")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DS1307",
        make: || Part::DS1307(DS1307::default()),
        extras: &[("TimeUpdated", "false", "true")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DcMotor",
        make: || Part::DcMotor(DcMotor::default()),
        extras: &[
            ("RpmNominal", "2000", "5000"),
            ("VoltNominal", "6 V", "24 V"),
            ("Resistance", "5 Ω", "20 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Stepper",
        make: || Part::Stepper(Stepper::default()),
        extras: &[
            ("Bipolar", "false", "true"),
            ("Steps", "200", "100"),
            ("Resistance", "5 Ω", "50 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Servo",
        make: || Part::Servo(Servo::default()),
        extras: &[
            ("Speed", "0.1", "0.5"),
            ("MinPulse", "800", "1200"),
            ("MaxPulse", "1800", "2200"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SdCard",
        make: || Part::SdCard(SdCard::default()),
        extras: &[("File", "disk.img", "data.bin")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Esp01",
        make: || Part::Esp01(Esp01::default()),
        extras: &[("Baudrate", "9600", "57600"), ("Debug", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "TftDisplay",
        make: || Part::TftDisplay(TftDisplay::default()),
        extras: &[
            ("Controller", "ILI9341", "ST7789"),
            ("Width", "160", "240"),
            ("Height", "128", "320"),
            ("Scale", "1.5", "0.5"),
            ("BGR", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Pcd8544",
        make: || Part::Pcd8544(Pcd8544::default()),
        extras: &[("Contrast", "60", "40"), ("Bias", "5", "3")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Sh1107",
        make: || Part::Sh1107(Sh1107::default()),
        extras: &[("Width", "128", "64"), ("Height", "64", "128")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Ks0108",
        make: || Part::Ks0108(Ks0108::default()),
        extras: &[("Width", "192", "64"), ("Height", "128", "64")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Pcf8833",
        make: || Part::Pcf8833(Pcf8833::default()),
        extras: &[("Width", "128", "132"), ("Height", "128", "132")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Aip31068",
        make: || Part::Aip31068(Aip31068::default()),
        extras: &[("Rows", "4", "1"), ("Cols", "20", "8")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Hd44780",
        make: || Part::Hd44780(Hd44780::default()),
        extras: &[("Rows", "4", "1"), ("Cols", "20", "8")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Ssd1306",
        make: || Part::Ssd1306(Ssd1306::default()),
        extras: &[
            ("Color", "Blue", "Yellow"),
            ("Width", "64", "256"),
            ("Height", "32", "128"),
            ("Rotate", "false", "true"),
            ("Control_Code", "62", "58"),
            ("Frequency", "400 kHz", "200 kHz"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SevenSegment",
        make: || Part::SevenSegment(SevenSegment::default()),
        extras: &[
            ("CommonAnode", "true", "false"),
            ("NumDisplays", "2", "4"),
            ("VerticalPins", "true", "false"),
            ("Color", "Green", "Blue"),
            ("Threshold", "2.2 V", "1.6 V"),
            ("MaxCurrent", "20 mA", "40 mA"),
            ("Resistance", "150 Ω", "220 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SevenSegmentBCD",
        make: || Part::SevenSegmentBCD(SevenSegmentBCD::default()),
        extras: &[("Color", "Green", "Blue"), ("CommonAnode", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "LedMatrix",
        make: || Part::LedMatrix(LedMatrix::default()),
        extras: &[
            ("Rows", "16", "4"),
            ("Cols", "16", "4"),
            ("VerticalPins", "true", "false"),
            ("Color", "Green", "Blue"),
            ("Threshold", "2.2 V", "1.6 V"),
            ("MaxCurrent", "20 mA", "40 mA"),
            ("Resistance", "150 Ω", "220 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Max72xx",
        make: || Part::Max72xx(Max72xx::default()),
        extras: &[("Modules", "2", "4"), ("Color", "Green", "Blue")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "LedBar",
        make: || Part::LedBar(LedBar::default()),
        extras: &[
            ("Segments", "8", "16"),
            ("Color", "Green", "Blue"),
            ("Grounded", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "RgbLed",
        make: || Part::RgbLed(RgbLed::default()),
        extras: &[
            ("CommonAnode", "true", "false"),
            ("ThresholdR", "2.2 V", "1.6 V"),
            ("MaxCurrentR", "25 mA", "30 mA"),
            ("ResistanceR", "120 Ω", "150 Ω"),
            ("ThresholdG", "3.2 V", "2.8 V"),
            ("MaxCurrentG", "25 mA", "30 mA"),
            ("ResistanceG", "120 Ω", "150 Ω"),
            ("ThresholdB", "3.4 V", "2.8 V"),
            ("MaxCurrentB", "25 mA", "30 mA"),
            ("ResistanceB", "120 Ω", "150 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Ws2812",
        make: || Part::Ws2812(Ws2812::default()),
        extras: &[
            ("Count", "16", "64"),
            ("Rows", "2", "8"),
            ("Cols", "8", "16"),
            ("RstTime", "60000", "40000"),
            ("T0H", "350", "450"),
            ("T0L", "900", "800"),
            ("T1H", "750", "850"),
            ("T1L", "500", "400"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Counter",
        make: || Part::Counter(Counter::default()),
        extras: &[("Bits", "8", "12"), ("MaxCount", "255", "4095")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "BinCounter",
        make: || Part::BinCounter(BinCounter::default()),
        extras: &[("IsDecade", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "ShiftReg",
        make: || Part::ShiftReg(ShiftReg::default()),
        extras: &[("Bits", "16", "4")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "MagnitudeComp",
        make: || Part::MagnitudeComp(MagnitudeComp::default()),
        extras: &[("Bits", "8", "2")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Adc",
        make: || Part::Adc(Adc::default()),
        extras: &[
            ("Bits", "10", "12"),
            ("VrefPos", "3.3 V", "2.5 V"),
            ("VrefNeg", "-2.5 V", "-5 V"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Dac",
        make: || Part::Dac(Dac::default()),
        extras: &[("Bits", "10", "12"), ("Vref", "3.3 V", "2.5 V")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "I2CToParallel",
        make: || Part::I2CToParallel(I2CToParallel::default()),
        extras: &[("Address", "33", "39")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Lm555",
        make: || Part::Lm555(Lm555::default()),
        extras: &[("OutHighV", "3.3 V", "12 V"), ("OutLowV", "1 V", "-1 V")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Memory",
        make: || Part::Memory(Memory::default()),
        extras: &[
            ("AddrBits", "10", "4"),
            ("DataBits", "16", "4"),
            ("IsRom", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "DynamicMemory",
        make: || Part::DynamicMemory(DynamicMemory::default()),
        extras: &[("AddrBits", "10", "12")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "I2CRam",
        make: || Part::I2CRam(I2CRam::default()),
        extras: &[("SizeBytes", "512", "1024"), ("DevAddress", "82", "84")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Function",
        make: || Part::Function(Function::default()),
        extras: &[("Inputs", "4", "2"), ("Expression", "!I0 & I1", "I0 | I1")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Transformer",
        make: || Part::Transformer(Transformer::default()),
        extras: &[
            ("Inductance1", "2.5 H", "100 mH"),
            ("Inductance2", "5 H", "200 mH"),
            ("Coupling", "0.95", "0.8"),
            ("Rcoil1", "1.2 Ω", "500 mΩ"),
            ("Rcoil2", "2.4 Ω", "250 mΩ"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Scr",
        make: || Part::Scr(Scr::default()),
        extras: &[
            ("GateTh", "800 mV", "1.2 V"),
            ("HoldCurr", "20 mA", "5 mA"),
            ("TrigCurr", "10 mA", "2 mA"),
            ("GateRes", "20 Ω", "5 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Diac",
        make: || Part::Diac(Diac::default()),
        extras: &[
            ("Breakover", "32 V", "25 V"),
            ("ResOff", "2 MΩ", "500 kΩ"),
            ("HoldCurr", "25 mA", "15 mA"),
            ("ResOn", "15 Ω", "5 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Triac",
        make: || Part::Triac(Triac::default()),
        extras: &[
            ("GateTh", "800 mV", "1.2 V"),
            ("HoldCurr", "20 mA", "5 mA"),
            ("TrigCurr", "10 mA", "2 mA"),
            ("GateRes", "20 Ω", "5 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Csource",
        make: || Part::Csource(Csource::default()),
        extras: &[
            ("CurrSource", "false", "true"),
            ("ControlPins", "true", "false"),
            ("CurrControl", "true", "false"),
            ("Gain", "2.5", "0.5"),
            ("Voltage", "10 V", "2.5 V"),
            ("Current", "50 mA", "5 mA"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "ResistorDip",
        make: || Part::ResistorDip(ResistorDip::default()),
        extras: &[
            ("Size", "4", "16"),
            ("Resistance", "4.7 kΩ", "220 Ω"),
            ("Bussed", "true", "false"),
            ("PuVolt", "3.3 V", "12 V"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Ldr",
        make: || Part::Ldr(Ldr::default()),
        extras: &[
            ("Lux", "25 lux", "100 lux"),
            ("RDark", "2 MΩ", "500 kΩ"),
            ("RLight", "200 Ω", "50 Ω"),
            ("DialStep", "10 lux", "1 lux"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Thermistor",
        make: || Part::Thermistor(Thermistor::default()),
        extras: &[
            ("Temp", "50 °C", "0 °C"),
            ("R0", "5 kΩ", "20 kΩ"),
            ("Beta", "4 kK", "3.5 kK"),
            ("T0", "20 °C", "0 °C"),
            ("DialStep", "2 °C", "5 °C"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Rtd",
        make: || Part::Rtd(Rtd::default()),
        extras: &[
            ("Temp", "50 °C", "0 °C"),
            ("R0", "1 kΩ", "500 Ω"),
            ("Alpha", "0.00392", "0.004"),
            ("DialStep", "5 °C", "2 °C"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Strain",
        make: || Part::Strain(Strain::default()),
        extras: &[
            ("Strain", "500 µε", "1 mε"),
            ("GaugeFactor", "2.1", "1.8"),
            ("R0", "350 Ω", "120 Ω"),
            ("Temp", "30 °C", "0 °C"),
            ("RefTemp", "20 °C", "0 °C"),
            ("DialStep", "50 µε", "100 µε"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "AudioOut",
        make: || Part::AudioOut(AudioOut::default()),
        extras: &[
            ("Impedance", "16 Ω", "4 Ω"),
            ("Volume", "5 %", "50 %"),
            ("Buzzer", "true", "false"),
            ("Frequency", "2 kHz", "440 Hz"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Lamp",
        make: || Part::Lamp(Lamp::default()),
        extras: &[
            ("Voltage", "24 V", "6 V"),
            ("Power", "10 W", "2 W"),
            ("Resistance", "5.76 Ω", "1.5 Ω"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "WaveGen",
        make: || Part::WaveGen(WaveGen::default()),
        extras: &[
            ("WaveType", "Square", "Triangle"),
            ("Frequency", "500 Hz", "2 kHz"),
            ("Amplitude", "2.5 V", "10 V"),
            ("Offset", "1 V", "-1 V"),
            ("Duty", "25 %", "75 %"),
            ("Phase", "45 °", "90 °"),
            ("Steps", "50", "200"),
            ("Bipolar", "true", "false"),
            ("Floating", "true", "false"),
            ("File", "test.wav", "other.wav"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Oscope",
        make: || Part::Oscope(Oscope::default()),
        extras: &[
            ("Basic_X", "200", "500"),
            ("Basic_Y", "200", "400"),
            ("BufferSize", "100000", "500000"),
            ("connectGnd", "false", "true"),
            ("InputImped", "1 MΩ", "10 MΩ"),
            ("TestTime", "1 s", "2 s"),
            ("DoTest", "true", "false"),
            ("Tunnel1", "ch1", "t1"),
            ("Tunnel2", "ch2", "t2"),
            ("Tunnel3", "ch3", "t3"),
            ("Tunnel4", "ch4", "t4"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Tunnel",
        make: || Part::Tunnel(Tunnel::default()),
        extras: &[("Name", "NET_A", "NET_B"), ("IsBus", "true", "false")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Bus",
        make: || Part::Bus(Bus::default()),
        extras: &[("Width", "16", "4")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Socket",
        make: || Part::Socket(Socket::default()),
        extras: &[("Pins", "16", "4")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Header",
        make: || Part::Header(Header::default()),
        extras: &[("Pins", "16", "4")],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SerialPort",
        make: || Part::SerialPort(SerialPort::default()),
        extras: &[
            ("Port", "/dev/ttyS0", "COM1"),
            ("Baud", "115200", "19200"),
            ("DataBits", "7", "8"),
            ("StopBits", "2", "1"),
            ("Auto", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SerialTerm",
        make: || Part::SerialTerm(SerialTerm::default()),
        extras: &[
            ("Baud", "115200", "19200"),
            ("DataBits", "7", "8"),
            ("StopBits", "2", "1"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Subcircuit",
        make: || Part::Subcircuit(Subcircuit::default()),
        extras: &[
            ("LogicSymbol", "true", "false"),
            ("Package", "DIP8", "SOIC8"),
            ("Device", "74HC00", "74HC04"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "SubPackage",
        make: || Part::SubPackage(SubPackage::default()),
        extras: &[
            ("Name", "CustomChip", "TestPkg"),
            ("Width", "6", "8"),
            ("Height", "10", "12"),
            ("SubcType", "MCU", "Logic"),
            ("LogicSymbol", "true", "false"),
            ("CustomColor", "true", "false"),
            ("BckGndColor", "#3a3a5a", "#123456"),
            ("Border", "true", "false"),
            ("Background", "grid", "plain"),
            ("PackageFile", "chip.package", "test.package"),
            (
                "Pins",
                "Pin; type=; xpos=0; ypos=8; angle=0; length=8; space=0; id=1; label=VCC&#xa;",
                "Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=2; label=GND&#xa;",
            ),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Dial",
        make: || Part::Dial(Dial::default()),
        extras: &[
            ("Value", "50", "25"),
            ("MinVal", "10", "0"),
            ("MaxVal", "200", "50"),
            ("Step", "5", "0.5"),
        ],
        dial_prop: Some("Value"),
        dial_value: 50.0,
    },
    Case {
        itemtype: "Shape",
        make: || Part::Shape(Shape::default()),
        extras: &[
            ("ShapeKind", "Ellipse", "Text"),
            ("Width", "120", "80"),
            ("Height", "80", "40"),
            ("Text", "Label", "Heading"),
            ("Color", "#ffffff", "#123456"),
            ("Font", "Monospace", "Arial"),
            ("FontColor", "#ff0000", "#00ff00"),
            ("FontSize", "14", "12"),
            ("Border", "2", "4"),
            ("Opacity", "0.8", "0.5"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "Mcu",
        make: || Part::Mcu(Mcu::default()),
        extras: &[
            ("Device", "p16f84", "atmega328p"),
            ("Frequency", "8 MHz", "16 MHz"),
            ("ForceFreq", "false", "true"),
            ("Program", "firmware1.hex", "firmware2.hex"),
            ("AutoLoad", "true", "false"),
            ("SavePgm", "true", "false"),
            ("Pgm", "1,2,3,", "4,5,6,"),
            ("LogicSymbol", "true", "false"),
            ("Package", "DIP18", "DIP28"),
            ("SaveEepr", "true", "false"),
            ("RstEnabled", "false", "true"),
            ("ExtOsc", "true", "false"),
            ("WdtEnabled", "true", "false"),
            ("ClkOut", "true", "false"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
    Case {
        itemtype: "QemuDevice",
        make: || Part::QemuDevice(QemuDevice::default()),
        extras: &[
            ("Device", "STM32F103C8", "ESP32"),
            ("Program", "firmware1.bin", "firmware2.bin"),
            ("Args", "-d in_asm", "-s"),
            ("LogicSymbol", "true", "false"),
            ("Package", "LQFP48", "QFN32"),
            ("Active", "false", "true"),
        ],
        dial_prop: None,
        dial_value: 0.0,
    },
];

#[test]
fn every_registered_type_has_a_harness_case() {
    let registered: Vec<_> = registered_itemtypes().collect();
    assert_eq!(registered.len(), CASES.len());
    for c in CASES {
        assert!(
            registered.contains(&c.itemtype),
            "harness case {} is not in the factory",
            c.itemtype
        );
        let p = (c.make)();
        assert_eq!(p.type_id(), c.itemtype);
        assert!(!p.description().is_empty());
        if !matches!(
            c.itemtype,
            "Dial" | "Shape" | "Subcircuit" | "SubPackage" | "Mcu" | "QemuDevice"
        ) {
            assert!(!p.pin_geoms().is_empty());
        }
        assert!(p.body().w > 0.0 && p.body().h > 0.0);
    }
}

#[test]
fn default_sim1_round_trips_every_type() {
    for c in CASES {
        let p = (c.make)();
        let circ_id = format!("{}-1", c.itemtype);
        let line = p.write_item(&circ_id, &GraphicAttrs::at(8.0, 16.0));
        assert!(
            line.contains(&format!("itemtype=\"{}\"", c.itemtype)),
            "{line}"
        );
        let xml = write_circuit_v3(&[line], &[], &CircSettings::default());
        let parsed = parse_circuit_v3(&xml)
            .unwrap_or_else(|e| panic!("{} default parse: {e}\n{xml}", c.itemtype));
        assert_eq!(parsed.items[0].part.type_id(), c.itemtype);
    }
}

#[test]
fn persist_props_default_second_third_and_sim1() {
    for c in CASES {
        for (id, v2, v3) in c.extras {
            let def = (c.make)();
            let default_text = def
                .get_prop_text(id)
                .unwrap_or_else(|| panic!("{} missing default {}", c.itemtype, id));

            let mut p = (c.make)();
            p.set_prop_text(id, v2)
                .unwrap_or_else(|e| panic!("{} set {}={v2}: {e}", c.itemtype, id));
            let got2 = p.get_prop_text(id).unwrap();
            assert_eq!(
                got2, *v2,
                "{} {} second value: default was {default_text}",
                c.itemtype, id
            );

            p.set_prop_text(id, v3)
                .unwrap_or_else(|e| panic!("{} set {}={v3}: {e}", c.itemtype, id));
            let got3 = p.get_prop_text(id).unwrap();
            assert_eq!(got3, *v3, "{} {} third value", c.itemtype, id);

            let circ_id = format!("{}-1", c.itemtype);
            let graphic = GraphicAttrs::at(8.0, 16.0);
            let line = p.write_item(&circ_id, &graphic);
            assert!(
                line.contains(&format!("itemtype=\"{}\"", c.itemtype)),
                "{line}"
            );
            assert!(line.contains(&format!("CircId=\"{circ_id}\"")), "{line}");
            assert!(!line.contains("Show_id"), "canonical ShowId, got {line}");
            assert!(!line.contains("Show_Val"), "canonical ShowVal, got {line}");
            assert!(!line.contains("Min_Resistance"), "{line}");
            assert!(!line.contains("elCapacitor"), "{line}");
            assert!(!line.contains("Norm_Close"), "{line}");
            assert!(!line.contains("Double_Throw"), "{line}");
            assert!(!line.contains("Value_Volt"), "{line}");
            assert!(!line.contains("Value_Amp"), "{line}");
            assert!(!line.contains("Common_Pin"), "{line}");
            assert!(!line.contains("itemtype=\"Fixed Voltage\""), "{line}");
            assert!(!line.contains("itemtype=\"Voltage Source\""), "{line}");
            assert!(!line.contains("P_Channel"), "{line}");
            assert!(!line.contains("1/Lambda"), "{line}");
            assert!(!line.contains("itemtype=\"opAmp\""), "{line}");
            assert!(!line.contains("itemtype=\"Voltimeter\""), "{line}");
            assert!(!line.contains("itemtype=\"Amperimeter\""), "{line}");
            assert!(!line.contains("itemtype=\"Volt. Regulator\""), "{line}");
            assert!(!line.contains("itemtype=\"BJT\""), "{line}");
            assert!(!line.contains("itemtype=\"And Gate\""), "{line}");
            assert!(!line.contains("itemtype=\"Or Gate\""), "{line}");
            assert!(!line.contains("itemtype=\"Xor Gate\""), "{line}");
            assert!(!line.contains("itemtype=\"Not Gate\""), "{line}");
            assert!(!line.contains("itemtype=\"Buffer Gate\""), "{line}");
            assert!(!line.contains("itemtype=\"LAnalizer\""), "{line}");
            assert!(!line.contains("itemtype=\"TFTDisplay\""), "{line}");
            assert!(!line.contains("itemtype=\"PCD8544\""), "{line}");
            assert!(!line.contains("itemtype=\"SH1107\""), "{line}");
            assert!(!line.contains("itemtype=\"KS0108\""), "{line}");
            assert!(!line.contains("itemtype=\"PCF8833\""), "{line}");
            assert!(!line.contains("itemtype=\"AIP31068\""), "{line}");
            assert!(!line.contains("itemtype=\"HD44780\""), "{line}");
            assert!(!line.contains("itemtype=\"SSD1306\""), "{line}");
            assert!(!line.contains("itemtype=\"7Segment\""), "{line}");
            assert!(!line.contains("itemtype=\"SevenSegmentBcd\""), "{line}");
            assert!(!line.contains("itemtype=\"Led Matrix\""), "{line}");
            assert!(!line.contains("itemtype=\"MAX7219\""), "{line}");
            assert!(!line.contains("itemtype=\"Led Bar\""), "{line}");
            assert!(!line.contains("itemtype=\"RGB LED\""), "{line}");
            assert!(!line.contains("itemtype=\"WS2812\""), "{line}");
            assert!(!line.contains("Common_Anode"), "{line}");
            assert!(!line.contains("Vertical_Pins"), "{line}");
            assert!(!line.contains("itemtype=\"ADC\""), "{line}");
            assert!(!line.contains("itemtype=\"DAC\""), "{line}");
            assert!(!line.contains("itemtype=\"LM555\""), "{line}");
            assert!(!line.contains("itemtype=\"555\""), "{line}");
            assert!(!line.contains("itemtype=\"i2c_to_parallel\""), "{line}");
            assert!(!line.contains("itemtype=\"I2C_RAM\""), "{line}");
            assert!(!line.contains("itemtype=\"RAM\""), "{line}");
            assert!(!line.contains("itemtype=\"ROM\""), "{line}");
            assert!(!line.contains("Max_Count"), "{line}");
            assert!(!line.contains("Is_Decade"), "{line}");
            assert!(!line.contains("Vref_Pos"), "{line}");
            assert!(!line.contains("Vref_Neg"), "{line}");
            assert!(!line.contains("Out_High_V"), "{line}");
            assert!(!line.contains("Out_Low_V"), "{line}");
            assert!(!line.contains("Addr_Bits"), "{line}");
            assert!(!line.contains("Data_Bits"), "{line}");
            assert!(!line.contains("Is_Rom"), "{line}");
            assert!(!line.contains("Size_Bytes"), "{line}");
            assert!(!line.contains("Dev_Address"), "{line}");
            assert!(!line.contains("itemtype=\"transformer\""), "{line}");
            assert!(!line.contains("itemtype=\"TRANSFORMER\""), "{line}");
            assert!(!line.contains("itemtype=\"SCR\""), "{line}");
            assert!(!line.contains("itemtype=\"scr\""), "{line}");
            assert!(!line.contains("itemtype=\"DIAC\""), "{line}");
            assert!(!line.contains("itemtype=\"diac\""), "{line}");
            assert!(!line.contains("itemtype=\"TRIAC\""), "{line}");
            assert!(!line.contains("itemtype=\"triac\""), "{line}");
            assert!(!line.contains("itemtype=\"csource\""), "{line}");
            assert!(!line.contains("itemtype=\"CSOURCE\""), "{line}");
            assert!(!line.contains("itemtype=\"resistor_dip\""), "{line}");
            assert!(!line.contains("itemtype=\"Resistor_Dip\""), "{line}");
            assert!(!line.contains("itemtype=\"LDR\""), "{line}");
            assert!(!line.contains("itemtype=\"ldr\""), "{line}");
            assert!(!line.contains("itemtype=\"THERMISTOR\""), "{line}");
            assert!(!line.contains("itemtype=\"thermistor\""), "{line}");
            assert!(!line.contains("itemtype=\"RTD\""), "{line}");
            assert!(!line.contains("itemtype=\"rtd\""), "{line}");
            assert!(!line.contains("itemtype=\"STRAIN\""), "{line}");
            assert!(!line.contains("itemtype=\"strain\""), "{line}");
            assert!(!line.contains("itemtype=\"Audio_Out\""), "{line}");
            assert!(!line.contains("itemtype=\"Buzzer\""), "{line}");
            assert!(!line.contains("itemtype=\"Speaker\""), "{line}");
            assert!(!line.contains("itemtype=\"LAMP\""), "{line}");
            assert!(!line.contains("Gate_Th"), "{line}");
            assert!(!line.contains("Hold_Curr"), "{line}");
            assert!(!line.contains("Trig_Curr"), "{line}");
            assert!(!line.contains("Gate_Res"), "{line}");
            assert!(!line.contains("Res_Off"), "{line}");
            assert!(!line.contains("Res_On"), "{line}");
            assert!(!line.contains("Curr_Source"), "{line}");
            assert!(!line.contains("Control_Pins"), "{line}");
            assert!(!line.contains("Curr_Control"), "{line}");
            assert!(!line.contains("Pu_Volt"), "{line}");
            assert!(!line.contains("R_Dark"), "{line}");
            assert!(!line.contains("R_Light"), "{line}");
            assert!(!line.contains("Dial_Step"), "{line}");
            assert!(!line.contains("Gauge_Factor"), "{line}");
            assert!(!line.contains("Ref_Temp"), "{line}");
            assert!(!line.contains("itemtype=\"subcircuit\""), "{line}");
            assert!(!line.contains("itemtype=\"SUBCIRCUIT\""), "{line}");
            assert!(!line.contains("itemtype=\"subpackage\""), "{line}");
            assert!(!line.contains("itemtype=\"SUBPACKAGE\""), "{line}");
            assert!(!line.contains("itemtype=\"Package\""), "{line}");
            assert!(!line.contains("itemtype=\"dial\""), "{line}");
            assert!(!line.contains("itemtype=\"DIAL\""), "{line}");
            assert!(!line.contains("itemtype=\"shape\""), "{line}");
            assert!(!line.contains("itemtype=\"SHAPE\""), "{line}");
            assert!(!line.contains("itemtype=\"Rectangle\""), "{line}");
            assert!(!line.contains("itemtype=\"Ellipse\""), "{line}");
            assert!(!line.contains("Min_Val"), "{line}");
            assert!(!line.contains("Max_Val"), "{line}");
            assert!(!line.contains("Logic_Symbol"), "{line}");
            assert!(!line.contains("Custom_Color"), "{line}");
            assert!(!line.contains("Package_File"), "{line}");
            assert!(!line.contains("Font_Color"), "{line}");
            assert!(!line.contains("Font_Size"), "{line}");
            assert!(!line.contains("itemtype=\"MCU\""), "{line}");
            assert!(!line.contains("itemtype=\"mcu\""), "{line}");
            assert!(!line.contains("itemtype=\"Micro\""), "{line}");
            assert!(!line.contains("itemtype=\"micro\""), "{line}");
            assert!(!line.contains("itemtype=\"qemu\""), "{line}");
            assert!(!line.contains("itemtype=\"QEMU\""), "{line}");
            assert!(!line.contains("itemtype=\"qemu_device\""), "{line}");
            assert!(!line.contains("itemtype=\"qemudevice\""), "{line}");
            assert!(!line.contains("Auto_Load"), "{line}");
            assert!(!line.contains("savePGM"), "{line}");
            assert!(!line.contains("saveEepr"), "{line}");
            assert!(!line.contains("Rst_enabled"), "{line}");
            assert!(!line.contains("Ext_Osc"), "{line}");
            assert!(!line.contains("Wdt_enabled"), "{line}");
            assert!(!line.contains("Clk_Out"), "{line}");

            let xml = write_circuit_v3(&[line], &[], &CircSettings::default());
            assert!(xml.starts_with("<circuit version=\"1.0.0\""));
            let parsed = parse_circuit_v3(&xml)
                .unwrap_or_else(|e| panic!("{} parse after setting {id}: {e}\n{xml}", c.itemtype));
            assert_eq!(parsed.items.len(), 1);
            let loaded = &parsed.items[0];
            assert_eq!(loaded.itemtype, c.itemtype);
            assert_eq!(loaded.circ_id, circ_id);
            assert_eq!(loaded.graphic.pos, Some((8.0, 16.0)));
            assert_eq!(loaded.part.type_id(), c.itemtype);
            assert_eq!(
                loaded.part.get_prop_text(id).as_deref(),
                Some(got3.as_str()),
                "{} {} did not round-trip",
                c.itemtype,
                id
            );
        }
    }
}

#[test]
fn dial_writes_saved_prop() {
    for c in CASES {
        let Some(prop) = c.dial_prop else {
            continue;
        };
        let mut p = (c.make)();
        let change = p
            .set_dial(c.dial_value)
            .unwrap_or_else(|| panic!("{} should be Dialed", c.itemtype));
        assert!(
            change.saved && change.sim,
            "{} dial must mark document",
            c.itemtype
        );
        let got = p.get_prop_text(prop).unwrap();
        let mut via_prop = (c.make)();
        via_prop.set_prop_text(prop, &got).unwrap();
        assert_eq!(
            via_prop.get_prop_text(prop).unwrap(),
            got,
            "{} dial vs prop {}",
            c.itemtype,
            prop
        );
        let (val, min, max) = p.dial().unwrap();
        assert!(val >= min && val <= max);
    }
}

#[test]
fn graphic_attrs_round_trip_with_resistor_family() {
    let mut r = Resistor::default();
    r.set_prop_text("Resistance", "4.7 kΩ").unwrap();
    r.set_prop_text("ShowBands", "false").unwrap();

    let mut graphic = GraphicAttrs::at(24.0, -8.0);
    graphic.rotation = 90.0;
    graphic.hflip = -1;
    graphic.vflip = 1;
    graphic.label = Some("Rbias".into());
    graphic.show_id = true;
    graphic.label_pos = Some((4.0, -12.0));
    graphic.label_rot = 90;
    graphic.show_val = true;
    graphic.show_prop = Some("Resistance".into());
    graphic.val_pos = Some((0.0, 10.0));
    graphic.val_rot = 180;

    let line = write_component_item("Resistor-1", &r, &graphic);
    assert!(line.contains("ShowId=\"true\""));
    assert!(line.contains("ShowVal=\"true\""));
    assert!(!line.contains("Show_id"));
    assert!(!line.contains("Show_Val"));
    assert!(line.contains("hflip=\"-1\""));
    assert!(line.contains("label=\"Rbias\""));
    assert!(line.contains("ShowProp=\"Resistance\""));

    let mut pot = Potentiometer::default();
    pot.set_prop_text("Wiper", "0.25").unwrap();
    let pot_line = write_component_item("Potentiometer-1", &pot, &GraphicAttrs::at(0.0, 16.0));

    let xml = write_circuit_v3(&[line, pot_line], &[], &CircSettings::default());
    let parsed = parse_circuit_v3(&xml).unwrap();
    assert_eq!(parsed.items.len(), 2);

    let g = &parsed.items[0].graphic;
    assert_eq!(g.pos, Some((24.0, -8.0)));
    assert_eq!(g.rotation, 90.0);
    assert_eq!(g.hflip, -1);
    assert_eq!(g.vflip, 1);
    assert_eq!(g.label.as_deref(), Some("Rbias"));
    assert!(g.show_id);
    assert_eq!(g.label_pos, Some((4.0, -12.0)));
    assert_eq!(g.label_rot, 90);
    assert!(g.show_val);
    assert_eq!(g.show_prop.as_deref(), Some("Resistance"));
    assert_eq!(g.val_pos, Some((0.0, 10.0)));
    assert_eq!(g.val_rot, 180);
    assert_eq!(
        parsed.items[0].part.get_prop_text("Resistance").as_deref(),
        Some("4.7 kΩ")
    );
    assert_eq!(
        parsed.items[0].part.get_prop_text("ShowBands").as_deref(),
        Some("false")
    );
    assert_eq!(
        parsed.items[1].part.get_prop_text("Wiper").as_deref(),
        Some("0.25")
    );
}

#[test]
fn live_sim_kind_without_rebuild() {
    for c in CASES {
        let p = (c.make)();
        let kind = p
            .to_element_kind()
            .unwrap_or_else(|| panic!("{} missing element kind", c.itemtype));
        match (c.itemtype, kind) {
            ("Resistor", Kind::Resistor { .. })
            | ("VarResistor", Kind::VarResistor { .. })
            | ("Potentiometer", Kind::Potentiometer { .. })
            | ("Capacitor", Kind::Capacitor { .. })
            | ("ElCapacitor", Kind::ElCapacitor { .. })
            | ("Inductor", Kind::Inductor { .. })
            | ("VarCapacitor", Kind::VarCapacitor { .. })
            | ("VarInductor", Kind::VarInductor { .. })
            | ("Battery", Kind::Battery { .. })
            | ("Ground", Kind::Ground)
            | ("Node", Kind::Junction)
            | ("FixedVolt", Kind::FixedVolt { .. })
            | ("Rail", Kind::Rail { .. })
            | ("Clock", Kind::Clock { .. })
            | ("VoltSource", Kind::VoltSource { .. })
            | ("CurrSource", Kind::CurrSource { .. })
            | ("Switch", Kind::Switch { .. })
            | ("Push", Kind::Push { .. })
            | ("SwitchDip", Kind::SwitchDip { .. })
            | ("Relay", Kind::Relay { .. })
            | ("KeyPad", Kind::KeyPad { .. })
            | ("Diode", Kind::Diode { zener: false, .. })
            | ("Zener", Kind::Diode { zener: true, .. })
            | ("Led", Kind::Led { .. })
            | ("Bjt", Kind::Bjt { .. })
            | ("Mosfet", Kind::Mosfet { .. })
            | ("Jfet", Kind::Jfet { .. })
            | ("OpAmp", Kind::OpAmp { .. })
            | ("Comparator", Kind::Comparator { .. })
            | ("VoltReg", Kind::VoltReg { .. })
            | ("Probe", Kind::Probe { .. })
            | ("Voltmeter", Kind::Voltmeter { .. })
            | ("Ammeter", Kind::Ammeter { .. })
            | ("FreqMeter", Kind::FreqMeter { .. })
            | ("AndGate", Kind::Gate(_))
            | ("OrGate", Kind::Gate(_))
            | ("XorGate", Kind::Gate(_))
            | ("NotGate", Kind::Gate(_))
            | ("BufferGate", Kind::Gate(_))
            | ("FlipFlop", Kind::FlipFlop(_))
            | ("Latch", Kind::Latch(_))
            | ("Mux", Kind::Mux(_))
            | ("Demux", Kind::Demux(_))
            | ("AnalogMux", Kind::AnalogMux { .. })
            | ("BcdToDec", Kind::BcdToDec(_))
            | ("DecToBcd", Kind::DecToBcd(_))
            | ("BcdTo7Segment", Kind::BcdTo7S(_))
            | ("FullAdder", Kind::FullAdder(_))
            | ("HalfAdder", Kind::HalfAdder(_))
            | ("TestUnit", Kind::TestUnit(_))
            | ("LogicAnalyzer", Kind::LAnalizer { .. })
            | ("TouchPad", Kind::TouchPad { .. })
            | ("KY023", Kind::Ky023 { .. })
            | ("KY040", Kind::Ky040 { .. })
            | ("SR04", Kind::Sr04 { .. })
            | ("DHT22", Kind::Dht22 { .. })
            | ("DS18B20", Kind::Ds18b20 { .. })
            | ("DS1621", Kind::Ds1621 { .. })
            | ("DS1307", Kind::Ds1307 { .. })
            | ("DcMotor", Kind::DcMotor { .. })
            | ("Stepper", Kind::Stepper { .. })
            | ("Servo", Kind::Servo { .. })
            | ("SdCard", Kind::SdCard { .. })
            | ("Esp01", Kind::Esp01 { .. })
            | ("TftDisplay", Kind::TftDisplay { .. })
            | ("Pcd8544", Kind::Pcd8544(_))
            | ("Sh1107", Kind::Sh1107(_))
            | ("Ks0108", Kind::Ks0108(_))
            | ("Pcf8833", Kind::Pcf8833Display { .. })
            | ("Aip31068", Kind::Aip31068(_))
            | ("Hd44780", Kind::Hd44780(_))
            | ("Ssd1306", Kind::Ssd1306(_))
            | ("SevenSegment", Kind::SevenSegment { .. })
            | ("SevenSegmentBCD", Kind::SevenSegmentBCD { .. })
            | ("LedMatrix", Kind::LedMatrix { .. })
            | ("Max72xx", Kind::Max72xx { .. })
            | ("LedBar", Kind::LedBar { .. })
            | ("RgbLed", Kind::RgbLed { .. })
            | ("Ws2812", Kind::Ws2812 { .. })
            | ("Counter", Kind::Counter(_))
            | ("BinCounter", Kind::BinCounter(_))
            | ("ShiftReg", Kind::ShiftReg(_))
            | ("MagnitudeComp", Kind::MagnitudeComp(_))
            | ("Adc", Kind::Adc(_))
            | ("Dac", Kind::Dac(_))
            | ("I2CToParallel", Kind::I2CToParallel(_))
            | ("Lm555", Kind::Lm555(_))
            | ("Memory", Kind::Memory(_))
            | ("DynamicMemory", Kind::DynamicMemory(_))
            | ("I2CRam", Kind::I2CRam(_))
            | ("Function", Kind::Function(_))
            | ("Transformer", Kind::Transformer { .. })
            | ("Scr", Kind::Scr { .. })
            | ("Diac", Kind::Diac { .. })
            | ("Triac", Kind::Triac { .. })
            | ("Csource", Kind::Csource { .. })
            | ("ResistorDip", Kind::ResistorDip { .. })
            | ("Ldr", Kind::Ldr { .. })
            | ("Thermistor", Kind::Thermistor { .. })
            | ("Rtd", Kind::Rtd { .. })
            | ("Strain", Kind::Strain { .. })
            | ("AudioOut", Kind::AudioOut { .. })
            | ("Lamp", Kind::Lamp { .. })
            | ("WaveGen", Kind::WaveGen { .. })
            | ("Oscope", Kind::Oscope { .. })
            | ("Tunnel", Kind::Tunnel { .. })
            | ("Bus", Kind::Bus { .. })
            | ("Socket", Kind::Socket { .. })
            | ("Header", Kind::Header { .. })
            | ("SerialPort", Kind::SerialPort { .. })
            | ("SerialTerm", Kind::SerialTerm { .. })
            | ("Subcircuit", Kind::Subcircuit { .. })
            | ("SubPackage", Kind::SubPackage)
            | ("Dial", Kind::Dial { .. })
            | ("Shape", Kind::Shape { .. })
            | ("Mcu", Kind::Mcu(_))
            | ("QemuDevice", Kind::QemuDevice(_)) => {}
            (t, k) => panic!("{t} mapped to {k:?}"),
        }
    }
}
