//! Native `.circ1` format parser, serializer, and component factory map.

use crate::components::{Component, Part, PropError};
use crate::settings::CircSettings;
use crate::sim1::{
    ParsedConnector, fmt_pos, parse_bool, parse_point, parse_point_list, parse_xml_props, prop,
};
use crate::{Error, Result};

/// Native .circ1 format version.
pub const FORMAT_VERSION: &str = "1.0.0";

/// Backwards-compatibility aliases
pub type ParsedV3 = ParsedCirc1;
pub type ParsedV3Item = ParsedCirc1Item;
pub use parse_circ1 as parse_circuit_v3;
pub use write_circ1 as write_circuit_v3;

/// Shared graphic attributes written beside the type's [`crate::components::PropDef`] list.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphicAttrs {
    pub pos: Option<(f64, f64)>,
    pub rotation: f64,
    pub hflip: i32,
    pub vflip: i32,
    pub label: Option<String>,
    pub show_id: bool,
    pub label_pos: Option<(f64, f64)>,
    pub label_rot: i32,
    pub show_val: bool,
    pub show_prop: Option<String>,
    pub val_pos: Option<(f64, f64)>,
    pub val_rot: i32,
}

impl Default for GraphicAttrs {
    fn default() -> Self {
        Self {
            pos: None,
            rotation: 0.0,
            hflip: 1,
            vflip: 1,
            label: None,
            show_id: false,
            label_pos: None,
            label_rot: 0,
            show_val: false,
            show_prop: None,
            val_pos: None,
            val_rot: 0,
        }
    }
}

impl GraphicAttrs {
    pub fn at(x: f64, y: f64) -> Self {
        Self {
            pos: Some((x, y)),
            ..Self::default()
        }
    }

    fn write_attrs(&self) -> String {
        let mut s = String::new();
        if let Some((x, y)) = self.pos {
            s.push_str(&format!(" Pos=\"{}\"", fmt_pos(x, y)));
        }
        if self.rotation != 0.0 {
            s.push_str(&format!(" rotation=\"{}\"", self.rotation));
        }
        if self.hflip != 1 {
            s.push_str(&format!(" hflip=\"{}\"", self.hflip));
        }
        if self.vflip != 1 {
            s.push_str(&format!(" vflip=\"{}\"", self.vflip));
        }
        if let Some(label) = &self.label {
            s.push_str(&format!(" label=\"{label}\""));
        }
        if self.show_id {
            s.push_str(" ShowId=\"true\"");
        }
        if let Some((x, y)) = self.label_pos {
            s.push_str(&format!(" idLabPos=\"{}\"", fmt_pos(x, y)));
        }
        if self.label_rot != 0 {
            s.push_str(&format!(" labelrot=\"{}\"", self.label_rot));
        }
        if self.show_val {
            s.push_str(" ShowVal=\"true\"");
        }
        if let Some(p) = &self.show_prop {
            s.push_str(&format!(" ShowProp=\"{p}\""));
        }
        if let Some((x, y)) = self.val_pos {
            s.push_str(&format!(" valLabPos=\"{}\"", fmt_pos(x, y)));
        }
        if self.val_rot != 0 {
            s.push_str(&format!(" valLabRot=\"{}\"", self.val_rot));
        }
        s
    }

    fn from_attrs(attrs: &[(String, String)]) -> Self {
        let mut g = Self::default();
        g.pos = prop(attrs, "Pos").and_then(parse_point).or_else(|| {
            let x = prop(attrs, "x")?.parse::<f64>().ok()?;
            let y = prop(attrs, "y")?.parse::<f64>().ok()?;
            Some((x, y))
        });
        g.rotation = prop(attrs, "rotation")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        g.hflip = parse_flip(prop(attrs, "hflip"));
        g.vflip = parse_flip(prop(attrs, "vflip"));
        g.label = prop(attrs, "label").map(str::to_string);
        g.show_id = prop(attrs, "ShowId").map(parse_bool).unwrap_or(false);
        g.label_pos = prop(attrs, "idLabPos").and_then(parse_point);
        g.label_rot = prop(attrs, "labelrot")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        g.show_val = prop(attrs, "ShowVal").map(parse_bool).unwrap_or(false);
        g.show_prop = prop(attrs, "ShowProp").map(str::to_string);
        g.val_pos = prop(attrs, "valLabPos").and_then(parse_point);
        g.val_rot = prop(attrs, "valLabRot")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        g
    }
}

fn parse_flip(v: Option<&str>) -> i32 {
    match v.and_then(|s| s.parse::<i32>().ok()) {
        Some(-1) => -1,
        _ => 1,
    }
}

#[allow(dead_code)]
fn is_graphic_attr(name: &str) -> bool {
    matches!(
        name,
        "CircId"
            | "Pos"
            | "rotation"
            | "hflip"
            | "vflip"
            | "label"
            | "ShowId"
            | "idLabPos"
            | "labelrot"
            | "ShowVal"
            | "ShowProp"
            | "valLabPos"
            | "valLabRot"
    )
}

pub type PartFactory = fn(circ_id: String, attrs: &[(String, String)]) -> Result<Part>;

struct FactoryEntry {
    itemtype: &'static str,
    create: PartFactory,
}

fn create_of<T, F>(_circ_id: String, attrs: &[(String, String)], wrap: F) -> Result<Part>
where
    T: crate::components::Component + Default,
    F: FnOnce(T) -> Part,
{
    let mut p = T::default();
    apply_item_attrs(&mut p, attrs)?;
    Ok(wrap(p))
}

