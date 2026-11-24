//! End-to-end tests for Board/Shield subcircuits:
//! - `boardPos`/`boardRot`/`boardHflip`/`boardVflip` serialization & parsing
//! - "Edit Board Layout" authoring mode on `SubPackage`
//! - Rendering & live simulation sync of board-mounted visual components (e.g. LED)
//! - Legacy `.sim1` board fixtures (`Uno.sim1`, `Encoder.sim1`)
//! - Interactive board-mounted controls (`Push`, `Switch`) in a parent circuit.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use cs_engine::canvas::draw::{Align, Color, Draw, PaintCtx, Palette};
use cs_engine::canvas::export::paint::paint_scene;
use cs_engine::canvas::scene::HitTarget;
use cs_engine::canvas::{BUTTON_LEFT, Canvas, KEY_ESCAPE, Part, Point, Rect, Scene};
use cs_engine::catalog::Catalog;
use cs_engine::circ1::BoardPlacement;
use cs_engine::elements::Kind;
use cs_engine::mcu;
use cs_engine::package::{PkgPin, SubcType};
use cs_engine::subcircuit::{self, SubcSearch};

#[derive(Default)]
struct PaintRecorder {
    fills: Vec<Color>,
    strokes: Vec<Color>,
    circle_fills: Vec<(f64, f64, f64, Color)>,
    texts: Vec<String>,
}

impl Draw for PaintRecorder {
    fn fill_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, c: Color) {
        self.fills.push(c);
    }
    fn stroke_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, c: Color, _width: f64) {
        self.strokes.push(c);
    }
    fn fill_round_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _r: f64, c: Color) {
        self.fills.push(c);
    }
    fn stroke_round_rect(
        &mut self,
        _x: f64,
        _y: f64,
        _w: f64,
        _h: f64,
        _r: f64,
        c: Color,
        _width: f64,
    ) {
        self.strokes.push(c);
    }
    fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color) {
        self.fills.push(c);
        self.circle_fills.push((cx, cy, r, c));
    }
    fn stroke_circle(&mut self, _cx: f64, _cy: f64, _r: f64, c: Color, _width: f64) {
        self.strokes.push(c);
    }
    fn fill_ellipse(&mut self, _cx: f64, _cy: f64, _rx: f64, _ry: f64, c: Color) {
        self.fills.push(c);
    }
    fn stroke_ellipse(&mut self, _cx: f64, _cy: f64, _rx: f64, _ry: f64, c: Color, _width: f64) {
        self.strokes.push(c);
    }
    fn fill_poly(&mut self, _pts: &[[f64; 2]], c: Color) {
        self.fills.push(c);
    }
    fn stroke_poly(&mut self, _pts: &[[f64; 2]], c: Color, _width: f64, _closed: bool) {
        self.strokes.push(c);
    }
    fn arc(&mut self, _cx: f64, _cy: f64, _r: f64, _start: f64, _span: f64, c: Color, _width: f64) {
        self.strokes.push(c);
    }
    fn line(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, c: Color, _width: f64) {
        self.strokes.push(c);
    }
    fn polyline(&mut self, _pts: &[[f64; 2]], c: Color, _width: f64, _dashed: bool) {
        self.strokes.push(c);
    }
    fn text(&mut self, _x: f64, _y: f64, s: &str, _size: f64, _c: Color, _align: Align) {
        self.texts.push(s.to_string());
    }
    fn grid_dots(&mut self, _scene: Rect, _step: i32, _c: Color) {}
    fn push(&mut self, _x: f64, _y: f64, _rot: f64, _sx: f64, _sy: f64) {}
    fn pop(&mut self) {}
}

fn wire(scene: &mut Scene, a: &str, b: &str) {
    let ap = scene.pin_scene(a).unwrap_or(Point::new(0.0, 0.0));
    let bp = scene.pin_scene(b).unwrap_or(Point::new(0.0, 0.0));
    scene.connect_pins(a, ap, b, bp);
}

