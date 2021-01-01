//! Serial monitor and terminal façades. Logs and print modes live in
//! `cs_engine::serial`; the OS port is `serialport`, not `QSerialPort`.

use crate::path_util::strip_file_url;
use cs_engine::serial::{PrintMode, TermBuffer, TermColors, list_ports, terminal_colors};
use qtbridge::qobject;
use serde_json::{Value, json};

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct MonitorSession {
    pub id: String,
    pub title: String,
}

pub struct SerialMonitor {
    title: String,
    sessions: HashMap<String, MonitorSession>,
    open_monitors: Vec<String>,
}

impl Default for SerialMonitor {
    fn default() -> Self {
        Self {
            title: cs_engine::i18n::tr("Serial Monitor"),
            sessions: HashMap::new(),
            open_monitors: Vec::new(),
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl SerialMonitor {
    qproperty!(
        "printMode",
        Read = print_mode,
        Write = set_print_mode,
        Notify = options_changed
    );
    qproperty!(
        "addCR",
        Read = add_cr,
        Write = set_add_cr,
        Notify = options_changed
    );
    qproperty!(
        "paused",
        Read = paused,
        Write = set_paused,
        Notify = options_changed
    );
    qproperty!(
        "sendEnabled",
        Read = send_enabled,
        Notify = send_enabled_changed
    );
    qproperty!("inText", Read = in_text, Notify = logs_changed);
    qproperty!("outText", Read = out_text, Notify = logs_changed);
    qproperty!(
        "windowTitle",
        Read = title,
        Write = set_title,
        Notify = title_changed
    );
    qproperty!("ports", Read = ports, Notify = ports_changed);
    qproperty!(
        "openMonitors",
        Read = open_monitors,
        Notify = open_monitors_changed
    );

    #[qsignal]
    fn options_changed(&mut self);
    #[qsignal]
    fn send_enabled_changed(&mut self);
    #[qsignal]
    fn logs_changed(&mut self);
    #[qsignal]
    fn title_changed(&mut self);
    #[qsignal]
    fn ports_changed(&mut self);
    #[qsignal]
    fn open_monitors_changed(&mut self);
    #[qsignal]
    fn request_show_monitor(&mut self, id: String);
    #[qsignal]
    fn monitor_updated(&mut self, id: String, in_text: String, out_text: String);

    fn print_mode(&self) -> i32 {
        cs_engine::serial::print_mode("default").index()
    }

    fn set_print_mode(&mut self, mode: i32) {
        let pm = PrintMode::from_index(mode);
        cs_engine::serial::set_print_mode("default", pm);
        self.options_changed();
    }

    fn add_cr(&self) -> bool {
        cs_engine::serial::add_cr("default")
    }

    fn set_add_cr(&mut self, add: bool) {
        cs_engine::serial::set_add_cr("default", add);
        self.options_changed();
    }

    fn paused(&self) -> bool {
        cs_engine::serial::paused("default")
    }

    fn set_paused(&mut self, paused: bool) {
        cs_engine::serial::set_paused("default", paused);
        self.options_changed();
    }

    fn send_enabled(&self) -> bool {
        cs_engine::serial::send_enabled("default")
    }

    fn in_text(&self) -> String {
        cs_engine::serial::in_text("default")
    }

    fn out_text(&self) -> String {
        cs_engine::serial::out_text("default")
    }

    fn ports(&self) -> Value {
        let arr: Vec<Value> = list_ports()
            .into_iter()
            .map(|p| json!({ "name": p.name, "description": p.description }))
            .collect();
        Value::Array(arr)
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, title: String) {
        if self.title != title {
            self.title = title;
            self.title_changed();
        }
    }

    #[qslot]
    fn send_text(&self, text: String) {
        cs_engine::serial::send_text("default", &text);
    }

    #[qslot]
    fn send_value(&self, value: String) {
        cs_engine::serial::send_value("default", &value);
    }

    #[qslot]
    fn clear_logs(&mut self) {
        cs_engine::serial::clear("default");
        self.logs_changed();
        self.monitor_updated("default".into(), String::new(), String::new());
    }

    #[qslot]
    fn print_in(&mut self, value: i32) {
        cs_engine::serial::publish_in("default", value as u8);
        self.logs_changed();
        let in_txt = cs_engine::serial::in_text("default");
        let out_txt = cs_engine::serial::out_text("default");
        self.monitor_updated("default".into(), in_txt, out_txt);
    }

    #[qslot]
    fn print_out(&mut self, value: i32) {
        cs_engine::serial::publish_out("default", value as u8);
        self.logs_changed();
        let in_txt = cs_engine::serial::in_text("default");
        let out_txt = cs_engine::serial::out_text("default");
        self.monitor_updated("default".into(), in_txt, out_txt);
    }

    #[qslot]
    fn activate_send(&mut self) {
        cs_engine::serial::set_send_enabled("default", true);
        self.send_enabled_changed();
    }

    #[qslot]
    fn flush(&mut self) {
        self.sync();
    }

    #[qslot]
    fn sync(&mut self) {
        let dirty = cs_engine::serial::flush_dirty();
        for id in dirty {
            let in_txt = cs_engine::serial::in_text(&id);
            let out_txt = cs_engine::serial::out_text(&id);
            if id == "default" {
                self.logs_changed();
            }
            self.monitor_updated(id, in_txt, out_txt);
        }
    }

    #[qslot]
    fn refresh_ports(&mut self) {
        self.ports_changed();
    }

    fn open_monitors(&self) -> Value {
        let arr: Vec<Value> = self
            .open_monitors
            .iter()
            .filter_map(|id| {
                self.sessions.get(id).map(|s| {
                    json!({
                        "id": s.id,
                        "title": s.title,
                    })
                })
            })
            .collect();
        Value::Array(arr)
    }

    #[qslot]
    fn open_monitor(&mut self, id: String, title: String) {
        let display_title = if title.is_empty() {
            cs_engine::i18n::tr("Serial Monitor")
        } else {
            title
        };
        cs_engine::serial::ensure_session(&id);
        if let Some(session) = self.sessions.get_mut(&id) {
            session.title = display_title;
        } else {
            self.sessions.insert(
                id.clone(),
                MonitorSession {
                    id: id.clone(),
                    title: display_title,
                },
            );
        }
        if !self.open_monitors.contains(&id) {
            self.open_monitors.push(id.clone());
            self.open_monitors_changed();
        }
        if id == "default" {
            self.logs_changed();
        }
        let in_txt = cs_engine::serial::in_text(&id);
        let out_txt = cs_engine::serial::out_text(&id);
        self.monitor_updated(id.clone(), in_txt, out_txt);
        self.request_show_monitor(id);
    }

    #[qslot]
    fn close_monitor(&mut self, id: String) {
        if let Some(pos) = self.open_monitors.iter().position(|x| x == &id) {
            self.open_monitors.remove(pos);
            self.open_monitors_changed();
        }
    }

    #[qslot]
    fn in_text_for(&self, id: String) -> String {
        cs_engine::serial::in_text(&id)
    }

    #[qslot]
    fn out_text_for(&self, id: String) -> String {
        cs_engine::serial::out_text(&id)
    }

    #[qslot]
    fn print_mode_for(&self, id: String) -> i32 {
        cs_engine::serial::print_mode(&id).index()
    }

    #[qslot]
    fn set_print_mode_for(&mut self, id: String, mode: i32) {
        let pm = PrintMode::from_index(mode);
        cs_engine::serial::set_print_mode(&id, pm);
        if id == "default" {
            self.options_changed();
        }
    }

    #[qslot]
    fn add_cr_for(&self, id: String) -> bool {
        cs_engine::serial::add_cr(&id)
    }

    #[qslot]
    fn set_add_cr_for(&mut self, id: String, add: bool) {
        cs_engine::serial::set_add_cr(&id, add);
        if id == "default" {
            self.options_changed();
        }
    }

    #[qslot]
    fn paused_for(&self, id: String) -> bool {
        cs_engine::serial::paused(&id)
    }

    #[qslot]
    fn set_paused_for(&mut self, id: String, paused: bool) {
        cs_engine::serial::set_paused(&id, paused);
        if id == "default" {
            self.options_changed();
        }
    }

    #[qslot]
    fn send_enabled_for(&self, id: String) -> bool {
        cs_engine::serial::send_enabled(&id)
    }

    #[qslot]
    fn send_text_for(&self, id: String, text: String) {
        cs_engine::serial::send_text(&id, &text);
    }

    #[qslot]
    fn send_value_for(&self, id: String, value: String) {
        cs_engine::serial::send_value(&id, &value);
    }

    #[qslot]
    fn clear_logs_for(&mut self, id: String) {
        cs_engine::serial::clear(&id);
        if id == "default" {
            self.logs_changed();
        }
        self.monitor_updated(id, String::new(), String::new());
    }

    #[qslot]
    fn print_in_for(&mut self, id: String, value: i32) {
        cs_engine::serial::publish_in(&id, value as u8);
        if id == "default" {
            self.logs_changed();
        }
        let in_txt = cs_engine::serial::in_text(&id);
        let out_txt = cs_engine::serial::out_text(&id);
        self.monitor_updated(id, in_txt, out_txt);
    }

    #[qslot]
    fn print_out_for(&mut self, id: String, value: i32) {
        cs_engine::serial::publish_out(&id, value as u8);
        if id == "default" {
            self.logs_changed();
        }
        let in_txt = cs_engine::serial::in_text(&id);
        let out_txt = cs_engine::serial::out_text(&id);
        self.monitor_updated(id, in_txt, out_txt);
    }
}

pub struct SerialTerminal {
    inner: TermBuffer,
    colors: TermColors,
    dark: bool,
}

impl Default for SerialTerminal {
    fn default() -> Self {
        Self {
            inner: TermBuffer::default(),
            colors: terminal_colors(false),
            dark: false,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl SerialTerminal {
    qproperty!("log", Read = log, Notify = log_changed);
    qproperty!(
        "inputText",
        Read = input_text,
        Write = set_input_text,
        Notify = input_text_changed
    );
    qproperty!(
        "printMode",
        Read = print_mode,
        Write = set_print_mode,
        Notify = modes_changed
    );
    qproperty!(
        "sendMode",
        Read = send_mode,
        Write = set_send_mode,
        Notify = modes_changed
    );
    qproperty!(
        "rxBackground",
        Read = rx_background,
        Notify = colors_changed
    );
    qproperty!("rxText", Read = rx_text, Notify = colors_changed);
    qproperty!(
        "txBackground",
        Read = tx_background,
        Notify = colors_changed
    );
    qproperty!("txText", Read = tx_text_color, Notify = colors_changed);

    #[qsignal]
    fn log_changed(&mut self);
    #[qsignal]
    fn input_text_changed(&mut self);
    #[qsignal]
    fn modes_changed(&mut self);
    #[qsignal]
    fn colors_changed(&mut self);
    #[qsignal]
    fn request_load_file(&mut self);
    #[qsignal]
    fn request_save_log(&mut self);

    fn log(&self) -> String {
        self.inner.log.clone()
    }

    fn input_text(&self) -> String {
        self.inner.input.clone()
    }

    fn set_input_text(&mut self, text: String) {
        if self.inner.input == text {
            return;
        }
        self.inner.input = text;
        self.input_text_changed();
    }

    fn print_mode(&self) -> String {
        self.inner.print_mode.name().into()
    }

    fn set_print_mode(&mut self, mode: String) {
        let mode = PrintMode::from_name(&mode);
        if self.inner.print_mode == mode {
            return;
        }
        self.inner.print_mode = mode;
        self.modes_changed();
    }

    fn send_mode(&self) -> String {
        self.inner.send_mode.name().into()
    }

    fn set_send_mode(&mut self, mode: String) {
        let mode = PrintMode::from_name(&mode);
        if self.inner.send_mode == mode {
            return;
        }
        self.inner.send_mode = mode;
        self.modes_changed();
    }

    fn rx_background(&self) -> String {
        self.colors.rx_bg.clone()
    }

    fn rx_text(&self) -> String {
        self.colors.rx_text.clone()
    }

    fn tx_background(&self) -> String {
        self.colors.tx_bg.clone()
    }

    fn tx_text_color(&self) -> String {
        self.colors.tx_text.clone()
    }

    #[qslot]
    fn set_dark(&mut self, dark: bool) {
        if self.dark == dark {
            return;
        }
        self.dark = dark;
        self.colors = terminal_colors(dark);
        self.colors_changed();
    }

    #[qslot]
    fn send(&mut self) {
        let _ = self.inner.send();
        if self.inner.flush() {
            self.log_changed();
        }
    }

    #[qslot]
    fn received(&mut self, byte: i32) {
        self.inner.received(byte as u8);
        if self.inner.flush() {
            self.log_changed();
        }
    }

    #[qslot]
    fn load_file(&mut self, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.inner.load_text(text);
                self.input_text_changed();
            }
            Err(e) => eprintln!("Cannot open file: {e}"),
        }
    }

    #[qslot]
    fn save_log(&mut self, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        if let Err(e) = std::fs::write(&path, self.inner.plain_log()) {
            eprintln!("Cannot save file {e}");
        }
    }

    #[qslot]
    fn request_load(&mut self) {
        self.request_load_file();
    }

    #[qslot]
    fn request_save(&mut self) {
        self.request_save_log();
    }

    #[qslot]
    fn clear_send(&mut self) {
        self.inner.clear_send();
        self.input_text_changed();
    }

    #[qslot]
    fn clear_receive(&mut self) {
        if self.inner.clear_receive() {
            self.log_changed();
        }
    }
}
