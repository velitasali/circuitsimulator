//! Pin identification, suffix helpers, and pin cache indexing.

use super::*;

// Standard pin name constants
pub const PIN_LEFT: &str = "lPin";
pub const PIN_RIGHT: &str = "rPin";
pub const PIN_MID: &str = "mid";
pub const PIN_GND: &str = "Gnd";
pub const PIN_G_PIN: &str = "gPin";
pub const PIN_OUTNOD: &str = "outnod";
pub const PIN_GNDNOD: &str = "gndnod";
pub const PIN_OUTPIN: &str = "outPin";
pub const PIN_COLLECTOR: &str = "collector";
pub const PIN_EMITTER: &str = "emiter";
pub const PIN_BASE: &str = "base";
pub const PIN_DRAIN: &str = "Dren";
pub const PIN_SOURCE: &str = "Sour";
pub const PIN_GATE: &str = "Gate";
pub const PIN_INPUT_PINV: &str = "inputNinv";
pub const PIN_INPUT_INV: &str = "inputInv";
pub const PIN_OUTPUT: &str = "output";
pub const PIN_POWER_POS: &str = "powerPos";
pub const PIN_POWER_NEG: &str = "powerNeg";
pub const PIN_IN0: &str = "in0";
pub const PIN_IN1: &str = "in1";
pub const PIN_IN2: &str = "in2";
pub const PIN_IN3: &str = "in3";
pub const PIN_IN4: &str = "in4";
pub const PIN_IN5: &str = "in5";
pub const PIN_OUT: &str = "out";
pub const PIN_INPUT: &str = "input";
pub const PIN_REF: &str = "ref";
pub const PIN_INPIN: &str = "inpin";
pub const PIN_RGB_R: &str = "rPin";
pub const PIN_RGB_G: &str = "gPin";
pub const PIN_RGB_B: &str = "bPin";
pub const PIN_RGB_C: &str = "cPin";
pub const PIN_POT_A: &str = "PinA";
pub const PIN_POT_B: &str = "PinB";
pub const PIN_POT_W: &str = "wPin";
pub const PIN_POT_M: &str = "PinM";
pub const PIN_DATA: &str = "Data";
pub const PIN_TRIG: &str = "Trig";
pub const PIN_ECHO: &str = "Echo";
pub const PIN_VCC: &str = "Vcc";
pub const PIN_VDD: &str = "Vdd";
pub const PIN_NC: &str = "NC";
pub const PIN_SDA: &str = "SDA";
pub const PIN_SCL: &str = "SCL";
pub const PIN_TOUT: &str = "Tout";
pub const PIN_A0: &str = "A0";
pub const PIN_A1: &str = "A1";
pub const PIN_A2: &str = "A2";
pub const PIN_CLK: &str = "CLK";
pub const PIN_DT: &str = "DT";
pub const PIN_SW: &str = "SW";
pub const PIN_VRX: &str = "VRX";
pub const PIN_VRY: &str = "VRY";
pub const PIN_XP: &str = "XP";
pub const PIN_XM: &str = "XM";
pub const PIN_YP: &str = "YP";
pub const PIN_YM: &str = "YM";
pub const PIN_COM: &str = "com";
pub const PIN_EN: &str = "en";

/// Strongly-typed categorization of component pin suffixes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PinSuffix<'a> {
    Left,
    Right,
    Mid,
    Ground,
    OutNod,
    GndNod,
    OutPin,
    Out,
    Collector,
    Emitter,
    Base,
    Drain,
    Source,
    Gate,
    InputPos,
    InputNeg,
    OutputAmp,
    PowerPos,
    PowerNeg,
    CompIn0,
    CompIn1,
    VoltRegIn,
    VoltRegOut,
    VoltRegRef,
    ProbeIn,
    RgbR,
    RgbG,
    RgbB,
    RgbC,
    SwitchP(usize),
    SwitchN(usize),
    IndexedLeft(usize),
    IndexedRight(usize),
    IndexedPin(usize),
    IndexedAddr(usize),
    IndexedChannel(usize),
    IndexedEPin(usize),
    Named(&'a str),
}

