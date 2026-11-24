//! SimulIDE `.sim1` / `.sim2` compatibility resolver.
//!
//! Converts legacy SimulIDE circuit documents into native [`ParsedCirc1`] (or canonical
//! `.circ1` XML text). All legacy itemtype aliases, split-type discriminators, legacy
//! attribute names/default units, and legacy pin-id remappings live exclusively here.

use std::collections::{BTreeMap, HashMap};

use crate::circ1::{
    self, FORMAT_VERSION, GraphicAttrs, ParsedCirc1, ParsedCirc1Item, ParsedConnector,
};
use crate::components::*;
use crate::elements::pins::{PIN_LEFT, PIN_POT_W, PIN_RIGHT};
use crate::settings::CircSettings;
use crate::subcircuit;
use crate::xml_util::{
    parse_bool, parse_flip, parse_point, parse_point_list, parse_xml_props, prop,
};
use crate::{Error, Result};

/// Supported circuit file extensions in load-priority order (native `.circ1` first,
/// followed by legacy SimulIDE formats imported via this compatibility resolver).
pub const CIRCUIT_EXTENSIONS: &[&str] = &["circ1", "sim2", "sim1"];

/// Legacy backup draft snapshot filename written by older versions of the app.
pub const LEGACY_BACKUP_SNAPSHOT_FILE: &str = "snapshot.sim2";

/// Returns `true` if `ext` (without leading dot) is a legacy SimulIDE circuit extension.
pub fn is_legacy_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("sim1")
        || ext.eq_ignore_ascii_case("sim2")
        || ext.eq_ignore_ascii_case("sim")
}

/// If `path` ends with a legacy SimulIDE circuit extension (`.sim1` or `.sim2`),
/// returns the prefix before the extension dot.
pub fn strip_legacy_extension(path: &str) -> Option<&str> {
    if path.len() >= 5 {
        let split = path.len() - 5;
        if path.is_char_boundary(split) {
            let suffix = &path[split..];
            if suffix.eq_ignore_ascii_case(".sim1") || suffix.eq_ignore_ascii_case(".sim2") {
                return Some(&path[..split]);
            }
        }
    }
    None
}

/// Returns `true` if `ext` (without leading dot) is a native `.circ1` or legacy SimulIDE circuit extension.
pub fn is_circuit_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("circ1")
        || ext.eq_ignore_ascii_case("circ")
        || is_legacy_extension(ext)
}

/// Priority rank for choosing among sibling circuit files with the same stem (lower wins).
pub fn extension_priority(ext: &str) -> u8 {
    if ext.eq_ignore_ascii_case("circ1") {
        0
    } else if ext.eq_ignore_ascii_case("sim2") {
        1
    } else if ext.eq_ignore_ascii_case("sim1") {
        2
    } else {
        3
    }
}

