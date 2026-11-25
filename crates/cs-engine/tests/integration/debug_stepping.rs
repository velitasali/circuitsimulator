use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use cs_engine::canvas::Scene;
use cs_engine::debug::gdb_client;
use cs_engine::debug::{
    BreakReason, DebugSession, DebugState, DebugSymbols, ElfParser, QemuGdbClient, SourceLocation,
    VarType, WatchEntry,
};
use cs_engine::mcu::McuSnap;
use cs_engine::qemu::{QemuComp, QemuFamily};
use cs_engine::subcircuit::SubcSearch;

#[test]
fn test_debug_session_lifecycle() {
    let session = DebugSession::default();
    assert!(!session.is_active());
    assert_eq!(session.state(), DebugState::Idle);

    session.step_into();
    assert!(session.is_active());
    assert_eq!(session.state(), DebugState::SteppingInto);

    // Step from 0x00 to 0x01
    let reason = session.on_mcu_step(0x01, 0x00, 0x100, 0x00);
    assert_eq!(reason, Some(BreakReason::StepCompleted));
    assert!(session.is_paused());

    session.resume();
    assert_eq!(session.state(), DebugState::Running);

    session.stop();
    assert!(!session.is_active());
    assert_eq!(session.state(), DebugState::Idle);
}

#[test]
fn test_debug_breakpoints() {
    let session = DebugSession::default();
    session.set_active(true);
    session.resume();

    // Add address breakpoint at 0x0020
    session.add_addr_breakpoint(0x0020);
    assert!(session.has_addr_breakpoint(0x0020));

    // Execution hits 0x0010 (no break)
    assert_eq!(session.on_mcu_step(0x0010, 0x000F, 0x100, 0), None);

    // Execution hits 0x0020 (break hit)
    let reason = session.on_mcu_step(0x0020, 0x001F, 0x100, 0);
    assert_eq!(
        reason,
        Some(BreakReason::Breakpoint {
            address: 0x0020,
            location: None
        })
    );
    assert!(session.is_paused());

    // Remove breakpoint
    session.remove_addr_breakpoint(0x0020);
    assert!(!session.has_addr_breakpoint(0x0020));
}

#[test]
fn test_debug_step_over() {
    let session = DebugSession::default();
    session.set_active(true);

    // Simulate stepping over a CALL at PC=0x10, return addr=0x12, initial SP=0x01FF
    session.step_over_target(0x01FF, Some(0x12));

    // While in subroutine: PC=0x50, SP=0x01FD (nested call stack, depth > target_sp)
    // No break
    assert_eq!(session.on_mcu_step(0x50, 0x10, 0x01FD, 0x12), None);

    // When subroutine returns to exit_pc: PC=0x12
    let reason = session.on_mcu_step(0x12, 0x55, 0x01FF, 0x00);
    assert_eq!(reason, Some(BreakReason::StepCompleted));
    assert!(session.is_paused());
}

#[test]
fn test_debug_step_out() {
    let session = DebugSession::default();
    session.set_active(true);

    // Inside subroutine with SP=0x01FD, stepping out to target SP >= 0x01FF
    session.step_out_target(0x01FF);

    // Subroutine returns and SP pops back to 0x01FF
    let reason = session.on_mcu_step(0x12, 0x55, 0x01FF, 0x12);
    assert_eq!(reason, Some(BreakReason::StepCompleted));
    assert!(session.is_paused());
}

#[test]
fn test_elf_parser_avr_word_addresses_and_breakpoint_resync() {
    let elf_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Mega_Display/Mega_Display.elf");
    let bytes = fs::read(&elf_path).expect("read Mega_Display.elf");
    let syms = ElfParser::parse(&bytes).expect("parse AVR ELF");

    // In Mega_Display.elf, line 53 is at byte address 0x3430 -> AVR word PC 0x1A18.
    let line53 = syms.resolve_breakpoint_addrs(Path::new("Mega_Display.ino"), 53);
    assert_eq!(
        line53,
        vec![0x3430 / 2],
        "AVR DWARF line 53 must map to word PC"
    );

    // Adding a breakpoint BEFORE loading symbols must resolve once symbols load.
    let session = DebugSession::default();
    session.add_breakpoint("Mega_Display.ino", 53);
    assert!(session.addr_breakpoints().is_empty());
    session.load_symbols(syms);
    assert!(session.has_addr_breakpoint(0x3430 / 2));

    // Stepping onto that PC must pause execution with BreakReason::Breakpoint.
    session.resume();
    let hit = session.on_mcu_step(0x3430 / 2, 0x1000, 0x08FF, 0);
    assert!(matches!(
        hit,
        Some(BreakReason::Breakpoint { address, .. }) if address == 0x3430 / 2
    ));
    assert!(session.is_paused());
}

