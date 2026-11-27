//! Hardware ADC peripheral for AVR (`ADCSRA`/`ADMUX`/`ADCL`/`ADCH`) and PIC (`ADCON0`/`ADCON1`/`ADRESL`/`ADRESH`).
//!
//! Samples the analog voltage on the selected channel pin, quantizes against the
//! configured reference voltage, writes the result registers with the selected
//! left/right alignment, clears the start bit, and raises the ADC interrupt.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{AdcSpec, CoreKind};
use crate::interrupts::Interrupts;
use crate::port::{self, Port};

#[derive(Clone, Debug)]
pub struct Adc {
    pub core: CoreKind,
    #[allow(dead_code)]
    pub name: String,
    type_id: i32,
    bits: u8,
    res_l_addr: Option<u16>,
    res_h_addr: Option<u16>,
    // AVR registers/bits
    admux_addr: Option<u16>,
    adcsrb_addr: Option<u16>,
    aden: RegBits,
    adsc: RegBits,
    adif: RegBits,
    adlar: RegBits,
    refs0: RegBits,
    refs1: RegBits,
    mux_mask: u8,
    mux5: RegBits,
    // PIC registers/bits
    adcon1_addr: Option<u16>,
    adon: RegBits,
    go_done: RegBits,
    adfm: RegBits,
    chs_bits: RegBits,
    vcfg0: RegBits,
    vcfg1: RegBits,
    interrupt: Option<usize>,
    /// Resolved `(port_idx, pin_idx)` for each channel in `adcpins`.
    channels: Vec<Option<(usize, usize)>>,
    /// External Vref+ pin (`vrefpins[0]` or `PORTV1` on AVR megax8).
    aref_pin: Option<(usize, usize)>,
    /// External Vref- pin (`vrefpins[1]` on PIC).
    vref_neg_pin: Option<(usize, usize)>,
    /// Analog supply AVCC pin (`PORTV0` on AVR megax8).
    avcc_pin: Option<(usize, usize)>,
}

impl Adc {
    pub fn from_spec(
        spec: &AdcSpec,
        data: &DataSpace,
        ports: &[Port],
        core: CoreKind,
        ints: &Interrupts,
    ) -> Self {
        let is_pic = matches!(core, CoreKind::Pic12 | CoreKind::Pic14);
        let res_l_name = spec
            .data_regs
            .first()
            .map(String::as_str)
            .unwrap_or(if is_pic { "ADRESL" } else { "ADCL" });
        let res_h_name = spec
            .data_regs
            .get(1)
            .map(String::as_str)
            .unwrap_or(if is_pic { "ADRESH" } else { "ADCH" });
        let admux_name = if spec.multiplex.is_empty() {
            "ADMUX"
        } else {
            spec.multiplex.as_str()
        };
        let cfg_b_name = spec
            .config
            .regs_b
            .split(',')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if is_pic { "ADCON1" } else { "ADCSRB" });

        let mut mux_mask = 0u8;
        for bit_name in ["MUX0", "MUX1", "MUX2", "MUX3"] {
            mux_mask |= data.get_reg_bits(bit_name).mask;
        }
        if mux_mask == 0 {
            mux_mask = 0x0F;
        }

        let chs_bits = data.get_reg_bits("CHS0,CHS1,CHS2,CHS3,CHS4");
        let go_done = {
            let b = data.get_reg_bits("GO/DONE");
            if b.mask != 0 {
                b
            } else {
                let b2 = data.get_reg_bits("GO_DONE");
                if b2.mask != 0 {
                    b2
                } else {
                    data.get_reg_bits("GO")
                }
            }
        };

        let channels = spec
            .adc_pins
            .iter()
            .map(|pin_name| port::find_gpio(ports, pin_name))
            .collect();

        let aref_pin = spec
            .vref_pins
            .first()
            .and_then(|name| port::find_gpio(ports, name))
            .or_else(|| {
                if !is_pic {
                    port::find_gpio(ports, "PORTV1")
                } else {
                    None
                }
            });
        let vref_neg_pin = spec
            .vref_pins
            .get(1)
            .and_then(|name| port::find_gpio(ports, name));
        let avcc_pin = if !is_pic {
            port::find_gpio(ports, "PORTV0")
        } else {
            None
        };

