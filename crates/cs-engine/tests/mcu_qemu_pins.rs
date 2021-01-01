use cs_engine::canvas::{Canvas, PinDirection, Point};
use cs_engine::components::Part;
use cs_engine::digital::PinMode;
use cs_engine::qemu::QemuComp;

#[test]
fn test_mcu_gpio_pins_default_direction_and_reset() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    let item = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .expect("MCU must be present");

    // GPIO pins default to "in"
    assert_eq!(item.default_pin_direction("PORTB0"), Some(PinDirection::In));
    assert_eq!(item.default_pin_direction("PORTD7"), Some(PinDirection::In));

    // Reset and NC pins should not display a direction chevron
    assert_eq!(item.default_pin_direction("RESET"), None);
}

#[test]
fn test_qemu_esp32_gpio_pins_default_direction_and_reset() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("ESP32,QemuDevice", Point::zero()));
    let item = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("ESP32 must be present");

    assert_eq!(item.default_pin_direction("G05"), Some(PinDirection::In));
    assert_eq!(item.default_pin_direction("Rst"), None);
}

#[test]
fn test_qemu_stm32_gpio_pins_default_direction() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("STM32F103,QemuDevice", Point::zero()));
    let item = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("STM32 must be present");

    assert_eq!(item.default_pin_direction("PA0"), Some(PinDirection::In));
}

fn avr_ldi(r: u8, k: u8) -> u16 {
    let h = r - 16;
    0xE000 | (u16::from(k & 0xF0) << 4) | (u16::from(h) << 4) | u16::from(k & 0x0F)
}

fn avr_out(io: u8, r: u8) -> u16 {
    0xB800 | (u16::from((io >> 4) & 3) << 9) | (u16::from(r) << 4) | u16::from(io & 0x0F)
}

fn avr_rjmp(k: i16) -> u16 {
    0xC000 | (k as u16) & 0x0FFF
}

#[test]
fn test_circuit_dynamic_pin_direction_pullup_and_unconnected_voltage() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    let mcu_item = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .unwrap()
        .clone();
    let mcu_id = mcu_item.id.clone();
    let pb5_id = format!("{mcu_id}-PORTB5");

    canvas.power_on();
    assert!(canvas.sim_running());

    // Initially pins default to "in"
    assert_eq!(canvas.pin_direction(&pb5_id), Some(PinDirection::In));

    // Program MCU to set DDRB5=1 (output) and PORTB5=1 (high):
    // LDI r16, 0x20
    // OUT 0x04, r16 (DDRB)
    // OUT 0x05, r16 (PORTB)
    // RJMP -1
    if let Some(circuit) = canvas.running_circuit_mut() {
        for c in circuit.components_mut() {
            if let cs_engine::elements::Kind::Mcu(ref mut m) = c.kind {
                m.device.load_words(&[
                    avr_ldi(16, 0x20),
                    avr_out(0x04, 16),
                    avr_out(0x05, 16),
                    avr_rjmp(-1),
                ]);
            }
        }
    }

    let _ = canvas.take_dirty();
    let change = canvas.tick();
    assert!(
        !change.items,
        "sim visual pin-direction updates must not rebuild the QML items model"
    );
    assert!(
        canvas.take_dirty().items.contains(&mcu_id),
        "MCU must dirty when pin direction changes"
    );
    assert_eq!(canvas.pin_direction(&pb5_id), Some(PinDirection::Out));
    assert_eq!(canvas.pin_voltage(&pb5_id), Some(5.0));

    // Next, program MCU to set DDRB5=0 (input) and PORTB5=1 (pullup enabled):
    // LDI r16, 0x00
    // OUT 0x04, r16 (DDRB = 0)
    // LDI r16, 0x20
    // OUT 0x05, r16 (PORTB = 0x20 => pullup)
    // RJMP -1
    if let Some(circuit) = canvas.running_circuit_mut() {
        for c in circuit.components_mut() {
            if let cs_engine::elements::Kind::Mcu(ref mut m) = c.kind {
                m.device.load_words(&[
                    avr_ldi(16, 0x00),
                    avr_out(0x04, 16),
                    avr_ldi(16, 0x20),
                    avr_out(0x05, 16),
                    avr_rjmp(-1),
                ]);
                m.device.set_pc(0);
            }
        }
    }

    let _ = canvas.take_dirty();
    let change = canvas.tick();
    assert!(!change.items);
    assert!(
        canvas.take_dirty().items.contains(&mcu_id),
        "MCU must dirty when pullup changes"
    );
    assert_eq!(canvas.pin_direction(&pb5_id), Some(PinDirection::In));
    assert_eq!(canvas.is_pin_pullup(&pb5_id), true);
}

