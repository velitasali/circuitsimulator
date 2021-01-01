//! Tests for memory, RAM, ROM, ADC, DAC, and I2C expander components.

use super::recorder::record_part_paint;
use crate::components::adc::Adc;
use crate::components::dac::Dac;
use crate::components::memory::Memory;
use crate::components::*;

#[test]
fn test_adc_bits_and_pins() {
    let mut adc = Adc::default();
    adc.set_prop_text("Bits", "4").unwrap();
    assert_eq!(adc.bits, 4);

    let pins4 = adc.pin_geoms();
    assert_eq!(
        pins4.len(),
        5,
        "4-bit ADC has 1 analog input pin + 4 data output pins"
    );

    let rec4 = record_part_paint(&Part::Adc(adc.clone()));
    assert!(rec4.is_all_finite());

    // Scale to 8-bit ADC
    adc.set_prop_text("Bits", "8").unwrap();
    assert_eq!(adc.bits, 8);

    let pins8 = adc.pin_geoms();
    assert_eq!(
        pins8.len(),
        9,
        "8-bit ADC has 1 analog input pin + 8 data pins"
    );

    // Scale to 12-bit ADC
    adc.set_prop_text("Bits", "12").unwrap();
    assert_eq!(adc.bits, 12);
    let pins12 = adc.pin_geoms();
    assert_eq!(
        pins12.len(),
        13,
        "12-bit ADC has 1 analog input pin + 12 data pins"
    );

    let rec12 = record_part_paint(&Part::Adc(adc));
    assert!(rec12.is_all_finite());
    assert_ne!(
        rec4.ops, rec12.ops,
        "ADC chip body height expands with bit count >= 12"
    );
}

#[test]
fn test_dac_bits_and_pins() {
    let mut dac = Dac::default();
    dac.set_prop_text("Bits", "4").unwrap();
    assert_eq!(dac.bits, 4);

    let pins4 = dac.pin_geoms();
    assert_eq!(
        pins4.len(),
        5,
        "4-bit DAC has 4 input data pins + 1 analog output"
    );

    let rec4 = record_part_paint(&Part::Dac(dac.clone()));
    assert!(rec4.is_all_finite());

    dac.set_prop_text("Bits", "8").unwrap();
    let pins8 = dac.pin_geoms();
    assert_eq!(
        pins8.len(),
        9,
        "8-bit DAC has 8 data pins + 1 analog output"
    );

    let rec8 = record_part_paint(&Part::Dac(dac));
    assert!(rec8.is_all_finite());
}

#[test]
fn test_i2c_to_parallel_expander() {
    let mut expander = I2CToParallel::default();
    expander.set_prop_text("Address", "33").unwrap();
    assert_eq!(expander.address, 33);

    let pins = expander.pin_geoms();
    assert!(
        pins.len() >= 10,
        "PCF8574 has 8 I/O pins (P0..P7) + SDA, SCL, INT"
    );

    let rec = record_part_paint(&Part::I2CToParallel(expander));
    assert!(rec.is_all_finite());
}

#[test]
fn test_memory_ram_and_rom() {
    let mut mem = Memory::default();
    mem.set_prop_text("AddrBits", "4").unwrap();
    mem.set_prop_text("DataBits", "4").unwrap();
    mem.set_prop_text("IsRom", "true").unwrap();
    assert!(mem.is_rom);

    let pins4 = mem.pin_geoms();
    assert!(pins4.len() >= 8, "4-addr + 4-data bits memory");

    let rec = record_part_paint(&Part::Memory(mem));
    assert!(rec.is_all_finite());
}

#[test]
fn test_dynamic_memory_and_i2c_ram() {
    let dram = DynamicMemory::default();
    let pins_dram = dram.pin_geoms();
    assert!(pins_dram.len() >= 10);
    let rec_dram = record_part_paint(&Part::DynamicMemory(dram));
    assert!(rec_dram.is_all_finite());

    let i2c_ram = I2CRam::default();
    let pins_i2c = i2c_ram.pin_geoms();
    assert!(
        pins_i2c.len() >= 2,
        "I2C EEPROM has SDA, SCL, and address pins"
    );
    let rec_i2c = record_part_paint(&Part::I2CRam(i2c_ram));
    assert!(rec_i2c.is_all_finite());
}
