//! C++ `McuTimer` overflow plus PIC0 / AVR 8-bit / 8051 configure,
//! AVR WGM / OC units / IC capture.

use crate::dataspace::{DataSpace, RegBits};
use crate::desc::{CoreKind, TimerSpec};
use crate::icunit::IcUnit;
use crate::interrupts::{IntCallback, Interrupts};
use crate::ocunit::{OcAct, OcUnit};
use crate::port::Port;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerKind {
    Generic,
    Pic0,
    Pic1,
    Pic2,
    Avr8,
    Avr16,
    Avr801,
    Avr810,
    I51,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WgmMode {
    Norm = 0,
    Phas = 1,
    Ctc = 2,
    Fast = 3,
}

#[derive(Clone, Debug)]
pub struct Timer {
    #[allow(dead_code)]
    pub name: String,
    kind: TimerKind,
    pub running: bool,
    ext_clock: bool,
    clk_edge: u8,
    clk_state: bool,
    clock_pin: Option<String>,
    clk_gpio: Option<(usize, usize)>,
    count_l: Option<u16>,
    count_h: Option<u16>,
    top_l: Option<u16>,
    top_h: Option<u16>,
    top1_l: Option<u16>,
    top1_h: Option<u16>,
    count_val: u32,
    count_start: u32,
    max_count: u32,
    ovf_match: u32,
    ovf_period: u32,
    prescaler: u16,
    pr_index: u8,
    presc_list: Vec<u16>,
    pr_sel: RegBits,
    ps_per_tick: u64,
    pub remain_ps: Option<u64>,
    interrupt: Option<usize>,
    mode: u8,
    bidirec: bool,
    reverse: bool,
    use_icr: bool,
    wgm_mode: WgmMode,
    wgm10_val: u8,
    wgm32_val: u8,
    wgm10: RegBits,
    wgm32: RegBits,
    foc_a: RegBits,
    foc_b: RegBits,
    foc_c: RegBits,
    pub oc_units: Vec<OcUnit>,
    pub ic_unit: Option<IcUnit>,
    // PIC0 OPTION bits
    t0cs: RegBits,
    t0se: RegBits,
    psa: RegBits,
    ps_bits: RegBits,
    // PIC2
    tmr2on: RegBits,
    t2ckps: RegBits,
    toutps: RegBits,
    // PIC TIMER1
    t1ckps: RegBits,
    tmr1on: RegBits,
    tmr1cs: RegBits,
    pic1_cs2: bool,
    // Tinyx5 T1
    pwm1a: RegBits,
    pwm1b: RegBits,
    ctc1: RegBits,
    psr1: RegBits,
    // I51
    txm: RegBits,
    ctx: RegBits,
    gate: bool,
    tr_enabled: bool,
    number: i32,
}

impl Timer {
    pub fn from_spec(
        spec: &TimerSpec,
        data: &DataSpace,
        core: CoreKind,
        ints: &Interrupts,
    ) -> Self {
        let kind = match core {
            CoreKind::I51 => TimerKind::I51,
            CoreKind::Avr if spec.type_id / 10 == 16 => TimerKind::Avr16,
            CoreKind::Avr if spec.type_id == 801 => TimerKind::Avr801,
            CoreKind::Avr if spec.type_id == 810 => TimerKind::Avr810,
            CoreKind::Avr => TimerKind::Avr8,
            CoreKind::Pic12 | CoreKind::Pic14 if spec.type_id == 820 => TimerKind::Pic2,
            CoreKind::Pic12 | CoreKind::Pic14 if spec.type_id / 10 == 16 => TimerKind::Pic1,
            CoreKind::Pic12 | CoreKind::Pic14 => TimerKind::Pic0,
            _ => TimerKind::Generic,
        };
        // C++ types 800/801/810/82x are 8-bit; 160/161 are 16-bit.
        let mut max_count = if spec.type_id / 10 == 16 || spec.counter.len() > 1 {
            0xFFFF
        } else {
            0xFF
        };
        if kind == TimerKind::I51 {
            max_count = 0x1FFF;
        }
        let presc_list = parse_prescalers(&spec.prescalers);
        let n = spec
            .name
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(0) as i32;
        let oc_units: Vec<OcUnit> = spec
            .oc_units
            .iter()
            .map(|s| OcUnit::from_spec(s, data, ints))
            .collect();
        let ic_unit = spec
            .ic_unit
            .as_ref()
            .map(|s| IcUnit::from_spec(s, data, ints));
        let mut top_l = spec.top_reg0.first().and_then(|n| data.reg_addr(n));
        let mut top_h = spec.top_reg0.get(1).and_then(|n| data.reg_addr(n));
        if top_l.is_none() {
            if let Some(oca) = spec.oc_units.iter().find(|o| o.name.ends_with('A')) {
                top_l = oca.ocreg.first().and_then(|n| data.reg_addr(n));
                top_h = oca.ocreg.get(1).and_then(|n| data.reg_addr(n));
            }
        }
        let (top1_l, top1_h) = if kind == TimerKind::Avr16 {
            (
                data.reg_addr(&format!("ICR{n}L")),
                data.reg_addr(&format!("ICR{n}H")),
            )
        } else {
            (None, None)
        };
        let mut t = Self {
            name: spec.name.clone(),
            kind,
            running: false,
            ext_clock: false,
            clk_edge: 1,
            clk_state: false,
            clock_pin: spec.clock_pin.first().cloned(),
            clk_gpio: None,
            count_l: spec.counter.first().and_then(|n| data.reg_addr(n)),
            count_h: spec.counter.get(1).and_then(|n| data.reg_addr(n)),
            top_l,
            top_h,
            top1_l,
            top1_h,
            count_val: 0,
            count_start: 0,
            max_count,
            ovf_match: max_count,
            ovf_period: max_count + 1,
            prescaler: 1,
            pr_index: 0,
            presc_list,
            pr_sel: data.get_reg_bits(&spec.pr_select),
            ps_per_tick: 1,
            remain_ps: None,
            interrupt: ints.get(&spec.interrupt),
            mode: 0,
            bidirec: false,
            reverse: false,
            use_icr: false,
            wgm_mode: WgmMode::Norm,
            wgm10_val: 0,
            wgm32_val: 0,
            wgm10: data.get_reg_bits(&format!("WGM{n}0,WGM{n}1")),
            wgm32: if kind == TimerKind::Avr16 {
                data.get_reg_bits(&format!("WGM{n}2,WGM{n}3"))
            } else {
                data.get_reg_bits(&format!("WGM{n}2"))
            },
            foc_a: data.get_reg_bits(&format!("FOC{n}A")),
            foc_b: data.get_reg_bits(&format!("FOC{n}B")),
            foc_c: data.get_reg_bits(&format!("FOC{n}C")),
            oc_units,
            ic_unit,
            t0cs: data.get_reg_bits("T0CS"),
            t0se: data.get_reg_bits("T0SE"),
            psa: data.get_reg_bits("PSA"),
            ps_bits: data.get_reg_bits("PS0,PS1,PS2"),
            tmr2on: data.get_reg_bits("TMR2ON"),
            t2ckps: data.get_reg_bits("T2CKPS0,T2CKPS1"),
            toutps: data.get_reg_bits("TOUTPS0,TOUTPS1,TOUTPS2,TOUTPS3"),
            t1ckps: data.get_reg_bits("T1CKPS0,T1CKPS1"),
            tmr1on: data.get_reg_bits("TMR1ON"),
            tmr1cs: if spec.type_id == 161 {
                data.get_reg_bits("TMR1CS0,TMR1CS1")
            } else {
                data.get_reg_bits("TMR1CS")
            },
            pic1_cs2: spec.type_id == 161,
            pwm1a: data.get_reg_bits("PWM1A"),
            pwm1b: data.get_reg_bits("PWM1B"),
            ctc1: data.get_reg_bits("CTC1"),
            psr1: data.get_reg_bits("PSR1"),
            txm: data.get_reg_bits(&format!("T{n}M0,T{n}M1")),
            ctx: data.get_reg_bits(&format!("C/T{n}")),
            gate: false,
            tr_enabled: false,
            number: n,
        };
        if t.kind == TimerKind::Pic0 {
            t.running = true;
        }
        t
    }

    pub fn initialize(&mut self, ports: &mut [Port]) {
        self.running = self.kind == TimerKind::Pic0;
        self.ext_clock = false;
        self.count_val = 0;
        self.count_start = 0;
        self.ovf_match = self.max_count;
        self.ovf_period = self.max_count + 1;
        self.prescaler = 1;
        self.pr_index = 0;
        self.remain_ps = None;
        self.mode = 0;
        self.tr_enabled = false;
        self.gate = false;
        self.bidirec = false;
        self.reverse = false;
        self.use_icr = false;
        self.wgm_mode = WgmMode::Norm;
        self.wgm10_val = 0;
        self.wgm32_val = 0;
        if self.kind == TimerKind::I51 {
            self.ovf_match = 0x1FFF;
            self.ovf_period = 0x2000;
        }
        for oc in &mut self.oc_units {
            oc.initialize(ports);
        }
        if let Some(ic) = &mut self.ic_unit {
            ic.initialize();
        }
    }

    pub fn enable(&mut self, en: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        if self.kind == TimerKind::I51 {
            self.tr_enabled = en != 0;
            let e = self.tr_enabled && !self.gate;
            self.set_running(e, data, ports, ps_inst);
            return;
        }
        self.set_running(en != 0, data, ports, ps_inst);
    }

    fn set_running(&mut self, e: bool, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        if self.running == e {
            return;
        }
        if !e {
            self.updt_count(data);
        }
        self.running = e;
        self.updt_cycles(data, ports, ps_inst);
    }

    pub fn count_write_l(
        &mut self,
        val: u8,
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        self.updt_count(data);
        if let Some(a) = self.count_l {
            data.set(a, val);
        }
        self.count_val = (self.count_val & 0xFFFF_FF00) | u32::from(val);
        self.updt_cycles(data, ports, ps_inst);
    }

    pub fn count_write_h(
        &mut self,
        val: u8,
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        self.updt_count(data);
        if let Some(a) = self.count_h {
            data.set(a, val);
        }
        self.count_val = (self.count_val & 0x0000_00FF) | (u32::from(val) << 8);
        self.updt_cycles(data, ports, ps_inst);
    }

    pub fn updt_count(&mut self, data: &mut DataSpace) {
        if !self.running {
            return;
        }
        if self.ext_clock {
            self.write_count_regs(data);
            return;
        }
        // C++ `calcCounter` from time-to-ovf. `remain_ps` is that time.
        if let Some(time2ovf) = self.remain_ps {
            let cycles2ovf = if self.ps_per_tick == 0 {
                0
            } else {
                time2ovf / self.ps_per_tick
            };
            if self.ovf_match as u64 > cycles2ovf {
                self.count_val = self.ovf_match - cycles2ovf as u32;
                let offset = if self.ps_per_tick == 0 {
                    0
                } else {
                    time2ovf % self.ps_per_tick
                };
                if offset != 0 {
                    self.count_val = self.count_val.saturating_sub(1);
                }
            }
        }
        let shown = if self.reverse {
            self.ovf_match.saturating_sub(self.count_val)
        } else {
            self.count_val
        };
        if let Some(a) = self.count_l {
            data.set(a, shown as u8);
        }
        if let Some(a) = self.count_h {
            data.set(a, (shown >> 8) as u8);
        }
    }

    fn write_count_regs(&self, data: &mut DataSpace) {
        if let Some(a) = self.count_l {
            data.set(a, self.count_val as u8);
        }
        if let Some(a) = self.count_h {
            data.set(a, (self.count_val >> 8) as u8);
        }
    }

    pub fn top_reg0_changed(
        &mut self,
        val: u8,
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        let _ = val;
        self.updt_count(data);
        if matches!(self.kind, TimerKind::Avr8 | TimerKind::Avr16) {
            self.updt_wgm(data, ports, ps_inst);
            if let Some(i) = self.oc_index_a() {
                let v = self.top_l.map(|a| data.get(a)).unwrap_or(val);
                self.oc_units[i].ocr_write_l(v);
            }
            if self.running {
                self.schedule(ports, ps_inst);
            }
            return;
        }
        if self.kind == TimerKind::Avr810 {
            self.update_avr810_mode(data, ports, ps_inst);
            return;
        }
        if let Some(a) = self.top_l {
            let mut top = u32::from(data.get(a));
            if let Some(h) = self.top_h {
                top |= u32::from(data.get(h)) << 8;
            }
            if top > 0 {
                self.ovf_match = top;
                self.ovf_period = top + 1;
            }
        }
        self.schedule(ports, ps_inst);
    }

    pub fn icr_changed(&mut self, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        if self.kind == TimerKind::Avr16 {
            self.updt_wgm(data, ports, ps_inst);
        }
    }

    pub fn ocr_write_l(
        &mut self,
        oc: usize,
        val: u8,
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        self.updt_count(data);
        if let Some(u) = self.oc_units.get_mut(oc) {
            u.ocr_write_l(val);
        }
        if self.running {
            self.schedule(ports, ps_inst);
        }
    }

    pub fn ocr_write_h(&mut self, oc: usize, val: u8) {
        if let Some(u) = self.oc_units.get_mut(oc) {
            u.ocr_write_h(val);
        }
    }

    pub fn oc_configure(&mut self, oc: usize, val: u8, ports: &mut [Port]) {
        if let Some(u) = self.oc_units.get_mut(oc) {
            u.configure(val, ports);
        }
    }

    pub fn ic_configure(&mut self, val: u8) {
        if let Some(ic) = &mut self.ic_unit {
            ic.configure(val);
        }
    }

    pub fn configure_a(&mut self, val: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        match self.kind {
            TimerKind::Pic0 => self.pic0_option(val, ports, ps_inst),
            TimerKind::Pic1 => self.pic1_t1con(val, data, ports, ps_inst),
            TimerKind::Pic2 => self.pic2_t2con(val, data, ports, ps_inst),
            TimerKind::I51 => self.i51_tmod(val, ports, ps_inst),
            TimerKind::Avr8 | TimerKind::Avr16 => {
                for oc in &mut self.oc_units {
                    oc.configure(val, ports);
                }
                self.wgm10_val = if self.wgm10.mask == 0 {
                    0
                } else {
                    self.wgm10.val(val)
                };
                self.updt_wgm(data, ports, ps_inst);
            }
            TimerKind::Avr801 => self.avr_clock(val, data, ports, ps_inst),
            TimerKind::Avr810 => self.avr810_tccr1(val, data, ports, ps_inst),
            TimerKind::Generic => {}
        }
    }

    pub fn configure_b(&mut self, val: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        match self.kind {
            TimerKind::Avr8 | TimerKind::Avr16 => {
                self.avr_clock(val, data, ports, ps_inst);
                let wgm32 = if self.wgm32.mask == 0 {
                    0
                } else {
                    self.wgm32.val(val) << 2
                };
                if wgm32 != self.wgm32_val {
                    self.wgm32_val = wgm32;
                    self.updt_wgm(data, ports, ps_inst);
                }
                if self.kind == TimerKind::Avr8 {
                    self.force_output(val, data, ports, ps_inst);
                }
            }
            TimerKind::Avr810 => self.avr810_gtccr(val, data, ports, ps_inst),
            TimerKind::Pic2 => {
                self.ovf_match = u32::from(val);
                self.ovf_period = u32::from(val) + 1;
                self.schedule(ports, ps_inst);
            }
            _ => {}
        }
    }

    pub fn configure_c(&mut self, val: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        if self.kind == TimerKind::Avr16 {
            self.force_output(val, data, ports, ps_inst);
        }
    }

    fn force_output(&mut self, val: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let mut cleared = 0u8;
        let ps = self.ps_per_tick.max(1);
        for (bits, suffix) in [(self.foc_a, 'A'), (self.foc_b, 'B'), (self.foc_c, 'C')] {
            if bits.mask != 0 && val & bits.mask != 0 {
                if let Some(i) = self.oc_index_ending(suffix) {
                    self.oc_units[i].com_match(ports, ps);
                }
                cleared |= bits.mask;
            }
        }
        if cleared != 0 {
            data.reg_override = Some(val & !cleared);
        }
        let _ = ps_inst;
    }

    fn oc_index_a(&self) -> Option<usize> {
        self.oc_index_ending('A').or_else(|| {
            if self.oc_units.is_empty() {
                None
            } else {
                Some(0)
            }
        })
    }

    fn oc_index_ending(&self, ch: char) -> Option<usize> {
        self.oc_units.iter().position(|o| o.name.ends_with(ch))
    }

    fn pic0_option(&mut self, option: u8, ports: &mut [Port], ps_inst: u64) {
        let ps = self.ps_bits.val(option);
        if self.psa.mask != 0 && option & self.psa.mask != 0 {
            self.prescaler = 1;
        } else {
            self.set_presc_index(ps);
        }
        self.ps_per_tick = u64::from(self.prescaler.max(1)) * ps_inst.max(1);
        self.clk_edge = self.t0se.val(option);
        let mode = self.t0cs.val(option);
        if mode != self.mode {
            self.mode = mode;
            self.enable_ext_clock(mode != 0, ports, ps_inst);
        } else {
            self.schedule(ports, ps_inst);
        }
    }

    fn pic2_t2con(&mut self, t2con: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let ckps = self.t2ckps.val(t2con);
        self.prescaler = match ckps {
            1 => 4,
            2 | 3 => 16,
            _ => 1,
        };
        let postc = if self.toutps.mask == 0 {
            0
        } else {
            self.toutps.val(t2con)
        };
        self.ps_per_tick = u64::from(self.prescaler) * u64::from(postc + 1) * ps_inst.max(1);
        let on = self.tmr2on.mask == 0 || t2con & self.tmr2on.mask != 0;
        self.set_running(on, data, ports, ps_inst);
    }

    fn pic1_t1con(&mut self, t1con: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let ps = if self.t1ckps.mask == 0 {
            0
        } else {
            self.t1ckps.val(t1con)
        };
        self.set_presc_index(ps);
        self.mode = if self.tmr1cs.mask == 0 {
            0
        } else {
            self.tmr1cs.val(t1con)
        };
        self.ps_per_tick = u64::from(self.prescaler.max(1)) * ps_inst.max(1);
        if self.pic1_cs2 {
            match self.mode {
                1 => self.ps_per_tick = (self.ps_per_tick / 4).max(1),
                2 => self.enable_ext_clock(true, ports, ps_inst),
                _ => self.enable_ext_clock(false, ports, ps_inst),
            }
        } else {
            self.enable_ext_clock(self.mode == 1, ports, ps_inst);
        }
        let on = self.tmr1on.mask != 0 && t1con & self.tmr1on.mask != 0;
        self.set_running(on, data, ports, ps_inst);
    }

    /// C++ `AvrTimer810::configureA` — TCCR1: CTC1, PWM1A, COM1A, CS.
    fn avr810_tccr1(&mut self, tccr1: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let mut mode = self.mode & !0b0000_0101;
        if self.ctc1.mask != 0 && tccr1 & self.ctc1.mask != 0 {
            mode |= 1 << 2;
        }
        let pwm = self.pwm1a.mask != 0 && tccr1 & self.pwm1a.mask != 0;
        if pwm {
            mode |= 1;
        }
        if let Some(i) = self.oc_index_ending('A') {
            self.oc_units[i].configure(tccr1, ports);
            self.update_avr810_oc(i, pwm, "PORTB0", ports);
        }
        if mode != self.mode {
            self.mode = mode;
            self.update_avr810_mode(data, ports, ps_inst);
        }
        self.avr_clock(tccr1, data, ports, ps_inst);
    }

    /// C++ `AvrTimer810::configureB` — GTCCR: PWM1B, COM1B, FOC, PSR1.
    fn avr810_gtccr(&mut self, gtccr: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let mut mode = self.mode & !0b0000_0010;
        let pwm = self.pwm1b.mask != 0 && gtccr & self.pwm1b.mask != 0;
        if pwm {
            mode |= 1 << 1;
        }
        if let Some(i) = self.oc_index_ending('B') {
            self.oc_units[i].configure(gtccr, ports);
            self.update_avr810_oc(i, pwm, "PORTB3", ports);
        }
        self.force_output(gtccr, data, ports, ps_inst);
        if mode != self.mode {
            self.mode = mode;
            self.update_avr810_mode(data, ports, ps_inst);
        }
        let mut o = data.reg_override.unwrap_or(gtccr);
        if self.psr1.mask != 0 {
            o &= !self.psr1.mask;
        }
        data.reg_override = Some(o);
    }

    fn update_avr810_oc(&mut self, oc: usize, pwm: bool, inv: &str, ports: &mut [Port]) {
        let Some(unit) = self.oc_units.get_mut(oc) else {
            return;
        };
        let mut com = OcAct::from_u8(unit.mode);
        let mut tov = OcAct::Non;
        if com == OcAct::Tog && pwm {
            com = OcAct::Clr;
            unit.set_pin_inv(Some(inv), ports);
        } else {
            unit.set_pin_inv(None, ports);
        }
        if com == OcAct::Clr {
            tov = OcAct::Set;
        } else if com == OcAct::Set {
            tov = OcAct::Clr;
        }
        unit.set_oc_acts(com, tov);
    }

    fn update_avr810_mode(&mut self, data: &DataSpace, ports: &mut [Port], ps_inst: u64) {
        let ovf = if self.mode != 0 {
            u32::from(self.top_l.map(|a| data.get(a)).unwrap_or(0xFF))
        } else {
            0xFF
        };
        self.ovf_match = ovf;
        self.ovf_period = ovf + 1;
        self.schedule(ports, ps_inst);
    }

    fn i51_tmod(&mut self, tmod: u8, ports: &mut [Port], ps_inst: u64) {
        let mode = self.txm.val(tmod);
        if mode != self.mode {
            self.mode = mode;
            self.ovf_match = match mode {
                0 => 0x1FFF,
                1 => 0xFFFF,
                _ => 0x00FF,
            };
            self.ovf_period = self.ovf_match + 1;
        }
        let ext = self.ctx.mask != 0 && tmod & self.ctx.mask != 0;
        self.enable_ext_clock(ext, ports, ps_inst);
        self.schedule(ports, ps_inst);
    }

    fn avr_clock(&mut self, tccrb: u8, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let pr = if self.pr_sel.mask == 0 {
            tccrb & 7
        } else {
            self.pr_sel.val(tccrb)
        };
        if pr != self.pr_index {
            self.set_presc_index(pr);
            if self.pr_index != 0 {
                self.configure_clock(ports, ps_inst);
            }
            self.set_running(self.pr_index != 0, data, ports, ps_inst);
        }
    }

    fn configure_clock(&mut self, ports: &mut [Port], ps_inst: u64) {
        if (self.pr_index as usize) < self.presc_list.len() {
            self.prescaler = self.presc_list[self.pr_index as usize];
        }
        if self.prescaler >= 0x8000 {
            self.clk_edge = (self.prescaler & 1) as u8;
            self.prescaler = 1;
            self.enable_ext_clock(true, ports, ps_inst);
        } else {
            self.enable_ext_clock(false, ports, ps_inst);
        }
        self.ps_per_tick = u64::from(self.prescaler.max(1)) * ps_inst.max(1);
    }

    fn set_presc_index(&mut self, p: u8) {
        self.pr_index = p;
        if (p as usize) < self.presc_list.len() {
            self.prescaler = self.presc_list[p as usize];
        } else {
            self.prescaler = 1 << p.min(7);
        }
    }

    fn enable_ext_clock(&mut self, en: bool, ports: &mut [Port], ps_inst: u64) {
        if self.ext_clock == en {
            return;
        }
        self.ext_clock = en;
        self.schedule(ports, ps_inst);
    }

    fn updt_cycles(&mut self, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        if self.kind == TimerKind::I51 {
            self.i51_updt_cycles(data);
        }
        self.schedule(ports, ps_inst);
    }

    fn i51_updt_cycles(&mut self, data: &DataSpace) {
        let l = self.count_l.map(|a| data.get(a)).unwrap_or(0);
        let h = self.count_h.map(|a| data.get(a)).unwrap_or(0);
        match self.mode {
            0 => {
                self.count_val = (u32::from(h) << 5) | u32::from(l & 0x1F);
                self.count_start = 0;
            }
            1 => {
                self.count_val = (u32::from(h) << 8) | u32::from(l);
                self.count_start = 0;
            }
            2 => {
                self.count_val = u32::from(h);
                self.count_start = u32::from(h);
            }
            _ => {
                self.count_val = if self.number == 0 {
                    u32::from(l)
                } else {
                    u32::from(h)
                };
                self.count_start = 0;
            }
        }
    }

    fn updt_wgm(&mut self, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        match self.kind {
            TimerKind::Avr8 => self.updt_wgm8(data, ports, ps_inst),
            TimerKind::Avr16 => self.updt_wgm16(data, ports, ps_inst),
            _ => {}
        }
    }

    fn updt_wgm8(&mut self, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        self.wgm_mode = match self.wgm10_val {
            1 => WgmMode::Phas,
            2 => WgmMode::Ctc,
            3 => WgmMode::Fast,
            _ => WgmMode::Norm,
        };
        if self.wgm_mode != WgmMode::Phas {
            self.reverse = false;
        }
        self.configure_oc_units(self.wgm32_val == 0);
        let mut ovf = 0xFFu32;
        let ocra = self.top_l.map(|a| u32::from(data.get(a))).unwrap_or(0xFF);
        if self.wgm_mode == WgmMode::Ctc
            || (self.wgm32_val != 0
                && (self.wgm_mode == WgmMode::Phas || self.wgm_mode == WgmMode::Fast))
        {
            ovf = ocra;
        }
        if self.ovf_match != ovf {
            self.ovf_match = ovf;
            self.ovf_period = if self.bidirec {
                self.ovf_match
            } else {
                self.ovf_match + 1
            };
            self.schedule(ports, ps_inst);
        }
        if let Some(i) = self.oc_index_a() {
            self.oc_units[i].ocr_write_l(ocra as u8);
        }
    }

    fn updt_wgm16(&mut self, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        let wgm = self.wgm32_val + self.wgm10_val;
        let ocra = self.ocr_a16(data);
        let icr = self.icr16(data);
        let mut mode = WgmMode::Norm;
        let mut ovf = 0xFFFFu32;
        let mut mask = 0xFFFFu16;
        self.use_icr = false;
        match wgm {
            1 => {
                mode = WgmMode::Phas;
                ovf = 0x00FF;
                mask = 0x00FF;
            }
            2 => {
                mode = WgmMode::Phas;
                ovf = 0x01FF;
                mask = 0x01FF;
            }
            3 => {
                mode = WgmMode::Phas;
                ovf = 0x03FF;
                mask = 0x03FF;
            }
            4 => {
                mode = WgmMode::Ctc;
                ovf = ocra;
            }
            5 => {
                mode = WgmMode::Fast;
                ovf = 0x00FF;
                mask = 0x00FF;
            }
            6 => {
                mode = WgmMode::Fast;
                ovf = 0x01FF;
                mask = 0x01FF;
            }
            7 => {
                mode = WgmMode::Fast;
                ovf = 0x03FF;
                mask = 0x03FF;
            }
            8 => {
                mode = WgmMode::Phas;
                self.use_icr = true;
            }
            9 => {
                mode = WgmMode::Phas;
                ovf = ocra;
            }
            10 => {
                mode = WgmMode::Phas;
                self.use_icr = true;
            }
            11 => {
                mode = WgmMode::Phas;
                ovf = ocra;
            }
            12 => {
                mode = WgmMode::Ctc;
                self.use_icr = true;
            }
            14 => {
                mode = WgmMode::Fast;
                self.use_icr = true;
            }
            15 => {
                mode = WgmMode::Fast;
                ovf = ocra;
            }
            _ => {}
        }
        if self.use_icr {
            ovf = icr;
        }
        if let Some(ic) = &mut self.ic_unit {
            ic.enable(!self.use_icr);
        }
        self.wgm_mode = mode;
        if self.wgm_mode != WgmMode::Phas {
            self.reverse = false;
        }
        for oc in &mut self.oc_units {
            oc.set_ocr_mask(mask, data);
        }
        let shedule = self.ovf_match != ovf;
        self.ovf_match = ovf;
        let wgm3 = self.wgm32_val & (1 << 3) == 0;
        self.configure_oc_units(wgm3);
        if shedule {
            self.schedule(ports, ps_inst);
        }
    }

    fn ocr_a16(&self, data: &DataSpace) -> u32 {
        let l = self.top_l.map(|a| u32::from(data.get(a))).unwrap_or(0);
        let h = self.top_h.map(|a| u32::from(data.get(a))).unwrap_or(0);
        l | (h << 8)
    }

    fn icr16(&self, data: &DataSpace) -> u32 {
        let l = self.top1_l.map(|a| u32::from(data.get(a))).unwrap_or(0);
        let h = self.top1_h.map(|a| u32::from(data.get(a))).unwrap_or(0);
        l | (h << 8)
    }

    fn configure_oc_units(&mut self, wgm3: bool) {
        self.bidirec = false;
        let modes: Vec<OcAct> = self
            .oc_units
            .iter()
            .map(|o| OcAct::from_u8(o.mode))
            .collect();
        let mut acts: Vec<(OcAct, OcAct)> = modes.iter().map(|&c| (c, OcAct::Non)).collect();
        if self.wgm_mode == WgmMode::Phas {
            for (i, (com, _)) in acts.iter_mut().enumerate() {
                let tog_off = match self.oc_units.get(i).map(|o| o.name.chars().last()) {
                    Some(Some('A')) => wgm3,
                    _ => true,
                };
                if *com == OcAct::Tog && tog_off {
                    *com = OcAct::Non;
                }
            }
            self.bidirec = true;
        } else if self.wgm_mode == WgmMode::Fast {
            for (i, (com, tov)) in acts.iter_mut().enumerate() {
                let tog_off = match self.oc_units.get(i).map(|o| o.name.chars().last()) {
                    Some(Some('A')) => wgm3,
                    _ => true,
                };
                if *com == OcAct::Tog && tog_off {
                    *com = OcAct::Non;
                } else if *com == OcAct::Clr {
                    *tov = OcAct::Set;
                } else if *com == OcAct::Set {
                    *tov = OcAct::Clr;
                }
            }
        }
        for (oc, (com, tov)) in self.oc_units.iter_mut().zip(acts) {
            oc.set_oc_acts(com, tov);
        }
        if self.bidirec {
            self.ovf_period = self.ovf_match;
        } else {
            self.ovf_period = self.ovf_match + 1;
        }
    }

    pub fn schedule(&mut self, ports: &mut [Port], ps_inst: u64) {
        let _ = ports;
        if self.ps_per_tick == 0 {
            self.ps_per_tick = u64::from(self.prescaler.max(1)) * ps_inst.max(1);
        }
        if !self.running || self.ext_clock {
            self.remain_ps = None;
            for oc in &mut self.oc_units {
                oc.shedule_events(
                    self.ovf_match,
                    self.count_val,
                    self.reverse,
                    true,
                    self.ps_per_tick,
                    0,
                );
            }
            return;
        }
        let mut ovf_period = u64::from(self.ovf_period);
        if self.count_val > self.ovf_match {
            ovf_period += u64::from(self.max_count);
        }
        let ticks = ovf_period.saturating_sub(u64::from(self.count_val));
        self.remain_ps = Some((ticks * self.ps_per_tick).max(1));
        for oc in &mut self.oc_units {
            oc.shedule_events(
                self.ovf_match,
                self.count_val,
                self.reverse,
                false,
                self.ps_per_tick,
                0,
            );
        }
    }

    pub fn next_remain(&self) -> Option<u64> {
        let mut n = self.remain_ps;
        for oc in &self.oc_units {
            if let Some(r) = oc.remain_ps {
                n = Some(n.map(|x| x.min(r)).unwrap_or(r));
            }
        }
        n
    }

    #[inline]
    pub fn advance(
        &mut self,
        elapsed: u64,
        data: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &mut [Port],
        cbs: &mut Vec<IntCallback>,
    ) {
        if !self.running {
            return;
        }
        let mut oc_fire = [0usize; 4];
        let mut oc_count = 0;
        for (i, oc) in self.oc_units.iter_mut().enumerate() {
            if let Some(r) = oc.remain_ps {
                if elapsed >= r {
                    if oc_count < oc_fire.len() {
                        oc_fire[oc_count] = i;
                        oc_count += 1;
                    }
                } else {
                    oc.remain_ps = Some(r - elapsed);
                }
            }
        }
        let ps = self.ps_per_tick.max(1);
        let mut reset = false;
        for idx in 0..oc_count {
            let i = oc_fire[idx];
            let (c, r) = self.oc_units[i].run_event(ports, ints, data, ps);
            if !c.is_empty() {
                cbs.extend(c);
            }
            reset |= r;
        }
        if reset {
            self.reset_timer(data, ports, ps);
        }
        let Some(r) = self.remain_ps else {
            return;
        };
        if elapsed < r {
            self.remain_ps = Some(r - elapsed);
            return;
        }
        let ovf = self.overflow(data, ints, ports);
        if !ovf.is_empty() {
            cbs.extend(ovf);
        }
    }

    pub fn overflow(
        &mut self,
        data: &mut DataSpace,
        ints: &mut Interrupts,
        ports: &mut [Port],
    ) -> Vec<IntCallback> {
        if !self.running {
            return Vec::new();
        }
        let ps = self.ps_per_tick.max(1);
        for oc in &mut self.oc_units {
            oc.tov(ports, ps);
        }
        self.count_val = self.count_start;
        self.write_count_regs(data);
        if self.bidirec {
            self.reverse = !self.reverse;
        }
        let mut cbs = Vec::new();
        if self.kind == TimerKind::Avr16 && self.wgm_mode == WgmMode::Fast && self.use_icr {
            if let Some(ic) = &self.ic_unit {
                let _ = ic;
            }
            // C++ raises the IC interrupt on Fast PWM + ICR top overflow.
            if let Some(ic) = &self.ic_unit {
                if let Some(i) = ic.interrupt {
                    cbs.extend(ints.raise(i, data));
                }
            }
        }
        if !self.reverse {
            if let Some(i) = self.interrupt {
                cbs.extend(ints.raise(i, data));
            }
        }
        self.schedule(ports, ps);
        cbs
    }

    pub fn clock_pin_changed(
        &mut self,
        ports: &mut [Port],
        data: &mut DataSpace,
        ints: &mut Interrupts,
    ) {
        let ic_active = self.ic_unit.as_ref().is_some_and(|ic| ic.enabled);
        if !self.ext_clock && !ic_active {
            return;
        }
        if self.ext_clock && self.running {
            if self.clk_gpio.is_none() {
                if let Some(name) = &self.clock_pin {
                    self.clk_gpio = crate::port::find_gpio(ports, name);
                }
            }
            if let Some((pi, pin_i)) = self.clk_gpio {
                if let Some(state) = ports
                    .get(pi)
                    .and_then(|p| p.pins.get(pin_i))
                    .map(|p| p.inp_state)
                {
                    if self.clk_state != state {
                        let edge = if self.clk_edge == 1 {
                            state && !self.clk_state
                        } else {
                            !state && self.clk_state
                        };
                        self.clk_state = state;
                        if edge {
                            self.count_val = self.count_val.wrapping_add(1);
                            let count = self.count_val as u16;
                            let ps = self.ps_per_tick.max(1);
                            let mut reset = false;
                            for oc in &mut self.oc_units {
                                let (c, r) = oc.clock_step(count, ports, ints, data, ps);
                                let _ = c;
                                reset |= r;
                            }
                            if reset {
                                self.reset_timer(data, ports, ps);
                            }
                            if self.count_val == self.ovf_match + 1 {
                                self.overflow(data, ints, ports);
                            }
                        }
                    }
                }
            }
        }
        if ic_active {
            self.ic_pin_changed(ports, data, ints);
        }
    }

    pub fn ic_pin_changed(&mut self, ports: &[Port], data: &mut DataSpace, ints: &mut Interrupts) {
        if self.ic_unit.as_ref().is_some_and(|ic| ic.enabled) {
            let count = self.get_count(data);
            if let Some(ic) = &mut self.ic_unit {
                ic.pin_changed(ports, count, data, ints);
            }
        }
    }

    pub fn get_count(&mut self, data: &mut DataSpace) -> u32 {
        self.updt_count(data);
        self.count_val
    }

    pub fn add_oc_unit(&mut self, oc: OcUnit) -> usize {
        let i = self.oc_units.len();
        self.oc_units.push(oc);
        i
    }

    pub fn set_ic_unit(&mut self, ic: IcUnit) {
        self.ic_unit = Some(ic);
    }

    /// C++ `McuTimer::resetTimer` (PIC CCP special event).
    pub fn reset_timer(&mut self, data: &mut DataSpace, ports: &mut [Port], ps_inst: u64) {
        self.count_val = self.count_start;
        self.write_count_regs(data);
        self.schedule(ports, ps_inst);
    }

    pub fn oc_shedule(
        &mut self,
        oc: usize,
        data: &mut DataSpace,
        ports: &mut [Port],
        ps_inst: u64,
    ) {
        self.updt_count(data);
        if self.ps_per_tick == 0 {
            self.ps_per_tick = u64::from(self.prescaler.max(1)) * ps_inst.max(1);
        }
        if let Some(u) = self.oc_units.get_mut(oc) {
            u.shedule_events(
                self.ovf_match,
                self.count_val,
                self.reverse,
                self.ext_clock,
                self.ps_per_tick,
                0,
            );
        }
        let _ = ports;
    }
}

fn parse_prescalers(s: &str) -> Vec<u16> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|t| match t.trim() {
            "EXT_F" => 0x8000,
            "EXT_R" => 0x8001,
            other => other.parse().unwrap_or(0),
        })
        .collect()
}