impl<'a> PinSuffix<'a> {
    pub fn parse(suffix: &'a str) -> Self {
        match suffix {
            PIN_LEFT | "PinA" | "pina" => PinSuffix::Left,
            PIN_RIGHT | "PinB" | "pinb" => PinSuffix::Right,
            PIN_MID | "PinM" | "pinm" | "wPin" => PinSuffix::Mid,
            PIN_GND | PIN_G_PIN | "gnd" | "gndPin" => PinSuffix::Ground,
            PIN_OUTNOD => PinSuffix::OutNod,
            PIN_GNDNOD => PinSuffix::GndNod,
            PIN_OUTPIN => PinSuffix::OutPin,
            PIN_OUT => PinSuffix::Out,
            PIN_COLLECTOR => PinSuffix::Collector,
            PIN_EMITTER | "emitter" | "ePin" => PinSuffix::Emitter,
            PIN_BASE => PinSuffix::Base,
            PIN_DRAIN | "drain" | "dPin" => PinSuffix::Drain,
            PIN_SOURCE | "source" | "sPin" => PinSuffix::Source,
            PIN_GATE | "gate" => PinSuffix::Gate,
            PIN_INPUT_PINV | "inp" | "pPin" => PinSuffix::InputPos,
            PIN_INPUT_INV | "inn" | "nPin" => PinSuffix::InputNeg,
            PIN_OUTPUT => PinSuffix::OutputAmp,
            PIN_POWER_POS | "vpos" | "posPin" => PinSuffix::PowerPos,
            PIN_POWER_NEG | "vneg" | "negPin" => PinSuffix::PowerNeg,
            PIN_IN0 => PinSuffix::CompIn0,
            PIN_IN1 => PinSuffix::CompIn1,
            PIN_INPUT => PinSuffix::VoltRegIn,
            PIN_REF => PinSuffix::VoltRegRef,
            PIN_INPIN => PinSuffix::ProbeIn,
            PIN_RGB_C => PinSuffix::RgbC,
            PIN_RGB_B => PinSuffix::RgbB,
            _ => {
                if let Some(rest) = suffix.strip_prefix("pinP") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::SwitchP(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("switch") {
                    if let Some(idx_str) = rest.strip_suffix("pinN") {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            return PinSuffix::SwitchN(idx);
                        }
                    }
                }
                if let Some(rest) = suffix.strip_prefix("lPin") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedLeft(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("rPin") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedRight(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("Pin") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedPin(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("pinAddr") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedAddr(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("pinY") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedChannel(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("ch") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedChannel(idx);
                    }
                }
                if let Some(rest) = suffix.strip_prefix("ePin") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        return PinSuffix::IndexedEPin(idx);
                    }
                }
                PinSuffix::Named(suffix)
            }
        }
    }
}

/// Split a full pin ID like `"Resistor-1-lPin"` with known component ID into `PinSuffix`.
pub fn split_comp_pin_suffix<'a>(comp_id: &str, pin_id: &'a str) -> Option<PinSuffix<'a>> {
    let suffix = pin_id.strip_prefix(comp_id)?.strip_prefix('-')?;
    Some(PinSuffix::parse(suffix))
}

/// Split an arbitrary full pin ID like `"Resistor-1-lPin"` into component ID `"Resistor-1"` and `PinSuffix::Left`.
pub fn split_pin_id(pin_id: &str) -> Option<(&str, PinSuffix<'_>)> {
    let (comp_id, suffix) = pin_id.rsplit_once('-')?;
    Some((comp_id, PinSuffix::parse(suffix)))
}

