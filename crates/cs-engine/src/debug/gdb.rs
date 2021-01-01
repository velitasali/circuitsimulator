//! GDB Remote Serial Protocol (RSP) TCP Server.
//! Allows external IDEs (CLion, VS Code, Eclipse) and command-line `avr-gdb` /
//! `gdb-multiarch` to debug simulated MCU targets over a TCP socket (port 1234).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use super::{BreakReason, DebugSession, DebugState};

/// GDB Remote Serial Protocol Server.
pub struct GdbServer {
    port: u16,
    running: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl GdbServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            running: Arc::new(AtomicBool::new(false)),
            handle: Mutex::new(None),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start listening on TCP `127.0.0.1:<port>`.
    pub fn start(&self) -> std::io::Result<()> {
        if self.is_running() {
            return Ok(());
        }

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(true)?;

        self.running.store(true, Ordering::SeqCst);
        let running_flag = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            while running_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        Self::handle_client(stream, Arc::clone(&running_flag));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        *self.handle.lock().unwrap() = Some(handle);
        Ok(())
    }

    /// Stop the server.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    fn handle_client(mut stream: TcpStream, running: Arc<AtomicBool>) {
        let mut buf = [0u8; 4096];
        let mut packet_buf = String::new();
        let session = DebugSession::global();

        while running.load(Ordering::Relaxed) {
            match stream.read(&mut buf) {
                Ok(0) => break, // Client disconnected
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]);
                    for c in text.chars() {
                        if c == '+' || c == '-' {
                            continue;
                        }
                        if c == '\x03' {
                            // Ctrl+C / Break from GDB
                            session.pause();
                            let resp = Self::make_packet("S02");
                            let _ = stream.write_all(resp.as_bytes());
                            continue;
                        }
                        if c == '$' {
                            packet_buf.clear();
                        } else if c == '#' {
                            // Checksum follows (next 2 chars handled when complete packet is parsed)
                        } else {
                            packet_buf.push(c);
                        }
                    }

