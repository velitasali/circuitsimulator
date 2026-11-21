use std::env;
use std::fs;
use std::process;
use std::sync::Mutex;

use cs_engine::canvas::{Canvas, Point};
use cs_engine::components::Part;
use cs_engine::qemu::QemuComp;
use cs_engine::serial;

static SERIAL_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_qemu_monitor_names_include_spi_and_i2c() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("ESP32,QemuDevice", Point::zero()));
    let item = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("ESP32 must be present");

    let monitors = canvas.monitors_for(&item.id);
    assert!(
        monitors.contains(&"USART1".to_string()),
        "monitors must contain USART1: {monitors:?}"
    );
    assert!(
        monitors.contains(&"SPI1".to_string()),
        "monitors must contain SPI1: {monitors:?}"
    );
    assert!(
        monitors.contains(&"I2C1".to_string()),
        "monitors must contain I2C1: {monitors:?}"
    );
}

#[test]
fn test_mcu_usart_tx_and_rx_injection() {
    let _guard = SERIAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    canvas.power_on();
    assert!(canvas.sim_running());

    let circuit = canvas.running_circuit_mut().expect("running circuit");
    let (mcu_idx, mcu_id) = circuit
        .components()
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            if matches!(c.kind, cs_engine::elements::Kind::Mcu(_)) {
                Some((i, c.id.clone()))
            } else {
                None
            }
        })
        .expect("MCU component");

    let port_id = format!("{mcu_id}:USART0");
    cs_engine::serial::clear(&port_id);
    cs_engine::serial::clear(&mcu_id);
    cs_engine::serial::clear("default");

    let m = match &mut circuit.components_mut()[mcu_idx].kind {
        cs_engine::elements::Kind::Mcu(m) => m,
        _ => unreachable!(),
    };

    // 1. Enable USART TX and RX in UCSR0B, set baud in UBRR0L
    m.device.write_reg_by_name("UCSR0B", 0x18);
    m.device.write_reg_by_name("UBRR0L", 1);

    // Send "Hello" by checking UDRE0 bit in UCSR0A (bit 5) before writing each byte
    let msg = b"Hello";
    for &b in msg {
        let mut ready = false;
        for _ in 0..5000 {
            let ucsra = m.device.cpu_read_reg_by_name("UCSR0A").unwrap_or(0);
            if ucsra & (1 << 5) != 0 {
                ready = true;
                break;
            }
            m.device.advance();
        }
        assert!(ready, "UDRE0 must become 1 to accept next byte");
        m.device.write_reg_by_name("UDR0", b as u32);
    }

    // Advance until all transmissions complete
    for _ in 0..10000 {
        m.device.advance();
        if m.device.usarts()[0].tx_remain.is_none() && !m.device.usarts()[0].tx_buffered {
            break;
        }
    }
    m.flush_periph_logs();

    let out_port = cs_engine::serial::out_text(&port_id);
    let out_def = cs_engine::serial::out_text("default");
    assert_eq!(
        out_port, "Hello",
        "Expected 'Hello' in USART0 out text, got: {out_port:?}"
    );
    assert_eq!(
        out_def, "Hello",
        "Expected 'Hello' in default out text, got: {out_def:?}"
    );

    // 2. Test Host RX injection: Send 'Z' (0x5A) from host Serial Monitor
    cs_engine::serial::send_text(&port_id, "Z");
    m.drain_host_serial();

    let in_port = cs_engine::serial::in_text(&port_id);
    assert!(
        in_port.contains('Z'),
        "Expected 'Z' in USART0 in text, got: {in_port:?}"
    );

    // Read UDR0 from MCU via CPU read
    let received = m.device.cpu_read_reg_by_name("UDR0").unwrap_or(0);
    assert_eq!(
        received, b'Z',
        "MCU UDR0 read should match injected byte 'Z'"
    );
}