        Self {
            core,
            name: spec.name.clone(),
            type_id: spec.type_id,
            bits: spec.bits.clamp(8, 12),
            res_l_addr: data.reg_addr(res_l_name),
            res_h_addr: data.reg_addr(res_h_name),
            admux_addr: data.reg_addr(admux_name),
            adcsrb_addr: data.reg_addr(cfg_b_name),
            aden: data.get_reg_bits("ADEN"),
            adsc: data.get_reg_bits("ADSC"),
            adif: data.get_reg_bits("ADIF"),
            adlar: data.get_reg_bits("ADLAR"),
            refs0: data.get_reg_bits("REFS0"),
            refs1: data.get_reg_bits("REFS1"),
            mux_mask,
            mux5: data.get_reg_bits("MUX5"),
            adcon1_addr: data.reg_addr(cfg_b_name),
            adon: data.get_reg_bits("ADON"),
            go_done,
            adfm: data.get_reg_bits("ADFM"),
            chs_bits,
            vcfg0: data.get_reg_bits("VCFG0"),
            vcfg1: data.get_reg_bits("VCFG1"),
            interrupt: ints.get(&spec.interrupt),
            channels,
            aref_pin,
            vref_neg_pin,
            avcc_pin,
        }
    }

    /// Execute an ADC conversion triggered by writing `ADCSRA` (AVR) or `ADCON0` (PIC).
    pub fn convert(
        &self,
        val: u8,
        data: &mut DataSpace,
        ports: &[Port],
        ints: &mut Interrupts,
        vdd: f64,
    ) {
        match self.core {
            CoreKind::Avr => self.convert_avr(val, data, ports, ints, vdd),
            CoreKind::Pic12 | CoreKind::Pic14 => self.convert_pic(val, data, ports, ints, vdd),
            _ => {}
        }
    }

    fn convert_avr(
        &self,
        val: u8,
        data: &mut DataSpace,
        ports: &[Port],
        ints: &mut Interrupts,
        vdd: f64,
    ) {
        if self.adsc.mask == 0 || (val & self.adsc.mask) == 0 {
            return;
        }
        if self.aden.mask != 0 && (val & self.aden.mask) == 0 {
            return;
        }

        let admux = self.admux_addr.map(|a| data.get(a)).unwrap_or(0);
        let mut channel = usize::from(admux & self.mux_mask);
        if self.mux5.mask != 0 {
            let adcsrb = self.adcsrb_addr.map(|a| data.get(a)).unwrap_or(0);
            if (adcsrb & self.mux5.mask) != 0 {
                channel |= 8;
            }
        }

        let vref = self.avr_reference_voltage(admux, ports, vdd);
        let vin = self.channel_voltage(channel, ports);
        let code = quantize(vin, 0.0, vref, self.bits);

        let left_adjust = self.adlar.mask != 0 && (admux & self.adlar.mask) != 0;
        self.store_result(data, code, left_adjust);

        // Clear ADSC and set ADIF now that conversion is complete (consume any pending
        // `reg_override` from `IntWriteFlag` on ADCSRA so ADSC stays cleared).
        let mut adcsra = data
            .reg_override
            .take()
            .unwrap_or_else(|| data.get(self.adsc.reg_addr))
            & !self.adsc.mask;
        if self.adif.mask != 0 {
            adcsra |= self.adif.mask;
        }
        data.set(self.adsc.reg_addr, adcsra);
        if let Some(int_idx) = self.interrupt {
            ints.raise(int_idx, data);
        }
    }

    fn convert_pic(
        &self,
        val: u8,
        data: &mut DataSpace,
        ports: &[Port],
        ints: &mut Interrupts,
        vdd: f64,
    ) {
        if self.go_done.mask == 0 || (val & self.go_done.mask) == 0 {
            return;
        }
        if self.adon.mask != 0 && (val & self.adon.mask) == 0 {
            return;
        }

        let channel = usize::from(self.chs_bits.val(val));
        let adcon1 = self.adcon1_addr.map(|a| data.get(a)).unwrap_or(0);
        let vdd_safe = if vdd > 0.1 { vdd } else { 5.0 };

        let use_ext_vpos = self.vcfg0.mask != 0 && (adcon1 & self.vcfg0.mask) != 0;
        let use_ext_vneg = self.vcfg1.mask != 0 && (adcon1 & self.vcfg1.mask) != 0;
        let vref_pos = if use_ext_vpos {
            pin_voltage(ports, self.aref_pin)
                .filter(|&v| v > 0.1)
                .unwrap_or(vdd_safe)
        } else {
            vdd_safe
        };
        let vref_neg = if use_ext_vneg {
            pin_voltage(ports, self.vref_neg_pin).unwrap_or(0.0)
        } else {
            0.0
        };

        let vin = self.channel_voltage(channel, ports);
        let code = quantize(vin, vref_neg, vref_pos, self.bits);

        // PIC `ADFM = 1` -> right-justified; `ADFM = 0` -> left-justified.
        let adfm_reg = if self.adfm.mask != 0 {
            data.get(self.adfm.reg_addr)
        } else {
            adcon1
        };
        let right_adjust = self.adfm.mask != 0 && (adfm_reg & self.adfm.mask) != 0;
        self.store_result(data, code, !right_adjust);

        // Clear GO/DONE in ADCON0 and raise ADC interrupt.
        let adcon0 = data
            .reg_override
            .take()
            .unwrap_or_else(|| data.get(self.go_done.reg_addr))
            & !self.go_done.mask;
        data.set(self.go_done.reg_addr, adcon0);
        if let Some(int_idx) = self.interrupt {
            ints.raise(int_idx, data);
        }
    }

    fn store_result(&self, data: &mut DataSpace, code: u16, left_adjust: bool) {
        let encoded = if left_adjust {
            code << (16 - self.bits)
        } else {
            code
        };
        if let Some(a) = self.res_l_addr {
            data.set(a, (encoded & 0xFF) as u8);
        }
        if let Some(a) = self.res_h_addr {
            data.set(a, (encoded >> 8) as u8);
        }
    }

    fn avr_reference_voltage(&self, admux: u8, ports: &[Port], vdd: f64) -> f64 {
        let r0 = self.refs0.mask != 0 && (admux & self.refs0.mask) != 0;
        let r1 = self.refs1.mask != 0 && (admux & self.refs1.mask) != 0;
        let vdd_safe = if vdd > 0.1 { vdd } else { 5.0 };
        match (r1, r0) {
            // 00: External AREF pin (fallback to Vdd when unwired in schematic)
            (false, false) => pin_voltage(ports, self.aref_pin)
                .filter(|&v| v > 0.1)
                .unwrap_or(vdd_safe),
            // 01: AVcc (fallback to Vdd when AVcc pin is left unwired)
            (false, true) => pin_voltage(ports, self.avcc_pin)
                .filter(|&v| v > 0.1)
                .unwrap_or(vdd_safe),
            // 10: 1.1V internal on mega640/1280/2560 (type 04), otherwise reserved -> Vdd
            (true, false) => {
                if self.type_id == 4 {
                    1.1
                } else {
                    vdd_safe
                }
            }
            // 11: Internal bandgap reference (1.1V on megax8/mxx4/tiny13, 2.56V on mega8/32/64/2560)
            (true, true) => match self.type_id {
                3 | 4 | 11 | 20 => 2.56,
                _ => 1.1,
            },
        }
    }

    fn channel_voltage(&self, channel: usize, ports: &[Port]) -> f64 {
        if let Some(&pin_loc) = self.channels.get(channel) {
            return pin_voltage(ports, pin_loc).unwrap_or(0.0);
        }
        // Standard ATmega internal channels when not mapped to an external pin:
        // 14 = 1.1V bandgap, 15 = GND (0V).
        if self.core == CoreKind::Avr {
            match channel {
                14 => 1.1,
                15 => 0.0,
                _ => 0.0,
            }
        } else {
            0.0
        }
    }
}