impl Comp {
    pub fn pin_ids(&self) -> Vec<String> {
        match &self.kind {
            Kind::Resistor { .. }
            | Kind::Battery { .. }
            | Kind::Capacitor { .. }
            | Kind::ElCapacitor { .. }
            | Kind::Inductor { .. } => {
                vec![format!("{}-lPin", self.id), format!("{}-rPin", self.id)]
            }
            Kind::Ground => vec![format!("{}-Gnd", self.id)],
            Kind::FixedVolt { .. } => vec![format!("{}-outnod", self.id)],
            Kind::Junction => (0..3).map(|i| format!("{}-{i}", self.id)).collect(),
            Kind::Switch {
                poles,
                double_throw,
                ..
            } => {
                let n = (*poles).max(1);
                let mut ps = Vec::with_capacity(if *double_throw { n * 3 } else { n * 2 });
                for i in 0..n {
                    ps.push(format!("{}-pinP{i}", self.id));
                    if *double_throw {
                        ps.push(format!("{}-switch{}pinN", self.id, 2 * i));
                        ps.push(format!("{}-switch{}pinN", self.id, 2 * i + 1));
                    } else {
                        ps.push(format!("{}-switch{i}pinN", self.id));
                    }
                }
                ps
            }
            Kind::Diode { .. } => vec![
                format!("{}-lPin", self.id),
                format!("{}-rPin", self.id),
                self.mid_pin(),
            ],
            Kind::Led { .. } => {
                vec![format!("{}-lPin", self.id), format!("{}-rPin", self.id)]
            }
            Kind::Bjt { .. } => vec![self.collector_pin(), self.emitter_pin(), self.base_pin()],
            Kind::Mosfet { .. } | Kind::Jfet { .. } => {
                vec![self.drain_pin(), self.source_pin(), self.gate_pin()]
            }
            Kind::OpAmp { .. } => vec![
                self.input_p_pin(),
                self.input_n_pin(),
                self.output_amp_pin(),
                self.power_pos_pin(),
                self.power_neg_pin(),
            ],
            Kind::Comparator { .. } => vec![
                self.comparator_in0_pin(),
                self.comparator_in1_pin(),
                self.comparator_out_pin(),
            ],
            Kind::VoltReg { .. } => vec![
                self.voltreg_in_pin(),
                self.voltreg_out_pin(),
                self.voltreg_ref_pin(),
            ],
            Kind::Probe { .. } => vec![self.probe_pin()],
            Kind::Voltmeter { .. } | Kind::Ammeter { .. } => {
                vec![self.left_pin(), self.right_pin(), self.out_pin()]
            }
            Kind::FreqMeter { .. } => vec![self.left_pin()],
            Kind::Oscope { .. } => {
                let mut pins: Vec<String> = (0..4).map(|i| self.plot_pin(i)).collect();
                pins.push(self.plot_gnd_pin());
                pins
            }
            Kind::LAnalizer { .. } => (0..8).map(|i| self.plot_pin(i)).collect(),
            Kind::Clock { .. } | Kind::Rail { .. } => vec![format!("{}-outnod", self.id)],
            Kind::WaveGen { bipolar, .. } => {
                if *bipolar {
                    vec![format!("{}-outnod", self.id), format!("{}-gndnod", self.id)]
                } else {
                    vec![format!("{}-outnod", self.id)]
                }
            }
            Kind::VoltSource { .. } | Kind::CurrSource { .. } => {
                vec![format!("{}-outPin", self.id)]
            }
            Kind::Csource { .. } => vec![
                format!("{}-cpPin", self.id),
                format!("{}-cmPin", self.id),
                format!("{}-s1Pin", self.id),
                format!("{}-s2Pin", self.id),
            ],
            Kind::Push { poles, .. } => (0..*poles)
                .flat_map(|i| {
                    vec![
                        format!("{}-lPin{i}", self.id),
                        format!("{}-rPin{i}", self.id),
                    ]
                })
                .collect(),
            Kind::SwitchDip {
                size, common_pin, ..
            } => {
                if *common_pin {
                    let mut v = vec![format!("{}-com", self.id)];
                    v.extend((0..*size).map(|i| format!("{}-pin{i}", self.id)));
                    v
                } else {
                    (0..*size * 2)
                        .map(|i| format!("{}-pin{i}", self.id))
                        .collect()
                }
            }
            Kind::KeyPad { rows, cols, .. } => {
                let r = (*rows).max(1);
                let c = (*cols).max(1);
                (0..(r + c))
                    .map(|i| format!("{}-Pin{i}", self.id))
                    .collect()
            }
            Kind::Relay { .. } => vec![
                format!("{}-lPin", self.id),
                format!("{}-rPin", self.id),
                format!("{}-c1", self.id),
                format!("{}-c2", self.id),
            ],
            Kind::Potentiometer { .. } => vec![
                format!("{}-lPin", self.id),
                format!("{}-rPin", self.id),
                format!("{}-wPin", self.id),
                format!("{}-PinA", self.id),
                format!("{}-PinB", self.id),
                format!("{}-PinM", self.id),
            ],
            Kind::TouchPad { .. } => vec![
                format!("{}-vrx_p", self.id),
                format!("{}-vrx_m", self.id),
                format!("{}-vry_p", self.id),
                format!("{}-vry_m", self.id),
                format!("{}-nodeX", self.id),
                format!("{}-nodeY", self.id),
            ],
            Kind::Ky023 { .. } => vec![
                format!("{}-vrx", self.id),
                format!("{}-vry", self.id),
                format!("{}-sw", self.id),
            ],
            Kind::Ky040 { .. } => vec![
                format!("{}-clk", self.id),
                format!("{}-dt", self.id),
                format!("{}-sw", self.id),
            ],
            Kind::Sr04 { .. } => vec![
                format!("{}-inpin", self.id),
                format!("{}-vccpin", self.id),
                format!("{}-trigpin", self.id),
                format!("{}-outpin", self.id),
                format!("{}-gndpin", self.id),
            ],
            Kind::Dht22 { .. } => vec![
                format!("{}-inPin", self.id),
                format!("{}-vccPin", self.id),
                format!("{}-ncPin", self.id),
                format!("{}-gdnPin", self.id),
            ],
            Kind::Ds18b20 { .. } => vec![
                format!("{}-inPin", self.id),
                format!("{}-vddPin", self.id),
                format!("{}-gndPin", self.id),
            ],
            Kind::Ds1621 { .. } => vec![
                format!("{}-inPin0", self.id),
                format!("{}-inPin1", self.id),
                format!("{}-outPin0", self.id),
                format!("{}-PinGnd", self.id),
                format!("{}-PinVdd", self.id),
                format!("{}-inPin2", self.id),
                format!("{}-inPin3", self.id),
                format!("{}-inPin4", self.id),
            ],
            Kind::Ds1307 { .. } => vec![
                format!("{}-PinSDA", self.id),
                format!("{}-PinSCL", self.id),
                format!("{}-PinSQW", self.id),
            ],
            Kind::DcMotor { .. } => vec![format!("{}-lPin", self.id), format!("{}-rPin", self.id)],
            Kind::Stepper { .. } => vec![
                format!("{}-PinA1", self.id),
                format!("{}-PinB1", self.id),
                format!("{}-PinCo", self.id),
                format!("{}-PinA2", self.id),
                format!("{}-PinB2", self.id),
            ],
            Kind::Servo { .. } => vec![
                format!("{}-PinV+", self.id),
                format!("{}-PinGnd", self.id),
                format!("{}-PinSig", self.id),
            ],
            Kind::SdCard { .. } => vec![
                format!("{}-PinCS", self.id),
                format!("{}-PinDI", self.id),
                format!("{}-PinCK", self.id),
                format!("{}-PinDO", self.id),
            ],
            Kind::Esp01 { .. } => vec![format!("{}-pin0", self.id), format!("{}-pin1", self.id)],
            Kind::TftDisplay { .. } => vec![
                format!("{}-PinDC", self.id),
                format!("{}-PinCS", self.id),
                format!("{}-PinDI", self.id),
                format!("{}-PinCK", self.id),
                format!("{}-PinRS", self.id),
                format!("{}-PinDO", self.id),
            ],
            Kind::Pcd8544(p) => p.pin_ids(),
            Kind::Sh1107(s) => s.pin_ids(),
            Kind::Ks0108(k) => k.pin_ids(),
            Kind::Pcf8833Display { .. } => vec![
                format!("{}-PinCS", self.id),
                format!("{}-PinCLK", self.id),
                format!("{}-PinDATA", self.id),
                format!("{}-PinRESET", self.id),
            ],
            Kind::Aip31068(a) => a.pin_ids(),
            Kind::VarResistor { .. }
            | Kind::Ldr { .. }
            | Kind::Thermistor { .. }
            | Kind::Rtd { .. }
            | Kind::Strain { .. }
            | Kind::VarCapacitor { .. }
            | Kind::VarInductor { .. }
            | Kind::Diac { .. }
            | Kind::Lamp { .. }
            | Kind::AudioOut { .. } => {
                vec![format!("{}-lPin", self.id), format!("{}-rPin", self.id)]
            }
            Kind::ResistorDip { size, bussed, .. } => {
                if *bussed {
                    let mut v = vec![format!("{}-com", self.id)];
                    v.extend((0..*size).map(|i| format!("{}-pin{i}", self.id)));
                    v
                } else {
                    (0..*size)
                        .flat_map(|i| {
                            vec![
                                format!("{}-lPin{i}", self.id),
                                format!("{}-rPin{i}", self.id),
                            ]
                        })
                        .collect()
                }
            }
            Kind::Transformer { .. } => vec![
                format!("{}-p1", self.id),
                format!("{}-p2", self.id),
                format!("{}-s1", self.id),
                format!("{}-s2", self.id),
            ],
            Kind::Scr { .. } => vec![
                format!("{}-lPin", self.id),
                format!("{}-rPin", self.id),
                format!("{}-gPin", self.id),
                format!("{}-aPin", self.id),
                format!("{}-kPin", self.id),
            ],
            Kind::Triac { .. } => vec![
                format!("{}-lPin", self.id),
                format!("{}-rPin", self.id),
                format!("{}-gPin", self.id),
                format!("{}-mt1Pin", self.id),
                format!("{}-mt2Pin", self.id),
            ],
            Kind::AnalogMux { channels, .. } => {
                let mut v = Vec::new();
                v.push(format!("{}-PinInput", self.id));
                v.push(format!("{}-com", self.id));
                v.push(format!("{}-PinEnable", self.id));
                v.push(format!("{}-en", self.id));
                for i in 0..4 {
                    v.push(format!("{}-pinAddr{i}", self.id));
                }
                for i in 0..*channels {
                    v.push(format!("{}-pinY{i}", self.id));
                    v.push(format!("{}-ch{i}", self.id));
                }
                v
            }
            Kind::RgbLed { .. } => vec![
                format!("{}-rPin", self.id),
                format!("{}-gPin", self.id),
                format!("{}-bPin", self.id),
                format!("{}-cPin", self.id),
            ],
            Kind::LedBar {
                segments, grounded, ..
            } => (0..*segments)
                .flat_map(|i| {
                    if *grounded {
                        vec![format!("{}-lPin{i}", self.id)]
                    } else {
                        vec![
                            format!("{}-lPin{i}", self.id),
                            format!("{}-rPin{i}", self.id),
                        ]
                    }
                })
                .collect(),
            Kind::SevenSegment { .. } => [
                "pin_a",
                "pin_b",
                "pin_c",
                "pin_d",
                "pin_e",
                "pin_f",
                "pin_g",
                "pin_dot",
                "pin_commona",
                "a",
                "b",
                "c",
                "d",
                "e",
                "f",
                "g",
                "dp",
                "com",
            ]
            .iter()
            .map(|s| format!("{}-{s}", self.id))
            .collect(),
            Kind::LedMatrix { rows, cols, .. } => {
                let r = (*rows).max(1);
                let c = (*cols).max(1);
                let mut v = Vec::with_capacity(r + c);
                for row in 0..r {
                    v.push(format!("{}-rPin{row}", self.id));
                }
                for col in 0..c {
                    v.push(format!("{}-cPin{col}", self.id));
                }
                v
            }
            Kind::Max72xx { .. } => vec![
                format!("{}-din", self.id),
                format!("{}-clk", self.id),
                format!("{}-cs", self.id),
                format!("{}-dout", self.id),
                format!("{}-vcc", self.id),
                format!("{}-gnd", self.id),
            ],
            Kind::Ws2812 { .. } => vec![
                format!("{}-din", self.id),
                format!("{}-dout", self.id),
                format!("{}-vdd", self.id),
                format!("{}-gnd", self.id),
            ],
            Kind::Dial { .. } | Kind::Shape { .. } | Kind::SubPackage => Vec::new(),
            Kind::SevenSegmentBCD { .. } => vec![
                format!("{}-in0", self.id),
                format!("{}-in1", self.id),
                format!("{}-in2", self.id),
                format!("{}-in3", self.id),
            ],
            Kind::Header { pins_count, .. } | Kind::Socket { pins_count, .. } => (0..*pins_count)
                .map(|i| format!("{}-pin{i}", self.id))
                .collect(),
            Kind::Bus { width, .. } => (0..*width).map(|i| format!("{}-pin{i}", self.id)).collect(),
            Kind::SerialPort { .. } => vec![
                format!("{}-rx", self.id),
                format!("{}-tx", self.id),
                format!("{}-gnd", self.id),
                format!("{}-vcc", self.id),
            ],
            Kind::SerialTerm { .. } => vec![format!("{}-rx", self.id), format!("{}-tx", self.id)],
            Kind::Gate(g) => g.pin_ids(),
            Kind::FlipFlop(f) => f.pin_ids(),
            Kind::Latch(l) => l.pin_ids(),
            Kind::McuPin(p) => p.pin_ids(),
            Kind::Mcu(m) => m.pin_ids(),
            Kind::McuItem(_) => Vec::new(),
            Kind::QemuDevice(q) => q.pin_ids(),
            Kind::ScriptCpu(s) => s.pin_ids(),
            Kind::TestUnit(t) => t.pin_ids(),
            Kind::Hd44780(h) => h.pin_ids(),
            Kind::Ssd1306(s) => vec![s.pin_scl.id.clone(), s.pin_sda.id.clone()],
            Kind::Mux(m) => m
                .inputs
                .iter()
                .chain(m.addr_pins.iter())
                .chain(m.enable.iter())
                .chain(std::iter::once(&m.output))
                .chain(std::iter::once(&m.out_inverted))
                .map(|p| p.id.clone())
                .collect(),
            Kind::Demux(d) => std::iter::once(&d.input)
                .chain(d.addr_pins.iter())
                .chain(d.enable.iter())
                .chain(d.outputs.iter())
                .map(|p| p.id.clone())
                .collect(),
            Kind::BcdToDec(b) => b
                .inputs
                .iter()
                .chain(b.outputs.iter())
                .map(|p| p.id.clone())
                .collect(),
            Kind::DecToBcd(d) => d
                .inputs
                .iter()
                .chain(d.outputs.iter())
                .chain(std::iter::once(&d.gs))
                .chain(std::iter::once(&d.eo))
                .chain(std::iter::once(&d.ei))
                .map(|p| p.id.clone())
                .collect(),
            Kind::BcdTo7S(b) => b
                .inputs
                .iter()
                .chain(std::iter::once(&b.lt))
                .chain(std::iter::once(&b.rbi))
                .chain(std::iter::once(&b.bi_rbo))
                .chain(b.segments.iter())
                .map(|p| p.id.clone())
                .collect(),
            Kind::I2CToParallel(p) => p
                .ports
                .iter()
                .chain(std::iter::once(&p.int_pin))
                .chain(std::iter::once(&p.scl))
                .chain(std::iter::once(&p.sda))
                .map(|p| p.id.clone())
                .collect(),
            Kind::Adc(a) => a
                .outputs
                .iter()
                .chain(std::iter::once(&a.soc))
                .chain(std::iter::once(&a.eoc))
                .chain(std::iter::once(&a.oe))
                .map(|p| p.id.clone())
                .collect(),
            Kind::Dac(d) => d
                .inputs
                .iter()
                .map(|p| p.id.clone())
                .chain(std::iter::once(format!("{}-vout", self.id)))
                .chain(std::iter::once(format!("{}-vref", self.id)))
                .collect(),
            Kind::Counter(c) => c
                .outputs
                .iter()
                .chain(std::iter::once(&c.clock))
                .chain(std::iter::once(&c.reset))
                .chain(std::iter::once(&c.enable))
                .chain(std::iter::once(&c.output))
                .map(|p| p.id.clone())
                .collect(),
            Kind::BinCounter(b) => b
                .outputs
                .iter()
                .chain(std::iter::once(&b.clk_a))
                .chain(std::iter::once(&b.clk_b))
                .chain(std::iter::once(&b.r0_1))
                .chain(std::iter::once(&b.r0_2))
                .chain(std::iter::once(&b.r9_1))
                .chain(std::iter::once(&b.r9_2))
                .map(|p| p.id.clone())
                .collect(),
            Kind::FullAdder(fa) => fa
                .a_inputs
                .iter()
                .chain(fa.b_inputs.iter())
                .chain(std::iter::once(&fa.ci))
                .chain(fa.sum_outputs.iter())
                .chain(std::iter::once(&fa.co))
                .map(|p| p.id.clone())
                .collect(),
            Kind::HalfAdder(fa) => fa
                .a_inputs
                .iter()
                .chain(fa.b_inputs.iter())
                .chain(fa.sum_outputs.iter())
                .chain(std::iter::once(&fa.co))
                .map(|p| p.id.clone())
                .collect(),
            Kind::MagnitudeComp(mc) => mc
                .a_inputs
                .iter()
                .chain(mc.b_inputs.iter())
                .chain(std::iter::once(&mc.cascade_gt))
                .chain(std::iter::once(&mc.cascade_eq))
                .chain(std::iter::once(&mc.cascade_lt))
                .chain(std::iter::once(&mc.out_gt))
                .chain(std::iter::once(&mc.out_eq))
                .chain(std::iter::once(&mc.out_lt))
                .map(|p| p.id.clone())
                .collect(),
            Kind::ShiftReg(sr) => sr
                .outputs
                .iter()
                .chain(std::iter::once(&sr.ser_in))
                .chain(std::iter::once(&sr.clk_shift))
                .chain(std::iter::once(&sr.clk_latch))
                .chain(std::iter::once(&sr.master_reset))
                .chain(std::iter::once(&sr.oe))
                .chain(std::iter::once(&sr.ser_out))
                .map(|p| p.id.clone())
                .collect(),
            Kind::Function(f) => f
                .inputs
                .iter()
                .chain(std::iter::once(&f.output))
                .map(|p| p.id.clone())
                .collect(),
            Kind::Memory(m) => m
                .addr_pins
                .iter()
                .chain(m.data_pins.iter())
                .chain(std::iter::once(&m.cs))
                .chain(std::iter::once(&m.oe))
                .chain(std::iter::once(&m.we))
                .map(|p| p.id.clone())
                .collect(),
            Kind::DynamicMemory(dm) => dm
                .addr_pins
                .iter()
                .chain(dm.data_pins.iter())
                .chain(std::iter::once(&dm.ras))
                .chain(std::iter::once(&dm.cas))
                .chain(std::iter::once(&dm.we))
                .chain(std::iter::once(&dm.oe))
                .map(|p| p.id.clone())
                .collect(),
            Kind::I2CRam(r) => vec![r.scl.id.clone(), r.sda.id.clone()],
            Kind::Lm555(_) => (0..8).map(|i| format!("{}-ePin{i}", self.id)).collect(),
            Kind::Tunnel { pin_id, .. } => vec![pin_id.clone()],
            Kind::Subcircuit { .. } => Vec::new(),
        }
    }

