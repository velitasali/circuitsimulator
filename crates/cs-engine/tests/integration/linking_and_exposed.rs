//! End-to-end tests for "Link to Component" and SubPackage "Select Exposed Components".

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cs_engine::canvas::draw::{Align, Color, Draw, PaintCtx, Palette};
use cs_engine::canvas::export::paint::paint_scene;
use cs_engine::canvas::{
    BUTTON_LEFT, BUTTON_RIGHT, Canvas, CursorKind, KEY_ESCAPE, LinkMode, Part, Point, Rect, Scene,
};
use cs_engine::components::{Csource, Dial, Ldr, Led, Potentiometer, Shape, ShapeKind};
use cs_engine::elements::Kind;
use cs_engine::script::ScriptCpu;
use cs_engine::subcircuit::{self, SubcSearch};
use cs_engine::{Circuit, digital::IoPort};

#[derive(Default)]
struct PaintRecorder {
    texts: Vec<(String, Color)>,
    fills: Vec<Color>,
    strokes: Vec<Color>,
    polylines: Vec<(usize, Color, bool)>,
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
    fn fill_circle(&mut self, _cx: f64, _cy: f64, _r: f64, c: Color) {
        self.fills.push(c);
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
    fn polyline(&mut self, pts: &[[f64; 2]], c: Color, _width: f64, dashed: bool) {
        self.polylines.push((pts.len(), c, dashed));
    }
    fn text(&mut self, _x: f64, _y: f64, s: &str, _size: f64, c: Color, _align: Align) {
        self.texts.push((s.to_string(), c));
    }
    fn grid_dots(&mut self, _scene: Rect, _step: i32, _c: Color) {}
    fn push(&mut self, _x: f64, _y: f64, _rot: f64, _sx: f64, _sy: f64) {}
    fn pop(&mut self) {}
}

fn item_pos_on_canvas(canvas: &Canvas, uid: &str) -> Point {
    let it = canvas.scene().item_by_id(uid).expect("item exists");
    let r = it.hit_rect();
    canvas
        .viewport()
        .map_from_circuit(Point::new(r.x + r.w * 0.5, r.y + r.h * 0.5))
}

fn wire(scene: &mut Scene, a: &str, b: &str) {
    let ap = scene.pin_scene(a).unwrap_or(Point::new(0.0, 0.0));
    let bp = scene.pin_scene(b).unwrap_or(Point::new(0.0, 0.0));
    scene.connect_pins(a, ap, b, bp);
}

#[test]
fn link_to_component_canvas_interaction_undo_delete_paste_and_overlay() {
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    let dial_id = canvas.scene_mut().add_dial(0.0, 0.0);
    let pot_id = canvas.scene_mut().add_potentiometer(120.0, 0.0);
    let r_id = canvas.scene_mut().add_resistor(240.0, 0.0, 1000.0);

    // Start linking from dial
    let c = canvas.start_linking(&dial_id);
    assert!(c.items);
    assert!(c.cursor);
    assert_eq!(
        canvas.link_mode(),
        Some(&LinkMode::Component(dial_id.clone()))
    );
    assert!(!canvas.is_linking_exposed());
    assert_eq!(canvas.cursor(), CursorKind::Cross);

    // Hover over valid target -> PointingHand; hover over empty -> Cross
    let pot_pt = item_pos_on_canvas(&canvas, &pot_id);
    canvas.mouse_move(pot_pt.x, pot_pt.y, 0, 0);
    assert_eq!(canvas.cursor(), CursorKind::PointingHand);

    let empty_pt = canvas
        .viewport()
        .map_from_circuit(Point::new(-300.0, -250.0));
    canvas.mouse_move(empty_pt.x, empty_pt.y, 0, 0);
    assert_eq!(canvas.cursor(), CursorKind::Cross);

    // Left-click pot and resistor to link in order [0: pot, 1: resistor]
    let pot_before = canvas.scene().item_by_id(&pot_id).unwrap().position();
    canvas.mouse_press(BUTTON_LEFT, pot_pt.x, pot_pt.y, 0);
    canvas.mouse_release(BUTTON_LEFT, pot_pt.x, pot_pt.y, 0);
    assert_eq!(
        canvas.scene().item_by_id(&dial_id).unwrap().links,
        vec![pot_id.clone()]
    );
    // Target must not have moved or been dragged
    assert_eq!(
        canvas.scene().item_by_id(&pot_id).unwrap().position(),
        pot_before
    );

    let r_pt = item_pos_on_canvas(&canvas, &r_id);
    canvas.mouse_press(BUTTON_LEFT, r_pt.x, r_pt.y, 0);
    canvas.mouse_release(BUTTON_LEFT, r_pt.x, r_pt.y, 0);
    assert_eq!(
        canvas.scene().item_by_id(&dial_id).unwrap().links,
        vec![pot_id.clone(), r_id.clone()]
    );

    // Visual overlay draws 0-based index badges ("0" and "1") and link highlight
    let pal = Palette::light();
    let mut rec = PaintRecorder::default();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: "",
        on_board: false,
        active_item_id: None,
    };
    paint_scene(&mut rec, &ctx);
    assert!(
        rec.texts
            .iter()
            .any(|(t, c)| t == "0" && *c == pal.link_badge_text)
    );
    assert!(
        rec.texts
            .iter()
            .any(|(t, c)| t == "1" && *c == pal.link_badge_text)
    );
    assert!(rec.fills.contains(&pal.link_highlight));

    // Clicking first target again toggles it OFF
    canvas.mouse_press(BUTTON_LEFT, pot_pt.x, pot_pt.y, 0);
    assert_eq!(
        canvas.scene().item_by_id(&dial_id).unwrap().links,
        vec![r_id.clone()]
    );

    // Undo restores [pot, r]
    canvas.undo();
    assert_eq!(
        canvas.scene().item_by_id(&dial_id).unwrap().links,
        vec![pot_id.clone(), r_id.clone()]
    );

