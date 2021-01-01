//! Hover tooltip formatting and pin description generators.

use crate::canvas::scene::Part;
use crate::canvas::{Canvas, Point};
use crate::digital::FlipFlopKind;
use crate::elements::pins::PinSuffix;
use crate::i18n;

impl Canvas {
    pub fn hover_tooltip(&self, scene: Point) -> Option<String> {
        fn escape(s: &str) -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        }

        if let Some(pin) = self.scene.hit_pin(scene) {
            let item = self.scene.item_by_id(&pin.item_id);
            let mut pin_text = pin_description(&pin, item);
            if pin.unused {
                let unused_str = i18n::tr("Unused");
                if !pin_text.is_empty() && !pin_text.to_lowercase().contains("unused") {
                    pin_text.push_str(&format!(" ({})", unused_str));
                } else if pin_text.is_empty() {
                    pin_text = unused_str;
                }
            }
            if self.sim_running {
                if let Some(v) = self.pin_volts.get(&pin.id) {
                    if !pin_text.is_empty() {
                        pin_text.push_str("<br>");
                    }
                    pin_text.push_str(&format!(
                        "{}: {}",
                        i18n::tr("Voltage"),
                        crate::units::format_si(*v, "V")
                    ));
                }
                if let Some(i) = self.pin_current(&pin.id) {
                    if !pin_text.is_empty() {
                        pin_text.push_str("<br>");
                    }
                    pin_text.push_str(&format!(
                        "{}: {}",
                        i18n::tr("Current"),
                        crate::units::format_si(i, "A")
                    ));
                }
            }
            let mut text = if !pin_text.is_empty() {
                format!("<b>{}:</b><br>{}<br><br>", i18n::tr("Pin"), pin_text)
            } else {
                format!("<b>{}:</b><br>{}<br><br>", i18n::tr("Pin"), escape(&pin.id))
            };
            if let Some(it) = item {
                text.push_str(&format!("<b>{}:</b><br>", i18n::tr("Component")));
                if let Some(st) = self.overload_states.get(&it.id) {
                    if st.warning || st.crashed {
                        let col = if st.crashed {
                            let (r, g, b, _) = crate::theme::ColorTheme::get_rgba(
                                crate::theme::ColorId::MsgErrorBg,
                                true,
                            );
                            format!("#{r:02x}{g:02x}{b:02x}")
                        } else {
                            let (r, g, b, _) = crate::theme::ColorTheme::get_rgba(
                                crate::theme::ColorId::MsgWarnBg,
                                true,
                            );
                            format!("#{r:02x}{g:02x}{b:02x}")
                        };
                        text.push_str(&format!(
                            "<font color=\"{}\"><b>{}</b></font><br>",
                            col,
                            escape(&i18n::tr(&st.reason))
                        ));
                    }
                }
                text.push_str(&component_hover_info(it));
            }
            return Some(text);
        }
        if let Some(widx) = self.scene.hit_wire(scene) {
            let w = &self.scene.wires()[widx];
            let mut text = format!("<b>{}:</b><br>", i18n::tr("Wire"));
            if self.sim_running {
                if let Some(i) = self.wire_currents.get(&w.id) {
                    text.push_str(&format!(
                        "{} = {}<br>",
                        i18n::tr("Current"),
                        crate::units::format_si(*i, "A")
                    ));
                }
                if let Some(v) = self.pin_volts.get(&w.start_pin) {
                    text.push_str(&format!(
                        "{} = {}",
                        i18n::tr("Voltage"),
                        crate::units::format_si(*v, "V")
                    ));
                }
            } else {
                text.push_str(&format!("{}: {}", i18n::tr("Connector"), escape(&w.id)));
            }
            return Some(text);
        }
        if let Some(idx) = self.scene.hit(scene) {
            if let Some(it) = self.scene.items().get(idx) {
                if let Some(st) = self.overload_states.get(&it.id) {
                    if st.warning || st.crashed {
                        let mut text = format!(
                            "<b>{}:</b> {} ({})<br>",
                            i18n::tr("Component"),
                            escape(&i18n::tr(it.human_name())),
                            escape(&it.id)
                        );
                        let col = if st.crashed {
                            let (r, g, b, _) = crate::theme::ColorTheme::get_rgba(
                                crate::theme::ColorId::MsgErrorBg,
                                true,
                            );
                            format!("#{r:02x}{g:02x}{b:02x}")
                        } else {
                            let (r, g, b, _) = crate::theme::ColorTheme::get_rgba(
                                crate::theme::ColorId::MsgWarnBg,
                                true,
                            );
                            format!("#{r:02x}{g:02x}{b:02x}")
                        };
                        text.push_str(&format!(
                            "<font color=\"{}\"><b>{}</b></font>",
                            col,
                            escape(&i18n::tr(&st.reason))
                        ));
                        return Some(text);
                    }
                }
                let mut text = format!("<b>{}:</b><br>", i18n::tr("Component"));
                text.push_str(&component_hover_info(it));
                return Some(text);
            }
        }
        None
    }
}

fn component_hover_info(it: &crate::canvas::scene::Item) -> String {
    let mut s = format!("{} ({})", i18n::tr(it.human_name()), it.id);
    let mut props = Vec::new();
    match &it.kind {
        Part::Resistor(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Resistance"),
                crate::units::format_si(p.resistance, "Ω")
            ));
        }
        Part::Capacitor(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Capacitance"),
                crate::units::format_si(p.capacitance, "F")
            ));
        }
        Part::ElCapacitor(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Capacitance"),
                crate::units::format_si(p.capacitance, "F")
            ));
        }
        Part::Inductor(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Inductance"),
                crate::units::format_si(p.inductance, "H")
            ));
        }
        Part::Battery(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Voltage"),
                crate::units::format_si(p.voltage, "V")
            ));
            props.push(format!(
                "{}: {}",
                i18n::tr("Resistance"),
                crate::units::format_si(p.resistance, "Ω")
            ));
        }
        Part::FixedVolt(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Voltage"),
                crate::units::format_si(p.voltage, "V")
            ));
        }
        Part::VoltReg(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Voltage"),
                crate::units::format_si(p.voltage, "V")
            ));
        }
        Part::Clock(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Voltage"),
                crate::units::format_si(p.voltage, "V")
            ));
        }
        Part::Rail(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Voltage"),
                crate::units::format_si(p.voltage, "V")
            ));
        }
        Part::Led(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Color"),
                i18n::tr(&p.color.to_string())
            ));
            props.push(format!(
                "{}: {}",
                i18n::tr("Threshold"),
                crate::units::format_si(p.threshold, "V")
            ));
            props.push(format!(
                "{}: {}",
                i18n::tr("Max Current"),
                crate::units::format_si(p.max_current, "A")
            ));
        }
        Part::WaveGen(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Wave Type"),
                i18n::tr(&p.wave_type.to_string())
            ));
            props.push(format!("{}: {} Hz", i18n::tr("Frequency"), p.freq_hz));
            props.push(format!(
                "{}: {}",
                i18n::tr("Amplitude"),
                crate::units::format_si(p.amplitude, "V")
            ));
        }
        Part::Potentiometer(p) => {
            props.push(format!(
                "{}: {}",
                i18n::tr("Resistance"),
                crate::units::format_si(p.resistance, "Ω")
            ));
            props.push(format!("{}: {:.0}%", i18n::tr("Wiper"), p.wiper * 100.0));
        }
        Part::Voltmeter(_) | Part::Ammeter(_) => {}
        _ => {}
    }
    if !props.is_empty() {
        s.push_str("<br><br>");
        s.push_str(&props.join("<br>"));
    }
    let desc = it.description();
    if !desc.is_empty() {
        s.push_str("<br><br>");
        s.push_str(&i18n::tr(desc));
    }
    s
}