#[test]
fn board_layout_authoring_roundtrip_and_live_led_rendering() {
    // 1. Build a subcircuit with SubPackage + 2 Tunnels + Resistor + Yellow LED.
    let mut sub_canvas = Canvas::empty();
    sub_canvas.set_view_size(800.0, 600.0);
    let pkg_id = sub_canvas.scene_mut().add_subpackage(-120.0, -80.0);
    if let Part::SubPackage(sp) = &mut sub_canvas.scene_mut().item_by_id_mut(&pkg_id).unwrap().kind
    {
        sp.package.name = "LED_MOD".into();
        sp.package.width = 6;
        sp.package.height = 5;
        sp.package.pins = vec![
            PkgPin::new("anode", "A", "", -8, 16, 180),
            PkgPin::new("cathode", "K", "", 56, 16, 0),
        ];
    }
    let t_a = sub_canvas.scene_mut().add_tunnel(80.0, 40.0);
    sub_canvas.set_prop_text_for(&t_a, "Name".into(), "anode".into());
    let r_id = sub_canvas.scene_mut().add_resistor(144.0, 40.0, 220.0);
    let led_id = sub_canvas.scene_mut().add_led(216.0, 40.0);
    let t_k = sub_canvas.scene_mut().add_tunnel(280.0, 40.0);
    sub_canvas.set_prop_text_for(&t_k, "Name".into(), "cathode".into());
    wire(
        sub_canvas.scene_mut(),
        &format!("{t_a}-pin"),
        &format!("{r_id}-lPin"),
    );
    wire(
        sub_canvas.scene_mut(),
        &format!("{r_id}-rPin"),
        &format!("{led_id}-lPin"),
    );
    wire(
        sub_canvas.scene_mut(),
        &format!("{led_id}-rPin"),
        &format!("{t_k}-pin"),
    );

    let orig_led_pos = sub_canvas.scene().item_by_id(&led_id).unwrap().position();
    let orig_wire_pts: Vec<Vec<Point>> = sub_canvas
        .scene()
        .wires()
        .iter()
        .map(|w| w.points.clone())
        .collect();

    // 2. Enter "Edit Board Layout" mode on the SubPackage.
    sub_canvas.enter_board_layout(&pkg_id);
    assert!(sub_canvas.is_board_layout());
    assert_eq!(sub_canvas.board_layout_pkg(), Some(pkg_id.as_str()));

    // SubPackage is promoted to Board, logic_symbol=false, anchored at (0, 0).
    let pkg_item = sub_canvas.scene().item_by_id(&pkg_id).unwrap();
    assert_eq!(pkg_item.position(), Point::new(0.0, 0.0));
    let Part::SubPackage(sp) = &pkg_item.kind else {
        panic!("expected SubPackage");
    };
    assert_eq!(sp.package.subc_type, SubcType::Board);
    assert!(!sp.package.logic_symbol);

    // Tunnels are hidden in board layout mode; LED is visible and parked off-board.
    assert!(
        !sub_canvas.is_item_visible_in_board_layout(sub_canvas.scene().item_by_id(&t_a).unwrap())
    );
    assert!(
        sub_canvas.is_item_visible_in_board_layout(sub_canvas.scene().item_by_id(&led_id).unwrap())
    );
    assert!(
        !sub_canvas
            .is_item_mounted_in_board_layout(sub_canvas.scene().item_by_id(&led_id).unwrap())
    );

    // Drag the LED onto the center of the SubPackage face (24, 20) and rotate 90°.
    let led_board_now = sub_canvas.scene().item_by_id(&led_id).unwrap().position();
    let start_screen = sub_canvas.viewport().map_from_circuit(led_board_now);
    let target_screen = sub_canvas
        .viewport()
        .map_from_circuit(Point::new(24.0, 20.0));
    sub_canvas.mouse_press(BUTTON_LEFT, start_screen.x, start_screen.y, 0);
    sub_canvas.mouse_move(target_screen.x, target_screen.y, BUTTON_LEFT, 0);
    sub_canvas.mouse_release(BUTTON_LEFT, target_screen.x, target_screen.y, 0);
    sub_canvas.rotate_cw();

    let mounted_led = sub_canvas.scene().item_by_id(&led_id).unwrap();
    assert!(sub_canvas.is_item_mounted_in_board_layout(mounted_led));
    assert_eq!(mounted_led.rotation, 90.0);

    // Saving while STILL in board layout mode preserves schematic Pos and writes boardPos/boardRot.
    let saved_mid_mode = sub_canvas.to_circ1();
    assert!(
        saved_mid_mode.contains("Pos=\"216,40\""),
        "{saved_mid_mode}"
    );
    assert!(saved_mid_mode.contains("boardPos="), "{saved_mid_mode}");
    assert!(
        saved_mid_mode.contains("boardRot=\"90\""),
        "{saved_mid_mode}"
    );

    // Escape exits board layout mode cleanly and restores schematic positions & wires untouched.
    sub_canvas.key_press(KEY_ESCAPE, 0);
    assert!(!sub_canvas.is_board_layout());
    let restored_led = sub_canvas.scene().item_by_id(&led_id).unwrap();
    assert_eq!(restored_led.position(), orig_led_pos);
    assert_eq!(restored_led.rotation, 0.0);
    let placement = restored_led
        .board
        .expect("LED should have saved board placement");
    assert_eq!(placement.rotation, 90.0);
    assert!(placement.x >= 0.0 && placement.x <= 48.0);
    assert!(placement.y >= 0.0 && placement.y <= 40.0);
    // Resistor stayed parked outside the package, so it was not mounted.
    assert!(
        sub_canvas
            .scene()
            .item_by_id(&r_id)
            .unwrap()
            .board
            .is_none()
    );
    // Schematic wires were completely untouched.
    let after_wire_pts: Vec<Vec<Point>> = sub_canvas
        .scene()
        .wires()
        .iter()
        .map(|w| w.points.clone())
        .collect();
    assert_eq!(after_wire_pts, orig_wire_pts);

    // 3. Place the saved board subcircuit in a parent Canvas and power it up!
    let subc_src = sub_canvas.to_circ1();
    let search = SubcSearch::empty().with_memory("LED_MOD", subc_src);
    let mut parent = Canvas::empty();
    parent.set_view_size(800.0, 600.0);
    let v_id = parent.scene_mut().add_fixed_volt(0.0, 0.0, 5.0);
    let g_id = parent.scene_mut().add_ground(200.0, 80.0);
    let subc_id = parent
        .scene_mut()
        .add_subcircuit("LED_MOD", 80.0, 0.0, &search)
        .unwrap();
    wire(
        parent.scene_mut(),
        &format!("{v_id}-outnod"),
        &format!("{subc_id}-anode"),
    );
    wire(
        parent.scene_mut(),
        &format!("{subc_id}-cathode"),
        &format!("{g_id}-Gnd"),
    );

    let Part::Subcircuit(subc) = &parent.scene().item_by_id(&subc_id).unwrap().kind else {
        panic!("expected Subcircuit");
    };
    assert_eq!(subc.board_children.len(), 1);
    assert_eq!(subc.board_children[0].inner_id, led_id);

    let pal = Palette::light();
    let yellow_lit = Color::rgb(255, 238, 34);

    // Before power_on: LED paints unlit (no bright yellow glow).
    let mut rec_off = PaintRecorder::default();
    paint_scene(
        &mut rec_off,
        &PaintCtx {
            canvas: &parent,
            pal: &pal,
            scale: 1.0,
            item_id: "",
            on_board: false,
            active_item_id: None,
        },
    );
    assert!(
        !rec_off.circle_fills.is_empty(),
        "board-mounted LED should paint circles even when off"
    );
    assert!(
        !rec_off
            .circle_fills
            .iter()
            .any(|(_, _, _, c)| c.r == yellow_lit.r && c.g == yellow_lit.g && c.b == yellow_lit.b),
        "LED should be unlit before power_on"
    );

    // After power_on: inner LED pin voltages sync and LED paints lit!
    parent.power_on();
    let sim_led_pin = format!("1@{led_id}-lPin");
    let v_anode = parent
        .pin_voltage(&sim_led_pin)
        .expect("inner board LED pin voltage should be synced into Canvas");
    assert!(v_anode > 2.0, "expected forward-biased LED, got {v_anode}");

    let mut rec_on = PaintRecorder::default();
    paint_scene(
        &mut rec_on,
        &PaintCtx {
            canvas: &parent,
            pal: &pal,
            scale: 1.0,
            item_id: "",
            on_board: false,
            active_item_id: None,
        },
    );
    assert!(
        rec_on
            .circle_fills
            .iter()
            .any(|(_, _, _, c)| c.r == yellow_lit.r && c.g == yellow_lit.g && c.b == yellow_lit.b),
        "board-mounted LED should glow yellow when powered"
    );

    // Drop supply voltage to 0 V -> parent subcircuit is marked dirty and LED turns off.
    let _ = parent.take_dirty();
    parent.set_prop_text_for(&v_id, "Voltage".into(), "0 V".into());
    let dirty = parent.take_dirty();
    assert!(
        dirty.full || dirty.items.contains(&subc_id),
        "parent subcircuit must be dirtied when board LED turns off"
    );
    let mut rec_zero = PaintRecorder::default();
    paint_scene(
        &mut rec_zero,
        &PaintCtx {
            canvas: &parent,
            pal: &pal,
            scale: 1.0,
            item_id: "",
            on_board: false,
            active_item_id: None,
        },
    );
    assert!(
        !rec_zero
            .circle_fills
            .iter()
            .any(|(_, _, _, c)| c.r == yellow_lit.r && c.g == yellow_lit.g && c.b == yellow_lit.b),
        "board-mounted LED should turn off at 0 V"
    );
}

