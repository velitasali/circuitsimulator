//! Host serial I/O: print modes, the monitor log, and a `serialport` link.
//!
//! Replaces Qt `QSerialPort` / `SerialLog` / `SerialMonitor` byte formatting.
//! UART pin timing stays with the later SerialPort / MCU catalog types.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// C++ `SerialMonitor` print-mode index / `Terminal` mode name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PrintMode {
    #[default]
    Ascii = 0,
    Hex = 1,
    Dec = 2,
    Oct = 3,
    Bin = 4,
}

impl PrintMode {
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => Self::Hex,
            2 => Self::Dec,
            3 => Self::Oct,
            4 => Self::Bin,
            _ => Self::Ascii,
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "HEX" => Self::Hex,
            "DEC" => Self::Dec,
            "OCT" => Self::Oct,
            "BIN" => Self::Bin,
            _ => Self::Ascii,
        }
    }

    pub fn index(self) -> i32 {
        self as i32
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ascii => "ASCII",
            Self::Hex => "HEX",
            Self::Dec => "DEC",
            Self::Oct => "OCT",
            Self::Bin => "BIN",
        }
    }
}

/// C++ `decToBase` (`utils.cpp`): always `digits` characters, uppercase hex.
pub fn dec_to_base(mut value: u32, base: u32, digits: usize) -> String {
    let base = base.max(2);
    let mut out = String::with_capacity(digits);
    for _ in 0..digits {
        let d = if value >= base { value % base } else { value };
        out.insert(
            0,
            char::from_digit(d, base)
                .unwrap_or('0')
                .to_ascii_uppercase(),
        );
        value /= base;
    }
    out
}

/// C++ `SerialMonitor::valToString` / `Terminal::received`.
pub fn format_byte(mode: PrintMode, byte: u8) -> String {
    match mode {
        PrintMode::Ascii => {
            // QChar(uint8_t) is Latin-1.
            char::from_u32(byte as u32)
                .unwrap_or('\u{FFFD}')
                .to_string()
        }
        PrintMode::Hex => format!("{} ", dec_to_base(byte as u32, 16, 2)),
        PrintMode::Dec => format!("{} ", dec_to_base(byte as u32, 10, 3)),
        PrintMode::Oct => format!("{} ", dec_to_base(byte as u32, 8, 3)),
        PrintMode::Bin => format!("{} ", dec_to_base(byte as u32, 2, 8)),
    }
}

/// C++ `SerialLog`: bytes accumulate, then flush; cap ~100k, keep last 90k.
#[derive(Clone, Debug, Default)]
pub struct SerialLog {
    text: String,
    pending: String,
}

impl SerialLog {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn append(&mut self, s: &str) {
        self.pending.push_str(s);
    }

    pub fn flush(&mut self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        self.text.push_str(&self.pending);
        self.pending.clear();
        if self.text.len() > 100_000 {
            let keep = self.text.len() - 90_000;
            self.text = self.text[self.byte_floor(keep)..].to_string();
        }
        true
    }

    fn byte_floor(&self, mut i: usize) -> usize {
        while i < self.text.len() && !self.text.is_char_boundary(i) {
            i += 1;
        }
        i.min(self.text.len())
    }

    pub fn clear(&mut self) -> bool {
        if self.text.is_empty() && self.pending.is_empty() {
            return false;
        }
        self.text.clear();
        self.pending.clear();
        true
    }
}

/// C++ `SerialMonitor` state: two logs plus an injected-byte queue.
#[derive(Clone, Debug)]
pub struct Monitor {
    pub in_log: SerialLog,
    pub out_log: SerialLog,
    pub print_mode: PrintMode,
    pub add_cr: bool,
    pub paused: bool,
    pub send_enabled: bool,
    send_queue: VecDeque<u8>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self {
            in_log: SerialLog::default(),
            out_log: SerialLog::default(),
            print_mode: PrintMode::Ascii,
            add_cr: false,
            paused: false,
            send_enabled: true,
            send_queue: VecDeque::new(),
        }
    }
}

impl Monitor {
    pub fn print_in(&mut self, byte: u8) {
        if self.paused {
            return;
        }
        self.in_log.append(&format_byte(self.print_mode, byte));
    }

    pub fn print_out(&mut self, byte: u8) {
        if self.paused {
            return;
        }
        self.out_log.append(&format_byte(self.print_mode, byte));
    }