#[test]
fn deleting_wire_from_mega328_keeps_the_chip() {
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    assert!(canvas.scene_add_component_spec("Resistor", Point::new(200.0, 0.0)));

    let mcu_id = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .expect("mega328")
        .id
        .clone();
    let pb5_id = format!("{mcu_id}-PORTB5");
    let pb5_pos = canvas.scene().pin_scene(&pb5_id).expect("PORTB5");
    let rpin = canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.id.starts_with("Resistor"))
        .expect("resistor")
        .pins()[0]
        .id
        .clone();
    let rpos = canvas.scene().pin_scene(&rpin).expect("resistor pin");
    canvas
        .scene_mut()
        .connect_pins(&pb5_id, pb5_pos, &rpin, rpos);

    assert_eq!(canvas.scene().wires().len(), 1);
    canvas.scene_mut().select_only_wire(0);
    assert!(canvas.scene().wires()[0].selected);
    assert!(!canvas.scene().item_by_id(&mcu_id).unwrap().selected);

    canvas.delete_selected();
    assert!(
        canvas.scene().item_by_id(&mcu_id).is_some(),
        "mega328 must remain after its attached wire is deleted"
    );
    assert!(canvas.scene().wires().is_empty());

    canvas.undo();
    assert!(
        canvas.scene().item_by_id(&mcu_id).is_some(),
        "ctrl+z after removing a mega328 wire must not drop the chip"
    );
    assert_eq!(canvas.scene().wires().len(), 1);
}

#[test]
fn undo_after_wiring_mega328_keeps_the_chip() {
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    assert!(canvas.add_component_at("mega328,MCU", Point::zero()).items);
    let mcu_id = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .expect("mega328")
        .id
        .clone();
    assert!(
        canvas
            .add_component_at("Resistor", Point::new(200.0, 0.0))
            .items
    );

    let pb5_id = format!("{mcu_id}-PORTB5");
    let pb5_pos = canvas.scene().pin_scene(&pb5_id).expect("PORTB5");
    let rpin = canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.id.starts_with("Resistor"))
        .expect("resistor")
        .pins()[0]
        .id
        .clone();
    let rpos = canvas.scene().pin_scene(&rpin).expect("resistor pin");
    canvas
        .scene_mut()
        .connect_pins(&pb5_id, pb5_pos, &rpin, rpos);
    assert_eq!(canvas.scene().wires().len(), 1);

    // Undo the resistor (and anything after MCU placement). The MCU-only
    // snapshot used to reload via Scene::from_sim1 with an empty catalog,
    // which silently dropped mega328.
    let change = canvas.undo();
    assert!(change.items);
    assert!(
        canvas.scene().item_by_id(&mcu_id).is_some(),
        "ctrl+z must not drop mega328"
    );
    assert!(
        canvas
            .scene()
            .items()
            .iter()
            .all(|it| !it.id.starts_with("Resistor")),
        "resistor should be undone"
    );
}

#[test]
fn test_qemu_stm32_dynamic_pin_modes() {
    let mut qemu = QemuComp::stm32("stm0", 2);
    // Pin 0 (PA0) starts as Input
    assert_eq!(qemu.pins[0].mode, PinMode::Input);
    assert_eq!(qemu.pins[0].has_pullup(), false);

    // Dynamic mode changes on QEMU pin
    qemu.pins[0].set_pin_mode(PinMode::Output);
    assert_eq!(qemu.pins[0].mode, PinMode::Output);

    qemu.pins[0].set_pin_mode(PinMode::OpenCo);
    assert_eq!(qemu.pins[0].mode, PinMode::OpenCo);

    qemu.pins[0].set_pin_mode(PinMode::Input);
    qemu.pins[0].set_pullup(1e5);
    assert_eq!(qemu.pins[0].mode, PinMode::Input);
    assert!(qemu.pins[0].has_pullup());
}

#[test]
fn test_qemu_esp32_dynamic_pin_direction() {
    let mut qemu = QemuComp::esp32("esp0");
    assert_eq!(qemu.pins[0].mode, PinMode::Input);

    qemu.pins[0].set_pin_mode(PinMode::Output);
    assert_eq!(qemu.pins[0].mode, PinMode::Output);

    qemu.pins[0].set_pin_mode(PinMode::Input);
    assert_eq!(qemu.pins[0].mode, PinMode::Input);
}

