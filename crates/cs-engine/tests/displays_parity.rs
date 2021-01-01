use cs_engine::canvas::{Canvas, Point};

#[test]
fn test_graphic_and_character_displays_canvas_and_sim1_roundtrip() {
    let mut canvas = Canvas::new();

    assert!(canvas.scene_add_component_spec("ILI9341", Point::new(100.0, 100.0)));
    assert!(canvas.scene_add_component_spec("ST7789", Point::new(300.0, 100.0)));
    assert!(canvas.scene_add_component_spec("ST7735", Point::new(500.0, 100.0)));
    assert!(canvas.scene_add_component_spec("GC9A01A", Point::new(700.0, 100.0)));
    assert!(canvas.scene_add_component_spec("PCD8544", Point::new(100.0, 300.0)));
    assert!(canvas.scene_add_component_spec("SH1107", Point::new(300.0, 300.0)));
    assert!(canvas.scene_add_component_spec("KS0108", Point::new(500.0, 300.0)));
    assert!(canvas.scene_add_component_spec("PCF8833", Point::new(700.0, 300.0)));
    assert!(canvas.scene_add_component_spec("AIP31068", Point::new(100.0, 500.0)));
    assert!(canvas.scene_add_component_spec("SSD1306", Point::new(300.0, 500.0)));

    // Verify all 10 items exist
    assert_eq!(canvas.scene().items().len(), 10);

    // Verify properties
    let ili = canvas.scene().items()[0].clone();
    assert_eq!(ili.controller(), "ILI9341");
    assert_eq!(ili.disp_width(), 320);
    assert_eq!(ili.disp_height(), 240);

    let pcd = canvas.scene().items()[4].clone();
    assert_eq!(pcd.disp_width(), 84);
    assert_eq!(pcd.disp_height(), 48);
    assert_eq!(pcd.contrast(), 50);
    assert_eq!(pcd.bias(), 4);

    let sh = canvas.scene().items()[5].clone();
    assert_eq!(sh.disp_width(), 128);
    assert_eq!(sh.disp_height(), 128);

    let ks = canvas.scene().items()[6].clone();
    assert_eq!(ks.disp_width(), 128);
    assert_eq!(ks.disp_height(), 64);

    let pcf = canvas.scene().items()[7].clone();
    assert_eq!(pcf.disp_width(), 132);
    assert_eq!(pcf.disp_height(), 132);

    let aip = canvas.scene().items()[8].clone();
    assert_eq!(aip.prop_text("Rows").as_deref(), Some("2"));
    assert_eq!(aip.prop_text("Cols").as_deref(), Some("16"));

    let oled = canvas.scene().items()[9].clone();
    assert_eq!(oled.disp_width(), 128);
    assert_eq!(oled.disp_height(), 64);
    assert_eq!(oled.prop_text("Color").as_deref(), Some("White"));

    // Test XML export & re-import
    let xml = canvas.scene().to_sim1();
    assert!(xml.contains("itemtype=\"TftDisplay\""));
    assert!(xml.contains("itemtype=\"Pcd8544\""));
    assert!(xml.contains("itemtype=\"Sh1107\""));
    assert!(xml.contains("itemtype=\"Ks0108\""));
    assert!(xml.contains("itemtype=\"Pcf8833\""));
    assert!(xml.contains("itemtype=\"Aip31068\""));
    assert!(xml.contains("itemtype=\"Ssd1306\""));

    let mut canvas2 = Canvas::new();
    canvas2.import_sim1(&xml);
    assert_eq!(canvas2.scene().items().len(), 10);
}