fn pin_description(
    pin: &crate::canvas::pin::Pin,
    item: Option<&crate::canvas::scene::Item>,
) -> String {
    let pin_id = if let Some(it) = item {
        if let Some(rest) = pin.id.strip_prefix(&it.id) {
            rest.strip_prefix('-').unwrap_or(rest)
        } else {
            pin.id.rsplit('-').next().unwrap_or(&pin.id)
        }
    } else {
        pin.id.rsplit('-').next().unwrap_or(&pin.id)
    };
    let pin_lower = pin_id.to_ascii_lowercase();

    if let Some(it) = item {
        match &it.kind {
            Part::Mux(mux) => {
                if pin_lower == "out" {
                    return i18n::tr("Non-inverted output");
                } else if pin_lower == "out_inv" {
                    return i18n::tr("Inverted output");
                } else if pin_lower == "enable" || pin_lower == "oe" || pin_lower == "in11" {
                    return i18n::tr("Output enable (active low)");
                } else if let Some(sub) = pin_lower.strip_prefix("addr") {
                    if let Ok(idx) = sub.parse::<usize>() {
                        if idx == 0 {
                            return i18n::tr("Address select bit 0 (LSB)");
                        } else if idx == mux.addr_bits - 1 {
                            return format!("{} {idx} (MSB)", i18n::tr("Address select bit"));
                        } else {
                            return format!("{} {idx}", i18n::tr("Address select bit"));
                        }
                    }
                } else if let Some(sub) = pin_lower.strip_prefix("in") {
                    return format!("{} {sub}", i18n::tr("Data input channel"));
                }
            }
            Part::Demux(demux) => {
                if pin_lower == "in" {
                    return i18n::tr("Data input");
                } else if pin_lower == "enable" || pin_lower == "oe" || pin_lower == "in4" {
                    return i18n::tr("Output enable (active low)");
                } else if let Some(sub) = pin_lower.strip_prefix("addr") {
                    if let Ok(idx) = sub.parse::<usize>() {
                        if idx == 0 {
                            return i18n::tr("Address select bit 0 (LSB)");
                        } else if idx == demux.addr_bits - 1 {
                            return format!("{} {idx} (MSB)", i18n::tr("Address select bit"));
                        } else {
                            return format!("{} {idx}", i18n::tr("Address select bit"));
                        }
                    }
                } else if let Some(sub) = pin_lower.strip_prefix("out") {
                    return format!("{} {sub}", i18n::tr("Data output channel"));
                }
            }
            Part::AnalogMux(_) => {
                if pin_lower == "pininput" || pin_lower == "z" {
                    return i18n::tr("Common I/O terminal");
                } else if pin_lower == "pinenable" || pin_lower == "enable" || pin_lower == "en" {
                    return i18n::tr("Enable input (active low)");
                } else if let Some(sub) = pin_lower.strip_prefix("pinaddr") {
                    return format!("{} {sub}", i18n::tr("Address select bit"));
                } else if let Some(sub) = pin_lower.strip_prefix("pinchan") {
                    return format!("{} {sub} {}", i18n::tr("Channel"), i18n::tr("I/O terminal"));
                }
            }
            Part::DynamicMemory(_) => {
                if pin_lower == "ras" {
                    return i18n::tr("Row Address Strobe (active low)");
                } else if pin_lower == "cas" {
                    return i18n::tr("Column Address Strobe (active low)");
                } else if pin_lower == "we" {
                    return i18n::tr("Write Enable (active low)");
                } else if pin_lower == "oe" {
                    return i18n::tr("Output Enable (active low)");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("addr")
                    .or_else(|| pin_lower.strip_prefix('a'))
                {
                    return format!("{} {sub}", i18n::tr("Address bit"));
                } else if let Some(sub) = pin_lower
                    .strip_prefix("data")
                    .or_else(|| pin_lower.strip_prefix('d'))
                {
                    return format!("{} {sub}", i18n::tr("Data bus bit"));
                }
            }
            Part::Memory(_) => {
                if pin_lower == "cs" {
                    return i18n::tr("Chip Select (active low)");
                } else if pin_lower == "we" {
                    return i18n::tr("Write Enable (active low)");
                } else if pin_lower == "oe" {
                    return i18n::tr("Output Enable (active low)");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("addr")
                    .or_else(|| pin_lower.strip_prefix('a'))
                {
                    return format!("{} {sub}", i18n::tr("Address bit"));
                } else if let Some(sub) = pin_lower
                    .strip_prefix("data")
                    .or_else(|| pin_lower.strip_prefix('d'))
                {
                    return format!("{} {sub}", i18n::tr("Data bus bit"));
                }
            }
            Part::FlipFlop(ff) => {
                if pin_lower == "set" {
                    return i18n::tr("Asynchronous set (active low)");
                } else if pin_lower == "rst" {
                    return i18n::tr("Asynchronous reset (active low)");
                } else if pin_lower == "clk" || pin_lower == "in3" {
                    return i18n::tr("Clock input");
                } else if pin_lower == "out0" {
                    return i18n::tr("Non-inverted output (Q)");
                } else if pin_lower == "out1" {
                    return i18n::tr("Inverted output (!Q)");
                } else if pin_lower == "in0"
                    || pin_lower == "d"
                    || pin_lower == "j"
                    || pin_lower == "r"
                    || pin_lower == "t"
                {
                    match ff.ff_kind {
                        FlipFlopKind::D => return i18n::tr("Data input"),
                        FlipFlopKind::Jk => return i18n::tr("J input (set)"),
                        FlipFlopKind::Rs => return i18n::tr("Set input"),
                        FlipFlopKind::T => return i18n::tr("T input (toggle)"),
                    }
                } else if pin_lower == "in1" || pin_lower == "k" || pin_lower == "s" {
                    match ff.ff_kind {
                        FlipFlopKind::Jk => return i18n::tr("K input (reset)"),
                        FlipFlopKind::Rs => return i18n::tr("Reset input"),
                        _ => {}
                    }
                }
            }
            Part::Counter(_) => {
                if pin_lower == "clk" {
                    return i18n::tr("Clock input");
                } else if pin_lower == "rst" {
                    return i18n::tr("Reset (active low)");
                } else if pin_lower == "set" {
                    return i18n::tr("Set output High (active low)");
                } else if pin_lower == "out" {
                    return i18n::tr("Counter output");
                }
            }
            Part::BinCounter(_) => {
                if pin_lower == "clk" {
                    return i18n::tr("Clock input");
                } else if pin_lower == "rst" {
                    return i18n::tr("Reset (active low)");
                } else if pin_lower == "dir" {
                    return i18n::tr("Count direction (up/down) select");
                } else if pin_lower == "ld" {
                    return i18n::tr("Parallel load");
                } else if pin_lower == "rco" {
                    return i18n::tr("Ripple carry out");
                } else if pin_lower == "rbo" {
                    return i18n::tr("Ripple borrow out");
                } else if let Some(sub) = pin_lower.strip_prefix("out") {
                    return format!("{} {sub}", i18n::tr("Counter output bit"));
                } else if let Some(sub) = pin_lower.strip_prefix("in") {
                    return format!("{} {sub}", i18n::tr("Parallel data input bit"));
                }
            }
            Part::Latch(_) => {
                if pin_lower == "oe" || pin_lower == "pin_outenable" {
                    return i18n::tr("Output enable (tristate control)");
                } else if pin_lower == "clk" || pin_lower == "pin_clock" || pin_lower == "le" {
                    return i18n::tr("Clock / enable input");
                } else if pin_lower == "rst" || pin_lower == "pin_reset" {
                    return i18n::tr("Reset (active low)");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("in")
                    .or_else(|| pin_lower.strip_prefix('d'))
                {
                    return format!("{} {sub}", i18n::tr("Data input bit"));
                } else if let Some(sub) = pin_lower
                    .strip_prefix("out")
                    .or_else(|| pin_lower.strip_prefix('q'))
                {
                    return format!("{} {sub}", i18n::tr("Output bit"));
                }
            }
            Part::ShiftReg(_) => {
                if pin_lower == "oe" || pin_lower == "in3" || pin_lower == "pin_outenable" {
                    return i18n::tr("Output enable (tristate control)");
                } else if pin_lower == "rst" || pin_lower == "mr" || pin_lower == "in2" {
                    return i18n::tr("Reset (active low)");
                } else if pin_lower == "ser" || pin_lower == "in4" {
                    return i18n::tr("Shift/load select");
                } else if pin_lower == "din" || pin_lower == "ds" || pin_lower == "in0" {
                    return i18n::tr("Serial data input");
                } else if pin_lower == "dil" || pin_lower == "in5" {
                    return i18n::tr("Parallel load data input");
                } else if pin_lower == "clk" || pin_lower == "sh_cp" || pin_lower == "in1" {
                    return i18n::tr("Clock input");
                } else if pin_lower == "st_cp" {
                    return i18n::tr("Storage register clock");
                } else if pin_lower == "dir" || pin_lower == "in6" {
                    return i18n::tr("Shift direction select");
                } else if pin_lower == "q7s" || pin_lower == "dout" {
                    return i18n::tr("Serial data output (Q7S)");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("out")
                    .or_else(|| pin_lower.strip_prefix('q'))
                {
                    return format!("{} {sub}", i18n::tr("Output bit"));
                }
            }
            Part::BcdTo7Segment(_) => {
                if pin_lower == "rst" {
                    return i18n::tr("Reset (active low)");
                } else if pin_lower == "enable" || pin_lower == "oe" || pin_lower == "in4" {
                    return i18n::tr("Output enable (active low)");
                } else if let Some(sub) = pin_lower.strip_prefix("in") {
                    if let Ok(idx) = sub.parse::<usize>() {
                        let msb = if idx == 3 {
                            " (MSB)"
                        } else if idx == 0 {
                            " (LSB)"
                        } else {
                            ""
                        };
                        return format!("{} {idx}{msb}", i18n::tr("BCD input bit"));
                    }
                } else if let Some(sub) = pin_lower.strip_prefix("out") {
                    if let Ok(idx) = sub.parse::<u8>() {
                        let seg = (b'a' + idx) as char;
                        return format!("{} {seg} {}", i18n::tr("Segment"), i18n::tr("output"));
                    }
                }
            }
            Part::BcdToDec(_) => {
                if pin_lower == "enable" || pin_lower == "oe" || pin_lower == "in4" {
                    return i18n::tr("Output enable (active low)");
                } else if let Some(sub) = pin_lower.strip_prefix("in") {
                    if let Ok(idx) = sub.parse::<usize>() {
                        let msb = if idx == 3 {
                            " (MSB)"
                        } else if idx == 0 {
                            " (LSB)"
                        } else {
                            ""
                        };
                        return format!("{} {idx}{msb}", i18n::tr("BCD input bit"));
                    }
                } else if let Some(sub) = pin_lower.strip_prefix("out") {
                    return format!("{} {sub}", i18n::tr("Decimal output"));
                }
            }
            Part::DecToBcd(_) => {
                if pin_lower == "enable" || pin_lower == "oe" || pin_lower == "in15" {
                    return i18n::tr("Output enable (active low)");
                } else if let Some(sub) = pin_lower.strip_prefix("in") {
                    return format!("{} {sub}", i18n::tr("Decimal input"));
                } else if let Some(sub) = pin_lower.strip_prefix("out") {
                    if let Ok(idx) = sub.parse::<usize>() {
                        let weights = ["8 (MSB)", "4", "2", "1 (LSB)"];
                        let w = weights.get(idx).unwrap_or(&"");
                        return format!("{}, {} {w}", i18n::tr("BCD output"), i18n::tr("weight"));
                    }
                }
            }
            Part::FullAdder(_) | Part::HalfAdder(_) => {
                if pin_lower == "ci" {
                    return i18n::tr("Carry in (Ci)");
                } else if pin_lower == "co" {
                    return i18n::tr("Carry out (Co)");
                } else if pin_lower == "a" || pin_lower == "in0" {
                    return i18n::tr("Operand input A");
                } else if pin_lower == "b" || pin_lower == "in1" {
                    return i18n::tr("Operand input B");
                } else if pin_lower == "s" || pin_lower == "out0" {
                    return i18n::tr("Sum output (S)");
                }
            }
            Part::MagnitudeComp(_) => {
                if pin_lower == "in_gt" {
                    return i18n::tr("Cascade input: previous stage result (A>B)");
                } else if pin_lower == "in_eq" {
                    return i18n::tr("Cascade input: previous stage result (A=B)");
                } else if pin_lower == "in_lt" {
                    return i18n::tr("Cascade input: previous stage result (A<B)");
                } else if pin_lower == "out_gt" {
                    return i18n::tr("Comparison output (A>B)");
                } else if pin_lower == "out_eq" {
                    return i18n::tr("Comparison output (A=B)");
                } else if pin_lower == "out_lt" {
                    return i18n::tr("Comparison output (A<B)");
                } else if let Some(sub) = pin_lower.strip_prefix('a') {
                    return format!("{} {sub}", i18n::tr("Operand A bit"));
                } else if let Some(sub) = pin_lower.strip_prefix('b') {
                    return format!("{} {sub}", i18n::tr("Operand B bit"));
                }
            }
            Part::Lm555(_) => {
                if pin_lower == "epin0" || pin_lower == "gnd" {
                    return i18n::tr("Ground");
                } else if pin_lower == "epin1" || pin_lower == "trg" || pin_lower == "trigger" {
                    return i18n::tr("Trigger input");
                } else if pin_lower == "epin2" || pin_lower == "out" || pin_lower == "output" {
                    return i18n::tr("Output");
                } else if pin_lower == "epin3" || pin_lower == "rst" || pin_lower == "reset" {
                    return i18n::tr("Reset input (active low)");
                } else if pin_lower == "epin4" || pin_lower == "ctl" || pin_lower == "control" {
                    return i18n::tr("Control voltage");
                } else if pin_lower == "epin5" || pin_lower == "thr" || pin_lower == "threshold" {
                    return i18n::tr("Threshold input");
                } else if pin_lower == "epin6" || pin_lower == "dis" || pin_lower == "discharge" {
                    return i18n::tr("Discharge terminal");
                } else if pin_lower == "epin7" || pin_lower == "vcc" {
                    return i18n::tr("Positive supply voltage");
                }
            }
            Part::Potentiometer(_) => {
                if pin_lower == "lpin" || pin_lower == "a" {
                    return i18n::tr("Terminal A");
                } else if pin_lower == "mpin" || pin_lower == "wpin" || pin_lower == "wiper" {
                    return i18n::tr("Wiper");
                } else if pin_lower == "rpin" || pin_lower == "b" {
                    return i18n::tr("Terminal B");
                }
            }
            Part::Diac(_) => {
                if pin_lower == "lpin" || pin_lower == "pin0" {
                    return i18n::tr("Terminal A1 (bidirectional, no polarity)");
                } else if pin_lower == "rpin" || pin_lower == "pin1" {
                    return i18n::tr("Terminal A2 (bidirectional, no polarity)");
                }
            }
            Part::Triac(_) => {
                if pin_lower == "lpin" || pin_lower == "mt1" {
                    return i18n::tr("Main Terminal 1 (MT1)");
                } else if pin_lower == "rpin" || pin_lower == "mt2" {
                    return i18n::tr("Main Terminal 2 (MT2)");
                } else if pin_lower == "gpin" || pin_lower == "gate" {
                    return i18n::tr("Gate");
                }
            }
            Part::Scr(_) => {
                if pin_lower == "lpin" || pin_lower == "anode" {
                    return i18n::tr("Anode");
                } else if pin_lower == "rpin" || pin_lower == "cathode" {
                    return i18n::tr("Cathode");
                } else if pin_lower == "gpin" || pin_lower == "gate" {
                    return i18n::tr("Gate");
                }
            }
            Part::VoltReg(_) => {
                if pin_lower == "inpin" || pin_lower == "vin" || pin_lower == "pin0" {
                    return i18n::tr("Input voltage");
                } else if pin_lower == "outpin" || pin_lower == "vout" || pin_lower == "pin1" {
                    return i18n::tr("Regulated output voltage");
                } else if pin_lower == "gndpin" || pin_lower == "gnd" || pin_lower == "pin2" {
                    return i18n::tr("Ground / reference");
                }
            }
            Part::DcMotor(_) => {
                if pin_lower == "lpin" || pin_lower == "pin0" {
                    return i18n::tr("Motor terminal +");
                } else if pin_lower == "rpin" || pin_lower == "pin1" {
                    return i18n::tr("Motor terminal -");
                }
            }
            Part::Stepper(_) => {
                if pin_lower == "a1" {
                    return i18n::tr("Coil A terminal +");
                } else if pin_lower == "a2" {
                    return i18n::tr("Coil A terminal -");
                } else if pin_lower == "b1" {
                    return i18n::tr("Coil B terminal +");
                } else if pin_lower == "b2" {
                    return i18n::tr("Coil B terminal -");
                } else if pin_lower == "co" {
                    return i18n::tr("Common terminal");
                }
            }
            Part::Servo(_) => {
                if pin_lower == "pos" || pin_lower == "vcc" || pin_lower == "in0" {
                    return i18n::tr("Positive supply");
                } else if pin_lower == "gnd" || pin_lower == "in1" {
                    return i18n::tr("Ground");
                } else if pin_lower == "sig" || pin_lower == "pwm" || pin_lower == "in2" {
                    return i18n::tr("PWM control signal");
                }
            }
            Part::Tunnel(_) => {
                return i18n::tr("Wirelessly connected to every other Tunnel sharing this name");
            }
            Part::SR04(_) => {
                if pin_lower == "inpin" || pin_lower == "in v=m" {
                    return "Simulated distance input (voltage represents the distance in meters)"
                        .to_string();
                } else if pin_lower == "vccpin" || pin_lower == "vcc" {
                    return i18n::tr("Positive supply voltage (Vcc)");
                } else if pin_lower == "trigpin" || pin_lower == "trig" {
                    return i18n::tr("Trigger pulse input");
                } else if pin_lower == "outpin" || pin_lower == "echo" {
                    return i18n::tr("Echo pulse output");
                } else if pin_lower == "gndpin" || pin_lower == "gnd" {
                    return i18n::tr("Ground");
                }
            }
            Part::DHT22(_) => {
                if pin_lower == "vccpin" || pin_lower == "vcc" {
                    return i18n::tr("Positive supply voltage (Vcc)");
                } else if pin_lower == "inpin" || pin_lower == "data" {
                    return i18n::tr("Single-wire data line (temperature & humidity)");
                } else if pin_lower == "ncpin" || pin_lower == "nc" {
                    return i18n::tr("Not connected");
                } else if pin_lower == "gdnpin" || pin_lower == "gndpin" || pin_lower == "gnd" {
                    return i18n::tr("Ground");
                }
            }
            Part::DS18B20(_) => {
                if pin_lower == "gndpin" || pin_lower == "gnd" {
                    return i18n::tr("Ground");
                } else if pin_lower == "inpin" || pin_lower == "dq" {
                    return i18n::tr("1-Wire data line (temperature output)");
                } else if pin_lower == "vddpin" || pin_lower == "vdd" {
                    return i18n::tr("Positive supply voltage (Vdd)");
                }
            }
            Part::DS1621(_) => {
                if pin_lower == "inpin0" || pin_lower == "sda" {
                    return i18n::tr("I2C data line (SDA)");
                } else if pin_lower == "inpin1" || pin_lower == "scl" {
                    return i18n::tr("I2C clock line (SCL)");
                } else if pin_lower == "outpin0" || pin_lower == "tout" {
                    return i18n::tr("Thermostat output (Tout)");
                } else if pin_lower == "pingnd" || pin_lower == "gnd" {
                    return i18n::tr("Ground");
                } else if pin_lower == "pinvdd" || pin_lower == "vdd" {
                    return i18n::tr("Positive supply voltage (Vdd)");
                } else if pin_lower == "inpin2" || pin_lower == "a0" {
                    return i18n::tr("I2C address bit A0");
                } else if pin_lower == "inpin3" || pin_lower == "a1" {
                    return i18n::tr("I2C address bit A1");
                } else if pin_lower == "inpin4" || pin_lower == "a2" {
                    return i18n::tr("I2C address bit A2");
                }
            }
            Part::DS1307(_) => {
                if pin_lower == "pinsda" || pin_lower == "sda" {
                    return i18n::tr("I2C data line (SDA)");
                } else if pin_lower == "pinscl" || pin_lower == "scl" {
                    return i18n::tr("I2C clock line (SCL)");
                } else if pin_lower == "pinsqw" || pin_lower == "sqw" {
                    return i18n::tr("Square-wave / interrupt output (SQW/OUT)");
                }
            }
            Part::TouchPad(_) => {
                if pin_lower == "vrx_p" || pin_lower == "xp" {
                    return i18n::tr("Touchscreen X+ terminal");
                } else if pin_lower == "vrx_m" || pin_lower == "xm" {
                    return i18n::tr("Touchscreen X− terminal");
                } else if pin_lower == "vry_p" || pin_lower == "yp" {
                    return i18n::tr("Touchscreen Y+ terminal");
                } else if pin_lower == "vry_m" || pin_lower == "ym" {
                    return i18n::tr("Touchscreen Y− terminal");
                }
            }
            Part::KY023(_) => {
                if pin_lower == "vrx" {
                    return i18n::tr("X-axis analog output");
                } else if pin_lower == "vry" {
                    return i18n::tr("Y-axis analog output");
                } else if pin_lower == "sw" {
                    return i18n::tr("Joystick button (active low)");
                }
            }
            Part::KY040(_) => {
                if pin_lower == "clk" {
                    return i18n::tr("Encoder clock output (CLK)");
                } else if pin_lower == "dt" {
                    return i18n::tr("Encoder data output / direction (DT)");
                } else if pin_lower == "sw" {
                    return i18n::tr("Push-button switch (active low)");
                }
            }
            Part::SdCard(_) => {
                if pin_lower == "pincs" || pin_lower == "cs" {
                    return i18n::tr("SPI chip select (active low)");
                } else if pin_lower == "pindi" || pin_lower == "di" || pin_lower == "mosi" {
                    return i18n::tr("SPI data in (MOSI)");
                } else if pin_lower == "pinck" || pin_lower == "ck" || pin_lower == "sck" {
                    return i18n::tr("SPI clock");
                } else if pin_lower == "pindo" || pin_lower == "do" || pin_lower == "miso" {
                    return i18n::tr("SPI data out (MISO)");
                }
            }
            Part::Esp01(_) => {
                if pin_lower == "pin0" || pin_lower == "tx" {
                    return i18n::tr("Serial transmit (TX)");
                } else if pin_lower == "pin1" || pin_lower == "rx" {
                    return i18n::tr("Serial receive (RX)");
                }
            }
            Part::TftDisplay(_) => {
                if pin_lower == "pindc" || pin_lower == "dc" {
                    return i18n::tr("Data/Command select");
                } else if pin_lower == "pincs" || pin_lower == "cs" {
                    return i18n::tr("SPI chip select (active low)");
                } else if pin_lower == "pindi" || pin_lower == "di" || pin_lower == "mosi" {
                    return i18n::tr("SPI data in (MOSI)");
                } else if pin_lower == "pinck" || pin_lower == "ck" || pin_lower == "sck" {
                    return i18n::tr("SPI clock");
                } else if pin_lower == "pinrs" || pin_lower == "rst" || pin_lower == "reset" {
                    return i18n::tr("Hardware reset (active low)");
                } else if pin_lower == "pindo" || pin_lower == "do" || pin_lower == "miso" {
                    return i18n::tr("SPI data out (MISO)");
                }
            }
            Part::Pcd8544(_) => {
                if pin_lower == "pindc" || pin_lower == "dc" {
                    return i18n::tr("Data/Command select");
                } else if pin_lower == "pincs" || pin_lower == "cs" {
                    return i18n::tr("SPI chip select (active low)");
                } else if pin_lower == "pinsi"
                    || pin_lower == "pindi"
                    || pin_lower == "di"
                    || pin_lower == "din"
                {
                    return i18n::tr("SPI data in");
                } else if pin_lower == "pinscl"
                    || pin_lower == "pinck"
                    || pin_lower == "ck"
                    || pin_lower == "clk"
                {
                    return i18n::tr("SPI clock");
                } else if pin_lower == "pinrst" || pin_lower == "pinrs" || pin_lower == "rst" {
                    return i18n::tr("Hardware reset (active low)");
                }
            }
            Part::Sh1107(_) | Part::Aip31068(_) => {
                if pin_lower == "pinsda" || pin_lower == "sda" {
                    return i18n::tr("I2C data line (SDA)");
                } else if pin_lower == "pinsck"
                    || pin_lower == "pinscl"
                    || pin_lower == "scl"
                    || pin_lower == "sck"
                {
                    return i18n::tr("I2C clock line (SCL)");
                }
            }
            Part::Ks0108(_) => {
                if let Some(sub) = pin_lower
                    .strip_prefix("datapin")
                    .or_else(|| pin_lower.strip_prefix("pindb"))
                {
                    return format!("{} D{sub}", i18n::tr("Data bus bit"));
                } else if pin_lower == "pincs1" || pin_lower == "cs1" {
                    return i18n::tr("Chip select 1 (left half)");
                } else if pin_lower == "pincs2" || pin_lower == "cs2" {
                    return i18n::tr("Chip select 2 (right half)");
                } else if pin_lower == "pinrs"
                    || pin_lower == "pindc"
                    || pin_lower == "rs"
                    || pin_lower == "dc"
                {
                    return i18n::tr("Register select (instruction / data)");
                } else if pin_lower == "pinrw" || pin_lower == "rw" {
                    return i18n::tr("Read/Write select");
                } else if pin_lower == "pinen" || pin_lower == "en" || pin_lower == "e" {
                    return i18n::tr("Enable strobe");
                } else if pin_lower == "pinrst" || pin_lower == "rst" {
                    return i18n::tr("Hardware reset (active low)");
                }
            }
            Part::Pcf8833(_) => {
                if pin_lower == "pincs" || pin_lower == "cs" {
                    return i18n::tr("SPI chip select (active low)");
                } else if pin_lower == "pinclk" || pin_lower == "clk" || pin_lower == "sck" {
                    return i18n::tr("SPI clock");
                } else if pin_lower == "pindata" || pin_lower == "data" || pin_lower == "mosi" {
                    return i18n::tr("SPI data in (MOSI)");
                } else if pin_lower == "pinreset" || pin_lower == "rst" || pin_lower == "reset" {
                    return i18n::tr("Hardware reset (active low)");
                }
            }
            Part::AudioOut(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Speaker (+) / Left channel");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Speaker (−) / Right channel or ground");
                }
            }
            Part::Transformer(_) => {
                if pin_lower == "p1" {
                    return i18n::tr("Primary winding terminal 1");
                } else if pin_lower == "p2" {
                    return i18n::tr("Primary winding terminal 2");
                } else if pin_lower == "s1" {
                    return i18n::tr("Secondary winding terminal 1");
                } else if pin_lower == "s2" {
                    return i18n::tr("Secondary winding terminal 2");
                }
            }
            Part::RgbLed(rgb) => {
                if pin_lower == "rpin" {
                    return i18n::tr("Red LED anode / cathode");
                } else if pin_lower == "gpin" {
                    return i18n::tr("Green LED anode / cathode");
                } else if pin_lower == "bpin" {
                    return i18n::tr("Blue LED anode / cathode");
                } else if pin_lower == "cpin" {
                    if rgb.common_anode {
                        return i18n::tr("Common anode (+)");
                    } else {
                        return i18n::tr("Common cathode (−)");
                    }
                }
            }
            Part::Relay(relay) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Coil terminal +");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Coil terminal −");
                } else if pin_lower == "c1" {
                    return i18n::tr("Switch common (C)");
                } else if pin_lower == "c2" {
                    if relay.double_throw {
                        return i18n::tr("Switch normally-closed contact (NC)");
                    } else {
                        return i18n::tr("Switch normally-open contact (NO)");
                    }
                }
            }
            Part::SerialPort(_) => {
                if pin_lower == "rx" {
                    return i18n::tr("Serial receive (RX)");
                } else if pin_lower == "tx" {
                    return i18n::tr("Serial transmit (TX)");
                } else if pin_lower == "gnd" {
                    return i18n::tr("Ground");
                } else if pin_lower == "vcc" {
                    return i18n::tr("Positive supply voltage (Vcc)");
                }
            }
            Part::SerialTerm(_) => {
                if pin_lower == "rx" {
                    return i18n::tr("Serial receive (RX)");
                } else if pin_lower == "tx" {
                    return i18n::tr("Serial transmit (TX)");
                }
            }
            Part::Probe(_) => {
                if pin_lower == "inpin" {
                    return i18n::tr("Voltage probe input");
                }
            }
            Part::Voltmeter(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Measurement terminal + (positive)");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Measurement terminal − (negative / reference)");
                } else if pin_lower == "outnod" {
                    return i18n::tr("Analog output proportional to measured voltage");
                }
            }
            Part::Ammeter(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Current in (positive terminal)");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Current out (negative terminal)");
                } else if pin_lower == "outnod" {
                    return i18n::tr("Analog output proportional to measured current");
                }
            }
            Part::FreqMeter(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Signal input");
                }
            }
            Part::Oscope(_) | Part::LogicAnalyzer(_) => {
                if let Some(sub) = pin_lower.strip_prefix("pin") {
                    if sub == "g" {
                        return i18n::tr("Ground reference");
                    } else if let Ok(idx) = sub.parse::<usize>() {
                        return format!(
                            "{} {} {}",
                            i18n::tr("Channel"),
                            idx + 1,
                            i18n::tr("input")
                        );
                    }
                }
            }
            Part::Clock(_) => {
                if pin_lower == "outnod" {
                    return i18n::tr("Clock signal output");
                }
            }
            Part::Rail(_) => {
                if pin_lower == "outnod" {
                    return i18n::tr("Voltage rail output");
                }
            }
            Part::WaveGen(wg) => {
                if pin_lower == "outnod" {
                    return i18n::tr("Waveform signal output");
                } else if pin_lower == "gndnod" {
                    if wg.bipolar {
                        return i18n::tr("Waveform reference ground (bipolar mode)");
                    } else {
                        return i18n::tr("Waveform reference ground");
                    }
                }
            }
            Part::VoltSource(_) => {
                if pin_lower == "outpin" {
                    return i18n::tr("Adjustable voltage output");
                }
            }
            Part::CurrSource(_) => {
                if pin_lower == "outpin" {
                    return i18n::tr("Adjustable current source output");
                }
            }
            Part::Csource(cs) => {
                if pin_lower == "cppin" {
                    return i18n::tr("Control input + (positive)");
                } else if pin_lower == "cmpin" {
                    return i18n::tr("Control input − (negative)");
                } else if pin_lower == "s1pin" {
                    if cs.curr_source {
                        return i18n::tr("Current source terminal +");
                    } else {
                        return i18n::tr("Voltage source terminal +");
                    }
                } else if pin_lower == "s2pin" {
                    if cs.curr_source {
                        return i18n::tr("Current source terminal −");
                    } else {
                        return i18n::tr("Voltage source terminal −");
                    }
                }
            }
            Part::Lamp(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Lamp terminal 1");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Lamp terminal 2");
                }
            }
            Part::Switch(sw) => {
                if let Some(sub) = pin_lower.strip_prefix("lpin") {
                    if sw.double_throw {
                        return format!(
                            "{} {} {}",
                            i18n::tr("Switch pole"),
                            sub,
                            i18n::tr("common terminal (COM)")
                        );
                    } else if sw.poles > 1 {
                        return format!(
                            "{} {} – {}",
                            i18n::tr("Switch pole"),
                            sub,
                            i18n::tr("terminal A")
                        );
                    } else {
                        return i18n::tr("Switch terminal A");
                    }
                } else if let Some(sub) = pin_lower.strip_prefix("rpin") {
                    if sw.double_throw {
                        return format!(
                            "{} {} {}",
                            i18n::tr("Switch pole"),
                            sub,
                            i18n::tr("contact")
                        );
                    } else if sw.poles > 1 {
                        return format!(
                            "{} {} – {}",
                            i18n::tr("Switch pole"),
                            sub,
                            i18n::tr("terminal B")
                        );
                    } else {
                        return i18n::tr("Switch terminal B");
                    }
                } else if let Some(rest) = pin_lower.strip_prefix("switch") {
                    if let Some(idx_str) = rest.strip_suffix("pinn") {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if sw.double_throw {
                                let pole = idx / 2;
                                let throw = idx % 2;
                                let label = if throw == 0 { "NC" } else { "NO" };
                                if sw.poles > 1 {
                                    return format!(
                                        "{} {pole}, {} {throw} ({label})",
                                        i18n::tr("Pole"),
                                        i18n::tr("throw")
                                    );
                                } else {
                                    return format!(
                                        "{} {throw} ({label}) {}",
                                        i18n::tr("Throw"),
                                        i18n::tr("contact")
                                    );
                                }
                            } else if sw.poles > 1 {
                                return format!(
                                    "{} {idx} – {}",
                                    i18n::tr("Pole"),
                                    i18n::tr("terminal B")
                                );
                            } else {
                                return i18n::tr("Switch terminal B");
                            }
                        }
                    }
                    return i18n::tr("Switch contact");
                } else if let Some(rest) = pin_lower.strip_prefix("pinp") {
                    if let Ok(pole) = rest.parse::<usize>() {
                        if sw.double_throw {
                            if sw.poles > 1 {
                                return format!(
                                    "{} {pole} {}",
                                    i18n::tr("Pole"),
                                    i18n::tr("common terminal (COM)")
                                );
                            } else {
                                return i18n::tr("Switch common terminal (COM)");
                            }
                        } else if sw.poles > 1 {
                            return format!(
                                "{} {pole} – {}",
                                i18n::tr("Pole"),
                                i18n::tr("terminal A")
                            );
                        } else {
                            return i18n::tr("Switch terminal A");
                        }
                    }
                    if sw.double_throw {
                        return i18n::tr("Switch common terminal (COM)");
                    } else {
                        return i18n::tr("Switch terminal A");
                    }
                } else if pin_lower.contains("pinn") || pin_lower == "switch0pinn" {
                    return i18n::tr("Switch terminal B");
                } else if pin_lower.contains("pinp") || pin_lower == "pinp0" {
                    if sw.double_throw {
                        return i18n::tr("Switch common terminal (COM)");
                    } else {
                        return i18n::tr("Switch terminal A");
                    }
                }
            }
            Part::Push(pu) => {
                if let Some(sub) = pin_lower.strip_prefix("lpin") {
                    if pu.poles > 1 {
                        return format!(
                            "{} {} – {}",
                            i18n::tr("Switch pole"),
                            sub,
                            i18n::tr("terminal A")
                        );
                    } else {
                        return i18n::tr("Switch terminal A");
                    }
                } else if let Some(sub) = pin_lower.strip_prefix("rpin") {
                    if pu.poles > 1 {
                        return format!(
                            "{} {} – {}",
                            i18n::tr("Switch pole"),
                            sub,
                            i18n::tr("terminal B")
                        );
                    } else {
                        return i18n::tr("Switch terminal B");
                    }
                } else if pin_lower.contains("pinn") || pin_lower == "switch0pinn" {
                    return i18n::tr("Switch normally-open contact");
                } else if pin_lower == "pinp" || pin_lower == "pinp0" {
                    return i18n::tr("Switch common contact");
                }
            }
            Part::Ground(_) => {
                if pin_lower == "gnd" {
                    return i18n::tr("Ground / 0 V reference");
                }
            }
            Part::FixedVolt(fv) => {
                if pin_lower == "outnod" {
                    return format!(
                        "{} {:.3} V {}",
                        i18n::tr("Fixed"),
                        fv.voltage,
                        i18n::tr("output")
                    );
                }
            }
            Part::Resistor(_)
            | Part::VarResistor(_)
            | Part::Ldr(_)
            | Part::Thermistor(_)
            | Part::Rtd(_)
            | Part::Strain(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Terminal 1");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Terminal 2");
                }
            }
            Part::Capacitor(_) | Part::VarCapacitor(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Terminal 1");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Terminal 2");
                }
            }
            Part::ElCapacitor(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("+ (Positive terminal)");
                } else if pin_lower == "rpin" {
                    return i18n::tr("− (Negative terminal)");
                }
            }
            Part::Inductor(_) | Part::VarInductor(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Terminal 1");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Terminal 2");
                }
            }
            Part::Battery(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Positive terminal (+)");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Negative terminal (−)");
                }
            }
            Part::Bjt(bjt) => {
                if pin_lower == "collector" {
                    return if bjt.pnp {
                        "Collector (PNP)"
                    } else {
                        "Collector (NPN)"
                    }
                    .to_string();
                } else if pin_lower == "emiter" {
                    return if bjt.pnp {
                        "Emitter (PNP)"
                    } else {
                        "Emitter (NPN)"
                    }
                    .to_string();
                } else if pin_lower == "base" {
                    return i18n::tr("Base");
                }
            }
            Part::Mosfet(mos) => {
                if pin_lower == "dren" {
                    return if mos.p_channel {
                        "Drain (P-channel)"
                    } else {
                        "Drain (N-channel)"
                    }
                    .to_string();
                } else if pin_lower == "sour" {
                    return if mos.p_channel {
                        "Source (P-channel)"
                    } else {
                        "Source (N-channel)"
                    }
                    .to_string();
                } else if pin_lower == "gate" {
                    return i18n::tr("Gate");
                }
            }
            Part::Jfet(jfet) => {
                if pin_lower == "dren" {
                    return if jfet.p_channel {
                        "Drain (P-channel JFET)"
                    } else {
                        "Drain (N-channel JFET)"
                    }
                    .to_string();
                } else if pin_lower == "sour" {
                    return if jfet.p_channel {
                        "Source (P-channel JFET)"
                    } else {
                        "Source (N-channel JFET)"
                    }
                    .to_string();
                } else if pin_lower == "gate" {
                    return i18n::tr("Gate");
                }
            }
            Part::OpAmp(_) => {
                if pin_lower == "inputninv" {
                    return i18n::tr("Non-inverting input (+)");
                } else if pin_lower == "inputinv" {
                    return i18n::tr("Inverting input (−)");
                } else if pin_lower == "output" {
                    return i18n::tr("Output");
                } else if pin_lower == "powerpos" {
                    return i18n::tr("Positive supply voltage (V+)");
                } else if pin_lower == "powerneg" {
                    return i18n::tr("Negative supply voltage (V−)");
                }
            }
            Part::Comparator(comp) => {
                if pin_lower == "in0" {
                    return if comp.inverted {
                        "Inverting input (−)"
                    } else {
                        "Non-inverting input (+)"
                    }
                    .to_string();
                } else if pin_lower == "in1" {
                    return if comp.inverted {
                        "Non-inverting input (+)"
                    } else {
                        "Inverting input (−)"
                    }
                    .to_string();
                } else if pin_lower == "out" {
                    return i18n::tr("Comparator output");
                }
            }
            Part::Diode(diode) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Anode");
                } else if pin_lower == "rpin" {
                    if diode.zener {
                        return i18n::tr("Cathode (Zener)");
                    } else {
                        return i18n::tr("Cathode");
                    }
                }
            }
            Part::Led(_) => {
                if pin_lower == "lpin" {
                    return i18n::tr("Anode (+)");
                } else if pin_lower == "rpin" {
                    return i18n::tr("Cathode (−)");
                }
            }
            Part::SevenSegment(ss) => {
                let seg_name = match pin_lower.as_str() {
                    "a" => Some("segment A (top)"),
                    "b" => Some("segment B (upper right)"),
                    "c" => Some("segment C (lower right)"),
                    "d" => Some("segment D (bottom)"),
                    "e" => Some("segment E (lower left)"),
                    "f" => Some("segment F (upper left)"),
                    "g" => Some("segment G (middle)"),
                    "dp" => Some("decimal point"),
                    _ => None,
                };
                if let Some(name) = seg_name {
                    if ss.common_anode {
                        return format!("LED {name} {}", i18n::tr("cathode"));
                    } else {
                        return format!("LED {name} {}", i18n::tr("anode"));
                    }
                } else if pin_lower == "com" {
                    if ss.common_anode {
                        return i18n::tr("Common anode (+)");
                    } else {
                        return i18n::tr("Common cathode (−)");
                    }
                }
            }
            Part::SevenSegmentBCD(_) => {
                if pin_lower == "in0" || pin_lower == "1" {
                    return i18n::tr("BCD input bit 0 (LSB, weight 1)");
                } else if pin_lower == "in1" || pin_lower == "2" {
                    return i18n::tr("BCD input bit 1 (weight 2)");
                } else if pin_lower == "in2" || pin_lower == "4" {
                    return i18n::tr("BCD input bit 2 (weight 4)");
                } else if pin_lower == "in3" || pin_lower == "8" {
                    return i18n::tr("BCD input bit 3 (MSB, weight 8)");
                } else if pin_lower == "in4" || pin_lower == "e" || pin_lower == "enable" {
                    return i18n::tr("Enable input (active low)");
                } else if pin_lower == "in5" || pin_lower == "." || pin_lower == "dp" {
                    return i18n::tr("Decimal point input");
                }
            }
            Part::Max72xx(_) => {
                if pin_lower == "din" {
                    return i18n::tr("SPI data in (DIN)");
                } else if pin_lower == "cs" {
                    return i18n::tr("SPI chip select / load (active low)");
                } else if pin_lower == "clk" {
                    return i18n::tr("SPI clock");
                } else if pin_lower == "vcc" {
                    return i18n::tr("Positive supply voltage (Vcc)");
                } else if pin_lower == "gnd" {
                    return i18n::tr("Ground");
                }
            }
            Part::Ws2812(_) => {
                if pin_lower == "din" {
                    return i18n::tr("LED data input");
                } else if pin_lower == "vcc" {
                    return i18n::tr("Positive supply voltage (Vcc)");
                } else if pin_lower == "dout" {
                    return i18n::tr("LED data output (cascade)");
                } else if pin_lower == "gnd" {
                    return i18n::tr("Ground");
                }
            }
            Part::Hd44780(_) => {
                if pin_lower == "pinrs" || pin_lower == "rs" {
                    return i18n::tr("Register select (instruction / data)");
                } else if pin_lower == "pinrw" || pin_lower == "rw" {
                    return i18n::tr("Read/Write select");
                } else if pin_lower == "pinen" || pin_lower == "en" || pin_lower == "e" {
                    return i18n::tr("Enable strobe");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("datapin")
                    .or_else(|| pin_lower.strip_prefix('d'))
                {
                    if let Ok(idx) = sub.parse::<usize>() {
                        return format!("{} D{idx}", i18n::tr("Data bus bit"));
                    }
                }
            }
            Part::I2CToParallel(_) => {
                if pin_lower == "scl" || pin_lower == "in1" || pin_lower == "pinscl" {
                    return i18n::tr("I2C clock line (SCL)");
                } else if pin_lower == "sda" || pin_lower == "in0" || pin_lower == "pinsda" {
                    return i18n::tr("I2C data line (SDA)");
                } else if pin_lower == "int" || pin_lower == "in5" {
                    return i18n::tr("Interrupt output (active low, open-drain)");
                } else if pin_lower == "a0" || pin_lower == "in2" {
                    return i18n::tr("I2C address bit A0");
                } else if pin_lower == "a1" || pin_lower == "in3" {
                    return i18n::tr("I2C address bit A1");
                } else if pin_lower == "a2" || pin_lower == "in4" {
                    return i18n::tr("I2C address bit A2");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("out")
                    .or_else(|| pin_lower.strip_prefix('d'))
                    .or_else(|| pin_lower.strip_prefix('p'))
                {
                    if let Ok(idx) = sub.parse::<usize>() {
                        return format!("{} D{idx}", i18n::tr("Parallel I/O bit"));
                    }
                }
            }
            Part::Ssd1306(_) => {
                if pin_lower == "pinsck"
                    || pin_lower == "sck"
                    || pin_lower == "scl"
                    || pin_lower == "pinscl"
                {
                    return i18n::tr("I2C clock line (SCL)");
                } else if pin_lower == "pinsda" || pin_lower == "sda" {
                    return i18n::tr("I2C data line (SDA)");
                }
            }
            Part::Adc(adc) => {
                if pin_lower == "vin"
                    || pin_lower == "input"
                    || pin_lower == "in"
                    || pin_lower == "in0"
                {
                    return i18n::tr("Analog voltage input");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("out")
                    .or_else(|| pin_lower.strip_prefix('d'))
                {
                    if let Ok(idx) = sub.parse::<usize>() {
                        let msb_note = if idx + 1 == adc.bits {
                            " (MSB)"
                        } else if idx == 0 {
                            " (LSB)"
                        } else {
                            ""
                        };
                        return format!("{} D{idx}{msb_note}", i18n::tr("Digital output bit"));
                    }
                }
            }
            Part::Dac(dac) => {
                if pin_lower == "vout"
                    || pin_lower == "output"
                    || pin_lower == "out"
                    || pin_lower == "out0"
                {
                    return i18n::tr("Analog voltage output");
                } else if let Some(sub) = pin_lower
                    .strip_prefix("in")
                    .or_else(|| pin_lower.strip_prefix('d'))
                {
                    if let Ok(idx) = sub.parse::<usize>() {
                        let msb_note = if idx + 1 == dac.bits {
                            " (MSB)"
                        } else if idx == 0 {
                            " (LSB)"
                        } else {
                            ""
                        };
                        return format!("{} D{idx}{msb_note}", i18n::tr("Digital input bit"));
                    }
                }
            }
            Part::KeyPad(kp) => {
                if let Some(sub) = pin_lower.strip_prefix("pin") {
                    if let Ok(idx) = sub.parse::<usize>() {
                        if idx < kp.rows {
                            return format!(
                                "{} {} {}",
                                i18n::tr("Row"),
                                idx,
                                i18n::tr("scan line")
                            );
                        } else {
                            return format!(
                                "{} {} {}",
                                i18n::tr("Column"),
                                idx - kp.rows,
                                i18n::tr("scan line")
                            );
                        }
                    }
                }
            }
            Part::LedBar(_) => {
                if let Some(sub) = pin_lower.strip_prefix("lpin") {
                    return format!("LED {} {}", sub, i18n::tr("anode (+)"));
                } else if let Some(sub) = pin_lower.strip_prefix("rpin") {
                    return format!("LED {} {}", sub, i18n::tr("cathode (−)"));
                }
            }
            Part::SwitchDip(_) => {
                if pin_lower == "com" {
                    return i18n::tr("Common terminal");
                } else if let Some(sub) = pin_lower.strip_prefix("pin") {
                    return format!("{} {} {}", i18n::tr("Switch"), sub, i18n::tr("terminal"));
                }
            }
            Part::ResistorDip(_) => {
                if pin_lower == "com" {
                    return i18n::tr("Bussed common terminal");
                } else if let Some(sub) = pin_lower.strip_prefix("lpin") {
                    return format!(
                        "{} {} {}",
                        i18n::tr("Resistor"),
                        sub,
                        i18n::tr("terminal A")
                    );
                } else if let Some(sub) = pin_lower.strip_prefix("rpin") {
                    return format!(
                        "{} {} {}",
                        i18n::tr("Resistor"),
                        sub,
                        i18n::tr("terminal B")
                    );
                } else if let Some(sub) = pin_lower.strip_prefix("pin") {
                    return format!("{} {} {}", i18n::tr("Resistor"), sub, i18n::tr("terminal"));
                }
            }
            Part::Function(_) => {
                if pin_lower == "out" || pin_lower == "out0" {
                    return i18n::tr("Function output");
                } else if let Some(sub) = pin_lower.strip_prefix("in") {
                    return format!("{} {sub}", i18n::tr("Function input"));
                }
            }
            Part::I2CRam(_) => {
                if pin_lower == "pinsda" || pin_lower == "sda" || pin_lower == "in0" {
                    return i18n::tr("I2C data line (SDA)");
                } else if pin_lower == "pinscl" || pin_lower == "scl" || pin_lower == "in1" {
                    return i18n::tr("I2C clock line (SCL)");
                } else if pin_lower == "a0" || pin_lower == "in2" {
                    return i18n::tr("I2C address bit A0");
                } else if pin_lower == "a1" || pin_lower == "in3" {
                    return i18n::tr("I2C address bit A1");
                } else if pin_lower == "a2" || pin_lower == "in4" {
                    return i18n::tr("I2C address bit A2");
                }
            }
            Part::Bus(_) => {
                // Trunk connector pins (angle 90 bottom / 270 top, length 1, is_bus true)
                if pin_lower == "epin0" || pin_lower == "buspini" {
                    return i18n::tr("Bus trunk connector");
                }
                // Line pins: ePin1..ePinN; label already contains the bit number
                if let Some(rest) = pin_lower.strip_prefix("epin") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        let bit = pin.label.trim();
                        if bit.is_empty() {
                            let line_num = idx.saturating_sub(1);
                            return format!("{} {line_num}", i18n::tr("Bus line"));
                        } else {
                            return format!("{} {bit}", i18n::tr("Bus line"));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let suffix = PinSuffix::parse(pin_id);
    let desc: std::borrow::Cow<'_, str> = match suffix {
        PinSuffix::Left => {
            if let Some(it) = item {
                match &it.kind {
                    Part::Led(_) | Part::Diode(_) => "Anode".into(),
                    Part::Battery(_) => "Positive terminal (+)".into(),
                    Part::ElCapacitor(_) => "+ (Positive terminal)".into(),
                    Part::AudioOut(_) => "Speaker (+)".into(),
                    _ => "Terminal 1".into(),
                }
            } else {
                "Terminal 1".into()
            }
        }
        PinSuffix::Right => {
            if let Some(it) = item {
                match &it.kind {
                    Part::Led(_) | Part::Diode(_) => "Cathode".into(),
                    Part::Battery(_) => "Negative terminal (-)".into(),
                    Part::ElCapacitor(_) => "- (Negative terminal)".into(),
                    Part::AudioOut(_) => "Speaker (-)".into(),
                    _ => "Terminal 2".into(),
                }
            } else {
                "Terminal 2".into()
            }
        }
        PinSuffix::Collector => "Collector".into(),
        PinSuffix::Emitter => "Emitter".into(),
        PinSuffix::Base => "Base".into(),
        PinSuffix::Drain => "Drain".into(),
        PinSuffix::Source => "Source".into(),
        PinSuffix::Gate => "Gate".into(),
        PinSuffix::InputPos => "Non-inverting input (+)".into(),
        PinSuffix::InputNeg => "Inverting input (-)".into(),
        PinSuffix::Out | PinSuffix::OutNod | PinSuffix::OutPin | PinSuffix::OutputAmp => {
            "Output".into()
        }
        PinSuffix::Ground => "Ground / 0V reference".into(),
        PinSuffix::PowerPos => "Positive supply voltage".into(),
        PinSuffix::PowerNeg => "Negative supply voltage".into(),
        _ => {
            let upper = pin.label.trim().to_uppercase();
            let upper_trimmed = if upper.starts_with('!') {
                &upper[1..]
            } else {
                &upper
            };
            match upper_trimmed {
                "GND" | "VSS" => "Ground / 0V reference".into(),
                "AGND" => "Analog ground".into(),
                "VCC" | "VDD" => "Positive supply voltage".into(),
                "AVCC" | "AVDD" | "VDDA" => "Analog positive supply voltage".into(),
                "AREF" => "Analog reference voltage".into(),
                "VIN" => "Input voltage".into(),
                "VOUT" => "Output voltage".into(),
                "CLK" | "CLOCK" => "Clock input".into(),
                "RST" | "RESET" => "Reset input".into(),
                "EN" | "ENABLE" => "Enable input".into(),
                "OE" => "Output enable".into(),
                "WE" => "Write enable".into(),
                "CE" | "CS" => "Chip select".into(),
                "SS" => "SPI slave select".into(),
                "SDA" => "I2C data line".into(),
                "SCL" => "I2C clock line".into(),
                "MOSI" => "SPI data output (master out, slave in)".into(),
                "MISO" => "SPI data input (master in, slave out)".into(),
                "SCK" | "SCLK" | "CK" => "SPI/serial clock".into(),
                "TX" => "Serial data transmit".into(),
                "RX" => "Serial data receive".into(),
                "NC" => "Not connected".into(),
                "IN" => "Signal input".into(),
                "OUT" => "Signal output".into(),
                _ => {
                    if let Some(num) = upper_trimmed.strip_prefix('A')
                        && num.chars().all(|c| c.is_ascii_digit())
                    {
                        return format!("{} {num}", i18n::tr("Address line"));
                    }
                    if let Some(num) = upper_trimmed.strip_prefix('D')
                        && num.chars().all(|c| c.is_ascii_digit())
                    {
                        return format!("{} {num}", i18n::tr("Data line"));
                    }
                    if let Some(num) = upper_trimmed.strip_prefix('I')
                        && num.chars().all(|c| c.is_ascii_digit())
                    {
                        return format!("{} {num}", i18n::tr("Input"));
                    }
                    if (upper_trimmed.starts_with('O')
                        || upper_trimmed.starts_with('Y')
                        || upper_trimmed.starts_with('Q'))
                        && upper_trimmed[1..].chars().all(|c| c.is_ascii_digit())
                    {
                        return format!("{} {}", i18n::tr("Output"), &upper_trimmed[1..]);
                    }
                    if let Some(num) = upper_trimmed.strip_prefix("CH")
                        && num.chars().all(|c| c.is_ascii_digit())
                    {
                        return format!("{} {num}", i18n::tr("Channel"));
                    }
                    if let Some(num) = upper_trimmed.strip_prefix('S')
                        && num.chars().all(|c| c.is_ascii_digit())
                    {
                        return format!("{} {num}", i18n::tr("Select line"));
                    }
                    if upper_trimmed.starts_with('P')
                        && upper_trimmed.len() >= 3
                        && upper_trimmed
                            .chars()
                            .nth(1)
                            .map_or(false, |c| c.is_ascii_alphabetic())
                        && upper_trimmed[2..].chars().all(|c| c.is_ascii_digit())
                    {
                        let port = upper_trimmed.chars().nth(1).unwrap();
                        let pin_num = &upper_trimmed[2..];
                        return format!(
                            "{} {port} {} {pin_num}",
                            i18n::tr("Port"),
                            i18n::tr("pin")
                        );
                    }
                    if !pin.label.is_empty() {
                        pin.label.as_str().into()
                    } else {
                        pin_id.into()
                    }
                }
            }
        }
    };
    i18n::tr(&desc)
}
