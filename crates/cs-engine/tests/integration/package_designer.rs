use cs_engine::canvas::Canvas;
use cs_engine::canvas::export::{Palette, svg_string};
use cs_engine::canvas::scene::Scene;
use cs_engine::package::{
    Package, PkgPin, convert_package, generate_dip_footprint, generate_ls_footprint, generate_pins,
    package_pins_to_sim1_prop, package_to_xml, parse_sim1_package_pins,
};

#[test]
fn test_package_struct_and_pin_helpers() {
    let mut pkg = Package::default();
    pkg.name = "TestIC".to_string();
    pkg.width = 4;
    pkg.height = 6;
    pkg.custom_color = true;
    pkg.bckgndcolor = "#2a3b4c".to_string();

    let mut pin1 = PkgPin::new("1", "VCC", "", -8, 8, 180);
    assert!(!pin1.inverted());
    assert!(!pin1.unused());
    assert!(!pin1.is_bus());
    assert!(!pin1.is_point());

    pin1.set_point(true);
    assert!(pin1.is_point());
    assert_eq!(pin1.length, 1);

    pkg.pins.push(pin1);

    let pin2 = PkgPin::new("2", "~RESET", "inv", -8, 16, 180);
    assert!(pin2.inverted());
    pkg.pins.push(pin2);

    let pin3 = PkgPin::new("3", "NC", "nc", 40, 8, 0);
    assert!(pin3.unused());
    pkg.pins.push(pin3);

    let pin4 = PkgPin::new("4", "DATA", "bus", 40, 16, 0);
    assert!(pin4.is_bus());
    pkg.pins.push(pin4);

    assert_eq!(pkg.pins.len(), 4);
    assert_eq!(pkg.electrical_pins().count(), 3); // NC is non-electrical
    assert!(pkg.find_pin("2").is_some());
    assert!(pkg.find_pin("99").is_none());
}

#[test]
fn test_package_xml_generation_and_roundtrip() {
    let mut pkg = Package {
        name: "ATmega328P".to_string(),
        width: 6,
        height: 15,
        subc_type: cs_engine::package::SubcType::MCU,
        logic_symbol: false,
        border: true,
        custom_color: true,
        bckgndcolor: "#1e1e2e".to_string(),
        background: "atmega.png".to_string(),
        package_file: "ATmega328P.package".to_string(),
        pins: Vec::new(),
    };

    pkg.pins.push(PkgPin::new("1", "PC6", "rst", -8, 8, 180));
    pkg.pins.push(PkgPin::new("2", "PD0", "", -8, 16, 180));
    pkg.pins.push(PkgPin::new("7", "VCC", "", -8, 56, 180));
    pkg.pins.push(PkgPin::new("8", "GND", "", -8, 64, 180));
    pkg.pins.push(PkgPin::new("14", "PB0", "", 56, 8, 0));
    pkg.pins.push(PkgPin::new("NC", "NC", "nc", 24, 128, 270));

    let xml = package_to_xml(&pkg);
    assert!(xml.contains("<packageB"));
    assert!(xml.contains("name=\"ATmega328P\""));
    assert!(xml.contains("custom_color=\"true\""));
    assert!(xml.contains("bckgndcolor=\"#1e1e2e\""));
    assert!(xml.contains("<pin type=\"rst\""));
    assert!(xml.contains("<pin type=\"nc\""));

    let (_, loaded) = convert_package(&xml);
    assert_eq!(loaded.name, "ATmega328P");
    assert_eq!(loaded.width, 6);
    assert_eq!(loaded.height, 15);
    assert_eq!(loaded.subc_type, cs_engine::package::SubcType::MCU);
    assert!(loaded.custom_color);
    assert_eq!(loaded.bckgndcolor, "#1e1e2e");
    assert_eq!(loaded.pins.len(), 6);
    assert_eq!(loaded.pins[0].id, "1");
    assert_eq!(loaded.pins[0].pin_type, "rst");
    assert!(loaded.pins[5].unused());
}