fn create_resistor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Resistor)
}
fn create_var_resistor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::VarResistor)
}
fn create_potentiometer(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Potentiometer)
}
fn create_capacitor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Capacitor)
}
fn create_el_capacitor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::ElCapacitor)
}
fn create_inductor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Inductor)
}
fn create_var_capacitor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::VarCapacitor)
}
fn create_var_inductor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::VarInductor)
}
fn create_battery(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Battery)
}
fn create_ground(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Ground)
}
fn create_node(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Node)
}
fn create_fixed_volt(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::FixedVolt)
}
fn create_rail(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Rail)
}
fn create_clock(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Clock)
}
fn create_volt_source(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::VoltSource)
}
fn create_curr_source(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::CurrSource)
}
fn create_switch(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Switch)
}
fn create_push(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Push)
}
fn create_switch_dip(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SwitchDip)
}
fn create_relay(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Relay)
}
fn create_keypad(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::KeyPad)
}
fn create_diode(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut p = crate::components::Diode::default();
    apply_item_attrs(&mut p, attrs)?;
    Ok(Part::Diode(p))
}
fn create_zener(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut p = crate::components::Diode::zener_default();
    apply_item_attrs(&mut p, attrs)?;
    Ok(Part::Diode(p))
}
fn create_led(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Led)
}
fn create_bjt(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Bjt)
}
fn create_mosfet(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Mosfet)
}
fn create_jfet(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Jfet)
}
fn create_opamp(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::OpAmp)
}
fn create_comparator(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Comparator)
}
fn create_volt_reg(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::VoltReg)
}
fn create_probe(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Probe)
}
fn create_voltmeter(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Voltmeter)
}
fn create_ammeter(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Ammeter)
}
fn create_freq_meter(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::FreqMeter)
}
fn create_and_gate(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::AndGate)
}
fn create_or_gate(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::OrGate)
}
fn create_xor_gate(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::XorGate)
}
fn create_not_gate(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::NotGate)
}
fn create_buffer_gate(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::BufferGate)
}
fn create_flip_flop(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::FlipFlop)
}
fn create_latch(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Latch)
}
fn create_mux(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Mux)
}
fn create_demux(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Demux)
}
fn create_analog_mux(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::AnalogMux)
}
fn create_bcd_to_dec(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::BcdToDec)
}
fn create_dec_to_bcd(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DecToBcd)
}
fn create_bcd_to_7segment(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::BcdTo7Segment)
}
fn create_full_adder(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::FullAdder)
}
fn create_half_adder(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::HalfAdder)
}
fn create_test_unit(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::TestUnit)
}
fn create_logic_analyzer(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::LogicAnalyzer)
}
fn create_touchpad(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::TouchPad)
}
fn create_ky023(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::KY023)
}
fn create_ky040(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::KY040)
}
fn create_sr04(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SR04)
}
fn create_dht22(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DHT22)
}
fn create_ds18b20(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DS18B20)
}
fn create_ds1621(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DS1621)
}
fn create_ds1307(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DS1307)
}
fn create_dc_motor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DcMotor)
}
fn create_stepper(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Stepper)
}
fn create_servo(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Servo)
}
fn create_sd_card(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SdCard)
}
fn create_esp01(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Esp01)
}
fn create_tft_display(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::TftDisplay)
}
fn create_pcd8544(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Pcd8544)
}
fn create_sh1107(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Sh1107)
}
fn create_ks0108(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Ks0108)
}
fn create_pcf8833(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Pcf8833)
}
fn create_aip31068(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Aip31068)
}
fn create_hd44780(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Hd44780)
}
fn create_ssd1306(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Ssd1306)
}
fn create_seven_segment(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SevenSegment)
}
fn create_seven_segment_bcd(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SevenSegmentBCD)
}
fn create_led_matrix(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::LedMatrix)
}
fn create_max72xx(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Max72xx)
}
fn create_led_bar(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::LedBar)
}
fn create_rgb_led(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::RgbLed)
}
fn create_ws2812(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Ws2812)
}
fn create_counter(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Counter)
}
fn create_bin_counter(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::BinCounter)
}
fn create_shift_reg(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::ShiftReg)
}
fn create_magnitude_comp(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::MagnitudeComp)
}
fn create_adc(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Adc)
}
fn create_dac(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Dac)
}
fn create_i2c_to_parallel(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::I2CToParallel)
}
fn create_lm555(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Lm555)
}
fn create_memory(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Memory)
}
fn create_dynamic_memory(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::DynamicMemory)
}
fn create_i2c_ram(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::I2CRam)
}
fn create_function(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Function)
}
fn create_transformer(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Transformer)
}
fn create_scr(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Scr)
}
fn create_diac(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Diac)
}
fn create_triac(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Triac)
}
fn create_csource(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Csource)
}
fn create_resistor_dip(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::ResistorDip)
}
fn create_ldr(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Ldr)
}
fn create_thermistor(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Thermistor)
}
fn create_rtd(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Rtd)
}
fn create_strain(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Strain)
}
fn create_audio_out(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::AudioOut)
}
fn create_lamp(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Lamp)
}
fn create_wave_gen(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::WaveGen)
}
fn create_oscope(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Oscope)
}
fn create_tunnel(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Tunnel)
}
fn create_bus(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Bus)
}
fn create_socket(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Socket)
}
fn create_header(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Header)
}
fn create_serial_port(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SerialPort)
}
fn create_serial_term(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SerialTerm)
}
fn create_subcircuit(id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Subcircuit::default();
    s.device = crate::subcircuit::device_from_id(&id);
    apply_item_attrs(&mut s, attrs)?;
    if s.device.is_empty() {
        s.device = crate::subcircuit::device_from_id(&id);
    }
    Ok(Part::Subcircuit(s))
}
fn create_sub_package(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::SubPackage)
}
fn create_dial(id: String, attrs: &[(String, String)]) -> Result<Part> {
    create_of(id, attrs, Part::Dial)
}
fn create_shape(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Shape::new_for_kind(crate::components::ShapeKind::Rectangle);
    apply_item_attrs(&mut s, attrs)?;
    Ok(Part::Shape(s))
}
fn create_rectangle(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Shape::new_for_kind(crate::components::ShapeKind::Rectangle);
    apply_item_attrs(&mut s, attrs)?;
    Ok(Part::Shape(s))
}
fn create_ellipse(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Shape::new_for_kind(crate::components::ShapeKind::Ellipse);
    apply_item_attrs(&mut s, attrs)?;
    Ok(Part::Shape(s))
}
fn create_line(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Shape::new_for_kind(crate::components::ShapeKind::Line);
    apply_item_attrs(&mut s, attrs)?;
    Ok(Part::Shape(s))
}
fn create_text_component(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Shape::new_for_kind(crate::components::ShapeKind::Text);
    apply_item_attrs(&mut s, attrs)?;
    Ok(Part::Shape(s))
}
fn create_image(_id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut s = crate::components::Shape::new_for_kind(crate::components::ShapeKind::Image);
    apply_item_attrs(&mut s, attrs)?;
    Ok(Part::Shape(s))
}
fn create_mcu(id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut m = crate::components::Mcu::default();
    m.device = crate::subcircuit::device_from_id(&id);
    m.mcu.device.id = id.clone();
    apply_item_attrs(&mut m, attrs)?;
    if m.device.is_empty() {
        m.device = crate::subcircuit::device_from_id(&id);
    }
    if m.mcu.device.id.is_empty() {
        m.mcu.device.id = id;
    }
    Ok(Part::Mcu(m))
}
fn create_qemu_device(id: String, attrs: &[(String, String)]) -> Result<Part> {
    let mut q = crate::components::QemuDevice::default();
    q.qemu.device = crate::subcircuit::device_from_id(&id);
    apply_item_attrs(&mut q, attrs)?;
    if q.qemu.device.is_empty() {
        q.qemu.device = crate::subcircuit::device_from_id(&id);
    }
    Ok(Part::QemuDevice(q))
}

