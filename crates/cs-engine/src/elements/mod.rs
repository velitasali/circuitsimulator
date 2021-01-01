pub mod comparator;
pub mod diode;
pub mod opamp;
pub mod transistor;
pub mod volt_reg;

use crate::CERO_DOUB;
use crate::digital::{FlipFlopState, GateState, LatchState, McuPinState, TestUnitState};
use crate::mcu::{McuComp, McuItemSpec};
use crate::net::ENode;
use crate::qemu::QemuComp;
use crate::script::ScriptCpu;
use std::sync::Arc;

pub use comparator::{
    COMPARATOR_DEFAULT_OUT_HIGH, COMPARATOR_DEFAULT_OUT_IMP, COMPARATOR_DEFAULT_OUT_LOW,
    ComparatorState,
};
pub use diode::{
    DIODE_DEFAULT_EM, DIODE_DEFAULT_RS, DIODE_DEFAULT_SAT_NA, DiodeState, LED_DEFAULT_IMAX,
    LED_DEFAULT_OHMS, LED_DEFAULT_VTH, LedState, VT as DIODE_VT, ZENER_DEFAULT_BV,
};
pub use opamp::{
    OPAMP_ACCURACY, OPAMP_DEFAULT_GAIN, OPAMP_DEFAULT_OUT_IMP, OPAMP_DEFAULT_VOLT_NEG,
    OPAMP_DEFAULT_VOLT_POS, OpAmpState,
};
pub use transistor::{
    BJT_DEFAULT_GAIN, BJT_DEFAULT_SAT, BJT_RGAIN, BjtState, JFET_ACCURACY, JFET_DEFAULT_IDSS,
    JFET_DEFAULT_LAMBDA_INV, JFET_DEFAULT_VP, JfetState, MOSFET_ACCURACY, MOSFET_DEFAULT_RDSON,
    MOSFET_DEFAULT_VTH, MosfetState, VT as BJT_VT,
};
pub use volt_reg::{
    VOLTREG_ADMIT, VOLTREG_CURRENT_EPS, VOLTREG_DEFAULT_VREF, VOLTREG_DROPOUT, VoltRegState,
};

/// IoPin source-mode admittance: `1 / cero_doub` = 1e9 S (1 nΩ).
pub const SOURCE_ADMIT: f64 = 1.0 / CERO_DOUB;

pub const RESISTOR_DEFAULT_OHMS: f64 = 100.0;
pub const BATTERY_DEFAULT_VOLTS: f64 = 5.0;
/// Battery constructor sets `m_admit = 1e3` → 1 mΩ.
pub const BATTERY_DEFAULT_OHMS: f64 = 1e-3;
pub const FIXED_VOLT_DEFAULT: f64 = 5.0;
/// C++ CapacitorBase default `10 µF`.
pub const CAPACITOR_DEFAULT_FARADS: f64 = 10e-6;
/// C++ Inductor default `1 H`.
pub const INDUCTOR_DEFAULT_HENRIES: f64 = 1.0;
/// C++ `MechContact::setSwitch` closed admitance.
pub const SWITCH_CLOSED_ADMIT: f64 = 1e3;
/// C++ AnalogClock default period `1e6` picoseconds.
pub const ANALOG_DT_DEFAULT: f64 = 1e-6;

const RESISTOR_MIN_OHMS: f64 = 1e-12;
const BATTERY_MIN_OHMS: f64 = 1e-14;
const CAP_MIN: f64 = 1e-15;
const IND_MIN: f64 = 1e-12;

mod constructors;
pub mod pins;
pub use pins::*;
mod stamp;

