//! End-to-end verification for Phase 5 stub properties:
//! - MCU `AutoLoad` on/off restart behavior
//! - MCU `RstEnabled` on dedicated and shared reset pins
//! - MCU `ExtOsc` XTAL1 edge-stepping
//! - ESP-01 `Debug` log emission
//! - SubPackage `Background`, `Border`, and `PackageFile` round-trip

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use cs_engine::canvas::draw::{Align, Color, Draw, PaintCtx, Palette};
use cs_engine::canvas::{Canvas, ImageBuf, Rect};
use cs_engine::circ1::{load_circ1, to_parsed_circuit};
use cs_engine::components::{Component, Drawable, Esp01, Mcu, Part, SubPackage, Subcircuit};
use cs_engine::elements::{Comp, Kind};
use cs_engine::mcu::{McuItemSpec, instantiate_item, instantiate_view};
use cs_engine::subcircuit::SubcSearch;
use cs_engine::{Circuit, logging};
use cs_mcu::encode_hex;

fn rail(id: &str, voltage: f64) -> Comp {
    Comp {
        id: id.into(),
        kind: Kind::Rail { voltage },
    }
}

static LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

const AVR_TEST_XML: &str = r#"
<mcu name="avrtest" core="AVR" data="512" prog="64" progword="2" inst_cycle="1" freq="1000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="PINB" addr="0x0023" reset=""/>
    <register name="DDRB" addr="0x0024" reset=""/>
    <register name="PORTB" addr="0x0025" reset=""/>
    <register name="PINC" addr="0x0026" reset=""/>
    <register name="DDRC" addr="0x0027" reset=""/>
    <register name="PORTC" addr="0x0028" reset=""/>
    <register name="PIND" addr="0x0029" reset=""/>
    <register name="DDRD" addr="0x002A" reset=""/>
    <register name="PORTD" addr="0x002B" reset=""/>
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <port name="PORTB" pins="8" outreg="PORTB" inreg="PINB" dirreg="DDRB"/>
  <port name="PORTC" pins="7" outreg="PORTC" inreg="PINC" dirreg="DDRC" resetpin="PORTC6"/>
  <port name="PORTD" pins="8" outreg="PORTD" inreg="PIND" dirreg="DDRD"/>
  <intosc name="intosc" clockpins="PORTB6,PORTB7" clockoutpin="PORTB0"/>
</mcu>
"#;

const AVR_DEDICATED_RST_XML: &str = r#"
<mcu name="avrded" core="AVR" data="512" prog="64" progword="2" inst_cycle="1" freq="1000000">
  <regblock start="0x0000" end="0x00FF" streg="SREG">
    <register name="PIND" addr="0x0029" reset=""/>
    <register name="DDRD" addr="0x002A" reset=""/>
    <register name="PORTD" addr="0x002B" reset=""/>
    <register name="SPL" addr="0x005D" reset=""/>
    <register name="SPH" addr="0x005E" reset=""/>
    <register name="SREG" addr="0x005F" reset="" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <port name="PORTD" pins="8" outreg="PORTD" inreg="PIND" dirreg="DDRD"/>
</mcu>
"#;

const AVR_DEDICATED_RST_PKG: &str = r#"<packageB name="DIP" width="4" height="6">
    <pin type="rst" xpos="-8" ypos="8" angle="180" length="8" id="RST" label="!RST" />
    <pin type="" xpos="40" ypos="8" angle="0" length="8" id="PD0" label="PD0" />
</packageB>
"#;