    pub fn left_pin(&self) -> String {
        match self.kind {
            Kind::Switch { .. } => format!("{}-pinP0", self.id),
            _ => format!("{}-{PIN_LEFT}", self.id),
        }
    }

    pub fn right_pin(&self) -> String {
        match self.kind {
            Kind::Switch { .. } => format!("{}-switch0pinN", self.id),
            _ => format!("{}-{PIN_RIGHT}", self.id),
        }
    }

    pub fn switch_p_pin(&self, pole: usize) -> String {
        format!("{}-pinP{pole}", self.id)
    }

    pub fn switch_n_pin(&self, pole: usize) -> String {
        format!("{}-switch{pole}pinN", self.id)
    }

    pub fn gnd_pin(&self) -> String {
        format!("{}-{PIN_GND}", self.id)
    }

    pub fn out_pin(&self) -> String {
        format!("{}-{PIN_OUTNOD}", self.id)
    }

    pub fn mid_pin(&self) -> String {
        format!("{}-{PIN_MID}", self.id)
    }

    /// C++ `id + "-collector"`.
    pub fn collector_pin(&self) -> String {
        format!("{}-{PIN_COLLECTOR}", self.id)
    }

    /// C++ spelling `id + "-emiter"`.
    pub fn emitter_pin(&self) -> String {
        format!("{}-{PIN_EMITTER}", self.id)
    }