#[derive(Clone, Debug)]
pub enum Kind {
    Resistor {
        resistance: f64,
    },
    Battery {
        voltage: f64,
        resistance: f64,
    },
    Ground,
    FixedVolt {
        voltage: f64,
    },
    Junction,
    /// Backward-Euler companion: `G = C/dt`, `Ieq = V_prev · G`.
    Capacitor {
        capacitance: f64,
        volt: f64,
    },
    ElCapacitor {
        capacitance: f64,
        volt: f64,
    },
    /// Backward-Euler companion: `G = dt/L`, `Ieq := Ieq - V · G`.
    Inductor {
        inductance: f64,
        ieq: f64,
    },
    Switch {
        closed: bool,
        poles: usize,
        double_throw: bool,
    },
    Diode {
        state: DiodeState,
        zener: bool,
    },
    Led {
        state: LedState,
    },
    Bjt {
        state: BjtState,
    },
    Mosfet {
        state: MosfetState,
    },
    OpAmp {
        state: OpAmpState,
    },
    Jfet {
        state: JfetState,
    },
    Comparator {
        state: ComparatorState,
    },
    VoltReg {
        state: VoltRegState,
    },
    Probe {
        threshold: f64,
        small: bool,
    },
    Voltmeter {
        rms: bool,
        last_out: f64,
    },
    Ammeter {
        rms: bool,
        last_out: f64,
    },
    FreqMeter {
        filter: f64,
    },
    Oscope {
        connect_gnd: bool,
        input_imped: f64,
        tunnels: [String; 4],
    },
    LAnalizer {
        connect_gnd: bool,
        input_imped: f64,
        tunnels: [String; 8],
    },
    Gate(GateState),
    FlipFlop(FlipFlopState),
    Latch(LatchState),
    McuPin(McuPinState),
    /// C++ `Mcu` / `eMcu`. Drawn later (catalog-last); simulates headless.
    Mcu(McuComp),
    /// Placeholder until [`crate::subcircuit::expand_parsed`] loads `{device}.mcu`.
    McuItem(McuItemSpec),
    /// C++ `QemuDevice` / `Esp32` / `Stm32`. GPIO attached; live spawn optional. Not drawn.
    QemuDevice(QemuComp),
    /// C++ `ScriptCpu`. AngelScript `IoPort` / `IoPin` / `McuPort` / `McuPin`; not drawn.
    ScriptCpu(Box<ScriptCpu>),
    /// C++ `TestUnit`. Not drawn on the canvas (catalog-last).
    TestUnit(TestUnitState),
    /// C++ `Hd44780` character LCD display.
    Hd44780(crate::digital::Hd44780State),
    /// C++ `Ssd1306` graphical OLED display.
    Ssd1306(crate::digital::Ssd1306State),
    Mux(crate::digital::MuxState),
    Demux(crate::digital::DemuxState),
    BcdToDec(crate::digital::BcdToDecState),
    DecToBcd(crate::digital::DecToBcdState),
    BcdTo7S(crate::digital::BcdTo7SState),
    I2CToParallel(crate::digital::I2CToParallelState),
    Adc(crate::digital::AdcState),
    Dac(crate::digital::DacState),
    Counter(crate::digital::CounterState),
    BinCounter(crate::digital::BinCounterState),
    FullAdder(crate::digital::FullAdderState),
    HalfAdder(crate::digital::FullAdderState),
    MagnitudeComp(crate::digital::MagnitudeCompState),
    ShiftReg(crate::digital::ShiftRegState),
    Function(crate::digital::FunctionState),
    Memory(crate::digital::MemoryState),
    DynamicMemory(crate::digital::DynamicMemoryState),
    I2CRam(crate::digital::I2CRamState),
    Lm555(crate::digital::Lm555State),
    /// Named net alias (C++ `Tunnel`). `pin_id` is the full pin id.
    Tunnel {
        name: String,
        pin_id: String,
    },
    /// Placeholder until [`crate::subcircuit::expand_parsed`] flattens it.
    Subcircuit {
        device: String,
        logic_symbol: bool,
        package_name: Option<String>,
    },
    Clock {
        voltage: f64,
        freq_khz: f64,
        state: bool,
    },
    Rail {
        voltage: f64,
    },
    VoltSource {
        value: f64,
        running: bool,
    },
    CurrSource {
        value: f64,
        running: bool,
    },
    Csource {
        control_pins: bool,
        curr_source: bool,
        curr_control: bool,
        gain: f64,
        volt: f64,
        current: f64,
    },
    WaveGen {
        wave_type: String,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
        phase: f64,
        steps: i32,
        bipolar: bool,
        floating: bool,
        file: String,
        wav_data: Option<Arc<Vec<f64>>>,
        wav_index: usize,
        v_out: f64,
    },
    Push {
        closed: bool,
        poles: usize,
    },
    SwitchDip {
        size: usize,
        state: u32,
        common_pin: bool,
    },
    KeyPad {
        rows: usize,
        cols: usize,
        key: String,
        diodes: bool,
        dir: bool,
        pressed: Option<(usize, usize)>,
    },
    Relay {
        norm_close: bool,
        double_throw: bool,
        poles: usize,
        i_on: f64,
        i_off: f64,
        active: bool,
    },
    Potentiometer {
        resistance: f64,
        wiper: f64,
    },
    TouchPad {
        width: i32,
        height: i32,
        rx_min: f64,
        rx_max: f64,
        ry_min: f64,
        ry_max: f64,
        x_pos: i32,
        y_pos: i32,
    },
    Ky023 {
        stick_x: f64,
        stick_y: f64,
        btn_down: bool,
    },
    Ky040 {
        steps: u32,
        dial_val: i32,
        btn_closed: bool,
        state_a: bool,
        state_b: bool,
    },
    Sr04 {
        distance: f64,
        use_slider: bool,
        trigger_high: bool,
        echo_high: bool,
    },
    Dht22 {
        model: String,
        temp: f64,
        humi: f64,
        out_state: bool,
        pin_driven: bool,
    },
    Ds18b20 {
        rom: String,
        temp: f64,
        dq_low: bool,
    },
    Ds1621 {
        temp: f64,
        th: f64,
        tl: f64,
        active: bool,
        tout_high: bool,
        sda_low: bool,
        scl_low: bool,
    },
    Ds1307 {
        time_updated: bool,
        sqw_freq: f64,
        sqw_enabled: bool,
        sqw_state: bool,
        sda_low: bool,
        scl_low: bool,
    },
    DcMotor {
        rpm_nominal: i32,
        volt_nominal: f64,
        resistance: f64,
        speed: f64,
        angle: f64,
    },
    Stepper {
        bipolar: bool,
        steps: i32,
        resistance: f64,
        angle: f64,
    },
    Servo {
        speed: f64,
        min_pulse: f64,
        max_pulse: f64,
        pos: f64,
        target_pos: f64,
        pulse_start_ps: u64,
        sig_high: bool,
    },
    SdCard {
        file: String,
        card_inserted: bool,
    },
    Esp01 {
        baud_rate: u32,
        debug: bool,
    },
    TftDisplay {
        controller: String,
        width: u32,
        height: u32,
        scale: f64,
        bgr: bool,
    },
    Pcd8544(crate::digital::Pcd8544State),
    Sh1107(crate::digital::Sh1107State),
    Ks0108(crate::digital::Ks0108State),
    Pcf8833Display {
        width: u32,
        height: u32,
    },
    Aip31068(crate::digital::Aip31068State),
    VarResistor {
        resistance: f64,
    },
    ResistorDip {
        size: usize,
        resistance: f64,
        bussed: bool,
    },
    Ldr {
        resistance: f64,
        lux: f64,
    },
    Thermistor {
        resistance: f64,
        temp_c: f64,
    },
    Rtd {
        resistance: f64,
        temp_c: f64,
    },
    Strain {
        resistance: f64,
        strain: f64,
    },
    VarCapacitor {
        capacitance: f64,
        volt: f64,
    },
    VarInductor {
        inductance: f64,
        ieq: f64,
    },
    Transformer {
        inductance1: f64,
        inductance2: f64,
        coupling: f64,
    },
    Scr {
        v_gate_th: f64,
        i_hold: f64,
        conducting: bool,
    },
    Diac {
        v_breakover: f64,
        conducting: bool,
    },
    Triac {
        v_gate_th: f64,
        i_hold: f64,
        conducting: bool,
    },
    AnalogMux {
        channels: usize,
        selected: usize,
        on_res: f64,
    },
    RgbLed {
        common_anode: bool,
        state_r: f64,
        state_g: f64,
        state_b: f64,
    },
    LedBar {
        segments: usize,
        states: u32,
        grounded: bool,
    },
    SevenSegment {
        common_anode: bool,
        segments: u8,
    },
    LedMatrix {
        rows: usize,
        cols: usize,
        vertical_pins: bool,
        color: String,
        threshold: f64,
        max_current: f64,
        resistance: f64,
    },
    Max72xx {
        modules: usize,
        color: String,
    },
    Ws2812 {
        count: usize,
        rows: usize,
        cols: usize,
        rst_time_ns: u32,
        t0h_ns: u32,
        t0l_ns: u32,
        t1h_ns: u32,
        t1l_ns: u32,
    },
    Dial {
        value: f64,
        min_val: f64,
        max_val: f64,
        step: f64,
    },
    Shape {
        shape_kind: String,
        width: f64,
        height: f64,
        text: String,
        color: String,
        font: String,
        font_color: String,
        font_size: i32,
        border: i32,
        opacity: f64,
    },
    SevenSegmentBCD {
        color: String,
        common_anode: bool,
    },
    Lamp {
        voltage: f64,
        power: f64,
        r_cold: f64,
        resistance: f64,
    },
    AudioOut {
        source: crate::audio::Source,
        last_v: f64,
    },
    Bus {
        width: usize,
    },
    Header {
        pins_count: usize,
    },
    Socket {
        pins_count: usize,
    },
    SerialPort {
        port_name: String,
        baud_rate: u32,
    },
    SerialTerm {
        baud_rate: u32,
    },
    SubPackage,
}