/// Legacy XML aliases mapping old SimulIDE item names / casing to canonical `circ1` TYPE_IDs.
pub const LEGACY_ITEMTYPE_ALIASES: &[(&str, &str)] = &[
    ("opAmp", OpAmp::TYPE_ID),
    ("opamp", OpAmp::TYPE_ID),
    ("Voltimeter", Voltmeter::TYPE_ID),
    ("Amperimeter", Ammeter::TYPE_ID),
    ("LAnalizer", LogicAnalyzer::TYPE_ID),
    ("Fixed Voltage", FixedVolt::TYPE_ID),
    ("And Gate", AndGate::TYPE_ID),
    ("Or Gate", OrGate::TYPE_ID),
    ("Xor Gate", XorGate::TYPE_ID),
    ("Not Gate", NotGate::TYPE_ID),
    ("elCapacitor", ElCapacitor::TYPE_ID),
    ("Frequencimeter", FreqMeter::TYPE_ID),
    ("Voltage Source", VoltSource::TYPE_ID),
    ("Current Source", CurrSource::TYPE_ID),
    ("diode", Diode::TYPE_ID),
    ("Zener", Diode::TYPE_ID),
    ("zener", Diode::TYPE_ID),
    ("Zener Diode", Diode::TYPE_ID),
    ("ZenerDiode", Diode::TYPE_ID),
    ("led", Led::TYPE_ID),
    ("LED", Led::TYPE_ID),
    ("BJT", Bjt::TYPE_ID),
    ("bjt", Bjt::TYPE_ID),
    ("mosfet", Mosfet::TYPE_ID),
    ("MOSFET", Mosfet::TYPE_ID),
    ("jfet", Jfet::TYPE_ID),
    ("JFET", Jfet::TYPE_ID),
    ("comparator", Comparator::TYPE_ID),
    ("voltreg", VoltReg::TYPE_ID),
    ("VoltRegulator", VoltReg::TYPE_ID),
    ("Volt. Regulator", VoltReg::TYPE_ID),
    ("KY-023", KY023::TYPE_ID),
    ("ky023", KY023::TYPE_ID),
    ("Ky023", KY023::TYPE_ID),
    ("KY-040", KY040::TYPE_ID),
    ("ky040", KY040::TYPE_ID),
    ("Ky040", KY040::TYPE_ID),
    ("Oscilloscope", Oscope::TYPE_ID),
    ("Buffer Gate", BufferGate::TYPE_ID),
    ("Buffer", BufferGate::TYPE_ID),
    ("BcdTo7S", BcdTo7Segment::TYPE_ID),
    ("LatchD", Latch::TYPE_ID),
    ("FlipFlopD", FlipFlop::TYPE_ID),
    ("FlipFlopJK", FlipFlop::TYPE_ID),
    ("FlipFlop RS", FlipFlop::TYPE_ID),
    ("FlipFlopRS", FlipFlop::TYPE_ID),
    ("FlipFlop T", FlipFlop::TYPE_ID),
    ("FlipFlopT", FlipFlop::TYPE_ID),
    ("potentiometer", Potentiometer::TYPE_ID),
    ("var_resistor", VarResistor::TYPE_ID),
    ("var_capacitor", VarCapacitor::TYPE_ID),
    ("Var_Capacitor", VarCapacitor::TYPE_ID),
    ("var_inductor", VarInductor::TYPE_ID),
    ("Var_Inductor", VarInductor::TYPE_ID),
    ("Analog_Mux", AnalogMux::TYPE_ID),
    ("touchpad", TouchPad::TYPE_ID),
    ("TouchPadR", TouchPad::TYPE_ID),
    ("HC-SR04", SR04::TYPE_ID),
    ("sr04", SR04::TYPE_ID),
    ("Sr04", SR04::TYPE_ID),
    ("dht22", DHT22::TYPE_ID),
    ("Dht22", DHT22::TYPE_ID),
    ("DHT11", DHT22::TYPE_ID),
    ("dht11", DHT22::TYPE_ID),
    ("Dht11", DHT22::TYPE_ID),
    ("ds18b20", DS18B20::TYPE_ID),
    ("Ds18b20", DS18B20::TYPE_ID),
    ("ds1621", DS1621::TYPE_ID),
    ("Ds1621", DS1621::TYPE_ID),
    ("ds1307", DS1307::TYPE_ID),
    ("Ds1307", DS1307::TYPE_ID),
    ("dcmotor", DcMotor::TYPE_ID),
    ("DC Motor", DcMotor::TYPE_ID),
    ("stepper", Stepper::TYPE_ID),
    ("Stepper Motor", Stepper::TYPE_ID),
    ("servo", Servo::TYPE_ID),
    ("Servo Motor", Servo::TYPE_ID),
    ("sdcard", SdCard::TYPE_ID),
    ("SDCard", SdCard::TYPE_ID),
    ("ESP-01", Esp01::TYPE_ID),
    ("esp01", Esp01::TYPE_ID),
    ("ESP01", Esp01::TYPE_ID),
    ("TFTDisplay", TftDisplay::TYPE_ID),
    ("tft_display", TftDisplay::TYPE_ID),
    ("TFT Display", TftDisplay::TYPE_ID),
    ("ILI9341", TftDisplay::TYPE_ID),
    ("ST7789", TftDisplay::TYPE_ID),
    ("ST7735", TftDisplay::TYPE_ID),
    ("GC9A01A", TftDisplay::TYPE_ID),
    ("PCD8544", Pcd8544::TYPE_ID),
    ("pcd8544", Pcd8544::TYPE_ID),
    ("Nokia5110", Pcd8544::TYPE_ID),
    ("SH1107", Sh1107::TYPE_ID),
    ("sh1107", Sh1107::TYPE_ID),
    ("KS0108", Ks0108::TYPE_ID),
    ("ks0108", Ks0108::TYPE_ID),
    ("PCF8833", Pcf8833::TYPE_ID),
    ("pcf8833", Pcf8833::TYPE_ID),
    ("AIP31068", Aip31068::TYPE_ID),
    ("aip31068", Aip31068::TYPE_ID),
    ("HD44780", Hd44780::TYPE_ID),
    ("hd44780", Hd44780::TYPE_ID),
    ("SSD1306", Ssd1306::TYPE_ID),
    ("ssd1306", Ssd1306::TYPE_ID),
    ("7Segment", SevenSegment::TYPE_ID),
    ("seven_segment", SevenSegment::TYPE_ID),
    ("Seven Segment", SevenSegment::TYPE_ID),
    ("7 Segment", SevenSegment::TYPE_ID),
    ("SevenSegmentBcd", SevenSegmentBCD::TYPE_ID),
    ("7SegmentBCD", SevenSegmentBCD::TYPE_ID),
    ("7Segment BCD", SevenSegmentBCD::TYPE_ID),
    ("7 Seg BCD", SevenSegmentBCD::TYPE_ID),
    ("SevenSegment BCD", SevenSegmentBCD::TYPE_ID),
    ("seven_segment_bcd", SevenSegmentBCD::TYPE_ID),
    ("Led Matrix", LedMatrix::TYPE_ID),
    ("led_matrix", LedMatrix::TYPE_ID),
    ("LED Matrix", LedMatrix::TYPE_ID),
    ("MAX7219", Max72xx::TYPE_ID),
    ("max7219", Max72xx::TYPE_ID),
    ("MAX72XX", Max72xx::TYPE_ID),
    ("max72xx", Max72xx::TYPE_ID),
    ("Max7219", Max72xx::TYPE_ID),
    ("Led Bar", LedBar::TYPE_ID),
    ("LED Bar", LedBar::TYPE_ID),
    ("led_bar", LedBar::TYPE_ID),
    ("RGB LED", RgbLed::TYPE_ID),
    ("RGBLed", RgbLed::TYPE_ID),
    ("rgb_led", RgbLed::TYPE_ID),
    ("rgbLed", RgbLed::TYPE_ID),
    ("RGB Led", RgbLed::TYPE_ID),
    ("WS2812", Ws2812::TYPE_ID),
    ("ws2812", Ws2812::TYPE_ID),
    ("WS2812B", Ws2812::TYPE_ID),
    ("ws2812b", Ws2812::TYPE_ID),
    ("74xx161", Counter::TYPE_ID),
    ("74xx163", Counter::TYPE_ID),
    ("BinaryCounter", Counter::TYPE_ID),
    ("binary_counter", Counter::TYPE_ID),
    ("counter", Counter::TYPE_ID),
    ("bin_counter", BinCounter::TYPE_ID),
    ("bincounter", BinCounter::TYPE_ID),
    ("74xx90", BinCounter::TYPE_ID),
    ("shift_reg", ShiftReg::TYPE_ID),
    ("shiftreg", ShiftReg::TYPE_ID),
    ("ShiftRegister", ShiftReg::TYPE_ID),
    ("74xx164", ShiftReg::TYPE_ID),
    ("74xx165", ShiftReg::TYPE_ID),
    ("74xx595", ShiftReg::TYPE_ID),
    ("magnitude_comp", MagnitudeComp::TYPE_ID),
    ("magnitudecomp", MagnitudeComp::TYPE_ID),
    ("MagnitudeComparator", MagnitudeComp::TYPE_ID),
    ("74xx85", MagnitudeComp::TYPE_ID),
    ("ADC", Adc::TYPE_ID),
    ("adc", Adc::TYPE_ID),
    ("DAC", Dac::TYPE_ID),
    ("dac", Dac::TYPE_ID),
    ("i2c_to_parallel", I2CToParallel::TYPE_ID),
    ("I2C_To_Parallel", I2CToParallel::TYPE_ID),
    ("PCF8574", I2CToParallel::TYPE_ID),
    ("pcf8574", I2CToParallel::TYPE_ID),
    ("i2ctoparallel", I2CToParallel::TYPE_ID),
    ("LM555", Lm555::TYPE_ID),
    ("555", Lm555::TYPE_ID),
    ("lm555", Lm555::TYPE_ID),
    ("NE555", Lm555::TYPE_ID),
    ("ne555", Lm555::TYPE_ID),
    ("RAM", Memory::TYPE_ID),
    ("ROM", Memory::TYPE_ID),
    ("ram", Memory::TYPE_ID),
    ("rom", Memory::TYPE_ID),
    ("memory", Memory::TYPE_ID),
    ("dynamic_memory", DynamicMemory::TYPE_ID),
    ("DRAM", DynamicMemory::TYPE_ID),
    ("dram", DynamicMemory::TYPE_ID),
    ("dynamicmemory", DynamicMemory::TYPE_ID),
    ("I2C_RAM", I2CRam::TYPE_ID),
    ("i2c_ram", I2CRam::TYPE_ID),
    ("i2cram", I2CRam::TYPE_ID),
    ("I2CRAM", I2CRam::TYPE_ID),
    ("24C04", I2CRam::TYPE_ID),
    ("24c04", I2CRam::TYPE_ID),
    ("function", Function::TYPE_ID),
    ("Func", Function::TYPE_ID),
    ("func", Function::TYPE_ID),
    ("transformer", Transformer::TYPE_ID),
    ("TRANSFORMER", Transformer::TYPE_ID),
    ("SCR", Scr::TYPE_ID),
    ("scr", Scr::TYPE_ID),
    ("DIAC", Diac::TYPE_ID),
    ("diac", Diac::TYPE_ID),
    ("TRIAC", Triac::TYPE_ID),
    ("triac", Triac::TYPE_ID),
    ("csource", Csource::TYPE_ID),
    ("CSOURCE", Csource::TYPE_ID),
    ("CSource", Csource::TYPE_ID),
    ("Controlled Source", Csource::TYPE_ID),
    ("controlled_source", Csource::TYPE_ID),
    ("resistor_dip", ResistorDip::TYPE_ID),
    ("Resistor_Dip", ResistorDip::TYPE_ID),
    ("resistordip", ResistorDip::TYPE_ID),
    ("RESISTOR_DIP", ResistorDip::TYPE_ID),
    ("Resistor DIP", ResistorDip::TYPE_ID),
    ("LDR", Ldr::TYPE_ID),
    ("ldr", Ldr::TYPE_ID),
    ("THERMISTOR", Thermistor::TYPE_ID),
    ("thermistor", Thermistor::TYPE_ID),
    ("RTD", Rtd::TYPE_ID),
    ("rtd", Rtd::TYPE_ID),
    ("STRAIN", Strain::TYPE_ID),
    ("strain", Strain::TYPE_ID),
    ("Audio_Out", AudioOut::TYPE_ID),
    ("Audio Out", AudioOut::TYPE_ID),
    ("audio_out", AudioOut::TYPE_ID),
    ("audioout", AudioOut::TYPE_ID),
    ("Buzzer", AudioOut::TYPE_ID),
    ("buzzer", AudioOut::TYPE_ID),
    ("Speaker", AudioOut::TYPE_ID),
    ("speaker", AudioOut::TYPE_ID),
    ("LAMP", Lamp::TYPE_ID),
    ("lamp", Lamp::TYPE_ID),
    ("oscope", Oscope::TYPE_ID),
    ("OSCOPE", Oscope::TYPE_ID),
    ("wavegen", WaveGen::TYPE_ID),
    ("Wave_Gen", WaveGen::TYPE_ID),
    ("Wave Generator", WaveGen::TYPE_ID),
    ("wave_gen", WaveGen::TYPE_ID),
    ("WAVEGEN", WaveGen::TYPE_ID),
    ("Wave_Generator", WaveGen::TYPE_ID),
    ("WaveGenerator", WaveGen::TYPE_ID),
    ("tunnel", Tunnel::TYPE_ID),
    ("TUNNEL", Tunnel::TYPE_ID),
    ("bus", Bus::TYPE_ID),
    ("BUS", Bus::TYPE_ID),
    ("socket", Socket::TYPE_ID),
    ("SOCKET", Socket::TYPE_ID),
    ("header", Header::TYPE_ID),
    ("HEADER", Header::TYPE_ID),
    ("PinHeader", Header::TYPE_ID),
    ("pin_header", Header::TYPE_ID),
    ("Pin_Header", Header::TYPE_ID),
    ("pinheader", Header::TYPE_ID),
    ("PIN_HEADER", Header::TYPE_ID),
    ("serial_port", SerialPort::TYPE_ID),
    ("Serial_Port", SerialPort::TYPE_ID),
    ("serialport", SerialPort::TYPE_ID),
    ("SERIAL_PORT", SerialPort::TYPE_ID),
    ("SERIALPORT", SerialPort::TYPE_ID),
    ("Serial Port", SerialPort::TYPE_ID),
    ("serial_term", SerialTerm::TYPE_ID),
    ("Serial_Term", SerialTerm::TYPE_ID),
    ("serialterm", SerialTerm::TYPE_ID),
    ("SERIAL_TERM", SerialTerm::TYPE_ID),
    ("SERIALTERM", SerialTerm::TYPE_ID),
    ("Serial Term", SerialTerm::TYPE_ID),
    ("Terminal", SerialTerm::TYPE_ID),
    ("terminal", SerialTerm::TYPE_ID),
    ("TERMINAL", SerialTerm::TYPE_ID),
    ("subcircuit", Subcircuit::TYPE_ID),
    ("SUBCIRCUIT", Subcircuit::TYPE_ID),
    ("SubCircuit", Subcircuit::TYPE_ID),
    ("Sub_Circuit", Subcircuit::TYPE_ID),
    ("sub_circuit", Subcircuit::TYPE_ID),
    ("subpackage", SubPackage::TYPE_ID),
    ("SUBPACKAGE", SubPackage::TYPE_ID),
    ("Sub_Package", SubPackage::TYPE_ID),
    ("sub_package", SubPackage::TYPE_ID),
    ("Subcircuit Package", SubPackage::TYPE_ID),
    ("subcircuit_package", SubPackage::TYPE_ID),
    ("Package", SubPackage::TYPE_ID),
    ("package", SubPackage::TYPE_ID),
    ("PACKAGE", SubPackage::TYPE_ID),
    ("dial", Dial::TYPE_ID),
    ("DIAL", Dial::TYPE_ID),
    ("shape", Shape::TYPE_ID),
    ("SHAPE", Shape::TYPE_ID),
    ("Rectangle", Shape::TYPE_ID),
    ("rectangle", Shape::TYPE_ID),
    ("RECTANGLE", Shape::TYPE_ID),
    ("Ellipse", Shape::TYPE_ID),
    ("ellipse", Shape::TYPE_ID),
    ("ELLIPSE", Shape::TYPE_ID),
    ("Line", Shape::TYPE_ID),
    ("line", Shape::TYPE_ID),
    ("LINE", Shape::TYPE_ID),
    ("Text", Shape::TYPE_ID),
    ("text", Shape::TYPE_ID),
    ("TEXT", Shape::TYPE_ID),
    ("TextComponent", Shape::TYPE_ID),
    ("Image", Shape::TYPE_ID),
    ("image", Shape::TYPE_ID),
    ("IMAGE", Shape::TYPE_ID),
    ("mcu", Mcu::TYPE_ID),
    ("MCU", Mcu::TYPE_ID),
    ("Micro", Mcu::TYPE_ID),
    ("micro", Mcu::TYPE_ID),
    ("MICRO", Mcu::TYPE_ID),
    ("Microcontroller", Mcu::TYPE_ID),
    ("microcontroller", Mcu::TYPE_ID),
    ("MICROCONTROLLER", Mcu::TYPE_ID),
    ("qemu", QemuDevice::TYPE_ID),
    ("QEMU", QemuDevice::TYPE_ID),
    ("qemu_device", QemuDevice::TYPE_ID),
    ("Qemu_Device", QemuDevice::TYPE_ID),
    ("qemudevice", QemuDevice::TYPE_ID),
    ("QEMUDEVICE", QemuDevice::TYPE_ID),
    ("Qemu", QemuDevice::TYPE_ID),
];

