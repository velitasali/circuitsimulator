//! Factory and constructor methods for simulation elements.

use super::comparator::ComparatorState;
use super::diode::{DiodeState, LedState};
use super::opamp::OpAmpState;
use super::transistor::{BjtState, JfetState, MosfetState};
use super::volt_reg::VoltRegState;
use super::*;
use crate::digital::{FlipFlopState, GateState, LatchState, McuPinState, TestUnitState};
use crate::mcu::{McuComp, McuItemSpec};
use crate::qemu::QemuComp;
use crate::script::ScriptCpu;

impl Comp {
    pub fn resistor(id: impl Into<String>, ohms: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Resistor {
                resistance: ohms.max(RESISTOR_MIN_OHMS),
            },
        }
    }

    pub fn battery(id: impl Into<String>, volts: f64, ohms: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Battery {
                voltage: volts.max(1e-12),
                resistance: ohms.max(BATTERY_MIN_OHMS),
            },
        }
    }

    pub fn ground(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ground,
        }
    }

    pub fn fixed_volt(id: impl Into<String>, volts: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::FixedVolt { voltage: volts },
        }
    }

    pub fn junction(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Junction,
        }
    }

    pub fn capacitor(id: impl Into<String>, farads: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Capacitor {
                capacitance: farads.max(CAP_MIN),
                volt: 0.0,
            },
        }
    }

    pub fn el_capacitor(id: impl Into<String>, farads: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::ElCapacitor {
                capacitance: farads.max(CAP_MIN),
                volt: 0.0,
            },
        }
    }

    pub fn inductor(id: impl Into<String>, henries: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Inductor {
                inductance: henries.max(IND_MIN),
                ieq: 0.0,
            },
        }
    }

    pub fn switch(id: impl Into<String>, closed: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Switch {
                closed,
                poles: 1,
                double_throw: false,
            },
        }
    }

    pub fn switch_with_poles(id: impl Into<String>, closed: bool, poles: usize) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Switch {
                closed,
                poles: poles.max(1),
                double_throw: false,
            },
        }
    }

    pub fn switch_full(
        id: impl Into<String>,
        closed: bool,
        poles: usize,
        double_throw: bool,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Switch {
                closed,
                poles: poles.max(1),
                double_throw,
            },
        }
    }

    pub fn diode(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Diode {
                state: DiodeState::diode_default(),
                zener: false,
            },
        }
    }

    pub fn zener(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Diode {
                state: DiodeState::zener_default(),
                zener: true,
            },
        }
    }

    pub fn led(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Led {
                state: LedState::default_led(),
            },
        }
    }

    pub fn bjt(id: impl Into<String>, pnp: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Bjt {
                state: BjtState::new(pnp),
            },
        }
    }

    pub fn mosfet(id: impl Into<String>, p_channel: bool, depletion: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Mosfet {
                state: MosfetState::new(p_channel, depletion),
            },
        }
    }

    pub fn opamp(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::OpAmp {
                state: OpAmpState::new(),
            },
        }
    }

    pub fn jfet(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Jfet {
                state: JfetState::new(),
            },
        }
    }

    pub fn comparator(id: impl Into<String>) -> Self {
        let id = id.into();
        let mut state = ComparatorState::new();
        state.attach_id(&id);
        Self {
            id,
            kind: Kind::Comparator { state },
        }
    }

    pub fn scr(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Scr {
                v_gate_th: 0.7,
                i_hold: 0.005,
                conducting: false,
            },
        }
    }

    pub fn triac(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Triac {
                v_gate_th: 0.7,
                i_hold: 0.005,
                conducting: false,
            },
        }
    }

    pub fn diac(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Diac {
                v_breakover: 30.0,
                conducting: false,
            },
        }
    }

    pub fn analog_mux(id: impl Into<String>, channels: usize) -> Self {
        Self {
            id: id.into(),
            kind: Kind::AnalogMux {
                channels,
                selected: 0,
                on_res: 100.0,
            },
        }
    }

    pub fn potentiometer(id: impl Into<String>, resistance: f64, wiper: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Potentiometer {
                resistance,
                wiper: wiper.clamp(0.0, 1.0),
            },
        }
    }

    pub fn led_matrix(
        id: impl Into<String>,
        rows: usize,
        cols: usize,
        vertical_pins: bool,
        color: impl Into<String>,
        threshold: f64,
        max_current: f64,
        resistance: f64,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::LedMatrix {
                rows,
                cols,
                vertical_pins,
                color: color.into(),
                threshold,
                max_current,
                resistance,
            },
        }
    }

    pub fn max72xx(id: impl Into<String>, modules: usize, color: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Max72xx {
                modules,
                color: color.into(),
            },
        }
    }

    pub fn ws2812(
        id: impl Into<String>,
        count: usize,
        rows: usize,
        cols: usize,
        rst_time_ns: u32,
        t0h_ns: u32,
        t0l_ns: u32,
        t1h_ns: u32,
        t1l_ns: u32,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ws2812 {
                count,
                rows,
                cols,
                rst_time_ns,
                t0h_ns,
                t0l_ns,
                t1h_ns,
                t1l_ns,
            },
        }
    }

    pub fn dial(id: impl Into<String>, value: f64, min_val: f64, max_val: f64, step: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Dial {
                value,
                min_val,
                max_val,
                step,
            },
        }
    }

    pub fn shape(
        id: impl Into<String>,
        shape_kind: impl Into<String>,
        width: f64,
        height: f64,
        text: impl Into<String>,
        color: impl Into<String>,
        font: impl Into<String>,
        font_color: impl Into<String>,
        font_size: i32,
        border: i32,
        opacity: f64,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Shape {
                shape_kind: shape_kind.into(),
                width,
                height,
                text: text.into(),
                color: color.into(),
                font: font.into(),
                font_color: font_color.into(),
                font_size,
                border,
                opacity,
            },
        }
    }

    pub fn seven_segment_bcd(
        id: impl Into<String>,
        color: impl Into<String>,
        common_anode: bool,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::SevenSegmentBCD {
                color: color.into(),
                common_anode,
            },
        }
    }

    pub fn touchpad(
        id: impl Into<String>,
        width: i32,
        height: i32,
        rx_min: f64,
        rx_max: f64,
        ry_min: f64,
        ry_max: f64,
        x_pos: i32,
        y_pos: i32,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::TouchPad {
                width,
                height,
                rx_min,
                rx_max,
                ry_min,
                ry_max,
                x_pos,
                y_pos,
            },
        }
    }

    pub fn keypad(
        id: impl Into<String>,
        rows: usize,
        cols: usize,
        key: impl Into<String>,
        diodes: bool,
        dir: bool,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::KeyPad {
                rows,
                cols,
                key: key.into(),
                diodes,
                dir,
                pressed: None,
            },
        }
    }

    pub fn ky023(id: impl Into<String>, stick_x: f64, stick_y: f64, btn_down: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ky023 {
                stick_x,
                stick_y,
                btn_down,
            },
        }
    }

    pub fn ky040(
        id: impl Into<String>,
        steps: u32,
        dial_val: i32,
        btn_closed: bool,
        state_a: bool,
        state_b: bool,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ky040 {
                steps,
                dial_val,
                btn_closed,
                state_a,
                state_b,
            },
        }
    }

    pub fn sr04(id: impl Into<String>, distance: f64, use_slider: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Sr04 {
                distance,
                use_slider,
                trigger_high: false,
                echo_high: false,
            },
        }
    }

    pub fn dht22(id: impl Into<String>, model: impl Into<String>, temp: f64, humi: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Dht22 {
                model: model.into(),
                temp,
                humi,
                out_state: true,
                pin_driven: false,
            },
        }
    }

    pub fn ds18b20(id: impl Into<String>, rom: impl Into<String>, temp: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ds18b20 {
                rom: rom.into(),
                temp,
                dq_low: false,
            },
        }
    }

    pub fn ds1621(id: impl Into<String>, temp: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ds1621 {
                temp,
                th: 30.0,
                tl: 10.0,
                active: true,
                tout_high: false,
                sda_low: false,
                scl_low: false,
            },
        }
    }

    pub fn ds1307(id: impl Into<String>, time_updated: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ds1307 {
                time_updated,
                sqw_freq: 1.0,
                sqw_enabled: true,
                sqw_state: false,
                sda_low: false,
                scl_low: false,
            },
        }
    }

    pub fn dcmotor(
        id: impl Into<String>,
        rpm_nominal: i32,
        volt_nominal: f64,
        resistance: f64,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::DcMotor {
                rpm_nominal,
                volt_nominal,
                resistance,
                speed: 0.0,
                angle: 0.0,
            },
        }
    }

    pub fn stepper(id: impl Into<String>, bipolar: bool, steps: i32, resistance: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Stepper {
                bipolar,
                steps,
                resistance,
                angle: 0.0,
            },
        }
    }

    pub fn servo(id: impl Into<String>, speed: f64, min_pulse: f64, max_pulse: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Servo {
                speed,
                min_pulse,
                max_pulse,
                pos: 90.0,
                target_pos: 90.0,
                pulse_start_ps: 0,
                sig_high: false,
            },
        }
    }

    pub fn sdcard(id: impl Into<String>, file: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::SdCard {
                file: file.into(),
                card_inserted: true,
            },
        }
    }

    pub fn esp01(id: impl Into<String>, baud_rate: u32, debug: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Esp01 { baud_rate, debug },
        }
    }

    pub fn tft_display(
        id: impl Into<String>,
        controller: impl Into<String>,
        width: u32,
        height: u32,
        scale: f64,
        bgr: bool,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::TftDisplay {
                controller: controller.into(),
                width,
                height,
                scale,
                bgr,
            },
        }
    }

    pub fn pcd8544(id: impl Into<String>, contrast: u8, bias: u8) -> Self {
        let id = id.into();
        Self {
            kind: Kind::Pcd8544(crate::digital::Pcd8544State::new(&id, contrast, bias)),
            id,
        }
    }

    pub fn sh1107(id: impl Into<String>, width: u32, height: u32) -> Self {
        let id = id.into();
        Self {
            kind: Kind::Sh1107(crate::digital::Sh1107State::new(
                &id,
                width as usize,
                height as usize,
                crate::digital::Sh1107State::DEFAULT_ADDRESS,
                true,
                100.0,
            )),
            id,
        }
    }

    pub fn ks0108(id: impl Into<String>, _width: u32, _height: u32) -> Self {
        let id = id.into();
        Self {
            kind: Kind::Ks0108(crate::digital::Ks0108State::new(&id, false)),
            id,
        }
    }

    pub fn pcf8833(id: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Pcf8833Display { width, height },
        }
    }

    pub fn aip31068(id: impl Into<String>, rows: u8, cols: u8) -> Self {
        let id = id.into();
        Self {
            kind: Kind::Aip31068(crate::digital::Aip31068State::new(
                &id,
                rows as usize,
                cols as usize,
                crate::digital::Aip31068State::DEFAULT_ADDRESS,
                100.0,
            )),
            id,
        }
    }

    pub fn gate(id: impl Into<String>, state: GateState) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Gate(state),
        }
    }

    pub fn flipflop(id: impl Into<String>, state: FlipFlopState) -> Self {
        Self {
            id: id.into(),
            kind: Kind::FlipFlop(state),
        }
    }

    pub fn latch(id: impl Into<String>, state: LatchState) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Latch(state),
        }
    }

    pub fn mcu_pin(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            kind: Kind::McuPin(McuPinState::new(&id)),
            id,
        }
    }

    pub fn mcu(id: impl Into<String>, device: cs_mcu::Device) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Mcu(McuComp::from_device(device)),
        }
    }

    pub fn mcu_item(id: impl Into<String>, spec: McuItemSpec) -> Self {
        Self {
            id: id.into(),
            kind: Kind::McuItem(spec),
        }
    }

    pub fn qemu(id: impl Into<String>, device: QemuComp) -> Self {
        Self {
            id: id.into(),
            kind: Kind::QemuDevice(device),
        }
    }

    pub fn script_cpu(id: impl Into<String>, cpu: ScriptCpu) -> Self {
        Self {
            id: id.into(),
            kind: Kind::ScriptCpu(Box::new(cpu)),
        }
    }

    pub fn test_unit(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            kind: Kind::TestUnit(TestUnitState::new(&id)),
            id,
        }
    }

    pub fn test_unit_with(id: impl Into<String>, state: TestUnitState) -> Self {
        Self {
            id: id.into(),
            kind: Kind::TestUnit(state),
        }
    }

    pub fn hd44780(id: impl Into<String>, rows: usize, cols: usize) -> Self {
        let id = id.into();
        Self {
            kind: Kind::Hd44780(crate::digital::Hd44780State::new(&id, rows, cols)),
            id,
        }
    }

    pub fn hd44780_with(id: impl Into<String>, state: crate::digital::Hd44780State) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Hd44780(state),
        }
    }

    pub fn ssd1306(
        id: impl Into<String>,
        width: usize,
        height: usize,
        control_code: u8,
        color: &str,
        rotate: bool,
        freq_khz: f64,
    ) -> Self {
        let id = id.into();
        Self {
            kind: Kind::Ssd1306(crate::digital::Ssd1306State::new(
                &id,
                width,
                height,
                control_code,
                color,
                rotate,
                freq_khz,
            )),
            id,
        }
    }

    pub fn ssd1306_with(id: impl Into<String>, state: crate::digital::Ssd1306State) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ssd1306(state),
        }
    }

    pub fn volt_reg(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: Kind::VoltReg {
                state: VoltRegState::new(),
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wave_gen(
        id: impl Into<String>,
        wave_type: impl Into<String>,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
        phase: f64,
        steps: i32,
        bipolar: bool,
        floating: bool,
    ) -> Self {
        Self::wave_gen_with_file(
            id, wave_type, freq_hz, amplitude, offset, duty, phase, steps, bipolar, floating, "",
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wave_gen_with_file(
        id: impl Into<String>,
        wave_type: impl Into<String>,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
        phase: f64,
        steps: i32,
        bipolar: bool,
        floating: bool,
        file: impl Into<String>,
    ) -> Self {
        Self::wave_gen_with_wav_data(
            id, wave_type, freq_hz, amplitude, offset, duty, phase, steps, bipolar, floating, file,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn wave_gen_with_wav_data(
        id: impl Into<String>,
        wave_type: impl Into<String>,
        freq_hz: f64,
        amplitude: f64,
        offset: f64,
        duty: f64,
        phase: f64,
        steps: i32,
        bipolar: bool,
        floating: bool,
        file: impl Into<String>,
        wav_data: Option<std::sync::Arc<Vec<f64>>>,
    ) -> Self {
        let file_str = file.into();
        let wav_data = wav_data.or_else(|| {
            if !file_str.is_empty() {
                crate::wav::WavData::from_file(&file_str)
                    .ok()
                    .map(|w| w.samples)
            } else {
                None
            }
        });
        Self {
            id: id.into(),
            kind: Kind::WaveGen {
                wave_type: wave_type.into(),
                freq_hz,
                amplitude,
                offset,
                duty,
                phase,
                steps: steps.max(10),
                bipolar,
                floating,
                file: file_str,
                wav_data,
                wav_index: 0,
                v_out: 0.0,
            },
        }
    }

    pub fn audio_out(id: impl Into<String>, impedance: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::AudioOut {
                source: crate::audio::Source::new(impedance, 100.0, 1000.0, false),
                last_v: 0.0,
            },
        }
    }

    pub fn set_pin_inverted(&mut self, pin_id: &str, inverted: bool) {
        match &mut self.kind {
            Kind::Gate(g) => {
                for p in &mut g.inputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                if g.output.id == pin_id {
                    g.output.set_inverted(inverted);
                }
            }
            Kind::FlipFlop(f) => {
                if f.d.id == pin_id {
                    f.d.set_inverted(inverted);
                }
                if f.j.id == pin_id {
                    f.j.set_inverted(inverted);
                }
                if f.k.id == pin_id {
                    f.k.set_inverted(inverted);
                }
                if f.t.id == pin_id {
                    f.t.set_inverted(inverted);
                }
                if f.clk.id == pin_id {
                    f.clk.set_inverted(inverted);
                }
                if f.rst.id == pin_id {
                    f.rst.set_inverted(inverted);
                }
                if f.set.id == pin_id {
                    f.set.set_inverted(inverted);
                }
                if f.q.id == pin_id {
                    f.q.set_inverted(inverted);
                }
                if f.qn.id == pin_id {
                    f.qn.set_inverted(inverted);
                }
            }
            Kind::Latch(l) => {
                for p in &mut l.inputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                for p in &mut l.outputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                if l.clk.id == pin_id {
                    l.clk.set_inverted(inverted);
                }
                if l.reset.id == pin_id {
                    l.reset.set_inverted(inverted);
                }
                if l.oe.id == pin_id {
                    l.oe.set_inverted(inverted);
                }
            }
            Kind::Mux(m) => {
                for p in &mut m.inputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                for p in &mut m.addr_pins {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                if let Some(oe) = &mut m.enable {
                    if oe.id == pin_id {
                        oe.set_inverted(inverted);
                    }
                }
                if m.output.id == pin_id {
                    m.output.set_inverted(inverted);
                }
                if m.out_inverted.id == pin_id {
                    m.out_inverted.set_inverted(inverted);
                }
            }
            Kind::Demux(d) => {
                if d.input.id == pin_id {
                    d.input.set_inverted(inverted);
                }
                for p in &mut d.addr_pins {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                if let Some(oe) = &mut d.enable {
                    if oe.id == pin_id {
                        oe.set_inverted(inverted);
                    }
                }
                for p in &mut d.outputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
            }
            Kind::Comparator { state } => {
                if state.output.id == pin_id {
                    state.output.set_inverted(inverted);
                }
            }
            Kind::BcdToDec(b) => {
                for p in &mut b.inputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                for p in &mut b.outputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
            }
            Kind::DecToBcd(d) => {
                for p in &mut d.inputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                for p in &mut d.outputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                if d.gs.id == pin_id {
                    d.gs.set_inverted(inverted);
                }
                if d.eo.id == pin_id {
                    d.eo.set_inverted(inverted);
                }
                if d.ei.id == pin_id {
                    d.ei.set_inverted(inverted);
                }
            }
            Kind::BcdTo7S(b) => {
                for p in &mut b.inputs {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                for p in &mut b.segments {
                    if p.id == pin_id {
                        p.set_inverted(inverted);
                    }
                }
                if b.lt.id == pin_id {
                    b.lt.set_inverted(inverted);
                }
                if b.rbi.id == pin_id {
                    b.rbi.set_inverted(inverted);
                }
                if b.bi_rbo.id == pin_id {
                    b.bi_rbo.set_inverted(inverted);
                }
            }
            _ => {}
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum WaveKind {
    Sine,
    Saw,
    Triangle,
    Square,
    Random,
    Wav,
    Other,
}

#[inline]
fn classify_wave_kind(s: &str) -> WaveKind {
    let s = s.trim();
    if s.eq_ignore_ascii_case("sine") {
        WaveKind::Sine
    } else if s.eq_ignore_ascii_case("saw") || s.eq_ignore_ascii_case("sawtooth") {
        WaveKind::Saw
    } else if s.eq_ignore_ascii_case("triangle") {
        WaveKind::Triangle
    } else if s.eq_ignore_ascii_case("square") {
        WaveKind::Square
    } else if s.eq_ignore_ascii_case("random") {
        WaveKind::Random
    } else if s.eq_ignore_ascii_case("wav") {
        WaveKind::Wav
    } else {
        WaveKind::Other
    }
}

impl Comp {
    /// Calculate normalized output [0.0, 1.0] for WaveGen at circuit time `circ_time_ps`.
    pub fn wavegen_calc_vout(&self, circ_time_ps: u64) -> f64 {
        let Kind::WaveGen {
            wave_type,
            freq_hz,
            duty,
            phase,
            wav_data,
            ..
        } = &self.kind
        else {
            return 0.0;
        };

        if *freq_hz <= 0.0 {
            return 0.0;
        }

        let ps_per_cycle = 1e12 / freq_hz;
        let phase_time_ps = ps_per_cycle * (phase / 360.0);
        let t_cycle = (circ_time_ps as f64 - phase_time_ps).rem_euclid(ps_per_cycle);
        // Duty is a 0..=1 fraction. Values > 1 are treated as percent so a
        // palette/legacy 50 (meaning 50%) does not clamp to 100% and collapse
        // triangle into saw / square into DC.
        let duty_frac = if *duty > 1.0 {
            (*duty / 100.0).clamp(0.0, 1.0)
        } else {
            duty.clamp(0.0, 1.0)
        };

        match classify_wave_kind(wave_type) {
            WaveKind::Sine => {
                let angle = t_cycle * (2.0 * std::f64::consts::PI) / ps_per_cycle;
                (angle.sin() / 2.0 + 0.5).clamp(0.0, 1.0)
            }
            WaveKind::Saw => (t_cycle / ps_per_cycle).clamp(0.0, 1.0),
            WaveKind::Triangle => {
                let half_w = ps_per_cycle * duty_frac;
                if half_w <= 0.0 {
                    (1.0 - t_cycle / ps_per_cycle).clamp(0.0, 1.0)
                } else if half_w >= ps_per_cycle {
                    (t_cycle / ps_per_cycle).clamp(0.0, 1.0)
                } else if t_cycle >= half_w {
                    (1.0 - (t_cycle - half_w) / (ps_per_cycle - half_w)).clamp(0.0, 1.0)
                } else {
                    (t_cycle / half_w).clamp(0.0, 1.0)
                }
            }
            WaveKind::Square => {
                let half_w = ps_per_cycle * duty_frac;
                if t_cycle < half_w { 1.0 } else { 0.0 }
            }
            WaveKind::Random => {
                // Pseudo-random noise steps matching C++ genRandom behavior
                let step_idx = (t_cycle / (ps_per_cycle / 3.0).max(1.0)) as u64;
                let cycle_idx = (circ_time_ps as f64 / ps_per_cycle) as u64;
                let mut h = (cycle_idx.wrapping_mul(0x9E3779B97F4A7C15)
                    ^ step_idx.wrapping_mul(0xBF58476D1CE4E5B9)) as u32;
                h ^= h >> 16;
                h = h.wrapping_mul(0x85ebca6b);
                h ^= h >> 13;
                (h as f64) / (u32::MAX as f64)
            }
            WaveKind::Wav => {
                if let Some(samples) = wav_data {
                    if !samples.is_empty() {
                        let sample_idx =
                            ((circ_time_ps as f64 / ps_per_cycle) as usize) % samples.len();
                        samples[sample_idx]
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
            WaveKind::Other => {
                let angle = t_cycle * (2.0 * std::f64::consts::PI) / ps_per_cycle;
                (angle.sin() / 2.0 + 0.5).clamp(0.0, 1.0)
            }
        }
    }

    /// Calculate delta picoseconds until next discrete event / demand sample for WaveGen.
    pub fn wavegen_next_event_ps(&self, circ_time_ps: u64) -> Option<u64> {
        let Kind::WaveGen {
            wave_type,
            freq_hz,
            duty,
            phase,
            ..
        } = &self.kind
        else {
            return None;
        };

        if *freq_hz <= 0.0 {
            return None;
        }

        let ps_per_cycle = (1e12 / freq_hz).round() as u64;
        if ps_per_cycle == 0 {
            return None;
        }
        let phase_time_ps =
            ((ps_per_cycle as f64) * (phase / 360.0)).rem_euclid(ps_per_cycle as f64) as u64;
        let t_cycle = (circ_time_ps.saturating_sub(phase_time_ps)) % ps_per_cycle;

        let duty_frac = if *duty > 1.0 {
            (*duty / 100.0).clamp(0.0, 1.0)
        } else {
            duty.clamp(0.0, 1.0)
        };

        match classify_wave_kind(wave_type) {
            WaveKind::Square => {
                let half_w = ((ps_per_cycle as f64) * duty_frac).round() as u64;
                if t_cycle < half_w {
                    Some(half_w.saturating_sub(t_cycle).max(1))
                } else {
                    Some(ps_per_cycle.saturating_sub(t_cycle).max(1))
                }
            }
            _ => {
                // Continuous waves: Sine, Triangle, Saw, Random, Wav
                // 40 steps per cycle (9 degrees resolution for sine)
                const SAMPLES_PER_CYCLE: u64 = 40;
                let dt = (ps_per_cycle / SAMPLES_PER_CYCLE).clamp(1, 1_000_000_000);
                Some(dt)
            }
        }
    }

    pub fn probe(id: impl Into<String>, threshold: f64, small: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Probe { threshold, small },
        }
    }

    pub fn voltmeter(id: impl Into<String>, rms: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Voltmeter { rms, last_out: 0.0 },
        }
    }

    pub fn ammeter(id: impl Into<String>, rms: bool) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Ammeter { rms, last_out: 0.0 },
        }
    }

    pub fn freq_meter(id: impl Into<String>, filter: f64) -> Self {
        Self {
            id: id.into(),
            kind: Kind::FreqMeter { filter },
        }
    }

    pub fn oscope(id: impl Into<String>, connect_gnd: bool) -> Self {
        Self::oscope_full(
            id,
            connect_gnd,
            10.0,
            [String::new(), String::new(), String::new(), String::new()],
        )
    }

    pub fn oscope_full(
        id: impl Into<String>,
        connect_gnd: bool,
        input_imped: f64,
        tunnels: [String; 4],
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Oscope {
                connect_gnd,
                input_imped,
                tunnels,
            },
        }
    }

    pub fn lanalizer(id: impl Into<String>, connect_gnd: bool) -> Self {
        Self::lanalizer_full(id, connect_gnd, 10.0, Default::default())
    }

    pub fn lanalizer_full(
        id: impl Into<String>,
        connect_gnd: bool,
        input_imped: f64,
        tunnels: [String; 8],
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::LAnalizer {
                connect_gnd,
                input_imped,
                tunnels,
            },
        }
    }

    pub fn tunnel(
        id: impl Into<String>,
        name: impl Into<String>,
        pin_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Tunnel {
                name: name.into(),
                pin_id: pin_id.into(),
            },
        }
    }

    pub fn subcircuit(
        id: impl Into<String>,
        device: impl Into<String>,
        logic_symbol: bool,
        package_name: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: Kind::Subcircuit {
                device: device.into(),
                logic_symbol,
                package_name,
            },
        }
    }
}