    pub fn send_text(&mut self, text: &str) {
        if self.paused {
            return;
        }
        self.send_queue.extend(text.as_bytes());
        if self.add_cr {
            self.send_queue.push_back(13);
        }
    }

    /// C++ `QString::toInt()` (base 10) appended as one byte.
    pub fn send_value(&mut self, value: &str) {
        if self.paused {
            return;
        }
        let n: i32 = value.trim().parse().unwrap_or(0);
        self.send_queue.push_back(n as u8);
    }

    pub fn take_send(&mut self) -> Vec<u8> {
        self.send_queue.drain(..).collect()
    }

    pub fn flush(&mut self) -> bool {
        if self.paused {
            return false;
        }
        let a = self.in_log.flush();
        let b = self.out_log.flush();
        a || b
    }

    pub fn has_send(&self) -> bool {
        !self.send_queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.in_log.clear();
        self.out_log.clear();
    }
}

static HAS_PENDING_SEND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[inline(always)]
pub fn has_pending_send() -> bool {
    HAS_PENDING_SEND.load(std::sync::atomic::Ordering::Relaxed)
}

fn sessions() -> &'static Mutex<HashMap<String, Monitor>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, Monitor>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn dirty_set() -> &'static Mutex<HashSet<String>> {
    static DIRTY: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    DIRTY.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Ensure a session exists in the registry.
pub fn ensure_session(port_id: &str) {
    let mut map = sessions().lock().unwrap();
    map.entry(port_id.to_string()).or_default();
}

/// Append an output byte (from MCU/device TX) to the given session's out_log.
pub fn publish_out(port_id: &str, byte: u8) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.print_out(byte);
    if mon.flush() {
        dirty_set().lock().unwrap().insert(port_id.to_string());
    }
}

/// Append an input byte (received by MCU/device RX) to the given session's in_log.
pub fn publish_in(port_id: &str, byte: u8) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.print_in(byte);
    if mon.flush() {
        dirty_set().lock().unwrap().insert(port_id.to_string());
    }
}

/// Enqueue text to be transmitted to the MCU/device RX.
pub fn send_text(port_id: &str, text: &str) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.send_text(text);
    if mon.has_send() {
        HAS_PENDING_SEND.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Enqueue a numeric byte value to be transmitted to the MCU/device RX.
pub fn send_value(port_id: &str, value: &str) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.send_value(value);
    if mon.has_send() {
        HAS_PENDING_SEND.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Drain pending host TX bytes for the MCU/device RX.
pub fn take_send(port_id: &str) -> Vec<u8> {
    if !has_pending_send() {
        return Vec::new();
    }
    let mut map = sessions().lock().unwrap();
    let res = if let Some(mon) = map.get_mut(port_id) {
        mon.take_send()
    } else {
        Vec::new()
    };
    if map.values().all(|m| !m.has_send()) {
        HAS_PENDING_SEND.store(false, std::sync::atomic::Ordering::Release);
    }
    res
}

/// Get current flushed output log text for the given session.
pub fn out_text(port_id: &str) -> String {
    let mut map = sessions().lock().unwrap();
    if let Some(mon) = map.get_mut(port_id) {
        mon.flush();
        mon.out_log.text().to_string()
    } else {
        String::new()
    }
}

/// Get current flushed input log text for the given session.
pub fn in_text(port_id: &str) -> String {
    let mut map = sessions().lock().unwrap();
    if let Some(mon) = map.get_mut(port_id) {
        mon.flush();
        mon.in_log.text().to_string()
    } else {
        String::new()
    }
}

/// Clear logs for the given session.
pub fn clear(port_id: &str) {
    let mut map = sessions().lock().unwrap();
    if let Some(mon) = map.get_mut(port_id) {
        mon.clear();
        dirty_set().lock().unwrap().insert(port_id.to_string());
    }
}

/// Drain the dirty session set.
pub fn flush_dirty() -> Vec<String> {
    let mut dirty = dirty_set().lock().unwrap();
    dirty.drain().collect()
}

pub fn print_mode(port_id: &str) -> PrintMode {
    let map = sessions().lock().unwrap();
    map.get(port_id)
        .map(|m| m.print_mode)
        .unwrap_or(PrintMode::Ascii)
}

pub fn set_print_mode(port_id: &str, mode: PrintMode) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.print_mode = mode;
}

pub fn paused(port_id: &str) -> bool {
    let map = sessions().lock().unwrap();
    map.get(port_id).map(|m| m.paused).unwrap_or(false)
}

pub fn set_paused(port_id: &str, paused: bool) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.paused = paused;
}