pub fn resolve_legacy_itemtype(itemtype: &str) -> Option<&'static str> {
    LEGACY_ITEMTYPE_ALIASES
        .iter()
        .find(|(alias, _)| *alias == itemtype)
        .map(|(_, canonical)| *canonical)
}

/// Discriminator attributes implied by split legacy SimulIDE itemtypes when collapsed onto
/// a single canonical `circ1` `Part` variant.
fn legacy_itemtype_discriminators(
    raw_itemtype: &str,
) -> (&'static str, &'static [(&'static str, &'static str)]) {
    match raw_itemtype {
        "FlipFlopD" => (FlipFlop::TYPE_ID, &[("Kind", "D")]),
        "FlipFlopJK" => (FlipFlop::TYPE_ID, &[("Kind", "JK")]),
        "FlipFlop RS" | "FlipFlopRS" => (FlipFlop::TYPE_ID, &[("Kind", "RS")]),
        "FlipFlop T" | "FlipFlopT" => (FlipFlop::TYPE_ID, &[("Kind", "T")]),
        "Zener" | "zener" | "Zener Diode" | "ZenerDiode" => (Diode::TYPE_ID, &[("Zener", "true")]),
        "DHT11" | "dht11" | "Dht11" => (DHT22::TYPE_ID, &[("Model", "DHT11")]),
        "ROM" | "rom" => (Memory::TYPE_ID, &[("IsRom", "true")]),
        "ILI9341" => (TftDisplay::TYPE_ID, &[("Controller", "ILI9341")]),
        "ST7789" => (TftDisplay::TYPE_ID, &[("Controller", "ST7789")]),
        "ST7735" => (TftDisplay::TYPE_ID, &[("Controller", "ST7735")]),
        "GC9A01A" => (TftDisplay::TYPE_ID, &[("Controller", "GC9A01A")]),
        "Buzzer" | "buzzer" => (AudioOut::TYPE_ID, &[("Buzzer", "true")]),
        "Speaker" | "speaker" => (AudioOut::TYPE_ID, &[("Buzzer", "false")]),
        "Rectangle" | "rectangle" | "RECTANGLE" => ("Rectangle", &[]),
        "Ellipse" | "ellipse" | "ELLIPSE" => ("Ellipse", &[]),
        "Line" | "line" | "LINE" => ("Line", &[]),
        "Text" | "text" | "TEXT" | "TextComponent" => ("Text", &[]),
        "Image" | "image" | "IMAGE" => ("Image", &[]),
        other => (resolve_legacy_itemtype(other).unwrap_or(""), &[]),
    }
}

