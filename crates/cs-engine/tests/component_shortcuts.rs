//! End-to-end verification of component keyboard shortcut (`Key` / `Key_Labels`)
//! support across `Push`, `Switch`, `KeyPad`, board `Subcircuit` children,
//! live simulation sync, undo/dirty semantics, and `Potentiometer`/`VarResistor`
//! cleanup.

use cs_engine::canvas::{Canvas, KEY_A, MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT, Part, Scene};
use cs_engine::components::{Component, Potentiometer, VarResistor};
use cs_engine::subcircuit::SubcSearch;

const KEY_SPACE: i32 = 0x20;
const KEY_1: i32 = 0x31;
const KEY_2: i32 = 0x32;
const KEY_5: i32 = 0x35;
const KEY_G: i32 = 0x47;
const KEY_P: i32 = 0x50;
const KEY_S: i32 = 0x53;
const KEY_T: i32 = 0x54;
const KEY_ASTERISK: i32 = 0x2a;
const KEY_HASH: i32 = 0x23;

fn wire(canvas: &mut Canvas, a_id: &str, a_pin: &str, b_id: &str, b_pin: &str) {
    let a = format!("{a_id}{a_pin}");
    let b = format!("{b_id}{b_pin}");
    let ap = canvas.scene().pin_scene(&a).expect("pin a exists");
    let bp = canvas.scene().pin_scene(&b).expect("pin b exists");
    canvas.scene_mut().connect_pins(&a, ap, &b, bp);
}

fn switch_checked(canvas: &Canvas, uid: &str) -> bool {
    let Part::Switch(sw) = &canvas.scene().item_by_id(uid).unwrap().kind else {
        panic!("expected Switch");
    };
    sw.checked
}

fn keypad_pressed(canvas: &Canvas, uid: &str) -> Option<(usize, usize)> {
    let Part::KeyPad(kp) = &canvas.scene().item_by_id(uid).unwrap().kind else {
        panic!("expected KeyPad");
    };
    kp.pressed
}

#[test]
fn push_shortcut_press_release_live_sim_and_focus_loss() {
    let mut canvas = Canvas::empty();
    let vcc = canvas.scene_mut().add_fixed_volt(0.0, 0.0, 5.0);
    let push_id = canvas.scene_mut().add_push(64.0, 0.0);
    let res_id = canvas.scene_mut().add_resistor(128.0, 0.0, 1_000.0);
    let gnd = canvas.scene_mut().add_ground(192.0, 0.0);

    canvas.set_prop_text_for(&push_id, "Key".into(), "A".into());

    wire(&mut canvas, &vcc, "-outnod", &push_id, "-lPin0");
    wire(&mut canvas, &push_id, "-rPin0", &res_id, "-lPin");
    wire(&mut canvas, &res_id, "-rPin", &gnd, "-Gnd");

    // Mark clean baseline after circuit setup.
    canvas.mark_saved();
    canvas.power_on();

    let out_pin = format!("{res_id}-lPin");
    assert!(canvas.pin_voltage(&out_pin).unwrap_or(-1.0) < 0.1);
    assert!(canvas.owns_key_shortcut(KEY_A, 0, "a"));
    assert!(canvas.owns_key_shortcut(KEY_A, MOD_SHIFT, "A"));
    assert!(!canvas.owns_key_shortcut(KEY_A, MOD_CTRL, "\x01"));
    assert!(!canvas.owns_key_shortcut(KEY_A, MOD_ALT, "a"));
    assert!(!canvas.owns_key_shortcut(KEY_A, MOD_META, "a"));
    assert!(!canvas.owns_key_shortcut(KEY_S, 0, "s"));

    // Lowercase 'a' activates Key="A".
    canvas.key_press_with_text(KEY_A, 0, "a", false);
    assert!(canvas.scene().item_by_id(&push_id).unwrap().pressed());
    assert!((canvas.pin_voltage(&out_pin).unwrap() - 5.0).abs() < 0.1);
    // Momentary push does not dirty document or push undo.
    assert!(!canvas.is_modified());

    // Auto-repeat press & release (Linux X11 style) must not drop pressed state.
    canvas.key_release(KEY_A, 0, "a", true);
    canvas.key_press_with_text(KEY_A, 0, "a", true);
    assert!(canvas.scene().item_by_id(&push_id).unwrap().pressed());
    assert!((canvas.pin_voltage(&out_pin).unwrap() - 5.0).abs() < 0.1);

    // Real key release opens the push button and updates sim.
    canvas.key_release(KEY_A, 0, "a", false);
    assert!(!canvas.scene().item_by_id(&push_id).unwrap().pressed());
    assert!(canvas.pin_voltage(&out_pin).unwrap() < 0.1);

    // Ctrl+A selects all instead of pressing Push.
    canvas.key_press_with_text(KEY_A, MOD_CTRL, "\x01", false);
    assert!(!canvas.scene().item_by_id(&push_id).unwrap().pressed());

    // Focus loss releases held push button.
    canvas.key_press_with_text(KEY_A, 0, "a", false);
    assert!(canvas.scene().item_by_id(&push_id).unwrap().pressed());
    canvas.release_all_keys();
    assert!(!canvas.scene().item_by_id(&push_id).unwrap().pressed());
    assert!(canvas.pin_voltage(&out_pin).unwrap() < 0.1);
}

