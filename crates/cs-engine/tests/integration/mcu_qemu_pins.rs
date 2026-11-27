use std::path::PathBuf;

use cs_engine::canvas::{Canvas, Item, Palette, PinDirection, Point, export};
use cs_engine::catalog::Catalog;
use cs_engine::components::Part;
use cs_engine::digital::PinMode;
use cs_engine::package::{Package, PkgPin};
use cs_engine::qemu::QemuComp;
use cs_engine::subcircuit::SubcSearch;

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

    // Explicit nested device ID resolves directly to its top-level board item
    canvas.set_active_device_id(Some("Arduino-1/atmega328-1".to_string()));
    assert_eq!(
        canvas.active_top_level_item_id().as_deref(),
        Some("Arduino-1")
    );
    assert!(canvas.is_item_active("Arduino-1"));

    // Non-existent nested device ID under an existing board is inactive
    canvas.set_active_device_id(Some("Arduino-1/missing-1".to_string()));
    assert_eq!(canvas.active_top_level_item_id(), None);
    assert!(!canvas.is_item_active("Arduino-1"));
}

#[test]
fn test_many_non_mcu_subcircuits_render_and_resolve_active_fast() {
    let mut canvas = Canvas::empty();
    let logic_sim1 = r#"<circuit version="2.0.0" >
<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 k" />
<item itemtype="AndGate" CircId="AndGate-1" />
</circuit>"#;
    let board_sim1 = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="atmega328-1" label="atmega328" />
</circuit>"#;

    for i in 0..80 {
        let item = Item::subcircuit(
            format!("74HC00-{i}"),
            (i % 10) as f64 * 80.0,
            (i / 10) as f64 * 80.0,
            "74HC00",
            Package::default(),
            logic_sim1,
            None,
            false,
        );
        canvas.scene_mut().add_saved_item(item);
    }
    let board_item = Item::subcircuit(
        "Arduino-1",
        0.0,
        700.0,
        "Arduino_Uno",
        Package::default(),
        board_sim1,
        None,
        false,
    );
    canvas.scene_mut().add_saved_item(board_item);

    assert_eq!(
        canvas.active_top_level_item_id().as_deref(),
        Some("Arduino-1")
    );
    let pal = Palette::light();
    let svg = export::svg_string(&canvas, &pal);
    assert!(svg.contains("#ffff00"));
}

#[test]
fn test_mcu_info_tracks_active_device_and_board_mcus() {
    let mut canvas = Canvas::empty();
    assert_eq!(canvas.mcu_info(), (String::new(), String::new(), false));

    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    assert!(canvas.scene_add_component_spec("ESP32,QemuDevice", Point::new(200.0, 0.0)));
    let board_sim1 = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="atmega328-1" label="atmega328" />
</circuit>"#;
    let board_item = Item::subcircuit(
        "Arduino-1",
        400.0,
        0.0,
        "Arduino_Uno",
        Package::default(),
        board_sim1,
        None,
        false,
    );
    canvas.scene_mut().add_saved_item(board_item);

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

    // Activating the ESP32 updates mcu_info to ESP32
    canvas.set_active_device_id(Some(esp.id.clone()));
    let (dev, name, has) = canvas.mcu_info();
    assert!(has);
    assert!(
        dev.to_lowercase().contains("esp32"),
        "expected ESP32 device, got {dev:?}"
    );
    assert_eq!(name, esp.label);

    // Switching to the standalone mega328 updates mcu_info to mega328
    canvas.set_active_device_id(Some(mcu.id.clone()));
    let (dev, name, has) = canvas.mcu_info();
    assert!(has);
    assert!(
        dev.to_lowercase().contains("mega328"),
        "expected mega328 device, got {dev:?}"
    );
    assert_eq!(name, mcu.label);

    // Switching to the board-mounted MCU updates mcu_info to the nested MCU
    canvas.set_active_device_id(Some("Arduino-1/atmega328-1".to_string()));
    let (dev, name, has) = canvas.mcu_info();
    assert!(has);
    assert_eq!(dev, "atmega328");
    assert_eq!(name, "Arduino-1 / atmega328");
}