    pub fn base_pin(&self) -> String {
        format!("{}-{PIN_BASE}", self.id)
    }

    /// C++ `id + "-Dren"`.
    pub fn drain_pin(&self) -> String {
        format!("{}-{PIN_DRAIN}", self.id)
    }

    /// C++ `id + "-Sour"`.
    pub fn source_pin(&self) -> String {
        format!("{}-{PIN_SOURCE}", self.id)
    }

    pub fn gate_pin(&self) -> String {
        format!("{}-{PIN_GATE}", self.id)
    }

    /// C++ `id + "-inputNinv"` (non-inverting).
    pub fn input_p_pin(&self) -> String {
        format!("{}-{PIN_INPUT_PINV}", self.id)
    }

    /// C++ `id + "-inputInv"` (inverting).
    pub fn input_n_pin(&self) -> String {
        format!("{}-{PIN_INPUT_INV}", self.id)
    }

    /// C++ `id + "-output"`.
    pub fn output_amp_pin(&self) -> String {
        format!("{}-{PIN_OUTPUT}", self.id)
    }

    pub fn power_pos_pin(&self) -> String {
        format!("{}-{PIN_POWER_POS}", self.id)
    }

    pub fn power_neg_pin(&self) -> String {
        format!("{}-{PIN_POWER_NEG}", self.id)
    }