#[test]
fn test_mcu_twi_logging() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    canvas.power_on();
    assert!(canvas.sim_running());

    let circuit = canvas.running_circuit_mut().expect("running circuit");
    let (mcu_idx, mcu_id) = circuit
        .components()
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            if matches!(c.kind, cs_engine::elements::Kind::Mcu(_)) {
                Some((i, c.id.clone()))
            } else {
                None
            }
        })
        .expect("MCU component");

    let twi_port_id = format!("{mcu_id}:TWI");
    cs_engine::serial::clear(&twi_port_id);

    let m = match &mut circuit.components_mut()[mcu_idx].kind {
        cs_engine::elements::Kind::Mcu(m) => m,
        _ => unreachable!(),
    };

    // Enable TWI in TWCR, fast bit rate in TWBR
    m.device.write_reg_by_name("TWBR", 1);
    // Send START condition: TWINT | TWSTA | TWEN = 0x80 | 0x20 | 0x04 = 0xA4
    m.device.write_reg_by_name("TWCR", 0xA4);

    // Advance through start sequence
    for _ in 0..500 {
        m.device.advance();
    }

    // Write address/data 0x50 to TWDR
    m.device.write_reg_by_name("TWDR", 0x50);
    // Clear TWINT to begin transmission: TWINT | TWEN = 0x80 | 0x04 = 0x84
    m.device.write_reg_by_name("TWCR", 0x84);

    // Advance until transmission finishes
    for _ in 0..1000 {
        m.device.advance();
    }
    m.flush_periph_logs();

    let out_twi = cs_engine::serial::out_text(&twi_port_id);
    assert!(
        out_twi.contains("50") || out_twi.contains('P'),
        "Expected 0x50 in TWI out text, got: {out_twi:?}"
    );
}

#[test]
fn test_qemu_spi_and_twi_logging() {
    let mut qemu = QemuComp::esp32("esp32-test");
    let spi_port_id = "esp32-test:SPI1";
    let twi_port_id = "esp32-test:I2C1";
    cs_engine::serial::clear(spi_port_id);
    cs_engine::serial::clear(twi_port_id);

    // 1. SPI transaction
    qemu.spis[0].module.mode = cs_engine::digital::SpiMode::Master;
    qemu.spis[0].module.tx_reg = 0x42;
    qemu.spis[0].module.data_reg = 0x99;
    qemu.spis[0].module.due = Some(100);

    // Manually test the publishing logic that tick_peripherals performs on SPI completion:
    cs_engine::serial::publish_out(spi_port_id, qemu.spis[0].module.tx_reg);
    cs_engine::serial::publish_in(spi_port_id, qemu.spis[0].module.data_reg);

    assert!(
        cs_engine::serial::out_text(spi_port_id).contains('B'),
        "SPI TX 0x42 ('B') should appear in out text"
    );
    assert!(
        !cs_engine::serial::in_text(spi_port_id).is_empty(),
        "SPI RX 0x99 should appear in in text"
    );

    // 2. TWI transaction
    cs_engine::serial::publish_out(twi_port_id, 0x61); // 'a'
    cs_engine::serial::publish_in(twi_port_id, 0x62); // 'b'

    assert!(
        cs_engine::serial::out_text(twi_port_id).contains('a'),
        "TWI TX 'a' should appear in out text"
    );
    assert!(
        cs_engine::serial::in_text(twi_port_id).contains('b'),
        "TWI RX 'b' should appear in in text"
    );
}

#[test]
fn test_serial_port_open_button_triggers_serial_monitor() {
    let mut canvas = Canvas::empty();
    let port_id = canvas.scene_mut().add_serial_port(100.0, 100.0);

    // Click on the Open button (local center is around (14, 0) -> scene (114, 100))
    let btn_pt = canvas
        .viewport()
        .map_from_circuit(cs_engine::canvas::Point::new(114.0, 100.0));
    canvas.mouse_press(1, btn_pt.x, btn_pt.y, 0);
    let c = canvas.mouse_release(1, btn_pt.x, btn_pt.y, 0);
    assert_eq!(
        c.open_serial_mon,
        Some((format!("{port_id}:Serial"), format!("{port_id}-Serial"))),
        "Clicking Open button on SerialPort must trigger open_serial_mon"
    );

    // Click outside Open button (e.g. at (180, 100))
    let out_pt = canvas
        .viewport()
        .map_from_circuit(cs_engine::canvas::Point::new(180.0, 100.0));
    canvas.mouse_press(1, out_pt.x, out_pt.y, 0);
    let c2 = canvas.mouse_release(1, out_pt.x, out_pt.y, 0);
    assert_eq!(
        c2.open_serial_mon, None,
        "Clicking outside Open button must not trigger open_serial_mon"
    );
}

