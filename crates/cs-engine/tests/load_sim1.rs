use cs_engine::{CERO_DOUB, Circuit};

const DIVIDER: &str = include_str!("fixtures/divider.sim1");

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn load_and_solve_divider_sim1() {
    let mut c = Circuit::from_sim1(DIVIDER).unwrap();
    assert!(c.skipped().is_empty());
    c.solve().unwrap();

    approx(c.pin_voltage("Fixed Voltage-1-outnod").unwrap(), 5.0);
    approx(c.pin_voltage("Node-5-0").unwrap(), 2.5);
    approx(c.pin_voltage("Ground-4-Gnd").unwrap(), CERO_DOUB);
    approx(c.resistor_current("Resistor-2").unwrap(), 2.5e-3);
    approx(c.resistor_current("Resistor-3").unwrap(), 2.5e-3);
}

#[test]
fn profile_divider_run_ps() {
    let mut c = Circuit::from_sim1(DIVIDER).unwrap();
    c.solve().unwrap();
    let start = std::time::Instant::now();
    // 33.33ms of simulation time (1 frame at 100% speed)
    c.run_ps(33_333_333_333).unwrap();
    let elapsed = start.elapsed();
    eprintln!("run_ps(33.33ms divider) took: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 50,
        "Divider run_ps should take < 50ms, took {:?}",
        elapsed
    );
}

#[test]
fn profile_rc_circuit_run_ps() {
    // 5V fixed volt -> 1k resistor -> capacitor 1uF -> Ground
    let sim1 = r#"<circuit version="2.0.0">
<item itemtype="Fixed Volt" CircId="Fixed-1" Voltage="5 V" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" />
<item itemtype="Capacitor" CircId="Capacitor-1" Capacitance="1 µF" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="c1" startpinid="Fixed-1-outnod" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="Resistor-1-rPin" endpinid="Capacitor-1-lPin" />
<item itemtype="Connector" uid="c3" startpinid="Capacitor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>"#;
    let mut c = Circuit::from_sim1(sim1).unwrap();
    c.solve().unwrap();
    assert!(c.has_reactive());
    let start = std::time::Instant::now();
    // 33.33ms of simulation time (33,333 analog clock steps)
    c.run_ps(33_333_333_333).unwrap();
    let elapsed = start.elapsed();
    eprintln!(
        "run_ps(33.33ms RC circuit - 33,333 steps) took: {:?}",
        elapsed
    );
    assert!(
        elapsed.as_millis() < 250,
        "RC circuit 33,333 steps should take < 250ms (debug) / < 20ms (release), took {:?}",
        elapsed
    );
}

#[test]
fn load_from_path() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/divider.sim1");
    let mut c = Circuit::load_sim1_path(path).unwrap();
    c.solve().unwrap();
    approx(c.pin_voltage("Node-5-0").unwrap(), 2.5);
}

#[test]
fn load_subcircuit_divider_from_path() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/packages/parent.sim1");
    let mut c = Circuit::load_sim1_path(path).unwrap();
    assert!(
        !c.skipped().iter().any(|s| s.starts_with("Subcircuit")),
        "skipped {:?}",
        c.skipped()
    );
    c.solve().unwrap();
    approx(c.pin_voltage("vdiv-1-in").unwrap(), 5.0);
    approx(c.pin_voltage("vdiv-1-out").unwrap(), 2.5);
    approx(c.pin_voltage("vdiv-1-gnd").unwrap(), CERO_DOUB);
}

#[test]
fn batch_folder_and_gate() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/batch");
    let report = cs_engine::headless::run_batch_folder(&folder).unwrap();
    assert_eq!(report.tested, 2);
    assert_eq!(report.failed.len(), 1);
    assert!(
        report.failed[0].contains("and_fail.sim1"),
        "failed {:?}",
        report.failed
    );
}

#[test]
fn runcirc_divider_steps() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/divider.sim1");
    let mut n = 0u32;
    cs_engine::headless::run_circ(&path, |_| {
        n += 1;
        n < 3
    })
    .unwrap();
    assert_eq!(n, 3);
}