/// Registered canonical `itemtype` values. Unknown types are parse errors
/// (no aliases).
static FACTORY: &[FactoryEntry] = &[
    FactoryEntry {
        itemtype: crate::components::Resistor::TYPE_ID,
        create: create_resistor,
    },
    FactoryEntry {
        itemtype: crate::components::VarResistor::TYPE_ID,
        create: create_var_resistor,
    },
    FactoryEntry {
        itemtype: crate::components::Potentiometer::TYPE_ID,
        create: create_potentiometer,
    },
    FactoryEntry {
        itemtype: crate::components::Capacitor::TYPE_ID,
        create: create_capacitor,
    },
    FactoryEntry {
        itemtype: crate::components::ElCapacitor::TYPE_ID,
        create: create_el_capacitor,
    },
    FactoryEntry {
        itemtype: crate::components::Inductor::TYPE_ID,
        create: create_inductor,
    },
    FactoryEntry {
        itemtype: crate::components::VarCapacitor::TYPE_ID,
        create: create_var_capacitor,
    },
    FactoryEntry {
        itemtype: crate::components::VarInductor::TYPE_ID,
        create: create_var_inductor,
    },
    FactoryEntry {
        itemtype: crate::components::Battery::TYPE_ID,
        create: create_battery,
    },
    FactoryEntry {
        itemtype: crate::components::Ground::TYPE_ID,
        create: create_ground,
    },
    FactoryEntry {
        itemtype: crate::components::Node::TYPE_ID,
        create: create_node,
    },
    FactoryEntry {
        itemtype: crate::components::FixedVolt::TYPE_ID,
        create: create_fixed_volt,
    },
    FactoryEntry {
        itemtype: crate::components::Rail::TYPE_ID,
        create: create_rail,
    },
    FactoryEntry {
        itemtype: crate::components::Clock::TYPE_ID,
        create: create_clock,
    },
    FactoryEntry {
        itemtype: crate::components::VoltSource::TYPE_ID,
        create: create_volt_source,
    },
    FactoryEntry {
        itemtype: crate::components::CurrSource::TYPE_ID,
        create: create_curr_source,
    },
    FactoryEntry {
        itemtype: crate::components::Switch::TYPE_ID,
        create: create_switch,
    },
    FactoryEntry {
        itemtype: crate::components::Push::TYPE_ID,
        create: create_push,
    },
    FactoryEntry {
        itemtype: crate::components::SwitchDip::TYPE_ID,
        create: create_switch_dip,
    },
    FactoryEntry {
        itemtype: crate::components::Relay::TYPE_ID,
        create: create_relay,
    },
    FactoryEntry {
        itemtype: crate::components::KeyPad::TYPE_ID,
        create: create_keypad,
    },
    FactoryEntry {
        itemtype: crate::components::Diode::TYPE_ID,
        create: create_diode,
    },
    FactoryEntry {
        itemtype: crate::components::Diode::TYPE_ID_ZENER,
        create: create_zener,
    },
    FactoryEntry {
        itemtype: crate::components::Led::TYPE_ID,
        create: create_led,
    },
    FactoryEntry {
        itemtype: crate::components::Bjt::TYPE_ID,
        create: create_bjt,
    },
    FactoryEntry {
        itemtype: crate::components::Mosfet::TYPE_ID,
        create: create_mosfet,
    },
    FactoryEntry {
        itemtype: crate::components::Jfet::TYPE_ID,
        create: create_jfet,
    },
    FactoryEntry {
        itemtype: crate::components::OpAmp::TYPE_ID,
        create: create_opamp,
    },
    FactoryEntry {
        itemtype: crate::components::Comparator::TYPE_ID,
        create: create_comparator,
    },
    FactoryEntry {
        itemtype: crate::components::VoltReg::TYPE_ID,
        create: create_volt_reg,
    },
    FactoryEntry {
        itemtype: crate::components::Probe::TYPE_ID,
        create: create_probe,
    },
    FactoryEntry {
        itemtype: crate::components::Voltmeter::TYPE_ID,
        create: create_voltmeter,
    },
    FactoryEntry {
        itemtype: crate::components::Ammeter::TYPE_ID,
        create: create_ammeter,
    },
    FactoryEntry {
        itemtype: crate::components::FreqMeter::TYPE_ID,
        create: create_freq_meter,
    },
    FactoryEntry {
        itemtype: crate::components::AndGate::TYPE_ID,
        create: create_and_gate,
    },
    FactoryEntry {
        itemtype: crate::components::OrGate::TYPE_ID,
        create: create_or_gate,
    },
    FactoryEntry {
        itemtype: crate::components::XorGate::TYPE_ID,
        create: create_xor_gate,
    },
    FactoryEntry {
        itemtype: crate::components::NotGate::TYPE_ID,
        create: create_not_gate,
    },
    FactoryEntry {
        itemtype: crate::components::BufferGate::TYPE_ID,
        create: create_buffer_gate,
    },
    FactoryEntry {
        itemtype: crate::components::FlipFlop::TYPE_ID,
        create: create_flip_flop,
    },
    FactoryEntry {
        itemtype: crate::components::Latch::TYPE_ID,
        create: create_latch,
    },
    FactoryEntry {
        itemtype: crate::components::Mux::TYPE_ID,
        create: create_mux,
    },
    FactoryEntry {
        itemtype: crate::components::Demux::TYPE_ID,
        create: create_demux,
    },
    FactoryEntry {
        itemtype: crate::components::AnalogMux::TYPE_ID,
        create: create_analog_mux,
    },
    FactoryEntry {
        itemtype: crate::components::BcdToDec::TYPE_ID,
        create: create_bcd_to_dec,
    },
    FactoryEntry {
        itemtype: crate::components::DecToBcd::TYPE_ID,
        create: create_dec_to_bcd,
    },
    FactoryEntry {
        itemtype: crate::components::BcdTo7Segment::TYPE_ID,
        create: create_bcd_to_7segment,
    },
    FactoryEntry {
        itemtype: crate::components::FullAdder::TYPE_ID,
        create: create_full_adder,
    },
    FactoryEntry {
        itemtype: crate::components::HalfAdder::TYPE_ID,
        create: create_half_adder,
    },
    FactoryEntry {
        itemtype: crate::components::TestUnit::TYPE_ID,
        create: create_test_unit,
    },
    FactoryEntry {
        itemtype: crate::components::LogicAnalyzer::TYPE_ID,
        create: create_logic_analyzer,
    },
    FactoryEntry {
        itemtype: crate::components::TouchPad::TYPE_ID,
        create: create_touchpad,
    },
    FactoryEntry {
        itemtype: crate::components::KY023::TYPE_ID,
        create: create_ky023,
    },
    FactoryEntry {
        itemtype: crate::components::KY040::TYPE_ID,
        create: create_ky040,
    },
    FactoryEntry {
        itemtype: crate::components::SR04::TYPE_ID,
        create: create_sr04,
    },
    FactoryEntry {
        itemtype: crate::components::DHT22::TYPE_ID,
        create: create_dht22,
    },
    FactoryEntry {
        itemtype: crate::components::DS18B20::TYPE_ID,
        create: create_ds18b20,
    },
    FactoryEntry {
        itemtype: crate::components::DS1621::TYPE_ID,
        create: create_ds1621,
    },
    FactoryEntry {
        itemtype: crate::components::DS1307::TYPE_ID,
        create: create_ds1307,
    },
    FactoryEntry {
        itemtype: crate::components::DcMotor::TYPE_ID,
        create: create_dc_motor,
    },
    FactoryEntry {
        itemtype: crate::components::Stepper::TYPE_ID,
        create: create_stepper,
    },
    FactoryEntry {
        itemtype: crate::components::Servo::TYPE_ID,
        create: create_servo,
    },
    FactoryEntry {
        itemtype: crate::components::SdCard::TYPE_ID,
        create: create_sd_card,
    },
    FactoryEntry {
        itemtype: crate::components::Esp01::TYPE_ID,
        create: create_esp01,
    },
    FactoryEntry {
        itemtype: crate::components::TftDisplay::TYPE_ID,
        create: create_tft_display,
    },
    FactoryEntry {
        itemtype: crate::components::Pcd8544::TYPE_ID,
        create: create_pcd8544,
    },
    FactoryEntry {
        itemtype: crate::components::Sh1107::TYPE_ID,
        create: create_sh1107,
    },
    FactoryEntry {
        itemtype: crate::components::Ks0108::TYPE_ID,
        create: create_ks0108,
    },
    FactoryEntry {
        itemtype: crate::components::Pcf8833::TYPE_ID,
        create: create_pcf8833,
    },
    FactoryEntry {
        itemtype: crate::components::Aip31068::TYPE_ID,
        create: create_aip31068,
    },
    FactoryEntry {
        itemtype: crate::components::Hd44780::TYPE_ID,
        create: create_hd44780,
    },
    FactoryEntry {
        itemtype: crate::components::Ssd1306::TYPE_ID,
        create: create_ssd1306,
    },
    FactoryEntry {
        itemtype: crate::components::SevenSegment::TYPE_ID,
        create: create_seven_segment,
    },
    FactoryEntry {
        itemtype: crate::components::SevenSegmentBCD::TYPE_ID,
        create: create_seven_segment_bcd,
    },
    FactoryEntry {
        itemtype: crate::components::LedMatrix::TYPE_ID,
        create: create_led_matrix,
    },
    FactoryEntry {
        itemtype: crate::components::Max72xx::TYPE_ID,
        create: create_max72xx,
    },
    FactoryEntry {
        itemtype: crate::components::LedBar::TYPE_ID,
        create: create_led_bar,
    },
    FactoryEntry {
        itemtype: crate::components::RgbLed::TYPE_ID,
        create: create_rgb_led,
    },
    FactoryEntry {
        itemtype: crate::components::Ws2812::TYPE_ID,
        create: create_ws2812,
    },
    FactoryEntry {
        itemtype: crate::components::Counter::TYPE_ID,
        create: create_counter,
    },
    FactoryEntry {
        itemtype: crate::components::BinCounter::TYPE_ID,
        create: create_bin_counter,
    },
    FactoryEntry {
        itemtype: crate::components::ShiftReg::TYPE_ID,
        create: create_shift_reg,
    },
    FactoryEntry {
        itemtype: crate::components::MagnitudeComp::TYPE_ID,
        create: create_magnitude_comp,
    },
    FactoryEntry {
        itemtype: crate::components::Adc::TYPE_ID,
        create: create_adc,
    },
    FactoryEntry {
        itemtype: crate::components::Dac::TYPE_ID,
        create: create_dac,
    },
    FactoryEntry {
        itemtype: crate::components::I2CToParallel::TYPE_ID,
        create: create_i2c_to_parallel,
    },
    FactoryEntry {
        itemtype: crate::components::Lm555::TYPE_ID,
        create: create_lm555,
    },
    FactoryEntry {
        itemtype: crate::components::Memory::TYPE_ID,
        create: create_memory,
    },
    FactoryEntry {
        itemtype: crate::components::DynamicMemory::TYPE_ID,
        create: create_dynamic_memory,
    },
    FactoryEntry {
        itemtype: crate::components::I2CRam::TYPE_ID,
        create: create_i2c_ram,
    },
    FactoryEntry {
        itemtype: crate::components::Function::TYPE_ID,
        create: create_function,
    },
    FactoryEntry {
        itemtype: crate::components::Transformer::TYPE_ID,
        create: create_transformer,
    },
    FactoryEntry {
        itemtype: crate::components::Scr::TYPE_ID,
        create: create_scr,
    },
    FactoryEntry {
        itemtype: crate::components::Diac::TYPE_ID,
        create: create_diac,
    },
    FactoryEntry {
        itemtype: crate::components::Triac::TYPE_ID,
        create: create_triac,
    },
    FactoryEntry {
        itemtype: crate::components::Csource::TYPE_ID,
        create: create_csource,
    },
    FactoryEntry {
        itemtype: crate::components::ResistorDip::TYPE_ID,
        create: create_resistor_dip,
    },
    FactoryEntry {
        itemtype: crate::components::Ldr::TYPE_ID,
        create: create_ldr,
    },
    FactoryEntry {
        itemtype: crate::components::Thermistor::TYPE_ID,
        create: create_thermistor,
    },
    FactoryEntry {
        itemtype: crate::components::Rtd::TYPE_ID,
        create: create_rtd,
    },
    FactoryEntry {
        itemtype: crate::components::Strain::TYPE_ID,
        create: create_strain,
    },
    FactoryEntry {
        itemtype: crate::components::AudioOut::TYPE_ID,
        create: create_audio_out,
    },
    FactoryEntry {
        itemtype: crate::components::Lamp::TYPE_ID,
        create: create_lamp,
    },
    FactoryEntry {
        itemtype: crate::components::WaveGen::TYPE_ID,
        create: create_wave_gen,
    },
    FactoryEntry {
        itemtype: crate::components::Oscope::TYPE_ID,
        create: create_oscope,
    },
    FactoryEntry {
        itemtype: crate::components::Tunnel::TYPE_ID,
        create: create_tunnel,
    },
    FactoryEntry {
        itemtype: crate::components::Bus::TYPE_ID,
        create: create_bus,
    },
    FactoryEntry {
        itemtype: crate::components::Socket::TYPE_ID,
        create: create_socket,
    },
    FactoryEntry {
        itemtype: crate::components::Header::TYPE_ID,
        create: create_header,
    },
    FactoryEntry {
        itemtype: crate::components::SerialPort::TYPE_ID,
        create: create_serial_port,
    },
    FactoryEntry {
        itemtype: crate::components::SerialTerm::TYPE_ID,
        create: create_serial_term,
    },
    FactoryEntry {
        itemtype: crate::components::Subcircuit::TYPE_ID,
        create: create_subcircuit,
    },
    FactoryEntry {
        itemtype: crate::components::SubPackage::TYPE_ID,
        create: create_sub_package,
    },
    FactoryEntry {
        itemtype: crate::components::Dial::TYPE_ID,
        create: create_dial,
    },
    FactoryEntry {
        itemtype: crate::components::Shape::TYPE_ID,
        create: create_shape,
    },
    FactoryEntry {
        itemtype: crate::components::Mcu::TYPE_ID,
        create: create_mcu,
    },
    FactoryEntry {
        itemtype: crate::components::QemuDevice::TYPE_ID,
        create: create_qemu_device,
    },
];

