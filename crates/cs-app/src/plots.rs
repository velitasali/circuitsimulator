//! Oscilloscope and logic analyzer façades. The plot screen is a QML Canvas
//! drawing `cs_engine::plot` samples — not a `QQuickPaintedItem`.

use cs_engine::plot::PlotBuffer;
use cs_engine::theme::ColorTheme;
use qtbridge::qobject;
use serde_json::{Value, json};

pub struct Oscilloscope {
    traces: PlotBuffer,
    current_channel: i32,
    trigger: i32,
    tracks: i32,
    hidden: Vec<bool>,
    time_div: f64,
    filter: f64,
    panning: bool,
    ch_volt_div: Vec<f64>,
    ch_volt_pos: Vec<f64>,
    ch_time_pos: Vec<f64>,
    ch_trig_level: Vec<f64>,
    ch_trig_rising: Vec<bool>,
    dark: bool,
}

impl Default for Oscilloscope {
    fn default() -> Self {
        Self {
            traces: cs_engine::instruments::empty_scope_traces(),
            current_channel: 0,
            trigger: -1,
            tracks: 1,
            hidden: vec![false; 4],
            time_div: 1e-3,
            filter: 0.0,
            panning: false,
            ch_volt_div: vec![1.0; 4],
            ch_volt_pos: vec![0.0; 4],
            ch_time_pos: vec![0.0; 4],
            ch_trig_level: vec![0.0; 4],
            ch_trig_rising: vec![true; 4],
            dark: false,
        }
    }
}

fn traces_json(buf: &PlotBuffer) -> Value {
    let arr: Vec<Value> = buf
        .channels
        .iter()
        .map(|ch| {
            json!({
                "color": ch.color,
                "samples": ch.samples,
            })
        })
        .collect();
    Value::Array(arr)
}

#[qobject(Singleton, ConvertToCamelCase)]
impl Oscilloscope {
    qproperty!("panning", Read = panning, Notify = panning_changed);
    qproperty!(
        "currentChannel",
        Read = current_channel,
        Write = set_current_channel,
        Notify = current_channel_changed
    );
    qproperty!("trigger", Read = trigger, Notify = trigger_changed);
    qproperty!("tracks", Read = tracks, Notify = tracks_changed);
    qproperty!("hidden", Read = hidden, Notify = hidden_changed);
    qproperty!("dark", Read = dark, Write = set_dark, Notify = dark_changed);
    qproperty!("channels", Read = channels, Notify = channels_changed);
    qproperty!("traces", Read = traces, Notify = traces_changed);
    qproperty!("timeDiv", Read = time_div, Notify = time_div_changed);
    qproperty!(
        "filter",
        Read = filter,
        Write = set_filter,
        Notify = filter_changed
    );
    qproperty!(
        "chVoltDiv",
        Read = ch_volt_div,
        Notify = ch_volt_div_changed
    );
    qproperty!(
        "chVoltPos",
        Read = ch_volt_pos,
        Notify = ch_volt_pos_changed
    );
    qproperty!(
        "chTimePos",
        Read = ch_time_pos,
        Notify = ch_time_pos_changed
    );
    qproperty!(
        "chTrigLevel",
        Read = ch_trig_level,
        Notify = ch_trig_level_changed
    );
    qproperty!(
        "chTrigRising",
        Read = ch_trig_rising,
        Notify = ch_trig_rising_changed
    );

    fn panning(&self) -> bool {
        self.panning
    }
    fn current_channel(&self) -> i32 {
        self.current_channel
    }
    fn trigger(&self) -> i32 {
        self.trigger
    }
    fn tracks(&self) -> i32 {
        self.tracks
    }
    fn time_div(&self) -> f64 {
        self.time_div
    }
    fn filter(&self) -> f64 {
        self.filter
    }
    fn dark(&self) -> bool {
        self.dark
    }
    fn set_filter(&mut self, f: f64) {
        if (self.filter - f).abs() > 1e-6 {
            self.filter = f;
            self.filter_changed();
        }
    }
    fn set_dark(&mut self, dark: bool) {
        if self.dark != dark {
            self.dark = dark;
            self.dark_changed();
            self.channels_changed();
        }
    }

