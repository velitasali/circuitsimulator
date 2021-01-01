//! Tests for active & semiconductor component properties, pin geometries, and circuit view rendering.

use super::recorder::{DrawOp, record_part_paint};
use crate::components::bjt::Bjt;
use crate::components::diode::Diode;
use crate::components::*;
use crate::elements::Kind;

#[test]
fn test_diode_regular_vs_zener_rendering() {
    let mut d = Diode::default();
    assert!(!d.zener);

    // Regular diode draw ops
    let rec_reg = record_part_paint(&Part::Diode(d.clone()));
    assert!(rec_reg.is_all_finite());

    // Switch to Zener mode
    d.set_prop_text("Zener", "true").unwrap();
    assert!(d.zener);

    let rec_zener = record_part_paint(&Part::Diode(d.clone()));
    assert!(rec_zener.is_all_finite());

    // Zener diode has additional bent cathode ticks or different stroke lines than regular diode
    assert_ne!(
        rec_reg.ops, rec_zener.ops,
        "Diode regular vs Zener must produce different draw operations"
    );

    // Verify simulation kind
    assert!(matches!(
        d.to_element_kind(),
        Kind::Diode { zener: true, .. }
    ));
}

#[test]
fn test_bjt_npn_vs_pnp_geometry_and_drawing() {
    // NPN
    let mut bjt = Bjt::default();
    assert!(!bjt.pnp);
    let pins_npn = bjt.pin_geoms();
    assert_eq!(
        pins_npn.len(),
        3,
        "BJT must have 3 pins (Collector, Emitter, Base)"
    );
    assert_eq!(pins_npn[0].suffix, "-collector");
    assert_eq!(pins_npn[1].suffix, "-emiter");
    assert_eq!(pins_npn[2].suffix, "-base");

    let rec_npn = record_part_paint(&Part::Bjt(bjt.clone()));
    assert!(rec_npn.is_all_finite());

    // PNP
    bjt.set_prop_text("PNP", "true").unwrap();
    assert!(bjt.pnp);

    let rec_pnp = record_part_paint(&Part::Bjt(bjt.clone()));
    assert!(rec_pnp.is_all_finite());
    assert_ne!(
        rec_npn.ops, rec_pnp.ops,
        "NPN and PNP must have distinct emitter arrow rendering"
    );

    // Check Gain and Threshold prop setting
    bjt.set_prop_text("Gain", "250").unwrap();
    assert_eq!(bjt.gain, 250.0);
    bjt.set_prop_text("Vcrit", "0.65 V").unwrap();
    assert!((bjt.threshold - 0.65).abs() < 1e-4);
}

#[test]
fn test_mosfet_modes_and_rendering() {
    let mut mos = Mosfet::default();
    assert_eq!(mos.pin_geoms().len(), 3);

    // NMOS Enhancement
    let rec_nmos = record_part_paint(&Part::Mosfet(mos.clone()));
    assert!(rec_nmos.is_all_finite());

    // PMOS Enhancement
    mos.set_prop_text("PChannel", "true").unwrap();
    assert!(mos.p_channel);
    let rec_pmos = record_part_paint(&Part::Mosfet(mos.clone()));
    assert!(rec_pmos.is_all_finite());
    assert_ne!(rec_nmos.ops, rec_pmos.ops);

    // PMOS Depletion
    mos.set_prop_text("Depletion", "true").unwrap();
    assert!(mos.depletion);
    let rec_dep = record_part_paint(&Part::Mosfet(mos));
    assert!(rec_dep.is_all_finite());
    assert_ne!(rec_pmos.ops, rec_dep.ops);
}

#[test]
fn test_jfet_modes_and_rendering() {
    let mut jfet = Jfet::default();
    assert_eq!(jfet.pin_geoms().len(), 3);

    // N-Channel
    let rec_n = record_part_paint(&Part::Jfet(jfet.clone()));
    assert!(rec_n.is_all_finite());

    // P-Channel
    jfet.set_prop_text("PChannel", "true").unwrap();
    assert!(jfet.p_channel);
    let rec_p = record_part_paint(&Part::Jfet(jfet));
    assert!(rec_p.is_all_finite());
    assert_ne!(rec_n.ops, rec_p.ops);
}

#[test]
fn test_opamp_and_comparator() {
    let opamp = OpAmp::default();
    let pins = opamp.pin_geoms();
    assert!(
        pins.len() >= 3,
        "OpAmp must have at least 3 pins (+, -, out)"
    );
    let rec_op = record_part_paint(&Part::OpAmp(opamp));
    assert!(rec_op.is_all_finite());
    // OpAmp should draw triangular body and +/- symbols
    assert!(
        rec_op
            .ops
            .iter()
            .any(|op| matches!(op, DrawOp::FillPoly { .. } | DrawOp::StrokePoly { .. }))
    );

    let mut comp = Comparator::default();
    comp.set_prop_text("Inverted", "true").unwrap();
    comp.set_prop_text("OpenCollector", "true").unwrap();
    let rec_comp = record_part_paint(&Part::Comparator(comp));
    assert!(rec_comp.is_all_finite());
}

#[test]
fn test_voltage_regulator() {
    let mut vr = VoltReg::default();
    vr.set_prop_text("Voltage", "3.3 V").unwrap();
    assert_eq!(vr.voltage, 3.3);

    let pins = vr.pin_geoms();
    assert_eq!(
        pins.len(),
        3,
        "Voltage regulator must have 3 pins (In, Out, Gnd)"
    );
    let rec = record_part_paint(&Part::VoltReg(vr));
    assert!(rec.is_all_finite());
}

#[test]
fn test_thyristors_scr_diac_triac() {
    let scr = Scr::default();
    assert_eq!(
        scr.pin_geoms().len(),
        3,
        "SCR must have 3 pins (Anode, Cathode, Gate)"
    );
    let rec_scr = record_part_paint(&Part::Scr(scr));
    assert!(rec_scr.is_all_finite());

    let diac = Diac::default();
    assert_eq!(diac.pin_geoms().len(), 2, "DIAC must have 2 pins");
    let rec_diac = record_part_paint(&Part::Diac(diac));
    assert!(rec_diac.is_all_finite());

    let triac = Triac::default();
    assert_eq!(
        triac.pin_geoms().len(),
        3,
        "TRIAC must have 3 pins (MT1, MT2, Gate)"
    );
    let rec_triac = record_part_paint(&Part::Triac(triac));
    assert!(rec_triac.is_all_finite());
}