static ALIASES: &[FactoryEntry] = &[
    FactoryEntry {
        itemtype: "Rectangle",
        create: create_rectangle,
    },
    FactoryEntry {
        itemtype: "Ellipse",
        create: create_ellipse,
    },
    FactoryEntry {
        itemtype: "Line",
        create: create_line,
    },
    FactoryEntry {
        itemtype: "TextComponent",
        create: create_text_component,
    },
    FactoryEntry {
        itemtype: "Text",
        create: create_text_component,
    },
    FactoryEntry {
        itemtype: "Image",
        create: create_image,
    },
];

pub fn lookup_factory(itemtype: &str) -> Option<PartFactory> {
    FACTORY
        .iter()
        .chain(ALIASES.iter())
        .find(|e| e.itemtype == itemtype)
        .map(|e| e.create)
}

pub fn registered_itemtypes() -> impl Iterator<Item = &'static str> {
    FACTORY.iter().map(|e| e.itemtype)
}

pub fn create_part(itemtype: &str, circ_id: String, attrs: &[(String, String)]) -> Result<Part> {
    match lookup_factory(itemtype) {
        Some(create) => create(circ_id, attrs),
        None => Err(Error::Parse(format!("unknown itemtype '{itemtype}'"))),
    }
}

pub fn write_circuit_header(circ: &CircSettings) -> String {
    format!(
        "<circuit version=\"{FORMAT_VERSION}\" rev=\"0\" {} >\n",
        circ.header_attrs()
    )
}

