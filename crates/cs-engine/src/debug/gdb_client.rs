//! GDB Remote Serial Protocol (RSP) TCP client for debugging QEMU targets
//! (ESP32 / Xtensa and STM32 / ARM).
//!
//! Connects to `qemu-system-*`'s `-gdb tcp:127.0.0.1:<port> -S` stub on a
//! background worker thread, synchronizes breakpoints from [`DebugSession`],
//! handles source-level stepping, and reads live target memory for watched
//! variables in the MCU Monitor.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::debug::{BreakReason, DebugSession, DebugState, RemoteDebugCmd};
use crate::qemu::QemuFamily;

const POLL_TIMEOUT: Duration = Duration::from_millis(20);
const CMD_TIMEOUT: Duration = Duration::from_millis(1500);
const MAX_SOURCE_STEP_INSNS: usize = 256;

/// Background GDB RSP client attached to a live QEMU process.
pub struct QemuGdbClient {
    stop_flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl QemuGdbClient {
    /// Connect to a QEMU GDB stub listening on `127.0.0.1:<port>` and start the
    /// background debug coordinator thread bound to [`DebugSession::global`].
    pub fn connect(port: u16, family: QemuFamily) -> Result<Self, String> {
        Self::connect_for_session(port, family, DebugSession::global())
    }

    /// Connect to a QEMU GDB stub using an explicit [`DebugSession`] instance.
    pub fn connect_for_session(
        port: u16,
        family: QemuFamily,
        session: &'static DebugSession,
    ) -> Result<Self, String> {
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        let deadline = Instant::now() + Duration::from_secs(3);
        let stream = loop {
            match TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
                Ok(s) => break s,
                Err(e) => {
                    if Instant::now() >= deadline {
                        return Err(format!(
                            "could not connect to QEMU GDB stub on 127.0.0.1:{port}: {e}"
                        ));
                    }
                    thread::sleep(Duration::from_millis(15));
                }
            }
        };
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(POLL_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CMD_TIMEOUT));

        crate::logging::log_sim(format!("[GDB] Connected to QEMU on 127.0.0.1:{port}"));