#[test]
fn mega328_hides_nul_pins_and_shows_accurate_hover_tooltips() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("mega328,MCU", Point::zero()));
    let mcu = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .expect("mega328 must be present")
        .clone();

    // DIP-28 must expose exactly 28 physical pins (hiding type="nul" PV2/PV3 placeholders).
    let pins = mcu.pins();
    assert_eq!(pins.len(), 28, "DIP-28 mega328 should have 28 pins");
    assert!(!pins.iter().any(|p| p.id.ends_with("-PORTV2")));
    assert!(!pins.iter().any(|p| p.id.ends_with("-PORTV3")));
    assert_eq!(
        mcu.local_total_bounding_rect().top(),
        0.0,
        "hidden top nul pins must not expand bounding rect above y=0"
    );

    let tip_for = |suffix: &str| {
        let pin = pins
            .iter()
            .find(|p| p.id.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing pin {suffix}"));
        let scene_pt = mcu.pin_scene_pos(pin);
        canvas
            .hover_tooltip(scene_pt)
            .unwrap_or_else(|| panic!("missing hover tooltip for {suffix}"))
    };

    let d0_tip = tip_for("-PORTD0");
    assert!(
        d0_tip.contains("Port D pin 0 (PD0)"),
        "expected Port D pin 0 tooltip, got: {d0_tip}"
    );
    assert!(
        !d0_tip.contains("Data line"),
        "D0 on AVR must not be mislabeled as a Data line: {d0_tip}"
    );

    let b0_tip = tip_for("-PORTB0");
    assert!(
        b0_tip.contains("Port B pin 0 (PB0)"),
        "expected Port B pin 0 tooltip, got: {b0_tip}"
    );

    let c6_tip = tip_for("-PORTC6");
    assert!(
        c6_tip.contains("Port C pin 6 (PC6 / RESET)"),
        "expected PC6 / RESET tooltip, got: {c6_tip}"
    );

    let avcc_tip = tip_for("-PORTV0");
    assert!(
        avcc_tip.contains("Analog positive supply voltage (AVCC)"),
        "expected AVCC tooltip for A+, got: {avcc_tip}"
    );

    let aref_tip = tip_for("-PORTV1");
    assert!(
        aref_tip.contains("Analog reference voltage (AREF)"),
        "expected AREF tooltip for Ar, got: {aref_tip}"
    );

    let vcc_tip = tip_for("-VCC");
    assert!(
        vcc_tip.contains("Positive supply voltage") && vcc_tip.contains("Unused"),
        "expected unused VCC tooltip, got: {vcc_tip}"
    );

    let gnd_tip = tip_for("-GND1");
    assert!(
        gnd_tip.contains("Ground / 0V reference") && gnd_tip.contains("Unused"),
        "expected unused GND tooltip, got: {gnd_tip}"
    );

    // Logic Symbol view also hides PV2/PV3 nul placeholders (25 pins instead of 27).
    canvas.set_prop_text_for(&mcu.id, "LogicSymbol".into(), "true".into());
    let mcu_ls = canvas.scene().item_by_id(&mcu.id).unwrap();
    let ls_pins = mcu_ls.pins();
    assert_eq!(ls_pins.len(), 25, "mega328 LS should hide 2 nul pins");
    assert!(!ls_pins.iter().any(|p| p.id.ends_with("-PORTV2")));
    assert!(!ls_pins.iter().any(|p| p.id.ends_with("-PORTV3")));
}

#[test]
fn m328_tqfp_keeps_adc6_adc7_and_subpackage_keeps_nul_editable() {
    let mut canvas = Canvas::empty();
    assert!(canvas.scene_add_component_spec("m328 TQFP,MCU", Point::zero()));
    let tqfp = canvas
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::Mcu(_)))
        .expect("m328 TQFP must be present")
        .clone();
    let pins = tqfp.pins();
    assert_eq!(pins.len(), 32, "TQFP-32 must expose all 32 physical pins");
    let adc6 = pins
        .iter()
        .find(|p| p.id.ends_with("-PORTV2"))
        .expect("TQFP-32 must have ADC6 (PORTV2)");
    let adc6_tip = canvas
        .hover_tooltip(tqfp.pin_scene_pos(adc6))
        .expect("ADC6 tooltip");
    assert!(
        adc6_tip.contains("Analog input channel 6 (ADC6)"),
        "expected ADC6 tooltip, got: {adc6_tip}"
    );

    // SubPackage (package designer) must still expose type="nul" pins for editing.
    let mut pkg = Package::default();
    pkg.pins.push(PkgPin::new("PV2", "", "nul", 8, -8, 90));
    let sp = Item::subpackage("SubPackage-1", 0.0, 0.0, pkg);
    assert_eq!(sp.pins().len(), 1);
}

fn fixture_mcu_search() -> SubcSearch {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/components");
    let mut cat = Catalog::new();
    for dir in ["AVR", "MCS51", "MCS65", "PIC", "Z80"] {
        cat.load_dir(&root.join(dir));
    }
    SubcSearch::default().with_catalog(cat)
}

