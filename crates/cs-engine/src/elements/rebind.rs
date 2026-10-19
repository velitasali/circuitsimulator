//! Prefix digital pin ids once a component has a placed id.
//!
//! `Part::to_element_kind` builds solver state with an empty id, so stored
//! `IoPin` ids look like `-in0`. `Circuit::add_comp` and
//! `Circuit::update_component_kind` call [`Kind::rebind_id`] with the real id.

use crate::digital::{IoPin, rebind_pin_id};

use super::Kind;

fn each(comp_id: &str, pins: &mut [IoPin]) {
    for pin in pins {
        pin.rebind_id(comp_id);
    }
}

fn one(comp_id: &str, pin: &mut IoPin) {
    pin.rebind_id(comp_id);
}

fn maybe(comp_id: &str, pin: &mut Option<IoPin>) {
    if let Some(pin) = pin {
        pin.rebind_id(comp_id);
    }
}

impl Kind {
    pub fn rebind_id(&mut self, comp_id: &str) {
        if comp_id.is_empty() {
            return;
        }
        match self {
            Kind::Gate(g) => {
                each(comp_id, &mut g.inputs);
                one(comp_id, &mut g.output);
                maybe(comp_id, &mut g.oe);
            }
            Kind::FlipFlop(f) => {
                one(comp_id, &mut f.d);
                one(comp_id, &mut f.j);
                one(comp_id, &mut f.k);
                one(comp_id, &mut f.t);
                one(comp_id, &mut f.set);
                one(comp_id, &mut f.rst);
                one(comp_id, &mut f.clk);
                one(comp_id, &mut f.q);
                one(comp_id, &mut f.qn);
            }
            Kind::Latch(l) => {
                each(comp_id, &mut l.inputs);
                each(comp_id, &mut l.outputs);
                one(comp_id, &mut l.clk);
                one(comp_id, &mut l.reset);
                one(comp_id, &mut l.oe);
            }
            Kind::McuPin(p) => one(comp_id, &mut p.pin),
            Kind::Mcu(m) => each(comp_id, &mut m.pins),
            Kind::QemuDevice(q) => {
                if q.uid.is_empty() {
                    q.uid = comp_id.to_string();
                }
                each(comp_id, &mut q.pins);
            }
            Kind::ScriptCpu(cpu) => each(comp_id, &mut cpu.pins),
            Kind::TestUnit(t) => {
                for pin in t.pins_mut() {
                    pin.rebind_id(comp_id);
                }
            }
            Kind::Hd44780(h) => {
                h.id = comp_id.to_string();
                for pin in h.pins_mut() {
                    pin.rebind_id(comp_id);
                }
            }
            Kind::Ssd1306(s) => {
                s.id = comp_id.to_string();
                one(comp_id, &mut s.pin_scl);
                one(comp_id, &mut s.pin_sda);
            }
            Kind::Mux(m) => {
                each(comp_id, &mut m.inputs);
                each(comp_id, &mut m.addr_pins);
                maybe(comp_id, &mut m.enable);
                one(comp_id, &mut m.output);
                one(comp_id, &mut m.out_inverted);
            }
            Kind::Demux(d) => {
                one(comp_id, &mut d.input);
                each(comp_id, &mut d.addr_pins);
                maybe(comp_id, &mut d.enable);
                each(comp_id, &mut d.outputs);
            }
            Kind::BcdToDec(b) => {
                each(comp_id, &mut b.inputs);
                each(comp_id, &mut b.outputs);
                one(comp_id, &mut b.oe);
            }
            Kind::DecToBcd(d) => {
                each(comp_id, &mut d.inputs);
                each(comp_id, &mut d.outputs);
                one(comp_id, &mut d.gs);
                one(comp_id, &mut d.eo);
                one(comp_id, &mut d.ei);
            }
            Kind::BcdTo7S(b) => {
                each(comp_id, &mut b.inputs);
                one(comp_id, &mut b.lt);
                one(comp_id, &mut b.rbi);
                one(comp_id, &mut b.bi_rbo);
                each(comp_id, &mut b.segments);
            }
            Kind::I2CToParallel(p) => {
                each(comp_id, &mut p.ports);
                one(comp_id, &mut p.int_pin);
                one(comp_id, &mut p.scl);
                one(comp_id, &mut p.sda);
            }
            Kind::Adc(a) => {
                each(comp_id, &mut a.outputs);
                one(comp_id, &mut a.soc);
                one(comp_id, &mut a.eoc);
                one(comp_id, &mut a.oe);
            }
            Kind::Dac(d) => each(comp_id, &mut d.inputs),
            Kind::Counter(c) => {
                each(comp_id, &mut c.outputs);
                one(comp_id, &mut c.clock);
                one(comp_id, &mut c.reset);
                one(comp_id, &mut c.enable);
                one(comp_id, &mut c.output);
            }
            Kind::BinCounter(b) => {
                each(comp_id, &mut b.outputs);
                one(comp_id, &mut b.clk_a);
                one(comp_id, &mut b.clk_b);
                one(comp_id, &mut b.r0_1);
                one(comp_id, &mut b.r0_2);
                one(comp_id, &mut b.r9_1);
                one(comp_id, &mut b.r9_2);
            }
            Kind::FullAdder(fa) | Kind::HalfAdder(fa) => {
                each(comp_id, &mut fa.a_inputs);
                each(comp_id, &mut fa.b_inputs);
                one(comp_id, &mut fa.ci);
                each(comp_id, &mut fa.sum_outputs);
                one(comp_id, &mut fa.co);
            }
            Kind::MagnitudeComp(mc) => {
                each(comp_id, &mut mc.a_inputs);
                each(comp_id, &mut mc.b_inputs);
                one(comp_id, &mut mc.cascade_gt);
                one(comp_id, &mut mc.cascade_eq);
                one(comp_id, &mut mc.cascade_lt);
                one(comp_id, &mut mc.out_gt);
                one(comp_id, &mut mc.out_eq);
                one(comp_id, &mut mc.out_lt);
            }
            Kind::ShiftReg(sr) => {
                each(comp_id, &mut sr.outputs);
                one(comp_id, &mut sr.ser_in);
                one(comp_id, &mut sr.clk_shift);
                one(comp_id, &mut sr.clk_latch);
                one(comp_id, &mut sr.master_reset);
                one(comp_id, &mut sr.oe);
                one(comp_id, &mut sr.ser_out);
            }
            Kind::Function(f) => {
                each(comp_id, &mut f.inputs);
                one(comp_id, &mut f.output);
            }
            Kind::Memory(m) => {
                each(comp_id, &mut m.addr_pins);
                each(comp_id, &mut m.data_pins);
                one(comp_id, &mut m.cs);
                one(comp_id, &mut m.oe);
                one(comp_id, &mut m.we);
            }
            Kind::DynamicMemory(dm) => {
                each(comp_id, &mut dm.addr_pins);
                each(comp_id, &mut dm.data_pins);
                one(comp_id, &mut dm.ras);
                one(comp_id, &mut dm.cas);
                one(comp_id, &mut dm.we);
                one(comp_id, &mut dm.oe);
            }
            Kind::I2CRam(r) => {
                one(comp_id, &mut r.scl);
                one(comp_id, &mut r.sda);
            }
            Kind::Lm555(lm) => {
                lm.id = comp_id.to_string();
            }
            Kind::Tunnel { pin_id, .. } => rebind_pin_id(comp_id, pin_id),
            Kind::Pcd8544(st) => {
                st.id = comp_id.to_string();
                for pin in st.pins_mut() {
                    pin.rebind_id(comp_id);
                }
            }
            Kind::Sh1107(st) => {
                st.id = comp_id.to_string();
                for pin in st.pins_mut() {
                    pin.rebind_id(comp_id);
                }
            }
            Kind::Ks0108(st) => {
                st.id = comp_id.to_string();
                for pin in st.pins_mut() {
                    pin.rebind_id(comp_id);
                }
            }
            Kind::Aip31068(st) => {
                st.id = comp_id.to_string();
                for pin in st.pins_mut() {
                    pin.rebind_id(comp_id);
                }
            }
            _ => {}
        }
    }
}