pub fn add_cr(port_id: &str) -> bool {
    let map = sessions().lock().unwrap();
    map.get(port_id).map(|m| m.add_cr).unwrap_or(false)
}

pub fn set_add_cr(port_id: &str, add: bool) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.add_cr = add;
}

pub fn send_enabled(port_id: &str) -> bool {
    let map = sessions().lock().unwrap();
    map.get(port_id).map(|m| m.send_enabled).unwrap_or(true)
}

pub fn set_send_enabled(port_id: &str, enabled: bool) {
    let mut map = sessions().lock().unwrap();
    let mon = map.entry(port_id.to_string()).or_default();
    mon.send_enabled = enabled;
}

/// C++ `Terminal` receive pane (HTML) plus send buffer.
#[derive(Clone, Debug)]
pub struct TermBuffer {
    pub log: String,
    pending: String,
    pub input: String,
    pub print_mode: PrintMode,
    pub send_mode: PrintMode,
    send_queue: VecDeque<u8>,
}

impl Default for TermBuffer {
    fn default() -> Self {
        Self {
            log: String::new(),
            pending: String::new(),
            input: String::new(),
            print_mode: PrintMode::Ascii,
            send_mode: PrintMode::Ascii,
            send_queue: VecDeque::new(),
        }
    }
}

impl TermBuffer {
    pub fn received(&mut self, byte: u8) {
        self.pending.push_str(&format_byte(self.print_mode, byte));
    }

    pub fn flush(&mut self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        self.log.push_str(&self.pending.replace('\n', "<br>"));
        self.pending.clear();
        true
    }

    pub fn send(&mut self) -> Vec<u8> {
        let data = match self.send_mode {
            PrintMode::Ascii => self.input.as_bytes().to_vec(),
            other => parse_numeric_send(&self.input, other),
        };
        if data.is_empty() && self.send_mode != PrintMode::Ascii {
            return Vec::new();
        }
        let shown = if self.send_mode == PrintMode::Ascii {
            self.input.clone()
        } else {
            let mut s = String::new();
            for part in self.input.split_whitespace() {
                s.push_str(part);
                s.push(' ');
            }
            s
        };
        self.pending
            .push_str(&format!("<br><font color='yellow'>{shown}</font><br>"));
        self.send_queue.extend(&data);
        data
    }

    pub fn take_send(&mut self) -> Vec<u8> {
        self.send_queue.drain(..).collect()
    }

    pub fn clear_send(&mut self) {
        self.input.clear();
    }

    pub fn clear_receive(&mut self) -> bool {
        if self.log.is_empty() && self.pending.is_empty() {
            return false;
        }
        self.log.clear();
        self.pending.clear();
        true
    }

    pub fn load_text(&mut self, text: String) {
        self.input = text;
    }

    /// C++ `QTextDocumentFragment::fromHtml(m_log).toPlainText()`.
    pub fn plain_log(&self) -> String {
        html_to_plain(&self.log)
    }
}

fn parse_numeric_send(text: &str, mode: PrintMode) -> Vec<u8> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let radix = match mode {
        PrintMode::Hex => 16,
        PrintMode::Dec => 10,
        PrintMode::Oct => 8,
        PrintMode::Bin => 2,
        PrintMode::Ascii => return text.as_bytes().to_vec(),
    };
    text.split_whitespace()
        .filter_map(|p| u8::from_str_radix(p, radix).ok())
        .collect()
}

fn html_to_plain(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for t in chars.by_ref() {
                if t == '>' {
                    break;
                }
                tag.push(t);
            }
            let tag_l = tag.to_ascii_lowercase();
            if tag_l == "br" || tag_l.starts_with("br ") || tag_l == "br/" || tag_l == "br /" {
                out.push('\n');
            }
            continue;
        }
        if c == '&' {
            let mut ent = String::new();
            while let Some(&n) = chars.peek() {
                chars.next();
                if n == ';' {
                    break;
                }
                ent.push(n);
                if ent.len() > 8 {
                    break;
                }
            }
            out.push(match ent.as_str() {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "nbsp" => ' ',
                _ => '&',
            });
            continue;
        }
        out.push(c);
    }
    out
}