    #[qsignal]
    fn panning_changed(&mut self);
    #[qsignal]
    fn current_channel_changed(&mut self);
    #[qsignal]
    fn trigger_changed(&mut self);
    #[qsignal]
    fn tracks_changed(&mut self);
    #[qsignal]
    fn hidden_changed(&mut self);
    #[qsignal]
    fn dark_changed(&mut self);
    #[qsignal]
    fn channels_changed(&mut self);
    #[qsignal]
    fn traces_changed(&mut self);
    #[qsignal]
    fn time_div_changed(&mut self);
    #[qsignal]
    fn filter_changed(&mut self);
    #[qsignal]
    fn ch_volt_div_changed(&mut self);
    #[qsignal]
    fn ch_volt_pos_changed(&mut self);
    #[qsignal]
    fn ch_time_pos_changed(&mut self);
    #[qsignal]
    fn ch_trig_level_changed(&mut self);
    #[qsignal]
    fn ch_trig_rising_changed(&mut self);

    fn set_current_channel(&mut self, ch: i32) {
        if ch < 0 || ch > 3 || ch == self.current_channel {
            return;
        }
        self.current_channel = ch;
        self.current_channel_changed();
    }

    fn channels(&self) -> Value {
        let colors = ColorTheme::scope_colors(self.dark);
        let arr: Vec<Value> = colors
            .iter()
            .enumerate()
            .map(|(i, c)| json!({ "color": c, "index": i }))
            .collect();
        Value::Array(arr)
    }

    fn hidden(&self) -> Value {
        json!(self.hidden)
    }

    fn ch_volt_div(&self) -> Value {
        json!(self.ch_volt_div)
    }

    fn ch_volt_pos(&self) -> Value {
        json!(self.ch_volt_pos)
    }

    fn ch_time_pos(&self) -> Value {
        json!(self.ch_time_pos)
    }

    fn ch_trig_level(&self) -> Value {
        json!(self.ch_trig_level)
    }

    fn ch_trig_rising(&self) -> Value {
        json!(self.ch_trig_rising)
    }

    #[qslot]
    fn set_ch_volt_div(&mut self, ch: i32, val: f64) {
        if ch >= 0 && (ch as usize) < self.ch_volt_div.len() {
            self.ch_volt_div[ch as usize] = val;
            self.ch_volt_div_changed();
        }
    }

    #[qslot]
    fn set_ch_volt_pos(&mut self, ch: i32, val: f64) {
        if ch >= 0 && (ch as usize) < self.ch_volt_pos.len() {
            self.ch_volt_pos[ch as usize] = val;
            self.ch_volt_pos_changed();
        }
    }

    #[qslot]
    fn set_ch_time_pos(&mut self, ch: i32, val: f64) {
        if ch >= 0 && (ch as usize) < self.ch_time_pos.len() {
            self.ch_time_pos[ch as usize] = val;
            self.ch_time_pos_changed();
        }
    }

    #[qslot]
    fn set_ch_trig_level(&mut self, ch: i32, val: f64) {
        if ch >= 0 && (ch as usize) < self.ch_trig_level.len() {
            self.ch_trig_level[ch as usize] = val;
            self.ch_trig_level_changed();
        }
    }

    #[qslot]
    fn set_ch_trig_rising(&mut self, ch: i32, val: bool) {
        if ch >= 0 && (ch as usize) < self.ch_trig_rising.len() {
            self.ch_trig_rising[ch as usize] = val;
            self.ch_trig_rising_changed();
        }
    }

    fn traces(&self) -> Value {
        traces_json(&self.traces)
    }

    #[qslot]
    fn auto_scale(&mut self, _ch: i32) {}

    #[qslot]
    fn trigger_clicked(&mut self, ch: i32) {
        if self.trigger == ch {
            self.trigger = -1;
        } else {
            self.trigger = ch;
        }
        self.trigger_changed();
    }

    #[qslot]
    fn hide_clicked(&mut self, ch: i32, hide: bool) {
        if let Some(h) = self.hidden.get_mut(ch as usize) {
            *h = hide;
            self.hidden_changed();
        }
    }

    #[qslot]
    fn tracks_clicked(&mut self, tracks: i32) {
        let valid = match tracks {
            2 => 2,
            4 => 4,
            _ => 1,
        };
        self.tracks = valid;
        self.tracks_changed();
    }

    #[qslot]
    fn plot_pressed(&mut self, _x: f64, _button: i32) {
        self.panning = true;
        self.panning_changed();
    }

    #[qslot]
    fn plot_moved(&mut self, _x: f64) {}

    #[qslot]
    fn plot_released(&mut self) {
        if self.panning {
            self.panning = false;
            self.panning_changed();
        }
    }

    #[qslot]
    fn fit_displays(&self) {}

    #[qslot]
    fn set_time_div(&mut self, v: f64) {
        if self.time_div == v {
            return;
        }
        self.time_div = v;
        self.time_div_changed();
    }
}

pub struct LogicAnalyzer {
    traces: PlotBuffer,
    trigger: i32,
    threshold_r: f64,
    threshold_f: f64,
    conds: String,
    buses: Vec<bool>,
    panning: bool,
    time_div: f64,
    time_pos: f64,
    dark: bool,
}