    // Escape exits linking mode
    canvas.key_press(KEY_ESCAPE, 0);
    assert_eq!(canvas.link_mode(), None);

    // Right-click also exits linking mode cleanly
    canvas.start_linking(&dial_id);
    assert!(canvas.linking_from().is_some());
    canvas.mouse_press(BUTTON_RIGHT, r_pt.x, r_pt.y, 0);
    assert_eq!(canvas.link_mode(), None);

    // Serialization roundtrip preserves Links="..."
    let saved = canvas.scene().to_circ1();
    assert!(
        saved.contains(&format!("Links=\"{pot_id},{r_id},\"")),
        "{saved}"
    );
    let reloaded = Scene::from_circ1(&saved).unwrap();
    assert_eq!(
        reloaded.item_by_id(&dial_id).unwrap().links,
        vec![pot_id.clone(), r_id.clone()]
    );

    // Copy & paste remaps links among pasted items
    canvas.select_all();
    canvas.copy_selection();
    canvas.paste_at_cursor();
    let pasted_dials: Vec<_> = canvas
        .scene()
        .items()
        .iter()
        .filter(|it| matches!(it.kind, Part::Dial(_)) && it.id != dial_id)
        .collect();
    assert_eq!(pasted_dials.len(), 1);
    assert_eq!(pasted_dials[0].links.len(), 2);
    assert!(!pasted_dials[0].links.contains(&pot_id));
    assert!(!pasted_dials[0].links.contains(&r_id));
    for tid in &pasted_dials[0].links {
        assert!(canvas.scene().item_by_id(tid).is_some());
    }

    // Deleting a target prunes it from links
    canvas.scene_mut().clear_selection();
    let r_idx = canvas
        .scene()
        .items()
        .iter()
        .position(|it| it.id == r_id)
        .unwrap();
    canvas.scene_mut().select_only(r_idx);
    canvas.delete_selected();
    assert_eq!(
        canvas.scene().item_by_id(&dial_id).unwrap().links,
        vec![pot_id]
    );
}

#[test]
fn dial_linked_to_potentiometer_and_text_updates_live_simulation() {
    let mut canvas = Canvas::empty();
    let v_id = canvas.scene_mut().add_fixed_volt(0.0, 0.0, 10.0);
    let gnd_id = canvas.scene_mut().add_ground(0.0, 80.0);
    let pot_id = canvas.scene_mut().add_potentiometer(80.0, 40.0);
    let dial_id = canvas.scene_mut().add_dial(200.0, 40.0);
    let text_id = canvas.scene_mut().add_shape("Text", 200.0, 120.0);

    // Wire 10V -> lPin, GND -> rPin
    wire(
        canvas.scene_mut(),
        &format!("{v_id}-outnod"),
        &format!("{pot_id}-lPin"),
    );
    wire(
        canvas.scene_mut(),
        &format!("{gnd_id}-Gnd"),
        &format!("{pot_id}-rPin"),
    );

    // Configure Dial range 0..1000, initial value 250 (wiper = 0.25)
    if let Part::Dial(d) = &mut canvas.scene_mut().item_by_id_mut(&dial_id).unwrap().kind {
        d.min_val = 0.0;
        d.max_val = 1000.0;
        d.value = 250.0;
    }
    canvas.scene_mut().item_by_id_mut(&dial_id).unwrap().links =
        vec![pot_id.clone(), text_id.clone()];

    canvas.power_on();

    // At value=250 -> wiper=0.25 -> check voltage & synced scene state
    let v_25 = canvas.pin_voltage(&format!("{pot_id}-wPin")).unwrap();
    assert!(
        (v_25 - 2.5).abs() < 0.05 || (v_25 - 7.5).abs() < 0.05,
        "v_25={v_25}"
    );
    let Part::Potentiometer(pot) = &canvas.scene().item_by_id(&pot_id).unwrap().kind else {
        panic!("expected pot");
    };
    assert!((pot.wiper - 0.25).abs() < 1e-6);
    let Part::Shape(txt) = &canvas.scene().item_by_id(&text_id).unwrap().kind else {
        panic!("expected text");
    };
    assert_eq!(txt.text, "250");

    // Turn dial live to 750 -> wiper becomes 0.75, voltage flips, text becomes "750"
    canvas.set_prop_text_for(&dial_id, "Value".into(), "750".into());
    let v_75 = canvas.pin_voltage(&format!("{pot_id}-wPin")).unwrap();
    assert!((v_75 - v_25).abs() > 4.5, "v_25={v_25}, v_75={v_75}");
    let Part::Potentiometer(pot) = &canvas.scene().item_by_id(&pot_id).unwrap().kind else {
        panic!("expected pot");
    };
    assert!((pot.wiper - 0.75).abs() < 1e-6);
    let Part::Shape(txt) = &canvas.scene().item_by_id(&text_id).unwrap().kind else {
        panic!("expected text");
    };
    assert_eq!(txt.text, "750");
}

