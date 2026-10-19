//! Keep solver-accumulated state when a live edit replaces a `Kind`.
//!
//! `Part::to_element_kind` builds a fresh value, so RAM, angles, heat, and
//! protocol cursors would otherwise reset on every property edit. User
//! properties stay on the new kind. Capacitor, inductor, and transformer
//! history is updated in place by `update_component_kind` and never reaches
//! this method.

use crate::digital::{Hd44780State, TwiModule};

use super::Kind;

fn copy_prefix(dst: &mut [u8], src: &[u8]) {
    let n = dst.len().min(src.len());
    dst[..n].copy_from_slice(&src[..n]);
}

fn copy_rows(dst: &mut [Vec<u8>], src: &[Vec<u8>]) {
    for (row, prev) in dst.iter_mut().zip(src.iter()) {
        copy_prefix(row, prev);
    }
}

fn keep_hd44780(next: &mut Hd44780State, prev: &Hd44780State) {
    next.ddram = prev.ddram;
    next.cgram = prev.cgram;
    next.dd_addr = prev.dd_addr;
    next.cg_addr = prev.cg_addr;
    next.write_ddram = prev.write_ddram;
    next.shift_pos = prev.shift_pos;
    next.direction = prev.direction;
    next.shift_disp = prev.shift_disp;
    next.disp_on = prev.disp_on;
    next.cursor_on = prev.cursor_on;
    next.cursor_blink = prev.cursor_blink;
    next.nibble = prev.nibble;
    next.input = prev.input;
    next.last_clock = prev.last_clock;
    next.blinking = prev.blinking;
    next.last_circ_time = prev.last_circ_time;
}

/// Protocol cursors survive. The new kind's slave address and mode stay.
fn keep_twi(next: &mut TwiModule, prev: &TwiModule) {
    let address = next.address;
    let mode = next.mode;
    *next = prev.clone();
    next.address = address;
    next.mode = mode;
}

