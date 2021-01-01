//! Tests for passive component properties, pin geometries, and circuit view rendering.

use super::recorder::{DrawOp, record_drawable_paint, record_part_paint};
use crate::canvas::draw::Color;
use crate::components::resistor::{Resistor, band_color, bands_of};
use crate::components::resistor_dip::ResistorDip;
use crate::components::*;
use crate::matrix::CircMatrix;

#[test]
fn test_resistor_bands_and_rendering() {
    let mut r = Resistor::default();
    r.resistance = 1000.0; // 1.0 kΩ: Brown (1), Black (0), Red (2), None (-1)
    r.show_bands = true;

    let bands = bands_of(r.resistance);
    assert_eq!(bands, [1, 0, 2, -1]);
    assert_eq!(band_color(1), crate::canvas::draw::parse_hex("#8b4513")); // Brown
    assert_eq!(band_color(0), Color::rgb(0, 0, 0)); // Black
    assert_eq!(band_color(2), Color::rgb(255, 0, 0)); // Red

    let rec_with_bands = record_drawable_paint(&r);
    // Should have 1 body fill_rect, 4 band fill_rects, and 1 stroke_rect border
    let fill_rects: Vec<_> = rec_with_bands
        .ops
        .iter()
        .filter(|op| matches!(op, DrawOp::FillRect { .. }))
        .collect();
    assert_eq!(
        fill_rects.len(),
        5,
        "Expected 1 body + 4 bands fill_rect ops"
    );

    // Toggle show_bands = false
    r.set_prop_text("ShowBands", "false").unwrap();
    assert!(!r.show_bands);
    let rec_no_bands = record_drawable_paint(&r);
    let fill_rects_no_bands: Vec<_> = rec_no_bands
        .ops
        .iter()
        .filter(|op| matches!(op, DrawOp::FillRect { .. }))
        .collect();
    assert_eq!(
        fill_rects_no_bands.len(),
        1,
        "Expected only 1 body fill_rect when ShowBands=false"
    );

    // Test 4.7 kΩ: Yellow (4), Violet (7), Red (2), None (-1)
    r.set_prop_text("Resistance", "4.7 kΩ").unwrap();
    r.set_prop_text("ShowBands", "true").unwrap();
    let bands_4k7 = bands_of(r.resistance);
    assert_eq!(bands_4k7, [4, 7, 2, -1]);
}

#[test]
fn test_resistor_matrix_stamping() {
    let r = Resistor::new(250.0);
    let mut matrix = CircMatrix::new(2);
    matrix.analyze(&[vec![0, 1], vec![1, 0]]);
    r.stamp(&mut matrix, &[0, 1], 0.0);

    // Conductance g = 1 / 250 = 0.004 S
    let mut v = vec![0.0, 0.0];
    matrix.add_coef(0, 1.0); // 1A injected at node 0
    matrix.add_matrix(1, 1, 1e9); // Ground node 1
    assert!(matrix.solve(&mut v));
    assert!(
        (v[0] - 250.0).abs() < 1e-4,
        "Expected 250V drop across 250 ohm resistor at 1A"
    );
}

#[test]
fn test_var_resistor_dial_and_props() {
    let mut vr = VarResistor::default();
    vr.set_prop_text("MinResistance", "100 Ω").unwrap();
    vr.set_prop_text("MaxResistance", "1 kΩ").unwrap();
    vr.set_prop_text("Resistance", "550 Ω").unwrap();

    assert_eq!(vr.min_r, 100.0);
    assert_eq!(vr.max_r, 1000.0);
    assert_eq!(vr.resistance, 550.0);
    assert!((vr.wiper() - 0.5).abs() < 1e-6);

    let rec = record_part_paint(&Part::VarResistor(vr));
    assert!(rec.is_all_finite());
    assert!(rec.ops.iter().any(|op| matches!(op, DrawOp::Arc { .. })));
    assert!(rec.ops.iter().any(|op| matches!(op, DrawOp::Line { .. })));
}

#[test]
fn test_potentiometer_pins_and_wiper() {
    let mut pot = Potentiometer::default();
    pot.set_prop_text("Wiper", "0.75").unwrap();
    assert_eq!(pot.wiper, 0.75);

    let pins = pot.pin_geoms();
    assert_eq!(pins.len(), 3, "Potentiometer must have 3 pins");
    assert_eq!(pins[0].suffix, "-lPin");
    assert_eq!(pins[1].suffix, "-rPin");
    assert_eq!(pins[2].suffix, "-wPin");

    let rec = record_part_paint(&Part::Potentiometer(pot));
    assert!(rec.is_all_finite());
}