#[test]
fn led_linked_to_csource_acts_as_optocoupler_and_led_to_ldr_lowers_resistance() {
    // 1) Optocoupler: 5V -> 330Ω -> LED (Vf=1.8V, ~9.7mA) --link--> Csource (ControlPins=false, Gain=600) -> ~5.8V
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0);
    c.add_resistor("R1", 330.0);
    c.add_led("Led1");
    c.configure_led("Led1", 1.8, 0.02, 0.6, false);
    c.add_ground("G1");
    c.add_comp(cs_engine::elements::Comp {
        id: "Cs1".into(),
        kind: Csource {
            control_pins: false,
            curr_source: false,
            curr_control: false,
            gain: 600.0,
            volt: 0.0,
            current: 0.0,
            sense_ohms: 1.0,
        }
        .to_element_kind(),
    });
    c.add_resistor("Rload", 1000.0);
    c.add_ground("G2");

    c.connect("V1-outnod", "R1-lPin");
    c.connect("R1-rPin", "Led1-lPin");
    c.connect("Led1-rPin", "G1-Gnd");
    c.connect("Cs1-s1Pin", "Rload-lPin");
    c.connect("Rload-rPin", "G2-Gnd");
    c.connect("Cs1-s2Pin", "G2-Gnd");
    c.set_links("Led1", vec!["Cs1".into()]);

    c.solve().unwrap();
    let v_out = c.pin_voltage("Cs1-s1Pin").unwrap();
    // I_led ≈ (5.0 - 1.8) / 330.6 ≈ 9.68 mA -> V_out ≈ 5.8 V
    assert!(v_out > 4.5 && v_out < 7.0, "expected ~5.8V, got {v_out}");

    // 2) LED linked to LDR: LED ON -> LDR resistance drops from RDark toward RLight
    let mut c2 = Circuit::new();
    c2.add_fixed_volt("V1", 5.0);
    c2.add_resistor("R1", 100.0);
    c2.add_led("Led1");
    c2.configure_led("Led1", 2.0, 0.03, 0.6, false);
    c2.add_ground("G1");
    c2.add_fixed_volt("V2", 10.0);
    c2.add_comp(cs_engine::elements::Comp {
        id: "Ldr1".into(),
        kind: Ldr {
            lux: 0.1,
            r_dark: 1e6,
            r_light: 1000.0,
            dial_step: 0.0,
        }
        .to_element_kind(),
    });
    c2.add_resistor("Rdiv", 1000.0);
    c2.add_ground("G2");

    c2.connect("V1-outnod", "R1-lPin");
    c2.connect("R1-rPin", "Led1-lPin");
    c2.connect("Led1-rPin", "G1-Gnd");
    c2.connect("V2-outnod", "Ldr1-lPin");
    c2.connect("Ldr1-rPin", "Rdiv-lPin");
    c2.connect("Rdiv-rPin", "G2-Gnd");
    c2.set_links("Led1", vec!["Ldr1".into()]);

    c2.solve().unwrap();
    // LED is at ~30 mA (= max_current) -> 1000 lux -> R_ldr ≈ 1000 Ω -> V_mid ≈ 5 V
    let v_mid = c2.pin_voltage("Rdiv-lPin").unwrap();
    assert!(
        (v_mid - 5.0).abs() < 0.3,
        "expected ~5V with illuminated LDR, got {v_mid}"
    );
}

#[test]
fn dcmotor_linked_to_potentiometer_forwards_rotation_angle() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0);
    c.add_ground("G1");
    c.add_comp(cs_engine::elements::Comp::dcmotor("M1", 60, 5.0, 100.0));
    c.add_comp(cs_engine::elements::Comp::potentiometer(
        "Pot1", 1000.0, 0.0,
    ));
    c.connect("V1-outnod", "M1-lPin");
    c.connect("M1-rPin", "G1-Gnd");
    c.set_links("M1", vec!["Pot1".into()]);

    c.solve().unwrap();
    // At 60 RPM (= 1 rev/sec = 360°/s), 0.25s = 90° -> scaled 250 -> wiper = 0.25
    c.run_ps(250_000_000_000).unwrap();
    let Kind::Potentiometer { wiper, .. } = c.comp("Pot1").unwrap().kind else {
        panic!("expected pot");
    };
    assert!(
        (wiper - 0.25).abs() < 0.02,
        "expected wiper ~0.25, got {wiper}"
    );
}