#[test]
fn test_discrete_components_default_pin_direction() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("FixedVolt", Point::zero()));
    let fv = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::FixedVolt(_)))
        .expect("FixedVolt must be present");
    assert_eq!(fv.default_pin_direction("outnod"), Some(PinDirection::Out));

    assert!(canvas.scene_add_component_spec("Clock", Point::zero()));
    let clk = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Clock(_)))
        .expect("Clock must be present");
    assert_eq!(clk.default_pin_direction("outnod"), Some(PinDirection::Out));

    assert!(canvas.scene_add_component_spec("AndGate", Point::zero()));
    let and_gate = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::AndGate(_)))
        .expect("Gate must be present");
    assert_eq!(
        and_gate.default_pin_direction("in0"),
        Some(PinDirection::In)
    );
    assert_eq!(
        and_gate.default_pin_direction("in1"),
        Some(PinDirection::In)
    );
    assert_eq!(
        and_gate.default_pin_direction("out"),
        Some(PinDirection::Out)
    );

    assert!(canvas.scene_add_component_spec("OpAmp", Point::zero()));
    let opamp = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::OpAmp(_)))
        .expect("OpAmp must be present");
    assert_eq!(
        opamp.default_pin_direction("inputInv"),
        Some(PinDirection::In)
    );
    assert_eq!(
        opamp.default_pin_direction("inputNinv"),
        Some(PinDirection::In)
    );
    assert_eq!(
        opamp.default_pin_direction("output"),
        Some(PinDirection::Out)
    );

    assert!(canvas.scene_add_component_spec("VoltReg", Point::zero()));
    let volt_reg = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::VoltReg(_)))
        .expect("VoltReg must be present");
    assert_eq!(
        volt_reg.default_pin_direction("input"),
        Some(PinDirection::In)
    );
    assert_eq!(
        volt_reg.default_pin_direction("output"),
        Some(PinDirection::Out)
    );
    assert_eq!(
        volt_reg.default_pin_direction("ref"),
        Some(PinDirection::In)
    );

    assert!(canvas.scene_add_component_spec("Voltmeter", Point::zero()));
    let vm = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Voltmeter(_)))
        .expect("Voltmeter must be present");
    assert_eq!(vm.default_pin_direction("lPin"), Some(PinDirection::In));
    assert_eq!(vm.default_pin_direction("rPin"), Some(PinDirection::In));
    assert_eq!(vm.default_pin_direction("outnod"), Some(PinDirection::Out));

    assert!(canvas.scene_add_component_spec("Mux", Point::zero()));
    let mux = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mux(_)))
        .expect("Mux must be present");
    assert_eq!(mux.default_pin_direction("in0"), Some(PinDirection::In));
    assert_eq!(mux.default_pin_direction("addr0"), Some(PinDirection::In));
    assert_eq!(mux.default_pin_direction("enable"), Some(PinDirection::In));
    assert_eq!(mux.default_pin_direction("out"), Some(PinDirection::Out));
    assert_eq!(
        mux.default_pin_direction("out_inv"),
        Some(PinDirection::Out)
    );
}

#[test]
fn test_mcu_esp32_active_indicator_detection_and_switching() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    assert!(canvas.scene_add_component_spec("ESP32,QemuDevice", Point::new(100.0, 0.0)));

    let mcu = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .expect("MCU must be present")
        .clone();
    let esp = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("ESP32 must be present")
        .clone();

    // By default, first programmable device is active
    assert!(canvas.is_item_active(&mcu.id) || canvas.is_item_active(&esp.id));

    // Explicitly activate ESP32
    canvas.set_active_device_id(Some(esp.id.clone()));
    assert!(canvas.is_item_active(&esp.id));
    assert!(!canvas.is_item_active(&mcu.id));

    // Switch active device to MCU
    canvas.set_active_device_id(Some(mcu.id.clone()));
    assert!(canvas.is_item_active(&mcu.id));
    assert!(!canvas.is_item_active(&esp.id));
}

#[test]
fn test_mcu_esp32_active_indicator_visual_svg_rendering() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("ESP32,QemuDevice", Point::zero()));
    let esp = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("ESP32 must be present")
        .clone();

    // When ESP32 is active, SVG rendering includes the yellow active indicator dot (#ffff00)
    canvas.set_active_device_id(Some(esp.id.clone()));
    let pal = cs_engine::canvas::Palette::light();
    let svg_active = cs_engine::canvas::export::svg_string(&canvas, &pal);
    assert!(
        svg_active.contains("#ffff00"),
        "SVG must contain the active indicator yellow dot (#ffff00)"
    );

    // Deactivate ESP32 by targeting a non-existent device
    canvas.set_active_device_id(Some("none".to_string()));
    let svg_inactive = cs_engine::canvas::export::svg_string(&canvas, &pal);
    assert!(
        !svg_inactive.contains("#ffff00"),
        "Inactive device must not draw the yellow active dot"
    );
}

#[test]
fn test_subcircuit_board_active_indicator_detection() {
    let mut canvas = Canvas::empty();
    let board_sim1 = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="atmega328-1" label="atmega328" />
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

    // Board containing active MCU is identified as the active item on canvas
    assert!(canvas.is_item_active("Arduino-1"));

    // Verify SVG export renders active indicator on the board
    let pal = cs_engine::canvas::Palette::light();
    let svg = cs_engine::canvas::export::svg_string(&canvas, &pal);
    assert!(
        svg.contains("#ffff00"),
        "Active board must render the yellow indicator dot"
    );
}