    /// C++ `IoComponent` input `id + "-in0"` (non-inverting).
    pub fn comparator_in0_pin(&self) -> String {
        format!("{}-{PIN_IN0}", self.id)
    }

    /// C++ `id + "-in1"` (inverting).
    pub fn comparator_in1_pin(&self) -> String {
        format!("{}-{PIN_IN1}", self.id)
    }

    /// C++ `setNumOuts(..., -1)` → `id + "-out"`.
    pub fn comparator_out_pin(&self) -> String {
        format!("{}-{PIN_OUT}", self.id)
    }

    /// C++ `id + "-input"`.
    pub fn voltreg_in_pin(&self) -> String {
        format!("{}-{PIN_INPUT}", self.id)
    }

    /// C++ `id + "-output"`.
    pub fn voltreg_out_pin(&self) -> String {
        format!("{}-{PIN_OUTPUT}", self.id)
    }

    /// C++ `id + "-ref"`.
    pub fn voltreg_ref_pin(&self) -> String {
        format!("{}-{PIN_REF}", self.id)
    }

    /// C++ Probe `id + "-inpin"`.
    pub fn probe_pin(&self) -> String {
        format!("{}-{PIN_INPIN}", self.id)
    }

    /// C++ Oscope/LAnalizer `id + "-Pin" + i`.
    pub fn plot_pin(&self, i: usize) -> String {
        format!("{}-Pin{i}", self.id)
    }