pub fn write_item_line(
    itemtype: &str,
    circ_id: &str,
    attrs: &[(&str, String)],
    graphic: &GraphicAttrs,
) -> String {
    let mut out = format!("<item itemtype=\"{itemtype}\" CircId=\"{circ_id}\"");
    for (k, v) in attrs {
        out.push_str(&format!(" {k}=\"{v}\""));
    }
    out.push_str(&graphic.write_attrs());
    out.push_str(" />\n");
    out
}

pub fn write_component_item<T: Component>(
    circ_id: &str,
    part: &T,
    graphic: &GraphicAttrs,
) -> String {
    let mut attrs = Vec::new();
    for def in T::props() {
        if !def.persist {
            continue;
        }
        let val = (def.get)(part);
        attrs.push((def.id, def.format(&val)));
    }
    let attr_refs: Vec<(&str, String)> = attrs;
    write_item_line(part.type_id(), circ_id, &attr_refs, graphic)
}

pub fn write_connector(c: &ParsedConnector) -> String {
    format!(
        "<item itemtype=\"Connector\" CircId=\"{}\" startpinid=\"{}\" endpinid=\"{}\" pointList=\"{}\" />\n",
        c.id,
        c.start,
        c.end,
        crate::sim1::fmt_point_list(&c.points),
    )
}