#[derive(Clone, Debug, Default)]
pub struct PinCache {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub gnd: Option<usize>,
    pub out: Option<usize>,
    pub mid: Option<usize>,
    pub in0: Option<usize>,
    pub in1: Option<usize>,
    pub collector: Option<usize>,
    pub emitter: Option<usize>,
    pub base: Option<usize>,
    pub drain: Option<usize>,
    pub source: Option<usize>,
    pub gate: Option<usize>,
    pub input_p: Option<usize>,
    pub input_n: Option<usize>,
    pub output_amp: Option<usize>,
    pub power_pos: Option<usize>,
    pub power_neg: Option<usize>,
    pub probe: Option<usize>,
    pub voltreg_in: Option<usize>,
    pub voltreg_out: Option<usize>,
    pub voltreg_ref: Option<usize>,
    pub oscope_gnd: Option<usize>,
    pub oscope_pins: [Option<usize>; 4],
    pub la_pins: [Option<usize>; 8],
    pub mux_en: Option<usize>,
    pub mux_addr: [Option<usize>; 4],
    pub switch_poles: Vec<(Option<usize>, Option<usize>, Option<usize>)>,
}

#[derive(Clone, Debug)]
pub struct Comp {
    pub id: String,
    pub kind: Kind,
}

impl Kind {
    /// Non-linear companion model update for Newton-Raphson iterations.
    /// Returns `true` if this element has converged, `false` otherwise.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        match self {
            Kind::Diode { state, .. } => state.update_nonlinear(cache, nodes),
            Kind::Led { state } => state.update_nonlinear(cache, nodes),
            Kind::Bjt { state } => state.update_nonlinear(cache, nodes),
            Kind::Mosfet { state } => state.update_nonlinear(cache, nodes),
            Kind::OpAmp { state } => state.update_nonlinear(cache, nodes),
            Kind::Jfet { state } => state.update_nonlinear(cache, nodes),
            Kind::VoltReg { state } => state.update_nonlinear(cache, nodes),
            Kind::Scr {
                v_gate_th,
                i_hold,
                conducting,
            } => {
                let va_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
                let vk_opt = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
                if va_opt.is_none() || vk_opt.is_none() {
                    let was_off = !*conducting;
                    *conducting = false;
                    return was_off;
                }
                let va = va_opt.unwrap();
                let vk = vk_opt.unwrap();
                let vg = cache
                    .gate
                    .and_then(|i| nodes.get(i))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let v_gate = vg - vk;
                let v_anode = va - vk;
                let mut state = *conducting;
                if state {
                    let i_anode = (v_anode - 0.7) * crate::SWITCH_CLOSED_ADMIT;
                    if i_anode < *i_hold || v_anode < 0.1 {
                        state = false;
                    }
                } else if v_gate > *v_gate_th && v_anode > 0.7 {
                    state = true;
                }
                if *conducting != state {
                    *conducting = state;
                    false
                } else {
                    true
                }
            }
            Kind::Triac {
                v_gate_th,
                i_hold,
                conducting,
            } => {
                let v1_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
                let v2_opt = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
                if v1_opt.is_none() || v2_opt.is_none() {
                    let was_off = !*conducting;
                    *conducting = false;
                    return was_off;
                }
                let v1 = v1_opt.unwrap();
                let v2 = v2_opt.unwrap();
                let vg = cache
                    .gate
                    .and_then(|i| nodes.get(i))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let v_gate = (vg - v1).abs();
                let v_main = (v2 - v1).abs();
                let mut state = *conducting;
                if state {
                    let i_main = v_main * crate::SWITCH_CLOSED_ADMIT;
                    if i_main < *i_hold {
                        state = false;
                    }
                } else if v_gate > *v_gate_th && v_main > 0.2 {
                    state = true;
                }
                if *conducting != state {
                    *conducting = state;
                    false
                } else {
                    true
                }
            }
            Kind::Diac {
                v_breakover,
                conducting,
            } => {
                let v1_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
                let v2_opt = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
                if v1_opt.is_none() || v2_opt.is_none() {
                    let was_off = !*conducting;
                    *conducting = false;
                    return was_off;
                }
                let v1 = v1_opt.unwrap();
                let v2 = v2_opt.unwrap();
                let v_diff = (v1 - v2).abs();
                let mut state = *conducting;
                if state {
                    let i_main = v_diff * (1.0 / 500.0);
                    if i_main < 0.005 {
                        state = false;
                    }
                } else if v_diff > *v_breakover {
                    state = true;
                }
                if *conducting != state {
                    *conducting = state;
                    false
                } else {
                    true
                }
            }
            Kind::AnalogMux {
                channels,
                selected,
                on_res: _,
            } => {
                let ven = cache
                    .mux_en
                    .and_then(|i| nodes.get(i))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let enabled = ven < 2.5;
                let mut sel = 0usize;
                for i in 0..4 {
                    let va = cache.mux_addr[i]
                        .and_then(|idx| nodes.get(idx))
                        .map(|n| n.volt)
                        .unwrap_or(0.0);
                    if va > 2.5 {
                        sel |= 1 << i;
                    }
                }
                if !enabled {
                    sel = usize::MAX;
                } else {
                    sel = sel.min(channels.saturating_sub(1));
                }
                if *selected != sel {
                    *selected = sel;
                    false
                } else {
                    true
                }
            }
            _ => true,
        }
    }

    /// Calculate the current leaving this component's pin into the connected wire (in Amperes).
    /// Positive current means leaving the pin into the circuit; negative means entering the pin.
    pub fn current_out_of_pin(
        &self,
        comp_id: &str,
        pin: &str,
        suffix: PinSuffix,
        cache: &PinCache,
        nodes: &[ENode],
        dt: f64,
        pin_volt: impl Fn(&str) -> Option<f64>,
    ) -> Option<f64> {
        match (self, suffix) {
            (Kind::Resistor { resistance }, PinSuffix::Left) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v1 - v0) / resistance)
            }
            (Kind::Resistor { resistance }, PinSuffix::Right) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v0 - v1) / resistance)
            }
            (
                Kind::Battery {
                    voltage,
                    resistance,
                },
                PinSuffix::Left,
            ) => {
                let g = 1.0 / *resistance;
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((*voltage - v0 + v1) * g)
            }
            (
                Kind::Battery {
                    voltage,
                    resistance,
                },
                PinSuffix::Right,
            ) => {
                let g = 1.0 / *resistance;
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v0 - v1 - *voltage) * g)
            }
            (Kind::Ground, PinSuffix::Ground) => {
                let v = pin_volt(pin)?;
                Some((crate::CERO_DOUB - v) * crate::SOURCE_ADMIT)
            }
            (Kind::FixedVolt { voltage } | Kind::Rail { voltage }, PinSuffix::OutNod) => {
                let v = pin_volt(pin)?;
                Some((*voltage - v) * crate::SOURCE_ADMIT)
            }
            (Kind::Clock { voltage, state, .. }, PinSuffix::OutNod) => {
                let v = pin_volt(pin)?;
                let target = if *state { *voltage } else { 0.0 };
                Some((target - v) * crate::SOURCE_ADMIT)
            }
            (Kind::VoltSource { value, running }, PinSuffix::OutPin) => {
                let v = pin_volt(pin)?;
                let target = if *running { *value } else { 0.0 };
                Some((target - v) * crate::SOURCE_ADMIT)
            }
            (Kind::CurrSource { value, running }, PinSuffix::OutPin) => {
                Some(if *running { *value } else { 0.0 })
            }
            (
                Kind::WaveGen {
                    amplitude,
                    offset,
                    bipolar,
                    floating,
                    v_out,
                    ..
                },
                PinSuffix::OutNod | PinSuffix::GndNod,
            ) => {
                if *bipolar {
                    let volt = 2.0 * *amplitude * (*v_out - 0.5);
                    if *floating {
                        let v0 = cache.out.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        let v1 = cache.gnd.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        Some(match suffix {
                            PinSuffix::OutNod => (volt - (v0 - v1)) * crate::SOURCE_ADMIT,
                            PinSuffix::GndNod => ((v0 - v1) - volt) * crate::SOURCE_ADMIT,
                            _ => 0.0,
                        })
                    } else {
                        let half_v = volt / 2.0;
                        let v = pin_volt(pin)?;
                        Some(match suffix {
                            PinSuffix::OutNod => (*offset + half_v - v) * crate::SOURCE_ADMIT,
                            PinSuffix::GndNod => (*offset - half_v - v) * crate::SOURCE_ADMIT,
                            _ => 0.0,
                        })
                    }
                } else if suffix == PinSuffix::OutNod {
                    let volt_base = *offset - *amplitude;
                    let target = volt_base + 2.0 * *amplitude * *v_out;
                    let v = pin_volt(pin)?;
                    Some((target - v) * crate::SOURCE_ADMIT)
                } else {
                    None
                }
            }
            (Kind::Potentiometer { resistance, wiper }, suffix) => {
                let r1 = (*wiper * resistance).max(RESISTOR_MIN_OHMS);
                let r2 = ((1.0 - wiper) * resistance).max(RESISTOR_MIN_OHMS);
                let va = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
                let vm = cache.mid.and_then(|i| nodes.get(i)).map(|n| n.volt);
                let vb = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
                match suffix {
                    PinSuffix::Left => {
                        let (va, vm) = (va?, vm?);
                        Some((vm - va) / r1)
                    }
                    PinSuffix::Right => {
                        let (vb, vm) = (vb?, vm?);
                        Some((vm - vb) / r2)
                    }
                    PinSuffix::Mid => {
                        let vm = vm?;
                        let ia = va.map(|va| (va - vm) / r1).unwrap_or(0.0);
                        let ib = vb.map(|vb| (vb - vm) / r2).unwrap_or(0.0);
                        Some(ia + ib)
                    }
                    _ => None,
                }
            }
            (
                Kind::VarResistor { resistance }
                | Kind::Ldr { resistance, .. }
                | Kind::Thermistor { resistance, .. }
                | Kind::Rtd { resistance, .. }
                | Kind::Strain { resistance, .. }
                | Kind::Lamp { resistance, .. }
                | Kind::DcMotor { resistance, .. },
                PinSuffix::Left,
            ) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v1 - v0) / resistance)
            }
            (
                Kind::VarResistor { resistance }
                | Kind::Ldr { resistance, .. }
                | Kind::Thermistor { resistance, .. }
                | Kind::Rtd { resistance, .. }
                | Kind::Strain { resistance, .. }
                | Kind::Lamp { resistance, .. }
                | Kind::DcMotor { resistance, .. },
                PinSuffix::Right,
            ) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v0 - v1) / resistance)
            }
            (Kind::AudioOut { source, .. }, PinSuffix::Left) => {
                let admit = source.stamp_admit();
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v1 - v0) * admit)
            }
            (Kind::AudioOut { source, .. }, PinSuffix::Right) => {
                let admit = source.stamp_admit();
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                Some((v0 - v1) * admit)
            }
            (Kind::Junction, _) => None,
            (
                Kind::Capacitor { capacitance, volt } | Kind::ElCapacitor { capacitance, volt },
                PinSuffix::Left | PinSuffix::Right,
            ) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let g = *capacitance / dt.max(1e-18);
                let i = (v0 - v1) * g - *volt * g;
                Some(if suffix == PinSuffix::Left { -i } else { i })
            }
            (Kind::Inductor { inductance, ieq }, PinSuffix::Left | PinSuffix::Right) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let g = dt / inductance.max(1e-18);
                let i = (v0 - v1) * g - *ieq;
                Some(if suffix == PinSuffix::Left { -i } else { i })
            }
            (
                Kind::Switch {
                    closed,
                    poles,
                    double_throw,
                },
                suffix,
            ) => match suffix {
                PinSuffix::SwitchP(pole_idx) | PinSuffix::IndexedLeft(pole_idx)
                    if pole_idx < *poles =>
                {
                    let (target_pin, conducts) = if *double_throw {
                        let target = if *closed {
                            format!("{comp_id}-switch{}pinN", 2 * pole_idx)
                        } else {
                            format!("{comp_id}-switch{}pinN", 2 * pole_idx + 1)
                        };
                        (target, true)
                    } else {
                        (format!("{comp_id}-switch{pole_idx}pinN"), *closed)
                    };
                    if !conducts {
                        return Some(0.0);
                    }
                    let (v0, v1) = if !*double_throw && *poles == 1 && pole_idx == 0 {
                        let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        (v0, v1)
                    } else {
                        let v0 = pin_volt(pin)?;
                        let v1 = pin_volt(&target_pin)?;
                        (v0, v1)
                    };
                    let i = (v0 - v1) * crate::SWITCH_CLOSED_ADMIT;
                    Some(-i)
                }
                PinSuffix::Left if *poles == 1 => {
                    let target_pin = if *double_throw {
                        if *closed {
                            format!("{comp_id}-switch0pinN")
                        } else {
                            format!("{comp_id}-switch1pinN")
                        }
                    } else {
                        format!("{comp_id}-switch0pinN")
                    };
                    let conducts = *double_throw || *closed;
                    if !conducts {
                        return Some(0.0);
                    }
                    let (v0, v1) = if !*double_throw {
                        let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        (v0, v1)
                    } else {
                        let v0 = pin_volt(pin)?;
                        let v1 = pin_volt(&target_pin)?;
                        (v0, v1)
                    };
                    let i = (v0 - v1) * crate::SWITCH_CLOSED_ADMIT;
                    Some(-i)
                }
                PinSuffix::SwitchN(idx) | PinSuffix::IndexedRight(idx) => {
                    let (pole_idx, conducts) = if *double_throw {
                        let pole = idx / 2;
                        let is_throw_0 = idx % 2 == 0;
                        (pole, is_throw_0 == *closed)
                    } else {
                        (idx, *closed)
                    };
                    if pole_idx >= *poles {
                        return None;
                    }
                    if !conducts {
                        return Some(0.0);
                    }
                    let (v0, v1) = if !*double_throw && *poles == 1 && pole_idx == 0 {
                        let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        (v0, v1)
                    } else {
                        let lp = format!("{comp_id}-pinP{pole_idx}");
                        let v0 = pin_volt(&lp)?;
                        let v1 = pin_volt(pin)?;
                        (v0, v1)
                    };
                    let i = (v0 - v1) * crate::SWITCH_CLOSED_ADMIT;
                    Some(i)
                }
                PinSuffix::Right if *poles == 1 => {
                    let idx = 0;
                    let conducts = if *double_throw { *closed } else { *closed };
                    if !conducts {
                        return Some(0.0);
                    }
                    let (v0, v1) = if !*double_throw {
                        let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                        (v0, v1)
                    } else {
                        let lp = format!("{comp_id}-pinP{idx}");
                        let v0 = pin_volt(&lp)?;
                        let v1 = pin_volt(pin)?;
                        (v0, v1)
                    };
                    let i = (v0 - v1) * crate::SWITCH_CLOSED_ADMIT;
                    Some(i)
                }
                _ => None,
            },
            (Kind::Diode { state, .. }, PinSuffix::Left) => Some(-state.current),
            (Kind::Led { state }, PinSuffix::Left) => Some(-state.current),
            (Kind::Diode { state, .. }, PinSuffix::Right) => Some(state.current),
            (Kind::Led { state }, PinSuffix::Right) => Some(state.current),
            (
                Kind::RgbLed { common_anode, .. },
                PinSuffix::Right | PinSuffix::Ground | PinSuffix::RgbB | PinSuffix::RgbC,
            ) => {
                let vr = cache
                    .left
                    .and_then(|idx| nodes.get(idx))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let vg = cache
                    .mid
                    .and_then(|idx| nodes.get(idx))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let vb = cache
                    .right
                    .and_then(|idx| nodes.get(idx))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let vc = cache
                    .out
                    .and_then(|idx| nodes.get(idx))
                    .map(|n| n.volt)
                    .unwrap_or(0.0);
                let eval_vd = |va: f64, vk: f64| -> f64 {
                    let vd = va - vk;
                    if vd > 1.8 { (vd - 1.8) / 100.0 } else { 0.0 }
                };
                let (ir, ig, ib) = if *common_anode {
                    (eval_vd(vc, vr), eval_vd(vc, vg), eval_vd(vc, vb))
                } else {
                    (eval_vd(vr, vc), eval_vd(vg, vc), eval_vd(vb, vc))
                };
                Some(match suffix {
                    PinSuffix::Right => {
                        if *common_anode {
                            ir
                        } else {
                            -ir
                        }
                    }
                    PinSuffix::Ground => {
                        if *common_anode {
                            ig
                        } else {
                            -ig
                        }
                    }
                    PinSuffix::RgbB => {
                        if *common_anode {
                            ib
                        } else {
                            -ib
                        }
                    }
                    PinSuffix::RgbC => {
                        if *common_anode {
                            -(ir + ig + ib)
                        } else {
                            ir + ig + ib
                        }
                    }
                    _ => 0.0,
                })
            }
            (
                Kind::LedBar {
                    segments, grounded, ..
                },
                PinSuffix::IndexedLeft(i),
            ) => {
                if i < *segments {
                    let va = pin_volt(pin)?;
                    let vk = if *grounded {
                        0.0
                    } else {
                        let ra = format!("{comp_id}-rPin{i}");
                        pin_volt(&ra)?
                    };
                    let v_diff = va - vk;
                    let current = if v_diff > 1.8 {
                        (v_diff - 1.8) / 100.0
                    } else {
                        0.0
                    };
                    Some(-current)
                } else {
                    None
                }
            }
            (
                Kind::LedBar {
                    segments, grounded, ..
                },
                PinSuffix::IndexedRight(i),
            ) if !*grounded => {
                if i < *segments {
                    let la = format!("{comp_id}-lPin{i}");
                    let va = pin_volt(&la)?;
                    let vk = pin_volt(pin)?;
                    let v_diff = va - vk;
                    let current = if v_diff > 1.8 {
                        (v_diff - 1.8) / 100.0
                    } else {
                        0.0
                    };
                    Some(current)
                } else {
                    None
                }
            }
            (Kind::Bjt { state }, PinSuffix::Collector) => Some(-state.ic),
            (Kind::Bjt { state }, PinSuffix::Emitter) => Some(-state.ie),
            (Kind::Bjt { state }, PinSuffix::Base) => Some(-state.base_curr),
            (Kind::Mosfet { state }, PinSuffix::Drain | PinSuffix::Source) => {
                let vd = cache.drain.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let vs = cache.source.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let i = state.admit * (vd - vs) - state.current;
                Some(if suffix == PinSuffix::Drain { -i } else { i })
            }
            (Kind::Mosfet { .. }, PinSuffix::Gate) => Some(0.0),
            (Kind::OpAmp { state }, PinSuffix::OutputAmp) => {
                let v = pin_volt(pin)?;
                Some((state.last_out - v) * state.admit())
            }
            (
                Kind::OpAmp { .. },
                PinSuffix::InputPos
                | PinSuffix::InputNeg
                | PinSuffix::PowerPos
                | PinSuffix::PowerNeg,
            ) => Some(0.0),
            (Kind::Jfet { state }, PinSuffix::Drain | PinSuffix::Source) => {
                let vd = cache.drain.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let vs = cache.source.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let i = state.admit * (vd - vs);
                Some(if suffix == PinSuffix::Drain { -i } else { i })
            }
            (Kind::Jfet { .. }, PinSuffix::Gate) => Some(0.0),
            (Kind::Comparator { state }, PinSuffix::Out) => {
                let v = pin_volt(pin)?;
                Some(state.output.current_out(v))
            }
            (Kind::Comparator { .. }, PinSuffix::CompIn0 | PinSuffix::CompIn1) => Some(0.0),
            (Kind::VoltReg { state }, PinSuffix::VoltRegIn | PinSuffix::OutputAmp) => {
                let vin = cache
                    .voltreg_in
                    .and_then(|i| nodes.get(i))
                    .map(|n| n.volt)?;
                let vout = cache
                    .voltreg_out
                    .and_then(|i| nodes.get(i))
                    .map(|n| n.volt)?;
                let i = (vin - vout) * state.admit - state.last_current;
                Some(if suffix == PinSuffix::VoltRegIn {
                    -i
                } else {
                    i
                })
            }
            (Kind::VoltReg { .. }, PinSuffix::VoltRegRef) => Some(0.0),
            (Kind::Probe { .. }, PinSuffix::ProbeIn) => {
                let v = pin_volt(pin)?;
                Some(-v * crate::instruments::PROBE_ADMIT)
            }
            (Kind::Voltmeter { .. }, PinSuffix::Left | PinSuffix::Right) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let i = (v0 - v1) / crate::instruments::VOLTMETER_OHMS;
                Some(if suffix == PinSuffix::Left { -i } else { i })
            }
            (Kind::Voltmeter { last_out, .. }, PinSuffix::OutNod) => {
                let v = pin_volt(pin)?;
                Some((*last_out - v) * crate::SOURCE_ADMIT)
            }
            (Kind::Ammeter { .. }, PinSuffix::Left | PinSuffix::Right) => {
                let v0 = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let v1 = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)?;
                let i = (v0 - v1) / crate::instruments::AMMETER_OHMS;
                Some(if suffix == PinSuffix::Left { -i } else { i })
            }
            (Kind::Ammeter { last_out, .. }, PinSuffix::OutNod) => {
                let v = pin_volt(pin)?;
                Some((*last_out - v) * crate::SOURCE_ADMIT)
            }
            (Kind::FreqMeter { .. }, PinSuffix::Left) => Some(0.0),
            (
                Kind::Oscope {
                    connect_gnd,
                    input_imped,
                    ..
                }
                | Kind::LAnalizer {
                    connect_gnd,
                    input_imped,
                    ..
                },
                _,
            ) => {
                if !*connect_gnd {
                    return Some(0.0);
                }
                let admit = if *input_imped > 0.0 {
                    1.0 / (*input_imped * 1e6)
                } else {
                    crate::instruments::PLOT_INPUT_ADMIT
                };
                let v = pin_volt(pin).unwrap_or(0.0);
                Some(-v * admit)
            }
            (Kind::Gate(g), _) => {
                for p in g
                    .inputs
                    .iter()
                    .chain(std::iter::once(&g.output))
                    .chain(g.oe.as_ref())
                {
                    if p.id == pin {
                        let v = pin_volt(pin).unwrap_or(0.0);
                        return Some(p.current_out(v));
                    }
                }
                None
            }
            (Kind::FlipFlop(f), _) => {
                for p in [&f.d, &f.j, &f.k, &f.t, &f.set, &f.rst, &f.clk, &f.q, &f.qn] {
                    if p.id == pin {
                        let v = pin_volt(pin).unwrap_or(0.0);
                        return Some(p.current_out(v));
                    }
                }
                None
            }
            (Kind::Latch(l), _) => {
                for p in l
                    .inputs
                    .iter()
                    .chain(l.outputs.iter())
                    .chain([&l.clk, &l.reset, &l.oe])
                {
                    if p.id == pin {
                        let v = pin_volt(pin).unwrap_or(0.0);
                        return Some(p.current_out(v));
                    }
                }
                None
            }
            (Kind::McuPin(mcu_pin), _) => {
                if mcu_pin.pin.id == pin {
                    let v = pin_volt(pin).unwrap_or(0.0);
                    Some(mcu_pin.pin.current_out(v))
                } else {
                    None
                }
            }
            (Kind::Mcu(mcu), _) => {
                let p = mcu.pins.iter().find(|p| p.id == pin)?;
                let v = pin_volt(pin).unwrap_or(0.0);
                Some(p.current_out(v))
            }
            (Kind::QemuDevice(qemu), _) => {
                let p = qemu.pins.iter().find(|p| p.id == pin)?;
                let v = pin_volt(pin).unwrap_or(0.0);
                Some(p.current_out(v))
            }
            (Kind::ScriptCpu(cpu), _) => {
                let p = cpu.pins.iter().find(|p| p.id == pin)?;
                let v = pin_volt(pin).unwrap_or(0.0);
                Some(p.current_out(v))
            }
            _ => None,
        }
    }
}