impl Kind {
    /// Copy runtime fields from `prev` when both values are the same variant.
    /// A different variant, or a kind with no accumulated state, is left as the
    /// fresh property snapshot.
    pub fn adopt_runtime(&mut self, prev: &Kind) {
        match (self, prev) {
            (
                Kind::Lamp {
                    voltage,
                    power,
                    r_cold,
                    resistance,
                    heat,
                    ..
                },
                Kind::Lamp {
                    voltage: pv,
                    power: pp,
                    r_cold: pc,
                    resistance: pr,
                    heat: ph,
                    ..
                },
            ) => {
                *heat = *ph;
                if *voltage == *pv && *power == *pp && *r_cold == *pc {
                    *resistance = *pr;
                }
            }
            (
                Kind::Relay { active, coil_i, .. },
                Kind::Relay {
                    active: prev,
                    coil_i: prev_i,
                    ..
                },
            ) => {
                *active = *prev;
                *coil_i = *prev_i;
            }
            (Kind::Csource { ctrl, .. }, Kind::Csource { ctrl: prev, .. }) => {
                *ctrl = *prev;
            }
            (Kind::Clock { state, .. }, Kind::Clock { state: prev, .. }) => {
                *state = *prev;
            }
            (
                Kind::WaveGen {
                    wav_data,
                    wav_index,
                    v_out,
                    ..
                },
                Kind::WaveGen {
                    wav_index: prev_index,
                    v_out: prev_v,
                    ..
                },
            ) => {
                *v_out = *prev_v;
                *wav_index = match wav_data {
                    Some(data) if !data.is_empty() => (*prev_index).min(data.len() - 1),
                    _ => *prev_index,
                };
            }
            (
                Kind::Stepper { angle, .. },
                Kind::Stepper {
                    angle: prev_angle, ..
                },
            ) => {
                *angle = *prev_angle;
            }
            (
                Kind::DcMotor { angle, speed, .. },
                Kind::DcMotor {
                    angle: prev_angle,
                    speed: prev_speed,
                    ..
                },
            ) => {
                *angle = *prev_angle;
                *speed = *prev_speed;
            }
            (
                Kind::Servo {
                    pos,
                    target_pos,
                    pulse_start_ps,
                    sig_high,
                    ..
                },
                Kind::Servo {
                    pos: prev_pos,
                    target_pos: prev_target,
                    pulse_start_ps: prev_pulse,
                    sig_high: prev_sig,
                    ..
                },
            ) => {
                *pos = *prev_pos;
                *target_pos = *prev_target;
                *pulse_start_ps = *prev_pulse;
                *sig_high = *prev_sig;
            }
            (
                Kind::Scr { conducting, .. },
                Kind::Scr {
                    conducting: prev, ..
                },
            )
            | (
                Kind::Triac { conducting, .. },
                Kind::Triac {
                    conducting: prev, ..
                },
            ) => {
                *conducting = *prev;
            }
            (
                Kind::Diac { conducting, .. },
                Kind::Diac {
                    conducting: prev, ..
                },
            ) => {
                *conducting = *prev;
            }
            (Kind::Voltmeter { last_out, .. }, Kind::Voltmeter { last_out: prev, .. })
            | (Kind::Ammeter { last_out, .. }, Kind::Ammeter { last_out: prev, .. }) => {
                *last_out = *prev;
            }
            (Kind::AudioOut { last_v, .. }, Kind::AudioOut { last_v: prev, .. }) => {
                *last_v = *prev;
            }
            (
                Kind::AnalogMux {
                    channels, selected, ..
                },
                Kind::AnalogMux {
                    channels: prev_channels,
                    selected: prev_selected,
                    ..
                },
            ) if *channels == *prev_channels => {
                *selected = *prev_selected;
            }
            (Kind::Memory(next), Kind::Memory(prev)) => {
                copy_prefix(&mut next.data, &prev.data);
            }
            (Kind::DynamicMemory(next), Kind::DynamicMemory(prev)) => {
                copy_prefix(&mut next.data, &prev.data);
                next.row_addr = prev.row_addr;
                next.col_addr = prev.col_addr;
                next.last_ras = prev.last_ras;
                next.last_cas = prev.last_cas;
            }
            (Kind::I2CRam(next), Kind::I2CRam(prev)) => {
                copy_prefix(&mut next.data, &prev.data);
                let last = next.data.len().saturating_sub(1) as u16;
                next.mem_ptr = prev.mem_ptr.min(last);
                next.mode = prev.mode;
                next.bit_count = prev.bit_count;
                next.rx_byte = prev.rx_byte;
                next.last_scl = prev.last_scl;
                next.last_sda = prev.last_sda;
            }
            (Kind::I2CToParallel(next), Kind::I2CToParallel(prev)) => {
                next.out_latch = prev.out_latch;
                next.in_latch = prev.in_latch;
                next.write_byte(prev.out_latch);
                keep_twi(&mut next.twi, &prev.twi);
            }
            (Kind::Counter(next), Kind::Counter(prev)) => {
                next.count = prev.count;
                next.last_clk = prev.last_clk;
            }
            (Kind::BinCounter(next), Kind::BinCounter(prev)) => {
                next.count_a = prev.count_a;
                next.count_b = prev.count_b;
                next.last_clk_a = prev.last_clk_a;
                next.last_clk_b = prev.last_clk_b;
            }
            (Kind::ShiftReg(next), Kind::ShiftReg(prev)) => {
                next.shift_reg = prev.shift_reg;
                next.latch_reg = prev.latch_reg;
                next.last_clk_shift = prev.last_clk_shift;
                next.last_clk_latch = prev.last_clk_latch;
            }
            (Kind::FlipFlop(next), Kind::FlipFlop(prev)) if next.kind == prev.kind => {
                next.q0 = prev.q0;
                let q = prev.q.get_out_state();
                let qn = prev.qn.get_out_state();
                next.q.set_out_state(q);
                next.qn.set_out_state(qn);
            }
            (Kind::Latch(next), Kind::Latch(prev)) => {
                let n = next.outputs.len().min(prev.outputs.len());
                for i in 0..n {
                    let level = prev.outputs[i].get_out_state();
                    next.outputs[i].set_out_state(level);
                }
            }
            (Kind::Lm555(next), Kind::Lm555(prev)) => {
                next.out_state = prev.out_state;
                next.discharge_on = prev.discharge_on;
            }
            (Kind::Pcd8544(next), Kind::Pcd8544(prev)) => {
                next.ram = prev.ram;
                next.addr_x = prev.addr_x;
                next.addr_y = prev.addr_y;
                next.in_buf = prev.in_buf;
                next.in_bit = prev.in_bit;
                next.last_scl = prev.last_scl;
                next.pd = prev.pd;
                next.v_mode = prev.v_mode;
                next.h_mode = prev.h_mode;
                next.d_bit = prev.d_bit;
                next.e_bit = prev.e_bit;
            }
            (Kind::Ks0108(next), Kind::Ks0108(prev)) => {
                next.left_ram = prev.left_ram;
                next.right_ram = prev.right_ram;
                next.left_page = prev.left_page;
                next.left_col = prev.left_col;
                next.right_page = prev.right_page;
                next.right_col = prev.right_col;
                next.left_start_line = prev.left_start_line;
                next.right_start_line = prev.right_start_line;
                next.disp_on_1 = prev.disp_on_1;
                next.disp_on_2 = prev.disp_on_2;
                next.last_en = prev.last_en;
            }
            (Kind::Hd44780(next), Kind::Hd44780(prev)) => keep_hd44780(next, prev),
            (Kind::Aip31068(next), Kind::Aip31068(prev)) => {
                keep_hd44780(&mut next.hd44780, &prev.hd44780);
                next.phase = prev.phase;
                next.control_byte = prev.control_byte;
                keep_twi(&mut next.twi, &prev.twi);
            }
            (Kind::Ssd1306(next), Kind::Ssd1306(prev)) => {
                copy_rows(&mut next.ddram, &prev.ddram);
                if next.width == prev.width && next.height == prev.height {
                    keep_twi(&mut next.twi, &prev.twi);
                    keep_panel_runtime(next, prev);
                }
            }
            (Kind::Sh1107(next), Kind::Sh1107(prev)) => {
                copy_rows(&mut next.ddram, &prev.ddram);
                if next.width == prev.width && next.height == prev.height {
                    keep_twi(&mut next.twi, &prev.twi);
                    keep_sh1107_runtime(next, prev);
                }
            }
            (
                Kind::Max72xx { modules, fb, .. },
                Kind::Max72xx {
                    modules: prev_modules,
                    fb: prev,
                    ..
                },
            ) => {
                if *modules == *prev_modules {
                    *fb = prev.clone();
                } else {
                    let n = fb.digits.len().min(prev.digits.len());
                    fb.digits[..n].copy_from_slice(&prev.digits[..n]);
                    fb.dout_high = prev.dout_high;
                }
            }
            (
                Kind::Ws2812 { count, fb, .. },
                Kind::Ws2812 {
                    count: prev_count,
                    fb: prev,
                    ..
                },
            ) => {
                if *count == *prev_count {
                    *fb = prev.clone();
                } else {
                    let n = fb.pixels.len().min(prev.pixels.len());
                    fb.pixels[..n].copy_from_slice(&prev.pixels[..n]);
                    fb.bit_index = prev.bit_index;
                    fb.fall_ps = prev.fall_ps;
                    fb.dout_high = prev.dout_high;
                }
            }
            (
                Kind::Sr04 {
                    trigger_high,
                    echo_high,
                    echo,
                    ..
                },
                Kind::Sr04 {
                    trigger_high: prev_trig,
                    echo_high: prev_echo,
                    echo: prev_link,
                    ..
                },
            ) => {
                *trigger_high = *prev_trig;
                *echo_high = *prev_echo;
                *echo = prev_link.clone();
            }
            (
                Kind::Dht22 {
                    out_state,
                    pin_driven,
                    link,
                    ..
                },
                Kind::Dht22 {
                    out_state: prev_out,
                    pin_driven: prev_driven,
                    link: prev_link,
                    ..
                },
            ) => {
                *out_state = *prev_out;
                *pin_driven = *prev_driven;
                *link = prev_link.clone();
            }
            (
                Kind::Ds18b20 { dq_low, wire, .. },
                Kind::Ds18b20 {
                    dq_low: prev_low,
                    wire: prev_wire,
                    ..
                },
            ) => {
                *dq_low = *prev_low;
                *wire = prev_wire.clone();
            }
            (
                Kind::Ds1621 {
                    tout_high,
                    sda_low,
                    scl_low,
                    bus,
                    ..
                },
                Kind::Ds1621 {
                    tout_high: prev_tout,
                    sda_low: prev_sda,
                    scl_low: prev_scl,
                    bus: prev_bus,
                    ..
                },
            ) => {
                *tout_high = *prev_tout;
                *sda_low = *prev_sda;
                *scl_low = *prev_scl;
                *bus = prev_bus.clone();
            }
            (
                Kind::Ds1307 {
                    sqw_state,
                    sda_low,
                    scl_low,
                    bus,
                    ..
                },
                Kind::Ds1307 {
                    sqw_state: prev_sqw,
                    sda_low: prev_sda,
                    scl_low: prev_scl,
                    bus: prev_bus,
                    ..
                },
            ) => {
                *sqw_state = *prev_sqw;
                *sda_low = *prev_sda;
                *scl_low = *prev_scl;
                *bus = prev_bus.clone();
            }
            (Kind::Esp01 { uart, .. }, Kind::Esp01 { uart: prev, .. }) => {
                *uart = prev.clone();
            }
            _ => {}
        }
    }
}