#[test]
fn test_qemu_scene_mcu_snap_and_remote_watch_eval() {
    let mut scene = Scene::new();
    let search = SubcSearch::default();
    let id = scene
        .add_qemu("Esp32", 0.0, 0.0, &search)
        .expect("add Esp32");

    let snap = scene.mcu_snap();
    assert_eq!(snap.id, id);
    assert!(!snap.has_status);
    assert_eq!(snap.word_size, 1);

    let mut syms = DebugSymbols::new();
    syms.insert_variable("counter", "uint32", 0x3FFC_1000, 4, false);

    let session = DebugSession::default();
    session.load_symbols(syms);
    session.add_watch(WatchEntry::new("counter", "counter", VarType::Uint32));

    // Before remote memory is populated, watch evaluates to placeholder with resolved address.
    let before = session.evaluate_watches(&snap);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].address, Some(0x3FFC_1000));
    assert_eq!(before[0].formatted, "---");

    // Once QEMU GDB supplies the 4 bytes (0x0000002A = 42 LE), watch shows live value.
    session.update_remote_mem(vec![(0x3FFC_1000, 42u32.to_le_bytes().to_vec())]);
    let after = session.evaluate_watches(&snap);
    assert_eq!(after[0].dec, "42");
    assert_eq!(after[0].hex, "0x0000002A");
}

#[test]
fn test_gdb_client_mock_esp32_breakpoint_and_watch_read() {
    // Use an isolated non-global session so parallel integration tests are never affected.
    let session: &'static DebugSession = Box::leak(Box::new(DebugSession::default()));

    let mut syms = DebugSymbols::new();
    syms.insert_line(0x400d_166c, SourceLocation::new("blink.ino", 12));
    syms.insert_line(0x400d_1670, SourceLocation::new("blink.ino", 13));
    syms.insert_variable("ledState", "uint32", 0x3FFC_2000, 4, false);
    session.set_breakpoints_for_file(Path::new("blink.ino"), &[12]);
    session.load_symbols(syms);
    session.add_watch(WatchEntry::new("ledState", "ledState", VarType::Uint32));
    session.resume();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind mock gdb");
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let mut pc: u32 = 0x4000_0400;
        let mut bp_set = false;
        let mut no_ack = false;
        let mut buf = [0u8; 512];
        let mut acc = Vec::new();

        loop {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => acc.extend_from_slice(&buf[..n]),
            }
            while let Some(dollar) = acc.iter().position(|&b| b == b'$') {
                acc.drain(..dollar);
                let Some(hash) = acc[1..].iter().position(|&b| b == b'#').map(|i| i + 1) else {
                    break;
                };
                if hash + 3 > acc.len() {
                    break;
                }
                let cmd = String::from_utf8_lossy(&acc[1..hash]).to_string();
                acc.drain(..hash + 3);
                if !no_ack {
                    let _ = stream.write_all(b"+");
                }

                let reply: Option<String> = if cmd.starts_with("qSupported") {
                    Some("PacketSize=1000;QStartNoAckMode+;swbreak+".into())
                } else if cmd == "QStartNoAckMode" {
                    no_ack = true;
                    Some("OK".into())
                } else if cmd == "?" {
                    Some("S05".into())
                } else if cmd == "Z0,400d166c,1" {
                    bp_set = true;
                    Some("OK".into())
                } else if cmd.starts_with('z') || cmd.starts_with('H') {
                    Some("OK".into())
                } else if cmd == "p0" {
                    let b = pc.to_le_bytes();
                    Some(format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3]))
                } else if cmd == "m3ffc2000,4" {
                    // Value = 1337 (0x00000539 LE -> 39050000)
                    Some("39050000".into())
                } else if cmd == "c" {
                    assert!(bp_set, "breakpoint must be installed before continue");
                    pc = 0x400d_166c;
                    Some("T05thread:02;".into())
                } else if cmd == "D" {
                    let pkt = gdb_client::format_rsp_packet("OK");
                    let _ = stream.write_all(pkt.as_bytes());
                    return;
                } else {
                    Some(String::new())
                };
                if let Some(r) = reply {
                    let pkt = gdb_client::format_rsp_packet(&r);
                    if stream.write_all(pkt.as_bytes()).is_err() {
                        return;
                    }
                    let _ = stream.flush();
                }
            }
        }
    });

    let mut client =
        QemuGdbClient::connect_for_session(port, QemuFamily::Esp32, session).expect("connect gdb");

    let deadline = Instant::now() + Duration::from_secs(3);
    while !session.is_paused() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }

    assert!(
        session.is_paused(),
        "QemuGdbClient should pause on T05 breakpoint"
    );
    assert_eq!(session.remote_pc(), 0x400d_166c);
    assert_eq!(
        session.active_location().map(|l| l.line),
        Some(12),
        "active source location should match breakpoint line"
    );

    let snap = McuSnap::from_qemu("Esp32-1", session.remote_pc());
    let watches = session.evaluate_watches(&snap);
    assert_eq!(watches[0].dec, "1337");

    client.shutdown();
    let _ = server.join();
    session.stop();
}