#[test]
fn interactive_board_mounted_push_and_switch_drive_inner_led_live() {
    let panel_subc = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="PANEL" SubcType="Board" Width="10" Height="6" Pins="Pin; type=; xpos=-8; ypos=16; angle=180; length=8; space=0; id=vcc; label=VCC&#xa;Pin; type=; xpos=88; ypos=16; angle=0; length=8; space=0; id=gnd; label=GND&#xa;" />
<item itemtype="Tunnel" CircId="T-vcc" Name="vcc" />
<item itemtype="Tunnel" CircId="T-gnd" Name="gnd" />
<item itemtype="Push" CircId="Push-1" Pos="100,0" boardPos="20,24" />
<item itemtype="Switch" CircId="Switch-1" Checked="false" Pos="160,0" boardPos="44,24" />
<item itemtype="Resistor" CircId="R-1" Resistance="220 Ω" Pos="220,0" />
<item itemtype="Led" CircId="Led-1" Color="Green" Threshold="2.2 V" MaxCurrent="30 mA" Resistance="0.6 Ω" Pos="280,0" boardPos="66,24" />
<item itemtype="Connector" uid="c1" startpinid="T-vcc-pin" endpinid="Push-1-lPin0" pointList="0,0,8,0" />
<item itemtype="Connector" uid="c2" startpinid="Push-1-rPin0" endpinid="Switch-1-pinP0" pointList="8,0,16,0" />
<item itemtype="Connector" uid="c3" startpinid="Switch-1-switch0pinN" endpinid="R-1-lPin" pointList="16,0,24,0" />
<item itemtype="Connector" uid="c4" startpinid="R-1-rPin" endpinid="Led-1-lPin" pointList="24,0,32,0" />
<item itemtype="Connector" uid="c5" startpinid="Led-1-rPin" endpinid="T-gnd-pin" pointList="32,0,40,0" />
</circuit>"#;

    let search = SubcSearch::empty().with_memory("panel", panel_subc);
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    let v_id = canvas.scene_mut().add_fixed_volt(0.0, 0.0, 5.0);
    let g_id = canvas.scene_mut().add_ground(240.0, 80.0);
    let subc_id = canvas
        .scene_mut()
        .add_subcircuit("panel", 80.0, 0.0, &search)
        .unwrap();
    wire(
        canvas.scene_mut(),
        &format!("{v_id}-outnod"),
        &format!("{subc_id}-vcc"),
    );
    wire(
        canvas.scene_mut(),
        &format!("{subc_id}-gnd"),
        &format!("{g_id}-Gnd"),
    );

    canvas.power_on();

    // Initially both Push and Switch are open -> LED anode voltage is ~0V.
    assert!(canvas.pin_voltage("1@Led-1-lPin").unwrap_or(0.0) < 0.5);

    // 1. Click Switch-1 on the board face (subcircuit is at (80, 0), Switch at local (44, 24) -> scene (124, 24)).
    let sw_pt = canvas
        .viewport()
        .map_from_circuit(Point::new(80.0 + 44.0, 24.0));
    canvas.mouse_press(BUTTON_LEFT, sw_pt.x, sw_pt.y, 0);
    canvas.mouse_release(BUTTON_LEFT, sw_pt.x, sw_pt.y, 0);
    // Switch is now closed, Push is still open -> LED still off.
    assert!(canvas.pin_voltage("1@Led-1-lPin").unwrap_or(0.0) < 0.5);

    // 2. Press and hold Push-1 on the board face (local (20, 24) -> scene (100, 24)).
    let push_pt = canvas
        .viewport()
        .map_from_circuit(Point::new(80.0 + 20.0, 24.0));
    canvas.mouse_press(BUTTON_LEFT, push_pt.x, push_pt.y, 0);
    let v_on = canvas.pin_voltage("1@Led-1-lPin").unwrap_or(0.0);
    assert!(
        v_on > 2.0,
        "LED should light while board-mounted Push is pressed (v={v_on})"
    );

    // 3. Release Push-1 -> LED turns back off!
    canvas.mouse_release(BUTTON_LEFT, push_pt.x, push_pt.y, 0);
    let v_off = canvas.pin_voltage("1@Led-1-lPin").unwrap_or(0.0);
    assert!(
        v_off < 0.5,
        "LED should turn off when board-mounted Push is released (v={v_off})"
    );

    // 4. Board-mounted Push renders as a tactile physical button (with "PUSH" label)
    //    even though ShowButton was left at default (false).
    let pal = Palette::light();
    let mut rec = PaintRecorder::default();
    paint_scene(
        &mut rec,
        &PaintCtx {
            canvas: &canvas,
            pal: &pal,
            scale: 1.0,
            item_id: "",
            on_board: false,
            active_item_id: None,
        },
    );
    assert!(
        rec.texts.iter().any(|t| t == "PUSH"),
        "board-mounted Push should render physical PUSH button even when ShowButton=false"
    );
}