                    // Process complete packets
                    if let Some(payload) = Self::extract_payload(&text) {
                        let _ = stream.write_all(b"+"); // ACK
                        let response_payload = Self::process_command(&payload, session);
                        if !response_payload.is_empty() {
                            let resp_packet = Self::make_packet(&response_payload);
                            let _ = stream.write_all(resp_packet.as_bytes());
                            let _ = stream.flush();
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    }

    fn extract_payload(raw: &str) -> Option<String> {
        let start = raw.find('$')? + 1;
        let end = raw[start..].find('#')? + start;
        Some(raw[start..end].to_string())
    }

    pub fn checksum(data: &str) -> u8 {
        data.bytes().fold(0u8, |acc, b| acc.wrapping_add(b))
    }

    pub fn format_packet(payload: &str) -> String {
        Self::make_packet(payload)
    }

    pub fn handle_command(cmd: &str) -> String {
        Self::process_command(cmd, DebugSession::global())
    }

    pub fn make_packet(payload: &str) -> String {
        let csum = Self::checksum(payload);
        format!("${}#{:02x}", payload, csum)
    }

    /// Process a single GDB command string and produce response payload.
    pub fn process_command(cmd: &str, session: &DebugSession) -> String {
        if cmd.is_empty() {
            return String::new();
        }

        // Halt reason
        if cmd == "?" {
            return match session.state() {
                DebugState::Paused(BreakReason::Breakpoint { .. }) => "S05".into(),
                DebugState::Paused(BreakReason::ManualPause) => "S02".into(),
                DebugState::Paused(_) => "S05".into(),
                _ => "S05".into(),
            };
        }

        // Supported queries
        if cmd.starts_with("qSupported") {
            return "PacketSize=1000;vContSupported+;multiprocess-".into();
        }
        if cmd.starts_with("qAttached") {
            return "1".into();
        }
        if cmd.starts_with("qC") {
            return "QC1".into();
        }
        if cmd.starts_with("qOffsets") {
            return "Text=0;Data=0;Bss=0".into();
        }
        if cmd.starts_with("qTStatus") {
            return "".into();
        }

        // vCont
        if cmd == "vCont?" {
            return "vCont;c;s".into();
        }
        if cmd.starts_with("vCont;c") || cmd == "c" {
            session.resume();
            return "".into(); // Execution continues asynchronously
        }
        if cmd.starts_with("vCont;s") || cmd == "s" {
            session.step_into();
            return "S05".into();
        }

        // Read registers: 'g'
        if cmd == "g" {
            return session
                .with_mcu(|mcu| {
                    if let Some(mcu) = mcu {
                        let mut hex = String::new();
                        // Return 32 general 8-bit registers (R0..R31 for AVR), SREG, SP, PC
                        for i in 0..32 {
                            let b = mcu.device.ram(i);
                            hex.push_str(&format!("{:02x}", b));
                        }
                        // SREG
                        hex.push_str(&format!("{:02x}", mcu.device.status()));
                        // SP (16-bit little-endian)
                        let sp = mcu.device.sp();
                        hex.push_str(&format!("{:02x}{:02x}", sp as u8, (sp >> 8) as u8));
                        // PC (32-bit little-endian byte address)
                        let pc_bytes = mcu.device.pc() * 2;
                        hex.push_str(&format!(
                            "{:02x}{:02x}{:02x}{:02x}",
                            pc_bytes as u8,
                            (pc_bytes >> 8) as u8,
                            (pc_bytes >> 16) as u8,
                            (pc_bytes >> 24) as u8
                        ));
                        hex
                    } else {
                        "00".repeat(37)
                    }
                })
                .unwrap_or_else(|| "00".repeat(37));
        }

        // Write registers: 'G <hex>'
        if let Some(hex) = cmd.strip_prefix('G') {
            let bytes = Self::hex_to_bytes(hex);
            session.with_mcu_mut(|mcu| {
                if let Some(mcu) = mcu {
                    for (i, &b) in bytes.iter().take(32).enumerate() {
                        mcu.device.set_monitor_ram(i as u16, b);
                    }
                }
            });
            return "OK".into();
        }

        // Read memory: 'm <addr>,<len>'
        if let Some(rest) = cmd.strip_prefix('m') {
            if let Some((addr_str, len_str)) = rest.split_once(',') {
                if let (Ok(addr), Ok(len)) = (
                    u32::from_str_radix(addr_str, 16),
                    usize::from_str_radix(len_str, 16),
                ) {
                    return session
                        .with_mcu(|mcu| {
                            if let Some(mcu) = mcu {
                                let mut hex = String::new();
                                for i in 0..len {
                                    let curr = addr.wrapping_add(i as u32);
                                    let byte = if curr < 0x800000 {
                                        // Flash / program memory
                                        let word_addr = (curr / 2) as usize;
                                        let word = mcu.device.flash_word(word_addr).unwrap_or(0);
                                        if (curr % 2) == 0 {
                                            word as u8
                                        } else {
                                            (word >> 8) as u8
                                        }
                                    } else {
                                        // RAM (0x800000 offset in avr-gdb)
                                        let ram_addr = (curr - 0x800000) as u16;
                                        mcu.device.ram(ram_addr)
                                    };
                                    hex.push_str(&format!("{:02x}", byte));
                                }
                                hex
                            } else {
                                "00".repeat(len)
                            }
                        })
                        .unwrap_or_else(|| "00".repeat(len));
                }
            }
            return "E01".into();
        }

        // Write memory: 'M <addr>,<len>:<data>'
        if let Some(rest) = cmd.strip_prefix('M') {
            if let Some((header, data_hex)) = rest.split_once(':') {
                if let Some((addr_str, len_str)) = header.split_once(',') {
                    if let (Ok(addr), Ok(_len)) = (
                        u32::from_str_radix(addr_str, 16),
                        usize::from_str_radix(len_str, 16),
                    ) {
                        let bytes = Self::hex_to_bytes(data_hex);
                        session.with_mcu_mut(|mcu| {
                            if let Some(mcu) = mcu {
                                for (i, &b) in bytes.iter().enumerate() {
                                    let curr = addr.wrapping_add(i as u32);
                                    if curr >= 0x800000 {
                                        let ram_addr = (curr - 0x800000) as u16;
                                        mcu.device.set_monitor_ram(ram_addr, b);
                                    }
                                }
                            }
                        });
                        return "OK".into();
                    }
                }
            }
            return "E01".into();
        }

        // Insert breakpoint: 'Z0,<addr>,<kind>' or 'Z1,<addr>,<kind>'
        if let Some(rest) = cmd.strip_prefix("Z0,").or_else(|| cmd.strip_prefix("Z1,")) {
            if let Some((addr_str, _)) = rest.split_once(',') {
                if let Ok(addr) = u32::from_str_radix(addr_str, 16) {
                    let word_addr = addr / 2;
                    session.add_addr_breakpoint(word_addr);
                    return "OK".into();
                }
            }
            return "E01".into();
        }

        // Remove breakpoint: 'z0,<addr>,<kind>' or 'z1,<addr>,<kind>'
        if let Some(rest) = cmd.strip_prefix("z0,").or_else(|| cmd.strip_prefix("z1,")) {
            if let Some((addr_str, _)) = rest.split_once(',') {
                if let Ok(addr) = u32::from_str_radix(addr_str, 16) {
                    let word_addr = addr / 2;
                    session.remove_addr_breakpoint(word_addr);
                    return "OK".into();
                }
            }
            return "E01".into();
        }

        // Disconnect or Kill: 'D' / 'k'
        if cmd == "D" || cmd == "k" {
            session.stop();
            return "OK".into();
        }

        // Unrecognized / empty fallback
        String::new()
    }

    fn hex_to_bytes(hex: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let chars: Vec<char> = hex.chars().collect();
        let mut i = 0;
        while i + 1 < chars.len() {
            let s: String = chars[i..i + 2].iter().collect();
            if let Ok(b) = u8::from_str_radix(&s, 16) {
                bytes.push(b);
            }
            i += 2;
        }
        bytes
    }
}
