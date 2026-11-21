use cs_engine::debug::symbols::DebugSymbols;
use cs_engine::debug::varset::{VarFormat, VarSet, VarType, WatchEntry};
use cs_engine::mcu::McuSnap;

#[test]
fn test_varset_direct_address() {
    let mut snap = McuSnap::default();
    snap.ram = vec![0x00; 256];
    snap.ram[0x20] = 42;
    snap.ram[0x21] = 0x01; // 0x012A = 298 in little endian

    let entry_u8 = WatchEntry::new("my_byte", "0x20", VarType::Uint8);
    let val_u8 = VarSet::eval_snap(&entry_u8, &snap, None).expect("eval u8");
    assert_eq!(val_u8.raw, 42);
    assert_eq!(val_u8.formatted, "42 (0x2A)");

    let entry_u16 = WatchEntry::new("my_word", "0x20", VarType::Uint16);
    let val_u16 = VarSet::eval_snap(&entry_u16, &snap, None).expect("eval u16");
    assert_eq!(val_u16.raw, 298);
    assert_eq!(val_u16.formatted, "298 (0x012A)");
}

#[test]
fn test_varset_sfr_register() {
    let mut snap = McuSnap::default();
    snap.ram = vec![0x00; 256];
    snap.registers.push(("PORTB".into(), 0x18));
    snap.ram[0x18] = 0x55;

    let entry = WatchEntry::new("port_b", "PORTB", VarType::Uint8);
    let val = VarSet::eval_snap(&entry, &snap, None).expect("eval SFR");
    assert_eq!(val.raw, 0x55);
    assert_eq!(val.formatted, "85 (0x55)");
}

#[test]
fn test_varset_symbol_lookup() {
    let mut snap = McuSnap::default();
    snap.ram = vec![0x00; 256];
    snap.ram[0x30] = 100;

    let mut syms = DebugSymbols::default();
    syms.insert_variable("counter", "uint8", 0x30, 1, false);

    let entry = WatchEntry::new("counter_var", "counter", VarType::Uint8);
    let val = VarSet::eval_snap(&entry, &snap, Some(&syms)).expect("eval symbol");
    assert_eq!(val.raw, 100);
    assert_eq!(val.formatted, "100 (0x64)");
}

#[test]
fn test_varset_formats() {
    let raw = vec![0x42];
    assert_eq!(VarFormat::Hex.format(&raw, &VarType::Uint8), "0x42");
    assert_eq!(VarFormat::Dec.format(&raw, &VarType::Uint8), "66");
    assert_eq!(VarFormat::Bin.format(&raw, &VarType::Uint8), "0b01000010");
    assert_eq!(VarFormat::Char.format(&raw, &VarType::Uint8), "'B'");
}