const PIC14_MCU: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTA" addr="0x05" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
  </regblock>
  <regblock start="0x80" end="0x8F">
    <mapped addr="0x80" mapto="0x00"/>
    <mapped addr="0x82" mapto="0x02"/>
    <mapped addr="0x83" mapto="0x03"/>
    <mapped addr="0x84" mapto="0x04"/>
    <register name="OPTION" addr="0x81" reset="11111111"/>
    <register name="TRISA" addr="0x85" reset="11111111"/>
    <mapped addr="0x8A" mapto="0x0A"/>
  </regblock>
  <datablock start="0x0C" end="0x4F"/>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
</mcu>
"#;

#[test]
fn load_mcu_sim1_from_path() {
    let dir = std::env::temp_dir().join(format!("cs-engine-mcu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pic14test")).unwrap();
    std::fs::write(dir.join("pic14test").join("pic14test.mcu"), PIC14_MCU).unwrap();
    // Intel HEX for PIC14 words 0x1683,0x0185,0x1283,0x3001,0x0085,0x2805.
    std::fs::write(
        dir.join("porta.hex"),
        ":0C0000008316850183120130850005285D\n:00000001FF\n",
    )
    .unwrap();
    let sim1 = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="pic14test-1" Frequency="4 MHz" Program="porta.hex" Auto_Load="true" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="10 kΩ" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="c1" startpinid="pic14test-1-PORTA0" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>
"#;
    let path = dir.join("porta.sim1");
    std::fs::write(&path, sim1).unwrap();
    let mut c = Circuit::load_sim1_path(&path).unwrap();
    assert!(
        !c.skipped().iter().any(|s| s.starts_with("MCU")),
        "skipped {:?}",
        c.skipped()
    );
    c.solve().unwrap();
    c.run_ps(10_000_000).unwrap();
    let v = c.pin_voltage("pic14test-1-PORTA0").unwrap();
    assert!(v > 2.5, "PORTA0 should be driven high, got {v}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_esp32_display_sim2_and_simulate_hd44780() {
    let sim2_content = r#"<circuit version="1.0.0" rev="260826" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="1" anicurr="1" ansi="0" width="1800" height="1200" >
<item itemtype="QemuDevice" CircId="Esp32-2" mainComp="false" Show_id="true" Show_Val="false" Pos="-120,12" rotation="0" hflip="1" vflip="1" label="Esp32-2" idLabPos="-2.54246,-31.3682" labelrot="0" valLabPos="-2,124" valLabRot="0" Program="Esp32_Display/Esp32_Display.bin" Active="true" />
<item itemtype="Push" CircId="Push-3" mainComp="false" Show_id="false" Show_Val="false" Pos="-156,92" rotation="0" hflip="1" vflip="1" label="Push-3" idLabPos="-2,-14" labelrot="0" valLabPos="-2,4" valLabRot="0" Norm_Close="false" Poles="1" ShowButton="false" />
<item itemtype="Ground" CircId="Ground-4" mainComp="false" Show_id="false" Show_Val="false" Pos="-188,124" rotation="0" hflip="1" vflip="1" label="Ground-4" idLabPos="-16,8" labelrot="0" valLabPos="-2,4" valLabRot="0" />
<item itemtype="Hd44780" CircId="Hd44780-5" mainComp="false" Show_id="true" Show_Val="false" Pos="-64,-52" rotation="0" hflip="1" vflip="1" label="Hd44780-5" idLabPos="70,-82" labelrot="0" valLabPos="-2,4" valLabRot="0" Rows="2" Cols="16" />
<item itemtype="Ground" CircId="Ground-6" mainComp="false" Show_id="false" Show_Val="false" Pos="-140,-4" rotation="0" hflip="1" vflip="1" label="Ground-6" idLabPos="-16,8" labelrot="0" valLabPos="-2,4" valLabRot="0" />
<item itemtype="Connector" uid="Connector-5" startpinid="Push-3-switch0pinN" endpinid="Esp32-2-Rst" pointList="-140,92,-128,92" />
<item itemtype="Connector" uid="Connector-6" startpinid="Ground-4-Gnd" endpinid="Push-3-pinP0" pointList="-188,108,-188,92,-172,92" />
<item itemtype="Connector" uid="Connector-18" startpinid="Hd44780-5-PinRS" endpinid="Esp32-2-G19" pointList="-48,-44,-48,-16,-24,-16,-24,4" />
<item itemtype="Connector" uid="Connector-19" startpinid="Hd44780-5-PinEn" endpinid="Esp32-2-G18" pointList="-32,-44,-32,-20,16,-20,16,36,8,36" />
<item itemtype="Connector" uid="Connector-20" startpinid="Ground-6-Gnd" endpinid="Hd44780-5-PinRW" pointList="-140,-20,-140,-28,-40,-28,-40,-44" />
<item itemtype="Connector" uid="Connector-21" startpinid="Hd44780-5-dataPin4" endpinid="Esp32-2-G05" pointList="8,-44,8,-24,20,-24,20,44,8,44" />
<item itemtype="Connector" uid="Connector-22" startpinid="Hd44780-5-dataPin5" endpinid="Esp32-2-G17" pointList="16,-44,16,-28,24,-28,24,100,8,100" />
<item itemtype="Connector" uid="Connector-23" startpinid="Hd44780-5-dataPin6" endpinid="Esp32-2-G16" pointList="24,-44,24,-32,28,-32,28,116,8,116" />
<item itemtype="Connector" uid="Connector-24" startpinid="Hd44780-5-dataPin7" endpinid="Esp32-2-G04" pointList="32,-44,32,144,-16,144,-16,140" />
</circuit>"#;

    let scene = cs_engine::canvas::Scene::from_sim1(sim2_content).unwrap();
    assert_eq!(scene.items().len(), 5);
    assert_eq!(scene.wires().len(), 9);

    // Verify Esp32-2 is parsed with package and pins
    let esp = scene.items().iter().find(|i| i.id == "Esp32-2").unwrap();
    assert_eq!(esp.kind.type_name(), "QemuDevice");
    assert_eq!(esp.pins().len(), 48);

    // Verify Hd44780-5 is parsed with 11 pins and 2x16 dimension
    let lcd = scene.items().iter().find(|i| i.id == "Hd44780-5").unwrap();
    assert_eq!(lcd.kind.type_name(), "Hd44780");
    assert_eq!(lcd.rows(), 2);
    assert_eq!(lcd.cols(), 16);
    assert_eq!(lcd.pins().len(), 11);

    // Verify Hd44780 protocol digital state machine
    let mut hd = cs_engine::digital::Hd44780State::new("Hd44780-5", 2, 16);
    // Function set: 8-bit, 2-line
    hd.process_command(0x38);
    // Display control: Display ON, Cursor ON
    hd.process_command(0x0E);
    // Clear display
    hd.process_command(0x01);

    // Write "Antigravity"
    for b in b"Antigravity" {
        hd.write_data(*b);
    }
    let lines = hd.lines();
    assert_eq!(lines.len(), 2);
    assert!(
        lines[0].starts_with("Antigravity"),
        "Got line 0: {:?}",
        lines[0]
    );
}

#[test]
fn test_mega_display_sim2_live_run() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mega_display.sim2");
    if !path.exists() {
        return;
    }
    let mut c = Circuit::load_sim1_path(&path).unwrap();
    c.solve().unwrap();

    let ssd_idx = c
        .components()
        .iter()
        .position(|comp| matches!(comp.kind, cs_engine::elements::Kind::Ssd1306(_)))
        .unwrap();

    // Run until display.begin and graphics are rendered (up to 20 frames)
    let mut frames_with_pixels = 0;
    for _f in 1..=20 {
        c.run_ps(33_333_333_333).unwrap();
        if let cs_engine::elements::Kind::Ssd1306(s) = &c.components()[ssd_idx].kind {
            let non_zero_ddram = s.ddram.iter().any(|row| row.iter().any(|&b| b != 0));
            if non_zero_ddram && s.disp_on {
                frames_with_pixels += 1;
                break;
            }
        }
    }

    let ssd = match &c.components()[ssd_idx].kind {
        cs_engine::elements::Kind::Ssd1306(s) => s,
        _ => panic!(),
    };

    assert_eq!(ssd.width, 128);
    assert_eq!(ssd.height, 64);
    assert_eq!(ssd.control_code, 60);
    assert!(ssd.disp_on, "SSD1306 display should be turned on");
    assert!(
        frames_with_pixels > 0,
        "SSD1306 OLED should have rendered graphics from ATmega328 firmware"
    );
}

