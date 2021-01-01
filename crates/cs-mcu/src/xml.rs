//! `.mcu` XML matching `McuCreator::processFile` for PIC / AVR / 8051 / 6502 / Z80
//! cores and scripted `usart` / `spi` / `twi`.

use std::collections::HashMap;
use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::dataspace::DataSpace;
use crate::desc::{
    CcpSpec, ConfigSpec, CoreKind, ExtIntSpec, IcUnitSpec, InterruptSpec, InterruptsSpec, McuDesc,
    OcUnitSpec, PortIntSpec, SpiSpec, StackSpec, TimerSpec, TrUnitSpec, TwiSpec, UsartSpec,
};
use crate::port::PortSpec;
use crate::{Error, Result};

#[derive(Clone, Debug)]
struct Elem {
    name: String,
    attrs: HashMap<String, String>,
    children: Vec<Elem>,
}

pub fn parse_mcu_xml(src: &str) -> Result<McuDesc> {
    let root = parse_root(src)?;
    build_desc(&root, None)
}

pub fn parse_mcu_file(path: impl AsRef<Path>) -> Result<McuDesc> {
    let path = path.as_ref();
    let src = std::fs::read_to_string(path)?;
    let root = parse_root(&src)?;
    let base = path.parent();
    build_desc(&root, base)
}