fn quantize(vin: f64, vref_neg: f64, vref_pos: f64, bits: u8) -> u16 {
    let span = vref_pos - vref_neg;
    if span <= 1e-9 {
        return 0;
    }
    let levels = (1u32 << bits) as f64;
    let max_code = (1u32 << bits) - 1;
    (((vin - vref_neg) / span).clamp(0.0, 1.0) * levels)
        .floor()
        .min(max_code as f64) as u16
}

fn pin_voltage(ports: &[Port], loc: Option<(usize, usize)>) -> Option<f64> {
    let (pi, idx) = loc?;
    Some(ports.get(pi)?.pins.get(idx)?.inp_volt)
}

#[cfg(test)]
mod tests {
    use crate::Device;

    const AVR_ADC_XML: &str = r#"
<mcu name="mega328test" core="AVR" data="512" prog="256" progword="2" inst_cycle="1" freq="16000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="PINC" addr="0x0026" reset="" mask="11111111"/>
    <register name="DDRC" addr="0x0027" reset="" mask="11111111"/>
    <register name="PORTC" addr="0x0028" reset="" mask="11111111"/>
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
    <register name="ADCL" addr="0x0078" reset="" mask="00000000"/>
    <register name="ADCH" addr="0x0079" reset="" mask="00000000"/>
    <register name="ADCSRA" addr="0x007A" reset="" bits="ADPS0,ADPS1,ADPS2,ADIE,ADIF,ADATE,ADSC,ADEN"/>
    <register name="ADCSRB" addr="0x007B" reset="" bits="ADTS0,ADTS1,ADTS2,0,0,0,ACME,0"/>
    <register name="ADMUX" addr="0x007C" reset="" bits="MUX0,MUX1,MUX2,MUX3,0,ADLAR,REFS0,REFS1"/>
  </regblock>
  <port name="PORTC" pins="6" outreg="PORTC" inreg="PINC" dirreg="DDRC"/>
  <port name="PORTV" pins="4"/>
  <interrupts enable="I">
    <interrupt name="ADC" enable="ADIE" flag="ADIF" priority="5" vector="0x002A" clear="1"/>
  </interrupts>
  <adc name="ADC" type="00" bits="10" configregsA="ADCSRA" configregsB="ADCSRB"
       dataregs="ADCL,ADCH" multiplex="ADMUX" interrupt="ADC"
       prescalers="2,2,4,8,16,32,64,128"
       adcpins="PORTC0,PORTC1,PORTC2,PORTC3,PORTC4,PORTC5,PORTV2,PORTV3"/>
</mcu>
"#;