#[test]
fn script_cpu_bidirectional_linking_value_string_and_prop_str() {
    let script = r#"
void setup() {}
void reset() {}
void setLinkedValue( double val, int i )
{
    component.setLinkedValue( 0, val * 10.0, 0 );
    component.setLinkedValue( 0, 20000.0, 1 );
    component.setLinkedString( 1, "VAL=" + int(val), 0 );
    component.setPropStr( 2, "Threshold", "2.5 V" );
}
"#;
    let mut cpu = ScriptCpu::new("Scr-1").with_port(IoPort::with_pins("PORTA", "Scr-1", 4));
    cpu.set_script(script);

    let mut c = Circuit::new();
    c.add_comp(cs_engine::elements::Comp {
        id: "Dial-1".into(),
        kind: Dial::new(50.0).to_element_kind(),
    });
    c.add_comp(cs_engine::elements::Comp::script_cpu("Scr-1", cpu));
    c.add_comp(cs_engine::elements::Comp {
        id: "Pot-1".into(),
        kind: Potentiometer::new(1000.0, 0.0).to_element_kind(),
    });
    c.add_comp(cs_engine::elements::Comp {
        id: "Txt-1".into(),
        kind: Shape::new_for_kind(ShapeKind::Text).to_element_kind(),
    });
    c.add_comp(cs_engine::elements::Comp {
        id: "Led-1".into(),
        kind: Led::default().to_element_kind(),
    });

    c.set_links("Dial-1", vec!["Scr-1".into()]);
    c.set_links(
        "Scr-1",
        vec!["Pot-1".into(), "Txt-1".into(), "Led-1".into()],
    );

    c.solve().unwrap();

    // Dial=50 -> Script sets Pot wiper to 500/1000=0.5, resistance=20k, Text="VAL=50", Led threshold=2.5V
    match &c.comp("Pot-1").unwrap().kind {
        Kind::Potentiometer { resistance, wiper } => {
            assert!((*wiper - 0.5).abs() < 1e-6, "wiper={wiper}");
            assert!(
                (*resistance - 20_000.0).abs() < 1e-6,
                "resistance={resistance}"
            );
        }
        other => panic!("unexpected {other:?}"),
    }
    match &c.comp("Txt-1").unwrap().kind {
        Kind::Shape { text, .. } => assert_eq!(text, "VAL=50"),
        other => panic!("unexpected {other:?}"),
    }
    match &c.comp("Led-1").unwrap().kind {
        Kind::Led { state } => assert!((state.threshold - 2.5).abs() < 1e-6),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn subcircuit_flattening_prefixes_and_runs_internal_links() {
    let opto_subc = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="OPTO" Width="3" Height="3" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=A; label=A&#xa;Pin; type=; xpos=-8; ypos=16; angle=180; length=8; space=0; id=C; label=C&#xa;Pin; type=; xpos=32; ypos=8; angle=0; length=8; space=0; id=Out; label=Out&#xa;Pin; type=; xpos=32; ypos=16; angle=0; length=8; space=0; id=Ref; label=Ref&#xa;" />
<item itemtype="Tunnel" CircId="T-A" Name="A" />
<item itemtype="Tunnel" CircId="T-C" Name="C" />
<item itemtype="Tunnel" CircId="T-Out" Name="Out" />
<item itemtype="Tunnel" CircId="T-Ref" Name="Ref" />
<item itemtype="Resistor" CircId="R-1" Resistance="330 Ω" />
<item itemtype="Led" CircId="Led-1" Threshold="1.8 V" MaxCurrent="20 mA" Resistance="0.6 Ω" Links="Cs-1," />
<item itemtype="Csource" CircId="Cs-1" ControlPins="false" CurrSource="false" Gain="500" Voltage="0 V" />
<item itemtype="Connector" uid="c1" startpinid="T-A-pin" endpinid="R-1-lPin" pointList="0,0,8,0" />
<item itemtype="Connector" uid="c2" startpinid="R-1-rPin" endpinid="Led-1-lPin" pointList="8,0,16,0" />
<item itemtype="Connector" uid="c3" startpinid="Led-1-rPin" endpinid="T-C-pin" pointList="16,0,24,0" />
<item itemtype="Connector" uid="c4" startpinid="Cs-1-s1Pin" endpinid="T-Out-pin" pointList="0,16,8,16" />
<item itemtype="Connector" uid="c5" startpinid="Cs-1-s2Pin" endpinid="T-Ref-pin" pointList="0,24,8,24" />
</circuit>"#;

    let parent = r#"<circuit version="1.0.0">
<item itemtype="FixedVolt" CircId="V-1" Voltage="5 V" />
<item itemtype="Ground" CircId="G-1" />
<item itemtype="Ground" CircId="G-2" />
<item itemtype="Resistor" CircId="Rload-1" Resistance="1 kΩ" />
<item itemtype="Subcircuit" CircId="myopto-1" Device="myopto" />
<item itemtype="Connector" uid="w1" startpinid="V-1-outnod" endpinid="myopto-1-A" pointList="0,0,8,0" />
<item itemtype="Connector" uid="w2" startpinid="myopto-1-C" endpinid="G-1-Gnd" pointList="0,8,8,8" />
<item itemtype="Connector" uid="w3" startpinid="myopto-1-Out" endpinid="Rload-1-lPin" pointList="32,0,40,0" />
<item itemtype="Connector" uid="w4" startpinid="Rload-1-rPin" endpinid="G-2-Gnd" pointList="40,8,40,16" />
<item itemtype="Connector" uid="w5" startpinid="myopto-1-Ref" endpinid="G-2-Gnd" pointList="32,8,40,16" />
</circuit>"#;

    let search = SubcSearch::empty().with_memory("myopto", opto_subc);
    let mut circuit = Circuit::from_circ1_with(parent, &search).unwrap();
    circuit.solve().unwrap();

    // Inner Led (~9.68 mA) -> inner Cs-1 (Gain=500) -> ~4.84 V on Rload-1-lPin
    let v_out = circuit.pin_voltage("Rload-1-lPin").unwrap();
    assert!(v_out > 4.0 && v_out < 5.5, "expected ~4.84V, got {v_out}");
}

#[test]
fn select_exposed_components_workflow_overlay_persistence_and_subcircuit_props() {
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);
    let pkg_id = canvas.scene_mut().add_subpackage(0.0, 0.0);
    let r1_id = canvas.scene_mut().add_resistor(120.0, 0.0, 1000.0);
    let r2_id = canvas.scene_mut().add_resistor(220.0, 0.0, 1000.0);

    // Start "Select Exposed Components" on the SubPackage
    canvas.start_linking(&pkg_id);
    assert_eq!(canvas.link_mode(), Some(&LinkMode::Exposed(pkg_id.clone())));
    assert!(canvas.is_linking_exposed());

    // Hovering resistor -> PointingHand; hovering package itself -> Cross
    let r1_pt = item_pos_on_canvas(&canvas, &r1_id);
    canvas.mouse_move(r1_pt.x, r1_pt.y, 0, 0);
    assert_eq!(canvas.cursor(), CursorKind::PointingHand);
    let pkg_pt = item_pos_on_canvas(&canvas, &pkg_id);
    canvas.mouse_move(pkg_pt.x, pkg_pt.y, 0, 0);
    assert_eq!(canvas.cursor(), CursorKind::Cross);

    // Click R1 -> marks main_comp = true
    canvas.mouse_press(BUTTON_LEFT, r1_pt.x, r1_pt.y, 0);
    assert!(canvas.scene().item_by_id(&r1_id).unwrap().main_comp);
    assert!(!canvas.scene().item_by_id(&r2_id).unwrap().main_comp);

    // Paint overlay highlights R1 in yellow (pal.exposed_highlight)
    let pal = Palette::light();
    let mut rec = PaintRecorder::default();
    let ctx = PaintCtx {
        canvas: &canvas,
        pal: &pal,
        scale: 1.0,
        item_id: "",
        on_board: false,
        active_item_id: None,
    };
    paint_scene(&mut rec, &ctx);
    assert!(rec.strokes.contains(&pal.exposed_highlight));

    // Clicking R1 again unexposes it; Undo restores exposed=true
    canvas.mouse_press(BUTTON_LEFT, r1_pt.x, r1_pt.y, 0);
    assert!(!canvas.scene().item_by_id(&r1_id).unwrap().main_comp);
    canvas.undo();
    assert!(canvas.scene().item_by_id(&r1_id).unwrap().main_comp);

    // Clicking the package itself exits exposed-selection mode
    canvas.mouse_press(BUTTON_LEFT, pkg_pt.x, pkg_pt.y, 0);
    assert_eq!(canvas.link_mode(), None);

    // Save/load preserves mainComp="true" on R1 and not R2
    let subc_src = canvas.scene().to_circ1();
    assert!(subc_src.contains("mainComp=\"true\""), "{subc_src}");
    let reloaded = Scene::from_circ1(&subc_src).unwrap();
    assert!(reloaded.item_by_id(&r1_id).unwrap().main_comp);
    assert!(!reloaded.item_by_id(&r2_id).unwrap().main_comp);

    // Exposed items query returns only R1
    let exposed = subcircuit::exposed_items(&subc_src);
    assert_eq!(exposed.len(), 1);
    assert_eq!(exposed[0].circ_id, r1_id);

    // Now place a voltage-divider subcircuit with an exposed resistor into a parent canvas
    // and edit that exposed resistor's resistance through the Subcircuit's exposed property!
    let divider_subc = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="VDIV" Width="4" Height="3" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in; label=in&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=out; label=out&#xa;Pin; type=; xpos=16; ypos=32; angle=270; length=8; space=0; id=gnd; label=gnd&#xa;" />
<item itemtype="Tunnel" CircId="T-in" Name="in" />
<item itemtype="Tunnel" CircId="T-out" Name="out" />
<item itemtype="Tunnel" CircId="T-gnd" Name="gnd" />
<item itemtype="Resistor" CircId="Rtop-1" label="Top Resistor" mainComp="true" Resistance="1 kΩ" />
<item itemtype="Resistor" CircId="Rbot-1" Resistance="1 kΩ" />
<item itemtype="Node" CircId="N-1" />
<item itemtype="Connector" uid="c1" startpinid="T-in-pin" endpinid="Rtop-1-lPin" pointList="0,0,8,0" />
<item itemtype="Connector" uid="c2" startpinid="Rtop-1-rPin" endpinid="N-1-0" pointList="8,0,16,0" />
<item itemtype="Connector" uid="c3" startpinid="N-1-1" endpinid="T-out-pin" pointList="16,0,24,0" />
<item itemtype="Connector" uid="c4" startpinid="N-1-2" endpinid="Rbot-1-lPin" pointList="16,0,16,8" />
<item itemtype="Connector" uid="c5" startpinid="Rbot-1-rPin" endpinid="T-gnd-pin" pointList="16,8,16,16" />
</circuit>"#;

    let search = SubcSearch::empty().with_memory("vdiv_exp", divider_subc);
    let mut parent = Canvas::empty();
    let v_id = parent.scene_mut().add_fixed_volt(0.0, 0.0, 10.0);
    let g_id = parent.scene_mut().add_ground(0.0, 80.0);
    let subc_id = parent
        .scene_mut()
        .add_subcircuit("vdiv_exp", 80.0, 0.0, &search)
        .unwrap();
    wire(
        parent.scene_mut(),
        &format!("{v_id}-outnod"),
        &format!("{subc_id}-in"),
    );
    wire(
        parent.scene_mut(),
        &format!("{g_id}-Gnd"),
        &format!("{subc_id}-gnd"),
    );

    parent.power_on();
    // 1k / (1k + 1k) * 10V = 5.0V
    let v_half = parent.pin_voltage(&format!("{subc_id}-out")).unwrap();
    assert!((v_half - 5.0).abs() < 1e-3, "expected 5.0V, got {v_half}");

    // Edit exposed Rtop-1 Resistance to 3 kΩ -> 1k / (3k + 1k) * 10V = 2.5V
    parent.set_prop_text_for(&subc_id, "exposed:Rtop-1:Resistance".into(), "3 kΩ".into());
    let v_quarter = parent.pin_voltage(&format!("{subc_id}-out")).unwrap();
    assert!(
        (v_quarter - 2.5).abs() < 1e-3,
        "expected 2.5V after editing exposed resistor, got {v_quarter}"
    );
}

#[test]
fn exposed_mcu_takes_priority_inside_subcircuit() {
    let board = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="BOARD" Width="4" Height="4" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=P1; label=P1&#xa;" />
<item itemtype="MCU" CircId="tiny85-1" label="AuxMCU" Program="aux.hex" />
<item itemtype="MCU" CircId="mega328-2" label="MainMCU" mainComp="true" Program="main.hex" />
</circuit>"#;

    assert_eq!(
        subcircuit::nested_mcu_program(board).as_deref(),
        Some("main.hex")
    );
    let updated = subcircuit::set_nested_mcu_program(board, None, "blink.hex");
    assert_eq!(
        subcircuit::nested_mcu_program(&updated).as_deref(),
        Some("blink.hex")
    );
    assert!(updated.contains("CircId=\"tiny85-1\" label=\"AuxMCU\" Program=\"aux.hex\""));

    let search = SubcSearch::empty().with_memory("dual_board", board);
    let mut canvas = Canvas::empty();
    let subc_id = canvas
        .scene_mut()
        .add_subcircuit("dual_board", 0.0, 0.0, &search)
        .unwrap();
    let subc_idx = canvas
        .scene()
        .items()
        .iter()
        .position(|it| it.id == subc_id)
        .unwrap();
    canvas.scene_mut().select_only(subc_idx);
    let devices = canvas.collect_programmable_devices(None);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, format!("{subc_id}/mega328-2"));
}