#[test]
fn other_mcus_packages_active_high_reset_and_qemu_tooltips() {
    let search = fixture_mcu_search();

    // 1. mega32 Logic Symbol: PV0 must be AVcc and PV1 must be Aref.
    let mut canvas = Canvas::empty();
    let m32_id = canvas
        .scene_mut()
        .add_mcu("mega32", 0.0, 0.0, &search)
        .expect("mega32");
    canvas.set_prop_text_for(&m32_id, "LogicSymbol".into(), "true".into());
    let m32_ls = canvas.scene().item_by_id(&m32_id).unwrap().clone();
    let ls_pins = m32_ls.pins();
    let avcc = ls_pins
        .iter()
        .find(|p| p.id.ends_with("-PORTV0"))
        .expect("mega32 PORTV0");
    let aref = ls_pins
        .iter()
        .find(|p| p.id.ends_with("-PORTV1"))
        .expect("mega32 PORTV1");
    assert_eq!(avcc.label, "AVcc");
    assert_eq!(aref.label, "Aref");
    assert!(
        canvas
            .hover_tooltip(m32_ls.pin_scene_pos(avcc))
            .unwrap()
            .contains("AVCC")
    );
    assert!(
        canvas
            .hover_tooltip(m32_ls.pin_scene_pos(aref))
            .unwrap()
            .contains("AREF")
    );

    // 2. mega1281 (64-pin TQFP) has no duplicate VCC1 pin.
    let mut c1281 = Canvas::empty();
    let m1281_id = c1281
        .scene_mut()
        .add_mcu("mega1281", 0.0, 0.0, &search)
        .expect("mega1281");
    let m1281 = c1281.scene().item_by_id(&m1281_id).unwrap();
    assert_eq!(m1281.pins().len(), 64);

    // 3. mega2560 (m640.package) PJ7 label has no leading whitespace.
    let mut c2560 = Canvas::empty();
    let m2560_id = c2560
        .scene_mut()
        .add_mcu("mega2560", 0.0, 0.0, &search)
        .expect("mega2560");
    let m2560 = c2560.scene().item_by_id(&m2560_id).unwrap();
    let pj7 = m2560
        .pins()
        .into_iter()
        .find(|p| p.id.ends_with("-PORTJ7"))
        .expect("PJ7");
    assert_eq!(pj7.label, "J7");

    // 4. 8051 active-high reset pin (!rst) is attached and described accurately.
    let mut c8051 = Canvas::empty();
    let m8051_id = c8051
        .scene_mut()
        .add_mcu("8051", 0.0, 0.0, &search)
        .expect("8051");
    let m8051 = c8051.scene().item_by_id(&m8051_id).unwrap().clone();
    if let Part::Mcu(ref mcu) = m8051.kind {
        assert!(mcu.mcu.reset_pin.is_some(), "8051 RST pin must be attached");
        assert!(mcu.mcu.reset_active_high, "8051 RST must be active-high");
        assert!(
            !mcu.mcu.reset_pin.as_ref().unwrap().has_pullup(),
            "active-high RST must not have an internal pullup"
        );
    }
    assert_eq!(m8051.default_pin_direction("RST"), None);
    let rst_pin = m8051
        .pins()
        .into_iter()
        .find(|p| p.id.ends_with("-RST"))
        .expect("8051 RST pin");
    let rst_tip = c8051
        .hover_tooltip(m8051.pin_scene_pos(&rst_pin))
        .expect("8051 RST tooltip");
    assert!(
        rst_tip.contains("Reset input (active high)"),
        "expected active-high reset tooltip, got: {rst_tip}"
    );

    // 5. p16F84 PX0 (Osc) has clean pin_type (not typo "n").
    let mut cpic = Canvas::empty();
    let pic_id = cpic
        .scene_mut()
        .add_mcu("p16F84", 0.0, 0.0, &search)
        .expect("p16F84");
    let pic = cpic.scene().item_by_id(&pic_id).unwrap();
    let px0 = pic
        .package()
        .unwrap()
        .find_pin("PORTX0")
        .expect("p16F84 PORTX0");
    assert_eq!(px0.pin_type, "");

    // 6. QEMU devices (ESP32, STM32F103) show clear GPIO / crystal / reset tooltips.
    let mut cesp = Canvas::empty();
    assert!(cesp.scene_add_component_spec("ESP32,QemuDevice", Point::zero()));
    let esp = cesp
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("ESP32")
        .clone();
    let esp_pins = esp.pins();
    let g05 = esp_pins.iter().find(|p| p.id.ends_with("-G05")).unwrap();
    let xtali = esp_pins.iter().find(|p| p.id.ends_with("-XTALi")).unwrap();
    let esp_rst = esp_pins.iter().find(|p| p.id.ends_with("-Rst")).unwrap();
    assert!(
        cesp.hover_tooltip(esp.pin_scene_pos(g05))
            .unwrap()
            .contains("GPIO 5 (G05)")
    );
    assert!(
        cesp.hover_tooltip(esp.pin_scene_pos(xtali))
            .unwrap()
            .contains("Crystal oscillator input (XTAL_IN)")
    );
    assert!(
        cesp.hover_tooltip(esp.pin_scene_pos(esp_rst))
            .unwrap()
            .contains("Reset input (active low)")
    );

    let mut cstm = Canvas::empty();
    assert!(cstm.scene_add_component_spec("STM32F103,QemuDevice", Point::zero()));
    let stm = cstm
        .scene()
        .items()
        .iter()
        .find(|it| matches!(it.kind, Part::QemuDevice(_)))
        .expect("STM32F103")
        .clone();
    let pa0 = stm
        .pins()
        .into_iter()
        .find(|p| p.id.ends_with("-PA0"))
        .unwrap();
    assert!(
        cstm.hover_tooltip(stm.pin_scene_pos(&pa0))
            .unwrap()
            .contains("Port A pin 0 (PA0)")
    );
}