pub fn write_circ1(
    item_lines: &[String],
    connectors: &[ParsedConnector],
    circ: &CircSettings,
) -> String {
    let mut out = write_circuit_header(circ);
    for line in item_lines {
        out.push_str(line);
    }
    for c in connectors {
        out.push_str(&write_connector(c));
    }
    out.push_str("</circuit>\n");
    out
}

/// Apply attributes by [`crate::components::PropDef`] id. Unknown optional attrs are
/// ignored (forward compat inside 3.x). Missing required attrs error.
pub fn apply_item_attrs<T: Component>(part: &mut T, attrs: &[(String, String)]) -> Result<()> {
    for def in T::props() {
        if !def.persist {
            continue;
        }
        let found = attrs
            .iter()
            .find(|(k, _)| k == def.id)
            .or_else(|| attrs.iter().find(|(k, _)| k.eq_ignore_ascii_case(def.id)))
            .or_else(|| {
                let clean_def = def.id.replace('_', "");
                attrs
                    .iter()
                    .find(|(k, _)| k.replace('_', "").eq_ignore_ascii_case(&clean_def))
            })
            .or_else(|| match def.id {
                "H_size" | "Width" => attrs
                    .iter()
                    .find(|(k, _)| matches!(k.as_str(), "H_size" | "Width" | "HSize" | "width")),
                "V_size" | "Height" => attrs
                    .iter()
                    .find(|(k, _)| matches!(k.as_str(), "V_size" | "Height" | "VSize" | "height")),
                "Embeed_bck" | "EmbedBck" => attrs.iter().find(|(k, _)| {
                    matches!(
                        k.as_str(),
                        "Embeed_bck" | "Embed_bck" | "EmbedBck" | "embed_bck" | "Embeed background"
                    )
                }),
                "Image_File" | "ImageFile" => attrs
                    .iter()
                    .find(|(k, _)| matches!(k.as_str(), "Image_File" | "ImageFile" | "image_file")),
                "BckGndData" => attrs
                    .iter()
                    .find(|(k, _)| matches!(k.as_str(), "BckGndData" | "bck_data")),
                _ => None,
            });
        match found {
            Some((_, v)) => {
                part.set_prop_text(def.id, v).map_err(prop_err)?;
            }
            None if def.required => {
                return Err(Error::Parse(format!(
                    "missing required attribute '{}'",
                    def.id
                )));
            }
            None => {}
        }
    }
    Ok(())
}

fn prop_err(e: PropError) -> Error {
    Error::Parse(e.to_string())
}

#[derive(Clone, Debug)]
pub struct ParsedCirc1Item {
    pub itemtype: String,
    pub circ_id: String,
    pub attrs: Vec<(String, String)>,
    pub graphic: GraphicAttrs,
    pub part: Part,
}

#[derive(Clone, Debug)]
pub struct ParsedCirc1 {
    pub version: String,
    pub circ: CircSettings,
    pub items: Vec<ParsedCirc1Item>,
    pub connectors: Vec<ParsedConnector>,
}