#[test]
fn push_space_alias_and_literal_space_and_grid_override() {
    let mut canvas = Canvas::empty();
    let p_alias = canvas.scene_mut().add_push(0.0, 0.0);
    let p_lit = canvas.scene_mut().add_push(64.0, 0.0);
    let p_grid = canvas.scene_mut().add_push(128.0, 0.0);

    canvas.set_prop_text_for(&p_alias, "Key".into(), "Space".into());
    canvas.set_prop_text_for(&p_lit, "Key".into(), " ".into());
    canvas.set_prop_text_for(&p_grid, "Key".into(), "G".into());

    assert!(canvas.owns_key_shortcut(KEY_SPACE, 0, " "));
    assert!(canvas.owns_key_shortcut(KEY_G, 0, "g"));

    canvas.key_press_with_text(KEY_SPACE, 0, " ", false);
    assert!(canvas.scene().item_by_id(&p_alias).unwrap().pressed());
    assert!(canvas.scene().item_by_id(&p_lit).unwrap().pressed());
    assert!(!canvas.scene().item_by_id(&p_grid).unwrap().pressed());

    canvas.key_release(KEY_SPACE, 0, " ", false);
    assert!(!canvas.scene().item_by_id(&p_alias).unwrap().pressed());
    assert!(!canvas.scene().item_by_id(&p_lit).unwrap().pressed());

    // Pressing 'g' activates the Push bound to "G" (ShortcutOverride prevents Toggle Grid).
    canvas.key_press_with_text(KEY_G, 0, "g", false);
    assert!(canvas.scene().item_by_id(&p_grid).unwrap().pressed());
    canvas.key_release(KEY_G, 0, "g", false);
    assert!(!canvas.scene().item_by_id(&p_grid).unwrap().pressed());
}

#[test]
fn switch_shortcut_toggles_live_sim_ignores_repeat_and_undoes_as_one_step() {
    let mut canvas = Canvas::empty();
    let vcc = canvas.scene_mut().add_fixed_volt(0.0, 0.0, 5.0);
    let sw1 = canvas.scene_mut().add_switch(64.0, 0.0, false);
    let sw2 = canvas.scene_mut().add_switch(64.0, 64.0, true);
    let res = canvas.scene_mut().add_resistor(128.0, 0.0, 1_000.0);
    let gnd = canvas.scene_mut().add_ground(192.0, 0.0);

    canvas.set_prop_text_for(&sw1, "Key".into(), "S".into());
    canvas.set_prop_text_for(&sw2, "Key".into(), "s".into());

    wire(&mut canvas, &vcc, "-outnod", &sw1, "-pinP0");
    wire(&mut canvas, &sw1, "-switch0pinN", &res, "-lPin");
    wire(&mut canvas, &res, "-rPin", &gnd, "-Gnd");

    canvas.open_properties(&sw1);
    canvas.power_on();

    let out_pin = format!("{res}-lPin");
    assert!(canvas.pin_voltage(&out_pin).unwrap_or(-1.0) < 0.1);

    // Press 's' toggles both switches, refreshes open prop dialog, updates sim.
    let change = canvas.key_press_with_text(KEY_S, 0, "s", false);
    assert!(change.props);
    assert!(switch_checked(&canvas, &sw1));
    assert!(!switch_checked(&canvas, &sw2));
    assert!((canvas.pin_voltage(&out_pin).unwrap() - 5.0).abs() < 0.1);

    // Auto-repeat and key release must not double-toggle.
    canvas.key_press_with_text(KEY_S, 0, "s", true);
    canvas.key_release(KEY_S, 0, "s", false);
    assert!(switch_checked(&canvas, &sw1));
    assert!(!switch_checked(&canvas, &sw2));

    // Single undo restores both switches that toggled together.
    canvas.undo();
    assert!(!switch_checked(&canvas, &sw1));
    assert!(switch_checked(&canvas, &sw2));
    assert!(canvas.pin_voltage(&out_pin).unwrap() < 0.1);
}

