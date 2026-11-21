//! A palette drop and `T::default()` must describe the same new part.
//! Variant entries (zener, flip-flop kind, display size) override only that axis.

use cs_engine::canvas::Scene;
use cs_engine::circ1::GraphicAttrs;
use cs_engine::components::{
    Adc, Aip31068, Ammeter, AnalogMux, AndGate, AudioOut, Battery, BcdTo7Segment, BcdToDec,
    BinCounter, Bjt, BufferGate, Bus, Capacitor, Clock, Comparator, Counter, Csource, CurrSource,
    DHT22, DS18B20, DS1307, DS1621, Dac, DcMotor, DecToBcd, Demux, Diac, Dial, Diode,
    DynamicMemory, ElCapacitor, Esp01, FixedVolt, FlipFlop, FreqMeter, FullAdder, Function, Ground,
    HalfAdder, Hd44780, Header, I2CRam, I2CToParallel, Inductor, Jfet, KY023, KY040, KeyPad,
    Ks0108, Lamp, Latch, Ldr, Led, LedBar, LedMatrix, Lm555, LogicAnalyzer, MagnitudeComp, Max72xx,
    Memory, Mosfet, Mux, Node, OpAmp, OrGate, Oscope, Part, Pcd8544, Pcf8833, Potentiometer, Probe,
    Push, Rail, Relay, Resistor, ResistorDip, RgbLed, Rtd, SR04, Scr, SdCard, SerialPort,
    SerialTerm, Servo, SevenSegment, SevenSegmentBCD, Sh1107, ShiftReg, Socket, Ssd1306, Stepper,
    Strain, SubPackage, Switch, SwitchDip, TestUnit, TftController, TftDisplay, Thermistor,
    TouchPad, Transformer, Triac, Tunnel, VarCapacitor, VarInductor, VarResistor, VoltReg,
    VoltSource, Voltmeter, WaveGen, Ws2812, XorGate,
};
use cs_engine::digital::FlipFlopKind;

fn props(part: &Part) -> String {
    part.write_item("ID", &GraphicAttrs::at(0.0, 0.0))
}

fn pins(part: &Part) -> Vec<String> {
    part.pin_geoms().into_iter().map(|p| p.suffix).collect()
}

fn check(
    label: &str,
    place: impl FnOnce(&mut Scene) -> String,
    expect: Part,
    out: &mut Vec<String>,
) {
    let mut scene = Scene::new();
    let id = place(&mut scene);
    let got = &scene.item_by_id(&id).unwrap().kind;
    let a = props(got);
    let b = props(&expect);
    if a != b {
        out.push(format!("{label}\npalette: {a}default: {b}"));
    }
    let pa = pins(got);
    let pb = pins(&expect);
    if pa != pb {
        out.push(format!("{label} pins\npalette: {pa:?}\ndefault: {pb:?}"));
    }
}

fn ff(kind: &str, use_rs: bool) -> Part {
    let mut f = FlipFlop::default();
    f.ff_kind = FlipFlopKind::from_str_name(kind);
    f.use_rs = use_rs;
    f.into()
}

fn tft(controller: &str, width: u32, height: u32) -> Part {
    let mut t = TftDisplay::default();
    t.controller = TftController::from_str_name(controller);
    t.width = width;
    t.height = height;
    t.into()
}