/// Parse a 1.x `.circ1` circuit. Any non-1.x `version` is refused.
pub fn parse_circ1(src: &str) -> Result<ParsedCirc1> {
    let mut version = None;
    let mut circ = CircSettings::default();
    let mut items = Vec::new();
    let mut connectors = Vec::new();
    let mut saw_circuit = false;

    for raw in src.lines() {
        let line = raw.trim();
        if line.starts_with("<circuit") {
            saw_circuit = true;
            let props = parse_xml_props(line);
            let v = if let Some(v) = prop(&props, "version") {
                require_v1(v)?;
                v.to_string()
            } else {
                FORMAT_VERSION.to_string()
            };
            version = Some(v);
            apply_circ_settings(&props, &mut circ);
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
            "Connector" => connectors.push(parse_connector(properties, connectors.len())?),
            other => {
                let circ_id = circ_id_only(&properties)?;
                let graphic = GraphicAttrs::from_attrs(&properties);
                let part = create_part(other, circ_id.clone(), &properties)?;
                items.push(ParsedCirc1Item {
                    itemtype: other.to_string(),
                    circ_id,
                    attrs: properties,
                    graphic,
                    part,
                });
            }
        }
    }

    if !saw_circuit {
        return Err(Error::Parse("missing <circuit> element".into()));
    }
    Ok(ParsedCirc1 {
        version: version.unwrap_or_else(|| FORMAT_VERSION.to_string()),
        circ,
        items,
        connectors,
    })
}

fn apply_circ_settings(props: &[(String, String)], circ: &mut CircSettings) {
    if let Some(v) = prop(props, "reaStep") {
        if let Ok(ps) = v.parse::<u64>() {
            circ.react_step_ps = ps.max(1);
        }
    }
    if let Some(v) = prop(props, "NLsteps") {
        if let Ok(n) = v.parse::<u32>() {
            circ.nl_steps = n.max(1);
        }
    }
    if let Some(v) = prop(props, "stepSize") {
        if let Ok(n) = v.parse::<u64>() {
            circ.step_size = n.max(1);
        }
    }
    if let Some(v) = prop(props, "stepsPS") {
        if let Ok(n) = v.parse::<u64>() {
            circ.steps_ps = n.max(1);
        }
    }
    if let Some(v) = prop(props, "animate") {
        circ.animate_logic = v != "0";
    }
    if let Some(v) = prop(props, "anicurr") {
        circ.animate_curr = v != "0";
    }
    if let Some(v) = prop(props, "ansi") {
        circ.ansi = v != "0";
    }
    if let Some(v) = prop(props, "width") {
        if let Ok(n) = v.parse::<i32>() {
            circ.width = n.clamp(1, 10_000);
        }
    }
    if let Some(v) = prop(props, "height") {
        if let Ok(n) = v.parse::<i32>() {
            circ.height = n.clamp(1, 10_000);
        }
    }
}

fn parse_connector(properties: Vec<(String, String)>, n: usize) -> Result<ParsedConnector> {
    let mut start = None;
    let mut end = None;
    let mut id = String::new();
    let mut points = Vec::new();
    for (k, v) in properties {
        match k.as_str() {
            "startpinid" | "start" => start = Some(v),
            "endpinid" | "end" => end = Some(v),
            "CircId" | "uid" | "id" => id = v,
            "pointList" => points = parse_point_list(&v),
            _ => {}
        }
    }
    match (start, end) {
        (Some(s), Some(e)) => {
            if id.is_empty() {
                id = format!("Connector-{}", n + 1);
            }
            Ok(ParsedConnector {
                id,
                start: s,
                end: e,
                points,
            })
        }
        _ => Err(Error::Parse(
            "Connector missing startpinid or endpinid".into(),
        )),
    }
}

fn circ_id_only(properties: &[(String, String)]) -> Result<String> {
    prop(properties, "CircId")
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Parse("item missing CircId".into()))
}

