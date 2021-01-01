//! Tests for display and indicator components: LEDs, 7-segments, matrices, and LCDs.

use super::recorder::record_part_paint;
use crate::components::led::Led;
use crate::components::seven_segment::SevenSegment;
use crate::components::*;

#[test]
fn test_led_color_and_properties() {
    let mut led = Led::default();
    led.set_prop_text("Color", "Green").unwrap();
    assert_eq!(led.color.as_str(), "Green");

    let rec = record_part_paint(&Part::Led(led.clone()));
    assert!(rec.is_all_finite());
    assert_eq!(led.pin_geoms().len(), 2, "LED has 2 pins (Anode, Cathode)");

    // Test Blue LED
    led.set_prop_text("Color", "Blue").unwrap();
    assert_eq!(led.color.as_str(), "Blue");
    let rec_blue = record_part_paint(&Part::Led(led));
    assert!(rec_blue.is_all_finite());
}

#[test]
fn test_seven_segment_displays_and_pins() {
    let mut seg = SevenSegment::default();
    assert_eq!(seg.num_displays, 1);
    assert!(!seg.common_anode);

    // Single display has 8 segment pins (a..g, dp) + common pin
    let pins1 = seg.pin_geoms();
    assert!(
        pins1.len() >= 8,
        "7-segment display must have at least 8 pins"
    );

    let rec1 = record_part_paint(&Part::SevenSegment(seg.clone()));
    assert!(rec1.is_all_finite());

    // Expand to 4 displays
    seg.set_prop_text("NumDisplays", "4").unwrap();
    assert_eq!(seg.num_displays, 4);

    let pins4 = seg.pin_geoms();
    assert!(
        pins4.len() > pins1.len(),
        "4-digit display must have more pins for digit selection or segments"
    );

    let rec4 = record_part_paint(&Part::SevenSegment(seg.clone()));
    assert!(rec4.is_all_finite());
    assert_ne!(
        rec1.ops, rec4.ops,
        "Display body width must expand with number of displays"
    );

    // Toggle common anode
    seg.set_prop_text("CommonAnode", "true").unwrap();
    assert!(seg.common_anode);
}

#[test]
fn test_rgb_led_and_ws2812() {
    let rgb = RgbLed::default();
    let pins = rgb.pin_geoms();
    assert_eq!(pins.len(), 4, "RGB LED must have 4 pins (R, G, B, Common)");
    let rec_rgb = record_part_paint(&Part::RgbLed(rgb));
    assert!(rec_rgb.is_all_finite());

    let ws = Ws2812::default();
    assert_eq!(ws.pin_geoms().len(), 4, "WS2812 has VDD, GND, DIN, DOUT");
    let rec_ws = record_part_paint(&Part::Ws2812(ws));
    assert!(rec_ws.is_all_finite());
}

#[test]
fn test_led_bar_and_led_matrix() {
    let mut bar = LedBar::default();
    bar.set_prop_text("Segments", "10").unwrap();
    assert_eq!(bar.segments, 10);
    let pins = bar.pin_geoms();
    assert!(
        pins.len() >= 10,
        "10-segment LED bar must have at least 10 pins (or 20 for isolated)"
    );
    let rec_bar = record_part_paint(&Part::LedBar(bar));
    assert!(rec_bar.is_all_finite());

    let mut mat = LedMatrix::default();
    mat.set_prop_text("Rows", "8").unwrap();
    mat.set_prop_text("Cols", "8").unwrap();
    let rec_mat = record_part_paint(&Part::LedMatrix(mat));
    assert!(rec_mat.is_all_finite());
}

#[test]
fn test_graphic_and_char_lcds() {
    // HD44780 (11 logic pins)
    let lcd = Hd44780::default();
    let pins_lcd = lcd.pin_geoms();
    assert_eq!(
        pins_lcd.len(),
        11,
        "HD44780 LCD logic controller has 11 pins (RS, RW, En, D0..D7)"
    );
    let rec_lcd = record_part_paint(&Part::Hd44780(lcd));
    assert!(rec_lcd.is_all_finite());

    // SSD1306 OLED (I2C: SCL, SDA)
    let oled = Ssd1306::default();
    let pins_oled = oled.pin_geoms();
    assert_eq!(pins_oled.len(), 2, "SSD1306 I2C has SCL and SDA pins");
    let rec_oled = record_part_paint(&Part::Ssd1306(oled));
    assert!(rec_oled.is_all_finite());

    // MAX7219 / MAX7221
    let max = Max72xx::default();
    assert_eq!(
        max.pin_geoms().len(),
        5,
        "MAX72xx module has VCC, GND, DIN, CS, CLK"
    );
    let rec_max = record_part_paint(&Part::Max72xx(max));
    assert!(rec_max.is_all_finite());
}