fn strip_legacy_count_unit(v: &str) -> String {
    let trimmed = v.trim();
    let Some((num, unit)) = trimmed.split_once(char::is_whitespace) else {
        return trimmed.to_string();
    };
    let num = num.trim();
    let unit = unit.trim();
    if num.parse::<f64>().is_ok()
        && (unit.starts_with('_')
            || matches!(
                unit.to_ascii_lowercase().as_str(),
                "bits"
                    | "bit"
                    | "pixels"
                    | "pixel"
                    | "px"
                    | "leds"
                    | "led"
                    | "poles"
                    | "pole"
                    | "cells"
                    | "cell"
                    | "inputs"
                    | "rpm"
                    | "steps"
                    | "rows"
                    | "cols"
                    | "digits"
            ))
    {
        num.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_legacy_freq_unit(v: &str) -> String {
    let trimmed = v.trim();
    if let Some((num, unit)) = trimmed.split_once(char::is_whitespace) {
        let num = num.trim();
        let unit = unit.trim();
        if let Some(rest) = unit.strip_prefix('_') {
            if num.parse::<f64>().is_ok() {
                let canon_unit = match rest.to_ascii_lowercase().as_str() {
                    "hz" => "Hz",
                    "khz" => "kHz",
                    "mhz" => "MHz",
                    "ghz" => "GHz",
                    _ => rest,
                };
                return format!("{num} {canon_unit}");
            }
        }
    }
    trimmed.to_string()
}

fn infer_legacy_type_from_comp_id(comp_id: &str) -> Option<&'static str> {
    // Strip optional subcircuit nesting prefix (`1@Mux-18` -> `Mux-18`) and trailing `-<num>`.
    let unnested = comp_id.rsplit_once('@').map(|(_, r)| r).unwrap_or(comp_id);
    let base = match unnested.rsplit_once('-') {
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => unnested,
    };
    let (mapped, _) = legacy_itemtype_discriminators(base);
    if !mapped.is_empty() {
        Some(mapped)
    } else {
        circ1::registered_itemtypes().find(|&t| t == base || t.eq_ignore_ascii_case(base))
    }
}

/// Remap legacy pin IDs from `.sim1` / `.sim2` XML files to canonical `.circ1` pin IDs.
pub fn canonicalize_legacy_pin_id(pin_id: &str) -> String {
    canonicalize_legacy_pin_id_with_context(pin_id, None)
}

fn canonicalize_legacy_pin_id_with_context(
    pin_id: &str,
    item_info: Option<&HashMap<String, (&'static str, bool)>>,
) -> String {
    let Some((prefix, suffix)) = pin_id.rsplit_once('-') else {
        return pin_id.to_string();
    };
    // SimulIDE LedBar segment pins: `{comp_id}-led{i}-pinP` / `pinN` -> `{comp_id}-lPin{i}` / `rPin{i}`.
    if matches!(suffix, "pinP" | "pinN") {
        if let Some((comp_id, seg)) = prefix.rsplit_once('-') {
            if let Some(idx) = seg.strip_prefix("led") {
                if !idx.is_empty() && idx.chars().all(|c| c.is_ascii_digit()) {
                    let side = if suffix == "pinP" { "lPin" } else { "rPin" };
                    return format!("{comp_id}-{side}{idx}");
                }
            }
        }
    }
    // SimulIDE ResistorDip element pins: `{comp_id}-resistor{i}-ePin{k}`.
    if let Some(epin_str) = suffix.strip_prefix("ePin") {
        if let Ok(epin_idx) = epin_str.parse::<usize>() {
            if let Some((comp_id, seg)) = prefix.rsplit_once('-') {
                if let Some(idx_str) = seg.strip_prefix("resistor") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        let bussed = item_info
                            .and_then(|m| m.get(comp_id))
                            .map(|&(_, b)| b)
                            .unwrap_or(false);
                        if bussed {
                            return format!("{comp_id}-pin{idx}");
                        }
                        let side = if epin_idx == 2 * idx { "lPin" } else { "rPin" };
                        return format!("{comp_id}-{side}{idx}");
                    }
                }
            }
        }
    }

    let comp_type = item_info
        .and_then(|m| m.get(prefix))
        .map(|&(ty, _)| ty)
        .or_else(|| infer_legacy_type_from_comp_id(prefix));

    if let Some(ty) = comp_type {
        match ty {
            Latch::TYPE_ID => match suffix {
                "Pin_clock" => return format!("{prefix}-clk"),
                "Pin_outEnable" => return format!("{prefix}-oe"),
                "Pin_reset" => return format!("{prefix}-rst"),
                _ => {}
            },
            BcdTo7Segment::TYPE_ID => {
                if suffix == "Pin_reset" {
                    return format!("{prefix}-rst");
                }
            }
            Mux::TYPE_ID => match suffix {
                "in8" => return format!("{prefix}-addr0"),
                "in9" => return format!("{prefix}-addr1"),
                "in10" => return format!("{prefix}-addr2"),
                "in11" => return format!("{prefix}-enable"),
                "out0" => return format!("{prefix}-out"),
                "out1" => return format!("{prefix}-out_inv"),
                _ => {}
            },
            Demux::TYPE_ID => match suffix {
                "in0" => return format!("{prefix}-addr0"),
                "in1" => return format!("{prefix}-addr1"),
                "in2" => return format!("{prefix}-addr2"),
                "in3" => return format!("{prefix}-enable"),
                _ => {}
            },
            FullAdder::TYPE_ID => {
                if let Some(n) = suffix
                    .strip_prefix("in")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    if n >= 11 {
                        return format!("{prefix}-in{}", n - 10);
                    }
                }
                if let Some(n) = suffix
                    .strip_prefix("out")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    if n >= 11 {
                        return format!("{prefix}-out{}", n - 10);
                    }
                }
            }
            BinCounter::TYPE_ID => match suffix {
                "Pin_ser" => return format!("{prefix}-Pin_ldp"),
                "Pin_rso" => return format!("{prefix}-Pin_rco"),
                _ => {}
            },
            _ => {}
        }
    }
    let canonical_suffix = match suffix {
        "PinS" | "PinSDA" => "PinSDA",
        "PinC" => {
            if prefix.to_ascii_lowercase().contains("ds1307")
                || prefix.to_ascii_lowercase().contains("ds1621")
            {
                "PinSCL"
            } else {
                "PinC"
            }
        }
        "PinSCL" => "PinSCL",
        "SQW" | "PinSQW" => "PinSQW",
        "PinA" | "aPin" | "mt1Pin" | "+" => PIN_LEFT,
        "PinB" | "kPin" | "mt2Pin" | "-" => PIN_RIGHT,
        "PinM" => PIN_POT_W,
        "PinInput" => "com",
        "PinEnable" => "en",
        "XP" | "vrx_p" => "vrx_p",
        "XM" | "vrx_m" => "vrx_m",
        "YP" | "vry_p" => "vry_p",
        "YM" | "vry_m" => "vry_m",
        "A+" | "PinA1" => "PinA1",
        "B+" | "PinB1" => "PinB1",
        "Co" | "PinCo" => "PinCo",
        "A-" | "PinA2" => "PinA2",
        "B-" | "PinB2" => "PinB2",
        "V+" | "PinV+" => "PinV+",
        "Sig" | "PinSig" => "PinSig",
        "CS" | "PinCS" => "PinCS",
        "DI" | "PinDI" => "PinDI",
        "CK" | "PinCK" => "PinCK",
        "DO" | "PinDO" => "PinDO",
        "Tx" | "pin0" => "pin0",
        "Rx" | "pin1" => "pin1",
        s if s.starts_with("pinY") => {
            return format!("{prefix}-ch{}", &s[4..]);
        }
        s if s.starts_with("pinP") && comp_type != Some(Switch::TYPE_ID) => {
            return format!("{prefix}-lPin{}", &s[4..]);
        }
        s if s.starts_with("switch")
            && s.ends_with("pinN")
            && comp_type != Some(Switch::TYPE_ID) =>
        {
            return format!("{prefix}-rPin{}", &s[6..s.len() - 4]);
        }
        _ => suffix,
    };

    format!("{prefix}-{canonical_suffix}")
}

fn is_bare_number(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.parse::<f64>().is_ok()
}