#[test]
fn test_display_property_mutations() {
    let mut canvas = Canvas::new();
    canvas.scene_add_component_spec("TFTDisplay", Point::new(100.0, 100.0));
    canvas.scene_add_component_spec("PCD8544", Point::new(200.0, 100.0));
    canvas.scene_add_component_spec("AIP31068", Point::new(300.0, 100.0));

    let tft_id = canvas.scene().items()[0].id.clone();
    let pcd_id = canvas.scene().items()[1].id.clone();
    let aip_id = canvas.scene().items()[2].id.clone();

    // Mutate TFT properties
    let mut tft = canvas.scene().item_by_id(&tft_id).unwrap().clone();
    assert!(tft.set_prop_text("Width", "240"));
    assert!(tft.set_prop_text("Height", "320"));
    assert!(tft.set_prop_bool("BGR", true));
    assert_eq!(tft.disp_width(), 240);
    assert_eq!(tft.disp_height(), 320);
    assert_eq!(tft.prop_bool("BGR"), Some(true));

    // Mutate PCD8544 properties
    let mut pcd = canvas.scene().item_by_id(&pcd_id).unwrap().clone();
    assert!(pcd.set_prop_text("Contrast", "70"));
    assert!(pcd.set_prop_text("Bias", "3"));
    assert_eq!(pcd.contrast(), 70);
    assert_eq!(pcd.bias(), 3);

    // Mutate AIP31068 properties
    let mut aip = canvas.scene().item_by_id(&aip_id).unwrap().clone();
    assert!(aip.set_prop_text("Rows", "4"));
    assert!(aip.set_prop_text("Cols", "20"));
    assert_eq!(aip.prop_text("Rows").as_deref(), Some("4"));
    assert_eq!(aip.prop_text("Cols").as_deref(), Some("20"));
}

#[test]
fn test_ssd1306_canvas_properties_and_sim_sync() {
    let mut canvas = Canvas::new();
    assert!(canvas.scene_add_component_spec("SSD1306", Point::new(150.0, 150.0)));

    let ssd_id = canvas.scene().items()[0].id.clone();

    // Default inspection
    let ssd_item = canvas.scene().item_by_id(&ssd_id).unwrap().clone();
    assert_eq!(ssd_item.disp_width(), 128);
    assert_eq!(ssd_item.disp_height(), 64);
    assert_eq!(ssd_item.prop_text("Color").as_deref(), Some("White"));
    assert_eq!(ssd_item.prop_bool("Rotate"), Some(true));
    assert_eq!(ssd_item.prop_text("Control_Code").as_deref(), Some("60")); // 0x3C = 60

    // Mutate properties
    {
        let mutated = canvas.scene_mut().item_by_id_mut(&ssd_id).unwrap();
        assert!(mutated.set_prop_text("Color", "Blue"));
        assert!(mutated.set_prop_text("Width", "64"));
        assert!(mutated.set_prop_text("Height", "32"));
        assert!(mutated.set_prop_bool("Rotate", false));
        assert!(mutated.set_prop_text("Control_Code", "61")); // 0x3D
        assert!(mutated.set_prop_text("Frequency", "400"));

        assert_eq!(mutated.prop_text("Color").as_deref(), Some("Blue"));
        assert_eq!(mutated.disp_width(), 64);
        assert_eq!(mutated.disp_height(), 32);
        assert_eq!(mutated.prop_bool("Rotate"), Some(false));
        assert_eq!(mutated.prop_text("Control_Code").as_deref(), Some("61"));
    }

    // Roundtrip through SIM1 XML
    let xml = canvas.scene().to_sim1();
    assert!(xml.contains("itemtype=\"Ssd1306\""));
    assert!(xml.contains("Color=\"Blue\""));
    assert!(xml.contains("Width=\"64\""));
    assert!(xml.contains("Height=\"32\""));
    assert!(xml.contains("Rotate=\"false\""));
    assert!(xml.contains("Control_Code=\"61\""));

    let mut canvas2 = Canvas::new();
    canvas2.import_sim1(&xml);
    let loaded = canvas2.scene().item_by_id(&ssd_id).unwrap();
    assert_eq!(loaded.prop_text("Color").as_deref(), Some("Blue"));
    assert_eq!(loaded.disp_width(), 64);
    assert_eq!(loaded.disp_height(), 32);
    assert_eq!(loaded.prop_bool("Rotate"), Some(false));
    assert_eq!(loaded.prop_text("Control_Code").as_deref(), Some("61"));
}