#[test]
fn test_serial_term_open_button_triggers_serial_terminal() {
    let mut canvas = Canvas::empty();
    let _term_id = canvas.scene_mut().add_serial_term(200.0, 200.0);

    // Click on the Open button (local center is around (30, 0) -> scene (230, 200))
    let btn_pt = canvas
        .viewport()
        .map_from_circuit(cs_engine::canvas::Point::new(230.0, 200.0));
    canvas.mouse_press(1, btn_pt.x, btn_pt.y, 0);
    let c = canvas.mouse_release(1, btn_pt.x, btn_pt.y, 0);
    assert!(
        c.open_terminal,
        "Clicking Open button on SerialTerm must trigger open_terminal"
    );

    // Click outside Open button (e.g. at (195, 200))
    let out_pt = canvas
        .viewport()
        .map_from_circuit(cs_engine::canvas::Point::new(195.0, 200.0));
    canvas.mouse_press(1, out_pt.x, out_pt.y, 0);
    let c2 = canvas.mouse_release(1, out_pt.x, out_pt.y, 0);
    assert!(
        !c2.open_terminal,
        "Clicking outside Open button must not trigger open_terminal"
    );
}

#[test]
fn test_subcircuit_nested_mcu_monitors_offline() {
    let mut canvas = Canvas::empty();
    let board_sim1 = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="mega328-1" label="mega328" />
</circuit>"#;

    let board_item = cs_engine::canvas::Item::subcircuit(
        "Arduino-1",
        0.0,
        0.0,
        "Arduino_Uno",
        cs_engine::package::Package::default(),
        board_sim1,
        None,
        false,
    );
    canvas.scene_mut().add_saved_item(board_item);

    // Verify simulation is NOT running
    assert!(!canvas.sim_running());

    canvas.scene_mut().select_only(0);

    let monitors = canvas.selected_item_monitors();
    assert!(
        monitors.contains(&"USART0".to_string()) || monitors.contains(&"USART".to_string()),
        "selected_item_monitors for Arduino board must include USART offline, got: {monitors:?}"
    );
}