/// Terminal colours as CSS hex (C++ `ColorTheme::TerminalRx*` / `Tx*`).
pub fn terminal_colors(dark: bool) -> TermColors {
    if dark {
        TermColors {
            rx_bg: "#1a1829".into(),
            rx_text: "#ffffff".into(),
            tx_bg: "#1e1e1e".into(),
            tx_text: "#dcdcdc".into(),
        }
    } else {
        TermColors {
            rx_bg: "#23203c".into(),
            rx_text: "#ffffff".into(),
            tx_bg: "#fcfcf6".into(),
            tx_text: "#000000".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TermColors {
    pub rx_bg: String,
    pub rx_text: String,
    pub tx_bg: String,
    pub tx_text: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Parity {
    #[default]
    None,
    Even,
    Odd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortConfig {
    pub name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: Parity,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PortInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug)]
pub enum SerialError {
    Open(String),
    Io(String),
    NotOpen,
}

impl std::fmt::Display for SerialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open(s) | Self::Io(s) => write!(f, "{s}"),
            Self::NotOpen => write!(f, "port is not open"),
        }
    }
}

impl std::error::Error for SerialError {}

/// Available OS serial ports (`serialport::available_ports`).
pub fn list_ports() -> Vec<PortInfo> {
    match serialport::available_ports() {
        Ok(list) => list
            .into_iter()
            .map(|p| PortInfo {
                name: p.port_name,
                description: match p.port_type {
                    serialport::SerialPortType::UsbPort(usb) => {
                        usb.product.unwrap_or_else(|| "USB serial".into())
                    }
                    serialport::SerialPortType::BluetoothPort => "Bluetooth".into(),
                    serialport::SerialPortType::PciPort => "PCI".into(),
                    serialport::SerialPortType::Unknown => String::new(),
                },
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

enum Backend {
    Loopback,
    Real(Box<dyn serialport::SerialPort>),
}

/// Two queues matching C++ `SerialPort`: `to_host` is UART RX → OS write,
/// `to_sim` is OS read → UART TX.
pub struct SerialLink {
    pub config: PortConfig,
    pub auto_open: bool,
    open: bool,
    backend: Backend,
    to_host: VecDeque<u8>,
    to_sim: VecDeque<u8>,
}

impl Default for SerialLink {
    fn default() -> Self {
        Self {
            config: PortConfig::default(),
            auto_open: false,
            open: false,
            backend: Backend::Loopback,
            to_host: VecDeque::new(),
            to_sim: VecDeque::new(),
        }
    }
}

impl SerialLink {
    pub fn loopback() -> Self {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) -> Result<(), SerialError> {
        if self.open {
            self.close();
        }
        if self.config.name.is_empty() || self.config.name == "loopback" {
            self.backend = Backend::Loopback;
            self.open = true;
            return Ok(());
        }
        let mut builder = serialport::new(&self.config.name, self.config.baud_rate)
            .timeout(Duration::from_millis(1));
        builder = builder.data_bits(match self.config.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            _ => serialport::DataBits::Eight,
        });
        builder = builder.stop_bits(if self.config.stop_bits >= 2 {
            serialport::StopBits::Two
        } else {
            serialport::StopBits::One
        });
        builder = builder.parity(match self.config.parity {
            Parity::None => serialport::Parity::None,
            Parity::Even => serialport::Parity::Even,
            Parity::Odd => serialport::Parity::Odd,
        });
        builder = builder.flow_control(serialport::FlowControl::None);
        match builder.open() {
            Ok(port) => {
                self.backend = Backend::Real(port);
                self.open = true;
                Ok(())
            }
            Err(e) => Err(SerialError::Open(format!(
                "Cannot Open Port {}: {e}.",
                self.config.name
            ))),
        }
    }

    pub fn close(&mut self) {
        self.backend = Backend::Loopback;
        self.open = false;
        self.to_host.clear();
        self.to_sim.clear();
    }

    /// UART RX byte (sim → host). C++ `SerialPort::byteReceived`.
    pub fn uart_received(&mut self, byte: u8) {
        self.to_host.push_back(byte);
    }

    /// Bytes the UART TX should emit (host → sim). C++ `m_uartData`.
    pub fn pop_uart_tx(&mut self) -> Option<u8> {
        self.to_sim.pop_front()
    }

    /// Host injected a byte (terminal / monitor send).
    pub fn host_write(&mut self, bytes: &[u8]) {
        self.to_sim.extend(bytes);
    }

    /// Drain host-side RX (what the OS port would have received).
    pub fn host_read(&mut self) -> Vec<u8> {
        self.to_host.drain(..).collect()
    }

    /// C++ `SerialPort::updateStep` + `readData`: write queued UART bytes to
    /// the OS port and read incoming host bytes into the UART TX queue.
    pub fn pump(&mut self) -> Result<(), SerialError> {
        if !self.open {
            return Err(SerialError::NotOpen);
        }
        match &mut self.backend {
            Backend::Loopback => {
                // Loopback: UART TX queue already holds host_write; UART RX
                // sits in to_host until host_read. Nothing to move.
                Ok(())
            }
            Backend::Real(port) => {
                if !self.to_host.is_empty() {
                    let buf: Vec<u8> = self.to_host.drain(..).collect();
                    if let Err(e) = port.write_all(&buf) {
                        return Err(SerialError::Io(e.to_string()));
                    }
                }
                let n = port.bytes_to_read().unwrap_or(0) as usize;
                if n > 0 {
                    let mut buf = vec![0u8; n];
                    match port.read(&mut buf) {
                        Ok(got) => self.to_sim.extend(&buf[..got]),
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(e) => return Err(SerialError::Io(e.to_string())),
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_to_base_matches_cpp() {
        assert_eq!(dec_to_base(0xAB, 16, 2), "AB");
        assert_eq!(dec_to_base(10, 16, 2), "0A");
        assert_eq!(dec_to_base(255, 10, 3), "255");
        assert_eq!(dec_to_base(7, 10, 3), "007");
        assert_eq!(dec_to_base(5, 2, 8), "00000101");
        assert_eq!(dec_to_base(8, 8, 3), "010");
    }

    #[test]
    fn format_modes() {
        assert_eq!(format_byte(PrintMode::Ascii, b'A'), "A");
        assert_eq!(format_byte(PrintMode::Hex, 0xAB), "AB ");
        assert_eq!(format_byte(PrintMode::Dec, 7), "007 ");
        assert_eq!(format_byte(PrintMode::Oct, 8), "010 ");
        assert_eq!(format_byte(PrintMode::Bin, 5), "00000101 ");
    }

    #[test]
    fn log_flush_and_cap() {
        let mut log = SerialLog::default();
        log.append("hi");
        assert!(log.text().is_empty());
        assert!(log.flush());
        assert_eq!(log.text(), "hi");
        log.append(&"x".repeat(120_000));
        log.flush();
        assert!(log.text().len() <= 100_000);
        assert!(log.text().len() >= 90_000);
    }

    #[test]
    fn monitor_send_and_pause() {
        let mut m = Monitor::default();
        m.add_cr = true;
        m.send_text("AB");
        assert_eq!(m.take_send(), vec![b'A', b'B', 13]);
        m.send_value("65");
        assert_eq!(m.take_send(), vec![65]);
        m.paused = true;
        m.print_in(b'X');
        m.flush();
        assert!(m.in_log.text().is_empty());
    }

    #[test]
    fn term_hex_send_and_html() {
        let mut t = TermBuffer::default();
        t.send_mode = PrintMode::Hex;
        t.input = "41 42".into();
        assert_eq!(t.send(), vec![0x41, 0x42]);
        t.flush();
        assert!(t.log.contains("yellow"));
        assert!(t.plain_log().contains("41 42"));
        t.received(b'Z');
        t.flush();
        assert!(t.log.contains('Z'));
    }

    #[test]
    fn loopback_link() {
        let mut link = SerialLink::loopback();
        link.open().unwrap();
        link.host_write(&[1, 2, 3]);
        assert_eq!(link.pop_uart_tx(), Some(1));
        assert_eq!(link.pop_uart_tx(), Some(2));
        link.uart_received(0x55);
        assert_eq!(link.host_read(), vec![0x55]);
        link.close();
        assert!(!link.is_open());
    }

    #[test]
    fn list_ports_does_not_panic() {
        let _ = list_ports();
    }
}