fn keep_panel_runtime(
    next: &mut crate::digital::Ssd1306State,
    prev: &crate::digital::Ssd1306State,
) {
    next.addr_mode = prev.addr_mode;
    next.addr_x = prev.addr_x;
    next.addr_y = prev.addr_y;
    next.start_x = prev.start_x;
    next.end_x = prev.end_x;
    next.start_y = prev.start_y;
    next.end_y = prev.end_y;
    next.scroll = prev.scroll;
    next.scroll_v = prev.scroll_v;
    next.scroll_dir = prev.scroll_dir;
    next.scroll_single = prev.scroll_single;
    next.scroll_start_y = prev.scroll_start_y;
    next.scroll_end_y = prev.scroll_end_y;
    next.scroll_step = prev.scroll_step;
    next.scroll_count = prev.scroll_count;
    next.v_scroll_offset = prev.v_scroll_offset;
    next.scroll_top = prev.scroll_top;
    next.scroll_rows = prev.scroll_rows;
    next.disp_offset = prev.disp_offset;
    next.ram_offset = prev.ram_offset;
    next.disp_on = prev.disp_on;
    next.disp_full = prev.disp_full;
    next.disp_inv = prev.disp_inv;
    next.scan_inv = prev.scan_inv;
    next.remap = prev.remap;
    next.mr = prev.mr;
    next.start = prev.start;
    next.co = prev.co;
    next.data = prev.data;
    next.last_command = prev.last_command;
    next.read_bytes = prev.read_bytes;
    next.read_index = prev.read_index;
}

fn keep_sh1107_runtime(next: &mut crate::digital::Sh1107State, prev: &crate::digital::Sh1107State) {
    next.addr_mode = prev.addr_mode;
    next.addr_x = prev.addr_x;
    next.addr_y = prev.addr_y;
    next.start_x = prev.start_x;
    next.end_x = prev.end_x;
    next.start_y = prev.start_y;
    next.end_y = prev.end_y;
    next.disp_offset = prev.disp_offset;
    next.ram_offset = prev.ram_offset;
    next.disp_on = prev.disp_on;
    next.disp_full = prev.disp_full;
    next.disp_inv = prev.disp_inv;
    next.scan_inv = prev.scan_inv;
    next.remap = prev.remap;
    next.mr = prev.mr;
    next.start = prev.start;
    next.co = prev.co;
    next.data = prev.data;
    next.last_command = prev.last_command;
    next.read_bytes = prev.read_bytes;
    next.read_index = prev.read_index;
}