fn parse_legacy_graphic_attrs(attrs: &[(String, String)]) -> GraphicAttrs {
    let mut g = GraphicAttrs::default();
    for (k, v) in attrs {
        match k.as_str() {
            "Pos" | "pos" => g.pos = parse_point(v),
            "rotation" | "Rotate" | "rot" => g.rotation = v.parse().unwrap_or(0.0),
            "hflip" => g.hflip = parse_flip(Some(v)),
            "vflip" => g.vflip = parse_flip(Some(v)),
            "label" => g.label = Some(v.clone()),
            "Show_id" | "ShowId" | "show_id" => g.show_id = parse_bool(v),
            "idLabPos" | "id_lab_pos" => g.label_pos = parse_point(v),
            "labelrot" | "label_rot" => g.label_rot = v.parse().unwrap_or(0),
            "Show_Val" | "ShowVal" | "show_val" => g.show_val = parse_bool(v),
            "ShowProp" | "show_prop" => g.show_prop = Some(v.clone()),
            "valLabPos" | "val_lab_pos" => g.val_pos = parse_point(v),
            "valLabRot" | "val_rot" => g.val_rot = v.parse().unwrap_or(0),
            "mainComp" | "MainComp" => g.main_comp = parse_bool(v),
            "Links" => g.links = circ1::parse_links_attr(v),
            _ => {}
        }
    }
    g.board = circ1::parse_board_placement(attrs);
    g
}

fn normalize_legacy_attrs(
    canonical_type: &str,
    attrs: &[(String, String)],
    discriminators: &[(&str, &str)],
) -> Vec<(String, String)> {
    let is_gate = matches!(
        canonical_type,
        "AndGate" | "OrGate" | "XorGate" | "NotGate" | "BufferGate"
    );
    let is_freq_device = matches!(canonical_type, "Clock" | "WaveGen");
    let is_cap = matches!(canonical_type, "Capacitor" | "ElCapacitor");
    let is_serial = matches!(canonical_type, "SerialPort" | "SerialTerm");

    let mut out: Vec<(String, String)> = Vec::with_capacity(attrs.len() + discriminators.len());
    for (k, v) in attrs {
        let mut norm_k: &str = match k.as_str() {
            "Show_id" | "ShowId" => "ShowId",
            "Show_Val" | "ShowVal" => "ShowVal",
            "ShowProp" | "show_prop" => "ShowProp",
            "Auto_Load" | "AutoLoad" => "AutoLoad",
            "savePGM" | "SavePgm" => "SavePgm",
            "saveEPR" | "SaveEepr" => "SaveEepr",
            "Active_Low" | "ActiveLow" => "ActiveLow",
            "Norm_Close" | "NormClose" => "NormClose",
            "DT" | "Double_Throw" | "DoubleThrow" => "DoubleThrow",
            "Poles" | "poles" => "Poles",
            "Freq" if is_freq_device => "Frequency",
            "Inverted" if is_gate => "InvertOutputs",
            "1/Lambda" if canonical_type == "Jfet" => "LambdaInv",
            "BrkVolt" if canonical_type == "Diode" => "BrkDownV",
            "SemiAmpli" if canonical_type == "WaveGen" => "Amplitude",
            "MidVolt" if canonical_type == "WaveGen" => "Offset",
            "Value_Volt" if canonical_type == "VoltSource" => "Value",
            "Value_Amp" if canonical_type == "CurrSource" => "Value",
            "Min" if canonical_type == "Dial" => "MinVal",
            "Max" if canonical_type == "Dial" => "MaxVal",
            "Baudrate" if is_serial => "Baud",
            "Num_Bits" if canonical_type == Bus::TYPE_ID => "Width",
            "_16_Bits" | "16_Bits" if canonical_type == BcdToDec::TYPE_ID => "Sixteen",
            "UseRS" if matches!(canonical_type, Latch::TYPE_ID | BcdTo7Segment::TYPE_ID) => {
                "UseReset"
            }
            "Inverted" if canonical_type == BcdTo7Segment::TYPE_ID => "CommonAnode",
            "Invert_Inputs" if canonical_type == Latch::TYPE_ID => "InvertInputs",
            "Address_Bits" if matches!(canonical_type, Mux::TYPE_ID | Demux::TYPE_ID) => "AddrBits",
            "Pin_SET" if canonical_type == Counter::TYPE_ID => "UseSet",
            "Max_Value" if canonical_type == Counter::TYPE_ID => "MaxCount",
            "Max_Value" if canonical_type == BinCounter::TYPE_ID => "MaxValue",
            "Parallel_input"
                if matches!(canonical_type, BinCounter::TYPE_ID | ShiftReg::TYPE_ID) =>
            {
                "ParallelInput"
            }
            "RCO" if canonical_type == BinCounter::TYPE_ID => "UseRco",
            "Bidir" if canonical_type == BinCounter::TYPE_ID => "Bidirectional",
            "PullUp" if canonical_type == ResistorDip::TYPE_ID => "Bussed",
            "Size" if canonical_type == LedBar::TYPE_ID => "Segments",
            "Font_Size" => "FontSize",
            "Font_Color" => "FontColor",
            _ if canonical_type == "Mcu" && k.eq_ignore_ascii_case("mhz") => "Frequency",
            _ => k.as_str(),
        };

        let mut norm_v = v.clone();
        if matches!(
            norm_k,
            "AddrBits"
                | "Bits"
                | "Channels"
                | "Width"
                | "Height"
                | "H_size"
                | "V_size"
                | "Border"
                | "Margin"
                | "FontSize"
                | "Poles"
                | "Segments"
                | "Size"
                | "Num_Inputs"
                | "RPM_Nominal"
                | "RpmNominal"
                | "Z_Value"
                | "Rows"
                | "Cols"
                | "Steps"
                | "Digits"
        ) {
            norm_v = strip_legacy_count_unit(&norm_v);
        }
        if canonical_type == Latch::TYPE_ID && norm_k == "Trigger" && norm_v.trim() == "InEnable" {
            norm_v = "Enable".to_string();
        }
        // Legacy SimulIDE bare-number unit defaults that differ from canonical circ1 base units.
        if canonical_type == "Clock" && k == "Freq" && is_bare_number(&norm_v) {
            norm_v = format!("{} kHz", norm_v.trim());
        } else if canonical_type == "Battery" && norm_k == "Resistance" && is_bare_number(&norm_v) {
            norm_v = format!("{} mΩ", norm_v.trim());
        } else if is_cap && norm_k == "Capacitance" && is_bare_number(&norm_v) {
            norm_v = format!("{} µF", norm_v.trim());
        } else if canonical_type == "Mcu"
            && (k.eq_ignore_ascii_case("mhz") || norm_k == "Frequency")
            && is_bare_number(&norm_v)
        {
            norm_v = format!("{} MHz", norm_v.trim());
        } else if canonical_type == "Ssd1306" && norm_k == "Frequency" {
            norm_v = normalize_legacy_freq_unit(&norm_v);
            if is_bare_number(&norm_v) {
                norm_v = format!("{} kHz", norm_v.trim());
            }
        } else if norm_k == "Frequency" {
            norm_v = normalize_legacy_freq_unit(&norm_v);
        } else if canonical_type == "WaveGen" && norm_k == "Duty" {
            // SimulIDE sometimes writes Duty as a fraction (0..1) or percentage (0..100).
            if let Ok(d) = norm_v.trim().parse::<f64>() {
                if d > 0.0 && d <= 1.0 {
                    norm_v = format!("{}", d * 100.0);
                }
            }
        } else if canonical_type == "AudioOut" && norm_k == "Volume" {
            if let Some(stripped) = norm_v.trim().strip_suffix('%') {
                norm_v = stripped.trim().to_string();
            }
        } else if canonical_type == "Subcircuit" && k == "Logic_Symbol" {
            norm_k = "LogicSymbol";
        }

        out.push((norm_k.to_string(), norm_v));
    }

    // Inject split-type discriminators unless the legacy file already specified them explicitly.
    for &(dk, dv) in discriminators {
        let present = out.iter().any(|(k, _)| k.eq_ignore_ascii_case(dk));
        if !present {
            out.push((dk.to_string(), dv.to_string()));
        }
    }

    out
}