#[test]
fn test_dip_footprint_generation() {
    let dip16 = generate_dip_footprint("74HC595", 16, None);
    assert_eq!(dip16.name, "74HC595");
    assert_eq!(dip16.width, 4);
    assert_eq!(dip16.height, 9);
    assert_eq!(dip16.pins.len(), 16);

    // Left side: 1 to 8 (top to bottom)
    for i in 0..8 {
        let pin = &dip16.pins[i];
        assert_eq!(pin.id, (i + 1).to_string());
        assert_eq!(pin.angle, 180);
        assert_eq!(pin.xpos, -8);
        assert_eq!(pin.ypos, 8 * (i as i32 + 1));
    }

    // Right side: 16 down to 9 (bottom to top counter-clockwise)
    for i in 0..8 {
        let pin = &dip16.pins[8 + i];
        assert_eq!(pin.id, (16 - i).to_string());
        assert_eq!(pin.angle, 0);
        assert_eq!(pin.xpos, 4 * 8 + 8);
        assert_eq!(pin.ypos, 8 * (i as i32 + 1));
    }
}

#[test]
fn test_ls_footprint_generation() {
    let ls = generate_ls_footprint(
        "NAND_QUAD",
        &["1A", "!1B", "2A", "!2B"],
        &["1Y", "2Y"],
        &["VCC"],
        &["GND"],
    );
    assert_eq!(ls.name, "NAND_QUAD");
    assert!(ls.logic_symbol);
    assert_eq!(ls.pins.len(), 8);

    assert_eq!(ls.pins[0].id, "1A");
    assert!(!ls.pins[0].inverted());

    assert_eq!(ls.pins[1].id, "1B");
    assert!(ls.pins[1].inverted());

    assert_eq!(ls.pins[4].id, "1Y");
    assert_eq!(ls.pins[4].angle, 0);

    assert_eq!(ls.pins[6].id, "VCC");
    assert_eq!(ls.pins[6].angle, 90);

    assert_eq!(ls.pins[7].id, "GND");
    assert_eq!(ls.pins[7].angle, 270);
}

#[test]
fn test_bulk_pin_generation_and_auto_resize() {
    let mut pkg = Package::default();
    pkg.width = 2;
    pkg.height = 2;

    generate_pins(&mut pkg, 6, 6, 3, 3, "GPIO_", 0, true);

    // Needed height should be max(6, 6) + 1 = 7
    assert_eq!(pkg.height, 7);
    // Needed width should be max(3, 3) + 1 = 4
    assert_eq!(pkg.width, 4);
    assert_eq!(pkg.pins.len(), 18);

    assert_eq!(pkg.pins[0].id, "GPIO_0");
    assert_eq!(pkg.pins[0].angle, 180);
    assert_eq!(pkg.pins[5].id, "GPIO_5");
    assert_eq!(pkg.pins[5].angle, 180);

    assert_eq!(pkg.pins[6].id, "GPIO_6");
    assert_eq!(pkg.pins[6].angle, 0);

    assert_eq!(pkg.pins[12].id, "GPIO_12");
    assert_eq!(pkg.pins[12].angle, 90);

    assert_eq!(pkg.pins[15].id, "GPIO_15");
    assert_eq!(pkg.pins[15].angle, 270);
}

#[test]
fn test_sim1_package_pins_prop_format() {
    let pins = vec![
        PkgPin::new("IN1", "IN1", "", -8, 8, 180),
        PkgPin::new("OUT1", "OUT1", "inv", 40, 8, 0),
        PkgPin::new("NC", "NC", "nc", 16, 40, 270),
    ];

    let prop_str = package_pins_to_sim1_prop(&pins);
    assert!(
        prop_str.contains(
            "Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=IN1; label=IN1"
        )
    );
    assert!(prop_str.contains(
        "Pin; type=inv; xpos=40; ypos=8; angle=0; length=8; space=0; id=OUT1; label=OUT1"
    ));
    assert!(
        prop_str.contains(
            "Pin; type=nc; xpos=16; ypos=40; angle=270; length=8; space=0; id=NC; label=NC"
        )
    );

    let parsed = parse_sim1_package_pins(&prop_str);
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].id, "IN1");
    assert!(parsed[1].inverted());
    assert!(parsed[2].unused());
}