#[test]
fn legacy_sim1_board_fixtures_extract_only_onboard_children() {
    // 1. Arduino Uno board fixture: LedBar-9 (1 LED) is on-board; tunnels/MCU/buffer are off-board.
    let uno_src = include_str!("../fixtures/components/Arduino/arduino/Uno/Uno.sim1");
    let search = SubcSearch::empty().with_memory("Uno", uno_src);
    let mut canvas = Canvas::empty();
    let uno_id = canvas
        .scene_mut()
        .add_subcircuit("Uno", 0.0, 0.0, &search)
        .unwrap();
    {
        let Part::Subcircuit(subc) = &canvas.scene().item_by_id(&uno_id).unwrap().kind else {
            panic!("expected Subcircuit");
        };
        assert_eq!(subc.board_children.len(), 1, "{:?}", subc.board_children);
        assert_eq!(subc.board_children[0].inner_id, "LedBar-9");
        assert_eq!(
            subc.board_children[0].placement,
            BoardPlacement {
                x: 40.0,
                y: 104.0,
                rotation: -180.0,
                hflip: 1,
                vflip: 1,
            }
        );
        let Part::LedBar(bar) = subc.board_children[0].part.as_ref() else {
            panic!("expected LedBar");
        };
        assert_eq!(
            bar.segments, 1,
            "Uno Size=\"1 _Leds\" should parse as 1 segment"
        );
    }

    // Switching Uno to Logic Symbol hides board children; switching back restores them.
    canvas.set_prop_bool_for(&uno_id, "LogicSymbol".into(), true);
    if let Part::Subcircuit(subc) = &canvas.scene().item_by_id(&uno_id).unwrap().kind {
        assert!(subc.board_children.is_empty());
    }
    canvas.set_prop_bool_for(&uno_id, "LogicSymbol".into(), false);
    if let Part::Subcircuit(subc) = &canvas.scene().item_by_id(&uno_id).unwrap().kind {
        assert_eq!(subc.board_children.len(), 1);
    }

    // 2. Encoder board fixture: Dial, 7-Seg BCD, Text, and 4 Lines are on-board; Tunnels & Script MCU are off-board.
    let enc_src = include_str!("../fixtures/components/Tools/tools/Encoder/Encoder.sim1");
    let enc_search = SubcSearch::empty().with_memory("Encoder", enc_src);
    let enc_id = canvas
        .scene_mut()
        .add_subcircuit("Encoder", 300.0, 0.0, &enc_search)
        .unwrap();
    let Part::Subcircuit(enc) = &canvas.scene().item_by_id(&enc_id).unwrap().kind else {
        panic!("expected Subcircuit");
    };
    let ids: Vec<&str> = enc
        .board_children
        .iter()
        .map(|c| c.inner_id.as_str())
        .collect();
    assert!(ids.contains(&"Dial-2"), "{ids:?}");
    assert!(ids.contains(&"7-Seg BCD-7"), "{ids:?}");
    assert!(ids.contains(&"TextComponent-16"), "{ids:?}");
    assert!(!ids.contains(&"Encoderscript-11"), "{ids:?}");
    assert!(!ids.iter().any(|id| id.starts_with("Tunnel")), "{ids:?}");
}

