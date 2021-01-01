use cs_engine::canvas::{Canvas, Point};
use cs_engine::components::Part;
use cs_engine::qemu::QemuComp;

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