    /// C++ Oscope `id + "-PinG"`.
    pub fn plot_gnd_pin(&self) -> String {
        format!("{}-PinG", self.id)
    }

    /// Potentiometer Terminal A (`id + "-PinA"` or `id + "-lPin"`).
    pub fn pot_pin_a(&self) -> String {
        format!("{}-PinA", self.id)
    }

    /// Potentiometer Wiper (`id + "-PinM"` or `id + "-wPin"`).
    pub fn pot_pin_m(&self) -> String {
        format!("{}-PinM", self.id)
    }

    /// Potentiometer Terminal B (`id + "-PinB"` or `id + "-rPin"`).
    pub fn pot_pin_b(&self) -> String {
        format!("{}-PinB", self.id)
    }

    pub fn pot_pin_a_node(&self, pin_net: &rustc_hash::FxHashMap<String, usize>) -> Option<usize> {
        pin_net
            .get(&format!("{}-lPin", self.id))
            .copied()
            .or_else(|| pin_net.get(&format!("{}-PinA", self.id)).copied())
    }

    pub fn pot_pin_m_node(&self, pin_net: &rustc_hash::FxHashMap<String, usize>) -> Option<usize> {
        pin_net
            .get(&format!("{}-wPin", self.id))
            .copied()
            .or_else(|| pin_net.get(&format!("{}-PinM", self.id)).copied())
    }

    pub fn pot_pin_b_node(&self, pin_net: &rustc_hash::FxHashMap<String, usize>) -> Option<usize> {
        pin_net
            .get(&format!("{}-rPin", self.id))
            .copied()
            .or_else(|| pin_net.get(&format!("{}-PinB", self.id)).copied())
    }

    /// SCR anode pin.
    pub fn scr_anode_pin(&self) -> String {
        format!("{}-lPin", self.id)
    }

    /// SCR cathode pin.
    pub fn scr_cathode_pin(&self) -> String {
        format!("{}-rPin", self.id)
    }

    /// SCR gate pin.
    pub fn scr_gate_pin(&self) -> String {
        format!("{}-gPin", self.id)
    }

    /// TRIAC MT1 pin.
    pub fn triac_mt1_pin(&self) -> String {
        format!("{}-lPin", self.id)
    }

    /// TRIAC MT2 pin.
    pub fn triac_mt2_pin(&self) -> String {
        format!("{}-rPin", self.id)
    }

    /// TRIAC gate pin.
    pub fn triac_gate_pin(&self) -> String {
        format!("{}-gPin", self.id)
    }

    /// Analog Mux Common pin.
    pub fn mux_common_pin(&self) -> String {
        format!("{}-PinInput", self.id)
    }

    /// Analog Mux Enable pin.
    pub fn mux_enable_pin(&self) -> String {
        format!("{}-PinEnable", self.id)
    }

    /// Analog Mux Address pin.
    pub fn mux_addr_pin(&self, i: usize) -> String {
        format!("{}-pinAddr{i}", self.id)
    }

    /// Analog Mux Channel pin.
    pub fn mux_channel_pin(&self, i: usize) -> String {
        format!("{}-pinY{i}", self.id)
    }

