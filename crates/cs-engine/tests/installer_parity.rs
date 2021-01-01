use cs_engine::catalog;
use cs_engine::installer;
use std::collections::BTreeMap;
use std::fs;

#[test]
fn test_installer_catalog_parsing_and_status() {
    let raw = r#"
MCUs and CPUs:
Arduino; Arduino boards.; Arduino.zip; 2507102250; AVR; Santiago
AVR; avr microcontrollers.; AVR.zip; 2507102250;; Atmel
PIC; 12/14 bit microcontrollers.; PIC.zip; 2512301110
Digital:
74; 7400 series ICs; 74.zip; 2507102250
Tools; Digital tools; Tools.zip; 2507102250
"#;

    let mut installed = BTreeMap::new();
    installed.insert("AVR".to_string(), 2507102250);
    installed.insert("74".to_string(), 2400000000); // Outdated version

    let items = installer::parse_components_txt(raw, &installed);
    assert_eq!(items.len(), 7);

    // Group header
    assert!(items[0].is_group_header());
    assert_eq!(items[0].name, "MCUs and CPUs:");

    // Arduino: not installed, depends on AVR
    let arduino = &items[1];
    assert_eq!(arduino.name, "Arduino");
    assert!(!arduino.installed());
    assert!(!arduino.can_update());
    assert_eq!(arduino.depends, "AVR");
    assert_eq!(arduino.author, "Santiago");

    // AVR: installed, up to date
    let avr = &items[2];
    assert_eq!(avr.name, "AVR");
    assert!(avr.installed());
    assert!(!avr.can_update());

    // 74: installed, can update
    let ic74 = &items[5];
    assert_eq!(ic74.name, "74");
    assert!(ic74.installed());
    assert!(ic74.can_update());

    // Tools: not installed
    let tools = &items[6];
    assert_eq!(tools.name, "Tools");
    assert!(!tools.installed());
    assert!(!tools.can_update());
}

#[test]
fn test_installer_zip_extraction_and_catalog_reload() {
    let tmp_dir =
        std::env::temp_dir().join(format!("cs_test_installer_extract_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    // Create a mock zip archive with a custom component XML and subcircuit
    let mut zip_data = Vec::new();
    let filename = b"MockSet/mock.xml";
    let content = br#"<itemlib><itemset name="MockSet" category="Custom">
    <item name="MockComp" type="Subcircuit" icon="mock_icon.png" />
</itemset></itemlib>"#;

    let local_header_offset = zip_data.len() as u32;
    zip_data.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
    zip_data.extend_from_slice(&[20, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0]); // Stored
    zip_data.extend_from_slice(&[0, 0, 0, 0]);
    zip_data.extend_from_slice(&[0, 0, 0, 0]);
    zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes());
    zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes());
    zip_data.extend_from_slice(&(filename.len() as u16).to_le_bytes());
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(filename);
    zip_data.extend_from_slice(content);

    let cd_offset = zip_data.len() as u32;
    zip_data.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]);
    zip_data.extend_from_slice(&[20, 0]);
    zip_data.extend_from_slice(&[20, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0, 0, 0]);
    zip_data.extend_from_slice(&[0, 0, 0, 0]);
    zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes());
    zip_data.extend_from_slice(&(content.len() as u32).to_le_bytes());
    zip_data.extend_from_slice(&(filename.len() as u16).to_le_bytes());
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0, 0, 0]);
    zip_data.extend_from_slice(&local_header_offset.to_le_bytes());
    zip_data.extend_from_slice(filename);

    let cd_size = (zip_data.len() as u32) - cd_offset;

    zip_data.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[0, 0]);
    zip_data.extend_from_slice(&[1, 0]);
    zip_data.extend_from_slice(&[1, 0]);
    zip_data.extend_from_slice(&cd_size.to_le_bytes());
    zip_data.extend_from_slice(&cd_offset.to_le_bytes());
    zip_data.extend_from_slice(&[0, 0]);

    let extracted = installer::extract_zip(&zip_data, &tmp_dir).unwrap();
    assert_eq!(extracted.len(), 1);

    let xml_file = tmp_dir.join("MockSet/mock.xml");
    assert!(xml_file.exists());

    // Test catalog loading from extracted set
    let mut cat = catalog::Catalog::new();
    cat.load_dir(&tmp_dir.join("MockSet"));
    assert!(cat.get("MockComp").is_some());
    let item = cat.get("MockComp").unwrap();
    assert_eq!(item.name, "MockComp");
    assert_eq!(item.typ, "Subcircuit");

    let _ = fs::remove_dir_all(&tmp_dir);
}