#[test]
fn test_mega_display_canvas_gui_ticks() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mega_display.sim2");
    if !path.exists() {
        return;
    }
    let src = std::fs::read_to_string(&path).unwrap();
    let mut canvas = cs_engine::canvas::Canvas::new();
    canvas
        .load_sim1(&src, Some(path.to_str().unwrap().to_string()))
        .unwrap();

    canvas.power_on();

    for _f in 1..=2 {
        canvas.tick();
    }
    assert!(canvas.sim_running());
    assert!(canvas.sim_error().is_none());
}

#[test]
fn test_programmable_devices_collection() {
    let sim1 = r#"<circuit version="1.0.0">
<item itemtype="Mcu" CircId="atmega328-1" label="Main MCU" device="atmega328p" />
<item itemtype="QemuDevice" CircId="Esp32-1" label="WiFi Co-processor" device="Esp32" />
<item itemtype="Subcircuit" CircId="Uno-1" label="Arduino Uno" device="uno_board" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" />
</circuit>"#;

    let uno_src = r#"<circuit version="1.0.0">
<item itemtype="Package" CircId="Package-1" label="Uno" Pins="Pin; id=p1; label=P1; xpos=0; ypos=0;" />
<item itemtype="Mcu" CircId="mcu-uno" label="atmega328" device="atmega328p" />
</circuit>"#;

    let mcu_xml = r#"<mcu name="atmega328" core="AVR">
<databank length="2048"/>
<progword length="32768"/>
<ioport name="PORTB" dir="0x24" pin="0x23" port="0x25">
  <pin name="PB0" num="0" mask="0x01"/>
</ioport>
</mcu>"#;

    let mut search =
        cs_engine::subcircuit::SubcSearch::from_circuit_path(None).with_standard_catalog();
    search
        .memory
        .insert("uno_board".to_string(), uno_src.to_string());
    search
        .memory_mcu
        .insert("atmega328".to_string(), mcu_xml.to_string());
    search
        .memory_mcu
        .insert("atmega328p".to_string(), mcu_xml.to_string());
    let scene = cs_engine::canvas::scene::Scene::from_sim1_with(sim1, &search).unwrap();
    let devices = scene.collect_programmable_devices(&search, None);
    eprintln!("Collected devices: {:#?}", devices);

    assert_eq!(devices.len(), 3);
    // Sorted alphabetically by label:
    // 1. "Arduino Uno / atmega328" (Board-mounted MCU)
    // 2. "Main MCU" (Top-level MCU)
    // 3. "WiFi Co-processor" (QEMU device)
    assert_eq!(devices[0].label, "Arduino Uno / atmega328");
    assert_eq!(devices[0].kind, "atmega328p");
    assert_eq!(
        devices[0].display_text,
        "Arduino Uno / atmega328  (atmega328p)"
    );
    assert!(devices[0].is_active); // Default first device is active

    assert_eq!(devices[1].label, "Main MCU");
    assert_eq!(devices[1].display_text, "Main MCU  (atmega328)");
    assert!(!devices[1].is_active);

    assert_eq!(devices[2].label, "WiFi Co-processor");
    assert_eq!(devices[2].display_text, "WiFi Co-processor  (Esp32)");
    assert!(!devices[2].is_active);

    // Test targeting specific active device
    let devices_with_active = scene.collect_programmable_devices(&search, Some("atmega328-1"));
    assert!(!devices_with_active[0].is_active);
    assert!(devices_with_active[1].is_active);
}
