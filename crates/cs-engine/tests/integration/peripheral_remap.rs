use cs_engine::qemu::QemuComp;
use cs_engine::qemu::periph::{ESP32_IOMUX_START, Esp32IoMux, STM32_AFIO_START, Stm32Afio};

#[test]
fn test_stm32_afio_register_read_write() {
    let mut afio = Stm32Afio::new(STM32_AFIO_START);
    assert_eq!(afio.read(0x04), 0); // MAPR reset

    // Write MAPR: set USART1_REMAP (bit 2) and SPI1_REMAP (bit 0)
    afio.write(0x04, 0x05);
    assert!(afio.spi1_remap());
    assert!(afio.usart1_remap());
    assert!(!afio.usart2_remap());
}

#[test]
fn test_esp32_iomux_read_write() {
    let mut iomux = Esp32IoMux::new(ESP32_IOMUX_START);
    assert_eq!(iomux.read(0), 0);

    iomux.write(0x00, 0x1234);
    assert_eq!(iomux.read(0x00), 0x1234);

    iomux.set_func_in(5, 0x55);
    assert_eq!(iomux.func_in_sel[5], 0x55);
}

#[test]
fn test_stm32_peripheral_remap_integration() {
    let stm = QemuComp::stm32("mcu0", 5);
    assert!(stm.stm32_afio.is_some());
}