#[test]
fn test_scene_subpackage_manipulation_and_sim1_serialization() {
    let mut scene = Scene::new();
    let id = scene.add_subpackage(100.0, 100.0);

    // Initial package
    assert!(scene.package_ref(&id).is_some());
    assert_eq!(scene.package_ref(&id).unwrap().pins.len(), 0);

    // Set DIP footprint
    assert!(scene.set_package_footprint_dip(&id, "DIP-8", 8, Some(4)));
    assert_eq!(scene.package_ref(&id).unwrap().pins.len(), 8);

    // Update pin
    let new_pin = PkgPin::new("1", "VCC_IN", "", -8, 8, 180);
    assert!(scene.update_package_pin(&id, "1", new_pin));
    assert_eq!(scene.package_ref(&id).unwrap().pins[0].label, "VCC_IN");

    // Remove pin
    assert!(scene.remove_package_pin(&id, "8"));
    assert_eq!(scene.package_ref(&id).unwrap().pins.len(), 7);

    // Add new pin
    assert!(scene.add_package_pin(&id, 90, 16, -8, "TOP", "TOP_PIN"));
    assert_eq!(scene.package_ref(&id).unwrap().pins.len(), 8);

    // Properties
    let item = scene.item_by_id_mut(&id).unwrap();
    assert_eq!(item.kind.type_name(), "SubPackage");
    assert!(item.set_prop_text("Name", "CustomChip"));
    assert!(item.set_prop_text("Width", "6"));
    assert!(item.set_prop_text("Height", "8"));
    assert!(item.set_prop_text("BckGndColor", "#3a3a5a"));
    assert!(item.set_prop_bool("CustomColor", true));

    assert_eq!(item.prop_text("Name").unwrap(), "CustomChip");
    assert_eq!(item.prop_text("Width").unwrap(), "6");
    assert_eq!(item.prop_text("Height").unwrap(), "8");
    assert_eq!(item.prop_text("BckGndColor").unwrap(), "#3a3a5a");
    assert_eq!(item.prop_bool("CustomColor"), Some(true));

    // Serialize to .sim1
    let sim1_str = scene.to_sim1();
    assert!(sim1_str.contains("itemtype=\"SubPackage\""));
    assert!(sim1_str.contains("Name=\"CustomChip\""));
    assert!(sim1_str.contains("Width=\"6\""));
    assert!(sim1_str.contains("Height=\"8\""));
    assert!(sim1_str.contains("CustomColor=\"true\""));
    assert!(sim1_str.contains("BckGndColor=\"#3a3a5a\""));
    assert!(sim1_str.contains("Pins=\""));

    // Roundtrip load from .sim1
    let loaded_scene = Scene::from_sim1(&sim1_str).expect("load sim1");
    assert_eq!(loaded_scene.items().len(), 1);
    let loaded_item = &loaded_scene.items()[0];
    assert_eq!(loaded_item.pkg_name(), "CustomChip");
    assert_eq!(loaded_item.pkg_w(), 6);
    assert_eq!(loaded_item.pkg_h(), 8);
    assert!(loaded_item.custom_color());
    assert_eq!(loaded_item.bckgnd_color(), "#3a3a5a");
    assert_eq!(loaded_item.pins().len(), 8);
}

#[test]
fn test_canvas_subpackage_workflow_and_svg_export() {
    let mut canvas = Canvas::new();
    let id = canvas.scene_mut().add_subpackage(50.0, 50.0);

    // Apply LS footprint via canvas
    canvas.set_package_footprint_ls(
        &id,
        "ALU_BLOCK",
        &["A0", "A1", "B0", "B1"],
        &["Y0", "Y1"],
        &["CLK"],
        &["GND"],
    );
    assert_eq!(canvas.scene().package_ref(&id).unwrap().pins.len(), 8);

    // Export XML
    let xml = canvas.export_package_xml(&id).expect("export xml");
    assert!(xml.contains("name=\"ALU_BLOCK\""));

    // Load XML into another package
    let id2 = canvas.scene_mut().add_subpackage(200.0, 50.0);
    canvas.load_package_xml(&id2, &xml);
    assert_eq!(canvas.scene().package_ref(&id2).unwrap().name, "ALU_BLOCK");
    assert_eq!(canvas.scene().package_ref(&id2).unwrap().pins.len(), 8);

    // Render SVG export
    let pal = Palette::light();
    let svg = svg_string(&canvas, &pal);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("ALU_BLOCK"));
}