    const PIC_ADC_XML: &str = r#"
<mcu name="p16f88test" core="Pic14" data="256" prog="256" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0x0000" end="0x00FF" streg="STATUS">
    <register name="STATUS" addr="0x0003" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="PORTA" addr="0x0005" reset=""/>
    <register name="TRISA" addr="0x0085" reset="11111111"/>
    <register name="PIR1" addr="0x000C" reset="" bits="TMR1IF,TMR2IF,CCP1IF,SSPIF,TXIF,RCIF,ADIF,0"/>
    <register name="PIE1" addr="0x008C" reset="" bits="TMR1IE,TMR2IE,CCP1IE,SSPIE,TXIE,RCIE,ADIE,0"/>
    <register name="ADRESH" addr="0x001E" reset=""/>
    <register name="ADCON0" addr="0x001F" reset="" bits="ADON,0,GO/DONE,CHS0,CHS1,CHS2,ADCS0,ADCS1"/>
    <register name="ADRESL" addr="0x009E" reset=""/>
    <register name="ADCON1" addr="0x009F" reset="" bits="0,0,0,0,VCFG0,VCFG1,0,ADFM"/>
  </regblock>
  <port name="PORTA" pins="6" outreg="PORTA" dirreg="!TRISA"/>
  <interrupts enable="GIE">
    <interrupt name="ADC" enable="ADIE" flag="ADIF" priority="1" vector="0x0004"/>
  </interrupts>
  <adc name="ADC" type="00" bits="10" configregsA="ADCON0" configregsB="ADCON1"
       dataregs="ADRESL,ADRESH" interrupt="ADC"
       prescalers="2,8,32"
       adcpins="PORTA0,PORTA1,PORTA2,PORTA3,PORTA5"
       vrefpins="PORTA3,PORTA2"/>
</mcu>
"#;