#[test]
fn keypad_shortcut_press_rollover_release_and_live_sim() {
    let mut canvas = Canvas::empty();
    let vcc = canvas.scene_mut().add_fixed_volt(-64.0, 0.0, 5.0);
    let kp_id = canvas.scene_mut().add_keypad(0.0, 0.0);
    let res = canvas.scene_mut().add_resistor(96.0, 0.0, 1_000.0);
    let gnd = canvas.scene_mut().add_ground(160.0, 0.0);

    // Default 4x3 keypad: "123456789*0#".
    // Row 1 is Pin1 (keys 4, 5, 6); Col 1 is Pin5 (keys 2, 5, 8, 0).
    wire(&mut canvas, &vcc, "-outnod", &kp_id, "-Pin1");
    wire(&mut canvas, &kp_id, "-Pin5", &res, "-lPin");
    wire(&mut canvas, &res, "-rPin", &gnd, "-Gnd");

    canvas.power_on();
    let out_pin = format!("{res}-lPin");
    assert!(canvas.pin_voltage(&out_pin).unwrap_or(-1.0) < 0.1);

    assert!(canvas.owns_key_shortcut(KEY_5, 0, "5"));
    assert!(canvas.owns_key_shortcut(KEY_ASTERISK, MOD_SHIFT, "*"));
    assert!(canvas.owns_key_shortcut(KEY_HASH, MOD_SHIFT, "#"));
    assert!(!canvas.owns_key_shortcut(KEY_A, 0, "a"));

    // Press '5' -> closes Row 1 to Col 1 -> 5 V across resistor.
    canvas.key_press_with_text(KEY_5, 0, "5", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), Some((1, 1)));
    assert!((canvas.pin_voltage(&out_pin).unwrap() - 5.0).abs() < 0.1);

    // Release '5' -> opens connection.
    canvas.key_release(KEY_5, 0, "5", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), None);
    assert!(canvas.pin_voltage(&out_pin).unwrap() < 0.1);

    // Shifted '*' and '#' work too.
    canvas.key_press_with_text(KEY_ASTERISK, MOD_SHIFT, "*", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), Some((3, 0)));
    canvas.key_release(KEY_ASTERISK, MOD_SHIFT, "*", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), None);

    // Rollover: press '1', press '2', release '1' -> '2' stays pressed until '2' released.
    canvas.key_press_with_text(KEY_1, 0, "1", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), Some((0, 0)));
    canvas.key_press_with_text(KEY_2, 0, "2", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), Some((0, 1)));
    canvas.key_release(KEY_1, 0, "1", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), Some((0, 1)));
    canvas.release_all_keys();
    assert_eq!(keypad_pressed(&canvas, &kp_id), None);

    // 4-column keypad ("123A456B789C*0#D"): lowercase 'a' matches 'A' at (0, 3).
    canvas.set_prop_text_for(&kp_id, "Cols".into(), "4".into());
    assert!(canvas.owns_key_shortcut(KEY_A, 0, "a"));
    canvas.key_press_with_text(KEY_A, 0, "a", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), Some((0, 3)));
    canvas.key_release(KEY_A, 0, "a", false);
    assert_eq!(keypad_pressed(&canvas, &kp_id), None);
}