        let stop_flag = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_flag);
        let handle = thread::Builder::new()
            .name(format!("qemu-gdb-{port}"))
            .spawn(move || {
                let mut worker = GdbWorker::new(stream, family, worker_stop, session);
                worker.run();
            })
            .map_err(|e| format!("failed to spawn QEMU GDB thread: {e}"))?;

        Ok(Self {
            stop_flag,
            handle: Some(handle),
        })
    }

    pub fn shutdown(&mut self) {
        self.stop_flag.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for QemuGdbClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct GdbWorker {
    stream: TcpStream,
    family: QemuFamily,
    stop_flag: Arc<AtomicBool>,
    session: &'static DebugSession,
    no_ack: bool,
    rx_buf: Vec<u8>,
    target_running: bool,
    installed_bps: HashSet<u32>,
    last_bp_gen: u64,
    current_pc: u32,
}

impl GdbWorker {
    fn new(
        stream: TcpStream,
        family: QemuFamily,
        stop_flag: Arc<AtomicBool>,
        session: &'static DebugSession,
    ) -> Self {
        Self {
            stream,
            family,
            stop_flag,
            session,
            no_ack: false,
            rx_buf: Vec::with_capacity(4096),
            target_running: false,
            installed_bps: HashSet::new(),
            last_bp_gen: u64::MAX,
            current_pc: 0,
        }
    }

    fn run(&mut self) {
        let session = self.session;
        session.set_remote_attached(true);

        if !self.handshake() {
            crate::logging::log_sim_err("[GDB] Handshake with QEMU GDB stub failed");
            session.set_remote_attached(false);
            return;
        }

        // Install all initial breakpoints while QEMU is halted by -S.
        self.sync_breakpoints(session);
        self.refresh_pc_and_watches(session);

        // Start execution if the debug session expects Running (the normal Debug flow).
        match session.state() {
            DebugState::Paused(_) => {
                // Stay paused at entry.
            }
            _ => {
                if self.send_continue() {
                    self.target_running = true;
                }
            }
        }

        while !self.stop_flag.load(Ordering::Acquire) {
            // 1. Poll for async stop replies while target is running.
            if self.target_running {
                match self.try_recv_packet(POLL_TIMEOUT) {
                    Ok(Some(pkt)) => {
                        self.handle_stop_reply(&pkt, session, None);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        // Connection closed (QEMU exited).
                        break;
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(10));
            }

            if self.stop_flag.load(Ordering::Acquire) {
                break;
            }

            // 2. Sync breakpoint additions/removals from editor.
            let bp_gen = session.breakpoints_generation();
            if bp_gen != self.last_bp_gen {
                let was_running = self.target_running;
                let mut hit_bp_during_interrupt = false;
                if was_running {
                    if let Some(stop_pkt) = self.interrupt_target() {
                        if self.is_breakpoint_stop(&stop_pkt) {
                            self.handle_stop_reply(&stop_pkt, session, None);
                            hit_bp_during_interrupt = true;
                        }
                    } else {
                        break;
                    }
                }
                self.sync_breakpoints(session);
                if was_running && !hit_bp_during_interrupt {
                    if self.send_continue() {
                        self.target_running = true;
                    } else {
                        break;
                    }
                }
            }

            // 3. Process debug commands from UI / DebugSession.
            while let Some(cmd) = session.pop_remote_cmd() {
                match cmd {
                    RemoteDebugCmd::Resume => {
                        if !self.target_running {
                            self.resume_past_breakpoint();
                        }
                    }
                    RemoteDebugCmd::Pause => {
                        if self.target_running {
                            if let Some(stop_pkt) = self.interrupt_target() {
                                self.handle_stop_reply(
                                    &stop_pkt,
                                    session,
                                    Some(BreakReason::ManualPause),
                                );
                            }
                        } else {
                            self.refresh_pc_and_watches(session);
                            session.set_paused_at(self.current_pc, BreakReason::ManualPause);
                        }
                    }
                    RemoteDebugCmd::StepInto => {
                        if self.target_running {
                            let _ = self.interrupt_target();
                        }
                        self.do_source_step(session, false);
                    }
                    RemoteDebugCmd::StepOver => {
                        if self.target_running {
                            let _ = self.interrupt_target();
                        }
                        self.do_source_step(session, true);
                    }
                    RemoteDebugCmd::StepOut => {
                        if self.target_running {
                            let _ = self.interrupt_target();
                        }
                        self.do_step_out(session);
                    }
                    RemoteDebugCmd::Stop => {
                        self.detach_cleanly();
                        session.set_remote_attached(false);
                        return;
                    }
                }
            }

            // 4. Refresh watched variable memory if MCU Monitor requested it.
            if session.take_remote_mem_refresh() {
                let ranges = session.watch_mem_ranges();
                if !ranges.is_empty() || !self.target_running {
                    let was_running = self.target_running;
                    let mut hit_bp = false;
                    if was_running {
                        if let Some(stop_pkt) = self.interrupt_target() {
                            if self.is_breakpoint_stop(&stop_pkt) {
                                self.handle_stop_reply(&stop_pkt, session, None);
                                hit_bp = true;
                            }
                        } else {
                            break;
                        }
                    }
                    self.refresh_pc_and_watches(session);
                    if was_running && !hit_bp {
                        if self.send_continue() {
                            self.target_running = true;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Clean up on thread exit.
        self.detach_cleanly();
        session.set_remote_attached(false);
    }

    fn handshake(&mut self) -> bool {
        // Send initial ACK in case QEMU expects one.
        let _ = self.stream.write_all(b"+");
        let _ = self.stream.flush();

        if let Some(supported) = self.exchange("qSupported:swbreak+;hwbreak+;multiprocess-") {
            if supported.contains("QStartNoAckMode+") {
                if self.exchange("QStartNoAckMode").as_deref() == Some("OK") {
                    self.no_ack = true;
                }
            }
        } else {
            return false;
        }

        // Query initial stop reason (? -> S05 / T05...)
        let _ = self.exchange("?");
        self.target_running = false;
        true
    }

    fn sync_breakpoints(&mut self, session: &DebugSession) {
        let desired_list = session.addr_breakpoints();
        self.last_bp_gen = session.breakpoints_generation();
        let desired: HashSet<u32> = desired_list.into_iter().collect();

        // Remove stale breakpoints
        let to_remove: Vec<u32> = self.installed_bps.difference(&desired).copied().collect();
        for addr in to_remove {
            let cmd = format!("z0,{addr:x},1");
            if self.exchange(&cmd).as_deref() == Some("OK") {
                self.installed_bps.remove(&addr);
            }
        }

        // Insert new breakpoints
        let to_add: Vec<u32> = desired.difference(&self.installed_bps).copied().collect();
        for addr in to_add {
            let cmd = format!("Z0,{addr:x},1");
            if self.exchange(&cmd).as_deref() == Some("OK") {
                self.installed_bps.insert(addr);
                crate::logging::log_sim(format!("[GDB] Breakpoint set at 0x{addr:08x}"));
            } else {
                // Fallback to hardware breakpoint Z1 if software breakpoint rejected.
                let hw_cmd = format!("Z1,{addr:x},1");
                if self.exchange(&hw_cmd).as_deref() == Some("OK") {
                    self.installed_bps.insert(addr);
                    crate::logging::log_sim(format!(
                        "[GDB] Hardware breakpoint set at 0x{addr:08x}"
                    ));
                }
            }
        }
    }

    fn interrupt_target(&mut self) -> Option<String> {
        if !self.target_running {
            return Some("S02".to_string());
        }
        if self.stream.write_all(&[0x03]).is_err() {
            return None;
        }
        let _ = self.stream.flush();
        let deadline = Instant::now() + CMD_TIMEOUT;
        while Instant::now() < deadline {
            match self.try_recv_packet(POLL_TIMEOUT) {
                Ok(Some(pkt)) => {
                    if is_stop_packet(&pkt) {
                        self.target_running = false;
                        return Some(pkt);
                    }
                }
                Ok(None) => {}
                Err(_) => return None,
            }
        }
        None
    }

    fn is_breakpoint_stop(&mut self, pkt: &str) -> bool {
        if !(pkt.starts_with("T05") || pkt.starts_with("S05")) {
            return false;
        }
        self.select_stop_thread(pkt);
        if let Some(pc) = self.read_pc() {
            self.current_pc = pc;
            return self.installed_bps.contains(&pc);
        }
        false
    }

    fn select_stop_thread(&mut self, pkt: &str) {
        if let Some(rest) = pkt.strip_prefix('T') {
            if let Some(idx) = rest.find("thread:") {
                let after = &rest[idx + 7..];
                let tid = after.split(';').next().unwrap_or("");
                if !tid.is_empty() {
                    let _ = self.exchange(&format!("Hg{tid}"));
                    let _ = self.exchange(&format!("Hc{tid}"));
                }
            }
        }
    }

    fn handle_stop_reply(
        &mut self,
        pkt: &str,
        session: &DebugSession,
        override_reason: Option<BreakReason>,
    ) {
        if !is_stop_packet(pkt) {
            return;
        }
        self.target_running = false;
        self.select_stop_thread(pkt);
        self.refresh_pc_and_watches(session);

        let pc = self.current_pc;
        let reason = if let Some(r) = override_reason {
            r
        } else if self.installed_bps.contains(&pc) {
            BreakReason::Breakpoint {
                address: pc,
                location: None,
            }
        } else if pkt.starts_with("T02") || pkt.starts_with("S02") {
            BreakReason::ManualPause
        } else {
            BreakReason::Trap
        };

        session.set_paused_at(pc, reason);
        if let Some(loc) = session.active_location() {
            crate::logging::log_sim(format!(
                "[GDB] Paused at 0x{pc:08x} ({}:{})",
                loc.file.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                loc.line
            ));
        } else {
            crate::logging::log_sim(format!("[GDB] Paused at 0x{pc:08x}"));
        }
    }

    fn resume_past_breakpoint(&mut self) {
        if let Some(pc) = self.read_pc() {
            self.current_pc = pc;
        }
        let pc = self.current_pc;
        if self.installed_bps.contains(&pc) {
            // Temporarily remove breakpoint at current PC, step 1 instruction,
            // re-insert breakpoint, then continue.
            let _ = self.exchange(&format!("z0,{pc:x},1"));
            let _ = self.single_step_one();
            let _ = self.exchange(&format!("Z0,{pc:x},1"));
        }
        if self.send_continue() {
            self.target_running = true;
        }
    }

    fn single_step_one(&mut self) -> Option<String> {
        if !self.send_packet("s") {
            return None;
        }
        let deadline = Instant::now() + CMD_TIMEOUT;
        while Instant::now() < deadline {
            match self.try_recv_packet(POLL_TIMEOUT) {
                Ok(Some(pkt)) => {
                    if is_stop_packet(&pkt) {
                        self.select_stop_thread(&pkt);
                        return Some(pkt);
                    }
                }
                Ok(None) => {}
                Err(_) => return None,
            }
        }
        None
    }

    fn do_source_step(&mut self, session: &DebugSession, step_over: bool) {
        if let Some(pc) = self.read_pc() {
            self.current_pc = pc;
        }
        let syms = session.symbols();
        let start_pc = self.current_pc;
        let start_loc = syms.lookup_address(start_pc).cloned();
        let start_fn = syms.function_at(start_pc).map(str::to_string);

        for _ in 0..MAX_SOURCE_STEP_INSNS {
            if self.stop_flag.load(Ordering::Acquire) {
                break;
            }
            let pc = self.current_pc;
            let had_bp = self.installed_bps.contains(&pc);
            if had_bp {
                let _ = self.exchange(&format!("z0,{pc:x},1"));
            }
            let stop = self.single_step_one();
            if had_bp {
                let _ = self.exchange(&format!("Z0,{pc:x},1"));
            }
            if stop.is_none() {
                break;
            }

            let Some(new_pc) = self.read_pc() else {
                break;
            };
            self.current_pc = new_pc;

            // If we landed on another user breakpoint, stop immediately.
            if new_pc != start_pc && self.installed_bps.contains(&new_pc) {
                self.refresh_pc_and_watches(session);
                session.set_paused_at(
                    new_pc,
                    BreakReason::Breakpoint {
                        address: new_pc,
                        location: None,
                    },
                );
                return;
            }

            // If Step Over entered a callee function, run until return address.
            if step_over {
                if let Some(ref orig_fn) = start_fn {
                    let cur_fn = syms.function_at(new_pc);
                    if cur_fn.is_some_and(|f| f != orig_fn.as_str()) {
                        // On Xtensa windowed ABI, the first instruction of a callee is `entry`,
                        // which rotates `windowbase` so `a0` holds the return address. Step past
                        // `entry` before reading `a0` if we stopped right on function entry.
                        let mut callee_pc = new_pc;
                        if self.family == QemuFamily::Esp32
                            && syms.pc_to_function.contains_key(&callee_pc)
                        {
                            if self.single_step_one().is_some() {
                                if let Some(after_entry) = self.read_pc() {
                                    callee_pc = after_entry;
                                    self.current_pc = after_entry;
                                }
                            }
                        }
                        if let Some(ret_addr) = self.read_return_address(callee_pc) {
                            if self.run_until_temp_bp(ret_addr, session) {
                                if let Some(after_pc) = self.read_pc() {
                                    self.current_pc = after_pc;
                                }
                            } else {
                                // Interrupted or hit another breakpoint while running to return.
                                return;
                            }
                        }
                    }
                }
            }

            // If we have no source mapping at start, 1 instruction step is complete.
            let Some(ref sl) = start_loc else {
                break;
            };
            if let Some(nl) = syms.lookup_address(self.current_pc) {
                if nl.line != sl.line || nl.file != sl.file {
                    break;
                }
            }
        }

        self.refresh_pc_and_watches(session);
        session.set_paused_at(self.current_pc, BreakReason::StepCompleted);
    }

    fn do_step_out(&mut self, session: &DebugSession) {
        if let Some(pc) = self.read_pc() {
            self.current_pc = pc;
        }
        if let Some(ret_addr) = self.read_return_address(self.current_pc) {
            if !self.run_until_temp_bp(ret_addr, session) {
                return;
            }
            if let Some(pc) = self.read_pc() {
                self.current_pc = pc;
            }
        } else {
            self.do_source_step(session, false);
            return;
        }
        self.refresh_pc_and_watches(session);
        session.set_paused_at(self.current_pc, BreakReason::StepCompleted);
    }

    /// Run to a temporary breakpoint at `addr`. Returns `true` if `addr` was
    /// reached cleanly, `false` if interrupted/stopped or another user breakpoint hit.
    fn run_until_temp_bp(&mut self, addr: u32, session: &DebugSession) -> bool {
        let already_bp = self.installed_bps.contains(&addr);
        if !already_bp && self.exchange(&format!("Z0,{addr:x},1")).as_deref() != Some("OK") {
            return false;
        }
        if !self.send_continue() {
            if !already_bp {
                let _ = self.exchange(&format!("z0,{addr:x},1"));
            }
            return false;
        }
        self.target_running = true;

        let reached = loop {
            if self.stop_flag.load(Ordering::Acquire) {
                let _ = self.interrupt_target();
                break false;
            }
            if let Some(cmd) = session.pop_remote_cmd() {
                match cmd {
                    RemoteDebugCmd::Pause | RemoteDebugCmd::Stop => {
                        if let Some(pkt) = self.interrupt_target() {
                            self.handle_stop_reply(&pkt, session, Some(BreakReason::ManualPause));
                        }
                        break false;
                    }
                    _ => {}
                }
            }
            match self.try_recv_packet(POLL_TIMEOUT) {
                Ok(Some(pkt)) if is_stop_packet(&pkt) => {
                    self.target_running = false;
                    self.select_stop_thread(&pkt);
                    let pc = self.read_pc().unwrap_or(0);
                    self.current_pc = pc;
                    if pc == addr {
                        break true;
                    }
                    self.handle_stop_reply(&pkt, session, None);
                    break false;
                }
                Ok(_) => {}
                Err(_) => break false,
            }
        };

        if !already_bp {
            let _ = self.exchange(&format!("z0,{addr:x},1"));
        }
        reached
    }

    fn read_pc(&mut self) -> Option<u32> {
        let reg_cmd = match self.family {
            QemuFamily::Esp32 => "p0", // Xtensa PC is reg 0
            QemuFamily::Stm32 => "pf", // ARM PC is reg 15 (0xf)
        };
        let resp = self.exchange(reg_cmd)?;
        parse_le_u32_hex(&resp)
    }

    fn read_reg_u32(&mut self, reg_num: u32) -> Option<u32> {
        let resp = self.exchange(&format!("p{reg_num:x}"))?;
        parse_le_u32_hex(&resp)
    }

    fn read_return_address(&mut self, pc: u32) -> Option<u32> {
        match self.family {
            QemuFamily::Esp32 => {
                // In ESP32 Xtensa core-esp32/gdb-config.inc.c:
                // reg 1..64 = ar0..ar63, reg 69 = windowbase.
                // Live a0 = ar[(windowbase * 4) % 64].
                let wb = self.read_reg_u32(69)? & 0x0F;
                let ar_idx = (wb * 4) % 64;
                let a0 = self.read_reg_u32(1 + ar_idx)?;
                // Xtensa windowed call ABI: top 2 bits of a0 encode call increment;
                // top 2 bits of return PC come from current PC.
                let ret = (pc & 0xC000_0000) | (a0 & 0x3FFF_FFFF);
                if ret == 0 { None } else { Some(ret) }
            }
            QemuFamily::Stm32 => {
                // ARM LR is reg 14 (0xe), clear Thumb bit 0.
                let lr = self.read_reg_u32(14)? & !1;
                if lr == 0 { None } else { Some(lr) }
            }
        }
    }

    fn read_memory(&mut self, addr: u32, len: usize) -> Option<Vec<u8>> {
        if len == 0 {
            return Some(Vec::new());
        }
        let resp = self.exchange(&format!("m{addr:x},{len:x}"))?;
        if resp.is_empty() || resp.starts_with('E') {
            return None;
        }
        hex_to_bytes(&resp)
    }

    fn refresh_pc_and_watches(&mut self, session: &DebugSession) {
        if let Some(pc) = self.read_pc() {
            self.current_pc = pc;
            session.set_remote_pc(pc);
        }
        let ranges = session.watch_mem_ranges();
        if ranges.is_empty() {
            return;
        }
        let mut updates = Vec::with_capacity(ranges.len());
        for (addr, len) in ranges {
            if let Some(bytes) = self.read_memory(addr, len) {
                updates.push((addr, bytes));
            }
        }
        if !updates.is_empty() {
            session.update_remote_mem(updates);
        }
    }

    fn detach_cleanly(&mut self) {
        if self.target_running {
            let _ = self.interrupt_target();
        }
        let bps: Vec<u32> = self.installed_bps.drain().collect();
        for addr in bps {
            let _ = self.exchange(&format!("z0,{addr:x},1"));
        }
        let _ = self.exchange("D");
        self.target_running = false;
    }

    fn send_continue(&mut self) -> bool {
        self.send_packet("c")
    }

    fn send_packet(&mut self, payload: &str) -> bool {
        let pkt = format_rsp_packet(payload);
        for _ in 0..3 {
            if self.stream.write_all(pkt.as_bytes()).is_err() {
                return false;
            }
            if self.stream.flush().is_err() {
                return false;
            }
            if self.no_ack {
                return true;
            }
            match self.wait_ack(CMD_TIMEOUT) {
                Ok(true) => return true,
                Ok(false) => continue, // '-' NACK, resend
                Err(_) => return false,
            }
        }
        false
    }

    fn exchange(&mut self, payload: &str) -> Option<String> {
        if !self.send_packet(payload) {
            return None;
        }
        let deadline = Instant::now() + CMD_TIMEOUT;
        while Instant::now() < deadline {
            match self.try_recv_packet(POLL_TIMEOUT) {
                Ok(Some(pkt)) => return Some(pkt),
                Ok(None) => {}
                Err(_) => return None,
            }
        }
        None
    }

    fn wait_ack(&mut self, timeout: Duration) -> io::Result<bool> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(pos) = self.rx_buf.iter().position(|&b| b == b'+' || b == b'-') {
                let b = self.rx_buf[pos];
                // Keep any packet bytes after the ACK, discard leading ack byte.
                self.rx_buf.remove(pos);
                return Ok(b == b'+');
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "gdb ack timeout"));
            }
            self.pump_read(POLL_TIMEOUT)?;
        }
    }

    fn try_recv_packet(&mut self, timeout: Duration) -> io::Result<Option<String>> {
        if let Some(pkt) = self.extract_buffered_packet() {
            return Ok(Some(pkt));
        }
        let deadline = Instant::now() + timeout;
        loop {
            match self.pump_read(timeout.min(POLL_TIMEOUT)) {
                Ok(0) => {
                    if Instant::now() >= deadline {
                        return Ok(None);
                    }
                }
                Ok(_) => {
                    if let Some(pkt) = self.extract_buffered_packet() {
                        return Ok(Some(pkt));
                    }
                }
                Err(e) => return Err(e),
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
        }
    }

    fn pump_read(&mut self, _timeout: Duration) -> io::Result<usize> {
        let mut tmp = [0u8; 4096];
        match self.stream.read(&mut tmp) {
            Ok(0) => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "gdb stub disconnected",
            )),
            Ok(n) => {
                self.rx_buf.extend_from_slice(&tmp[..n]);
                Ok(n)
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut =>
            {
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    fn extract_buffered_packet(&mut self) -> Option<String> {
        // Discard leading '+'/'-' ACK bytes outside packets.
        while let Some(&first) = self.rx_buf.first() {
            if first == b'$' {
                break;
            }
            self.rx_buf.remove(0);
        }
        if self.rx_buf.is_empty() {
            return None;
        }
        // Find '#' after leading '$'
        let hash_pos = self.rx_buf[1..].iter().position(|&b| b == b'#')? + 1;
        if hash_pos + 3 > self.rx_buf.len() {
            return None;
        }
        let raw_payload = self.rx_buf[1..hash_pos].to_vec();
        self.rx_buf.drain(..hash_pos + 3);
        if !self.no_ack {
            let _ = self.stream.write_all(b"+");
            let _ = self.stream.flush();
        }
        let decoded = decode_rle(&raw_payload);
        Some(String::from_utf8_lossy(&decoded).into_owned())
    }
}

fn is_stop_packet(pkt: &str) -> bool {
    pkt.starts_with('T') || pkt.starts_with('S') || pkt.starts_with('W') || pkt.starts_with('X')
}

pub fn format_rsp_packet(payload: &str) -> String {
    let csum = payload.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
    format!("${payload}#{csum:02x}")
}

/// Decode GDB RSP run-length encoding (`<ch>*<count_byte>` where repeat = `count_byte - 29`).
pub fn decode_rle(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'*' && i + 1 < raw.len() && !out.is_empty() {
            let prev = *out.last().unwrap();
            let count = raw[i + 1].saturating_sub(29) as usize;
            for _ in 0..count {
                out.push(prev);
            }
            i += 2;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    out
}

fn parse_le_u32_hex(hex: &str) -> Option<u32> {
    let bytes = hex_to_bytes(hex)?;
    if bytes.len() < 4 {
        return None;
    }
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    let raw = hex.as_bytes();
    if raw.len() < 2 || raw.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    for pair in raw.chunks_exact(2) {
        let s = std::str::from_utf8(pair).ok()?;
        let b = u8::from_str_radix(s, 16).ok()?;
        out.push(b);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_decoding_expands_runs() {
        // '0' followed by '*' and byte (29 + 3 = 32 = ' ') -> four '0's
        let raw = b"0* 12";
        assert_eq!(decode_rle(raw), b"000012");
    }

    #[test]
    fn parse_le_u32_from_gdb_register_reply() {
        // 0x400d1630 in little-endian hex is "30160d40"
        assert_eq!(parse_le_u32_hex("30160d40"), Some(0x400d_1630));
    }

    #[test]
    fn format_rsp_packet_checksum() {
        assert_eq!(format_rsp_packet("c"), "$c#63");
        assert_eq!(format_rsp_packet("?"), "$?#3f");
        assert_eq!(format_rsp_packet("p0"), "$p0#a0");
    }
}