#[test]
fn test_resistor_dip_bussed_and_isolated_modes() {
    // 1. Isolated 4-resistor DIP
    let mut dip = ResistorDip::default();
    dip.set_prop_text("Size", "4").unwrap();
    dip.set_prop_text("Bussed", "false").unwrap();

    let pins_iso = dip.pin_geoms();
    assert_eq!(
        pins_iso.len(),
        8,
        "4-resistor isolated DIP must have 8 pins (2 per resistor)"
    );

    let rec_iso = record_part_paint(&Part::ResistorDip(dip.clone()));
    assert!(rec_iso.is_all_finite());

    // 2. Bussed 8-resistor DIP
    dip.set_prop_text("Size", "8").unwrap();
    dip.set_prop_text("Bussed", "true").unwrap();

    let pins_bussed = dip.pin_geoms();
    assert_eq!(
        pins_bussed.len(),
        9,
        "8-resistor bussed DIP must have 9 pins (8 + 1 common)"
    );
    assert_eq!(
        pins_bussed[0].suffix, "-com",
        "Pin 0 in bussed mode must be common pin"
    );

    let rec_bussed = record_part_paint(&Part::ResistorDip(dip));
    assert!(rec_bussed.is_all_finite());
}

#[test]
fn test_capacitors_and_inductors_geometry() {
    let cap = Capacitor::default();
    assert_eq!(cap.pin_geoms().len(), 2);
    let rec_cap = record_part_paint(&Part::Capacitor(cap));
    assert!(rec_cap.is_all_finite());

    let el_cap = ElCapacitor::default();
    assert_eq!(el_cap.pin_geoms().len(), 2);
    let rec_el = record_part_paint(&Part::ElCapacitor(el_cap));
    assert!(rec_el.is_all_finite());

    let ind = Inductor::default();
    assert_eq!(ind.pin_geoms().len(), 2);
    let rec_ind = record_part_paint(&Part::Inductor(ind));
    assert!(rec_ind.is_all_finite());
}

#[test]
fn test_transformer_pins_and_props() {
    let mut tr = Transformer::default();
    tr.set_prop_text("Inductance1", "2.5 mH").unwrap();
    assert_eq!(tr.inductance1, 2.5e-3);

    let pins = tr.pin_geoms();
    assert_eq!(
        pins.len(),
        4,
        "Transformer must have 4 pins (p1, p2, s1, s2)"
    );
    assert_eq!(pins[0].suffix, "-p1");
    assert_eq!(pins[1].suffix, "-p2");
    assert_eq!(pins[2].suffix, "-s1");
    assert_eq!(pins[3].suffix, "-s2");

    let rec = record_part_paint(&Part::Transformer(tr));
    assert!(rec.is_all_finite());
}

#[test]
fn test_environmental_sensors_passive() {
    // LDR
    let mut ldr = Ldr::default();
    ldr.set_prop_text("Lux", "150").unwrap();
    assert_eq!(ldr.lux, 150.0);
    let rec_ldr = record_part_paint(&Part::Ldr(ldr));
    assert!(rec_ldr.is_all_finite());

    // Thermistor
    let mut th = Thermistor::default();
    th.set_prop_text("Temp", "35 °C").unwrap();
    assert_eq!(th.temp_c, 35.0);
    let rec_th = record_part_paint(&Part::Thermistor(th));
    assert!(rec_th.is_all_finite());

    // RTD
    let mut rtd = Rtd::default();
    rtd.set_prop_text("Temp", "100 °C").unwrap();
    assert_eq!(rtd.temp_c, 100.0);
    let rec_rtd = record_part_paint(&Part::Rtd(rtd));
    assert!(rec_rtd.is_all_finite());

    // Strain
    let mut strain = Strain::default();
    strain.set_prop_text("Strain", "500 µε").unwrap();
    assert_eq!(strain.strain, 500e-6);
    let rec_strain = record_part_paint(&Part::Strain(strain));
    assert!(rec_strain.is_all_finite());
}
