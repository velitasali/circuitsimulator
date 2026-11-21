use cs_engine::canvas::{Canvas, Point};

#[test]
fn test_multiple_property_dialogs_lifecycle() {
    let mut canvas = Canvas::empty();
    canvas.set_view_size(800.0, 600.0);

    // 1. Add three distinct components
    canvas.add_component_at("Resistor", Point::new(10.0, 10.0));
    canvas.add_component_at("Capacitor", Point::new(100.0, 10.0));
    canvas.add_component_at("Switch", Point::new(200.0, 10.0));

    let res_id = canvas.scene().items()[0].id.clone();
    let cap_id = canvas.scene().items()[1].id.clone();
    let sw_id = canvas.scene().items()[2].id.clone();

    assert_eq!(canvas.open_prop_uids().len(), 0);
    assert!(!canvas.prop_open());

    // 2. Open properties for each component
    let c1 = canvas.open_properties(&res_id);
    assert!(c1.props);
    assert!(c1.open_props);
    assert_eq!(canvas.open_prop_uids(), &[res_id.clone()]);
    assert_eq!(canvas.last_opened_prop_uid(), Some(res_id.as_str()));
    assert!(canvas.prop_open());

    let c2 = canvas.open_properties(&cap_id);
    assert!(c2.props);
    assert!(c2.open_props);
    assert_eq!(canvas.open_prop_uids(), &[res_id.clone(), cap_id.clone()]);
    assert_eq!(canvas.last_opened_prop_uid(), Some(cap_id.as_str()));

    let c3 = canvas.open_properties(&sw_id);
    assert!(c3.props);
    assert!(c3.open_props);
    assert_eq!(
        canvas.open_prop_uids(),
        &[res_id.clone(), cap_id.clone(), sw_id.clone()]
    );
    assert_eq!(canvas.last_opened_prop_uid(), Some(sw_id.as_str()));

    // 3. Re-opening an already open component maintains uniqueness and updates last_opened_prop_uid
    let c_reopen = canvas.open_properties(&res_id);
    assert!(c_reopen.open_props);
    assert_eq!(
        canvas.open_prop_uids(),
        &[res_id.clone(), cap_id.clone(), sw_id.clone()]
    );
    assert_eq!(canvas.last_opened_prop_uid(), Some(res_id.as_str()));

    // 4. Independent mutations using set_prop_*_for
    canvas.set_prop_text_for(&res_id, "Resistance".into(), "2.2 kΩ".into());
    canvas.set_prop_text_for(&cap_id, "Capacitance".into(), "47 µF".into());
    canvas.set_prop_bool_for(&sw_id, "ShowButton".into(), true);
    canvas.set_prop_label_for(&res_id, "R_LOAD".into());
    canvas.set_prop_show_id_for(&res_id, true);

    let res_item = canvas.scene().item_by_id(&res_id).unwrap();
    assert_eq!(res_item.label, "R_LOAD");
    assert!(res_item.show_id);
    match &res_item.kind {
        cs_engine::components::Part::Resistor(r) => {
            assert!((r.resistance - 2200.0).abs() < 1e-6);
        }
        _ => panic!("expected resistor"),
    }

    let cap_item = canvas.scene().item_by_id(&cap_id).unwrap();
    match &cap_item.kind {
        cs_engine::components::Part::Capacitor(c) => {
            assert!((c.capacitance - 47e-6).abs() < 1e-12);
        }
        _ => panic!("expected capacitor"),
    }

    let sw_item = canvas.scene().item_by_id(&sw_id).unwrap();
    assert!(sw_item.show_button());

    // 5. Close a single dialog
    canvas.close_property_dialog(&cap_id);
    assert_eq!(canvas.open_prop_uids(), &[res_id.clone(), sw_id.clone()]);
    assert!(canvas.prop_open());

    // 6. Delete a component from the scene; delete_selected automatically cleans up open_prop_uids
    let res_idx = canvas
        .scene()
        .items()
        .iter()
        .position(|it| it.id == res_id)
        .unwrap();
    canvas.scene_mut().select_only(res_idx);
    let c_del = canvas.delete_selected();
    assert!(c_del.open_props);
    assert_eq!(canvas.open_prop_uids(), &[sw_id.clone()]);

    // 7. Clear circuit closes all property dialogs
    let c_new = canvas.new_circuit();
    assert!(c_new.open_props);
    assert_eq!(canvas.open_prop_uids().len(), 0);
    assert!(!canvas.prop_open());
}

#[test]
fn test_backward_compatibility_single_prop() {
    let mut canvas = Canvas::empty();
    canvas.add_component_at("Resistor", Point::zero());
    let id = canvas.scene().items()[0].id.clone();

    // Legacy open_properties
    canvas.open_properties(&id);
    assert!(canvas.prop_open());
    assert_eq!(canvas.prop_uid(), Some(id.as_str()));

    // Legacy set_prop_text
    canvas.set_prop_text("Resistance".into(), "10 kΩ".into());
    match &canvas.scene().items()[0].kind {
        cs_engine::components::Part::Resistor(r) => {
            assert!((r.resistance - 10_000.0).abs() < 1e-6);
        }
        _ => panic!("expected resistor"),
    }

    // Legacy close_properties
    canvas.close_properties();
    assert!(!canvas.prop_open());
    assert_eq!(canvas.open_prop_uids().len(), 0);
}