struct PendingSubcItem {
    circ_id: String,
    graphic: GraphicAttrs,
    normalized_attrs: Vec<(String, String)>,
    exposed_overrides: BTreeMap<(String, String), String>,
    program: Option<String>,
}

fn finalize_pending_subc(
    mut pending: PendingSubcItem,
    items: &mut Vec<ParsedCirc1Item>,
    item_info: &mut HashMap<String, (&'static str, bool)>,
) -> Result<()> {
    if let Some(prog) = pending.program {
        let has_prog = pending
            .normalized_attrs
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Program"));
        if !has_prog {
            pending.normalized_attrs.push(("Program".to_string(), prog));
        }
    }
    if !pending.exposed_overrides.is_empty() {
        pending.normalized_attrs.push((
            "ExposedProps".to_string(),
            subcircuit::encode_exposed_overrides(&pending.exposed_overrides),
        ));
    }
    let part = circ1::create_part_lenient(
        Subcircuit::TYPE_ID,
        pending.circ_id.clone(),
        &pending.normalized_attrs,
    )?;
    let type_id = part.type_id();
    item_info.insert(pending.circ_id.clone(), (type_id, false));
    items.push(ParsedCirc1Item {
        itemtype: type_id.to_string(),
        circ_id: pending.circ_id,
        attrs: pending.normalized_attrs,
        graphic: pending.graphic,
        part,
    });
    Ok(())
}

/// Parse a legacy SimulIDE `.sim1` / `.sim2` document into canonical [`ParsedCirc1`].
///
/// Unknown legacy itemtypes are recorded in `ParsedCirc1::skipped` rather than failing the load,
/// matching SimulIDE's tolerant loading behavior.
pub fn parse_to_circ1(src: &str) -> Result<ParsedCirc1> {
    let mut circ = CircSettings::default();
    let mut items = Vec::new();
    let mut raw_connectors: Vec<(String, String, String, Vec<(f64, f64)>)> = Vec::new();
    let mut item_info: HashMap<String, (&'static str, bool)> = HashMap::new();
    let mut pending_subc: Option<PendingSubcItem> = None;
    let mut skipped = Vec::new();
    let mut saw_circuit = false;

    for raw in src.lines() {
        let line = raw.trim();
        if let Some(subc) = pending_subc.as_mut() {
            if line.starts_with("<mainCompProps") {
                let mut child_props = parse_xml_props(line);
                if let Some(pos) = child_props.iter().position(|(k, _)| k == "MainCompId") {
                    let (_, main_comp_id) = child_props.remove(pos);
                    let inferred_type = infer_legacy_type_from_comp_id(&main_comp_id).unwrap_or(
                        if child_props
                            .iter()
                            .any(|(k, _)| k == "Program" || k == "Frequency")
                        {
                            "Mcu"
                        } else {
                            ""
                        },
                    );
                    let norm_child = normalize_legacy_attrs(inferred_type, &child_props, &[]);
                    for (k, v) in norm_child {
                        if k == "Program" && !v.is_empty() {
                            subc.program = Some(v.clone());
                        }
                        subc.exposed_overrides.insert((main_comp_id.clone(), k), v);
                    }
                }
                continue;
            }
            if line.starts_with("</item") {
                if let Some(done) = pending_subc.take() {
                    finalize_pending_subc(done, &mut items, &mut item_info)?;
                }
                continue;
            }
        }
        if line.starts_with("<circuit") {
            saw_circuit = true;
            let props = parse_xml_props(line);
            if let Some(v) = prop(&props, "reaStep").and_then(|v| v.parse::<u64>().ok()) {
                circ.react_step_ps = v.max(1);
            }
            if let Some(v) = prop(&props, "NLsteps").and_then(|v| v.parse::<u32>().ok()) {
                circ.nl_steps = v.max(1);
            }
            if let Some(v) = prop(&props, "stepSize").and_then(|v| v.parse::<u64>().ok()) {
                circ.step_size = v.max(1);
            }
            if let Some(v) = prop(&props, "stepsPS").and_then(|v| v.parse::<u64>().ok()) {
                circ.steps_ps = v.max(1);
            }
            if let Some(v) = prop(&props, "animate") {
                circ.animate_logic = v != "0";
            }
            if let Some(v) = prop(&props, "anicurr") {
                circ.animate_curr = v != "0";
            }
            if let Some(v) = prop(&props, "ansi") {
                circ.ansi = v != "0";
            }
            if let Some(v) = prop(&props, "width").and_then(|v| v.parse::<i32>().ok()) {
                circ.width = v.clamp(1, 10_000);
            }
            if let Some(v) = prop(&props, "height").and_then(|v| v.parse::<i32>().ok()) {
                circ.height = v.clamp(1, 10_000);
            }
            continue;
        }
        if !line.starts_with("<item") {
            continue;
        }
        let mut properties = parse_xml_props(line);
        if properties.is_empty() {
            continue;
        }
        let (n0, type_) = properties.remove(0);
        if n0 != "itemtype" {
            continue;
        }
        match type_.as_str() {
            "Connector" => {
                let mut start = None;
                let mut end = None;
                let mut id = String::new();
                let mut points = Vec::new();
                for (k, v) in properties {
                    match k.as_str() {
                        "startpinid" | "start" => start = Some(v),
                        "endpinid" | "end" => end = Some(v),
                        "uid" | "CircId" | "id" => id = v,
                        "pointList" => points = parse_point_list(&v),
                        _ => {}
                    }
                }
                match (start, end) {
                    (Some(s), Some(e)) => {
                        if id.is_empty() {
                            id = format!("Connector-{}", raw_connectors.len() + 1);
                        }
                        raw_connectors.push((id, s, e, points));
                    }
                    _ => {
                        return Err(Error::Parse(
                            "Connector missing startpinid or endpinid".into(),
                        ));
                    }
                }
            }
            other => {
                let (mapped, discriminators) = legacy_itemtype_discriminators(other);
                let canonical_type = if !mapped.is_empty() { mapped } else { other };
                if circ1::lookup_factory(canonical_type).is_none() {
                    skipped.push(other.to_string());
                    continue;
                }
                let circ_id = prop(&properties, "CircId")
                    .or_else(|| prop(&properties, "uid"))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{other}-{}", items.len() + 1));
                let graphic = parse_legacy_graphic_attrs(&properties);
                let normalized_attrs =
                    normalize_legacy_attrs(canonical_type, &properties, discriminators);
                if canonical_type == Subcircuit::TYPE_ID && !line.ends_with("/>") {
                    if let Some(prev) = pending_subc.take() {
                        finalize_pending_subc(prev, &mut items, &mut item_info)?;
                    }
                    pending_subc = Some(PendingSubcItem {
                        circ_id,
                        graphic,
                        normalized_attrs,
                        exposed_overrides: BTreeMap::new(),
                        program: None,
                    });
                    continue;
                }
                let part =
                    circ1::create_part_lenient(canonical_type, circ_id.clone(), &normalized_attrs)?;
                let bussed = match &part {
                    Part::ResistorDip(r) => r.bussed,
                    _ => false,
                };
                let type_id = part.type_id();
                item_info.insert(circ_id.clone(), (type_id, bussed));
                let emitted_type = type_id.to_string();
                items.push(ParsedCirc1Item {
                    itemtype: emitted_type,
                    circ_id,
                    attrs: normalized_attrs,
                    graphic,
                    part,
                });
            }
        }
    }

    if let Some(done) = pending_subc.take() {
        finalize_pending_subc(done, &mut items, &mut item_info)?;
    }

    let connectors: Vec<ParsedConnector> = raw_connectors
        .into_iter()
        .map(|(id, s, e, points)| ParsedConnector {
            id,
            start: canonicalize_legacy_pin_id_with_context(&s, Some(&item_info)),
            end: canonicalize_legacy_pin_id_with_context(&e, Some(&item_info)),
            points,
        })
        .collect();

    if !saw_circuit && items.is_empty() && connectors.is_empty() && skipped.is_empty() {
        return Err(Error::Parse("missing <circuit> element".into()));
    }

    Ok(ParsedCirc1 {
        version: FORMAT_VERSION.to_string(),
        circ,
        items,
        connectors,
        skipped,
    })
}

/// Convert a legacy SimulIDE `.sim1` / `.sim2` document into canonical `.circ1` XML text.
pub fn convert_to_circ1_text(src: &str) -> Result<String> {
    let parsed = parse_to_circ1(src)?;
    let item_lines: Vec<String> = parsed
        .items
        .iter()
        .map(|it| it.part.write_item(&it.circ_id, &it.graphic))
        .collect();
    Ok(circ1::write_circ1(
        &item_lines,
        &parsed.connectors,
        &parsed.circ,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circ1::ParsedCircuit;
    use crate::elements::Kind;

    fn load_circuit(src: &str) -> ParsedCircuit {
        circ1::to_parsed_circuit(circ1::load_circ1(src).unwrap())
    }

    #[test]
    fn parse_bjt_and_mosfet() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="BJT" CircId="BJT-1" PNP="true" Gain="200" Pos="0,0" />
<item itemtype="Mosfet" CircId="Mosfet-1" P_Channel="true" Depletion="true" RDSon="2 Ω" Threshold="1 V" Pos="40,0" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 2);
        match &p.items[0].comp.kind {
            Kind::Bjt { state } => {
                assert!(state.pnp);
                assert!((state.gain - 200.0).abs() < 1e-9);
            }
            other => panic!("expected BJT, got {other:?}"),
        }
        match &p.items[1].comp.kind {
            Kind::Mosfet { state } => {
                assert!(state.p_channel);
                assert!(state.depletion);
                assert!((state.rdson - 2.0).abs() < 1e-9);
                assert!((state.threshold - 1.0).abs() < 1e-9);
            }
            other => panic!("expected Mosfet, got {other:?}"),
        }
    }

    #[test]
    fn parse_opamp() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="opAmp" CircId="opAmp-1" Gain="500" Out_Imped="1 Ω" Volt_Pos="12 V" Volt_Neg="-12 V" Power_Pins="true" Switch_Pins="true" Pos="8,4" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::OpAmp { state } => {
                assert!((state.gain - 500.0).abs() < 1e-9);
                assert!((state.out_imp - 1.0).abs() < 1e-9);
                assert!((state.volt_pos - 12.0).abs() < 1e-9);
                assert!((state.volt_neg + 12.0).abs() < 1e-9);
                assert!(state.power_pins);
                assert!(state.switch_pins);
            }
            other => panic!("expected opAmp, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((8.0, 4.0)));
    }

    #[test]
    fn parse_jfet() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Jfet" CircId="Jfet-1" Idss="20 mA" Vp="-2 V" 1/Lambda="500 V" Pos="16,8" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::Jfet { state } => {
                assert!((state.idss - 0.020).abs() < 1e-9);
                assert!((state.vp + 2.0).abs() < 1e-9);
                assert!((state.lambda_inv - 500.0).abs() < 1e-9);
            }
            other => panic!("expected Jfet, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((16.0, 8.0)));
    }

    #[test]
    fn parse_comparator() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Comparator" CircId="Comparator-1" Out_High_V="3.3 V" Out_Low_V="0.2 V" Out_Imped="20 Ω" Inverted="true" Open_Collector="true" Pos="8,4" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::Comparator { state } => {
                assert!((state.out_high() - 3.3).abs() < 1e-9);
                assert!((state.out_low() - 0.2).abs() < 1e-9);
                assert!((state.out_imp() - 20.0).abs() < 1e-9);
                assert!(state.inverted);
                assert!(state.open_col);
            }
            other => panic!("expected Comparator, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((8.0, 4.0)));
    }

    #[test]
    fn parse_test_unit() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="TestUnit" CircId="TestUnit-1" Inputs="A,B" Outputs="Y" Period="100 ns" Truth="0,0,0,1," Pos="0,0" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::TestUnit(t) => {
                assert_eq!(t.drive.len(), 2);
                assert_eq!(t.sense.len(), 1);
                assert!((t.period - 100e-9).abs() < 1e-15, "period {}", t.period);
                assert_eq!(t.truth, vec![0, 0, 0, 1]);
                assert_eq!(t.drive[0].id, "TestUnit-1-out0");
                assert_eq!(t.sense[0].id, "TestUnit-1-in0");
            }
            other => panic!("expected TestUnit, got {other:?}"),
        }
    }

    #[test]
    fn parse_and_gate() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="And Gate" CircId="And Gate-1" Num_Inputs="3" Tpd_ps="20 ns" initHigh="true" Pos="8,4" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::Gate(g) => {
                assert_eq!(g.inputs.len(), 3);
                assert!(g.init_high);
                assert!((g.family.delay_base - 20_000.0).abs() < 1.0);
            }
            other => panic!("expected And Gate, got {other:?}"),
        }
    }

    #[test]
    fn parse_volt_reg() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="VoltReg" CircId="VoltReg-1" Voltage="5 V" Pos="16,8" />
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::VoltReg { state } => {
                assert!((state.v_ref - 5.0).abs() < 1e-9);
            }
            other => panic!("expected VoltReg, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((16.0, 8.0)));
    }

    #[test]
    fn parse_instruments() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="Probe" CircId="Probe-1" Threshold="1.2 V" Small="true" Pos="0,0" />