#[test]
fn board_subpackage_named_ls_still_renders_board_children_even_with_stale_logic_symbol() {
    // SubPackage originally created from the "Logic Symbol" preset keeps Name="LS",
    // then user enters Edit Board Layout (promoting SubcType="Board", LogicSymbol="false")
    // and mounts Led-1 on the board face.
    let subc_src = r#"<circuit version="1.0.0">
<item itemtype="Led" CircId="Led-1" Color="Yellow" Pos="-124,-36" boardPos="24,20" />
<item itemtype="Tunnel" CircId="Tunnel-2" Name="anode" Pos="-156,-36" />
<item itemtype="Tunnel" CircId="Tunnel-3" Name="cathode" Pos="-108,-52" />
<item itemtype="SubPackage" CircId="SubPackage-2" Name="LS" Width="6" Height="5" SubcType="Board" LogicSymbol="false" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=anode; label=an&#xa;Pin; type=; xpos=56; ypos=8; angle=0; length=8; space=0; id=cathode; label=ca&#xa;" Pos="-116,12" />
<item itemtype="Connector" CircId="Connector-1" startpinid="Tunnel-2-pin" endpinid="Led-1-lPin" pointList="-156,-36,-140,-36" />
<item itemtype="Connector" CircId="Connector-2" startpinid="Tunnel-3-pin" endpinid="Led-1-rPin" pointList="-108,-52,-108,-36" />
</circuit>"#;

    let search = SubcSearch::empty().with_memory("LED_PKG", subc_src);
    // 1. Fresh placement in consumer circuit.
    let mut canvas = Canvas::empty();
    let id = canvas
        .scene_mut()
        .add_subcircuit("LED_PKG", 80.0, 80.0, &search)
        .unwrap();
    let Part::Subcircuit(subc) = &canvas.scene().item_by_id(&id).unwrap().kind else {
        panic!("expected Subcircuit");
    };
    assert!(
        !subc.logic_symbol,
        "Board package named LS must not force logic_symbol=true"
    );
    assert_eq!(subc.board_children.len(), 1);
    assert_eq!(subc.board_children[0].inner_id, "Led-1");

    // 2. Existing consumer circuit that previously saved LogicSymbol="true" before board conversion.
    let consumer = r#"<circuit version="1.0.0">
<item itemtype="Subcircuit" CircId="LED_PKG-1" Device="LED_PKG" LogicSymbol="true" Package="LS" Pos="80,80" />
</circuit>"#;
    let reloaded = Scene::from_circ1_with(consumer, &search).unwrap();
    let Part::Subcircuit(subc2) = &reloaded.item_by_id("LED_PKG-1").unwrap().kind else {
        panic!("expected Subcircuit");
    };
    assert!(!subc2.logic_symbol);
    assert_eq!(subc2.board_children.len(), 1);
    assert_eq!(subc2.board_children[0].inner_id, "Led-1");
}

#[derive(Default)]
struct OrderRecorder {
    events: Vec<&'static str>,
    pin_circle_colors: Vec<Color>,
    small_stem_rects: usize,
}