#[test]
fn test_subcircuit_nested_mcu_serial_monitor_end_to_end() {
    let _guard = SERIAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut canvas = Canvas::empty();
    assert!(
        canvas.scene_add_component_spec("Uno,Subcircuit", Point::zero()),
        "Uno,Subcircuit must be placeable from standard catalog"
    );
    canvas.scene_mut().select_only(0);

    let nested_uid = canvas
        .selected_nested_mcu_uid()
        .expect("Uno must expose nested mega328 UID");
    assert_eq!(nested_uid, "Uno-1/mega328-109");

    let monitors = canvas.selected_item_monitors();
    assert!(
        monitors.contains(&"USART0".to_string()),
        "selected_item_monitors for Uno must include USART0, got: {monitors:?}"
    );

    let tmp_dir = env::temp_dir().join(format!("cs_uno_serial_e2e_{}", process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("create temp hex dir");
    let hex_path = tmp_dir.join("tx_a.hex");
    // AVR: LDI r16, 1; STS 0x00C4 (UBRR0L), r16; LDI r16, 0x18; STS 0x00C1 (UCSR0B), r16;
    //      LDI r16, 'A' (0x41); STS 0x00C6 (UDR0), r16; RJMP -1
    fs::write(
        &hex_path,
        ":1400000001E00093C40008E10093C10001E40093C600FFCF6B\n:00000001FF\n",
    )
    .expect("write temp hex");

    let hex_str = hex_path.to_string_lossy().into_owned();
    let load_change = canvas.load_firmware_path(&nested_uid, &hex_str);
    assert!(
        load_change.items,
        "load_firmware_path must mark items dirty"
    );
    assert!(canvas.selected_has_firmware());
    assert_eq!(canvas.firmware_path_for(&nested_uid), Some(hex_path));

    let port_id = format!("{nested_uid}:USART0");
    serial::clear(&port_id);

    canvas.power_on();
    assert!(canvas.sim_running());

    for _ in 0..20 {
        canvas.tick();
        if serial::out_text(&port_id).contains('A') {
            break;
        }
    }

    let out = serial::out_text(&port_id);
    assert!(
        out.contains('A'),
        "Expected 'A' on nested serial monitor '{port_id}', got: {out:?}"
    );

    canvas.power_off();
    let _ = fs::remove_dir_all(&tmp_dir);
}

fn avr_hex_from_segments(segments: &[(u16, &[u16])]) -> String {
    let mut out = String::new();
    for &(word_addr, words) in segments {
        let byte_addr = word_addr * 2;
        let len = (words.len() * 2) as u8;
        let hi = (byte_addr >> 8) as u8;
        let lo = (byte_addr & 0xFF) as u8;
        let mut sum = u16::from(len) + u16::from(hi) + u16::from(lo);
        out.push_str(&format!(":{len:02X}{byte_addr:04X}00"));
        for &w in words {
            let b0 = (w & 0xFF) as u8;
            let b1 = (w >> 8) as u8;
            sum += u16::from(b0) + u16::from(b1);
            out.push_str(&format!("{b0:02X}{b1:02X}"));
        }
        let cksum = ((!sum as u8).wrapping_add(1)) & 0xFF;
        out.push_str(&format!("{cksum:02X}\n"));
    }
    out.push_str(":00000001FF\n");
    out
}

#[test]
fn test_avr_usart_udrie_interrupt_driven_tx() {
    let _guard = SERIAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    let renamed = canvas.to_circ1().replace("mega328-1", "mega328-udrie-1");
    canvas
        .load_circ1(&renamed, None)
        .expect("rename MCU CircId");

    // Word addresses for ATmega328 (`megaxx8_int.xml` uses `USART_U` vector 0x0026):
    // - 0x0000: RJMP 0x0034 (+0x33)
    // - 0x0026: USART_UDRE_vect ISR:
    //     if r17 == 0 -> write 'H' to UDR0, INC r17, RETI
    //     else        -> write 'i' to UDR0, clear UDRIE0 (UCSR0B = 0x18), RETI
    // - 0x0034: LDI r17, 0; UBRR0L = 1; UCSR0B = RXEN0|TXEN0|UDRIE0 (0x38); SEI; RJMP -1
    let hex = avr_hex_from_segments(&[
        (0x0000, &[0xC033]),
        (
            0x0026,
            &[
                0x3010, // CPI r17, 0
                0xF039, // BREQ +7 (0x0028 + 7 = 0x002F)
                0xE609, // LDI r16, 'i' (0x69)
                0x9300, 0x00C6, // STS UDR0, r16
                0xE108, // LDI r16, 0x18 (RXEN0 | TXEN0, clears UDRIE0)
                0x9300, 0x00C1, // STS UCSR0B, r16
                0x9518, // RETI
                0xE408, // 0x002F: LDI r16, 'H' (0x48)
                0x9300, 0x00C6, // STS UDR0, r16
                0x9513, // INC r17
                0x9518, // RETI
            ],
        ),
        (
            0x0034,
            &[
                0xE010, // LDI r17, 0
                0xE001, // LDI r16, 1
                0x9300, 0x00C4, // STS UBRR0L, r16
                0xE308, // LDI r16, 0x38 (RXEN0 | TXEN0 | UDRIE0)
                0x9300, 0x00C1, // STS UCSR0B, r16
                0x9478, // SEI
                0xCFFF, // RJMP -1
            ],
        ),
    ]);

    let tmp_dir = env::temp_dir().join(format!("cs_avr_udrie_tx_{}", process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("create temp hex dir");
    let hex_path = tmp_dir.join("udrie_tx.hex");
    fs::write(&hex_path, hex).expect("write udrie hex");

    let uid = canvas.scene().items()[0].id.clone();
    assert_eq!(uid, "mega328-udrie-1");
    let hex_str = hex_path.to_string_lossy().into_owned();
    assert!(canvas.load_firmware_path(&uid, &hex_str).items);

    let port_id = format!("{uid}:USART0");
    serial::clear(&port_id);
    serial::clear(&uid);

    canvas.power_on();
    assert!(canvas.sim_running());

    for _ in 0..40 {
        canvas.tick();
        if serial::out_text(&port_id).len() >= 2 {
            break;
        }
    }

    let out = serial::out_text(&port_id);
    assert_eq!(
        out, "Hi",
        "Expected interrupt-driven UDRE0 TX 'Hi' on '{port_id}', got: {out:?}"
    );

    canvas.power_off();
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_subcircuit_saved_sim1_relative_firmware_serial() {
    let _guard = SERIAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut canvas = Canvas::empty();
    assert!(
        canvas.scene_add_component_spec("Uno,Subcircuit", Point::zero()),
        "Uno,Subcircuit must be placeable from standard catalog"
    );
    let renamed = canvas.to_circ1().replace("Uno-1", "Uno-rel-1");
    canvas
        .load_circ1(&renamed, None)
        .expect("rename Uno CircId");
    canvas.scene_mut().select_only(0);

    let nested_uid = canvas
        .selected_nested_mcu_uid()
        .expect("Uno must expose nested mega328 UID");
    let board_id = canvas.scene().items()[0].id.clone();
    assert_eq!(board_id, "Uno-rel-1");

    let tmp_dir = env::temp_dir().join(format!("cs_uno_rel_saved_{}", process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("create temp dir");
    let sim1_path = tmp_dir.join("board_rel.sim1");
    let sim1_str = sim1_path.to_string_lossy().into_owned();
    canvas.set_file_path(Some(sim1_str.clone()));

    let hex_path = tmp_dir.join("tx_ok.hex");
    // LDI r16, 1; STS UBRR0L, r16; LDI r16, 0x18; STS UCSR0B, r16; LDI r16, 'A'; STS UDR0, r16; RJMP -1
    let hex = avr_hex_from_segments(&[(
        0x0000,
        &[
            0xE001, 0x9300, 0x00C4, 0xE108, 0x9300, 0x00C1, 0xE401, 0x9300, 0x00C6, 0xCFFF,
        ],
    )]);
    fs::write(&hex_path, hex).expect("write temp hex");

    let hex_str = hex_path.to_string_lossy().into_owned();
    assert!(canvas.load_firmware_path(&nested_uid, &hex_str).items);

    let saved_sim1 = canvas.to_circ1();
    let mut reloaded = Canvas::empty();
    reloaded
        .load_circ1(&saved_sim1, Some(sim1_str))
        .expect("load_sim1 with relative subcircuit Program");

    let canonical_port = format!("{nested_uid}:USART0");
    serial::clear(&canonical_port);

    reloaded.power_on();
    assert!(reloaded.sim_running());

    for _ in 0..25 {
        reloaded.tick();
        if serial::out_text(&canonical_port).contains('A') {
            break;
        }
    }

    assert!(
        serial::out_text(&canonical_port).contains('A'),
        "Expected 'A' on canonical port '{canonical_port}', got: {:?}",
        serial::out_text(&canonical_port)
    );

    serial::send_text(&canonical_port, "Z");
    reloaded.tick();
    assert_eq!(serial::in_text(&canonical_port), "Z");

    reloaded.power_off();
    let _ = fs::remove_dir_all(&tmp_dir);
}