#[test]
fn palette_drop_matches_default() {
    let mut out = Vec::new();
    macro_rules! plain {
        ($label:expr, $place:path, $expect:expr) => {
            check($label, |s| $place(s, 0.0, 0.0), $expect, &mut out);
        };
    }

    plain!(
        "resistor",
        Scene::add_default_resistor,
        Resistor::default().into()
    );
    plain!(
        "battery",
        Scene::add_default_battery,
        Battery::default().into()
    );
    plain!("ground", Scene::add_ground, Ground::default().into());
    plain!(
        "fixed_volt",
        Scene::add_default_fixed_volt,
        FixedVolt::default().into()
    );
    plain!(
        "capacitor",
        Scene::add_default_capacitor,
        Capacitor::default().into()
    );
    plain!(
        "el_capacitor",
        Scene::add_default_el_capacitor,
        ElCapacitor::default().into()
    );
    plain!(
        "inductor",
        Scene::add_default_inductor,
        Inductor::default().into()
    );
    plain!(
        "switch",
        Scene::add_default_switch,
        Switch::default().into()
    );
    check(
        "diode",
        |s| s.add_diode(0.0, 0.0, false),
        Diode::default().into(),
        &mut out,
    );
    check(
        "zener",
        |s| s.add_diode(0.0, 0.0, true),
        Diode::zener_default().into(),
        &mut out,
    );
    plain!("led", Scene::add_led, Led::default().into());
    check(
        "bjt",
        |s| s.add_bjt(0.0, 0.0, false),
        Bjt::default().into(),
        &mut out,
    );
    check(
        "mosfet",
        |s| s.add_mosfet(0.0, 0.0, false, false),
        Mosfet::default().into(),
        &mut out,
    );
    plain!("opamp", Scene::add_opamp, OpAmp::default().into());
    plain!("jfet", Scene::add_jfet, Jfet::default().into());
    plain!(
        "comparator",
        Scene::add_comparator,
        Comparator::default().into()
    );
    plain!("volt_reg", Scene::add_volt_reg, VoltReg::default().into());
    plain!("probe", Scene::add_probe, Probe::default().into());
    plain!(
        "voltmeter",
        Scene::add_voltmeter,
        Voltmeter::default().into()
    );
    plain!("ammeter", Scene::add_ammeter, Ammeter::default().into());
    plain!(
        "freq_meter",
        Scene::add_freq_meter,
        FreqMeter::default().into()
    );
    plain!("oscope", Scene::add_oscope, Oscope::default().into());
    plain!(
        "lanalizer",
        Scene::add_lanalizer,
        LogicAnalyzer::default().into()
    );
    plain!("clock", Scene::add_clock, Clock::default().into());
    plain!("rail", Scene::add_rail, Rail::default().into());
    plain!("wave_gen", Scene::add_wave_gen, WaveGen::default().into());
    plain!(
        "volt_source",
        Scene::add_volt_source,
        VoltSource::default().into()
    );
    plain!(
        "curr_source",
        Scene::add_curr_source,
        CurrSource::default().into()
    );
    plain!("csource", Scene::add_csource, Csource::default().into());
    plain!("push", Scene::add_push, Push::default().into());
    plain!(
        "switch_dip",
        Scene::add_switch_dip,
        SwitchDip::default().into()
    );
    plain!("relay", Scene::add_relay, Relay::default().into());
    plain!("keypad", Scene::add_keypad, KeyPad::default().into());
    plain!("touchpad", Scene::add_touchpad, TouchPad::default().into());
    plain!("ky023", Scene::add_ky023, KY023::default().into());
    plain!("ky040", Scene::add_ky040, KY040::default().into());
    plain!("sr04", Scene::add_sr04, SR04::default().into());
    plain!("dht22", Scene::add_dht22, DHT22::default().into());
    plain!("ds18b20", Scene::add_ds18b20, DS18B20::default().into());
    plain!("ds1621", Scene::add_ds1621, DS1621::default().into());
    plain!("ds1307", Scene::add_ds1307, DS1307::default().into());
    plain!("dcmotor", Scene::add_dcmotor, DcMotor::default().into());
    plain!("stepper", Scene::add_stepper, Stepper::default().into());
    plain!("servo", Scene::add_servo, Servo::default().into());
    plain!("sdcard", Scene::add_sdcard, SdCard::default().into());
    plain!("esp01", Scene::add_esp01, Esp01::default().into());
    check(
        "tft_ili9341",
        |s| s.add_tft_display(0.0, 0.0, "ILI9341", 240, 320),
        TftDisplay::default().into(),
        &mut out,
    );
    check(
        "tft_generic",
        |s| s.add_tft_display(0.0, 0.0, "ILI9341", 240, 320),
        TftDisplay::default().into(),
        &mut out,
    );
    check(
        "tft_st7789",
        |s| s.add_tft_display(0.0, 0.0, "ST7789", 240, 320),
        tft("ST7789", 240, 320),
        &mut out,
    );
    check(
        "tft_st7735",
        |s| s.add_tft_display(0.0, 0.0, "ST7735", 132, 162),
        tft("ST7735", 132, 162),
        &mut out,
    );
    check(
        "tft_gc9a01a",
        |s| s.add_tft_display(0.0, 0.0, "GC9A01A", 240, 240),
        tft("GC9A01A", 240, 240),
        &mut out,
    );
    plain!("pcd8544", Scene::add_pcd8544, Pcd8544::default().into());
    check(
        "sh1107",
        |s| s.add_sh1107(0.0, 0.0, 128, 128),
        Sh1107::default().into(),
        &mut out,
    );
    plain!("ks0108", Scene::add_ks0108, Ks0108::default().into());
    plain!("pcf8833", Scene::add_pcf8833, Pcf8833::default().into());
    check(
        "aip31068",
        |s| s.add_aip31068(0.0, 0.0, 2, 16),
        Aip31068::default().into(),
        &mut out,
    );
    plain!(
        "potentiometer",
        Scene::add_potentiometer,
        Potentiometer::default().into()
    );
    plain!(
        "var_resistor",
        Scene::add_var_resistor,
        VarResistor::default().into()
    );
    plain!(
        "resistor_dip",
        Scene::add_resistor_dip,
        ResistorDip::default().into()
    );
    plain!("ldr", Scene::add_ldr, Ldr::default().into());
    plain!(
        "thermistor",
        Scene::add_thermistor,
        Thermistor::default().into()
    );
    plain!("rtd", Scene::add_rtd, Rtd::default().into());
    plain!("strain", Scene::add_strain, Strain::default().into());
    plain!(
        "var_capacitor",
        Scene::add_var_capacitor,
        VarCapacitor::default().into()
    );
    plain!(
        "var_inductor",
        Scene::add_var_inductor,
        VarInductor::default().into()
    );
    plain!(
        "transformer",
        Scene::add_transformer,
        Transformer::default().into()
    );
    plain!("scr", Scene::add_scr, Scr::default().into());
    plain!("diac", Scene::add_diac, Diac::default().into());
    plain!("triac", Scene::add_triac, Triac::default().into());
    plain!(
        "analog_mux",
        Scene::add_analog_mux,
        AnalogMux::default().into()
    );
    plain!("rgb_led", Scene::add_rgb_led, RgbLed::default().into());
    plain!("led_bar", Scene::add_led_bar, LedBar::default().into());
    plain!(
        "seven_segment",
        Scene::add_seven_segment,
        SevenSegment::default().into()
    );
    plain!(
        "led_matrix",
        Scene::add_led_matrix,
        LedMatrix::default().into()
    );
    plain!("max72xx", Scene::add_max72xx, Max72xx::default().into());
    plain!("ws2812", Scene::add_ws2812, Ws2812::default().into());
    plain!("hd44780", Scene::add_hd44780, Hd44780::default().into());
    plain!("ssd1306", Scene::add_ssd1306, Ssd1306::default().into());
    plain!(
        "audio_out",
        Scene::add_audio_out,
        AudioOut::default().into()
    );
    plain!("lamp", Scene::add_lamp, Lamp::default().into());
    plain!("tunnel", Scene::add_tunnel, Tunnel::default().into());
    plain!("bus", Scene::add_bus, Bus::default().into());
    plain!("socket", Scene::add_socket, Socket::default().into());
    plain!("header", Scene::add_header, Header::default().into());
    plain!(
        "serial_port",
        Scene::add_serial_port,
        SerialPort::default().into()
    );
    plain!(
        "serial_term",
        Scene::add_serial_term,
        SerialTerm::default().into()
    );
    plain!("dial", Scene::add_dial, Dial::default().into());
    check(
        "subpackage",
        |s| s.add_subpackage(0.0, 0.0),
        {
            let mut p = SubPackage::default();
            p.package.name = "SubPackage-2".into();
            p.into()
        },
        &mut out,
    );
    plain!("and", Scene::add_and_gate, AndGate::default().into());
    plain!("or", Scene::add_or_gate, OrGate::default().into());
    plain!("xor", Scene::add_xor_gate, XorGate::default().into());
    plain!("buffer", Scene::add_buffer, BufferGate::default().into());
    plain!(
        "flipflop_d",
        Scene::add_flipflop_d,
        FlipFlop::default().into()
    );
    check(
        "flipflop_jk",
        |s| s.add_flipflop_jk(0.0, 0.0),
        ff("JK", true),
        &mut out,
    );
    check(
        "flipflop_rs",
        |s| s.add_flipflop_rs(0.0, 0.0),
        ff("RS", false),
        &mut out,
    );
    check(
        "flipflop_t",
        |s| s.add_flipflop_t(0.0, 0.0),
        ff("T", true),
        &mut out,
    );
    plain!("latch", Scene::add_latch_d, Latch::default().into());
    plain!(
        "test_unit",
        Scene::add_test_unit,
        TestUnit::default().into()
    );
    plain!("mux", Scene::add_mux, Mux::default().into());
    plain!("demux", Scene::add_demux, Demux::default().into());
    plain!(
        "bcd_to_dec",
        Scene::add_bcd_to_dec,
        BcdToDec::default().into()
    );
    plain!(
        "dec_to_bcd",
        Scene::add_dec_to_bcd,
        DecToBcd::default().into()
    );
    plain!(
        "bcd_to_7s",
        Scene::add_bcd_to_7s,
        BcdTo7Segment::default().into()
    );
    plain!(
        "seven_segment_bcd",
        Scene::add_seven_segment_bcd,
        SevenSegmentBCD::default().into()
    );
    plain!(
        "i2c_to_parallel",
        Scene::add_i2c_to_parallel,
        I2CToParallel::default().into()
    );
    plain!("adc", Scene::add_adc, Adc::default().into());
    plain!("dac", Scene::add_dac, Dac::default().into());
    plain!("counter", Scene::add_counter, Counter::default().into());
    plain!(
        "bin_counter",
        Scene::add_bin_counter,
        BinCounter::default().into()
    );
    plain!(
        "full_adder",
        Scene::add_full_adder,
        FullAdder::default().into()
    );
    plain!(
        "half_adder",
        Scene::add_half_adder,
        HalfAdder::default().into()
    );
    plain!(
        "magnitude_comp",
        Scene::add_magnitude_comp,
        MagnitudeComp::default().into()
    );
    plain!(
        "shift_reg",
        Scene::add_shift_reg,
        ShiftReg::default().into()
    );
    plain!("function", Scene::add_function, Function::default().into());
    plain!("memory", Scene::add_memory, Memory::default().into());
    plain!(
        "dynamic_memory",
        Scene::add_dynamic_memory,
        DynamicMemory::default().into()
    );
    plain!("i2c_ram", Scene::add_i2c_ram, I2CRam::default().into());
    plain!("lm555", Scene::add_lm555, Lm555::default().into());
    plain!("node", Scene::add_node, Node::default().into());
    check(
        "shape_rect",
        |s| s.add_shape("Rectangle", 0.0, 0.0),
        cs_engine::components::Shape::default().into(),
        &mut out,
    );

    assert!(out.is_empty(), "\n{}", out.join("\n"));
}