    fn trigger_adc(dev: &mut Device, admux: u8) -> u16 {
        dev.poke_reg(0x007C, admux);
        // ADEN (0x80) | ADSC (0x40) | prescaler 128 (0x07)
        dev.poke_reg(0x007A, 0xC7);
        let adcsra = dev.ram(0x007A);
        assert_eq!(adcsra & 0x40, 0, "ADSC should clear after conversion");
        assert_ne!(adcsra & 0x10, 0, "ADIF should be set after conversion");
        let low = u16::from(dev.ram(0x0078));
        let high = u16::from(dev.ram(0x0079));
        low | (high << 8)
    }

    #[test]
    fn avr_adc_samples_analog_voltages_and_channels() {
        let mut dev = Device::from_xml("m", AVR_ADC_XML).unwrap();
        dev.set_pin_volt("PORTC0", 0.0);
        assert_eq!(trigger_adc(&mut dev, 0x40), 0);

        dev.set_pin_volt("PORTC0", 2.5);
        assert_eq!(trigger_adc(&mut dev, 0x40), 512);

        dev.set_pin_volt("PORTC0", 5.0);
        assert_eq!(trigger_adc(&mut dev, 0x40), 1023);

        // Channel 1 (PORTC1) independent from channel 0
        dev.set_pin_volt("PORTC1", 1.25);
        assert_eq!(trigger_adc(&mut dev, 0x41), 256);

        // Left-adjusted (ADLAR = 0x20) puts top 8 bits in ADCH
        dev.set_pin_volt("PORTC0", 2.5);
        let raw = trigger_adc(&mut dev, 0x60);
        assert_eq!(raw >> 6, 512);

        // Internal 1.1V reference (REFS1:0 = 11 -> 0xC0)
        dev.set_pin_volt("PORTC0", 0.55);
        let code = trigger_adc(&mut dev, 0xC0);
        assert!((510..=514).contains(&code), "expected ~512, got {code}");
    }

    #[test]
    fn avr_adc_requires_aden_enabled() {
        let mut dev = Device::from_xml("m", AVR_ADC_XML).unwrap();
        dev.set_pin_volt("PORTC0", 5.0);
        dev.poke_reg(0x007C, 0x40);
        // ADSC without ADEN should not write ADCL/ADCH
        dev.poke_reg(0x007A, 0x40);
        assert_eq!(dev.ram(0x0078), 0);
        assert_eq!(dev.ram(0x0079), 0);
    }

    #[test]
    fn pic_adc_samples_channels_and_adfm_alignment() {
        let mut dev = Device::from_xml("p", PIC_ADC_XML).unwrap();
        dev.set_pin_volt("PORTA0", 2.5);
        dev.set_pin_volt("PORTA1", 5.0);

        // ADFM = 1 (right-justified) in ADCON1
        dev.poke_reg(0x009F, 0x80);
        // ADON (0x01) | GO/DONE (0x04) | CH0 (0)
        dev.poke_reg(0x001F, 0x05);
        assert_eq!(dev.ram(0x001F) & 0x04, 0, "GO/DONE should clear");
        assert_ne!(dev.ram(0x000C) & 0x40, 0, "ADIF in PIR1 should set");
        let val = u16::from(dev.ram(0x009E)) | (u16::from(dev.ram(0x001E)) << 8);
        assert_eq!(val, 512);

        // ADFM = 0 (left-justified) on CH1 (PORTA1 = 5.0V -> 1023 << 6)
        dev.poke_reg(0x009F, 0x00);
        dev.poke_reg(0x001F, 0x05 | (1 << 3));
        let raw = u16::from(dev.ram(0x009E)) | (u16::from(dev.ram(0x001E)) << 8);
        assert_eq!(raw >> 6, 1023);
    }
}