<item itemtype="Voltimeter" CircId="Voltimeter-1" RMS="true" Pos="8,0" />
<item itemtype="Amperimeter" CircId="Amperimeter-1" RMS="false" Pos="16,0" />
<item itemtype="Oscope" CircId="Oscope-1" connectGnd="false" Pos="32,0" />
<item itemtype="LAnalizer" CircId="LAnalizer-1" Pos="64,0" />
</circuit>
"#;
        let p = load_circuit(src);
        assert!(p.skipped.is_empty());
        assert_eq!(p.items.len(), 5);
        match &p.items[0].comp.kind {
            Kind::Probe { threshold, small } => {
                assert!((threshold - 1.2).abs() < 1e-9);
                assert!(small);
            }
            other => panic!("expected Probe, got {other:?}"),
        }
        match &p.items[1].comp.kind {
            Kind::Voltmeter { rms, .. } => assert!(rms),
            other => panic!("expected Voltmeter, got {other:?}"),
        }
        match &p.items[3].comp.kind {
            Kind::Oscope { connect_gnd, .. } => assert!(!connect_gnd),
            other => panic!("expected Oscope, got {other:?}"),
        }
        match &p.items[4].comp.kind {
            Kind::LAnalizer { connect_gnd, .. } => assert!(connect_gnd),
            other => panic!("expected LAnalizer, got {other:?}"),
        }
    }

    #[test]
    fn parse_circuit_header() {
        let src = r#"<circuit version="2.0.0" stepSize="1000" stepsPS="500000" NLsteps="50" reaStep="2000" animate="1" anicurr="0" ansi="1" width="1600" height="1200" >
</circuit>
"#;
        let p = load_circuit(src);
        assert_eq!(p.circ.step_size, 1000);
        assert_eq!(p.circ.steps_ps, 500_000);
        assert_eq!(p.circ.nl_steps, 50);
        assert_eq!(p.circ.react_step_ps, 2000);
        assert!(p.circ.animate_logic);
        assert!(!p.circ.animate_curr);
        assert!(p.circ.ansi);
        assert_eq!(p.circ.width, 1600);
        assert_eq!(p.circ.height, 1200);
        assert!((p.analog_dt - 2e-9).abs() < 1e-20);
    }

    #[test]
    fn parse_mcu_item() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="pic14test-1" Frequency="4 MHz" ForceFreq="true" Program="blink.hex" Auto_Load="true" savePGM="false" pgm="" Pos="8,4" />