fn build_desc(root: &Elem, base: Option<&Path>) -> Result<McuDesc> {
    let mut root = find_mcu(root).ok_or_else(|| Error::Parse("no <mcu> root".into()))?;
    // Includes: C++ processes them inline while walking children.
    expand_includes(&mut root, base)?;

    let core_name = root.attr("core").unwrap_or("").to_string();
    let core = CoreKind::from_attr(&core_name);
    let mut desc = McuDesc::new(core, core_name);

    if let Some(v) = root.attr("data") {
        desc.data_size = parse_uint_auto(v);
        desc.data = DataSpace::new(desc.data_size);
    }
    if let Some(v) = root.attr("prog") {
        desc.prog_size = parse_uint_auto(v);
    }
    if let Some(v) = root.attr("progword") {
        desc.word_size = parse_uint_auto(v) as u8;
    }
    if let Some(v) = root.attr("eeprom") {
        desc.eeprom_size = parse_uint_auto(v);
    }
    if let Some(v) = root.attr("inst_cycle") {
        desc.inst_cycle = parse_f64(v);
        desc.cpu_cycle = desc.inst_cycle;
    }
    if let Some(v) = root.attr("cpu_cycle") {
        desc.cpu_cycle = parse_f64(v);
    }
    if let Some(v) = root.attr("freq") {
        desc.freq = parse_f64(v);
    }
    desc.prog_fill = match desc.core {
        CoreKind::Pic12 => 0x0FFF,
        CoreKind::Pic14 => 0x3FFF,
        _ => 0xFFFF,
    };
    if let Some(v) = root.attr("clkpin") {
        desc.clkpin = Some(v.to_string());
    }
    if let Some(v) = root.attr("progpage") {
        desc.prog_page = parse_uint_auto(v) as u8;
    }
    if let Some(v) = root.attr("script").filter(|s| !s.is_empty()) {
        desc.script = Some(v.to_string());
    }

    for child in &root.children {
        match child.name.as_str() {
            "regblock" => create_registers(&mut desc, child, false),
            "datablock" => create_data_block(&mut desc, child),
            "progblock" => {
                for el in &child.children {
                    if el.name == "progval" {
                        let addr = parse_uint_auto(el.attr("addr").unwrap_or("0")) as u16;
                        let value = parse_uint_auto(el.attr("value").unwrap_or("0")) as u16;
                        desc.prog_init.push((addr, value));
                    }
                }
            }
            "stack" => {
                let regs = child
                    .attr("spreg")
                    .unwrap_or("")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                desc.stack = Some(StackSpec {
                    regs,
                    increment: child.attr("increment").unwrap_or("").to_string(),
                });
            }
            "port" | "mcuport" => desc.ports.push(parse_port(child, false)),
            "ioport" => desc.ports.push(parse_port(child, true)),
            "interrupts" => desc.interrupts = parse_interrupts(child),
            "timer" => desc.timers.push(parse_timer(child)),
            "ccpunit" => desc.ccps.push(parse_ccp(child)),
            "usart" => desc.usarts.push(parse_usart(child)),
            "spi" => desc.spis.push(parse_spi(child)),
            "twi" => desc.twis.push(parse_twi(child)),
            "msspunit" => {
                for el in &child.children {
                    match el.name.as_str() {
                        "spi" => desc.spis.push(parse_spi(el)),
                        "twi" => desc.twis.push(parse_twi(el)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(desc)
}

fn expand_includes(elem: &mut Elem, base: Option<&Path>) -> Result<()> {
    let mut i = 0;
    while i < elem.children.len() {
        if elem.children[i].name == "include" {
            let file = elem.children[i].attr("file").unwrap_or("").to_string();
            let path = match base {
                Some(b) => b.join(&file),
                None => Path::new(&file).to_path_buf(),
            };
            let src = std::fs::read_to_string(&path)
                .map_err(|e| Error::Parse(format!("include {file}: {e}")))?;
            let inc = parse_root(&src)?;
            let mut inc_root = find_mcu(&inc).unwrap_or(inc);
            expand_includes(&mut inc_root, path.parent())?;
            let kids = std::mem::take(&mut inc_root.children);
            elem.children.remove(i);
            for (k, child) in kids.into_iter().enumerate() {
                elem.children.insert(i + k, child);
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    Ok(())
}

fn find_mcu(root: &Elem) -> Option<Elem> {
    if matches!(root.name.as_str(), "mcu" | "iou" | "mpu" | "parts") {
        return Some(root.clone());
    }
    for c in &root.children {
        if matches!(c.name.as_str(), "mcu" | "iou" | "mpu" | "parts") {
            return Some(c.clone());
        }
        if let Some(f) = find_mcu(c) {
            return Some(f);
        }
    }
    None
}

fn create_registers(desc: &mut McuDesc, e: &Elem, data_block: bool) {
    let start = parse_uint_auto(e.attr("start").unwrap_or("0")) as u16;
    let end = parse_uint_auto(e.attr("end").unwrap_or("0")) as u16;
    let offset = parse_uint_auto(e.attr("offset").unwrap_or("0")) as u16;
    if end as u32 >= desc.data.ram_size {
        return;
    }
    if !data_block {
        if start < desc.data.reg_start {
            desc.data.reg_start = start;
        }
        if end > desc.data.reg_end {
            desc.data.reg_end = end;
        }
        desc.data.ensure_reg_mask(end);
        for i in start..=end {
            if desc.data.mapper_addr(i) == 0xFFFF {
                desc.data.set_map(i, i);
            }
        }
    }
    get_registers(desc, e, offset);
}

fn create_data_block(desc: &mut McuDesc, d: &Elem) {
    let start = parse_uint_auto(d.attr("start").unwrap_or("0")) as u16;
    let end = parse_uint_auto(d.attr("end").unwrap_or("0")) as u16;
    let mut map_to = start;
    if let Some(v) = d.attr("mapto") {
        map_to = parse_uint_auto(v) as u16;
    }
    if end as u32 >= desc.data.ram_size {
        return;
    }
    desc.data.map_range(start, end, map_to);
    get_registers(desc, d, 0);
}

fn get_registers(desc: &mut McuDesc, e: &Elem, offset: u16) {
    let streg = e.attr("streg").unwrap_or("").to_string();
    for el in &e.children {
        if el.name == "register" {
            let name = el.attr("name").unwrap_or("").to_string();
            let addr = parse_uint_auto(el.attr("addr").unwrap_or("0")) as u16 + offset;
            let reset = parse_uint_bin(el.attr("reset").unwrap_or(""));
            let mask = el
                .attr("mask")
                .filter(|s| !s.is_empty())
                .map(parse_uint_bin);
            let bits = el.attr("bits").unwrap_or("").to_string();
            desc.data.add_register(&name, addr, reset, mask, &bits);
            if !streg.is_empty() && name == streg {
                let bit_list: Vec<String> = bits
                    .split(',')
                    .map(|s| s.split('|').next().unwrap_or(s).to_string())
                    .collect();
                desc.data.set_status(&name, bit_list);
            }
        } else if el.name == "mapped" {
            let addr = parse_uint_auto(el.attr("addr").unwrap_or("0")) as u16 + offset;
            let map_to = parse_uint_auto(el.attr("mapto").unwrap_or("0")) as u16;
            desc.data.set_map(addr, map_to);
        }
    }
}

fn parse_port(p: &Elem, is_io: bool) -> PortSpec {
    let name = p.attr("name").unwrap_or("PORT").to_string();
    let pins_attr = p.attr("pins").unwrap_or("0");
    let pin_labels = parse_pin_labels(pins_attr);
    let n_pins = if pin_labels.is_empty() {
        parse_uint_auto(pins_attr) as u8
    } else {
        pin_labels.len() as u8
    };
    let pinmask = p
        .attr("pinmask")
        .map(parse_uint_bin_u32)
        .unwrap_or(0xFFFF_FFFF);
    let mut dirreg = p.attr("dirreg").unwrap_or("").to_string();
    let mut dir_inv = false;
    if let Some(rest) = dirreg.strip_prefix('!') {
        dir_inv = true;
        dirreg = rest.to_string();
    }
    let outreg = p
        .attr("outreg")
        .and_then(|s| s.split(',').next())
        .unwrap_or("")
        .to_string();
    PortSpec {
        name,
        n_pins,
        pin_mask: pinmask,
        out_reg: outreg,
        in_reg: p.attr("inreg").unwrap_or("").to_string(),
        dir_reg: dirreg,
        dir_inv,
        out_mask: p.attr("outmask").map(parse_uint_bin_u32).unwrap_or(0),
        inp_mask: p
            .attr("inpmask")
            .map(parse_uint_bin_u32)
            .unwrap_or(if is_io { 0xFFFF_FFFF } else { 0xFF }),
        pullups: p.attr("pullups").unwrap_or("").to_string(),
        reset_pin: p.attr("resetpin").unwrap_or("").to_string(),
        open_col: p.attr("opencol").map(parse_uint_bin_u32).unwrap_or(0),
        is_io,
        pin_labels,
        interrupt: p
            .children
            .iter()
            .find(|c| c.name == "interrupt")
            .map(|el| PortIntSpec {
                name: el.attr("name").unwrap_or("").to_string(),
                mask: el.attr("mask").unwrap_or("").to_string(),
                bitmask: el.attr("bitmask").unwrap_or("").to_string(),
            }),
        extints: p
            .children
            .iter()
            .filter(|c| c.name == "extint")
            .map(|el| ExtIntSpec {
                name: el.attr("name").unwrap_or("").to_string(),
                pin: el.attr("pin").unwrap_or("").to_string(),
                config_bits: el.attr("configbits").unwrap_or("").to_string(),
            })
            .collect(),
    }
}

fn parse_config(e: &Elem) -> ConfigSpec {
    ConfigSpec {
        regs_a: e.attr("configregsA").unwrap_or("").to_string(),
        regs_b: e.attr("configregsB").unwrap_or("").to_string(),
        regs_c: e.attr("configregsC").unwrap_or("").to_string(),
        bits_a: e.attr("configbitsA").unwrap_or("").to_string(),
        bits_b: e.attr("configbitsB").unwrap_or("").to_string(),
        bits_c: e.attr("configbitsC").unwrap_or("").to_string(),
    }
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn parse_interrupts(e: &Elem) -> InterruptsSpec {
    let mut spec = InterruptsSpec {
        enable: e.attr("enable").unwrap_or("").to_string(),
        ints: Vec::new(),
    };
    for el in &e.children {
        if el.name != "interrupt" {
            continue;
        }
        let name = el.attr("name").unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let auto_clear = el.attr("autoclear").map(|v| parse_uint_auto(v) != 0);
        spec.ints.push(InterruptSpec {
            name,
            vector: parse_uint_auto(el.attr("vector").unwrap_or("0")) as u16,
            enable: el.attr("enable").unwrap_or("").to_string(),
            flag: el.attr("flag").unwrap_or("").to_string(),
            priority: el.attr("priority").unwrap_or("").to_string(),
            clear_on_one: el
                .attr("clear")
                .map(|v| parse_uint_auto(v) != 0)
                .unwrap_or(false),
            auto_clear,
            pin: el.attr("pin").map(|s| s.to_string()),
            wakeup: el
                .attr("wakeup")
                .map(|s| {
                    u8::from_str_radix(s.trim(), 2).unwrap_or_else(|_| parse_uint_auto(s) as u8)
                })
                .unwrap_or(0),
        });
    }
    spec
}

fn parse_timer(t: &Elem) -> TimerSpec {
    let mut oc_units = Vec::new();
    let mut ic_unit = None;
    for el in &t.children {
        match el.name.as_str() {
            "ocunit" => oc_units.push(OcUnitSpec {
                name: el.attr("name").unwrap_or("").to_string(),
                pin: el.attr("pin").unwrap_or("").to_string(),
                ocreg: split_csv(el.attr("ocreg").unwrap_or("")),
                interrupt: el.attr("interrupt").unwrap_or("").to_string(),
                bits: el.attr("bits").unwrap_or("").to_string(),
                config: parse_config(el),
            }),
            "icunit" => {
                ic_unit = Some(IcUnitSpec {
                    name: el.attr("name").unwrap_or("").to_string(),
                    pin: el.attr("pin").unwrap_or("").to_string(),
                    icreg: split_csv(el.attr("icreg").unwrap_or("")),
                    interrupt: el.attr("interrupt").unwrap_or("").to_string(),
                    bits: el.attr("bits").unwrap_or("").to_string(),
                });
            }
            _ => {}
        }
    }
    TimerSpec {
        name: t.attr("name").unwrap_or("TIMER").to_string(),
        type_id: parse_uint_auto(t.attr("type").unwrap_or("0")) as i32,
        counter: split_csv(t.attr("counter").unwrap_or("")),
        enable: t.attr("enable").unwrap_or("").to_string(),
        interrupt: t.attr("interrupt").unwrap_or("").to_string(),
        clock_pin: split_csv(t.attr("clockpin").unwrap_or("")),
        top_reg0: split_csv(t.attr("topreg0").unwrap_or("")),
        prescalers: t.attr("prescalers").unwrap_or("").replace(' ', ""),
        pr_select: t.attr("prselect").unwrap_or("").to_string(),
        config: parse_config(t),
        oc_units,
        ic_unit,
    }
}

fn parse_ccp(c: &Elem) -> CcpSpec {
    CcpSpec {
        name: c.attr("name").unwrap_or("CCP").to_string(),
        type_id: parse_uint_auto(c.attr("type").unwrap_or("0")) as i32,
        pin: c.attr("pin").unwrap_or("").to_string(),
        ccpreg: split_csv(c.attr("ccpreg").unwrap_or("")),
        interrupt: c.attr("interrupt").unwrap_or("").to_string(),
        config: parse_config(c),
    }
}

fn parse_trunit(el: &Elem) -> TrUnitSpec {
    TrUnitSpec {
        register: el.attr("register").unwrap_or("").to_string(),
        pins: split_csv(el.attr("pin").unwrap_or("")),
        enable: el.attr("enable").unwrap_or("").to_string(),
        interrupt: el.attr("interrupt").unwrap_or("").to_string(),
        config: parse_config(el),
    }
}

fn parse_spi(e: &Elem) -> SpiSpec {
    SpiSpec {
        name: e.attr("name").unwrap_or("SPI").to_string(),
        pins: split_csv(e.attr("pins").unwrap_or("")),
        data_reg: e.attr("dataregs").unwrap_or("").to_string(),
        status_reg: e.attr("statusreg").unwrap_or("").to_string(),
        interrupt: e.attr("interrupt").unwrap_or("").to_string(),
        prescalers: e.attr("prescalers").unwrap_or("").replace(' ', ""),
        config: parse_config(e),
    }
}

fn parse_twi(e: &Elem) -> TwiSpec {
    TwiSpec {
        name: e.attr("name").unwrap_or("TWI").to_string(),
        pins: split_csv(e.attr("pins").unwrap_or("")),
        data_reg: e.attr("dataregs").unwrap_or("").to_string(),
        addr_reg: e.attr("addressreg").unwrap_or("").to_string(),
        status_reg: e.attr("statusreg").unwrap_or("").to_string(),
        interrupt: e.attr("interrupt").unwrap_or("").to_string(),
        prescalers: e.attr("prescalers").unwrap_or("").replace(' ', ""),
        config: parse_config(e),
    }
}

fn parse_usart(u: &Elem) -> UsartSpec {
    let mut spec = UsartSpec {
        name: u.attr("name").unwrap_or("USART").to_string(),
        number: parse_uint_auto(u.attr("number").unwrap_or("0")) as i32,
        core: u.attr("core").map(|s| s.to_string()),
        interrupt: u.attr("interrupt").unwrap_or("").to_string(),
        config: parse_config(u),
        tx: None,
        rx: None,
    };
    let mut tx_reg = String::new();
    for el in &u.children {
        if el.name != "trunit" {
            continue;
        }
        match el.attr("type").unwrap_or("") {
            "tx" => {
                let tx = parse_trunit(el);
                tx_reg = tx.register.clone();
                spec.tx = Some(tx);
            }
            "rx" => {
                let mut rx = parse_trunit(el);
                if rx.register.is_empty() {
                    rx.register = tx_reg.clone();
                }
                spec.rx = Some(rx);
            }
            _ => {}
        }
    }
    spec
}

fn parse_pin_labels(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.is_empty()
        || s.chars()
            .all(|c| c.is_ascii_digit() || c == 'x' || c == 'X')
    {
        return Vec::new();
    }
    if s.contains(',') || s.chars().any(|c| c.is_ascii_alphabetic()) {
        s.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

impl Elem {
    fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.get(k).map(String::as_str)
    }
}

fn parse_root(src: &str) -> Result<Elem> {
    let mut reader = Reader::from_str(src);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut attrs = HashMap::new();
                for a in e.attributes().flatten() {
                    attrs.insert(
                        String::from_utf8_lossy(a.key.as_ref()).into_owned(),
                        a.unescape_value()
                            .map(|v| v.into_owned())
                            .unwrap_or_default(),
                    );
                }
                return Ok(Elem {
                    name,
                    attrs,
                    children: Vec::new(),
                });
            }
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut attrs = HashMap::new();
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                    let val = a
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    attrs.insert(key, val);
                }
                buf.clear();
                return read_tree(&mut reader, name, attrs);
            }
            Ok(Event::Eof) => return Err(Error::Parse("empty xml".into())),
            Err(e) => return Err(Error::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

fn read_tree(
    reader: &mut Reader<&[u8]>,
    name: String,
    attrs: HashMap<String, String>,
) -> Result<Elem> {
    let mut children = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let n = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut a = HashMap::new();
                for at in e.attributes().flatten() {
                    a.insert(
                        String::from_utf8_lossy(at.key.as_ref()).into_owned(),
                        at.unescape_value()
                            .map(|v| v.into_owned())
                            .unwrap_or_default(),
                    );
                }
                children.push(read_tree(reader, n, a)?);
            }
            Ok(Event::Empty(e)) => {
                let n = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut a = HashMap::new();
                for at in e.attributes().flatten() {
                    a.insert(
                        String::from_utf8_lossy(at.key.as_ref()).into_owned(),
                        at.unescape_value()
                            .map(|v| v.into_owned())
                            .unwrap_or_default(),
                    );
                }
                children.push(Elem {
                    name: n,
                    attrs: a,
                    children: Vec::new(),
                });
            }
            Ok(Event::End(e)) => {
                let n = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if n == name {
                    return Ok(Elem {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            Ok(Event::Eof) => {
                return Ok(Elem {
                    name,
                    attrs,
                    children,
                });
            }
            Err(e) => return Err(Error::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

pub fn parse_uint_auto(s: &str) -> u32 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(h, 16).unwrap_or(0);
    }
    if s.len() > 1 && s.starts_with('0') && s.as_bytes()[1].is_ascii_digit() {
        return u32::from_str_radix(s, 8).unwrap_or(0);
    }
    s.parse().unwrap_or(0)
}

fn parse_uint_bin(s: &str) -> u8 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    u8::from_str_radix(s, 2).unwrap_or_else(|_| parse_uint_auto(s) as u8)
}

fn parse_uint_bin_u32(s: &str) -> u32 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    u32::from_str_radix(s, 2).unwrap_or_else(|_| parse_uint_auto(s))
}

fn parse_f64(s: &str) -> f64 {
    s.trim().parse().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIC14: &str = r#"
<mcu core="Pic14" data="256" prog="512" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTA" addr="0x05" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
  </regblock>
  <regblock start="0x80" end="0x8F">
    <mapped addr="0x80" mapto="0x00"/>
    <register name="OPTION" addr="0x81" reset="11111111"/>
    <register name="TRISA" addr="0x85" reset="11111111"/>
    <mapped addr="0x83" mapto="0x03"/>
  </regblock>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
</mcu>
"#;

    #[test]
    fn parse_pic14_desc() {
        let d = parse_mcu_xml(PIC14).unwrap();
        assert_eq!(d.core, CoreKind::Pic14);
        assert_eq!(d.data_size, 256);
        assert_eq!(d.prog_size, 512);
        assert_eq!(d.inst_cycle, 4.0);
        assert_eq!(d.data.reg_addr("STATUS"), Some(3));
        assert_eq!(d.data.get_reg_bits("R0,R1").mask, 0x60);
        assert_eq!(d.ports.len(), 1);
        assert!(d.ports[0].dir_inv);
        assert_eq!(d.ports[0].dir_reg, "TRISA");
    }

    #[test]
    fn parse_avr_stack() {
        let d = parse_mcu_xml(
            r#"
<mcu core="AVR" data="256" prog="64" progword="2" progpage="32">
  <stack spreg="SPL,SPH" increment="post-dec"/>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="DDRB" inreg="PINB"/>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.core, CoreKind::Avr);
        assert_eq!(d.prog_page, 32);
        let st = d.stack.unwrap();
        assert_eq!(st.regs, ["SPL", "SPH"]);
        assert!(!st.pre());
        assert_eq!(st.inc(), -1);
        assert!(!d.ports[0].dir_inv);
        assert_eq!(d.ports[0].in_reg, "PINB");
    }

    #[test]
    fn parse_i51_stack() {
        let d = parse_mcu_xml(
            r#"
<mcu core="8051" data="256" prog="4096" progword="1" inst_cycle="12" cpu_cycle="6">
  <stack spreg="SP" increment="pre-inc"/>
  <port name="PORT1" pins="8" outreg="P1"/>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.core, CoreKind::I51);
        assert_eq!(d.word_size, 1);
        assert_eq!(d.inst_cycle, 12.0);
        assert_eq!(d.cpu_cycle, 6.0);
        let st = d.stack.unwrap();
        assert_eq!(st.regs, ["SP"]);
        assert!(st.pre());
        assert_eq!(st.inc(), 1);
        assert!(d.ports[0].dir_reg.is_empty());
    }

    #[test]
    fn parse_i51_ioport_named_pins() {
        let d = parse_mcu_xml(
            r#"
<mcu core="8051" data="256" prog="4096" progword="1">
  <port name="PORT0" pins="8" outreg="P0" outmask="11111111" opencol="11111111"/>
  <ioport name="PORTE" pins="ALE,PSEN,EA"/>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.ports.len(), 2);
        assert!(!d.ports[0].is_io);
        assert_eq!(d.ports[0].open_col, 0xFF);
        assert_eq!(d.ports[0].out_mask, 0xFF);
        assert!(d.ports[1].is_io);
        assert_eq!(d.ports[1].pin_labels, ["ALE", "PSEN", "EA"]);
        assert_eq!(d.ports[1].n_pins, 3);
    }

    #[test]
    fn parse_mcs65_progword() {
        let d = parse_mcu_xml(
            r#"
<mcu core="6502" data="1" prog="65536" progword="1" inst_cycle="1">
  <progblock>
    <progval addr="0xFFFC" value="0"/>
    <progval addr="0xFFFD" value="0"/>
  </progblock>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.core, CoreKind::Mcs65);
        assert_eq!(d.word_size, 1);
        assert_eq!(d.prog_size, 65536);
        assert_eq!(d.prog_init, vec![(0xFFFC, 0), (0xFFFD, 0)]);
    }

    #[test]
    fn parse_z80_progword() {
        let d = parse_mcu_xml(
            r#"
<mcu core="Z80" data="1" prog="65536" progword="1" inst_cycle="1">
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.core, CoreKind::Z80);
        assert_eq!(d.word_size, 1);
        assert_eq!(d.prog_size, 65536);
    }

    #[test]
    fn parse_interrupts_timer_usart() {
        let d = parse_mcu_xml(
            r#"
<mcu core="Pic14" data="256" prog="64" progword="2">
  <regblock start="0" end="0x0B">
    <register name="TMR0" addr="0x01" reset="0"/>
    <register name="INTCON" addr="0x0B" bits="RBIF,INTF,T0IF,RBIE,INTE,T0IE,PEIE,GIE"/>
    <register name="TXSTA" addr="0x18" bits="TX9D,TRMT,BRGH,SYNC,TXEN,TX9"/>
    <register name="TXREG" addr="0x19"/>
    <register name="RCREG" addr="0x1A"/>
  </regblock>
  <interrupts enable="GIE">
    <interrupt name="T0_OVF" enable="T0IE" flag="T0IF" priority="1" vector="0x0004"/>
  </interrupts>
  <timer name="TIMER0" type="800" configregsA="OPTION" counter="TMR0"
         clockpin="PORTA4" interrupt="T0_OVF" prescalers="2,4,8,16,32,64,128,256"/>
  <usart name="USART0" number="1" configregsA="TXSTA" interrupt="USART_T">
    <trunit type="tx" pin="PORTC6" register="TXREG"/>
    <trunit type="rx" pin="PORTC7" register="RCREG" interrupt="USART_R"/>
  </usart>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.interrupts.enable, "GIE");
        assert_eq!(d.interrupts.ints.len(), 1);
        assert_eq!(d.interrupts.ints[0].name, "T0_OVF");
        assert_eq!(d.interrupts.ints[0].vector, 4);
        assert_eq!(d.timers.len(), 1);
        assert_eq!(d.timers[0].type_id, 800);
        assert_eq!(d.timers[0].counter, ["TMR0"]);
        assert_eq!(d.timers[0].prescalers, "2,4,8,16,32,64,128,256");
        assert_eq!(d.usarts.len(), 1);
        assert_eq!(d.usarts[0].tx.as_ref().unwrap().register, "TXREG");
        assert_eq!(d.usarts[0].rx.as_ref().unwrap().pins, ["PORTC7"]);
    }

    #[test]
    fn parse_scripted_usart_spi_twi() {
        let d = parse_mcu_xml(
            r#"
<mcu core="scripted" script="cpu.as" data="256" prog="1024" inst_cycle="1" freq="16000000">
  <ioport name="P" pins="TX,RX,MOSI,MISO,SCK,SS,SDA,SCL"/>
  <usart name="UART0" number="0">
    <trunit type="tx" pin="TX"/>
    <trunit type="rx" pin="RX"/>
  </usart>
  <spi name="SPI0" pins="MOSI,MISO,SCK,SS"/>
  <twi name="TWI0" pins="SDA,SCL"/>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.core, CoreKind::Scripted);
        assert_eq!(d.script.as_deref(), Some("cpu.as"));
        assert_eq!(d.ports.len(), 1);
        assert_eq!(
            d.ports[0].pin_labels,
            ["TX", "RX", "MOSI", "MISO", "SCK", "SS", "SDA", "SCL"]
        );
        assert_eq!(d.usarts.len(), 1);
        assert_eq!(d.usarts[0].name, "UART0");
        assert_eq!(d.usarts[0].tx.as_ref().unwrap().pins, ["TX"]);
        assert_eq!(d.usarts[0].rx.as_ref().unwrap().pins, ["RX"]);
        assert_eq!(d.spis.len(), 1);
        assert_eq!(d.spis[0].name, "SPI0");
        assert_eq!(d.spis[0].pins, ["MOSI", "MISO", "SCK", "SS"]);
        assert_eq!(d.twis.len(), 1);
        assert_eq!(d.twis[0].name, "TWI0");
        assert_eq!(d.twis[0].pins, ["SDA", "SCL"]);
    }

    #[test]
    fn parse_ocunit_icunit_and_port_int() {
        let d = parse_mcu_xml(
            r#"
<mcu core="AVR" data="256" prog="64" progword="2">
  <regblock start="0" end="0x5F">
    <register name="PORTB" addr="0x25"/>
    <register name="DDRB" addr="0x24"/>
    <register name="PINB" addr="0x23"/>
    <register name="TCCR0A" addr="0x44" bits="WGM00,WGM01,0,0,COM0B0,COM0B1,COM0A0,COM0A1"/>
    <register name="TCCR0B" addr="0x45" bits="CS00,CS01,CS02,WGM02,0,0,FOC0B,FOC0A"/>
    <register name="TCNT0" addr="0x46"/>
    <register name="OCR0A" addr="0x47"/>
    <register name="OCR0B" addr="0x48"/>
    <register name="ICR1L" addr="0x86"/>
    <register name="ICR1H" addr="0x87"/>
    <register name="PCMSK" addr="0x15"/>
    <register name="EICRA" addr="0x69" bits="ISC00,ISC01"/>
  </regblock>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="DDRB" inreg="PINB">
    <interrupt name="PCINT" mask="PCMSK"/>
    <extint name="INT0" pin="PORTB2" configbits="ISC00,ISC01"/>
  </port>
  <timer name="TIMER0" type="800" configregsA="TCCR0A" configregsB="TCCR0B"
         counter="TCNT0" interrupt="TIM0_OVF" prescalers="0,1,8,64,256,1024,EXT_F,EXT_R"
         prselect="CS00,CS01,CS02">
    <ocunit name="OC0A" pin="PORTB3" ocreg="OCR0A" bits="COM0A0,COM0A1" interrupt="TIM0_COMPA"/>
    <ocunit name="OC0B" pin="PORTB4" ocreg="OCR0B" bits="COM0B0,COM0B1" interrupt="TIM0_COMPB"/>
    <icunit name="IC1" pin="PORTB6" icreg="ICR1L,ICR1H" bits="ICES1,ICNC1" interrupt="TIM1_CAPT"/>
  </timer>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.ports[0].interrupt.as_ref().unwrap().name, "PCINT");
        assert_eq!(d.ports[0].interrupt.as_ref().unwrap().mask, "PCMSK");
        assert_eq!(d.ports[0].extints.len(), 1);
        assert_eq!(d.ports[0].extints[0].name, "INT0");
        assert_eq!(d.timers[0].oc_units.len(), 2);
        assert_eq!(d.timers[0].oc_units[0].name, "OC0A");
        assert_eq!(d.timers[0].oc_units[0].pin, "PORTB3");
        assert_eq!(d.timers[0].oc_units[0].ocreg, ["OCR0A"]);
        let ic = d.timers[0].ic_unit.as_ref().unwrap();
        assert_eq!(ic.name, "IC1");
        assert_eq!(ic.icreg, ["ICR1L", "ICR1H"]);
    }

    #[test]
    fn parse_ccpunit() {
        let d = parse_mcu_xml(
            r#"
<mcu core="Pic14" data="256" prog="64" progword="2">
  <regblock start="0" end="0x1F">
    <register name="CCP1CON" addr="0x17" bits="CCP1M0,CCP1M1,CCP1M2,CCP1M3,DC1B0,DC1B1"/>
    <register name="CCPR1L" addr="0x15"/>
    <register name="CCPR1H" addr="0x16"/>
  </regblock>
  <ccpunit name="CCP1" type="00" pin="PORTB3" ccpreg="CCPR1L,CCPR1H"
           interrupt="CCP1" configregsA="CCP1CON"/>
</mcu>
"#,
        )
        .unwrap();
        assert_eq!(d.ccps.len(), 1);
        assert_eq!(d.ccps[0].name, "CCP1");
        assert_eq!(d.ccps[0].type_id, 0);
        assert_eq!(d.ccps[0].pin, "PORTB3");
        assert_eq!(d.ccps[0].ccpreg, ["CCPR1L", "CCPR1H"]);
        assert_eq!(d.ccps[0].config.regs_a, "CCP1CON");
    }
}