fn temp_test_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cs_exposed_{tag}_{}_{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn exposed_props_save_reload_modified_undo_redo_and_copy_paste() {
    let dir = temp_test_dir("persist");
    let divider_subc = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="VDIV" Width="4" Height="3" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in; label=in&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=out; label=out&#xa;Pin; type=; xpos=16; ypos=32; angle=270; length=8; space=0; id=gnd; label=gnd&#xa;" />
<item itemtype="Tunnel" CircId="T-in" Name="in" />
<item itemtype="Tunnel" CircId="T-out" Name="out" />
<item itemtype="Tunnel" CircId="T-gnd" Name="gnd" />
<item itemtype="Resistor" CircId="Rtop-1" label="Top Resistor" mainComp="true" Resistance="1 kΩ" />
<item itemtype="Resistor" CircId="Rbot-1" Resistance="1 kΩ" />
<item itemtype="Node" CircId="N-1" />
<item itemtype="Connector" CircId="c1" startpinid="T-in-pin" endpinid="Rtop-1-lPin" pointList="0,0,8,0" />
<item itemtype="Connector" CircId="c2" startpinid="Rtop-1-rPin" endpinid="N-1-0" pointList="8,0,16,0" />
<item itemtype="Connector" CircId="c3" startpinid="N-1-1" endpinid="T-out-pin" pointList="16,0,24,0" />
<item itemtype="Connector" CircId="c4" startpinid="N-1-2" endpinid="Rbot-1-lPin" pointList="16,0,16,8" />
<item itemtype="Connector" CircId="c5" startpinid="Rbot-1-rPin" endpinid="T-gnd-pin" pointList="16,8,16,16" />
</circuit>"#;
    fs::write(dir.join("vdiv_disk.circ1"), divider_subc).unwrap();
    let parent_path = dir.join("parent.circ1");

    let mut parent = Canvas::empty();
    parent.set_file_path(Some(parent_path.to_string_lossy().into_owned()));
    let search = parent.subc_search();
    let v_id = parent.scene_mut().add_fixed_volt(0.0, 0.0, 10.0);
    let g_id = parent.scene_mut().add_ground(0.0, 80.0);
    let subc_id = parent
        .scene_mut()
        .add_subcircuit("vdiv_disk", 80.0, 0.0, &search)
        .unwrap();
    wire(
        parent.scene_mut(),
        &format!("{v_id}-outnod"),
        &format!("{subc_id}-in"),
    );
    wire(
        parent.scene_mut(),
        &format!("{g_id}-Gnd"),
        &format!("{subc_id}-gnd"),
    );
    parent.mark_saved();
    assert!(!parent.is_modified());

    parent.power_on();
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 5.0).abs() < 1e-3);

    // First edit: 1k -> 3k => 2.5V, canvas becomes modified.
    parent.set_prop_text_for(&subc_id, "exposed:Rtop-1:Resistance".into(), "3 k".into());
    assert!(
        parent.is_modified(),
        "editing exposed prop must mark circuit modified"
    );
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 2.5).abs() < 1e-3);

    // Second edit: 3k -> 9k => 1.0V.
    parent.set_prop_text_for(&subc_id, "exposed:Rtop-1:Resistance".into(), "9 k".into());
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 1.0).abs() < 1e-3);

    // Undo once: restores 3k (2.5V), NOT the disk default (5.0V).
    parent.undo();
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 2.5).abs() < 1e-3);
    assert!(parent.is_modified());

    // Undo twice: restores 1k (5.0V) and clears modified.
    parent.undo();
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 5.0).abs() < 1e-3);
    assert!(!parent.is_modified());

    // Redo twice: restores 3k then 9k!
    parent.redo();
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 2.5).abs() < 1e-3);
    parent.redo();
    assert!((parent.pin_voltage(&format!("{subc_id}-out")).unwrap() - 1.0).abs() < 1e-3);

    // Save to disk and reload in a fresh Canvas.
    let saved = parent.to_circ1();
    assert!(
        saved.contains("ExposedProps="),
        "expected ExposedProps in saved XML:\n{saved}"
    );
    fs::write(&parent_path, &saved).unwrap();

    let mut reloaded = Canvas::empty();
    reloaded
        .load_circ1(&saved, Some(parent_path.to_string_lossy().into_owned()))
        .unwrap();
    assert!(!reloaded.is_modified());
    let reloaded_exposed = match &reloaded.scene().item_by_id(&subc_id).unwrap().kind {
        Part::Subcircuit(s) => subcircuit::exposed_items(&s.nested_src),
        _ => panic!("expected Subcircuit"),
    };
    assert_eq!(
        reloaded_exposed[0]
            .part
            .get_prop_text("Resistance")
            .as_deref(),
        Some("9 kΩ")
    );
    reloaded.power_on();
    assert!((reloaded.pin_voltage(&format!("{subc_id}-out")).unwrap() - 1.0).abs() < 1e-3);

    // Copy/paste preserves exposed overrides.
    let subc_idx = reloaded
        .scene()
        .items()
        .iter()
        .position(|it| it.id == subc_id)
        .unwrap();
    reloaded.scene_mut().select_only(subc_idx);
    let clip = reloaded.scene().selection_to_circ1().unwrap();
    let paste_search = reloaded.subc_search();
    reloaded
        .scene_mut()
        .paste_circ1_with(&clip, Point::new(80.0, 0.0), &paste_search)
        .unwrap();
    let pasted = reloaded
        .scene()
        .items()
        .iter()
        .find(|it| it.id != subc_id && matches!(it.kind, Part::Subcircuit(_)))
        .expect("pasted subcircuit");
    if let Part::Subcircuit(s) = &pasted.kind {
        let exp = subcircuit::exposed_items(&s.nested_src);
        assert_eq!(
            exp[0].part.get_prop_text("Resistance").as_deref(),
            Some("9 kΩ")
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exposed_circ1_canonical_props_apply_to_simulation_and_persist() {
    let dir = temp_test_dir("canonical");
    let subc_xml = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="GATE" Width="4" Height="4" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=a; label=a&#xa;Pin; type=; xpos=-8; ypos=16; angle=180; length=8; space=0; id=b; label=b&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=y; label=y&#xa;Pin; type=; xpos=-8; ypos=24; angle=180; length=8; space=0; id=p1; label=p1&#xa;Pin; type=; xpos=40; ypos=24; angle=0; length=8; space=0; id=p2; label=p2&#xa;Pin; type=; xpos=40; ypos=32; angle=0; length=8; space=0; id=clk; label=clk&#xa;" />
<item itemtype="Tunnel" CircId="T-a" Name="a" />
<item itemtype="Tunnel" CircId="T-b" Name="b" />
<item itemtype="Tunnel" CircId="T-y" Name="y" />
<item itemtype="Tunnel" CircId="T-p1" Name="p1" />
<item itemtype="Tunnel" CircId="T-p2" Name="p2" />
<item itemtype="Tunnel" CircId="T-clk" Name="clk" />
<item itemtype="AndGate" CircId="AndGate-1" NumInputs="2" InvertInputs="false" InvertOutputs="false" InitHigh="false" OpenCollector="false" Tristate="false" Small="false" mainComp="true" />
<item itemtype="Push" CircId="Push-1" NormClose="false" Poles="1" Key="" ShowButton="false" mainComp="true" />
<item itemtype="Clock" CircId="Clock-1" Voltage="5 V" Frequency="1 kHz" Small="false" mainComp="true" />
<item itemtype="Connector" CircId="c1" startpinid="T-a-pin" endpinid="AndGate-1-in0" pointList="0,0,8,0" />
<item itemtype="Connector" CircId="c2" startpinid="T-b-pin" endpinid="AndGate-1-in1" pointList="0,8,8,8" />
<item itemtype="Connector" CircId="c3" startpinid="AndGate-1-out" endpinid="T-y-pin" pointList="16,0,24,0" />
<item itemtype="Connector" CircId="c4" startpinid="T-p1-pin" endpinid="Push-1-lPin0" pointList="0,16,8,16" />
<item itemtype="Connector" CircId="c5" startpinid="Push-1-rPin0" endpinid="T-p2-pin" pointList="16,16,24,16" />
<item itemtype="Connector" CircId="c6" startpinid="Clock-1-outnod" endpinid="T-clk-pin" pointList="16,24,24,24" />
</circuit>"#;
    fs::write(dir.join("gate_exp.circ1"), subc_xml).unwrap();
    let parent_path = dir.join("parent.circ1");

    let mut parent = Canvas::empty();
    parent.set_file_path(Some(parent_path.to_string_lossy().into_owned()));
    let search = parent.subc_search();
    let va = parent.scene_mut().add_fixed_volt(0.0, 0.0, 5.0);
    let vb = parent.scene_mut().add_fixed_volt(0.0, 40.0, 5.0);
    let r_load = parent.scene_mut().add_resistor(200.0, 40.0, 1000.0);
    let gnd = parent.scene_mut().add_ground(200.0, 80.0);
    let subc_id = parent
        .scene_mut()
        .add_subcircuit("gate_exp", 80.0, 0.0, &search)
        .unwrap();
    wire(
        parent.scene_mut(),
        &format!("{va}-outnod"),
        &format!("{subc_id}-a"),
    );
    wire(
        parent.scene_mut(),
        &format!("{vb}-outnod"),
        &format!("{subc_id}-b"),
    );
    wire(
        parent.scene_mut(),
        &format!("{va}-outnod"),
        &format!("{subc_id}-p1"),
    );
    wire(
        parent.scene_mut(),
        &format!("{subc_id}-p2"),
        &format!("{r_load}-lPin"),
    );
    wire(
        parent.scene_mut(),
        &format!("{r_load}-rPin"),
        &format!("{gnd}-Gnd"),
    );

    parent.power_on();
    // Before edits: AND(1, 1) = 5V; Push open => p2 = 0V.
    assert!(parent.pin_voltage(&format!("{subc_id}-y")).unwrap() > 4.0);
    assert!(parent.pin_voltage(&format!("{subc_id}-p2")).unwrap() < 0.5);

    // Edit canonical .circ1 props that legacy parse_sim1 previously ignored!
    parent.set_prop_bool_for(&subc_id, "exposed:AndGate-1:InvertOutputs".into(), true);
    parent.set_prop_bool_for(&subc_id, "exposed:Push-1:NormClose".into(), true);
    parent.set_prop_text_for(&subc_id, "exposed:Clock-1:Frequency".into(), "5 kHz".into());

    // After edits: NAND(1, 1) = 0V; Push NormClose => p2 = 5V; Clock freq_khz = 5.0!
    assert!(
        parent.pin_voltage(&format!("{subc_id}-y")).unwrap() < 0.5,
        "expected NAND output low after setting InvertOutputs=true"
    );
    assert!(
        parent.pin_voltage(&format!("{subc_id}-p2")).unwrap() > 4.5,
        "expected Push to conduct after setting NormClose=true"
    );
    let circ = parent.scene().to_circuit();
    let clk_freq = circ.components().iter().find_map(|c| match &c.kind {
        Kind::Clock { freq_khz, .. } => Some(*freq_khz),
        _ => None,
    });
    assert_eq!(clk_freq, Some(5.0));

    // Save & reload preserves all three canonical prop edits in simulation!
    let saved = parent.to_circ1();
    let mut reloaded = Canvas::empty();
    reloaded
        .load_circ1(&saved, Some(parent_path.to_string_lossy().into_owned()))
        .unwrap();
    reloaded.power_on();
    assert!(reloaded.pin_voltage(&format!("{subc_id}-y")).unwrap() < 0.5);
    assert!(reloaded.pin_voltage(&format!("{subc_id}-p2")).unwrap() > 4.5);
    let reloaded_clk = reloaded
        .scene()
        .to_circuit()
        .components()
        .iter()
        .find_map(|c| match &c.kind {
            Kind::Clock { freq_khz, .. } => Some(*freq_khz),
            _ => None,
        });
    assert_eq!(reloaded_clk, Some(5.0));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exposed_overrides_nested_subcircuit_coupled_prop_and_legacy_uid() {
    // 1. Round-trip special characters in ExposedProps encoding.
    let mut map = BTreeMap::new();
    map.insert(
        ("R;1".to_string(), "Note=1".to_string()),
        "\"100%\" a;b=c&d\nline2 Ω µ".to_string(),
    );
    let encoded = subcircuit::encode_exposed_overrides(&map);
    assert!(!encoded.contains('"'));
    assert!(!encoded.contains('\n'));
    assert_eq!(subcircuit::decode_exposed_overrides(&encoded), map);

    // 2. Legacy .sim1 item identified only by `uid` (no `CircId`) can be updated.
    let legacy_uid_src = r#"<circuit version="1.1.0-RC1">
<item itemtype="Resistor" uid="R-legacy" mainComp="true" Resistance="1 kΩ" />
</circuit>"#;
    let upd =
        subcircuit::apply_exposed_item_prop(legacy_uid_src, "R-legacy", "Resistance", "4.7 k")
            .expect("should update legacy uid item");
    assert!(upd.updated_src.contains("Resistance=\"4.7 kΩ\""));

    // 3. Coupled PropUpdate (Diode::Threshold -> SatCurrent) persists via SatCurrent.
    let diode_subc = r#"<circuit version="1.0.0">
<item itemtype="Diode" CircId="D-1" mainComp="true" Resistance="0.6 Ω" SatCurrent="10 nA" EmCoef="1" />
</circuit>"#;
    let upd = subcircuit::apply_exposed_item_prop(diode_subc, "D-1", "Threshold", "0.3 V")
        .expect("should apply Diode::Threshold");
    assert!(
        upd.persisted_changes.iter().any(|(k, _)| k == "SatCurrent"),
        "expected SatCurrent in persisted_changes, got {:?}",
        upd.persisted_changes
    );
    let exp = subcircuit::exposed_items(&upd.updated_src);
    let th = exp[0].part.get_prop_text("Threshold").unwrap();
    assert!(
        th.starts_with("300 mV") || th.starts_with("0.3"),
        "got {th}"
    );

    // 4. Nested subcircuit preserves ExposedProps on inner Subcircuit when flattened.
    let inner_vdiv = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="INNER" Width="4" Height="3" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in; label=in&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=out; label=out&#xa;Pin; type=; xpos=16; ypos=32; angle=270; length=8; space=0; id=gnd; label=gnd&#xa;" />
<item itemtype="Tunnel" CircId="T-in" Name="in" />
<item itemtype="Tunnel" CircId="T-out" Name="out" />
<item itemtype="Tunnel" CircId="T-gnd" Name="gnd" />
<item itemtype="Resistor" CircId="Rtop-1" mainComp="true" Resistance="1 kΩ" />
<item itemtype="Resistor" CircId="Rbot-1" Resistance="1 kΩ" />
<item itemtype="Node" CircId="N-1" />
<item itemtype="Connector" CircId="c1" startpinid="T-in-pin" endpinid="Rtop-1-lPin" pointList="0,0,8,0" />
<item itemtype="Connector" CircId="c2" startpinid="Rtop-1-rPin" endpinid="N-1-0" pointList="8,0,16,0" />
<item itemtype="Connector" CircId="c3" startpinid="N-1-1" endpinid="T-out-pin" pointList="16,0,24,0" />
<item itemtype="Connector" CircId="c4" startpinid="N-1-2" endpinid="Rbot-1-lPin" pointList="16,0,16,8" />
<item itemtype="Connector" CircId="c5" startpinid="Rbot-1-rPin" endpinid="T-gnd-pin" pointList="16,8,16,16" />
</circuit>"#;
    let outer_subc = r#"<circuit version="1.0.0">
<item itemtype="SubPackage" CircId="Package-1" Name="OUTER" Width="4" Height="3" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in; label=in&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=out; label=out&#xa;Pin; type=; xpos=16; ypos=32; angle=270; length=8; space=0; id=gnd; label=gnd&#xa;" />
<item itemtype="Tunnel" CircId="T-in" Name="in" />
<item itemtype="Tunnel" CircId="T-out" Name="out" />
<item itemtype="Tunnel" CircId="T-gnd" Name="gnd" />
<item itemtype="Subcircuit" CircId="inner_vdiv-1" Device="inner_vdiv" ExposedProps="Rtop-1;Resistance=3 kΩ" />
<item itemtype="Connector" CircId="c1" startpinid="T-in-pin" endpinid="inner_vdiv-1-in" pointList="0,0,8,0" />
<item itemtype="Connector" CircId="c2" startpinid="inner_vdiv-1-out" endpinid="T-out-pin" pointList="8,0,16,0" />
<item itemtype="Connector" CircId="c3" startpinid="inner_vdiv-1-gnd" endpinid="T-gnd-pin" pointList="16,0,16,8" />
</circuit>"#;

    let search = SubcSearch::empty()
        .with_memory("inner_vdiv", inner_vdiv)
        .with_memory("outer_vdiv", outer_subc);
    let inst = subcircuit::instantiate(
        "outer_vdiv-1",
        "outer_vdiv",
        outer_subc,
        &search,
        false,
        None,
        0,
        None,
    )
    .unwrap();
    let mut c = Circuit::new();
    for comp in inst.components {
        c.add_comp(comp);
    }
    for (a, b) in inst.connectors {
        c.connect(a, b);
    }
    c.add_fixed_volt("V-1", 10.0);
    c.add_ground("G-1");
    c.connect("V-1-outnod", "outer_vdiv-1-in");
    c.connect("G-1-Gnd", "outer_vdiv-1-gnd");
    c.solve().unwrap();
    let v_out = c.pin_voltage("outer_vdiv-1-out").unwrap();
    assert!(
        (v_out - 2.5).abs() < 1e-3,
        "expected 2.5V from nested subcircuit ExposedProps override, got {v_out}"
    );
}