pub fn require_v1(version: &str) -> Result<()> {
    let major = version
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| Error::Parse(format!("invalid circuit version '{version}'")))?;
    if major != 1 {
        return Err(Error::Parse(format!(
            "unsupported circuit version '{version}'; expected 1.x"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::props::Harness;
    use crate::units::format_si;

    #[test]
    fn writes_1_0_0_header() {
        let xml = write_circ1(&[], &[], &CircSettings::default());
        assert!(
            xml.starts_with("<circuit version=\"1.0.0\""),
            "header was: {xml}"
        );
        assert!(xml.contains("</circuit>"));
    }

    #[test]
    fn refuses_non_1_x() {
        for v in ["2.0.0", "3.0.0", "4.0.0", "0.0.0", ""] {
            let src = format!("<circuit version=\"{v}\">\n</circuit>\n");
            let err = parse_circ1(&src).unwrap_err().to_string();
            assert!(
                err.contains("version") || err.contains("invalid"),
                "v={v} err={err}"
            );
        }
    }

    #[test]
    fn accepts_1_x() {
        parse_circ1("<circuit version=\"1.0.0\">\n</circuit>\n").unwrap();
        parse_circ1("<circuit version=\"1.1.0\">\n</circuit>\n").unwrap();
    }

    #[test]
    fn unknown_itemtype_is_error() {
        let src = r#"<circuit version="1.0.0">
<item itemtype="Banana" CircId="Banana-1" />
</circuit>
"#;
        let err = parse_circ1(src).unwrap_err().to_string();
        assert!(err.contains("unknown itemtype"), "{err}");
    }

    #[test]
    fn resistor_family_parses_via_factory() {
        let src = r#"<circuit version="1.0.0">
<item itemtype="Resistor" CircId="Resistor-1" Resistance="100 Ω" />
</circuit>
"#;
        let parsed = parse_circ1(src).unwrap();
        assert_eq!(parsed.items[0].itemtype, "Resistor");
        assert!(matches!(parsed.items[0].part, Part::Resistor(_)));
        assert_eq!(
            parsed.items[0].part.get_prop_text("Resistance").as_deref(),
            Some("100 Ω")
        );
    }

    #[test]
    fn missing_circ_id_is_error() {
        let src = r#"<circuit version="1.0.0">
<item itemtype="OpAmp" uid="opAmp-1" />
</circuit>
"#;
        let err = parse_circ1(src).unwrap_err().to_string();
        assert!(err.contains("CircId"), "{err}");
    }

    #[test]
    fn factory_registers_all_components() {
        for name in [
            "Resistor",
            "VarResistor",
            "Potentiometer",
            "Capacitor",
            "ElCapacitor",
            "Inductor",
            "VarCapacitor",
            "VarInductor",
            "Battery",
            "Ground",
            "FixedVolt",
            "Rail",
            "Clock",
            "VoltSource",
            "CurrSource",
            "Switch",
            "Push",
            "SwitchDip",
            "Relay",
            "KeyPad",
            "Diode",
            "Zener",
            "Led",
            "Bjt",
            "Mosfet",
            "Jfet",
            "OpAmp",
            "Comparator",
            "VoltReg",
            "Probe",
            "Voltmeter",
            "Ammeter",
            "FreqMeter",
            "AndGate",
            "OrGate",
            "XorGate",
            "NotGate",
            "BufferGate",
            "FlipFlop",
            "Latch",
            "Mux",
            "Demux",
            "AnalogMux",
            "BcdToDec",
            "DecToBcd",
            "BcdTo7Segment",
            "FullAdder",
            "HalfAdder",
            "TestUnit",
            "LogicAnalyzer",
            "TouchPad",
            "KY023",
            "KY040",
            "SR04",
            "DHT22",
            "DS18B20",
            "DS1621",
            "DS1307",
            "DcMotor",
            "Stepper",
            "Servo",
            "SdCard",
            "Esp01",
            "TftDisplay",
            "Pcd8544",
            "Sh1107",
            "Ks0108",
            "Pcf8833",
            "Aip31068",
            "Hd44780",
            "Ssd1306",
            "SevenSegment",
            "SevenSegmentBCD",
            "LedMatrix",
            "Max72xx",
            "LedBar",
            "RgbLed",
            "Ws2812",
            "Counter",
            "BinCounter",
            "ShiftReg",
            "MagnitudeComp",
            "Adc",
            "Dac",
            "I2CToParallel",
            "Lm555",
            "Memory",
            "DynamicMemory",
            "I2CRam",
            "Function",
            "Transformer",
            "Scr",
            "Diac",
            "Triac",
            "Csource",
            "ResistorDip",
            "Ldr",
            "Thermistor",
            "Rtd",
            "Strain",
            "AudioOut",
            "Lamp",
            "WaveGen",
            "Oscope",
            "Tunnel",
            "Bus",
            "Socket",
            "Header",
            "SerialPort",
            "SerialTerm",
            "Subcircuit",
            "SubPackage",
            "Dial",
            "Shape",
            "Rectangle",
            "Ellipse",
            "Line",
            "TextComponent",
            "Text",
            "Image",
            "Mcu",
            "QemuDevice",
        ] {
            assert!(lookup_factory(name).is_some(), "{name}");
        }
        assert!(lookup_factory("Banana").is_none());
        assert_eq!(registered_itemtypes().count(), 117);
    }

    #[test]
    fn harness_sim1_round_trip() {
        let mut h = Harness::default();
        assert_eq!(
            h.get_prop_text("Resistance").unwrap(),
            format_si(Harness::DEFAULT_R, "Ω")
        );
        h.set_prop_text("Resistance", "4.7 kΩ").unwrap();
        h.set_prop_text("Enabled", "false").unwrap();
        h.set_prop_text("Mode", "B").unwrap();
        h.set_prop_text("Note", "not saved").unwrap();

        let line = write_component_item("Harness-1", &h, &GraphicAttrs::at(10.0, 20.0));
        assert!(line.contains("itemtype=\"Harness\""));
        assert!(line.contains("CircId=\"Harness-1\""));
        assert!(line.contains("Resistance=\"4.7 kΩ\""));
        assert!(line.contains("Enabled=\"false\""));
        assert!(line.contains("Mode=\"B\""));
        assert!(!line.contains("Note="), "transient Note must not persist");
        assert!(line.contains("Pos=\"10,20\""));

        let xml = write_circ1(&[line.clone()], &[], &CircSettings::default());
        assert!(xml.starts_with("<circuit version=\"1.0.0\""));

        let mut props = parse_xml_props(line.trim());
        assert_eq!(props.remove(0), ("itemtype".into(), "Harness".into()));
        let mut loaded = Harness::default();
        apply_item_attrs(&mut loaded, &props).unwrap();
        assert_eq!(loaded.resistance, 4.7e3);
        assert!(!loaded.enabled);
        assert_eq!(loaded.mode, "B");
        assert!(loaded.note.is_empty());
    }

    #[test]
    fn missing_required_attr_is_error() {
        let mut h = Harness::default();
        let err = apply_item_attrs(&mut h, &[]).unwrap_err().to_string();
        assert!(err.contains("Resistance"), "{err}");
    }

    #[test]
    fn connector_round_trip() {
        let c = ParsedConnector {
            id: "Connector-1".into(),
            start: "R-1-lPin".into(),
            end: "R-2-rPin".into(),
            points: vec![(0.0, 0.0), (16.0, 0.0)],
        };
        let xml = write_circ1(&[], &[c.clone()], &CircSettings::default());
        let parsed = parse_circ1(&xml).unwrap();
        assert_eq!(parsed.connectors.len(), 1);
        assert_eq!(parsed.connectors[0].id, "Connector-1");
        assert_eq!(parsed.connectors[0].start, "R-1-lPin");
        assert_eq!(parsed.connectors[0].end, "R-2-rPin");
        assert_eq!(parsed.connectors[0].points, c.points);
    }

    #[test]
    fn is_graphic_attr_skips_table_props() {
        assert!(is_graphic_attr("CircId"));
        assert!(is_graphic_attr("ShowId"));
        assert!(!is_graphic_attr("Resistance"));
    }
}