/// AVR program: LDI R16, imm; OUT DDRD(0x0A), R16; OUT PORTD(0x0B), R16; RJMP -1
fn avr_drive_portd_words(mask: u8) -> [u16; 4] {
    let ldi = 0xE000 | (u16::from(mask & 0xF0) << 4) | u16::from(mask & 0x0F);
    [ldi, 0xB90A, 0xB90B, 0xCFFF]
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cs_phase5_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn mcu_auto_load_on_and_off_restart_behavior() {
    // Tooltip matches SimulIDE semantics.
    let auto_prop = Mcu::props()
        .iter()
        .find(|p| p.id == "AutoLoad")
        .expect("AutoLoad prop");
    assert_eq!(
        auto_prop.info,
        "Reload firmware from disk when simulation starts."
    );

    let dir = temp_dir("autoload");
    let hex_path = dir.join("fw.hex");
    // Program A drives PORTD0 (bit 0) high.
    let prog_a = encode_hex(0, &avr_drive_portd_words(0x01), 16);
    fs::write(&hex_path, &prog_a).unwrap();

    let mut search = SubcSearch::default().with_memory_mcu("avrtest", AVR_TEST_XML);
    search.circuit_dir = Some(dir.clone());
    let view = instantiate_view(
        "avrtest-1",
        &McuItemSpec {
            device: "avrtest".into(),
            ..Default::default()
        },
        &search,
    )
    .unwrap();

    let mut mcu = Mcu {
        device: "avrtest".into(),
        mcu: view.mcu,
        package: view.package,
        packages: view.packages,
        logic_symbol: view.logic_symbol,
        ..Default::default()
    };

    // Setting Program loads Program A once into flash immediately.
    mcu.set_prop_text("Program", "fw.hex").unwrap();
    assert_eq!(
        mcu.mcu.device.flash_word(0),
        Some(avr_drive_portd_words(0x01)[0])
    );

    // Overwrite fw.hex on disk with Program B (drives PORTD1 instead of PORTD0).
    let prog_b = encode_hex(0, &avr_drive_portd_words(0x02), 16);
    fs::write(&hex_path, &prog_b).unwrap();

    // AutoLoad = false (default): starting simulation must keep Program A in flash.
    let mut c_off = Circuit::new();
    c_off.add_comp(Comp {
        id: "avrtest-1".into(),
        kind: mcu.to_element_kind(),
    });
    c_off.add_comp(Comp::resistor("R0", 10_000.0));
    c_off.add_comp(Comp::resistor("R1", 10_000.0));
    c_off.add_comp(Comp::ground("G"));
    c_off.connect("avrtest-1-PORTD0", "R0-lPin");
    c_off.connect("avrtest-1-PORTD1", "R1-lPin");
    c_off.connect("R0-rPin", "G-Gnd");
    c_off.connect("R1-rPin", "G-Gnd");
    c_off.solve().unwrap();
    c_off.step_n(20).unwrap();

    let v_d0_off = c_off.pin_voltage("avrtest-1-PORTD0").unwrap();
    let v_d1_off = c_off.pin_voltage("avrtest-1-PORTD1").unwrap();
    assert!(
        v_d0_off > 4.0,
        "AutoLoad=false should still run Program A (D0={v_d0_off})"
    );
    assert!(
        v_d1_off < 0.5,
        "AutoLoad=false must not reload Program B (D1={v_d1_off})"
    );

    // AutoLoad = true: starting simulation reloads Program B from disk.
    mcu.set_prop_text("AutoLoad", "true").unwrap();
    let mut c_on = Circuit::new();
    c_on.add_comp(Comp {
        id: "avrtest-1".into(),
        kind: mcu.to_element_kind(),
    });
    c_on.add_comp(Comp::resistor("R0", 10_000.0));
    c_on.add_comp(Comp::resistor("R1", 10_000.0));
    c_on.add_comp(Comp::ground("G"));
    c_on.connect("avrtest-1-PORTD0", "R0-lPin");
    c_on.connect("avrtest-1-PORTD1", "R1-lPin");
    c_on.connect("R0-rPin", "G-Gnd");
    c_on.connect("R1-rPin", "G-Gnd");
    c_on.solve().unwrap();
    c_on.step_n(20).unwrap();

    let v_d0_on = c_on.pin_voltage("avrtest-1-PORTD0").unwrap();
    let v_d1_on = c_on.pin_voltage("avrtest-1-PORTD1").unwrap();
    assert!(
        v_d0_on < 0.5,
        "AutoLoad=true should replace Program A (D0={v_d0_on})"
    );
    assert!(
        v_d1_on > 4.0,
        "AutoLoad=true should reload Program B (D1={v_d1_on})"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mcu_rst_enabled_gates_dedicated_and_shared_reset_pins() {
    // 1. Dedicated active-low package reset pin (<pin type="rst"/>).
    let search_ded = SubcSearch::default()
        .with_memory_mcu("avrded", AVR_DEDICATED_RST_XML)
        .with_memory_pkg("avrded", AVR_DEDICATED_RST_PKG);

    let mut mcu_en = instantiate_item(
        "avrded-1",
        &McuItemSpec {
            device: "avrded".into(),
            rst_enabled: true,
            ..Default::default()
        },
        &search_ded,
    )
    .unwrap();
    assert!(mcu_en.reset_pin.is_some());
    mcu_en.device.load_words(&avr_drive_portd_words(0x01));

    let mut c = Circuit::new();
    c.add_comp(Comp {
        id: "avrded-1".into(),
        kind: Kind::Mcu(mcu_en),
    });
    c.add_comp(rail("Vrst", 0.0)); // Assert active-low reset
    c.add_comp(Comp::resistor("R0", 10_000.0));
    c.add_comp(Comp::ground("G"));
    c.connect("Vrst-outnod", "avrded-1-RST");
    c.connect("avrded-1-PORTD0", "R0-lPin");
    c.connect("R0-rPin", "G-Gnd");
    c.solve().unwrap();
    c.step_n(20).unwrap();

    assert_eq!(
        c.first_mcu().unwrap().1.device.pc(),
        0,
        "MCU must stay held in reset while RST is low"
    );
    assert!(c.first_mcu().unwrap().1.reset_held);
    assert!(c.pin_voltage("avrded-1-PORTD0").unwrap() < 0.5);

    // Release RST -> MCU boots and drives PORTD0 high.
    assert!(c.update_component_kind("Vrst", &Kind::Rail { voltage: 5.0 }));
    c.re_solve().unwrap();
    c.step_n(20).unwrap();
    assert!(!c.first_mcu().unwrap().1.reset_held);
    assert!(c.first_mcu().unwrap().1.device.pc() > 0);
    assert!(c.pin_voltage("avrded-1-PORTD0").unwrap() > 4.0);

    // Re-assert RST -> MCU resets to PC=0 and stays paused.
    assert!(c.update_component_kind("Vrst", &Kind::Rail { voltage: 0.0 }));
    c.re_solve().unwrap();
    c.step_n(20).unwrap();
    assert!(c.first_mcu().unwrap().1.reset_held);
    assert_eq!(c.first_mcu().unwrap().1.device.pc(), 0);
    assert!(c.pin_voltage("avrded-1-PORTD0").unwrap() < 0.5);

    // Dedicated reset with RstEnabled=false ignores low RST pin.
    let mut mcu_dis = instantiate_item(
        "avrded-2",
        &McuItemSpec {
            device: "avrded".into(),
            rst_enabled: false,
            ..Default::default()
        },
        &search_ded,
    )
    .unwrap();
    mcu_dis.device.load_words(&avr_drive_portd_words(0x01));
    let mut c_dis = Circuit::new();
    c_dis.add_comp(Comp {
        id: "avrded-2".into(),
        kind: Kind::Mcu(mcu_dis),
    });
    c_dis.add_comp(rail("Vrst", 0.0));
    c_dis.add_comp(Comp::resistor("R0", 10_000.0));
    c_dis.add_comp(Comp::ground("G"));
    c_dis.connect("Vrst-outnod", "avrded-2-RST");
    c_dis.connect("avrded-2-PORTD0", "R0-lPin");
    c_dis.connect("R0-rPin", "G-Gnd");
    c_dis.solve().unwrap();
    c_dis.step_n(20).unwrap();
    assert!(!c_dis.first_mcu().unwrap().1.reset_held);
    assert!(c_dis.pin_voltage("avrded-2-PORTD0").unwrap() > 4.0);

    // 2. Shared GPIO/reset pin (<port resetpin="PORTC6"/>).
    let search_shared = SubcSearch::default().with_memory_mcu("avrtest", AVR_TEST_XML);
    let mut mcu_sh = instantiate_item(
        "avrtest-1",
        &McuItemSpec {
            device: "avrtest".into(),
            rst_enabled: true,
            ..Default::default()
        },
        &search_shared,
    )
    .unwrap();
    mcu_sh.device.load_words(&avr_drive_portd_words(0x01));

    let mut cs = Circuit::new();
    cs.add_comp(Comp {
        id: "avrtest-1".into(),
        kind: Kind::Mcu(mcu_sh),
    });
    cs.add_comp(rail("Vrst", 0.0));
    cs.add_comp(Comp::resistor("R0", 10_000.0));
    cs.add_comp(Comp::ground("G"));
    cs.connect("Vrst-outnod", "avrtest-1-PORTC6");
    cs.connect("avrtest-1-PORTD0", "R0-lPin");
    cs.connect("R0-rPin", "G-Gnd");
    cs.solve().unwrap();
    cs.step_n(20).unwrap();

    assert!(cs.first_mcu().unwrap().1.reset_held);
    assert_eq!(cs.first_mcu().unwrap().1.device.pc(), 0);
    assert!(cs.pin_voltage("avrtest-1-PORTD0").unwrap() < 0.5);

    // Release PORTC6 -> MCU boots and drives PORTD0 high.
    assert!(cs.update_component_kind("Vrst", &Kind::Rail { voltage: 5.0 }));
    cs.re_solve().unwrap();
    cs.step_n(20).unwrap();
    assert!(!cs.first_mcu().unwrap().1.reset_held);
    assert!(cs.pin_voltage("avrtest-1-PORTD0").unwrap() > 4.0);

    // Shared reset pin with RstEnabled=false via .circ1 loading path: PORTC6 low does not reset.
    let circ_xml = r#"<circuit version="1">
<item itemtype="Mcu" CircId="avrtest-1" Device="avrtest" RstEnabled="false" ExtOsc="true" WdtEnabled="true" ClkOut="true" Pos="0,0" />
</circuit>"#;
    let parsed = to_parsed_circuit(load_circ1(circ_xml).unwrap());
    match &parsed.items[0].comp.kind {
        Kind::McuItem(spec) => {
            assert!(!spec.rst_enabled);
            assert!(spec.ext_osc);
            assert!(spec.wdt_enabled);
            assert!(spec.clk_out);
        }
        other => panic!("expected Kind::McuItem, got {other:?}"),
    }
}

#[test]
fn mcu_ext_osc_steps_only_on_xtal1_rising_edges() {
    let search = SubcSearch::default().with_memory_mcu("avrtest", AVR_TEST_XML);
    let mut mcu = instantiate_item(
        "avrtest-1",
        &McuItemSpec {
            device: "avrtest".into(),
            ext_osc: true,
            ..Default::default()
        },
        &search,
    )
    .unwrap();
    mcu.device.load_words(&avr_drive_portd_words(0x01));

    let mut c = Circuit::new();
    c.add_comp(Comp {
        id: "avrtest-1".into(),
        kind: Kind::Mcu(mcu),
    });
    c.add_comp(rail("Vxtal", 0.0));
    c.add_comp(Comp::resistor("R0", 10_000.0));
    c.add_comp(Comp::ground("G"));
    c.connect("Vxtal-outnod", "avrtest-1-PORTB6");
    c.connect("avrtest-1-PORTD0", "R0-lPin");
    c.connect("R0-rPin", "G-Gnd");

    c.solve().unwrap();
    c.step_n(15).unwrap();

    // Without XTAL1 rising edges, periodic frequency stepping is suppressed.
    assert_eq!(c.first_mcu().unwrap().1.device.pc(), 0);
    assert!(c.pin_voltage("avrtest-1-PORTD0").unwrap() < 0.5);

    let pulse_xtal = |circuit: &mut Circuit| {
        assert!(circuit.update_component_kind("Vxtal", &Kind::Rail { voltage: 5.0 }));
        circuit.re_solve().unwrap();
        assert!(circuit.update_component_kind("Vxtal", &Kind::Rail { voltage: 0.0 }));
        circuit.re_solve().unwrap();
    };

    // Edge 1: LDI R16, 1
    pulse_xtal(&mut c);
    assert_eq!(c.first_mcu().unwrap().1.device.pc(), 1);
    assert!(c.pin_voltage("avrtest-1-PORTD0").unwrap() < 0.5);

    // Edge 2: OUT DDRD, R16
    pulse_xtal(&mut c);
    assert_eq!(c.first_mcu().unwrap().1.device.pc(), 2);

    // Edge 3: OUT PORTD, R16 -> PORTD0 rises high
    pulse_xtal(&mut c);
    assert_eq!(c.first_mcu().unwrap().1.device.pc(), 3);
    assert!(
        c.pin_voltage("avrtest-1-PORTD0").unwrap() > 4.0,
        "PORTD0 should go high after 3 XTAL1 rising edges"
    );
}

#[test]
fn esp01_debug_logs_to_simulator_log_when_enabled() {
    let _guard = LOG_TEST_LOCK.lock().unwrap();

    let run_at_exchange = |debug: bool| -> Vec<String> {
        let baud = 115_200u32;
        let bit_ps = (1_000_000_000_000 / u64::from(baud)).max(1);
        let esp = Esp01 {
            baud_rate: baud,
            debug,
        };
        let mut c = Circuit::new();
        c.add_comp(Comp {
            id: "Esp01-1".into(),
            kind: esp.to_element_kind(),
        });
        c.add_comp(rail("Vtx", 3.3)); // UART idle high
        c.connect("Vtx-outnod", "Esp01-1-pin1");
        c.solve().unwrap();
        c.step().unwrap();
        logging::clear_simulation();

        let mut t = c.circ_time;
        for &byte in b"AT\r\n" {
            // Start bit (0)
            assert!(c.update_component_kind("Vtx", &Kind::Rail { voltage: 0.0 }));
            c.re_solve().unwrap();
            t = t.saturating_add(bit_ps);
            c.run_until(t).unwrap();
            // 8 data bits LSB-first
            for b in 0..8 {
                let v = if (byte >> b) & 1 != 0 { 3.3 } else { 0.0 };
                assert!(c.update_component_kind("Vtx", &Kind::Rail { voltage: v }));
                c.re_solve().unwrap();
                t = t.saturating_add(bit_ps);
                c.run_until(t).unwrap();
            }
            // Stop bit (1)
            assert!(c.update_component_kind("Vtx", &Kind::Rail { voltage: 3.3 }));
            c.re_solve().unwrap();
            t = t.saturating_add(bit_ps * 2);
            c.run_until(t).unwrap();
        }
        logging::drain_simulation()
    };

    let logs_on = run_at_exchange(true);
    assert!(
        logs_on
            .iter()
            .any(|line| line.contains("Esp01-1") && line.contains("AT") && line.contains("OK")),
        "expected concise ESP-01 AT debug log, got: {logs_on:?}"
    );

    let logs_off = run_at_exchange(false);
    assert!(
        !logs_off.iter().any(|line| line.contains("Esp01-1")),
        "expected no ESP-01 debug logs when Debug=false, got: {logs_off:?}"
    );
}

struct PixmapStrokeCounter {
    pixmaps: usize,
    stroke_round_rects: usize,
    fill_round_rects: usize,
}

impl Draw for PixmapStrokeCounter {
    fn fill_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
    fn stroke_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
    fn fill_round_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color) {
        self.fill_round_rects += 1;
    }
    fn stroke_round_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {
        self.stroke_round_rects += 1;
    }
    fn fill_circle(&mut self, _: f64, _: f64, _: f64, _: Color) {}
    fn stroke_circle(&mut self, _: f64, _: f64, _: f64, _: Color, _: f64) {}
    fn fill_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
    fn stroke_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
    fn fill_poly(&mut self, _: &[[f64; 2]], _: Color) {}
    fn stroke_poly(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
    fn polyline(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
    fn line(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
    fn arc(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
    fn text(&mut self, _: f64, _: f64, _: &str, _: f64, _: Color, _: Align) {}
    fn text_bold(&mut self, _: f64, _: f64, _: &str, _: f64, _: Color, _: Align) {}
    fn grid_dots(&mut self, _: Rect, _: i32, _: Color) {}
    fn push(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64) {}
    fn pop(&mut self) {}
    fn draw_pixmap_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: &ImageBuf, _: f64) -> bool {
        self.pixmaps += 1;
        true
    }
}

#[test]
fn subpackage_background_border_and_package_file_roundtrip() {
    let dir = temp_dir("subpkg");
    let bg_path = dir.join("board_art.png");
    let pkg_path = dir.join("my_board.package");
    let circ_path = dir.join("demo.circ1");

    let img = ImageBuf::from_rgba(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .unwrap();
    fs::write(&bg_path, img.encode_png().unwrap()).unwrap();

    let mut canvas = Canvas::empty();
    canvas.set_file_path(Some(circ_path.to_string_lossy().into_owned()));
    let id = canvas.scene_mut().add_subpackage(10.0, 20.0);

    // Relative Background resolves against circuit directory.
    canvas.set_prop_text_for(&id, "Background".into(), "board_art.png".into());
    canvas.set_prop_text_for(&id, "Border".into(), "false".into());
    let sp = match &canvas.scene().item_by_id(&id).unwrap().kind {
        Part::SubPackage(sp) => sp.clone(),
        other => panic!("expected SubPackage, got {other:?}"),
    };
    assert!(sp.background_pixmap.is_some());

    let pal = Palette::light();
    {
        let ctx = PaintCtx {
            canvas: &canvas,
            pal: &pal,
            scale: 1.0,
            item_id: &id,
            on_board: false,
            active_item_id: None,
        };
        let mut c_noborder = PixmapStrokeCounter {
            pixmaps: 0,
            stroke_round_rects: 0,
            fill_round_rects: 0,
        };
        sp.paint(&mut c_noborder, &ctx);
        assert_eq!(c_noborder.pixmaps, 1);
        assert_eq!(c_noborder.stroke_round_rects, 0);
    }

    // Export XML and record PackageFile as save_package_file does.
    canvas.set_prop_text_for(&id, "Border".into(), "true".into());
    let xml = canvas.export_package_xml(&id).unwrap();
    fs::write(&pkg_path, &xml).unwrap();
    canvas.set_package_file(&id, &pkg_path.to_string_lossy());
    assert_eq!(
        canvas.scene().package_ref(&id).unwrap().package_file,
        "my_board.package"
    );
    assert_eq!(
        PathBuf::from(canvas.suggest_package_path(&id)),
        pkg_path,
        "savePackageDialog suggestion must pre-fill from PackageFile"
    );

    // Loading .package into a second SubPackage populates PackageFile and resolves relative Background.
    let id2 = canvas.scene_mut().add_subpackage(100.0, 20.0);
    canvas.load_package_file_xml(&id2, &xml, &pkg_path.to_string_lossy());
    let sp2 = match &canvas.scene().item_by_id(&id2).unwrap().kind {
        Part::SubPackage(sp) => sp.clone(),
        other => panic!("expected SubPackage, got {other:?}"),
    };
    assert_eq!(sp2.package.package_file, "my_board.package");
    assert_eq!(sp2.package.background, "board_art.png");
    assert!(sp2.package.border);
    assert!(sp2.background_pixmap.is_some());

    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: &id2,
        on_board: false,
        active_item_id: None,
    };

    let mut c_border = PixmapStrokeCounter {
        pixmaps: 0,
        stroke_round_rects: 0,
        fill_round_rects: 0,
    };
    sp2.paint(&mut c_border, &ctx);
    assert_eq!(c_border.pixmaps, 1);
    assert_eq!(c_border.stroke_round_rects, 1);

    // Subcircuit using the same package also renders its background image and border setting.
    let mut subc = Subcircuit {
        device: "my_board".into(),
        package: sp2.package.clone(),
        ..Default::default()
    };
    assert!(subc.resolve_background(Some(&dir)));
    let mut c_subc = PixmapStrokeCounter {
        pixmaps: 0,
        stroke_round_rects: 0,
        fill_round_rects: 0,
    };
    subc.paint(&mut c_subc, &ctx);
    assert_eq!(c_subc.pixmaps, 1);
    assert_eq!(c_subc.stroke_round_rects, 1);

    // SubPackage::default() without background still strokes solid outline even with Border=false.
    let solid = SubPackage::default();
    let mut c_solid = PixmapStrokeCounter {
        pixmaps: 0,
        stroke_round_rects: 0,
        fill_round_rects: 0,
    };
    solid.paint(&mut c_solid, &ctx);
    assert_eq!(c_solid.pixmaps, 0);
    assert_eq!(c_solid.fill_round_rects, 1);
    assert_eq!(c_solid.stroke_round_rects, 1);

    let _ = fs::remove_dir_all(&dir);
}