</circuit>
"#;
        let p = load_circuit(src);
        assert!(p.skipped.is_empty());
        assert_eq!(p.items.len(), 1);
        match &p.items[0].comp.kind {
            Kind::McuItem(spec) => {
                assert_eq!(spec.device, "pic14test");
                assert!((spec.frequency.unwrap() - 4e6).abs() < 1.0);
                assert!(spec.force_freq);
                assert_eq!(spec.program.as_deref(), Some("blink.hex"));
                assert!(spec.auto_load);
                assert!(!spec.save_pgm);
                assert!(spec.pgm.is_none());
            }
            other => panic!("expected McuItem, got {other:?}"),
        }
        assert_eq!(p.items[0].pos, Some((8.0, 4.0)));
    }

    #[test]
    fn parse_mcu_old_mhz() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="p16f84-1" Mhz="16" />
</circuit>
"#;
        let p = load_circuit(src);
        match &p.items[0].comp.kind {
            Kind::McuItem(spec) => {
                assert_eq!(spec.device, "p16F84");
                assert!((spec.frequency.unwrap() - 16e6).abs() < 1.0);
            }
            other => panic!("expected McuItem, got {other:?}"),
        }
    }

    #[test]
    fn parse_qemu_device() {
        let src = r#"<circuit version="2.0.0" >
<item itemtype="QemuDevice" CircId="Esp32-1" Program="app.bin" Args="-d,in_asm" Pos="8,4" />
<item itemtype="QemuDevice" CircId="STM32F103C8-1" Program="fw.bin" />
</circuit>
"#;
        let p = load_circuit(src);
        assert!(p.skipped.is_empty(), "{:?}", p.skipped);
        assert_eq!(p.items.len(), 2);
        match &p.items[0].comp.kind {
            Kind::QemuDevice(q) => {
                assert_eq!(q.family, crate::qemu::QemuFamily::Esp32);
                assert_eq!(q.firmware, "app.bin");
                assert_eq!(q.extra_args, "-d,in_asm");
            }
            other => panic!("expected QemuDevice Esp32, got {other:?}"),
        }
        match &p.items[1].comp.kind {
            Kind::QemuDevice(q) => {
                assert_eq!(q.family, crate::qemu::QemuFamily::Stm32);
                assert_eq!(q.firmware, "fw.bin");
            }
            other => panic!("expected QemuDevice STM32, got {other:?}"),
        }
    }

    #[test]
    fn convert_legacy_split_types_to_strict_circ1() {
        let src = r#"<circuit version="1.1.0-RC1" >
<item itemtype="Fixed Voltage" CircId="Fixed-1" Pos="0,0" Voltage="5 V" />
<item itemtype="Voltimeter" CircId="Voltimeter-1" Pos="100,0" Show_id="true" />
<item itemtype="FlipFlopJK" CircId="FF-1" Pos="16,16" />
<item itemtype="Zener Diode" CircId="Z-1" Pos="32,16" />
<item itemtype="DHT11" CircId="S-1" Pos="48,16" />
<item itemtype="ROM" CircId="M-1" Pos="64,16" />
<item itemtype="Connector" uid="c1" startpinid="Fixed-1-outnod" endpinid="Voltimeter-1-lPin" />
</circuit>
"#;
        let circ1_xml = convert_to_circ1_text(src).unwrap();
        let strict =
            circ1::parse_circ1(&circ1_xml).expect("converted XML must pass strict parse_circ1");
        assert_eq!(strict.version, FORMAT_VERSION);
        assert_eq!(strict.items.len(), 6);
        assert_eq!(strict.items[0].itemtype, "FixedVolt");
        assert_eq!(strict.items[1].itemtype, "Voltmeter");
        assert!(strict.items[1].graphic.show_id);
        assert_eq!(
            strict.items[2].part.get_prop_text("Kind").as_deref(),
            Some("JK")
        );
        assert_eq!(
            strict.items[3].part.get_prop_text("Zener").as_deref(),
            Some("true")
        );
        assert_eq!(
            strict.items[4].part.get_prop_text("Model").as_deref(),
            Some("DHT11")
        );
        assert_eq!(
            strict.items[5].part.get_prop_text("IsRom").as_deref(),
            Some("true")
        );
        assert_eq!(strict.connectors.len(), 1);
        assert_eq!(strict.connectors[0].id, "c1");
    }

    #[test]
    fn parse_simulide_1_1_unit_suffixes_main_comp_props_and_switch_pins() {
        let src = r##"<circuit version="1.1.0-RC1" >
<item itemtype="Rectangle" CircId="Rectangle-1" H_size="680 _Pixels" V_size="400 Pixels" Border="2 _Pixels" Color="#a0a0a4" Opacity="0" Pos="0,0" />
<item itemtype="TextComponent" CircId="TextComponent-1" Margin="0 Pixels" Border="1 Pixels" Font_Size="9 _Pixels" Font_Color="#ffff00" Text="Hello" Pos="10,10" />
<item itemtype="Switch" CircId="Switch-1" Norm_Close="true" DT="false" Poles="1 _Poles" Pos="20,20" />
<item itemtype="Push" CircId="Push-1" Poles="1 _Poles" Pos="30,20" />
<item itemtype="DcMotor" CircId="DcMotor-1" RPM_Nominal="60 _RPM" Volt_Nominal="5 V" Resistance="100 Ω" Pos="40,20" />
<item itemtype="LedBar" CircId="LedBar-1" Color="Yellow" Size="1 Leds" Grounded="false" Pos="50,20" />
<item itemtype="Ssd1306" CircId="Ssd1306-1" Width="128 _Pixels" Height="64 _Pixels" Frequency="100 _KHz" Pos="60,20" />
<item itemtype="Subcircuit" CircId="UNO-1" Pos="70,20">
<mainCompProps MainCompId="mega328-519" Frequency="16 MHz" Program="app.hex" />
<mainCompProps MainCompId="LedBar-2" Size="1 _Leds" Color="Yellow" />
</item>
<item itemtype="Connector" uid="c1" startpinid="Switch-1-pinP0" endpinid="Switch-1-switch0pinN" />
<item itemtype="Connector" uid="c2" startpinid="Push-1-pinP0" endpinid="Push-1-switch0pinN" />
</circuit>
"##;
        let parsed = parse_to_circ1(src).expect("SimulIDE 1.1.0-RC1 circuit must parse");
        assert_eq!(parsed.items.len(), 8);
        assert_eq!(
            parsed.items[0].part.get_prop_text("H_size").as_deref(),
            Some("680")
        );
        assert_eq!(
            parsed.items[1].part.get_prop_text("FontSize").as_deref(),
            Some("9")
        );
        assert_eq!(
            parsed.items[4].part.get_prop_text("RpmNominal").as_deref(),
            Some("60")
        );
        assert_eq!(
            parsed.items[5].part.get_prop_text("Segments").as_deref(),
            Some("1")
        );
        match &parsed.items[7].part {
            Part::Subcircuit(s) => {
                assert_eq!(s.program, "app.hex");
                assert_eq!(
                    s.exposed_overrides
                        .get(&("mega328-519".to_string(), "Program".to_string()))
                        .map(String::as_str),
                    Some("app.hex")
                );
                assert_eq!(
                    s.exposed_overrides
                        .get(&("LedBar-2".to_string(), "Segments".to_string()))
                        .map(String::as_str),
                    Some("1")
                );
            }
            other => panic!("expected Subcircuit, got {other:?}"),
        }
        // Switch preserves pinP0 / switch0pinN; Push remaps to lPin0 / rPin0.
        assert_eq!(parsed.connectors[0].start, "Switch-1-pinP0");
        assert_eq!(parsed.connectors[0].end, "Switch-1-switch0pinN");
        assert_eq!(parsed.connectors[1].start, "Push-1-lPin0");
        assert_eq!(parsed.connectors[1].end, "Push-1-rPin0");
    }
}