#[test]
fn board_subcircuit_exposed_push_and_switch_shortcuts_work_in_live_sim() {
    let subc_src = r#"<circuit version="1.0.0">
<item itemtype="Push" CircId="Push-1" Key="P" ShowButton="true" Pos="-120,-36" boardPos="16,16" />
<item itemtype="Switch" CircId="Switch-1" Key="T" Checked="false" ShowButton="true" Pos="-120,0" boardPos="16,40" />
<item itemtype="Tunnel" CircId="Tunnel-1" Name="in1" Pos="-160,-36" />
<item itemtype="Tunnel" CircId="Tunnel-2" Name="out1" Pos="-80,-36" />
<item itemtype="Tunnel" CircId="Tunnel-3" Name="in2" Pos="-160,0" />
<item itemtype="Tunnel" CircId="Tunnel-4" Name="out2" Pos="-80,0" />
<item itemtype="SubPackage" CircId="SubPackage-1" Name="BoardCtrl" Width="6" Height="7" SubcType="Board" LogicSymbol="false" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in1; label=in1&#xa;Pin; type=; xpos=56; ypos=8; angle=0; length=8; space=0; id=out1; label=out1&#xa;Pin; type=; xpos=-8; ypos=24; angle=180; length=8; space=0; id=in2; label=in2&#xa;Pin; type=; xpos=56; ypos=24; angle=0; length=8; space=0; id=out2; label=out2&#xa;" Pos="0,0" />
<item itemtype="Connector" CircId="Connector-1" startpinid="Tunnel-1-pin" endpinid="Push-1-lPin0" pointList="-160,-36,-136,-36" />
<item itemtype="Connector" CircId="Connector-2" startpinid="Push-1-rPin0" endpinid="Tunnel-2-pin" pointList="-104,-36,-80,-36" />
<item itemtype="Connector" CircId="Connector-3" startpinid="Tunnel-3-pin" endpinid="Switch-1-pinP0" pointList="-160,0,-136,0" />
<item itemtype="Connector" CircId="Connector-4" startpinid="Switch-1-switch0pinN" endpinid="Tunnel-4-pin" pointList="-104,0,-80,0" />
</circuit>"#;

    let search = SubcSearch::empty().with_memory("BoardCtrl", subc_src);
    let mut canvas = Canvas::empty();
    let vcc = canvas.scene_mut().add_fixed_volt(-96.0, 0.0, 5.0);
    let sub_id = canvas
        .scene_mut()
        .add_subcircuit("BoardCtrl", 0.0, 0.0, &search)
        .unwrap();
    let r1 = canvas.scene_mut().add_resistor(128.0, 0.0, 1_000.0);
    let r2 = canvas.scene_mut().add_resistor(128.0, 32.0, 1_000.0);
    let gnd = canvas.scene_mut().add_ground(192.0, 0.0);

    wire(&mut canvas, &vcc, "-outnod", &sub_id, "-in1");
    wire(&mut canvas, &sub_id, "-out1", &r1, "-lPin");
    wire(&mut canvas, &r1, "-rPin", &gnd, "-Gnd");

    wire(&mut canvas, &vcc, "-outnod", &sub_id, "-in2");
    wire(&mut canvas, &sub_id, "-out2", &r2, "-lPin");
    wire(&mut canvas, &r2, "-rPin", &gnd, "-Gnd");

    canvas.power_on();
    let out1 = format!("{r1}-lPin");
    let out2 = format!("{r2}-lPin");
    assert!(canvas.pin_voltage(&out1).unwrap_or(-1.0) < 0.1);
    assert!(canvas.pin_voltage(&out2).unwrap_or(-1.0) < 0.1);

    assert!(canvas.owns_key_shortcut(KEY_P, 0, "p"));
    assert!(canvas.owns_key_shortcut(KEY_T, 0, "t"));

    // Press 'p' activates exposed Push inside board subcircuit.
    canvas.key_press_with_text(KEY_P, 0, "p", false);
    assert!((canvas.pin_voltage(&out1).unwrap() - 5.0).abs() < 0.1);
    canvas.key_release(KEY_P, 0, "p", false);
    assert!(canvas.pin_voltage(&out1).unwrap() < 0.1);

    // Press 't' toggles exposed Switch inside board subcircuit.
    canvas.key_press_with_text(KEY_T, 0, "t", false);
    assert!((canvas.pin_voltage(&out2).unwrap() - 5.0).abs() < 0.1);
    canvas.key_release(KEY_T, 0, "t", false);
    assert!((canvas.pin_voltage(&out2).unwrap() - 5.0).abs() < 0.1);

    // Toggling again opens it.
    canvas.key_press_with_text(KEY_T, 0, "t", false);
    assert!(canvas.pin_voltage(&out2).unwrap() < 0.1);

    // When switched to Logic Symbol, board shortcuts are inactive.
    canvas.set_prop_bool_for(&sub_id, "LogicSymbol".into(), true);
    assert!(!canvas.owns_key_shortcut(KEY_P, 0, "p"));
    assert!(!canvas.owns_key_shortcut(KEY_T, 0, "t"));
}

#[test]
fn potentiometer_and_var_resistor_have_no_key_prop_and_ignore_legacy_key_attr() {
    assert!(!Potentiometer::props().iter().any(|p| p.id == "Key"));
    assert!(!VarResistor::props().iter().any(|p| p.id == "Key"));

    // Legacy `.circ1` containing Key="A" on Potentiometer/VarResistor still loads cleanly.
    let legacy = r#"<circuit version="1.0.0">
<item itemtype="Potentiometer" CircId="Potentiometer-1" Resistance="10 kΩ" Wiper="0.5" Key="A" DialStep="5 %" Pos="0,0" />
<item itemtype="VarResistor" CircId="VarResistor-1" Resistance="500 Ω" MinResistance="0 Ω" MaxResistance="1 kΩ" Key="B" DialStep="10 Ω" Pos="64,0" />
</circuit>"#;
    let scene = Scene::from_sim1(legacy).expect("legacy Key attribute should be ignored");
    assert!(matches!(
        scene.item_by_id("Potentiometer-1").unwrap().kind,
        Part::Potentiometer(_)
    ));
    assert!(matches!(
        scene.item_by_id("VarResistor-1").unwrap().kind,
        Part::VarResistor(_)
    ));
}
