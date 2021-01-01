//! Tests for switches, relays, motors, and digital/analog sensors.

use super::recorder::record_part_paint;
use crate::components::switch_dip::SwitchDip;
use crate::components::*;

#[test]
fn test_switch_and_push_button() {
    let mut sw = Switch::default();
    let pins_sw = sw.pin_geoms();
    assert_eq!(pins_sw.len(), 2, "SPST switch has 2 pins");
    let rec_sw_open = record_part_paint(&Part::Switch(sw.clone()));
    assert!(rec_sw_open.is_all_finite());

    // Toggle switch closed
    sw.set_prop_text("Checked", "true").unwrap();
    let rec_sw_closed = record_part_paint(&Part::Switch(sw));
    assert!(rec_sw_closed.is_all_finite());

    let push = Push::default();
    assert_eq!(push.pin_geoms().len(), 2);
    let rec_push = record_part_paint(&Part::Push(push));
    assert!(rec_push.is_all_finite());
}

#[test]
fn test_switch_dip_size_and_states() {
    let mut dip = SwitchDip::default();
    dip.set_prop_text("Size", "4").unwrap();
    dip.set_prop_text("State", "5").unwrap(); // 0b0101

    let pins4 = dip.pin_geoms();
    assert_eq!(pins4.len(), 8, "4-switch DIP has 8 pins");

    let rec4 = record_part_paint(&Part::SwitchDip(dip.clone()));
    assert!(rec4.is_all_finite());

    // Expand to 8 switches
    dip.set_prop_text("Size", "8").unwrap();
    dip.set_prop_text("State", "170").unwrap(); // 0b10101010

    let pins8 = dip.pin_geoms();
    assert_eq!(pins8.len(), 16, "8-switch DIP has 16 pins");

    let rec8 = record_part_paint(&Part::SwitchDip(dip));
    assert!(rec8.is_all_finite());
    assert_ne!(
        rec4.ops, rec8.ops,
        "DIP package width/height and switch count must expand"
    );
}

#[test]
fn test_relay_pinout_and_drawing() {
    let relay = Relay::default();
    let pins = relay.pin_geoms();
    assert!(
        pins.len() >= 4,
        "Relay has coil pins and switch contact pins"
    );
    let rec = record_part_paint(&Part::Relay(relay));
    assert!(rec.is_all_finite());
}

#[test]
fn test_motors_dc_stepper_servo() {
    // DC Motor
    let motor = DcMotor::default();
    assert_eq!(motor.pin_geoms().len(), 2, "DC motor has 2 terminal pins");
    let rec_dc = record_part_paint(&Part::DcMotor(motor));
    assert!(rec_dc.is_all_finite());

    // Stepper Motor
    let stepper = Stepper::default();
    assert!(
        stepper.pin_geoms().len() >= 4,
        "Stepper motor has 4/5/6 coil pins"
    );
    let rec_step = record_part_paint(&Part::Stepper(stepper));
    assert!(rec_step.is_all_finite());

    // Servo Motor
    let servo = Servo::default();
    assert_eq!(servo.pin_geoms().len(), 3, "Servo has PWM, VCC, GND");
    let rec_servo = record_part_paint(&Part::Servo(servo));
    assert!(rec_servo.is_all_finite());
}

#[test]
fn test_sensors_ultrasonic_and_temp() {
    // HC-SR04 Ultrasonic (5 pins: -inpin, -vccpin, -trigpin, -outpin, -gndpin)
    let mut sr04 = SR04::default();
    sr04.set_prop_text("Distance", "125 cm").unwrap();
    assert_eq!(sr04.distance, 1.25);
    assert_eq!(
        sr04.pin_geoms().len(),
        5,
        "HC-SR04 has simulated in, VCC, TRIG, ECHO, GND"
    );
    let rec_sr04 = record_part_paint(&Part::SR04(sr04));
    assert!(rec_sr04.is_all_finite());

    // DHT22 Temp & Humidity
    let mut dht = DHT22::default();
    dht.set_prop_text("Temp", "24.5 °C").unwrap();
    dht.set_prop_text("Humi", "60 %").unwrap();
    assert_eq!(dht.temp, 24.5);
    assert_eq!(dht.humi, 60.0);
    assert_eq!(dht.pin_geoms().len(), 4, "DHT22 has 4 pins");
    let rec_dht = record_part_paint(&Part::DHT22(dht));
    assert!(rec_dht.is_all_finite());

    // DS18B20 1-Wire Temp
    let mut ds18 = DS18B20::default();
    ds18.set_prop_text("Temp", "37.0 °C").unwrap();
    assert_eq!(ds18.temp, 37.0);
    assert_eq!(ds18.pin_geoms().len(), 3, "DS18B20 has GND, DQ, VDD");
    let rec_ds18 = record_part_paint(&Part::DS18B20(ds18));
    assert!(rec_ds18.is_all_finite());

    // DS1307 RTC (3 pins: SDA, SCL, SQW)
    let rtc = DS1307::default();
    assert_eq!(
        rtc.pin_geoms().len(),
        3,
        "DS1307 RTC has SDA, SCL, SQW pins"
    );
    let rec_rtc = record_part_paint(&Part::DS1307(rtc));
    assert!(rec_rtc.is_all_finite());
}

#[test]
fn test_joystick_and_rotary_encoder() {
    // KY-023 Joystick (3 active pins: X, Y, Switch)
    let joy = KY023::default();
    assert_eq!(
        joy.pin_geoms().len(),
        3,
        "KY-023 Joystick has VRx, VRy, SW pins"
    );
    let rec_joy = record_part_paint(&Part::KY023(joy));
    assert!(rec_joy.is_all_finite());

    // KY-040 Rotary Encoder (3 active pins: CLK, DT, SW)
    let enc = KY040::default();
    assert_eq!(enc.pin_geoms().len(), 3, "KY-040 has CLK, DT, SW pins");
    let rec_enc = record_part_paint(&Part::KY040(enc));
    assert!(rec_enc.is_all_finite());
}