impl Draw for OrderRecorder {
    fn fill_rect(&mut self, _x: f64, _y: f64, w: f64, h: f64, _c: Color) {
        if w <= 2.0 && h <= 2.0 {
            self.small_stem_rects += 1;
        }
    }
    fn stroke_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _c: Color, _width: f64) {}
    fn fill_round_rect(&mut self, _x: f64, _y: f64, w: f64, h: f64, _r: f64, _c: Color) {
        if w >= 60.0 && h >= 100.0 {
            self.events.push("board_body");
        }
    }
    fn stroke_round_rect(
        &mut self,
        _x: f64,
        _y: f64,
        _w: f64,
        _h: f64,
        _r: f64,
        _c: Color,
        _width: f64,
    ) {
    }
    fn fill_circle(&mut self, _cx: f64, _cy: f64, _r: f64, _c: Color) {}
    fn stroke_circle(&mut self, _cx: f64, _cy: f64, r: f64, c: Color, _width: f64) {
        if (r - 2.0).abs() < 0.01 {
            self.events.push("pin_ring");
            self.pin_circle_colors.push(c);
        }
    }
    fn fill_ellipse(&mut self, _cx: f64, _cy: f64, _rx: f64, _ry: f64, _c: Color) {}
    fn stroke_ellipse(&mut self, _cx: f64, _cy: f64, _rx: f64, _ry: f64, _c: Color, _width: f64) {}
    fn fill_poly(&mut self, _pts: &[[f64; 2]], _c: Color) {}
    fn stroke_poly(&mut self, _pts: &[[f64; 2]], _c: Color, _width: f64, _closed: bool) {}
    fn arc(
        &mut self,
        _cx: f64,
        _cy: f64,
        _r: f64,
        _start: f64,
        _span: f64,
        _c: Color,
        _width: f64,
    ) {
    }
    fn line(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _c: Color, _width: f64) {}
    fn polyline(&mut self, _pts: &[[f64; 2]], _c: Color, _width: f64, _dashed: bool) {
        self.events.push("wire");
    }
    fn text(&mut self, _x: f64, _y: f64, _s: &str, _size: f64, _c: Color, _align: Align) {}
    fn grid_dots(&mut self, _scene: Rect, _step: i32, _c: Color) {}
    fn push(&mut self, _x: f64, _y: f64, _rot: f64, _sx: f64, _sy: f64) {}
    fn pop(&mut self) {}
}

#[test]
fn arduino_boards_paint_hit_test_and_wire_interior_pins() {
    let boards = [
        (
            "Uno",
            include_str!("../fixtures/components/Arduino/arduino/Uno/Uno.sim1"),
            Point::new(8.0, 16.0),
            "0",
            Point::new(80.0, 112.0),
        ),
        (
            "Mega",
            include_str!("../fixtures/components/Arduino/arduino/Mega/Mega.sim1"),
            Point::new(16.0, 24.0),
            "22",
            Point::new(80.0, 160.0),
        ),
        (
            "Nano",
            include_str!("../fixtures/components/Arduino/arduino/Nano/Nano.sim1"),
            Point::new(8.0, 16.0),
            "1",
            Point::new(32.0, 76.0),
        ),
        (
            "Duemilanove",
            include_str!("../fixtures/components/Arduino/arduino/Duemilanove/Duemilanove.sim1"),
            Point::new(152.0, 16.0),
            "A5",
            Point::new(80.0, 112.0),
        ),
    ];

    for (device, src, pin_pt, pin_suffix, body_pt) in boards {
        let search = SubcSearch::empty().with_memory(device, src);
        let mut canvas = Canvas::empty();
        canvas.set_view_size(800.0, 600.0);
        let id = canvas
            .scene_mut()
            .add_subcircuit(device, 0.0, 0.0, &search)
            .unwrap();

        // Interior header pin must be hit before the board body.
        let expected_pin = format!("{id}-{pin_suffix}");
        match canvas.scene().hit_top(pin_pt) {
            Some(HitTarget::Pin { pin, .. }) => assert_eq!(pin.id, expected_pin),
            other => panic!("{device}: expected HitTarget::Pin at {pin_pt:?}, got {other:?}"),
        }

        // Open PCB area still hits the board body so the board can be selected and dragged.
        match canvas.scene().hit_top(body_pt) {
            Some(HitTarget::Body { index }) => assert_eq!(canvas.scene().items()[index].id, id),
            other => panic!("{device}: expected HitTarget::Body at {body_pt:?}, got {other:?}"),
        }
    }

    // Paint order & interactive wiring on Arduino Uno.
    let uno_src = include_str!("../fixtures/components/Arduino/arduino/Uno/Uno.sim1");
    let search = SubcSearch::empty().with_memory("Uno", uno_src);
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    let uno_id = canvas
        .scene_mut()
        .add_subcircuit("Uno", 0.0, 0.0, &search)
        .unwrap();
    let r_id = canvas.scene_mut().add_resistor(-64.0, 16.0, 1000.0);

    // Click Uno pin 0 at (8, 16) to start a wire, then close onto resistor rPin at (-48, 16).
    let p0_screen = canvas.viewport().map_from_circuit(Point::new(8.0, 16.0));
    canvas.mouse_press(BUTTON_LEFT, p0_screen.x, p0_screen.y, 0);
    assert!(
        canvas.scene().drawing(),
        "clicking an Arduino interior pin must start drawing a wire"
    );
    let r_pin = canvas
        .scene()
        .pin_scene(&format!("{r_id}-rPin"))
        .expect("resistor rPin");
    let r_screen = canvas.viewport().map_from_circuit(r_pin);
    canvas.mouse_press(BUTTON_LEFT, r_screen.x, r_screen.y, 0);
    assert!(
        !canvas.scene().drawing(),
        "clicking resistor pin should close the wire"
    );
    assert!(
        canvas.scene().pin_connected(&format!("{uno_id}-0")),
        "Uno pin 0 should be connected"
    );

    // Paint the scene and check: board body paints BEFORE interior pin rings and BEFORE wires,
    // point pins draw no stray stem rects, and idle interior pins use chip_label_text contrast.
    let pal = Palette::light();
    let mut rec = OrderRecorder::default();
    paint_scene(
        &mut rec,
        &PaintCtx {
            canvas: &canvas,
            pal: &pal,
            scale: 1.0,
            item_id: "",
            on_board: false,
            active_item_id: None,
        },
    );

    let body_idx = rec
        .events
        .iter()
        .position(|&e| e == "board_body")
        .expect("board body should be painted");
    let first_ring_idx = rec
        .events
        .iter()
        .position(|&e| e == "pin_ring")
        .expect("unconnected Arduino pins should paint rings");
    let wire_idx = rec
        .events
        .iter()
        .position(|&e| e == "wire")
        .expect("connected wire should be painted");
    assert!(
        body_idx < first_ring_idx,
        "interior pin rings must paint after board body: {:?}",
        rec.events
    );
    assert!(
        body_idx < wire_idx,
        "wires must paint after board body so interior header wires stay visible: {:?}",
        rec.events
    );
    assert_eq!(
        rec.small_stem_rects, 0,
        "point pins (length=1) must not emit stray stem rectangles"
    );
    // 26 unconnected non-directional Uno pins draw rings with chip_label_text contrast
    // (pin 0 is connected, RST draws a directional chevron).
    let board_ring_count = rec
        .pin_circle_colors
        .iter()
        .filter(|&&c| c == pal.chip_label_text)
        .count();
    assert_eq!(
        board_ring_count, 26,
        "expected 26 unconnected non-directional Uno interior pins with chip_label_text contrast"
    );
}

