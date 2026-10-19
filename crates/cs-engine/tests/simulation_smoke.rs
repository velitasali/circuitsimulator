//! Scene-level logic drive and canvas readings, plus relay and controlled source.

use cs_engine::Circuit;
use cs_engine::canvas::Canvas;
use cs_engine::elements::{Comp, Kind};

fn join(canvas: &mut Canvas, a: &str, b: &str) {
    let ap = canvas.scene().pin_scene(a).unwrap_or_else(|| panic!("{a}"));
    let bp = canvas.scene().pin_scene(b).unwrap_or_else(|| panic!("{b}"));
    canvas.scene_mut().connect_pins(a, ap, b, bp);
}

#[test]
fn and_gate_drives_a_load_through_the_canvas() {
    let mut canvas = Canvas::empty();
    let gnd = canvas.scene_mut().add_ground(0.0, 80.0);
    let va = canvas.scene_mut().add_fixed_volt(-40.0, -20.0, 5.0);
    let vb = canvas.scene_mut().add_fixed_volt(-40.0, 20.0, 5.0);
    let gate = canvas.scene_mut().add_and_gate(0.0, 0.0);
    let load = canvas.scene_mut().add_resistor(80.0, 0.0, 10_000.0);
    join(&mut canvas, &format!("{va}-outnod"), &format!("{gate}-in0"));
    join(&mut canvas, &format!("{vb}-outnod"), &format!("{gate}-in1"));
    join(&mut canvas, &format!("{gate}-out"), &format!("{load}-lPin"));
    join(&mut canvas, &format!("{load}-rPin"), &format!("{gnd}-Gnd"));
    canvas.power_on();
    let out = canvas
        .pin_voltage(&format!("{gate}-out"))
        .unwrap_or_else(|| panic!("gate output has no voltage"));
    assert!(out > 4.0, "AND output should be high, got {out}");
}

#[test]
fn relay_contact_closes_from_coil_current() {
    let mut circuit = Circuit::new();
    circuit
        .add_fixed_volt("VC", 5.0)
        .add_fixed_volt("VS", 5.0)
        .add_comp(Comp {
            id: "RY".into(),
            kind: Kind::Relay {
                norm_close: false,
                double_throw: false,
                poles: 1,
                i_on: 0.02,
                i_off: 0.01,
                r_coil: 100.0,
                inductance: 0.0,
                coil_i: 0.0,
                active: false,
            },
        })
        .add_resistor("RL", 1_000.0)
        .add_ground("GND")
        .connect("VC-outnod", "RY-lPin")
        .connect("RY-rPin", "GND-Gnd")
        .connect("VS-outnod", "RY-c1")
        .connect("RY-c2", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    circuit.solve().unwrap();
    let closed = circuit.pin_voltage("RL-lPin").unwrap();
    assert!(closed > 4.5, "contact should close, got {closed}");
}

#[test]
fn controlled_source_gain_sets_the_output() {
    let mut circuit = Circuit::new();
    circuit
        .add_fixed_volt("VCTRL", 1.5)
        .add_comp(Comp {
            id: "CS".into(),
            kind: Kind::Csource {
                control_pins: true,
                curr_source: false,
                curr_control: false,
                gain: 2.0,
                volt: 9.0,
                current: 1.0,
                ctrl: 0.0,
                sense_ohms: 1.0,
            },
        })
        .add_resistor("RL", 1_000.0)
        .add_ground("GND")
        .connect("VCTRL-outnod", "CS-cpPin")
        .connect("CS-cmPin", "GND-Gnd")
        .connect("CS-s2Pin", "GND-Gnd")
        .connect("CS-s1Pin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    circuit.solve().unwrap();
    let v = circuit.pin_voltage("CS-s1Pin").unwrap();
    assert!((v - 3.0).abs() < 1e-3, "vcvs out {v}");
}

#[test]
fn pcd8544_framebuffer_is_published_to_canvas_readings() {
    let mut canvas = Canvas::empty();
    let lcd = canvas.scene_mut().add_pcd8544(0.0, 0.0);
    let rst = canvas.scene_mut().add_fixed_volt(-40.0, 0.0, 5.0);
    join(
        &mut canvas,
        &format!("{rst}-outnod"),
        &format!("{lcd}-PinRst"),
    );
    canvas.power_on();
    {
        let circuit = canvas.running_circuit_mut().expect("running");
        match &mut circuit.comp_mut(&lcd).expect("lcd").kind {
            Kind::Pcd8544(panel) => {
                panel.pd = false;
                panel.d_bit = true;
                panel.ram[0][0] = 0xA5;
            }
            other => panic!("expected PCD8544, got {other:?}"),
        }
    }
    canvas.tick();
    let text = canvas
        .readings()
        .get(&lcd)
        .map(|reading| reading.text.clone())
        .unwrap_or_default();
    assert!(
        text.contains("A5"),
        "canvas reading should publish the framebuffer, got {text}"
    );
}