    pub fn build_pin_cache(&self, pin_net: &rustc_hash::FxHashMap<String, usize>) -> PinCache {
        let (oscope_gnd, oscope_pins, la_pins) = match self.kind {
            Kind::Oscope { .. } => {
                let gnd = pin_net.get(&self.plot_gnd_pin()).copied();
                let mut pins = [None; 4];
                for i in 0..4 {
                    pins[i] = pin_net.get(&self.plot_pin(i)).copied();
                }
                (gnd, pins, [None; 8])
            }
            Kind::LAnalizer { .. } => {
                let mut pins = [None; 8];
                for i in 0..8 {
                    pins[i] = pin_net.get(&self.plot_pin(i)).copied();
                }
                (None, [None; 4], pins)
            }
            Kind::Lm555(_) => {
                let mut pins = [None; 8];
                for i in 0..8 {
                    pins[i] = pin_net.get(&format!("{}-ePin{i}", self.id)).copied();
                }
                (None, [None; 4], pins)
            }
            _ => (None, [None; 4], [None; 8]),
        };
        let gnd = match self.kind {
            Kind::WaveGen { .. } => pin_net
                .get(&format!("{}-gndnod", self.id))
                .copied()
                .or_else(|| pin_net.get(&self.gnd_pin()).copied()),
            _ => pin_net.get(&self.gnd_pin()).copied(),
        };
        let (left, mid, right) = match self.kind {
            Kind::Potentiometer { .. } => (
                self.pot_pin_a_node(pin_net),
                self.pot_pin_m_node(pin_net),
                self.pot_pin_b_node(pin_net),
            ),
            Kind::Scr { .. } => (
                pin_net
                    .get(&format!("{}-lPin", self.id))
                    .or_else(|| pin_net.get(&format!("{}-aPin", self.id)))
                    .copied(),
                None,
                pin_net
                    .get(&format!("{}-rPin", self.id))
                    .or_else(|| pin_net.get(&format!("{}-kPin", self.id)))
                    .copied(),
            ),
            Kind::Triac { .. } => (
                pin_net
                    .get(&format!("{}-lPin", self.id))
                    .or_else(|| pin_net.get(&format!("{}-mt1Pin", self.id)))
                    .copied(),
                None,
                pin_net
                    .get(&format!("{}-rPin", self.id))
                    .or_else(|| pin_net.get(&format!("{}-mt2Pin", self.id)))
                    .copied(),
            ),
            Kind::RgbLed { .. } => (
                pin_net.get(&format!("{}-rPin", self.id)).copied(),
                pin_net.get(&format!("{}-gPin", self.id)).copied(),
                pin_net.get(&format!("{}-bPin", self.id)).copied(),
            ),
            Kind::Servo { .. } => (
                pin_net.get(&format!("{}-PinV+", self.id)).copied(),
                pin_net.get(&format!("{}-PinGnd", self.id)).copied(),
                pin_net.get(&format!("{}-PinSig", self.id)).copied(),
            ),
            _ => (
                pin_net.get(&self.left_pin()).copied(),
                pin_net.get(&self.mid_pin()).copied(),
                pin_net.get(&self.right_pin()).copied(),
            ),
        };
        let gate = match self.kind {
            Kind::Scr { .. } | Kind::Triac { .. } => {
                pin_net.get(&format!("{}-gPin", self.id)).copied()
            }
            _ => pin_net.get(&self.gate_pin()).copied(),
        };
        let (mux_en, mux_addr) = match self.kind {
            Kind::AnalogMux { .. } => {
                let en = pin_net
                    .get(&format!("{}-PinEnable", self.id))
                    .or_else(|| pin_net.get(&format!("{}-en", self.id)))
                    .copied();
                let mut addr = [None; 4];
                for i in 0..4 {
                    addr[i] = pin_net.get(&format!("{}-pinAddr{i}", self.id)).copied();
                }
                (en, addr)
            }
            _ => (None, [None; 4]),
        };
        let in0 = match self.kind {
            Kind::Adc { .. } => pin_net.get(&format!("{}-vin", self.id)).copied(),
            _ => pin_net.get(&self.comparator_in0_pin()).copied(),
        };
        let out = match self.kind {
            Kind::RgbLed { .. } => pin_net.get(&format!("{}-cPin", self.id)).copied(),
            _ => pin_net.get(&self.out_pin()).copied(),
        };
        let switch_poles = match self.kind {
            Kind::Switch {
                poles,
                double_throw,
                ..
            } => (0..poles)
                .map(|i| {
                    let p = pin_net.get(&format!("{}-pinP{i}", self.id)).copied();
                    if double_throw {
                        let n0 = pin_net
                            .get(&format!("{}-switch{}pinN", self.id, 2 * i))
                            .copied();
                        let n1 = pin_net
                            .get(&format!("{}-switch{}pinN", self.id, 2 * i + 1))
                            .copied();
                        (p, n0, n1)
                    } else {
                        let n0 = pin_net.get(&format!("{}-switch{i}pinN", self.id)).copied();
                        (p, n0, None)
                    }
                })
                .collect(),
            _ => Vec::new(),
        };
        PinCache {
            left,
            right,
            gnd,
            out,
            mid,
            in0,
            in1: pin_net.get(&self.comparator_in1_pin()).copied(),
            collector: pin_net.get(&self.collector_pin()).copied(),
            emitter: pin_net.get(&self.emitter_pin()).copied(),
            base: pin_net.get(&self.base_pin()).copied(),
            drain: pin_net.get(&self.drain_pin()).copied(),
            source: pin_net.get(&self.source_pin()).copied(),
            gate,
            input_p: pin_net.get(&self.input_p_pin()).copied(),
            input_n: pin_net.get(&self.input_n_pin()).copied(),
            output_amp: pin_net.get(&self.output_amp_pin()).copied(),
            power_pos: pin_net.get(&self.power_pos_pin()).copied(),
            power_neg: pin_net.get(&self.power_neg_pin()).copied(),
            probe: pin_net.get(&self.probe_pin()).copied(),
            voltreg_in: pin_net.get(&self.voltreg_in_pin()).copied(),
            voltreg_out: pin_net.get(&self.voltreg_out_pin()).copied(),
            voltreg_ref: pin_net.get(&self.voltreg_ref_pin()).copied(),
            oscope_gnd,
            oscope_pins,
            la_pins,
            mux_en,
            mux_addr,
            switch_poles,
        }
    }
}