#[test]
fn socket_body_remains_draggable_and_unaffected_by_interior_package_pin_rule() {
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    let sock_id = canvas.scene_mut().add_socket(0.0, 0.0);
    match canvas.scene().hit_top(Point::new(0.0, 0.0)) {
        Some(HitTarget::Body { index }) => assert_eq!(canvas.scene().items()[index].id, sock_id),
        other => panic!("expected Socket body hit at (0, 0), got {other:?}"),
    }
}

#[test]
fn arduino_uno_onboard_ledbar_wire_is_connected_in_scene_and_simulation() {
    let uno_src = include_str!("../fixtures/components/Arduino/arduino/Uno/Uno.sim1");
    let scene = Scene::from_circ1(uno_src).unwrap();
    let led_wire = scene
        .wires()
        .iter()
        .find(|w| {
            w.start_pin.starts_with("LedBar-9-")
                || w.end_pin
                    .as_deref()
                    .is_some_and(|p| p.starts_with("LedBar-9-"))
        })
        .expect("Uno should contain wire to LedBar-9");
    assert_eq!(led_wire.start_pin, "LedBar-9-lPin0");
    assert!(
        scene.pin_scene(&led_wire.start_pin).is_some(),
        "LedBar-9-lPin0 must resolve on canvas"
    );
    assert!(
        scene
            .pin_scene(led_wire.end_pin.as_deref().unwrap())
            .is_some(),
        "Buffer-10-out must resolve on canvas"
    );

    // Driving Uno pin 13 high in a parent circuit drives 1@LedBar-9-lPin0 high through Buffer-10.
    let search = SubcSearch::empty()
        .with_standard_catalog()
        .with_memory("Uno", uno_src);
    let mut canvas = Canvas::empty();
    let v_id = canvas.scene_mut().add_fixed_volt(0.0, 0.0, 5.0);
    let uno_id = canvas
        .scene_mut()
        .add_subcircuit("Uno", 80.0, 0.0, &search)
        .unwrap();
    wire(
        canvas.scene_mut(),
        &format!("{v_id}-outnod"),
        &format!("{uno_id}-13"),
    );
    canvas.power_on();
    let v_led = canvas
        .pin_voltage("1@LedBar-9-lPin0")
        .expect("on-board LedBar-9 anode pin should be wired in simulation");
    assert!(
        v_led > 2.5,
        "driving Uno pin 13 high must drive LedBar-9-lPin0 high (got {v_led} V)"
    );
}