impl Default for LogicAnalyzer {
    fn default() -> Self {
        Self {
            traces: cs_engine::instruments::empty_la_traces(),
            trigger: 0,
            threshold_r: 2.5,
            threshold_f: 2.5,
            conds: String::new(),
            buses: vec![false; 8],
            panning: false,
            time_div: 1e-3,
            time_pos: 0.0,
            dark: false,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl LogicAnalyzer {
    qproperty!("panning", Read = panning, Notify = panning_changed);
    qproperty!(
        "trigger",
        Read = trigger,
        Write = set_trigger,
        Notify = trigger_changed
    );
    qproperty!(
        "thresholdR",
        Read = threshold_r,
        Write = set_threshold_r,
        Notify = threshold_changed
    );
    qproperty!(
        "thresholdF",
        Read = threshold_f,
        Write = set_threshold_f,
        Notify = threshold_changed
    );
    qproperty!(
        "conds",
        Read = conds,
        Write = set_conds,
        Notify = conds_changed
    );
    qproperty!("dark", Read = dark, Write = set_dark, Notify = dark_changed);
    qproperty!("channels", Read = channels, Notify = channels_changed);
    qproperty!("buses", Read = buses, Notify = buses_changed);
    qproperty!("traces", Read = traces, Notify = traces_changed);
    qproperty!("timeDiv", Read = time_div, Notify = time_changed);
    qproperty!(
        "timePos",
        Read = time_pos,
        Write = set_time_pos,
        Notify = time_changed
    );

    fn panning(&self) -> bool {
        self.panning
    }
    fn trigger(&self) -> i32 {
        self.trigger
    }
    fn threshold_r(&self) -> f64 {
        self.threshold_r
    }
    fn threshold_f(&self) -> f64 {
        self.threshold_f
    }
    fn conds(&self) -> String {
        self.conds.clone()
    }
    fn dark(&self) -> bool {
        self.dark
    }
    fn time_div(&self) -> f64 {
        self.time_div
    }
    fn time_pos(&self) -> f64 {
        self.time_pos
    }
    fn set_time_pos(&mut self, p: f64) {
        if (self.time_pos - p).abs() > 1e-9 {
            self.time_pos = p;
            self.time_changed();
        }
    }
    fn set_dark(&mut self, dark: bool) {
        if self.dark != dark {
            self.dark = dark;
            self.dark_changed();
            self.channels_changed();
        }
    }

    #[qsignal]
    fn panning_changed(&mut self);
    #[qsignal]
    fn trigger_changed(&mut self);
    #[qsignal]
    fn threshold_changed(&mut self);
    #[qsignal]
    fn conds_changed(&mut self);
    #[qsignal]
    fn dark_changed(&mut self);
    #[qsignal]
    fn channels_changed(&mut self);
    #[qsignal]
    fn buses_changed(&mut self);
    #[qsignal]
    fn traces_changed(&mut self);
    #[qsignal]
    fn time_changed(&mut self);

    fn set_trigger(&mut self, v: i32) {
        self.trigger = v;
        self.trigger_changed();
    }

    fn set_threshold_r(&mut self, v: f64) {
        self.threshold_r = v;
        self.threshold_changed();
    }

    fn set_threshold_f(&mut self, v: f64) {
        self.threshold_f = v;
        self.threshold_changed();
    }

    fn set_conds(&mut self, v: String) {
        self.conds = v;
        self.conds_changed();
    }

    fn channels(&self) -> Value {
        let colors = ColorTheme::la_colors(self.dark);
        let arr: Vec<Value> = colors
            .iter()
            .enumerate()
            .map(|(i, c)| json!({ "color": c, "index": i }))
            .collect();
        Value::Array(arr)
    }

    fn buses(&self) -> Value {
        json!(self.buses)
    }

    fn traces(&self) -> Value {
        traces_json(&self.traces)
    }

    #[qslot]
    fn bus_clicked(&mut self, ch: i32, is_bus: bool) {
        if let Some(b) = self.buses.get_mut(ch as usize) {
            *b = is_bus;
            self.buses_changed();
        }
    }

    #[qslot]
    fn export_data(&self) {}

    #[qslot]
    fn plot_pressed(&mut self, _x: f64, _button: i32) {
        self.panning = true;
        self.panning_changed();
    }

    #[qslot]
    fn plot_moved(&mut self, _x: f64) {}

    #[qslot]
    fn plot_released(&mut self) {
        if self.panning {
            self.panning = false;
            self.panning_changed();
        }
    }

    #[qslot]
    fn fit_displays(&self) {}
}