/// Live end-to-end test against a real `qemu-system-xtensa` + compiled ESP32 sketch.
/// Run explicitly with: `cargo test -p cs-engine --test integration test_live_esp32_qemu_breakpoint -- --ignored --nocapture`
#[test]
#[ignore]
fn test_live_esp32_qemu_breakpoint() {
    let Some(local_app_data) = env::var_os("LOCALAPPDATA") else {
        return;
    };
    let build_dir = PathBuf::from(local_app_data)
        .join("CircuitSimulator/arduino_build/19df039f70e9ea28_esp32.esp32.esp32doit-devkit-v1");
    let elf_path = build_dir.join("esp32_uart.ino.elf");
    let merged_bin = build_dir.join("esp32_uart.ino.merged.bin");
    if !elf_path.is_file() || !merged_bin.is_file() {
        eprintln!("Skipping live ESP32 test: build artifacts not found");
        return;
    }

    // Copy merged.bin to a temp file named `esp32_bp_test.bin` (esp32_args appends `.bin` to stem).
    let tmp_dir = env::temp_dir().join("cs_esp32_bp_test");
    let _ = fs::create_dir_all(&tmp_dir);
    let bin_path = tmp_dir.join("esp32_bp_test.bin");
    fs::copy(&merged_bin, &bin_path).expect("copy esp32 merged bin");

    let elf_bytes = fs::read(&elf_path).expect("read esp32 elf");
    let syms = ElfParser::parse(&elf_bytes).expect("parse esp32 elf");

    let line5_addrs = syms.resolve_breakpoint_addrs(Path::new("esp32_uart.ino"), 5);
    assert!(
        !line5_addrs.is_empty(),
        "esp32_uart.ino line 5 must resolve in DWARF v4 ELF"
    );
    let expected_pc = line5_addrs[0];
    eprintln!("Resolved esp32_uart.ino:5 -> 0x{expected_pc:08x}");

    let session = DebugSession::global();
    session.stop();
    session.clear_watches();
    session.set_breakpoints_for_file(Path::new("esp32_uart.ino"), &[5]);
    session.load_symbols(syms);
    // Watch a real DRAM symbol from the ELF
    if let Some(var) = session
        .debug_variables()
        .into_iter()
        .find(|v| v.address >= 0x3FFB_0000 && v.address < 0x3FFF_0000 && v.size == 4)
    {
        eprintln!("Watching DRAM symbol {} @ 0x{:08x}", var.name, var.address);
        session.add_watch(WatchEntry::new(var.name.clone(), var.name, VarType::Uint32));
    }
    session.resume();

    let mut q = QemuComp::esp32("Esp32-1");
    q.firmware = bin_path.to_string_lossy().into_owned();
    q.stamp_init(1);
    assert!(q.is_live(), "QEMU failed to start: {:?}", q.last_error);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut now = 1u64;
    let mut acts = Vec::new();
    while !session.is_paused() && Instant::now() < deadline {
        let horizon = now.saturating_add(16_000_000_000);
        q.run_event_nodes_buf(
            &[],
            now,
            horizon,
            Some(Instant::now() + Duration::from_millis(12)),
            &mut acts,
        );
        now = horizon;
    }

    assert!(
        session.is_paused(),
        "Real QEMU ESP32 must stop at breakpoint 0x{expected_pc:08x}! (remote_pc=0x{:08x})",
        session.remote_pc()
    );
    let hit_pc = session.remote_pc();
    let hit_loc = session.active_location();
    eprintln!("HIT BREAKPOINT! PC=0x{hit_pc:08x}, loc={hit_loc:?}");
    assert_eq!(hit_pc, expected_pc);
    assert_eq!(hit_loc.as_ref().map(|l| l.line), Some(5));

    // Verify watched DRAM variable was actually read from live QEMU!
    let snap = McuSnap::from_qemu("Esp32-1", hit_pc);
    let watches = session.evaluate_watches(&snap);
    if !watches.is_empty() {
        eprintln!("Watch value: {:?}", watches[0]);
        assert_ne!(
            watches[0].formatted, "---",
            "ESP32 watch should evaluate live RAM"
        );
    }

    // Verify Step Over steps OVER Serial.begin(115200) directly to line 7 (delay(1000))!
    session.step_over();
    let step_deadline = Instant::now() + Duration::from_secs(5);
    while !session.is_paused() && Instant::now() < step_deadline {
        let horizon = now.saturating_add(16_000_000_000);
        q.run_event_nodes_buf(
            &[],
            now,
            horizon,
            Some(Instant::now() + Duration::from_millis(12)),
            &mut acts,
        );
        now = horizon;
    }
    assert!(
        session.is_paused(),
        "Step over should pause at next source line"
    );
    let next_loc = session.active_location();
    eprintln!(
        "AFTER STEP OVER: PC=0x{:08x}, loc={next_loc:?}",
        session.remote_pc()
    );
    assert_eq!(next_loc.as_ref().map(|l| l.line), Some(7));

    q.stop_live();
    session.stop();
    session.clear_watches();
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_elf_cache_and_fast_break_pcs() {
    let elf_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Mega_Display/Mega_Display.elf");
    let session = DebugSession::default();
    assert!(!session.has_symbols());

    assert!(session.load_elf_if_changed(&elf_path));
    assert!(session.has_symbols());
    let gen1 = session.symbols_generation();

    // Calling again on the unchanged ELF must hit the stamp cache without re-parsing.
    assert!(session.load_elf_if_changed(&elf_path));
    assert_eq!(session.symbols_generation(), gen1);

    // Line breakpoints populate `fast_break_pcs` (sorted, deduped) for batch checking.
    session.add_breakpoint("Mega_Display.ino", 53);
    let fast = session.fast_break_pcs();
    assert!(fast.contains(&(0x3430 / 2)));
    assert!(fast.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn test_batch_debug_hits_breakpoint_step_over_and_resumes() {
    use cs_engine::debug::BatchDebugState;
    use cs_mcu::BatchDebugMode;

    const MINI_AVR: &str = r#"
<mcu name="mini" core="AVR" data="256" prog="64" progword="2" inst_cycle="1" freq="16000000">
  <datablock name="GPR" start="0x0000" end="0x001F"/>
  <regblock name="SFR" start="0x0020" end="0x005F">
    <register name="SREG" addr="0x005F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
    <register name="SPL" addr="0x005D" reset="0xFF"/>
    <register name="SPH" addr="0x005E" reset="0x00"/>
  </regblock>
  <datablock name="RAM" start="0x0060" end="0x00FF"/>
  <stack spreg="SPL,SPH" increment="postdec"/>
</mcu>
"#;
    let mut dev = cs_mcu::Device::from_xml("U1", MINI_AVR).expect("build AVR");
    // Program:
    // 0: LDI R16, 0x00   (0xE000)
    // 1: INC R16         (0x9503)
    // 2: CPI R16, 0x10   (0x3100)
    // 3: BRNE -3 (pc=1)  (0xF7E9)
    // 4: NOP             (0x0000)  <-- breakpoint here!
    // 5: RJMP -1 (pc=5)  (0xCFFF)
    dev.load_words(&[0xE000, 0x9503, 0x3100, 0xF7E9, 0x0000, 0xCFFF]);
    dev.start();
    dev.clear_ports_dirty();

    // Use an isolated non-global session so parallel integration tests are never affected.
    let session = DebugSession::default();
    session.add_addr_breakpoint(4);
    session.resume();
    assert_eq!(session.batch_debug_state(), BatchDebugState::Running);

    let break_pcs = session.fast_break_pcs();
    let stop = dev.advance_batch(
        u64::MAX,
        4096,
        BatchDebugMode::Running {
            break_pcs: &break_pcs,
        },
    );
    let (prev_pc, pc, sp, ret_addr) = stop.debug_hit.expect("batch must hit breakpoint at PC=4");
    assert_eq!(pc, 4);
    assert_eq!(dev.ram_dump()[16], 0x10);
    let reason = session.on_mcu_step(pc, prev_pc, sp, ret_addr);
    assert!(matches!(
        reason,
        Some(BreakReason::Breakpoint { address: 4, .. })
    ));
    assert!(session.is_paused());

    // Resume past the breakpoint; next batch advances to PC=5 and does not re-trigger PC=4.
    session.resume();
    let stop2 = dev.advance_batch(
        10_000_000,
        4096,
        BatchDebugMode::Running {
            break_pcs: &break_pcs,
        },
    );
    assert!(
        stop2.debug_hit.is_none(),
        "resuming past PC=4 must not re-trigger PC=4"
    );
    assert_eq!(dev.pc(), 5);
}