#[test]
fn all_bundled_simulide_subcircuits_have_zero_dangling_pins_and_compile_scripts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/components");
    let mut cat = Catalog::new();
    cat.load_dir(&root);
    let mut subdirs: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    for d in subdirs {
        cat.load_dir(&d);
    }
    let search = SubcSearch::empty().with_catalog(cat.clone());
    let subc_items: Vec<_> = cat
        .items()
        .iter()
        .filter(|it| it.typ == "Subcircuit")
        .collect();
    assert!(
        subc_items.len() >= 150,
        "expected full bundled SimulIDE subcircuit catalog, found {}",
        subc_items.len()
    );

    let mut sim_dangling = Vec::new();
    let mut canvas_dangling = Vec::new();
    let mut script_errors = Vec::new();

    for item in subc_items {
        let name = &item.name;
        let inst_id = format!("{name}-1");
        let view = subcircuit::load_view(&inst_id, name, &search, false, None)
            .unwrap_or_else(|e| panic!("load_view failed for {name}: {e}"));
        let child_search = view
            .nested_path
            .as_deref()
            .map(|p| search.child_for(Path::new(p)))
            .unwrap_or_else(|| search.clone());

        let scene = Scene::from_circ1_with(&view.nested_src, &child_search)
            .unwrap_or_else(|e| panic!("Scene::from_circ1_with failed for {name}: {e}"));
        for w in scene.wires() {
            if scene.pin_scene(&w.start_pin).is_none() {
                canvas_dangling.push(format!("{name}: {}", w.start_pin));
            }
            if let Some(ep) = &w.end_pin {
                if scene.pin_scene(ep).is_none() {
                    canvas_dangling.push(format!("{name}: {ep}"));
                }
            }
        }

        let mut inst = subcircuit::instantiate(
            &inst_id,
            name,
            &view.nested_src,
            &child_search,
            false,
            None,
            0,
            None,
        )
        .unwrap_or_else(|e| panic!("instantiate failed for {name}: {e}"));

        let mut known: HashSet<String> = inst.components.iter().flat_map(|c| c.pin_ids()).collect();
        for (a, b) in &inst.connectors {
            for p in [a, b] {
                if p.contains("Node-") {
                    known.insert(p.clone());
                }
            }
        }
        for (a, b) in &inst.connectors {
            for ep in [a, b] {
                if known.contains(ep) {
                    continue;
                }
                let is_mcu_gpio = inst.components.iter().any(|c| {
                    if let Kind::Mcu(m) = &c.kind {
                        if let Some(local) = ep.strip_prefix(&format!("{}-", c.id)) {
                            return mcu::match_gpio(m, &c.id, local).is_some();
                        }
                    }
                    false
                });
                if !is_mcu_gpio {
                    sim_dangling.push(format!("{name}: {ep}"));
                }
            }
        }
        for c in &mut inst.components {
            if let Kind::ScriptCpu(s) = &mut c.kind {
                if let Err(e) = s.compile() {
                    script_errors.push(format!("{name} ({}): {e}", c.id));
                }
            }
        }
    }

    assert!(
        sim_dangling.is_empty(),
        "dangling simulation connector pins: {sim_dangling:?}"
    );
    assert!(
        canvas_dangling.is_empty(),
        "dangling canvas wire pins: {canvas_dangling:?}"
    );
    assert!(
        script_errors.is_empty(),
        "AngelScript compile errors: {script_errors:?}"
    );
}

#[test]
fn simulide_compat_pullup_resistor_dip_and_demux_decoder_simulate() {
    let src = r#"<circuit version="1.1.0-RC1" >
<item itemtype="ResistorDip" CircId="ResistorDip-1" Resistance="10 kΩ" Size="2" PullUp="true" PuVolt="5 V" Pos="0,0" />
<item itemtype="Resistor" CircId="R-1" Resistance="10 kΩ" Pos="40,0" />
<item itemtype="Ground" CircId="G-1" Pos="80,0" />
<item itemtype="Demux" CircId="Demux-1" Address_Bits="2 _bits" Inverted="false" Pos="120,0" />
<item itemtype="Fixed Voltage" CircId="V-1" Voltage="5 V" Pos="100,40" />
<item itemtype="Connector" uid="c1" startpinid="ResistorDip-1-resistor0-ePin0" endpinid="R-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="R-1-rPin" endpinid="G-1-Gnd" />
<item itemtype="Connector" uid="c3" startpinid="V-1-outnod" endpinid="Demux-1-in0" />
<item itemtype="Connector" uid="c4" startpinid="G-1-Gnd" endpinid="Demux-1-in1" />
<item itemtype="Connector" uid="c5" startpinid="G-1-Gnd" endpinid="Demux-1-in3" />
<item itemtype="Connector" uid="c6" startpinid="Demux-1-out1" endpinid="ResistorDip-1-resistor1-ePin2" />
</circuit>
"#;
    let mut canvas = Canvas::empty();
    canvas.load_circ1(src, None).unwrap();
    canvas.power_on();

    // 10k internal pull-up to 5V with 10k to GND => 2.5 V divider on pin0.
    let v_div = canvas
        .pin_voltage("ResistorDip-1-pin0")
        .expect("ResistorDip-1-pin0 should be wired");
    assert!(
        (v_div - 2.5).abs() < 0.05,
        "expected ~2.5 V from PullUp=true PuVolt=5V divider, got {v_div} V"
    );

    // Demux addr=0b01 (in0=5V, in1=0V), enable=0V, unwired data input -> out1 asserts HIGH (~5V).
    let v_out1 = canvas
        .pin_voltage("Demux-1-out1")
        .expect("Demux-1-out1 should be wired");
    assert!(
        v_out1 > 4.0,
        "expected Demux decoder out1 HIGH when addr=1 and enabled, got {v_out1} V"
    );
}
